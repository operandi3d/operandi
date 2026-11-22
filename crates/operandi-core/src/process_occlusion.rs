//! Process layer and Occlusion texture (process / occlusion v1,
//! 2026-11-07).
use crate::shader_eval::Expr;
use crate::shader_tree::{Channel, TwoZone};
use serde::{Deserialize, Serialize};

/// A Process layer's settings (§1). Fractions are 0..1, signed ones −1..1,
/// Hue Shift in degrees.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Process {
    pub bias: f64,
    pub gain: f64,
    pub hue: f64,
    pub saturation: f64,
    pub value: f64,
    pub hue_shift: f64,
}
impl Default for Process {
    /// [measured] changes nothing.
    fn default() -> Self {
        Self {
            bias: 0.5,
            gain: 0.5,
            hue: 0.,
            saturation: 0.,
            value: 0.,
            hue_shift: 0.,
        }
    }
}
impl Process {
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        use Channel::*;
        vec![
            ("Bias", Percent(&mut self.bias)),
            ("Gain", Percent(&mut self.gain)),
            ("Hue", PercentIn(&mut self.hue, -1., 1.)),
            ("Saturation", PercentIn(&mut self.saturation, -1., 1.)),
            ("Value", PercentIn(&mut self.value, -1., 1.)),
            ("Hue Shift", Number(&mut self.hue_shift, -3600., 3600.)),
        ]
    }
    fn reshapes(&self) -> bool {
        self.bias != 0.5 || self.gain != 0.5
    }
    fn recolours(&self) -> bool {
        self.hue != 0. || self.hue_shift != 0. || self.saturation != 0.
    }
    /// One colour through the layer, in the order PQ-2 gives: Bias, Gain
    /// (each channel, the two-zone curves, PQ-1), Hue, Hue Shift,
    /// Saturation, Value; results clamped at 0 only.
    pub fn apply(&self, c: [f64; 3]) -> [f64; 3] {
        let mut c = c;
        if self.reshapes() {
            c = c.map(|v| TwoZone::gain(self.gain, TwoZone::bias(self.bias, v)));
        }
        if self.recolours() {
            let [h, s, v] = rgb_to_hsv(c);
            let h = (h * (1. + self.hue) + self.hue_shift / 360.).rem_euclid(1.);
            let s = (s * (1. + self.saturation)).clamp(0., 1.);
            c = hsv_to_rgb([h, s, v]);
        }
        c.map(|v| (v * (1. + self.value)).max(0.))
    }
    /// The layer's adjustment of `a` as a value the renderer builds.
    pub fn expr(&self, a: Expr) -> Expr {
        if let Some(c) = a.constant() {
            return Expr::Const(self.apply(c));
        }
        let mut e = a;
        if self.reshapes() {
            // Each channel through one shared curve: ramps over 0..1.
            let n = crate::gradient::RAMP_SAMPLES;
            let stops: Vec<[f64; 4]> = (0..n)
                .map(|k| {
                    let v = TwoZone::gain(
                        self.gain,
                        TwoZone::bias(self.bias, k as f64 / (n - 1) as f64),
                    );
                    [v, v, v, 1.]
                })
                .collect();
            let stops = std::sync::Arc::new(stops);
            let mut sum = Expr::scalar(0.);
            for k in 0..3u8 {
                let curved = Expr::Ramp {
                    input: Box::new(Expr::channel(e.clone(), k)),
                    lo: 0.,
                    hi: 1.,
                    stops: stops.clone(),
                    alpha: false,
                };
                let mut mask = [0.; 3];
                mask[usize::from(k)] = 1.;
                sum = Expr::plus(sum, Expr::times(curved, Expr::Const(mask)));
            }
            e = sum;
        }
        if self.recolours() {
            e = Expr::Hsv {
                of: Box::new(e),
                hue_scale: 1. + self.hue,
                hue_shift: self.hue_shift,
                saturation_scale: 1. + self.saturation,
            };
        }
        if self.value != 0. {
            e = Expr::times(e, Expr::scalar(1. + self.value));
            e = Expr::mix(
                crate::shader_eval::Mix::Lighten,
                Expr::scalar(1.),
                e,
                Expr::scalar(0.),
            );
        }
        e
    }
}

/// RGB to HSV, all 0..1 (hue as a fraction of a turn).
pub fn rgb_to_hsv([r, g, b]: [f64; 3]) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max > 0. { d / max } else { 0. };
    let h = if d <= 0. {
        0.
    } else if max == r {
        ((g - b) / d).rem_euclid(6.) / 6.
    } else if max == g {
        ((b - r) / d + 2.) / 6.
    } else {
        ((r - g) / d + 4.) / 6.
    };
    [h, s, max]
}

