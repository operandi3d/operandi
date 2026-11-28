//! The Snaps and Precision panel (spec `snaps-precision-2026-11-10`, panel
//! part): a small floating palette of preset tools, each switching on a
//! transform with geometry snapping, plus Advanced Handles and the
//! Absolute Scaling window. Opened from the Measure group or Edit ▸
//! Snapping ▸ Snaps and Precision; it doesn't reopen after a restart (§2).
//!
//! A preset is a shortcut to settings the user could make by hand (§1):
//! pressing one turns global snapping on (left on afterwards, Q2 guess),
//! starts its tool with the preset's options, and pressing it again drops
//! the tool. Leaving the preset's tool puts the action centre back.
use super::*;
use crate::handle_preferences::HandleDetail;
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use operandi_core::{TweakConnect, TweakMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SnapPreset {
    DragSnap,
    DragSnapRigid,
    PlanarRotateSnap,
    PlanarRotateSnapAxis,
    ScaleAbsolute,
    SizeAbsolute,
}

impl SnapPreset {
    pub(super) const ALL: [Self; 6] = [
        Self::DragSnap,
        Self::DragSnapRigid,
        Self::PlanarRotateSnap,
        Self::PlanarRotateSnapAxis,
        Self::ScaleAbsolute,
        Self::SizeAbsolute,
    ];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::DragSnap => "Drag Snap",
            Self::DragSnapRigid => "Drag Snap Rigid",
            Self::PlanarRotateSnap => "Planar Rotate Snap",
            Self::PlanarRotateSnapAxis => "Planar Rotate Snap Axis",
            Self::ScaleAbsolute => "Scale Absolute",
            Self::SizeAbsolute => "Size Absolute",
        }
    }
    /// Batch 14 icons (§3: icon and label).
    pub(super) fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Self::DragSnap => Icon::DragSnap,
            Self::DragSnapRigid => Icon::DragSnapRigid,
            Self::PlanarRotateSnap => Icon::PlanarRotateSnap,
            Self::PlanarRotateSnapAxis => Icon::PlanarRotateSnapAxis,
            Self::ScaleAbsolute => Icon::ScaleAbsolute,
            Self::SizeAbsolute => Icon::SizeAbsolute,
        }
    }
    /// The `tool.set` id (§6).
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::DragSnap => "SnapDrag",
            Self::DragSnapRigid => "SnapDragRigid",
            Self::PlanarRotateSnap => "PlanarRotateSnap",
            Self::PlanarRotateSnapAxis => "PlanarRotateSnapAxis",
            Self::ScaleAbsolute => "ScaleAbsolute",
            Self::SizeAbsolute => "SizeAbsolute",
        }
    }
    /// Tooltips (§3).
    pub(super) fn tip(self) -> &'static str {
        match self {
            Self::DragSnap => "Drag any vertex, edge or polygon onto another to snap it there",
            Self::DragSnapRigid => {
                "Drag any vertex, edge or polygon onto another; everything connected moves with it"
            }
            Self::PlanarRotateSnap => {
                "Rotate around one axis, snapping to a start and an end point"
            }
            Self::PlanarRotateSnapAxis => {
                "Rotate around an axis set by snapped points, then snap to a start and an end point"
            }
            Self::ScaleAbsolute => "Scale by snapping to a start and an end point",
            Self::SizeAbsolute => "Scale by snapping to a start point and setting a size",
        }
    }
    /// The tool the preset runs.
    pub(super) fn tool(self) -> Tool {
        match self {
            Self::DragSnap | Self::DragSnapRigid => Tool::Tweak,
            Self::PlanarRotateSnap | Self::PlanarRotateSnapAxis => Tool::Rotate,
            Self::ScaleAbsolute | Self::SizeAbsolute => Tool::Scale,
        }
    }
    pub(super) fn named(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.id().eq_ignore_ascii_case(id) || p.label().eq_ignore_ascii_case(id))
    }
}

