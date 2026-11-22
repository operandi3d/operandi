//! Smooth shading (spec smooth-shading v0, SS-01..SS-04).
//!
//! Smoothing gives each polygon corner a shading normal averaged from the
//! polygons around its point instead of the polygon's own flat normal. It
//! never moves geometry. Which neighbours join the average is decided by
//! the Smoothing Angle and, optionally, smoothing group labels; how far the
//! corner bends toward that average is the Smoothing amount.
//!
//! Until materials carry properties, one default material's settings apply
//! to every mesh (SS-01), stored on the scene as [`SmoothingSettings`].
//! The whole spec is *provisional*: it was read from documentation and has
//! not been observed yet, so the defaults are constants here, easy to
//! change once Research confirms them.
use crate::Mesh;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Smoothing amount of a new scene, in percent (SS-01, provisional).
pub const DEFAULT_SMOOTHING_AMOUNT: f64 = 100.0;
/// Smoothing Angle of a new scene, in degrees (SS-01, provisional).
pub const DEFAULT_SMOOTHING_ANGLE: f64 = 40.0;
/// Range the Smoothing amount field accepts, in percent. The spec allows
/// overshoot above 100 % and reversal below 0 % (SS-02 step 4) but gives no
/// limits; this is an Operandi budget (SS-Q04).
pub const SMOOTHING_AMOUNT_RANGE: std::ops::RangeInclusive<f64> = -100.0..=200.0;
/// Range the Smoothing Angle field accepts, in degrees.
pub const SMOOTHING_ANGLE_RANGE: std::ops::RangeInclusive<f64> = 0.0..=180.0;

/// How smoothing group labels take part (SS-03).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmoothingGroups {
    /// Labels are ignored; only the angle rule decides.
    Off,
    /// Polygons that share no group never blend; within a group the angle
    /// rule still decides. The spec's default.
    #[default]
    Crease,
    /// Polygons that share no group never blend; those that share one
    /// always blend, even past the smoothing angle.
    Force,
}

/// The default material's smoothing settings (SS-01).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmoothingSettings {
    /// Percent: 0 flat, 100 fully smooth, above overshoots, below reverses.
    pub amount: f64,
    /// Degrees: neighbours whose polygon normal is within this angle of the
    /// corner's polygon join its average.
    pub angle: f64,
    /// Off makes the angle 180°: every connected polygon blends, so creases
    /// come only from smoothing groups (SS-02).
    pub crease_by_angle: bool,
    pub groups: SmoothingGroups,
}
impl Default for SmoothingSettings {
    fn default() -> Self {
        Self {
            amount: DEFAULT_SMOOTHING_AMOUNT,
            angle: DEFAULT_SMOOTHING_ANGLE,
            crease_by_angle: true,
            groups: SmoothingGroups::Crease,
        }
    }
}
impl SmoothingSettings {
    /// Settings a scene can hold: finite and inside the fields' ranges.
    pub fn validate(&self) -> crate::Result<()> {
        if !self.amount.is_finite() || !SMOOTHING_AMOUNT_RANGE.contains(&self.amount) {
            return Err(format!(
                "Smoothing amount must be between {}% and {}%",
                SMOOTHING_AMOUNT_RANGE.start(),
                SMOOTHING_AMOUNT_RANGE.end()
            ));
        }
        if !self.angle.is_finite() || !SMOOTHING_ANGLE_RANGE.contains(&self.angle) {
            return Err("Smoothing angle must be between 0° and 180°".into());
        }
        Ok(())
    }

    /// True when every corner shades with its polygon's own normal, so
    /// callers can skip the neighbour search.
    pub fn is_flat(&self) -> bool {
        self.amount == 0.0
    }

    /// Cosine the dot product of two unit normals must reach to blend. A
    /// small tolerance keeps an exact 40° step (or a 90° cube side at 90°)
    /// on the blending side despite rounding.
    fn cos_limit(&self) -> f64 {
        let angle = if self.crease_by_angle {
            self.angle
        } else {
            180.0
        };
        angle.to_radians().cos() - 1e-9
    }
}

/// One polygon's smoothing group label: the set of characters it carries,
/// or `None` when it has none (SS-03).
fn label(mesh: &Mesh, face: usize) -> Option<&str> {
    mesh.attributes
        .smoothing_labels
        .get(face)
        .and_then(|l| l.as_deref())
        .filter(|l| !l.is_empty())
}

