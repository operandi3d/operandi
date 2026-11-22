// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-FileCopyrightText: 2026 Operandi contributors
//
// SPDX-License-Identifier: Apache-2.0
//
// The node sums below are adapted from Cycles
// (https://projects.blender.org/blender/cycles) at commit
// 3eb35f85996baa6c1b2e19bf297b22745c85e321: src/kernel/svm/math_util.h
// (svm_math, svm_vector_math), color_util.h (the Hue, Saturation, Value,
// Color, Linear Light and Exclusion blends), hsv.h, map_range.h,
// mapping_util.h, fresnel.h, src/kernel/closure/bsdf_util.h
// (fresnel_dielectric_cos), src/util/color.h (rgb_to_hsl, hsl_to_rgb,
// color_linear_to_srgb), src/util/math_base.h and math_float3.h (the safe_*
// helpers) and src/util/transform.h (euler_to_transform), Apache-2.0. The
// C++ is rewritten as Rust and, in `shader_func.wgsl`, as WGSL. See
// docs/DEPENDENCIES.md and docs/licenses/cycles/.

//! Cycles node sums the shader tree's own expression has no word for
//! (MaterialX stage 5): a graph layer's Math, Vector Math, Map Range,
//! Mapping, Hue/Saturation/Value, Fresnel and procedural textures, as one
//! more kind of [`Expr`](crate::shader_eval::Expr) node, `Expr::Func`.
//! `graph_view` builds them from a node graph; [`eval`] is the CPU sum and
//! [`wgsl`] the GPU call (helpers in `shader_func.wgsl`), so the viewport
//! draws a graph per pixel the way Cycles renders it.
//!
//! Every argument is a value of three components as `Expr` carries them;
//! a scalar argument reads its first.
use crate::cycles_noise::{self as noise, GradientKind, NoiseKind, WaveKind};
use crate::cycles_voronoi::{Voronoi, VoronoiFeature, VoronoiMetric, VoronoiOut};

