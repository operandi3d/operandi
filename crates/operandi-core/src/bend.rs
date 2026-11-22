//! Bend (tutorial-gaps-2026-11-13, SPEC-BEND; tool `xfrm.bend`, preset
//! "Bend"): curls the target into an arc. Points further along the spine
//! turn further, up to the full Angle at the spine's end, so a straight
//! column becomes a section of a torus.
//!
//! v2 (bend-handles-2026-11-15): the pivot is the action centre (BN-02),
//! filled in by the editor, and the bend turns around the chosen action
//! axis X, Y or Z (BN-03).
//!
//! v2.1 measured (BN-40..44, BH-Q01..Q03 answered): the spine keeps its
//! length (a true arc of radius L / Angle); points beyond the spine's end
//! turn rigidly with it; points behind the centre stay, or mirror with Both
//! Sides; +Angle leans toward spine × axis (a turn of -Angle about the
//! axis). A falloff blends positions. Axis along the spine: BH-Q12.
use crate::{Mesh, Result, Scene};
use glam::{DMat3, DMat4, DVec3};
use std::collections::BTreeSet;

/// Bend's settings. `base` and `spine` are world space; `spine` is the
/// vector from the base to the spine's end (`spineX/Y/Z`, BN-21).
///
/// `base` is not a setting of Bend's own (BN-02): it is the action centre,
/// which the editor keeps here so previews, undo and symmetry see one
/// value. The panel shows it as Auto Action Center X/Y/Z.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BendOptions {
    /// Degrees at the spine's end (`angle`, radians in scripts).
    pub angle: f64,
    pub base: DVec3,
    pub spine: DVec3,
    /// Also bend the points behind the base, as a mirror image.
    pub both_sides: bool,
    /// BN-03 / G5: the action axis the bend turns around, 0/1/2 for X/Y/Z
    /// of the action axes; one is always chosen (N01's Auto is gone).
    pub axis: usize,
    /// BN-34 Deform Space (Auto Action Center). Kept and shown; it has no
    /// effect on plain meshes (provisional, BH-Q09).
    pub deform_space: bool,
}
impl Default for BendOptions {
    /// The registry's defaults: Angle 0, Spine (0, 1, 0), Both Sides off;
    /// the axis Auto picks for that spine (Z).
    fn default() -> Self {
        Self {
            angle: 0.,
            base: DVec3::ZERO,
            spine: DVec3::Y,
            both_sides: false,
            axis: starting_axis(DVec3::Y, DMat3::IDENTITY),
            deform_space: true,
        }
    }
}

impl BendOptions {
    pub fn validate(&self) -> Result<()> {
        if !(self.angle.is_finite() && self.base.is_finite() && self.spine.is_finite()) {
            return Err("Bend values must be finite".into());
        }
        if self.spine.length() < 1e-9 {
            return Err("Bend's spine needs a length; move its end away from the base".into());
        }
        Ok(())
    }

    /// The direction the bend leans toward: square to the spine and to the
    /// chosen action axis (spine × axis), so the spine curls round that
    /// axis. An axis along the spine can't turn it; then the lean falls back
    /// to the action axis most square to the spine (the old Auto, BD-Q4).
    pub fn direction(&self, axes: DMat3) -> DVec3 {
        let u = self.spine.normalize_or_zero();
        if self.axis < 3
            && let Some(d) = u
                .cross(axes.col(self.axis).normalize_or_zero())
                .try_normalize()
        {
            return d;
        }
        auto_lean(u, axes)
    }

    /// BN-45 (BH-Q12, provisional): the chosen action axis runs along the
    /// spine, so it can't turn it. The bend then leaves the mesh unmoved and
    /// the status bar says [`AXIS_ALONG_SPINE`].
    pub fn axis_along_spine(&self, axes: DMat3) -> bool {
        let u = self.spine.normalize_or_zero();
        self.axis < 3 && u.cross(axes.col(self.axis).normalize_or_zero()).length() < 1e-9
    }

