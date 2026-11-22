//! Mini Properties (CV-03, MP-01..07, UIC-62/64/66): a tap or hold of
//! Space runs `attr.formPopover form:MiniProps ...`, which opens a small
//! pop-over just above and left of the pointer. It shows the active
//! tool's short view (the fields its form marks `mini`, MP-02) or, when
//! no tool is on or the tool marks none, the selected item's short view
//! (Name and Item Transform). The fields are the panels' own code, so
//! edits, undo and confirmation are exactly theirs.
//!
//! It opens pinned (UIC-64): the pointer leaving it doesn't close it (the
//! UIC-62 roll-off doesn't apply), nor does Esc or a click outside, which
//! go to the viewport and tool as usual (MP-04/06). Only Space again or
//! its pin circle closes it (the standard popover header, N07). It stays
//! where it opened, across tool drops and changes, its contents follow the
//! scene live, and it drags by its header strip (MP-07, UIC-185).
//!
//! Space is a fixed key that runs [`SPACE_LINE`]: the Input Map binds
//! action names, not command lines, so it isn't rebindable yet.
use super::*;

/// MP-01: what Space runs (the registry's Space binding).
pub(crate) const SPACE_LINE: &str =
    "attr.formPopover form:MiniProps proficiency:mini lockProficiency:true pin:true";

/// About the Tool Properties panel's width (MP-03).
const WIDTH: f32 = 300.;
/// Most height the rows take before they scroll; shorter content leaves
/// the pop-over just tall enough for its rows (MP-03).
const MAX_HEIGHT: f32 = 480.;
/// MP-04: the pop-over's bottom-right corner sits this far above and left
/// of the pointer.
const OFFSET: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiniProperties {
    /// The pointer when it opened: the pop-over hangs above-left of it.
    pub at: Pos2,
    /// Moved by dragging the title bar (MP-07).
    pub moved: Vec2,
}

impl Operandi {
    /// `attr.formPopover` (MP-01). Only `form:MiniProps` exists. `open`
    /// opens or closes; left out, it toggles.
    pub(super) fn form_popover(&mut self, args: &[String]) -> std::result::Result<(), String> {
        let mut form = None;
        let mut open = None;
        for a in args {
            let (key, value) = a.split_once(':').ok_or_else(|| {
                format!("attr.formPopover: \"{a}\" needs a name, like form:MiniProps")
            })?;
            let flag = |name: &str| operandi_core::command_args::boolean(name, value);
            match key.to_ascii_lowercase().as_str() {
                "form" => form = Some(value.trim_start_matches('&').to_owned()),
                // Open question (MP-02): the map's proficiency is an
                // integer; "mini" is taken as the readable short view.
                // Every value shows the short view for now.
                "proficiency" => {
                    if !value.eq_ignore_ascii_case("mini") && value.parse::<i64>().is_err() {
                        return Err(format!(
                            "attr.formPopover: proficiency \"{value}\" isn't mini or a number"
                        ));
                    }
                }
                // Mini Properties is always locked to the short view.
                "lockproficiency" => {
                    flag("lockProficiency")?;
                }
                "pin" => {
                    if !flag("pin")? {
                        return Err(
                            "attr.formPopover: unpinned form pop-overs aren't built yet; use pin:true"
                                .into(),
                        );
                    }
                }
                "focusfirstfield" => {
                    if flag("focusFirstField")? {
                        return Err(
                            "attr.formPopover: focusFirstField isn't built yet; leave it false"
                                .into(),
                        );
                    }
                }
                "open" => open = Some(flag("open")?),
                _ => return Err(format!("attr.formPopover: unknown argument \"{key}\"")),
            }
        }
        match form.as_deref() {
            Some(f) if f.eq_ignore_ascii_case("MiniProps") => {}
            Some(f) => {
                return Err(format!(
                    "attr.formPopover: no form \"{f}\"; use form:MiniProps"
                ));
            }
            None => return Err("attr.formPopover: name the form, like form:MiniProps".into()),
        }
        let anchor = self.mini_properties_anchor.take();
        let open = open.unwrap_or(self.mini_properties.is_none());
        self.mini_properties = open.then(|| MiniProperties {
            at: anchor.unwrap_or_else(|| self.viewport_rect.center()),
            moved: Vec2::ZERO,
        });
        Ok(())
    }

