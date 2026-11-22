//! Procedural textures, stage 2 (docs/specs/BEHAVIOR-PROCEDURAL-TEXTURES.md):
//! Checker, Grid, Dots and Noise.
//!
//! A procedural layer computes a pattern value t in 0..1 at the shading
//! point's locator coordinates, reshapes it with Bias and Gain, and mixes
//! its two zones (Value, Color, Alpha) by it: zone 1 at t = 0, zone 2 at
//! t = 1. Everything else (Effect, Blend, Opacity, Invert, Locator) is the
//! shared texture layer.
//!
//! The functions here are the reference the renderer's node graphs follow
//! (operandi_cycles.cpp, `OC_EXPR_PATTERN`). Noise is our own gradient
//! noise; the renderer uses Cycles' Noise Texture per octave, so the two
//! match in scale, contrast and look, not value for value.
use crate::shader_tree::TwoZone;
use serde::{Deserialize, Serialize};

macro_rules! labelled_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),+];
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }
        }
    };
}

labelled_enum! {
    /// The procedural types Add Layer offers.
    ProceduralKind {
        Checker => "Checker",
        Grid => "Grid",
        Dots => "Dots",
        Noise => "Noise",
        Cellular => "Cellular",
        Wood => "Wood",
        Ripples => "Ripples",
        Weave => "Weave",
        GaborNoise => "Gabor Noise",
        PoissonCells => "Poisson Cells",
    }
}

labelled_enum! {
    CellularType {
        Angular => "Angular",
        Round => "Round",
    }
}

labelled_enum! {
    CheckerType {
        Square => "Square",
        Cube => "Cube",
    }
}

labelled_enum! {
    GridType {
        Square => "Square",
        Line => "Line",
        Triangle => "Triangle",
        Hexagon => "Hexagon",
        Cube => "Cube",
    }
}

labelled_enum! {
    DotsType {
        Square => "Square",
        Hexagon => "Hexagon",
        Cube => "Cube",
    }
}

labelled_enum! {
    NoiseType {
        Fractal => "Fractal",
        Simple => "Simple",
        Turbulence => "Turbulence",
    }
}

/// Per-cell variation (stored only; fractions, 0 = off).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variation {
    pub value: f64,
    pub hue: f64,
    pub saturation: f64,
    pub alpha: f64,
}

/// Noise settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoiseSettings {
    pub kind: NoiseType,
    /// Octaves, 1..=MAX_OCTAVES (Simple uses 1).
    pub frequencies: u32,
    pub frequency_ratio: f64,
    pub amplitude_ratio: f64,
    /// 0..1; 0 = off.
    pub smooth_tops: f64,
    pub flow: bool,
    /// One full cycle per 1.0.
    pub flow_phase: f64,
}
impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            kind: NoiseType::Fractal,
            frequencies: 4,
            frequency_ratio: 2.,
            amplitude_ratio: 0.5,
            smooth_tops: 0.,
            flow: false,
            flow_phase: 0.,
        }
    }
}

/// Most octaves Noise sums (the renderer builds one node per octave).
pub const MAX_OCTAVES: u32 = 8;

/// A pattern and its own settings. Widths are fractions (shown as
/// percent): of the square size for Checker, of the cell for Grid and Dots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Pattern {
    Checker {
        kind: CheckerType,
        transition: f64,
        variation: Variation,
    },
    Grid {
        kind: GridType,
        line_width: f64,
        transition: f64,
        variation: Variation,
    },
    Dots {
        kind: DotsType,
        dot_width: f64,
        transition: f64,
        variation: Variation,
    },
    Noise(NoiseSettings),
    /// Random feature points about one per locator-size cube; cells are
    /// zone 1, gaps zone 2.
    Cellular {
        kind: CellularType,
        cell_width: f64,
        transition: f64,
        /// Octaves, 1..=MAX_OCTAVES, and their ratios (as Noise).
        frequencies: u32,
        frequency_ratio: f64,
        amplitude_ratio: f64,
        smooth_tops: f64,
        variation: Variation,
    },
    /// Rings around the Projection Axis (zone 1), filler between.
    Wood {
        ring_scale: f64,
        waviness: f64,
        wave_scale: f64,
        graininess: f64,
        grain_scale: f64,
    },
    /// Circular waves from random sources: crests zone 1, troughs zone 2.
    Ripples {
        sources: u32,
        wavelength: f64,
        /// Fraction of a wavelength; positive moves the waves inward.
        phase: f64,
    },
    /// An over-under basket weave: yarn zone 1, gaps zone 2.
    Weave {
        yarn_width: f64,
        roundness: f64,
    },
    /// Stripy directional noise in the plane perpendicular to the axis
    /// (type val.noise.gabor). Brightness and Contrast replace Bias/Gain.
    Gabor {
        /// Degrees about the axis.
        orientation: f64,
        frequency: f64,
        bandwidth: f64,
        random_offset: f64,
        brightness: f64,
        contrast: f64,
    },
    /// Evenly spaced cells (type val.noise.poisson); each Level doubles
    /// the scale. Brightness and Contrast replace Bias/Gain.
    Poisson {
        kind: CellularType,
        cell_width: f64,
        transition: f64,
        density: f64,
        level: u32,
        solid: bool,
        brightness: f64,
        contrast: f64,
        frequencies: u32,
        frequency_ratio: f64,
        amplitude_ratio: f64,
        profile_gradient: bool,
        variation: Variation,
    },
}

/// Most Poisson Cells levels.
pub const MAX_LEVEL: u32 = 8;
/// Poisson Cells' feature-point randomness (low: evenly spaced).
pub const POISSON_RANDOMNESS: f64 = 0.25;

/// Most Ripples sources (the renderer builds nodes per source).
pub const MAX_SOURCES: u32 = 32;

/// A procedural texture layer's source: its two zones (with Bias and Gain)
/// and its pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Procedural {
    pub zones: TwoZone,
    pub pattern: Pattern,
}

