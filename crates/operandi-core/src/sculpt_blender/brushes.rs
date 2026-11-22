// SPDX-FileCopyrightText: 2024 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/editors/sculpt_paint/mesh/brushes/`
// (`draw.cc`, `draw_sharp.cc`, `inflate.cc`, `clay.cc`, `clay_strips.cc`,
// `crease.cc`, `pinch.cc`, `plane.cc`, `layer.cc`, `grab.cc`, `thumb.cc`,
// `snake_hook.cc`, `rotate.cc`, `smooth.cc`) and `../sculpt.cc`
// (`grab_delta_get`, `sculpt_project_v3_normal_align`) at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Per-point vertex rules: each function returns the move (translation)
//! of one point for one dab. `factor` is the point's tip weight (falloff
//! curve × masks × pressure) and `strength` Blender's signed brush strength
//! (`bstrength`); a caller applies `position += translation`.

use glam::{DMat3, DVec3};

/// Draw: along the brush normal by `radius × strength`.
pub fn draw(normal: DVec3, radius: f64, strength: f64, factor: f64) -> DVec3 {
    normal * (radius * strength * factor)
}

/// Inflate: along each point's own normal by `radius × strength`.
pub fn inflate(vertex_normal: DVec3, radius: f64, strength: f64, factor: f64) -> DVec3 {
    vertex_normal * (radius * strength * factor)
}

/// Smooth (one iteration): toward the neighbours' average.
pub fn smooth(position: DVec3, neighbour_average: DVec3, factor: f64) -> DVec3 {
    (neighbour_average - position) * factor
}

/// Smooth's strength as iterations: whole passes at 1, then the rest, at
/// most four passes in all (strengths above 1 clamp).
pub fn smooth_iterations(strength: f64) -> Vec<f64> {
    const MAX: f64 = 4.;
    let s = strength.clamp(0., 1.);
    let count = (s * MAX) as usize;
    let last = MAX * (s - count as f64 / MAX);
    let mut out = vec![1.; count];
    out.push(last);
    out
}

/// Grab, Thumb and Nudge: carry `offset` by the weight.
pub fn grab(offset: DVec3, factor: f64) -> DVec3 {
    offset * factor
}

/// Thumb and Nudge's offset: the pointer's move flattened into the brush
/// plane.
pub fn tangent_offset(normal: DVec3, delta: DVec3) -> DVec3 {
    normal.cross(delta).cross(normal)
}

/// Grab's silhouette mask: only points whose original normal leans the way
/// the grab pulls (relative to the press normal) move.
pub fn silhouette_factor(
    initial_normal: DVec3,
    grab_delta: DVec3,
    offset: DVec3,
    vertex_normal: DVec3,
) -> f64 {
    let sign = initial_normal.dot(grab_delta).signum();
    let test = offset.normalize_or_zero() * sign;
    test.dot(vertex_normal).max(0.)
}

/// `grab_delta_get` with Normal Weight: blends the pointer's move toward the
/// brush normal, scaled so the pull keeps up with the pointer when the
/// normal leans toward the viewer.
pub fn normal_weighted_delta(delta: DVec3, normal: DVec3, view: DVec3, weight: f64) -> DVec3 {
    if weight <= 0. {
        return delta;
    }
    let along = normal.dot(delta);
    let in_view = normal - view * normal.dot(view);
    let s = in_view.dot(normal).abs();
    let view_scale = if s > f32::EPSILON as f64 { 1. / s } else { 1. };
    delta * (1. - weight) + normal * (along * weight * view_scale)
}

/// Crease and Blob's pinch strength: `crease_pinch²` relative to the brush
/// alpha, signed (Blob pushes out where Crease pulls in).
pub fn crease_strength(crease_pinch: f64, alpha: f64, strength: f64, blob: bool) -> f64 {
    let mut correction = crease_pinch * crease_pinch;
    if alpha > 0. {
        correction /= alpha * alpha;
    }
    strength.abs() * correction * if blob { -1. } else { 1. }
}

/// Crease / Blob: pull toward the brush centre in the brush plane (by
/// `pinch`, from `crease_strength`) plus Draw's `offset`. `tube_view`
/// projects the pull flat to the view for tube falloff.
pub fn crease(
    position: DVec3,
    location: DVec3,
    normal: DVec3,
    offset: DVec3,
    pinch: f64,
    factor: f64,
    tube_view: Option<DVec3>,
) -> DVec3 {
    let mut t = location - position;
    if let Some(v) = tube_view {
        t -= v * t.dot(v);
    }
    t *= factor * pinch;
    t -= normal * t.dot(normal);
    t + offset * factor
}

