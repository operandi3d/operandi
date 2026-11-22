//! Twist Bevel (specifications/twist-bevel-2026-11-15, v1.1): Polygon
//! Bevel's ring builder with each ring placed on a curved, twisting spine
//! instead of a straight line. Shift is the spine's length, Inset (with
//! Profile) tapers each ring, Rotation twists the rings and Bend curls the
//! spine into an arc toward Bend Direction (spec section 3).
//!
//! The rings, walls, materials, polygon types, attributes and the cap
//! selection all come from Bevel (`bevel::bevel_placed`); this module only
//! works out each tube's frame and where each ring goes.
use crate::bevel::{BevelOptions, BevelResult, RingPlacer};
use crate::symmetry::{Side, Symmetry};
use crate::{Mesh, Result};
use glam::{DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// What the twist spins about (spec 3.4, TB-Q02).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TwistAxis {
    /// The spine's own tangent at each ring: the tube twists like a towel.
    #[default]
    Normal,
    /// The action axis's main direction, through each ring's spine point.
    ActionAxis,
}
impl TwistAxis {
    pub const ALL: [TwistAxis; 2] = [TwistAxis::Normal, TwistAxis::ActionAxis];
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::ActionAxis => "Action Axis",
        }
    }
}

/// The tool's properties, in panel order (spec section 2). Angles are in
/// degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TwistBevelOptions {
    pub shift: f64,
    pub inset: f64,
    pub segments: u32,
    pub group_polygons: bool,
    pub rotation: f64,
    pub rotation_axis: TwistAxis,
    pub profile: f64,
    pub profile_bias: f64,
    pub segment_spacing: f64,
    pub center_bias: f64,
    pub bend: f64,
    pub bend_direction: f64,
}
impl Default for TwistBevelOptions {
    fn default() -> Self {
        Self {
            shift: 0.,
            inset: 0.,
            // TB-Q01 (provisional): 4.
            segments: 4,
            group_polygons: true,
            rotation: 0.,
            rotation_axis: TwistAxis::Normal,
            profile: 0.,
            profile_bias: 0.,
            segment_spacing: 0.,
            center_bias: 0.,
            bend: 0.,
            bend_direction: 0.,
        }
    }
}

/// Segments range (TB-Q01 cap).
pub const TWIST_BEVEL_SEGMENTS: std::ops::RangeInclusive<u32> = 1..=256;
/// Bend clamp in degrees (TB-Q07).
pub const TWIST_BEVEL_BEND: f64 = 360.;

impl TwistBevelOptions {
    /// The options with every value in its range (spec section 2): Segments
    /// 1..256, Profile and Profile Bias -1..1, the spacing controls -2..2,
    /// Bend -360..360 and Bend Direction wrapped into 0..360.
    pub fn clamped(self) -> Self {
        Self {
            segments: self
                .segments
                .clamp(*TWIST_BEVEL_SEGMENTS.start(), *TWIST_BEVEL_SEGMENTS.end()),
            profile: self.profile.clamp(-1., 1.),
            profile_bias: self.profile_bias.clamp(-1., 1.),
            segment_spacing: self.segment_spacing.clamp(-2., 2.),
            center_bias: self.center_bias.clamp(-2., 2.),
            bend: self.bend.clamp(-TWIST_BEVEL_BEND, TWIST_BEVEL_BEND),
            bend_direction: self.bend_direction.rem_euclid(360.),
            ..self
        }
    }
    fn validate(&self) -> Result<()> {
        let values = [
            self.shift,
            self.inset,
            self.rotation,
            self.profile,
            self.profile_bias,
            self.segment_spacing,
            self.center_bias,
            self.bend,
            self.bend_direction,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err("Twist Bevel values must be finite".into());
        }
        if !TWIST_BEVEL_SEGMENTS.contains(&self.segments) {
            return Err("Twist Bevel Segments must be 1 to 256".into());
        }
        Ok(())
    }
    /// Shift and Inset both 0: nothing is made (spec section 6).
    pub fn is_zero(&self) -> bool {
        self.shift == 0. && self.inset == 0.
    }
}

/// Spec 3.7: the even ring position `u` bunched by Segment Spacing `s`
/// (toward the cap when positive) and then Center Bias `c` (toward the
/// middle when positive). Keeps 0 and 1 and never reorders.
pub fn ring_spacing(u: f64, s: f64, c: f64) -> f64 {
    let u = u.clamp(0., 1.);
    let u1 = 1. - (1. - u).powf(2f64.powf(s));
    let d = 2. * u1 - 1.;
    (0.5 + 0.5 * d.signum() * d.abs().powf(2f64.powf(c))).clamp(0., 1.)
}

