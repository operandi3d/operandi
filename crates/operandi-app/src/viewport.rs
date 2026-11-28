use super::*;
use theme::viewport as palette;
mod base_memo;
mod dense_box;
mod extend_handles;
mod falloff_drag;
mod gesture;
mod handle_drag;
mod handles;
mod lasso_select;
mod nav_cursor;
mod numeric_drag;
mod pick;
pub(super) mod plane_handles;
mod pointer_input;
pub(crate) mod precision_handles;
mod select_click;
mod sketch_pen;
mod view_ui;
mod work_plane;
pub(crate) use crate::picking::PickCandidates;
pub(crate) use base_memo::{BaseMemo, SymmetryMemo};
use dense_box::{DENSE_BOX_POLYGONS, box_corner_strokes, box_strokes, too_dense_for_wire};
use operandi_core::click_select::{self, Click, ClickHow};
use operandi_core::tool_card::{HandleAnchor, HandleAxes, HandleKind};
use plane_handles::is_plane_handle;
#[cfg(test)]
pub(crate) use view_ui::scale_look_handle;
pub(crate) mod bend_view;
pub(crate) mod click_start_view;
pub(crate) mod falloff_screen_view;
pub(crate) mod falloff_view;
pub(crate) mod meshop_handles;
pub(crate) mod tweak_view;
pub(crate) mod twist_bevel_handles;

#[cfg(test)]
thread_local! {
    pub(crate) static PROJECTED_VERTEX_COUNTS: std::cell::Cell<Option<[usize; 2]>> =
        const { std::cell::Cell::new(None) };
}

/// Smallest half-width of the Linear falloff wedge's open end, in points,
/// so the wedge stays visible when the falloff points at the camera.
const FALLOFF_WEDGE_MIN_HALF_WIDTH: f32 = 6.;
/// Original-compatible A1: handles have a size in the scene.
const WORLD_SIZE_HANDLES: &str = "A1";

/// Screen outline of a Linear falloff: a point at the Start handle `a`
/// widening to an open end across the End handle `b`, about a quarter of
/// the on-screen length wide. Returns `[tip, left, right]`.
fn linear_falloff_wedge(a: Pos2, b: Pos2) -> [Pos2; 3] {
    let d = b - a;
    let length = d.length();
    // Seen end-on the direction is lost; any perpendicular will do.
    let across = if length > 1e-3 {
        vec2(-d.y, d.x) / length
    } else {
        vec2(1., 0.)
    };
    let half = (length * 0.125).max(FALLOFF_WEDGE_MIN_HALF_WIDTH);
    [a, b + across * half, b - across * half]
}

/// Filled arrowhead ending at `tip`, pointing from `from`.
fn handle_arrow_head(painter: &egui::Painter, from: Pos2, tip: Pos2, color: Color32) {
    let d = tip - from;
    if d.length() < 1e-3 {
        return;
    }
    let dir = d.normalized();
    let side = vec2(-dir.y, dir.x) * 4.5;
    let base = tip - dir * 11.;
    painter.add(Shape::convex_polygon(
        vec![tip, base + side, base - side],
        color,
        Stroke::NONE,
    ));
}

pub(crate) const SCALE_UNIFORM_HANDLE: usize = 3;
/// Pseudo-handle id for dragging an Extrude off its gizmo (free movement).
const EXTRUDE_FREE_HANDLE: usize = 9;
/// Pseudo-handle id for dragging Vertex Bevel anywhere in the viewport.
const VERTEX_BEVEL_FREE_HANDLE: usize = 10;
/// The center handle of Move, Rotate and Transform: free movement in the
/// view plane, or free rotation (Jason, 2026-11-03; provisional).
pub(super) const FREE_HANDLE: usize = 11;
/// How far (points) the second press of an edge double-click may land from
/// the first; egui's own click tolerance.
pub(super) const EDGE_DOUBLE_CLICK_DISTANCE: f32 = 6.;
/// Pseudo-handle id for a drag anywhere off the handles (Jason, 2026-11-04):
/// up/down acts like the tool's move arrow (its first handle), left/right
/// like its scale widget (its second handle). See `update_off_handle`.
pub(super) const OFF_HANDLE: usize = 12;
/// Edge Extend's Local handles (Inset, Shift); only Edge Extend uses these
/// ids, as Extrude and Vertex Bevel use 9 and 10 for theirs.
pub(super) const EXTEND_INSET_HANDLE: usize = 9;
pub(super) const EXTEND_SHIFT_HANDLE: usize = 10;
/// Tools with Transform's handle layout: 0..3 move, 3..6 rotate rings,
/// 6..9 scale (Edge Extend shows the sets its Handles option picks).
pub(super) fn combined_handles(tool: Tool) -> bool {
    tool.card().handles.kind == HandleKind::Combined
}
/// Pointer travel (points) before a Ctrl haul picks its axis (HM-12).
const HAUL_AXIS_PICK_PIXELS: f32 = 4.;

