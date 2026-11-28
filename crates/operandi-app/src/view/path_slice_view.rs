//! Viewport side of the Edit tab's wave 2 and 3 tools (Pen Slice, Curve
//! Slice, Primitive Slice; Polygon Axis Slice): presses, drags and their
//! overlays. Geometry and tool state live in `operandi_core::path_slice`
//! and `axis_slice`; every change goes through `Command::Preview`, so a
//! press-to-release is one tweak and Drop records one step (PS-16, AX-14).
use super::*;
use operandi_core::{PrimitiveShape, ShapePlace};
use slice_view::GRAB;

/// Screen points the pointer must move off a press before it counts as a
/// drag (direction locks, SNH-Q01).
const CLICK_MOVE: f32 = 3.;

/// A wave 2 or 3 drag in progress.
#[derive(Clone)]
pub enum PathDrag {
    /// Pen or Curve Slice: point `index` follows the pointer on the plane;
    /// `start` is the press (a release where it began is a click).
    Point { index: usize, start: Pos2 },
    /// Primitive Slice: sizing the shape from `anchor` (the press on the
    /// face), from its centre or from a corner; `size` is the half size at
    /// the press (Keep Aspect).
    Shape {
        anchor: DVec3,
        start: Pos2,
        from_center: bool,
        size: (f64, f64),
    },
    /// Polygon Axis Slice: Offset follows the drag (AX-13): sideways moves
    /// world axis `right.0`, up/down moves `up.0` (each with its sign).
    Offset {
        start: Pos2,
        from: DVec3,
        right: (usize, f64),
        up: (usize, f64),
        metres_per_pixel: f64,
    },
}

/// The work plane index (Operandi's 0 = XZ, 1 = YZ, 2 = XY) of a normal.
fn plane_of(normal: DVec3) -> u8 {
    if normal == DVec3::X {
        1
    } else if normal == DVec3::Z {
        2
    } else {
        0
    }
}

/// The world axis closest to `v`, with its sign, skipping `not`.
fn nearest_axis(v: DVec3, not: Option<usize>) -> (usize, f64) {
    (0..3)
        .filter(|&a| Some(a) != not)
        .map(|a| (a, v[a]))
        .max_by(|x, y| x.1.abs().total_cmp(&y.1.abs()))
        .map_or((0, 1.), |(a, c)| (a, c.signum()))
}

impl Operandi {
    /// The drawing plane of Pen and Curve Slice: fixed by the first point
    /// (its normal is kept in the tool), else the plane the view draws on.
    fn path_slice_plane(&self, p: &Parameters, tool: Tool) -> (u8, DVec3) {
        let (points, normal) = match tool {
            Tool::PenSlice => (&p.slice.pen_slice.points, p.slice.pen_slice.plane_normal),
            _ => (
                &p.slice.curve_slice.points,
                p.slice.curve_slice.plane_normal,
            ),
        };
        if points.is_empty() {
            let plane = self.actual_plane();
            let normal = match plane {
                1 => DVec3::X,
                2 => DVec3::Z,
                _ => DVec3::Y,
            };
            (plane, normal)
        } else {
            (plane_of(normal), normal)
        }
    }

    /// Pen Slice's snaps, the Pen's (SN2-34..37): `at` snapped to the
    /// active targets, the path's other points among them, then its
    /// segment from the point before locked to a direction. `dragged` is
    /// the point being moved (None: a new one after the last).
    fn pen_slice_snap(
        &mut self,
        points: &[DVec3],
        dragged: Option<usize>,
        at: DVec3,
        normal: DVec3,
        lock: bool,
        r: Rect,
    ) -> DVec3 {
        // The path's points aren't mesh points, so nothing travelling with
        // the drag is a mesh target (a point dragged onto a vertex must
        // stay snapped to it on the next frame); the dragged point itself
        // is left out of `own`.
        let own: Vec<DVec3> = points
            .iter()
            .enumerate()
            .filter(|&(i, _)| Some(i) != dragged)
            .map(|(_, &p)| p)
            .collect();
        let from_index = match dragged {
            Some(i) => i.checked_sub(1),
            None => points.len().checked_sub(1),
        };
        let path = crate::snap_kinds::Path {
            from: from_index.and_then(|i| points.get(i)).copied(),
            before: from_index
                .and_then(|i| i.checked_sub(1))
                .and_then(|i| points.get(i))
                .copied(),
            normal,
        };
        self.snap_path_point(at, None, &own, &path, lock, r)
    }

