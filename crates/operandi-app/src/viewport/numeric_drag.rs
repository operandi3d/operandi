//! A drag anywhere in the viewport for the numeric (panel-driven) sessions:
//! Spike, Bridge, Polygon Reduction and the Torus/Capsule/Arc/Tube shapes
//! (Jason, 2026-11-04: a click-drag anywhere drives the tool's main action).
//! Up/down acts like a move arrow, left/right like a scale widget. Each
//! press-to-release is one tweak of the session. Provisional (DRAG-Q01).
use super::*;
use input::NumericDrag;

/// Pixels of left/right drag per Bridge segment.
const BRIDGE_PIXELS_PER_SEGMENT: f32 = 12.;
/// A dragged Bridge stops here; more can be typed in the panel.
const BRIDGE_DRAG_SEGMENTS: f32 = 64.;

impl Operandi {
    /// Start the drag for whichever numeric session is armed.
    pub(crate) fn begin_numeric_drag(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        if let Some(spike) = &self.editor.session.spike {
            return input::GestureKind::Numeric(NumericDrag::Spike(spike.requested));
        }
        if let Some(bridge) = &self.editor.session.bridge {
            return input::GestureKind::Numeric(NumericDrag::Bridge(bridge.requested));
        }
        if let Some(reduction) = &self.editor.session.reduction {
            // The press initializes from the selected mesh (RED-002's
            // stationary release still works: it finds it initialized).
            if !reduction.initialized() {
                if !self.run_ok(Command::InitializeReduction) {
                    return input::GestureKind::Blocked;
                }
                self.status = self.reduction_status();
            }
            let Some(session) = &self.editor.session.reduction else {
                return input::GestureKind::Blocked;
            };
            let Some(count) = session.source.as_ref().map(|s| s.count) else {
                return input::GestureKind::Blocked;
            };
            return input::GestureKind::Numeric(NumericDrag::Reduction(session.parameters, count));
        }
        if self.editor.session.shape.is_some() {
            let plane = self.actual_plane();
            return match self.work_point(pos, r, plane, self.workplane_center()) {
                Some(anchor) if self.tube_points().is_some() => {
                    self.begin_tube_point(plane, anchor, pos, r)
                }
                Some(anchor) => input::GestureKind::Numeric(NumericDrag::Shape { plane, anchor }),
                None => {
                    self.status =
                        "View is parallel to the construction plane; align the view before drawing"
                            .into();
                    input::GestureKind::Blocked
                }
            };
        }
        input::GestureKind::Blocked
    }

    pub(crate) fn numeric_drag(
        &mut self,
        drag: &NumericDrag,
        start: Pos2,
        pos: Pos2,
        r: Rect,
        uniform: bool,
    ) {
        let d = pos - start;
        match drag {
            NumericDrag::Spike(base) => {
                // Strength is relative to the mean edge length; up lengthens.
                let strength = base - d.y as f64 * 0.01;
                if self
                    .editor
                    .session
                    .spike
                    .as_ref()
                    .is_some_and(|s| s.requested != strength)
                {
                    self.run(Command::ConfigureSpike(strength));
                }
            }
            NumericDrag::Bridge(base) => {
                let steps = (d.x / BRIDGE_PIXELS_PER_SEGMENT).trunc();
                let segments = (base.segments as f32 + steps)
                    .clamp(0., BRIDGE_DRAG_SEGMENTS.max(base.segments as f32))
                    as u32;
                if self
                    .editor
                    .session
                    .bridge
                    .as_ref()
                    .is_some_and(|b| b.requested.segments != segments)
                {
                    let mut p = *base;
                    p.segments = segments;
                    self.run(Command::ConfigureBridge(p));
                }
            }
            NumericDrag::Reduction(base, count) => {
                // Half a percent of the source per pixel.
                let change = d.x as f64 * 0.005 * *count as f64;
                let number = (base.number as f64 + change)
                    .round()
                    .clamp(1., *count as f64) as u32;
                if self
                    .editor
                    .session
                    .reduction
                    .as_ref()
                    .is_some_and(|s| s.parameters.number != number)
                {
                    let mut p = *base;
                    p.number = number;
                    if self.run_ok(Command::ConfigureReduction(p)) {
                        self.status = self.reduction_status();
                    }
                }
            }
            NumericDrag::Shape { plane, anchor } => {
                self.shape_draw(*plane, *anchor, pos, r, uniform)
            }
            NumericDrag::TubePoint { plane, index } => {
                if let Some(q) = self.work_point(pos, r, *plane, self.workplane_center()) {
                    let q = self.tube_snap(q, Some(*index), r);
                    self.set_tube_points(|points| {
                        if let Some(p) = points.get_mut(*index) {
                            *p = q;
                        }
                    });
                }
            }
        }
        self.clear_fields();
    }