/// The one affine map taking every point of `from` to the same point of
/// `to`, if there is one: solved from four spread-out points, then checked
/// against all of them (a cheap pass over the points, once per frame).
/// `None` when the points don't span a volume or any point moved
/// differently. A mirroring map is refused too (the cached triangles would
/// face the wrong way).
#[cfg(test)]
pub(super) fn whole_mesh_map(from: &[DVec3], to: &[DVec3]) -> Option<glam::DMat4> {
    whole_mesh_map_from(from, to, whole_mesh_basis(from)?)
}

/// Three points of `from` that span its volume with the first point, for
/// `whole_mesh_map_from`; `None` when the points don't span one. Depends only
/// on `from`, so a drag finds them once (`BaseMemo`).
pub(super) fn whole_mesh_basis(from: &[DVec3]) -> Option<[usize; 3]> {
    if from.len() < 4 {
        return None;
    }
    let farthest = |score: &dyn Fn(DVec3) -> f64| {
        (0..from.len()).max_by(|&a, &b| score(from[a]).total_cmp(&score(from[b])))
    };
    let p0 = from[0];
    let i1 = farthest(&|p| p.distance_squared(p0))?;
    let e1 = (from[i1] - p0).normalize_or_zero();
    let i2 = farthest(&|p| (p - p0).reject_from_normalized(e1).length_squared())?;
    let e2 = (from[i2] - p0).cross(e1).normalize_or_zero();
    let i3 = farthest(&|p| (p - p0).dot(e2).abs())?;
    let basis = glam::DMat3::from_cols(from[i1] - p0, from[i2] - p0, from[i3] - p0);
    let size = from[i1].distance(p0);
    (size > 0. && basis.determinant().abs() > 1e-9 * size * size * size).then_some([i1, i2, i3])
}

/// `whole_mesh_map` with the spanning points already found.
pub(super) fn whole_mesh_map_from(
    from: &[DVec3],
    to: &[DVec3],
    [i1, i2, i3]: [usize; 3],
) -> Option<glam::DMat4> {
    if from.len() != to.len() || from.len() <= i1.max(i2).max(i3) {
        return None;
    }
    let p0 = from[0];
    let basis = glam::DMat3::from_cols(from[i1] - p0, from[i2] - p0, from[i3] - p0);
    let size = from[i1].distance(p0);
    let q0 = to[0];
    let moved = glam::DMat3::from_cols(to[i1] - q0, to[i2] - q0, to[i3] - q0);
    let linear = moved * basis.inverse();
    if !linear.is_finite() || linear.determinant() <= 0. {
        return None;
    }
    let map = glam::DMat4::from_mat3_translation(linear, q0 - linear * p0);
    let tolerance = 1e-9 * size.max(1.);
    let close = |p: DVec3, q: DVec3| p.distance_squared(q) <= tolerance * tolerance;
    // The four spanning points stayed put: either nothing moved (no map
    // needed) or only some points did, which the full pass below would find
    // at the cost of reading every point each frame. Drawing the live mesh
    // itself is right in both cases, and patches only what moved.
    if [0, i1, i2, i3].iter().all(|&i| close(from[i], to[i])) {
        return None;
    }
    from.iter()
        .zip(to)
        .all(|(&p, &q)| close(map.transform_point3(p), q))
        .then_some(map)
}