    pub(super) fn path_slice_press(
        &mut self,
        tool: Tool,
        mut p: Parameters,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) -> Option<PathDrag> {
        match tool {
            Tool::PenSlice | Tool::CurveSlice => {
                let points = if tool == Tool::PenSlice {
                    &p.slice.pen_slice.points
                } else {
                    &p.slice.curve_slice.points
                };
                let hit = points
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &w)| Some((i, self.camera.project(w, r)?.0.distance(pos))))
                    .filter(|(_, d)| *d <= GRAB)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((index, _)) = hit {
                    return Some(PathDrag::Point { index, start: pos });
                }
                let (plane, normal) = self.path_slice_plane(&p, tool);
                let Some(at) = self.work_point(pos, r, plane, self.workplane_center()) else {
                    self.status =
                        format!("{}: the view is edge-on to the work plane", tool.label());
                    return None;
                };
                let index = if tool == Tool::PenSlice {
                    // DSN-07: a placed point snaps as a dragged one does.
                    self.snap_session = Default::default();
                    let at =
                        self.pen_slice_snap(&p.slice.pen_slice.points, None, at, normal, false, r);
                    let o = &mut p.slice.pen_slice;
                    o.plane_normal = normal;
                    o.points.push(at);
                    o.points.len() - 1
                } else {
                    let o = &mut p.slice.curve_slice;
                    o.plane_normal = normal;
                    o.points.push(at);
                    o.points.len() - 1
                };
                self.slice_publish(p);
                Some(PathDrag::Point { index, start: pos })
            }
            Tool::PrimitiveSlice => {
                let Some((at, normal)) = self.path_slice_face_hit(pos, r) else {
                    self.status = "Primitive Slice: click on a face of the mesh".into();
                    return None;
                };
                let o = &mut p.slice.primitive_slice;
                // A click moves the shape there; a drag sizes it (PS-41).
                o.place = Some(ShapePlace::on(at, normal));
                o.held = o.lazy_apply;
                let size = o.half_size();
                let from_center = mods.ctrl || o.edit_from_center;
                self.slice_publish(p);
                Some(PathDrag::Shape {
                    anchor: at,
                    start: pos,
                    from_center,
                    size,
                })
            }
            Tool::AxisSlice => {
                let (_, right, up, _) = self.camera.basis();
                let right = nearest_axis(right, None);
                let up = nearest_axis(up, Some(right.0));
                let centre = self
                    .axis_slice_box()
                    .map_or(DVec3::ZERO, |(lo, hi)| (lo + hi) / 2.);
                let metres_per_pixel = self.camera.metres_per_pixel(centre, r).unwrap_or(0.01);
                Some(PathDrag::Offset {
                    start: pos,
                    from: p.slice.axis_slice.offset,
                    right,
                    up,
                    metres_per_pixel,
                })
            }
            _ => None,
        }
    }

    /// The point and world normal of the face under `pos` on the active
    /// mesh.
    fn path_slice_face_hit(&self, pos: Pos2, r: Rect) -> Option<(DVec3, DVec3)> {
        let surface = self.pick_surface();
        let (item, face, depth) = surface.pick(&self.camera, pos, r)?;
        let active = self.editor.scene.active()?;
        if active.id != item {
            return None;
        }
        let mesh = &active.mesh;
        if face as usize >= mesh.faces.len() || mesh.is_polyline(face as usize) {
            return None;
        }
        let (o, d) = self.camera.ray(pos, r);
        let at = o + d * depth;
        let matrix = self.editor.scene.world_matrix(active.id).ok()?;
        let normal = matrix
            .inverse()
            .transpose()
            .transform_vector3(mesh.normal(face as usize))
            .normalize_or_zero();
        (normal.length() > 0.5).then_some((at, normal))
    }

    /// Polygon Axis Slice's box: the world bounds of the base scene's
    /// selected polygons (all when none).
    fn axis_slice_box(&self) -> Option<(DVec3, DVec3)> {
        let live = self.editor.session.live.as_ref()?;
        let item = live.base.active()?;
        let world = live.base.world_matrix(item.id).ok()?;
        let faces = if live.base.selection.mode == Mode::Polygon {
            live.base.selection.faces.clone()
        } else {
            Vec::new()
        };
        operandi_core::axis_slice_bounds(&item.mesh, world, &faces)
    }

    pub(super) fn path_slice_drag(&mut self, drag: &PathDrag, pos: Pos2, r: Rect, mods: Modifiers) {
        let Some(tool) = self.slice_tool() else {
            return;
        };
        let Some(mut p) = self.slice_params() else {
            return;
        };
        let _ = mods;
        match *drag {
            PathDrag::Point { index, start } => {
                let (plane, normal) = self.path_slice_plane(&p, tool);
                let Some(at) = self.work_point(pos, r, plane, self.workplane_center()) else {
                    return;
                };
                if tool == Tool::PenSlice {
                    // SNH-Q01: locks act once the pointer has moved off the
                    // press, never on a plain click.
                    let lock = pos.distance(start) >= CLICK_MOVE;
                    let points = &p.slice.pen_slice.points;
                    let at = self.pen_slice_snap(points, Some(index), at, normal, lock, r);
                    let o = &mut p.slice.pen_slice;
                    match o.points.get_mut(index) {
                        Some(q) if *q != at => *q = at,
                        _ => return,
                    }
                } else {
                    match p.slice.curve_slice.points.get_mut(index) {
                        Some(q) if *q != at => *q = at,
                        _ => return,
                    }
                }
            }
            PathDrag::Shape {
                anchor,
                start,
                from_center,
                size,
            } => {
                if pos.distance(start) < 3. {
                    return;
                }
                let o = &mut p.slice.primitive_slice;
                let Some(place) = o.place else {
                    return;
                };
                let (ro, rd) = self.camera.ray(pos, r);
                let denom = place.normal.dot(rd);
                if denom.abs() < 1e-9 {
                    return;
                }
                let at = ro + rd * (place.normal.dot(anchor - ro) / denom);
                let (x, y) = (place.x_axis, place.y_axis());
                let (dx, dy) = ((at - anchor).dot(x), (at - anchor).dot(y));
                let (sx, sy) = (
                    if dx < 0. { -1. } else { 1. },
                    if dy < 0. { -1. } else { 1. },
                );
                let minimum = 1e-4;
                let (mut hx, mut hy) = if from_center {
                    (dx.abs(), dy.abs())
                } else {
                    (dx.abs() / 2., dy.abs() / 2.)
                };
                if o.shape == PrimitiveShape::Circle {
                    let radius = if from_center {
                        (dx * dx + dy * dy).sqrt()
                    } else {
                        hx.max(hy)
                    };
                    (hx, hy) = (radius, radius);
                    o.radius = radius.max(minimum);
                } else {
                    if o.keep_aspect && size.0 > 0. && size.1 > 0. {
                        let scale = (hx / size.0).max(hy / size.1);
                        (hx, hy) = (size.0 * scale, size.1 * scale);
                    }
                    o.half_x = hx.max(minimum);
                    o.half_y = hy.max(minimum);
                }
                let center = if from_center {
                    anchor
                } else {
                    anchor + x * sx * hx + y * sy * hy
                };
                o.place = Some(ShapePlace { center, ..place });
                o.held = o.lazy_apply;
            }
            PathDrag::Offset {
                start,
                from,
                right,
                up,
                metres_per_pixel,
            } => {
                let o = &mut p.slice.axis_slice;
                let mut offset = from;
                offset[right.0] += (pos.x - start.x) as f64 * metres_per_pixel * right.1;
                offset[up.0] -= (pos.y - start.y) as f64 * metres_per_pixel * up.1;
                if o.offset == offset {
                    return;
                }
                o.offset = offset;
            }
        }
        self.slice_publish(p);
    }

    /// The drag ended: Lazy Apply cuts now.
    pub(super) fn path_slice_release(&mut self, drag: &PathDrag) {
        if matches!(drag, PathDrag::Shape { .. })
            && let Some(mut p) = self.slice_params()
            && p.slice.primitive_slice.held
        {
            p.slice.primitive_slice.held = false;
            self.slice_publish(p);
        }
    }

    /// Remove Last Point (Pen and Curve Slice's Tool Properties button).
    pub(super) fn path_slice_remove_last(&mut self, tool: Tool) {
        let Some(mut p) = self.slice_params() else {
            return;
        };
        let removed = match tool {
            Tool::PenSlice => p.slice.pen_slice.points.pop(),
            Tool::CurveSlice => p.slice.curve_slice.points.pop(),
            _ => None,
        };
        if removed.is_some() {
            self.slice_publish(p);
            self.editor.end_tweak();
        }
    }

    fn polyline(
        &self,
        painter: &egui::Painter,
        r: Rect,
        points: &[DVec3],
        closed: bool,
        stroke: Stroke,
    ) {
        let screen: Vec<Option<Pos2>> = points
            .iter()
            .map(|&w| self.camera.project(w, r).map(|(q, _)| q))
            .collect();
        for w in screen.windows(2) {
            if let (Some(a), Some(b)) = (w[0], w[1]) {
                painter.line_segment([a, b], stroke);
            }
        }
        if closed
            && screen.len() >= 3
            && let (Some(Some(a)), Some(Some(b))) = (screen.last(), screen.first())
        {
            painter.line_segment([*a, *b], stroke);
        }
    }

    pub(super) fn draw_path_slice(
        &self,
        painter: &egui::Painter,
        r: Rect,
        tool: Tool,
        p: &Parameters,
    ) {
        let bar = theme::viewport::slice_bar();
        let point = theme::viewport::slice_point();
        match tool {
            Tool::PenSlice | Tool::CurveSlice => {
                let (controls, path, closed) = if tool == Tool::PenSlice {
                    let o = &p.slice.pen_slice;
                    (o.points.clone(), o.points.clone(), o.closed)
                } else {
                    let o = &p.slice.curve_slice;
                    (o.points.clone(), o.sampled(), o.closed)
                };
                self.polyline(painter, r, &path, closed, Stroke::new(1.5f32, point));
                for &w in &controls {
                    if let Some((q, _)) = self.camera.project(w, r) {
                        painter.circle_filled(q, 4., point);
                    }
                }
                // The next segment, from the last point to the pointer.
                if let (Some(&last), Some(h)) = (controls.last(), painter.ctx().pointer_hover_pos())
                    && r.contains(h)
                    && let Some((a, _)) = self.camera.project(last, r)
                {
                    painter.extend(egui::Shape::dashed_line(
                        &[a, h],
                        Stroke::new(1.0f32, point),
                        4.,
                        3.,
                    ));
                }
            }
            Tool::PrimitiveSlice => {
                let o = &p.slice.primitive_slice;
                let width = if o.held { 1.0f32 } else { 1.5 };
                for path in o.paths() {
                    self.polyline(painter, r, &path.points, true, Stroke::new(width, bar));
                }
                if let Some(place) = o.place
                    && let Some((q, _)) = self.camera.project(place.center, r)
                {
                    painter.circle_filled(q, 4., point);
                }
            }
            Tool::AxisSlice => {
                let o = &p.slice.axis_slice;
                let Some((lo, hi)) = self.axis_slice_box() else {
                    return;
                };
                let planes = operandi_core::axis_slice_planes(lo, hi, o);
                let stroke = Stroke::new(if o.hold { 1.0f32 } else { 1.5 }, bar);
                for (axis, list) in planes.iter().enumerate() {
                    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                    for &x in list {
                        // The plane's rectangle across the box.
                        let corner = |a: f64, b: f64| {
                            let mut c = DVec3::ZERO;
                            c[axis] = x;
                            c[u] = a;
                            c[v] = b;
                            c
                        };
                        let rect = [
                            corner(lo[u], lo[v]),
                            corner(hi[u], lo[v]),
                            corner(hi[u], hi[v]),
                            corner(lo[u], hi[v]),
                        ];
                        self.polyline(painter, r, &rect, true, stroke);
                    }
                }
            }
            _ => {}
        }
    }
}
