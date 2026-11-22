//! Painting stage 4: the tips beyond Smooth and Hard, the Tablet Nozzle
//! and their values as one brush (PNT-130..150). Painting builds only.
//!
//! - **Tips (PNT-130..134).** Each has Size, Auto Scale and Bias (a shared
//!   `Tip` holds them). Procedural modulates a round profile by a
//!   Shader-Tree procedural pattern (Noise, Cellular, Dots) evaluated at the
//!   texel's UV; Sphere paints a shaded ball; Image ("Texture") and Stamp
//!   lay a picture (Normal, Roller, Stamp); Text lays a string rendered from
//!   Operandi's own fonts.
//! - A tip answers, for a point of the dab (offset across the dab in units
//!   of its radius, turned by the dab's angle), how strongly it paints and,
//!   for tips that carry colour (Sphere, Image), with which colour
//!   ([`TipExtra::eval`]).
//! - **Tablet Nozzle (PNT-140, 141).** Pressure curve, what pressure and
//!   speed drive, dab rotation and the jitters. Random values come from a
//!   per-stroke seed through `pcg3d`, adapted from TextureCraft
//!   `crates/texel/src/noise.rs` at 9c1b9cbc (MIT OR Apache-2.0, Copyright
//!   (c) 2026 ArtCraft Team and the TextureCraft contributors; see
//!   docs/licenses/texturecraft-LICENSE-MIT.txt).
use crate::Result;
use crate::brush_engine::{Tip, TipKind, TipShape};
use crate::procedural::{Procedural, ProceduralKind};
use crate::shader_tree::TwoZone;
use glam::DVec2;
use operandi_paint::{Depth, Format, Layout, PaintImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Which tip is active (PNT-135's row: Smooth, Hard, Procedural / Sphere,
/// Texture, Stamp, Text).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TipChoice {
    #[default]
    Smooth,
    Hard,
    Procedural,
    Sphere,
    /// The Image tip (toolbox "Texture").
    Image,
    Stamp,
    Text,
}

impl TipChoice {
    pub const ALL: [TipChoice; 7] = [
        TipChoice::Smooth,
        TipChoice::Hard,
        TipChoice::Procedural,
        TipChoice::Sphere,
        TipChoice::Image,
        TipChoice::Stamp,
        TipChoice::Text,
    ];
    pub fn label(self) -> &'static str {
        match self {
            TipChoice::Smooth => "Smooth",
            TipChoice::Hard => "Hard",
            TipChoice::Procedural => "Procedural",
            TipChoice::Sphere => "Sphere",
            TipChoice::Image => "Texture",
            TipChoice::Stamp => "Stamp",
            TipChoice::Text => "Text",
        }
    }
    /// The command word (`operandi.paint.tip kind:image`).
    pub fn word(self) -> &'static str {
        match self {
            TipChoice::Smooth => "smooth",
            TipChoice::Hard => "hard",
            TipChoice::Procedural => "procedural",
            TipChoice::Sphere => "sphere",
            TipChoice::Image => "image",
            TipChoice::Stamp => "stamp",
            TipChoice::Text => "text",
        }
    }
    pub fn named(word: &str) -> Option<TipChoice> {
        let w = word.trim();
        TipChoice::ALL
            .into_iter()
            .find(|c| c.word().eq_ignore_ascii_case(w) || c.label().eq_ignore_ascii_case(w))
    }
    /// Press sets the centre, dragging sets size and angle, release lays
    /// it once (PNT-132 Stamp mode, 133, 134).
    pub fn stamps(self, image: &ImageTip) -> bool {
        match self {
            TipChoice::Stamp | TipChoice::Text => true,
            TipChoice::Image => image.mode == ImageMode::Stamp,
            _ => false,
        }
    }
}

/// Procedural tip pattern types (PNT-130).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    #[default]
    Noise,
    Cellular,
    Dots,
}