/// Pixels of right-drag per Selection falloff Step.
pub(crate) const FALLOFF_STEP_PIXELS: f32 = 20.;
/// A right-press that moves less than this is a click (adds one Step).
pub(crate) const FALLOFF_CLICK_PIXELS: f32 = 4.;

/// Tools whose plain drag away from the handles goes through `OFF_HANDLE`
/// (`Handles::drag_anywhere` on the tool's card). The others have a gesture
/// of their own (transforms, primitives, Extrude, Vertex Bevel, Tweak, Poly
/// Haul, Sketch Extrude) or place points by clicking (Create).
pub(super) fn off_handle_drag(tool: Tool) -> bool {
    tool.card().handles.drag_anywhere
}

fn selected_vertex_mask(count: usize, selected: &[u32]) -> Vec<bool> {
    let mut mask = vec![false; count];
    for &index in selected {
        if let Some(value) = mask.get_mut(index as usize) {
            *value = true;
        }
    }
    mask
}

#[cfg(test)]
mod falloff_wedge_tests {
    use super::{FALLOFF_WEDGE_MIN_HALF_WIDTH, linear_falloff_wedge};
    use crate::egui::pos2;
    #[test]
    fn wedge_starts_at_a_point_and_opens_a_quarter_length_across_the_end() {
        let (a, b) = (pos2(10., 20.), pos2(110., 20.));
        let [tip, left, right] = linear_falloff_wedge(a, b);
        assert_eq!(tip, a);
        // The open end crosses the End handle at right angles.
        assert_eq!((left + (right - left) / 2.), b);
        assert!((left - right).dot(b - a).abs() < 1e-4);
        assert!(((left - right).length() - 25.).abs() < 1e-4);
    }
    #[test]
    fn wedge_stays_visible_when_short_or_seen_end_on() {
        for b in [pos2(13., 20.), pos2(10., 20.)] {
            let [_, left, right] = linear_falloff_wedge(pos2(10., 20.), b);
            let width = (left - right).length();
            assert!((width - 2. * FALLOFF_WEDGE_MIN_HALF_WIDTH).abs() < 1e-4);
            assert!(left.is_finite() && right.is_finite());
        }
    }
}

#[cfg(test)]
mod pen_review_tests {
    use super::selected_vertex_mask;
    #[test]
    fn pen_rev03_point_membership_is_linear_and_preserves_mixed_selection() {
        for n in [1_000, 10_000, 100_000] {
            let selected: Vec<u32> = (0..n as u32).step_by(2).rev().collect();
            let mask = selected_vertex_mask(n, &selected);
            assert_eq!(mask.len(), n);
            assert!(
                mask.iter()
                    .enumerate()
                    .all(|(i, &value)| value == (i % 2 == 0))
            );
            assert_eq!(selected[0], n as u32 - 2);
        }
    }
}

/// VMT-30's colours at 0, +1 and −1 (gamma-encoded 0–255); the 3D
/// view's weight tint uses the same (gpu_scene.rs `tint_wgsl`).
pub(crate) const WEIGHT_ZERO: [f64; 3] = [128., 140., 128.];
pub(crate) const WEIGHT_POSITIVE: [f64; 3] = [255., 0., 0.];
pub(crate) const WEIGHT_NEGATIVE: [f64; 3] = [0., 0., 255.];

/// VMT-30 weight shading: grey-green at 0, blending to red at +1 and blue
/// at −1, clamped beyond ±1 (measured, VM-CASE-16).
#[cfg(test)]
pub(crate) fn weight_colour(w: f64) -> Color32 {
    let zero = WEIGHT_ZERO;
    let end = if w >= 0. {
        WEIGHT_POSITIVE
    } else {
        WEIGHT_NEGATIVE
    };
    let t = if w.is_finite() { w.abs().min(1.) } else { 0. };
    let c = |i: usize| (zero[i] + (end[i] - zero[i]) * t) as u8;
    Color32::from_rgb(c(0), c(1), c(2))
}
