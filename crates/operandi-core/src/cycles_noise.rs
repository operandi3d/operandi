// SPDX-FileCopyrightText: 2009-2010 Sony Pictures Imageworks Inc., et al.
// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-FileCopyrightText: 2026 Operandi contributors
//
// SPDX-License-Identifier: Apache-2.0 AND BSD-3-Clause
//
// Adapted from Cycles (https://projects.blender.org/blender/cycles) at
// commit 3eb35f85996baa6c1b2e19bf297b22745c85e321: src/util/hash.h
// (Jenkins lookup3 hashes, Apache-2.0), src/kernel/svm/noise.h (Perlin
// noise, BSD-3-Clause, adapted there from Open Shading Language),
// fractal_noise.h, noisetex.h, white_noise.h, checker.h, gradient.h and
// wave.h (Apache-2.0). The C++ is rewritten as Rust, one function per
// dimension count folded into one taking the count; the sums, constants and
// hash seeds are Cycles' own, in f32 as Cycles computes them, so the
// viewport draws the pattern the render does. `shader_func.wgsl` holds the
// same code as WGSL. See docs/DEPENDENCIES.md and docs/licenses/cycles/.

//! Cycles' procedural textures on the CPU (Noise, White Noise, Checker,
//! Gradient, Wave), for node graph layers drawn in the viewport
//! (MaterialX stage 5). The GPU runs the WGSL twin in `shader_func.wgsl`;
//! tests check the two agree.

/// A point of up to four dimensions; unused components are 0.
pub type P4 = [f32; 4];

fn rot(x: u32, k: u32) -> u32 {
    x.rotate_left(k)
}

fn finalise(a: &mut u32, b: &mut u32, c: &mut u32) {
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 14));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 11));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 25));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 16));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 4));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 14));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 24));
}

fn mix_words(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 4);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 6);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 8);
    *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 16);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 19);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 4);
    *b = b.wrapping_add(*a);
}

/// Jenkins lookup3 of `n` (1 to 4) words, as Cycles' `hash_uint{,2,3,4}`.
pub fn hash(k: &[u32]) -> u32 {
    let n = k.len() as u32;
    let start = 0xdeadbeef_u32.wrapping_add(n << 2).wrapping_add(13);
    let (mut a, mut b, mut c) = (start, start, start);
    match k {
        [x] => a = a.wrapping_add(*x),
        [x, y] => {
            b = b.wrapping_add(*y);
            a = a.wrapping_add(*x);
        }
        [x, y, z] => {
            c = c.wrapping_add(*z);
            b = b.wrapping_add(*y);
            a = a.wrapping_add(*x);
        }
        [x, y, z, w] => {
            a = a.wrapping_add(*x);
            b = b.wrapping_add(*y);
            c = c.wrapping_add(*z);
            mix_words(&mut a, &mut b, &mut c);
            a = a.wrapping_add(*w);
        }
        _ => return 0,
    }
    finalise(&mut a, &mut b, &mut c);
    c
}

/// [0, u32::MAX] to [0, 1] (`uint_to_float_incl`).
fn unit(n: u32) -> f32 {
    n as f32 * (1.0 / 4294967295u32 as f32)
}