impl PatternType {
    pub const ALL: [PatternType; 3] =
        [PatternType::Noise, PatternType::Cellular, PatternType::Dots];
    pub fn label(self) -> &'static str {
        match self {
            PatternType::Noise => "Noise",
            PatternType::Cellular => "Cellular",
            PatternType::Dots => "Dots",
        }
    }
    fn kind(self) -> ProceduralKind {
        match self {
            PatternType::Noise => ProceduralKind::Noise,
            PatternType::Cellular => ProceduralKind::Cellular,
            PatternType::Dots => ProceduralKind::Dots,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProceduralTip {
    pub pattern: PatternType,
    pub invert: bool,
    /// Scale U, V, W (1 = 100 %): the pattern's size along U, V and the
    /// normal of the UV plane.
    pub scale: [f64; 3],
    pub amplitude: f64,
    pub gain: f64,
}

impl Default for ProceduralTip {
    fn default() -> Self {
        ProceduralTip {
            pattern: PatternType::Noise,
            invert: false,
            scale: [1.; 3],
            amplitude: 1.,
            gain: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SphereTip {
    pub height_shading: bool,
    /// 0 fully round, 1 flat like Hard.
    pub flatness: f64,
    /// The fake highlight's strength.
    pub specular: f64,
}

impl Default for SphereTip {
    fn default() -> Self {
        SphereTip {
            height_shading: true,
            flatness: 0.,
            specular: 0.5,
        }
    }
}

/// Image tip Mode (PNT-132).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageMode {
    #[default]
    Normal,
    Roller,
    Stamp,
}

impl ImageMode {
    pub const ALL: [ImageMode; 3] = [ImageMode::Normal, ImageMode::Roller, ImageMode::Stamp];
    pub fn label(self) -> &'static str {
        match self {
            ImageMode::Normal => "Normal",
            ImageMode::Roller => "Roller",
            ImageMode::Stamp => "Stamp",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageTip {
    /// A scene image's id (any loaded image); none to start.
    pub image: Option<u64>,
    pub mode: ImageMode,
    /// Remaps the image's zero point (0.5: mid-grey is "nothing").
    pub offset: f64,
    /// Radians.
    pub rotation: f64,
    /// Fade from this far inside the rim (0..1).
    pub soft_border: f64,
    /// Dark areas become transparent.
    pub use_as_mask: bool,
    /// A non-square image keeps its proportions.
    pub keep_aspect: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextTip {
    pub text: String,
    /// A built-in font's name (`FONTS`).
    pub font: String,
    pub bold: bool,
    pub italics: bool,
    /// Radians.
    pub rotation: f64,
}

impl Default for TextTip {
    fn default() -> Self {
        TextTip {
            text: "TEXT".into(),
            font: FONTS[0].0.into(),
            bold: false,
            italics: false,
            rotation: 0.,
        }
    }
}

/// The fonts the Text tip offers: Operandi's own UI font first (PNT-134;
/// installed system fonts are PNT-Q36).
pub const FONTS: &[(&str, &[u8])] = &[
    ("Ubuntu Light", epaint_default_fonts::UBUNTU_LIGHT),
    ("Hack", epaint_default_fonts::HACK_REGULAR),
];

/// Nozzle Pressure Sensitivity (PNT-141).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sensitivity {
    Low,
    #[default]
    Medium,
    High,
}

/// What pressure and speed drive (PNT-141).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Drive {
    #[default]
    None,
    Pressure,
    Speed,
    Both,
}

impl Drive {
    pub const ALL: [Drive; 4] = [Drive::None, Drive::Pressure, Drive::Speed, Drive::Both];
    pub fn label(self) -> &'static str {
        match self {
            Drive::None => "None",
            Drive::Pressure => "Pressure",
            Drive::Speed => "Speed",
            Drive::Both => "Both",
        }
    }
    fn factor(self, pressure: f64, speed: f64) -> f64 {
        // Faster is lighter (PNT-Q30): half at 1000 points a second.
        let speed = 1. / (1. + speed.max(0.) / 1000.);
        match self {
            Drive::None => 1.,
            Drive::Pressure => pressure,
            Drive::Speed => speed,
            Drive::Both => pressure * speed,
        }
    }
}

/// The Tablet Nozzle (PNT-140, 141).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Nozzle {
    pub sensitivity: Sensitivity,
    pub strength: Drive,
    pub size: Drive,
    /// Align: each dab turns with the stroke; else upright on screen.
    pub align: bool,
    pub jitter: bool,
    pub jitter_spacing: f64,
    pub jitter_position: f64,
    pub jitter_size: f64,
    pub jitter_rotation: f64,
    pub jitter_strength: f64,
}

impl Default for Nozzle {
    fn default() -> Self {
        Nozzle {
            sensitivity: Sensitivity::Medium,
            strength: Drive::Pressure,
            size: Drive::None,
            align: true,
            jitter: false,
            jitter_spacing: 0.,
            jitter_position: 0.1,
            jitter_size: 0.,
            jitter_rotation: 0.,
            jitter_strength: 0.,
        }
    }
}

/// What the nozzle does to one dab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DabMods {
    pub strength: f64,
    pub size: f64,
    /// Radians, screen space (y down).
    pub angle: f64,
    /// Scatter, in units of the tip radius.
    pub offset: DVec2,
    /// The gap to the next dab, as a share of the usual one.
    pub gap: f64,
}

impl Nozzle {
    /// The pen pressure after Pressure Sensitivity's curve.
    pub fn curve(&self, pressure: f64) -> f64 {
        let p = pressure.clamp(0., 1.);
        match self.sensitivity {
            Sensitivity::Low => p * p,
            Sensitivity::Medium => p,
            Sensitivity::High => p.sqrt(),
        }
    }

    /// Dab number `n` of a stroke seeded `seed`, at pen `pressure`, pointer
    /// `speed` (screen points a second) moving along `direction` (radians).
    pub fn dab(&self, seed: u32, n: u32, pressure: f64, speed: f64, direction: f64) -> DabMods {
        let p = self.curve(pressure);
        let mut m = DabMods {
            strength: self.strength.factor(p, speed),
            size: self.size.factor(p, speed).max(0.05),
            angle: if self.align { direction } else { 0. },
            offset: DVec2::ZERO,
            gap: 1.,
        };
        if self.jitter {
            let r = pcg3d(seed, n, 0x5EED);
            let s = pcg3d(seed, n, 0xD1CE);
            let unit = |v: u32| f64::from(v) / f64::from(u32::MAX);
            let signed = |v: u32| unit(v) * 2. - 1.;
            m.gap = (1. + signed(r[0]) * self.jitter_spacing).max(0.05);
            let a = unit(r[1]) * std::f64::consts::TAU;
            let d = unit(r[2]).sqrt() * self.jitter_position;
            m.offset = DVec2::new(a.cos(), a.sin()) * d;
            m.size = (m.size * (1. + signed(s[0]) * self.jitter_size)).max(0.05);
            m.angle += signed(s[1]) * self.jitter_rotation * std::f64::consts::PI;
            m.strength = (m.strength * (1. - unit(s[2]) * self.jitter_strength)).max(0.);
        }
        m
    }
}

/// A 3D hash of three integers (TextureCraft `noise::pcg3d`).
pub fn pcg3d(x: u32, y: u32, z: u32) -> [u32; 3] {
    let mut v = [
        x.wrapping_mul(1664525).wrapping_add(1013904223),
        y.wrapping_mul(1664525).wrapping_add(1013904223),
        z.wrapping_mul(1664525).wrapping_add(1013904223),
    ];
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    v = v.map(|a| a ^ (a >> 16));
    v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
    v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
    v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
    v
}

/// Every tip's values and the nozzle: what a brush preset stores
/// (PNT-150; not the tool).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Brush {
    pub choice: TipChoice,
    /// Smooth and Hard (their kind follows `choice`).
    pub basic: Tip,
    /// Each other tip's Size, Auto Scale and Bias.
    pub procedural_tip: Tip,
    pub procedural: ProceduralTip,
    pub sphere_tip: Tip,
    pub sphere: SphereTip,
    pub image_tip: Tip,
    pub image: ImageTip,
    pub stamp_tip: Tip,
    pub stamp: ImageTip,
    pub text_tip: Tip,
    pub text: TextTip,
    /// The Nozzle button added the Tablet Nozzle (PNT-140).
    pub nozzle_on: bool,
    pub nozzle: Nozzle,
}

fn sized(size: f64) -> Tip {
    Tip {
        kind: TipKind::Smooth,
        shape: TipShape::Smooth,
        size,
        ..Tip::default()
    }
}

impl Default for Brush {
    fn default() -> Self {
        Brush {
            choice: TipChoice::Smooth,
            basic: crate::paint_stroke::default_tip(),
            procedural_tip: sized(10.),
            procedural: ProceduralTip::default(),
            sphere_tip: sized(10.),
            sphere: SphereTip::default(),
            image_tip: sized(20.),
            image: ImageTip::default(),
            stamp_tip: sized(20.),
            stamp: ImageTip {
                mode: ImageMode::Stamp,
                ..ImageTip::default()
            },
            text_tip: sized(30.),
            text: TextTip::default(),
            nozzle_on: false,
            nozzle: Nozzle::default(),
        }
    }
}

impl Brush {
    /// The active tip's Size, Auto Scale and Bias (and, for Smooth and
    /// Hard, their own rows).
    pub fn tip(&self) -> Tip {
        match self.choice {
            TipChoice::Smooth => Tip {
                kind: TipKind::Smooth,
                ..self.basic
            },
            TipChoice::Hard => Tip {
                kind: TipKind::Hard,
                ..self.basic
            },
            TipChoice::Procedural => self.procedural_tip,
            TipChoice::Sphere => self.sphere_tip,
            TipChoice::Image => self.image_tip,
            TipChoice::Stamp => self.stamp_tip,
            TipChoice::Text => self.text_tip,
        }
    }
    pub fn tip_mut(&mut self) -> &mut Tip {
        match self.choice {
            TipChoice::Smooth | TipChoice::Hard => &mut self.basic,
            TipChoice::Procedural => &mut self.procedural_tip,
            TipChoice::Sphere => &mut self.sphere_tip,
            TipChoice::Image => &mut self.image_tip,
            TipChoice::Stamp => &mut self.stamp_tip,
            TipChoice::Text => &mut self.text_tip,
        }
    }
    /// The active image settings (Image or Stamp tip).
    pub fn image_settings(&self) -> &ImageTip {
        if self.choice == TipChoice::Stamp {
            &self.stamp
        } else {
            &self.image
        }
    }
    /// Press-drag-release stamping (PNT-132..134).
    pub fn stamps(&self) -> bool {
        self.choice.stamps(&self.image)
    }

    /// What the stroke needs at dab time. `image` gives a scene image's
    /// pixels; `fg` is the paint colour (Sphere's centre).
    pub fn extra(
        &self,
        image: impl Fn(u64) -> Option<Arc<PaintImage>>,
        fg: [f32; 3],
    ) -> Result<TipExtra> {
        Ok(match self.choice {
            TipChoice::Smooth | TipChoice::Hard => TipExtra::Basic,
            TipChoice::Procedural => {
                let p = self.procedural;
                TipExtra::Procedural {
                    pattern: Procedural::new(p.pattern.kind()),
                    tip: p,
                    bias: self.procedural_tip.bias,
                }
            }
            TipChoice::Sphere => TipExtra::Sphere {
                tip: self.sphere,
                fg,
            },
            TipChoice::Image | TipChoice::Stamp => {
                let s = *self.image_settings();
                let id = s.image.ok_or("Pick an image for the tip first")?;
                let px = image(id).ok_or("The tip's image can't be read")?;
                TipExtra::Image { px, tip: s }
            }
            TipChoice::Text => {
                let t = &self.text;
                if t.text.trim().is_empty() {
                    return Err("Type the Text tip's text first".into());
                }
                let mask = render_text(&t.text, &t.font, t.bold, t.italics)?;
                TipExtra::Image {
                    px: Arc::new(mask),
                    tip: ImageTip {
                        image: None,
                        mode: ImageMode::Stamp,
                        offset: 0.,
                        rotation: t.rotation,
                        soft_border: 0.,
                        use_as_mask: true,
                        keep_aspect: true,
                    },
                }
            }
        })
    }
}

/// What a dab needs beyond the basic tip, ready to evaluate.
#[derive(Clone, Debug)]
pub enum TipExtra {
    Basic,
    Procedural {
        pattern: Procedural,
        tip: ProceduralTip,
        bias: f64,
    },
    Sphere {
        tip: SphereTip,
        fg: [f32; 3],
    },
    /// Image, Stamp and Text tips.
    Image {
        px: Arc<PaintImage>,
        tip: ImageTip,
    },
}

impl TipExtra {
    /// How far texels can be from the dab centre, in radii (square
    /// pictures reach into the corners).
    pub fn reach(&self) -> f64 {
        match self {
            TipExtra::Image { .. } => std::f64::consts::SQRT_2,
            _ => 1.,
        }
    }

    pub fn carries_colour(&self) -> bool {
        match self {
            TipExtra::Sphere { .. } => true,
            TipExtra::Image { tip, .. } => !tip.use_as_mask,
            _ => false,
        }
    }

    /// The dab at `d` (offset from the centre in radii, screen axes, y
    /// down), turned by `angle`: its weight and, for tips with colour, the
    /// colour. `uv` is the texel's UV (Procedural), `travel` how far along
    /// the stroke this dab is in radii (Roller). `None`: outside.
    pub fn eval(
        &self,
        base: &Tip,
        d: DVec2,
        angle: f64,
        uv: DVec2,
        travel: f64,
    ) -> Option<(f32, Option<[f32; 3]>)> {
        let t = d.length();
        match self {
            TipExtra::Basic => {
                let w = base.weight(t);
                (w > 0.).then_some((w as f32, None))
            }
            TipExtra::Procedural { pattern, tip, bias } => {
                // The round Smooth profile times the pattern (PNT-Q28).
                let round = Tip {
                    kind: TipKind::Smooth,
                    shape: TipShape::Smooth,
                    bias: 0.5,
                    ..*base
                }
                .weight(t);
                if round <= 0. {
                    return None;
                }
                let s = tip.scale.map(|v| v.max(1e-6));
                let raw = pattern
                    .pattern
                    .value(2, [uv.x * 8. / s[0], uv.y * 8. / s[1], 0.5 / s[2]]);
                let v = TwoZone::gain(tip.gain, TwoZone::bias(*bias, raw.clamp(0., 1.)));
                let v = if tip.invert { 1. - v } else { v };
                let w = round * (1. - tip.amplitude.clamp(0., 1.) * (1. - v));
                (w > 0.).then_some((w as f32, None))
            }
            TipExtra::Sphere { tip, fg } => {
                if t > 1. {
                    return None;
                }
                let round = (1. - t * t).max(0.).sqrt();
                let h = round + (1. - round) * tip.flatness.clamp(0., 1.);
                let shade = if tip.height_shading { h } else { 1. };
                // A highlight up and to the left.
                let spot = (d - DVec2::new(-0.35, -0.35)).length();
                let spec = (tip.specular * (1. - spot / 0.5).max(0.).powi(2)) as f32;
                let c = fg.map(|v| (v * shade as f32 + spec).min(v.max(1.)));
                // Edge softness: the rim fades as the sphere meets the
                // surface.
                let w = (h * 4.).min(1.);
                Some((w as f32, Some(c)))
            }
            TipExtra::Image { px, tip } => {
                // Turn into the picture's frame.
                let a = -(angle + tip.rotation);
                let (sin, cos) = a.sin_cos();
                let mut q = DVec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
                let (w, h) = (f64::from(px.width), f64::from(px.height));
                if tip.keep_aspect && w > 0. && h > 0. {
                    // The longer side spans the dab.
                    let k = w.max(h);
                    q = DVec2::new(q.x * k / w, q.y * k / h);
                }
                let mut u = (q.x + 1.) / 2.;
                let v = (q.y + 1.) / 2.;
                if tip.mode == ImageMode::Roller {
                    // Unrolled along the stroke, tiling in its direction.
                    u = (u + travel / 2.).rem_euclid(1.);
                }
                if !(0. ..=1.).contains(&u) || !(0. ..=1.).contains(&v) {
                    return None;
                }
                let p = px.sample(u as f32 * px.width as f32, v as f32 * px.height as f32);
                let luma = operandi_paint::image::luma([p[0], p[1], p[2]]);
                let off = tip.offset as f32;
                let remap = |x: f32| ((x - off) / (1. - off).max(1e-6)).clamp(0., 1.);
                let mut weight = if tip.use_as_mask {
                    remap(luma) * p[3]
                } else {
                    p[3]
                };
                if tip.soft_border > 0. {
                    let edge = q.x.abs().max(q.y.abs());
                    let inner = 1. - tip.soft_border.clamp(0., 1.);
                    if edge > inner {
                        weight *=
                            (1. - (edge - inner) / (1. - inner).max(1e-6)).clamp(0., 1.) as f32;
                    }
                }
                let colour = (!tip.use_as_mask).then(|| [p[0], p[1], p[2]].map(remap));
                (weight > 0.).then_some((weight, colour))
            }
        }
    }
}

/// Renders `text` in built-in font `font` as a grey mask (white text on
/// black, 1 = ink), about 128 px tall. Bold thickens the ink, Italics
/// slants it (both faked, PNT-134).
pub fn render_text(text: &str, font: &str, bold: bool, italics: bool) -> Result<PaintImage> {
    let bytes = FONTS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(font))
        .map_or(FONTS[0].1, |(_, b)| *b);
    let face = ttf_parser::Face::parse(bytes, 0).map_err(|e| format!("Font: {e}"))?;
    const HEIGHT: f64 = 128.;
    let units = f64::from(face.units_per_em());
    let scale = HEIGHT * 0.8 / units;
    let ascender = f64::from(face.ascender()) * scale;
    // Glyph outlines as polygons, laid out left to right.
    let mut polys: Vec<Vec<DVec2>> = Vec::new();
    let mut pen = 4.;
    let slant = if italics { 0.2 } else { 0. };
    for ch in text.chars() {
        let Some(g) = face.glyph_index(ch) else {
            pen += HEIGHT * 0.3;
            continue;
        };
        let mut b = Outline {
            polys: Vec::new(),
            cur: Vec::new(),
            scale,
            dx: pen,
            base: ascender + 8.,
            slant,
            last: DVec2::ZERO,
        };
        face.outline_glyph(g, &mut b);
        b.close();
        polys.extend(b.polys);
        pen += f64::from(face.glyph_hor_advance(g).unwrap_or(0)) * scale;
    }
    let width = ((pen + 8. + HEIGHT * slant) as u32).clamp(8, 8192);
    let height = (HEIGHT + 16.) as u32;
    let mut img = PaintImage::new(Layout {
        width,
        height,
        format: Format::Gray,
        depth: Depth::F32,
    })?;
    // Non-zero winding at 4 × 4 samples a pixel.
    const SS: u32 = 4;
    for y in 0..height {
        for x in 0..width {
            let mut hits = 0;
            for sy in 0..SS {
                for sx in 0..SS {
                    let p = DVec2::new(
                        f64::from(x) + (f64::from(sx) + 0.5) / f64::from(SS),
                        f64::from(y) + (f64::from(sy) + 0.5) / f64::from(SS),
                    );
                    if winding(&polys, p) != 0 {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                let v = hits as f32 / (SS * SS) as f32;
                img.set((y * width + x) as usize, [v; 4]);
            }
        }
    }
    if bold {
        // One pixel thicker.
        let src = img.clone();
        for y in 0..height as i64 {
            for x in 0..width as i64 {
                let mut m = 0f32;
                for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < width as i64 && ny < height as i64 {
                        m = m.max(src.get((ny * width as i64 + nx) as usize)[0]);
                    }
                }
                img.set((y * width as i64 + x) as usize, [m; 4]);
            }
        }
    }
    Ok(img)
}

/// Glyph outlines flattened into polygons, in image pixels (y down).
struct Outline {
    polys: Vec<Vec<DVec2>>,
    cur: Vec<DVec2>,
    scale: f64,
    dx: f64,
    base: f64,
    slant: f64,
    last: DVec2,
}

impl Outline {
    fn at(&self, x: f32, y: f32) -> DVec2 {
        let (x, y) = (f64::from(x) * self.scale, f64::from(y) * self.scale);
        DVec2::new(self.dx + x + y * self.slant, self.base - y)
    }
    fn close(&mut self) {
        if self.cur.len() > 2 {
            self.polys.push(std::mem::take(&mut self.cur));
        } else {
            self.cur.clear();
        }
    }
}

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.last = self.at(x, y);
        self.cur.push(self.last);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.last = self.at(x, y);
        self.cur.push(self.last);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (p0, p1, p2) = (self.last, self.at(x1, y1), self.at(x, y));
        for k in 1..=8 {
            let t = f64::from(k) / 8.;
            let u = 1. - t;
            self.cur.push(p0 * u * u + p1 * 2. * u * t + p2 * t * t);
        }
        self.last = p2;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (p0, p1, p2, p3) = (self.last, self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        for k in 1..=12 {
            let t = f64::from(k) / 12.;
            let u = 1. - t;
            self.cur
                .push(p0 * u * u * u + p1 * 3. * u * u * t + p2 * 3. * u * t * t + p3 * t * t * t);
        }
        self.last = p3;
    }
    fn close(&mut self) {
        Outline::close(self);
    }
}

