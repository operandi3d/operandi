//! Handles for the selected primitive MeshOp (Steve Hill via Jason,
//! 2026-11-25): with a primitive op's row selected in the Mesh Stack and
//! no tool on, the view shows its centre and one size square per axis; a
//! selected Falloff step shows its falloff's points (the primitives read
//! the Falloff step below them since 2026-11-26). Dragging one edits the op's values live,
//! one undo step per drag (SAFE-13, as the op's fields do).
//!
//! The handles follow the tool handles' look (UIC section 7): a move
//! arrow per axis and a plane square per axis pair (as the primitive
//! tools' Move handles; Jason, 2026-11-26), the centre and size squares
//! in the axis colours. The op's falloff draws and drags
//! exactly as the tool's falloff does (GitHub #27): its shape, its points,
//! per-axis handles and Size boxes, from `falloff_view.rs`.
use super::*;
use crate::viewport::falloff_view::FalloffHandle;
use operandi_core::ShapeWeights;
use operandi_core::mesh_stack_editor::OpRef;

/// One handle of the selected primitive op.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OpHandle {
    /// Moves the shape (`cenX`..).
    Centre,
    /// Resizes along one axis (`sizeX`..).
    Size(usize),
    /// Moves the shape along one axis.
    Move(usize),
    /// Moves the shape in the plane facing one axis.
    Plane(usize),
    /// A handle of the op's own falloff, as the tool falloff's handles
    /// (GitHub #27): its points, their per-axis handles and Size boxes.
    Falloff(FalloffHandle),
    /// A Lattice step's control point, by number.
    Lattice(usize),
}

/// A drag on one handle: the op, the handle, the op's values at the press
/// and the undo gesture the drag folds into.
#[derive(Clone, Debug)]
pub(crate) struct OpGrab {
    pub(crate) op: OpRef,
    pub(crate) handle: OpHandle,
    pub(crate) start: serde_json::Value,
    pub(crate) gesture: u64,
}

const AXES: [&str; 3] = ["X", "Y", "Z"];

fn vec3(values: &serde_json::Value, prefix: &str) -> Option<DVec3> {
    let c = |a: &str| values.get(format!("{prefix}{a}"))?.as_f64();
    Some(DVec3::new(c("X")?, c("Y")?, c("Z")?))
}

