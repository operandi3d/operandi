//! Handles of the Snaps and Precision presets (spec
//! `snaps-precision-2026-11-10`): Scale Absolute / Size Absolute's base and
//! scale handles, and Planar Rotate Snap's rest and rotate handles with
//! Planar Rotate Snap Axis's two axis handles. Every one of them snaps
//! (to vertices, through the shared snapping), which is the point of the
//! presets.
//!
//! The drag rules are the spec's guesses (panel Q4, Q6):
//! - Scale/Size: per axis a base handle at `base` from the centre and a
//!   scale handle at `base × factor`. Drag the base onto point A, then the
//!   scale handle onto point B: factor = (B − centre) / (A − centre) along
//!   that axis.
//! - Planar rotate: the rotation axis is the action frame's Z (the clicked
//!   element's normal). The outer handle sets the rest direction (start
//!   +X); the inner handle rotates, the angle measured from the rest
//!   direction to it in the rotation plane.
//! - Planar Rotate Snap Axis: an axis start and end handle; once both are
//!   dragged, the rotation axis runs from the start point to the end point
//!   through the centre.
use super::*;
use operandi_core::{AbsoluteHandles, PickedFrame};

/// First handle id of the precision handles (after the plane handles).
const BASE: usize = 40;
pub(crate) const ABS_BASE: usize = BASE; // + axis
pub(crate) const ABS_SCALE: usize = BASE + 3; // + axis
pub(crate) const ROT_REST: usize = BASE + 6;
pub(crate) const ROT_TARGET: usize = BASE + 7;
pub(crate) const AXIS_START: usize = BASE + 8;
pub(crate) const AXIS_END: usize = BASE + 9;

pub(crate) fn is_precision_handle(id: usize) -> bool {
    (BASE..BASE + 10).contains(&id)
}

/// What a planar rotate preset keeps between drags, in world space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PlanarState {
    /// The rest (zero) direction, across the rotation axis; `None` until
    /// the outer handle is dragged (the frame's X is used).
    pub rest: Option<DVec3>,
    /// Planar Rotate Snap Axis: the axis handles' points.
    pub axis_start: Option<DVec3>,
    pub axis_end: Option<DVec3>,
    /// Scale/Size Absolute: the base handle dragged last, until its scale
    /// handle is.
    pub last_base: Option<usize>,
}

/// The pixel lengths the handles sit at (times Handle Scale).
const REST_PIXELS: f64 = 96.;
const TARGET_PIXELS: f64 = 60.;
const AXIS_PIXELS: f64 = 80.;

impl Operandi {
    /// Which precision handles the live tool shows.
    fn planar_on(&self) -> bool {
        let Some(l) = &self.editor.session.live else {
            return false;
        };
        l.tool == Tool::Rotate
            && (matches!(
                self.snap_preset.map(|(p, _)| p),
                Some(SnapPreset::PlanarRotateSnap | SnapPreset::PlanarRotateSnapAxis)
            ) || self.handle_preferences.selected
                == crate::handle_preferences::HandleDetail::Advanced)
    }
    fn axis_handles_on(&self) -> bool {
        self.snap_preset
            .is_some_and(|(p, _)| p == SnapPreset::PlanarRotateSnapAxis)
            && self.planar_on()
    }
    fn absolute_on(&self) -> Option<AbsoluteHandles> {
        let l = self.editor.session.live.as_ref()?;
        (l.tool == Tool::Scale)
            .then_some(l.params.absolute_handles)
            .flatten()
    }

    /// World length of `pixels` on screen at the handle centre.
    fn world_pixels(&self, center: DVec3, pixels: f64, r: Rect) -> f64 {
        let depth = self.camera.project(center, r).map_or(1., |(_, d)| d);
        let scale = if self.camera.view == View::Perspective {
            depth
        } else {
            self.camera.distance
        };
        pixels * self.handle_scale_at(center) as f64 * scale / (r.height() as f64 * 0.9)
    }

