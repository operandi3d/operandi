//! Gradient layer and its curves (shader tree remaining, gradient family
//! v1 §1–2, 2026-11-07).
//!
//! A Gradient reads one input at the shading point (an angle, a distance,
//! the folded value of the layers below...) and looks it up on a Value
//! curve (scalar effects) or a Color curve with alpha (colour effects).
//! Keys sit at input positions in the input's own units: degrees for
//! angles, metres for distances, fractions (0..1, shown as percent) for
//! values.
use serde::{Deserialize, Serialize};

/// How the curve runs from a key to the next one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyShape {
    /// Smooth through the keys, never overshooting between two keys (GQ-1).
    #[default]
    Smooth,
    Linear,
    /// Holds this key's value until the next key.
    Stepped,
}
impl KeyShape {
    pub const ALL: [KeyShape; 3] = [KeyShape::Smooth, KeyShape::Linear, KeyShape::Stepped];
    pub fn label(self) -> &'static str {
        match self {
            KeyShape::Smooth => "Smooth",
            KeyShape::Linear => "Linear",
            KeyShape::Stepped => "Stepped",
        }
    }
}

/// One curve key: input position, output (value, or RGB plus alpha) and the
/// shape toward the next key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key<V> {
    pub at: f64,
    pub out: V,
    #[serde(default)]
    pub shape: KeyShape,
}

/// Outputs a curve can interpolate.
pub trait CurveOut: Copy {
    const N: usize;
    fn get(&self, k: usize) -> f64;
    fn from_fn(f: impl FnMut(usize) -> f64) -> Self;
}
impl CurveOut for f64 {
    const N: usize = 1;
    fn get(&self, _: usize) -> f64 {
        *self
    }
    fn from_fn(mut f: impl FnMut(usize) -> f64) -> Self {
        f(0)
    }
}
/// RGB and alpha.
impl CurveOut for [f64; 4] {
    const N: usize = 4;
    fn get(&self, k: usize) -> f64 {
        self[k]
    }
    fn from_fn(f: impl FnMut(usize) -> f64) -> Self {
        std::array::from_fn(f)
    }
}

/// A curve: keys kept in input order. Before the first key and after the
/// last one it holds the end value; with no keys it is all 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Curve<V> {
    pub keys: Vec<Key<V>>,
}

/// Most keys a curve may hold.
pub const MAX_KEYS: usize = 256;

impl<V: CurveOut> Curve<V> {
    pub fn single(out: V) -> Self {
        Self {
            keys: vec![Key {
                at: 0.,
                out,
                shape: KeyShape::Smooth,
            }],
        }
    }
    /// Puts the keys back in input order (a key dragged past another).
    pub fn sort(&mut self) {
        self.keys.sort_by(|a, b| a.at.total_cmp(&b.at));
    }
    /// The input span the keys cover.
    pub fn span(&self) -> Option<(f64, f64)> {
        let first = self.keys.first()?.at;
        let last = self.keys.last()?.at;
        Some((first, last))
    }
    pub fn valid(&self) -> bool {
        self.keys.len() <= MAX_KEYS
            && self
                .keys
                .iter()
                .all(|k| k.at.is_finite() && (0..V::N).all(|c| k.out.get(c).is_finite()))
            && self.keys.windows(2).all(|w| w[0].at <= w[1].at)
    }
    /// The curve's output at input `x`.
    pub fn sample(&self, x: f64) -> V {
        let keys = &self.keys;
        let Some(first) = keys.first() else {
            return V::from_fn(|_| 0.);
        };
        let last = &keys[keys.len() - 1];
        if x <= first.at || keys.len() == 1 {
            return first.out;
        }
        if x >= last.at {
            return last.out;
        }
        // The segment holding x: keys[i].at <= x < keys[i + 1].at.
        let i = keys.partition_point(|k| k.at <= x) - 1;
        let (a, b) = (&keys[i], &keys[i + 1]);
        let w = b.at - a.at;
        if w <= 0. {
            return b.out;
        }
        let t = (x - a.at) / w;
        match a.shape {
            KeyShape::Stepped => a.out,
            KeyShape::Linear => V::from_fn(|c| a.out.get(c) + (b.out.get(c) - a.out.get(c)) * t),
            KeyShape::Smooth => {
                // Catmull-Rom style slopes from the neighbours (flat at the
                // ends, where the curve holds), clamped to the segment's
                // own range so it never overshoots (GQ-1).
                let slope = |j: usize, c: usize| -> f64 {
                    if j == 0 || j + 1 >= keys.len() {
                        return 0.;
                    }
                    let (p, n) = (&keys[j - 1], &keys[j + 1]);
                    let dx = n.at - p.at;
                    if dx <= 0. {
                        0.
                    } else {
                        (n.out.get(c) - p.out.get(c)) / dx
                    }
                };
                let (h00, h10, h01, h11) = (
                    2. * t.powi(3) - 3. * t * t + 1.,
                    t.powi(3) - 2. * t * t + t,
                    -2. * t.powi(3) + 3. * t * t,
                    t.powi(3) - t * t,
                );
                V::from_fn(|c| {
                    let (p0, p1) = (a.out.get(c), b.out.get(c));
                    let v = h00 * p0 + h10 * w * slope(i, c) + h01 * p1 + h11 * w * slope(i + 1, c);
                    v.clamp(p0.min(p1), p0.max(p1))
                })
            }
        }
    }
    /// `n` evenly spaced samples over [lo, hi] (n ≥ 2), for the renderer's
    /// colour ramp.
    pub fn table(&self, lo: f64, hi: f64, n: usize) -> Vec<V> {
        (0..n)
            .map(|k| self.sample(lo + (hi - lo) * k as f64 / (n - 1) as f64))
            .collect()
    }
}

