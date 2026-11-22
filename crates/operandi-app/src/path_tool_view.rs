//! Viewport side of the path tools (CX-25, SPEC-CURVE-CLONE "Interaction"):
//! clicks add path points, drags move them, Delete mode removes them, and
//! Sketch mode turns a freehand stroke into a smooth path
//! (`operandi_core::curve_fit`). Points land on the surface under the
//! pointer (the smooth Sub-D surface the view draws) of any mesh but the
//! one being changed, else on the work plane, and go through the snapping
//! layer. Every change is a `Command::Preview`, so each press-to-release
//! is one tweak while the tool is on, and Drop records one undo step.
use super::*;
use crate::duplicate_view::DupDrag;
use crate::path_tool_panel::{PathMode, lens};
use operandi_core::curve_fit;
use operandi_core::curves::{bezier_anchors, bezier_points};
use operandi_core::path_tools::{PathKind, PathTool};
use slice_view::GRAB;

/// The path's points, for the overlay and hit tests.
fn tool_path(p: &Parameters, tool: Tool) -> &PathTool {
    (lens(tool).0)(p)
}

/// Smooth handles for a Bezier anchor list: each anchor's handles lie
/// along the line between its neighbours, a sixth of that apart (the
/// Curve's own tangents), the ends a third of their span.
pub(crate) fn smooth_handles(anchors: &[DVec3]) -> Vec<[DVec3; 3]> {
    let n = anchors.len();
    (0..n)
        .map(|i| {
            let a = anchors[i];
            let d = match (i.checked_sub(1), anchors.get(i + 1)) {
                (Some(b), Some(&c)) => (c - anchors[b]) / 6.,
                (None, Some(&c)) => (c - a) / 3.,
                (Some(b), None) => (a - anchors[b]) / 3.,
                (None, None) => DVec3::ZERO,
            };
            [a - d, a, a + d]
        })
        .collect()
}

/// Add `at` after the last point. Bezier: a new anchor, with the last two
/// anchors' handles made smooth.
fn push_point(t: &mut PathTool, at: DVec3) -> usize {
    let c = &mut t.curve;
    if c.kind != PathKind::Bezier {
        c.points.push(at);
        return c.points.len() - 1;
    }
    let mut anchors: Vec<[DVec3; 3]> = bezier_anchors(&c.points, false);
    let mut plain: Vec<DVec3> = anchors.iter().map(|a| a[1]).collect();
    plain.push(at);
    let smooth = smooth_handles(&plain);
    let n = plain.len();
    if n >= 2 {
        // Keep earlier handles; reshape only the two newest anchors.
        if n >= 3 {
            anchors[n - 2] = smooth[n - 2];
        } else {
            anchors = vec![smooth[0]];
        }
        anchors.push(smooth[n - 1]);
    } else {
        anchors = vec![[at, at, at]];
    }
    c.points = bezier_points(&anchors, false);
    c.points.len() - 1
}

/// Remove point `index`; a Bezier anchor goes with its handles. Handles
/// alone can't be removed.
fn remove_point(t: &mut PathTool, index: usize) -> bool {
    let c = &mut t.curve;
    if c.kind != PathKind::Bezier || c.points.len() < 4 {
        if index < c.points.len() {
            c.points.remove(index);
            return true;
        }
        return false;
    }
    if !index.is_multiple_of(3) {
        return false;
    }
    let mut anchors = bezier_anchors(&c.points, false);
    if index / 3 >= anchors.len() {
        return false;
    }
    anchors.remove(index / 3);
    c.points = if anchors.len() == 1 {
        vec![anchors[0][1]]
    } else {
        bezier_points(&anchors, false)
    };
    true
}

/// The path from a freehand stroke (world points): fitted cubics, kept as
/// Bezier spans or as their end points for the other kinds.
pub(crate) fn path_from_stroke(kind: PathKind, stroke: &[DVec3], tolerance: f64) -> Vec<DVec3> {
    let spans = curve_fit::fit_stroke(stroke, tolerance);
    if spans.is_empty() {
        return Vec::new();
    }
    if kind == PathKind::Bezier {
        return curve_fit::bezier_point_list(&spans);
    }
    let mut out = vec![spans[0][0]];
    out.extend(spans.iter().map(|c| c[3]));
    // A Curve through two points is straight; a one-span stroke keeps its
    // middle so the path still bends.
    if spans.len() == 1 && kind != PathKind::Pen {
        let c = spans[0];
        let mid = c[0] * 0.125 + c[1] * 0.375 + c[2] * 0.375 + c[3] * 0.125;
        out.insert(1, mid);
    }
    out
}

