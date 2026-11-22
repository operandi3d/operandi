// SPDX-FileCopyrightText: 2019-2026 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
//
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender source/blender/gpu/shaders/material/
// gpu_shader_material_principled.bsl.hh and
// gpu_shader_material_open_pbr_util.bsl.hh, and from
// source/blender/draw/engines/eevee/shaders/eevee_nodetree_lib.bsl.hh
// (bsdf_lut, brdf_f82_tint_lut, F_brdf_multi_scatter), at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22. The GLSL is rewritten as WGSL
// and as Rust; the layer order, the weights each layer leaves for the ones
// below, the F82-tint metal, the IOR modulation by Specular IOR Level and
// the sheen albedo fit are Blender's. The reflection table is Operandi's
// own integration of the same quantities (EEVEE ships a precomputed one).
// See docs/DEPENDENCIES.md and docs/licenses/blender-gpu-shaders/.

//! The realtime approximation of Cycles' principled surface that the
//! viewport's Advanced style draws (viewport materials, VM-1). Cycles
//! renders the shader tree through `shader_eval::Surface::principled`; this
//! module takes the same principled inputs (folded to one value each, image
//! layers as their average) and lights them the way Blender's EEVEE does:
//! the layers are stacked sheen, coat, emission, metal, gloss, diffuse, and
//! each one dims what is under it by the light it took.
//!
//! The lights are the viewport's, not the scene's: one key light (the view
//! light, or the scene's with Show Scene Lighting) and a flat ambient that
//! both lights the diffuse part and stands in for the environment the
//! reflections see. Everything is in linear light.
//!
//! One formula, written twice: [`WGSL`] for the graphics card and
//! [`shade`] on the CPU (tests and the projected preview path). Both read
//! the same reflection table ([`lut`]), so they agree to float precision.
use crate::shader_eval::{Expr, Input, Principled};
use std::sync::OnceLock;

/// The reflection table's side: rows run over √(1 − N·V), columns over
/// roughness, both 0..1 inclusive.
pub const LUT_N: usize = 16;
/// Rows of the table at the start of the viewport's parameter buffer.
pub const LUT_ROWS: usize = LUT_N * LUT_N;
/// Rows one surface takes in the parameter buffer ([`Params::rows`]).
pub const SURFACE_ROWS: usize = 6;
/// Samples per table cell.
const LUT_SAMPLES: u32 = 1024;
/// Roughness the key light's highlight is drawn with at least: a mirror
/// would show the light as a point a pixel wide, or not at all; this
/// stands in for the light having some size
/// (provisional, VM-Q02).
pub const LIGHT_MIN_ROUGHNESS: f32 = 0.1;
/// Cycles' defaults for inputs Operandi doesn't set: Coat IOR, Sheen
/// Roughness.
const COAT_IOR: f32 = 1.5;
const SHEEN_ROUGHNESS: f32 = 0.5;

/// One surface's principled values for the viewport, linear light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Specular Tint: tints the dielectric reflection, and is the metal's
    /// colour near grazing (F82 tint).
    pub tint: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    /// Specular IOR Level: 0.5 reflects as `ior` does.
    pub level: f32,
    pub ior: f32,
    pub coat: f32,
    pub coat_roughness: f32,
    pub sheen: f32,
    /// What the polygon's drawn colour is multiplied by to give the base
    /// colour (the other styles draw Diffuse Color × Diffuse Amount; the
    /// renderer's base colour is Diffuse Color alone).
    pub base_scale: f32,
    /// Emission Color × Strength.
    pub emission: [f32; 3],
    /// Coat Tint: what passes through the coat is multiplied by it, by
    /// Coat Weight (Blender's principled).
    pub coat_tint: [f32; 3],
    /// How much of the bump and normal map the coat follows (Clearcoat
    /// Bump, 0..1); the fill shader turns it into the coat's normal.
    pub coat_bump: f32,
    /// Subsurface Weight: the diffuse light wraps past the terminator,
    /// per channel by `subsurface_radius` (VM-Q08, provisional).
    pub subsurface: f32,
    /// Subsurface Radius, largest channel 1.
    pub subsurface_radius: [f32; 3],
}

/// How far (in N·L) light wraps past the terminator on a surface with
/// Subsurface Weight 1, for its widest-scattering channel (VM-Q08).
pub const SUBSURFACE_WRAP: f32 = 0.5;

impl Default for Params {
    /// The default material's values (shader_tree `MaterialChannels`
    /// default, through `Surface::principled`), for a polygon drawn in
    /// the plain fill.
    fn default() -> Self {
        Self {
            tint: [1.; 3],
            metallic: 0.,
            roughness: 0.4,
            level: 0.5,
            ior: crate::shader_eval::SPECULAR_IOR as f32,
            coat: 0.,
            coat_roughness: 0.,
            sheen: 0.,
            base_scale: 1.,
            emission: [0.; 3],
            coat_tint: [1.; 3],
            coat_bump: 1.,
            subsurface: 0.,
            subsurface_radius: [1.; 3],
        }
    }
}

