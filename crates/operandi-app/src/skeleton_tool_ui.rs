//! The Skeleton tool (BEHAVIOR-RIG-PHASE5 §2, SK-10..19): clicks in a 3D
//! view draw joints. Every click runs one `operandi.skel.*` line (UIC-160),
//! so undo, scripts and the bridge agree; a press that drags folds its
//! moves into the click's one step (SK-19, UIC-161). Its options sit in a
//! small floating window (UIC-183), hidden with the animation switch.
use super::*;
use crate::command_handlers::Commands;
use operandi_core::command_registry::{self as table, Area};
use operandi_core::skeleton;

/// Screen distance (points) within which a press picks a joint.
const PICK: f32 = 8.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SkelMode {
    #[default]
    Add,
    Edit,
    Insert,
    Delete,
}

pub(crate) struct SkeletonTool {
    pub(crate) on: bool,
    pub(crate) mode: SkelMode,
    /// SK-11 Intersection (on by default).
    pub(crate) intersection: bool,
    /// SK-13 Compensation (on by default).
    pub(crate) compensate: bool,
    /// SK-14: "off", "X", "Y" or "Z".
    pub(crate) symmetry: &'static str,
    pub(crate) prefix: String,
    pub(crate) name: String,
    pub(crate) suffix_plus: String,
    pub(crate) suffix_minus: String,
    /// SK-15 Read Prefix/Suffix.
    pub(crate) read_names: bool,
    /// SK-16.
    pub(crate) direction: &'static str,
    pub(crate) axis: &'static str,
    pub(crate) preserve: bool,
    /// The joint the next click grows from (SK-10).
    pub(crate) current: Option<u64>,
    /// The open Bind dialog (bind_dialog_ui.rs).
    pub(crate) bind: Option<form::Dialog>,
    /// The item selected last frame, for SK-40.
    pub(crate) seen: Option<u64>,
}

impl Default for SkeletonTool {
    fn default() -> Self {
        SkeletonTool {
            on: false,
            mode: SkelMode::Add,
            intersection: true,
            compensate: true,
            symmetry: "off",
            prefix: String::new(),
            name: "Joint".into(),
            suffix_plus: "_L".into(),
            suffix_minus: "_R".into(),
            read_names: false,
            direction: "Z",
            axis: "X",
            preserve: false,
            current: None,
            bind: None,
            seen: None,
        }
    }
}

/// A press in progress: the joint being placed or moved, and the history
/// mark its moves fold back to.
#[derive(Clone, Debug)]
pub(crate) struct SkelDrag {
    pub(crate) joint: u64,
    pub(crate) mark: u64,
    pub(crate) label: &'static str,
    /// The drag moves the joint in the plane through `anchor` facing the
    /// press's ray `normal`.
    pub(crate) anchor: DVec3,
    pub(crate) normal: DVec3,
}

pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.skel.tool",
            Area::Items,
            "Turns the Skeleton tool on or off (SK-10): clicks in a 3D view add, move, insert or delete joints.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !crate::anim_switch::enabled() {
                a.status = "Animation is off (Preferences > Experimental > Animation)".into();
                return;
            }
            a.skeleton.on = !a.skeleton.on;
            a.skeleton.current = None;
            a.status = if a.skeleton.on {
                "Skeleton: click to add joints, Shift+click starts a new chain".into()
            } else {
                "Skeleton tool off".into()
            };
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

/// The naming and alignment arguments every add/insert line carries.
fn options_args(t: &SkeletonTool) -> String {
    let mut s = format!(
        " direction:{} axis:{} preserveAlignment:{}",
        t.direction, t.axis, t.preserve
    );
    for (key, value, default) in [
        ("prefix", &t.prefix, ""),
        ("name", &t.name, "Joint"),
        ("suffixPlus", &t.suffix_plus, "_L"),
        ("suffixMinus", &t.suffix_minus, "_R"),
    ] {
        if value != default {
            s.push_str(&format!(" {key}:{}", quote(value)));
        }
    }
    s
}