/// Cycles' Math node operations (`NodeMathType`), by their Cycles names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    MultiplyAdd,
    Power,
    Logarithm,
    Sqrt,
    InverseSqrt,
    Absolute,
    Exponent,
    Minimum,
    Maximum,
    LessThan,
    GreaterThan,
    Sign,
    Compare,
    SmoothMin,
    SmoothMax,
    Round,
    Floor,
    Ceil,
    Trunc,
    Fraction,
    Modulo,
    FlooredModulo,
    Wrap,
    Snap,
    PingPong,
    Sine,
    Cosine,
    Tangent,
    Arcsine,
    Arccosine,
    Arctangent,
    Arctan2,
    Sinh,
    Cosh,
    Tanh,
    Radians,
    Degrees,
}
const MATH_NAMES: [(&str, MathOp); 41] = [
    ("add", MathOp::Add),
    ("subtract", MathOp::Subtract),
    ("multiply", MathOp::Multiply),
    ("divide", MathOp::Divide),
    ("multiply_add", MathOp::MultiplyAdd),
    ("power", MathOp::Power),
    ("logarithm", MathOp::Logarithm),
    ("sqrt", MathOp::Sqrt),
    ("inversesqrt", MathOp::InverseSqrt),
    ("absolute", MathOp::Absolute),
    ("exponent", MathOp::Exponent),
    ("minimum", MathOp::Minimum),
    ("maximum", MathOp::Maximum),
    ("less_than", MathOp::LessThan),
    ("greater_than", MathOp::GreaterThan),
    ("sign", MathOp::Sign),
    ("compare", MathOp::Compare),
    ("smoothmin", MathOp::SmoothMin),
    ("smoothmax", MathOp::SmoothMax),
    ("round", MathOp::Round),
    ("floor", MathOp::Floor),
    ("ceil", MathOp::Ceil),
    ("trunc", MathOp::Trunc),
    ("fraction", MathOp::Fraction),
    ("modulo", MathOp::Modulo),
    ("floored_modulo", MathOp::FlooredModulo),
    ("wrap", MathOp::Wrap),
    ("snap", MathOp::Snap),
    ("pingpong", MathOp::PingPong),
    ("sine", MathOp::Sine),
    ("cosine", MathOp::Cosine),
    ("tangent", MathOp::Tangent),
    ("arcsine", MathOp::Arcsine),
    ("arccosine", MathOp::Arccosine),
    ("arctangent", MathOp::Arctangent),
    ("arctan2", MathOp::Arctan2),
    ("sinh", MathOp::Sinh),
    ("cosh", MathOp::Cosh),
    ("tanh", MathOp::Tanh),
    ("radians", MathOp::Radians),
    ("degrees", MathOp::Degrees),
];
impl MathOp {
    pub fn from_name(name: &str) -> Option<Self> {
        MATH_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, op)| *op)
    }
    /// The number `op_g_math` switches on.
    pub fn code(self) -> u32 {
        self as u32
    }
    /// One component (`svm_math`).
    pub fn apply(self, a: f64, b: f64, c: f64) -> f64 {
        use MathOp::*;
        let safe_div = |a: f64, b: f64| if b != 0. { a / b } else { 0. };
        match self {
            Add => a + b,
            Subtract => a - b,
            Multiply => a * b,
            Divide => safe_div(a, b),
            MultiplyAdd => a * b + c,
            Power => safe_pow(a, b),
            Logarithm => {
                if a <= 0. || b <= 0. {
                    0.
                } else {
                    safe_div(a.ln(), b.ln())
                }
            }
            Sqrt => a.max(0.).sqrt(),
            InverseSqrt => {
                if a > 0. {
                    1. / a.sqrt()
                } else {
                    0.
                }
            }
            Absolute => a.abs(),
            Exponent => a.exp(),
            Minimum => a.min(b),
            Maximum => a.max(b),
            LessThan => f64::from(u8::from(a < b)),
            GreaterThan => f64::from(u8::from(a > b)),
            Sign => sign(a),
            Compare => {
                if a == b || (a - b).abs() <= c.max(f64::from(f32::EPSILON)) {
                    1.
                } else {
                    0.
                }
            }
            SmoothMin => smooth_min(a, b, c),
            SmoothMax => -smooth_min(-a, -b, c),
            Round => (a + 0.5).floor(),
            Floor => a.floor(),
            Ceil => a.ceil(),
            Trunc => a.trunc(),
            Fraction => a - a.floor(),
            Modulo => {
                if b != 0. {
                    a % b
                } else {
                    0.
                }
            }
            FlooredModulo => {
                if b != 0. {
                    a - (a / b).floor() * b
                } else {
                    0.
                }
            }
            Wrap => {
                let range = b - c;
                if range != 0. {
                    a - range * ((a - c) / range).floor()
                } else {
                    c
                }
            }
            Snap => safe_div(a, b).floor() * b,
            PingPong => {
                if b != 0. {
                    let f = (a - b) / (b * 2.);
                    ((f - f.floor()) * b * 2. - b).abs()
                } else {
                    0.
                }
            }
            Sine => a.sin(),
            Cosine => a.cos(),
            Tangent => a.tan(),
            Arcsine => a.clamp(-1., 1.).asin(),
            Arccosine => a.clamp(-1., 1.).acos(),
            Arctangent => a.atan(),
            Arctan2 => {
                if a == 0. && b == 0. {
                    0.
                } else {
                    a.atan2(b)
                }
            }
            Sinh => a.sinh(),
            Cosh => a.cosh(),
            Tanh => a.tanh(),
            Radians => a.to_radians(),
            Degrees => a.to_degrees(),
        }
    }
}

fn sign(a: f64) -> f64 {
    if a == 0. {
        0.
    } else if a < 0. {
        -1.
    } else {
        1.
    }
}

fn safe_pow(a: f64, b: f64) -> f64 {
    if a < 0. && b != b.trunc() {
        return 0.;
    }
    if b == 0. {
        return 1.;
    }
    if a == 0. {
        return 0.;
    }
    a.powf(b)
}

fn smooth_min(a: f64, b: f64, k: f64) -> f64 {
    if k != 0. {
        let h = (k - (a - b).abs()).max(0.) / k;
        a.min(b) - h * h * h * k / 6.
    } else {
        a.min(b)
    }
}