/// Pinch's two axes: across the stroke in the plane, and the normal. None
/// before the pointer has moved.
pub fn pinch_axes(normal: DVec3, delta: DVec3) -> Option<[DVec3; 2]> {
    let x = normal.cross(delta).normalize_or_zero();
    let z = normal.normalize_or_zero();
    (x != DVec3::ZERO && z != DVec3::ZERO).then_some([x, z])
}

/// Pinch: toward `location` along each of `axes` (unit vectors), by the
/// weight (signed strength already in `factor`). Blender's Pinch passes
/// `pinch_axes`; Operandi's pinches pass the axes they slide along.
pub fn pinch(position: DVec3, location: DVec3, axes: &[DVec3], factor: f64) -> DVec3 {
    let d = location - position;
    axes.iter().map(|a| *a * d.dot(*a)).sum::<DVec3>() * factor
}

/// Clay: onto the plane through `plane_point` (the surface moved out by the
/// plane offset), by `|strength| × factor`.
pub fn clay(
    plane_point: DVec3,
    normal: DVec3,
    position: DVec3,
    strength: f64,
    factor: f64,
) -> DVec3 {
    super::plane::to_plane(plane_point, normal, position) * (strength.abs() * factor)
}

/// Plane (Flatten, Fill, Scrape): toward the plane by the point's height
/// above it (`local_z`, in radii, from the brush's local frame).
pub fn plane(normal: DVec3, radius: f64, strength: f64, local_z: f64, factor: f64) -> DVec3 {
    -normal * (radius * strength * local_z * factor)
}

/// Layer: one dab's step of a point's displacement (−1..1 of the layer
/// height). Ctrl (`invert`) wears the layer back toward zero.
pub fn layer_step(displacement: f64, factor: f64, strength: f64, invert: bool) -> f64 {
    let d = if invert {
        let step = (factor * strength * displacement).abs();
        displacement + if displacement > 0. { -step } else { step }
    } else {
        displacement + factor * strength * (1.05 - displacement.abs())
    };
    d.clamp(-1., 1.)
}

/// Layer: toward the original position raised along the original normal by
/// `height × displacement`.
pub fn layer(
    original: DVec3,
    original_normal: DVec3,
    position: DVec3,
    height: f64,
    displacement: f64,
    factor: f64,
) -> DVec3 {
    (original + original_normal * (height * displacement) - position) * factor
}

/// Snake Hook's pinch around the pulled tip (`crease_pinch` 0.5 is off):
/// below 0.5 it pulls the sides in toward the pull, above it puffs them
/// out.
pub fn snake_hook_pinch(
    position: DVec3,
    location: DVec3,
    delta: DVec3,
    radius: f64,
    crease_pinch: f64,
    factor: f64,
) -> DVec3 {
    if crease_pinch == 0.5 || (radius.is_nan() || radius <= 0.) {
        return DVec3::ZERO;
    }
    let pinch = 2. * (0.5 - crease_pinch) * delta.length() / radius;
    let len_sq = delta.length_squared();
    let project = |v: DVec3| {
        if len_sq > f32::EPSILON as f64 {
            v - delta * (v.dot(delta) / len_sq)
        } else {
            v
        }
    };
    let d = project(position - location + delta);
    let mut fade = pinch * factor;
    if pinch > 0. {
        fade *= (d.length() / radius).min(1.).powi(2);
    }
    d - d * (1. + fade)
}