impl Procedural {
    /// Starting values (spec: zone 1 Value 0 / black, zone 2 Value 1 /
    /// white, Alphas 100 %, Bias and Gain 50 %).
    pub fn new(kind: ProceduralKind) -> Self {
        let pattern = match kind {
            ProceduralKind::Checker => Pattern::Checker {
                kind: CheckerType::Square,
                transition: 0.1,
                variation: Variation::default(),
            },
            ProceduralKind::Grid => Pattern::Grid {
                kind: GridType::Square,
                line_width: 0.1,
                transition: 0.1,
                variation: Variation::default(),
            },
            ProceduralKind::Dots => Pattern::Dots {
                kind: DotsType::Square,
                dot_width: 0.8,
                transition: 0.1,
                variation: Variation::default(),
            },
            ProceduralKind::Noise => Pattern::Noise(NoiseSettings::default()),
            ProceduralKind::Cellular => Pattern::Cellular {
                kind: CellularType::Angular,
                cell_width: 0.6,
                transition: 0.3,
                frequencies: 1,
                frequency_ratio: 2.,
                amplitude_ratio: 0.5,
                smooth_tops: 0.,
                variation: Variation::default(),
            },
            ProceduralKind::Wood => Pattern::Wood {
                ring_scale: 0.25,
                waviness: 0.5,
                wave_scale: 1.,
                graininess: 0.5,
                grain_scale: 0.1,
            },
            ProceduralKind::Ripples => Pattern::Ripples {
                sources: 8,
                wavelength: 0.2,
                phase: 0.,
            },
            ProceduralKind::Weave => Pattern::Weave {
                yarn_width: 0.6,
                roundness: 0.6,
            },
            ProceduralKind::GaborNoise => Pattern::Gabor {
                orientation: 45.,
                frequency: 0.0625,
                bandwidth: 0.05,
                random_offset: 1.,
                brightness: 0.,
                contrast: 0.,
            },
            ProceduralKind::PoissonCells => Pattern::Poisson {
                kind: CellularType::Angular,
                cell_width: 0.9,
                transition: 0.05,
                density: 1.,
                level: 3,
                solid: false,
                brightness: 0.,
                contrast: 0.,
                frequencies: 1,
                frequency_ratio: 2.,
                amplitude_ratio: 0.5,
                profile_gradient: false,
                variation: Variation::default(),
            },
        };
        let mut zones = TwoZone::default();
        match kind {
            // Spec §2: Ring 0.4 grey / Value 0, Filler 0.6 grey / Value 1.
            ProceduralKind::Wood => {
                zones.zone1_color = [0.4; 3];
                zones.zone2_color = [0.6; 3];
            }
            // Crest / Yarn start white and 1, Trough / Gap black and 0.
            ProceduralKind::Ripples
            | ProceduralKind::Weave
            | ProceduralKind::GaborNoise
            | ProceduralKind::PoissonCells => {
                zones.zone1_color = [1.; 3];
                zones.zone1_value = 1.;
                zones.zone2_color = [0.; 3];
                zones.zone2_value = 0.;
            }
            _ => {}
        }
        Self { zones, pattern }
    }
    /// The per-cell variation amounts, when the pattern has cells.
    pub fn variation(&self) -> Option<&Variation> {
        match &self.pattern {
            Pattern::Checker { variation, .. }
            | Pattern::Grid { variation, .. }
            | Pattern::Dots { variation, .. }
            | Pattern::Cellular { variation, .. }
            | Pattern::Poisson { variation, .. } => Some(variation),
            _ => None,
        }
    }
    pub fn kind(&self) -> ProceduralKind {
        match self.pattern {
            Pattern::Checker { .. } => ProceduralKind::Checker,
            Pattern::Grid { .. } => ProceduralKind::Grid,
            Pattern::Dots { .. } => ProceduralKind::Dots,
            Pattern::Noise(_) => ProceduralKind::Noise,
            Pattern::Cellular { .. } => ProceduralKind::Cellular,
            Pattern::Wood { .. } => ProceduralKind::Wood,
            Pattern::Ripples { .. } => ProceduralKind::Ripples,
            Pattern::Weave { .. } => ProceduralKind::Weave,
            Pattern::Gabor { .. } => ProceduralKind::GaborNoise,
            Pattern::Poisson { .. } => ProceduralKind::PoissonCells,
        }
    }
    /// Brightness and Contrast, for the types where they replace Bias and
    /// Gain.
    pub fn brightness_contrast(&self) -> Option<(f64, f64)> {
        match self.pattern {
            Pattern::Gabor {
                brightness,
                contrast,
                ..
            }
            | Pattern::Poisson {
                brightness,
                contrast,
                ..
            } => Some((brightness, contrast)),
            _ => None,
        }
    }
    /// The zones' names in Properties (zone 1, zone 2).
    pub fn zone_names(&self) -> [&'static str; 2] {
        match self.pattern {
            Pattern::Checker { .. } => ["Checker 1", "Checker 2"],
            Pattern::Grid { .. } => ["Line", "Filler"],
            Pattern::Dots { .. } => ["Dot", "Filler"],
            Pattern::Noise(_) => ["Zone 1", "Zone 2"],
            Pattern::Cellular { .. } => ["Cell", "Filler"],
            Pattern::Wood { .. } => ["Ring", "Filler"],
            Pattern::Ripples { .. } => ["Crest", "Trough"],
            Pattern::Weave { .. } => ["Yarn", "Gap"],
            Pattern::Gabor { .. } => ["Zone 1", "Zone 2"],
            Pattern::Poisson { .. } => ["Cell", "Filler"],
        }
    }
    /// Every editable number, with its Properties name, zones first.
    pub fn channels(&mut self) -> Vec<(&'static str, crate::shader_tree::Channel<'_>)> {
        use crate::shader_tree::Channel::*;
        let [one, two] = self.zone_names();
        let names = zone_channel_names(one, two);
        let z = &mut self.zones;
        let mut out = vec![
            (names[0], Number(&mut z.zone1_value, -1e6, 1e6)),
            (names[1], Color(&mut z.zone1_color)),
            (names[2], Percent(&mut z.zone1_alpha)),
            (names[3], Number(&mut z.zone2_value, -1e6, 1e6)),
            (names[4], Color(&mut z.zone2_color)),
            (names[5], Percent(&mut z.zone2_alpha)),
        ];
        if !matches!(
            self.pattern,
            Pattern::Gabor { .. } | Pattern::Poisson { .. }
        ) {
            out.push(("Bias", Percent(&mut z.bias)));
            out.push(("Gain", Percent(&mut z.gain)));
        }
        let variation = variation_channels;
        match &mut self.pattern {
            Pattern::Checker {
                transition,
                variation: v,
                ..
            } => {
                out.push(("Transition Width", PercentIn(transition, 0., 100.)));
                out.extend(variation(v));
            }
            Pattern::Grid {
                line_width,
                transition,
                variation: v,
                ..
            } => {
                out.push(("Line Width", Percent(line_width)));
                out.push(("Transition Width", PercentIn(transition, 0., 100.)));
                out.extend(variation(v));
            }
            Pattern::Dots {
                dot_width,
                transition,
                variation: v,
                ..
            } => {
                out.push(("Dot Width", PercentIn(dot_width, 0., 10.)));
                out.push(("Transition Width", PercentIn(transition, 0., 100.)));
                out.extend(variation(v));
            }
            Pattern::Cellular {
                cell_width,
                transition,
                frequency_ratio,
                amplitude_ratio,
                smooth_tops,
                variation: v,
                ..
            } => {
                out.push(("Cell Width", Percent(cell_width)));
                out.push(("Transition Width", PercentIn(transition, 0., 10.)));
                out.push(("Frequency Ratio", Number(frequency_ratio, 1., 16.)));
                out.push(("Amplitude Ratio", Number(amplitude_ratio, 0., 4.)));
                out.push(("Smooth Tops", Percent(smooth_tops)));
                // Cellular has no Alpha Variation (spec §2).
                out.extend(variation(v).into_iter().take(3));
            }
            Pattern::Wood {
                ring_scale,
                waviness,
                wave_scale,
                graininess,
                grain_scale,
            } => {
                out.push(("Ring Scale", PercentIn(ring_scale, 0.001, 100.)));
                out.push(("Waviness", PercentIn(waviness, 0., 10.)));
                out.push(("Wave Scale", PercentIn(wave_scale, 0.001, 100.)));
                out.push(("Graininess", Percent(graininess)));
                out.push(("Grain Scale", PercentIn(grain_scale, 0.001, 100.)));
            }
            Pattern::Ripples {
                wavelength, phase, ..
            } => {
                out.push(("Wavelength", PercentIn(wavelength, 0.001, 100.)));
                out.push(("Phase", PercentIn(phase, -1e4, 1e4)));
            }
            Pattern::Weave {
                yarn_width,
                roundness,
            } => {
                out.push(("Yarn Width", Percent(yarn_width)));
                out.push(("Roundness", Percent(roundness)));
            }
            Pattern::Gabor {
                orientation,
                frequency,
                bandwidth,
                random_offset,
                brightness,
                contrast,
            } => {
                out.push(("Orientation", Number(orientation, -360., 360.)));
                out.push(("Frequency", Number(frequency, 0.0001, 100.)));
                out.push(("Bandwidth", Number(bandwidth, 0., 1.)));
                out.push(("Random Offset", Number(random_offset, -1e6, 1e6)));
                out.push(("Brightness", PercentIn(brightness, -1., 1.)));
                out.push(("Contrast", PercentIn(contrast, -1., 10.)));
            }
            Pattern::Poisson {
                cell_width,
                transition,
                density,
                brightness,
                contrast,
                frequency_ratio,
                amplitude_ratio,
                variation: v,
                ..
            } => {
                out.push(("Cell Width", Percent(cell_width)));
                out.push(("Transition Width", PercentIn(transition, 0., 10.)));
                out.push(("Cell Density", Percent(density)));
                out.push(("Brightness", PercentIn(brightness, -1., 1.)));
                out.push(("Contrast", PercentIn(contrast, -1., 10.)));
                out.push(("Frequency Ratio", Number(frequency_ratio, 1., 16.)));
                out.push(("Amplitude Ratio", Number(amplitude_ratio, 0., 4.)));
                // Hue / Saturation / Value only (spec §3).
                out.extend(variation(v).into_iter().take(3));
            }
            Pattern::Noise(n) => {
                out.push(("Frequency Ratio", Number(&mut n.frequency_ratio, 1., 16.)));
                out.push(("Amplitude Ratio", Number(&mut n.amplitude_ratio, 0., 4.)));
                out.push(("Smooth Tops", Percent(&mut n.smooth_tops)));
                out.push(("Flow Phase", Number(&mut n.flow_phase, -1e6, 1e6)));
            }
        }
        out
    }
    pub fn validate(&self) -> crate::Result<()> {
        let octaves = match &self.pattern {
            Pattern::Noise(n) => Some(n.frequencies),
            Pattern::Cellular { frequencies, .. } | Pattern::Poisson { frequencies, .. } => {
                Some(*frequencies)
            }
            _ => None,
        };
        if let Pattern::Poisson { level, .. } = &self.pattern
            && !(1..=MAX_LEVEL).contains(level)
        {
            return Err(format!("Level must be 1 to {MAX_LEVEL}"));
        }
        if octaves.is_some_and(|f| !(1..=MAX_OCTAVES).contains(&f)) {
            return Err(format!("Frequencies must be 1 to {MAX_OCTAVES}"));
        }
        if let Pattern::Ripples { sources, .. } = &self.pattern
            && !(1..=MAX_SOURCES).contains(sources)
        {
            return Err(format!("Sources must be 1 to {MAX_SOURCES}"));
        }
        Ok(())
    }
}

/// "<zone> Value / Color / Alpha" for both zones.
fn zone_channel_names(one: &'static str, two: &'static str) -> [&'static str; 6] {
    macro_rules! names {
        ($a:literal, $b:literal) => {
            [
                concat!($a, " Value"),
                concat!($a, " Color"),
                concat!($a, " Alpha"),
                concat!($b, " Value"),
                concat!($b, " Color"),
                concat!($b, " Alpha"),
            ]
        };
    }
    match (one, two) {
        ("Line", _) => names!("Line", "Filler"),
        ("Dot", _) => names!("Dot", "Filler"),
        ("Checker 1", _) => names!("Checker 1", "Checker 2"),
        ("Cell", _) => names!("Cell", "Filler"),
        ("Ring", _) => names!("Ring", "Filler"),
        ("Crest", _) => names!("Crest", "Trough"),
        ("Yarn", _) => names!("Yarn", "Gap"),
        _ => names!("Zone 1", "Zone 2"),
    }
}

/// The stored per-cell variation settings.
fn variation_channels(v: &mut Variation) -> Vec<(&'static str, crate::shader_tree::Channel<'_>)> {
    use crate::shader_tree::Channel::Percent;
    vec![
        ("Value Variation", Percent(&mut v.value)),
        ("Hue Variation", Percent(&mut v.hue)),
        ("Saturation Variation", Percent(&mut v.saturation)),
        ("Alpha Variation", Percent(&mut v.alpha)),
    ]
}

// ---------------------------------------------------------------------------
// Pattern maths (the reference for the renderer).