/// Cycles' Vector Math node operations (`NodeVectorMathType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VectorOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    CrossProduct,
    Project,
    Reflect,
    Refract,
    Faceforward,
    MultiplyAdd,
    DotProduct,
    Distance,
    Length,
    Scale,
    Normalize,
    Snap,
    Round,
    Floor,
    Ceil,
    Modulo,
    Wrap,
    Fraction,
    Absolute,
    Power,
    Sign,
    Minimum,
    Maximum,
    Sine,
    Cosine,
    Tangent,
}
const VECTOR_NAMES: [(&str, VectorOp); 30] = [
    ("add", VectorOp::Add),
    ("subtract", VectorOp::Subtract),
    ("multiply", VectorOp::Multiply),
    ("divide", VectorOp::Divide),
    ("cross_product", VectorOp::CrossProduct),
    ("project", VectorOp::Project),
    ("reflect", VectorOp::Reflect),
    ("refract", VectorOp::Refract),
    ("faceforward", VectorOp::Faceforward),
    ("multiply_add", VectorOp::MultiplyAdd),
    ("dot_product", VectorOp::DotProduct),
    ("distance", VectorOp::Distance),
    ("length", VectorOp::Length),
    ("scale", VectorOp::Scale),
    ("normalize", VectorOp::Normalize),
    ("snap", VectorOp::Snap),
    ("round", VectorOp::Round),
    ("floor", VectorOp::Floor),
    ("ceil", VectorOp::Ceil),
    ("modulo", VectorOp::Modulo),
    ("wrap", VectorOp::Wrap),
    ("fraction", VectorOp::Fraction),
    ("absolute", VectorOp::Absolute),
    ("power", VectorOp::Power),
    ("sign", VectorOp::Sign),
    ("minimum", VectorOp::Minimum),
    ("maximum", VectorOp::Maximum),
    ("sine", VectorOp::Sine),
    ("cosine", VectorOp::Cosine),
    ("tangent", VectorOp::Tangent),
];
impl VectorOp {
    pub fn from_name(name: &str) -> Option<Self> {
        VECTOR_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, op)| *op)
    }
    pub fn code(self) -> u32 {
        self as u32
    }
    /// Whether the node's result is its Value (a scalar) rather than its
    /// Vector.
    pub fn gives_value(self) -> bool {
        matches!(
            self,
            VectorOp::DotProduct | VectorOp::Distance | VectorOp::Length
        )
    }
    /// `svm_vector_math`: the Vector, or for [`gives_value`] the Value as a
    /// grey.
    pub fn apply(self, a: [f64; 3], b: [f64; 3], c: [f64; 3], scale: f64) -> [f64; 3] {
        use VectorOp::*;
        let per = |f: &dyn Fn(f64, f64, f64) -> f64| [0, 1, 2].map(|k| f(a[k], b[k], c[k]));
        let dot = |x: [f64; 3], y: [f64; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
        let len = |x: [f64; 3]| dot(x, x).sqrt();
        let normalize = |x: [f64; 3]| {
            let l = len(x);
            if l != 0. { x.map(|v| v / l) } else { [0.; 3] }
        };
        match self {
            Add => per(&|a, b, _| a + b),
            Subtract => per(&|a, b, _| a - b),
            Multiply => per(&|a, b, _| a * b),
            Divide => per(&|a, b, _| if b != 0. { a / b } else { 0. }),
            CrossProduct => [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ],
            Project => {
                let l2 = dot(b, b);
                if l2 != 0. {
                    let k = dot(a, b) / l2;
                    b.map(|v| v * k)
                } else {
                    [0.; 3]
                }
            }
            Reflect => {
                let n = normalize(b);
                let d = 2. * dot(n, a);
                [0, 1, 2].map(|k| a[k] - d * n[k])
            }
            Refract => {
                let n = normalize(b);
                let eta = scale;
                let ni = dot(n, a);
                let k = 1. - eta * eta * (1. - ni * ni);
                if k < 0. {
                    [0.; 3]
                } else {
                    [0, 1, 2].map(|i| eta * a[i] - (eta * ni + k.sqrt()) * n[i])
                }
            }
            Faceforward => {
                if dot(c, b) < 0. {
                    a
                } else {
                    a.map(|v| -v)
                }
            }
            MultiplyAdd => per(&|a, b, c| a * b + c),
            DotProduct => [dot(a, b); 3],
            Distance => [len([a[0] - b[0], a[1] - b[1], a[2] - b[2]]); 3],
            Length => [len(a); 3],
            Scale => a.map(|v| v * scale),
            Normalize => normalize(a),
            Snap => per(&|a, b, _| if b != 0. { (a / b).floor() * b } else { 0. }),
            Round => a.map(|v| (v + 0.5).floor()),
            Floor => a.map(f64::floor),
            Ceil => a.map(f64::ceil),
            Modulo => per(&|a, b, _| if b != 0. { a % b } else { 0. }),
            Wrap => per(&|a, b, c| MathOp::Wrap.apply(a, b, c)),
            Fraction => a.map(|v| v - v.floor()),
            Absolute => a.map(f64::abs),
            Power => per(&|a, b, _| safe_pow(a, b)),
            Sign => a.map(sign),
            Minimum => per(&|a, b, _| a.min(b)),
            Maximum => per(&|a, b, _| a.max(b)),
            Sine => a.map(f64::sin),
            Cosine => a.map(f64::cos),
            Tangent => a.map(f64::tan),
        }
    }
}