/// `hash_float{,2,3,4}_to_float`: the floats' bits hashed to 0..1.
pub fn hash_floats(k: &[f32]) -> f32 {
    let bits: Vec<u32> = k.iter().map(|v| v.to_bits()).collect();
    unit(hash(&bits))
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn negate_if(v: f32, c: u32) -> f32 {
    if c != 0 { -v } else { v }
}

/// Perlin gradient of `dims` components (`grad1` to `grad4`).
fn grad(dims: usize, h: u32, f: P4) -> f32 {
    match dims {
        1 => {
            let h = h & 15;
            negate_if(1.0 + (h & 7) as f32, h & 8) * f[0]
        }
        2 => {
            let h = h & 7;
            let u = if h < 4 { f[0] } else { f[1] };
            let v = 2.0 * if h < 4 { f[1] } else { f[0] };
            negate_if(u, h & 1) + negate_if(v, h & 2)
        }
        3 => {
            let h = h & 15;
            let u = if h < 8 { f[0] } else { f[1] };
            let vt = if h == 12 || h == 14 { f[0] } else { f[2] };
            let v = if h < 4 { f[1] } else { vt };
            negate_if(u, h & 1) + negate_if(v, h & 2)
        }
        _ => {
            let h = h & 31;
            let u = if h < 24 { f[0] } else { f[1] };
            let v = if h < 16 { f[1] } else { f[2] };
            let s = if h < 8 { f[2] } else { f[3] };
            negate_if(u, h & 1) + negate_if(v, h & 2) + negate_if(s, h & 4)
        }
    }
}

/// Perlin noise of `dims` (1 to 4) dimensions, roughly −1..1 before scaling.
fn perlin(dims: usize, p: P4) -> f32 {
    let mut cell = [0i32; 4];
    let mut f = [0f32; 4];
    for k in 0..dims {
        let fl = p[k].floor();
        cell[k] = fl as i32;
        f[k] = p[k] - fl;
    }
    // The corners in Cycles' order (x fastest), interpolated the same way:
    // x pairs first, then y, z and w.
    let corners = 1usize << dims;
    let mut v = [0f32; 16];
    for (i, slot) in v.iter_mut().enumerate().take(corners) {
        let mut key = [0u32; 4];
        let mut d = [0f32; 4];
        for k in 0..dims {
            let bit = (i >> k) & 1;
            key[k] = cell[k].wrapping_add(bit as i32) as u32;
            d[k] = f[k] - bit as f32;
        }
        *slot = grad(dims, hash(&key[..dims]), d);
    }
    let mut n = corners;
    for (k, fk) in f.iter().enumerate().take(dims) {
        let t = fade(*fk);
        n /= 2;
        // `bi_mix`/`tri_mix` weigh as (1 − t)·a + t·b; 1D and the 4D w
        // step use `mix` (a + t·(b − a)). Both are kept so results stay
        // within f32 rounding of Cycles'.
        let lerp = dims == 1 || (dims == 4 && k == 3);
        for i in 0..n {
            let (a, b) = (v[2 * i], v[2 * i + 1]);
            v[i] = if lerp {
                a + t * (b - a)
            } else {
                (1.0 - t) * a + t * b
            };
        }
    }
    v[0]
}

const NOISE_SCALE: [f32; 4] = [0.25, 0.6616, 0.9820, 0.8344];

/// Signed noise, −1..1 (`snoise_1d` to `snoise_4d`).
pub fn snoise(dims: usize, p: P4) -> f32 {
    let mut q = [0f32; 4];
    for k in 0..dims {
        let correction = if p[k].abs() >= 1_000_000.0 { 0.5 } else { 0.0 };
        q[k] = p[k] % 100000.0 + correction;
    }
    NOISE_SCALE[dims - 1] * perlin(dims, q)
}

fn scaled(p: P4, s: f32) -> P4 {
    p.map(|v| v * s)
}

/// The Noise texture's fractal types (`NodeNoiseType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NoiseKind {
    MultiFractal,
    Fbm,
    HybridMultiFractal,
    RidgedMultiFractal,
    HeteroTerrain,
}
impl NoiseKind {
    /// From Cycles' enum name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "multifractal" => NoiseKind::MultiFractal,
            "fBM" | "fbm" => NoiseKind::Fbm,
            "hybrid_multifractal" => NoiseKind::HybridMultiFractal,
            "ridged_multifractal" => NoiseKind::RidgedMultiFractal,
            "hetero_terrain" => NoiseKind::HeteroTerrain,
            _ => return None,
        })
    }
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// The fractal settings of a Noise texture.
#[derive(Clone, Copy, Debug)]
pub struct Fractal {
    pub detail: f32,
    pub roughness: f32,
    pub lacunarity: f32,
    pub offset: f32,
    pub gain: f32,
}

