//! Bloom and Glare: post effects on the finished render (the Render
//! window's Image Processing tab).
//!
//! They work on the linear float picture before the view transform, so
//! they see real brightness: a light at 20 glows far more than a white
//! wall at 1. They are applied after the render (UIC-170: the render
//! itself stays Cycles'), never written into the Recent Frames cache, so
//! a change re-applies at once without rendering again. Both are off in
//! new and old scenes.
//!
//! - Bloom: the parts brighter than Threshold, blurred into a soft glow
//!   (several Gaussians from small to Radius, added together, so the glow
//!   has a bright core and a long, soft tail).
//! - Glare: the same bright parts drawn out into straight streaks (a star
//!   filter): Streaks rays, turned by Angle, fading out over Length.
//!
//! Sizes are fractions of the picture's width, so a glow looks the same
//! at any resolution. Each effect adds `Intensity` times the bright light
//! it spreads (the blur and streak kernels sum to 1), so 100 % adds as
//! much light again as was over the threshold.
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// The Streaks choices: rays around each bright point.
pub const STREAK_CHOICES: [u32; 4] = [2, 4, 6, 8];
/// Largest Intensity (as a factor; 10 = 1000 %).
pub const MAX_INTENSITY: f64 = 10.0;
/// Largest Threshold (linear brightness).
pub const MAX_THRESHOLD: f64 = 1000.0;
/// Bloom Radius range, as a fraction of the picture's width.
pub const RADIUS_RANGE: std::ops::RangeInclusive<f64> = 0.001..=1.0;
/// Glare Length range, as a fraction of the picture's width.
pub const LENGTH_RANGE: std::ops::RangeInclusive<f64> = 0.005..=2.0;

/// Bloom: a soft glow around bright parts of the picture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bloom {
    pub enabled: bool,
    /// Linear brightness where the glow starts (1 is white).
    pub threshold: f64,
    /// How much of the light over the threshold is spread into the glow.
    pub intensity: f64,
    /// How far the glow reaches, as a fraction of the picture's width.
    pub radius: f64,
    /// The glow's colour, linear RGB (white keeps the light's own colour).
    pub tint: [f64; 3],
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 1.0,
            intensity: 0.5,
            radius: 0.05,
            tint: [1.0; 3],
        }
    }
}

/// Glare: straight streaks out of bright points (a star filter).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Glare {
    pub enabled: bool,
    /// Linear brightness where streaks start.
    pub threshold: f64,
    /// How much of the light over the threshold goes into the streaks.
    pub intensity: f64,
    /// Rays around each bright point: one of `STREAK_CHOICES`.
    pub streaks: u32,
    /// The first ray's direction, degrees counter-clockwise from the right.
    pub angle: f64,
    /// How far a streak reaches, as a fraction of the picture's width.
    pub length: f64,
    /// The streaks' colour, linear RGB.
    pub tint: [f64; 3],
}

impl Default for Glare {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 2.0,
            intensity: 0.3,
            streaks: 4,
            angle: 45.0,
            length: 0.2,
            tint: [1.0; 3],
        }
    }
}

/// The scene's post effects (`RenderSettings::post`, native version 68).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PostEffects {
    pub bloom: Bloom,
    pub glare: Glare,
}

impl PostEffects {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Whether `apply` changes anything.
    pub fn is_active(&self) -> bool {
        (self.bloom.enabled && self.bloom.intensity > 0.0)
            || (self.glare.enabled && self.glare.intensity > 0.0)
    }