/// Samples the renderer's ramp takes from a curve between its first and
/// last key.
pub const RAMP_SAMPLES: usize = 64;

macro_rules! inputs {
    ($($variant:ident => $label:literal, $group:literal, $unit:ident);+ $(;)?) => {
        /// A Gradient's Input Parameter (§1), in menu order.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum GradientInput { $($variant),+ }
        impl GradientInput {
            pub const ALL: &[GradientInput] = &[$(GradientInput::$variant),+];
            pub fn label(self) -> &'static str {
                match self { $(GradientInput::$variant => $label),+ }
            }
            /// The menu group it is listed under.
            pub fn group(self) -> &'static str {
                match self { $(GradientInput::$variant => $group),+ }
            }
            pub fn unit(self) -> Unit {
                match self { $(GradientInput::$variant => Unit::$unit),+ }
            }
        }
    };
}

/// How an input's key positions are shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Degrees,
    Metres,
    /// A fraction, shown as percent.
    Percent,
    /// A plain number (0..1 coordinates, 0/1 flags).
    Number,
}

inputs! {
    DiffuseAmount => "Diffuse Amount", "Basic Channels", Percent;
    DiffuseRoughness => "Diffuse Roughness", "Basic Channels", Percent;
    SpecularAmount => "Specular Amount", "Basic Channels", Percent;
    SpecularFresnel => "Specular Fresnel", "Basic Channels", Percent;
    ReflectionAmount => "Reflection Amount", "Basic Channels", Percent;
    ReflectionFresnel => "Reflection Fresnel", "Basic Channels", Percent;
    Roughness => "Roughness", "Basic Channels", Percent;
    ClearcoatAmount => "Clearcoat Amount", "Basic Channels", Percent;
    TransparentAmount => "Transparent Amount", "Basic Channels", Percent;
    RefractionRoughness => "Refraction Roughness", "Basic Channels", Percent;
    SubsurfaceAmount => "Subsurface Amount", "Basic Channels", Percent;
    LuminousAmount => "Luminous Amount", "Basic Channels", Percent;
    Dissolve => "Dissolve", "Basic Channels", Percent;
    Incidence => "Incidence", "Sample Parameters", Degrees;
    Slope => "Slope", "Sample Parameters", Degrees;
    TextureValue => "Texture Value", "Sample Parameters", Percent;
    DistanceToCamera => "Distance to Camera", "Sample Parameters", Metres;
    DistanceToLocator => "Distance to Locator", "Sample Parameters", Metres;
    TextureU => "Texture U", "Sample Parameters", Number;
    TextureV => "Texture V", "Sample Parameters", Number;
    BackFacing => "Back Facing", "Sample Parameters", Number;
    Thickness => "Thickness", "Sample Parameters", Metres;
    BumplessIncident => "Bumpless Incident", "Sample Parameters", Degrees;
    LocatorIncidence => "Locator Incidence", "Sample Parameters", Degrees;
    XyzDistanceToLocator => "XYZ Distance to Locator", "Sample Parameters", Metres;
    ScreenX => "Screen X", "Sample Parameters", Number;
    ScreenY => "Screen Y", "Sample Parameters", Number;
    ScreenR => "Screen R", "Sample Parameters", Number;
    DriverA => "Driver A", "Shader Control", Number;
    DriverB => "Driver B", "Shader Control", Number;
    DriverC => "Driver C", "Shader Control", Number;
    DriverD => "Driver D", "Shader Control", Number;
    GroupMask => "Group Mask", "Shader Control", Number;
    LayerMask => "Layer Mask", "Shader Control", Number;
    Stencil => "Stencil", "Special Effects", Number;
    BumpHeight => "Bump Height", "Surface Shading", Metres;
    DisplacementHeight => "Displacement Height", "Surface Shading", Metres;
    ParticleId => "Particle ID", "Particle Effects", Number;
    ParticleAge => "Particle Age", "Particle Effects", Number;
    ParticleVelocity => "Particle Velocity", "Particle Effects", Number;
    TextureParticleId => "Texture Particle ID", "Particle Effects", Number;
}