fn fbm(dims: usize, p: P4, f: &Fractal, normalize: bool) -> f32 {
    let (mut fscale, mut amp, mut maxamp, mut sum) = (1f32, 1f32, 0f32, 0f32);
    for _ in 0..=(f.detail as i32) {
        let t = snoise(dims, scaled(p, fscale));
        sum += t * amp;
        maxamp += amp;
        amp *= f.roughness;
        fscale *= f.lacunarity;
    }
    let rmd = f.detail - f.detail.floor();
    if rmd != 0.0 {
        let t = snoise(dims, scaled(p, fscale));
        let sum2 = sum + t * amp;
        return if normalize {
            let a = 0.5 * sum / maxamp + 0.5;
            let b = 0.5 * sum2 / (maxamp + amp) + 0.5;
            a + rmd * (b - a)
        } else {
            sum + rmd * (sum2 - sum)
        };
    }
    if normalize {
        0.5 * sum / maxamp + 0.5
    } else {
        sum
    }
}

fn multi_fractal(dims: usize, mut p: P4, f: &Fractal) -> f32 {
    let (mut value, mut pwr) = (1f32, 1f32);
    for _ in 0..=(f.detail as i32) {
        value *= pwr * snoise(dims, p) + 1.0;
        pwr *= f.roughness;
        p = scaled(p, f.lacunarity);
    }
    let rmd = f.detail - f.detail.floor();
    if rmd != 0.0 {
        value *= rmd * pwr * snoise(dims, p) + 1.0;
    }
    value
}

fn hetero_terrain(dims: usize, mut p: P4, f: &Fractal) -> f32 {
    let mut pwr = f.roughness;
    let mut value = f.offset + snoise(dims, p);
    p = scaled(p, f.lacunarity);
    for _ in 1..=(f.detail as i32) {
        let increment = (snoise(dims, p) + f.offset) * pwr * value;
        value += increment;
        pwr *= f.roughness;
        p = scaled(p, f.lacunarity);
    }
    let rmd = f.detail - f.detail.floor();
    if rmd != 0.0 {
        let increment = (snoise(dims, p) + f.offset) * pwr * value;
        value += rmd * increment;
    }
    value
}

fn hybrid_multi_fractal(dims: usize, mut p: P4, f: &Fractal) -> f32 {
    let (mut pwr, mut value, mut weight) = (1f32, 0f32, 1f32);
    let mut i = 0;
    while weight > 0.001 && i <= f.detail as i32 {
        weight = weight.min(1.0);
        let signal = (snoise(dims, p) + f.offset) * pwr;
        pwr *= f.roughness;
        value += weight * signal;
        weight *= f.gain * signal;
        p = scaled(p, f.lacunarity);
        i += 1;
    }
    let rmd = f.detail - f.detail.floor();
    if rmd != 0.0 && weight > 0.001 {
        weight = weight.min(1.0);
        let signal = (snoise(dims, p) + f.offset) * pwr;
        value += rmd * weight * signal;
    }
    value
}

fn ridged_multi_fractal(dims: usize, mut p: P4, f: &Fractal) -> f32 {
    let mut pwr = f.roughness;
    let mut signal = f.offset - snoise(dims, p).abs();
    signal *= signal;
    let mut value = signal;
    for _ in 1..=(f.detail as i32) {
        p = scaled(p, f.lacunarity);
        let weight = (signal * f.gain).clamp(0.0, 1.0);
        signal = f.offset - snoise(dims, p).abs();
        signal *= signal;
        signal *= weight;
        value += signal * pwr;
        pwr *= f.roughness;
    }
    value
}