    pub fn validate(&self) -> crate::Result<()> {
        let fine = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        let tint = |t: &[f64; 3]| t.iter().all(|v| fine(*v, 0.0, 1e6));
        let (b, g) = (&self.bloom, &self.glare);
        if !fine(b.threshold, 0.0, MAX_THRESHOLD) || !fine(g.threshold, 0.0, MAX_THRESHOLD) {
            return Err(format!("Threshold must be 0 to {MAX_THRESHOLD}"));
        }
        if !fine(b.intensity, 0.0, MAX_INTENSITY) || !fine(g.intensity, 0.0, MAX_INTENSITY) {
            return Err(format!(
                "Intensity must be 0 to {} %",
                MAX_INTENSITY * 100.0
            ));
        }
        if !fine(b.radius, *RADIUS_RANGE.start(), *RADIUS_RANGE.end()) {
            return Err("Bloom Radius must be 0.1 to 100 % of the picture's width".into());
        }
        if !fine(g.length, *LENGTH_RANGE.start(), *LENGTH_RANGE.end()) {
            return Err("Glare Length must be 0.5 to 200 % of the picture's width".into());
        }
        if !STREAK_CHOICES.contains(&g.streaks) {
            return Err("Streaks must be 2, 4, 6 or 8".into());
        }
        if !g.angle.is_finite() {
            return Err("Glare Angle must be a number".into());
        }
        if !tint(&b.tint) || !tint(&g.tint) {
            return Err("Tint must be a colour with no negative parts".into());
        }
        Ok(())
    }
}

impl crate::Scene {
    /// Old files: a Final Color output's own Bloom (removed, Jason BG-Q2)
    /// becomes the scene's Bloom when the scene has none of its own, at
    /// the same threshold, full strength and about the same reach. Every
    /// output's old Bloom fields are then cleared, so nothing writes them.
    pub(crate) fn move_old_output_bloom(&mut self) {
        use crate::shader_tree::{NodeKind, OutputEffect};
        let fresh = crate::render_outputs::OutputSettings::default();
        let tree = &self.shader_tree;
        let finals: Vec<_> = tree
            .outputs
            .keys()
            .filter(|id| {
                matches!(
                    tree.get(**id).map(|n| &n.kind),
                    Some(NodeKind::RenderOutput(OutputEffect::FinalColor))
                )
            })
            .copied()
            .collect();
        let mut old = None;
        for (id, s) in &mut self.shader_tree.outputs {
            if finals.contains(id) && s.bloom && old.is_none() {
                old = Some((s.bloom_threshold, s.bloom_radius));
            }
            s.bloom = fresh.bloom;
            s.bloom_threshold = fresh.bloom_threshold;
            s.bloom_radius = fresh.bloom_radius;
        }
        let Some((threshold, radius)) = old else {
            return;
        };
        if self.render.post.bloom != Bloom::default() {
            return;
        }
        // The old radius was a share of the diagonal, a blur of about a
        // third of it; the new one is a share of the width reaching two
        // sigma. Measured on the render frame.
        let [w, h] = self.render.frame.map(|v| f64::from(v.max(1)));
        let reach = radius * (w * w + h * h).sqrt() * 2.0 / 3.0 / w;
        self.render.post.bloom = Bloom {
            enabled: true,
            threshold: threshold.clamp(0.0, MAX_THRESHOLD),
            intensity: 1.0,
            radius: reach.clamp(*RADIUS_RANGE.start(), *RADIUS_RANGE.end()),
            tint: [1.0; 3],
        };
    }
}

/// A float RGB picture, row by row.
struct Plane {
    w: usize,
    h: usize,
    px: Vec<[f32; 3]>,
}

impl Plane {
    fn zero(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            px: vec![[0.0; 3]; w * h],
        }
    }
    /// Bilinear sample at pixel-centre coordinates; 0 outside.
    fn sample(&self, x: f32, y: f32) -> [f32; 3] {
        let (x0, y0) = (x.floor(), y.floor());
        let (tx, ty) = (x - x0, y - y0);
        let (x0, y0) = (x0 as isize, y0 as isize);
        let mut out = [0.0; 3];
        for (dy, wy) in [(0, 1.0 - ty), (1, ty)] {
            let yy = y0 + dy;
            if wy == 0.0 || yy < 0 || yy >= self.h as isize {
                continue;
            }
            for (dx, wx) in [(0, 1.0 - tx), (1, tx)] {
                let xx = x0 + dx;
                if wx == 0.0 || xx < 0 || xx >= self.w as isize {
                    continue;
                }
                let p = self.px[yy as usize * self.w + xx as usize];
                let k = wx * wy;
                for c in 0..3 {
                    out[c] += p[c] * k;
                }
            }
        }
        out
    }
}

