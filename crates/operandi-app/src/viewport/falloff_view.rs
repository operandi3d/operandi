//! Geometric falloffs in the viewport (falloff spec 2026-11-15 batch 3):
//! the Linear wedge, Radial rings and Cylinder tube, their handles (points
//! plus per-axis handles, FO-40..42), drawing a falloff by click-drag
//! (FO-05) and Use World Transforms' space (FO-35).
use super::*;
use glam::DMat4;
use operandi_core::CylinderFalloff;

/// Screen length of a per-axis handle, in pixels.
const AXIS_HANDLE_PIXELS: f64 = 34.;

pub(crate) use crate::handle_ids::FalloffHandle;

impl Operandi {
    /// FO-35: where the active falloff's positions live. With Use World
    /// Transforms off they are in the active mesh's own space, drawn and
    /// dragged through its world matrix.
    pub(crate) fn falloff_space(&self) -> DMat4 {
        let world = self.editor.spatial_falloff.is_none_or(|f| f.uses_world());
        if world {
            return DMat4::IDENTITY;
        }
        self.editor
            .scene
            .active()
            .and_then(|i| self.editor.scene.world_matrix(i.id).ok())
            .filter(|m| m.determinant().is_finite() && m.determinant() != 0.)
            .unwrap_or(DMat4::IDENTITY)
    }

    /// The falloff's handle points in world space: Linear Start and End;
    /// Radial Center then one Size handle per axis; Cylinder Center then
    /// the Size handles across its Axis. (The app uses `falloff_handles`;
    /// tests read the points.)
    #[cfg(test)]
    pub(crate) fn falloff_points(&self) -> Vec<DVec3> {
        falloff_points_in(
            self.editor.spatial_falloff,
            self.falloff_space(),
            &self.editor.falloff_path,
        )
    }

    /// Every handle with its world position: the points first (so point
    /// `i` is handle `i`), then the per-axis handles of the points that
    /// have them.
    pub(crate) fn falloff_handles(&self, r: Rect) -> Vec<(DVec3, FalloffHandle)> {
        self.falloff_handles_in(
            self.editor.spatial_falloff,
            self.falloff_space(),
            &self.editor.falloff_path,
            r,
        )
    }

    /// `falloff_handles` for any falloff `field` whose positions live in
    /// `space`: the tool's falloff, or a Mesh Stack op's own copy (GitHub
    /// #27), so both get the same handles.
    pub(crate) fn falloff_handles_in(
        &self,
        field: Option<Falloff>,
        space: DMat4,
        path: &[DVec3],
        r: Rect,
    ) -> Vec<(DVec3, FalloffHandle)> {
        let points = falloff_points_in(field, space, path);
        let mut out: Vec<(DVec3, FalloffHandle)> = points
            .iter()
            .enumerate()
            .map(|(i, &p)| (p, FalloffHandle::Point(i)))
            .collect();
        let axis_points: &[usize] = match field {
            Some(Falloff::Linear(_)) => &[0, 1],
            Some(Falloff::Radial(_) | Falloff::Cylinder(_)) => &[0],
            _ => &[],
        };
        for &i in axis_points {
            let Some(&p) = points.get(i) else { continue };
            let Some(mpp) = self.camera.metres_per_pixel(p, r) else {
                continue;
            };
            for axis in 0..3 {
                let dir = space
                    .transform_vector3(DVec3::AXES[axis])
                    .normalize_or_zero();
                out.push((
                    p + dir * AXIS_HANDLE_PIXELS * mpp,
                    FalloffHandle::Axis(i, axis),
                ));
            }
        }
        out
    }
}

