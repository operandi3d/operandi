//! Contour's Tool Properties and viewport drag (missing-tools-g4
//! G5-CONTOUR v1; core in operandi-core contour.rs).
//!
//! MT13-14: a drag in the viewport sets Start and End, each on the
//! background surface under the pointer (the work plane when nothing is
//! there); the slice direction is toward the viewer. With Fast Slice on
//! the cut comes on release; off, it follows the drag. Dragging again
//! replaces the last contour until the tool is dropped.
use super::*;
use crate::form::Unit;
use crate::live_fields::{check, pick, vec3, whole};
use crate::slice_view::SliceDrag;
use crate::tool_form::ToolForm;
use operandi_core::contour::{ContourKind, SlicingSide};

/// Contour's Tool Properties (the spec's settings, in its order).
pub(super) fn contour_form(p: &Parameters, f: &mut ToolForm<Parameters>) {
    vec3(
        f,
        "contour_start",
        "Start",
        Unit::Distance,
        "",
        |p| p.contour.start,
        |p, a, v| {
            p.contour.start[a] = v;
            p.contour.placed = true;
        },
    );
    vec3(
        f,
        "contour_end",
        "End",
        Unit::Distance,
        "",
        |p| p.contour.end,
        |p, a, v| {
            p.contour.end[a] = v;
            p.contour.placed = true;
        },
    );
    vec3(
        f,
        "contour_direction",
        "Slice Direction",
        Unit::Scalar,
        "",
        |p| p.contour.direction,
        |p, a, v| p.contour.direction[a] = v,
    );
    f.separator()
        .field(pick(
            "contour-type",
            "Contour Type",
            &[
                (ContourKind::Lines, "Lines"),
                (ContourKind::Curve, "Curve"),
                (ContourKind::Face, "Face"),
            ],
            |p| p.contour.kind,
            |p, k| p.contour.kind = k,
        ))
        .field(check(
            "contour_automatic",
            "Automatic",
            |p| p.contour.automatic,
            |p, on| p.contour.automatic = on,
        ))
        .field(
            whole(
                "contour_segments",
                "Segments",
                (1., 10_000.),
                "Segments must be a whole number from 1 to 10000",
                |p| f64::from(p.contour.segments),
                |p, n| p.contour.segments = n,
            )
            .enabled_if(
                !p.contour.automatic,
                "Automatic is on: the surface sets the count",
            ),
        )
        .field(check(
            "contour_uniform",
            "Uniform Span",
            |p| p.contour.uniform,
            |p, on| p.contour.uniform = on,
        ))
        .field(pick(
            "contour-side",
            "Slicing Side",
            &[
                (SlicingSide::Positive, "Positive"),
                (SlicingSide::Negative, "Negative"),
                (SlicingSide::Both, "Both"),
            ],
            |p| p.contour.side,
            |p, s| p.contour.side = s,
        ))
        .field(check(
            "contour_cull",
            "Cull Occluded Open Curves",
            |p| p.contour.cull,
            |p, on| p.contour.cull = on,
        ))
        .field(check(
            "contour_intersect",
            "Intersect",
            |p| p.contour.intersect,
            |p, on| p.contour.intersect = on,
        ))
        .field(check(
            "contour_select",
            "Select New Polygons",
            |p| p.contour.select_new,
            |p, on| p.contour.select_new = on,
        ))
        .separator()
        .field(check(
            "contour_angle_snap",
            "Angle Snap",
            |p| p.contour.angle_snap,
            |p, on| p.contour.angle_snap = on,
        ))
        .field(
            crate::tool_form::number(
                "contour_angle",
                "Angle",
                Unit::Angle,
                |p: &Parameters| p.contour.angle,
                |p, v| {
                    if !(v > 0. && v <= 180.) {
                        return Err("Angle must be more than 0° and at most 180°".into());
                    }
                    p.contour.angle = v;
                    Ok(())
                },
            )
            .enabled_if(p.contour.angle_snap, "Turn Angle Snap on first"),
        )
        .field(check(
            "contour_fast",
            "Fast Slice",
            |p| p.contour.fast,
            |p, on| p.contour.fast = on,
        ));
    f.note(if p.contour.placed {
        "Drag again to replace the contour; Drop keeps it."
    } else {
        "Drag across a background mesh: the cut under the line becomes a new curve in the active mesh."
    });
}

impl Operandi {
    /// The world point under `pos`: on a background surface, else on the
    /// work plane.
    fn contour_point(&self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (origin, dir) = self.camera.ray(pos, r);
        self.editor.background_hit(origin, dir).or_else(|| {
            self.camera
                .plane_point(pos, r, self.actual_plane(), DVec3::ZERO)
        })
    }

    /// A press with Contour: the line starts here (MT13-14).
    pub(super) fn contour_press(
        &mut self,
        mut p: Parameters,
        pos: Pos2,
        r: Rect,
    ) -> Option<SliceDrag> {
        let at = self.contour_point(pos, r)?;
        let (_, _, _, forward) = self.camera.basis();
        let c = &mut p.contour;
        c.start = at;
        c.end = at;
        // Toward the viewer, so Positive and the culled piece face it.
        c.direction = -forward;
        c.placed = false;
        self.slice_publish(p);
        Some(SliceDrag::Contour { from: pos })
    }

    /// A drag frame: the line's end follows; with Fast Slice off the cut
    /// does too. Angle Snap turns the line on screen in steps.
    pub(super) fn contour_drag(&mut self, from: Pos2, pos: Pos2, r: Rect) {
        let Some(mut p) = self.slice_params() else {
            return;
        };
        let mut to = pos;
        if p.contour.angle_snap && p.contour.angle > 0. {
            let v = pos - from;
            let step = p.contour.angle.to_radians() as f32;
            let a = (v.y.atan2(v.x) / step).round() * step;
            to = from + vec2(a.cos(), a.sin()) * v.length();
        }
        let Some(end) = self.contour_point(to, r) else {
            return;
        };
        p.contour.end = end;
        p.contour.placed = !p.contour.fast;
        self.slice_publish(p);
    }

    /// Release: the cut is made (MT13-14).
    pub(super) fn contour_release(&mut self) {
        let Some(mut p) = self.slice_params() else {
            return;
        };
        if p.contour.start.distance(p.contour.end) < 1e-9 {
            return;
        }
        p.contour.placed = true;
        self.slice_publish(p);
    }

    /// The drawn line over the viewport.
    pub(super) fn draw_contour(&self, painter: &egui::Painter, r: Rect, p: &Parameters) {
        let (Some((a, _)), Some((b, _))) = (
            self.camera.project(p.contour.start, r),
            self.camera.project(p.contour.end, r),
        ) else {
            return;
        };
        painter.line_segment(
            [a, b],
            egui::Stroke::new(1.5_f32, theme::viewport::slice_bar()),
        );
    }
}
