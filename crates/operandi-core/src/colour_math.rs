//! Colour maths for the shared colour field and picker (colour-nav note
//! 07): HSV with an unbounded value (HDR), the swatch drags, Kelvin, the
//! artist (RYB) wheel and Stops. Values are linear RGB, as stored.
//!
//! Drag rates are in float units per pixel so every Color Unit System feels
//! the same (A6).

/// Float units per pixel for component, brightness and saturation drags.
pub const DRAG_RATE: f64 = 0.005;
/// Degrees of hue per pixel for a hue drag.
pub const HUE_RATE: f64 = 0.5;
/// Kelvin range of the picker's Kelvin model (OP) and its default.
pub const KELVIN_MIN: f64 = 1000.;
pub const KELVIN_MAX: f64 = 12000.;
pub const KELVIN_DEFAULT: f64 = 6500.;

/// Hue in degrees [0, 360), saturation 0–1, value ≥ 0 (unbounded for HDR).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
}

pub fn rgb_to_hsv([r, g, b]: [f64; 3]) -> Hsv {
    let (r, g, b) = (r.max(0.), g.max(0.), b.max(0.));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0. {
        0.
    } else if max == r {
        60. * ((g - b) / d).rem_euclid(6.)
    } else if max == g {
        60. * ((b - r) / d + 2.)
    } else {
        60. * ((r - g) / d + 4.)
    };
    let s = if max > 0. { d / max } else { 0. };
    Hsv {
        h: h.rem_euclid(360.),
        s,
        v: max,
    }
}

pub fn hsv_to_rgb(Hsv { h, s, v }: Hsv) -> [f64; 3] {
    let s = s.clamp(0., 1.);
    let v = v.max(0.);
    let h = h.rem_euclid(360.) / 60.;
    let c = v * s;
    let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

/// Which drag a press on the swatch numbers starts, chosen by the modifier
/// held at press time (A2/A3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwatchDrag {
    /// No modifier: the pressed component alone (0 R, 1 G, 2 B).
    Component(usize),
    /// Shift: HSV value, R, G and B scaled together.
    Brightness,
    /// Ctrl: hue, wrapping at 360°.
    Hue,
    /// Alt: saturation 0–1.
    Saturation,
}

impl SwatchDrag {
    /// The drag for a press on component `index` with these modifiers. Shift
    /// wins over Ctrl, Ctrl over Alt (only one applies).
    pub fn from_modifiers(index: usize, shift: bool, ctrl: bool, alt: bool) -> Self {
        if shift {
            Self::Brightness
        } else if ctrl {
            Self::Hue
        } else if alt {
            Self::Saturation
        } else {
            Self::Component(index.min(2))
        }
    }
}

/// The state captured when a swatch drag starts, so the whole drag is
/// computed from its start (no drift) and the hue survives passing through
/// grey or black (A3, OP).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragStart {
    pub rgb: [f64; 3],
    pub hsv: Hsv,
    pub drag: SwatchDrag,
    /// Clamp to 1 (UI/theme colours) or not (scene colours, HDR).
    pub hdr: bool,
}

impl DragStart {
    pub fn new(rgb: [f64; 3], drag: SwatchDrag, hdr: bool) -> Self {
        Self {
            rgb,
            hsv: rgb_to_hsv(rgb),
            drag,
            hdr,
        }
    }

    fn top(&self) -> f64 {
        if self.hdr { f64::INFINITY } else { 1. }
    }

    /// The colour after dragging `dx` pixels right (negative = left) from
    /// the start.
    pub fn at(&self, dx: f64) -> [f64; 3] {
        let top = self.top();
        match self.drag {
            SwatchDrag::Component(i) => {
                let mut out = self.rgb;
                out[i] = (self.rgb[i] + dx * DRAG_RATE).clamp(0., top);
                out
            }
            SwatchDrag::Brightness => {
                let v = (self.hsv.v + dx * DRAG_RATE).clamp(0., top);
                if self.hsv.v <= 1e-12 {
                    // From pure black, step to grey so it can brighten.
                    [v; 3]
                } else {
                    let k = v / self.hsv.v;
                    self.rgb.map(|c| (c.max(0.) * k).min(top))
                }
            }
            SwatchDrag::Hue => hsv_to_rgb(Hsv {
                h: (self.hsv.h + dx * HUE_RATE).rem_euclid(360.),
                ..self.hsv
            }),
            SwatchDrag::Saturation => hsv_to_rgb(Hsv {
                s: (self.hsv.s + dx * DRAG_RATE).clamp(0., 1.),
                ..self.hsv
            }),
        }
    }