/// 0 below `e0`, 1 above `e1`, an S-shaped (smoothstep) ramp between; a
/// hard step at `e0` when the ramp has no width.
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    if e1 <= e0 {
        return if x <= e0 { 0. } else { 1. };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// The in-plane coordinates of a pattern point for 2D patterns: the plane
/// perpendicular to `axis` (as Planar images: X (-z, y), Y (x, -z),
/// Z (x, y)).
pub fn in_plane(axis: u8, p: [f64; 3]) -> [f64; 2] {
    crate::texture_locator::planar(axis, p.into(), 0.)
}

/// One axis of the checker: +1 inside squares of even index, -1 odd, easing
/// to 0 on the edges over `width` (a fraction of the square), centred on
/// the edge. Squares are 0.5 apart.
pub fn soft_parity(x: f64, width: f64) -> f64 {
    let p = 2. * x;
    let floor = p.floor();
    // 1 - 4 fract(floor / 2): +1 even, -1 odd (the renderer's sum).
    let half = floor * 0.5;
    let sign = 1. - 4. * (half - half.floor());
    let f = p - floor;
    let d = f.min(1. - f);
    sign * smoothstep(0., width * 0.5, d)
}

/// How much of a line of half-width `half` (with a ramp `transition` wide,
/// centred on the line's edge) covers a point `dist` from the line.
pub fn line_cover(dist: f64, half: f64, transition: f64) -> f64 {
    1. - smoothstep(half - transition * 0.5, half + transition * 0.5, dist)
}

/// Distance from `x` to the nearest grid line (lines at half-integers).
fn line_dist(x: f64) -> f64 {
    ((x - x.floor()) - 0.5).abs()
}

/// Distance to the edge of the hexagonal cell (flat-to-flat 1, a cell
/// centred on the origin, neighbours along X).
pub fn hex_edge_dist(u: f64, v: f64) -> f64 {
    let r = [1., 3f64.sqrt()];
    let h = [0.5, 3f64.sqrt() * 0.5];
    let modulo = |a: f64, b: f64| a - b * (a / b).floor();
    let a = [modulo(u, r[0]) - h[0], modulo(v, r[1]) - h[1]];
    let b = [modulo(u - h[0], r[0]) - h[0], modulo(v - h[1], r[1]) - h[1]];
    let g = if a[0] * a[0] + a[1] * a[1] < b[0] * b[0] + b[1] * b[1] {
        a
    } else {
        b
    };
    let (x, y) = (g[0].abs(), g[1].abs());
    0.5 - x.max(0.5 * x + h[1] * y)
}

impl Pattern {
    /// The pattern as a mode and 12 numbers, as the renderer (`OC_EXPR_PATTERN`)
    /// and the viewport (`op_pat`) read it.
    pub fn packed(&self, axis: u8) -> (u32, [f32; 12]) {
        let p = self;
        let mut out = [0f32; 12];
        out[1] = f32::from(axis.min(2));
        let mode = match p {
            Pattern::Checker {
                kind, transition, ..
            } => {
                out[0] = match kind {
                    CheckerType::Square => 0.,
                    CheckerType::Cube => 1.,
                };
                out[2] = *transition as f32;
                0
            }
            Pattern::Grid {
                kind,
                line_width,
                transition,
                ..
            } => {
                out[0] = match kind {
                    GridType::Square => 0.,
                    GridType::Line => 1.,
                    GridType::Triangle => 2.,
                    GridType::Hexagon => 3.,
                    GridType::Cube => 4.,
                };
                out[2] = *line_width as f32;
                out[3] = *transition as f32;
                1
            }
            Pattern::Dots {
                kind,
                dot_width,
                transition,
                ..
            } => {
                out[0] = match kind {
                    DotsType::Square => 0.,
                    DotsType::Hexagon => 1.,
                    DotsType::Cube => 2.,
                };
                out[2] = *dot_width as f32;
                out[3] = *transition as f32;
                2
            }
            Pattern::Noise(n) => {
                out[0] = match n.kind {
                    NoiseType::Fractal => 0.,
                    NoiseType::Simple => 1.,
                    NoiseType::Turbulence => 2.,
                };
                let octaves = match n.kind {
                    NoiseType::Simple => 1,
                    _ => n.frequencies.clamp(1, MAX_OCTAVES),
                };
                out[2] = octaves as f32;
                out[3] = n.frequency_ratio as f32;
                out[4] = n.amplitude_ratio as f32;
                out[5] = n.smooth_tops as f32;
                out[6] = if n.flow { 1. } else { 0. };
                out[7] = n.flow_phase as f32;
                3
            }
            Pattern::Cellular {
                kind,
                cell_width,
                transition,
                frequencies,
                frequency_ratio,
                amplitude_ratio,
                smooth_tops,
                ..
            } => {
                out[0] = match kind {
                    CellularType::Angular => 0.,
                    CellularType::Round => 1.,
                };
                out[2] = *cell_width as f32;
                out[3] = *transition as f32;
                out[4] = (*frequencies).clamp(1, MAX_OCTAVES) as f32;
                out[5] = *frequency_ratio as f32;
                out[6] = *amplitude_ratio as f32;
                out[7] = *smooth_tops as f32;
                4
            }
            Pattern::Wood {
                ring_scale,
                waviness,
                wave_scale,
                graininess,
                grain_scale,
            } => {
                out[2] = *ring_scale as f32;
                out[3] = *waviness as f32;
                out[4] = *wave_scale as f32;
                out[5] = *graininess as f32;
                out[6] = *grain_scale as f32;
                5
            }
            Pattern::Ripples {
                sources,
                wavelength,
                phase,
            } => {
                out[2] = (*sources).clamp(1, MAX_SOURCES) as f32;
                out[3] = *wavelength as f32;
                out[4] = *phase as f32;
                6
            }
            Pattern::Weave {
                yarn_width,
                roundness,
            } => {
                out[2] = *yarn_width as f32;
                out[3] = *roundness as f32;
                7
            }
            Pattern::Gabor {
                orientation,
                frequency,
                bandwidth,
                random_offset,
                ..
            } => {
                out[2] = orientation.to_radians() as f32;
                out[3] = (*frequency * GABOR_FREQUENCY) as f32;
                out[4] = *bandwidth as f32;
                out[5] = *random_offset as f32;
                out[6] = GABOR_SCALE as f32;
                out[7] = GABOR_SPECK.0 as f32;
                out[8] = GABOR_SPECK.1 as f32;
                8
            }
            Pattern::Poisson {
                kind,
                cell_width,
                transition,
                density,
                level,
                solid,
                frequencies,
                frequency_ratio,
                amplitude_ratio,
                profile_gradient,
                ..
            } => {
                out[0] = match kind {
                    CellularType::Angular => 0.,
                    CellularType::Round => 1.,
                };
                out[2] = (*cell_width * *density) as f32;
                out[3] = *transition as f32;
                out[4] = poisson_scale(*level) as f32;
                out[5] = if *solid { 1. } else { 0. };
                out[6] = (*frequencies).clamp(1, MAX_OCTAVES) as f32;
                out[7] = *frequency_ratio as f32;
                out[8] = *amplitude_ratio as f32;
                out[9] = if *profile_gradient { 1. } else { 0. };
                out[10] = POISSON_RANDOMNESS as f32;
                9
            }
        };
        (mode, out)
    }

    /// The pattern value t (0..1, before Bias and Gain) at point `p` of the
    /// coordinates the layer's projection gives; 2D patterns use the plane
    /// perpendicular to `axis`.
    pub fn value(&self, axis: u8, p: [f64; 3]) -> f64 {
        match self {
            Pattern::Checker {
                kind, transition, ..
            } => {
                let s = match kind {
                    CheckerType::Square => {
                        let [u, v] = in_plane(axis, p);
                        soft_parity(u, *transition) * soft_parity(v, *transition)
                    }
                    CheckerType::Cube => p.iter().map(|&c| soft_parity(c, *transition)).product(),
                };
                0.5 + 0.5 * s
            }
            Pattern::Grid {
                kind,
                line_width,
                transition,
                ..
            } => {
                let half = line_width * 0.5;
                let cover = |d: f64| line_cover(d, half, *transition);
                let [u, v] = in_plane(axis, p);
                let line = match kind {
                    GridType::Line => cover(line_dist(u)),
                    GridType::Square => cover(line_dist(u)).max(cover(line_dist(v))),
                    GridType::Triangle => {
                        let diagonal = line_dist(u - v + 0.5) / 2f64.sqrt();
                        cover(line_dist(u))
                            .max(cover(line_dist(v)))
                            .max(cover(diagonal))
                    }
                    GridType::Hexagon => cover(hex_edge_dist(u, v)),
                    GridType::Cube => p.iter().map(|&c| cover(line_dist(c))).fold(0., f64::max),
                };
                1. - line
            }
            Pattern::Dots {
                kind,
                dot_width,
                transition,
                ..
            } => {
                let round = |x: f64| (x + 0.5).floor();
                let r = match kind {
                    DotsType::Square => {
                        let [u, v] = in_plane(axis, p);
                        (u - round(u)).hypot(v - round(v))
                    }
                    DotsType::Hexagon => {
                        let [u, v] = in_plane(axis, p);
                        let row = round(v);
                        // Every other row shifted half a cell.
                        let shift = 0.5 * (row - 2. * (row * 0.5).floor());
                        let u = u - shift;
                        (u - round(u)).hypot(v - row)
                    }
                    DotsType::Cube => p
                        .iter()
                        .map(|&c| (c - round(c)).powi(2))
                        .sum::<f64>()
                        .sqrt(),
                };
                1. - line_cover(r, dot_width * 0.5, *transition)
            }
            Pattern::Noise(n) => noise_value(n, p),
            Pattern::Cellular {
                kind,
                cell_width,
                transition,
                frequencies,
                frequency_ratio,
                amplitude_ratio,
                smooth_tops,
                ..
            } => {
                let octaves = (*frequencies).clamp(1, MAX_OCTAVES);
                let (mut amplitude, mut frequency) = (1., 1.);
                let (mut first, mut rest, mut total) = (0., 0., 0.);
                for i in 0..octaves {
                    let (f1, f2, _) = cells(p.map(|c| c * frequency));
                    let d = match kind {
                        // Faceted: how far from the edge between the two
                        // nearest points (0 deep inside, 1 on the edge).
                        CellularType::Angular => 1. - (2. * (f2 - f1)).clamp(0., 1.),
                        CellularType::Round => 2. * f1,
                    };
                    let t = 1. - line_cover(d, *cell_width, *transition);
                    if i == 0 {
                        first = t;
                    } else {
                        rest += amplitude * t;
                    }
                    total += amplitude;
                    amplitude *= amplitude_ratio;
                    frequency *= frequency_ratio;
                }
                let k = 1. - smooth_tops * first.clamp(0., 1.);
                (first + k * rest) / total
            }
            Pattern::Wood {
                ring_scale,
                waviness,
                wave_scale,
                graininess,
                grain_scale,
            } => {
                let [a, b] = in_plane(axis, p);
                let wave = gradient_noise(p.map(|c| c / wave_scale.max(1e-6)), 0.);
                let r = a.hypot(b) + waviness * ring_scale * wave;
                let ring = 0.5 + 0.5 * (std::f64::consts::TAU * r / ring_scale.max(1e-6)).cos();
                let grain = 0.5 + 0.5 * gradient_noise(p.map(|c| c / grain_scale.max(1e-6)), 0.);
                let g = graininess * 0.5;
                ring * (1. - g) + grain * g
            }
            Pattern::Ripples {
                sources,
                wavelength,
                phase,
            } => {
                let n = (*sources).clamp(1, MAX_SOURCES);
                let sum: f64 = (0..n)
                    .map(|i| {
                        let c = ripple_source(i);
                        let d =
                            ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2))
                                .sqrt();
                        (std::f64::consts::TAU * (d / wavelength.max(1e-6) + phase)).cos()
                    })
                    .sum();
                // Crests (zone 1) at t = 0.
                0.5 - 0.5 * sum / f64::from(n)
            }
            Pattern::Weave {
                yarn_width,
                roundness,
            } => {
                let [u, v] = in_plane(axis, p);
                1. - weave_shade(u, v, *yarn_width, *roundness)
            }
            Pattern::Gabor {
                orientation,
                frequency,
                bandwidth,
                random_offset,
                ..
            } => {
                let [u, v] = in_plane(axis, p);
                let g = gabor(u, v, *orientation, *frequency, *bandwidth, *random_offset);
                // Mostly zone 2 with specks where the noise peaks (spec §3).
                1. - smoothstep(GABOR_SPECK.0, GABOR_SPECK.1, g.abs())
            }
            Pattern::Poisson {
                kind,
                cell_width,
                transition,
                density,
                level,
                solid,
                frequencies,
                frequency_ratio,
                amplitude_ratio,
                profile_gradient,
                ..
            } => {
                let scale = poisson_scale(*level);
                let octaves = (*frequencies).clamp(1, MAX_OCTAVES);
                let width = cell_width * density;
                let (mut amplitude, mut frequency) = (1., 1.);
                let (mut sum, mut total) = (0., 0.);
                for _ in 0..octaves {
                    let q = poisson_point(p, axis, *solid, scale * frequency);
                    let (f1, f2, _) = cells_with(q, POISSON_RANDOMNESS, !*solid);
                    let d = match kind {
                        CellularType::Angular => 1. - (2. * (f2 - f1)).clamp(0., 1.),
                        CellularType::Round => 2. * f1,
                    };
                    let t = if *profile_gradient {
                        // The ramp runs across the whole cell.
                        smoothstep(0., width.max(1e-6), d)
                    } else {
                        1. - line_cover(d, width, *transition)
                    };
                    sum += amplitude * t;
                    total += amplitude;
                    amplitude *= amplitude_ratio;
                    frequency *= frequency_ratio;
                }
                sum / total
            }
        }
    }

    /// Three random numbers (value, hue, saturation) for the cell or
    /// square the point is in, for the per-cell variations; `None` for
    /// patterns without cells.
    pub fn cell_random(&self, axis: u8, p: [f64; 3]) -> Option<[f64; 3]> {
        let round = |x: f64| (x + 0.5).floor();
        let [u, v] = in_plane(axis, p);
        let id = match self {
            Pattern::Checker { kind, .. } => match kind {
                CheckerType::Square => [(2. * u).floor(), (2. * v).floor(), 0.],
                CheckerType::Cube => p.map(|c| (2. * c).floor()),
            },
            Pattern::Grid { kind, .. } => match kind {
                GridType::Line => [round(u), 0., 0.],
                GridType::Square => [round(u), round(v), 0.],
                // Each square splits along its diagonal.
                GridType::Triangle => {
                    let d = u - v + 0.5;
                    [
                        round(u),
                        round(v),
                        if d - d.floor() > 0.5 { 1. } else { 0. },
                    ]
                }
                GridType::Hexagon => {
                    let c = hex_centre(u, v);
                    [(c[0] * 2.).round(), (c[1] * 2. / 3f64.sqrt()).round(), 0.]
                }
                GridType::Cube => p.map(round),
            },
            Pattern::Dots { kind, .. } => match kind {
                DotsType::Square => [round(u), round(v), 0.],
                DotsType::Hexagon => {
                    let row = round(v);
                    let shift = 0.5 * (row - 2. * (row * 0.5).floor());
                    [round(u - shift), row, 0.]
                }
                DotsType::Cube => p.map(round),
            },
            // Cellular and Poisson Cells take the render's own randoms: the
            // Voronoi cell colour (PT-Q13, MXG-Q11).
            Pattern::Cellular { .. } => return Some(cells(p).2),
            Pattern::Poisson { level, solid, .. } => {
                let q = poisson_point(p, axis, *solid, poisson_scale(*level));
                return Some(cells_with(q, POISSON_RANDOMNESS, !*solid).2);
            }
            _ => return None,
        };
        Some(random3(id))
    }
}