impl Operandi {
    /// The Snaps and Precision button and menu item: open or close (§2).
    pub(super) fn toggle_snaps_panel(&mut self) {
        self.snaps_open = !self.snaps_open;
    }

    /// The preset whose tool is running, if any. Forgets a preset whose
    /// tool was dropped or replaced, and puts the action centre back.
    pub(super) fn settle_snap_preset(&mut self) {
        let Some((preset, _)) = self.snap_preset else {
            return;
        };
        let running = self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.tool == preset.tool());
        if !running {
            self.leave_snap_preset();
        }
    }

    fn leave_snap_preset(&mut self) {
        if let Some((_, before)) = self.snap_preset.take() {
            self.preset_center_click = false;
            if self.editor.action_context != before {
                self.run(Command::ActionContext(before));
            }
        }
    }

    /// A preset button (§3, Q2): again on the running preset drops it;
    /// otherwise snapping on and the preset's tool with its settings.
    pub(super) fn activate_snap_preset(&mut self, preset: SnapPreset) {
        if self.snap_preset.is_some_and(|(p, _)| p == preset)
            && self
                .editor
                .live
                .as_ref()
                .is_some_and(|l| l.tool == preset.tool())
        {
            self.run(Command::Drop);
            self.leave_snap_preset();
            self.status = format!("{} dropped", preset.label());
            return;
        }
        // A different preset: the first one's tool goes, its centre returns.
        if self.snap_preset.is_some() {
            if self.editor.live.is_some() {
                self.run(Command::Drop);
            }
            self.leave_snap_preset();
        }
        let before = self.editor.action_context;
        if self.editor.live.as_ref().map(|l| l.tool) != Some(preset.tool()) {
            self.activate(preset.tool());
        }
        if self.editor.live.as_ref().map(|l| l.tool) != Some(preset.tool()) {
            // Activation refused; its status line says why.
            return;
        }
        if !self.snapping.enabled {
            self.snapping.enabled = true;
            self.save_snapping();
        }
        self.snap_preset = Some((preset, before));
        match preset {
            SnapPreset::DragSnap | SnapPreset::DragSnapRigid => {
                // §4.1/4.2: Element Move, Auto (Q3), Range 0, Use
                // Connectivity or Rigid Connections.
                let mut p = self.editor.live.as_ref().unwrap().params.clone();
                p.tweak_options.mode = TweakMode::Auto;
                p.tweak_options.range = 0.;
                p.tweak_options.connect = if preset == SnapPreset::DragSnap {
                    TweakConnect::Connectivity
                } else {
                    TweakConnect::Rigid
                };
                self.run(Command::Preview(p));
                self.status = format!(
                    "{}: drag an element onto another; snapping is on",
                    preset.label()
                );
            }
            _ => self.begin_precision_preset(preset),
        }
    }

    /// The four handle presets: Scale/Size get their absolute handles;
    /// every one waits for the click that places the Element centre (§4,
    /// Q6: "clicking an element sets the center there and aligns the
    /// handles to that element's normal").
    fn begin_precision_preset(&mut self, preset: SnapPreset) {
        self.planar = Default::default();
        let handles = match preset {
            SnapPreset::ScaleAbsolute => Some(operandi_core::AbsoluteHandles::scale_absolute()),
            SnapPreset::SizeAbsolute => Some(operandi_core::AbsoluteHandles::size_absolute()),
            _ => None,
        };
        if let Some(handles) = handles
            && let Some(live) = self.editor.live.as_ref()
        {
            let mut p = live.params.clone();
            p.absolute_handles = Some(handles);
            self.run(Command::Preview(p));
        }
        self.preset_center_click = true;
        self.status = format!(
            "{}: click a vertex, edge or polygon to centre on it; snapping is on",
            preset.label()
        );
    }

    /// The preset's centre click: the element under the pointer (Auto
    /// Center, Q3), its centre and its normal as the axes' Z.
    pub(super) fn place_preset_center(&mut self, pos: Pos2, r: Rect) {
        // The click stays armed until it lands on an element.
        let Some(pick) = self.tweak_pick_in(pos, r, TweakMode::AutoCenter) else {
            self.status = "Nothing under the pointer to centre on".into();
            return;
        };
        self.preset_center_click = false;
        let scene = &self.editor.scene;
        let Some(item) = scene.item(pick.grab.item) else {
            return;
        };
        let matrix = scene.world_matrix(item.id).unwrap_or_default();
        let points = pick.grab.element.points(&item.mesh).unwrap_or_default();
        let at = points
            .iter()
            .map(|&v| matrix.transform_point3(item.mesh.vertices[v as usize]))
            .sum::<DVec3>()
            / points.len().max(1) as f64;
        let at = if at.is_finite() { at } else { pick.at };
        let normal = matrix
            .transform_vector3(element_normal(&item.mesh, pick.grab.element))
            .try_normalize()
            .unwrap_or(DVec3::Y);
        let axis = operandi_core::PickedFrame::from_z(normal)
            .map_or(AxisOrientation::World, AxisOrientation::Element);
        let context = ActionContext {
            center: CenterMode::Element(operandi_core::PickedPoint::new(at)),
            axis,
        };
        if self.run_ok(Command::ActionContext(context)) {
            self.planar = Default::default();
            self.status = match self.snap_preset.map(|(p, _)| p) {
                Some(SnapPreset::ScaleAbsolute | SnapPreset::SizeAbsolute) => {
                    "Drag a base handle (circle) onto one point, then its scale handle (square)                      onto another"
                }
                Some(SnapPreset::PlanarRotateSnapAxis) => {
                    "Drag the two axis handles onto points to set the axis, then the outer handle                      to the rest direction and the inner one to rotate"
                }
                _ => "Drag the outer handle to the rest direction, then the inner one to rotate",
            }
            .into();
        }
    }

    /// Advanced Handles (§5, Q5 guess): a one-way set of the selected
    /// tool's handle style; greyed once it is Advanced.
    pub(super) fn set_handle_style(&mut self, style: Option<HandleDetail>) {
        let style = style.unwrap_or_default();
        if self.handle_preferences.selected == style {
            return;
        }
        self.handle_preferences.selected = style;
        self.status = format!("Tool handles: {}", style.label());
        if let Err(error) = self
            .handle_preferences
            .save(&self.environment.paths().handle_preferences())
        {
            self.status = format!("Tool handles changed; could not save it: {error}");
        }
    }

    /// `tool.handleStyle invisible|basic|advanced|default` (§6).
    pub(super) fn command_handle_style(&mut self, args: &[String]) {
        let names = ["invisible", "basic", "advanced", "default"];
        let pick = match args {
            [one] => operandi_core::command_args::choice(
                "style",
                one.split_once(':').map_or(one.as_str(), |(_, v)| v),
                &names,
            ),
            _ => Err("Use tool.handleStyle invisible|basic|advanced|default".into()),
        };
        match pick {
            Ok(i) => self.set_handle_style(HandleDetail::ALL.get(i).copied()),
            Err(error) => self.status = error,
        }
    }

    /// `tool.set <preset id>` for the six presets (§6); `false` when `id`
    /// isn't one.
    pub(super) fn tool_set_snap_preset(&mut self, id: &str, on: bool) -> bool {
        let Some(preset) = SnapPreset::named(id) else {
            return false;
        };
        let running = self.snap_preset.is_some_and(|(p, _)| p == preset);
        if on != running {
            self.activate_snap_preset(preset);
        }
        true
    }

    pub(super) fn snaps_window(&mut self, ctx: &egui::Context) {
        self.settle_snap_preset();
        if !self.snaps_open {
            return;
        }
        let mut pressed = None;
        let mut advanced = false;
        let mut absolute = false;
        let active = self.snap_preset.map(|(p, _)| p);
        let is_advanced = self.handle_preferences.selected == HandleDetail::Advanced;
        let absolute_open = self.absolute_open;
        // Its own OS window, so it can float outside the main one (Jason,
        // 2026-11-10); about the reference's 150 × 320 (§2).
        let (open, _) = self.palette_window(ctx, "snaps", "Snaps", [200., 330.], None, |_, ui| {
            use crate::form::rows;
            rows::scope(ui, "snaps-precision", |ui| {
                rows::section(ui, "Presets", false, |ui| {
                    for preset in SnapPreset::ALL {
                        if toggle_row(ui, preset.icon(), preset.label(), active == Some(preset))
                            .on_hover_text(preset.tip())
                            .clicked()
                        {
                            pressed = Some(preset);
                        }
                    }
                    let why = is_advanced.then_some(
                        "The tool handles are Advanced already; Preferences > Tool Handles \
                         changes them back",
                    );
                    let response = ui
                        .gated_ui(why, |ui| {
                            toggle_row(
                                ui,
                                crate::icons::Icon::AdvancedHandles,
                                "Advanced Handles",
                                false,
                            )
                        })
                        .inner;
                    let response = if why.is_none() {
                        response.on_hover_text("Show tools' extra handles (Tool Handles: Advanced)")
                    } else {
                        response
                    };
                    advanced = response.clicked();
                });
                rows::section(ui, "Commands", false, |ui| {
                    absolute = toggle_row(
                        ui,
                        crate::icons::Icon::AbsoluteScaling,
                        "Absolute Scaling",
                        absolute_open,
                    )
                    .on_hover_text("Open or close the Absolute Scaling window")
                    .clicked();
                });
            });
        });
        if !open {
            self.snaps_open = false;
        }
        if let Some(preset) = pressed {
            self.activate_snap_preset(preset);
        }
        if advanced {
            self.set_handle_style(Some(HandleDetail::Advanced));
        }
        if absolute {
            self.toggle_absolute_scaling();
        }
    }
}