    /// The live Tube's control points, when the shape session is a Tube.
    fn tube_points(&self) -> Option<&[DVec3]> {
        match &self.editor.session.shape.as_ref()?.requested {
            ShapeParameters::Tube(t) => Some(&t.points),
            _ => None,
        }
    }

    /// DSN-08/10: a Tube path point snapped to the active targets; the
    /// path's other points are targets, the dragged one (`index`) isn't.
    fn tube_snap(&mut self, point: DVec3, index: Option<usize>, r: Rect) -> DVec3 {
        self.note_snap_drag();
        if !self.snap_targets().any() {
            return point;
        }
        let points = self.tube_points().unwrap_or_default().to_vec();
        // The path's points aren't mesh points: the dragged one is left out
        // of `own`, and nothing is passed as moving, so a point dragged onto
        // a mesh point keeps that target on the next frame.
        let own: Vec<DVec3> = points
            .iter()
            .enumerate()
            .filter(|&(i, _)| Some(i) != index)
            .map(|(_, &p)| p)
            .collect();
        self.snap_draw_point(point, &[], &own, None, r)
            .unwrap_or(point)
    }

    fn set_tube_points(&mut self, edit: impl FnOnce(&mut Vec<DVec3>)) {
        let Some(session) = &self.editor.session.shape else {
            return;
        };
        let mut p = session.requested.clone();
        if let ShapeParameters::Tube(t) = &mut p {
            let before = t.points.clone();
            edit(&mut t.points);
            if t.points != before {
                self.run(Command::ConfigureShape(p));
            }
        }
    }

    /// A Tube shape session is on.
    pub(crate) fn tube_active(&self) -> bool {
        self.tube_points().is_some()
    }