impl Params {
    /// `p`'s inputs folded to one value each by `value` (images as their
    /// average). `base_scale` as for [`Params::base_scale`].
    pub fn of(p: &Principled, base_scale: f64, value: &dyn Fn(&Expr) -> [f64; 3]) -> Params {
        let get = |input: Input| {
            p.inputs
                .iter()
                .find(|(i, _)| *i == input)
                .map(|(_, e)| value(e))
        };
        let one = |input: Input, default: f32| get(input).map_or(default, |v| v[0] as f32);
        let three =
            |input: Input, default: [f32; 3]| get(input).map_or(default, |v| v.map(|c| c as f32));
        let d = Params::default();
        let emission = three(Input::EmissionColor, [0.; 3]);
        let strength = one(Input::EmissionStrength, 0.);
        // Cycles' default Subsurface Radius (1, 0.2, 0.1).
        let radius = three(Input::SubsurfaceRadius, [1., 0.2, 0.1]).map(|c| c.max(0.));
        let widest = radius[0].max(radius[1]).max(radius[2]);
        Params {
            tint: three(Input::SpecularTint, d.tint),
            metallic: one(Input::Metallic, d.metallic),
            roughness: one(Input::Roughness, d.roughness),
            level: one(Input::SpecularIorLevel, d.level),
            ior: one(Input::Ior, d.ior),
            coat: one(Input::CoatWeight, d.coat),
            coat_roughness: one(Input::CoatRoughness, d.coat_roughness),
            sheen: one(Input::SheenWeight, d.sheen),
            base_scale: base_scale as f32,
            emission: emission.map(|c| c * strength),
            coat_tint: three(Input::CoatTint, d.coat_tint),
            coat_bump: p.coat_bump.clamp(0., 1.) as f32,
            subsurface: one(Input::SubsurfaceWeight, 0.).clamp(0., 1.),
            subsurface_radius: if widest > 0. {
                radius.map(|c| c / widest)
            } else {
                [0.; 3]
            },
        }
    }
    /// The parameter rows the shader's `op_surface` reads.
    pub fn rows(&self) -> [[f32; 4]; SURFACE_ROWS] {
        let t = self.tint;
        let e = self.emission;
        let c = self.coat_tint;
        let r = self.subsurface_radius;
        [
            [t[0], t[1], t[2], self.metallic],
            [self.roughness, self.level, self.ior, self.coat],
            [
                self.coat_roughness,
                self.sheen,
                self.base_scale,
                self.subsurface,
            ],
            [e[0], e[1], e[2], self.coat_bump],
            [c[0], c[1], c[2], 0.],
            [r[0], r[1], r[2], 0.],
        ]
    }
}

/// The reflection table, row-major (row = √(1 − N·V), column = roughness):
/// for a GGX surface lit evenly from every direction, the reflected
/// fraction is `F0 × x + F90 × y` with Schlick's Fresnel, and `z` is the
/// part the F82-tint metal model takes back per unit of its `b` term
/// (Blender's `GGXBrdfData` scale, bias and metal_bias).
pub fn lut() -> &'static [[f32; 4]] {
    static LUT: OnceLock<Vec<[f32; 4]>> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut out = Vec::with_capacity(LUT_ROWS);
        let last = (LUT_N - 1) as f64;
        for row in 0..LUT_N {
            let v = row as f64 / last;
            let nv = (1. - v * v).max(1e-4);
            for col in 0..LUT_N {
                let r = col as f64 / last;
                out.push(integrate(nv, r).map(|c| c as f32));
            }
        }
        out
    })
}

/// The split-sum integrals of one table cell, by importance sampling the
/// GGX normal distribution (Hammersley points) with the height-correlated
/// Smith shadowing.
fn integrate(nv: f64, roughness: f64) -> [f64; 4] {
    let a = (roughness * roughness).max(1e-6);
    let a2 = a * a;
    let v = [(1. - nv * nv).max(0.).sqrt(), 0., nv];
    let lambda = |c: f64| {
        let c2 = (c * c).max(1e-12);
        ((1. + a2 * (1. - c2) / c2).sqrt() - 1.) * 0.5
    };
    let (mut scale, mut bias, mut metal) = (0., 0., 0.);
    for i in 0..LUT_SAMPLES {
        let u = f64::from(i) / f64::from(LUT_SAMPLES);
        let w = f64::from(i.reverse_bits()) / 4294967296.;
        let phi = std::f64::consts::TAU * u;
        let cos_t = ((1. - w) / (1. + (a2 - 1.) * w)).sqrt();
        let sin_t = (1. - cos_t * cos_t).max(0.).sqrt();
        let h = [sin_t * phi.cos(), sin_t * phi.sin(), cos_t];
        let vh = v[0] * h[0] + v[2] * h[2];
        let l_z = 2. * vh * h[2] - v[2];
        if l_z <= 0. || vh <= 0. {
            continue;
        }
        let g = 1. / (1. + lambda(nv) + lambda(l_z));
        let weight = g * vh / (h[2] * nv);
        let fc = (1. - vh).powi(5);
        scale += (1. - fc) * weight;
        bias += fc * weight;
        metal += vh * (1. - vh).powi(6) * weight;
    }
    let n = f64::from(LUT_SAMPLES);
    [scale / n, bias / n, metal / n, 0.]
}

/// The table read at (roughness, N·V), bilinear: what the shader's
/// `op_lut` does.
pub fn lut_at(table: &[[f32; 4]], roughness: f32, nv: f32) -> [f32; 4] {
    let n = (LUT_N - 1) as f32;
    let x = roughness.clamp(0., 1.) * n;
    let y = (1. - nv).clamp(0., 1.).sqrt() * n;
    let x0 = (x as usize).min(LUT_N - 2);
    let y0 = (y as usize).min(LUT_N - 2);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |r: usize, c: usize| table[r * LUT_N + c];
    let mut out = [0.; 4];
    for (k, o) in out.iter_mut().enumerate() {
        let top = at(y0, x0)[k] * (1. - fx) + at(y0, x0 + 1)[k] * fx;
        let bottom = at(y0 + 1, x0)[k] * (1. - fx) + at(y0 + 1, x0 + 1)[k] * fx;
        *o = top * (1. - fy) + bottom * fy;
    }
    out
}