/// Distances to the nearest and second nearest feature points (one jittered
/// point per unit cell) and the nearest cell's three randoms: Cycles'
/// Voronoi texture (`cycles_voronoi`), as renders build Cellular, so the
/// viewport shows the cells the render does (MXG-Q11).
pub fn cells(p: [f64; 3]) -> (f64, f64, [f64; 3]) {
    cells_with(p, 1., false)
}

/// `cells` with Voronoi Randomness `randomness` (1 = anywhere in the cell);
/// `flat` uses 2D cells in the x, y plane (Poisson Cells without Solid 3D
/// Cells).
pub fn cells_with(p: [f64; 3], randomness: f64, flat: bool) -> (f64, f64, [f64; 3]) {
    let n = crate::cycles_voronoi::Voronoi::nearest_two(
        if flat { 2 } else { 3 },
        randomness as f32,
        p.map(|c| c as f32),
    );
    (f64::from(n.f1), f64::from(n.f2), n.color.map(f64::from))
}

/// Poisson Cells' scale: each Level doubles it (Level 1 = one cell per
/// metre).
pub fn poisson_scale(level: u32) -> f64 {
    f64::from(1u32 << level.clamp(1, MAX_LEVEL).saturating_sub(1))
}

/// Where Poisson Cells reads its cells: the in-plane coordinates (2D) or
/// the full point (Solid 3D Cells), scaled.
fn poisson_point(p: [f64; 3], axis: u8, solid: bool, scale: f64) -> [f64; 3] {
    if solid {
        p.map(|c| c * scale)
    } else {
        let [u, v] = in_plane(axis, p);
        [u * scale, v * scale, 0.]
    }
}

/// Gabor specks: where |noise| passes from the first number to the second
/// the layer turns to zone 1.
pub const GABOR_SPECK: (f64, f64) = (0.5, 0.9);
/// Gabor kernels per metre, and how the Frequency setting maps to cycles
/// per kernel (provisional, PT-Q19).
pub const GABOR_SCALE: f64 = 4.;
pub const GABOR_FREQUENCY: f64 = 32.;

/// Sparse Gabor noise (our own; the renderer uses Cycles' Gabor Texture):
/// impulses of oriented Gaussian-windowed cosines, about -1..1. Bandwidth
/// 0 aligns every kernel with Orientation; 1 orients them at random.
pub fn gabor(u: f64, v: f64, orientation: f64, frequency: f64, bandwidth: f64, offset: f64) -> f64 {
    let (x, y) = (
        u * GABOR_SCALE + offset * 17.31,
        v * GABOR_SCALE + offset * 31.71,
    );
    let f = frequency * GABOR_FREQUENCY;
    let (cx, cy) = (x.floor(), y.floor());
    let mut sum = 0.;
    for dx in -1..=1 {
        for dy in -1..=1 {
            let cell = [cx + dx as f64, cy + dy as f64];
            for k in 0..8 {
                let r = random3([cell[0], cell[1], f64::from(k)]);
                let (px, py) = (cell[0] + r[0], cell[1] + r[1]);
                let (ddx, ddy) = (x - px, y - py);
                let d2 = ddx * ddx + ddy * ddy;
                if d2 > 1. {
                    continue;
                }
                let angle = orientation.to_radians()
                    + bandwidth.clamp(0., 1.) * (r[2] - 0.5) * std::f64::consts::PI;
                let along = ddx * angle.cos() + ddy * angle.sin();
                let weight = if r[2] < 0.5 { -1. } else { 1. };
                sum += weight
                    * (-std::f64::consts::PI * d2).exp()
                    * (std::f64::consts::TAU * f * along).cos();
            }
        }
    }
    sum / 4.
}

/// Three numbers in 0..1 from an integer lattice point.
pub fn random3(c: [f64; 3]) -> [f64; 3] {
    let (x, y, z) = (c[0] as i64, c[1] as i64, c[2] as i64);
    [0u32, 1, 2].map(|k| {
        let h = hash(x.wrapping_add(i64::from(k) * 7919), y, z);
        f64::from(h >> 8) / f64::from(1u32 << 24)
    })
}

/// Ripples source `i`: a fixed pseudo-random point in the unit cube
/// centred on the locator. The renderer computes the same points.
pub fn ripple_source(i: u32) -> [f64; 3] {
    [0u32, 1, 2].map(|k| {
        let h = ripple_hash(i.wrapping_mul(3).wrapping_add(k));
        f64::from(h >> 8) / f64::from(1u32 << 24) - 0.5
    })
}

/// A 32-bit integer mix (the C++ bridge has the same function).
pub fn ripple_hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// The centre of the hexagonal cell holding (u, v) (as `hex_edge_dist`).
fn hex_centre(u: f64, v: f64) -> [f64; 2] {
    let r = [1., 3f64.sqrt()];
    let h = [0.5, 3f64.sqrt() * 0.5];
    let modulo = |a: f64, b: f64| a - b * (a / b).floor();
    let a = [modulo(u, r[0]) - h[0], modulo(v, r[1]) - h[1]];
    let b = [modulo(u - h[0], r[0]) - h[0], modulo(v - h[1], r[1]) - h[1]];
    let g = if a[0] * a[0] + a[1] * a[1] < b[0] * b[0] + b[1] * b[1] {
        a
    } else {
        b
    };
    [u - g[0], v - g[1]]
}

