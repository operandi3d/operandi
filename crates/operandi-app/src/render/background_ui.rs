//! Constrain to Background in the app (SPEC-BACKGROUND-CONSTRAINT v1.1,
//! BEHAVIOR-MESH-CONSTRAINT v1): the Mesh Constraint toolbar button and
//! popover (MC-03..06), its Tool Properties rows, Esc to turn it off, the
//! saved settings, and the view each press happens in (what Screen Axis
//! searches along). The constraint itself is in
//! `operandi_core::background_constraint`. Mesh Constraint is not part of
//! Snapping (MC-01/02): its own switch, popover and saved settings.
use super::*;
use crate::form::Unit;
use crate::prefs_file::PrefsFile;
use crate::snapping::{HeaderAction, PopoverState, popover_header, show_popover};
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use crate::unavailable::Gate;
use operandi_core::background_constraint::{
    BackgroundConstraint, ConstraintView, GeometryConstraint,
};
use serde::{Deserialize, Serialize};

/// The input-map action (no default shortcut, MC-03).
pub(crate) const ACTION: &str = "Constrain to Background";
/// The toolbar button's name while off (MC-03).
pub(crate) const BUTTON: &str = "Mesh Constraint";
pub(crate) const POPOVER_ACTION: &str = "Mesh Constraints";
/// Guide and Primitive modes aren't built (MC-05, Operandi choice).
pub(crate) const NOT_BUILT: &str = "Not built yet";

/// The constraint modes on the popover's strip (MC-05). Only Background
/// is built, so it is also the mode the button turns on (MC-03, MC-11).
pub(crate) const MODES: [&str; 4] = ["(none)", "Guide", "Background", "Primitive"];

/// The saved settings (MC-06): the background options the button turns on
/// with, kept across on/off and across launches, and a pinned popover's
/// place. The on/off state itself starts off each launch. Target Item
/// names an item of one scene, so it isn't saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshConstraintPreferences {
    pub(crate) version: u8,
    pub background: BackgroundConstraint,
    /// A pinned popover's place, so it comes back next launch (UIC-64).
    pub pinned_at: Option<[f32; 2]>,
}
impl Default for MeshConstraintPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            background: BackgroundConstraint::default(),
            pinned_at: None,
        }
    }
}
impl PrefsFile for MeshConstraintPreferences {
    const INVALID: &'static str = "Invalid Mesh Constraint preference";
    fn valid(&self) -> bool {
        self.version == 1
            && self.background.validate().is_ok()
            && self
                .pinned_at
                .is_none_or(|[x, y]| x.is_finite() && y.is_finite())
    }
}

