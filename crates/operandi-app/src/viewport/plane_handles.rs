//! Two-axis plane handles on Move, Scale and Transform's move part
//! (plane-handle spec v1, Jason 2026-11-08 after his hand test; PH-xx
//! rules). A circle lies flat in the plane of its two arrows, at their outer
//! corner; dragging it moves (Scale: scales) along those two axes only.
//! Scale also gets a larger circle at its center for uniform scale.
use super::*;
use operandi_core::ExtendPlane;

/// Handle ids: `PLANE_HANDLE_BASE + n` is the circle whose plane has handle
/// axis `n` as its normal (0: YZ, 1: XZ, 2: XY). Clear of every other id.
pub(crate) const PLANE_HANDLE_BASE: usize = 13;
/// PH-08: circle diameter as a share of the arrow length (about 1/6),
/// scaled by Handle Scale and the Plane Handle Ratio preference.
pub(crate) const PLANE_HANDLE_DIAMETER: f32 = 1. / 6.;
/// PH-31: Scale's center circle, as a share of the arrow length (larger
/// than the plane circles).
pub(crate) const SCALE_CENTER_DIAMETER: f32 = 1. / 4.;
/// PH-07: where the circle's center sits along each of its two arrows, as a
/// share of the arrow length: 1 is the outer corner of the square the two
/// arrows make.
pub(crate) const PLANE_HANDLE_PLACEMENT: f32 = 1.;
/// PH-08: the faint dark see-through fill (alpha out of 255).
pub(crate) const PLANE_HANDLE_FILL_ALPHA: u8 = 70;
/// PH-05: outline width, px.
pub(crate) const PLANE_HANDLE_LINE: f32 = 1.;
/// PH-05: pick reach beyond the outline, px (slightly wider than the line).
pub(crate) const PLANE_HANDLE_PICK_SLACK: f32 = 2.;
/// PH-07a/11: a plane is hidden and can't be picked when the view looks
/// this close to along it (|plane normal · view direction| below this).
pub(crate) const PLANE_HANDLE_EDGE_ON: f64 = 0.2;
/// On-screen arrow length at Handle Scale 1, the same as the axis arrows.
const ARROW_PIXELS: f32 = 72.;
/// Points per outline, enough for a smooth ellipse at these sizes.
const RIM_SEGMENTS: usize = 32;

pub(crate) fn is_plane_handle(id: usize) -> bool {
    (PLANE_HANDLE_BASE..PLANE_HANDLE_BASE + 3).contains(&id)
}
/// The plane's normal axis and its two in-plane axes, in order.
fn plane_axes_of(id: usize) -> (usize, usize, usize) {
    let n = id - PLANE_HANDLE_BASE;
    let (a, b) = ((n + 1) % 3, (n + 2) % 3);
    (n, a.min(b), a.max(b))
}
fn plane_name(id: usize) -> &'static str {
    ["YZ", "XZ", "XY"][id - PLANE_HANDLE_BASE]
}
/// The world axes, for handles whose arrows aren't a frame.
fn world_axis(axis: usize) -> DVec3 {
    [DVec3::X, DVec3::Y, DVec3::Z][axis]
}