/// Where a primitive op keeps its centre, and every point that moves with
/// it: `cen` for most primitives, `pos` for Text, and for Arc its `center`
/// with the Start and End points that set its direction (#42: Arc and
/// Text had no handles because only `cen` was looked for).
fn centre_keys(values: &serde_json::Value) -> Option<(&'static str, &'static [&'static str])> {
    const KEYS: [(&str, &[&str]); 3] = [
        ("cen", &["cen"]),
        ("center", &["center", "start", "end"]),
        ("pos", &["pos"]),
    ];
    KEYS.into_iter()
        .find(|(key, _)| vec3(values, key).is_some())
}

/// The op's centre (mesh space).
fn centre_of(values: &serde_json::Value) -> Option<DVec3> {
    vec3(values, centre_keys(values)?.0)
}

/// The changes that move the op's centre, and what goes with it, by `d`.
fn moved_by(
    values: &serde_json::Value,
    d: DVec3,
    changes: &mut serde_json::Map<String, serde_json::Value>,
) {
    let Some((_, moving)) = centre_keys(values) else {
        return;
    };
    for prefix in moving {
        let Some(p) = vec3(values, prefix) else {
            continue;
        };
        for k in 0..3 {
            changes.insert(format!("{prefix}{}", AXES[k]), (p[k] + d[k]).into());
        }
    }
}

/// A Lattice step's values and its control points (mesh space).
fn lattice_of(values: &serde_json::Value) -> Option<operandi_core::meshop_lattice::LatticeOp> {
    values.get("divX")?;
    serde_json::from_value(values.clone()).ok()
}
fn lattice_points(values: &serde_json::Value) -> Option<Vec<DVec3>> {
    lattice_of(values)?.control_points().ok()
}

/// The move arrows' length on screen, as the tool's Move handles.
const ARROW_PIXELS: f64 = 72.;

/// The op's own falloff, if it has one.
fn op_falloff(values: &serde_json::Value) -> Option<ShapeWeights> {
    serde_json::from_value::<Option<ShapeWeights>>(values.get("falloff")?.clone())
        .ok()
        .flatten()
}

impl Operandi {
    /// The selected primitive op or Falloff step and its values, when no
    /// tool is on.
    pub(crate) fn selected_primitive_op(&self) -> Option<(OpRef, serde_json::Value)> {
        if self.editor.live.is_some() || self.editor.has_active_task() {
            return None;
        }
        let row = self.editor.meshop_row?;
        let op_id = row.op?;
        let item = self.editor.scene.item(row.item)?;
        let op = item
            .mesh_stack
            .as_ref()?
            .ops
            .iter()
            .find(|o| o.id == op_id)?;
        let def = op.kind.def()?;
        let falloff_step = def.id() == operandi_core::meshop_tool_pipe::FALLOFF_ID;
        let lattice = def.id() == operandi_core::meshop_lattice::LATTICE_ID;
        if !def.id().starts_with("prim.") && !falloff_step && !lattice {
            return None;
        }
        let values = op.kind.values();
        if !falloff_step && !lattice {
            centre_of(&values)?;
        }
        Some((
            OpRef {
                item: row.item,
                op: op_id,
            },
            values,
        ))
    }

    /// The selected primitive op's handles in world space.
    pub(crate) fn meshop_handles(&self, r: Rect) -> Vec<(DVec3, OpHandle)> {
        let Some((op, values)) = self.selected_primitive_op() else {
            return Vec::new();
        };
        let Ok(world) = self.editor.scene.world_matrix(op.item) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(points) = lattice_points(&values) {
            out.extend(
                points
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| (world.transform_point3(p), OpHandle::Lattice(i))),
            );
        }
        if let Some(centre) = centre_of(&values) {
            self.shape_handles(world, centre, &values, r, &mut out);
        }
        if let Some(w) = op_falloff(&values) {
            // The falloff lives in the item's space, as the shapes do.
            out.extend(
                self.falloff_handles_in(Some(w.first), world, &[], r)
                    .into_iter()
                    .map(|(p, h)| (p, OpHandle::Falloff(h))),
            );
        }
        out
    }

    /// A primitive's centre, move, plane and size handles.
    fn shape_handles(
        &self,
        world: glam::DMat4,
        centre: DVec3,
        values: &serde_json::Value,
        r: Rect,
        out: &mut Vec<(DVec3, OpHandle)>,
    ) {
        let at = world.transform_point3(centre);
        out.push((at, OpHandle::Centre));
        // Move arrows and plane squares, a fixed size on screen.
        if let Some(mpp) = self.camera.metres_per_pixel(at, r) {
            let len = ARROW_PIXELS * mpp * self.handle_scale_at(at) as f64;
            let dir = |a: usize| world.transform_vector3(DVec3::AXES[a]).normalize_or_zero();
            for a in 0..3 {
                out.push((at + dir(a) * len, OpHandle::Move(a)));
            }
            for n in 0..3 {
                let (u, v) = ((n + 1) % 3, (n + 2) % 3);
                out.push((at + (dir(u) + dir(v)) * len * 0.3, OpHandle::Plane(n)));
            }
        }
        if let Some(size) = vec3(values, "size") {
            // The Cube's Size is its full width; the others' are radii.
            let full = values.get("segmentsX").is_some();
            for a in 0..3 {
                let mut p = centre;
                p[a] += if full { size[a] * 0.5 } else { size[a] };
                out.push((world.transform_point3(p), OpHandle::Size(a)));
            }
        }
    }

    /// The handle under `pos`, nearest first.
    pub(crate) fn meshop_handle_hit(&self, pos: Pos2, r: Rect) -> Option<OpGrab> {
        let reach = self.handle_preferences.small_point_hit;
        let (op, start) = self.selected_primitive_op()?;
        let field = op_falloff(&start).map(|w| w.first);
        let handles = self.meshop_handles(r);
        let centre_q = handles
            .first()
            .and_then(|&(p, _)| self.camera.project(p, r))
            .map(|(q, _)| q);
        let centre = handles
            .iter()
            .find(|(_, h)| *h == OpHandle::Falloff(FalloffHandle::Point(0)))
            .and_then(|&(p, _)| self.camera.project(p, r))
            .map(|(q, _)| q);
        let handle = handles
            .into_iter()
            // TP-19: a Size box seen end-on sits on the centre; skip it.
            .filter(|&(p, h)| match h {
                OpHandle::Falloff(f) => {
                    !falloff_view::is_size_box_for(f, field) || !self.box_edge_on(p, centre, r)
                }
                _ => true,
            })
            .filter_map(|(p, h)| {
                let q = self.camera.project(p, r)?.0;
                // An arrow is picked anywhere along its shaft.
                let d = match (h, centre_q) {
                    (OpHandle::Move(_), Some(c)) => segment_distance(pos, c, q),
                    _ => q.distance(pos),
                };
                Some((d, h))
            })
            .filter(|&(d, _)| d <= reach)
            // A square or point on an arrow's shaft wins over the shaft.
            .min_by(|a, b| {
                let shaft = |h: &OpHandle| matches!(h, OpHandle::Move(_));
                shaft(&a.1).cmp(&shaft(&b.1)).then(a.0.total_cmp(&b.0))
            })?
            .1;
        Some(OpGrab {
            op,
            handle,
            start,
            gesture: crate::colour_field::next_gesture(),
        })
    }

    /// A drag from `from` to `to`: the op's values from the press, moved.
    pub(crate) fn drag_meshop_handle(&mut self, grab: &OpGrab, from: Pos2, to: Pos2, r: Rect) {
        let (_, right, up, _) = self.camera.basis();
        let scale = self.camera.distance / (r.height() as f64 * 0.9);
        let d = to - from;
        let world_delta = (right * d.x as f64 - up * d.y as f64) * scale;
        let Ok(world) = self.editor.scene.world_matrix(grab.op.item) else {
            return;
        };
        let delta = world.inverse().transform_vector3(world_delta);
        let v = &grab.start;
        let mut changes = serde_json::Map::new();
        match grab.handle {
            OpHandle::Centre => moved_by(v, delta, &mut changes),
            OpHandle::Move(a) | OpHandle::Plane(a) => {
                // Along the arrow's axis only, or across the plane.
                let mut d = DVec3::ZERO;
                if let OpHandle::Move(_) = grab.handle {
                    d[a] = delta[a];
                } else {
                    d = delta;
                    d[a] = 0.;
                }
                moved_by(v, d, &mut changes);
            }
            OpHandle::Size(a) => {
                let Some(s) = vec3(v, "size") else { return };
                let full = v.get("segmentsX").is_some();
                let grow = if full { 2. * delta[a] } else { delta[a] };
                changes.insert(format!("size{}", AXES[a]), (s[a] + grow).max(0.).into());
            }
            OpHandle::Lattice(i) => {
                let Some(lattice) = lattice_of(v) else { return };
                let Ok(mut moves) = lattice.moves() else {
                    return;
                };
                let Some(m) = moves.get_mut(i) else { return };
                *m += delta;
                let offsets: Vec<[f64; 3]> = moves.iter().map(|m| m.to_array()).collect();
                changes.insert("offsets".into(), serde_json::json!(offsets));
            }
            OpHandle::Falloff(h) => {
                let Some(mut w) = op_falloff(v) else {
                    return;
                };
                // The tool falloff's own rule (falloff_view::moved_falloff).
                let Some(next) = falloff_view::moved_falloff(w.first, h, delta) else {
                    return;
                };
                if next.validate().is_err() {
                    return;
                }
                w.first = next;
                let Ok(json) = serde_json::to_value(Some(w)) else {
                    return;
                };
                changes.insert("falloff".into(), json);
            }
        }
        // One undo step for the whole drag (SAFE-13).
        let previous = self.editor.history.undo_fold.replace(grab.gesture);
        self.run(Command::MeshOpSet {
            op: grab.op,
            values: changes,
        });
        self.editor.history.undo_fold = previous;
    }

    /// Draws the selected primitive op's handles.
    pub(super) fn draw_meshop_handles(&self, painter: &egui::Painter, r: Rect) {
        if !self.handle_preferences.unselected_shown() {
            return;
        }
        let handles = self.meshop_handles(r);
        // The op's falloff first, drawn as the tool's falloff is.
        if let Some((op, values)) = self.selected_primitive_op()
            && let Some(w) = op_falloff(&values)
            && let Ok(world) = self.editor.scene.world_matrix(op.item)
        {
            self.draw_falloff_shape_in(painter, r, w.first, world, &[]);
            self.draw_falloff_size_boxes_in(painter, r, w.first, world);
        }
        // A Lattice step's cage: a line to each next control point.
        if let Some((op, values)) = self.selected_primitive_op()
            && let Some(lattice) = lattice_of(&values)
            && let Ok(points) = lattice.control_points()
            && let Ok([a, b, c]) = lattice.divisions()
            && let Ok(world) = self.editor.scene.world_matrix(op.item)
        {
            let q = |n: usize| {
                self.camera
                    .project(world.transform_point3(points[n]), r)
                    .map(|x| x.0)
            };
            let stroke = Stroke::new(self.handle_preferences.line_width, palette::selection());
            for k in 0..c {
                for j in 0..b {
                    for i in 0..a {
                        let here = lattice.index(i, j, k);
                        let next = [
                            (i + 1 < a).then(|| lattice.index(i + 1, j, k)),
                            (j + 1 < b).then(|| lattice.index(i, j + 1, k)),
                            (k + 1 < c).then(|| lattice.index(i, j, k + 1)),
                        ];
                        for n in next.into_iter().flatten() {
                            if let (Some(p), Some(q)) = (q(here), q(n)) {
                                painter.line_segment([p, q], stroke);
                            }
                        }
                    }
                }
            }
        }
        let Some(&(centre, _)) = handles.first() else {
            return;
        };
        let Some((c, _)) = self.camera.project(centre, r) else {
            return;
        };
        let side = self.handle_preferences.small_point_size();
        for &(p, h) in &handles {
            let Some((q, _)) = self.camera.project(p, r) else {
                continue;
            };
            let size = side * self.handle_zoom_at(p);
            match h {
                OpHandle::Centre => {
                    painter.circle_filled(q, size * 0.6, palette::selection());
                }
                OpHandle::Size(a) => {
                    let color = palette::handles()[a];
                    painter.line_segment(
                        [c, q],
                        Stroke::new(self.handle_preferences.line_width, color),
                    );
                    painter.rect_filled(Rect::from_center_size(q, vec2(size, size)), 0., color);
                }
                OpHandle::Move(a) => {
                    let color = palette::handles()[a];
                    painter.line_segment(
                        [c, q],
                        Stroke::new(self.handle_preferences.line_width, color),
                    );
                    handle_arrow_head(painter, c, q, color);
                }
                OpHandle::Plane(n) => {
                    let color = palette::handles()[n].gamma_multiply(0.7);
                    painter.rect_filled(
                        Rect::from_center_size(q, vec2(size * 1.2, size * 1.2)),
                        0.,
                        color,
                    );
                }
                OpHandle::Lattice(_) => {
                    painter.rect_filled(
                        Rect::from_center_size(q, vec2(size, size)),
                        0.,
                        palette::selection(),
                    );
                }
                // Drawn with the falloff above.
                OpHandle::Falloff(_) => {}
            }
        }
    }
}

/// How far `p` is from the screen segment `a`-`b`.
fn segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = if ab.length_sq() > 0. {
        ((p - a).dot(ab) / ab.length_sq()).clamp(0., 1.)
    } else {
        0.
    };
    p.distance(a + ab * t)
}