impl GradientInput {
    /// The menu groups, in order (§1; the Recent group is the panel's).
    pub const GROUPS: &[&str] = &[
        "Basic Channels",
        "Sample Parameters",
        "Shader Control",
        "Special Effects",
        "Surface Shading",
        "Particle Effects",
    ];
    /// Inputs kept in the menu that render as a constant 0 until built
    /// (§1 "later / stored-only", GQ / GR-Q02).
    pub fn is_stored_only(self) -> bool {
        use GradientInput::*;
        matches!(
            self,
            ParticleId | ParticleAge | ParticleVelocity | TextureParticleId | GroupMask | LayerMask
        )
    }
}

/// A Gradient layer's own settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gradient {
    pub input: GradientInput,
    pub value: Curve<f64>,
    /// RGB and alpha per key.
    pub color: Curve<[f64; 4]>,
}
impl Default for Gradient {
    /// G-20: Incidence, one key at input 0 (GQ-6): value 100 %, white at
    /// alpha 100 %, so a new Gradient renders flat white.
    fn default() -> Self {
        Self {
            input: GradientInput::Incidence,
            value: Curve::single(1.),
            color: Curve::single([1.; 4]),
        }
    }
}
impl Gradient {
    pub fn valid(&self) -> bool {
        self.value.valid()
            && self.color.valid()
            && !self.color.keys.is_empty()
            && !self.value.keys.is_empty()
    }
}

/// Edit Gradient window actions (GR-Q03 v2, provisional): whole-curve
/// edits that keep the keys' span.
impl<V: CurveOut> Curve<V> {
    /// The keys mirrored across their span: the first key's output ends up
    /// at the last key's input. Each key keeps its shape, which now leads
    /// into the key before it, so a Stepped key holds toward the start.
    pub fn reversed(&self) -> Self {
        let Some((lo, hi)) = self.span() else {
            return self.clone();
        };
        let mut keys: Vec<Key<V>> = self
            .keys
            .iter()
            .map(|k| Key {
                at: lo + hi - k.at,
                ..*k
            })
            .collect();
        keys.reverse();
        Self { keys }
    }
    /// The keys spread evenly across their span, in order.
    pub fn distributed(&self) -> Self {
        let n = self.keys.len();
        let Some((lo, hi)) = self.span() else {
            return self.clone();
        };
        if n < 2 {
            return self.clone();
        }
        let keys = self
            .keys
            .iter()
            .enumerate()
            .map(|(k, key)| Key {
                at: lo + (hi - lo) * k as f64 / (n - 1) as f64,
                ..*key
            })
            .collect();
        Self { keys }
    }
}

