// SPDX-FileCopyrightText: 2006 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/editors/sculpt_paint/mesh/sculpt.cc`
// (`sample_plane_normal_and_center`, `calc_brush_local_mat`) and
// `source/blender/editors/sculpt_paint/mesh/brushes/plane.cc`,
// `clay_strips.cc` at commit b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! The brush plane: sampling the surface under the brush for a normal and
//! a centre, and the brush's local frame for square and plane tips.

use glam::{DMat4, DVec3, DVec4};

/// Accumulates the points near the brush into an average normal and a
/// plane centre. Points facing away from the viewer are kept apart and used
/// only when nothing faces the viewer.
#[derive(Clone, Debug)]
pub struct PlaneSampler {
    location: DVec3,
    view: DVec3,
    normal_radius: f64,
    position_radius: f64,
    displacements: [DVec3; 2],
    displacement_count: [usize; 2],
    normals: [DVec3; 2],
    normal_count: [usize; 2],
}

/// What a `PlaneSampler` found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneSample {
    /// None when no point was near enough or the normals cancelled.
    pub normal: Option<DVec3>,
    pub center: DVec3,
}

impl PlaneSampler {
    /// `normal_radius` and `position_radius` are Blender's brush radius ×
    /// Normal Radius / Area Radius factors (0.5 by default); 0 skips that
    /// half.
    pub fn new(location: DVec3, view: DVec3, normal_radius: f64, position_radius: f64) -> Self {
        Self {
            location,
            view,
            normal_radius,
            position_radius,
            displacements: [DVec3::ZERO; 2],
            displacement_count: [0; 2],
            normals: [DVec3::ZERO; 2],
            normal_count: [0; 2],
        }
    }

    /// Adds a point at `distance` from the brush location.
    pub fn add(&mut self, position: DVec3, normal: DVec3, distance: f64) {
        let flip = usize::from(self.view.dot(normal) <= 0.);
        if self.position_radius > 0. && distance <= self.position_radius {
            let p = distance / self.position_radius;
            let factor = (3. - 2. * p) * p * p;
            self.displacements[flip] += (position - self.location) * factor;
            self.displacement_count[flip] += 1;
        }
        if self.normal_radius > 0. && distance <= self.normal_radius {
            let p = 1. - distance / self.normal_radius;
            let factor = (3. - 2. * p) * p * p;
            self.normals[flip] += normal * factor;
            self.normal_count[flip] += 1;
        }
    }

    pub fn finish(&self) -> PlaneSample {
        let mut center = self.location;
        for i in 0..2 {
            if self.displacement_count[i] != 0 {
                center += self.displacements[i] / self.displacement_count[i] as f64;
                break;
            }
        }
        let mut normal = None;
        for i in 0..2 {
            if self.normal_count[i] != 0 && self.normals[i] != DVec3::ZERO {
                normal = Some(self.normals[i].normalize());
                break;
            }
        }
        PlaneSample { normal, center }
    }
}

/// The brush's local frame as a world → local matrix: z along
/// `tip_normal`, y across the `motion` direction in the tangent plane, one
/// unit = `radius`, and the local y axis stretched by `tip_scale_x`. None
/// when the motion is parallel to the normal or zero.
pub fn local_frame(
    tip_normal: DVec3,
    motion: DVec3,
    location: DVec3,
    radius: f64,
    tip_scale_x: f64,
) -> Option<DMat4> {
    let y = tip_normal.cross(motion).normalize_or_zero();
    let z = tip_normal.normalize_or_zero();
    if y == DVec3::ZERO || z == DVec3::ZERO || (radius.is_nan() || radius <= 0.) {
        return None;
    }
    let x = y.cross(z).normalize();
    let to_world = DMat4::from_cols(
        (x * radius).extend(0.),
        (y * radius * tip_scale_x).extend(0.),
        (z * radius).extend(0.),
        DVec4::new(location.x, location.y, location.z, 1.),
    );
    let inv = to_world.inverse();
    inv.is_finite().then_some(inv)
}

/// Clay Strips' frame (`clay_strips::calc_local_matrix`) as world → local:
/// like `local_frame` but with x across the stroke and z pointing into the
/// surface unless the stroke is inverted (`flip`), so local z 0..1 is the
/// strip's depth below the plane.
pub fn clay_strips_frame(
    normal: DVec3,
    delta: DVec3,
    center: DVec3,
    radius: f64,
    tip_scale_x: f64,
    flip: bool,
) -> Option<DMat4> {
    let x = normal.cross(delta).normalize_or_zero();
    let y = normal.cross(x).normalize_or_zero();
    let z = normal.normalize_or_zero() * if flip { 1. } else { -1. };
    if x == DVec3::ZERO || y == DVec3::ZERO || (radius.is_nan() || radius <= 0.) {
        return None;
    }
    let to_world = DMat4::from_cols(
        (x * radius).extend(0.),
        (y * radius * tip_scale_x).extend(0.),
        (z * radius).extend(0.),
        DVec4::new(center.x, center.y, center.z, 1.),
    );
    let inv = to_world.inverse();
    inv.is_finite().then_some(inv)
}

/// The plane tip's distance for a point in the frame's local space (z up
/// from the plane): an ellipsoid (or, below roundness 1, a rounded box)
/// `height` tall above the plane and `depth` deep below it. 1 means
/// outside.
pub fn plane_distance(roundness: f64, height: f64, depth: f64, local: DVec3) -> f64 {
    let reach = if local.z >= 0. { height } else { depth };
    if reach == 0. {
        return 1.;
    }
    let z = local.z / reach;
    if roundness == 1. {
        return DVec3::new(local.x, local.y, z).length();
    }
    let (ax, ay) = (local.x.abs(), local.y.abs());
    if ax.max(ay) > 1. {
        return 1.;
    }
    let hardness = 1. - roundness;
    let rcp = if roundness != 0. { 1. / roundness } else { 0. };
    let ex = (ax - hardness).max(0.) * rcp;
    let ey = (ay - hardness).max(0.) * rcp;
    DVec3::new(ex, ey, z).length()
}