/// The constraint's rows. `target`: Target Item is listed, last (Tool
/// Properties only, MC-05). UIC-105: one section. UIC-93: the controlling
/// Geometry Constraint first, its Off greying the rest in place (UIC-117).
fn background_form(
    app: &Operandi,
    p: &BackgroundConstraint,
    target: bool,
) -> ToolForm<BackgroundConstraint> {
    let mut f = ToolForm::new();
    let modes: Vec<(GeometryConstraint, &str)> = GeometryConstraint::ALL
        .into_iter()
        .map(|m| (m, m.label()))
        .collect();
    let off = p.mode == GeometryConstraint::Off;
    let why_off = "Geometry Constraint is Off";
    let mut rows = vec![
        tf::choice(
            "bg_mode",
            "Geometry Constraint",
            &modes,
            |p: &BackgroundConstraint| p.mode,
            |p, m| p.mode = m,
        )
        .tip(
            "How a point finds the background: Screen Axis along the view, Vector \
             along its own motion, Point to the nearest surface",
        ),
        tf::number(
            "bg_offset",
            "Offset",
            Unit::Distance,
            |p: &BackgroundConstraint| p.offset,
            |p, v| {
                p.offset = v;
                p.validate()
            },
        )
        .suffix("m")
        .tip("How far off the surface constrained points rest, along its normal")
        .enabled_if(!off, why_off),
        // UIC-115 checkbox rows.
        tf::check(
            "bg_negative",
            "Negative Offset",
            |p: &BackgroundConstraint| p.negative_offset,
            |p, on| {
                p.negative_offset = on;
                if !on {
                    p.offset = p.offset.max(0.);
                }
            },
        )
        .tip("Allow a negative Offset (points sit under the surface)")
        .enabled_if(!off, why_off),
        // Stored and saved, but the only curve handles (the Draw tools'
        // Bezier handles) aren't constrained, as Draw tools aren't (BG-Q8):
        // greyed with the reason rather than a switch with nothing behind it.
        tf::check(
            "bg_handle",
            "Handle Constraint",
            |p: &BackgroundConstraint| p.handle_constraint,
            |p, on| p.handle_constraint = on,
        )
        .tip("Also constrain the handles of curve-drawing tools")
        .enabled_if(
            false,
            "Draw tools aren't constrained yet, so neither are their curve handles",
        ),
        tf::check(
            "bg_double",
            "Double Sided",
            |p: &BackgroundConstraint| p.double_sided,
            |p, on| p.double_sided = on,
        )
        .tip("Background polygons count from both sides; off, only those facing you")
        .enabled_if(!off, why_off),
    ];
    if target {
        // Target Item (spec v1.1): every background mesh, or one of them.
        let scene = &app.editor.scene;
        let mut targets: Vec<(Option<u64>, String)> = vec![(None, "(All Background)".into())];
        targets.extend(
            operandi_core::background_constraint::background_ids(scene)
                .into_iter()
                .chain(p.target)
                .filter_map(|id| scene.item(id))
                .map(|i| (Some(i.id), i.name.clone())),
        );
        targets.dedup_by_key(|t| t.0);
        let target_refs: Vec<(Option<u64>, &str)> =
            targets.iter().map(|(id, n)| (*id, n.as_str())).collect();
        rows.push(
            tf::choice(
                "bg_target",
                "Target Item",
                &target_refs,
                |p: &BackgroundConstraint| p.target,
                |p, t| p.target = t,
            )
            .tip("Only this item counts as background; All Background uses every shown, inactive mesh")
            .enabled_if(!off, why_off),
        );
    }
    f.section(
        "Constrain to Background",
        false,
        rows.into_iter()
            .map(|f| tf::Item::Field(Box::new(f)))
            .collect(),
    );
    f
}

fn publish_background(app: &mut Operandi, p: BackgroundConstraint) -> Result<()> {
    if app.editor.background_constraint == Some(p) {
        return Ok(());
    }
    app.editor.dispatch(Command::BackgroundConstraint(Some(p)))
}

/// The constraint's rows in Tool Properties, under whatever tool is open
/// (or alone) while it is on: it is part of the tool setup.
pub(super) struct BackgroundFields;

impl ToolFields for BackgroundFields {
    type P = BackgroundConstraint;
    fn params(app: &Operandi) -> Option<BackgroundConstraint> {
        app.editor.background_constraint
    }
    fn form(app: &Operandi, p: &BackgroundConstraint) -> ToolForm<BackgroundConstraint> {
        let mut f = background_form(app, p, true);
        f.note("Esc with no tool edit in progress turns it off (Mesh Constraint button).");
        f
    }
    fn publish(app: &mut Operandi, p: BackgroundConstraint) -> Result<()> {
        publish_background(app, p)
    }
    fn published(app: &mut Operandi) {
        app.status = "Background Constraint changed".into();
    }
}

/// The same rows in the Mesh Constraint popover, without Target Item
/// (MC-05).
pub(super) struct PopoverFields;

impl ToolFields for PopoverFields {
    type P = BackgroundConstraint;
    fn params(app: &Operandi) -> Option<BackgroundConstraint> {
        app.editor.background_constraint
    }
    fn form(app: &Operandi, p: &BackgroundConstraint) -> ToolForm<BackgroundConstraint> {
        background_form(app, p, false)
    }
    fn publish(app: &mut Operandi, p: BackgroundConstraint) -> Result<()> {
        publish_background(app, p)
    }
    fn published(app: &mut Operandi) {
        app.status = "Background Constraint changed".into();
    }
}