/// A built-in curve for the Edit Gradient window's Presets menu
/// (provisional, GR-Q09): keys at fractions of a span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Value 0 → 100 % (black → white for colour).
    Ramp,
    /// Value 100 % → 0 (white → black).
    RampDown,
    /// 0 → 100 % → 0.
    Bell,
    /// Four even steps from 0 to 100 %.
    Steps,
    /// Red, yellow, green, cyan, blue, magenta (colour only).
    Rainbow,
    /// Black, red, yellow, white (colour only).
    Heat,
    /// Blue → white → orange (colour only).
    CoolWarm,
}
impl Preset {
    pub const VALUE: [Preset; 4] = [Preset::Ramp, Preset::RampDown, Preset::Bell, Preset::Steps];
    pub const COLOR: [Preset; 7] = [
        Preset::Ramp,
        Preset::RampDown,
        Preset::Bell,
        Preset::Steps,
        Preset::Rainbow,
        Preset::Heat,
        Preset::CoolWarm,
    ];
    pub fn label(self, color: bool) -> &'static str {
        match (self, color) {
            (Preset::Ramp, false) => "Ramp Up",
            (Preset::Ramp, true) => "Black to White",
            (Preset::RampDown, false) => "Ramp Down",
            (Preset::RampDown, true) => "White to Black",
            (Preset::Bell, _) => "Bell",
            (Preset::Steps, _) => "Four Steps",
            (Preset::Rainbow, _) => "Rainbow",
            (Preset::Heat, _) => "Heat",
            (Preset::CoolWarm, _) => "Cool to Warm",
        }
    }
    /// (fraction of the span, grey or colour, shape) per key.
    fn keys(self) -> Vec<(f64, [f64; 3], KeyShape)> {
        use KeyShape::{Linear, Smooth, Stepped};
        let g = |v: f64| [v; 3];
        match self {
            Preset::Ramp => vec![(0., g(0.), Linear), (1., g(1.), Linear)],
            Preset::RampDown => vec![(0., g(1.), Linear), (1., g(0.), Linear)],
            Preset::Bell => vec![
                (0., g(0.), Smooth),
                (0.5, g(1.), Smooth),
                (1., g(0.), Smooth),
            ],
            Preset::Steps => (0..4)
                .map(|k| (f64::from(k) / 4., g(f64::from(k) / 3.), Stepped))
                .chain([(1., g(1.), Stepped)])
                .collect(),
            Preset::Rainbow => [
                [1., 0., 0.],
                [1., 1., 0.],
                [0., 1., 0.],
                [0., 1., 1.],
                [0., 0., 1.],
                [1., 0., 1.],
            ]
            .into_iter()
            .enumerate()
            .map(|(k, c)| (k as f64 / 5., c, Linear))
            .collect(),
            Preset::Heat => vec![
                (0., g(0.), Linear),
                (0.4, [1., 0., 0.], Linear),
                (0.75, [1., 1., 0.], Linear),
                (1., g(1.), Linear),
            ],
            Preset::CoolWarm => vec![
                (0., [0.1, 0.25, 1.], Smooth),
                (0.5, g(1.), Smooth),
                (1., [1., 0.45, 0.05], Smooth),
            ],
        }
    }
    /// The Value curve over input `lo..hi` (a colour's brightness).
    pub fn value(self, lo: f64, hi: f64) -> Curve<f64> {
        Curve {
            keys: self
                .keys()
                .into_iter()
                .map(|(t, c, shape)| Key {
                    at: lo + (hi - lo) * t,
                    out: 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2],
                    shape,
                })
                .collect(),
        }
    }
    /// The Color curve over input `lo..hi`, opaque.
    pub fn color(self, lo: f64, hi: f64) -> Curve<[f64; 4]> {
        Curve {
            keys: self
                .keys()
                .into_iter()
                .map(|(t, c, shape)| Key {
                    at: lo + (hi - lo) * t,
                    out: [c[0], c[1], c[2], 1.],
                    shape,
                })
                .collect(),
        }
    }
}

impl GradientInput {
    /// The span a preset covers when a curve's keys span nothing: the
    /// input's usual range (GR-Q09).
    pub fn natural_span(self) -> (f64, f64) {
        match self.unit() {
            Unit::Degrees if self == GradientInput::Slope => (0., 180.),
            Unit::Degrees => (0., 90.),
            Unit::Metres => (0., 10.),
            Unit::Percent | Unit::Number => (0., 1.),
        }
    }
}