/// The weave's shading at (u, v): 0 in a gap, else the top yarn's profile
/// (1 flat, a tube's shading with Roundness). Yarns 0.5 apart, one
/// over-under repeat per unit; the horizontal yarn is on top where
/// floor(2u) + floor(2v) is even.
pub fn weave_shade(u: f64, v: f64, width: f64, roundness: f64) -> f64 {
    let (pu, pv) = (2. * u, 2. * v);
    let (fu, fv) = (pu - pu.floor(), pv - pv.floor());
    let sum = pu.floor() + pv.floor();
    let top_h = if (sum - 2. * (sum * 0.5).floor()) < 0.5 {
        1.
    } else {
        0.
    };
    let half = (width * 0.5).max(1e-6);
    let xh = (fv - 0.5).abs() / half;
    let xv = (fu - 0.5).abs() / half;
    let in_h = if xh < 1. { 1. } else { 0. };
    let in_v = if xv < 1. { 1. } else { 0. };
    let profile = |x: f64| 1. + roundness * ((1. - x * x).max(0.).sqrt() - 1.);
    let sel_h = in_h * (top_h + (1. - top_h) * (1. - in_v));
    let sel_v = in_v * ((1. - top_h) + top_h * (1. - in_h));
    sel_h * profile(xh) + sel_v * profile(xv)
}

/// Noise t at `p` (spec: centred on 0.5, not stretched to 0..1).
pub fn noise_value(n: &NoiseSettings, p: [f64; 3]) -> f64 {
    let octaves = match n.kind {
        NoiseType::Simple => 1,
        _ => n.frequencies.clamp(1, MAX_OCTAVES),
    };
    let phase = if n.flow { n.flow_phase } else { 0. };
    let mut amplitude = 1.;
    let mut frequency = 1.;
    let (mut first, mut rest, mut total) = (0., 0., 0.);
    for i in 0..octaves {
        let s = gradient_noise(p.map(|c| c * frequency), phase);
        let s = if n.kind == NoiseType::Turbulence {
            s.abs()
        } else {
            s
        };
        if i == 0 {
            first = s;
        } else {
            rest += amplitude * s;
        }
        total += amplitude;
        amplitude *= n.amplitude_ratio;
        frequency *= n.frequency_ratio;
    }
    // Smooth Tops: the higher the first octave puts t, the weaker the finer
    // octaves.
    let t0 = if n.kind == NoiseType::Turbulence {
        2. * first
    } else {
        0.5 + 0.5 * first
    };
    let k = 1. - n.smooth_tops * t0.clamp(0., 1.);
    let sum = (first + k * rest) / total.max(1e-12);
    if n.kind == NoiseType::Turbulence {
        (2. * sum).clamp(0., 1.)
    } else {
        0.5 + 0.5 * sum
    }
}

/// A 32-bit integer hash of a lattice point.
fn hash(x: i64, y: i64, z: i64) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

/// The gradient at a lattice point: one of the 12 cube-edge directions,
/// turned about a hashed axis by `phase` full turns (Flow Noise).
fn gradient(x: i64, y: i64, z: i64, phase: f64) -> [f64; 3] {
    const G: [[f64; 3]; 12] = [
        [1., 1., 0.],
        [-1., 1., 0.],
        [1., -1., 0.],
        [-1., -1., 0.],
        [1., 0., 1.],
        [-1., 0., 1.],
        [1., 0., -1.],
        [-1., 0., -1.],
        [0., 1., 1.],
        [0., -1., 1.],
        [0., 1., -1.],
        [0., -1., -1.],
    ];
    let h = hash(x, y, z);
    let g = G[(h % 12) as usize];
    if phase == 0. {
        return g;
    }
    // Rodrigues' rotation about an axis picked from the hash.
    let k = G[((h >> 8) % 12) as usize].map(|c| c / 2f64.sqrt());
    let angle = std::f64::consts::TAU * phase;
    let (s, c) = angle.sin_cos();
    let dot = k[0] * g[0] + k[1] * g[1] + k[2] * g[2];
    let cross = [
        k[1] * g[2] - k[2] * g[1],
        k[2] * g[0] - k[0] * g[2],
        k[0] * g[1] - k[1] * g[0],
    ];
    [0, 1, 2].map(|i| g[i] * c + cross[i] * s + k[i] * dot * (1. - c))
}