/// Whether two polygons share a smoothing group: their labels have a
/// character in common ("AB" shares with "A" and "B"). Two untagged
/// polygons count as sharing; a tagged and an untagged one don't
/// (*provisional*, SS-Q07).
pub(crate) fn share_group(mesh: &Mesh, a: usize, b: usize) -> bool {
    match (label(mesh, a), label(mesh, b)) {
        (None, None) => true,
        (Some(x), Some(y)) => x.chars().any(|c| y.contains(c)),
        _ => false,
    }
}

/// The shading normal of polygon `face`'s corner at point `point` (SS-02).
///
/// `around` lists the polygons using `point` (from `Mesh::point_faces`),
/// `normal_of` gives a polygon's unit normal (zero for degenerate ones or
/// polylines, which never join), and `flat` is `face`'s own normal. Each
/// neighbour is compared with `face` directly, not through chains of
/// neighbours (*provisional*, SS-Q03), and weighted 1.
pub fn corner_normal(
    mesh: &Mesh,
    settings: &SmoothingSettings,
    face: usize,
    around: &[u32],
    flat: DVec3,
    normal_of: impl Fn(usize) -> DVec3,
) -> DVec3 {
    if settings.is_flat() || flat == DVec3::ZERO {
        return flat;
    }
    let cos_limit = settings.cos_limit();
    let mut sum = flat;
    for &other in around {
        let other = other as usize;
        if other == face {
            continue;
        }
        let n = normal_of(other);
        if n == DVec3::ZERO {
            continue;
        }
        let joins = match settings.groups {
            SmoothingGroups::Off => flat.dot(n) >= cos_limit,
            SmoothingGroups::Crease => share_group(mesh, face, other) && flat.dot(n) >= cos_limit,
            SmoothingGroups::Force => share_group(mesh, face, other),
        };
        if joins {
            sum += n;
        }
    }
    let smooth = sum.normalize_or_zero();
    if smooth == DVec3::ZERO {
        return flat;
    }
    // Linear blend from flat toward smooth, then normalize (SS-Q04).
    let t = settings.amount / 100.0;
    let n = (flat + (smooth - flat) * t).normalize_or_zero();
    if n == DVec3::ZERO { flat } else { n }
}

/// Every polygon's corner normals for `mesh` as it stands (local space),
/// in corner order. Polylines and degenerate polygons get zero normals.
pub fn mesh_corner_normals(mesh: &Mesh, settings: &SmoothingSettings) -> Vec<Vec<DVec3>> {
    let normals: Vec<DVec3> = (0..mesh.faces.len())
        .map(|f| {
            if mesh.is_polyline(f) {
                DVec3::ZERO
            } else {
                mesh.normal(f).normalize_or_zero()
            }
        })
        .collect();
    let around = mesh.point_faces();
    mesh.faces
        .iter()
        .enumerate()
        .map(|(f, face)| {
            face.vertices
                .iter()
                .enumerate()
                .map(|(c, &v)| {
                    // G9 VN-1: the mesh's vertex normal map wins where it
                    // sets a corner.
                    match crate::vertex_normals::custom_corner_normal(mesh, f, c) {
                        Some(n) if normals[f] != DVec3::ZERO => n.normalize_or_zero(),
                        _ => corner_normal(mesh, settings, f, around.of(v), normals[f], |g| {
                            normals[g]
                        }),
                    }
                })
                .collect()
        })
        .collect()
}

/// Label of the undo step a smoothing change makes.
pub const SMOOTHING_STEP: &str = "Smoothing";

impl crate::Editor {
    /// SS-01/SS-06: change the default material's smoothing, one undo step.
    /// With `continuing`, a change straight after another smoothing change
    /// (nothing else in between) updates that step instead of adding one.
    pub(crate) fn set_smoothing(
        &mut self,
        settings: SmoothingSettings,
        continuing: bool,
    ) -> crate::Result<()> {
        self.subdivision_ready("changing smoothing")?;
        settings.validate()?;
        if self.scene.smoothing == settings {
            return Ok(());
        }
        let mut candidate = self.scene.clone();
        candidate.smoothing = settings;
        if continuing
            && self.redo.is_empty()
            && let Some(last) = self.undo.last_mut().map(std::sync::Arc::make_mut)
            && last.label == SMOOTHING_STEP
            && last.after == self.scene
        {
            last.after = candidate.clone();
            self.scene = candidate;
            return Ok(());
        }
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record(SMOOTHING_STEP.into(), before);
        Ok(())
    }
}