/// The colour blends `shader_eval::Mix` doesn't have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MixMore {
    Hue,
    Saturation,
    Value,
    Color,
    LinearLight,
    Exclusion,
}
impl MixMore {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "hue" => MixMore::Hue,
            "saturation" => MixMore::Saturation,
            "value" => MixMore::Value,
            "color" => MixMore::Color,
            "linear_light" => MixMore::LinearLight,
            "exclusion" => MixMore::Exclusion,
            _ => return None,
        })
    }
    pub fn code(self) -> u32 {
        self as u32
    }
    fn apply(self, t: f64, a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        use crate::process_occlusion::{hsv_to_rgb, rgb_to_hsv};
        let lerp = |x: [f64; 3], y: [f64; 3]| [0, 1, 2].map(|k| x[k] + (y[k] - x[k]) * t);
        match self {
            MixMore::Hue => {
                let h2 = rgb_to_hsv(b);
                if h2[1] != 0. {
                    let mut h = rgb_to_hsv(a);
                    h[0] = h2[0];
                    lerp(a, hsv_to_rgb(h))
                } else {
                    a
                }
            }
            MixMore::Saturation => {
                let mut h = rgb_to_hsv(a);
                if h[1] != 0. {
                    let h2 = rgb_to_hsv(b);
                    h[1] = (1. - t) * h[1] + t * h2[1];
                    hsv_to_rgb(h)
                } else {
                    a
                }
            }
            MixMore::Value => {
                let mut h = rgb_to_hsv(a);
                let h2 = rgb_to_hsv(b);
                h[2] = (1. - t) * h[2] + t * h2[2];
                hsv_to_rgb(h)
            }
            MixMore::Color => {
                let h2 = rgb_to_hsv(b);
                if h2[1] != 0. {
                    let mut h = rgb_to_hsv(a);
                    h[0] = h2[0];
                    h[1] = h2[1];
                    lerp(a, hsv_to_rgb(h))
                } else {
                    a
                }
            }
            MixMore::LinearLight => [0, 1, 2].map(|k| a[k] + t * (2. * b[k] - 1.)),
            MixMore::Exclusion => {
                let e = [0, 1, 2].map(|k| a[k] + b[k] - 2. * a[k] * b[k]);
                lerp(a, e).map(|v| v.max(0.))
            }
        }
    }
}

/// Map Range's interpolation types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RangeKind {
    Linear,
    Stepped,
    Smoothstep,
    Smootherstep,
}
impl RangeKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "linear" => RangeKind::Linear,
            "stepped" => RangeKind::Stepped,
            "smoothstep" => RangeKind::Smoothstep,
            "smootherstep" => RangeKind::Smootherstep,
            _ => return None,
        })
    }
}

/// The Mapping node's types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MappingKind {
    Point,
    Texture,
    Vector,
    Normal,
}
impl MappingKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "point" => MappingKind::Point,
            "texture" => MappingKind::Texture,
            "vector" => MappingKind::Vector,
            "normal" => MappingKind::Normal,
            _ => return None,
        })
    }
}