/// Smooth gradient noise, signed, roughly -1..1 (0 at lattice points).
pub fn gradient_noise(p: [f64; 3], phase: f64) -> f64 {
    let cell = p.map(|c| c.floor());
    let f = [p[0] - cell[0], p[1] - cell[1], p[2] - cell[2]];
    let fade = |t: f64| t * t * t * (t * (t * 6. - 15.) + 10.);
    let w = f.map(fade);
    let mut out = 0.;
    for corner in 0..8 {
        let o = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1].map(|b| b as f64);
        let g = gradient(
            (cell[0] + o[0]) as i64,
            (cell[1] + o[1]) as i64,
            (cell[2] + o[2]) as i64,
            phase,
        );
        let d = [f[0] - o[0], f[1] - o[1], f[2] - o[2]];
        let dot = g[0] * d[0] + g[1] * d[1] + g[2] * d[2];
        let weight: f64 = (0..3)
            .map(|i| if o[i] > 0. { w[i] } else { 1. - w[i] })
            .product();
        out += weight * dot;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker(kind: CheckerType, transition: f64) -> Pattern {
        Pattern::Checker {
            kind,
            transition,
            variation: Variation::default(),
        }
    }

    #[test]
    fn checker_squares_are_half_the_size_with_even_parity_zone_2() {
        let c = checker(CheckerType::Square, 0.);
        // Axis Z: floor(2x) + floor(2y) even -> zone 2 (t = 1).
        assert_eq!(c.value(2, [0.25, 0.25, 9.]), 1.);
        assert_eq!(c.value(2, [0.75, 0.25, 9.]), 0.);
        assert_eq!(c.value(2, [0.75, 0.75, -3.]), 1.);
        assert_eq!(c.value(2, [-0.25, 0.25, 0.]), 0.);
        // Squares end on multiples of 0.5.
        assert_eq!(c.value(2, [0.49, 0.1, 0.]), 1.);
        assert_eq!(c.value(2, [0.51, 0.1, 0.]), 0.);
        // Axis X: the plane (-z, y), as Planar images; depth (x) ignored.
        assert_eq!(c.value(0, [7., 0.25, -0.25]), 1.);
        assert_eq!(c.value(0, [7., 0.75, -0.25]), 0.);
        // Cube: zone 1 where the three floors sum odd.
        let cube = checker(CheckerType::Cube, 0.);
        assert_eq!(cube.value(2, [0.25, 0.25, 0.25]), 1.);
        assert_eq!(cube.value(2, [0.25, 0.25, 0.75]), 0.);
        assert_eq!(cube.value(2, [0.75, 0.75, 0.75]), 0.);
    }

    #[test]
    fn checker_transition_is_a_smooth_ramp_centred_on_the_edge() {
        // 10 % of a 0.5 m square: 5 cm wide, ±2.5 cm about the edge at 0.5.
        let c = checker(CheckerType::Square, 0.1);
        let at = |x: f64| c.value(2, [x, 0.25, 0.]);
        assert!((at(0.5) - 0.5).abs() < 1e-12);
        assert_eq!(at(0.474), 1.);
        assert_eq!(at(0.526), 0.);
        assert!(at(0.49) > 0.5 && at(0.49) < 1.);
        assert!(at(0.51) < 0.5 && at(0.51) > 0.);
        // Symmetric S shape.
        assert!((at(0.49) + at(0.51) - 1.).abs() < 1e-9);
        // Cube on a Planar projection (depth 0): a flat 50 % mix.
        let cube = checker(CheckerType::Cube, 0.1);
        for p in [[0.1, 0.3, 0.], [0.7, -0.2, 0.]] {
            assert!((cube.value(2, p) - 0.5).abs() < 1e-12);
        }
    }

    #[test]
    fn grid_lines_sit_on_half_cells_with_their_width() {
        let grid = |kind, w| Pattern::Grid {
            kind,
            line_width: w,
            transition: 0.,
            variation: Variation::default(),
        };
        let g = grid(GridType::Square, 0.1);
        // Lines (zone 1) at ±0.5, 5 cm each side.
        assert_eq!(g.value(2, [0.5, 0.2, 0.]), 0.);
        assert_eq!(g.value(2, [0.46, 0.2, 0.]), 0.);
        assert_eq!(g.value(2, [0.44, 0.2, 0.]), 1.);
        assert_eq!(g.value(2, [-1.5, 0.2, 0.]), 0.);
        assert_eq!(g.value(2, [0.2, 0.5, 0.]), 0.);
        assert_eq!(g.value(2, [0., 0., 0.]), 1.);
        // Line: one set only.
        let l = grid(GridType::Line, 0.1);
        assert_eq!(l.value(2, [0.5, 0.2, 0.]), 0.);
        assert_eq!(l.value(2, [0.2, 0.5, 0.]), 1.);
        // Triangle adds the diagonal through the corners.
        let t = grid(GridType::Triangle, 0.1);
        assert_eq!(t.value(2, [0.1, 0.1, 0.]), 0.);
        assert_eq!(g.value(2, [0.1, 0.1, 0.]), 1.);
        // Cube adds depth.
        let c = grid(GridType::Cube, 0.1);
        assert_eq!(c.value(2, [0.1, 0.2, 0.5]), 0.);
        assert_eq!(g.value(2, [0.1, 0.2, 0.5]), 1.);
        // Hexagon: centre is filler; the edge 0.5 out along X is line.
        let h = grid(GridType::Hexagon, 0.1);
        assert_eq!(h.value(2, [0., 0., 0.]), 1.);
        assert_eq!(h.value(2, [0.5, 0., 0.]), 0.);
        assert_eq!(h.value(2, [1., 0., 0.]), 1.);
        // All line at 100 % with no ramp.
        let full = grid(GridType::Square, 1.);
        assert_eq!(full.value(2, [0., 0., 0.]), 0.);
        // A transition softens the edge around ±(w/2).
        let soft = Pattern::Grid {
            kind: GridType::Line,
            line_width: 0.1,
            transition: 0.1,
            variation: Variation::default(),
        };
        assert!((soft.value(2, [0.45, 0., 0.]) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn dots_centres_diameter_and_hexagon_offset() {
        let dots = |kind, w| Pattern::Dots {
            kind,
            dot_width: w,
            transition: 0.,
            variation: Variation::default(),
        };
        let d = dots(DotsType::Square, 0.8);
        // Dots (zone 1) centred on integers, 0.8 m across.
        assert_eq!(d.value(2, [0., 0., 0.]), 0.);
        assert_eq!(d.value(2, [2., -1., 0.]), 0.);
        assert_eq!(d.value(2, [0.39, 0., 0.]), 0.);
        assert_eq!(d.value(2, [0.41, 0., 0.]), 1.);
        assert_eq!(d.value(2, [0.5, 0.5, 0.]), 1.);
        // Hexagon: odd rows shift half a cell.
        let h = dots(DotsType::Hexagon, 0.8);
        assert_eq!(h.value(2, [0., 0., 0.]), 0.);
        assert_eq!(h.value(2, [0., 1., 0.]), 1.);
        assert_eq!(h.value(2, [0.5, 1., 0.]), 0.);
        // Cube: spheres, so a plane off their centres cuts smaller dots.
        let c = dots(DotsType::Cube, 0.8);
        assert_eq!(c.value(2, [0.3, 0., 0.]), 0.);
        assert_eq!(c.value(2, [0.3, 0., 0.3]), 1.);
    }

    fn noise_stats(n: &NoiseSettings) -> (f64, f64, f64) {
        // A 1.4 m patch, sampled on a grid (offset off the lattice).
        let mut values = Vec::new();
        for i in 0..57 {
            for j in 0..57 {
                let p = [0.13 + i as f64 * 0.025, 0.37 + j as f64 * 0.025, 0.21];
                values.push(noise_value(n, p));
            }
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let low = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let high = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (low, high, mean)
    }

    #[test]
    fn default_noise_is_soft_low_contrast_and_centred() {
        let n = NoiseSettings::default();
        let (low, high, mean) = noise_stats(&n);
        // Spec: about 0.41..0.73, mean about 0.55; not stretched to 0..1.
        assert!((0.38..=0.62).contains(&mean), "mean {mean}");
        assert!(low > 0.15 && high < 0.85, "{low}..{high}");
        assert!(high - low > 0.1, "{low}..{high}");
        // Simple is one octave: Frequencies don't matter.
        let simple = NoiseSettings {
            kind: NoiseType::Simple,
            ..n.clone()
        };
        let p = [0.3, 0.7, 0.2];
        let s1 = noise_value(&simple, p);
        let s2 = noise_value(
            &NoiseSettings {
                frequencies: 7,
                ..simple.clone()
            },
            p,
        );
        assert_eq!(s1, s2);
        assert_eq!(
            s1,
            noise_value(
                &NoiseSettings {
                    frequencies: 1,
                    ..n.clone()
                },
                p
            )
        );
        // Turbulence differs and creases toward 0.
        let turbulence = NoiseSettings {
            kind: NoiseType::Turbulence,
            ..n.clone()
        };
        assert_ne!(noise_value(&turbulence, p), noise_value(&n, p));
        let (low, _, _) = noise_stats(&turbulence);
        assert!(low < 0.2, "turbulence valleys {low}");
        // Flow Phase animates in place, one cycle per 1.0.
        let flow = |phase| NoiseSettings {
            flow: true,
            flow_phase: phase,
            ..n.clone()
        };
        assert_ne!(noise_value(&flow(0.25), p), noise_value(&n, p));
        assert!((noise_value(&flow(1.), p) - noise_value(&n, p)).abs() < 1e-9);
        // Smooth Tops makes the fine octaves weaker where t is high.
        let smooth = NoiseSettings {
            smooth_tops: 1.,
            ..n.clone()
        };
        assert_ne!(noise_value(&smooth, p), noise_value(&n, p));
    }

    #[test]
    fn starting_values() {
        for kind in ProceduralKind::ALL {
            let p = Procedural::new(*kind);
            assert_eq!(p.kind(), *kind);
        }
        // Stage 2's four use the common starts (stage 4 has exceptions).
        for kind in &ProceduralKind::ALL[..4] {
            let p = Procedural::new(*kind);
            assert_eq!(p.zones, TwoZone::default());
            assert_eq!(p.zones.zone1_color, [0.; 3]);
            assert_eq!(p.zones.zone2_color, [1.; 3]);
            assert_eq!((p.zones.bias, p.zones.gain), (0.5, 0.5));
        }
        assert!(matches!(
            Procedural::new(ProceduralKind::Grid).pattern,
            Pattern::Grid { kind: GridType::Square, line_width, transition, .. }
                if line_width == 0.1 && transition == 0.1
        ));
        assert!(matches!(
            Procedural::new(ProceduralKind::Dots).pattern,
            Pattern::Dots { kind: DotsType::Square, dot_width, .. } if dot_width == 0.8
        ));
        assert_eq!(
            Procedural::new(ProceduralKind::Noise).pattern,
            Pattern::Noise(NoiseSettings::default())
        );
    }
}

#[cfg(test)]
mod stage4_tests {
    use super::*;

    #[test]
    fn cellular_cells_sit_on_feature_points_about_one_per_cube() {
        let c = Procedural::new(ProceduralKind::Cellular).pattern;
        // A feature point is well inside its cell (mostly zone 1).
        let near =
            |p: [f64; 3]| crate::cycles_voronoi::Voronoi::nearest_two(3, 1., p.map(|c| c as f32));
        let j = near([0.5; 3]).point.map(f64::from);
        assert!(c.value(0, j) < 0.5, "{}", c.value(0, j));
        // Halfway between two feature points is filler (Angular: on the edge).
        let q = near([1.5, 0.5, 0.5]).point.map(f64::from);
        let mid = [0, 1, 2].map(|i| (j[i] + q[i]) * 0.5);
        let (f1, f2, _) = cells(mid);
        if (f1 - f2).abs() < 1e-6 {
            assert!(c.value(0, mid) > 0.99);
        }
        // About one cell per cubic metre: over a 10 m cube about 1000 cells,
        // plus those reaching in across its sides.
        let mut ids = std::collections::BTreeSet::new();
        for i in 0..40 {
            for j in 0..40 {
                for k in 0..40 {
                    let p = [i as f64 * 0.25, j as f64 * 0.25, k as f64 * 0.25];
                    // Each cell's three randoms tell it apart.
                    let id = cells(p).2;
                    ids.insert(id.map(|v| (v * 1e6) as i64));
                }
            }
        }
        assert!((1000..=1500).contains(&ids.len()), "{} cells", ids.len());
        // Round differs from Angular; more Frequencies layer finer copies.
        let round = Pattern::Cellular {
            kind: CellularType::Round,
            cell_width: 0.6,
            transition: 0.3,
            frequencies: 1,
            frequency_ratio: 2.,
            amplitude_ratio: 0.5,
            smooth_tops: 0.,
            variation: Variation::default(),
        };
        let p = [0.37, 0.81, 0.15];
        assert!(
            (0..40).any(|i| {
                let q = [i as f64 * 0.05, 0.3, 0.6];
                (round.value(0, q) - c.value(0, q)).abs() > 0.05
            }),
            "Round looks like Angular"
        );
        if let Pattern::Cellular { frequencies, .. } = &mut c.clone() {
            *frequencies = 3;
        }
        let mut layered = c.clone();
        if let Pattern::Cellular { frequencies, .. } = &mut layered {
            *frequencies = 3;
        }
        assert!((0.0..=1.0).contains(&layered.value(0, p)));
    }

    #[test]
    fn wood_rings_are_ring_scale_apart_around_the_axis() {
        let wood = Pattern::Wood {
            ring_scale: 0.25,
            waviness: 0.,
            wave_scale: 1.,
            graininess: 0.,
            grain_scale: 0.1,
        };
        // Axis Z: rings in the X-Y plane; filler at the centre, a ring
        // (zone 1) half a spacing out, filler again a spacing out.
        assert!((wood.value(2, [0., 0., 3.]) - 1.).abs() < 1e-9);
        assert!(wood.value(2, [0.125, 0., 0.]).abs() < 1e-9);
        assert!((wood.value(2, [0., 0.25, -1.]) - 1.).abs() < 1e-9);
        // Waviness and grain move it.
        let wavy = Procedural::new(ProceduralKind::Wood).pattern;
        assert_ne!(
            wavy.value(2, [0.3, 0.1, 0.2]),
            wood.value(2, [0.3, 0.1, 0.2])
        );
        let start = Procedural::new(ProceduralKind::Wood);
        assert_eq!(start.zones.zone1_color, [0.4; 3]);
        assert_eq!(start.zones.zone2_color, [0.6; 3]);
        assert_eq!((start.zones.zone1_value, start.zones.zone2_value), (0., 1.));
    }

    #[test]
    fn ripples_crest_at_a_source_and_phase_moves_them() {
        let one = |phase| Pattern::Ripples {
            sources: 1,
            wavelength: 0.2,
            phase,
        };
        let c = ripple_source(0);
        assert!(c.iter().all(|v| (-0.5..0.5).contains(v)));
        assert!(one(0.).value(0, c).abs() < 1e-9);
        assert!((one(0.5).value(0, c) - 1.).abs() < 1e-9);
        // A wavelength out from the source: a crest again.
        let out = [c[0] + 0.2, c[1], c[2]];
        assert!(one(0.).value(0, out).abs() < 1e-9);
        // Eight sources, rescaled into 0..1.
        let eight = Procedural::new(ProceduralKind::Ripples).pattern;
        for p in [[0.1, 0.2, 0.3], [2., -1., 0.5]] {
            assert!((0.0..=1.0).contains(&eight.value(0, p)));
        }
        let start = Procedural::new(ProceduralKind::Ripples);
        assert_eq!(
            (start.zones.zone1_color, start.zones.zone2_color),
            ([1.; 3], [0.; 3])
        );
        assert_eq!(start.zone_names(), ["Crest", "Trough"]);
        // The renderer's hash is the same function.
        assert_eq!(ripple_hash(0), 0);
        assert_ne!(ripple_hash(1), ripple_hash(2));
    }

    #[test]
    fn weave_yarns_gaps_and_roundness() {
        let weave = |w, r| Pattern::Weave {
            yarn_width: w,
            roundness: r,
        };
        let flat = weave(0.6, 0.);
        // Yarn centres are yarn (zone 1); cell corners are gaps.
        assert_eq!(flat.value(2, [0.25, 0.25, 0.]), 0.);
        assert_eq!(flat.value(2, [0., 0., 0.]), 1.);
        // Full width closes the gaps.
        assert_eq!(weave(1., 0.).value(2, [0.01, 0.01, 0.]), 0.);
        // Roundness shades across the strand.
        let round = weave(0.6, 1.);
        let across = round.value(2, [0.25, 0.25 + 0.075, 0.]);
        assert!(across > 0. && across < 1., "{across}");
        // Over-under: the top yarn alternates cell to cell.
        assert!((weave_shade(0.25, 0.25, 0.6, 0.) - 1.).abs() < 1e-12);
        assert_eq!(
            Procedural::new(ProceduralKind::Weave).zone_names(),
            ["Yarn", "Gap"]
        );
    }

    #[test]
    fn per_cell_randoms_and_hsv_variation() {
        let checker = Procedural::new(ProceduralKind::Checker).pattern;
        let a = checker.cell_random(2, [0.1, 0.1, 0.]).unwrap();
        assert_eq!(a, checker.cell_random(2, [0.4, 0.3, 5.]).unwrap());
        assert_ne!(a, checker.cell_random(2, [0.6, 0.1, 0.]).unwrap());
        assert!(
            Procedural::new(ProceduralKind::Noise)
                .pattern
                .cell_random(2, [0.; 3])
                .is_none()
        );
        use crate::shader_eval::hsv_vary;
        let c = [0.8, 0.4, 0.2];
        assert_eq!(hsv_vary(c, [0.3, 0.5, 0.9], [0.; 3]), c);
        let dark = hsv_vary(c, [1., 0.5, 0.], [1., 0., 0.]);
        assert!(dark.iter().all(|v| v.abs() < 1e-12));
        let grey = hsv_vary(c, [0., 0.5, 1.], [0., 0., 1.]);
        assert!((grey[0] - grey[1]).abs() < 1e-12 && (grey[1] - grey[2]).abs() < 1e-12);
        let shifted = hsv_vary(c, [0., 1., 0.], [0., 1., 0.]);
        assert!((shifted[0] - c[0]).abs() > 0.1);
    }
}

#[cfg(test)]
mod stage5_tests {
    use super::*;

    fn gabor_pattern(bandwidth: f64) -> Pattern {
        Pattern::Gabor {
            orientation: 45.,
            frequency: 0.0625,
            bandwidth,
            random_offset: 1.,
            brightness: 0.,
            contrast: 0.,
        }
    }

    #[test]
    fn gabor_default_is_mostly_zone_2_with_specks() {
        let g = Procedural::new(ProceduralKind::GaborNoise);
        assert_eq!(
            (g.zones.zone1_color, g.zones.zone2_color),
            ([1.; 3], [0.; 3])
        );
        assert_eq!(g.brightness_contrast(), Some((0., 0.)));
        let mut high = 0;
        let mut low = 0;
        for i in 0..60 {
            for j in 0..60 {
                let t = g.pattern.value(2, [i as f64 * 0.05, j as f64 * 0.05, 0.]);
                if t > 0.9 {
                    high += 1;
                }
                if t < 0.5 {
                    low += 1;
                }
            }
        }
        assert!(high > 3600 * 6 / 10, "{high} of 3600 in zone 2");
        assert!(low > 0, "no specks");
        // Orientation turns the stripes: with clean stripes (bandwidth 0)
        // the noise changes less along the stripes than across them.
        let raw = |x: f64, y: f64| gabor(x, y, 45., 0.0625, 0., 1.);
        let (mut along, mut across) = (0., 0.);
        let step = 0.01;
        let d = std::f64::consts::FRAC_1_SQRT_2 * step;
        for i in 0..400 {
            let (x, y) = (0.3 + i as f64 * 0.007, 0.1 + i as f64 * 0.003);
            // Stripes run across the orientation direction.
            along += (raw(x - d, y + d) - raw(x, y)).abs();
            across += (raw(x + d, y + d) - raw(x, y)).abs();
        }
        assert!(along < across, "{along} vs {across}");
        assert_ne!(
            gabor(0.3, 0.2, 45., 0.0625, 1., 1.),
            gabor(0.3, 0.2, 45., 0.0625, 0., 1.)
        );
        assert!((0.0..=1.0).contains(&gabor_pattern(1.).value(2, [0.3, 0.2, 0.])));
    }

    #[test]
    fn poisson_cells_are_even_and_level_doubles_the_scale() {
        let p = Procedural::new(ProceduralKind::PoissonCells);
        assert_eq!(p.zone_names(), ["Cell", "Filler"]);
        assert_eq!(poisson_scale(3), 4.);
        assert_eq!(poisson_scale(4), 8.);
        // Cells in a 1 m square at Level 3: about 4 × 4.
        let mut ids = std::collections::BTreeSet::new();
        for i in 0..80 {
            for j in 0..80 {
                let (u, v) = (i as f64 / 80., j as f64 / 80.);
                let id = cells_with([u * 4., v * 4., 0.], POISSON_RANDOMNESS, true).2;
                ids.insert(id.map(|c| (c * 1e6) as i64));
            }
        }
        // (plus cells reaching in across the square's sides)
        assert!((16..=32).contains(&ids.len()), "{} cells", ids.len());
        // Each feature point sits near its slot's corner: Cycles' Voronoi
        // offsets points from the cell's corner by Randomness (low here).
        let point = crate::cycles_voronoi::Voronoi::nearest_two(
            2,
            POISSON_RANDOMNESS as f32,
            [2.1, 1.1, 0.],
        )
        .point
        .map(f64::from);
        assert!(
            (0..2).all(|k| (0.0..=POISSON_RANDOMNESS).contains(&(point[k] - point[k].floor())))
        );
        // Cells (zone 1) at feature points, filler between with Angular.
        let centre = [point[0] / 4., point[1] / 4., 0.];
        assert!(p.pattern.value(2, centre) < 0.5);
        // Density shrinks the cells.
        let mut sparse = p.clone();
        if let Pattern::Poisson { density, .. } = &mut sparse.pattern {
            *density = 0.3;
        }
        let share = |pat: &Pattern| {
            (0..40 * 40)
                .filter(|k| pat.value(2, [(k % 40) as f64 / 40., (k / 40) as f64 / 40., 0.]) < 0.5)
                .count()
        };
        assert!(share(&sparse.pattern) < share(&p.pattern));
    }

    #[test]
    fn brightness_and_contrast_replace_bias_and_gain() {
        let mut p = Procedural::new(ProceduralKind::PoissonCells);
        let names: Vec<&str> = p.channels().into_iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"Brightness") && names.contains(&"Contrast"));
        assert!(!names.contains(&"Bias") && !names.contains(&"Gain"));
        let mut c = Procedural::new(ProceduralKind::Checker);
        let names: Vec<&str> = c.channels().into_iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"Bias"));
    }
}