/// Plane brushes weaken points above the plane by `height` and below it by
/// `depth` (unless that side is 0 or 1).
pub fn height_depth_scale(height: f64, depth: f64, z: f64) -> f64 {
    if z > 0. && height != 0. && height != 1. {
        height
    } else if z < 0. && depth != 0. && depth != 1. {
        depth
    } else {
        1.
    }
}

/// Clay Strips' weight across the strip's depth: zero at the plane and one
/// radius below, strongest halfway (local z in radii).
pub fn clay_strips_depth_factor(z: f64) -> f64 {
    (z * (1. - z)).max(0.)
}

/// The move that puts `p` onto the plane through `point` with unit
/// `normal`.
pub fn to_plane(point: DVec3, normal: DVec3, p: DVec3) -> DVec3 {
    -normal * normal.dot(p - point)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_averages_nearby_normals_and_centres() {
        let mut s = PlaneSampler::new(DVec3::ZERO, DVec3::Z, 1., 1.);
        s.add(DVec3::new(0.5, 0., 0.2), DVec3::Z, 0.5);
        s.add(
            DVec3::new(-0.5, 0., 0.2),
            DVec3::new(0.2, 0., 1.).normalize(),
            0.5,
        );
        let r = s.finish();
        let n = r.normal.unwrap();
        assert!(n.z > 0.9 && n.x > 0.);
        // Centre weights grow with distance: (3 − 2p) p² at p = 0.5 is 0.5.
        assert!((r.center - DVec3::new(0., 0., 0.1)).length() < 1e-12);
    }

    #[test]
    fn sampler_uses_back_faces_only_when_nothing_faces_the_viewer() {
        let mut s = PlaneSampler::new(DVec3::ZERO, DVec3::Z, 1., 1.);
        s.add(DVec3::X * 0.1, -DVec3::Z, 0.1);
        assert_eq!(s.finish().normal, Some(-DVec3::Z));
        let front = DVec3::new(0., 1., 1.).normalize();
        s.add(DVec3::X * 0.1, front, 0.1);
        assert!((s.finish().normal.unwrap() - front).length() < 1e-12);
        // Out of reach adds nothing.
        let mut far = PlaneSampler::new(DVec3::ZERO, DVec3::Z, 1., 1.);
        far.add(DVec3::X * 3., DVec3::Z, 3.);
        assert_eq!(far.finish().normal, None);
        assert_eq!(far.finish().center, DVec3::ZERO);
    }

    #[test]
    fn local_frame_maps_the_brush_to_unit_space() {
        let m = local_frame(DVec3::Z, DVec3::X, DVec3::new(1., 2., 3.), 0.5, 1.).unwrap();
        let local = |p: DVec3| m.transform_point3(p);
        assert!(local(DVec3::new(1., 2., 3.)).length() < 1e-12);
        assert!((local(DVec3::new(1., 2., 3.5)) - DVec3::Z).length() < 1e-12);
        // Motion along x: x stays the motion axis in local space too.
        assert!((local(DVec3::new(1.5, 2., 3.)).x.abs() - 1.).abs() < 1e-12);
        assert!(local_frame(DVec3::Z, DVec3::Z, DVec3::ZERO, 1., 1.).is_none());
        assert!(local_frame(DVec3::Z, DVec3::ZERO, DVec3::ZERO, 1., 1.).is_none());
    }

    #[test]
    fn clay_strips_frame_points_into_the_surface() {
        let m = clay_strips_frame(DVec3::Z, DVec3::X, DVec3::ZERO, 1., 1., false).unwrap();
        assert!((m.transform_point3(-DVec3::Z * 0.5).z - 0.5).abs() < 1e-12);
        let f = clay_strips_frame(DVec3::Z, DVec3::X, DVec3::ZERO, 1., 1., true).unwrap();
        assert!((f.transform_point3(DVec3::Z * 0.5).z - 0.5).abs() < 1e-12);
        assert!(clay_strips_frame(DVec3::Z, DVec3::ZERO, DVec3::ZERO, 1., 1., false).is_none());
    }

    #[test]
    fn plane_tip_reaches_height_above_and_depth_below() {
        assert!((plane_distance(1., 1., 1., DVec3::new(0., 0., 0.5)) - 0.5).abs() < 1e-12);
        assert!((plane_distance(1., 0.5, 1., DVec3::new(0., 0., 0.25)) - 0.5).abs() < 1e-12);
        assert_eq!(plane_distance(1., 0., 1., DVec3::new(0., 0., 0.1)), 1.);
        assert_eq!(plane_distance(1., 1., 0., DVec3::new(0., 0., -0.1)), 1.);
        assert_eq!(plane_distance(0., 1., 1., DVec3::new(0.9, 0.9, 0.)), 0.);
        assert_eq!(height_depth_scale(0.5, 1., 0.2), 0.5);
        assert_eq!(height_depth_scale(0.5, 1., -0.2), 1.);
    }

    #[test]
    fn clay_strips_depth_and_plane_projection() {
        assert_eq!(clay_strips_depth_factor(0.), 0.);
        assert_eq!(clay_strips_depth_factor(0.5), 0.25);
        assert_eq!(clay_strips_depth_factor(-0.5), 0.);
        let m = to_plane(DVec3::ZERO, DVec3::Z, DVec3::new(1., 1., 0.3));
        assert!((m - DVec3::new(0., 0., -0.3)).length() < 1e-12);
    }
}
