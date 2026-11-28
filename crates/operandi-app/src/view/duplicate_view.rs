//! Viewport side of the Duplicate tools (specifications/duplicate-tools):
//! presses, drags, right-drags and the handles. Every change goes through
//! `Command::Preview`, so a press-to-release is one tweak of the tool and
//! Drop records the session as one step (DS-33).
//!
//! Handles (DS-35): Mirror's plane square with a centre grip (slide) and an
//! edge grip (hinge Angle; in Free Rotation a top grip turns the plane);
//! the Circle's axis line with end cubes (tilt) and an arc with start and
//! end cubes (angles); Clone's arrow to the first copy; Array's three axis
//! arrows. Grips use the axis colours or the slice colours, at a constant
//! screen size, and light up under the pointer.
use super::*;
use operandi_core::{Circle, duplicate};
use slice_view::GRAB;

/// A Duplicate tool drag in progress.
#[derive(Clone)]
pub enum DupDrag {
    /// A path tool point follows the pointer (path_tool_view.rs).
    PathPoint { index: usize },
    /// A Sketch stroke being drawn: its world points so far.
    PathStroke { points: Vec<DVec3> },
    /// Mirror's centre grip: Center slides along the plane's facing.
    MirrorCenter,
    /// Mirror's edge grip: Angle follows the pointer round the hinge.
    MirrorAngle,
    /// Free Rotation's top grip: the plane turns about its Left line.
    MirrorTurn,
    /// The Circle's centre follows the pointer on the work plane.
    CircleCenter { plane: u8 },
    /// The Circle's start (`end` false) or end cube: its angle round the
    /// axis, unwrapped so a drag past a full turn keeps going (coils).
    CircleAngle { end: bool },
    /// An axis end cube (`top` or bottom): tilts the axis.
    CircleAxis { top: bool },
    /// Clone: Offset is the pointer's work-plane point less `start`.
    CloneOffset { plane: u8, start: DVec3 },
    /// Array: left-drag spacing on the view's two axes (AR-21), from the
    /// Offset at the press.
    ArraySpacing {
        start: Pos2,
        from: DVec3,
        right: (usize, f64),
        up: (usize, f64),
        metres_per_pixel: f64,
    },
    /// Array: the grip on `axis` (AR-24).
    ArrayGrip { axis: usize },
    /// Right-drag: counts, one per 20 px (CL-22, AR-22, RA-21).
    Count {
        start: Pos2,
        from: [u32; 3],
        right: usize,
        up: usize,
    },
}

/// Pixels of right-drag per count.
const COUNT_STEP: f32 = 20.;

/// The world axis closest to `v`, with its sign, skipping `not`.
fn nearest_axis(v: DVec3, not: Option<usize>) -> (usize, f64) {
    (0..3)
        .filter(|&a| Some(a) != not)
        .map(|a| (a, v[a]))
        .max_by(|x, y| x.1.abs().total_cmp(&y.1.abs()))
        .map_or((0, 1.), |(a, c)| (a, c.signum()))
}

/// Where the ray meets the plane through `origin` facing `normal`.
fn ray_plane(ray: (DVec3, DVec3), origin: DVec3, normal: DVec3) -> Option<DVec3> {
    let denom = ray.1.dot(normal);
    if denom.abs() < 1e-9 {
        return None;
    }
    let t = (origin - ray.0).dot(normal) / denom;
    Some(ray.0 + ray.1 * t)
}

/// The parameter along the line `origin + t·dir` closest to the ray.
fn ray_line(ray: (DVec3, DVec3), origin: DVec3, dir: DVec3) -> Option<f64> {
    let (o, d) = ray;
    let w = origin - o;
    let (a, b, c) = (dir.dot(dir), dir.dot(d), d.dot(d));
    let (dd, e) = (dir.dot(w), d.dot(w));
    let denom = a * c - b * b;
    if denom.abs() < 1e-12 {
        return None;
    }
    Some((b * e - c * dd) / denom)
}

/// Unit vectors at right angles to `d` (the arc's 0° and 90° directions).
fn arc_frame(c: &Circle, towards: DVec3) -> (DVec3, DVec3, DVec3) {
    let d = c.axis_direction().unwrap_or(DVec3::Y);
    let mut u = (towards - c.center) - d * (towards - c.center).dot(d);
    if u.length() < 1e-9 {
        u = d.any_orthonormal_vector();
    }
    let u = u.normalize();
    (d, u, d.cross(u))
}