/// A circle on screen: its center and the two half-axis vectors of the
/// ellipse it projects to (for a plane circle, its radius along each
/// in-plane world axis as the camera sees it).
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlaneEllipse {
    pub center: Pos2,
    pub u: Vec2,
    pub v: Vec2,
}
impl PlaneEllipse {
    fn rim(&self) -> Vec<Pos2> {
        (0..RIM_SEGMENTS)
            .map(|i| {
                let t = i as f32 * std::f32::consts::TAU / RIM_SEGMENTS as f32;
                self.center + self.u * t.cos() + self.v * t.sin()
            })
            .collect()
    }
    /// Inside the ellipse, or within the pick slack of its outline (PH-05).
    pub(crate) fn contains(&self, p: Pos2) -> bool {
        let det = self.u.x * self.v.y - self.u.y * self.v.x;
        if det.abs() > 1e-6 {
            let d = p - self.center;
            let x = (d.x * self.v.y - d.y * self.v.x) / det;
            let y = (self.u.x * d.y - self.u.y * d.x) / det;
            if x * x + y * y <= 1. {
                return true;
            }
        }
        let rim = self.rim();
        (0..rim.len()).any(|i| {
            input::segment_distance(p, rim[i], rim[(i + 1) % rim.len()]) <= PLANE_HANDLE_PICK_SLACK
        })
    }
    /// PH-08/10: the faint dark fill inside a thin outline in `rim`.
    fn draw(&self, painter: &Painter, rim: Color32, fade: f32) {
        let fill = Color32::from_black_alpha((f32::from(PLANE_HANDLE_FILL_ALPHA) * fade) as u8);
        let outline = self.rim();
        painter.add(Shape::convex_polygon(outline.clone(), fill, Stroke::NONE));
        painter.add(Shape::closed_line(
            outline,
            Stroke::new(PLANE_HANDLE_LINE, rim.gamma_multiply(fade)),
        ));
    }
}