/// Spec 3.3: a hump that is 0 at both ends and 1 at its peak, which Profile
/// Bias `b` slides to `0.5 + 0.45 b` (TB-Q12, provisional formula).
pub fn profile_bump(t: f64, b: f64) -> f64 {
    let t = t.clamp(0., 1.);
    let peak = 0.5 + 0.45 * b.clamp(-1., 1.);
    let t = if (peak - 0.5).abs() < 1e-12 {
        t
    } else {
        t.powf(0.5f64.ln() / peak.ln())
    };
    4. * t * (1. - t)
}

/// One tube's frame and its weighted amounts (spec 3.1, 3.2 and 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TubeFrame {
    /// Base centre C: the mean of the group's points.
    pub center: DVec3,
    /// Normal N: the mean of the group's polygon normals.
    pub normal: DVec3,
    /// Bend direction B: the side axis turned about N by Bend Direction.
    pub bend_dir: DVec3,
    /// Up axis U = N × B; the bend turns about it.
    pub up: DVec3,
    /// Half width W: the mean distance from C to the boundary points in
    /// the base plane.
    pub half_width: f64,
    /// Falloff-weighted Shift, Inset, Rotation (radians) and Bend (radians).
    pub shift: f64,
    pub inset: f64,
    pub rotation: f64,
    pub bend: f64,
    /// Profile and Profile Bias (not weighted).
    pub profile: f64,
    pub profile_bias: f64,
    /// The twist axis for Action Axis (unit), or `None` for Normal.
    pub twist_axis: Option<DVec3>,
}
impl TubeFrame {
    /// Spine point P(t) (spec 3.2).
    pub fn spine(&self, t: f64) -> DVec3 {
        let length = self.shift;
        if self.bend.abs() < 1e-9 {
            return self.center + self.normal * length * t;
        }
        let radius = length / self.bend;
        let a = self.bend * t;
        self.center + self.normal * radius * a.sin() + self.bend_dir * radius * (1. - a.cos())
    }
    /// Spine tangent T(t).
    pub fn tangent(&self, t: f64) -> DVec3 {
        self.bend_turn(t) * self.normal
    }
    /// The bend's turn at t: the base frame rotated by θ·t about U.
    fn bend_turn(&self, t: f64) -> DQuat {
        DQuat::from_axis_angle(self.up, self.bend * t)
    }
    /// The twist about the action axis at t (identity for Normal).
    fn axis_turn(&self, t: f64) -> DQuat {
        match self.twist_axis {
            Some(axis) => DQuat::from_axis_angle(axis, self.rotation * t),
            None => DQuat::IDENTITY,
        }
    }
    /// The signed inset at t (spec 3.3).
    pub fn inset_at(&self, t: f64) -> f64 {
        self.inset * t - self.profile * self.half_width * profile_bump(t, self.profile_bias)
    }
    /// Where a base point with unit inset offset `unit_inset` sits at t
    /// (spec 3.5).
    pub fn place(&self, t: f64, point: DVec3, unit_inset: DVec3) -> DVec3 {
        let mut offset = point - self.center + unit_inset * self.inset_at(t);
        if self.twist_axis.is_none() {
            offset = DQuat::from_axis_angle(self.normal, self.rotation * t) * offset;
        }
        let spine = self.spine(t);
        spine + self.axis_turn(t) * (self.bend_turn(t) * offset)
    }
    /// The orientation of the ring at t: bend, then the twist about the
    /// normal or the action axis.
    pub fn ring_turn(&self, t: f64) -> DQuat {
        let twist = if self.twist_axis.is_none() {
            DQuat::from_axis_angle(self.normal, self.rotation * t)
        } else {
            DQuat::IDENTITY
        };
        self.axis_turn(t) * self.bend_turn(t) * twist
    }
}

/// A group's falloff weight from its base centre and its points.
pub type GroupWeight<'a> = &'a dyn Fn(DVec3, &[u32]) -> f64;

/// What Twist Bevel reads from the shared tool pipe (spec section 5), in
/// the mesh's own coordinates.
pub struct TwistBevelPipe<'a> {
    /// Falloff weight of a group from its base centre and its points
    /// (`None`: no falloff).
    pub weight: Option<GroupWeight<'a>>,
    /// Symmetry, and the side that gets the edit as entered.
    pub symmetry: Option<(Symmetry, Side)>,
    /// The action axis's main direction (for Rotation Axis = Action Axis).
    pub action_axis: DVec3,
}
impl Default for TwistBevelPipe<'_> {
    fn default() -> Self {
        Self {
            weight: None,
            symmetry: None,
            action_axis: DVec3::Y,
        }
    }
}