/// The world point at `degrees` on the arc of radius `radius`.
fn arc_point(c: &Circle, frame: (DVec3, DVec3, DVec3), radius: f64, degrees: f64) -> DVec3 {
    let (_, u, v) = frame;
    let a = degrees.to_radians();
    c.center + (u * a.cos() + v * a.sin()) * radius
}

/// The source's world bounding box, cached per base mesh.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct DupBounds {
    key: (u64, usize, usize),
    pub lo: DVec3,
    pub hi: DVec3,
}

impl Operandi {
    /// The live Duplicate tool, if one is active.
    pub(super) fn duplicate_tool(&self) -> Option<Tool> {
        self.editor
            .session
            .live
            .as_ref()
            .map(|l| l.tool)
            .filter(|t| t.is_duplicate())
    }

    /// The source's world bounds in the session's base scene (the selected
    /// items in Item mode), remembered while the base mesh is the same.
    fn duplicate_bounds(&self) -> Option<(DVec3, DVec3)> {
        let live = self.editor.session.live.as_ref()?;
        let item = live.base.active()?;
        let key = (
            item.id,
            &*item.mesh as *const operandi_core::Mesh as usize,
            live.base.selection.faces.len()
                + live.base.selection.edges.len()
                + live.base.selection.vertices.len(),
        );
        if let Some(b) = self.duplicate_cache.get().filter(|b| b.key == key) {
            return Some((b.lo, b.hi));
        }
        let (lo, hi) = if live.base.selection.mode == Mode::Item {
            let mut lo = DVec3::splat(f64::INFINITY);
            let mut hi = DVec3::splat(f64::NEG_INFINITY);
            for id in live.base.selected_items() {
                let Some(it) = live.base.item(id) else {
                    continue;
                };
                let all: Vec<usize> = (0..it.mesh.faces.len()).collect();
                if let Some((a, b)) = duplicate::faces_bounds(&live.base, id, &all) {
                    lo = lo.min(a);
                    hi = hi.max(b);
                }
            }
            (lo.is_finite() && hi.is_finite()).then_some((lo, hi))?
        } else {
            let faces = duplicate::source_faces(&live.base);
            duplicate::faces_bounds(&live.base, item.id, &faces)?
        };
        self.duplicate_cache.set(Some(DupBounds { key, lo, hi }));
        Some((lo, hi))
    }

    fn duplicate_centre(&self) -> DVec3 {
        self.duplicate_bounds()
            .map_or(self.workplane_center(), |(lo, hi)| (lo + hi) / 2.)
    }

    /// A handle length: the source's size, at least 50 screen points.
    fn duplicate_reach(&self, at: DVec3, r: Rect) -> f64 {
        let px = self.camera.metres_per_pixel(at, r).unwrap_or(0.01);
        let size = self
            .duplicate_bounds()
            .map_or(0., |(lo, hi)| (hi - lo).max_element());
        (size * 0.6).max(50. * px)
    }

    fn duplicate_circle(p: &Parameters, tool: Tool) -> Circle {
        if tool == Tool::Lathe {
            p.duplicate.lathe.circle
        } else {
            p.duplicate.radial.circle
        }
    }

    fn circle_mut(p: &mut Parameters, tool: Tool) -> &mut Circle {
        if tool == Tool::Lathe {
            &mut p.duplicate.lathe.circle
        } else {
            &mut p.duplicate.radial.circle
        }
    }

    /// The arc's radius and frame for the Circle handle.
    fn circle_arc(&self, c: &Circle, r: Rect) -> (f64, (DVec3, DVec3, DVec3)) {
        let centre = self.duplicate_centre();
        let frame = arc_frame(c, centre);
        let radius = (centre - c.foot(centre)).length();
        (radius.max(self.duplicate_reach(c.center, r) * 0.8), frame)
    }

    /// The Mirror plane handle: centre, Left (the hinge), Up turned by the
    /// hinge, half size.
    fn mirror_handle(&self, p: &Parameters, r: Rect) -> Option<(DVec3, DVec3, DVec3, f64)> {
        let m = &p.duplicate.mirror;
        let (_, left, up) = m.frame().ok()?;
        let up = m.hinge().ok()? * up;
        Some((m.center, left, up, self.duplicate_reach(m.center, r)))
    }