    /// The axis the bend turns around, in world space.
    pub fn turn_axis(&self, axes: DMat3) -> DVec3 {
        axes.col(self.axis.min(2)).normalize_or(DVec3::Y)
    }

    /// Where world point `p` goes. `direction` is from [`Self::direction`].
    pub fn bend_point(&self, p: DVec3, direction: DVec3) -> DVec3 {
        let length = self.spine.length();
        let theta_end = self.angle.to_radians();
        if theta_end.abs() < 1e-12 || length < 1e-12 {
            return p;
        }
        let u = self.spine / length;
        let d = direction;
        let rel = p - self.base;
        let s = rel.dot(u);
        if s < 0. && !self.both_sides {
            return p;
        }
        let x = rel.dot(d);
        let rest = rel - u * s - d * x;
        // A true arc: the spine curls onto a circle of radius L / Angle
        // round a centre beside the base, toward `d`.
        let radius = length / theta_end;
        let along = s.clamp(-length, length);
        let beyond = s - along;
        let theta = along / radius;
        let r = radius - x;
        let (sin, cos) = theta.sin_cos();
        let centre = d * radius;
        let tangent = d * sin + u * cos;
        self.base + rest + centre + (-d * cos + u * sin) * r + tangent * beyond
    }

    /// BN-24 (revised 2026-11-16): Bend fitted to a centre click at
    /// `centre`: the spine from [`fit_spine`] over the target's box (the
    /// selection, or the whole mesh with nothing selected), and the Axis
    /// on the work plane's normal (BN-30b).
    pub fn fitted(scene: &Scene, centre: DVec3, plane: u8, axes: DMat3) -> Result<Self> {
        let (lo, hi) = crate::falloff::selection_bounds(scene)?;
        Ok(Self {
            base: centre,
            spine: fit_spine(lo, hi, centre, plane),
            axis: click_axis(plane, axes),
            ..Self::default()
        })
    }
}

/// BN-24 (revised 2026-11-16): the spine for a centre `c` over the box
/// `lo..hi`. The candidates are the two world axes in work plane `plane`
/// (never the one toward the viewer, whatever the action axis is); along
/// each, the distance from `c` to the box's far side. The spine runs along
/// the candidate with the larger distance, toward that side, exactly that
/// long. A box with no size falls back to 1 m along the first candidate.
pub fn fit_spine(lo: DVec3, hi: DVec3, c: DVec3, plane: u8) -> DVec3 {
    let mut best = (0., DVec3::ZERO);
    for axis in [DVec3::X, DVec3::Z].map(|a| crate::mesh::orient(a, plane).abs()) {
        let (low, high, at) = (lo.dot(axis), hi.dot(axis), c.dot(axis));
        // The far side: whichever face of the box is further from `c`.
        let (far, toward) = if at - low >= high - at {
            (at - low, -axis)
        } else {
            (high - at, axis)
        };
        // Ties keep the first candidate (X before the other one).
        if far > best.0 + 1e-9 {
            best = (far, toward * far);
        }
    }
    if best.0 > 1e-9 {
        best.1
    } else {
        crate::mesh::orient(DVec3::X, plane).abs()
    }
}

/// BN-30b: the Axis a centre click sets, the action axis nearest the work
/// plane's normal (Z in a front view, Y looking down).
pub fn click_axis(plane: u8, axes: DMat3) -> usize {
    let normal = crate::mesh::orient(DVec3::Y, plane);
    (0..3)
        .max_by(|&a, &b| {
            axes.col(a)
                .dot(normal)
                .abs()
                .total_cmp(&axes.col(b).dot(normal).abs())
        })
        .unwrap_or(1)
}

/// BN-45 status bar text (BH-Q12, provisional).
pub const AXIS_ALONG_SPINE: &str = "The bend axis runs along the spine; pick another axis";

