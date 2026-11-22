//! Flatten Layers' compositing (PNT-27): image layers stacked bottom to
//! top with their blend mode, opacity and invert into one image.
//!
//! The blend formulas are the shader tree's (operandi-core
//! `shader_eval::blend`, Tier 1 G-06): every mode is
//! `lerp(below, f(below, layer), coverage)`, with coverage the layer's
//! alpha times its opacity. They are written out here per pixel because
//! that version builds render expressions, not numbers.
use crate::image::{Layout, PaintImage, Result};

/// The shader tree's blend modes, in its order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Blend {
    #[default]
    Normal,
    Add,
    Subtract,
    Difference,
    NormalMultiply,
    Divide,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    Lighten,
    Darken,
    ColorDodge,
    ColorBurn,
    NormalMapBlend,
}

/// `f(below, layer)` for one channel, at full strength.
pub fn full(mode: Blend, b: f32, l: f32) -> f32 {
    use Blend::*;
    match mode {
        // Normal-map blends are Normal for now, as in the renderer.
        Normal | NormalMultiply | NormalMapBlend => l,
        Add => b + l,
        Subtract => (b - l).max(0.),
        Difference => (b - l).abs(),
        Multiply => b * l,
        // (1 - B + 2L) / (3 (1 + B)), as in the renderer.
        Divide => {
            let bottom = 3. * (1. + b);
            if bottom == 0. {
                0.
            } else {
                (1. - b + 2. * l) / bottom
            }
        }
        Screen => 1. - (1. - b) * (1. - l),
        Overlay => overlay(b, l),
        // Overlay keyed on the layer instead of the layer below.
        HardLight => overlay(l, b),
        SoftLight => {
            let t = 2. * l - 1.;
            let d = if t <= 0. {
                b * (1. - b)
            } else {
                b.max(0.).sqrt() - b
            };
            b + t * d
        }
        Lighten => b.max(l),
        Darken => b.min(l),
        ColorDodge => {
            if l >= 1. {
                if b > 0. { 1. } else { 0. }
            } else {
                (b / (1. - l)).min(1.)
            }
        }
        ColorBurn => {
            if l <= 0. {
                0.
            } else {
                (1. - (1. - b) / l).max(0.)
            }
        }
    }
}

/// Cycles' Overlay at full strength, keyed on `key`.
fn overlay(key: f32, other: f32) -> f32 {
    if key < 0.5 {
        2. * key * other
    } else {
        1. - 2. * (1. - key) * (1. - other)
    }
}

/// One layer for [`composite`].
pub struct Source<'a> {
    pub image: &'a PaintImage,
    pub blend: Blend,
    /// 0..1.
    pub opacity: f32,
    /// The layer's Invert: colour is 1 − colour before blending.
    pub invert: bool,
}

/// Stacks `layers` (bottom first) into a new image with `layout`. All
/// must have its width and height. The result starts transparent; each
/// layer covers by its alpha × opacity, and colour blends as the
/// renderer's layers do where the result below is opaque.
pub fn composite(layers: &[Source<'_>], layout: Layout) -> Result<PaintImage> {
    if layers.is_empty() {
        return Err("Nothing to flatten".into());
    }
    if layers
        .iter()
        .any(|s| s.image.width != layout.width || s.image.height != layout.height)
    {
        return Err("Flatten Layers needs images of one size".into());
    }
    let mut out = PaintImage::new(layout)?;
    for i in 0..out.pixel_count() {
        let mut acc = [0f32; 4];
        for s in layers {
            let mut px = s.image.get(i);
            if s.invert {
                for v in &mut px[..3] {
                    *v = 1. - v.clamp(0., 1.);
                }
            }
            let cover = (px[3] * s.opacity).clamp(0., 1.);
            if cover == 0. {
                continue;
            }
            let below_a = acc[3];
            let a = cover + below_a * (1. - cover);
            for k in 0..3 {
                let b = acc[k];
                // Over an opaque result this is lerp(b, f(b, l), cover);
                // where the result is still transparent the layer shows
                // as itself.
                let f = full(s.blend, b, px[k]);
                let blended = f * below_a + px[k] * (1. - below_a);
                acc[k] = if a > 0. {
                    (blended * cover + b * below_a * (1. - cover)) / a
                } else {
                    0.
                };
            }
            acc[3] = a;
        }
        out.set(i, acc);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{Depth, Format};

    fn solid(rgba: [f32; 4]) -> PaintImage {
        let mut i = PaintImage::new(Layout {
            width: 64,
            height: 64,
            format: Format::Rgba,
            depth: Depth::F32,
        })
        .unwrap();
        i.fill(rgba);
        i
    }

    fn layout() -> Layout {
        Layout {
            width: 64,
            height: 64,
            format: Format::Rgba,
            depth: Depth::F32,
        }
    }

    #[test]
    fn blend_formulas_at_known_points() {
        assert_eq!(full(Blend::Multiply, 0.5, 0.5), 0.25);
        assert_eq!(full(Blend::Screen, 0.5, 0.5), 0.75);
        assert_eq!(full(Blend::Subtract, 0.2, 0.5), 0.);
        assert_eq!(full(Blend::Overlay, 0.25, 0.5), 0.25);
        assert_eq!(full(Blend::HardLight, 0.5, 0.25), 0.25);
        assert_eq!(full(Blend::ColorDodge, 0.5, 0.5), 1.);
        assert_eq!(full(Blend::ColorBurn, 0.5, 0.5), 0.);
        assert_eq!(full(Blend::Lighten, 0.2, 0.7), 0.7);
    }

    #[test]
    fn opaque_stack_blends_like_layers() {
        let base = solid([0.5, 0.5, 0.5, 1.]);
        let top = solid([0.5, 1., 0., 1.]);
        let out = composite(
            &[
                Source {
                    image: &base,
                    blend: Blend::Normal,
                    opacity: 1.,
                    invert: false,
                },
                Source {
                    image: &top,
                    blend: Blend::Multiply,
                    opacity: 0.5,
                    invert: false,
                },
            ],
            layout(),
        )
        .unwrap();
        // lerp(0.5, 0.5·L, 0.5)
        let p = out.get(0);
        assert!((p[0] - 0.375).abs() < 1e-6);
        assert!((p[1] - 0.5).abs() < 1e-6);
        assert!((p[2] - 0.25).abs() < 1e-6);
        assert_eq!(p[3], 1.);
    }

    #[test]
    fn transparent_layers_pass_through_and_sizes_must_match() {
        let base = solid([1., 0., 0., 1.]);
        let clear = solid([0., 1., 0., 0.]);
        let out = composite(
            &[
                Source {
                    image: &base,
                    blend: Blend::Normal,
                    opacity: 1.,
                    invert: false,
                },
                Source {
                    image: &clear,
                    blend: Blend::Normal,
                    opacity: 1.,
                    invert: false,
                },
            ],
            layout(),
        )
        .unwrap();
        assert_eq!(out.get(5), [1., 0., 0., 1.]);
        let small = PaintImage::new(Layout {
            width: 32,
            ..layout()
        })
        .unwrap();
        assert!(
            composite(
                &[Source {
                    image: &small,
                    blend: Blend::Normal,
                    opacity: 1.,
                    invert: false
                }],
                layout()
            )
            .is_err()
        );
    }
}