/// Adds the scene's bloom and glare to a linear RGBA picture, `width` ×
/// `height`. Off (or at zero intensity) it leaves every byte as it was.
/// Alpha is never changed.
pub fn apply(p: &PostEffects, rgba: &mut [f32], width: u32, height: u32) {
    let (w, h) = (width as usize, height as usize);
    if !p.is_active() || w == 0 || h == 0 || rgba.len() != w * h * 4 {
        return;
    }
    let mut add = Plane::zero(w, h);
    if p.bloom.enabled && p.bloom.intensity > 0.0 {
        let glow = bloom(&p.bloom, rgba, w, h);
        accumulate(&mut add, &glow, scale(p.bloom.intensity, p.bloom.tint));
    }
    if p.glare.enabled && p.glare.intensity > 0.0 {
        let streaks = glare(&p.glare, rgba, w, h);
        accumulate(&mut add, &streaks, scale(p.glare.intensity, p.glare.tint));
    }
    rgba.par_chunks_mut(4)
        .zip(add.px.par_iter())
        .for_each(|(px, a)| {
            for c in 0..3 {
                px[c] += a[c];
            }
        });
}

fn scale(intensity: f64, tint: [f64; 3]) -> [f32; 3] {
    tint.map(|t| (t * intensity) as f32)
}

fn accumulate(into: &mut Plane, from: &Plane, k: [f32; 3]) {
    into.px
        .par_iter_mut()
        .zip(from.px.par_iter())
        .for_each(|(a, b)| {
            for c in 0..3 {
                a[c] += b[c] * k[c];
            }
        });
}

/// The light over `threshold`, keeping each pixel's colour: a pixel of
/// luminance `l` keeps `(l - t) / l` of itself. Negative, NaN and infinite
/// values give nothing, so one bad pixel can't flood the picture.
fn bright(rgba: &[f32], w: usize, h: usize, threshold: f64) -> Plane {
    let t = threshold as f32;
    let px = rgba
        .par_chunks(4)
        .map(|p| {
            let c = [0, 1, 2].map(|k| {
                let v = p[k];
                if v.is_finite() { v.max(0.0) } else { 0.0 }
            });
            let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            if l <= t || l <= 0.0 {
                [0.0; 3]
            } else {
                let k = (l - t) / l;
                c.map(|v| v * k)
            }
        })
        .collect();
    Plane { w, h, px }
}

/// Gaussians from Radius down by halves, added with equal weights.
const BLOOM_SCALES: usize = 5;

fn bloom(b: &Bloom, rgba: &[f32], w: usize, h: usize) -> Plane {
    let src = bright(rgba, w, h, b.threshold);
    // Half-size copies: a wide blur runs on a small picture (a pyramid).
    let mut pyramid = vec![src];
    let mut out = Plane::zero(w, h);
    let base = (b.radius * w as f64).max(0.5);
    let weight = 1.0 / BLOOM_SCALES as f32;
    for i in 0..BLOOM_SCALES {
        // Sigma in full-size pixels. The three-box blur ends at about two
        // sigma, so the widest glow reaches Radius.
        let sigma = base / 2.0 / f64::from(1u32 << i);
        // Blur at the level where sigma is still 2 pixels or more.
        let level = ((sigma / 2.0).log2().floor().max(0.0) as usize).min(8);
        while pyramid.len() <= level {
            let next = halve(pyramid.last().expect("level 0"));
            pyramid.push(next);
        }
        let at = &pyramid[level];
        let s = sigma * at.w as f64 / w as f64;
        let blurred = gaussian(at, s);
        add_resampled(&mut out, &blurred, weight);
    }
    out
}