impl Operandi {
    /// PH-30/31/32/32a: which tools show plane circles. Transform shows only
    /// its move part's circles; Rotate shows none.
    pub(crate) fn tool_has_plane_handles(tool: Tool) -> bool {
        tool.card().handles.plane_circles
    }
    /// The live tool shows plane circles: those above, and Edge Extend
    /// with its Plane option on (spec: Plane None / Move / Scale).
    fn shows_plane_handles(&self, tool: Tool) -> bool {
        Self::tool_has_plane_handles(tool)
            || (tool == Tool::EdgeExtend && self.extend_plane() != ExtendPlane::None)
    }
    fn extend_plane(&self) -> ExtendPlane {
        self.editor
            .session
            .live
            .as_ref()
            .map_or(ExtendPlane::None, |l| l.params.edge_extend.plane)
    }
    /// What a circle does for `tool`: Edge Extend's circles act as Move's
    /// or Scale's, as its Plane option says.
    pub(crate) fn plane_tool(&self, tool: Tool) -> Tool {
        match (tool, self.extend_plane()) {
            (Tool::EdgeExtend, ExtendPlane::Scale) => Tool::Scale,
            (Tool::EdgeExtend, _) => Tool::Move,
            _ => tool,
        }
    }
    /// The arrow length in px (PH-04: constant on screen at any zoom;
    /// original-compatible A1 scales it with the zoom).
    fn arrow_pixels(&self) -> f32 {
        ARROW_PIXELS * self.handle_scale_at(self.transform_handle_center())
    }
    /// PH-08: circle diameter in px.
    pub(crate) fn plane_handle_size(&self) -> f32 {
        self.arrow_pixels() * PLANE_HANDLE_DIAMETER * self.handle_preferences.plane_ratio
    }
    /// PH-31: Scale's center circle diameter in px.
    pub(crate) fn scale_center_size(&self) -> f32 {
        self.arrow_pixels() * SCALE_CENTER_DIAMETER * self.handle_preferences.plane_ratio
    }
    /// PH-21 (FG-Q01, Jason 2026-11-22): the circles turn with the arrows.
    /// A circle lies in the plane of the two arrows it sits between, so
    /// under a Local or Selection action axis it stays with its gizmo
    /// rather than lying in a world plane. The handle's axes when they
    /// make a frame (Move, Scale, Transform), the world's otherwise.
    fn plane_axes(&self) -> [DVec3; 3] {
        let axes = [0, 1, 2].map(|k| self.handle_axis(k));
        let unit = axes.iter().all(|v| (v.length() - 1.).abs() < 1e-6);
        let square = (0..3).all(|k| axes[k].dot(axes[(k + 1) % 3]).abs() < 1e-6);
        if unit && square {
            axes
        } else {
            [0, 1, 2].map(world_axis)
        }
    }
    /// PH-07/07a: circle `id` as drawn: centered at the outer corner of its
    /// two arrows, lying flat in its arrows' plane (PH-21). `None` when the plane is
    /// nearly edge-on to the view (PH-11) or an axis is out of view.
    pub(crate) fn plane_ellipse(&self, id: usize, r: Rect) -> Option<PlaneEllipse> {
        let (normal, a, b) = plane_axes_of(id);
        let axes = self.plane_axes();
        if axes[normal].dot(self.camera.basis().3).abs() < PLANE_HANDLE_EDGE_ON {
            return None;
        }
        let center = self.transform_handle_center();
        let (origin, depth) = self.camera.project(center, r)?;
        // World length of one pixel at the gizmo, as the rotation rings use.
        let per_pixel = if self.camera.view == View::Perspective {
            depth
        } else {
            self.camera.distance
        } / (r.height() as f64 * 0.9);
        // The circle sits where the two arrows' square closes. The arrows
        // are drawn in perspective (`handle_arrow`), so the corner is too.
        let arrow = |axis: usize| -> Option<Vec2> {
            let (_, v) =
                self.handle_arrow(center, self.handle_axis(axis), self.arrow_pixels(), r)?;
            Some(v)
        };
        let at = origin + (arrow(a)? + arrow(b)?) * PLANE_HANDLE_PLACEMENT;
        // The circle's radius along each world axis, as the camera sees it.
        let radius = (self.plane_handle_size() / 2.) as f64 * per_pixel;
        let along = |axis: usize| -> Option<Vec2> {
            let (p, _) = self.camera.project(center + axes[axis] * radius, r)?;
            Some(p - origin)
        };
        Some(PlaneEllipse {
            center: at,
            u: along(a)?,
            v: along(b)?,
        })
    }
    /// The plane circles for the live tool, as `handles()` entries (id,
    /// circle center, circle center, label).
    pub(crate) fn plane_handles(&self, tool: Tool, r: Rect) -> Vec<(usize, Pos2, Pos2, String)> {
        if !self.shows_plane_handles(tool) {
            return Vec::new();
        }
        (0..3)
            .filter_map(|n| {
                let id = PLANE_HANDLE_BASE + n;
                let e = self.plane_ellipse(id, r)?;
                Some((id, e.center, e.center, format!("{} plane", plane_name(id))))
            })
            .collect()
    }
    /// PH-11: the circle under `p`, asked only after the center handle and
    /// the arrows found nothing.
    pub(crate) fn plane_handle_hit(&self, p: Pos2, r: Rect) -> Option<usize> {
        let tool = self.editor.session.live.as_ref()?.tool;
        if !self.shows_plane_handles(tool) {
            return None;
        }
        (0..3)
            .map(|n| PLANE_HANDLE_BASE + n)
            .filter_map(|id| self.plane_ellipse(id, r).map(|e| (id, e)))
            .filter(|(_, e)| e.contains(p))
            .min_by(|a, b| a.1.center.distance(p).total_cmp(&b.1.center.distance(p)))
            .map(|(id, _)| id)
    }
    /// PH-10: a plane circle's rim colour: the axis it doesn't move along
    /// (PH-02), or the hover colour while hovered or dragged. The fill never
    /// changes.
    pub(crate) fn plane_handle_rim(id: usize, active: bool) -> Color32 {
        if active {
            palette::handle_active()
        } else {
            palette::handles()[plane_axes_of(id).0]
        }
    }
    /// PH-01/02/03/07a/08/10: draw a plane circle flat in its plane.
    pub(crate) fn draw_plane_handle(
        &self,
        painter: &Painter,
        id: usize,
        r: Rect,
        active: bool,
        fade: f32,
    ) {
        if let Some(ellipse) = self.plane_ellipse(id, r) {
            ellipse.draw(painter, Self::plane_handle_rim(id, active), fade);
        }
    }
    /// PH-31: Scale's center circle (facing the view), with the same look.
    pub(crate) fn draw_scale_center(&self, painter: &Painter, at: Pos2, rim: Color32, fade: f32) {
        let radius = self.scale_center_size() / 2.;
        PlaneEllipse {
            center: at,
            u: vec2(radius, 0.),
            v: vec2(0., radius),
        }
        .draw(painter, rim, fade);
    }
    /// Whether `p` is on Scale's center circle (PH-05 pick slack).
    pub(crate) fn scale_center_contains(&self, at: Pos2, p: Pos2) -> bool {
        at.distance(p) <= self.scale_center_size() / 2. + PLANE_HANDLE_PICK_SLACK
    }
    /// PH-10: the circle's tooltip (UIC-05).
    pub(crate) fn plane_handle_tooltip(tool: Tool, id: usize) -> String {
        let (_, a, b) = plane_axes_of(id);
        let axes = ["X", "Y", "Z"];
        if tool == Tool::Scale {
            format!(
                "Plane handle: drag to scale {} and {} equally",
                axes[a], axes[b]
            )
        } else {
            format!("Plane handle: drag to move in the {} plane", plane_name(id))
        }
    }
    /// PH-20 (Move, Transform) and PH-31 (Scale): one sample of a circle
    /// drag (UIC-76 hauling). Move follows the pointer in the circle's
    /// plane, always on both axes: Ctrl and other modifiers change nothing
    /// here (PH-22). Scale grows its two axes by one factor (right or up
    /// enlarges, like the center).
    pub(crate) fn update_plane_handle(
        &mut self,
        id: usize,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        tool: Tool,
        r: Rect,
    ) {
        let (normal, a, b) = plane_axes_of(id);
        let mut params = base;
        // Edge Extend's Scale circles scale its Scale values (the
        // tutorial's even shrink in the view plane); its Move circles
        // move its Offset like Move's.
        if tool == Tool::EdgeExtend && self.extend_plane() == ExtendPlane::Scale {
            let pixels = ((p.x - start.x) - (p.y - start.y)) as f64;
            let factor = (1. + pixels * 0.01).max(1e-3);
            let s = &mut params.scale_delta;
            s[a] = (s[a] * factor).max(0.001);
            s[b] = (s[b] * factor).max(0.001);
            self.publish_handle_params(tool, params);
            return;
        }
        if tool == Tool::Scale {
            // Scale's values are per handle axis: the circle scales the
            // two arrows it sits between (PH-21).
            let pixels = ((p.x - start.x) - (p.y - start.y)) as f64;
            // #24: down to 0% (items 0.1%); an axis at 0 grows from 0.
            let floor = self.scale_floor();
            let factor = (1. + pixels * 0.01).max(0.);
            for axis in [a, b] {
                let v = params.delta[axis];
                params.delta[axis] = if v.abs() <= 1e-9 {
                    (pixels * 0.01).max(floor)
                } else {
                    (v * factor).max(floor)
                };
            }
            self.publish_handle_params(tool, params);
            return;
        }
        let Some(frame) = self
            .editor
            .session
            .live
            .as_ref()
            .and_then(|l| l.action_frame)
        else {
            return;
        };
        let center = self.transform_handle_center();
        let plane_normal = self.plane_axes()[normal];
        let hit = |q: Pos2| {
            let (origin, direction) = self.camera.ray(q, r);
            let along = direction.dot(plane_normal);
            (along.abs() > 1e-9)
                .then(|| origin + direction * ((center - origin).dot(plane_normal) / along))
        };
        let (Some(from), Some(to)) = (hit(start), hit(p)) else {
            return;
        };
        // Move's values are along the handle's axes (Local's cluster
        // frame under Local), the circle's own two among them.
        let basis = self
            .transform_handle_frame()
            .map_or(frame.basis, |f| f.basis);
        let mut along = basis.transpose() * (to - from);
        // Exactly in the circle's plane: no rounding creeps off it.
        along[normal] = 0.;
        params.delta += along;
        let params = self.snap_move(tool, params, None, r);
        self.publish_handle_params(tool, params);
    }
}
