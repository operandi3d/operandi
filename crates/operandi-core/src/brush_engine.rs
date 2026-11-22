//! The shared brush engine (painting stage 2, PNT-40..54; sculpt
//! foundation SC-20..45): the tips and the stroke's dab spacing, one
//! implementation for the sculpt brushes and the paint tools (PNT-42).
//!
//! The sculpt brushes moved their tips and dab placement here unchanged
//! (the regression test `sculpt_dabs_are_unchanged` holds them to the old
//! numbers). What a dab does stays with each user: `sculpt::Stroke` moves
//! points, `paint_stroke` (painting builds) paints texels.
use glam::{DVec3, Vec2};
use serde::{Deserialize, Serialize};

/// SC-31: the two foundation tips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipKind {
    #[default]
    Smooth,
    Hard,
}

impl TipKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smooth => "Smooth",
            Self::Hard => "Hard",
        }
    }
    /// Command names (spec section 14).
    pub fn command(self) -> &'static str {
        match self {
            Self::Smooth => "brush.smooth",
            Self::Hard => "brush.hard",
        }
    }
}

/// SC-31 Smooth tip Shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipShape {
    Linear,
    Smooth,
    Bulge,
    #[default]
    Fat,
    Sharp,
    VerySharp,
    /// The tip's own profile from In/Out Weight and Slope (PNT-80): what
    /// editing them makes. Not in `ALL`, the preset list.
    Custom,
}

impl TipShape {
    pub const ALL: [TipShape; 6] = [
        Self::Linear,
        Self::Smooth,
        Self::Bulge,
        Self::Fat,
        Self::Sharp,
        Self::VerySharp,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Smooth => "Smooth",
            Self::Bulge => "Bulge",
            Self::Fat => "Fat",
            Self::Sharp => "Sharp",
            Self::VerySharp => "Very Sharp",
            Self::Custom => "Custom",
        }
    }
    /// Weight at `t` (0 centre, 1 rim). The curves follow the spec's words
    /// (cone, S-curve, dome, soft U steep at the rim, steep at the centre,
    /// steeper); exact curves are provisional (SQ-03 measured only names).
    /// All but Fat are Blender's falloff presets of the same shape.
    pub fn weight(self, t: f64) -> f64 {
        use crate::sculpt_blender::curve::Curve;
        let t = t.clamp(0., 1.);
        let curve = match self {
            Self::Linear => Curve::Linear,
            Self::Smooth => Curve::Smooth,
            Self::Bulge => Curve::Sphere,
            Self::Fat => return 1. - t.powi(4),
            Self::Sharp => Curve::Sharp,
            Self::VerySharp => Curve::Pow4,
            // Needs the tip's profile: `Tip::weight` handles it.
            Self::Custom => return CustomProfile::default().weight(t),
        };
        curve.strength(t, 1.)
    }
}

/// The brush tip (SC-20, SC-21, SC-31).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tip {
    pub kind: TipKind,
    /// Radius in screen pixels (points).
    pub size: f64,
    /// SC-21: keep the size constant relative to the model.
    pub auto_scale: bool,
    /// With Auto Scale: metres per pixel when it was turned on (or the size
    /// last set), so the world radius is `size × scale_reference`. 0 until
    /// first used.
    pub scale_reference: f64,
    pub shape: TipShape,
    /// Hard tip's edge ramp, 0..1 of the radius.
    pub soft_border: f64,
    /// 0..1; 0.5 bends nothing.
    pub bias: f64,
    /// The Custom shape's profile (PNT-80); presets ignore it.
    pub custom: CustomProfile,
}

impl Default for Tip {
    fn default() -> Self {
        Self {
            kind: TipKind::Smooth,
            size: 10.,
            auto_scale: false,
            scale_reference: 0.,
            shape: TipShape::Fat,
            soft_border: 0.2,
            bias: 0.5,
            custom: CustomProfile::default(),
        }
    }
}

/// Smallest and largest brush radius in pixels.
pub const SIZE_RANGE: (f64, f64) = (1., 2000.);

impl Tip {
    /// SC-30: weight 0..1 at distance `t` from the centre (1 = rim);
    /// nothing outside the rim.
    pub fn weight(&self, t: f64) -> f64 {
        if !(t.is_finite() && t <= 1.) {
            return 0.;
        }
        let t = t.max(0.);
        let base = match self.kind {
            TipKind::Smooth if self.shape == TipShape::Custom => self.custom.weight(t),
            TipKind::Smooth => self.shape.weight(t),
            TipKind::Hard => {
                let soft = self.soft_border.clamp(0., 1.);
                let edge = 1. - soft;
                if soft <= 0. || t <= edge {
                    1.
                } else {
                    let u = ((t - edge) / soft).clamp(0., 1.);
                    1. - u * u * (3. - 2. * u)
                }
            }
        };
        bias(self.bias, base)
    }
}