/// BN-47: Reset's values: Angle 0 and Spine (0, 1 m, 0), on the axis Auto
/// picks for that spine; the centre stays where it is.
pub fn reset(o: BendOptions, axes: DMat3) -> BendOptions {
    BendOptions {
        angle: 0.,
        spine: DVec3::Y,
        axis: starting_axis(DVec3::Y, axes),
        ..o
    }
}

/// Of the action axes, the one most square to the spine `u`, made exactly
/// square. Ties go to X, then Z, then Y, so a spine up Y leans toward +X.
fn auto_lean(u: DVec3, axes: DMat3) -> DVec3 {
    let mut best = (f64::INFINITY, DVec3::X);
    for axis in [axes.x_axis, axes.z_axis, axes.y_axis] {
        let along = axis.normalize_or_zero().dot(u).abs();
        if along < best.0 - 1e-9 {
            best = (along, axis);
        }
    }
    let d = best.1 - u * best.1.dot(u);
    d.try_normalize()
        .unwrap_or_else(|| u.any_orthonormal_vector())
}

/// The Axis button lit when Bend starts (BH-Q10, provisional: the axis the
/// old Auto turned around): the action axis nearest spine × lean. A spine
/// up Y starts on Z, so it curls toward +X; a spine along Z starts on Y.
pub fn starting_axis(spine: DVec3, axes: DMat3) -> usize {
    let u = spine.normalize_or(DVec3::Y);
    let turn = u.cross(auto_lean(u, axes));
    (0..3)
        .max_by(|&a, &b| {
            axes.col(a)
                .dot(turn)
                .abs()
                .total_cmp(&axes.col(b).dot(turn).abs())
        })
        .unwrap_or(2)
}