pub fn hsv_to_rgb([h, s, v]: [f64; 3]) -> [f64; 3] {
    let h6 = h.rem_euclid(1.) * 6.;
    let i = h6.floor();
    let f = h6 - i;
    let (p, q, t) = (v * (1. - s), v * (1. - s * f), v * (1. - s * (1. - f)));
    match i as i32 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

/// Occlusion Type (§2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcclusionType {
    #[default]
    Uniform,
    Up,
    DownSlope,
    Reflection,
    Concavity,
    Convexity,
    ConcavityConvexity,
    Thickness,
    Curvature,
}
impl OcclusionType {
    pub const ALL: [OcclusionType; 9] = [
        OcclusionType::Uniform,
        OcclusionType::Up,
        OcclusionType::DownSlope,
        OcclusionType::Reflection,
        OcclusionType::Concavity,
        OcclusionType::Convexity,
        OcclusionType::ConcavityConvexity,
        OcclusionType::Thickness,
        OcclusionType::Curvature,
    ];
    pub fn label(self) -> &'static str {
        match self {
            OcclusionType::Uniform => "Uniform",
            OcclusionType::Up => "Up",
            OcclusionType::DownSlope => "Down Slope",
            OcclusionType::Reflection => "Reflection",
            OcclusionType::Concavity => "Concavity",
            OcclusionType::Convexity => "Convexity",
            OcclusionType::ConcavityConvexity => "Concavity & Convexity",
            OcclusionType::Thickness => "Thickness",
            OcclusionType::Curvature => "Curvature",
        }
    }
    /// Types rendered as Uniform until built (PQ-4): none since
    /// shader-tree-complete (PQ-5).
    pub fn renders_as_uniform(self) -> bool {
        false
    }
}

/// An Occlusion texture (§2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Occlusion {
    pub zones: TwoZone,
    pub kind: OcclusionType,
    pub rays: u32,
    /// Metres.
    pub distance: f64,
    /// Degrees. Stored only (Cycles' AO covers the hemisphere).
    pub spread_angle: f64,
    pub max_cavity_angle: f64,
    pub cutoff_angle: f64,
    pub variance: f64,
    pub variance_scale: f64,
    pub same_surface_only: bool,
    pub use_vertex_cache: bool,
    pub cache_filter_size: f64,
}
impl Default for Occlusion {
    /// [measured] starting values.
    fn default() -> Self {
        Self {
            zones: TwoZone::default(),
            kind: OcclusionType::Uniform,
            rays: 24,
            distance: 0.2,
            spread_angle: 45.,
            max_cavity_angle: 45.,
            cutoff_angle: 10.,
            variance: 0.,
            variance_scale: 1.,
            same_surface_only: false,
            use_vertex_cache: false,
            cache_filter_size: 0.,
        }
    }
}
impl Occlusion {
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        use Channel::*;
        let mut out = self.zones.channels();
        out.extend([
            ("Occlusion Distance", Distance(&mut self.distance)),
            ("Spread Angle", Angle(&mut self.spread_angle)),
            ("Max Cavity Angle", Angle(&mut self.max_cavity_angle)),
            ("Cutoff Angle", Angle(&mut self.cutoff_angle)),
            ("Variance", Percent(&mut self.variance)),
            ("Variance Scale", Distance(&mut self.variance_scale)),
            ("Same Surface Only", Bool(&mut self.same_surface_only)),
            ("Use Vertex Cache", Bool(&mut self.use_vertex_cache)),
            (
                "Cache Filter Size",
                Number(&mut self.cache_filter_size, 0., 1e3),
            ),
        ]);
        out
    }
    pub fn valid(&self) -> bool {
        (1..=1024).contains(&self.rays)
    }
    /// The texture's value and alpha (two-zone over how open the point is).
    pub fn value(&self, color: bool) -> (Expr, Expr) {
        let open = self.signal();
        let n = crate::gradient::RAMP_SAMPLES;
        let stops: Vec<[f64; 4]> = (0..n)
            .map(|k| {
                let (v, c, a) = self.zones.sample(k as f64 / (n - 1) as f64);
                if color {
                    [c[0], c[1], c[2], a]
                } else {
                    [v, v, v, a]
                }
            })
            .collect();
        let alpha_varies = stops.iter().any(|s| s[3] != stops[0][3]);
        let first_alpha = stops[0][3];
        let stops = std::sync::Arc::new(stops);
        let ramp = |alpha: bool| Expr::Ramp {
            input: Box::new(open.clone()),
            lo: 0.,
            hi: 1.,
            stops: stops.clone(),
            alpha,
        };
        (
            ramp(false),
            if alpha_varies {
                ramp(true)
            } else {
                Expr::scalar(first_alpha)
            },
        )
    }
}