/// sRGB decoding of a gamma-encoded value (egui's encoding, extended past
/// 1 by the power curve).
pub fn decode(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
pub fn encode(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}

/// The viewport's lights for this model, linear: `ambient` per channel and
/// the key light's strength `key` (a Lambert surface facing it gets
/// `colour × key`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lights {
    pub ambient: [f32; 3],
    pub key: f32,
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn f0_amplitude(ior: f32) -> f32 {
    (ior - 1.) / (ior + 1.)
}
fn f0_from_ior(ior: f32) -> f32 {
    let f = f0_amplitude(ior);
    f * f
}
fn ior_from_f0(f0: f32) -> f32 {
    let f = f0.clamp(0., 0.99).sqrt();
    (-f - 1.) / (f - 1.)
}
/// Specular IOR Level as an index: Blender's `openpbr_modulate_ior`.
fn modulate_ior(weight: f32, ior: f32) -> f32 {
    if weight == 1. {
        return ior;
    }
    let m = ior_from_f0(weight * f0_from_ior(ior));
    if ior < 1. { 1. / m } else { m }
}
/// Blender's fit to the sheen layer's albedo (`openpbr_fuzz`).
fn fuzz(nv: f32, rough: f32) -> f32 {
    let den = 35.6694 * rough * rough - 24.4269 * rough * nv - 0.1405 * nv * nv
        + 6.1211 * rough
        + 0.28105 * nv
        - 0.1405;
    let num = 58.5299 * rough * rough - 85.0941 * rough * nv
        + 9.8955 * nv * nv
        + 1.925 * rough
        + 74.2268 * nv
        - 0.2246;
    (den / num).clamp(0., 1.)
}
/// The GGX lobe toward the key light, scaled to reflect all it takes
/// (`ess`: its albedo from the table), times π so it matches the key
/// light's Lambert strength; N·L is in it.
fn lobe(n: [f32; 3], v: [f32; 3], l: [f32; 3], roughness: f32, ess: f32) -> f32 {
    let nl = dot(n, l);
    let nv = dot(n, v).max(1e-4);
    if nl <= 0. || ess <= 0. {
        return 0.;
    }
    let h = [v[0] + l[0], v[1] + l[1], v[2] + l[2]];
    let len = dot(h, h).sqrt();
    if len < 1e-6 {
        return 0.;
    }
    let nh = (dot(n, h) / len).max(0.);
    let r = roughness.max(LIGHT_MIN_ROUGHNESS);
    let a2 = (r * r) * (r * r);
    let d = a2 / (std::f32::consts::PI * (nh * nh * (a2 - 1.) + 1.).powi(2));
    let lambda = |c: f32| {
        let c2 = (c * c).max(1e-8);
        ((1. + a2 * (1. - c2) / c2).sqrt() - 1.) * 0.5
    };
    let g = 1. / (1. + lambda(nv) + lambda(nl));
    d * g / (4. * nv) * std::f32::consts::PI / ess
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn add_scaled(acc: &mut [f32; 3], v: [f32; 3], k: f32) {
    (0..3).for_each(|c| acc[c] += v[c] * k);
}
fn normalized_or(v: [f32; 3], or: [f32; 3]) -> [f32; 3] {
    let l = dot(v, v).sqrt();
    if l > 0. { v.map(|c| c / l) } else { or }
}

/// Blender's node_bump (strength 1, distance 1): the unit normal `n`
/// tilted by height `h` at the point, `hx` one pixel right and `hy` one
/// pixel down, `dpdx` / `dpdy` the position's change across them. The
/// shader's `op_bump`.
pub fn bump(n: [f32; 3], dpdx: [f32; 3], dpdy: [f32; 3], h: f32, hx: f32, hy: f32) -> [f32; 3] {
    let rx = cross(dpdy, n);
    let ry = cross(n, dpdx);
    let det = dot(dpdx, rx);
    let mut out = n.map(|c| c * det.abs());
    add_scaled(&mut out, rx, -det.signum() * (hx - h));
    add_scaled(&mut out, ry, -det.signum() * (hy - h));
    normalized_or(out, n)
}

/// Blender's node_normal_map (strength 1) with the tangent frame from
/// the position and UV changes across the pixel (`T` along +U, `B` along
/// +V): the shader's `op_normal_map`. `colour` is the map's 0..1 colour.
pub fn normal_map(
    n: [f32; 3],
    dpdx: [f32; 3],
    dpdy: [f32; 3],
    duvx: [f32; 2],
    duvy: [f32; 2],
    colour: [f32; 3],
) -> [f32; 3] {
    let p2 = cross(dpdy, n);
    let p1 = cross(n, dpdx);
    // The screen's x, y and the normal may be a left-handed frame (y runs
    // down): the determinant's sign keeps T along +U either way.
    let k = if dot(dpdx, p2) < 0. { -1. } else { 1. };
    let mut t0 = [0f32; 3];
    add_scaled(&mut t0, p2, k * duvx[0]);
    add_scaled(&mut t0, p1, k * duvy[0]);
    let mut b0 = [0f32; 3];
    add_scaled(&mut b0, p2, k * duvx[1]);
    add_scaled(&mut b0, p1, k * duvy[1]);
    if dot(t0, t0).max(dot(b0, b0)) < 1e-24 {
        return n;
    }
    let mut t1 = t0;
    add_scaled(&mut t1, n, -dot(n, t0));
    if dot(t1, t1) < 1e-24 {
        return n;
    }
    let t = normalized_or(t1, n);
    let w = if dot(cross(n, t), b0) >= 0. { 1. } else { -1. };
    let b = cross(n, t).map(|c| c * w);
    let c = colour.map(|v| 2. * v - 1.);
    let mut out = [0f32; 3];
    add_scaled(&mut out, t, c[0]);
    add_scaled(&mut out, b, c[1]);
    add_scaled(&mut out, n, c[2]);
    normalized_or(out, n)
}

/// The colour (linear) of a surface with base colour `base` (linear) and
/// values `s`, with unit normal `n`, unit direction to the eye `v` and to
/// the key light `l`; `lit` scales the key light (1 unshadowed).
pub fn shade(
    base: [f32; 3],
    s: &Params,
    n: [f32; 3],
    v: [f32; 3],
    l: [f32; 3],
    lit: f32,
    lights: &Lights,
) -> [f32; 3] {
    let table = lut();
    let nv = dot(n, v).clamp(1e-4, 1.);
    let spec = lut_at(table, s.roughness, nv);
    let ess = spec[0] + spec[1];
    let mut w = [1f32; 3];
    let mut diffuse = [0f32; 3];
    let mut refl = [0f32; 3];
    let mut coat = [0f32; 3];
    let mut coat_ess = 1.;
    let scale = |w: &mut [f32; 3], k: [f32; 3]| (0..3).for_each(|c| w[c] *= k[c]);
    // Sheen (white tint), taken as diffuse as EEVEE does.
    if s.sheen > 0. {
        let fz = s.sheen.max(0.) * fuzz(nv, SHEEN_ROUGHNESS);
        (0..3).for_each(|c| diffuse[c] += w[c] * fz);
        scale(&mut w, [(1. - fz).max(0.); 3]);
    }
    // Coat (white tint, IOR 1.5).
    let coat_w = s.coat.clamp(0., 1.);
    if coat_w > 0. {
        let t = lut_at(table, s.coat_roughness.clamp(0., 1.), nv);
        coat_ess = t[0] + t[1];
        let f90 = (2.33 / 0.33 * f0_amplitude(COAT_IOR)).clamp(0., 1.);
        let r = f0_from_ior(COAT_IOR) * t[0] + f90 * t[1];
        (0..3).for_each(|c| coat[c] = w[c] * coat_w * r);
        scale(&mut w, [(1. - r * coat_w).clamp(0., 1.); 3]);
        // Coat Tint colours everything under the coat.
        let tint = s.coat_tint.map(|t| 1. + (t.max(0.) - 1.) * coat_w);
        scale(&mut w, tint);
    }
    let emission: [f32; 3] = std::array::from_fn(|c| w[c] * s.emission[c].max(0.));
    // Metal: F82-tint Fresnel, multiple-scattering compensated.
    let metallic = s.metallic.clamp(0., 1.);
    let tint = s.tint.map(|c| c.max(0.));
    if metallic > 0. {
        const F: f32 = 6. / 7.;
        let f5 = F.powi(5);
        let f6 = F.powi(6);
        for c in 0..3 {
            let f0 = base[c].clamp(0., 1.);
            let fss = f0 * spec[0] + spec[1];
            let favg = f0 + (1. - f0) / 21.;
            let mut r = fss / (1. - (1. - ess) * favg);
            let schlick = f0 + (1. - f0) * f5;
            let b = schlick * (7. / f6) * (1. - tint[c].min(1.));
            r -= b * spec[2];
            refl[c] += w[c] * metallic * r.max(0.);
        }
        scale(&mut w, [1. - metallic; 3]);
    }
    // Gloss: the dielectric reflection, its index set by Specular IOR Level.
    let ior = modulate_ior((s.level * 2.).max(0.), s.ior.max(1e-5));
    if ior != 1. {
        let f90 = (2.33 / 0.33 * f0_amplitude(ior)).clamp(0., 1.);
        let mut r = [0f32; 3];
        for c in 0..3 {
            let f0 = (f0_from_ior(ior) * tint[c]).clamp(0., 1.);
            let favg = f0 + (f90 - f0) / 21.;
            r[c] = (f0 * spec[0] + f90 * spec[1]) / (1. - (1. - ess) * favg);
            refl[c] += w[c] * r[c];
        }
        let most = r[0].max(r[1]).max(r[2]);
        scale(&mut w, [(1. - most).max(0.); 3]);
    }
    for c in 0..3 {
        diffuse[c] += w[c] * base[c].max(0.);
    }
    let nl_raw = dot(n, l);
    let nl = nl_raw.max(0.);
    // Subsurface: the diffuse light wraps past the terminator (VM-Q08).
    let wrap: [f32; 3] = std::array::from_fn(|c| {
        let k = SUBSURFACE_WRAP * s.subsurface * s.subsurface_radius[c];
        if k > 0. {
            ((nl_raw + k) / (1. + k)).max(0.)
        } else {
            nl
        }
    });
    let key = lights.key * lit;
    let spec_lobe = lobe(n, v, l, s.roughness, ess) * key;
    let coat_lobe = if coat_w > 0. {
        lobe(n, v, l, s.coat_roughness, coat_ess) * key
    } else {
        0.
    };
    std::array::from_fn(|c| {
        let amb = lights.ambient[c];
        diffuse[c] * (amb + key * wrap[c])
            + refl[c] * (amb + spec_lobe)
            + coat[c] * (amb + coat_lobe)
            + emission[c]
    })
}

/// Where each per-pixel value sits in a list function's result
/// (`shader_wgsl::translate_list`, read by the shader's
/// `op_listed_surface` and the fill shader): the textured principled
/// inputs, the bump height at the point and one pixel right and down, the
/// tangent-space normal colour, the cut-out and which of the last three
/// apply.
pub mod list {
    pub const ROUGHNESS: usize = 0;
    pub const METALLIC: usize = 1;
    pub const LEVEL: usize = 2;
    pub const TINT: usize = 3;
    pub const COAT: usize = 4;
    pub const COAT_ROUGHNESS: usize = 5;
    pub const SHEEN: usize = 6;
    pub const EMISSION: usize = 7;
    pub const HEIGHT: usize = 8;
    pub const HEIGHT_RIGHT: usize = 9;
    pub const HEIGHT_DOWN: usize = 10;
    pub const NORMAL: usize = 11;
    pub const CUT: usize = 12;
    /// (bump, normal map, cut-out): 1 where it applies.
    pub const FLAGS: usize = 13;
    /// How far the surface moves along its normal, scene units (VM-Q11):
    /// the fill vertex shader reads it in the Advanced style.
    pub const DISPLACEMENT: usize = 14;
    /// (displaced, -, -): 1 when the surface has a displacement.
    pub const DISPLACED: usize = 15;
    pub const LEN: usize = 16;
}

/// The values the viewport evaluates per pixel for a surface whose inputs
/// read images: `p`'s principled inputs, its bump height and normal map,
/// and `cut` (1 where the surface is cut away), as (value, shading point)
/// for `shader_wgsl::translate_list`, in [`list`] order.
pub fn per_pixel(p: &Principled, cut: Option<&Expr>) -> Vec<(Expr, u8)> {
    let d = Params::default();
    let get = |input: Input, default: [f64; 3]| {
        p.inputs
            .iter()
            .find(|(i, _)| *i == input)
            .map_or(Expr::Const(default), |(_, e)| e.clone())
    };
    let scalar = |v: f32| [f64::from(v); 3];
    let tint = d.tint.map(f64::from);
    let emission = Expr::times(
        get(Input::EmissionColor, [0.; 3]),
        get(Input::EmissionStrength, [0.; 3]),
    );
    let height = p.bump.clone().unwrap_or(Expr::scalar(0.));
    let flags = [
        f64::from(u8::from(p.bump.is_some())),
        f64::from(u8::from(p.normal_map.is_some())),
        f64::from(u8::from(cut.is_some())),
    ];
    vec![
        (get(Input::Roughness, scalar(d.roughness)), 0),
        (get(Input::Metallic, scalar(d.metallic)), 0),
        (get(Input::SpecularIorLevel, scalar(d.level)), 0),
        (get(Input::SpecularTint, tint), 0),
        (get(Input::CoatWeight, scalar(d.coat)), 0),
        (get(Input::CoatRoughness, scalar(d.coat_roughness)), 0),
        (get(Input::SheenWeight, scalar(d.sheen)), 0),
        (emission, 0),
        (height.clone(), 0),
        (height.clone(), 1),
        (height, 2),
        (
            p.normal_map
                .clone()
                .unwrap_or(Expr::Const(crate::shader_eval::FLAT_NORMAL)),
            0,
        ),
        (cut.cloned().unwrap_or(Expr::scalar(0.)), 0),
        (Expr::Const(flags), 0),
        (p.displacement.clone().unwrap_or(Expr::scalar(0.)), 0),
        (
            Expr::Const([f64::from(u8::from(p.displacement.is_some())), 0., 0.]),
            0,
        ),
    ]
}

/// [`WGSL`] with its numbers spliced in.
pub fn wgsl() -> String {
    let d = Params::default();
    let lit = |v: f32| format!("{v:?}");
    let surface = format!(
        "OpSurface(vec3<f32>({}, {}, {}), {}, {}, {}, {}, {}, {}, {}, {}, vec3<f32>({}, {}, {}), 0.0, 0.0, vec3<f32>(1.0), vec3<f32>(1.0), vec3<f32>(0.0))",
        lit(d.tint[0]),
        lit(d.tint[1]),
        lit(d.tint[2]),
        lit(d.metallic),
        lit(d.roughness),
        lit(d.level),
        lit(d.ior),
        lit(d.coat),
        lit(d.coat_roughness),
        lit(d.sheen),
        lit(d.base_scale),
        lit(d.emission[0]),
        lit(d.emission[1]),
        lit(d.emission[2]),
    );
    WGSL.replace("{lut_n}", &format!("{LUT_N}u"))
        .replace("{list_len}", &crate::shader_wgsl::LIST_LEN.to_string())
        .replace("{min_rough}", &lit(LIGHT_MIN_ROUGHNESS))
        .replace("{coat_f0}", &lit(f0_from_ior(COAT_IOR)))
        .replace(
            "{coat_f90}",
            &lit((2.33 / 0.33 * f0_amplitude(COAT_IOR)).clamp(0., 1.)),
        )
        .replace("{sheen_rough}", &lit(SHEEN_ROUGHNESS))
        .replace("{sss_wrap}", &lit(SUBSURFACE_WRAP))
        .replace("{default_surface}", &surface)
}

/// The shader side. Reads `op_params` (the table in rows 0..LUT_ROWS, then
/// each surface's [`SURFACE_ROWS`] rows), which the world shader declares.
const WGSL: &str = r#"
struct OpSurface {
  tint: vec3<f32>, metallic: f32, roughness: f32, level: f32, ior: f32,
  coat: f32, coat_roughness: f32, sheen: f32, base_scale: f32, emission: vec3<f32>,
  subsurface: f32, coat_bump: f32, coat_tint: vec3<f32>, radius: vec3<f32>,
  // The coat's normal when it differs from the surface's (Clearcoat Bump,
  // set by the fill shader), else zero.
  coat_n: vec3<f32>,
};
fn op_default_surface() -> OpSurface { return {default_surface}; }
fn op_surface(row: u32) -> OpSurface {
  let a = op_params[row]; let b = op_params[row + 1u];
  let c = op_params[row + 2u]; let d = op_params[row + 3u];
  let e = op_params[row + 4u]; let f = op_params[row + 5u];
  return OpSurface(a.xyz, a.w, b.x, b.y, b.z, b.w, c.x, c.y, c.z, d.xyz, c.w, d.w, e.xyz, f.xyz, vec3<f32>(0.0));
}
fn op_lut(roughness: f32, nv: f32) -> vec4<f32> {
  let n = f32({lut_n} - 1u);
  let x = clamp(roughness, 0.0, 1.0) * n;
  let y = sqrt(clamp(1.0 - nv, 0.0, 1.0)) * n;
  let x0 = min(u32(x), {lut_n} - 2u);
  let y0 = min(u32(y), {lut_n} - 2u);
  let fx = x - f32(x0); let fy = y - f32(y0);
  let r0 = y0 * {lut_n} + x0; let r1 = r0 + {lut_n};
  let top = op_params[r0] * (1.0 - fx) + op_params[r0 + 1u] * fx;
  let bottom = op_params[r1] * (1.0 - fx) + op_params[r1 + 1u] * fx;
  return top * (1.0 - fy) + bottom * fy;
}
fn op_f0_amp(ior: f32) -> f32 { return (ior - 1.0) / (ior + 1.0); }
fn op_f0_from_ior(ior: f32) -> f32 { let f = op_f0_amp(ior); return f * f; }
fn op_ior_from_f0(f0: f32) -> f32 { let f = sqrt(clamp(f0, 0.0, 0.99)); return (-f - 1.0) / (f - 1.0); }
fn op_modulate_ior(w: f32, ior: f32) -> f32 {
  if (w == 1.0) { return ior; }
  let m = op_ior_from_f0(w * op_f0_from_ior(ior));
  return select(m, 1.0 / m, ior < 1.0);
}
fn op_fuzz(nv: f32, r: f32) -> f32 {
  let den = 35.6694 * r * r - 24.4269 * r * nv - 0.1405 * nv * nv + 6.1211 * r + 0.28105 * nv - 0.1405;
  let num = 58.5299 * r * r - 85.0941 * r * nv + 9.8955 * nv * nv + 1.925 * r + 74.2268 * nv - 0.2246;
  return clamp(den / num, 0.0, 1.0);
}
fn op_lobe(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32, ess: f32) -> f32 {
  let nl = dot(n, l);
  let nv = max(dot(n, v), 1e-4);
  if (nl <= 0.0 || ess <= 0.0) { return 0.0; }
  let h = v + l;
  let len = length(h);
  if (len < 1e-6) { return 0.0; }
  let nh = max(dot(n, h) / len, 0.0);
  let r = max(roughness, {min_rough});
  let a2 = (r * r) * (r * r);
  let q = nh * nh * (a2 - 1.0) + 1.0;
  let d = a2 / (3.14159265 * q * q);
  let lv = (sqrt(1.0 + a2 * (1.0 - nv * nv) / max(nv * nv, 1e-8)) - 1.0) * 0.5;
  let ll = (sqrt(1.0 + a2 * (1.0 - nl * nl) / max(nl * nl, 1e-8)) - 1.0) * 0.5;
  let g = 1.0 / (1.0 + lv + ll);
  return d * g / (4.0 * nv) * 3.14159265 / ess;
}
// A surface with the inputs a list function evaluated per pixel
// (blender_principled.rs `list`).
fn op_listed_surface(s: OpSurface, l: array<vec3<f32>, {list_len}>) -> OpSurface {
  var o = s;
  o.roughness = l[0].x; o.metallic = l[1].x; o.level = l[2].x; o.tint = l[3];
  o.coat = l[4].x; o.coat_roughness = l[5].x; o.sheen = l[6].x; o.emission = l[7];
  return o;
}
// Blender's node_bump (strength 1, distance 1, filter width 1): the unit
// normal n tilted by the height h at the point, hx one pixel right and
// hy one pixel down, with dpdx / dpdy the position's change across them.
fn op_bump(n: vec3<f32>, dpdx: vec3<f32>, dpdy: vec3<f32>, h: f32, hx: f32, hy: f32) -> vec3<f32> {
  let rx = cross(dpdy, n);
  let ry = cross(n, dpdx);
  let det = dot(dpdx, rx);
  let surfgrad = (hx - h) * rx + (hy - h) * ry;
  let out = abs(det) * n - sign(det) * surfgrad;
  if (length(out) <= 0.0) { return n; }
  return normalize(out);
}
// Blender's node_normal_map (strength 1) with the tangent frame taken
// from how the position and UVs change across the pixel, not from stored
// tangents (provisional, VM-Q04): T along +U, B along +V.
fn op_normal_map(n: vec3<f32>, dpdx: vec3<f32>, dpdy: vec3<f32>, duvx: vec2<f32>, duvy: vec2<f32>, colour: vec3<f32>) -> vec3<f32> {
  let p2 = cross(dpdy, n);
  let p1 = cross(n, dpdx);
  // The screen's x, y and the normal may be a left-handed frame (y runs
  // down): the determinant's sign keeps T along +U either way.
  let k = select(1.0, -1.0, dot(dpdx, p2) < 0.0);
  let t0 = k * (p2 * duvx.x + p1 * duvy.x);
  let b0 = k * (p2 * duvx.y + p1 * duvy.y);
  if (max(dot(t0, t0), dot(b0, b0)) < 1e-24) { return n; }
  let t1 = t0 - n * dot(n, t0);
  if (dot(t1, t1) < 1e-24) { return n; }
  let t = normalize(t1);
  let w = select(-1.0, 1.0, dot(cross(n, t), b0) >= 0.0);
  let b = w * cross(n, t);
  let c = 2.0 * colour - vec3<f32>(1.0);
  let out = c.x * t + c.y * b + c.z * n;
  if (length(out) <= 0.0) { return n; }
  return normalize(out);
}
// blender_principled.rs `shade`: base and the result in linear light.
fn op_principled(base: vec3<f32>, s: OpSurface, n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, lit: f32, ambient: vec3<f32>, key_strength: f32) -> vec3<f32> {
  return op_principled_lit(base, s, n, v, l, lit, ambient, ambient, key_strength);
}
// op_principled with the ambient light split (HDRI stage 3): `ambient`
// lights the diffuse and sheen layers (an environment picture's
// irradiance), `reflected` the metal, gloss and coat layers (the picture
// seen along the reflection). With both equal it is op_principled.
fn op_principled_lit(base: vec3<f32>, s: OpSurface, n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, lit: f32, ambient: vec3<f32>, reflected: vec3<f32>, key_strength: f32) -> vec3<f32> {
  let nv = clamp(dot(n, v), 1e-4, 1.0);
  let spec = op_lut(s.roughness, nv);
  let ess = spec.x + spec.y;
  var w = vec3<f32>(1.0);
  var diffuse = vec3<f32>(0.0);
  var refl = vec3<f32>(0.0);
  var coat = vec3<f32>(0.0);
  var coat_ess = 1.0;
  if (s.sheen > 0.0) {
    let fz = max(s.sheen, 0.0) * op_fuzz(nv, {sheen_rough});
    diffuse += w * fz;
    w *= max(1.0 - fz, 0.0);
  }
  let coat_w = clamp(s.coat, 0.0, 1.0);
  if (coat_w > 0.0) {
    let t = op_lut(clamp(s.coat_roughness, 0.0, 1.0), nv);
    coat_ess = t.x + t.y;
    let r = {coat_f0} * t.x + {coat_f90} * t.y;
    coat = w * coat_w * r;
    w *= clamp(1.0 - r * coat_w, 0.0, 1.0);
    w *= vec3<f32>(1.0) + (max(s.coat_tint, vec3<f32>(0.0)) - vec3<f32>(1.0)) * coat_w;
  }
  let emission = w * max(s.emission, vec3<f32>(0.0));
  let metallic = clamp(s.metallic, 0.0, 1.0);
  let tint = max(s.tint, vec3<f32>(0.0));
  if (metallic > 0.0) {
    let f = 6.0 / 7.0;
    let f2 = f * f;
    let f5 = f2 * f2 * f;
    let f6 = f2 * f2 * f2;
    let f0 = clamp(base, vec3<f32>(0.0), vec3<f32>(1.0));
    let fss = f0 * spec.x + spec.y;
    let favg = f0 + (1.0 - f0) / 21.0;
    var r = fss / (1.0 - (1.0 - ess) * favg);
    let schlick = f0 + (1.0 - f0) * f5;
    let b = schlick * (7.0 / f6) * (1.0 - min(tint, vec3<f32>(1.0)));
    r -= b * spec.z;
    refl += w * metallic * max(r, vec3<f32>(0.0));
    w *= 1.0 - metallic;
  }
  let ior = op_modulate_ior(max(s.level * 2.0, 0.0), max(s.ior, 1e-5));
  if (ior != 1.0) {
    let f90 = clamp(2.33 / 0.33 * op_f0_amp(ior), 0.0, 1.0);
    let f0 = clamp(vec3<f32>(op_f0_from_ior(ior)) * tint, vec3<f32>(0.0), vec3<f32>(1.0));
    let favg = f0 + (f90 - f0) / 21.0;
    let r = (f0 * spec.x + f90 * spec.y) / (1.0 - (1.0 - ess) * favg);
    refl += w * r;
    w *= max(1.0 - max(r.x, max(r.y, r.z)), 0.0);
  }
  diffuse += w * max(base, vec3<f32>(0.0));
  let nl_raw = dot(n, l);
  let nl = max(nl_raw, 0.0);
  // Subsurface: the diffuse light wraps past the terminator (VM-Q08).
  let k = {sss_wrap} * s.subsurface * s.radius;
  let wrapped = select(vec3<f32>(nl), max((vec3<f32>(nl_raw) + k) / (vec3<f32>(1.0) + k), vec3<f32>(0.0)), k > vec3<f32>(0.0));
  let key = key_strength * lit;
  let spec_lobe = op_lobe(n, v, l, s.roughness, ess) * key;
  var coat_lobe = 0.0;
  var cn = n;
  if (dot(s.coat_n, s.coat_n) > 0.25) { cn = s.coat_n; }
  if (coat_w > 0.0) { coat_lobe = op_lobe(cn, v, l, s.coat_roughness, coat_ess) * key; }
  return diffuse * (ambient + key * wrapped) + refl * (reflected + spec_lobe) + coat * (reflected + coat_lobe) + emission;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32, tol: f32) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    /// A smooth surface seen straight on reflects with Schlick at its F0
    /// (scale ≈ 1, bias ≈ 0), and the albedo falls as roughness grows at
    /// grazing angles (single-scattering GGX loses energy).
    #[test]
    fn table_has_the_known_ends() {
        let t = lut();
        assert_eq!(t.len(), LUT_ROWS);
        let smooth = lut_at(t, 0., 1.);
        near(smooth[0], 1., 0.01);
        near(smooth[1], 0., 0.01);
        for cell in t {
            assert!(cell.iter().all(|v| v.is_finite() && *v >= 0.));
            assert!(cell[0] + cell[1] <= 1.01, "{cell:?}");
        }
        let rough_grazing = lut_at(t, 1., 0.1);
        assert!(rough_grazing[0] + rough_grazing[1] < 0.8);
        // Bilinear: the cell corners are read back exactly.
        assert_eq!(lut_at(t, 1. / 15., 1.), t[1]);
    }

    fn lights() -> Lights {
        Lights {
            ambient: [0.2; 3],
            key: 1.,
        }
    }
    const UP: [f32; 3] = [0., 0., 1.];

    /// Diffuse is dimmed by what the gloss reflects; a rougher surface
    /// spreads its highlight lower and wider.
    #[test]
    fn rough_highlights_are_broad_and_low() {
        let base = [0.5; 3];
        let at = |rough: f32, n: [f32; 3]| {
            let s = Params {
                roughness: rough,
                ..Params::default()
            };
            shade(base, &s, n, UP, UP, 1., &lights())[0]
        };
        let tilt = {
            let a: f32 = 0.25;
            [a.sin(), 0., a.cos()]
        };
        assert!(at(0.2, UP) > at(0.6, UP));
        assert!(at(0.2, tilt) < at(0.6, tilt));
    }

    /// Fully metallic: no diffuse, the base colour tints the reflection;
    /// emission adds as given.
    #[test]
    fn metal_reflects_its_colour_and_emission_adds() {
        let s = Params {
            metallic: 1.,
            ..Params::default()
        };
        let red = shade([0.9, 0.1, 0.1], &s, UP, UP, [1., 0., 0.], 1., &lights());
        // Lit from the side: no highlight, no diffuse; the ambient
        // reflection of a red metal.
        assert!(red[0] > 4. * red[1], "{red:?}");
        near(red[1], red[2], 1e-4);
        let glow = Params {
            emission: [0.3, 0., 0.],
            ..Params::default()
        };
        let plain = shade([0.; 3], &Params::default(), UP, UP, UP, 0., &lights());
        let lit = shade([0.; 3], &glow, UP, UP, UP, 0., &lights());
        near(lit[0] - plain[0], 0.3, 1e-3);
    }

    /// A coat adds a second highlight and dims the base under it.
    #[test]
    fn coat_dims_the_base() {
        let base = [0.5; 3];
        let side = [0.6f32.sin(), 0., 0.6f32.cos()];
        let plain = shade(base, &Params::default(), UP, UP, side, 1., &lights());
        let coated = Params {
            coat: 1.,
            ..Params::default()
        };
        let c = shade(base, &coated, UP, UP, side, 1., &lights());
        assert!(c[0] != plain[0]);
        // Straight on and lit from the eye: the coat's sharp highlight.
        let on = shade(base, &coated, UP, UP, UP, 1., &lights());
        assert!(on[0] > shade(base, &Params::default(), UP, UP, UP, 1., &lights())[0]);
    }

    /// Specular IOR Level 0 turns the gloss off: a black dielectric then
    /// reflects nothing at all.
    #[test]
    fn level_zero_has_no_gloss() {
        let s = Params {
            level: 0.,
            ..Params::default()
        };
        let c = shade([0.; 3], &s, UP, UP, UP, 1., &lights());
        assert!(c.iter().all(|&v| v.abs() < 1e-5), "{c:?}");
    }

    /// A height rising to the right tilts the normal left; a flat normal
    /// map colour keeps the normal; a map leaning +U leans along +U.
    #[test]
    fn bump_and_normal_map_lean_the_right_way() {
        let n = [0., 0., 1.];
        let (dx, dy) = ([0.01, 0., 0.], [0., -0.01, 0.]);
        assert_eq!(bump(n, dx, dy, 0., 0., 0.), n);
        let tilted = bump(n, dx, dy, 0., 0.002, 0.);
        assert!(tilted[0] < -0.1 && tilted[2] > 0.9, "{tilted:?}");
        let (ux, uy) = ([0.01, 0.], [0., 0.01]);
        let flat = normal_map(n, dx, dy, ux, uy, [0.5, 0.5, 1.]);
        assert!((0..3).all(|c| (flat[c] - n[c]).abs() < 1e-6), "{flat:?}");
        let lean = normal_map(n, dx, dy, ux, uy, [0.85, 0.5, 0.85]);
        assert!(lean[0] > 0.3 && lean[1].abs() < 1e-5, "{lean:?}");
        // V runs down the screen here (dy is -Y, V grows): a +V lean
        // leans toward -Y.
        let up = normal_map(n, dx, dy, ux, uy, [0.5, 0.85, 0.85]);
        assert!(up[1] < -0.3, "{up:?}");
        // No UVs: unchanged.
        assert_eq!(
            normal_map(n, dx, dy, [0.; 2], [0.; 2], [0.85, 0.5, 0.85]),
            n
        );
    }

    /// The list carries every per-pixel value in order, the height at all
    /// three points, and flags what applies.
    #[test]
    fn per_pixel_list_order() {
        let mut p = crate::shader_tree::ShaderTree::default()
            .evaluate(&crate::shader_eval::Context {
                item: 0,
                ancestors: &[],
                material: crate::material::DEFAULT_MATERIAL,
                part: None,
                sets: &[],
                locators: &[],
            })
            .principled();
        p.bump = Some(Expr::scalar(0.25));
        let l = per_pixel(&p, None);
        assert_eq!(l.len(), list::LEN);
        assert!(l.len() <= crate::shader_wgsl::LIST_LEN);
        assert_eq!(l[list::ROUGHNESS].0.constant(), Some([0.4; 3]));
        for (k, at) in [
            (list::HEIGHT, 0),
            (list::HEIGHT_RIGHT, 1),
            (list::HEIGHT_DOWN, 2),
        ] {
            assert_eq!(l[k], (Expr::scalar(0.25), at));
        }
        assert_eq!(l[list::FLAGS].0, Expr::Const([1., 0., 0.]));
        let w = wgsl();
        assert!(w.contains(&format!(
            "array<vec3<f32>, {}>",
            crate::shader_wgsl::LIST_LEN
        )));
    }

    #[test]
    fn rows_round_trip_the_wgsl_layout() {
        let p = Params {
            tint: [0.1, 0.2, 0.3],
            metallic: 0.4,
            roughness: 0.5,
            level: 0.6,
            ior: 1.7,
            coat: 0.8,
            coat_roughness: 0.9,
            sheen: 0.25,
            base_scale: 1.25,
            emission: [2., 3., 4.],
            coat_tint: [0.7, 0.6, 0.5],
            coat_bump: 0.35,
            subsurface: 0.45,
            subsurface_radius: [1., 0.5, 0.25],
        };
        let r = p.rows();
        assert_eq!(r[0], [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(r[1], [0.5, 0.6, 1.7, 0.8]);
        assert_eq!(r[2], [0.9, 0.25, 1.25, 0.45]);
        assert_eq!(r[3], [2., 3., 4., 0.35]);
        assert_eq!(r[4], [0.7, 0.6, 0.5, 0.]);
        assert_eq!(r[5], [1., 0.5, 0.25, 0.]);
        let w = wgsl();
        assert!(!w.contains("{lut_n}") && !w.contains("{default_surface}"));
        assert!(!w.contains("{sss_wrap}"));
    }

    /// VM-Q08: subsurface wraps the key light past the terminator, most
    /// for the channel that scatters furthest; Coat Tint colours what is
    /// under the coat.
    #[test]
    fn subsurface_wraps_and_coat_tint_colours() {
        let lights = Lights {
            key: 1.,
            ambient: [0.; 3],
        };
        let n = [0., 0., 1.];
        let v = [0., 0., 1.];
        // Light just past the terminator.
        let l = [0.995f32.sqrt(), 0., -0.0707];
        let plain = Params::default();
        let dark = shade([0.8; 3], &plain, n, v, l, 1., &lights);
        assert!(dark.iter().all(|c| *c < 1e-6), "{dark:?}");
        let skin = Params {
            subsurface: 1.,
            subsurface_radius: [1., 0.2, 0.1],
            ..plain
        };
        let lit = shade([0.8; 3], &skin, n, v, l, 1., &lights);
        assert!(
            lit[0] > 0.1 && lit[0] > lit[1] && lit[1] > lit[2],
            "{lit:?}"
        );
        // Coat Tint red over a white base: red comes back (the light off
        // the mirror direction, so the coat's own highlight stays small).
        let l = [0.6, 0., 0.8];
        let coated = Params {
            coat: 1.,
            coat_tint: [1., 0., 0.],
            ..plain
        };
        let c = shade([1.; 3], &coated, n, v, l, 1., &lights);
        assert!(c[0] > 3. * c[1], "{c:?}");
    }

    /// The default parameters are the default material's.
    #[test]
    fn default_is_the_default_material() {
        let ctx = crate::shader_eval::Context {
            item: 0,
            ancestors: &[],
            material: crate::material::DEFAULT_MATERIAL,
            part: None,
            sets: &[],
            locators: &[],
        };
        let p = crate::shader_tree::ShaderTree::default()
            .evaluate(&ctx)
            .principled();
        let folded = Params::of(&p, 1., &|e| e.eval(&|_, _| [0.; 4]));
        assert_eq!(folded, Params::default());
    }
}