#[cfg(test)]
mod layer_tests {
    use super::*;
    use crate::shader_eval::{Ch, Context, ShadingPoint};
    use crate::shader_tree::{
        NewLayer, NodeId, NodeKind, Projection, ShaderTreeEdit, TextureEffect, TextureSource,
    };
    use crate::{Command, Editor};
    use glam::DVec3;

    fn edit(e: &mut Editor, edit: ShaderTreeEdit) {
        e.dispatch(Command::ShaderTree(edit)).unwrap();
    }
    fn add(e: &mut Editor, kind: ProceduralKind) -> NodeId {
        let id = e.scene.shader_tree.next_id;
        edit(
            e,
            ShaderTreeEdit::Add {
                layer: NewLayer::Procedural(kind),
                selected: None,
            },
        );
        id
    }
    /// Change a procedural layer (its node) in one step.
    fn change(e: &mut Editor, id: NodeId, f: impl FnOnce(&mut crate::shader_tree::Node)) {
        let mut node = e.scene.shader_tree.get(id).unwrap().clone();
        f(&mut node);
        edit(
            e,
            ShaderTreeEdit::Replace {
                node,
                continuing: false,
            },
        );
    }
    fn procedural(n: &mut crate::shader_tree::Node) -> &mut Procedural {
        match &mut n.kind {
            NodeKind::Texture(t) => match &mut t.source {
                TextureSource::Procedural(p) => p,
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
    fn locator_of(e: &Editor, id: NodeId) -> u64 {
        match &e.scene.shader_tree.get(id).unwrap().kind {
            NodeKind::Texture(t) => t.locator.unwrap(),
            _ => panic!(),
        }
    }
    fn set_locator(
        e: &mut Editor,
        locator: u64,
        f: impl FnOnce(&mut crate::texture_locator::TextureLocator),
    ) {
        let item = e.scene.item(locator).unwrap();
        let mut settings = item.texture_locator().unwrap().clone();
        f(&mut settings);
        let transform = item.transform.clone();
        edit(
            e,
            ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(settings),
                transform,
                continuing: false,
            },
        );
    }
    /// A mesh item (a plane), its Base Material's diffuse colour blue.
    fn editor() -> (Editor, u64) {
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        let item = e.scene.selection.item.unwrap();
        let mut base = e.scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.diffuse_color = [0., 0., 1.];
            m.roughness = 0.4;
        }
        e.scene.shader_tree.replace(base).unwrap();
        (e, item)
    }
    fn channel_at(e: &Editor, item: u64, ch: Ch, p: [f64; 3], uv: [f64; 2]) -> [f64; 3] {
        let frames = e.scene.locator_frames(item);
        let ctx = Context {
            item,
            ancestors: &[],
            material: crate::material::DEFAULT_MATERIAL,
            part: None,
            sets: &[],
            locators: &frames,
        };
        e.scene.shader_tree.evaluate(&ctx).get(ch).eval_at(
            &ShadingPoint {
                position: p,
                uv,
                ..Default::default()
            },
            &|_, _| [0.; 4],
        )
    }
    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
    }