    /// The same drag applied to another colour (gang edits, A5): the
    /// modifier drags are relative (each colour's own hue, saturation or
    /// value moves by the same amount); a component drag sets the same
    /// absolute value.
    pub fn apply_to(&self, other: [f64; 3], dx: f64) -> [f64; 3] {
        match self.drag {
            SwatchDrag::Component(i) => {
                let mut out = other;
                out[i] = self.at(dx)[i];
                out
            }
            _ => DragStart::new(other, self.drag, self.hdr).at(dx),
        }
    }
}

/// Brightness multiplier for a Stops setting (B4): stops².
pub fn stops_multiplier(stops: f64) -> f64 {
    stops * stops
}

/// Approximate linear RGB of a black body at `kelvin`, normalised so the
/// brightest component is 1 (a curve fit to the CIE black-body locus, good
/// to a few percent over 1000–12000 K).
pub fn kelvin_to_rgb(kelvin: f64) -> [f64; 3] {
    let t = kelvin.clamp(KELVIN_MIN, 40000.) / 100.;
    // sRGB-encoded fit, then decoded to linear.
    let r = if t <= 66. {
        255.
    } else {
        329.698_727_446 * (t - 60.).powf(-0.133_204_759_2)
    };
    let g = if t <= 66. {
        99.470_802_586_1 * t.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (t - 60.).powf(-0.075_514_849_2)
    };
    let b = if t >= 66. {
        255.
    } else if t <= 19. {
        0.
    } else {
        138.517_731_223_1 * (t - 10.).ln() - 305.044_792_730_7
    };
    let lin = [r, g, b].map(|c| srgb_to_linear((c / 255.).clamp(0., 1.)));
    let max = lin.iter().cloned().fold(0., f64::max).max(1e-9);
    lin.map(|c| c / max)
}

/// The Kelvin temperature whose colour is nearest `rgb` in chromaticity
/// (B5's Kelvin field). Greys read about 6500 K.
pub fn rgb_to_kelvin(rgb: [f64; 3]) -> f64 {
    let sum = rgb.iter().map(|c| c.max(0.)).sum::<f64>();
    if sum <= 1e-12 {
        return KELVIN_DEFAULT;
    }
    let chroma = |c: [f64; 3]| {
        let s = c.iter().sum::<f64>().max(1e-12);
        [c[0] / s, c[2] / s]
    };
    let want = chroma(rgb.map(|c| c.max(0.)));
    let dist = |k: f64| {
        let c = chroma(kelvin_to_rgb(k));
        (c[0] - want[0]).powi(2) + (c[1] - want[1]).powi(2)
    };
    // Coarse scan, then golden-section refine.
    let mut best = KELVIN_MIN;
    let mut best_d = f64::INFINITY;
    let mut k = KELVIN_MIN;
    while k <= KELVIN_MAX {
        let d = dist(k);
        if d < best_d {
            best_d = d;
            best = k;
        }
        k += 100.;
    }
    let (mut a, mut b) = ((best - 100.).max(KELVIN_MIN), (best + 100.).min(KELVIN_MAX));
    for _ in 0..40 {
        let m1 = a + (b - a) * 0.382;
        let m2 = a + (b - a) * 0.618;
        if dist(m1) < dist(m2) {
            b = m2;
        } else {
            a = m1;
        }
    }
    ((a + b) / 2.).round()
}

/// Artist (RYB) wheel: the RGB hue at RYB angle `ryb` (degrees). Red stays
/// at 0, yellow sits opposite violet at 120, blue at 240.
pub fn ryb_to_rgb_hue(ryb: f64) -> f64 {
    let a = ryb.rem_euclid(360.);
    if a < 120. {
        a / 2.
    } else if a < 240. {
        60. + (a - 120.) * 1.5
    } else {
        a
    }
}