/// Rotate: `position` turned about `axis` through `center` by `angle`
/// (radians, already scaled by the weight).
pub fn rotate(position: DVec3, center: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let axis = axis.normalize_or_zero();
    if axis == DVec3::ZERO {
        return DVec3::ZERO;
    }
    let m = DMat3::from_axis_angle(axis, angle);
    m * (position - center) + center - position
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: DVec3, b: DVec3) -> bool {
        (a - b).length() < 1e-12
    }

    #[test]
    fn draw_and_inflate_scale_by_radius_strength_and_weight() {
        assert!(close(draw(DVec3::Z, 2., 0.5, 0.5), DVec3::Z * 0.5));
        assert!(close(inflate(DVec3::X, 2., -0.5, 1.), -DVec3::X));
        assert!(close(grab(DVec3::Y, 0.25), DVec3::Y * 0.25));
    }

    #[test]
    fn smooth_moves_toward_the_average_in_iterations() {
        assert!(close(smooth(DVec3::Z, DVec3::ZERO, 0.5), -DVec3::Z * 0.5));
        assert_eq!(smooth_iterations(0.25), vec![1., 0.]);
        assert_eq!(smooth_iterations(0.6).len(), 3);
        assert!((smooth_iterations(0.6)[2] - 0.4).abs() < 1e-12);
        assert_eq!(smooth_iterations(2.), vec![1., 1., 1., 1., 0.]);
    }

    #[test]
    fn thumb_offset_stays_in_the_plane() {
        let t = tangent_offset(DVec3::Z, DVec3::new(1., 0., 1.));
        assert!(close(t, DVec3::X));
    }

    #[test]
    fn silhouette_masks_points_facing_away() {
        let f = silhouette_factor(DVec3::Z, DVec3::Z, DVec3::Z, DVec3::Z);
        assert_eq!(f, 1.);
        assert_eq!(
            silhouette_factor(DVec3::Z, DVec3::Z, DVec3::Z, -DVec3::Z),
            0.
        );
    }

    #[test]
    fn normal_weight_bends_the_grab_toward_the_normal() {
        let d = DVec3::new(1., 0., 1.);
        assert_eq!(normal_weighted_delta(d, DVec3::Z, DVec3::Y, 0.), d);
        let full = normal_weighted_delta(d, DVec3::Z, DVec3::Y, 1.);
        assert!(close(full, DVec3::Z));
    }

    #[test]
    fn crease_pulls_in_across_and_pushes_along_the_normal() {
        let s = crease_strength(0.5, 1., 1., false);
        assert_eq!(s, 0.25);
        assert_eq!(crease_strength(0.5, 0.5, 1., true), -1.);
        let t = crease(
            DVec3::new(0.5, 0., 0.),
            DVec3::ZERO,
            DVec3::Z,
            DVec3::Z * 0.1,
            s,
            1.,
            None,
        );
        assert!(t.x < 0. && (t.z - 0.1).abs() < 1e-12);
    }

    #[test]
    fn pinch_pulls_toward_the_centre_line() {
        let axes = pinch_axes(DVec3::Z, DVec3::Y).unwrap();
        // Across the stroke (x) and the normal pull in; along it (y) not.
        let t = pinch(DVec3::new(0.5, 0.3, 0.2), DVec3::ZERO, &axes, 1.);
        assert!(close(t, DVec3::new(-0.5, 0., -0.2)));
        assert!(pinch_axes(DVec3::Z, DVec3::ZERO).is_none());
    }

    #[test]
    fn clay_and_plane_move_toward_the_plane() {
        let c = clay(DVec3::Z * 0.1, DVec3::Z, DVec3::ZERO, -0.5, 1.);
        assert!(close(c, DVec3::Z * 0.05));
        let p = plane(DVec3::Z, 2., 0.5, 0.2, 1.);
        assert!(close(p, -DVec3::Z * 0.2));
    }

    #[test]
    fn layer_rises_to_the_height_and_no_further() {
        let mut d = 0.;
        for _ in 0..100 {
            d = layer_step(d, 1., 1., false);
        }
        assert_eq!(d, 1.);
        assert!(layer_step(1., 1., 0.5, true) < 1.);
        let t = layer(DVec3::ZERO, DVec3::Z, DVec3::ZERO, 0.2, 1., 1.);
        assert!(close(t, DVec3::Z * 0.2));
    }

    #[test]
    fn snake_hook_pinch_is_off_at_half() {
        let p = DVec3::new(0.3, 0., 0.);
        assert_eq!(
            snake_hook_pinch(p, DVec3::ZERO, DVec3::Y, 1., 0.5, 1.),
            DVec3::ZERO
        );
        // Below half pinches in toward the pull, above puffs out.
        let t = snake_hook_pinch(p, DVec3::ZERO, DVec3::Y, 1., 0., 1.);
        assert!(t.x < 0. && t.y.abs() < 1e-12);
        let t = snake_hook_pinch(p, DVec3::ZERO, DVec3::Y, 1., 1., 1.);
        assert!(t.x > 0.);
    }

    #[test]
    fn rotate_turns_about_the_axis() {
        let t = rotate(DVec3::X, DVec3::ZERO, DVec3::Z, std::f64::consts::FRAC_PI_2);
        assert!(close(DVec3::X + t, DVec3::Y));
        assert_eq!(rotate(DVec3::X, DVec3::ZERO, DVec3::ZERO, 1.), DVec3::ZERO);
    }
}