impl Operandi {
    /// The path tool that's live, if any.
    fn path_tool(&self) -> Option<Tool> {
        self.duplicate_tool()
            .filter(|&t| crate::path_tool_panel::is_path_tool(t))
    }

    /// Where a press puts a path point: on another mesh's surface under the
    /// pointer, else the work plane, then snapped.
    fn path_point(&mut self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let surface = self.pick_surface();
        let active = self.editor.scene.active().map(|i| i.id);
        let on_surface = surface
            .pick(&self.camera, pos, r)
            .filter(|&(item, _, _)| Some(item) != active)
            .map(|(_, _, depth)| {
                let (o, d) = self.camera.ray(pos, r);
                o + d * depth
            });
        let at = match on_surface {
            Some(at) => at,
            None => {
                let plane = self.actual_plane();
                self.work_point(pos, r, plane, self.workplane_center())?
            }
        };
        Some(self.snap_point(at, None, r).unwrap_or(at))
    }

    /// The path point under `pos`, if any.
    fn path_hit(&self, t: &PathTool, pos: Pos2, r: Rect) -> Option<usize> {
        t.curve
            .points
            .iter()
            .enumerate()
            .filter_map(|(i, &w)| Some((i, self.camera.project(w, r)?.0.distance(pos))))
            .filter(|(_, d)| *d <= GRAB)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// A primary press with a path tool live.
    pub(super) fn path_tool_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        middle: bool,
    ) -> Option<input::GestureKind> {
        let tool = self.path_tool()?;
        let mut p = self.slice_params()?;
        let (_, get_mut) = lens(tool);
        let mode = if middle {
            PathMode::Delete
        } else {
            self.point_modes.path
        };
        let hit = self.path_hit(get_mut(&mut p), pos, r);
        let drag = match (mode, hit) {
            (PathMode::Delete, Some(i)) => {
                if remove_point(get_mut(&mut p), i) {
                    self.slice_publish(p);
                } else {
                    self.status =
                        "Delete removes an anchor with its handles; drag a handle to reshape"
                            .into();
                }
                None
            }
            (PathMode::Delete, None) => None,
            (PathMode::Sketch, _) => {
                let at = self.path_point(pos, r)?;
                Some(DupDrag::PathStroke { points: vec![at] })
            }
            (_, Some(index)) => Some(DupDrag::PathPoint { index }),
            (PathMode::Edit, None) => None,
            (PathMode::Add, None) => {
                let Some(at) = self.path_point(pos, r) else {
                    self.status =
                        format!("{}: the view is edge-on to the work plane", tool.label());
                    return Some(input::GestureKind::Blocked);
                };
                let index = push_point(get_mut(&mut p), at);
                self.slice_publish(p);
                Some(DupDrag::PathPoint { index })
            }
        };
        Some(drag.map_or(input::GestureKind::Blocked, input::GestureKind::Duplicate))
    }

    /// Pointer moved while a path drag is held.
    pub(super) fn path_tool_drag(&mut self, drag: &DupDrag, pos: Pos2, r: Rect) {
        let Some(tool) = self.path_tool() else {
            return;
        };
        match drag {
            DupDrag::PathPoint { index } => {
                let Some(mut p) = self.slice_params() else {
                    return;
                };
                let Some(at) = self.path_point(pos, r) else {
                    return;
                };
                let t = (lens(tool).1)(&mut p);
                let Some(old) = t.curve.points.get(*index).copied() else {
                    return;
                };
                // A Bezier anchor carries its handles along.
                if t.curve.kind == PathKind::Bezier
                    && index.is_multiple_of(3)
                    && t.curve.points.len() >= 4
                {
                    let shift = at - old;
                    for j in index.saturating_sub(1)..=(index + 1).min(t.curve.points.len() - 1) {
                        t.curve.points[j] += shift;
                    }
                } else {
                    t.curve.points[*index] = at;
                }
                self.slice_publish(p);
            }
            DupDrag::PathStroke { .. } => {
                let Some(at) = self.path_point(pos, r) else {
                    return;
                };
                if let Some(g) = &mut self.navigation
                    && let input::GestureKind::Duplicate(DupDrag::PathStroke { points }) =
                        &mut g.kind
                {
                    points.push(at);
                }
            }
            _ => {}
        }
    }