/// The falloff `field`'s handle points in world space (`space` takes its
/// positions there): Linear Start and End; Radial Center then one Size
/// handle per axis; Cylinder Center then the Size handles across its
/// Axis; a Path's control points `path`.
pub(crate) fn falloff_points_in(
    field: Option<Falloff>,
    space: DMat4,
    path: &[DVec3],
) -> Vec<DVec3> {
    let local: Vec<DVec3> = match field {
        Some(Falloff::Linear(f)) => vec![f.start, f.end],
        Some(Falloff::Radial(f)) => {
            let mut v = vec![f.center];
            for axis in 0..3 {
                let mut p = f.center;
                p[axis] += f.size[axis];
                v.push(p);
            }
            v
        }
        Some(Falloff::Cylinder(f)) => {
            let mut v = vec![f.center];
            for axis in cross_axes(f) {
                let mut p = f.center;
                p[axis] += f.size[axis];
                v.push(p);
            }
            v
        }
        // FO-53: the Path's control points.
        Some(Falloff::Path(_)) => path.to_vec(),
        // VMF-02: Vertex Map (and the others) have no handles.
        _ => Vec::new(),
    };
    local
        .into_iter()
        .map(|p| space.transform_point3(p))
        .collect()
}

/// TP-19: whether `h` is a Radial or Cylinder Size handle of `field`,
/// drawn as a box in its axis colour.
pub(crate) fn is_size_box_for(h: FalloffHandle, field: Option<Falloff>) -> bool {
    matches!(
        (h, field),
        (
            FalloffHandle::Point(1..),
            Some(Falloff::Radial(_) | Falloff::Cylinder(_))
        )
    )
}

/// Where point handle `kind` of `start` is, in the falloff's own space:
/// Linear Start or End, the Radial or Cylinder centre. `None` for size
/// handles and falloffs without point handles (#65: only points snap).
fn handle_point(start: Falloff, kind: FalloffHandle) -> Option<DVec3> {
    let (FalloffHandle::Point(point) | FalloffHandle::Axis(point, _)) = kind;
    match start {
        Falloff::Linear(f) => Some(if point == 0 { f.start } else { f.end }),
        Falloff::Radial(f) if point == 0 => Some(f.center),
        Falloff::Cylinder(f) if point == 0 => Some(f.center),
        _ => None,
    }
}

/// `delta` as handle `kind` applies it: a per-axis handle keeps only its
/// axis.
fn moved_only(kind: FalloffHandle, delta: DVec3) -> DVec3 {
    match kind {
        FalloffHandle::Axis(_, axis) => {
            let mut only = DVec3::ZERO;
            only[axis] = delta[axis];
            only
        }
        FalloffHandle::Point(_) => delta,
    }
}

/// `start` with handle `kind` moved by `delta` in the falloff's own
/// space; a per-axis handle keeps only its axis. `None` for a Path (its
/// points live apart) and falloffs without handles.
pub(crate) fn moved_falloff(
    start: Falloff,
    kind: FalloffHandle,
    mut delta: DVec3,
) -> Option<Falloff> {
    let point = match kind {
        FalloffHandle::Point(i) => i,
        FalloffHandle::Axis(i, axis) => {
            let only = delta[axis];
            delta = DVec3::ZERO;
            delta[axis] = only;
            i
        }
    };
    Some(match start {
        Falloff::Linear(mut f) => {
            if point == 0 {
                f.start += delta;
            } else {
                f.end += delta;
            }
            Falloff::Linear(f)
        }
        Falloff::Radial(mut f) => {
            if point == 0 {
                f.center += delta;
            } else {
                let axis = point - 1;
                f.size[axis] = (f.size[axis] + delta[axis]).max(0.);
            }
            Falloff::Radial(f)
        }
        Falloff::Cylinder(mut f) => {
            if point == 0 {
                f.center += delta;
            } else if let Some(&axis) = cross_axes(f).get(point - 1) {
                f.size[axis] = (f.size[axis] + delta[axis]).max(0.);
            }
            Falloff::Cylinder(f)
        }
        _ => return None,
    })
}