/// An element's normal in its mesh: a polygon's own, an edge's or a
/// point's the average of the polygons around it (Element action center).
pub(super) fn element_normal(
    mesh: &operandi_core::Mesh,
    element: operandi_core::TweakElement,
) -> DVec3 {
    use operandi_core::TweakElement as E;
    let faces = mesh.point_faces();
    let around = |f: &dyn Fn(usize) -> bool, points: &[u32]| {
        let mut sum = DVec3::ZERO;
        let mut seen = Vec::new();
        for &v in points {
            for &face in faces.of(v) {
                let face = face as usize;
                if !seen.contains(&face) && f(face) {
                    seen.push(face);
                    sum += mesh.normal(face);
                }
            }
        }
        sum
    };
    let n = match element {
        E::Polygon(f) => mesh.normal(f as usize),
        E::Edge(a, b) => around(
            &|face| {
                let v = &mesh.faces[face].vertices;
                v.contains(&a) && v.contains(&b)
            },
            &[a],
        ),
        E::Vertex(v) => around(&|_| true, &[v]),
    };
    n.try_normalize().unwrap_or(DVec3::Y)
}

/// A full-width button row, icon then label (§3), lit in the accent
/// while `on` (UIC-22, UIC-23, UIC-116).
fn toggle_row(ui: &mut Ui, icon: crate::icons::Icon, label: &str, on: bool) -> egui::Response {
    let height = 30.;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
    let (fill, text) = if on {
        (theme::orange(), theme::on_orange())
    } else if response.hovered() && ui.is_enabled() {
        (theme::hover(), theme::text())
    } else {
        (
            theme::control(),
            if ui.is_enabled() {
                theme::text()
            } else {
                theme::disabled_text()
            },
        )
    };
    ui.painter().rect_filled(rect, 3., fill);
    let square =
        egui::Rect::from_min_size(rect.left_top() + egui::vec2(4., 3.), egui::vec2(24., 24.));
    if ui.is_enabled() {
        crate::icons::paint(ui, square, icon);
    } else {
        crate::icons::paint_faded(ui, square, icon);
    }
    ui.painter().text(
        egui::pos2(square.right() + 6., rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.),
        text,
    );
    response
}