/// What an `Expr::Func` computes, with its arguments in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Func {
    /// [a, b, c], per component; `clamp` saturates the result.
    Math {
        op: MathOp,
        clamp: bool,
    },
    /// [a, b, c, scale].
    VectorMath(VectorOp),
    /// [a]: each component clamped to 0..1.
    Clamp01,
    /// [x, y, z]: (x₀, y₀, z₀).
    Combine,
    /// [a]: the components' mean, as a grey (a vector read as a float).
    Average,
    /// [fac, a, b].
    MixMore(MixMore),
    /// [fac, a, b]: a + (b − a)·fac per component, unclamped.
    Lerp,
    /// [hue, saturation, value, fac, colour]: the Hue/Saturation/Value node.
    HueSatVal,
    /// [colour] to HSV, HSL, and back.
    ToHsv,
    ToHsl,
    FromHsv,
    FromHsl,
    /// [value, from min, from max, to min, to max, steps] (scalars).
    MapRange {
        kind: RangeKind,
        clamp: bool,
    },
    /// [vector, location, rotation, scale].
    Mapping(MappingKind),
    /// [ior, incidence in degrees]: the Fresnel node.
    Fresnel,
    /// [blend, incidence in degrees]: Layer Weight's Fresnel or Facing.
    LayerWeight {
        facing: bool,
    },
    /// [colour]: linear to sRGB-encoded numbers, for a data picture the
    /// viewport decoded as sRGB.
    SrgbEncode,
    /// [colour, gamma]: the Gamma node with a linked gamma.
    Gamma,
    /// [vector, w, scale, detail, roughness, lacunarity, offset, gain,
    /// distortion]: Fac (`color` false) or Color.
    Noise {
        dims: u8,
        kind: NoiseKind,
        normalize: bool,
        color: bool,
    },
    /// [vector, w].
    WhiteNoise {
        dims: u8,
        color: bool,
    },
    /// [vector, colour 1, colour 2, scale]: Color, or Fac.
    Checker {
        color: bool,
    },
    /// [vector].
    Gradient(GradientKind),
    /// [vector, scale, distortion, detail, detail scale, detail roughness,
    /// phase offset].
    Wave(WaveKind),
    /// [vector, w, scale, detail, roughness, lacunarity, smoothness,
    /// exponent, randomness]: one output of the Voronoi texture.
    Voronoi {
        dims: u8,
        feature: VoronoiFeature,
        metric: VoronoiMetric,
        normalize: bool,
        out: VoronoiOut,
    },
}

impl Func {
    /// How many arguments it takes.
    pub fn arity(self) -> usize {
        match self {
            Func::Math { .. } | Func::Combine => 3,
            Func::VectorMath(_) | Func::Mapping(_) | Func::Checker { .. } => 4,
            Func::Clamp01
            | Func::Average
            | Func::ToHsv
            | Func::ToHsl
            | Func::FromHsv
            | Func::FromHsl
            | Func::SrgbEncode
            | Func::Gradient(_) => 1,
            Func::MixMore(_) | Func::Lerp => 3,
            Func::HueSatVal => 5,
            Func::MapRange { .. } => 6,
            Func::Fresnel | Func::LayerWeight { .. } | Func::Gamma | Func::WhiteNoise { .. } => 2,
            Func::Noise { .. } | Func::Voronoi { .. } => 9,
            Func::Wave(_) => 7,
        }
    }