#[cfg(test)]
mod tests {
    /// GR-Q03 v2: Reverse mirrors the keys across their span;
    /// Distribute Evenly spaces them; presets fill the span.
    #[test]
    fn window_actions_keep_the_span() {
        let c = Curve {
            keys: vec![
                Key {
                    at: 10.,
                    out: 0.,
                    shape: KeyShape::Linear,
                },
                Key {
                    at: 20.,
                    out: 0.3,
                    shape: KeyShape::Stepped,
                },
                Key {
                    at: 50.,
                    out: 1.,
                    shape: KeyShape::Smooth,
                },
            ],
        };
        let r = c.reversed();
        assert_eq!(r.span(), Some((10., 50.)));
        assert_eq!(r.keys[0].out, 1.);
        assert_eq!(r.keys[1].at, 40.);
        assert_eq!(r.keys[1].shape, KeyShape::Stepped);
        assert_eq!(r.reversed(), c);
        let d = c.distributed();
        assert_eq!(
            d.keys.iter().map(|k| k.at).collect::<Vec<_>>(),
            [10., 30., 50.]
        );
        let ramp = Preset::Ramp.value(0., 90.);
        assert!((ramp.sample(45.) - 0.5).abs() < 1e-9);
        let rainbow = Preset::Rainbow.color(0., 1.);
        assert_eq!(rainbow.keys.len(), 6);
        assert_eq!(rainbow.sample(0.), [1., 0., 0., 1.]);
        let steps = Preset::Steps.value(0., 1.);
        assert!((steps.sample(0.3) - 1. / 3.).abs() < 1e-9);
        assert_eq!(GradientInput::Slope.natural_span(), (0., 180.));
    }

    use super::*;

    fn keys(points: &[(f64, f64, KeyShape)]) -> Curve<f64> {
        Curve {
            keys: points
                .iter()
                .map(|&(at, out, shape)| Key { at, out, shape })
                .collect(),
        }
    }

    #[test]
    fn ends_hold_and_one_key_is_flat() {
        let c = Curve::single(0.7);
        assert_eq!(
            (c.sample(-5.), c.sample(0.), c.sample(90.)),
            (0.7, 0.7, 0.7)
        );
        let c = keys(&[(10., 0.2, KeyShape::Linear), (20., 0.8, KeyShape::Linear)]);
        assert_eq!(c.sample(0.), 0.2);
        assert_eq!(c.sample(30.), 0.8);
        assert!((c.sample(15.) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn stepped_holds_until_the_next_key() {
        let c = keys(&[(0., 0., KeyShape::Stepped), (0.75, 1., KeyShape::Stepped)]);
        assert_eq!(c.sample(0.74), 0.);
        assert_eq!(c.sample(0.75), 1.);
    }

    #[test]
    fn smooth_never_overshoots() {
        let c = keys(&[
            (0., 0., KeyShape::Smooth),
            (1., 1., KeyShape::Smooth),
            (2., 1., KeyShape::Smooth),
            (3., 0., KeyShape::Smooth),
        ]);
        for k in 0..=300 {
            let v = c.sample(k as f64 / 100.);
            assert!((-1e-12..=1. + 1e-12).contains(&v), "{v}");
        }
        // Through the keys, eased between them.
        assert_eq!(c.sample(1.), 1.);
        let mid = c.sample(0.5);
        assert!(mid > 0.3 && mid < 0.7);
    }

    #[test]
    fn colour_curves_interpolate_every_component() {
        let c = Curve {
            keys: vec![
                Key {
                    at: 0.,
                    out: [0., 0., 0., 1.],
                    shape: KeyShape::Linear,
                },
                Key {
                    at: 90.,
                    out: [1., 0.5, 0., 0.],
                    shape: KeyShape::Linear,
                },
            ],
        };
        let v = c.sample(45.);
        assert!((v[0] - 0.5).abs() < 1e-12 && (v[1] - 0.25).abs() < 1e-12);
        assert!((v[3] - 0.5).abs() < 1e-12);
        assert_eq!(c.table(0., 90., 3)[2], [1., 0.5, 0., 0.]);
    }

    #[test]
    fn new_gradient_is_flat_white_on_incidence() {
        let g = Gradient::default();
        assert_eq!(g.input, GradientInput::Incidence);
        assert_eq!(g.color.sample(37.), [1.; 4]);
        assert_eq!(g.value.sample(80.), 1.);
        assert!(g.valid());
    }
}