    #[test]
    fn add_lands_above_base_material_with_a_solid_locator_in_one_step() {
        let (mut e, _) = editor();
        let before = e.scene.clone();
        let id = add(&mut e, ProceduralKind::Checker);
        let tree = &e.scene.shader_tree;
        let top: Vec<NodeId> = tree.children(None).map(|n| n.id).collect();
        // Below the Base Shader, directly above the Base Material.
        assert_eq!(top, vec![1, 2, 3, id, 4]);
        let node = tree.get(id).unwrap();
        assert_eq!(node.name, "Checker");
        assert_eq!(node.type_label(), "Checker");
        let NodeKind::Texture(t) = &node.kind else {
            panic!()
        };
        assert_eq!(t.effect, TextureEffect::DiffuseColor);
        let locator = t.locator.unwrap();
        let l = e.scene.texture_locator(locator).unwrap();
        assert_eq!((l.projection, l.axis), (Projection::Solid, 0));
        let item = e.scene.item(locator).unwrap();
        assert_eq!(item.transform, Default::default());
        assert_eq!(item.name, "Checker (Texture)");
        // One undo step: layer, locator and the new Texture Group.
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        // Each setting edit is one step.
        e.dispatch(Command::Redo).unwrap();
        let undo = e.undo.len();
        change(&mut e, id, |n| procedural(n).zones.gain = 0.7);
        assert_eq!(e.undo.len(), undo + 1);
    }

    #[test]
    fn zones_mix_colour_and_value_effects_and_alpha_fades_the_layer() {
        let (mut e, item) = editor();
        let id = add(&mut e, ProceduralKind::Checker);
        let locator = locator_of(&e, id);
        // Axis Z, hard edges: (0.25, 0.25) is zone 2, (0.75, 0.25) zone 1.
        set_locator(&mut e, locator, |l| l.axis = 2);
        change(&mut e, id, |n| {
            let p = procedural(n);
            p.zones.zone1_color = [1., 0., 0.];
            p.zones.zone2_color = [0., 1., 0.];
            p.zones.zone1_value = 0.2;
            p.zones.zone2_value = 0.9;
            if let Pattern::Checker { transition, .. } = &mut p.pattern {
                *transition = 0.;
            }
        });
        let at = |e: &Editor, ch, x| channel_at(e, item, ch, [x, 0.25, 0.], [0.; 2]);
        assert!(close(at(&e, Ch::DiffuseColor, 0.25), [0., 1., 0.]));
        assert!(close(at(&e, Ch::DiffuseColor, 0.75), [1., 0., 0.]));
        // A number effect mixes the values.
        change(&mut e, id, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = TextureEffect::Roughness;
            }
        });
        assert!(close(at(&e, Ch::Roughness, 0.25), [0.9; 3]));
        assert!(close(at(&e, Ch::Roughness, 0.75), [0.2; 3]));
        // Zone 1 alpha 0: the layer below (Roughness 0.4) shows there.
        change(&mut e, id, |n| procedural(n).zones.zone1_alpha = 0.);
        assert!(close(at(&e, Ch::Roughness, 0.75), [0.4; 3]));
        assert!(close(at(&e, Ch::Roughness, 0.25), [0.9; 3]));
        // Alpha multiplies Opacity: 50 % alpha × 50 % opacity = 25 %.
        change(&mut e, id, |n| {
            procedural(n).zones.zone2_alpha = 0.5;
            n.layer.opacity = 0.5;
        });
        assert!(close(at(&e, Ch::Roughness, 0.25), [0.4 + 0.25 * 0.5; 3]));
        // Invert applies after the zones, as for Constant.
        change(&mut e, id, |n| {
            procedural(n).zones.zone2_alpha = 1.;
            n.layer.opacity = 1.;
            n.layer.invert = true;
        });
        assert!(close(at(&e, Ch::Roughness, 0.25), [0.1; 3]));
    }

    #[test]
    fn bias_and_gain_reshape_t() {
        let (mut e, item) = editor();
        let id = add(&mut e, ProceduralKind::Checker);
        let locator = locator_of(&e, id);
        set_locator(&mut e, locator, |l| l.axis = 2);
        // On an edge t = 0.5 (zone mix 0.5 grey with the defaults).
        let edge = |e: &Editor| channel_at(e, item, Ch::DiffuseColor, [0.5, 0.25, 0.], [0.; 2]);
        assert!(close(edge(&e), [0.5; 3]));
        change(&mut e, id, |n| procedural(n).zones.bias = 0.8);
        let t = TwoZone::bias(0.8, 0.5);
        assert!(t > 0.5);
        assert!(close(edge(&e), [t; 3]));
        change(&mut e, id, |n| {
            let z = &mut procedural(n).zones;
            z.bias = 0.5;
            z.gain = 0.8;
        });
        // Gain leaves 0.5 where it is and steepens around it.
        assert!(close(edge(&e), [0.5; 3]));
        let near = channel_at(&e, item, Ch::DiffuseColor, [0.49, 0.25, 0.], [0.; 2])[0];
        let plain = TwoZone::gain(0.5, 0.5 + 0.5 * soft_parity(0.49, 0.1));
        assert!(near > plain, "{near} vs {plain}");
    }

    #[test]
    fn projections_feed_the_pattern() {
        let (mut e, item) = editor();
        let id = add(&mut e, ProceduralKind::Checker);
        let locator = locator_of(&e, id);
        change(&mut e, id, |n| {
            if let Pattern::Checker { transition, .. } = &mut procedural(n).pattern {
                *transition = 0.;
            }
        });
        let at = |e: &Editor, p, uv| channel_at(e, item, Ch::DiffuseColor, p, uv)[0];
        // Solid, axis X: the plane (-Lz, Ly); X ignored.
        assert_eq!(at(&e, [9., 0.25, -0.25], [0.; 2]), 1.);
        assert_eq!(at(&e, [9., 0.75, -0.25], [0.; 2]), 0.);
        // Size scales the squares: at 2 m they are 1 m.
        let item_mut = e.scene.item(locator).unwrap();
        let mut transform = item_mut.transform.clone();
        transform.scale = DVec3::splat(2.);
        let settings = item_mut.texture_locator().unwrap().clone();
        edit(
            &mut e,
            ShaderTreeEdit::SetLocator {
                locator,
                settings: Box::new(settings),
                transform,
                continuing: false,
            },
        );
        assert_eq!(at(&e, [0., 0.75, -0.25], [0.; 2]), 1.);
        // Planar Z: the pattern gets (Lx, Ly, 0); Cube becomes a flat 50 %.
        set_locator(&mut e, locator, |l| {
            l.projection = Projection::Planar;
            l.axis = 2;
        });
        assert_eq!(at(&e, [0.5, 0.5, 7.], [0.; 2]), 1.);
        assert_eq!(at(&e, [1.5, 0.5, 7.], [0.; 2]), 0.);
        change(&mut e, id, |n| {
            procedural(n).pattern = Pattern::Checker {
                kind: CheckerType::Cube,
                transition: 0.1,
                variation: Variation::default(),
            }
        });
        assert!((at(&e, [0.3, 0.6, 7.], [0.; 2]) - 0.5).abs() < 1e-9);
        // UV Map: (U × repeat U, V × repeat V); UV Rotation and tiling
        // don't apply.
        change(&mut e, id, |n| {
            procedural(n).pattern = Pattern::Checker {
                kind: CheckerType::Square,
                transition: 0.,
                variation: Variation::default(),
            }
        });
        set_locator(&mut e, locator, |l| {
            l.projection = Projection::UvMap;
            l.repeat = [2., 1.];
            l.uv_rotation = 90.;
            l.tiling = [crate::texture_locator::Tiling::Reset; 2];
        });
        // U = 0.2 × 2 = 0.4, V = 0.3: both in the first square: zone 2.
        assert_eq!(at(&e, [0.; 3], [0.2, 0.3]), 1.);
        // U = 0.3 × 2 = 0.6: next square over.
        assert_eq!(at(&e, [0.; 3], [0.3, 0.3]), 0.);
        // Far outside 0..1: no Reset for procedurals.
        assert_eq!(at(&e, [0.; 3], [2.1, 0.3]), 1.);
    }

    #[test]
    fn the_viewport_average_is_the_zone_mix_at_half() {
        let (mut e, item) = editor();
        let id = add(&mut e, ProceduralKind::Noise);
        let frames = e.scene.locator_frames(item);
        let ctx = Context {
            item,
            ancestors: &[],
            material: "Default",
            part: None,
            sets: &[],
            locators: &frames,
        };
        let s = e.scene.shader_tree.evaluate(&ctx);
        assert!(close(
            s.get(Ch::DiffuseColor).eval(&|_, _| [0.; 4]),
            [0.5; 3]
        ));
        // Bias doesn't move the flat average (t = 0.5 is shown as it is).
        change(&mut e, id, |n| procedural(n).zones.bias = 0.9);
        let frames = e.scene.locator_frames(item);
        let s = e.scene.shader_tree.evaluate(&Context {
            locators: &frames,
            ..ctx
        });
        assert!(close(
            s.get(Ch::DiffuseColor).eval(&|_, _| [0.; 4]),
            [0.5; 3]
        ));
    }

    #[test]
    fn procedurals_save_with_their_version_and_reopen() {
        let (mut e, _) = editor();
        let mut ids = Vec::new();
        for kind in &ProceduralKind::ALL[..4] {
            ids.push(add(&mut e, *kind));
        }
        change(&mut e, ids[3], |n| {
            if let Pattern::Noise(s) = &mut procedural(n).pattern {
                s.kind = NoiseType::Turbulence;
                s.frequencies = 6;
                s.flow = true;
                s.flow_phase = 0.3;
                s.smooth_tops = 0.2;
            }
        });
        change(&mut e, ids[0], |n| {
            if let Pattern::Checker { variation, .. } = &mut procedural(n).pattern {
                variation.hue = 0.4;
            }
        });
        // A pre-OpenPBR material, so the older version is what is tested.
        e.scene.shader_tree.legacy_openpbr();
        let bytes = crate::persistence::encode(&e.scene).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("\"version\": 22"), "{}", &text[..200]);
        assert_eq!(crate::persistence::decode(&bytes).unwrap(), e.scene);
        let old = text.replace("\"version\": 22", "\"version\": 21");
        assert!(crate::persistence::decode(old.as_bytes()).is_err());
        // Out-of-range settings are refused.
        let mut node = e.scene.shader_tree.get(ids[3]).unwrap().clone();
        if let Pattern::Noise(s) = &mut procedural(&mut node).pattern {
            s.frequencies = 0;
        }
        assert!(
            e.dispatch(Command::ShaderTree(ShaderTreeEdit::Replace {
                node,
                continuing: false
            }))
            .is_err()
        );
    }
}
