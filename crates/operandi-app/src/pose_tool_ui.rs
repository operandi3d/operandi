//! The Pose tool (BEHAVIOR-RIG-PHASE6 §6, IK-50..54): circles on every
//! item of the selected hierarchy; dragging one runs `operandi.pose.drag`
//! each frame (UIC-160) and the drag folds into one undo step (UIC-161).
//! Mouse map (IK-51): drag = IK in the view plane, Ctrl+drag = on the work
//! plane, Shift+drag = twist, middle-drag = FK, drag the root or empty
//! space = move the hierarchy, right-click = pin / unpin, right-click
//! empty space = release all pins. Its options sit in a small floating
//! window (UIC-183), hidden with the animation switch.
use super::*;
use crate::command_handlers::Commands;
use operandi_core::command_registry::{self as table, Area};

/// Screen distance (points) within which a press picks an item.
const PICK: f32 = 9.;

pub(crate) struct PoseTool {
    pub(crate) on: bool,
    pub(crate) pins: Vec<u64>,
    /// IK-53 options.
    pub(crate) root_weight: f64,
    pub(crate) pin_weight: f64,
    pub(crate) pin_precision: f64,
}

impl Default for PoseTool {
    fn default() -> Self {
        PoseTool {
            on: false,
            pins: Vec::new(),
            root_weight: 0.,
            pin_weight: 1.,
            pin_precision: 1.,
        }
    }
}

/// A pose drag in progress.
#[derive(Clone, Debug)]
pub(crate) struct PoseDrag {
    pub(crate) item: u64,
    pub(crate) mark: u64,
    /// ik, plane, rigid, twist or fk.
    pub(crate) mode: &'static str,
    pub(crate) anchor: DVec3,
    pub(crate) normal: DVec3,
}

pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.pose.tool",
            Area::Items,
            "Turns the Pose tool on or off (IK-50): drag the selected hierarchy's items, right-click to pin.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !crate::anim_switch::enabled() {
                a.status = "Animation is off (Preferences > Experimental > Animation)".into();
                return;
            }
            a.pose.on = !a.pose.on;
            a.status = if a.pose.on {
                "Pose: drag items of the selected hierarchy; right-click pins".into()
            } else {
                "Pose tool off".into()
            };
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

fn xyz(p: DVec3) -> String {
    format!("{},{},{}", p.x, p.y, p.z)
}

impl Operandi {
    pub(super) fn pose_active(&self) -> bool {
        self.pose.on && crate::anim_switch::enabled()
    }

    /// The hierarchy the tool works on: the selected item's top item and
    /// everything below it.
    fn pose_items(&self) -> Vec<u64> {
        let scene = &self.editor.scene;
        let Some(sel) = scene.selection.item else {
            return Vec::new();
        };
        let root = operandi_core::pose_tool::top(scene, sel);
        let mut out = vec![root];
        let mut k = 0;
        while k < out.len() {
            let j = out[k];
            out.extend(
                scene
                    .items
                    .iter()
                    .filter(|i| i.parent == Some(j))
                    .map(|i| i.id),
            );
            k += 1;
        }
        out
    }

    fn pose_world(&self, id: u64) -> Option<DVec3> {
        let s = self
            .editor
            .evaluated(operandi_core::evaluate::Purpose::Final);
        Some(s.world_matrix(id).ok()?.w_axis.truncate())
    }