/// A 2×2 box average, rounding odd sizes up.
fn halve(p: &Plane) -> Plane {
    let (w, h) = (p.w.div_ceil(2).max(1), p.h.div_ceil(2).max(1));
    let mut out = Plane::zero(w, h);
    out.px.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let mut sum = [0.0; 3];
            let mut n = 0.0;
            for yy in [2 * y, 2 * y + 1] {
                for xx in [2 * x, 2 * x + 1] {
                    if xx < p.w && yy < p.h {
                        let v = p.px[yy * p.w + xx];
                        for c in 0..3 {
                            sum[c] += v[c];
                        }
                        n += 1.0;
                    }
                }
            }
            *o = sum.map(|v| v / n);
        }
    });
    out
}

/// Adds `src` (any size) into `out`, stretched to `out`'s size with
/// bilinear filtering, times `k`.
fn add_resampled(out: &mut Plane, src: &Plane, k: f32) {
    let (sx, sy) = (src.w as f32 / out.w as f32, src.h as f32 / out.h as f32);
    let same = src.w == out.w && src.h == out.h;
    let w = out.w;
    out.px.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let v = if same {
                src.px[y * src.w + x]
            } else {
                // Clamp to the edge pixels: the picture's border isn't dark.
                let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, (src.w - 1) as f32);
                let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, (src.h - 1) as f32);
                src.sample(fx, fy)
            };
            for c in 0..3 {
                o[c] += v[c] * k;
            }
        }
    });
}

/// A Gaussian blur of `sigma` pixels: three box blurs each way.
fn gaussian(p: &Plane, sigma: f64) -> Plane {
    if sigma < 0.3 {
        return Plane {
            w: p.w,
            h: p.h,
            px: p.px.clone(),
        };
    }
    // Three boxes of width 2r + 1 have variance 3 ((2r + 1)² - 1) / 12.
    let r = (((4.0 * sigma * sigma + 1.0).sqrt() - 1.0) / 2.0)
        .round()
        .max(1.0) as usize;
    let mut rows = p.px.clone();
    for _ in 0..3 {
        rows = box_rows(&rows, p.w, r);
    }
    let mut cols = transpose(&rows, p.w, p.h);
    for _ in 0..3 {
        cols = box_rows(&cols, p.h, r);
    }
    Plane {
        w: p.w,
        h: p.h,
        px: transpose(&cols, p.h, p.w),
    }
}

/// A running-sum box blur of radius `r` along each row of width `w`,
/// averaging over the part of the window inside the row.
fn box_rows(src: &[[f32; 3]], w: usize, r: usize) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0; 3]; src.len()];
    out.par_chunks_mut(w)
        .zip(src.par_chunks(w))
        .for_each(|(o, s)| {
            let mut sum = [0.0f64; 3];
            // Window [i - r, i + r]; prime it with [0, r).
            for v in s.iter().take(r.min(w)) {
                for c in 0..3 {
                    sum[c] += f64::from(v[c]);
                }
            }
            for i in 0..w {
                if i + r < w {
                    for c in 0..3 {
                        sum[c] += f64::from(s[i + r][c]);
                    }
                }
                if i > r {
                    for c in 0..3 {
                        sum[c] -= f64::from(s[i - r - 1][c]);
                    }
                }
                let n = ((i + r).min(w - 1) - i.saturating_sub(r) + 1) as f64;
                o[i] = sum.map(|v| (v / n) as f32);
            }
        });
    out
}

fn transpose(src: &[[f32; 3]], w: usize, h: usize) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0; 3]; src.len()];
    out.par_chunks_mut(h).enumerate().for_each(|(x, col)| {
        for (y, o) in col.iter_mut().enumerate() {
            *o = src[y * w + x];
        }
    });
    out
}