    /// The value at arguments `x` (the CPU sum).
    pub fn eval(self, x: &[[f64; 3]]) -> [f64; 3] {
        if x.len() < self.arity() {
            return [0.; 3];
        }
        let s = |i: usize| x[i][0];
        let f3 = |i: usize| x[i].map(|v| v as f32);
        let grey = |v: f32| [f64::from(v); 3];
        let wide = |v: [f32; 3]| v.map(f64::from);
        match self {
            Func::Math { op, clamp } => {
                let v = [0, 1, 2].map(|k| op.apply(x[0][k], x[1][k], x[2][k]));
                if clamp { v.map(|c| c.clamp(0., 1.)) } else { v }
            }
            Func::VectorMath(op) => op.apply(x[0], x[1], x[2], s(3)),
            Func::Clamp01 => x[0].map(|c| c.clamp(0., 1.)),
            Func::Combine => [s(0), s(1), s(2)],
            Func::Average => [(x[0][0] + x[0][1] + x[0][2]) / 3.; 3],
            Func::MixMore(m) => m.apply(s(0), x[1], x[2]),
            Func::Lerp => [0, 1, 2].map(|k| x[1][k] + (x[2][k] - x[1][k]) * x[0][k]),
            Func::HueSatVal => {
                use crate::process_occlusion::{hsv_to_rgb, rgb_to_hsv};
                let c = x[4];
                let mut h = rgb_to_hsv(c);
                let hue = h[0] + s(0) + 0.5;
                h[0] = hue - hue.floor();
                h[1] = (h[1] * s(1)).clamp(0., 1.);
                h[2] *= s(2);
                let out = hsv_to_rgb(h);
                let fac = s(3);
                [0, 1, 2].map(|k| (fac * out[k] + (1. - fac) * c[k]).max(0.))
            }
            Func::ToHsv => crate::process_occlusion::rgb_to_hsv(x[0]),
            Func::FromHsv => crate::process_occlusion::hsv_to_rgb(x[0]),
            Func::ToHsl => rgb_to_hsl(x[0]),
            Func::FromHsl => hsl_to_rgb(x[0]),
            Func::MapRange { kind, clamp } => {
                [map_range(kind, clamp, s(0), s(1), s(2), s(3), s(4), s(5)); 3]
            }
            Func::Mapping(kind) => mapping(kind, x[0], x[1], x[2], x[3]),
            Func::Fresnel => {
                let eta = s(0).max(1e-5);
                [fresnel_dielectric_cos(s(1).to_radians().cos(), eta); 3]
            }
            Func::LayerWeight { facing } => [layer_weight(facing, s(0), s(1)); 3],
            Func::SrgbEncode => x[0].map(linear_to_srgb),
            Func::Gamma => {
                let g = s(1);
                if g == 0. {
                    [1.; 3]
                } else {
                    x[0].map(|c| if c > 0. { c.powf(g) } else { c })
                }
            }
            Func::Noise {
                dims,
                kind,
                normalize,
                color,
            } => wide(noise::noise_texture(
                dims.into(),
                kind,
                normalize,
                color,
                f3(0),
                s(1) as f32,
                s(2) as f32,
                noise::Fractal {
                    detail: s(3) as f32,
                    roughness: s(4) as f32,
                    lacunarity: s(5) as f32,
                    offset: s(6) as f32,
                    gain: s(7) as f32,
                },
                s(8) as f32,
            )),
            Func::WhiteNoise { dims, color } => {
                wide(noise::white_noise(dims.into(), color, f3(0), s(1) as f32))
            }
            Func::Checker { color } => {
                let scale = s(3) as f32;
                let f = noise::checker(f3(0).map(|c| c * scale));
                match (color, f == 1.0) {
                    (false, _) => grey(f),
                    (true, true) => x[1],
                    (true, false) => x[2],
                }
            }
            Func::Gradient(kind) => grey(noise::gradient(kind, f3(0))),
            Func::Wave(kind) => {
                let scale = s(1) as f32;
                grey(noise::wave(
                    kind,
                    f3(0).map(|c| c * scale),
                    s(2) as f32,
                    s(3) as f32,
                    s(4) as f32,
                    s(5) as f32,
                    s(6) as f32,
                ))
            }
            Func::Voronoi {
                dims,
                feature,
                metric,
                normalize,
                out,
            } => {
                let v = Voronoi {
                    dims: dims.into(),
                    feature,
                    metric,
                    normalize,
                    scale: s(2) as f32,
                    detail: s(3) as f32,
                    roughness: s(4) as f32,
                    lacunarity: s(5) as f32,
                    smoothness: s(6) as f32,
                    exponent: s(7) as f32,
                    randomness: s(8) as f32,
                };
                wide(v.eval(out, f3(0), s(1) as f32))
            }
        }
    }

    /// Whether the value changes across a polygon even when its arguments
    /// are smooth there (a procedural texture), so the viewport draws it
    /// per pixel.
    pub fn procedural(self) -> bool {
        matches!(
            self,
            Func::Noise { .. }
                | Func::WhiteNoise { .. }
                | Func::Checker { .. }
                | Func::Gradient(_)
                | Func::Wave(_)
                | Func::Voronoi { .. }
        )
    }