impl Operandi {
    /// The handle under `pos`, as an index into `falloff_handles`.
    pub(crate) fn falloff_hit(&self, pos: Pos2, r: Rect) -> Option<usize> {
        if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| !l.tool.card().layers.falloff)
        {
            return None;
        }
        if !self.handle_preferences.unselected_shown() {
            return None;
        }
        let reach = self.handle_preferences.small_point_hit / 2.;
        let handles = self.falloff_handles(r);
        let centre = handles
            .first()
            .and_then(|&(p, _)| self.camera.project(p, r))
            .map(|(q, _)| q);
        let nearest = |boxes_only: bool| {
            handles
                .iter()
                .enumerate()
                .filter(|&(_, &(_, h))| !boxes_only || self.is_size_box(h))
                .filter(|&(_, &(p, h))| !self.is_size_box(h) || !self.box_edge_on(p, centre, r))
                .filter_map(|(i, &(p, _))| Some((i, self.camera.project(p, r)?.0.distance(pos))))
                .filter(|&(_, d)| d <= reach)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
        };
        // TP-19: a tool that brings its own falloff starts both at one spot
        // (Soft Move), so the falloff's Centre and its axis handles sit on
        // the tool's handle. There the tool's handle wins the press, unless
        // a Size box (a small, separate target) is in reach too.
        let hit = nearest(false)?;
        if self.is_size_box(handles[hit].1)
            || self.editor.session.live.is_none()
            || self.handle_hit(pos, r).is_none()
        {
            return Some(hit);
        }
        nearest(true)
    }

    /// TP-19: whether `h` is a Radial or Cylinder Size handle, drawn as a
    /// box in its axis colour (the Scale handles' shape, so it reads as
    /// "drag to size").
    pub(crate) fn is_size_box(&self, h: FalloffHandle) -> bool {
        is_size_box_for(h, self.editor.spatial_falloff)
    }

    /// TP-19: a Size box whose axis points along the view lands on the
    /// falloff's centre and can't be dragged there (a drag moves in the
    /// view plane), so it is neither drawn nor hit in that view.
    pub(crate) fn box_edge_on(&self, at: DVec3, centre: Option<Pos2>, r: Rect) -> bool {
        let reach = self.handle_preferences.small_point_hit;
        match (self.camera.project(at, r), centre) {
            (Some((q, _)), Some(c)) => q.distance(c) < reach,
            _ => true,
        }
    }

    /// TP-19: the Size boxes, drawn after the tool's handles so they stay
    /// visible where the two overlap. Filled squares in the axis colour
    /// with a dark rim, at the secondary handles' size (Handle Scale:
    /// Small).
    pub(crate) fn draw_falloff_size_boxes(&self, painter: &egui::Painter, r: Rect) {
        if self.editor.is_armed() || !self.handle_preferences.unselected_shown() {
            return;
        }
        if self
            .editor
            .session
            .live
            .as_ref()
            .is_some_and(|l| !l.tool.card().layers.falloff)
            || self.editor.active_falloff().is_none()
        {
            return;
        }
        let Some(field) = self.editor.spatial_falloff else {
            return;
        };
        self.draw_falloff_size_boxes_in(painter, r, field, self.falloff_space());
    }

    /// The Size boxes of any falloff `field` in `space` (GitHub #27: a Mesh
    /// Stack op's own falloff too).
    pub(crate) fn draw_falloff_size_boxes_in(
        &self,
        painter: &egui::Painter,
        r: Rect,
        field: Falloff,
        space: DMat4,
    ) {
        let side = self.handle_preferences.small_point_size();
        let handles = self.falloff_handles_in(Some(field), space, &[], r);
        let centre = handles
            .first()
            .and_then(|&(p, _)| self.camera.project(p, r))
            .map(|(q, _)| q);
        for (p, kind) in handles {
            let FalloffHandle::Point(i) = kind else {
                continue;
            };
            if !is_size_box_for(kind, Some(field)) || self.box_edge_on(p, centre, r) {
                continue;
            }
            let Some((q, _)) = self.camera.project(p, r) else {
                continue;
            };
            let axis = match field {
                Falloff::Cylinder(f) => cross_axes(f)[(i - 1).min(1)],
                _ => (i - 1).min(2),
            };
            let side = side * self.handle_zoom_at(p);
            let rect = Rect::from_center_size(q, Vec2::splat(side));
            painter.rect_filled(rect, 0., palette::handles()[axis]);
            painter.rect_stroke(
                rect,
                0.,
                Stroke::new(1_f32, palette::edge_backing()),
                egui::StrokeKind::Outside,
            );
        }
    }

    /// Move falloff handle `handle` (an index into `falloff_handles`) by the
    /// drag in the view plane; a per-axis handle keeps only that axis.
    pub(crate) fn drag_falloff(
        &mut self,
        handle: usize,
        start: Falloff,
        from: Pos2,
        to: Pos2,
        r: Rect,
    ) {
        let Some(&(_, kind)) = self.falloff_handles(r).get(handle) else {
            return;
        };
        let (_, right, up, _) = self.camera.basis();
        let scale = self.camera.distance / (r.height() as f64 * 0.9);
        let d = to - from;
        let world_delta = (right * d.x as f64 - up * d.y as f64) * scale;
        // Into the falloff's own space (FO-35).
        let mut delta = self
            .falloff_space()
            .inverse()
            .transform_vector3(world_delta);
        if let Falloff::Path(_) = start {
            // The points move in the falloff's own space, from where they
            // were at the press (`path_drag_base`).
            let (FalloffHandle::Point(point) | FalloffHandle::Axis(point, _)) = kind;
            if let FalloffHandle::Axis(_, axis) = kind {
                let only = delta[axis];
                delta = DVec3::ZERO;
                delta[axis] = only;
            }
            let mut points = self
                .path_drag_base
                .clone()
                .unwrap_or_else(|| self.editor.falloff_path.clone());
            if let Some(p) = points.get_mut(point) {
                *p += delta;
                self.run(Command::FalloffPath(points));
            }
            return;
        }
        // #65: with snapping on, a point handle (Linear Start/End, the
        // Radial and Cylinder centres) snaps where it lands; an axis handle
        // snaps along its axis.
        if let Some(base) = handle_point(start, kind) {
            let space = self.falloff_space();
            let from_world = space.transform_point3(base);
            let landed = from_world + space.transform_vector3(moved_only(kind, delta));
            let line = match kind {
                FalloffHandle::Axis(_, axis) => space
                    .transform_vector3(DVec3::AXES[axis])
                    .try_normalize()
                    .map(|d| (from_world, d)),
                FalloffHandle::Point(_) => None,
            };
            let snapped = self.snap_placement(landed, &[], line, r);
            delta = space.inverse().transform_vector3(snapped - from_world);
        }
        let Some(next) = moved_falloff(start, kind, delta) else {
            return;
        };
        if Some(next) != self.editor.spatial_falloff && next.validate().is_ok() {
            self.run(Command::SpatialFalloff(Some(next)));
        }
    }

    /// FO-53: while adding points, a click off the handles adds one on the
    /// plane facing the view through the last point (or the tool's
    /// centre).
    pub(crate) fn path_falloff_click(&mut self, pos: Pos2, r: Rect) -> bool {
        if !self.falloff_path_editing
            || !matches!(self.editor.spatial_falloff, Some(Falloff::Path(_)))
        {
            return false;
        }
        let space = self.falloff_space();
        let through = self
            .editor
            .falloff_path
            .last()
            .map(|&p| space.transform_point3(p))
            .unwrap_or_else(|| self.transform_handle_center());
        let Some(at) = self.view_plane_point(through, pos, r) else {
            return false;
        };
        let mut points = self.editor.falloff_path.clone();
        points.push(space.inverse().transform_point3(at));
        self.run(Command::FalloffPath(points));
        self.status = format!(
            "Path: {} point(s). Click to add more; Stop Adding Points when done",
            self.editor.falloff_path.len()
        );
        true
    }

    /// The Linear wedge, Radial rings or Cylinder tube, then the handles.
    pub(crate) fn draw_falloff_shape(&self, painter: &egui::Painter, r: Rect, field: Falloff) {
        self.draw_falloff_shape_in(
            painter,
            r,
            field,
            self.falloff_space(),
            &self.editor.falloff_path,
        );
    }

    /// `draw_falloff_shape` for any falloff `field` in `space` with Path
    /// points `path` (GitHub #27: a Mesh Stack op's own falloff too).
    pub(crate) fn draw_falloff_shape_in(
        &self,
        painter: &egui::Painter,
        r: Rect,
        field: Falloff,
        space: DMat4,
        path: &[DVec3],
    ) {
        let project = |p: DVec3| {
            self.camera
                .project(space.transform_point3(p), r)
                .map(|(q, _)| q)
        };
        let stroke = Stroke::new(1.5_f32, palette::selection());
        let faint = Stroke::new(1_f32, palette::selection().gamma_multiply(0.6));
        let ellipse = |center: DVec3, size: DVec3, u: usize, v: usize| {
            let ring: Vec<Pos2> = (0..=48)
                .filter_map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / 48.;
                    let mut p = center;
                    p[u] += size[u] * t.cos();
                    p[v] += size[v] * t.sin();
                    project(p)
                })
                .collect();
            for pair in ring.windows(2) {
                painter.line_segment([pair[0], pair[1]], faint);
            }
        };
        match field {
            Falloff::Linear(f) => {
                if let (Some(a), Some(b)) = (project(f.start), project(f.end)) {
                    let [tip, left, right] = linear_falloff_wedge(a, b);
                    painter.line_segment([tip, left], stroke);
                    painter.line_segment([tip, right], stroke);
                    painter.line_segment([left, right], stroke);
                }
            }
            Falloff::Radial(f) => {
                for (u, v) in [(0, 1), (1, 2), (0, 2)] {
                    ellipse(f.center, f.size, u, v);
                }
            }
            // FO-42 (proposed): rings across the Axis at the centre and
            // either end of a stretch, joined by lines at the radius.
            Falloff::Cylinder(f) => {
                let [u, v] = cross_axes(f);
                let reach = 2. * f.size[u].abs().max(f.size[v].abs()).max(1e-6);
                for k in [-1., 0., 1.] {
                    let mut c = f.center;
                    c[f.axis] += k * reach;
                    ellipse(c, f.size, u, v);
                }
                for (axis, sign) in [(u, 1.), (u, -1.), (v, 1.), (v, -1.)] {
                    let mut a = f.center;
                    a[axis] += sign * f.size[axis];
                    let mut b = a;
                    a[f.axis] -= reach;
                    b[f.axis] += reach;
                    if let (Some(a), Some(b)) = (project(a), project(b)) {
                        painter.line_segment([a, b], faint);
                    }
                }
            }
            // FO-53: the curve through the points.
            Falloff::Path(_) => {
                let curve = operandi_core::falloff_path::path_curve(path);
                let pts: Vec<Pos2> = curve.iter().filter_map(|&p| project(p)).collect();
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts, stroke));
                }
            }
            _ => {}
        }
        // Falloff handles are the secondary (unselected) tool handles: Small
        // Points at Handle Scale: Small, or none when Invisible. Per-axis
        // handles are short lines in the axis colours.
        if !self.handle_preferences.unselected_shown() {
            return;
        }
        let radius = self.handle_preferences.small_point_size() / 2.;
        let world = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        let handles = self.falloff_handles_in(Some(field), space, path, r);
        for &(p, kind) in &handles {
            // TP-19: the Size boxes draw later, over the tool's handles.
            if is_size_box_for(kind, Some(field)) {
                continue;
            }
            let radius = radius * self.handle_zoom_at(p);
            let Some(q) = world(p) else { continue };
            let color = match kind {
                FalloffHandle::Point(0) => palette::selection(),
                FalloffHandle::Point(i) => match field {
                    // The Size handles take their axis' colour.
                    Falloff::Cylinder(f) => palette::handles()[cross_axes(f)[(i - 1).min(1)]],
                    _ => palette::handles()[(i + 2) % 3],
                },
                FalloffHandle::Axis(i, axis) => {
                    if let Some(&(base, _)) = handles.get(i)
                        && let Some(b) = world(base)
                    {
                        painter
                            .line_segment([b, q], Stroke::new(1.5_f32, palette::handles()[axis]));
                    }
                    palette::handles()[axis]
                }
            };
            let radius = if matches!(kind, FalloffHandle::Axis(..)) {
                radius * 0.75
            } else {
                radius
            };
            painter.circle_filled(q, radius, color);
            painter.circle_stroke(q, radius, Stroke::new(1_f32, palette::edge_backing()));
        }
    }

    /// FO-05: whether the next press off the handles draws the falloff.
    /// Armed when a geometric falloff is chosen, or by its Draw button.
    pub(crate) fn falloff_draw_ready(&self) -> bool {
        self.falloff_draw_armed
            && matches!(
                self.editor.spatial_falloff,
                Some(Falloff::Linear(_) | Falloff::Radial(_) | Falloff::Cylinder(_))
            )
            && self
                .editor
                .session
                .live
                .as_ref()
                .is_none_or(|l| l.tool.card().layers.falloff)
    }

    /// FO-05: a press starts drawing the falloff. The press sets the
    /// origin (on the plane facing the view through the tool's centre), the
    /// drag sets the size, the release accepts.
    pub(crate) fn begin_falloff_draw(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        let center = self.transform_handle_center();
        let Some(origin) = self.view_plane_point(center, pos, r) else {
            return input::GestureKind::Blocked;
        };
        self.falloff_draw_armed = false;
        self.status = "Drag to size the falloff; release to accept".into();
        input::GestureKind::FalloffDraw { origin }
    }

    /// FO-05 drag: Linear runs Start (the press) to End (the pointer);
    /// Radial is a sphere and Cylinder a tube around the press, their size
    /// the distance dragged (proposed).
    pub(crate) fn falloff_draw_drag(&mut self, origin: DVec3, pos: Pos2, r: Rect) {
        let Some(at) = self.view_plane_point(origin, pos, r) else {
            return;
        };
        let space = self.falloff_space().inverse();
        let (o, p) = (space.transform_point3(origin), space.transform_point3(at));
        let size = o.distance(p);
        let next = match self.editor.spatial_falloff {
            Some(Falloff::Linear(f)) if size > 1e-9 => Falloff::Linear(LinearFalloff {
                start: o,
                end: p,
                ..f
            }),
            Some(Falloff::Radial(f)) if size > 1e-9 => Falloff::Radial(RadialFalloff {
                center: o,
                size: DVec3::splat(size),
                ..f
            }),
            Some(Falloff::Cylinder(f)) if size > 1e-9 => Falloff::Cylinder(CylinderFalloff {
                center: o,
                size: DVec3::splat(size),
                ..f
            }),
            _ => return,
        };
        if Some(next) != self.editor.spatial_falloff && next.validate().is_ok() {
            self.run(Command::SpatialFalloff(Some(next)));
        }
    }

    /// Where `pos` meets the plane through `through` facing the view.
    fn view_plane_point(&self, through: DVec3, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (_, _, _, forward) = self.camera.basis();
        let (origin, direction) = self.camera.ray(pos, r);
        let along = direction.dot(forward);
        if along.abs() < 1e-9 {
            return None;
        }
        Some(origin + direction * ((through - origin).dot(forward) / along))
    }
}

/// The two axes across a Cylinder's Axis, in order.
pub(crate) fn cross_axes(f: CylinderFalloff) -> [usize; 2] {
    match f.axis {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    }
}