/// The toolbar button's label (MC-03): the mode's name while on, "Mesh
/// Constraint" while off, "Options" with Option held (UIC-27).
pub(crate) fn button_label(on: bool, options: bool) -> &'static str {
    if options {
        "Options"
    } else if on {
        "Background"
    } else {
        BUTTON
    }
}

impl Operandi {
    /// MC-03: the Mesh Constraint toolbar button. Click turns it on with
    /// the last settings, or off; on is the accent colour (UIC-23). With
    /// Option held it reads Options (UIC-27) and a click opens the popover
    /// at the pointer; right-click opens it too.
    pub(super) fn mesh_constraint_button(&mut self, ui: &mut egui::Ui) {
        let options = ui.input(|i| i.modifiers.alt);
        let on = self.editor.background_constraint.is_some();
        let label = button_label(on, options);
        // UIC-04: name and shortcut, the usage hint and the Option
        // alternate, then the state.
        let tip = format!(
            "{BUTTON}{} · {}\nPoints tools move land on the background meshes · hold Option and \
             click for Mesh Constraint options",
            self.shortcut(ACTION),
            if on { "Background" } else { "off" }
        );
        let response = ui
            .add(icons::button(ui, icons::Icon::Cube, label).selected(on && !options))
            .on_hover_text(tip);
        if response.clicked() {
            if options {
                self.open_mesh_constraint_popover();
            } else {
                self.toggle_background_constraint();
            }
        }
        if response.secondary_clicked() {
            self.open_mesh_constraint_popover();
        }
    }
    /// On with the last options, or off. The open tool stays.
    pub(super) fn toggle_background_constraint(&mut self) {
        let next = match self.editor.background_constraint {
            Some(_) => None,
            None => Some(self.editor.background_constraint_preset),
        };
        self.run(Command::BackgroundConstraint(next));
        if self.editor.background_constraint.is_some() {
            self.tool_properties_open = true;
        }
        self.status = format!(
            "Mesh Constraint {}",
            if self.editor.background_constraint.is_some() {
                "Background"
            } else {
                "off"
            }
        );
    }
    /// Esc over a view with the constraint on and no tool edit under way
    /// (no live tool edit, Pen draft, draw tool or brush): it goes, its
    /// settings kept, and nothing else happens (MC-04, MC-Q4; BG-Q10).
    pub(super) fn escape_clears_background_constraint(&self) -> bool {
        self.editor.background_constraint.is_some()
            && !self.editor.session.live.as_ref().is_some_and(|l| l.applied)
            && self
                .editor
                .session
                .pen_draft
                .as_ref()
                .is_none_or(|d| d.points().is_empty())
            && self.draw.is_none()
            && self.utility.is_none()
            && self.sculpt.brush.is_none()
    }
    /// Record the view a press happens in, for Screen Axis.
    pub(super) fn note_constraint_view(&mut self) {
        let (eye, _, _, forward) = self.camera.basis();
        self.editor.set_constraint_view(ConstraintView {
            forward,
            eye: (self.camera.view == View::Perspective).then_some(eye),
        });
    }

    /// The saved settings, at launch: the button turns on with them.
    pub(super) fn load_mesh_constraint(&mut self) {
        self.mesh_constraint = MeshConstraintPreferences::load(
            &self.environment.paths().mesh_constraint_preferences(),
        );
        self.editor.background_constraint_preset = self.mesh_constraint.background;
        if let Some([x, y]) = self.mesh_constraint.pinned_at {
            self.mesh_constraint_popover = Some(Pos2::new(x, y));
            self.mesh_constraint_pinned = true;
        }
    }
    /// Once a frame: save the settings when they changed, however they
    /// changed (popover, Tool Properties, `tool.attr`, the MCP bridge).
    pub(super) fn sync_mesh_constraint(&mut self) {
        let mut now = self.editor.background_constraint_preset;
        now.target = None;
        let pinned_at = self
            .mesh_constraint_popover
            .filter(|_| self.mesh_constraint_pinned)
            .map(|p| [p.x, p.y]);
        if now == self.mesh_constraint.background && pinned_at == self.mesh_constraint.pinned_at {
            return;
        }
        self.mesh_constraint.background = now;
        self.mesh_constraint.pinned_at = pinned_at;
        if let Err(error) = self
            .mesh_constraint
            .save(&self.environment.paths().mesh_constraint_preferences())
        {
            self.status = format!("Mesh Constraint changed; could not save it: {error}");
        }
    }