fn winding(polys: &[Vec<DVec2>], p: DVec2) -> i32 {
    let mut w = 0;
    for poly in polys {
        let n = poly.len();
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if a.y <= p.y {
                if b.y > p.y && (b - a).perp_dot(p - a) > 0. {
                    w += 1;
                }
            } else if b.y <= p.y && (b - a).perp_dot(p - a) < 0. {
                w -= 1;
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_renders_ink() {
        let img = render_text("TEXT", "Ubuntu Light", false, false).unwrap();
        let ink = (0..img.pixel_count())
            .filter(|&i| img.get(i)[0] > 0.5)
            .count();
        assert!(ink > 200, "{ink}");
        let bold = render_text("TEXT", "Ubuntu Light", true, false).unwrap();
        let bold_ink = (0..bold.pixel_count())
            .filter(|&i| bold.get(i)[0] > 0.5)
            .count();
        assert!(bold_ink > ink);
        assert!(
            render_text("TEXT", "Ubuntu Light", false, true)
                .unwrap()
                .width
                > img.width
        );
    }

    #[test]
    fn nozzle_defaults_are_stage_twos_pressure_to_strength() {
        let n = Nozzle::default();
        let m = n.dab(1, 0, 0.5, 0., 0.3);
        assert_eq!(
            (m.strength, m.size, m.offset, m.gap),
            (0.5, 1., DVec2::ZERO, 1.)
        );
        assert_eq!(m.angle, 0.3, "Align turns with the stroke");
        let jitter = Nozzle {
            jitter: true,
            jitter_position: 0.5,
            ..n
        };
        let offsets: Vec<f64> = (0..50)
            .map(|k| jitter.dab(7, k, 1., 0., 0.).offset.length())
            .collect();
        assert!(offsets.iter().all(|&d| d <= 0.5 + 1e-9));
        assert!(offsets.iter().any(|&d| d > 0.2));
        // The same seed, the same jitter.
        assert_eq!(jitter.dab(7, 3, 1., 0., 0.), jitter.dab(7, 3, 1., 0., 0.));
        assert!(Drive::Speed.factor(1., 2000.) < Drive::Speed.factor(1., 100.));
    }
}

/// A saved brush (PNT-150): the tip, every tip's values and the nozzle,
/// never the tool, so one preset serves any paint tool (and sculpt brushes
/// once they take these tips). JSON with a kind and version, in its own
/// `.opbrush` file (provisional, PNT-Q37).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrushPreset {
    pub kind: String,
    pub version: u32,
    pub name: String,
    pub brush: Brush,
}

pub const BRUSH_PRESET_KIND: &str = "operandi.brush";
pub const BRUSH_PRESET_EXTENSION: &str = "opbrush";

impl BrushPreset {
    pub fn new(name: &str, brush: &Brush) -> BrushPreset {
        BrushPreset {
            kind: BRUSH_PRESET_KIND.into(),
            version: 1,
            name: name.into(),
            brush: brush.clone(),
        }
    }
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Cannot make {}: {e}", dir.display()))?;
        }
        std::fs::write(path, text).map_err(|e| format!("Cannot write {}: {e}", path.display()))
    }
    pub fn load(path: &std::path::Path) -> Result<BrushPreset> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        let p: BrushPreset = serde_json::from_str(&text)
            .map_err(|e| format!("{}: not a brush preset ({e})", path.display()))?;
        if p.kind != BRUSH_PRESET_KIND {
            return Err(format!("{} isn't a brush preset", path.display()));
        }
        Ok(p)
    }
}