    /// The rotation's rest direction: the dragged one, else the frame's X.
    fn planar_rest(&self) -> DVec3 {
        let axis = self.handle_axis(2);
        let rest = self.planar.rest.unwrap_or_else(|| self.handle_axis(0));
        let rest = rest - axis * rest.dot(axis);
        rest.try_normalize().unwrap_or_else(|| self.handle_axis(0))
    }

    /// The precision handles as (id, world point, label).
    pub(crate) fn precision_points(&self, r: Rect) -> Vec<(usize, DVec3, String)> {
        let mut out = Vec::new();
        if self.editor.session.live.is_none() {
            return out;
        }
        let center = self.transform_handle_center();
        if let Some(abs) = self.absolute_on() {
            let factor = self
                .editor
                .session
                .live
                .as_ref()
                .map_or(DVec3::ONE, |l| l.params.delta);
            for axis in 0..3 {
                let dir = self.handle_axis(axis);
                let name = ["X", "Y", "Z"][axis];
                let (base, scale) = if abs.size {
                    ("Absolute", "Relative")
                } else {
                    ("Base size", "Scale")
                };
                out.push((
                    ABS_BASE + axis,
                    center + dir * abs.base[axis],
                    format!("{base} {name}: drag onto a point to set the base length"),
                ));
                out.push((
                    ABS_SCALE + axis,
                    center + dir * abs.base[axis] * factor[axis],
                    format!("{scale} {name}: drag onto a point; factor = its distance ÷ the base"),
                ));
            }
        }
        if self.planar_on() {
            let axis = self.handle_axis(2);
            let rest = self.planar_rest();
            let angle = self
                .editor
                .session
                .live
                .as_ref()
                .map_or(0., |l| l.params.delta.z.to_radians());
            let turned = glam::DQuat::from_axis_angle(axis, angle) * rest;
            out.push((
                ROT_REST,
                center + rest * self.world_pixels(center, REST_PIXELS, r),
                "Rest direction: drag onto a point to set where the rotation starts".into(),
            ));
            out.push((
                ROT_TARGET,
                center + turned * self.world_pixels(center, TARGET_PIXELS, r),
                "Rotate: drag onto a point to turn the rest direction toward it".into(),
            ));
            if self.axis_handles_on() {
                let reach = self.world_pixels(center, AXIS_PIXELS, r);
                out.push((
                    AXIS_START,
                    self.planar.axis_start.unwrap_or(center - axis * reach),
                    "Axis start: drag onto a point; the axis runs from here to the end".into(),
                ));
                out.push((
                    AXIS_END,
                    self.planar.axis_end.unwrap_or(center + axis * reach),
                    "Axis end: drag onto a point; the axis runs from the start to here".into(),
                ));
            }
        }
        out
    }

    /// The precision handle under `p`, if any. A base and a scale handle on
    /// the same spot (factor 1) give the base first, then, once the base
    /// was dragged, the scale handle (the order the spec uses them in).
    pub(crate) fn precision_hit(&self, p: Pos2, r: Rect) -> Option<usize> {
        let size = self.handle_preferences.point_hit_size.max(10.);
        let rank = |id: usize| {
            let scale_next =
                (ABS_SCALE..ROT_REST).contains(&id) && self.planar.last_base == Some(id - 3);
            let base_first =
                (ABS_BASE..ABS_SCALE).contains(&id) && self.planar.last_base != Some(id);
            if scale_next || base_first { 0 } else { 1 }
        };
        self.precision_points(r)
            .into_iter()
            .filter_map(|(id, at, _)| {
                let (q, _) = self.camera.project(at, r)?;
                let d = q.distance(p);
                (d <= size / 2.).then_some((id, (d * 2.).round() as i64, rank(id)))
            })
            .min_by_key(|&(_, d, rank)| (d, rank))
            .map(|(id, _, _)| id)
    }