    /// The WGSL call computing it from arguments named `a` (each a
    /// `vec3<f32>`), with `shader_func.wgsl` in the module.
    pub fn wgsl(self, a: &[String]) -> String {
        let all = a.join(", ");
        let b = |v: bool| if v { "true" } else { "false" };
        match self {
            Func::Math { op, clamp } => {
                format!("op_g_math({}u, {all}, {})", op.code(), b(clamp))
            }
            Func::VectorMath(op) => format!("op_g_vmath({}u, {all})", op.code()),
            Func::Clamp01 => format!("clamp({all}, vec3<f32>(0.0), vec3<f32>(1.0))"),
            Func::Combine => format!("vec3<f32>({}.x, {}.x, {}.x)", a[0], a[1], a[2]),
            Func::Average => format!("vec3<f32>(({0}.x + {0}.y + {0}.z) / 3.0)", a[0]),
            Func::MixMore(m) => format!("op_g_mix_more({}u, {all})", m.code()),
            Func::Lerp => format!("({1} + ({2} - {1}) * {0})", a[0], a[1], a[2]),
            Func::HueSatVal => format!("op_g_hsv_node({all})"),
            Func::ToHsv => format!("op_rgb_to_hsv({all})"),
            Func::FromHsv => format!("op_hsv_to_rgb({all})"),
            Func::ToHsl => format!("op_g_rgb_to_hsl({all})"),
            Func::FromHsl => format!("op_g_hsl_to_rgb({all})"),
            Func::MapRange { kind, clamp } => {
                format!("op_g_map_range({}u, {}, {all})", kind as u32, b(clamp))
            }
            Func::Mapping(kind) => format!("op_g_mapping({}u, {all})", kind as u32),
            Func::Fresnel => format!("op_g_fresnel({all})"),
            Func::LayerWeight { facing } => format!("op_g_layer_weight({}, {all})", b(facing)),
            Func::SrgbEncode => format!("op_g_srgb_encode({all})"),
            Func::Gamma => format!("op_g_gamma({all})"),
            Func::Noise {
                dims,
                kind,
                normalize,
                color,
            } => format!(
                "op_g_noise({dims}u, {}u, {}, {}, {all})",
                kind.code(),
                b(normalize),
                b(color)
            ),
            Func::WhiteNoise { dims, color } => {
                format!("op_g_white_noise({dims}u, {}, {all})", b(color))
            }
            Func::Checker { color } => format!("op_g_checker({}, {all})", b(color)),
            Func::Gradient(kind) => format!("vec3<f32>(op_g_gradient({}u, {all}))", kind.code()),
            Func::Wave(k) => format!(
                "vec3<f32>(op_g_wave({}, {}u, {}u, {all}))",
                b(k.rings),
                k.direction,
                k.profile
            ),
            Func::Voronoi {
                dims,
                feature,
                metric,
                normalize,
                out,
            } => format!(
                "op_g_voronoi({dims}u, {}u, {}u, {}, {}u, {all})",
                feature.code(),
                metric.code(),
                b(normalize),
                out.code()
            ),
        }
    }
}

/// The helpers [`Func::wgsl`] calls, besides [`VORONOI_WGSL`].
pub const WGSL: &str = include_str!("shader_func.wgsl");
/// Cycles' Voronoi texture and the Jenkins hashes, in every material
/// module (the tree's Cellular patterns use them too).
pub const VORONOI_WGSL: &str = include_str!("cycles_voronoi.wgsl");

pub fn rgb_to_hsl([r, g, b]: [f64; 3]) -> [f64; 3] {
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let l = ((cmax + cmin) / 2.).min(1.);
    let (mut h, s);
    if cmax == cmin {
        h = 0.;
        s = 0.;
    } else {
        let d = cmax - cmin;
        s = if l > 0.5 {
            d / (2. - cmax - cmin)
        } else {
            d / (cmax + cmin)
        };
        h = if cmax == r {
            (g - b) / d + if g < b { 6. } else { 0. }
        } else if cmax == g {
            (b - r) / d + 2.
        } else {
            (r - g) / d + 4.
        };
    }
    h /= 6.;
    [h, s, l]
}

pub fn hsl_to_rgb([h, s, l]: [f64; 3]) -> [f64; 3] {
    let nr = ((h * 6. - 3.).abs() - 1.).clamp(0., 1.);
    let ng = (2. - (h * 6. - 2.).abs()).clamp(0., 1.);
    let nb = (2. - (h * 6. - 4.).abs()).clamp(0., 1.);
    let chroma = (1. - (2. * l - 1.).abs()) * s;
    [nr, ng, nb].map(|n| (n - 0.5) * chroma + l)
}