    /// The handle points under `pos`, nearest first, as drags.
    fn duplicate_grip(&self, tool: Tool, p: &Parameters, pos: Pos2, r: Rect) -> Option<DupDrag> {
        let near = |w: DVec3| {
            self.camera
                .project(w, r)
                .map(|(q, _)| q.distance(pos))
                .filter(|d| *d <= GRAB)
        };
        let mut hits: Vec<(f32, DupDrag)> = Vec::new();
        match tool {
            Tool::Mirror => {
                let (c, left, up, s) = self.mirror_handle(p, r)?;
                if let Some(d) = near(c) {
                    hits.push((d, DupDrag::MirrorCenter));
                }
                if let Some(d) = near(c + up * s) {
                    hits.push((d, DupDrag::MirrorAngle));
                }
                if p.duplicate.mirror.mode == operandi_core::MirrorMode::FreeRotation
                    && let Some(d) = near(c + left * s)
                {
                    hits.push((d, DupDrag::MirrorTurn));
                }
            }
            Tool::Lathe | Tool::RadialArray => {
                let c = Self::duplicate_circle(p, tool);
                let (radius, frame) = self.circle_arc(&c, r);
                let (d, _, _) = frame;
                let reach = self.duplicate_reach(c.center, r);
                for (end, angle) in [(false, c.start), (true, c.end)] {
                    if let Some(x) = near(arc_point(&c, frame, radius, angle)) {
                        hits.push((x, DupDrag::CircleAngle { end }));
                    }
                }
                for (top, sign) in [(true, 1.), (false, -1.)] {
                    if let Some(x) = near(c.center + d * reach * sign) {
                        hits.push((x, DupDrag::CircleAxis { top }));
                    }
                }
                // The axis line itself moves the centre (LT-31 [OP]).
                if let (Some((a, _)), Some((b, _))) = (
                    self.camera.project(c.center - d * reach, r),
                    self.camera.project(c.center + d * reach, r),
                ) {
                    let x = input::segment_distance(pos, a, b);
                    if x <= GRAB {
                        hits.push((
                            x + 1.,
                            DupDrag::CircleCenter {
                                plane: self.actual_plane(),
                            },
                        ));
                    }
                }
            }
            Tool::Clone if p.duplicate.placed => {
                let centre = self.duplicate_centre();
                if let Some(d) = near(centre + p.duplicate.clone.offset) {
                    hits.push((
                        d,
                        DupDrag::CloneOffset {
                            plane: self.actual_plane(),
                            start: centre,
                        },
                    ));
                }
            }
            Tool::Array if p.duplicate.placed => {
                let centre = self.duplicate_centre();
                for axis in 0..3 {
                    if let Some(d) = near(centre + self.array_grip(p, axis, r)) {
                        hits.push((d, DupDrag::ArrayGrip { axis }));
                    }
                }
            }
            _ => {}
        }
        hits.into_iter()
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|h| h.1)
    }

    /// Array grip `axis`: the first neighbour's place from the centre, or a
    /// short arrow while that Offset is 0.
    fn array_grip(&self, p: &Parameters, axis: usize, r: Rect) -> DVec3 {
        let a = &p.duplicate.array;
        let n = a.count[axis];
        let step = if a.between && n > 1 {
            a.offset[axis] / (n - 1) as f64
        } else {
            a.offset[axis]
        };
        let mut v = DVec3::ZERO;
        v[axis] = if step.abs() < 1e-9 {
            self.duplicate_reach(self.duplicate_centre(), r) * 0.5
        } else {
            step
        };
        v
    }

    /// The work-plane point under `pos`, snapped like any point (DS-37).
    fn duplicate_point(&mut self, pos: Pos2, r: Rect, plane: u8) -> Option<DVec3> {
        let at = self.work_point(pos, r, plane, self.workplane_center())?;
        Some(self.snap_point(at, None, r).unwrap_or(at))
    }

    /// A primary press with a Duplicate tool active. Returns the drag to
    /// follow (a plain click is a drag that doesn't move).
    pub(super) fn duplicate_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        _mods: Modifiers,
    ) -> Option<input::GestureKind> {
        if self
            .duplicate_tool()
            .is_some_and(crate::path_tool_panel::is_path_tool)
        {
            return self.path_tool_press(pos, r, false);
        }
        let tool = self.duplicate_tool()?;
        let mut p = self.slice_params()?;
        if let Some(drag) = self.duplicate_grip(tool, &p, pos, r) {
            return Some(input::GestureKind::Duplicate(drag));
        }
        let placed = p.duplicate.placed;
        let drag = match tool {
            // MR-30: a click shows the result; the plane stays put.
            Tool::Mirror | Tool::ScatterClone => {
                if !placed {
                    p.duplicate.placed = true;
                    self.slice_publish(p);
                }
                None
            }
            Tool::Lathe | Tool::RadialArray => {
                let plane = self.actual_plane();
                let Some(at) = self.duplicate_point(pos, r, plane) else {
                    self.status =
                        format!("{}: the view is edge-on to the work plane", tool.label());
                    return Some(input::GestureKind::Blocked);
                };
                // A click away from the handle moves the axis of this sweep
                // there (Jason, 2026-11-08: DS-34's second sweep of the
                // result surprised him).
                Self::circle_mut(&mut p, tool).center = at;
                p.duplicate.placed = true;
                self.slice_publish(p);
                Some(DupDrag::CircleCenter { plane })
            }
            Tool::Clone => {
                let plane = self.actual_plane();
                let Some(at) = self.duplicate_point(pos, r, plane) else {
                    return Some(input::GestureKind::Blocked);
                };
                if placed && self.run_ok(Command::Continue) {
                    p = self.slice_params()?;
                }
                p.duplicate.clone.offset = DVec3::ZERO;
                p.duplicate.placed = true;
                self.slice_publish(p);
                Some(DupDrag::CloneOffset { plane, start: at })
            }
            Tool::Array => {
                // AR-20: the first click spaces the copies by the source's size.
                if !placed {
                    if let Some((lo, hi)) = self.duplicate_bounds() {
                        p.duplicate.array.offset = hi - lo;
                    }
                    p.duplicate.placed = true;
                    self.slice_publish(p.clone());
                }
                let (_, right, up, _) = self.camera.basis();
                let right = nearest_axis(right, None);
                let up = nearest_axis(up, Some(right.0));
                let centre = self.duplicate_centre();
                Some(DupDrag::ArraySpacing {
                    start: pos,
                    from: p.duplicate.array.offset,
                    right,
                    up,
                    metres_per_pixel: self.camera.metres_per_pixel(centre, r).unwrap_or(0.01),
                })
            }
            _ => None,
        };
        Some(drag.map_or(input::GestureKind::Blocked, input::GestureKind::Duplicate))
    }

    /// A right press: counts follow the drag (CL-22, AR-22, RA-21).
    pub(super) fn duplicate_count_press(&mut self, pos: Pos2) -> Option<input::GestureKind> {
        let tool = self.duplicate_tool()?;
        let p = self.slice_params()?;
        let d = &p.duplicate;
        let (from, right, up) = match tool {
            Tool::Clone => ([d.clone.count, 0, 0], 0, 3),
            Tool::RadialArray => ([d.radial.circle.count, 0, 0], 0, 3),
            Tool::Array => {
                let (_, right, up, _) = self.camera.basis();
                let right = nearest_axis(right, None).0;
                let up = nearest_axis(up, Some(right)).0;
                (d.array.count, right, up)
            }
            _ => return None,
        };
        Some(input::GestureKind::Duplicate(DupDrag::Count {
            start: pos,
            from,
            right,
            up,
        }))
    }

    /// Pointer moved while a Duplicate drag is held.
    pub(super) fn duplicate_drag(&mut self, drag: &DupDrag, pos: Pos2, r: Rect, mods: Modifiers) {
        if matches!(drag, DupDrag::PathPoint { .. } | DupDrag::PathStroke { .. }) {
            self.path_tool_drag(drag, pos, r);
            return;
        }
        let Some(tool) = self.duplicate_tool() else {
            return;
        };
        let Some(mut p) = self.slice_params() else {
            return;
        };
        let before = p.clone();
        let ray = self.camera.ray(pos, r);
        match drag {
            DupDrag::PathPoint { .. } | DupDrag::PathStroke { .. } => {}
            DupDrag::MirrorCenter => {
                let m = &p.duplicate.mirror;
                let Ok(n) = m.plane_normal() else {
                    return;
                };
                let Some(t) = ray_line(ray, m.center, n) else {
                    return;
                };
                let mut centre = m.center + n * t;
                // MR-33: with snapping on, Snap to World Axis puts the plane
                // through the origin when the grip comes near it.
                if m.snap_world_axis && self.snap_targets().any() {
                    let on = centre - n * centre.dot(n);
                    if let (Some((a, _)), Some((b, _))) =
                        (self.camera.project(centre, r), self.camera.project(on, r))
                        && a.distance(b) <= self.snapping.inner_range
                    {
                        centre = on;
                    }
                }
                p.duplicate.mirror.center = centre;
            }
            DupDrag::MirrorAngle => {
                let m = &p.duplicate.mirror;
                let Ok((_, left, up)) = m.frame() else {
                    return;
                };
                // MR-21: round the hinge (Left); the grip sits on Up.
                let Some(hit) = ray_plane(ray, m.center, left) else {
                    return;
                };
                let v = hit - m.center;
                let theta = v.dot(left.cross(up)).atan2(v.dot(up)).to_degrees();
                let mut angle = theta;
                // Ctrl steps by Rotation Snap Angle, as Rotate does (HM-20).
                if mods.ctrl {
                    angle = self.handle_preferences.round_to_snap_angle(angle);
                }
                p.duplicate.mirror.angle = angle;
            }
            DupDrag::MirrorTurn => {
                let m = &p.duplicate.mirror;
                // Free Rotation: the plane turns about its Up line; the grip
                // sits on Left.
                let Ok((_, _, up)) = m.frame() else {
                    return;
                };
                let Some(hit) = ray_plane(ray, m.center, up) else {
                    return;
                };
                let left = (hit - m.center).normalize_or_zero();
                if left != DVec3::ZERO {
                    p.duplicate.mirror.left = left;
                    p.duplicate.mirror.up = up;
                }
            }
            DupDrag::CircleCenter { plane } => {
                let Some(at) = self.duplicate_point(pos, r, *plane) else {
                    return;
                };
                Self::circle_mut(&mut p, tool).center = at;
            }
            DupDrag::CircleAngle { end } => {
                let c = Self::duplicate_circle(&p, tool);
                let (_, frame) = self.circle_arc(&c, r);
                let (d, u, v) = frame;
                let Some(hit) = ray_plane(ray, c.center, d) else {
                    return;
                };
                let w = hit - c.center;
                let angle = w.dot(v).atan2(w.dot(u)).to_degrees();
                let c = Self::circle_mut(&mut p, tool);
                let previous = if *end { c.end } else { c.start };
                // Unwrapped: the nearest turn to the last value (coils).
                let delta = (angle - previous).rem_euclid(360.);
                let delta = if delta > 180. { delta - 360. } else { delta };
                let mut value = previous + delta;
                // Ctrl, or snapping on (SN2-40: angle handles step by
                // Snap Angle while snapping), steps by Rotation Snap Angle.
                if mods.ctrl || self.snapping.enabled {
                    value = self.handle_preferences.round_to_snap_angle(value);
                }
                if *end {
                    c.end = value;
                } else {
                    c.start = value;
                }
            }
            DupDrag::CircleAxis { top } => {
                let c = Self::duplicate_circle(&p, tool);
                let (_, _, _, forward) = self.camera.basis();
                let Some(hit) = ray_plane(ray, c.center, forward) else {
                    return;
                };
                let dir = (hit - c.center).normalize_or_zero() * if *top { 1. } else { -1. };
                if dir != DVec3::ZERO {
                    Self::circle_mut(&mut p, tool).direction = dir;
                }
            }
            DupDrag::CloneOffset { plane, start } => {
                let Some(at) = self.duplicate_point(pos, r, *plane) else {
                    return;
                };
                let mut offset = at - *start;
                if mods.ctrl {
                    // DS-36: only the longer direction moved.
                    let axis = nearest_axis(offset, None).0;
                    offset = DVec3::from_array(std::array::from_fn(|i| {
                        if i == axis { offset[i] } else { 0. }
                    }));
                }
                p.duplicate.clone.offset = offset;
            }
            DupDrag::ArraySpacing {
                start,
                from,
                right,
                up,
                metres_per_pixel,
            } => {
                let (mut dx, mut dy) = (pos.x - start.x, pos.y - start.y);
                if mods.ctrl {
                    if dx.abs() >= dy.abs() {
                        dy = 0.;
                    } else {
                        dx = 0.;
                    }
                }
                let mut offset = *from;
                offset[right.0] += dx as f64 * metres_per_pixel * right.1;
                offset[up.0] -= dy as f64 * metres_per_pixel * up.1;
                p.duplicate.array.offset = offset;
            }
            DupDrag::ArrayGrip { axis } => {
                let centre = self.duplicate_centre();
                let mut dir = DVec3::ZERO;
                dir[*axis] = 1.;
                let Some(t) = ray_line(ray, centre, dir) else {
                    return;
                };
                let a = &mut p.duplicate.array;
                let n = a.count[*axis];
                a.offset[*axis] = if a.between && n > 1 {
                    t * (n - 1) as f64
                } else {
                    t
                };
            }
            DupDrag::Count {
                start,
                from,
                right,
                up,
            } => {
                let steps = |d: f32| (d / COUNT_STEP).round() as i64;
                let (dx, dy) = (steps(pos.x - start.x), steps(start.y - pos.y));
                let count = |base: u32, d: i64| (base as i64 + d).clamp(1, 100_000) as u32;
                let d = &mut p.duplicate;
                match tool {
                    Tool::Clone => d.clone.count = count(from[0], dx),
                    Tool::RadialArray => d.radial.circle.count = count(from[0], dx),
                    Tool::Array => {
                        let (dx, dy) = if mods.ctrl {
                            if dx.abs() >= dy.abs() {
                                (dx, 0)
                            } else {
                                (0, dy)
                            }
                        } else {
                            (dx, dy)
                        };
                        d.array.count[*right] = count(from[*right], dx);
                        d.array.count[*up] = count(from[*up], dy);
                    }
                    _ => {}
                }
                d.placed = true;
            }
        }
        if p != before {
            self.slice_publish(p);
        }
    }

    /// DS-38 (UIC-05): what dragging a grip does, for its tooltip.
    fn grip_tip(drag: &DupDrag) -> String {
        match drag {
            DupDrag::MirrorCenter => "Drag to move the mirror plane".into(),
            DupDrag::MirrorAngle => "Drag to hinge the mirror plane (Angle)".into(),
            DupDrag::MirrorTurn => "Drag to tilt the mirror plane".into(),
            DupDrag::CircleAngle { end: false } => "Drag to set Start Angle".into(),
            DupDrag::CircleAngle { end: true } => "Drag to set End Angle".into(),
            DupDrag::CircleAxis { .. } => "Drag to tilt the axis".into(),
            DupDrag::CircleCenter { .. } => "Drag to move the centre".into(),
            DupDrag::ArrayGrip { axis } => {
                format!("Drag to set {} spacing", ["X", "Y", "Z"][*axis])
            }
            DupDrag::CloneOffset { .. } => "Drag to set the step between copies".into(),
            _ => String::new(),
        }
    }

    /// Overlays of the active Duplicate tool.
    pub(super) fn draw_duplicate(&self, painter: &egui::Painter, r: Rect) {
        self.draw_path_tool(painter, r);
        self.draw_duplicate_handles(painter, r);
        // DS-38: the tooltip of the grip under the pointer (not while a
        // drag is held).
        let Some(tool) = self.duplicate_tool() else {
            return;
        };
        let (Some(p), Some(hover)) = (self.slice_params(), painter.ctx().pointer_hover_pos())
        else {
            return;
        };
        if self.navigation.is_some() || !r.contains(hover) {
            return;
        }
        if let Some(grip) = self.duplicate_grip(tool, &p, hover, r) {
            let tip = Self::grip_tip(&grip);
            if !tip.is_empty() {
                painter.text(
                    hover + vec2(14., 16.),
                    Align2::LEFT_TOP,
                    tip,
                    FontId::proportional(11.),
                    theme::viewport::label(),
                );
            }
        }
    }

    fn draw_duplicate_handles(&self, painter: &egui::Painter, r: Rect) {
        let Some(tool) = self.duplicate_tool() else {
            return;
        };
        let Some(p) = self.slice_params() else {
            return;
        };
        let [hx, hy, hz] = theme::viewport::handles();
        let active = theme::viewport::handle_active();
        let bar = theme::viewport::slice_bar();
        let hover = painter.ctx().pointer_hover_pos();
        let held = self.navigation.as_ref().and_then(|g| match &g.kind {
            input::GestureKind::Duplicate(d) => Some(d.clone()),
            _ => None,
        });
        let lit = |w: DVec3| {
            hover
                .zip(self.camera.project(w, r))
                .is_some_and(|(h, (q, _))| h.distance(q) <= GRAB)
        };
        let line = |a: DVec3, b: DVec3, stroke: Stroke| {
            if let (Some((qa, _)), Some((qb, _))) =
                (self.camera.project(a, r), self.camera.project(b, r))
            {
                painter.line_segment([qa, qb], stroke);
            }
        };
        let dot = |w: DVec3, color: Color32, on: bool| {
            if let Some((q, _)) = self.camera.project(w, r) {
                painter.circle_filled(
                    q,
                    if on { 6. } else { 4.5 },
                    if on { active } else { color },
                );
            }
        };
        let cube = |w: DVec3, color: Color32, on: bool| {
            if let Some((q, _)) = self.camera.project(w, r) {
                let s = if on { 6. } else { 4.5 };
                painter.rect_filled(
                    Rect::from_center_size(q, vec2(2. * s, 2. * s)),
                    0.,
                    if on { active } else { color },
                );
            }
        };
        match tool {
            Tool::Mirror => {
                let Some((c, left, up, s)) = self.mirror_handle(&p, r) else {
                    return;
                };
                let corners = [
                    c + (left + up) * s,
                    c + (-left + up) * s,
                    c + (-left - up) * s,
                    c + (left - up) * s,
                ];
                let colour = [hx, hy, hz][p.duplicate.mirror.axis.min(2)];
                for i in 0..4 {
                    line(
                        corners[i],
                        corners[(i + 1) % 4],
                        Stroke::new(1.5f32, colour),
                    );
                }
                let dragging = |d: fn(&DupDrag) -> bool| held.as_ref().is_some_and(d);
                dot(
                    c,
                    colour,
                    lit(c) || dragging(|d| matches!(d, DupDrag::MirrorCenter)),
                );
                cube(
                    c + up * s,
                    colour,
                    lit(c + up * s) || dragging(|d| matches!(d, DupDrag::MirrorAngle)),
                );
                if p.duplicate.mirror.mode == operandi_core::MirrorMode::FreeRotation {
                    cube(
                        c + left * s,
                        hy,
                        lit(c + left * s) || dragging(|d| matches!(d, DupDrag::MirrorTurn)),
                    );
                }
            }
            Tool::Lathe | Tool::RadialArray => {
                let c = Self::duplicate_circle(&p, tool);
                let (radius, frame) = self.circle_arc(&c, r);
                let (d, _, _) = frame;
                let reach = self.duplicate_reach(c.center, r);
                let colour = [hx, hy, hz][c.axis.min(2)];
                line(
                    c.center - d * reach,
                    c.center + d * reach,
                    Stroke::new(1.5f32, colour),
                );
                for top in [1., -1.] {
                    let w = c.center + d * reach * top;
                    cube(w, colour, lit(w));
                }
                // The swept range, at most two turns drawn.
                let span = (c.end - c.start).clamp(-720., 720.);
                let steps = ((span.abs() / 5.).ceil() as usize).max(2);
                let pts: Vec<Pos2> = (0..=steps)
                    .filter_map(|i| {
                        let a = c.start + span * i as f64 / steps as f64;
                        self.camera
                            .project(arc_point(&c, frame, radius, a), r)
                            .map(|(q, _)| q)
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(1.5f32, bar)));
                for angle in [c.start, c.end] {
                    let w = arc_point(&c, frame, radius, angle);
                    cube(w, bar, lit(w));
                }
                dot(c.center, colour, false);
            }
            Tool::Clone => {
                if !p.duplicate.placed {
                    return;
                }
                let centre = self.duplicate_centre();
                let tip = centre + p.duplicate.clone.offset;
                line(centre, tip, Stroke::new(1.5f32, bar));
                dot(tip, bar, lit(tip));
            }
            Tool::Array => {
                if !p.duplicate.placed {
                    return;
                }
                let centre = self.duplicate_centre();
                for (axis, colour) in [hx, hy, hz].into_iter().enumerate() {
                    let tip = centre + self.array_grip(&p, axis, r);
                    line(centre, tip, Stroke::new(1.5f32, colour));
                    cube(tip, colour, lit(tip));
                }
            }
            _ => {}
        }
    }
}