    /// Draws the handles; returns the hovered one's tooltip (UIC-05).
    pub(crate) fn draw_precision_handles(
        &self,
        painter: &egui::Painter,
        r: Rect,
        hover: Option<Pos2>,
    ) -> Option<String> {
        let hovered = hover.and_then(|p| self.precision_hit(p, r));
        let mut tip = None;
        let dragging = self.navigation.as_ref().and_then(|g| match &g.kind {
            input::GestureKind::Handle(id, _, _) if is_precision_handle(*id) => Some(*id),
            _ => None,
        });
        let (center, _) = self.camera.project(self.transform_handle_center(), r)?;
        let size = self.handle_preferences.large_point_size()
            * self.handle_zoom_at(self.transform_handle_center());
        let line = self.handle_preferences.line_width;
        for (id, at, label) in self.precision_points(r) {
            let Some((q, _)) = self.camera.project(at, r) else {
                continue;
            };
            if hovered == Some(id) {
                tip = Some(label);
            }
            let lit = hovered == Some(id) || dragging == Some(id);
            let colour = if lit {
                palette::handle_active()
            } else {
                match id {
                    i if i < ABS_SCALE => palette::handles()[i - ABS_BASE],
                    i if i < ROT_REST => palette::handles()[i - ABS_SCALE],
                    ROT_REST | ROT_TARGET => palette::handles()[2],
                    _ => Color32::WHITE,
                }
            };
            match id {
                // Base handles: hollow circles on a thin line from the centre.
                i if i < ABS_SCALE => {
                    painter
                        .line_segment([center, q], Stroke::new(1_f32, colour.gamma_multiply(0.6)));
                    painter.circle_stroke(q, size * 0.6, Stroke::new(line.max(1.5), colour));
                }
                // Scale handles: filled squares.
                i if i < ROT_REST => {
                    painter.rect_filled(Rect::from_center_size(q, Vec2::splat(size)), 0., colour);
                }
                ROT_REST => {
                    painter.line_segment([center, q], Stroke::new(line, colour));
                    painter.circle_stroke(q, size * 0.7, Stroke::new(line.max(1.5), colour));
                }
                ROT_TARGET => {
                    painter.line_segment([center, q], Stroke::new(line, colour));
                    painter.circle_filled(q, size * 0.6, colour);
                }
                _ => {
                    painter.circle_filled(q, size * 0.5, colour);
                    painter.circle_stroke(
                        q,
                        size * 0.5,
                        Stroke::new(1_f32, Color32::from_gray(40)),
                    );
                }
            }
        }
        if self.axis_handles_on()
            && let (Some(a), Some(b)) = (
                self.camera.project(self.precision_point(AXIS_START, r), r),
                self.camera.project(self.precision_point(AXIS_END, r), r),
            )
        {
            painter.line_segment(
                [a.0, b.0],
                Stroke::new(1_f32, Color32::WHITE.gamma_multiply(0.7)),
            );
        }
        tip
    }

    fn precision_point(&self, id: usize, r: Rect) -> DVec3 {
        self.precision_points(r)
            .into_iter()
            .find(|h| h.0 == id)
            .map_or(DVec3::ZERO, |h| h.1)
    }

    /// Where the pointer puts a dragged precision handle: snapped onto a
    /// target when one is in range (SN-08), else on the camera-facing plane
    /// through the handle's press position.
    fn precision_target(&mut self, id: usize, p: Pos2, r: Rect) -> Option<DVec3> {
        let at = self.precision_point(id, r);
        let (origin, direction) = self.camera.ray(p, r);
        let forward = self.camera.basis().3;
        let along = direction.dot(forward);
        let free = if along.abs() < 1e-9 {
            at
        } else {
            origin + direction * ((at - origin).dot(forward) / along)
        };
        // Nothing is left out of the targets: the base handle's whole use is
        // snapping onto the selection's own points (moved ones drop out by
        // themselves, as their positions change in the preview).
        let snapped = self.snap_point(free, Some(Default::default()), r);
        Some(snapped.unwrap_or(free))
    }

