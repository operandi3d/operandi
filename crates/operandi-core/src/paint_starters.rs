//! Painting stage 7, Part B (PNT-245): Operandi's own starter brushes.
//! Seven brush presets built from the shared parts (tips, Tablet Nozzle),
//! for paint tools and sculpt brushes alike. The two "crumb" pictures are
//! generated here, never taken from anywhere. Painting builds only.
use crate::paint_tips::{Brush, Drive, ImageTip, PatternType, Sensitivity, TipChoice, pcg3d};
use operandi_paint::{Depth, Format, Layout, PaintImage};

/// One starter brush: its name, its tips and nozzle, and the picture its
/// Texture tip uses (made into a scene image when applied).
pub struct StarterBrush {
    pub name: &'static str,
    pub brush: Brush,
    pub picture: Option<fn() -> PaintImage>,
}

/// The starter set, in the strip's order.
pub fn starter_brushes() -> Vec<StarterBrush> {
    let base = Brush::default;
    let mut out = Vec::new();

    // A dynamic-size smooth brush: size and strength follow the pen.
    let mut b = base();
    b.choice = TipChoice::Smooth;
    b.basic.size = 30.;
    b.nozzle_on = true;
    b.nozzle.sensitivity = Sensitivity::High;
    b.nozzle.strength = Drive::Pressure;
    b.nozzle.size = Drive::Pressure;
    out.push(StarterBrush {
        name: "Dynamic Smooth",
        brush: b,
        picture: None,
    });

    // A jitter brush: position, size and strength scattered.
    let mut b = base();
    b.choice = TipChoice::Smooth;
    b.basic.size = 24.;
    b.nozzle_on = true;
    b.nozzle.jitter = true;
    b.nozzle.jitter_position = 0.6;
    b.nozzle.jitter_size = 0.5;
    b.nozzle.jitter_strength = 0.5;
    out.push(StarterBrush {
        name: "Jitter",
        brush: b,
        picture: None,
    });

    // Scattered dots: a hard tip, heavy spacing and position jitter,
    // strength jitter, no pressure.
    let mut b = base();
    b.choice = TipChoice::Hard;
    b.basic.size = 8.;
    b.nozzle_on = true;
    b.nozzle.strength = Drive::None;
    b.nozzle.size = Drive::None;
    b.nozzle.jitter = true;
    b.nozzle.jitter_spacing = 2.5;
    b.nozzle.jitter_position = 5.;
    b.nozzle.jitter_strength = 0.6;
    out.push(StarterBrush {
        name: "Scattered Dots",
        brush: b,
        picture: None,
    });

    // Two crumb brushes: a picture tip used as a mask, mid-grey as the
    // zero point, turned at random.
    for (name, picture) in [
        ("Crumb Coarse", crumb_coarse as fn() -> PaintImage),
        ("Crumb Fine", crumb_fine as fn() -> PaintImage),
    ] {
        let mut b = base();
        b.choice = TipChoice::Image;
        b.image_tip.size = 40.;
        b.image = ImageTip {
            use_as_mask: true,
            offset: 0.5,
            ..ImageTip::default()
        };
        b.nozzle_on = true;
        b.nozzle.jitter = true;
        b.nozzle.jitter_rotation = 1.;
        out.push(StarterBrush {
            name,
            brush: b,
            picture: Some(picture),
        });
    }

    // Two procedural brushes.
    let mut b = base();
    b.choice = TipChoice::Procedural;
    b.procedural.pattern = PatternType::Cellular;
    b.procedural.invert = true;
    out.push(StarterBrush {
        name: "Cellular",
        brush: b,
        picture: None,
    });
    let mut b = base();
    b.choice = TipChoice::Procedural;
    b.procedural.pattern = PatternType::Dots;
    out.push(StarterBrush {
        name: "Dots",
        brush: b,
        picture: None,
    });
    out
}

/// A crumb picture: soft lumps scattered on mid-grey (the zero point),
/// fading out toward the edge so its border leaves no step.
fn crumb(seed: u32, lumps: u32, size: f64) -> PaintImage {
    const SIDE: u32 = 128;
    let mut p = PaintImage::new(Layout {
        width: SIDE,
        height: SIDE,
        format: Format::Gray,
        depth: Depth::F32,
    })
    .expect("a small picture");
    let unit = |v: u32| f64::from(v) / f64::from(u32::MAX);
    let centres: Vec<(f64, f64, f64, f64)> = (0..lumps)
        .map(|k| {
            let r = pcg3d(seed, k, 0xC0B);
            let s = pcg3d(seed, k, 0xB0C);
            let a = unit(r[0]) * std::f64::consts::TAU;
            let d = unit(r[1]).sqrt() * 0.62;
            let height = if unit(s[0]) < 0.3 { -0.5 } else { 1. };
            (
                0.5 + d * a.cos() * 0.8,
                0.5 + d * a.sin() * 0.8,
                size * (0.5 + unit(r[2])),
                height,
            )
        })
        .collect();
    for y in 0..SIDE {
        for x in 0..SIDE {
            let (u, v) = (
                (f64::from(x) + 0.5) / f64::from(SIDE),
                (f64::from(y) + 0.5) / f64::from(SIDE),
            );
            let mut h = 0.;
            for &(cx, cy, r, k) in &centres {
                let d2 = ((u - cx).powi(2) + (v - cy).powi(2)) / (r * r);
                h += k * (-d2 * 2.).exp();
            }
            // Fade to the zero point at the rim.
            let rim = ((u - 0.5).abs().max((v - 0.5).abs()) * 2.).clamp(0., 1.);
            let fade = (1. - rim.powi(4)).clamp(0., 1.);
            let value = (0.5 + 0.5 * h.tanh() * fade) as f32;
            p.set((y * SIDE + x) as usize, [value, value, value, 1.]);
        }
    }
    p
}

pub fn crumb_coarse() -> PaintImage {
    crumb(11, 9, 0.12)
}

pub fn crumb_fine() -> PaintImage {
    crumb(29, 26, 0.06)
}