    /// The Tube path point under `pos` (nearest within 8 px).
    pub(crate) fn tube_hit(&self, pos: Pos2, r: Rect) -> Option<usize> {
        const GRAB_PIXELS: f32 = 8.;
        self.tube_points()?
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| {
                let (screen, _) = self.camera.project(p, r)?;
                let d = screen.distance(pos);
                (d <= GRAB_PIXELS).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// DPM-04/06/08: remove Tube path point `index` (with `and_after`, it
    /// and every later one); the tube re-forms along the rest with its
    /// Sides and Radius unchanged. The point before the gap is current.
    pub(crate) fn delete_tube_point(&mut self, index: usize, and_after: bool) {
        self.set_tube_points(|points| {
            if index >= points.len() {
                return;
            }
            if and_after {
                points.truncate(index);
            } else {
                points.remove(index);
            }
        });
        let len = self.tube_points().map_or(0, <[DVec3]>::len);
        self.point_modes.tube_current = len
            .checked_sub(1)
            .map(|last| index.saturating_sub(1).min(last));
        self.clear_fields();
    }

    /// Tube drawing (Jason, 2026-11-05) with Mode (SPEC-DRAW-POINT-MODE
    /// DPM-08): a press on a path point makes it current and grabs it (in
    /// Delete mode removes it); a press anywhere else, in Add mode, adds a
    /// point right after the current one and the same drag moves it.
    fn begin_tube_point(
        &mut self,
        plane: u8,
        anchor: DVec3,
        pos: Pos2,
        r: Rect,
    ) -> input::GestureKind {
        use crate::draw_tools::DrawMode;
        self.snap_session = Default::default();
        let mode = self.point_modes.tube;
        let points = self.tube_points().unwrap_or_default().to_vec();
        if let Some(i) = self.tube_hit(pos, r) {
            if mode == DrawMode::Delete {
                self.delete_tube_point(i, false);
                return input::GestureKind::Blocked;
            }
            self.point_modes.tube_current = Some(i);
            return input::GestureKind::Numeric(NumericDrag::TubePoint { plane, index: i });
        }
        if mode != DrawMode::Add {
            return input::GestureKind::Blocked;
        }
        let anchor = self.tube_snap(anchor, None, r);
        let at = self
            .point_modes
            .tube_current
            .map_or(points.len(), |c| (c + 1).min(points.len()));
        self.set_tube_points(|points| points.insert(at, anchor));
        if self.tube_points().map_or(0, <[DVec3]>::len) != points.len() + 1 {
            return input::GestureKind::Blocked;
        }
        self.point_modes.tube_current = Some(at);
        input::GestureKind::Numeric(NumericDrag::TubePoint { plane, index: at })
    }

    /// The press is the centre; the distance on the
    /// plane sets the size, keeping the shape's proportions, standing along
    /// the plane's normal.
    fn shape_draw(&mut self, plane: u8, anchor: DVec3, pos: Pos2, r: Rect, uniform: bool) {
        let Some(q) = self.work_point(pos, r, plane, self.workplane_center()) else {
            return;
        };
        let size = q.distance(anchor);
        if size < 1e-6 {
            return;
        }
        let Some(session) = &self.editor.session.shape else {
            return;
        };
        let normal = match plane {
            0 => 1,
            1 => 0,
            _ => 2,
        };
        let mut p = session.requested.clone();
        // Without Ctrl, Torus and Capsule start as a flat base on the plane
        // centred on the press (drag-creation-2026-11-06; `session_draw`).
        if !uniform {
            let extent = crate::session_draw::flat_extent(anchor, q, normal);
            match &mut p {
                ShapeParameters::Torus(t) => {
                    let ratio = if t.ring_radius > 0. {
                        t.cross_section / t.ring_radius
                    } else {
                        0.5
                    };
                    t.position = anchor;
                    t.ring_radius = size;
                    t.cross_section = size * ratio;
                    t.height = 0.;
                    t.axis = normal;
                    self.run(Command::ConfigureShape(p));
                    return;
                }
                ShapeParameters::Capsule(c) => {
                    if (0..3).filter(|&i| i != normal).any(|i| extent[i] < 1e-6) {
                        return;
                    }
                    c.radius = extent;
                    c.position = anchor;
                    c.axis = normal;
                    self.run(Command::ConfigureShape(p));
                    return;
                }
                _ => {}
            }
        }
        match &mut p {
            ShapeParameters::Torus(t) => {
                let ratio = if t.ring_radius > 0. {
                    t.cross_section / t.ring_radius
                } else {
                    0.5
                };
                let height = if t.cross_section > 0. {
                    t.height / t.cross_section
                } else {
                    1.
                };
                t.position = anchor;
                t.ring_radius = size;
                t.cross_section = size * ratio;
                t.height = t.cross_section * height;
                t.axis = normal;
            }
            ShapeParameters::Capsule(c) => {
                let mut radius = c.radius;
                // Keep the proportions with the long axis along the normal.
                let old = c.axis.min(2);
                if old != normal {
                    radius.as_mut().swap(old, normal);
                }
                let across = (0..3)
                    .filter(|&i| i != normal)
                    .map(|i| radius[i])
                    .fold(0., f64::max);
                let factor = if across > 0. { size / across } else { 1. };
                c.end_size *= factor;
                c.radius = radius * factor;
                c.position = anchor;
                c.axis = normal;
            }
            ShapeParameters::Arc(a) => {
                let mut n = DVec3::ZERO;
                n[normal] = 1.;
                a.center = anchor;
                a.normal = n;
                a.start_direction = (q - anchor).normalize();
                a.radius = size;
            }
            // Text: a drag anywhere moves the start point on the plane
            // (provisional; TXT-17's size and tracking handles aren't built).
            ShapeParameters::Text(t) => {
                t.position = q;
            }
            // Teapot: the same, its box centre follows (missing-tools-g2,
            // provisional: no drawn size).
            ShapeParameters::Teapot(t) => {
                t.position = q;
            }
            // Tube presses never get here: see `begin_tube_point`.
            ShapeParameters::Tube(t) => {
                t.points = vec![anchor, q];
            }
        }
        self.run(Command::ConfigureShape(p));
    }
}