/// What is left of a streak's brightness at its Length.
const STREAK_TAIL: f64 = 0.01;
/// Taps per streak pass: each pass reaches four times as far.
const TAPS: usize = 4;

/// Pictures wider than this draw their streaks at half size (or less), so
/// a 4K glare stays quick to change; streaks are soft enough not to show it.
const GLARE_FULL_WIDTH: usize = 1280;

fn glare(g: &Glare, rgba: &[f32], w: usize, h: usize) -> Plane {
    let mut src = bright(rgba, w, h, g.threshold);
    let mut width = w;
    while width > GLARE_FULL_WIDTH {
        src = halve(&src);
        width = src.w;
    }
    let drawn = streaks(g, &src);
    if drawn.w == w && drawn.h == h {
        return drawn;
    }
    let mut out = Plane::zero(w, h);
    add_resampled(&mut out, &drawn, 1.0);
    out
}

/// The streaks of `src`'s light, at `src`'s size.
fn streaks(g: &Glare, src: &Plane) -> Plane {
    let (w, h) = (src.w, src.h);
    let length = (g.length * w as f64).max(1.0);
    // Per-pixel fade, so the streak is down to STREAK_TAIL at its length.
    let a = STREAK_TAIL.powf(1.0 / length);
    let passes = (length.log(TAPS as f64).ceil() as u32).clamp(1, 8);
    let reach = (TAPS as f64).powi(passes as i32);
    // The whole streak's weights, sum of a^n for n < reach: dividing by it
    // makes each streak carry exactly the light it spreads.
    let total = (1.0 - a.powf(reach)) / (1.0 - a);
    let n = g.streaks.max(1);
    let mut out = Plane::zero(w, h);
    for k in 0..n {
        let theta = (g.angle + 360.0 * f64::from(k) / f64::from(n)).to_radians();
        // Picture rows run down, so "up" is minus y.
        let dir = [theta.cos() as f32, -theta.sin() as f32];
        let mut s = src.px.clone();
        for pass in 0..passes {
            let step = (TAPS as f64).powi(pass as i32);
            let weights: Vec<f32> = (0..TAPS).map(|t| a.powf(step * t as f64) as f32).collect();
            let from = Plane { w, h, px: s };
            let mut next = vec![[0.0; 3]; w * h];
            next.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
                for (x, o) in row.iter_mut().enumerate() {
                    let mut sum = [0.0; 3];
                    for (t, wt) in weights.iter().enumerate() {
                        let d = (step * t as f64) as f32;
                        // Sampling ahead along `dir` draws each bright
                        // point's streak back the other way: rays come in
                        // opposite pairs once every direction has run.
                        let v = from.sample(x as f32 + dir[0] * d, y as f32 + dir[1] * d);
                        for c in 0..3 {
                            sum[c] += v[c] * wt;
                        }
                    }
                    *o = sum;
                }
            });
            s = next;
        }
        let k = (1.0 / (total * f64::from(n))) as f32;
        out.px.par_iter_mut().zip(s.par_iter()).for_each(|(o, v)| {
            for c in 0..3 {
                o[c] += v[c] * k;
            }
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dark grey picture with one bright pixel at (cx, cy).
    fn spot(w: u32, h: u32, cx: u32, cy: u32, v: f32) -> Vec<f32> {
        let mut p = vec![0.1f32; (w * h * 4) as usize];
        for a in p.iter_mut().skip(3).step_by(4) {
            *a = 1.0;
        }
        let i = ((cy * w + cx) * 4) as usize;
        p[i..i + 3].copy_from_slice(&[v; 3]);
        p
    }

    fn on() -> PostEffects {
        PostEffects {
            bloom: Bloom {
                enabled: true,
                ..Default::default()
            },
            glare: Glare {
                enabled: true,
                ..Default::default()
            },
        }
    }

    fn rgb_sum(p: &[f32]) -> f64 {
        p.chunks(4)
            .map(|c| f64::from(c[0]) + f64::from(c[1]) + f64::from(c[2]))
            .sum()
    }

    #[test]
    fn off_leaves_every_byte_as_it_was() {
        let before = spot(32, 16, 8, 8, 50.0);
        for p in [PostEffects::default(), {
            let mut p = on();
            p.bloom.intensity = 0.0;
            p.glare.intensity = 0.0;
            p
        }] {
            let mut px = before.clone();
            apply(&p, &mut px, 32, 16);
            assert_eq!(
                px.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                before.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn nothing_over_the_threshold_changes_nothing() {
        let before = spot(40, 30, 20, 15, 0.9);
        let mut px = before.clone();
        apply(&on(), &mut px, 40, 30);
        assert_eq!(px, before);
    }

    #[test]
    fn bloom_adds_its_intensity_of_the_light_over_the_threshold() {
        let (w, h) = (257, 201);
        let before = spot(w, h, 128, 100, 101.0);
        let mut p = on();
        p.glare.enabled = false;
        p.bloom.radius = 0.04;
        p.bloom.intensity = 0.5;
        let mut px = before.clone();
        apply(&p, &mut px, w, h);
        // 100 over the threshold in each of three channels, half of it
        // spread out: the box blurs keep the light they spread.
        let added = rgb_sum(&px) - rgb_sum(&before);
        assert!((added - 150.0).abs() < 150.0 * 0.03, "added {added}");
        // It glows around the spot, fading with distance, and leaves the
        // far corner alone.
        let at = |x: u32, y: u32| px[((y * w + x) * 4) as usize];
        assert!(at(129, 100) > at(132, 100) && at(132, 100) > at(135, 100));
        assert!(at(135, 100) > 0.1);
        assert!((at(0, 0) - 0.1).abs() < 1e-4);
        // Alpha stays.
        assert!(px.iter().skip(3).step_by(4).all(|a| *a == 1.0));
    }

    #[test]
    fn bloom_looks_the_same_at_twice_the_size() {
        let glow = |w: u32, h: u32| {
            let mut p = on();
            p.glare.enabled = false;
            p.bloom.radius = 0.2;
            let mut px = spot(w, h, w / 2, h / 2, 1001.0);
            // A 2×2 spot at twice the size carries four times the light.
            if w > 200 {
                for (dx, dy) in [(1, 0), (0, 1), (1, 1)] {
                    let i = (((h / 2 + dy) * w + w / 2 + dx) * 4) as usize;
                    px[i..i + 3].copy_from_slice(&[1001.0; 3]);
                }
            }
            apply(&p, &mut px, w, h);
            // The glow 1/10 of the width out along the row (the big
            // spot's centre is half a pixel right of w / 2).
            let at = |x: u32| px[(((h / 2) * w + x) * 4) as usize] - 0.1;
            if w > 200 {
                (at(w / 2 + w / 10) + at(w / 2 + w / 10 + 1)) / 2.0
            } else {
                at(w / 2 + w / 10)
            }
        };
        let (small, big) = (glow(120, 60), glow(240, 120));
        assert!(small > 0.01, "{small}");
        let ratio = big / small;
        assert!((0.8..1.25).contains(&ratio), "{small} vs {big}");
    }

    #[test]
    fn glare_draws_streaks_along_its_angle_only() {
        let (w, h) = (161, 161);
        let before = spot(w, h, 80, 80, 1001.0);
        let mut p = on();
        p.bloom.enabled = false;
        p.glare.angle = 0.0;
        p.glare.streaks = 4;
        p.glare.length = 0.25;
        let mut px = before.clone();
        apply(&p, &mut px, w, h);
        let at = |x: u32, y: u32| px[((y * w + x) * 4) as usize] - 0.1;
        // Along the four rays...
        for (x, y) in [(100, 80), (60, 80), (80, 60), (80, 100)] {
            assert!(at(x, y) > 0.05, "({x}, {y}) {}", at(x, y));
        }
        // ...fading out, and nothing on the diagonals.
        assert!(at(90, 80) > at(110, 80));
        assert!(at(95, 95).abs() < 1e-5);
        // The streaks carry Intensity of the light over the threshold,
        // less what runs off the picture (none here: it fades first).
        let added = rgb_sum(&px) - rgb_sum(&before);
        let expect = 0.3 * 999.0 * 3.0;
        assert!((added - expect).abs() < expect * 0.03, "added {added}");
    }

    #[test]
    fn two_streaks_make_one_line_through_the_point() {
        let (w, h) = (121, 61);
        let mut px = spot(w, h, 60, 30, 1001.0);
        let mut p = on();
        p.bloom.enabled = false;
        p.glare.angle = 0.0;
        p.glare.streaks = 2;
        apply(&p, &mut px, w, h);
        let at = |x: u32, y: u32| px[((y * w + x) * 4) as usize] - 0.1;
        assert!(at(70, 30) > 0.05 && at(50, 30) > 0.05);
        assert!((at(70, 30) - at(50, 30)).abs() < at(70, 30) * 0.05);
        assert!(at(60, 40).abs() < 1e-5);
    }

    #[test]
    fn tint_colours_the_glow_only() {
        let (w, h) = (64, 64);
        let before = spot(w, h, 32, 32, 11.0);
        let mut p = on();
        p.glare.enabled = false;
        p.bloom.tint = [1.0, 0.0, 0.0];
        let mut px = before.clone();
        apply(&p, &mut px, w, h);
        let i = ((32 * w + 34) * 4) as usize;
        assert!(px[i] > 0.1 + 1e-3);
        assert_eq!(px[i + 1], 0.1);
        assert_eq!(px[i + 2], 0.1);
    }

    #[test]
    fn bad_pixels_and_odd_sizes_are_safe() {
        for (w, h) in [(1, 1), (1, 7), (7, 1), (3, 2)] {
            let mut px = spot(w, h, 0, 0, f32::INFINITY);
            px[1] = f32::NAN;
            apply(&on(), &mut px, w, h);
            // The bad pixel is kept as it was; nothing else turns bad.
            assert!(px[4..].iter().all(|v| v.is_finite()), "{w}x{h}");
        }
        // A picture of the wrong length is left alone.
        let mut px = vec![5.0; 7];
        apply(&on(), &mut px, 2, 2);
        assert_eq!(px, vec![5.0; 7]);
    }

    /// Time on a 1920×1080 picture. Run with
    /// `cargo test --release --workspace -- post_effects_speed_probe --ignored --nocapture`.
    #[test]
    #[ignore = "speed probe"]
    fn post_effects_speed_probe() {
        let (w, h) = (1920, 1080);
        let base = spot(w, h, 900, 500, 50.0);
        for (name, p) in [
            ("bloom", {
                let mut p = on();
                p.glare.enabled = false;
                p
            }),
            ("glare 8", {
                let mut p = on();
                p.bloom.enabled = false;
                p.glare.streaks = 8;
                p
            }),
            ("both", on()),
        ] {
            let mut px = base.clone();
            let t = std::time::Instant::now();
            apply(&p, &mut px, w, h);
            println!("{name}: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
        }
    }

    #[test]
    fn validate_refuses_out_of_range_settings() {
        assert!(on().validate().is_ok());
        let bad: [fn(&mut PostEffects); 6] = [
            |p| p.bloom.threshold = -1.0,
            |p| p.glare.intensity = f64::NAN,
            |p| p.bloom.radius = 0.0,
            |p| p.glare.length = 5.0,
            |p| p.glare.streaks = 3,
            |p| p.bloom.tint = [1.0, -0.5, 1.0],
        ];
        for f in bad {
            let mut p = on();
            f(&mut p);
            assert!(p.validate().is_err(), "{p:?}");
        }
    }
}