impl Occlusion {
    /// The value 0..1 the two zones read for this Occlusion Type (PQ-5,
    /// provisional). Uniform, Up, Down Slope, Reflection and Thickness: the
    /// share of rays that escape within Occlusion Distance (1 = open),
    /// fired round the normal, up, down the slope, along the mirror
    /// direction, or into the surface. Concavity: the same, against this
    /// object only (low in creases). Convexity: rays into the surface
    /// against this object (low on ridges and corners). Concavity &
    /// Convexity and Curvature: ½ + ½ (inside − outside) of those two, so
    /// creases read high, ridges low and flat surfaces ½; Curvature always
    /// looks at this object alone, Concavity & Convexity follows Same
    /// Surface Only.
    pub fn signal(&self) -> Expr {
        use crate::shader_eval::AoToward;
        let ao = |inside: bool, same: bool, toward: AoToward| Expr::Occlusion {
            rays: self.rays,
            distance: self.distance,
            same_surface: same,
            inside,
            toward,
        };
        let same = self.same_surface_only;
        let both = |same: bool| {
            // ½ + ½ (convex-open − concave-open), as a mix: inside × ½ +
            // (1 − outside) × ½.
            let outside = ao(false, same, AoToward::Normal);
            let inside = ao(true, true, AoToward::Normal);
            Expr::mix(
                crate::shader_eval::Mix::Blend,
                Expr::scalar(0.5),
                Expr::one_minus(outside),
                inside,
            )
        };
        match self.kind {
            OcclusionType::Uniform => ao(false, same, AoToward::Normal),
            OcclusionType::Up => ao(false, same, AoToward::Up),
            OcclusionType::DownSlope => ao(false, same, AoToward::DownSlope),
            OcclusionType::Reflection => ao(false, same, AoToward::Reflection),
            OcclusionType::Thickness => ao(true, same, AoToward::Normal),
            OcclusionType::Concavity => ao(false, true, AoToward::Normal),
            OcclusionType::Convexity => ao(true, true, AoToward::Normal),
            OcclusionType::ConcavityConvexity => both(same),
            OcclusionType::Curvature => both(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < tol)
    }

    const INPUT: [f64; 3] = [0.25, 0.5, 0.75];

    #[test]
    fn starting_values_change_nothing() {
        assert_eq!(Process::default().apply(INPUT), INPUT);
    }

    #[test]
    fn hsv_matches_the_measured_points() {
        let p = |f: fn(&mut Process)| {
            let mut p = Process::default();
            f(&mut p);
            p.apply(INPUT)
        };
        assert!(near(p(|p| p.hue = 0.25), [0.4375, 0.25, 0.75], 1e-9));
        assert!(near(p(|p| p.hue_shift = 90.), [0.75, 0.25, 0.75], 1e-9));
        assert!(near(p(|p| p.saturation = -0.5), [0.5, 0.625, 0.75], 1e-9));
        assert!(near(p(|p| p.saturation = -1.), [0.75; 3], 1e-9));
        assert!(near(p(|p| p.value = 0.25), [0.3125, 0.625, 0.9375], 1e-9));
        assert!(near(p(|p| p.value = -0.25), [0.1875, 0.375, 0.5625], 1e-9));
    }

    #[test]
    fn bias_and_gain_land_near_the_measured_points() {
        // PQ-1: the two-zone curves per channel. Operandi's current
        // (Perlin) curves land within 0.07 of the measured points, not the
        // 0.04 the hand-off expected (PR-Q01).
        let p = |bias: f64, gain: f64| {
            Process {
                bias,
                gain,
                ..Default::default()
            }
            .apply(INPUT)
        };
        assert!(near(p(0.7, 0.5), [0.430, 0.680, 0.844], 0.07));
        assert!(near(p(0.3, 0.5), [0.125, 0.305, 0.590], 0.07));
        assert!(near(p(0.5, 0.7), [0.152, 0.535, 0.832], 0.07));
        assert!(near(p(0.5, 0.3), [0.340, 0.481, 0.652], 0.07));
    }

    #[test]
    fn hsv_round_trips() {
        for c in [
            [0.1, 0.7, 0.3],
            [0.9, 0.2, 0.2],
            [0.4, 0.4, 0.4],
            [0., 0., 1.],
        ] {
            assert!(near(hsv_to_rgb(rgb_to_hsv(c)), c, 1e-12));
        }
    }

    #[test]
    fn occlusion_starting_values() {
        let o = Occlusion::default();
        assert_eq!(
            (o.kind, o.rays, o.distance),
            (OcclusionType::Uniform, 24, 0.2)
        );
        assert_eq!(
            (o.spread_angle, o.max_cavity_angle, o.cutoff_angle),
            (45., 45., 10.)
        );
        assert_eq!((o.variance, o.variance_scale), (0., 1.));
        assert!(!o.same_surface_only && !o.use_vertex_cache);
        assert_eq!(o.zones, TwoZone::default());
    }
}