    /// Release of a path drag: a Sketch stroke becomes the path.
    pub(super) fn path_tool_release(&mut self, drag: &DupDrag, r: Rect) {
        let DupDrag::PathStroke { points } = drag else {
            return;
        };
        let Some(tool) = self.path_tool() else {
            return;
        };
        let Some(mut p) = self.slice_params() else {
            return;
        };
        // Within about three pixels of the stroke.
        let mid = points.get(points.len() / 2).copied().unwrap_or_default();
        let tolerance = 3. * self.camera.metres_per_pixel(mid, r).unwrap_or(0.01);
        let t = (lens(tool).1)(&mut p);
        let path = path_from_stroke(t.curve.kind, points, tolerance);
        if path.len() < 2 {
            self.status = "Sketch: drag a longer stroke".into();
            return;
        }
        t.curve.points = path;
        self.slice_publish(p);
    }

    /// The path and its points over the view.
    pub(super) fn draw_path_tool(&self, painter: &egui::Painter, r: Rect) {
        let Some(tool) = self.path_tool() else {
            return;
        };
        let Some(p) = self.slice_params() else {
            return;
        };
        let t = tool_path(&p, tool);
        let bar = Stroke::new(1.5_f32, theme::viewport::slice_bar());
        let project = |w: DVec3| self.camera.project(w, r).map(|(q, _)| q);
        let line = |pts: &[DVec3], stroke: Stroke| {
            for w in pts.windows(2) {
                if let (Some(a), Some(b)) = (project(w[0]), project(w[1])) {
                    painter.line_segment([a, b], stroke);
                }
            }
        };
        if t.curve.points.len() >= 2 {
            line(&t.curve.drawn(), bar);
        }
        // A stroke being sketched.
        if let Some(g) = &self.navigation
            && let input::GestureKind::Duplicate(DupDrag::PathStroke { points }) = &g.kind
        {
            line(points, bar);
        }
        let handle = theme::viewport::handles()[1];
        let active = theme::viewport::handle_active();
        let hover = painter.ctx().pointer_hover_pos();
        let bezier = t.curve.kind == PathKind::Bezier && t.curve.points.len() >= 4;
        if bezier {
            for a in bezier_anchors(&t.curve.points, false) {
                line(&[a[0], a[1], a[2]], Stroke::new(1_f32, handle));
            }
        }
        for (i, &w) in t.curve.points.iter().enumerate() {
            if let Some(q) = project(w) {
                let lit = hover.is_some_and(|h| h.distance(q) <= GRAB);
                let anchor = !bezier || i.is_multiple_of(3);
                let s = if lit { 6. } else { 4.5 };
                let color = if lit { active } else { handle };
                if anchor {
                    painter.rect_filled(Rect::from_center_size(q, vec2(2. * s, 2. * s)), 0., color);
                } else {
                    painter.circle_filled(q, s - 1., color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_clicks_get_smooth_handles_and_delete_takes_an_anchor() {
        let mut t = PathTool::default();
        t.curve.kind = PathKind::Bezier;
        push_point(&mut t, DVec3::ZERO);
        assert_eq!(t.curve.points.len(), 1);
        push_point(&mut t, DVec3::new(1., 0., 0.));
        assert_eq!(t.curve.points.len(), 4);
        push_point(&mut t, DVec3::new(2., 1., 0.));
        assert_eq!(t.curve.points.len(), 7);
        assert!(t.has_path());
        // The middle anchor's handles line up through it.
        let a = bezier_anchors(&t.curve.points, false);
        let (i, o) = (a[1][1] - a[1][0], a[1][2] - a[1][1]);
        assert!(i.normalize().dot(o.normalize()) > 0.999999);
        assert!(remove_point(&mut t, 3));
        assert_eq!(t.curve.points.len(), 4);
        assert!(!remove_point(&mut t, 1), "a handle alone stays");
    }

    #[test]
    fn a_sketched_stroke_becomes_a_short_path() {
        let stroke: Vec<DVec3> = (0..100)
            .map(|i| {
                let a = i as f64 / 99. * std::f64::consts::PI;
                DVec3::new(a.cos(), a.sin(), 0.)
            })
            .collect();
        for kind in PathKind::ALL {
            let path = path_from_stroke(kind, &stroke, 0.01);
            assert!(
                path.len() >= 3 && path.len() < 20,
                "{kind:?} {}",
                path.len()
            );
            assert!(path[0].distance(stroke[0]) < 1e-9);
            assert!(path.last().unwrap().distance(stroke[99]) < 1e-9);
        }
    }
}