/// Schlick's bias: a gamma-like bend that keeps 0 and 1 in place; `b` 0.5
/// is the identity, higher lifts the curve (SC-31 Bias, provisional curve).
fn bias(b: f64, x: f64) -> f64 {
    let b = b.clamp(0.01, 0.99);
    let x = x.clamp(0., 1.);
    if (b - 0.5).abs() < 1e-12 {
        return x;
    }
    x / ((1. / b - 2.) * (1. - x) + 1.)
}

/// A custom tip profile (PNT-80, Edit Brush Settings): weight from the
/// centre (1) to the rim (0) as a cubic Bézier whose handles the In / Out
/// Weight (handle length, 0..1 of the radius) and In / Out Slope (handle
/// rise) set. 0.5 / 0.5 with flat slopes is a smooth S-curve
/// (provisional curve, PNT-Q17).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomProfile {
    pub in_weight: f64,
    pub out_weight: f64,
    pub in_slope: f64,
    pub out_slope: f64,
}

impl Default for CustomProfile {
    fn default() -> Self {
        Self {
            in_weight: 0.5,
            out_weight: 0.5,
            in_slope: 0.,
            out_slope: 0.,
        }
    }
}

impl CustomProfile {
    /// Weight at `t` (0 centre, 1 rim).
    pub fn weight(&self, t: f64) -> f64 {
        let t = t.clamp(0., 1.);
        let iw = self.in_weight.clamp(0., 1.);
        let ow = self.out_weight.clamp(0., 1.);
        // Control points (x = distance, y = weight).
        let p = [
            (0., 1.),
            (iw, 1. + self.in_slope * iw),
            (1. - ow, -self.out_slope * ow),
            (1., 0.),
        ];
        let at = |u: f64, k: usize| -> f64 {
            let v = 1. - u;
            let c = |i: usize| if k == 0 { p[i].0 } else { p[i].1 };
            v * v * v * c(0) + 3. * v * v * u * c(1) + 3. * v * u * u * c(2) + u * u * u * c(3)
        };
        // x(u) is monotonic for handles inside the square: bisect for u.
        let (mut lo, mut hi) = (0., 1.);
        for _ in 0..40 {
            let mid = (lo + hi) / 2.;
            if at(mid, 0) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        at((lo + hi) / 2., 1).clamp(0., 1.)
    }
}

/// One dab's place along a stroke: the pointer (screen points), the
/// surface point and normal under it, and the pen pressure there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DabPlace {
    pub screen: Vec2,
    pub point: DVec3,
    pub normal: DVec3,
    pub pressure: f64,
}

/// Where the last dab went (the stroke's spacing state, SC-41, PNT-47).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    pub last: Option<DabPlace>,
    /// Pointer travel since the last dab, in screen points.
    pub carry: f64,
}

impl Spacing {
    /// The press: one dab under the pointer (SC-40, PNT-48).
    pub fn press(at: DabPlace) -> (Self, DabPlace) {
        (
            Self {
                last: Some(at),
                carry: 0.,
            },
            at,
        )
    }

    /// Off the surface the stroke pauses (SC-43).
    pub fn lift(&mut self) {
        self.last = None;
    }

    /// The pointer moved to `to`: the dabs along the straight path from
    /// the last dab, every `gap` screen points (Interpolation Step × tip
    /// size, PNT-47), the travel left over carried to the next move. Coming
    /// back onto the surface lays one dab where it came back. At most `max`
    /// dabs per move.
    pub fn advance(
        &mut self,
        to: DabPlace,
        gap: impl FnOnce() -> f64,
        max: usize,
    ) -> Vec<DabPlace> {
        let Some(from) = self.last else {
            self.last = Some(to);
            self.carry = 0.;
            return vec![to];
        };
        let travel = from.screen.distance(to.screen) as f64;
        if travel <= 0. {
            return Vec::new();
        }
        let gap = gap();
        let mut out = Vec::new();
        let mut s = gap - self.carry;
        while s <= travel && out.len() < max {
            let t = s / travel;
            out.push(DabPlace {
                screen: from.screen.lerp(to.screen, t as f32),
                point: from.point.lerp(to.point, t),
                normal: from.normal.lerp(to.normal, t).normalize_or(to.normal),
                pressure: from.pressure + (to.pressure - from.pressure) * t,
            });
            s += gap;
        }
        self.carry = (travel - (s - gap)).max(0.);
        self.last = Some(to);
        out
    }
}