    /// MC-03/05: open the popover at the pointer (Option-click on the
    /// button, right-click, Edit ▸ Snapping ▸ Mesh Constraints). Pressed
    /// again while open and unpinned, it closes.
    pub(super) fn open_mesh_constraint_popover(&mut self) {
        if self.mesh_constraint_popover.is_some() && !self.mesh_constraint_pinned {
            self.close_mesh_constraint_popover();
            return;
        }
        // UIC-181: one unpinned popover at a time.
        if self.snapping_options.is_some() && !self.snapping_pinned {
            self.close_snapping_options();
        }
        if self.corner.properties.view.is_some() && !self.corner.properties.pinned {
            self.close_view_properties();
        }
        self.mesh_constraint_popover = Some(Pos2::new(f32::NAN, f32::NAN));
        self.mesh_constraint_entered = false;
    }
    pub(super) fn close_mesh_constraint_popover(&mut self) {
        self.mesh_constraint_popover = None;
        self.mesh_constraint_pinned = false;
        self.mesh_constraint_entered = false;
    }
    /// The popover, each frame it is open: the Snapping popover's frame
    /// and rules (UIC-62/64/181/185/187).
    pub(super) fn mesh_constraint_window(&mut self, ctx: &egui::Context) {
        let Some(at) = self.mesh_constraint_popover else {
            return;
        };
        let state = PopoverState {
            at,
            pinned: self.mesh_constraint_pinned,
            entered: self.mesh_constraint_entered,
            opening: false,
        };
        let fallback = self.viewport_rect.left_top() + egui::vec2(40., 40.);
        let click_off = self.ui_preferences.click_off();
        match show_popover(
            ctx,
            "mesh-constraint-popover",
            state,
            fallback,
            click_off,
            |ui| self.mesh_constraint_popover_ui(ui),
        ) {
            None => self.close_mesh_constraint_popover(),
            Some(state) => {
                self.mesh_constraint_popover = Some(state.at);
                self.mesh_constraint_pinned = state.pinned;
                self.mesh_constraint_entered = state.entered;
            }
        }
    }
    /// MC-05: the rows. UIC-181: no title bar (the button names it), only
    /// the pin circle; then the Constraint Mode strip and, while
    /// Background is the mode, its group.
    fn mesh_constraint_popover_ui(&mut self, ui: &mut egui::Ui) -> HeaderAction {
        ui.set_width(300.);
        ui.spacing_mut().item_spacing.y = 2.;
        let action = popover_header(ui, self.mesh_constraint_pinned);
        let on = self.editor.background_constraint.is_some();
        ui.label("Constraint Mode");
        ui.horizontal(|ui| {
            for mode in MODES {
                match mode {
                    "(none)" => {
                        let r = ui
                            .add(egui::Button::selectable(!on, mode))
                            .on_hover_text("No mesh constraint");
                        if r.clicked() && on {
                            self.toggle_background_constraint();
                        }
                    }
                    "Background" => {
                        let r = ui
                            .add(egui::Button::selectable(on, mode))
                            .on_hover_text("Points land on the background meshes");
                        if r.clicked() && !on {
                            self.toggle_background_constraint();
                        }
                    }
                    _ => {
                        ui.gated(Some(NOT_BUILT), egui::Button::selectable(false, mode));
                    }
                }
            }
        });
        if self.editor.background_constraint.is_some() {
            ui.separator();
            self.tool_panel::<PopoverFields>(ui);
        }
        action
    }
}

/// The reference tool id (registry thread, 2026-11-13).
pub(crate) const TOOL_ID: &str = "const.bg";