fn noise_select(dims: usize, p: P4, f: &Fractal, kind: NoiseKind, normalize: bool) -> f32 {
    match kind {
        NoiseKind::MultiFractal => multi_fractal(dims, p, f),
        NoiseKind::Fbm => fbm(dims, p, f, normalize),
        NoiseKind::HybridMultiFractal => hybrid_multi_fractal(dims, p, f),
        NoiseKind::RidgedMultiFractal => ridged_multi_fractal(dims, p, f),
        NoiseKind::HeteroTerrain => hetero_terrain(dims, p, f),
    }
}

/// `random_float{,2,3,4}_offset`: a 100..200 offset per component from a
/// seed.
fn random_offset(dims: usize, seed: f32) -> P4 {
    let mut out = [0f32; 4];
    if dims == 1 {
        out[0] = 100.0 + hash_floats(&[seed]) * 100.0;
        return out;
    }
    for (k, o) in out.iter_mut().enumerate().take(dims) {
        *o = 100.0 + hash_floats(&[seed, k as f32]) * 100.0;
    }
    out
}

fn add(a: P4, b: P4) -> P4 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

/// The Noise texture (`svm_node_tex_noise`) at `vector` and `w`, before
/// Scale: its Fac (`color` false, as a grey) or its Color.
#[allow(clippy::too_many_arguments)]
pub fn noise_texture(
    dims: usize,
    kind: NoiseKind,
    normalize: bool,
    color: bool,
    vector: [f32; 3],
    w: f32,
    scale: f32,
    fractal: Fractal,
    distortion: f32,
) -> [f32; 3] {
    let dims = dims.clamp(1, 4);
    let f = Fractal {
        detail: fractal.detail.clamp(0.0, 15.0),
        roughness: fractal.roughness.max(0.0),
        ..fractal
    };
    let (v, w) = (vector.map(|c| c * scale), w * scale);
    let mut p: P4 = match dims {
        1 => [w, 0., 0., 0.],
        2 => [v[0], v[1], 0., 0.],
        3 => [v[0], v[1], v[2], 0.],
        _ => [v[0], v[1], v[2], w],
    };
    if distortion != 0.0 {
        let mut d = [0f32; 4];
        for (k, dk) in d.iter_mut().enumerate().take(dims) {
            let seed = if dims == 1 { 0.0 } else { k as f32 };
            *dk = snoise(dims, add(p, random_offset(dims, seed))) * distortion;
        }
        p = add(p, d);
    }
    let value = noise_select(dims, p, &f, kind, normalize);
    if !color {
        return [value; 3];
    }
    let first = dims.max(1) as f32;
    [
        value,
        noise_select(
            dims,
            add(p, random_offset(dims, first)),
            &f,
            kind,
            normalize,
        ),
        noise_select(
            dims,
            add(p, random_offset(dims, first + 1.0)),
            &f,
            kind,
            normalize,
        ),
    ]
}

/// The White Noise texture: its Value (as a grey) or its Color.
pub fn white_noise(dims: usize, color: bool, v: [f32; 3], w: f32) -> [f32; 3] {
    let h = hash_floats;
    match (dims.clamp(1, 4), color) {
        (1, false) => [h(&[w]); 3],
        (1, true) => [h(&[w]), h(&[w, 1.0]), h(&[w, 2.0])],
        (2, false) => [h(&[v[0], v[1]]); 3],
        (2, true) => [
            h(&[v[0], v[1]]),
            h(&[v[0], v[1], 1.0]),
            h(&[v[0], v[1], 2.0]),
        ],
        (3, false) => [h(&v); 3],
        (3, true) => [
            h(&v),
            h(&[v[0], v[1], v[2], 1.0]),
            h(&[v[0], v[1], v[2], 2.0]),
        ],
        (_, false) => [h(&[v[0], v[1], v[2], w]); 3],
        (_, true) => [
            h(&[v[0], v[1], v[2], w]),
            h(&[v[2], v[0], w, v[1]]),
            h(&[w, v[2], v[1], v[0]]),
        ],
    }
}

