//! Bend in the view, v2 (specifications/bend-handles-2026-11-15 §2-3).
//!
//! Three things are on screen while Bend is live: the blue Angle ring at
//! the action centre, square to the action axis (BN-10..15); the thin
//! pale-violet spine from the centre to its end, drawn straight (BN-20,
//! BN-25); and the small marker at the spine's end (BN-22). Dragging the
//! ring sets Angle like a Rotate ring; dragging the marker moves the
//! spine's end; any other press moves the centre through the snapping
//! layer, in any selection mode (BN-30/31). There is no sideways
//! drag-to-bend and no haul row (§3, G2).
use super::*;

/// The ring's radius on screen at Handle Scale 1: the Rotate ring's
/// (BN-12, `rotation_ring`).
const RING_PIXELS: f64 = 72.;
/// Half the spine-end marker's arm, in points.
const MARK_PIXELS: f32 = 5.;
/// How near (points) a press must land to grab the ring's line.
const RING_GRAB: f32 = 6.;
/// Points per ring.
const RING_SEGMENTS: usize = 64;
/// The spine's pale violet (BN-20). Not a theme colour yet: only Bend
/// draws a spine.
const SPINE_COLOUR: Color32 = Color32::from_rgb(196, 176, 240);
/// The spine-end marker's light cyan (BN-22).
const MARK_COLOUR: Color32 = Color32::from_rgb(150, 230, 240);

/// The part of Bend's handles a press or the pointer is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BendPart {
    /// The Angle ring (handle 1).
    Ring,
    /// The spine's end (handle 2).
    SpineEnd,
    /// Off the handles: the drag moves the action centre (BH-Q08,
    /// provisional: it only moves the centre).
    Centre,
}

/// What a Bend press drags, from the parameters at the press.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BendDrag {
    pub base: Parameters,
    pub part: BendPart,
}