fn quote(v: &str) -> String {
    if v.is_empty() || v.contains(char::is_whitespace) || v.contains('"') {
        format!("\"{}\"", v.replace('"', "\\\""))
    } else {
        v.to_owned()
    }
}

fn xyz(p: DVec3) -> String {
    format!("{},{},{}", p.x, p.y, p.z)
}

impl Operandi {
    pub(super) fn skeleton_active(&self) -> bool {
        self.skeleton.on && crate::anim_switch::enabled()
    }

    /// The joint whose screen position is within `PICK` of `pos`, nearest
    /// first.
    fn pick_joint(&self, pos: Pos2, r: Rect) -> Option<u64> {
        let scene = &self.editor.scene;
        let projector = self.camera.projector(r);
        scene
            .items
            .iter()
            .filter(|i| i.is_joint() && scene.shows(i))
            .filter_map(|i| {
                let w = scene.world_matrix(i.id).ok()?.w_axis.truncate();
                let (p, _) = projector.project(w)?;
                let d = p.distance(pos);
                (d <= PICK).then_some((d, i.id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    /// SK-11: where a click puts a new joint: through the mesh under the
    /// pointer, else on the work plane.
    fn joint_placement(&self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (o, d) = self.camera.ray(pos, r);
        let scene = self
            .editor
            .evaluated(operandi_core::evaluate::Purpose::Final);
        skeleton::placement(&scene, o, d, self.skeleton.intersection).or_else(|| {
            self.camera
                .plane_point(pos, r, self.actual_plane(), self.workplane_center())
        })
    }

    /// The point under `pos` on the drag's plane.
    fn drag_point(&self, d: &SkelDrag, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (o, dir) = self.camera.ray(pos, r);
        let denom = dir.dot(d.normal);
        if denom.abs() < 1e-9 {
            return None;
        }
        Some(o + dir * ((d.anchor - o).dot(d.normal) / denom))
    }

    fn drag_of(
        &self,
        joint: u64,
        mark: u64,
        label: &'static str,
        pos: Pos2,
        r: Rect,
    ) -> Option<SkelDrag> {
        Some(SkelDrag {
            joint,
            mark,
            label,
            anchor: self.joint_world(joint)?,
            normal: self.camera.ray(pos, r).1,
        })
    }

    fn joint_world(&self, id: u64) -> Option<DVec3> {
        Some(self.editor.scene.world_matrix(id).ok()?.w_axis.truncate())
    }

    /// A primary press with the Skeleton tool: what the press starts.
    pub(super) fn skeleton_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        modifiers: egui::Modifiers,
    ) -> Option<input::GestureKind> {
        let hit = self.pick_joint(pos, r);
        let mark = self.editor.history_mark();
        match self.skeleton.mode {
            SkelMode::Delete => {
                let j = hit?;
                self.execute(&format!("operandi.skel.delete joint:{j}"));
                if self.skeleton.current == Some(j) {
                    self.skeleton.current = None;
                }
                Some(input::GestureKind::Blocked)
            }
            SkelMode::Insert => {
                let j = hit?;
                self.execute(&format!(
                    "operandi.skel.insert joint:{j}{}",
                    options_args(&self.skeleton)
                ));
                self.skeleton.current = self.editor.scene.selection.item;
                Some(input::GestureKind::Blocked)
            }
            SkelMode::Edit => {
                let j = hit?;
                self.editor.scene.select_item(Some(j));
                self.skeleton.current = Some(j);
                Some(input::GestureKind::Skeleton(self.drag_of(
                    j,
                    mark,
                    "Move Joint",
                    pos,
                    r,
                )?))
            }
            SkelMode::Add => {
                if let Some(j) = hit {
                    // SK-10: clicking a joint selects it; the next click
                    // grows a branch from it. SK-15 Read Prefix/Suffix.
                    self.editor.scene.select_item(Some(j));
                    self.skeleton.current = Some(j);
                    if self.skeleton.read_names {
                        self.read_joint_names(j);
                    }
                    return Some(input::GestureKind::Skeleton(self.drag_of(
                        j,
                        mark,
                        "Move Joint",
                        pos,
                        r,
                    )?));
                }
                let at = self.joint_placement(pos, r)?;
                let parent = if modifiers.shift {
                    None
                } else {
                    self.skeleton
                        .current
                        .filter(|c| self.editor.scene.item(*c).is_some_and(|i| i.is_joint()))
                };
                let mut line = format!("operandi.skel.add position:{}", xyz(at));
                if let Some(p) = parent {
                    line.push_str(&format!(" parent:{p}"));
                }
                if self.skeleton.symmetry != "off" {
                    line.push_str(&format!(" symmetry:{}", self.skeleton.symmetry));
                }
                line.push_str(&options_args(&self.skeleton));
                self.execute(&line);
                if self.editor.history_mark() == mark {
                    return Some(input::GestureKind::Blocked);
                }
                let joint = self.editor.scene.selection.item?;
                self.skeleton.current = Some(joint);
                Some(input::GestureKind::Skeleton(self.drag_of(
                    joint,
                    mark,
                    "Add Joint",
                    pos,
                    r,
                )?))
            }
        }
    }

    /// Dragging places the joint (SK-10) or moves it (SK-12 Edit), in the
    /// plane facing the view at its depth.
    pub(super) fn skeleton_drag(&mut self, d: &SkelDrag, start: Pos2, pos: Pos2, r: Rect) {
        if pos.distance(start) < 2. || self.editor.scene.item(d.joint).is_none() {
            return;
        }
        let Some(to) = self.drag_point(d, pos, r) else {
            return;
        };
        let line = format!(
            "operandi.skel.move joint:{} position:{} compensate:{} direction:{} axis:{} preserveAlignment:{}",
            d.joint,
            xyz(to),
            self.skeleton.compensate,
            self.skeleton.direction,
            self.skeleton.axis,
            self.skeleton.preserve
        );
        if let Some(Err(e)) = operandi_core::command_line::run_line(&mut self.editor, &line) {
            self.status = e;
        }
    }

    /// The press's steps become one (SK-19).
    pub(super) fn skeleton_release(&mut self, d: &SkelDrag) {
        operandi_core::command_exec::fold_steps(&mut self.editor, d.mark, d.label);
    }

    /// SK-15 Read Prefix/Suffix: the clicked joint's prefix and side
    /// suffix go into the fields.
    fn read_joint_names(&mut self, j: u64) {
        let Some(name) = self.editor.scene.item(j).map(|i| i.name.clone()) else {
            return;
        };
        let t = &mut self.skeleton;
        if let Some(at) = name.find(&t.name) {
            t.prefix = name[..at].to_owned();
        }
        for suffix in [t.suffix_plus.clone(), t.suffix_minus.clone()] {
            if !suffix.is_empty() && name.ends_with(&suffix) {
                // The side the next joints take comes from where they are.
                t.symmetry = if t.symmetry == "off" { "X" } else { t.symmetry };
            }
        }
    }

    /// SK-40: selecting a bound joint selects its weight map, so the
    /// Vertex Map Tools paint it and Vertex Map shading shows it.
    pub(super) fn joint_selects_its_map(&mut self) {
        let now = self.editor.scene.selection.item;
        if now == self.skeleton.seen {
            return;
        }
        self.skeleton.seen = now;
        if let Some(j) = now
            && let Some((_, map)) =
                operandi_core::skin_weights::map_for_joint(&self.editor.scene, j)
        {
            self.editor.selected_weight_map = Some(map);
        }
    }

    /// The Skeleton tool's options (SK-12..16), while it is on.
    pub(super) fn skeleton_window(&mut self, ctx: &Context) {
        self.bind_dialog(ctx);
        self.joint_selects_its_map();
        if !self.skeleton_active() {
            return;
        }
        let mut open = true;
        let mut lines: Vec<String> = Vec::new();
        egui::Window::new("Skeleton")
            .open(&mut open)
            .resizable(false)
            .default_width(220.)
            .show(ctx, |ui| {
                let t = &mut self.skeleton;
                ui.horizontal(|ui| {
                    for (m, label) in [
                        (SkelMode::Add, "Add"),
                        (SkelMode::Edit, "Edit"),
                        (SkelMode::Insert, "Insert"),
                        (SkelMode::Delete, "Delete"),
                    ] {
                        ui.selectable_value(&mut t.mode, m, label);
                    }
                });
                ui.checkbox(&mut t.intersection, "Intersection")
                    .on_hover_text("Place joints halfway through the surface under the pointer");
                ui.checkbox(&mut t.compensate, "Compensation")
                    .on_hover_text("Dragging a joint leaves the rest of the chain where it is");
                egui::Grid::new("skel-grid").num_columns(2).show(ui, |ui| {
                    ui.label("Symmetry");
                    egui::ComboBox::from_id_salt("skel-sym")
                        .selected_text(t.symmetry)
                        .show_ui(ui, |ui| {
                            for s in ["off", "X", "Y", "Z"] {
                                ui.selectable_value(&mut t.symmetry, s, s);
                            }
                        });
                    ui.end_row();
                    for (label, field) in [
                        ("Prefix", &mut t.prefix),
                        ("Name", &mut t.name),
                        ("Suffix +", &mut t.suffix_plus),
                        ("Suffix −", &mut t.suffix_minus),
                    ] {
                        ui.label(label);
                        ui.text_edit_singleline(field);
                        ui.end_row();
                    }
                    for (label, field, salt) in [
                        ("Direction", &mut t.direction, "skel-dir"),
                        ("Axis", &mut t.axis, "skel-axis"),
                    ] {
                        ui.label(label);
                        egui::ComboBox::from_id_salt(salt)
                            .selected_text(*field)
                            .show_ui(ui, |ui| {
                                for s in ["X", "Y", "Z"] {
                                    ui.selectable_value(field, s, s);
                                }
                            });
                        ui.end_row();
                    }
                });
                ui.checkbox(&mut t.read_names, "Read Prefix/Suffix");
                ui.checkbox(&mut t.preserve, "Preserve Alignment");
                // SK-12: the current joint's world position and index.
                if let Some(j) = t.current
                    && let Ok(w) = self.editor.scene.world_matrix(j)
                {
                    let mut p = w.w_axis.truncate();
                    let before = p;
                    ui.separator();
                    ui.horizontal(|ui| {
                        for (label, v) in [("X", &mut p.x), ("Y", &mut p.y), ("Z", &mut p.z)] {
                            ui.label(label);
                            ui.add(crate::numeric_tab::DragValue::new(v).speed(0.01));
                        }
                    });
                    let mut depth = 0;
                    let mut up = self.editor.scene.item(j).and_then(|i| i.parent);
                    while let Some(u) =
                        up.filter(|u| self.editor.scene.item(*u).is_some_and(|i| i.is_joint()))
                    {
                        depth += 1;
                        up = self.editor.scene.item(u).and_then(|i| i.parent);
                    }
                    ui.label(format!("Current: {depth}"));
                    if p != before {
                        lines.push(format!(
                            "operandi.skel.move joint:{j} position:{} compensate:{}",
                            xyz(p),
                            t.compensate
                        ));
                    }
                }
                ui.separator();
                if ui
                    .button("Align")
                    .on_hover_text(
                        "Re-aim the selected joints and their chains\noperandi.skel.align",
                    )
                    .clicked()
                {
                    lines.push(format!(
                        "operandi.skel.align direction:{} axis:{}",
                        t.direction, t.axis
                    ));
                }
            });
        for l in lines {
            self.execute(&l);
        }
        if !open {
            self.skeleton.on = false;
        }
    }
}