/// Bend `points` of `mesh` (local coordinates under `world`) with
/// `options`, leaning toward `direction` (world). With `weights` each point
/// moves its weight's share of the way (FAL-M-02).
pub fn bend_geometry(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    world: DMat4,
    options: &BendOptions,
    direction: DVec3,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    options.validate()?;
    if world.determinant().abs() < 1e-18 || !world.determinant().is_finite() {
        return Err("Bend needs an invertible item transform".into());
    }
    let local = world.inverse();
    let mut out = mesh.clone();
    for &v in points {
        let Some(p) = out.vertices.get_mut(v as usize) else {
            continue;
        };
        let w = weights.map_or(1., |w| w.get(v as usize).copied().unwrap_or(0.));
        if w == 0. {
            continue;
        }
        let at = world.transform_point3(*p);
        let bent = options.bend_point(at, direction);
        *p = local.transform_point3(at + (bent - at) * w);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(angle: f64) -> BendOptions {
        BendOptions {
            angle,
            base: DVec3::ZERO,
            spine: DVec3::new(0., 2., 0.),
            both_sides: false,
            axis: 2,
            deform_space: true,
        }
    }

    /// BD-01: 90° turns the top end horizontal; the centre line follows a
    /// quarter circle of radius 2 / (π/2).
    #[test]
    fn bd01_quarter_turn_is_a_quarter_circle() {
        let o = options(90.);
        let d = o.direction(DMat3::IDENTITY);
        assert!((d - DVec3::X).length() < 1e-12);
        let radius = 2. / std::f64::consts::FRAC_PI_2;
        let centre = DVec3::new(radius, 0., 0.);
        for i in 0..=10 {
            let p = o.bend_point(DVec3::new(0., 0.2 * i as f64, 0.), d);
            assert!(((p - centre).length() - radius).abs() < 1e-9, "{p}");
        }
        let top = o.bend_point(DVec3::new(0., 2., 0.), d);
        assert!((top - DVec3::new(radius, radius, 0.)).length() < 1e-9);
        // The end's tangent is horizontal: a point just past it moves in X.
        let past = o.bend_point(DVec3::new(0., 2.1, 0.), d);
        assert!((past - top - DVec3::new(0.1, 0., 0.)).length() < 1e-9);
    }

    /// BD-02: 180° turns the top end down; a half circle of radius 2/π.
    #[test]
    fn bd02_half_turn_points_down() {
        let o = options(180.);
        let d = o.direction(DMat3::IDENTITY);
        let radius = 2. / std::f64::consts::PI;
        let top = o.bend_point(DVec3::new(0., 2., 0.), d);
        assert!((top - DVec3::new(2. * radius, 0., 0.)).length() < 1e-9);
        let past = o.bend_point(DVec3::new(0., 2.1, 0.), d);
        assert!((past - top - DVec3::new(0., -0.1, 0.)).length() < 1e-9);
        // The spine keeps its length: points along it stay 0.2 m apart
        // along the arc.
        let a = o.bend_point(DVec3::new(0., 0.4, 0.), d);
        let b = o.bend_point(DVec3::new(0., 0.6, 0.), d);
        let chord = 2. * radius * (0.1 / radius).sin();
        assert!(((a - b).length() - chord).abs() < 1e-9);
    }

    /// BD-03: behind the base nothing moves with Both Sides off; with it on
    /// the negative side mirrors the positive one.
    #[test]
    fn bd03_both_sides_mirrors_behind_the_base() {
        let mut o = options(90.);
        let d = o.direction(DMat3::IDENTITY);
        let below = DVec3::new(0.1, -1., 0.);
        assert_eq!(o.bend_point(below, d), below);
        o.both_sides = true;
        let up = o.bend_point(DVec3::new(0.1, 1., 0.), d);
        let down = o.bend_point(below, d);
        assert!((down - DVec3::new(up.x, -up.y, up.z)).length() < 1e-9);
    }

    #[test]
    fn zero_angle_and_bad_spines() {
        let o = options(0.);
        let p = DVec3::new(0.3, 1.2, -0.4);
        assert_eq!(o.bend_point(p, DVec3::X), p);
        let flat = BendOptions {
            spine: DVec3::ZERO,
            ..options(30.)
        };
        assert!(flat.validate().is_err());
        assert!(
            BendOptions {
                angle: f64::NAN,
                ..options(30.)
            }
            .validate()
            .is_err()
        );
    }

    /// G5: the chosen Action Axis is the one the bend turns around: Z on
    /// an upright spine leans +X, X leans -Z, and Y (along the spine) falls
    /// back to the old lean.
    #[test]
    fn action_axis_sets_the_turn_axis() {
        let with = |axis| BendOptions {
            axis,
            ..options(90.)
        };
        let i = DMat3::IDENTITY;
        assert!((with(2).direction(i) - DVec3::X).length() < 1e-12);
        assert!((with(0).direction(i) + DVec3::Z).length() < 1e-12);
        assert!((with(1).direction(i) - DVec3::X).length() < 1e-12);
        // Turning about X: the spine's end lands in the YZ plane.
        let o = with(0);
        let end = o.bend_point(DVec3::new(0., 2., 0.), o.direction(i));
        assert!(end.x.abs() < 1e-12 && end.z < -0.5, "{end}");
    }

    /// BH-Q10 (provisional): Bend starts on the axis the old Auto turned
    /// around: Z for a spine up Y, Y for a spine along Z or X.
    #[test]
    fn starting_axis_follows_the_spine() {
        let i = DMat3::IDENTITY;
        assert_eq!(starting_axis(DVec3::Y, i), 2);
        assert_eq!(starting_axis(DVec3::Z * 3., i), 1);
        assert_eq!(starting_axis(-DVec3::X, i), 1);
    }

    /// Off-axis points keep their sideways offset (along the bend axis).
    #[test]
    fn points_beside_the_bend_plane_keep_their_offset() {
        let o = options(90.);
        let d = o.direction(DMat3::IDENTITY);
        let a = o.bend_point(DVec3::new(0., 1., 0.), d);
        let b = o.bend_point(DVec3::new(0., 1., 0.5), d);
        assert!((b - a - DVec3::new(0., 0., 0.5)).length() < 1e-9);
    }
}