    /// One frame of a precision handle drag. `base` is the tool's
    /// parameters at the press.
    pub(crate) fn update_precision_handle(
        &mut self,
        id: usize,
        base: Parameters,
        p: Pos2,
        tool: Tool,
        r: Rect,
    ) {
        let Some(point) = self.precision_target(id, p, r) else {
            return;
        };
        let center = self.transform_handle_center();
        match id {
            i if i < ROT_REST => {
                let Some(abs) = base.absolute_handles else {
                    return;
                };
                let axis = (i - ABS_BASE) % 3;
                let dir = self.handle_axis(axis);
                let along = (point - center).dot(dir);
                let mut params = base;
                if i < ABS_SCALE {
                    // A base length can't be zero; keep its side.
                    let length = if along.abs() < 1e-6 {
                        1e-6_f64.copysign(abs.base[axis])
                    } else {
                        along
                    };
                    let mut abs = abs;
                    abs.base[axis] = length;
                    params.absolute_handles = Some(abs);
                    self.planar.last_base = Some(i);
                } else {
                    self.planar.last_base = None;
                    // #24: down to 0% (items 0.1%); the base length is
                    // never 0.
                    let factor = (along / abs.base[axis]).max(self.scale_floor());
                    if self.handle_constrain {
                        // Ctrl: every axis by the same factor (HM-30).
                        params.delta = DVec3::splat(factor);
                    } else {
                        params.delta[axis] = factor;
                    }
                }
                self.publish_handle_params(tool, params);
            }
            ROT_REST => {
                let axis = self.handle_axis(2);
                let across = (point - center) - axis * (point - center).dot(axis);
                // Only the rest direction moves; any turn so far stays.
                if let Some(rest) = across.try_normalize() {
                    self.planar.rest = Some(rest);
                }
            }
            ROT_TARGET => {
                let axis = self.handle_axis(2);
                let rest = self.planar_rest();
                let across = (point - center) - axis * (point - center).dot(axis);
                let Some(to) = across.try_normalize() else {
                    return;
                };
                let mut angle = rest.cross(to).dot(axis).atan2(rest.dot(to)).to_degrees();
                if self.handle_constrain {
                    angle = self.handle_preferences.round_to_snap_angle(angle);
                }
                let mut params = base;
                params.delta = DVec3::new(0., 0., angle);
                self.publish_handle_params(tool, params);
            }
            AXIS_START | AXIS_END => {
                if id == AXIS_START {
                    self.planar.axis_start = Some(point);
                } else {
                    self.planar.axis_end = Some(point);
                }
            }
            _ => {}
        }
    }

    /// The release of a precision drag: both axis points set → the new
    /// rotation axis through the centre (Q4 guess).
    pub(crate) fn finish_precision_handle(&mut self, id: usize) {
        if !matches!(id, AXIS_START | AXIS_END) {
            return;
        }
        let (Some(a), Some(b)) = (self.planar.axis_start, self.planar.axis_end) else {
            self.status = "Planar Rotate Snap Axis: now drag the other axis handle".into();
            return;
        };
        let Some(frame) = PickedFrame::from_z(b - a) else {
            self.status = "The axis handles are on the same point; drag one away".into();
            return;
        };
        if self.editor.session.live.as_ref().is_some_and(|l| l.applied) {
            // Keep the turn so far as its own step, then turn about the new axis.
            self.run(Command::Drop);
            self.activate(Tool::Rotate);
        }
        let context = ActionContext {
            center: self.editor.action_context.center,
            axis: AxisOrientation::Element(frame),
        };
        if self.run_ok(Command::ActionContext(context)) {
            self.planar.rest = None;
            self.status =
                "Axis set from the two points; now drag the rest and rotate handles".into();
        }
    }
}