/// Inverse of [`ryb_to_rgb_hue`].
pub fn rgb_to_ryb_hue(rgb: f64) -> f64 {
    let a = rgb.rem_euclid(360.);
    if a < 60. {
        a * 2.
    } else if a < 240. {
        120. + (a - 60.) / 1.5
    } else {
        a
    }
}

pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}

/// Relative luminance of a linear colour (Rec. 709 weights).
pub fn luminance([r, g, b]: [f64; 3]) -> f64 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Whether the swatch numbers should be drawn light over this fill (A1, OP:
/// light text when the displayed fill's luminance is under 0.5).
pub fn light_text_over(linear: [f64; 3]) -> bool {
    linear_to_srgb(luminance(linear.map(|c| c.clamp(0., 1.)))) < 0.5
}

/// `#rrggbb` for a colour (components clipped to 0–1; display-encoded
/// when `display` is set).
pub fn hex_rgb(rgb: [f64; 3], display: bool) -> String {
    let [r, g, b] = rgb.map(|c| {
        let c = c.clamp(0., 1.);
        let c = if display { linear_to_srgb(c) } else { c };
        (c * 255.).round() as u8
    });
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Parse `#rrggbb`, `rrggbb` or `#rgb` into 0–1 components.
pub fn parse_hex_rgb(text: &str, display: bool) -> Option<[f64; 3]> {
    let t = text.trim().trim_start_matches('#');
    let digits: Vec<u8> = match t.len() {
        6 => (0..3)
            .map(|i| u8::from_str_radix(&t[i * 2..i * 2 + 2], 16).ok())
            .collect::<Option<_>>()?,
        3 => t
            .chars()
            .map(|c| c.to_digit(16).map(|d| (d * 17) as u8))
            .collect::<Option<_>>()?,
        _ => return None,
    };
    let c = |v: u8| {
        let c = f64::from(v) / 255.;
        if display { srgb_to_linear(c) } else { c }
    };
    Some([c(digits[0]), c(digits[1]), c(digits[2])])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn hsv_round_trips_including_hdr() {
        for rgb in [
            [0.8, 0.4, 0.2],
            [0.1, 0.9, 0.3],
            [2.5, 1.0, 0.5],
            [0., 0., 0.],
            [0.5, 0.5, 0.5],
        ] {
            assert!(close(hsv_to_rgb(rgb_to_hsv(rgb)), rgb), "{rgb:?}");
        }
    }

    /// CN-01: a plain drag right 100 px on G raises it by 0.5 alone.
    #[test]
    fn cn01_component_drag() {
        let start = DragStart::new(
            [0.2, 0.3, 0.4],
            SwatchDrag::from_modifiers(1, false, false, false),
            true,
        );
        assert!(close(start.at(100.), [0.2, 0.8, 0.4]));
        // Lower clamp 0; UI colours clamp at 1, scene colours don't.
        assert!(close(start.at(-1000.), [0.2, 0., 0.4]));
        let ui = DragStart::new([0.2, 0.3, 0.4], SwatchDrag::Component(1), false);
        assert!(close(ui.at(1000.), [0.2, 1., 0.4]));
        assert!(close(start.at(1000.), [0.2, 5.3, 0.4]));
    }

    /// CN-02: Shift (brightness) keeps the 4:2:1 ratio and the hue.
    #[test]
    fn cn02_brightness_drag() {
        let start = DragStart::new(
            [0.8, 0.4, 0.2],
            SwatchDrag::from_modifiers(0, true, false, false),
            true,
        );
        let out = start.at(40.);
        assert!((out[0] / out[2] - 4.).abs() < 1e-9);
        assert!((out[1] / out[2] - 2.).abs() < 1e-9);
        assert!((out[0] - 1.0).abs() < 1e-9, "value +0.2: {out:?}");
        assert!((rgb_to_hsv(out).h - rgb_to_hsv([0.8, 0.4, 0.2]).h).abs() < 1e-9);
        // Not clamped at 1 for scene colours.
        assert!(start.at(400.)[0] > 1.);
        // From black it steps to grey.
        let black = DragStart::new([0.; 3], SwatchDrag::Brightness, true);
        assert!(close(black.at(20.), [0.1; 3]));
    }

    /// CN-03: Ctrl (hue) 720 px goes once round and back to red.
    #[test]
    fn cn03_hue_drag_wraps() {
        let start = DragStart::new(
            [1., 0., 0.],
            SwatchDrag::from_modifiers(0, false, true, false),
            true,
        );
        let half = rgb_to_hsv(start.at(360.));
        assert!((half.h - 180.).abs() < 1e-9);
        assert!((half.s - 1.).abs() < 1e-9 && (half.v - 1.).abs() < 1e-9);
        assert!(close(start.at(720.), [1., 0., 0.]));
        // Greys don't change.
        let grey = DragStart::new([0.5; 3], SwatchDrag::Hue, true);
        assert!(close(grey.at(123.), [0.5; 3]));
    }

    /// CN-04: Alt (saturation) left to the end gives grey of the same value.
    #[test]
    fn cn04_saturation_drag() {
        let start = DragStart::new(
            [0.8, 0.4, 0.2],
            SwatchDrag::from_modifiers(0, false, false, true),
            true,
        );
        assert!(close(start.at(-10_000.), [0.8, 0.8, 0.8]));
        // And back right: the remembered hue comes back.
        assert!(close(start.at(0.), [0.8, 0.4, 0.2]));
        let full = rgb_to_hsv(start.at(10_000.));
        assert!((full.s - 1.).abs() < 1e-9);
    }

    /// CN-08's maths: a hue drag applied to other colours turns each by the
    /// same angle; a component drag sets the same value.
    #[test]
    fn gang_edits_are_relative_for_modifier_drags() {
        let start = DragStart::new([1., 0., 0.], SwatchDrag::Hue, true);
        for other in [[0., 1., 0.], [0.2, 0.4, 0.9]] {
            let before = rgb_to_hsv(other);
            let after = rgb_to_hsv(start.apply_to(other, 60.));
            assert!(((after.h - before.h).rem_euclid(360.) - 30.).abs() < 1e-6);
            assert!((after.s - before.s).abs() < 1e-9);
        }
        let comp = DragStart::new([0.1, 0.2, 0.3], SwatchDrag::Component(2), true);
        assert!(close(comp.apply_to([0.9, 0.9, 0.9], 20.), [0.9, 0.9, 0.4]));
    }

    /// CN-07: Stops 2 makes the brightest point of the wheel value 4.
    #[test]
    fn cn07_stops() {
        assert_eq!(stops_multiplier(1.), 1.);
        assert_eq!(stops_multiplier(2.), 4.);
        assert!((stops_multiplier(1.4) - 1.96).abs() < 1e-12);
    }

    #[test]
    fn kelvin_runs_warm_to_cool_and_round_trips() {
        let warm = kelvin_to_rgb(2000.);
        let cool = kelvin_to_rgb(11000.);
        assert!(warm[0] > warm[2] && cool[2] > cool[0]);
        let white = kelvin_to_rgb(6500.);
        assert!(white.iter().all(|&c| c > 0.85), "{white:?}");
        for k in [2000., 3200., 5000., 6500., 9000.] {
            let back = rgb_to_kelvin(kelvin_to_rgb(k));
            assert!((back - k).abs() / k < 0.03, "{k} -> {back}");
        }
    }

    #[test]
    fn ryb_wheel_maps_primaries() {
        assert_eq!(ryb_to_rgb_hue(0.), 0.);
        assert_eq!(ryb_to_rgb_hue(120.), 60.);
        assert_eq!(ryb_to_rgb_hue(240.), 240.);
        for a in [0., 37., 120., 200., 300.] {
            assert!((rgb_to_ryb_hue(ryb_to_rgb_hue(a)) - a).abs() < 1e-9);
        }
    }

    #[test]
    fn hex_and_contrast() {
        assert_eq!(hex_rgb([1., 0.5, 0.], false), "#ff8000");
        let back = parse_hex_rgb("#ff8000", false).unwrap();
        assert!((back[1] - 128. / 255.).abs() < 1e-12);
        assert!(parse_hex_rgb("zz", false).is_none());
        assert!(close(parse_hex_rgb("#fff", false).unwrap(), [1.; 3]));
        assert!(light_text_over([0.02; 3]));
        assert!(!light_text_over([0.9; 3]));
    }
}