impl Operandi {
    /// `tool.set const.bg on|off|?`.
    pub(super) fn const_bg_set(&mut self, mode: &str) {
        let on = self.editor.background_constraint.is_some();
        match mode.to_ascii_lowercase().as_str() {
            "on" | "1" | "true" if !on => self.toggle_background_constraint(),
            "off" | "0" | "false" if on => self.toggle_background_constraint(),
            "on" | "1" | "true" | "off" | "0" | "false" => self.status = "Ready".into(),
            "?" => self.status = if on { "1" } else { "0" }.into(),
            _ => self.status = "Use tool.set const.bg on, off or ?".into(),
        }
    }
    /// `tool.attr const.bg <attr> <value|?>`: geometry (off, screen,
    /// vector, point or 0–3), bkgOffset, negative, dblSided, handle
    /// (stored; Draw tools' handles aren't constrained yet) and target (an item name or id;
    /// "none" for every background mesh). Values set while the constraint
    /// is off become its options for next time.
    pub(super) fn const_bg_attr(&mut self, attr: &str, value: &str) {
        let current = self
            .editor
            .background_constraint
            .unwrap_or(self.editor.background_constraint_preset);
        let query = value == "?";
        let flag = |v: &str| match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "on" => Ok(true),
            "0" | "false" | "off" => Ok(false),
            _ => Err(format!("{attr} is a boolean (true or false)")),
        };
        let mut next = current;
        let result: std::result::Result<Option<String>, String> = match attr {
            "geometry" if query => Ok(Some(current.mode.word().into())),
            "geometry" => value
                .parse::<usize>()
                .ok()
                .and_then(|i| GeometryConstraint::ALL.get(i).copied())
                .or_else(|| GeometryConstraint::from_word(value))
                .map(|m| {
                    next.mode = m;
                    None
                })
                .ok_or_else(|| "geometry is off, screen, vector or point (0–3)".into()),
            "bkgoffset" if query => Ok(Some(workspace::number_text(current.offset, "m"))),
            "bkgoffset" => numeric::parse_quantity(value, numeric::Quantity::Distance).map(|v| {
                next.offset = v;
                None
            }),
            "negative" if query => Ok(Some((current.negative_offset as u8).to_string())),
            "negative" => flag(value).map(|on| {
                next.negative_offset = on;
                None
            }),
            "dblsided" if query => Ok(Some((current.double_sided as u8).to_string())),
            "dblsided" => flag(value).map(|on| {
                next.double_sided = on;
                None
            }),
            "handle" if query => Ok(Some((current.handle_constraint as u8).to_string())),
            // Stored and saved; nothing uses it yet (Draw tools aren't
            // constrained, BG-Q8).
            "handle" => flag(value).map(|on| {
                next.handle_constraint = on;
                None
            }),
            "target" if query => Ok(Some(
                current
                    .target
                    .and_then(|id| self.editor.scene.item(id))
                    .map_or("none".into(), |i| i.name.clone()),
            )),
            "target" if value.is_empty() || value.eq_ignore_ascii_case("none") => {
                next.target = None;
                Ok(None)
            }
            "target" => self
                .editor
                .scene
                .items
                .iter()
                .find(|i| i.is_mesh() && (i.name == value || i.id.to_string() == value))
                .map(|i| {
                    next.target = Some(i.id);
                    None
                })
                .ok_or_else(|| format!("No mesh item named {value}")),
            _ => Err(format!(
                "const.bg has geometry, bkgOffset, negative, dblSided, handle and target, not {attr}"
            )),
        };
        match result {
            Err(e) => self.status = e,
            Ok(Some(answer)) => self.status = answer,
            Ok(None) if next == current => self.status = "Ready".into(),
            Ok(None) => {
                if let Err(e) = next.validate() {
                    self.status = e;
                } else if self.editor.background_constraint.is_some() {
                    self.run(Command::BackgroundConstraint(Some(next)));
                } else {
                    self.editor.background_constraint_preset = next;
                    self.status = "Ready".into();
                }
            }
        }
    }
}