pub fn linear_to_srgb(c: f64) -> f64 {
    if c < 0.0031308 {
        if c < 0. { 0. } else { c * 12.92 }
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    if x < e0 {
        0.
    } else if x >= e1 {
        1.
    } else {
        let t = (x - e0) / (e1 - e0);
        (3. - 2. * t) * t * t
    }
}

fn smootherstep(e0: f64, e1: f64, x: f64) -> f64 {
    let d = e1 - e0;
    let x = if d != 0. { (x - e0) / d } else { 0. }.clamp(0., 1.);
    x * x * x * (x * (x * 6. - 15.) + 10.)
}

/// `svm_node_map_range` plus the clamp Cycles adds after it.
#[allow(clippy::too_many_arguments)]
fn map_range(
    kind: RangeKind,
    clamp: bool,
    value: f64,
    from_min: f64,
    from_max: f64,
    to_min: f64,
    to_max: f64,
    steps: f64,
) -> f64 {
    if from_max == from_min {
        return 0.;
    }
    let factor = match kind {
        RangeKind::Linear => (value - from_min) / (from_max - from_min),
        RangeKind::Stepped => {
            let f = (value - from_min) / (from_max - from_min);
            if steps > 0. {
                (f * (steps + 1.)).floor() / steps
            } else {
                0.
            }
        }
        RangeKind::Smoothstep => {
            if from_min > from_max {
                1. - smoothstep(from_max, from_min, value)
            } else {
                smoothstep(from_min, from_max, value)
            }
        }
        RangeKind::Smootherstep => {
            if from_min > from_max {
                1. - smootherstep(from_max, from_min, value)
            } else {
                smootherstep(from_min, from_max, value)
            }
        }
    };
    let out = to_min + factor * (to_max - to_min);
    if clamp && matches!(kind, RangeKind::Linear | RangeKind::Stepped) {
        out.clamp(to_min.min(to_max), to_min.max(to_max))
    } else {
        out
    }
}

/// Cycles' `euler_to_transform` rows.
fn euler_rows(e: [f64; 3]) -> [[f64; 3]; 3] {
    let (sx, cx) = e[0].sin_cos();
    let (sy, cy) = e[1].sin_cos();
    let (sz, cz) = e[2].sin_cos();
    [
        [cy * cz, sy * sx * cz - cx * sz, sy * cx * cz + sx * sz],
        [cy * sz, sy * sx * sz + cx * cz, sy * cx * sz - sx * cz],
        [-sy, cy * sx, cy * cx],
    ]
}

fn mapping(
    kind: MappingKind,
    v: [f64; 3],
    location: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
) -> [f64; 3] {
    let m = euler_rows(rotation);
    let rotate = |p: [f64; 3]| m.map(|r| r[0] * p[0] + r[1] * p[1] + r[2] * p[2]);
    let rotate_back =
        |p: [f64; 3]| [0, 1, 2].map(|k| m[0][k] * p[0] + m[1][k] * p[1] + m[2][k] * p[2]);
    let div = |a: f64, b: f64| if b != 0. { a / b } else { 0. };
    match kind {
        MappingKind::Point => {
            let r = rotate([0, 1, 2].map(|k| v[k] * scale[k]));
            [0, 1, 2].map(|k| r[k] + location[k])
        }
        MappingKind::Texture => {
            let r = rotate_back([0, 1, 2].map(|k| v[k] - location[k]));
            [0, 1, 2].map(|k| div(r[k], scale[k]))
        }
        MappingKind::Vector => rotate([0, 1, 2].map(|k| v[k] * scale[k])),
        MappingKind::Normal => {
            let r = rotate([0, 1, 2].map(|k| div(v[k], scale[k])));
            let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
            if l != 0. { r.map(|c| c / l) } else { [0.; 3] }
        }
    }
}

pub fn fresnel_dielectric_cos(cosi: f64, eta: f64) -> f64 {
    let c = cosi.abs();
    let g = eta * eta - 1. + c * c;
    if g > 0. {
        let g = g.sqrt();
        let a = (g - c) / (g + c);
        let b = (c * (g + c) - 1.) / (c * (g - c) + 1.);
        0.5 * a * a * (1. + b * b)
    } else {
        1.
    }
}

/// Layer Weight on a front face, the normal toward the viewer at
/// `incidence` degrees.
fn layer_weight(facing: bool, blend: f64, incidence: f64) -> f64 {
    let cos = incidence.to_radians().cos();
    if !facing {
        let eta = (1. - blend).max(1e-5);
        return fresnel_dielectric_cos(cos, 1. / eta);
    }
    let mut f = cos.abs();
    if blend != 0.5 {
        let b = blend.clamp(0., 1. - 1e-5);
        let b = if b < 0.5 { 2. * b } else { 0.5 / (1. - b) };
        f = f.powf(b);
    }
    1. - f
}