/// A Twist Bevel's result: Bevel's, plus a status note when a group
/// crossing the symmetry plane had its Bend Direction locked (TB-Q06).
#[derive(Clone, Debug)]
pub struct TwistBevelResult {
    pub mesh: Mesh,
    pub selected: Vec<u32>,
    pub note: Option<String>,
}

/// The side axis S for normal `n` (TB-Q04, provisional): world +X flattened
/// onto the plane at right angles to N, else +Z when N is nearly along X.
fn side_axis(n: DVec3) -> DVec3 {
    let flat = |d: DVec3| d - n * d.dot(n);
    let x = flat(DVec3::X);
    if x.length() > 0.05 {
        x.normalize()
    } else {
        flat(DVec3::Z).normalize_or(DVec3::Y)
    }
}

/// Each tube's frame, in region order (`Mesh::polygon_regions`), and the
/// symmetry note.
pub fn twist_bevel_frames(
    mesh: &Mesh,
    selected: &[u32],
    options: &TwistBevelOptions,
    pipe: &TwistBevelPipe<'_>,
) -> Result<(Vec<TubeFrame>, Option<String>)> {
    let o = options.clamped();
    o.validate()?;
    let regions = mesh.polygon_regions(selected, o.group_polygons)?;
    let tolerance = crate::symmetry::match_tolerance(mesh);
    let mut note = None;
    let mut frames = Vec::with_capacity(regions.len());
    for region in &regions {
        let count = region.vertices.len().max(1) as f64;
        let center = region
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .sum::<DVec3>()
            / count;
        let normal = region.faces.iter().map(|&f| mesh.normal(f)).sum::<DVec3>()
            / region.faces.len().max(1) as f64;
        if !normal.is_finite() || normal.length() < 1e-9 {
            return Err("Twist Bevel: can't find a direction for a closed selection".into());
        }
        let normal = normal.normalize();
        let mut boundary_points: Vec<u32> = region.boundary.iter().map(|e| e.a).collect();
        boundary_points.sort_unstable();
        boundary_points.dedup();
        let half_width = boundary_points
            .iter()
            .map(|&v| {
                let d = mesh.vertices[v as usize] - center;
                (d - normal * d.dot(normal)).length()
            })
            .sum::<f64>()
            / boundary_points.len().max(1) as f64;
        // Spec 3.1 with section 5's symmetry: a group on the mirror side
        // takes the mirror image of its partner's frame (side axis mirrored,
        // Bend Direction and Rotation reversed), so mirrored groups give
        // mirror-image tubes.
        let side = pipe
            .symmetry
            .and_then(|(s, primary)| s.side(center, tolerance).map(|side| (s, side, primary)));
        let mirrored =
            side.is_some_and(|(_, side, primary)| side != Side::Plane && side != primary);
        let (side_axis, direction, rotation_sign) = match side {
            Some((s, _, _)) if mirrored => (
                s.mirror_vector(side_axis(s.mirror_vector(normal))),
                -o.bend_direction,
                -1.,
            ),
            _ => (side_axis(normal), o.bend_direction, 1.),
        };
        let mut bend_dir = DQuat::from_axis_angle(normal, direction.to_radians()) * side_axis;
        // TB-Q06 (provisional): a group across the plane keeps its own frame
        // with B locked into the symmetry plane.
        if let Some((s, Side::Plane, _)) = side
            && let Some(plane_normal) = s.normal()
        {
            let along = normal.cross(plane_normal);
            if along.length() > 1e-6 {
                let along = along.normalize();
                let locked = if along.dot(bend_dir) >= 0. {
                    along
                } else {
                    -along
                };
                if o.bend != 0. && locked.distance(bend_dir) > 1e-9 {
                    note = Some(
                        "Twist Bevel: Bend Direction locked to the symmetry plane for a group across it"
                            .to_owned(),
                    );
                }
                bend_dir = locked;
            }
        }
        let up = normal.cross(bend_dir).normalize();
        let points: Vec<u32> = region.vertices.iter().copied().collect();
        let w = pipe.weight.map_or(1., |f| f(center, &points)).clamp(0., 1.);
        let twist_axis = (o.rotation_axis == TwistAxis::ActionAxis).then(|| {
            let axis = pipe.action_axis.normalize_or(normal);
            match side {
                Some((s, _, _)) if mirrored => s.mirror_vector(axis),
                _ => axis,
            }
        });
        frames.push(TubeFrame {
            center,
            normal,
            bend_dir,
            up,
            half_width,
            shift: o.shift * w,
            inset: o.inset * w,
            rotation: o.rotation.to_radians() * w * rotation_sign,
            // Spec section 6: a spine with no length doesn't bend.
            bend: if o.shift * w == 0. {
                0.
            } else {
                o.bend.to_radians() * w
            },
            profile: o.profile,
            profile_bias: o.profile_bias,
            twist_axis,
        });
    }
    Ok((frames, note))
}