impl Operandi {
    fn bend_params(&self) -> Option<Parameters> {
        self.editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Bend && !self.editor.is_armed())
            .map(|l| l.params.clone())
    }

    /// The action axis the ring is square to (BN-11).
    fn bend_turn_axis(&self, params: &Parameters) -> DVec3 {
        params
            .bend
            .turn_axis(self.editor.bend_axes(&self.editor.scene))
    }

    /// The ring as a screen polyline, closed, at constant screen size
    /// (BN-12).
    pub(crate) fn bend_ring(&self, params: &Parameters, r: Rect) -> Vec<Pos2> {
        let center = params.bend.base;
        let Some((_, depth)) = self.camera.project(center, r) else {
            return Vec::new();
        };
        let pixels = RING_PIXELS * self.handle_scale_at(center) as f64;
        let radius = pixels
            * if self.camera.view == View::Perspective {
                depth
            } else {
                self.camera.distance
            }
            / (r.height() as f64 * 0.9);
        let axis = self.bend_turn_axis(params);
        let u = axis.any_orthonormal_vector();
        let v = axis.cross(u);
        (0..=RING_SEGMENTS)
            .filter_map(|i| {
                let a = i as f64 * std::f64::consts::TAU / RING_SEGMENTS as f64;
                self.camera
                    .project(center + (u * a.cos() + v * a.sin()) * radius, r)
                    .map(|p| p.0)
            })
            .collect()
    }

    /// The spine's end on screen.
    fn bend_spine_end(&self, params: &Parameters, r: Rect) -> Option<Pos2> {
        Some(
            self.camera
                .project(params.bend.base + params.bend.spine, r)?
                .0,
        )
    }

    /// Which handle is under `pos`: the spine's end first (it is small and
    /// may sit on the ring), then the ring's line. `None` off both.
    pub(crate) fn bend_hit(&self, pos: Pos2, r: Rect) -> Option<BendPart> {
        let params = self.bend_params()?;
        let grab = self.handle_preferences.point_hit_size / 2.;
        if self
            .bend_spine_end(&params, r)
            .is_some_and(|end| end.distance(pos) <= grab.max(MARK_PIXELS + 3.))
        {
            return Some(BendPart::SpineEnd);
        }
        self.bend_ring(&params, r)
            .windows(2)
            .any(|s| input::segment_distance(pos, s[0], s[1]) <= RING_GRAB)
            .then_some(BendPart::Ring)
    }

    /// UIC-05: what dragging the handle under the pointer does (the
    /// reference's handle tooltips, BN-13 and BN-22).
    pub(crate) fn bend_tooltip(&self, pos: Pos2, r: Rect) -> Option<&'static str> {
        Some(match self.bend_hit(pos, r)? {
            BendPart::Ring => {
                "Angle handle. Drag here to bend positions. Ctrl-drag: turn in steps of Rotation Snap Angle."
            }
            BendPart::SpineEnd => {
                "Spine handle. Drag here to move spine vector. Ctrl-drag: keep to one work-plane axis."
            }
            BendPart::Centre => return None,
        })
    }

    /// Where a press at `pos` puts the centre (BN-30/30a/31): the shared
    /// click point (click_start_view.rs), else at the current centre's
    /// depth.
    pub(crate) fn bend_centre_at(&mut self, pos: Pos2, r: Rect, current: DVec3) -> Option<DVec3> {
        self.click_point(pos, r).or_else(|| {
            let (origin, direction) = self.camera.ray(pos, r);
            let (_, _, _, forward) = self.camera.basis();
            let along = direction.dot(forward);
            (along.abs() > 1e-9)
                .then(|| origin + direction * ((current - origin).dot(forward) / along))
        })
    }

    /// A plain or Ctrl press with Bend live.
    pub(crate) fn begin_bend(&mut self, pos: Pos2, r: Rect) -> input::GestureKind {
        let Some(params) = self.bend_params() else {
            return input::GestureKind::Blocked;
        };
        self.clear_fields();
        self.haul_axis = None;
        // The ring's unwrapped turn shares Twist Bevel's ring state: only
        // one tool is live at a time.
        self.twist_bevel_turn = None;
        let part = self.bend_hit(pos, r).unwrap_or(BendPart::Centre);
        self.status = match part {
            BendPart::Ring => "Bend: drag round the ring to bend; Ctrl steps the angle".into(),
            BendPart::SpineEnd => {
                "Bend: drag the spine's end; Ctrl keeps it on one work-plane axis".into()
            }
            BendPart::Centre => {
                // G4: any press that misses the handles moves the centre,
                // on or off the model, through the snapping layer.
                let Some(at) = self.bend_centre_at(pos, r, params.bend.base) else {
                    return input::GestureKind::Blocked;
                };
                let plane = self.actual_plane();
                if !self.run_ok(Command::ToolClick { point: at, plane }) {
                    return input::GestureKind::Blocked;
                }
                "Bend: centre moved and spine fitted; drag the ring to bend".into()
            }
        };
        let base = self.bend_params().unwrap_or(params);
        input::GestureKind::Bend(BendDrag { base, part })
    }

    pub(crate) fn bend_drag(
        &mut self,
        drag: &BendDrag,
        start: Pos2,
        pos: Pos2,
        r: Rect,
        ctrl: bool,
    ) {
        let mut params = drag.base.clone();
        match drag.part {
            BendPart::Centre => {
                let Some(at) = self.bend_centre_at(pos, r, params.bend.base) else {
                    return;
                };
                let plane = self.actual_plane();
                self.run(Command::ToolClick { point: at, plane });
                return;
            }
            BendPart::Ring => {
                let Some(degrees) = self.bend_ring_turn(&params, start, pos, r) else {
                    return;
                };
                params.bend.angle += degrees;
                if ctrl {
                    // BN-14 (provisional, BH-Q06): steps of Rotation Snap
                    // Angle, as the Rotate ring does (HM-20).
                    params.bend.angle = self
                        .handle_preferences
                        .round_to_snap_angle(params.bend.angle);
                }
            }
            BendPart::SpineEnd => {
                let d = pos - start;
                let scale = self.camera.distance / (r.height() as f64 * 0.9);
                let (_, right, up, _) = self.camera.basis();
                let mut moved = (right * d.x as f64 - up * d.y as f64) * scale;
                if ctrl {
                    // BN-23: one work-plane axis, the one the drag heads
                    // toward first (HM-10, shared with Move's hauls).
                    let Some(axis) = self.haul_lock_axis(d) else {
                        return;
                    };
                    moved = axis * moved.dot(axis);
                }
                let mut end = params.bend.base + params.bend.spine + moved;
                // BN-23 (provisional, BH-Q07): the end snaps like any point
                // handle; a Ctrl drag stays on its axis.
                if !ctrl && let Some(snapped) = self.snap_point(end, Some(Default::default()), r) {
                    end = snapped;
                }
                let spine = end - params.bend.base;
                if spine.length() < 1e-6 {
                    return;
                }
                params.bend.spine = spine;
            }
        }
        if self.run_ok(Command::Preview(params.clone())) {
            self.clear_fields();
            self.status = format!(
                "Bend: Angle {:.1}°, spine {}",
                params.bend.angle,
                workspace::number_text(params.bend.spine.length(), " m")
            );
        }
    }

    /// The pointer's turn round the ring's centre on screen since the
    /// press, in degrees of Angle (BN-13): the angle swept, so the same
    /// travel further out turns less. A turn the way the spine's end
    /// leans adds Angle. Turns past half a circle carry on from the last
    /// sample.
    fn bend_ring_turn(
        &mut self,
        params: &Parameters,
        start: Pos2,
        pos: Pos2,
        r: Rect,
    ) -> Option<f64> {
        let center = params.bend.base;
        let (c, _) = self.camera.project(center, r)?;
        let (a, b) = (start - c, pos - c);
        if a.length() < 2. || b.length() < 2. {
            return Some(self.twist_bevel_turn.unwrap_or(0.));
        }
        // Screen y runs down: an anticlockwise turn on screen lowers atan2.
        let raw = -((b.y.atan2(b.x) - a.y.atan2(a.x)) as f64).to_degrees();
        let (eye, _, _, _) = self.camera.basis();
        let axis = self.bend_turn_axis(params);
        // Right-handed about the axis as seen from the eye.
        let raw = if axis.dot(eye - center) >= 0. {
            raw
        } else {
            -raw
        };
        // A positive Angle turns the spine toward spine × axis, which is a
        // left-handed turn about the axis (bend.rs `direction`).
        let raw = -raw;
        let previous = self.twist_bevel_turn.unwrap_or(0.);
        let unwrapped = raw + 360. * ((previous - raw) / 360.).round();
        self.twist_bevel_turn = Some(unwrapped);
        Some(unwrapped)
    }

    /// The ring, the straight spine and its end marker (G1, G6, G7). The
    /// part under the pointer, or being dragged, is highlighted (BN-15).
    pub(crate) fn draw_bend(&self, painter: &Painter, r: Rect, hover: Option<Pos2>) {
        let Some(p) = self.bend_params() else {
            return;
        };
        let dragging = self.navigation.as_ref().and_then(|g| match &g.kind {
            input::GestureKind::Bend(drag) => Some(drag.part),
            _ => None,
        });
        let lit = dragging.or_else(|| hover.and_then(|h| self.bend_hit(h, r)));
        let colour = |part: BendPart, normal: Color32| {
            if lit == Some(part) {
                palette::handle_active()
            } else {
                normal
            }
        };
        let width = self.handle_preferences.line_width;
        let project = |w: DVec3| self.camera.project(w, r).map(|(q, _)| q);
        if let (Some(a), Some(b)) = (project(p.bend.base), project(p.bend.base + p.bend.spine)) {
            painter.line_segment([a, b], Stroke::new(1_f32, SPINE_COLOUR));
            let mark = Stroke::new(1.5_f32, colour(BendPart::SpineEnd, MARK_COLOUR));
            let arm = MARK_PIXELS * self.handle_scale_at(p.bend.base + p.bend.spine);
            painter.line_segment([b - vec2(arm, 0.), b + vec2(arm, 0.)], mark);
            painter.line_segment([b - vec2(0., arm), b + vec2(0., arm)], mark);
        }
        let ring = self.bend_ring(&p, r);
        if ring.len() > 1 {
            painter.add(Shape::line(
                ring,
                Stroke::new(width, colour(BendPart::Ring, palette::handles()[2])),
            ));
        }
    }
}