    /// MP-02: whether the active tool marks fields for the short view.
    pub(super) fn mini_shows_tool(&self) -> bool {
        use panels::ToolPanel as T;
        match self.tool_panel_kind() {
            T::Utility => self.tool_has_mini::<create_utility::UtilityFields>(),
            T::Sculpt => self.tool_has_mini::<sculpt_ui::SculptFields>(),
            T::Draw => self.tool_has_mini::<draw_tools::DrawFields>(),
            T::Pen => self.tool_has_mini::<pen_interactive_ui::PenFields>(),
            T::SolidSketch => self.tool_has_mini::<solid_sketch_ui::SolidSketchFields>(),
            T::Reduction => self.tool_has_mini::<reduction_panel::ReductionFields>(),
            T::Bridge => self.tool_has_mini::<bridge_panel::BridgeFields>(),
            T::Spike => self.tool_has_mini::<spike_panel::SpikeFields>(),
            T::Ellipsoid => self.tool_has_mini::<ellipsoid_panel::EllipsoidFields>(),
            T::Shape => self.tool_has_mini::<shape_panel::ShapeFields>(),
            T::Cone => self.tool_has_mini::<cone_panel::ConeFields>(),
            T::Surface => self.tool_has_mini::<smooth_shift_panel::SurfaceFields>(),
            T::Primitive => self.tool_has_mini::<primitive_panel::PrimitiveFields>(),
            T::Live => self.tool_has_mini::<live_fields::LiveFields>(),
            T::EdgeInsert => self.tool_has_mini::<edge_panel::EdgeInsertFields>(),
            T::None => false,
        }
    }

    fn mini_tool_rows(&mut self, ui: &mut Ui) {
        use panels::ToolPanel as T;
        match self.tool_panel_kind() {
            T::Utility => self.tool_panel_mini::<create_utility::UtilityFields>(ui),
            T::Sculpt => self.tool_panel_mini::<sculpt_ui::SculptFields>(ui),
            T::Draw => self.tool_panel_mini::<draw_tools::DrawFields>(ui),
            T::Pen => self.tool_panel_mini::<pen_interactive_ui::PenFields>(ui),
            T::SolidSketch => self.tool_panel_mini::<solid_sketch_ui::SolidSketchFields>(ui),
            T::Reduction => self.tool_panel_mini::<reduction_panel::ReductionFields>(ui),
            T::Bridge => self.tool_panel_mini::<bridge_panel::BridgeFields>(ui),
            T::Spike => self.tool_panel_mini::<spike_panel::SpikeFields>(ui),
            T::Ellipsoid => self.tool_panel_mini::<ellipsoid_panel::EllipsoidFields>(ui),
            T::Shape => self.tool_panel_mini::<shape_panel::ShapeFields>(ui),
            T::Cone => self.tool_panel_mini::<cone_panel::ConeFields>(ui),
            T::Surface => self.tool_panel_mini::<smooth_shift_panel::SurfaceFields>(ui),
            T::Primitive => self.tool_panel_mini::<primitive_panel::PrimitiveFields>(ui),
            T::Live => self.tool_panel_mini::<live_fields::LiveFields>(ui),
            T::EdgeInsert => self.tool_panel_mini::<edge_panel::EdgeInsertFields>(ui),
            T::None => {}
        }
    }

    /// Draws the pop-over when open; returns where it was drawn.
    pub(super) fn mini_properties_window(&mut self, ctx: &Context) -> Option<Rect> {
        let MiniProperties { at, moved } = self.mini_properties?;
        let tool = self.mini_shows_tool();
        let shown = egui::Area::new(Id::new("mini-properties"))
            .order(egui::Order::Foreground)
            .constrain_to(ctx.content_rect())
            .pivot(Align2::RIGHT_BOTTOM)
            .fixed_pos(at - Vec2::splat(OFFSET) + moved)
            .show(ctx, |ui| {
                egui::Frame::popup(&ctx.style())
                    .fill(theme::popup_fill())
                    .show(ui, |ui| {
                        ui.set_width(WIDTH);
                        // N07: the standard popover header (UIC-64/181):
                        // no title, the pin circle at the top right. Mini
                        // Properties opens pinned (MP-06), so the circle
                        // closes it and the strip drags it (MP-07).
                        let action = crate::snapping::popover_header(ui, true);
                        ScrollArea::vertical()
                            .id_salt("mini-properties-rows")
                            .max_height(MAX_HEIGHT)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y =
                                    crate::form::layout::panel::ROW_GAP;
                                crate::form::rows::scope(ui, "mini-properties", |ui| {
                                    if tool {
                                        self.mini_tool_rows(ui);
                                    } else if self.editor.scene.active().is_some() {
                                        self.item_mini_properties(ui);
                                    } else {
                                        ui.weak("Select an item or a tool.");
                                    }
                                });
                            });
                        action
                    })
                    .inner
            });
        use crate::snapping::HeaderAction;
        match shown.inner {
            HeaderAction::Pin => self.mini_properties = None,
            HeaderAction::Drag(delta) => {
                if let Some(m) = self.mini_properties.as_mut() {
                    m.moved += delta;
                }
            }
            HeaderAction::None => {}
        }
        Some(shown.response.rect)
    }
}

impl Operandi {
    /// `select.nextMode` / Select Next Mode (Alt+Shift+Space, UIC-66):
    /// Vertices → Edges → Polygons → Vertices; Items goes to Vertices.
    pub(super) fn select_next_mode(&mut self) {
        self.run(Command::Mode(match self.editor.scene.selection.mode {
            Mode::Vertex => Mode::Edge,
            Mode::Edge => Mode::Polygon,
            Mode::Polygon | Mode::Item => Mode::Vertex,
        }));
    }
}