struct Placer {
    rings: Vec<f64>,
    frames: Vec<TubeFrame>,
}
impl RingPlacer for Placer {
    fn rings(&self) -> &[f64] {
        &self.rings
    }
    fn skip(&self, region: usize) -> bool {
        let f = &self.frames[region];
        f.shift == 0. && f.inset == 0.
    }
    fn inset(&self, region: usize, level: u32) -> f64 {
        self.frames[region].inset_at(self.rings[level as usize - 1])
    }
    fn place(&self, region: usize, level: u32, point: DVec3, unit_inset: DVec3) -> DVec3 {
        self.frames[region].place(self.rings[level as usize - 1], point, unit_inset)
    }
    fn cap_normal(&self, region: usize, normal: DVec3) -> DVec3 {
        self.frames[region].ring_turn(1.) * normal
    }
}

/// The ring positions t for `options` (spec 3.3 and 3.7).
pub fn twist_bevel_rings(options: &TwistBevelOptions) -> Vec<f64> {
    let o = options.clamped();
    (1..=o.segments)
        .map(|k| {
            if k == o.segments {
                1.
            } else {
                ring_spacing(
                    k as f64 / o.segments as f64,
                    o.segment_spacing,
                    o.center_bias,
                )
            }
        })
        .collect()
}

/// Twist Bevel on `selected` polygons of `mesh` (spec section 3). The
/// original polygons become the caps and stay selected; Shift and Inset
/// both 0 leave the mesh unchanged.
pub fn twist_bevel_geometry(
    mesh: &Mesh,
    selected: &[u32],
    options: &TwistBevelOptions,
    pipe: &TwistBevelPipe<'_>,
) -> Result<TwistBevelResult> {
    let o = options.clamped();
    o.validate()?;
    mesh.require_surface_elements()?;
    let (frames, note) = twist_bevel_frames(mesh, selected, &o, pipe)?;
    let placer = Placer {
        rings: twist_bevel_rings(&o),
        frames,
    };
    if o.is_zero() || (0..placer.frames.len()).all(|r| placer.skip(r)) {
        return Ok(TwistBevelResult {
            mesh: mesh.clone(),
            selected: selected.to_vec(),
            note: None,
        });
    }
    let bevel = BevelOptions {
        segments: o.segments,
        ..BevelOptions::default()
    };
    let BevelResult { mesh, selected } =
        crate::bevel::bevel_placed(mesh, selected, o.group_polygons, &bevel, &placer).map_err(
            |e| {
                if e.contains("Bevel") {
                    e.replace("Bevel", "Twist Bevel")
                } else {
                    format!("Twist Bevel: {e}")
                }
            },
        )?;
    Ok(TwistBevelResult {
        mesh,
        selected,
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_keeps_the_ends_and_order() {
        for (s, c) in [
            (0., 0.),
            (1., 0.),
            (-1., 0.),
            (0., 1.),
            (0., -1.),
            (2., -2.),
        ] {
            assert_eq!(ring_spacing(0., s, c), 0.);
            assert!((ring_spacing(1., s, c) - 1.).abs() < 1e-12);
            let mut last = 0.;
            for k in 1..=8 {
                let t = ring_spacing(k as f64 / 8., s, c);
                assert!(t > last, "s {s} c {c} k {k}");
                last = t;
            }
        }
        assert!((ring_spacing(0.25, 0., 0.) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn bump_peaks_where_the_bias_says() {
        assert!((profile_bump(0.5, 0.) - 1.).abs() < 1e-12);
        assert_eq!(profile_bump(0., 0.7), 0.);
        assert!(profile_bump(1., 0.7).abs() < 1e-12);
        assert!((profile_bump(0.95, 1.) - 1.).abs() < 1e-9);
        assert!((profile_bump(0.05, -1.) - 1.).abs() < 1e-9);
    }
}