/// The Checker texture's Fac at a point already times Scale.
pub fn checker(p: [f32; 3]) -> f32 {
    let i = p.map(|c| (((c + 0.000001) * 0.999999).floor() as i32).abs() % 2);
    if (i[0] == i[1]) == (i[2] == 1) {
        1.0
    } else {
        0.0
    }
}

/// The Gradient texture's types (`NodeGradientType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GradientKind {
    Linear,
    Quadratic,
    Easing,
    Diagonal,
    Radial,
    QuadraticSphere,
    Spherical,
}
impl GradientKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "linear" => GradientKind::Linear,
            "quadratic" => GradientKind::Quadratic,
            "easing" => GradientKind::Easing,
            "diagonal" => GradientKind::Diagonal,
            "radial" => GradientKind::Radial,
            "quadratic_sphere" => GradientKind::QuadraticSphere,
            "spherical" => GradientKind::Spherical,
            _ => return None,
        })
    }
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// The Gradient texture's Fac (0..1).
pub fn gradient(kind: GradientKind, p: [f32; 3]) -> f32 {
    let [x, y, z] = p;
    let f = match kind {
        GradientKind::Linear => x,
        GradientKind::Quadratic => {
            let r = x.max(0.0);
            r * r
        }
        GradientKind::Easing => {
            let r = x.clamp(0.0, 1.0);
            let t = r * r;
            3.0 * t - 2.0 * t * r
        }
        GradientKind::Diagonal => (x + y) * 0.5,
        GradientKind::Radial => y.atan2(x) / std::f32::consts::TAU + 0.5,
        GradientKind::QuadraticSphere | GradientKind::Spherical => {
            let r = (0.999999 - (x * x + y * y + z * z).sqrt()).max(0.0);
            if kind == GradientKind::Spherical {
                r
            } else {
                r * r
            }
        }
    };
    f.clamp(0.0, 1.0)
}

/// The Wave texture's settings (`NodeWave*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaveKind {
    /// Rings instead of bands.
    pub rings: bool,
    /// Bands: x, y, z, diagonal; rings: x, y, z, spherical.
    pub direction: u8,
    /// Sine, saw, triangle.
    pub profile: u8,
}

/// The Wave texture's Fac at a point already times Scale.
pub fn wave(
    kind: WaveKind,
    p: [f32; 3],
    distortion: f32,
    detail: f32,
    detail_scale: f32,
    detail_roughness: f32,
    phase: f32,
) -> f32 {
    let p = p.map(|c| (c + 0.000001) * 0.999999);
    let mut n = if !kind.rings {
        match kind.direction {
            0 => p[0] * 20.0,
            1 => p[1] * 20.0,
            2 => p[2] * 20.0,
            _ => (p[0] + p[1] + p[2]) * 10.0,
        }
    } else {
        let keep = match kind.direction {
            0 => [0.0, 1.0, 1.0],
            1 => [1.0, 0.0, 1.0],
            2 => [1.0, 1.0, 0.0],
            _ => [1.0; 3],
        };
        let r = [p[0] * keep[0], p[1] * keep[1], p[2] * keep[2]];
        (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt() * 20.0
    };
    n += phase;
    if distortion != 0.0 {
        let f = Fractal {
            detail: detail.clamp(0.0, 15.0),
            roughness: detail_roughness,
            lacunarity: 2.0,
            offset: 0.0,
            gain: 1.0,
        };
        let q = scaled([p[0], p[1], p[2], 0.0], detail_scale);
        n += distortion * (fbm(3, q, &f, true) * 2.0 - 1.0);
    }
    match kind.profile {
        0 => 0.5 + 0.5 * (n - std::f32::consts::FRAC_PI_2).sin(),
        1 => {
            let n = n / std::f32::consts::TAU;
            n - n.floor()
        }
        _ => {
            let n = n / std::f32::consts::TAU;
            (n - (n + 0.5).floor()).abs() * 2.0
        }
    }
}