    fn pose_pick(&self, pos: Pos2, r: Rect) -> Option<u64> {
        let projector = self.camera.projector(r);
        self.pose_items()
            .into_iter()
            .filter_map(|id| {
                let (p, _) = projector.project(self.pose_world(id)?)?;
                let d = p.distance(pos);
                (d <= PICK).then_some((d, id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    /// A press with the Pose tool: what it starts.
    pub(super) fn pose_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        button: PointerButton,
        modifiers: egui::Modifiers,
    ) -> Option<input::GestureKind> {
        let items = self.pose_items();
        if items.is_empty() {
            return None;
        }
        let hit = self.pose_pick(pos, r);
        if button == PointerButton::Secondary {
            // IK-51: pin / unpin, or release all pins.
            match hit {
                Some(id) => {
                    if let Some(i) = self.pose.pins.iter().position(|p| *p == id) {
                        self.pose.pins.remove(i);
                    } else {
                        self.pose.pins.push(id);
                    }
                }
                None => self.pose.pins.clear(),
            }
            return Some(input::GestureKind::Blocked);
        }
        let root = items[0];
        let (item, mode) = match (hit, button) {
            (Some(id), PointerButton::Middle) => (id, "fk"),
            (Some(id), _) if id == root => (id, "rigid"),
            (Some(id), _) if modifiers.shift => (id, "twist"),
            (Some(id), _) if modifiers.command => (id, "plane"),
            (Some(id), _) => (id, "ik"),
            (None, PointerButton::Primary) => (root, "rigid"),
            (None, _) => return None,
        };
        let anchor = self.pose_world(item)?;
        Some(input::GestureKind::Pose(PoseDrag {
            item,
            mark: self.editor.history_mark(),
            mode,
            anchor,
            normal: self.camera.ray(pos, r).1,
        }))
    }

    /// One frame of a pose drag.
    pub(super) fn pose_drag(&mut self, d: &PoseDrag, start: Pos2, pos: Pos2, r: Rect) {
        if pos.distance(start) < 2. {
            return;
        }
        // Each frame starts from the press: undo this drag's earlier frames.
        while self.editor.steps_since(d.mark) > 0 {
            self.run(Command::Undo);
        }
        let on_plane = |app: &Operandi, p: Pos2| -> Option<DVec3> {
            let (o, dir) = app.camera.ray(p, r);
            let denom = dir.dot(d.normal);
            (denom.abs() > 1e-9).then(|| o + dir * ((d.anchor - o).dot(d.normal) / denom))
        };
        let pins = self
            .pose
            .pins
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let opts = format!(
            " rootWeight:{} pinWeight:{} pinPrecision:{}",
            self.pose.root_weight, self.pose.pin_weight, self.pose.pin_precision
        );
        let line = match d.mode {
            "twist" => format!(
                "operandi.pose.drag joint:{} mode:twist angle:{}",
                d.item,
                (pos.x - start.x) * 0.5
            ),
            "rigid" => {
                let (Some(a), Some(b)) = (on_plane(self, start), on_plane(self, pos)) else {
                    return;
                };
                format!(
                    "operandi.pose.drag joint:{} mode:rigid by:{}",
                    d.item,
                    xyz(b - a)
                )
            }
            mode => {
                let to = if mode == "plane" {
                    let plane = self.actual_plane();
                    let center = self.workplane_center();
                    self.camera.plane_point(pos, r, plane, center)
                } else {
                    on_plane(self, pos)
                };
                let Some(to) = to else {
                    return;
                };
                let m = if mode == "fk" { "fk" } else { "ik" };
                format!(
                    "operandi.pose.drag joint:{} mode:{m} to:{} pins:{pins}{opts}",
                    d.item,
                    xyz(to)
                )
            }
        };
        if let Some(Err(e)) = operandi_core::command_line::run_line(&mut self.editor, &line) {
            self.status = e;
        }
    }

    /// The drag's frames become one step (IK-54).
    pub(super) fn pose_release(&mut self, d: &PoseDrag) {
        operandi_core::command_exec::fold_steps(&mut self.editor, d.mark, "Pose");
    }

    /// IK-50: circles on the hierarchy's items; hovered yellow, pinned red.
    pub(super) fn draw_pose_handles(&self, painter: &egui::Painter, r: Rect) {
        if !self.pose_active() {
            return;
        }
        let projector = self.camera.projector(r);
        let hover = painter
            .ctx()
            .input(|i| i.pointer.hover_pos())
            .and_then(|p| self.pose_pick(p, r));
        for id in self.pose_items() {
            let Some((p, _)) = self.pose_world(id).and_then(|w| projector.project(w)) else {
                continue;
            };
            let color = if self.pose.pins.contains(&id) {
                theme::viewport::grid_axes()[0]
            } else if hover == Some(id) {
                theme::viewport::selection()
            } else {
                Color32::WHITE
            };
            painter.circle_stroke(p, 6., Stroke::new(1.5f32, color));
        }
    }

    /// The tool's options (IK-53), while it is on.
    pub(super) fn pose_window(&mut self, ctx: &Context) {
        if !self.pose_active() {
            return;
        }
        let mut open = true;
        egui::Window::new("Pose")
            .open(&mut open)
            .resizable(false)
            .default_width(200.)
            .show(ctx, |ui| {
                let t = &mut self.pose;
                egui::Grid::new("pose-grid").num_columns(2).show(ui, |ui| {
                    for (label, v) in [
                        ("Root Weight", &mut t.root_weight),
                        ("Position Pins Weight", &mut t.pin_weight),
                        ("Position Pins Precision", &mut t.pin_precision),
                    ] {
                        ui.label(label);
                        ui.add(
                            crate::numeric_tab::DragValue::new(v)
                                .speed(0.01)
                                .range(0.0..=10.0),
                        );
                        ui.end_row();
                    }
                });
                ui.label(format!("Pins: {}", t.pins.len()));
                if ui.button("Release Pins").clicked() {
                    t.pins.clear();
                }
            });
        if !open {
            self.pose.on = false;
        }
    }
}
