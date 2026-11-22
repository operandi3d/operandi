//! Evaluates the shader tree for one surface context, as a first step to
//! rendering it (shader tree hand-off v0.2, §8; slice ST-4).
//!
//! A *context* is what decides which layers apply to a polygon: its item
//! and its polygon tags. For a context the tree folds bottom to top into one
//! value per material channel. A value is an [`Expr`]: a constant, an image
//! lookup, or a mix of two values. Constants fold as they are built, so a
//! tree of plain materials and Constant textures gives plain numbers; image
//! maps keep a small expression the renderer turns into nodes.
//!
//! The renderer then reads the result through [`Principled`], the inputs of
//! Cycles' principled surface. How Operandi's channels map onto those inputs
//! is our own choice (provisional, see `PRINCIPLED` notes below).
use crate::shader_tree::{
    BlendMode, Filtering, GroupMask, ImageAlpha, ImageSettings, ItemMask, MaterialChannels, NodeId,
    NodeKind, Scope, ShaderChannels, ShaderTree, ShadingModel, Swizzle, TagMask, TagType,
    TextureEffect, TextureLayer, TextureSource, TwoZone,
};
use crate::texture_locator::{LocatorFrame, Tiling};

/// Material channels the evaluator folds, in a fixed order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ch {
    DiffuseColor,
    DiffuseAmount,
    DiffuseRoughness,
    SpecularColor,
    SpecularAmount,
    Fresnel,
    Roughness,
    Anisotropy,
    ReflectionColor,
    ReflectionAmount,
    ClearcoatAmount,
    ClearcoatRoughness,
    Metallic,
    Sheen,
    SheenTint,
    SpecularTint,
    Flatness,
    TransparentColor,
    TransparentAmount,
    RefractiveIndex,
    RefractionRoughness,
    Dissolve,
    SubsurfaceColor,
    SubsurfaceAmount,
    SubsurfaceDistance,
    LuminousColor,
    LuminousIntensity,
    ReflectionFresnel,
    BumpAmplitude,
    DisplacementDistance,
    ClearcoatBump,
    /// The folded bump height (texture layers only; flat 0 to start).
    Bump,
    /// The folded displacement value, before the clamp to ±1.
    Displacement,
    /// The folded tangent-space normal colour; flat (0.5, 0.5, 1) to start.
    NormalMap,
    /// The folded Stencil value (cut-out effects v1): above 50 % the
    /// surface is gone. Materials start it at 0.
    Stencil,
    /// RGBA layers' cut-out: 1 where the image's alpha is below 50 %.
    RgbaCut,
    /// Driver A–D (gradient family §2): hidden channels only texture
    /// layers fold into, read by Gradients; 0 to start.
    DriverA,
    DriverB,
    DriverC,
    DriverD,
    /// Material extras (group extras §3): Absorption Distance (metres),
    /// Subsurface Phase (raw 0..1) and Rounded Edge Width (metres).
    AbsorptionDistance,
    SubsurfacePhase,
    RoundedEdge,
    /// The OpenPBR slots of native v41 (OPM-02, `OpenPbrChannels`).
    CoatColor,
    CoatIor,
    FuzzColor,
    FuzzRoughness,
    ThinFilmWeight,
    /// Micrometres.
    ThinFilmThickness,
    ThinFilmIor,
    /// Dispersion scale (Material "Dispersion", 0..1).
    Dispersion,
    DispersionAbbe,
    TransmissionScatter,
    ScatterAnisotropy,
    SubsurfaceAlbedo,
}
impl Ch {
    pub const ALL: [Ch; 55] = {
        use Ch::*;
        [
            DiffuseColor,
            DiffuseAmount,
            DiffuseRoughness,
            SpecularColor,
            SpecularAmount,
            Fresnel,
            Roughness,
            Anisotropy,
            ReflectionColor,
            ReflectionAmount,
            ClearcoatAmount,
            ClearcoatRoughness,
            Metallic,
            Sheen,
            SheenTint,
            SpecularTint,
            Flatness,
            TransparentColor,
            TransparentAmount,
            RefractiveIndex,
            RefractionRoughness,
            Dissolve,
            SubsurfaceColor,
            SubsurfaceAmount,
            SubsurfaceDistance,
            LuminousColor,
            LuminousIntensity,
            ReflectionFresnel,
            BumpAmplitude,
            DisplacementDistance,
            ClearcoatBump,
            Bump,
            Displacement,
            NormalMap,
            Stencil,
            RgbaCut,
            DriverA,
            DriverB,
            DriverC,
            DriverD,
            AbsorptionDistance,
            SubsurfacePhase,
            RoundedEdge,
            CoatColor,
            CoatIor,
            FuzzColor,
            FuzzRoughness,
            ThinFilmWeight,
            ThinFilmThickness,
            ThinFilmIor,
            Dispersion,
            DispersionAbbe,
            TransmissionScatter,
            ScatterAnisotropy,
            SubsurfaceAlbedo,
        ]
    };
    /// Channels only texture layers drive: a Material layer has no value
    /// of its own there, so it leaves what is below untouched (G-07).
    pub fn is_texture_only(self) -> bool {
        matches!(
            self,
            Ch::Bump
                | Ch::Displacement
                | Ch::NormalMap
                | Ch::DriverA
                | Ch::DriverB
                | Ch::DriverC
                | Ch::DriverD
        )
    }
    pub fn is_color(self) -> bool {
        use Ch::*;
        matches!(
            self,
            DiffuseColor
                | SpecularColor
                | ReflectionColor
                | TransparentColor
                | SubsurfaceColor
                | LuminousColor
                | NormalMap
                | CoatColor
                | FuzzColor
                | TransmissionScatter
                | SubsurfaceAlbedo
        )
    }
    /// The channel a texture effect drives, `None` for effects this first
    /// version doesn't render (drivers, stencil, vector displacement...).
    pub fn of_effect(effect: TextureEffect) -> Option<Ch> {
        use TextureEffect as E;
        Some(match effect {
            E::DiffuseColor => Ch::DiffuseColor,
            E::DiffuseAmount => Ch::DiffuseAmount,
            E::DiffuseRoughness => Ch::DiffuseRoughness,
            E::SpecularColor => Ch::SpecularColor,
            E::SpecularAmount => Ch::SpecularAmount,
            E::SpecularFresnel => Ch::Fresnel,
            E::ReflectionColor => Ch::ReflectionColor,
            E::ReflectionAmount => Ch::ReflectionAmount,
            E::ReflectionFresnel => Ch::ReflectionFresnel,
            E::Roughness => Ch::Roughness,
            E::ClearcoatAmount => Ch::ClearcoatAmount,
            E::TransparentColor => Ch::TransparentColor,
            E::TransparentAmount => Ch::TransparentAmount,
            E::RefractionRoughness => Ch::RefractionRoughness,
            E::SubsurfaceColor => Ch::SubsurfaceColor,
            E::SubsurfaceAmount => Ch::SubsurfaceAmount,
            E::LuminousColor => Ch::LuminousColor,
            E::LuminousAmount => Ch::LuminousIntensity,
            E::Dissolve => Ch::Dissolve,
            E::Flatness => Ch::Flatness,
            E::Metallic => Ch::Metallic,
            E::Sheen => Ch::Sheen,
            E::SheenTint => Ch::SheenTint,
            E::SpecularTint => Ch::SpecularTint,
            E::Bump => Ch::Bump,
            E::Displacement => Ch::Displacement,
            E::Normal => Ch::NormalMap,
            E::Stencil => Ch::Stencil,
            E::DriverA => Ch::DriverA,
            E::DriverB => Ch::DriverB,
            E::DriverC => Ch::DriverC,
            E::DriverD => Ch::DriverD,
            _ => return None,
        })
    }
}

/// A Material layer's own value for a channel.
pub fn material_value(m: &MaterialChannels, ch: Ch) -> [f64; 3] {
    let s = |v: f64| [v; 3];
    match ch {
        Ch::DiffuseColor => m.diffuse_color,
        Ch::DiffuseAmount => s(m.diffuse_amount),
        Ch::DiffuseRoughness => s(m.diffuse_roughness),
        Ch::SpecularColor => m.specular_color,
        Ch::SpecularAmount => s(m.specular_amount),
        Ch::Fresnel => s(m.fresnel),
        Ch::Roughness => s(m.roughness),
        Ch::Anisotropy => s(m.anisotropy),
        Ch::ReflectionColor => m.reflection_color,
        Ch::ReflectionAmount => s(m.reflection_amount),
        Ch::ClearcoatAmount => s(m.clearcoat_amount),
        Ch::ClearcoatRoughness => s(m.clearcoat_roughness),
        Ch::Metallic => s(m.metallic),
        Ch::Sheen => s(m.sheen),
        Ch::SheenTint => s(m.sheen_tint),
        Ch::SpecularTint => s(m.specular_tint),
        Ch::Flatness => s(m.flatness),
        Ch::TransparentColor => m.transparent_color,
        Ch::TransparentAmount => s(m.transparent_amount),
        Ch::RefractiveIndex => s(m.refractive_index),
        Ch::RefractionRoughness => s(m.refraction_roughness),
        Ch::Dissolve => s(m.dissolve),
        Ch::SubsurfaceColor => m.subsurface_color,
        Ch::SubsurfaceAmount => s(m.subsurface_amount),
        Ch::SubsurfaceDistance => s(m.subsurface_distance),
        Ch::AbsorptionDistance => s(m.absorption_distance),
        Ch::SubsurfacePhase => s(m.more.subsurface_phase),
        Ch::RoundedEdge => s(m.more.rounded_edge_width),
        Ch::CoatColor => m.openpbr.coat_color,
        Ch::CoatIor => s(m.openpbr.coat_ior),
        Ch::FuzzColor => m.openpbr.fuzz_color,
        Ch::FuzzRoughness => s(m.openpbr.fuzz_roughness),
        Ch::ThinFilmWeight => s(m.openpbr.thin_film_weight),
        Ch::ThinFilmThickness => s(m.openpbr.thin_film_thickness),
        Ch::ThinFilmIor => s(m.openpbr.thin_film_ior),
        Ch::Dispersion => s(m.more.dispersion),
        Ch::DispersionAbbe => s(m.openpbr.dispersion_abbe_number),
        Ch::TransmissionScatter => m.openpbr.transmission_scatter,
        Ch::ScatterAnisotropy => s(m.openpbr.transmission_scatter_anisotropy),
        Ch::SubsurfaceAlbedo => m.openpbr.subsurface_albedo,
        Ch::LuminousColor => m.luminous_color,
        Ch::LuminousIntensity => s(m.luminous_intensity),
        Ch::ReflectionFresnel => s(m.reflection_fresnel),
        Ch::BumpAmplitude => s(m.bump_amplitude),
        Ch::DisplacementDistance => s(m.displacement_distance),
        Ch::ClearcoatBump => s(m.clearcoat_bump),
        Ch::Bump
        | Ch::Displacement
        | Ch::Stencil
        | Ch::RgbaCut
        | Ch::DriverA
        | Ch::DriverB
        | Ch::DriverC
        | Ch::DriverD => s(0.),
        Ch::NormalMap => FLAT_NORMAL,
    }
}

/// How steeply [`Expr::step_above`] rises past its edge.
const STEP_SLOPE: f64 = 1e6;

/// A tangent-space normal colour that leaves the surface normal as it is.
pub const FLAT_NORMAL: [f64; 3] = [0.5, 0.5, 1.];

/// The renderer's mix operations (Cycles' Mix Color modes). Every one is
/// `lerp(a, f(a, b), fac)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mix {
    Blend,
    Add,
    Multiply,
    Subtract,
    Screen,
    Divide,
    Difference,
    Darken,
    Lighten,
    Overlay,
    Dodge,
    Burn,
    SoftLight,
}
impl Mix {
    /// Cycles' own `NodeMix` number for this mode.
    pub fn cycles_code(self) -> u32 {
        match self {
            Mix::Blend => 0,
            Mix::Add => 1,
            Mix::Multiply => 2,
            Mix::Subtract => 3,
            Mix::Screen => 4,
            Mix::Divide => 5,
            Mix::Difference => 6,
            Mix::Darken => 7,
            Mix::Lighten => 8,
            Mix::Overlay => 9,
            Mix::Dodge => 10,
            Mix::Burn => 11,
            Mix::SoftLight => 16,
        }
    }
    /// One component, the same sums Cycles' Mix Color node does.
    pub fn apply(self, a: f64, b: f64, t: f64) -> f64 {
        let t = t.clamp(0., 1.);
        let lerp = |x: f64| a + (x - a) * t;
        match self {
            Mix::Blend => lerp(b),
            Mix::Add => a + t * b,
            Mix::Multiply => a * (1. - t + t * b),
            Mix::Subtract => a - t * b,
            Mix::Screen => 1. - (1. - t + t * (1. - b)) * (1. - a),
            Mix::Divide => {
                if b != 0. {
                    a * (1. - t) + t * a / b
                } else {
                    a
                }
            }
            Mix::Difference => lerp((a - b).abs()),
            Mix::Darken => lerp(a.min(b)),
            Mix::Lighten => lerp(a.max(b)),
            Mix::Overlay => {
                if a < 0.5 {
                    a * (1. - t + 2. * t * b)
                } else {
                    1. - (1. - t + 2. * t * (1. - b)) * (1. - a)
                }
            }
            Mix::Dodge => {
                if a == 0. {
                    a
                } else {
                    let tmp = 1. - t * b;
                    if tmp <= 0. { 1. } else { (a / tmp).min(1.) }
                }
            }
            Mix::Burn => {
                let tmp = 1. - t + t * b;
                if tmp <= 0. {
                    0.
                } else {
                    (1. - (1. - a) / tmp).clamp(0., 1.)
                }
            }
            Mix::SoftLight => {
                let screen = 1. - (1. - b) * (1. - a);
                lerp((1. - a) * b * a + a * screen)
            }
        }
    }
}

/// Where an image is read: the image, its filtering, the coordinate vector
/// (an [`Expr`] giving u, v) and what the image does outside 0..1.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageLookup {
    /// `Scene::images` id.
    pub image: u64,
    pub filtering: Filtering,
    pub vector: Expr,
    /// The renderer's image extension: Repeat, Mirror, Edge (extend) or
    /// Reset (clip: transparent black outside).
    pub extension: Tiling,
}
impl ImageLookup {
    /// The mesh's first UV map, repeated: how every image read before
    /// Texture Locators.
    pub fn uv(image: u64, filtering: Filtering) -> Self {
        Self {
            image,
            filtering,
            vector: Expr::mesh_uv(None, [1., 1.], 0.),
            extension: Tiling::Repeat,
        }
    }
}

/// A procedural pattern read at a coordinate vector; 2D patterns use the
/// plane perpendicular to `axis`.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternLookup {
    pub pattern: crate::procedural::Pattern,
    pub axis: u8,
    pub vector: Expr,
}

/// How a projection face is weighted (`Expr::FaceWeight`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaceKind {
    /// Box: faces 0..6 are +X, +Y, +Z, -X, -Y, -Z; 1 on the face the
    /// normal points out of most, else 0.
    Signed,
    /// Cubic: faces 0..3 are X, Y, Z; 1 for the largest normal component.
    Dominant,
    /// Triplanar: |n_face|^k / Σ |n_i|^k.
    Power(f64),
}

/// The dominant axis of a normal: X on ties with Y or Z, Y on ties with Z.
pub fn dominant_axis(n: [f64; 3]) -> usize {
    let a = n.map(f64::abs);
    if a[0] >= a[1] && a[0] >= a[2] {
        0
    } else if a[1] >= a[2] {
        1
    } else {
        2
    }
}

/// A face weight (see [`FaceKind`]).
pub fn face_weight(n: [f64; 3], face: u8, kind: FaceKind) -> f64 {
    let face = usize::from(face);
    match kind {
        FaceKind::Signed => {
            let axis = face % 3;
            let positive = face < 3;
            let on = dominant_axis(n) == axis && (n[axis] >= 0.) == positive;
            if on { 1. } else { 0. }
        }
        FaceKind::Dominant => {
            if dominant_axis(n) == face {
                1.
            } else {
                0.
            }
        }
        FaceKind::Power(k) => {
            let w = n.map(|c| c.abs().powf(k));
            let total: f64 = w.iter().sum();
            if total > 0. {
                w[face.min(2)] / total
            } else {
                1. / 3.
            }
        }
    }
}

/// Falloff opacity at locator-space point `l` (spec §6): linear from 1 at
/// the centre to 0 at |L| = 1 / f per axis (f = 0 is off).
pub fn falloff(kind: crate::texture_locator::FalloffType, f: [f64; 3], l: [f64; 3]) -> f64 {
    use crate::texture_locator::FalloffType as F;
    let d = [0, 1, 2].map(|i| (l[i] * f[i]).abs());
    let reach = match kind {
        F::Cubic => d[0].max(d[1]).max(d[2]),
        F::Spherical => (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(),
        F::LinearX => d[0],
        F::LinearY => d[1],
        F::LinearZ => d[2],
    };
    (1. - reach).clamp(0., 1.)
}

/// Whether a falloff type with amounts `f` fades anything.
pub fn falloff_on(kind: crate::texture_locator::FalloffType, f: [f64; 3]) -> bool {
    use crate::texture_locator::FalloffType as F;
    match kind {
        F::Cubic | F::Spherical => f.iter().any(|&v| v > 0.),
        F::LinearX => f[0] > 0.,
        F::LinearY => f[1] > 0.,
        F::LinearZ => f[2] > 0.,
    }
}

/// Cylindrical / Spherical raw coordinates (u0, v0) of a locator point:
/// the angle runs from the next axis toward the one after (axis Y: from
/// +Z toward +X).
pub fn wrap_coords(axis: u8, l: [f64; 3], spherical: bool) -> [f64; 2] {
    let axis = usize::from(axis.min(2));
    let (from, toward) = ((axis + 1) % 3, (axis + 2) % 3);
    let u0 = l[toward].atan2(l[from]) / std::f64::consts::TAU;
    let v0 = if spherical {
        l[axis].atan2(l[from].hypot(l[toward])) / std::f64::consts::PI
    } else {
        l[axis]
    };
    [u0, v0]
}

/// Light Probe raw coordinates (u, v) in 0..1 of a locator point (TL-Q31,
/// provisional): the direction from the locator's centre read off a
/// mirror-ball photo exactly as an environment Light Probe reads it
/// (`environment::environment_probe_uv`, through `pole_frame`): the
/// Projection Axis is the picture's up, the ball's centre reflects the axis
/// after it (Y: +Z) and its rim the opposite way. v runs up the picture
/// (image UV convention), so v = 1 − the environment's top-down v.
pub fn probe_coords(axis: u8, l: [f64; 3]) -> [f64; 2] {
    // Image frame (`pole_frame`): right, up (the axis), toward the viewer.
    let a = usize::from(axis.min(2));
    let (r, u, z) = ((a + 2) % 3, a, (a + 1) % 3);
    let len = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
    if len <= 0. {
        return [0.5, 0.5];
    }
    let d = [l[r] / len, l[u] / len, l[z] / len + 1.];
    let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if n <= 1e-12 {
        // Straight away from the viewer: the rim.
        return [0.5, 0.];
    }
    [0.5 + 0.5 * d[0] / n, 0.5 + 0.5 * d[1] / n]
}

/// `Expr::HsvVary`, as the renderer's Hue/Saturation/Value node does it.
pub fn hsv_vary(c: [f64; 3], r: [f64; 3], a: [f64; 3]) -> [f64; 3] {
    if a == [0.; 3] {
        return c;
    }
    let (h, s, v) = rgb_to_hsv(c);
    let h = {
        let x = h + a[1] * (r[1] - 0.5);
        x - x.floor()
    };
    let s = (s * (1. - a[2] * r[2])).clamp(0., 1.);
    let v = (v * (1. - a[0] * r[0])).max(0.);
    hsv_to_rgb(h, s, v)
}

fn rgb_to_hsv(c: [f64; 3]) -> (f64, f64, f64) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let d = max - min;
    let s = if max > 0. { d / max } else { 0. };
    let h = if d <= 0. {
        0.
    } else if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.) / 6.
    } else if max == c[1] {
        ((c[2] - c[0]) / d + 2.) / 6.
    } else {
        ((c[0] - c[1]) / d + 4.) / 6.
    };
    (h, s, max)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [f64; 3] {
    let h6 = (h - h.floor()) * 6.;
    let i = h6.floor();
    let f = h6 - i;
    let (p, q, t) = (v * (1. - s), v * (1. - s * f), v * (1. - s * (1. - f)));
    match i as i64 % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

/// One shading point, for [`Expr::eval_at`]: its world position and the
/// mesh UVs there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShadingPoint {
    pub position: [f64; 3],
    pub uv: [f64; 2],
    /// The world surface normal (Cubic, Box, Triplanar, Front Culling).
    pub normal: [f64; 3],
}

/// One channel's value. Scalars carry the same number in all three slots;
/// coordinate vectors (Locator, Planar, MeshUv, Tile) carry x, y, z.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Const([f64; 3]),
    /// The colour of a scene image at a lookup.
    ImageColor(Box<ImageLookup>),
    /// That image's alpha.
    ImageAlpha(Box<ImageLookup>),
    /// The shading point in a Texture Locator's space: `xform` (row-major
    /// 3×4) applied to the world position.
    Locator(Box<[f64; 12]>),
    /// Planar coordinates of a locator-space vector `of` (spec §4):
    /// ((in-plane coordinate + `centre`) × `repeat`) per axis, depth 0.
    Planar {
        of: Box<Expr>,
        axis: u8,
        centre: f64,
        repeat: [f64; 2],
        /// Bit 0 mirrors the in-plane u, bit 1 v (before centring): Box's
        /// negative faces, Front Culling's flips.
        mirror: u8,
    },
    /// Cylindrical (or Spherical) coordinates of a locator-space vector
    /// (spec §4): u0 = angle / 360° around `axis`, v0 = L along it
    /// (Spherical: latitude / 180°); out = ((u0, v0) + centre) × repeat.
    Wrap {
        of: Box<Expr>,
        axis: u8,
        spherical: bool,
        centre: f64,
        repeat: [f64; 2],
    },
    /// Light Probe coordinates of a locator-space vector (TL-Q31): the
    /// mirror-ball picture's (u, v) for the direction from the locator's
    /// centre, the Projection Axis toward the viewer of the ball
    /// (`probe_coords`), × `repeat`; depth 0.
    Probe {
        of: Box<Expr>,
        axis: u8,
        repeat: [f64; 2],
    },
    /// The world normal through a 3×3 matrix (row-major), normalised.
    Normal(Box<[f64; 9]>),
    /// A projection face's weight from a normal vector.
    FaceWeight {
        normal: Box<Expr>,
        face: u8,
        kind: FaceKind,
    },
    /// 1 where the normal's `axis` component is > 0 (the face looks toward
    /// the projection), else 0.
    Facing(Box<Expr>, u8),
    /// Falloff opacity (spec §6) of a locator-space point.
    Falloff {
        of: Box<Expr>,
        kind: crate::texture_locator::FalloffType,
        amount: [f64; 3],
    },
    /// The mesh's UVs (`map`, else its first map) × `repeat`, then rotated
    /// `rotation` degrees about (0, 0); depth 0.
    MeshUv {
        map: Option<String>,
        repeat: [f64; 2],
        rotation: f64,
    },
    /// A coordinate vector tiled per axis (Reset leaves the axis as it is).
    Tile(Box<Expr>, [Tiling; 2]),
    /// 1 where a coordinate vector lies in 0..1 on every clipped axis, else
    /// 0: the coverage of a Reset tile.
    Inside(Box<Expr>, [bool; 2]),
    /// A procedural pattern's value t (0..1) at a coordinate vector
    /// (procedural.rs).
    Pattern(Box<PatternLookup>),
    /// Perlin bias then gain of a value in 0..1 (`TwoZone::bias`/`gain`).
    BiasGain(Box<Expr>, f64, f64),
    /// Three random numbers (value, hue, saturation) for the pattern cell
    /// at a coordinate vector (`Pattern::cell_random`).
    CellRandom(Box<PatternLookup>),
    /// A colour's brightness, hue and saturation varied by random numbers
    /// `random` (value, hue, saturation) by up to `amounts` (the same
    /// order): value × (1 − a·r), hue + a·(r − ½), saturation × (1 − a·r).
    HsvVary {
        color: Box<Expr>,
        random: Box<Expr>,
        amounts: [f64; 3],
    },
    /// Luminance of a colour, for scalar channels driven by an image.
    Luma(Box<Expr>),
    /// One component (0 red, 1 green, 2 blue) of a colour, as a grey.
    Channel(Box<Expr>, u8),
    /// Each component raised to a power (Cycles' Gamma node: components
    /// at or below 0 stay as they are).
    Gamma(Box<Expr>, f64),
    Mix {
        mode: Mix,
        fac: Box<Expr>,
        a: Box<Expr>,
        b: Box<Expr>,
    },
    /// A value of the shading point's geometry (gradient inputs).
    Geometry(GeometryInput),
    /// Metres from the shading point to a world position.
    DistanceTo([f64; 3]),
    /// Degrees between the shading normal and the direction to a world
    /// position: 0 facing it, 180 facing away (Locator Incidence).
    LocatorIncidence([f64; 3]),
    /// The colour in HSV: hue × `hue_scale` + `hue_shift` degrees (wrapped),
    /// saturation × `saturation_scale` (clamped), value kept (Process).
    Hsv {
        of: Box<Expr>,
        hue_scale: f64,
        hue_shift: f64,
        saturation_scale: f64,
    },
    /// The fraction of `rays` from the point that escape within `distance`
    /// metres (1 = open); fired into the surface with `inside`, against the
    /// same object only with `same_surface` (Occlusion texture).
    Occlusion {
        rays: u32,
        distance: f64,
        same_surface: bool,
        inside: bool,
        /// Which way the rays' hemisphere faces (Occlusion Type, PQ-5).
        toward: AoToward,
    },
    /// A per-corner RGBA attribute the renderer exports for the mesh
    /// (vertex_map_texture.rs): its colour, or with `alpha` its alpha.
    Attribute {
        name: String,
        alpha: bool,
    },
    /// A Gradient curve (gradient.rs) sampled `stops` times evenly from
    /// input `lo` to `hi`, read at `input` (held at the ends): its colour,
    /// or with `alpha` its alpha as a grey.
    Ramp {
        input: Box<Expr>,
        lo: f64,
        hi: f64,
        stops: std::sync::Arc<Vec<[f64; 4]>>,
        alpha: bool,
    },
    /// A Cycles node sum the tree itself never makes (a graph layer's Math,
    /// Noise texture...: `shader_func`, MaterialX stage 5).
    Func(Box<FuncCall>),
    /// A free node's output driving a channel (node editing, NE-03;
    /// node_view.rs): renders through its MaterialX nodes, draws through
    /// their lowered expression.
    Nodes(std::sync::Arc<crate::node_view::NodeOut>),
}

/// A [`shader_func::Func`](crate::shader_func::Func) with its arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct FuncCall {
    pub func: crate::shader_func::Func,
    pub args: Vec<Expr>,
}
impl Expr {
    /// `func` of `args`, folded to a constant when every argument is one.
    pub fn func(func: crate::shader_func::Func, args: Vec<Expr>) -> Expr {
        let consts: Option<Vec<[f64; 3]>> = args.iter().map(Expr::constant).collect();
        if let Some(c) = consts {
            return Expr::Const(func.eval(&c));
        }
        Expr::Func(Box::new(FuncCall { func, args }))
    }
}

/// The direction an Occlusion texture's ray hemisphere faces (PQ-5,
/// provisional): the surface normal; world up (Up); halfway between the
/// normal and straight down (Down Slope); the view's mirror direction
/// (Reflection).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AoToward {
    #[default]
    Normal,
    Up,
    DownSlope,
    Reflection,
}
impl AoToward {
    /// `OC_AO_*` in the C bridge (mode bits 2-3).
    pub fn code(self) -> u32 {
        match self {
            AoToward::Normal => 0,
            AoToward::Up => 1,
            AoToward::DownSlope => 2,
            AoToward::Reflection => 3,
        }
    }
}

/// SH-2 (shader-tree-complete, provisional): a Shader whose Effect isn't
/// Full Shading makes its surfaces render only that part of their light,
/// by switching the other parts of the principled surface off. Diffuse:
/// no reflections (metal, gloss, coat), emission or see-through part.
/// Specular and Reflection (both Cycles' glossy light, RO-Q01): the base
/// colour kept only where metallic, no sheen, subsurface, emission or
/// see-through part. Transparent: only the see-through part. Subsurface:
/// no reflections, sheen, emission or see-through part (the share
/// Subsurface Amount leaves stays diffuse). Luminous: only the emission.
/// Fog: the surface black (fog still lies over it). Alpha and cut-outs
/// stay as they are.
fn shader_effect_only(
    effect: crate::shader_tree::ShaderEffect,
    inputs: &mut [(Input, Expr)],
    see_through: Option<SeeThrough>,
) -> Option<SeeThrough> {
    use crate::shader_tree::ShaderEffect as E;
    use Input::*;
    let off: &[Input] = match effect {
        E::FullShading => return see_through,
        E::DiffuseShading => &[Metallic, SpecularIorLevel, CoatWeight, EmissionStrength],
        E::SpecularShading | E::ReflectionShading => {
            &[SheenWeight, SubsurfaceWeight, EmissionStrength]
        }
        E::TransparentShading | E::LuminousShading | E::FogShading => &[
            BaseColor,
            Metallic,
            SpecularIorLevel,
            CoatWeight,
            SheenWeight,
            SubsurfaceWeight,
        ],
        E::SubsurfaceShading => &[
            Metallic,
            SpecularIorLevel,
            CoatWeight,
            SheenWeight,
            EmissionStrength,
        ],
    };
    let metallic = inputs
        .iter()
        .find(|(i, _)| *i == Metallic)
        .map(|(_, e)| e.clone());
    for (input, value) in inputs.iter_mut() {
        if off.contains(input) {
            *value = Expr::scalar(0.);
        } else if *input == BaseColor && matches!(effect, E::SpecularShading | E::ReflectionShading)
        {
            *value = Expr::times(value.clone(), metallic.clone().unwrap_or(Expr::scalar(0.)));
        }
    }
    if matches!(effect, E::LuminousShading) {
        // Emission is all there is; Fog and Transparent drop it too.
    } else if matches!(effect, E::FogShading | E::TransparentShading) {
        for (input, value) in inputs.iter_mut() {
            if *input == EmissionStrength {
                *value = Expr::scalar(0.);
            }
        }
    }
    match effect {
        E::TransparentShading => see_through,
        _ => None,
    }
}

/// Shading-point values a Gradient reads (gradient family §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryInput {
    /// Degrees between the shading normal and the direction to the
    /// viewer: 0 facing it, 90 edge-on.
    Incidence,
    /// Degrees between the shading normal and +Y: 0 facing up, 180 down
    /// (GQ-2).
    Slope,
    /// Straight-line metres from the camera.
    CameraDistance,
    /// 1 on a polygon's back side, else 0.
    BackFacing,
    /// Across the render frame: 0 at the left edge, 1 at the right
    /// (provisional range, GR-Q07).
    ScreenX,
    /// Up the render frame: 0 at the bottom edge, 1 at the top.
    ScreenY,
    /// From the frame's centre (0) to its corners (1): the distance in
    /// pixels over half the frame's diagonal.
    ScreenR,
}
impl GeometryInput {
    /// `OC_GEOMETRY_*` in the C bridge.
    pub fn code(self) -> u32 {
        match self {
            GeometryInput::Incidence => 0,
            GeometryInput::Slope => 1,
            GeometryInput::CameraDistance => 2,
            GeometryInput::BackFacing => 3,
            GeometryInput::ScreenX => 4,
            GeometryInput::ScreenY => 5,
            GeometryInput::ScreenR => 6,
        }
    }
}

/// A ramp table read at `x` between `lo` and `hi` (linear between stops).
pub fn ramp_at(stops: &[[f64; 4]], lo: f64, hi: f64, x: f64) -> [f64; 4] {
    let n = stops.len();
    if n == 0 {
        return [0.; 4];
    }
    let t = if hi > lo {
        ((x - lo) / (hi - lo)).clamp(0., 1.)
    } else {
        0.
    };
    let f = t * (n - 1) as f64;
    let i = (f.floor() as usize).min(n - 1);
    let j = (i + 1).min(n - 1);
    let w = f - i as f64;
    std::array::from_fn(|k| stops[i][k] + (stops[j][k] - stops[i][k]) * w)
}
impl Expr {
    pub fn mesh_uv(map: Option<String>, repeat: [f64; 2], rotation: f64) -> Expr {
        Expr::MeshUv {
            map,
            repeat,
            rotation,
        }
    }
    pub fn scalar(v: f64) -> Expr {
        Expr::Const([v; 3])
    }
    pub fn constant(&self) -> Option<[f64; 3]> {
        match self {
            Expr::Const(c) => Some(*c),
            _ => None,
        }
    }
    /// `lerp(a, mode(a, b), fac)`, folded when it can be.
    pub fn mix(mode: Mix, fac: Expr, a: Expr, b: Expr) -> Expr {
        if let Some(f) = fac.constant() {
            if f[0] <= 0. {
                return a;
            }
            if mode == Mix::Blend && f[0] >= 1. {
                return b;
            }
        }
        if let (Some(f), Some(x), Some(y)) = (fac.constant(), a.constant(), b.constant()) {
            return Expr::Const([0, 1, 2].map(|i| mode.apply(x[i], y[i], f[0])));
        }
        if mode == Mix::Blend && a == b {
            return a;
        }
        Expr::Mix {
            mode,
            fac: Box::new(fac),
            a: Box::new(a),
            b: Box::new(b),
        }
    }
    pub fn times(a: Expr, b: Expr) -> Expr {
        if a == Expr::scalar(1.) {
            return b;
        }
        if b == Expr::scalar(1.) {
            return a;
        }
        Expr::mix(Mix::Multiply, Expr::scalar(1.), a, b)
    }
    pub fn plus(a: Expr, b: Expr) -> Expr {
        if b == Expr::scalar(0.) {
            return a;
        }
        Expr::mix(Mix::Add, Expr::scalar(1.), a, b)
    }
    pub fn one_minus(a: Expr) -> Expr {
        Expr::mix(Mix::Subtract, Expr::scalar(1.), Expr::scalar(1.), a)
    }
    /// 1 where `a` is above `edge`, else 0: a hard cut-out test built from
    /// mixes, clamp((a − edge) × 10⁶, 0, 1). Values within a millionth
    /// above the edge come out partial; at the edge it is 0.
    pub fn step_above(a: Expr, edge: f64) -> Expr {
        let over = Expr::mix(Mix::Subtract, Expr::scalar(1.), a, Expr::scalar(edge));
        let steep = Expr::times(over, Expr::scalar(STEP_SLOPE));
        let low = Expr::mix(Mix::Lighten, Expr::scalar(1.), steep, Expr::scalar(0.));
        Expr::mix(Mix::Darken, Expr::scalar(1.), low, Expr::scalar(1.))
    }
    /// 1 where `a` is below `edge`, else 0 (see [`Expr::step_above`]).
    pub fn step_below(a: Expr, edge: f64) -> Expr {
        Expr::step_above(Expr::one_minus(a), 1. - edge)
    }
    pub fn luma(a: Expr) -> Expr {
        match a {
            Expr::Const(c) => Expr::scalar(luma(c)),
            other => Expr::Luma(Box::new(other)),
        }
    }
    pub fn channel(a: Expr, k: u8) -> Expr {
        match a {
            Expr::Const(c) => Expr::scalar(c[usize::from(k.min(2))]),
            other => Expr::Channel(Box::new(other), k.min(2)),
        }
    }
    pub fn gamma(a: Expr, power: f64) -> Expr {
        if power == 1. {
            return a;
        }
        match a {
            Expr::Const(c) => Expr::Const(c.map(|v| gamma(v, power))),
            other => Expr::Gamma(Box::new(other), power),
        }
    }
    /// The value with every image read at one spot, wherever the point
    /// is: `sample` gives an image's RGBA. For the viewport's averages.
    /// Procedural patterns read t = 0.5 there: the zones' average.
    pub fn eval(&self, sample: &dyn Fn(u64, Filtering) -> [f64; 4]) -> [f64; 3] {
        self.eval_inner(
            &ShadingPoint::default(),
            &|l, _| sample(l.image, l.filtering),
            true,
        )
    }
    /// The value at one shading point, the same sums the renderer does.
    /// `sample` gives an image's RGBA at a lookup's raw coordinates (before
    /// its extension; [`extended`] applies one). For tests and CPU previews.
    pub fn eval_at(
        &self,
        at: &ShadingPoint,
        sample: &dyn Fn(&ImageLookup, [f64; 2]) -> [f64; 4],
    ) -> [f64; 3] {
        self.eval_inner(at, sample, false)
    }
    fn eval_inner(
        &self,
        at: &ShadingPoint,
        sample: &dyn Fn(&ImageLookup, [f64; 2]) -> [f64; 4],
        average: bool,
    ) -> [f64; 3] {
        let ev = |e: &Expr| e.eval_inner(at, sample, average);
        match self {
            Expr::Pattern(_) if average => [0.5; 3],
            Expr::CellRandom(_) if average => [0.5; 3],
            Expr::CellRandom(p) => p
                .pattern
                .cell_random(p.axis, ev(&p.vector))
                .unwrap_or([0.5; 3]),
            Expr::HsvVary {
                color,
                random,
                amounts,
            } => hsv_vary(ev(color), ev(random), *amounts),
            Expr::BiasGain(a, _, _) if average => ev(a),
            Expr::Pattern(p) => [p.pattern.value(p.axis, ev(&p.vector)); 3],
            Expr::BiasGain(a, bias, gain) => {
                let t = ev(a)[0];
                [TwoZone::gain(*gain, TwoZone::bias(*bias, t)); 3]
            }
            Expr::Const(c) => *c,
            Expr::ImageColor(l) => {
                let v = ev(&l.vector);
                let [r, g, b, _] = sample(l, [v[0], v[1]]);
                [r, g, b]
            }
            Expr::ImageAlpha(l) => {
                let v = ev(&l.vector);
                [sample(l, [v[0], v[1]])[3]; 3]
            }
            Expr::Locator(m) => {
                let p = at.position;
                [0, 1, 2].map(|r| {
                    m[r * 4] * p[0] + m[r * 4 + 1] * p[1] + m[r * 4 + 2] * p[2] + m[r * 4 + 3]
                })
            }
            Expr::Planar {
                of,
                axis,
                centre,
                repeat,
                mirror,
            } => {
                let l = ev(of);
                let [mut u, mut v] = crate::texture_locator::planar(*axis, l.into(), 0.);
                if mirror & 1 != 0 {
                    u = -u;
                }
                if mirror & 2 != 0 {
                    v = -v;
                }
                [(u + centre) * repeat[0], (v + centre) * repeat[1], 0.]
            }
            Expr::Wrap {
                of,
                axis,
                spherical,
                centre,
                repeat,
            } => {
                let [u, v] = wrap_coords(*axis, ev(of), *spherical);
                [(u + centre) * repeat[0], (v + centre) * repeat[1], 0.]
            }
            Expr::Probe { of, axis, repeat } => {
                let [u, v] = probe_coords(*axis, ev(of));
                [u * repeat[0], v * repeat[1], 0.]
            }
            Expr::Normal(m) => {
                let n = at.normal;
                let out =
                    [0, 1, 2].map(|r| m[r * 3] * n[0] + m[r * 3 + 1] * n[1] + m[r * 3 + 2] * n[2]);
                let len = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
                if len > 0. { out.map(|c| c / len) } else { out }
            }
            Expr::FaceWeight { normal, face, kind } => [face_weight(ev(normal), *face, *kind); 3],
            Expr::Facing(normal, axis) => {
                [if ev(normal)[usize::from((*axis).min(2))] > 0. {
                    1.
                } else {
                    0.
                }; 3]
            }
            Expr::Falloff { of, kind, amount } => [falloff(*kind, *amount, ev(of)); 3],
            Expr::MeshUv {
                repeat, rotation, ..
            } => {
                let [u, v] = crate::texture_locator::uv_transform(at.uv, *repeat, *rotation);
                [u, v, 0.]
            }
            Expr::Tile(a, tiling) => {
                let v = ev(a);
                let t = |k: usize| match tiling[k] {
                    Tiling::Reset => v[k],
                    mode => crate::texture_locator::tile(v[k], mode).unwrap_or(v[k]),
                };
                [t(0), t(1), v[2]]
            }
            Expr::Inside(a, clip) => {
                let v = ev(a);
                let inside = (0..2).all(|k| !clip[k] || (0.0..=1.0).contains(&v[k]));
                [if inside { 1. } else { 0. }; 3]
            }
            Expr::Luma(a) => [luma(ev(a)); 3],
            Expr::Channel(a, k) => [ev(a)[usize::from(*k)]; 3],
            Expr::Gamma(a, p) => ev(a).map(|v| gamma(v, *p)),
            Expr::Mix { mode, fac, a, b } => {
                let t = ev(fac)[0];
                let (a, b) = (ev(a), ev(b));
                [0, 1, 2].map(|i| mode.apply(a[i], b[i], t))
            }
            // A CPU point has no view or normal: facing the camera, at it;
            // nor mesh attributes.
            Expr::Geometry(_) | Expr::Attribute { .. } => [0.; 3],
            // A CPU point sees nothing around it.
            Expr::Occlusion { .. } => [1.; 3],
            Expr::Hsv {
                of,
                hue_scale,
                hue_shift,
                saturation_scale,
            } => {
                use crate::process_occlusion::{hsv_to_rgb, rgb_to_hsv};
                let [h, s, v] = rgb_to_hsv(ev(of));
                hsv_to_rgb([
                    (h * hue_scale + hue_shift / 360.).rem_euclid(1.),
                    (s * saturation_scale).clamp(0., 1.),
                    v,
                ])
            }
            Expr::LocatorIncidence(p) => {
                let d: [f64; 3] = std::array::from_fn(|k| p[k] - at.position[k]);
                let (ld, ln) = (
                    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt(),
                    at.normal.iter().map(|v| v * v).sum::<f64>().sqrt(),
                );
                if ld <= 0. || ln <= 0. {
                    return [0.; 3];
                }
                let c = (0..3).map(|k| d[k] * at.normal[k]).sum::<f64>() / (ld * ln);
                [c.clamp(-1., 1.).acos().to_degrees(); 3]
            }
            Expr::DistanceTo(p) => {
                let d = (0..3)
                    .map(|k| (at.position[k] - p[k]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                [d; 3]
            }
            Expr::Ramp {
                input,
                lo,
                hi,
                stops,
                alpha,
            } => {
                let v = ramp_at(stops, *lo, *hi, ev(input)[0]);
                if *alpha {
                    [v[3]; 3]
                } else {
                    [v[0], v[1], v[2]]
                }
            }
            // A procedural texture's average, like a pattern's: mid grey.
            Expr::Func(c) if average && c.func.procedural() => [0.5; 3],
            Expr::Func(c) => {
                let args: Vec<[f64; 3]> = c.args.iter().map(ev).collect();
                c.func.eval(&args)
            }
            Expr::Nodes(n) => ev(&n.lowered),
        }
    }
    /// Whether the value changes from point to point by a procedural
    /// pattern (the viewport evaluates those per mesh point).
    pub fn varies_by_pattern(&self) -> bool {
        match self {
            Expr::Pattern(_) | Expr::CellRandom(_) => true,
            Expr::Func(c) => c.func.procedural() || c.args.iter().any(Expr::varies_by_pattern),
            Expr::Nodes(n) => n.lowered.varies_by_pattern(),
            Expr::Const(_)
            | Expr::Locator(_)
            | Expr::MeshUv { .. }
            | Expr::Normal(_)
            | Expr::ImageColor(_)
            | Expr::ImageAlpha(_)
            | Expr::Geometry(_)
            | Expr::DistanceTo(_)
            | Expr::LocatorIncidence(_)
            | Expr::Attribute { .. }
            | Expr::Occlusion { .. } => false,
            Expr::Hsv { of: a, .. } | Expr::Ramp { input: a, .. } => a.varies_by_pattern(),
            Expr::Planar { of: a, .. }
            | Expr::Wrap { of: a, .. }
            | Expr::Probe { of: a, .. }
            | Expr::Tile(a, _)
            | Expr::Inside(a, _)
            | Expr::BiasGain(a, _, _)
            | Expr::FaceWeight { normal: a, .. }
            | Expr::Facing(a, _)
            | Expr::Falloff { of: a, .. }
            | Expr::Luma(a)
            | Expr::Channel(a, _)
            | Expr::Gamma(a, _) => a.varies_by_pattern(),
            Expr::HsvVary { color, random, .. } => {
                color.varies_by_pattern() || random.varies_by_pattern()
            }
            Expr::Mix { fac, a, b, .. } => {
                fac.varies_by_pattern() || a.varies_by_pattern() || b.varies_by_pattern()
            }
        }
    }
    /// Images this value reads.
    pub fn images(&self, out: &mut Vec<u64>) {
        match self {
            Expr::Const(_)
            | Expr::Locator(_)
            | Expr::MeshUv { .. }
            | Expr::Normal(_)
            | Expr::Geometry(_)
            | Expr::Attribute { .. }
            | Expr::Occlusion { .. }
            | Expr::LocatorIncidence(_)
            | Expr::DistanceTo(_) => {}
            Expr::Hsv { of, .. } => of.images(out),
            Expr::Ramp { input, .. } => input.images(out),
            Expr::Wrap { of: a, .. }
            | Expr::Probe { of: a, .. }
            | Expr::FaceWeight { normal: a, .. }
            | Expr::Facing(a, _)
            | Expr::Falloff { of: a, .. } => a.images(out),
            Expr::ImageColor(l) | Expr::ImageAlpha(l) => {
                if !out.contains(&l.image) {
                    out.push(l.image)
                }
                l.vector.images(out);
            }
            Expr::Planar { of: a, .. }
            | Expr::Tile(a, _)
            | Expr::Inside(a, _)
            | Expr::BiasGain(a, _, _) => a.images(out),
            Expr::Pattern(p) | Expr::CellRandom(p) => p.vector.images(out),
            Expr::HsvVary { color, random, .. } => {
                color.images(out);
                random.images(out);
            }
            Expr::Luma(a) | Expr::Channel(a, _) | Expr::Gamma(a, _) => a.images(out),
            Expr::Mix { fac, a, b, .. } => {
                fac.images(out);
                a.images(out);
                b.images(out);
            }
            Expr::Func(c) => c.args.iter().for_each(|a| a.images(out)),
            Expr::Nodes(n) => n.lowered.images(out),
        }
    }
}

/// A lookup's raw coordinates after an image extension, `None` where a
/// Reset (clip) image is transparent black: for `eval_at` samplers.
pub fn extended(uv: [f64; 2], extension: Tiling) -> Option<[f64; 2]> {
    crate::texture_locator::tile_uv(uv, [extension; 2])
}

fn luma(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn gamma(v: f64, power: f64) -> f64 {
    if v > 0. { v.powf(power) } else { v }
}

/// A Modo-style layer blend of `layer` over `below` at `coverage` (§8 item
/// 2), in the renderer's mix modes.
///
/// Tier 1 G-06: every mode is `lerp(below, f(below, layer), coverage)`.
/// Subtract clamps at 0, Color Dodge at 1 and Color Burn at 0 before the
/// lerp (Cycles' own Dodge/Burn/Subtract fold the coverage in first, so the
/// full-strength result is built and then mixed). Overlay keys on the layer
/// below, Hard Light on the layer. Soft Light and Divide (spec v0.4) differ
/// from Cycles' built-ins, so they are built from plain mixes and a square
/// root (Gamma 0.5), the same expression in the eval fold and in Cycles.
pub fn blend(mode: BlendMode, below: Expr, layer: Expr, coverage: Expr) -> Expr {
    use BlendMode as B;
    let full =
        |m: Mix, below: &Expr, layer: Expr| Expr::mix(m, Expr::scalar(1.), below.clone(), layer);
    let m = match mode {
        B::Subtract => {
            let d = full(Mix::Subtract, &below, layer);
            let d = Expr::mix(Mix::Lighten, Expr::scalar(1.), d, Expr::scalar(0.));
            return Expr::mix(Mix::Blend, coverage, below, d);
        }
        // Cycles' Dodge at full strength is min(B / (1 - L), 1) and its
        // Burn max(1 - (1 - B) / L, 0), clamped.
        B::ColorDodge => {
            let d = full(Mix::Dodge, &below, layer);
            return Expr::mix(Mix::Blend, coverage, below, d);
        }
        B::ColorBurn => {
            let d = full(Mix::Burn, &below, layer);
            return Expr::mix(Mix::Blend, coverage, below, d);
        }
        // B + (2L - 1) D, with D = B (1 - B) for L up to 0.5 and
        // √B - B above. Only one of min(2L - 1, 0) and max(2L - 1, 0) is
        // not zero, so no per-channel test is needed.
        B::SoftLight => {
            let one = || Expr::scalar(1.);
            let zero = || Expr::scalar(0.);
            let t = Expr::mix(
                Mix::Subtract,
                one(),
                Expr::times(layer, Expr::scalar(2.)),
                one(),
            );
            let low = Expr::mix(Mix::Darken, one(), t.clone(), zero());
            let high = Expr::mix(Mix::Lighten, one(), t, zero());
            let d_low = Expr::times(below.clone(), Expr::one_minus(below.clone()));
            let d_high = Expr::mix(
                Mix::Subtract,
                one(),
                Expr::gamma(below.clone(), 0.5),
                below.clone(),
            );
            let soft = Expr::plus(
                below.clone(),
                Expr::plus(Expr::times(low, d_low), Expr::times(high, d_high)),
            );
            return Expr::mix(Mix::Blend, coverage, below, soft);
        }
        // (1 - B + 2L) / (3 (1 + B)), fitted to every measured point.
        B::Divide => {
            let top = Expr::plus(
                Expr::one_minus(below.clone()),
                Expr::times(layer, Expr::scalar(2.)),
            );
            let bottom = Expr::times(
                Expr::plus(below.clone(), Expr::scalar(1.)),
                Expr::scalar(3.),
            );
            let d = Expr::mix(Mix::Divide, Expr::scalar(1.), top, bottom);
            return Expr::mix(Mix::Blend, coverage, below, d);
        }
        B::Normal => Mix::Blend,
        B::Add => Mix::Add,
        B::Difference => Mix::Difference,
        B::Multiply => Mix::Multiply,
        B::Screen => Mix::Screen,
        B::Overlay => Mix::Overlay,
        B::Lighten => Mix::Lighten,
        B::Darken => Mix::Darken,
        // Hard Light is Overlay with the two sides swapped.
        B::HardLight => {
            let hard = Expr::mix(Mix::Overlay, Expr::scalar(1.), layer, below.clone());
            return Expr::mix(Mix::Blend, coverage, below, hard);
        }
        // Normal-map blends: Normal for now (Q-12, ST-Q22).
        B::NormalMultiply | B::NormalMapBlend => Mix::Blend,
    };
    Expr::mix(m, coverage, below, layer)
}

/// The polygon a surface is evaluated for.
#[derive(Clone, Debug, PartialEq)]
pub struct Context<'a> {
    pub item: u64,
    /// The item and every item group above it.
    pub ancestors: &'a [u64],
    pub material: &'a str,
    pub part: Option<&'a str>,
    /// Every selection set the polygon is in (G-01).
    pub sets: &'a [&'a str],
    /// The Texture Locators as seen from this item
    /// (`Scene::locator_frames`). A layer whose locator isn't here renders
    /// as UV Map with repeat 1.
    pub locators: &'a [LocatorFrame],
}

/// Where a texture layer lands on the surface (texture_locator.rs).
#[derive(Clone, Debug, PartialEq)]
pub enum Placement {
    /// Solid: a 3D locator-space point. Images show nothing; procedurals
    /// read the point, their 2D patterns in the plane perpendicular to
    /// `axis` (the Projection Axis).
    Solid { point: Expr, axis: u8 },
    /// One 2D coordinate vector (Planar, Cylindrical, Spherical, UV Map):
    /// `image` for images (centred, repeated; tiled per axis), `procedural`
    /// for procedurals (depth 0, no repeat or tiling).
    Coords {
        image: Expr,
        procedural: Expr,
        tiling: [Tiling; 2],
    },
    /// Several planar faces mixed by weights (Cubic, Box, Triplanar).
    Faces {
        faces: Vec<Face>,
        tiling: [Tiling; 2],
    },
}

/// How one face reads an image: vector, extension, weight and Reset
/// coverage.
type FaceLookup = (Expr, Tiling, Expr, Option<Expr>);

/// One face of a Cubic, Box or Triplanar projection.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub image: Expr,
    pub procedural: Expr,
    pub weight: Expr,
}

/// A 4×4 matrix's top three rows, row-major.
fn rows3x4(m: &glam::DMat4) -> [f64; 12] {
    let mut out = [0.; 12];
    for r in 0..3 {
        out[r * 4..r * 4 + 4].copy_from_slice(&m.row(r).to_array());
    }
    out
}
fn rows3x3(m: &glam::DMat3) -> [f64; 9] {
    let mut out = [0.; 9];
    for r in 0..3 {
        out[r * 3..r * 3 + 3].copy_from_slice(&m.row(r).to_array());
    }
    out
}

fn planar_expr(of: &Expr, axis: u8, mirror: u8, centre: f64, repeat: [f64; 2]) -> Expr {
    Expr::Planar {
        of: Box::new(of.clone()),
        axis,
        centre,
        repeat,
        mirror,
    }
}

impl Placement {
    /// A layer's placement for this context.
    pub fn of(t: &TextureLayer, ctx: &Context) -> Placement {
        use crate::shader_tree::Projection as P;
        let Some(frame) = t
            .locator
            .and_then(|id| ctx.locators.iter().find(|f| f.id == id))
        else {
            let uv = Expr::mesh_uv(None, [1., 1.], 0.);
            return Placement::Coords {
                image: uv.clone(),
                procedural: uv,
                tiling: [Tiling::Repeat; 2],
            };
        };
        let l = &frame.settings;
        let axis = l.axis.min(2);
        let locator = Expr::Locator(Box::new(rows3x4(&frame.world_to_locator)));
        let normal = || Expr::Normal(Box::new(rows3x3(&frame.normal_to_locator)));
        let faces = |of: &Expr, n: Expr, list: &[(u8, u8, u8)], kind: FaceKind| Placement::Faces {
            faces: list
                .iter()
                .map(|&(axis, mirror, face)| Face {
                    image: planar_expr(of, axis, mirror, 0.5, l.repeat),
                    procedural: planar_expr(of, axis, mirror, 0., [1., 1.]),
                    weight: Expr::FaceWeight {
                        normal: Box::new(n.clone()),
                        face,
                        kind,
                    },
                })
                .collect(),
            tiling: l.tiling,
        };
        match l.rendered_projection() {
            P::Solid => Placement::Solid {
                point: locator,
                axis,
            },
            P::Planar => {
                let mut image = planar_expr(&locator, axis, 0, 0.5, l.repeat);
                let mut procedural = planar_expr(&locator, axis, 0, 0., [1., 1.]);
                use crate::texture_locator::FrontCulling as C;
                let flip = match l.front_culling {
                    C::FlipBackfacing => 3,
                    C::FlipUBackfacing => 1,
                    C::FlipVBackfacing => 2,
                    C::ProjectThrough | C::CullBackfacing => 0,
                };
                if flip != 0 {
                    // Back faces read the projection mirrored.
                    let back = Expr::one_minus(Expr::Facing(Box::new(normal()), axis));
                    image = Expr::mix(
                        Mix::Blend,
                        back.clone(),
                        image,
                        planar_expr(&locator, axis, flip, 0.5, l.repeat),
                    );
                    procedural = Expr::mix(
                        Mix::Blend,
                        back,
                        procedural,
                        planar_expr(&locator, axis, flip, 0., [1., 1.]),
                    );
                }
                Placement::Coords {
                    image,
                    procedural,
                    tiling: l.tiling,
                }
            }
            p @ (P::Cylindrical | P::Spherical) => {
                let wrap = |centre, repeat| Expr::Wrap {
                    of: Box::new(locator.clone()),
                    axis,
                    spherical: p == P::Spherical,
                    centre,
                    repeat,
                };
                Placement::Coords {
                    image: wrap(0.5, l.repeat),
                    procedural: wrap(0., [1., 1.]),
                    tiling: l.tiling,
                }
            }
            P::Cubic => faces(
                &locator,
                normal(),
                &[(0, 0, 0), (1, 0, 1), (2, 0, 2)],
                FaceKind::Dominant,
            ),
            P::Triplanar => faces(
                &locator,
                normal(),
                &[(0, 0, 0), (1, 0, 1), (2, 0, 2)],
                FaceKind::Power(l.triplanar_exponent()),
            ),
            P::Box => {
                // World aligned: the locator's position and Size only; each
                // negative face mirrors u so it reads right way round.
                let boxed = Expr::Locator(Box::new(rows3x4(&frame.world_to_box)));
                let n = Expr::Normal(Box::new(rows3x3(&frame.normal_to_space)));
                faces(
                    &boxed,
                    n,
                    &[
                        (0, 0, 0),
                        (1, 0, 1),
                        (2, 0, 2),
                        (0, 1, 3),
                        (1, 1, 4),
                        (2, 1, 5),
                    ],
                    FaceKind::Signed,
                )
            }
            P::LightProbe => Placement::Coords {
                image: Expr::Probe {
                    of: Box::new(locator.clone()),
                    axis,
                    repeat: l.repeat,
                },
                procedural: Expr::Probe {
                    of: Box::new(locator),
                    axis,
                    repeat: [1., 1.],
                },
                tiling: l.tiling,
            },
            P::Front if frame.front.is_some() => {
                // (u·w, v·w, w) / w, then Planar Z keeps (u, v) × repeat.
                let m = frame.front.as_ref().map_or([0.; 12], rows3x4);
                let h = Expr::Locator(Box::new(m));
                let uvw = Expr::mix(
                    Mix::Divide,
                    Expr::scalar(1.),
                    h.clone(),
                    Expr::channel(h, 2),
                );
                Placement::Coords {
                    image: planar_expr(&uvw, 2, 0, 0., l.repeat),
                    procedural: planar_expr(&uvw, 2, 0, 0., [1., 1.]),
                    tiling: l.tiling,
                }
            }
            P::ImplicitUv => {
                let map = Some(crate::texture_locator::IMPLICIT_UV_MAP.to_owned());
                Placement::Coords {
                    image: Expr::mesh_uv(map.clone(), l.repeat, l.uv_rotation),
                    procedural: Expr::mesh_uv(map, l.repeat, 0.),
                    tiling: l.tiling,
                }
            }
            // UV Map, and Front without a usable camera.
            _ => Placement::Coords {
                image: Expr::mesh_uv(l.uv_map.clone(), l.repeat, l.uv_rotation),
                procedural: Expr::mesh_uv(l.uv_map.clone(), l.repeat, 0.),
                tiling: l.tiling,
            },
        }
    }
    /// The coordinates a procedural texture reads (the first face's for
    /// Cubic, Box and Triplanar).
    pub fn procedural_vector(&self) -> Expr {
        self.procedural_coords()
            .into_iter()
            .next()
            .map(|(v, _, _)| v)
            .unwrap_or(Expr::scalar(0.))
    }
    /// What a procedural reads: (vector, the axis 2D patterns are
    /// perpendicular to, weight) per face. Solid gives L with the
    /// Projection Axis; the flat projections give depth-0 vectors (axis Z).
    /// Tiling and UV Rotation don't apply to procedurals.
    pub fn procedural_coords(&self) -> Vec<(Expr, u8, Expr)> {
        match self {
            Placement::Solid { point, axis } => vec![(point.clone(), *axis, Expr::scalar(1.))],
            Placement::Coords { procedural, .. } => vec![(procedural.clone(), 2, Expr::scalar(1.))],
            Placement::Faces { faces, .. } => faces
                .iter()
                .map(|f| (f.procedural.clone(), 2, f.weight.clone()))
                .collect(),
        }
    }
    /// How an image is read here: per face its vector, extension, weight
    /// and Reset coverage. `None` for Solid.
    fn image_lookups(&self) -> Option<Vec<FaceLookup>> {
        let (list, tiling): (Vec<(Expr, Expr)>, [Tiling; 2]) = match self {
            Placement::Solid { .. } => return None,
            Placement::Coords { image, tiling, .. } => {
                (vec![(image.clone(), Expr::scalar(1.))], *tiling)
            }
            Placement::Faces { faces, tiling } => (
                faces
                    .iter()
                    .map(|f| (f.image.clone(), f.weight.clone()))
                    .collect(),
                *tiling,
            ),
        };
        let clip = tiling.map(|t| t == Tiling::Reset);
        Some(
            list.into_iter()
                .map(|(vector, weight)| {
                    let reset = clip
                        .contains(&true)
                        .then(|| Expr::Inside(Box::new(vector.clone()), clip));
                    // One mode on both axes is the image's own extension;
                    // mixed modes tile the vector first (Reset clips, so
                    // the image clips too).
                    let (vector, ext) = if tiling[0] == tiling[1] {
                        (vector, tiling[0])
                    } else {
                        let ext = if clip.contains(&true) {
                            Tiling::Reset
                        } else {
                            Tiling::Edge
                        };
                        (Expr::Tile(Box::new(vector), tiling), ext)
                    };
                    (vector, ext, weight, reset)
                })
                .collect(),
        )
    }
}

/// The locator's coverage besides the pattern: Falloff, and Front
/// Culling's Cull Backfacing (Planar). `None` when neither applies.
fn locator_mask(t: &TextureLayer, ctx: &Context) -> Option<Expr> {
    let frame = t
        .locator
        .and_then(|id| ctx.locators.iter().find(|f| f.id == id))?;
    let l = &frame.settings;
    let locator = || Expr::Locator(Box::new(rows3x4(&frame.world_to_locator)));
    let mut mask: Option<Expr> = None;
    if falloff_on(l.falloff_type, l.falloff) {
        mask = Some(Expr::Falloff {
            of: Box::new(locator()),
            kind: l.falloff_type,
            amount: l.falloff,
        });
    }
    if l.front_culling == crate::texture_locator::FrontCulling::CullBackfacing
        && l.rendered_projection() == crate::shader_tree::Projection::Planar
    {
        let facing = Expr::Facing(
            Box::new(Expr::Normal(Box::new(rows3x3(&frame.normal_to_locator)))),
            l.axis.min(2),
        );
        mask = Some(match mask {
            Some(m) => Expr::times(m, facing),
            None => facing,
        });
    }
    mask
}

/// Σ weight × value, folding a single full-weight term to the value.
fn weighted_sum(terms: Vec<(Expr, Expr)>) -> Expr {
    let mut out: Option<Expr> = None;
    for (weight, value) in terms {
        let term = Expr::times(weight, value);
        out = Some(match out {
            Some(acc) => Expr::plus(acc, term),
            None => term,
        });
    }
    out.unwrap_or(Expr::scalar(0.))
}

/// The folded channels plus the shader that won (§8).
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    pub channels: Vec<Expr>,
    pub shader: ShaderChannels,
    /// A Layer Mask sitting directly under the winning Shader (§8 item 4):
    /// it masks the whole surface.
    pub shader_mask: Option<Expr>,
    /// The topmost applied Material's settings that aren't channels
    /// (provisional: a material layer's shading model and Match Specular
    /// replace the ones below it, whatever its opacity).
    pub shading_model: ShadingModel,
    pub match_specular: bool,
    /// The topmost applied Material's Use Refractive Index (group extras §3).
    pub use_refractive_index: bool,
    /// The topmost applied Material's Thin Walled and Subsurface Uses
    /// Diffuse Color (OpenPBR switches, native v41).
    pub thin_walled: bool,
    pub subsurface_uses_diffuse: bool,
    /// The enclosing groups' Scale factors while folding (group extras
    /// §1), in [`ScaleOf`] order; 1 outside any scaled group. Only the UV
    /// and Size factors are read (texture placement); the distances are
    /// scaled on the group's result.
    pub scale: [f64; 6],
    /// The Node Material reaching this context (NM-11), if any.
    pub node: Option<NodeUse>,
}

/// A Node Material over the folded surface (NM-11..13, NM-31): the
/// renderer mixes the graph's surface over the principled one by
/// `coverage`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeUse {
    /// The Node Material layer.
    pub layer: NodeId,
    /// Its graph (`ShaderTree::node_graphs`).
    pub graph: u64,
    /// Opacity × the masks and groups around it (NM-12).
    pub coverage: Expr,
    /// Exposed input values: the layer's overrides, else the graph's
    /// defaults (NM-04).
    pub values: std::collections::BTreeMap<String, Vec<f64>>,
    /// Exposed inputs texture layers above drive (NM-13), by name.
    pub drivers: Vec<(String, Expr)>,
}
impl NodeUse {
    pub fn driver(&self, name: &str) -> Option<&Expr> {
        self.drivers.iter().find(|(n, _)| n == name).map(|(_, e)| e)
    }
}

/// What a group's Scale resizes, as indices into `Surface::scale`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleOf {
    Uv,
    Size,
    BumpAmplitude,
    AbsorptionDistance,
    DisplacementDistance,
    SubsurfaceDistance,
}
impl Surface {
    pub fn get(&self, ch: Ch) -> &Expr {
        &self.channels[ch as usize]
    }
    fn start() -> Self {
        let m = MaterialChannels::default();
        Self {
            channels: Ch::ALL
                .iter()
                .map(|c| Expr::Const(material_value(&m, *c)))
                .collect(),
            shader: ShaderChannels::default(),
            shader_mask: None,
            shading_model: m.shading_model,
            match_specular: m.match_specular,
            use_refractive_index: m.more.use_refractive_index,
            thin_walled: m.openpbr.thin_walled,
            subsurface_uses_diffuse: m.openpbr.subsurface_uses_diffuse,
            scale: [1.; 6],
            node: None,
        }
    }
}

pub(crate) fn mask_matches(mask: &GroupMask, ctx: &Context) -> bool {
    if mask.scope == Scope::Fur {
        return false;
    }
    let item = match mask.item {
        ItemMask::All => true,
        ItemMask::Item(id) => id == ctx.item,
        ItemMask::ItemGroup(id) => id == ctx.item || ctx.ancestors.contains(&id),
    };
    let tag = match &mask.tag {
        TagMask::All => true,
        TagMask::None => false,
        TagMask::Name(name) => match mask.tag_type {
            TagType::Material => {
                name == ctx.material
                    || (crate::material::is_default_name(name)
                        && crate::material::is_default_name(ctx.material))
            }
            TagType::Part => ctx.part.unwrap_or(crate::material::DEFAULT_MATERIAL) == name,
            // A polygon may be in any number of sets; each group tests its
            // own (G-01).
            TagType::SelectionSet => ctx.sets.contains(&name.as_str()),
        },
    };
    item && tag
}

/// A texture's value and alpha for a channel kind, placed by its locator.
/// One texture layer's fold, kept for a Process layer directly above.
struct Folded {
    ch: Ch,
    below: Expr,
    value: Expr,
    coverage: Expr,
    blend: BlendMode,
}

/// What a Gradient may read from the layers below it (GQ-3): the folded
/// surface so far and the channel of the layer's own effect, if any.
#[derive(Clone, Copy)]
struct Below<'a> {
    state: &'a Surface,
    own: Option<Ch>,
    /// The layer's settings free nodes drive (node editing, NE-03), by
    /// `node_view_settings` key.
    drives: &'a [(&'static str, Expr)],
}

impl Below<'_> {
    fn drive(&self, key: &str) -> Option<Expr> {
        self.drives
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, e)| e.clone())
    }
}

impl ShaderTree {
    /// What free nodes drive among texture layer `node`'s settings
    /// (node editing, NE-03); empty for most layers.
    fn texture_drives(&self, node: &crate::shader_tree::Node) -> Vec<(&'static str, Expr)> {
        let NodeKind::Texture(t) = &node.kind else {
            return Vec::new();
        };
        if self.node_views.is_empty() {
            return Vec::new();
        }
        crate::node_view_settings::settings(t)
            .into_iter()
            .filter_map(|(key, _, _, _)| Some((key, self.driven_setting(node.id, key)?)))
            .collect()
    }
}

/// A Gradient's input at the shading point (gradient family §1).
fn gradient_input(
    g: &crate::gradient::Gradient,
    t: &TextureLayer,
    below: Below,
    ctx: &Context,
) -> Expr {
    use crate::gradient::GradientInput as I;
    let ch = |c: Ch| below.state.get(c).clone();
    let scalar = |e: Expr| match e {
        Expr::Const(c) => Expr::scalar(c[0]),
        e => e,
    };
    match g.input {
        I::DiffuseAmount => ch(Ch::DiffuseAmount),
        I::DiffuseRoughness => ch(Ch::DiffuseRoughness),
        I::SpecularAmount => ch(Ch::SpecularAmount),
        I::SpecularFresnel => ch(Ch::Fresnel),
        I::ReflectionAmount => ch(Ch::ReflectionAmount),
        I::ReflectionFresnel => ch(Ch::ReflectionFresnel),
        I::Roughness => ch(Ch::Roughness),
        I::ClearcoatAmount => ch(Ch::ClearcoatAmount),
        I::TransparentAmount => ch(Ch::TransparentAmount),
        I::RefractionRoughness => ch(Ch::RefractionRoughness),
        I::SubsurfaceAmount => ch(Ch::SubsurfaceAmount),
        I::LuminousAmount => ch(Ch::LuminousIntensity),
        I::Dissolve => ch(Ch::Dissolve),
        I::Stencil => ch(Ch::Stencil),
        I::DriverA => ch(Ch::DriverA),
        I::DriverB => ch(Ch::DriverB),
        I::DriverC => ch(Ch::DriverC),
        I::DriverD => ch(Ch::DriverD),
        // The folded value of this layer's own channel below it; colour
        // channels by brightness (GQ-4).
        I::TextureValue => match below.own {
            Some(c) if c.is_color() => Expr::luma(ch(c)),
            Some(c) => scalar(ch(c)),
            None => Expr::scalar(0.),
        },
        I::BumpHeight => Expr::times(ch(Ch::Bump), ch(Ch::BumpAmplitude)),
        I::DisplacementHeight => Expr::times(ch(Ch::Displacement), ch(Ch::DisplacementDistance)),
        I::Incidence => Expr::Geometry(GeometryInput::Incidence),
        I::Slope => Expr::Geometry(GeometryInput::Slope),
        I::DistanceToCamera => Expr::Geometry(GeometryInput::CameraDistance),
        I::BackFacing => Expr::Geometry(GeometryInput::BackFacing),
        I::ScreenX => Expr::Geometry(GeometryInput::ScreenX),
        I::ScreenY => Expr::Geometry(GeometryInput::ScreenY),
        I::ScreenR => Expr::Geometry(GeometryInput::ScreenR),
        // Renders don't put the bump into the normal a Gradient reads, so
        // Bumpless Incident is Incidence (GR-Q10, provisional).
        I::BumplessIncident => Expr::Geometry(GeometryInput::Incidence),
        I::LocatorIncidence => Expr::LocatorIncidence(locator_origin(t, ctx)),
        // The largest of |x|, |y|, |z| in the locator's own space (its Size
        // scales it): a box-shaped distance (GR-Q10, provisional).
        I::XyzDistanceToLocator => {
            let l = t
                .locator
                .and_then(|id| ctx.locators.iter().find(|f| f.id == id))
                .map_or(
                    Expr::Locator(Box::new([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.])),
                    |f| Expr::Locator(Box::new(rows3x4(&f.world_to_locator))),
                );
            let abs = |k: u8| {
                Expr::mix(
                    Mix::Difference,
                    Expr::scalar(1.),
                    Expr::channel(l.clone(), k),
                    Expr::scalar(0.),
                )
            };
            let max = |a, b| Expr::mix(Mix::Lighten, Expr::scalar(1.), a, b);
            max(max(abs(0), abs(1)), abs(2))
        }
        // Metres of material behind the point, up to 1 m: 1 m × the share of
        // rays fired into the surface that leave it within 1 m (GR-Q10,
        // provisional; the viewport reads 1 m).
        I::Thickness => Expr::times(
            Expr::Occlusion {
                rays: 16,
                distance: 1.,
                same_surface: true,
                inside: true,
                toward: AoToward::Normal,
            },
            Expr::scalar(1.),
        ),
        I::DistanceToLocator => Expr::DistanceTo(locator_origin(t, ctx)),
        // The UVs 0 → 1 (no repeat or rotation): the map the layer's UV Map
        // projection names, else the mesh's first map (GR-Q04). A mesh
        // without UVs reads 0, and the render notes say so.
        I::TextureU | I::TextureV => {
            let map = t
                .locator
                .and_then(|id| ctx.locators.iter().find(|f| f.id == id))
                .map(|f| &f.settings)
                .filter(|l| l.rendered_projection() == crate::shader_tree::Projection::UvMap)
                .and_then(|l| l.uv_map.clone());
            let k = if g.input == I::TextureU { 0 } else { 1 };
            Expr::channel(Expr::mesh_uv(map, [1., 1.], 0.), k)
        }
        // Stored only: a constant 0 until built (GR-Q02).
        _ => Expr::scalar(0.),
    }
}

/// A layer's locator origin in world space; the world origin without one.
fn locator_origin(t: &TextureLayer, ctx: &Context) -> [f64; 3] {
    t.locator
        .and_then(|id| ctx.locators.iter().find(|f| f.id == id))
        .map(|f| {
            f.world_to_locator
                .inverse()
                .transform_point3(glam::DVec3::ZERO)
                .to_array()
        })
        .filter(|p| p.iter().all(|v| v.is_finite()))
        .unwrap_or([0.; 3])
}

/// A Gradient's value and alpha: the Value curve for scalar effects, the
/// Color curve and its alpha for colour effects.
fn gradient_value(
    g: &crate::gradient::Gradient,
    t: &TextureLayer,
    color: bool,
    below: Below,
    ctx: &Context,
) -> (Expr, Expr) {
    // NE-03: a wired Input is the number the curves look up.
    let input = below
        .drive("texture.gradient_input")
        .map(|e| Expr::channel(e, 0))
        .unwrap_or_else(|| gradient_input(g, t, below, ctx));
    curve_value(input, &g.value, &g.color, color)
}

/// An attribute's colour or alpha.
fn attribute(name: String, alpha: bool) -> Expr {
    Expr::Attribute { name, alpha }
}

/// A Vertex Map Texture (gradient family §3): a weight through the
/// two-zone block (Invert flips the weight), or a colour map's own colour
/// with its alpha. A mesh without the map exports alpha 0: no effect.
fn vertex_map_value(
    map: &crate::vertex_map_texture::VertexMapRef,
    zones: &crate::shader_tree::TwoZone,
    color: bool,
    invert: bool,
    below: &Below,
) -> (Expr, Expr) {
    use crate::gradient::RAMP_SAMPLES;
    use crate::vertex_map_texture::{VertexMapKind, attribute_name};
    let name = attribute_name(map);
    let present = attribute(name.clone(), true);
    match map.kind {
        VertexMapKind::Color => {
            let c = attribute(name, false);
            let v = if color { c } else { Expr::luma(c) };
            let v = if invert { Expr::one_minus(v) } else { v };
            (v, present)
        }
        VertexMapKind::Weight => {
            let mut w = Expr::channel(attribute(name, false), 0);
            if invert {
                w = Expr::one_minus(w);
            }
            let stops: Vec<[f64; 4]> = (0..RAMP_SAMPLES)
                .map(|k| {
                    let (v, c, a) = zones.sample(k as f64 / (RAMP_SAMPLES - 1) as f64);
                    if color {
                        [c[0], c[1], c[2], a]
                    } else {
                        [v, v, v, a]
                    }
                })
                .collect();
            let alpha_varies = stops.iter().any(|s| s[3] != stops[0][3]);
            let stops = std::sync::Arc::new(stops);
            let ramp = |alpha: bool| Expr::Ramp {
                input: Box::new(w.clone()),
                lo: 0.,
                hi: 1.,
                stops: stops.clone(),
                alpha,
            };
            let zone_alpha = if alpha_varies {
                ramp(true)
            } else {
                Expr::scalar(stops[0][3])
            };
            let value = driven_zones(&w, zones, color, below).unwrap_or_else(|| ramp(false));
            (value, Expr::times(present, zone_alpha))
        }
    }
}

/// NE-03: a two-zone block whose zone colours or values free nodes drive
/// (a weight Vertex Map Texture, an Occlusion): the zone blend of `signal`
/// as a factor ramp, mixing the wired or own zone values. `None` with no
/// wire, so the unwired ramp stays exactly as it was. The zone values are
/// linear in the factor, so this matches the colour ramp when the wired
/// values equal the own ones.
fn driven_zones(
    signal: &Expr,
    zones: &crate::shader_tree::TwoZone,
    color: bool,
    below: &Below,
) -> Option<Expr> {
    use crate::gradient::RAMP_SAMPLES;
    let (k1, k2, own1, own2) = if color {
        (
            "texture.zone1_color",
            "texture.zone2_color",
            zones.zone1_color,
            zones.zone2_color,
        )
    } else {
        (
            "texture.zone1_value",
            "texture.zone2_value",
            [zones.zone1_value; 3],
            [zones.zone2_value; 3],
        )
    };
    let (d1, d2) = (below.drive(k1), below.drive(k2));
    if d1.is_none() && d2.is_none() {
        return None;
    }
    // The factor 0..1 between Zone 1 and Zone 2 after bias and gain.
    let factor = crate::shader_tree::TwoZone {
        zone1_value: 0.,
        zone2_value: 1.,
        ..zones.clone()
    };
    let stops: Vec<[f64; 4]> = (0..RAMP_SAMPLES)
        .map(|k| {
            let (w, _, _) = factor.sample(k as f64 / (RAMP_SAMPLES - 1) as f64);
            [w, w, w, 1.]
        })
        .collect();
    let ramp = Expr::Ramp {
        input: Box::new(signal.clone()),
        lo: 0.,
        hi: 1.,
        stops: std::sync::Arc::new(stops),
        alpha: false,
    };
    Some(Expr::mix(
        Mix::Blend,
        ramp,
        d1.unwrap_or(Expr::Const(own1)),
        d2.unwrap_or(Expr::Const(own2)),
    ))
}

/// A Variation Texture (§4): each element's random number on the curves.
fn variation_value(v: &crate::vertex_map_texture::Variation, color: bool) -> (Expr, Expr) {
    let input = match crate::vertex_map_texture::variation_attribute(v.source) {
        Some(name) => Expr::channel(attribute(name.into(), false), 0),
        // Particles: r = 0 until Operandi has them.
        None => Expr::scalar(0.),
    };
    curve_value(input, &v.value, &v.color, color)
}

/// `input` looked up on the Value curve (scalar effects) or the Color
/// curve and its alpha (colour effects).
fn curve_value(
    input: Expr,
    value: &crate::gradient::Curve<f64>,
    color_curve: &crate::gradient::Curve<[f64; 4]>,
    color: bool,
) -> (Expr, Expr) {
    use crate::gradient::RAMP_SAMPLES;
    let g = (value, color_curve);
    if let Some(x) = input.constant() {
        return if color {
            let c = g.1.sample(x[0]);
            (Expr::Const([c[0], c[1], c[2]]), Expr::scalar(c[3]))
        } else {
            (Expr::scalar(g.0.sample(x[0])), Expr::scalar(1.))
        };
    }
    let ramp = |stops: Vec<[f64; 4]>, lo: f64, hi: f64, alpha: bool| {
        // A curve without span is flat: no ramp.
        if hi <= lo {
            let v = stops[0];
            return if alpha {
                Expr::scalar(v[3])
            } else {
                Expr::Const([v[0], v[1], v[2]])
            };
        }
        Expr::Ramp {
            input: Box::new(input.clone()),
            lo,
            hi,
            stops: std::sync::Arc::new(stops),
            alpha,
        }
    };
    if color {
        let (lo, hi) = g.1.span().unwrap_or((0., 0.));
        let stops = g.1.table(lo, hi, RAMP_SAMPLES);
        let alpha = if stops.iter().all(|s| s[3] == stops[0][3]) {
            Expr::scalar(stops[0][3])
        } else {
            ramp(stops.clone(), lo, hi, true)
        };
        (ramp(stops, lo, hi, false), alpha)
    } else {
        let (lo, hi) = g.0.span().unwrap_or((0., 0.));
        let stops =
            g.0.table(lo, hi, RAMP_SAMPLES)
                .into_iter()
                .map(|v| [v, v, v, 1.])
                .collect();
        (ramp(stops, lo, hi, false), Expr::scalar(1.))
    }
}

fn texture_value(
    t: &TextureLayer,
    color: bool,
    invert: bool,
    ctx: &Context,
    below: Below,
) -> Option<(Expr, Expr)> {
    // Group Scale (group extras §1): the layer's locator as if its Size
    // were × Scale (or, for UV Map, its repeat counts ÷ Scale).
    let scaled: Option<Vec<crate::texture_locator::LocatorFrame>> = (!t.ignore_scale_group)
        .then(|| {
            let id = t.locator?;
            let frame = ctx.locators.iter().find(|f| f.id == id)?;
            let uv = frame.settings.reads_uvs();
            let k = below.state.scale[if uv { ScaleOf::Uv } else { ScaleOf::Size } as usize];
            if k == 1. || k <= 0. || !k.is_finite() {
                return None;
            }
            let mut f = frame.clone();
            if uv {
                f.settings.repeat = f.settings.repeat.map(|r| r / k);
            } else {
                f.world_to_locator =
                    glam::DMat4::from_scale(glam::DVec3::splat(1. / k)) * f.world_to_locator;
            }
            Some(vec![f])
        })
        .flatten();
    let placement = match &scaled {
        Some(frames) => Placement::of(
            t,
            &Context {
                locators: frames,
                ..ctx.clone()
            },
        ),
        None => Placement::of(t, ctx),
    };
    let (value, alpha) = match &t.source {
        TextureSource::Gradient(g) => gradient_value(g, t, color, below, ctx),
        TextureSource::VertexMap { map: None, .. } => return None,
        // Invert is part of the weight lookup here.
        TextureSource::VertexMap {
            map: Some(map),
            zones,
        } => return Some(vertex_map_value(map, zones, color, invert, &below)),
        TextureSource::Variation(v) => variation_value(v, color),
        TextureSource::Occlusion(o) => {
            let (v, a) = o.value(color);
            let v = driven_zones(&o.signal(), &o.zones, color, &below).unwrap_or(v);
            (v, a)
        }
        // Folded with the layer below it, not as a texture of its own.
        TextureSource::Process(_) => return None,
        TextureSource::Constant { color: c, value } => {
            // Reset tiles clip a Constant too.
            let coverage = placement.image_lookups().map(|lookups| {
                weighted_sum(
                    lookups
                        .into_iter()
                        .map(|(_, _, w, reset)| (w, reset.unwrap_or(Expr::scalar(1.))))
                        .collect(),
                )
            });
            let own = Expr::Const(if color { *c } else { [*value; 3] });
            let key = if color {
                "texture.color"
            } else {
                "texture.value"
            };
            (
                below.drive(key).unwrap_or(own),
                coverage.unwrap_or(Expr::scalar(1.)),
            )
        }
        TextureSource::ImageMap {
            image: Some(id),
            settings,
        } => {
            // Solid: an image has no 3D form; the layer is transparent.
            let lookups = placement.image_lookups()?;
            let mut values = Vec::new();
            let mut alphas = Vec::new();
            for (vector, extension, weight, reset) in lookups {
                let (v, a) = image_value(
                    ImageLookup {
                        image: *id,
                        filtering: settings.filtering,
                        vector,
                        extension,
                    },
                    settings,
                    color,
                    &below,
                );
                // Reset: outside the tile the layer is fully transparent.
                let a = match reset {
                    Some(mask) => Expr::times(a, mask),
                    None => a,
                };
                values.push((weight.clone(), v));
                alphas.push((weight, a));
            }
            (weighted_sum(values), weighted_sum(alphas))
        }
        TextureSource::ImageMap { image: None, .. } => return None,
        TextureSource::Procedural(p) => {
            let raw = weighted_sum(
                placement
                    .procedural_coords()
                    .into_iter()
                    .map(|(vector, axis, weight)| {
                        (
                            weight,
                            Expr::Pattern(Box::new(PatternLookup {
                                pattern: p.pattern.clone(),
                                axis,
                                vector,
                            })),
                        )
                    })
                    .collect(),
            );
            let z = &p.zones;
            let t = if let Some((brightness, contrast)) = p.brightness_contrast() {
                // Gabor and Poisson: (t − ½)(1 + contrast) + ½ + brightness,
                // clamped (provisional, PT-Q21).
                if brightness == 0. && contrast == 0. {
                    raw
                } else {
                    let centred = Expr::plus(raw, Expr::scalar(-0.5));
                    let shaped = Expr::plus(
                        Expr::times(centred, Expr::scalar(1. + contrast)),
                        Expr::scalar(0.5 + brightness),
                    );
                    let low = Expr::mix(Mix::Lighten, Expr::scalar(1.), shaped, Expr::scalar(0.));
                    Expr::mix(Mix::Darken, Expr::scalar(1.), low, Expr::scalar(1.))
                }
            } else if z.bias == 0.5 && z.gain == 0.5 {
                raw
            } else {
                Expr::BiasGain(Box::new(raw), z.bias, z.gain)
            };
            let zone = |a: [f64; 3], b: [f64; 3]| {
                Expr::mix(Mix::Blend, t.clone(), Expr::Const(a), Expr::Const(b))
            };
            // NE-03: wired zone colours or values replace their own.
            let driven = |key: &str, own: [f64; 3]| below.drive(key).unwrap_or(Expr::Const(own));
            let mut value = if color {
                Expr::mix(
                    Mix::Blend,
                    t.clone(),
                    driven("texture.zone1_color", z.zone1_color),
                    driven("texture.zone2_color", z.zone2_color),
                )
            } else {
                Expr::mix(
                    Mix::Blend,
                    t.clone(),
                    driven("texture.zone1_value", [z.zone1_value; 3]),
                    driven("texture.zone2_value", [z.zone2_value; 3]),
                )
            };
            // Tiling doesn't apply to procedurals.
            let mut alpha = zone([z.zone1_alpha; 3], [z.zone2_alpha; 3]);
            // Per-cell variations: brightness, hue and saturation for colour
            // effects; alpha for any.
            if let Some(var) = p.variation() {
                let amounts = [var.value, var.hue, var.saturation];
                let random = || {
                    weighted_sum(
                        placement
                            .procedural_coords()
                            .into_iter()
                            .map(|(vector, axis, weight)| {
                                (
                                    weight,
                                    Expr::CellRandom(Box::new(PatternLookup {
                                        pattern: p.pattern.clone(),
                                        axis,
                                        vector,
                                    })),
                                )
                            })
                            .collect(),
                    )
                };
                if color && amounts.iter().any(|&a| a > 0.) {
                    value = Expr::HsvVary {
                        color: Box::new(value),
                        random: Box::new(random()),
                        amounts,
                    };
                }
                if var.alpha > 0. {
                    // Provisional (PT-Q15): the brightness number drives it.
                    let fade = Expr::one_minus(Expr::times(
                        Expr::scalar(var.alpha),
                        Expr::channel(random(), 0),
                    ));
                    alpha = Expr::times(alpha, fade);
                }
            }
            (value, alpha)
        }
    };
    let alpha = match locator_mask(t, ctx) {
        Some(mask) => Expr::times(alpha, mask),
        None => alpha,
    };
    let value = if invert {
        Expr::one_minus(value)
    } else {
        value
    };
    Some((value, alpha))
}

/// An Image Map's value and coverage after its adjustments (G-05), in
/// this order (provisional, ST-Q14): swizzle, alpha mode, per-channel
/// invert, gamma, brightness, contrast, clamp; then, for scalar effects,
/// the luminance mapped from Low (black) to High (white).
fn image_value(lookup: ImageLookup, s: &ImageSettings, color: bool, below: &Below) -> (Expr, Expr) {
    let rgb = Expr::ImageColor(Box::new(lookup.clone()));
    let image_alpha = Expr::ImageAlpha(Box::new(lookup));
    let mut c = match s.swizzle {
        Swizzle::Rgba | Swizzle::Rgb => rgb,
        Swizzle::Alpha => image_alpha.clone(),
        Swizzle::Red => Expr::channel(rgb, 0),
        Swizzle::Green => Expr::channel(rgb, 1),
        Swizzle::Blue => Expr::channel(rgb, 2),
    };
    let alpha = match s.alpha {
        // RGB swizzling leaves the image's alpha out.
        ImageAlpha::Use if s.swizzle == Swizzle::Rgb => Expr::scalar(1.),
        ImageAlpha::Use => image_alpha,
        ImageAlpha::Ignore => Expr::scalar(1.),
        // The alpha becomes the layer's value; the layer covers fully.
        ImageAlpha::AlphaOnly => {
            c = image_alpha;
            Expr::scalar(1.)
        }
    };
    let flips = [s.invert_red, s.invert_green, s.invert_blue];
    if flips.iter().all(|&f| f) {
        c = Expr::one_minus(c);
    } else if flips.iter().any(|&f| f) {
        // c + m (1 - 2c), m = 1 on the inverted channels.
        let m = Expr::Const(flips.map(|f| if f { 1. } else { 0. }));
        let away = Expr::mix(
            Mix::Subtract,
            Expr::scalar(1.),
            Expr::one_minus(c.clone()),
            c.clone(),
        );
        c = Expr::plus(c, Expr::times(m, away));
    }
    // NE-03: Gamma, Brightness and Contrast may come from free nodes.
    let gamma = below.drive("texture.gamma");
    let brightness = below.drive("texture.brightness");
    let contrast = below.drive("texture.contrast");
    // Gamma brightens above 1, as a display gamma does.
    if let Some(g) = gamma {
        use crate::shader_func::{Func, MathOp};
        let math = |op| Func::Math { op, clamp: false };
        let power = Expr::func(
            math(MathOp::Divide),
            vec![Expr::scalar(1.), Expr::channel(g, 0), Expr::scalar(0.)],
        );
        c = Expr::func(Func::Gamma, vec![c, power]);
    } else if s.gamma > 0. {
        c = Expr::gamma(c, 1. / s.gamma);
    }
    if let Some(b) = brightness {
        c = Expr::times(c, b);
    } else if s.brightness != 1. {
        c = Expr::times(c, Expr::scalar(s.brightness));
    }
    if contrast.is_some() || s.contrast != 1. {
        let k = contrast.unwrap_or(Expr::scalar(s.contrast));
        let centred = Expr::mix(Mix::Subtract, Expr::scalar(1.), c, Expr::scalar(0.5));
        c = Expr::plus(Expr::times(centred, k), Expr::scalar(0.5));
    }
    if s.clamp {
        let low = Expr::mix(Mix::Lighten, Expr::scalar(1.), c, Expr::scalar(0.));
        c = Expr::mix(Mix::Darken, Expr::scalar(1.), low, Expr::scalar(1.));
    }
    if color {
        return (c, alpha);
    }
    let mut v = Expr::luma(c);
    let low = below.drive("texture.low_value");
    let high = below.drive("texture.high_value");
    if low.is_some() || high.is_some() {
        // NE-03: black maps to the wired or own Low, white to High.
        let low = low.unwrap_or(Expr::scalar(s.low_value));
        let high = high.unwrap_or(Expr::scalar(s.high_value));
        let span = Expr::mix(Mix::Subtract, Expr::scalar(1.), high, low.clone());
        v = Expr::plus(Expr::times(v, span), low);
    } else if s.low_value != 0. || s.high_value != 1. {
        v = Expr::plus(
            Expr::times(v, Expr::scalar(s.high_value - s.low_value)),
            Expr::scalar(s.low_value),
        );
    }
    (v, alpha)
}

impl ShaderTree {
    /// Folds the whole tree for one polygon context (§8).
    pub fn evaluate(&self, ctx: &Context) -> Surface {
        let above = self.above_winning_shader(ctx);
        self.fold(None, Surface::start(), ctx, &above)
    }

    /// Whether layer `id` reaches a polygon: every Material Group around
    /// it matches the context (Auto Size uses this).
    pub fn layer_applies(&self, id: NodeId, ctx: &Context) -> bool {
        let mut at = self.get(id).and_then(|n| n.parent);
        for _ in 0..=self.nodes.len() {
            let Some(node) = at.and_then(|a| self.get(a)) else {
                return true;
            };
            if let NodeKind::Group(g) = &node.kind
                && !mask_matches(g, ctx)
            {
                return false;
            }
            at = node.parent;
        }
        false
    }

    /// Tier 1 G-19: Material and texture layers above the topmost
    /// applicable Shader (in panel order) don't surface the polygon; only
    /// layers below it count. Masks keep working wherever they sit, and
    /// Groups/Folders still blend what they hold below the shader.
    fn above_winning_shader(&self, ctx: &Context) -> std::collections::HashSet<NodeId> {
        let rows = self.rows(&|_| false);
        // A shader applies when it and every layer around it are enabled
        // and every Material Group around it matches.
        let applies = |id: NodeId| {
            let mut at = Some(id);
            for _ in 0..=self.nodes.len() {
                let Some(node) = at.and_then(|a| self.get(a)) else {
                    return true;
                };
                if !node.layer.enable {
                    return false;
                }
                if let NodeKind::Group(g) = &node.kind
                    && !mask_matches(g, ctx)
                {
                    return false;
                }
                at = node.parent;
            }
            false
        };
        let Some(top) = rows
            .iter()
            .position(|(n, _)| matches!(n.kind, NodeKind::Shader(_)) && applies(n.id))
        else {
            return Default::default();
        };
        rows[..top]
            .iter()
            .filter(|(n, _)| match &n.kind {
                NodeKind::Material(_) | NodeKind::NodeMaterial(_) => true,
                NodeKind::Texture(t) => !matches!(
                    t.effect,
                    TextureEffect::LayerMask | TextureEffect::GroupMask
                ),
                _ => false,
            })
            .map(|(n, _)| n.id)
            .collect()
    }

    /// Folds `parent`'s children bottom to top onto `state`, leaving out
    /// the layers in `skip`.
    fn fold(
        &self,
        parent: Option<NodeId>,
        mut state: Surface,
        ctx: &Context,
        skip: &std::collections::HashSet<NodeId>,
    ) -> Surface {
        let children: Vec<_> = self.children(parent).collect();
        // A Layer Mask waiting for the layer directly above it.
        let mut pending: Option<Expr> = None;
        // The texture layer just folded, for a Process layer above it.
        let mut last: Option<Folded> = None;
        for node in children.into_iter().rev() {
            if !node.layer.enable {
                continue;
            }
            let below_layer = last.take();
            let drives = self.texture_drives(node);
            let layer = &node.layer;
            let mask = pending.take();
            if skip.contains(&node.id) {
                continue;
            }
            let coverage = |extra: Expr| {
                let c = Expr::times(Expr::scalar(layer.opacity), extra);
                match &mask {
                    Some(m) => Expr::times(c, m.clone()),
                    None => c,
                }
            };
            match &node.kind {
                NodeKind::RenderOutput(_) => {}
                // Process (process / occlusion §1): redo the texture layer
                // directly below with its value adjusted, then blend that
                // over the unadjusted result with the Process layer's own
                // mode, opacity and invert. Nothing below: nothing (PQ-3).
                NodeKind::Texture(TextureLayer {
                    source: TextureSource::Process(p),
                    effect,
                    ..
                }) => {
                    let (Some(prev), Some(ch)) = (below_layer, Ch::of_effect(*effect)) else {
                        continue;
                    };
                    if prev.ch != ch {
                        continue;
                    }
                    let adjusted = blend(prev.blend, prev.below, p.expr(prev.value), prev.coverage);
                    let adjusted = if layer.invert {
                        Expr::one_minus(adjusted)
                    } else {
                        adjusted
                    };
                    let i = ch as usize;
                    let unadjusted = std::mem::replace(&mut state.channels[i], Expr::scalar(0.));
                    state.channels[i] = blend(
                        layer.blend,
                        unadjusted,
                        adjusted,
                        coverage(Expr::scalar(1.)),
                    );
                }
                // NM-13: a texture layer driving a Node Material's exposed
                // input below it; nothing below with that input: nothing.
                NodeKind::Texture(t) if t.node_input.is_some() => {
                    let name = t.node_input.as_deref().map_or("", String::as_str);
                    let Some(kind) = state.node.as_ref().and_then(|n| {
                        self.node_graphs
                            .get(n.graph)
                            .and_then(|g| g.exposed(name))
                            .map(|e| e.value_kind())
                    }) else {
                        continue;
                    };
                    let color = kind != crate::node_graph::ValueKind::Float;
                    let below = Below {
                        state: &state,
                        own: None,
                        drives: &drives,
                    };
                    let Some((v, a)) = texture_value(t, color, layer.invert, ctx, below) else {
                        continue;
                    };
                    let cov = coverage(a);
                    let node = state.node.as_mut().expect("checked");
                    let under = node.driver(name).cloned().unwrap_or_else(|| {
                        let v = kind.fit(node.values.get(name).map_or(&[][..], |v| v));
                        if color {
                            Expr::Const([v[0], v[1], v[2]])
                        } else {
                            Expr::scalar(v[0])
                        }
                    });
                    let value = blend(layer.blend, under, v, cov);
                    node.drivers.retain(|(n, _)| n != name);
                    node.drivers.push((name.to_string(), value));
                }
                NodeKind::NodeMaterial(m) => {
                    let Some(g) = m.graph.and_then(|g| self.node_graphs.get(g)) else {
                        continue;
                    };
                    // Unknown nodes (NM-03) don't render: the layers below show.
                    if !g.unsupported().is_empty() {
                        continue;
                    }
                    // Only the topmost Node Material in a context renders
                    // (provisional, NM-Q05).
                    state.node = Some(NodeUse {
                        layer: node.id,
                        graph: g.id,
                        coverage: coverage(Expr::scalar(1.)),
                        values: g
                            .material
                            .exposed
                            .iter()
                            .map(|e| (e.name.clone(), ShaderTree::exposed_value(m, e)))
                            .collect(),
                        drivers: Vec::new(),
                    });
                }
                NodeKind::Texture(t) => match t.effect {
                    TextureEffect::LayerMask => {
                        let below = Below {
                            state: &state,
                            own: None,
                            drives: &drives,
                        };
                        if let Some((v, a)) = texture_value(t, false, layer.invert, ctx, below) {
                            // The mask's own opacity fades it toward "no mask" (Q-11).
                            let fade = Expr::times(Expr::scalar(layer.opacity), a);
                            pending = Some(Expr::mix(Mix::Blend, fade, Expr::scalar(1.), v));
                        }
                    }
                    // Folded by the enclosing group (§8 item 6).
                    TextureEffect::GroupMask => {}
                    // Cut-out effects v1: Diffuse Color from the layer's
                    // colour, as a Diffuse Color layer would fold it, and a
                    // hard cut where its alpha is below 50 % (SQ-1).
                    TextureEffect::Rgba => {
                        let below = Below {
                            state: &state,
                            own: Some(Ch::DiffuseColor),
                            drives: &drives,
                        };
                        let Some((v, a)) = texture_value(t, true, layer.invert, ctx, below) else {
                            continue;
                        };
                        let i = Ch::DiffuseColor as usize;
                        let below = std::mem::replace(&mut state.channels[i], Expr::scalar(0.));
                        state.channels[i] = blend(layer.blend, below, v, coverage(a.clone()));
                        let i = Ch::RgbaCut as usize;
                        let below = std::mem::replace(&mut state.channels[i], Expr::scalar(0.));
                        state.channels[i] = blend(
                            BlendMode::Normal,
                            below,
                            Expr::step_below(a, 0.5),
                            coverage(Expr::scalar(1.)),
                        );
                    }
                    effect => {
                        let Some(ch) = Ch::of_effect(effect) else {
                            continue;
                        };
                        let below = Below {
                            state: &state,
                            own: Some(ch),
                            drives: &drives,
                        };
                        let Some((v, a)) =
                            texture_value(t, ch.is_color(), layer.invert, ctx, below)
                        else {
                            continue;
                        };
                        let i = ch as usize;
                        let below = std::mem::replace(&mut state.channels[i], Expr::scalar(0.));
                        let cov = coverage(a);
                        last = Some(Folded {
                            ch,
                            below: below.clone(),
                            value: v.clone(),
                            coverage: cov.clone(),
                            blend: layer.blend,
                        });
                        state.channels[i] = blend(layer.blend, below, v, cov);
                    }
                },
                NodeKind::Material(m) => {
                    let c = coverage(Expr::scalar(1.));
                    for ch in Ch::ALL {
                        if ch.is_texture_only() {
                            continue;
                        }
                        let i = ch as usize;
                        let below = std::mem::replace(&mut state.channels[i], Expr::scalar(0.));
                        // NE-03: a wire from a free node drives the channel;
                        // the layer's own value stays stored.
                        let value = self
                            .driven_value(node.id, ch)
                            .unwrap_or_else(|| Expr::Const(material_value(m, ch)));
                        state.channels[i] = blend(layer.blend, below, value, c.clone());
                    }
                    state.shading_model = m.shading_model;
                    state.match_specular = m.match_specular;
                    state.use_refractive_index = m.more.use_refractive_index;
                    state.thin_walled = m.openpbr.thin_walled;
                    state.subsurface_uses_diffuse = m.openpbr.subsurface_uses_diffuse;
                }
                NodeKind::Shader(s) => {
                    // Topmost applicable shader wins (§8 item 8), section
                    // by section: a switched-off section comes from the
                    // shader below (SH-1).
                    state.shader = s.over(&state.shader);
                    state.shader_mask = mask;
                }
                NodeKind::Group(_) | NodeKind::Folder => {
                    if let NodeKind::Group(g) = &node.kind
                        && !mask_matches(g, ctx)
                    {
                        continue;
                    }
                    // Apply to Sub-Group (group extras §2): an untagged group
                    // applies only where a tagged group inside it matches.
                    if let NodeKind::Group(g) = &node.kind
                        && g.apply_to_sub_group
                        && g.is_untagged()
                        && !self.tagged_group_inside_matches(node.id, ctx)
                    {
                        continue;
                    }
                    let group_mask = self.group_mask(node.id, ctx, &state);
                    let mut start = state.clone();
                    if let NodeKind::Group(g) = &node.kind {
                        // Nested Scales multiply (XQ-2).
                        let sc = &g.scale;
                        let on = [
                            sc.uv,
                            sc.size,
                            sc.bump_amplitude,
                            sc.absorption_distance,
                            sc.displacement_distance,
                            sc.subsurface_distance,
                        ];
                        for (k, on) in on.into_iter().enumerate() {
                            if on {
                                start.scale[k] *= sc.scale;
                            }
                        }
                    }
                    let mut inner = self.fold(Some(node.id), start, ctx, skip);
                    // The material distances of the surfaces in the group
                    // (group extras §1), wherever their material sits.
                    if let NodeKind::Group(g) = &node.kind
                        && g.scale.scale != 1.
                    {
                        let sc = &g.scale;
                        for (on, ch) in [
                            (sc.bump_amplitude, Ch::BumpAmplitude),
                            (sc.absorption_distance, Ch::AbsorptionDistance),
                            (sc.displacement_distance, Ch::DisplacementDistance),
                            (sc.subsurface_distance, Ch::SubsurfaceDistance),
                        ] {
                            if on {
                                let i = ch as usize;
                                let v = std::mem::replace(&mut inner.channels[i], Expr::scalar(0.));
                                inner.channels[i] = Expr::times(v, Expr::scalar(sc.scale));
                            }
                        }
                    }
                    // The group acts as one layer over every channel (§8 item 7).
                    let c = coverage(group_mask);
                    for (i, value) in inner.channels.into_iter().enumerate() {
                        let below = std::mem::replace(&mut state.channels[i], Expr::scalar(0.));
                        state.channels[i] = blend(layer.blend, below, value, c.clone());
                    }
                    if inner.shader != state.shader || inner.shader_mask != state.shader_mask {
                        state.shader = inner.shader;
                        state.shader_mask = inner.shader_mask;
                    }
                    // A Node Material inside the group is covered by the
                    // group like any channel; drivers of one from below the
                    // group blend by it too.
                    match (inner.node, &mut state.node) {
                        (Some(mut n), Some(outer)) if n.layer == outer.layer => {
                            for (name, e) in n.drivers.drain(..) {
                                let under = outer.driver(&name).cloned();
                                let under = under.unwrap_or_else(|| {
                                    let v = outer.values.get(&name).cloned().unwrap_or_default();
                                    if v.len() >= 3 {
                                        Expr::Const([v[0], v[1], v[2]])
                                    } else {
                                        Expr::scalar(v.first().copied().unwrap_or(0.))
                                    }
                                });
                                let value = blend(BlendMode::Normal, under, e, c.clone());
                                outer.drivers.retain(|(x, _)| *x != name);
                                outer.drivers.push((name, value));
                            }
                        }
                        (Some(mut n), _) => {
                            n.coverage = Expr::times(n.coverage, c.clone());
                            state.node = Some(n);
                        }
                        (None, _) => {}
                    }
                    state.shading_model = inner.shading_model;
                    state.match_specular = inner.match_specular;
                    state.use_refractive_index = inner.use_refractive_index;
                    state.thin_walled = inner.thin_walled;
                    state.subsurface_uses_diffuse = inner.subsurface_uses_diffuse;
                }
            }
        }
        state
    }

    /// Whether a group with a tag (item or polygon tag) somewhere inside
    /// `group` matches this context.
    fn tagged_group_inside_matches(&self, group: NodeId, ctx: &Context) -> bool {
        self.nodes.iter().any(|n| {
            n.id != group
                && self.is_within(n.id, group)
                && matches!(&n.kind, NodeKind::Group(g) if !g.is_untagged() && mask_matches(g, ctx))
        })
    }

    /// A group's Group Mask layers folded with their own blend modes,
    /// starting from full coverage (§8 item 6).
    fn group_mask(&self, group: NodeId, ctx: &Context, state: &Surface) -> Expr {
        let mut m = Expr::scalar(1.);
        let masks: Vec<_> = self.children(Some(group)).collect();
        for node in masks.into_iter().rev() {
            let NodeKind::Texture(t) = &node.kind else {
                continue;
            };
            if !node.layer.enable || t.effect != TextureEffect::GroupMask {
                continue;
            }
            let drives = self.texture_drives(node);
            let below = Below {
                state,
                own: None,
                drives: &drives,
            };
            if let Some((v, a)) = texture_value(t, false, node.layer.invert, ctx, below) {
                let c = Expr::times(Expr::scalar(node.layer.opacity), a);
                m = blend(node.layer.blend, m, v, c);
            }
        }
        m
    }
}

/// Inputs of Cycles' principled surface, in the order the C bridge names
/// them (operandi_cycles.h, `OC_INPUT_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    BaseColor,
    Metallic,
    Roughness,
    Ior,
    Alpha,
    DiffuseRoughness,
    SubsurfaceWeight,
    SubsurfaceRadius,
    SubsurfaceScale,
    SpecularIorLevel,
    SpecularTint,
    Anisotropic,
    TransmissionWeight,
    SheenWeight,
    CoatWeight,
    CoatRoughness,
    EmissionColor,
    EmissionStrength,
    /// Subsurface Phase as Cycles' anisotropy (XQ-4).
    SubsurfaceAnisotropy,
    // The OpenPBR inputs (OPM-03, native v41, Cycles ABI 19). Each is
    // only given when it differs from Cycles' own default, so surfaces
    // that don't use them translate as before.
    CoatTint,
    CoatIor,
    SheenTint,
    SheenRoughness,
    /// Nanometres.
    ThinFilmThickness,
    ThinFilmIor,
    DispersionScale,
    DispersionAbbe,
}
impl Input {
    pub const COUNT: usize = 27;
    /// Cycles' Principled BSDF socket (OC_INPUT_* order in
    /// operandi_cycles.h, checked against the pinned Cycles, OPM-Q07).
    pub fn socket(self) -> &'static str {
        use Input::*;
        match self {
            BaseColor => "Base Color",
            Metallic => "Metallic",
            Roughness => "Roughness",
            Ior => "IOR",
            Alpha => "Alpha",
            DiffuseRoughness => "Diffuse Roughness",
            SubsurfaceWeight => "Subsurface Weight",
            SubsurfaceRadius => "Subsurface Radius",
            SubsurfaceScale => "Subsurface Scale",
            SpecularIorLevel => "Specular IOR Level",
            SpecularTint => "Specular Tint",
            Anisotropic => "Anisotropic",
            TransmissionWeight => "Transmission Weight",
            SheenWeight => "Sheen Weight",
            CoatWeight => "Coat Weight",
            CoatRoughness => "Coat Roughness",
            EmissionColor => "Emission Color",
            EmissionStrength => "Emission Strength",
            SubsurfaceAnisotropy => "Subsurface Anisotropy",
            CoatTint => "Coat Tint",
            CoatIor => "Coat IOR",
            SheenTint => "Sheen Tint",
            SheenRoughness => "Sheen Roughness",
            ThinFilmThickness => "Thin Film Thickness",
            ThinFilmIor => "Thin Film IOR",
            DispersionScale => "Transmission Dispersion Scale",
            DispersionAbbe => "Transmission Dispersion Abbe Number",
        }
    }
    /// Inputs that are colours: their pictures are read as sRGB. Every
    /// other input is a number (roughness, metal, alpha...) whose pictures
    /// are read as plain data, as Blender's Non-Color does.
    pub fn is_colour(self) -> bool {
        matches!(
            self,
            Input::BaseColor
                | Input::SubsurfaceRadius
                | Input::SpecularTint
                | Input::EmissionColor
                | Input::CoatTint
                | Input::SheenTint
        )
    }
}

/// A surface as the principled shader's inputs, plus the shader settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Principled {
    pub inputs: Vec<(Input, Expr)>,
    pub shader: ShaderChannels,
    /// The surface relief (G-07): what the renderer's Bump node reads as its
    /// height, in scene units (folded bump height × Bump Amplitude), with
    /// Distance 1 and Strength 1. `None` when the height has no slope.
    pub bump: Option<Expr>,
    /// The folded tangent-space normal colour, `None` when flat. It needs
    /// UVs ([`Principled::without_uvs`] drops it).
    pub normal_map: Option<Expr>,
    /// How far the surface moves along its normal, in scene units:
    /// clamp(folded displacement, −1, 1) × Displacement Distance. `None`
    /// when it is 0 everywhere.
    pub displacement: Option<Expr>,
    /// How much of the bump and normal map the clear coat follows (G-03,
    /// Clearcoat Bump, 0..1).
    pub coat_bump: f64,
    /// The see-through part, `None` when Transparent Amount is 0
    /// everywhere. The output is then a Mix Shader of the principled
    /// surface (factor 0) and this (factor 1), and the principled Alpha
    /// becomes an outer mix with a clear Transparent BSDF instead.
    pub see_through: Option<SeeThrough>,
    /// Rounded Edge Width in metres (Cycles' Bevel normal), `None` for 0.
    pub rounded_edge: Option<f64>,
    /// Absorption Distance on a see-through Glass: Beer's-law absorption
    /// as Cycles' Volume Absorption (colour, density per metre).
    pub absorption: Option<([f64; 3], f64)>,
    /// OpenPBR transmission_scatter inside a see-through Glass (OPM-03):
    /// Cycles' Volume Scatter (colour, density per metre, anisotropy).
    pub scatter: Option<([f64; 3], f64, f64)>,
    /// geometry_thin_walled: Cycles' Thin Wall, and the see-through part
    /// is a thin principled glass with no inside.
    pub thin_walled: bool,
    /// The see-through Glass is built as a principled transmission (for
    /// dispersion or a thin wall, which Cycles' Glass BSDF lacks).
    pub glass_principled: bool,
    /// A Node Material mixed over the whole surface (NM-31).
    pub node: Option<NodeUse>,
    /// Material values the render's outputs write (material_values.rs),
    /// empty unless asked for with [`Surface::principled_with_values`].
    pub values: Vec<(crate::material_values::MaterialValue, Expr)>,
}

/// What the see-through part of a transparent surface is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeeThroughKind {
    /// Refractive Index 1.0: a Transparent BSDF (no bending).
    Transparent,
    /// Refractive Index above 1.0 (or textured): a Glass BSDF.
    Glass,
}

/// The see-through part of a transparent surface (translation map v0.1,
/// fix 3).
#[derive(Clone, Debug, PartialEq)]
pub struct SeeThrough {
    pub kind: SeeThroughKind,
    /// Transparent Amount: the Mix Shader's factor.
    pub amount: Expr,
    /// Transparent Color; Diffuse Color never tints this part.
    pub color: Expr,
    /// Refractive Index (Glass only).
    pub ior: Expr,
    /// Refraction Roughness (Glass only).
    pub roughness: Expr,
}

/// The IOR every surface reflects with; Specular IOR Level then scales its
/// straight-on reflectance (0.04 at level 0.5) in a straight line.
pub const SPECULAR_IOR: f64 = 1.5;

impl Principled {
    /// The inputs that change the render: subsurface radius and scale go
    /// when there is no subsurface, coat roughness when there is no coat.
    /// Two surfaces that normalise the same render the same.
    pub fn normalized(&self) -> Principled {
        let zero = |input: Input| {
            self.inputs
                .iter()
                .any(|(i, e)| *i == input && *e == Expr::scalar(0.))
        };
        let no_subsurface = zero(Input::SubsurfaceWeight);
        let no_coat = zero(Input::CoatWeight);
        let no_sheen = zero(Input::SheenWeight);
        Principled {
            inputs: self
                .inputs
                .iter()
                .filter(|(i, _)| match i {
                    Input::SubsurfaceRadius | Input::SubsurfaceScale => !no_subsurface,
                    Input::CoatRoughness | Input::CoatTint | Input::CoatIor => !no_coat,
                    Input::SheenTint | Input::SheenRoughness => !no_sheen,
                    _ => true,
                })
                .cloned()
                .collect(),
            shader: self.shader.clone(),
            bump: self.bump.clone(),
            normal_map: self.normal_map.clone(),
            displacement: self.displacement.clone(),
            // Only matters when something tilts the normal.
            coat_bump: if self.bump.is_some() || self.normal_map.is_some() {
                self.coat_bump
            } else {
                1.
            },
            see_through: self.see_through.clone(),
            rounded_edge: self.rounded_edge,
            absorption: self.absorption,
            scatter: self.scatter,
            thin_walled: self.thin_walled,
            glass_principled: self.glass_principled,
            node: self.node.clone(),
            values: self.values.clone(),
        }
    }
    /// The surface on a mesh without UVs: a normal map has no tangent
    /// space there, so it does nothing (G-07). Bump and displacement stay.
    pub fn without_uvs(mut self) -> Principled {
        self.normal_map = None;
        self
    }
    /// Normalised surfaces that render as the studio clay (ST-Q11): the
    /// default tree's, and the default before Tier 1 changed the Base
    /// Material's starting values, so older files keep their clay.
    pub fn clay_surfaces() -> Vec<Principled> {
        let ctx = Context {
            item: 0,
            ancestors: &[],
            material: crate::material::DEFAULT_MATERIAL,
            part: None,
            sets: &[],
            locators: &[],
        };
        let mut old = ShaderTree::default();
        for node in &mut old.nodes {
            if let NodeKind::Material(m) = &mut node.kind {
                **m = MaterialChannels::before_tier1();
            }
        }
        [ShaderTree::default(), old]
            .iter()
            .map(|t| t.evaluate(&ctx).principled().normalized())
            .collect()
    }
}

impl Surface {
    /// Maps the folded channels onto Cycles (research hand-off "shader
    /// settings -> Cycles translation map v0.1", 2026-11-06, fixes 1-4):
    /// - Base Color = Diffuse Color; Diffuse Amount is ignored, except by
    ///   the Physically Based model (Diffuse Color × Diffuse Amount).
    /// - IOR is always 1.5 and Specular IOR Level = Specular Amount: 50 %
    ///   reflects 0.04 straight on, 100 % 0.08, in a straight line.
    /// - With Match Specular on (the default), Reflection Amount, Color and
    ///   Fresnel do nothing. Off (provisional, ST-Q27): Reflection Amount
    ///   splits by Reflection Fresnel (G-02) into Metallic and the Coat.
    /// - Transparent Amount mixes in a see-through BSDF ([`SeeThrough`]);
    ///   it never feeds the principled Transmission.
    /// - Specular Fresnel isn't used: the principled shader is always
    ///   Fresnel-weighted (100 %, the default).
    /// - Alpha = 1 − Dissolve, × 0 where Stencil or an RGBA layer cuts, × a
    ///   Layer Mask under the shader. The shader's Alpha Type Constant is
    ///   not in it: the renderer holds the surface out instead (it stays
    ///   visible, only the picture's alpha changes).
    /// - Luminous Color × Luminous Intensity is the emission.
    /// - Traditional and Energy Conserving translate as Principled
    ///   (provisional, ST-Q28).
    pub fn principled(&self) -> Principled {
        use Input::*;
        let c = |ch: Ch| self.get(ch).clone();
        let base = match self.shading_model {
            ShadingModel::PhysicallyBased => Expr::times(c(Ch::DiffuseColor), c(Ch::DiffuseAmount)),
            ShadingModel::Principled
            | ShadingModel::Traditional
            | ShadingModel::EnergyConserving => c(Ch::DiffuseColor),
        };
        // OpenPBR subsurface_color (OPM-Q01): Cycles scatters with the
        // Base Color, so it leans toward the albedo by Subsurface Amount
        // (exact at 0 and 100 %). Older files keep the Diffuse Color.
        let base = if self.subsurface_uses_diffuse {
            base
        } else {
            Expr::mix(
                Mix::Blend,
                c(Ch::SubsurfaceAmount),
                base,
                c(Ch::SubsurfaceAlbedo),
            )
        };
        let (metallic, coat) = self.metallic_and_coat();
        let mut inputs = vec![
            (BaseColor, base),
            (Metallic, metallic),
            (Roughness, c(Ch::Roughness)),
            (DiffuseRoughness, c(Ch::DiffuseRoughness)),
            (SubsurfaceWeight, c(Ch::SubsurfaceAmount)),
            (SubsurfaceRadius, c(Ch::SubsurfaceColor)),
            (SubsurfaceScale, c(Ch::SubsurfaceDistance)),
            (SpecularTint, c(Ch::SpecularColor)),
            (Anisotropic, c(Ch::Anisotropy)),
            (SheenWeight, c(Ch::Sheen)),
            (CoatWeight, coat),
            (CoatRoughness, c(Ch::ClearcoatRoughness)),
            (EmissionColor, c(Ch::LuminousColor)),
            (EmissionStrength, c(Ch::LuminousIntensity)),
            // Use Refractive Index (group extras §3): the material's own
            // index sets the reflections; Specular Amount is then unused.
            if self.use_refractive_index {
                (Ior, c(Ch::RefractiveIndex))
            } else {
                (Ior, Expr::scalar(SPECULAR_IOR))
            },
            if self.use_refractive_index {
                (SpecularIorLevel, Expr::scalar(0.5))
            } else {
                (SpecularIorLevel, c(Ch::SpecularAmount))
            },
        ];
        let amount = c(Ch::TransparentAmount);
        let see_through = (amount != Expr::scalar(0.)).then(|| {
            let ior = c(Ch::RefractiveIndex);
            // A textured index may bend somewhere, so it takes the Glass.
            let kind = match ior.constant() {
                Some(n) if n[0] <= 1. => SeeThroughKind::Transparent,
                _ => SeeThroughKind::Glass,
            };
            SeeThrough {
                kind,
                amount,
                color: c(Ch::TransparentColor),
                ior,
                roughness: c(Ch::RefractionRoughness),
            }
        });
        let see_through = shader_effect_only(self.shader.effect, &mut inputs, see_through);
        // Cut-out effects v1: Dissolve keeps 1 − v; Stencil above 50 % and
        // RGBA cuts remove the whole surface (SQ-2: for every ray).
        let mut alpha = Expr::one_minus(c(Ch::Dissolve));
        for cut in [Ch::Stencil, Ch::RgbaCut] {
            if c(cut) != Expr::scalar(0.) {
                alpha = Expr::times(alpha, Expr::one_minus(Expr::step_above(c(cut), 0.5)));
            }
        }
        if let Some(m) = &self.shader_mask {
            alpha = Expr::times(alpha, m.clone());
        }
        inputs.push((Alpha, alpha));
        crate::openpbr::cycles_extras(&c, &mut inputs);
        // XQ-4: anisotropy = 2 (phase − 0.5); left out at 0 (even).
        let phase = Expr::times(
            Expr::mix(
                Mix::Subtract,
                Expr::scalar(1.),
                c(Ch::SubsurfacePhase),
                Expr::scalar(0.5),
            ),
            Expr::scalar(2.),
        );
        if phase != Expr::scalar(0.) {
            inputs.push((SubsurfaceAnisotropy, phase));
        }
        let rounded_edge = c(Ch::RoundedEdge)
            .constant()
            .map(|v| v[0])
            .filter(|&w| w > 0.);
        // Beer's law inside a see-through part (OPM-03, OpenPBR
        // transmission_depth/scatter; Absorption Distance spec AD-01..03):
        // light keeps T^(x/D) after x metres inside. Index 1 surfaces
        // (a Transparent BSDF) absorb too (AD-01 uses one); a thin wall
        // has no inside.
        let (absorption, scatter) = match (&see_through, c(Ch::AbsorptionDistance).constant()) {
            (Some(st), Some(d)) if d[0] > 0. && !self.thin_walled => {
                match (st.color.constant(), c(Ch::TransmissionScatter).constant()) {
                    (Some(t), Some(sc)) => {
                        let aniso = c(Ch::ScatterAnisotropy).constant().map_or(0., |v| v[0]);
                        crate::openpbr::interior_volume(t, sc, aniso, d[0])
                    }
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };
        // With an inside that tints, the surface itself passes light
        // untinted: the Transparent Color is reached only after the
        // Absorption Distance, not at every crossing (spec "Above 0").
        let mut see_through = see_through;
        if absorption.is_some()
            && let Some(st) = &mut see_through
        {
            st.color = Expr::scalar(1.);
        }
        // Render fix F4 (AD-02): at Absorption Distance 0 a solid takes on
        // the Transparent Color once, whatever its thickness. Light crosses
        // a closed object's surface twice (in and out), so each crossing
        // passes the square root. A thin wall is crossed once and keeps the
        // full colour. Provisional: a textured colour still tints at each
        // crossing (no square root in the expression set yet), and an open
        // single-sided sheet passes the root (AD-Q7).
        let at_zero = c(Ch::AbsorptionDistance)
            .constant()
            .is_some_and(|d| d[0] <= 0.);
        if at_zero
            && !self.thin_walled
            && let Some(st) = &mut see_through
            && let Some(t) = st.color.constant()
        {
            st.color = Expr::Const(t.map(|v| v.max(0.).sqrt()));
        }
        let thin_walled = self.thin_walled;
        let dispersion = inputs.iter().any(|(i, _)| *i == Input::DispersionScale);
        let film = inputs.iter().any(|(i, _)| *i == Input::ThinFilmThickness);
        // Dispersion, a thin wall or a film on the see-through part need
        // Cycles' principled transmission; a Transparent BSDF (index 1)
        // has no reflection for a film to colour (OpenPBR soap bubble).
        let glass_principled = see_through.is_some()
            && (dispersion
                || thin_walled
                || (film
                    && see_through
                        .as_ref()
                        .is_some_and(|st| st.kind == SeeThroughKind::Transparent)));
        if glass_principled && let Some(st) = &mut see_through {
            st.kind = SeeThroughKind::Glass;
        }
        Principled {
            inputs,
            shader: self.shader.clone(),
            bump: self.bump(),
            normal_map: self.normal_map(),
            displacement: self.displacement(),
            coat_bump: self
                .get(Ch::ClearcoatBump)
                .constant()
                .map_or(1., |v| v[0].clamp(0., 1.)),
            see_through,
            rounded_edge,
            absorption,
            scatter,
            thin_walled,
            glass_principled,
            // A Node Material nothing shows of renders as the layers.
            node: self.node.clone().filter(|n| n.coverage != Expr::scalar(0.)),
            values: Vec::new(),
        }
    }

    /// The straight-on reflectance the principled surface's specular
    /// layer has (no angle dependence): Specular IOR Level × 0.08 at IOR
    /// [`SPECULAR_IOR`] (0.04 at 50 %), or ((n − 1) / (n + 1))² with Use
    /// Refractive Index on (Specular IOR Level 0.5 there).
    pub(crate) fn specular_f0(&self) -> Expr {
        if self.use_refractive_index {
            let n = self.get(Ch::RefractiveIndex).clone();
            let ratio = Expr::mix(
                Mix::Divide,
                Expr::scalar(1.),
                Expr::mix(Mix::Subtract, Expr::scalar(1.), n.clone(), Expr::scalar(1.)),
                Expr::plus(n, Expr::scalar(1.)),
            );
            Expr::times(ratio.clone(), ratio)
        } else {
            Expr::times(self.get(Ch::SpecularAmount).clone(), Expr::scalar(0.08))
        }
    }

    /// The principled Metallic and Coat Weight: the channels themselves
    /// with Match Specular on; off (ST-Q27), Reflection Amount split by
    /// Reflection Fresnel into the two.
    pub(crate) fn metallic_and_coat(&self) -> (Expr, Expr) {
        let c = |ch: Ch| self.get(ch).clone();
        if self.match_specular {
            (c(Ch::Metallic), c(Ch::ClearcoatAmount))
        } else {
            (
                Expr::plus(
                    c(Ch::Metallic),
                    Expr::times(
                        c(Ch::ReflectionAmount),
                        Expr::one_minus(c(Ch::ReflectionFresnel)),
                    ),
                ),
                Expr::plus(
                    c(Ch::ClearcoatAmount),
                    Expr::times(c(Ch::ReflectionAmount), c(Ch::ReflectionFresnel)),
                ),
            )
        }
    }

    /// One height from every bump layer, scaled by Bump Amplitude (G-07):
    /// the tilt grows with both. A constant height has no slope.
    fn bump(&self) -> Option<Expr> {
        let height = self.get(Ch::Bump).clone();
        if height.constant().is_some() {
            return None;
        }
        let amplitude = self.get(Ch::BumpAmplitude).clone();
        if amplitude == Expr::scalar(0.) {
            return None;
        }
        Some(Expr::times(height, amplitude))
    }

    fn normal_map(&self) -> Option<Expr> {
        let n = self.get(Ch::NormalMap);
        (*n != Expr::Const(FLAT_NORMAL)).then(|| n.clone())
    }

    /// The folded displacement clamped to ±1 (the clamp applies to the
    /// result, not to each layer), times Displacement Distance.
    fn displacement(&self) -> Option<Expr> {
        let d = self.get(Ch::Displacement).clone();
        let distance = self.get(Ch::DisplacementDistance).clone();
        if d == Expr::scalar(0.) || distance == Expr::scalar(0.) {
            return None;
        }
        let low = Expr::mix(Mix::Lighten, Expr::scalar(1.), d, Expr::scalar(-1.));
        let clamped = Expr::mix(Mix::Darken, Expr::scalar(1.), low, Expr::scalar(1.));
        let out = Expr::times(clamped, distance);
        (out != Expr::scalar(0.)).then_some(out)
    }
}

impl crate::Scene {
    /// The tree renders use: the scene's own, or for a scene whose tree was
    /// never edited (older files), the default tree with the scene's
    /// materials mapped in, so those files render as before.
    pub fn render_shader_tree(&self) -> std::borrow::Cow<'_, ShaderTree> {
        match self.mapped_default_tree() {
            Ok(Some(tree)) => std::borrow::Cow::Owned(tree),
            _ => std::borrow::Cow::Borrowed(&self.shader_tree),
        }
    }
    /// An item's id followed by the ids of the items above it.
    pub fn item_ancestors(&self, item: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut at = self.item(item).and_then(|i| i.parent);
        while let Some(id) = at {
            if out.contains(&id) {
                break;
            }
            out.push(id);
            at = self.item(id).and_then(|i| i.parent);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_tree::{Layer, NewLayer, Node};

    fn ctx<'a>(material: &'a str) -> Context<'a> {
        Context {
            item: 1,
            ancestors: &[],
            material,
            part: None,
            sets: &[],
            locators: &[],
        }
    }
    #[test]
    fn eval_matches_folding_and_reads_images() {
        let folded = Expr::mix(
            Mix::Overlay,
            Expr::scalar(0.7),
            Expr::Const([0.2, 0.5, 0.9]),
            Expr::Const([0.6, 0.1, 0.4]),
        );
        let lazy = Expr::Mix {
            mode: Mix::Overlay,
            fac: Box::new(Expr::scalar(0.7)),
            a: Box::new(Expr::Const([0.2, 0.5, 0.9])),
            b: Box::new(Expr::Const([0.6, 0.1, 0.4])),
        };
        let none = |_: u64, _: Filtering| [0.; 4];
        assert!(near(lazy.eval(&none), folded.constant().unwrap()));
        let img = |id: u64, _: Filtering| {
            assert_eq!(id, 7);
            [1., 0.5, 0., 0.25]
        };
        let lookup = || Box::new(ImageLookup::uv(7, Filtering::Bilinear));
        let tinted = Expr::times(Expr::ImageColor(lookup()), Expr::scalar(0.5));
        assert!(near(tinted.eval(&img), [0.5, 0.25, 0.]));
        assert!(near(Expr::ImageAlpha(lookup()).eval(&img), [0.25; 3]));
        let luma = Expr::luma(Expr::ImageColor(lookup()));
        assert!(near(luma.eval(&img), [0.2126 + 0.7152 * 0.5; 3]));
    }
    fn near(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }
    fn value(s: &Surface, ch: Ch) -> [f64; 3] {
        s.get(ch).constant().expect("constant")
    }
    fn set_layer(tree: &mut ShaderTree, id: NodeId, f: impl FnOnce(&mut Node)) {
        let mut node = tree.get(id).unwrap().clone();
        f(&mut node);
        tree.replace(node).unwrap();
    }
    fn constant(
        tree: &mut ShaderTree,
        parent: NodeId,
        effect: TextureEffect,
        color: [f64; 3],
        value: f64,
        layer: Layer,
    ) -> NodeId {
        let id = tree.add(NewLayer::Constant, Some(parent)).unwrap();
        set_layer(tree, id, |n| {
            n.layer = layer;
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = effect;
                t.source = TextureSource::Constant { color, value };
            }
        });
        id
    }
    fn layer(blend: BlendMode, opacity: f64) -> Layer {
        Layer {
            blend,
            opacity,
            ..Layer::default()
        }
    }
    /// A tree whose Base Material has luminous colour (0.8, 0.4, 0.2) at
    /// intensity 1, and an empty group above it with `blend`/`opacity`.
    fn measured_setup(blend: BlendMode, opacity: f64) -> (ShaderTree, NodeId) {
        let mut tree = ShaderTree::default();
        set_layer(&mut tree, 4, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                m.luminous_color = [0.8, 0.4, 0.2];
                m.luminous_intensity = 1.;
            }
        });
        let group = tree.add(NewLayer::Group, Some(4)).unwrap();
        set_layer(&mut tree, group, |n| n.layer = layer(blend, opacity));
        (tree, group)
    }

    #[test]
    fn default_tree_gives_the_default_material() {
        let s = ShaderTree::default().evaluate(&ctx("Default"));
        let m = MaterialChannels::default();
        assert!(near(value(&s, Ch::DiffuseColor), m.diffuse_color));
        assert_eq!(s.shader, ShaderChannels::default());
        let p = s.principled();
        assert!(p.inputs.iter().all(|(_, e)| e.constant().is_some()));
    }

    #[test]
    fn group_folds_first_then_blends_as_one_layer() {
        // §8 item 7: Normal 50 % over { red Normal, Multiply 0.5 }.
        let (mut tree, group) = measured_setup(BlendMode::Normal, 0.5);
        let e = TextureEffect::LuminousColor;
        // Each new layer lands at the top of the group.
        let red = constant(
            &mut tree,
            group,
            e,
            [1., 0., 0.],
            0.,
            layer(BlendMode::Normal, 1.),
        );
        constant(
            &mut tree,
            group,
            e,
            [0.5; 3],
            0.,
            layer(BlendMode::Multiply, 1.),
        );
        let kids: Vec<_> = tree.children(Some(group)).map(|n| n.id).collect();
        assert_eq!(*kids.last().unwrap(), red, "{kids:?}");
        let s = tree.evaluate(&ctx("Default"));
        assert!(near(value(&s, Ch::LuminousColor), [0.65, 0.2, 0.1]));
    }

    #[test]
    fn group_add_blends_every_channel() {
        // §8 item 7: Add 100 % over { Normal 0.1 } doubles the untouched
        // luminous intensity too.
        let (mut tree, group) = measured_setup(BlendMode::Add, 1.);
        constant(
            &mut tree,
            group,
            TextureEffect::LuminousColor,
            [0.1; 3],
            0.,
            layer(BlendMode::Normal, 1.),
        );
        let s = tree.evaluate(&ctx("Default"));
        let c = value(&s, Ch::LuminousColor);
        let i = value(&s, Ch::LuminousIntensity)[0];
        assert!(near(c.map(|v| v * i), [1.8, 1.0, 0.6]), "{c:?} × {i}");
    }

    #[test]
    fn group_multiply_and_zero_opacity() {
        let (mut tree, group) = measured_setup(BlendMode::Multiply, 1.);
        constant(
            &mut tree,
            group,
            TextureEffect::LuminousColor,
            [0.5; 3],
            0.,
            layer(BlendMode::Normal, 1.),
        );
        let s = tree.evaluate(&ctx("Default"));
        assert!(near(value(&s, Ch::LuminousColor), [0.4, 0.2, 0.1]));
        set_layer(&mut tree, group, |n| n.layer.opacity = 0.5);
        let s = tree.evaluate(&ctx("Default"));
        assert!(near(value(&s, Ch::LuminousColor), [0.6, 0.3, 0.15]));
        set_layer(&mut tree, group, |n| n.layer.opacity = 0.);
        let s = tree.evaluate(&ctx("Default"));
        assert!(near(value(&s, Ch::LuminousColor), [0.8, 0.4, 0.2]));
    }

    #[test]
    fn texture_only_group_starts_from_below() {
        // §8 item 5: Multiply 0.5 alone in a masked group halves the base.
        let (mut tree, group) = measured_setup(BlendMode::Normal, 1.);
        set_layer(&mut tree, group, |n| {
            if let NodeKind::Group(g) = &mut n.kind {
                g.tag = TagMask::Name("Red".into());
            }
        });
        constant(
            &mut tree,
            group,
            TextureEffect::LuminousColor,
            [0.5; 3],
            0.,
            layer(BlendMode::Multiply, 1.),
        );
        let s = tree.evaluate(&ctx("Red"));
        assert!(near(value(&s, Ch::LuminousColor), [0.4, 0.2, 0.1]));
        // Other polygons don't see the masked group.
        let s = tree.evaluate(&ctx("Blue"));
        assert!(near(value(&s, Ch::LuminousColor), [0.8, 0.4, 0.2]));
    }

    #[test]
    fn layer_mask_hides_only_the_layer_directly_above() {
        // §8 item 4: black mask under red: red gone; with blue on top of
        // red, the mask under blue leaves red showing. New layers land at
        // the top of the folder.
        let e = TextureEffect::DiffuseColor;
        let n = layer(BlendMode::Normal, 1.);
        let mut tree = ShaderTree::default();
        let folder = tree.add(NewLayer::Folder, Some(4)).unwrap();
        constant(
            &mut tree,
            folder,
            TextureEffect::LayerMask,
            [0.; 3],
            0.,
            n.clone(),
        );
        constant(&mut tree, folder, e, [1., 0., 0.], 0., n.clone());
        let s = tree.evaluate(&ctx("Default"));
        assert!(near(
            value(&s, Ch::DiffuseColor),
            MaterialChannels::default().diffuse_color
        ));

        let mut tree = ShaderTree::default();
        let folder = tree.add(NewLayer::Folder, Some(4)).unwrap();
        constant(&mut tree, folder, e, [1., 0., 0.], 0., n.clone());
        constant(
            &mut tree,
            folder,
            TextureEffect::LayerMask,
            [0.; 3],
            0.,
            n.clone(),
        );
        constant(&mut tree, folder, e, [0., 0., 1.], 0., n);
        let s = tree.evaluate(&ctx("Default"));
        assert!(near(value(&s, Ch::DiffuseColor), [1., 0., 0.]));
    }

    #[test]
    fn group_masks_combine_by_their_own_blend() {
        // §8 item 6: two 0.5 masks: 0.5 when the upper is Normal, 0.25
        // when Multiply. Seen through a red Normal layer in the group.
        for (upper, expected) in [(BlendMode::Normal, 0.5), (BlendMode::Multiply, 0.25)] {
            let mut tree = ShaderTree::default();
            set_layer(&mut tree, 4, |n| {
                if let NodeKind::Material(m) = &mut n.kind {
                    m.diffuse_color = [0.; 3];
                }
            });
            let group = tree.add(NewLayer::Group, Some(4)).unwrap();
            let n = layer(BlendMode::Normal, 1.);
            constant(
                &mut tree,
                group,
                TextureEffect::DiffuseColor,
                [1., 0., 0.],
                0.,
                n.clone(),
            );
            constant(&mut tree, group, TextureEffect::GroupMask, [0.; 3], 0.5, n);
            constant(
                &mut tree,
                group,
                TextureEffect::GroupMask,
                [0.; 3],
                0.5,
                layer(upper, 1.),
            );
            let s = tree.evaluate(&ctx("Default"));
            assert!(
                near(value(&s, Ch::DiffuseColor), [expected, 0., 0.]),
                "{upper:?}"
            );
        }
    }

    #[test]
    fn topmost_applicable_shader_wins() {
        // §8 item 8: an item-masked shader below the Base Shader does
        // nothing; above it, it hides the item from the camera.
        let mut tree = ShaderTree::default();
        let group = tree.add(NewLayer::Group, Some(4)).unwrap();
        set_layer(&mut tree, group, |n| {
            if let NodeKind::Group(g) = &mut n.kind {
                g.item = ItemMask::Item(1);
            }
        });
        let shader = tree.add(NewLayer::Shader, Some(group)).unwrap();
        set_layer(&mut tree, shader, |n| {
            if let NodeKind::Shader(s) = &mut n.kind {
                s.visible_to_camera = false;
            }
        });
        // The group sits right above the Base Material, below the Base Shader.
        assert!(tree.evaluate(&ctx("Default")).shader.visible_to_camera);
        // Move the group to the top of Render.
        tree.move_to(
            &[group],
            crate::shader_tree::Drop {
                parent: None,
                index: 0,
            },
        )
        .unwrap();
        assert!(!tree.evaluate(&ctx("Default")).shader.visible_to_camera);
        // Another item isn't affected.
        let other = Context {
            item: 2,
            ..ctx("Default")
        };
        assert!(tree.evaluate(&other).shader.visible_to_camera);
    }

    #[test]
    fn image_maps_keep_an_expression() {
        let mut tree = ShaderTree::default();
        let id = tree.add(NewLayer::ImageMap, Some(4)).unwrap();
        set_layer(&mut tree, id, |n| {
            n.layer.opacity = 0.5;
            if let NodeKind::Texture(t) = &mut n.kind
                && let TextureSource::ImageMap { image, .. } = &mut t.source
            {
                *image = Some(9);
            }
        });
        let s = tree.evaluate(&ctx("Default"));
        let mut images = Vec::new();
        s.get(Ch::DiffuseColor).images(&mut images);
        assert_eq!(images, vec![9]);
        assert!(s.get(Ch::Roughness).constant().is_some());
    }

    /// The default tree with its Base Material changed by `f`, translated.
    fn translated(f: impl FnOnce(&mut MaterialChannels)) -> Principled {
        let mut tree = ShaderTree::default();
        base_material(&mut tree, f);
        tree.evaluate(&ctx("Default")).principled()
    }

    #[test]
    fn base_color_is_diffuse_color_except_physically_based() {
        let p = translated(|m| {
            m.diffuse_color = [0.6; 3];
            m.diffuse_amount = 0.8;
        });
        assert!((input(&p, Input::BaseColor).unwrap() - 0.6).abs() < 1e-9);
        let p = translated(|m| {
            m.diffuse_color = [0.6; 3];
            m.diffuse_amount = 0.8;
            m.shading_model = ShadingModel::PhysicallyBased;
        });
        assert!((input(&p, Input::BaseColor).unwrap() - 0.48).abs() < 1e-9);
        // Traditional and Energy Conserving act as Principled (ST-Q28).
        for model in [ShadingModel::Traditional, ShadingModel::EnergyConserving] {
            let p = translated(|m| {
                m.diffuse_color = [0.6; 3];
                m.diffuse_amount = 0.8;
                m.shading_model = model;
            });
            assert!((input(&p, Input::BaseColor).unwrap() - 0.6).abs() < 1e-9);
        }
    }

    #[test]
    fn specular_amount_is_a_straight_line_at_ior_1_5() {
        for (amount, reflectance) in [(0.25, 0.02), (0.5, 0.04), (1.0, 0.08)] {
            let p = translated(|m| m.specular_amount = amount);
            assert_eq!(input(&p, Input::Ior), Some(1.5));
            let level = input(&p, Input::SpecularIorLevel).unwrap();
            assert!((level - amount).abs() < 1e-9);
            // Cycles: F0 = ((n − 1) / (n + 1))² × 2 × level.
            let f0 = (0.5f64 / 2.5).powi(2) * 2. * level;
            assert!((f0 - reflectance).abs() < 1e-9, "{amount}: {f0}");
        }
        // Transparent surfaces reflect the same way.
        let p = translated(|m| {
            m.transparent_amount = 1.;
            m.refractive_index = 1.33;
        });
        assert_eq!(input(&p, Input::Ior), Some(1.5));
        assert_eq!(input(&p, Input::SpecularIorLevel), Some(0.5));
    }

    #[test]
    fn match_specular_turns_reflection_off() {
        let p = translated(|m| {
            m.reflection_amount = 1.;
            m.reflection_fresnel = 0.5;
        });
        assert_eq!(input(&p, Input::Metallic), Some(0.));
        assert_eq!(input(&p, Input::CoatWeight), Some(0.));
        let p = translated(|m| {
            m.reflection_amount = 1.;
            m.metallic = 0.3;
        });
        assert_eq!(input(&p, Input::Metallic), Some(0.3));
        // Off: the earlier mapping (provisional, ST-Q27).
        let p = translated(|m| {
            m.reflection_amount = 1.;
            m.match_specular = false;
        });
        assert_eq!(input(&p, Input::Metallic), Some(1.));
    }

    #[test]
    fn transparency_mixes_in_a_see_through_bsdf() {
        let p = translated(|_| {});
        assert_eq!(p.see_through, None);
        let p = translated(|m| {
            m.diffuse_color = [0.8, 0.1, 0.1];
            m.transparent_amount = 0.5;
            m.transparent_color = [0.9, 0.8, 0.7];
            m.refractive_index = 1.;
        });
        assert_eq!(input(&p, Input::TransmissionWeight), None);
        let s = p.see_through.as_ref().unwrap();
        assert_eq!(s.kind, SeeThroughKind::Transparent);
        assert_eq!(s.amount, Expr::scalar(0.5));
        // Render fix F4: at Absorption Distance 0 a solid passes the square
        // root at each of its two crossings.
        assert_eq!(s.color, Expr::Const([0.9f64, 0.8, 0.7].map(f64::sqrt)));
        let p = translated(|m| {
            m.transparent_amount = 1.;
            m.refractive_index = 1.5;
            m.refraction_roughness = 0.2;
        });
        assert_eq!(input(&p, Input::TransmissionWeight), None);
        let s = p.see_through.as_ref().unwrap();
        assert_eq!(s.kind, SeeThroughKind::Glass);
        assert_eq!(s.ior, Expr::scalar(1.5));
        assert_eq!(s.roughness, Expr::scalar(0.2));
        // A Refractive Index that varies (e.g. two materials blended by an
        // image) takes the Glass.
        let mut tree = ShaderTree::default();
        base_material(&mut tree, |m| m.transparent_amount = 1.);
        let mut s = tree.evaluate(&ctx("Default"));
        s.channels[Ch::RefractiveIndex as usize] =
            Expr::ImageColor(Box::new(ImageLookup::uv(9, Filtering::Bilinear)));
        assert_eq!(
            s.principled().see_through.unwrap().kind,
            SeeThroughKind::Glass
        );
    }

    #[test]
    fn blend_modes_fold_like_the_renderer() {
        let a = Expr::Const([0.25, 0.5, 0.75]);
        let b = Expr::Const([0.5; 3]);
        let one = Expr::scalar(1.);
        let r = |m| {
            blend(m, a.clone(), b.clone(), one.clone())
                .constant()
                .unwrap()
        };
        assert!(near(r(BlendMode::Add), [0.75, 1.0, 1.25]));
        assert!(near(r(BlendMode::Multiply), [0.125, 0.25, 0.375]));
        assert!(near(r(BlendMode::Subtract), [0., 0., 0.25]));
        assert!(near(r(BlendMode::Darken), [0.25, 0.5, 0.5]));
        assert!(near(r(BlendMode::Lighten), [0.5, 0.5, 0.75]));
        assert!(near(r(BlendMode::Screen), [0.625, 0.75, 0.875]));
        // Hard Light with a 0.5 layer leaves the base alone.
        assert!(near(r(BlendMode::HardLight), [0.25, 0.5, 0.75]));
    }

    fn input(p: &Principled, which: Input) -> Option<f64> {
        p.inputs
            .iter()
            .find(|(i, _)| *i == which)
            .map(|(_, e)| e.constant().expect("constant")[0])
    }

    fn sets_ctx<'a>(sets: &'a [&'a str]) -> Context<'a> {
        Context {
            sets,
            ..ctx("Default")
        }
    }

    /// A Selection Set group named `set` holding a Constant on `effect`.
    fn set_group(tree: &mut ShaderTree, set: &str, effect: TextureEffect, v: [f64; 3]) -> NodeId {
        let group = tree.add(NewLayer::Group, Some(4)).unwrap();
        set_layer(tree, group, |n| {
            if let NodeKind::Group(g) = &mut n.kind {
                g.tag_type = TagType::SelectionSet;
                g.tag = TagMask::Name(set.into());
            }
        });
        constant(tree, group, effect, v, 0., layer(BlendMode::Normal, 1.));
        group
    }

    #[test]
    fn selection_set_groups_match_polygons_in_the_set() {
        // G-01: the group applies to polygons in the set, AND the item mask.
        let mut tree = ShaderTree::default();
        let group = set_group(&mut tree, "Top", TextureEffect::DiffuseColor, [1., 0., 0.]);
        let base = MaterialChannels::default().diffuse_color;
        assert!(near(
            value(&tree.evaluate(&sets_ctx(&["Top"])), Ch::DiffuseColor),
            [1., 0., 0.]
        ));
        assert!(near(
            value(
                &tree.evaluate(&sets_ctx(&["Side", "Top"])),
                Ch::DiffuseColor
            ),
            [1., 0., 0.]
        ));
        assert!(near(
            value(&tree.evaluate(&sets_ctx(&["Side"])), Ch::DiffuseColor),
            base
        ));
        assert!(near(
            value(&tree.evaluate(&sets_ctx(&[])), Ch::DiffuseColor),
            base
        ));
        set_layer(&mut tree, group, |n| {
            if let NodeKind::Group(g) = &mut n.kind {
                g.item = ItemMask::Item(2);
            }
        });
        assert!(near(
            value(&tree.evaluate(&sets_ctx(&["Top"])), Ch::DiffuseColor),
            base
        ));
        // A material name doesn't match a set group of the same name.
        assert!(near(
            value(&tree.evaluate(&ctx("Top")), Ch::DiffuseColor),
            base
        ));
    }

    #[test]
    fn overlapping_set_groups_both_apply_in_tree_order() {
        // Each group tests on its own: a polygon in both sets gets both,
        // the upper group last (on top).
        let mut tree = ShaderTree::default();
        set_group(&mut tree, "A", TextureEffect::DiffuseColor, [1., 0., 0.]);
        // Added second, so it lands above A.
        set_group(&mut tree, "B", TextureEffect::DiffuseColor, [0., 0., 1.]);
        let both = tree.evaluate(&sets_ctx(&["A", "B"]));
        assert!(near(value(&both, Ch::DiffuseColor), [0., 0., 1.]));
        let a = tree.evaluate(&sets_ctx(&["A"]));
        assert!(near(value(&a, Ch::DiffuseColor), [1., 0., 0.]));
        // Two groups on different channels both reach the polygon.
        let mut tree = ShaderTree::default();
        set_group(&mut tree, "A", TextureEffect::DiffuseColor, [1., 0., 0.]);
        // The helper's Constant has value 0: Roughness 0 in set B.
        set_group(&mut tree, "B", TextureEffect::Roughness, [0.; 3]);
        let both = tree.evaluate(&sets_ctx(&["A", "B"]));
        assert!(near(value(&both, Ch::DiffuseColor), [1., 0., 0.]));
        assert_eq!(value(&both, Ch::Roughness)[0], 0.);
        let b = tree.evaluate(&sets_ctx(&["B"]));
        assert_eq!(value(&b, Ch::Roughness)[0], 0.);
        assert!(near(
            value(&b, Ch::DiffuseColor),
            MaterialChannels::default().diffuse_color
        ));
    }

    #[test]
    fn reflection_fresnel_is_a_channel_and_splits_reflection() {
        // G-02: the effect drives its own channel (default 0 %).
        assert_eq!(
            Ch::of_effect(TextureEffect::ReflectionFresnel),
            Some(Ch::ReflectionFresnel)
        );
        let mut tree = ShaderTree::default();
        assert_eq!(
            value(&tree.evaluate(&ctx("Default")), Ch::ReflectionFresnel)[0],
            0.
        );
        // The split applies with Match Specular off (provisional, ST-Q27).
        set_layer(&mut tree, 4, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                m.reflection_amount = 0.8;
                m.match_specular = false;
            }
        });
        let p = tree.evaluate(&ctx("Default")).principled();
        assert!((input(&p, Input::Metallic).unwrap() - 0.8).abs() < 1e-9);
        assert_eq!(input(&p, Input::CoatWeight), Some(0.));
        // A texture at 25 % Reflection Fresnel moves a quarter to the coat.
        constant(
            &mut tree,
            4,
            TextureEffect::ReflectionFresnel,
            [0.; 3],
            0.25,
            layer(BlendMode::Normal, 1.),
        );
        let s = tree.evaluate(&ctx("Default"));
        assert_eq!(value(&s, Ch::ReflectionFresnel)[0], 0.25);
        let p = s.principled();
        assert!((input(&p, Input::Metallic).unwrap() - 0.6).abs() < 1e-9);
        assert!((input(&p, Input::CoatWeight).unwrap() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn half_specular_is_a_plain_dielectric_and_ior_one_keeps_reflections() {
        // 50 % → F0 0.04 (IOR 1.5, level 0.5) whatever the Refractive
        // Index, so 1.0 never turns reflections off.
        let p = translated(|_| {});
        assert_eq!(input(&p, Input::Ior), Some(1.5));
        assert_eq!(input(&p, Input::SpecularIorLevel), Some(0.5));
        // Refractive Index 1.33 refracts as water in the Glass; the
        // surface still reflects 0.04.
        let p = translated(|m| {
            m.transparent_amount = 1.;
            m.refractive_index = 1.33;
        });
        assert_eq!(input(&p, Input::Ior), Some(1.5));
        assert_eq!(input(&p, Input::SpecularIorLevel), Some(0.5));
        assert_eq!(p.see_through.unwrap().ior, Expr::scalar(1.33));
    }

    #[test]
    fn old_defaults_and_new_defaults_both_render_as_clay() {
        let clay = Principled::clay_surfaces();
        assert_eq!(clay.len(), 2);
        assert!(
            clay.contains(
                &ShaderTree::default()
                    .evaluate(&ctx("Default"))
                    .principled()
                    .normalized()
            )
        );
        let mut old = ShaderTree::default();
        set_layer(&mut old, 4, |n| {
            n.kind = NodeKind::Material(Box::new(MaterialChannels::before_tier1()))
        });
        assert!(clay.contains(&old.evaluate(&ctx("Default")).principled().normalized()));
        // A real change is not clay.
        set_layer(&mut old, 4, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                m.roughness = 0.1;
            }
        });
        assert!(!clay.contains(&old.evaluate(&ctx("Default")).principled().normalized()));
    }

    /// An Image Map on `effect` with `f` applied to its settings, image 9.
    fn image_tree(effect: TextureEffect, f: impl FnOnce(&mut ImageSettings)) -> ShaderTree {
        let mut tree = ShaderTree::default();
        let id = tree.add(NewLayer::ImageMap, Some(4)).unwrap();
        set_layer(&mut tree, id, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.effect = effect;
                if let TextureSource::ImageMap { image, settings } = &mut t.source {
                    *image = Some(9);
                    f(settings);
                }
            }
        });
        tree
    }
    /// The channel's value where image 9 reads `rgba`.
    fn image_eval(tree: &ShaderTree, ch: Ch, rgba: [f64; 4]) -> [f64; 3] {
        tree.evaluate(&ctx("Default")).get(ch).eval(&|i, _| {
            assert_eq!(i, 9);
            rgba
        })
    }

    #[test]
    fn image_map_defaults_pass_the_image_through() {
        let tree = image_tree(TextureEffect::DiffuseColor, |_| {});
        let c = image_eval(&tree, Ch::DiffuseColor, [0.2, 0.4, 0.6, 1.]);
        assert!(near(c, [0.2, 0.4, 0.6]));
        // Alpha 0 shows the material below.
        let c = image_eval(&tree, Ch::DiffuseColor, [0.2, 0.4, 0.6, 0.]);
        assert!(near(c, MaterialChannels::default().diffuse_color));
    }

    #[test]
    fn image_map_low_and_high_remap_scalar_effects() {
        let tree = image_tree(TextureEffect::Roughness, |s| {
            s.low_value = 0.2;
            s.high_value = 0.6;
        });
        let at = |grey: f64| image_eval(&tree, Ch::Roughness, [grey, grey, grey, 1.])[0];
        assert!((at(0.) - 0.2).abs() < 1e-9);
        assert!((at(1.) - 0.6).abs() < 1e-9);
        assert!((at(0.5) - 0.4).abs() < 1e-9);
        // Colour effects ignore Low/High.
        let tree = image_tree(TextureEffect::DiffuseColor, |s| s.low_value = 0.5);
        assert!(near(
            image_eval(&tree, Ch::DiffuseColor, [0., 0., 0., 1.]),
            [0.; 3]
        ));
    }

    #[test]
    fn image_map_invert_gamma_brightness_contrast_clamp() {
        let px = [0.25, 0.5, 0.8, 1.];
        let c = |f: fn(&mut ImageSettings)| {
            image_eval(
                &image_tree(TextureEffect::DiffuseColor, f),
                Ch::DiffuseColor,
                px,
            )
        };
        assert!(near(c(|s| s.invert_green = true), [0.25, 0.5, 0.8]));
        assert!(near(c(|s| s.invert_red = true), [0.75, 0.5, 0.8]));
        assert!(near(
            c(|s| {
                s.invert_red = true;
                s.invert_green = true;
                s.invert_blue = true
            }),
            [0.75, 0.5, 0.2]
        ));
        assert!(near(
            c(|s| s.gamma = 2.),
            [0.5, 0.5f64.sqrt(), 0.8f64.sqrt()]
        ));
        assert!(near(c(|s| s.brightness = 2.), [0.5, 1., 1.6]));
        assert!(near(c(|s| s.contrast = 2.), [0., 0.5, 1.1]));
        assert!(near(
            c(|s| {
                s.brightness = 2.;
                s.clamp = true
            }),
            [0.5, 1., 1.]
        ));
    }

    #[test]
    fn image_map_swizzle_and_alpha_modes() {
        let px = [0.1, 0.2, 0.3, 0.5];
        let base = MaterialChannels::default().diffuse_color;
        let full = |f: fn(&mut ImageSettings)| {
            image_eval(
                &image_tree(TextureEffect::DiffuseColor, f),
                Ch::DiffuseColor,
                px,
            )
        };
        // Use: half coverage over the base.
        let half = |v: [f64; 3]| [0, 1, 2].map(|i| (base[i] + v[i]) / 2.);
        assert!(near(full(|_| {}), half([0.1, 0.2, 0.3])));
        assert!(near(full(|s| s.swizzle = Swizzle::Green), half([0.2; 3])));
        assert!(near(full(|s| s.swizzle = Swizzle::Alpha), half([0.5; 3])));
        // RGB swizzling and Ignore leave the alpha out: full coverage.
        assert!(near(full(|s| s.swizzle = Swizzle::Rgb), [0.1, 0.2, 0.3]));
        assert!(near(
            full(|s| s.alpha = ImageAlpha::Ignore),
            [0.1, 0.2, 0.3]
        ));
        // Alpha Only: the alpha is the value, fully covering.
        assert!(near(full(|s| s.alpha = ImageAlpha::AlphaOnly), [0.5; 3]));
    }

    #[test]
    fn image_filtering_reaches_the_expression() {
        let tree = image_tree(TextureEffect::DiffuseColor, |s| {
            s.filtering = Filtering::Nearest
        });
        let seen = std::cell::Cell::new(None);
        tree.evaluate(&ctx("Default"))
            .get(Ch::DiffuseColor)
            .eval(&|_, f| {
                seen.set(Some(f));
                [0.; 4]
            });
        assert_eq!(seen.get(), Some(Filtering::Nearest));
    }

    #[test]
    fn two_zone_bias_and_gain_curves() {
        use crate::shader_tree::TwoZone;
        let z = TwoZone::default();
        // 50 % bias and gain leave the pattern alone.
        for t in [0., 0.1, 0.5, 0.9, 1.] {
            assert!((TwoZone::bias(0.5, t) - t).abs() < 1e-12);
            assert!((TwoZone::gain(0.5, t) - t).abs() < 1e-12);
        }
        // Bias pushes toward a zone; gain sharpens around the middle.
        assert!(TwoZone::bias(0.8, 0.5) > 0.5);
        assert!((TwoZone::bias(0.8, 0.5) - 0.8).abs() < 1e-12);
        assert!(TwoZone::gain(0.8, 0.25) < 0.25 && TwoZone::gain(0.8, 0.75) > 0.75);
        assert_eq!(TwoZone::gain(0.8, 0.5), 0.5);
        // Zone 1 black / 0 at 0, zone 2 white / 1 at 1, alphas 100 %.
        assert_eq!(z.sample(0.), (0., [0.; 3], 1.));
        assert_eq!(z.sample(1.), (1., [1.; 3], 1.));
        let (v, c, a) = z.sample(0.25);
        assert!((v - 0.25).abs() < 1e-12 && near(c, [0.25; 3]) && a == 1.);
    }

    #[test]
    fn blend_modes_match_the_measured_points() {
        // G-06: below (0.8, 0.4, 0.2), layer (0.25, 0.5, 0.75), 100 %.
        let b = Expr::Const([0.8, 0.4, 0.2]);
        let l = Expr::Const([0.25, 0.5, 0.75]);
        let r = |m, cov: f64| {
            blend(m, b.clone(), l.clone(), Expr::scalar(cov))
                .constant()
                .unwrap()
        };
        assert!(near(r(BlendMode::Normal, 1.), [0.25, 0.5, 0.75]));
        assert!(near(r(BlendMode::Add, 1.), [1.05, 0.9, 0.95]));
        assert!(near(r(BlendMode::Subtract, 1.), [0.55, 0., 0.]));
        assert!(near(r(BlendMode::Difference, 1.), [0.55, 0.1, 0.55]));
        assert!(near(r(BlendMode::Multiply, 1.), [0.2, 0.2, 0.15]));
        assert!(near(r(BlendMode::Screen, 1.), [0.85, 0.7, 0.8]));
        assert!(near(r(BlendMode::Overlay, 1.), [0.7, 0.4, 0.3]));
        assert!(near(r(BlendMode::HardLight, 1.), [0.4, 0.4, 0.6]));
        assert!(near(r(BlendMode::Darken, 1.), [0.25, 0.4, 0.2]));
        assert!(near(r(BlendMode::Lighten, 1.), [0.8, 0.5, 0.75]));
        assert!(near(r(BlendMode::ColorDodge, 1.), [1., 0.8, 0.8]));
        assert!(near(r(BlendMode::ColorBurn, 1.), [0.2, 0., 0.]));
        // Soft Light: up to 0.5, B - (1 - 2L) B (1 - B); above,
        // B + (2L - 1)(√B - B).
        let soft = r(BlendMode::SoftLight, 1.);
        let blue = 0.2 + 0.5 * (0.2f64.sqrt() - 0.2);
        assert!(near(soft, [0.72, 0.4, blue]), "{soft:?}");
        // The measured B 0.2, L 0.75 point: 0.320 (the formula gives 0.324).
        assert!((blue - 0.320).abs() < 5e-3);
        // Divide: (1 - B + 2L) / (3 (1 + B)); B 0.8, L 0.25 gives 0.130.
        let div = r(BlendMode::Divide, 1.);
        assert!((div[0] - 0.130).abs() < 1e-3, "{div:?}");
        // The other measured points (grey layers and the coloured one).
        for (l, expected) in [
            (0.25, [0.129, 0.262, 0.359]),
            (0.5, [0.219, 0.379, 0.5]),
            (1.0, [0.406, 0.617, 0.777]),
        ] {
            let got = blend(
                BlendMode::Divide,
                b.clone(),
                Expr::scalar(l),
                Expr::scalar(1.),
            )
            .constant()
            .unwrap();
            assert!(
                (0..3).all(|i| (got[i] - expected[i]).abs() < 5e-3),
                "{l}: {got:?}"
            );
        }
        let got = r(BlendMode::Divide, 1.);
        assert!(
            (0..3).all(|i| (got[i] - [0.129, 0.379, 0.637][i]).abs() < 5e-3),
            "{got:?}"
        );
        // Opacity lerps between below and the clamped result.
        assert!(near(r(BlendMode::Subtract, 0.5), [0.675, 0.2, 0.1]));
        assert!(near(r(BlendMode::ColorBurn, 0.5), [0.5, 0.2, 0.1]));
        assert!(near(r(BlendMode::ColorDodge, 0.5), [0.9, 0.6, 0.5]));
        // Through an image the same maths runs per pixel.
        let img = Expr::ImageColor(Box::new(ImageLookup::uv(1, Filtering::Bilinear)));
        let e = blend(BlendMode::Subtract, b.clone(), img, Expr::scalar(1.));
        assert!(near(e.eval(&|_, _| [0.25, 0.5, 0.75, 1.]), [0.55, 0., 0.]));
    }

    #[test]
    fn layers_above_the_winning_shader_do_nothing() {
        // G-19: a red Constant at the very top, above the Base Shader.
        let base = MaterialChannels::default().diffuse_color;
        let mut tree = ShaderTree::default();
        let red = tree.add(NewLayer::Constant, None).unwrap();
        tree.move_to(
            &[red],
            crate::shader_tree::Drop {
                parent: None,
                index: 0,
            },
        )
        .unwrap();
        set_layer(&mut tree, red, |n| {
            if let NodeKind::Texture(t) = &mut n.kind {
                t.source = TextureSource::Constant {
                    color: [1., 0., 0.],
                    value: 0.,
                };
            }
        });
        assert!(near(
            value(&tree.evaluate(&ctx("Default")), Ch::DiffuseColor),
            base
        ));
        // Inside a group that sits above the Base Shader: still nothing.
        let group = tree.add(NewLayer::Group, None).unwrap();
        tree.move_to(
            &[group],
            crate::shader_tree::Drop {
                parent: None,
                index: 0,
            },
        )
        .unwrap();
        tree.move_to(
            &[red],
            crate::shader_tree::Drop {
                parent: Some(group),
                index: 0,
            },
        )
        .unwrap();
        assert!(near(
            value(&tree.evaluate(&ctx("Default")), Ch::DiffuseColor),
            base
        ));
        // Moved below the Base Shader it takes effect.
        let below = tree.children(None).position(|n| n.id == 3).unwrap() + 1;
        tree.move_to(
            &[group],
            crate::shader_tree::Drop {
                parent: None,
                index: below,
            },
        )
        .unwrap();
        assert!(near(
            value(&tree.evaluate(&ctx("Default")), Ch::DiffuseColor),
            [1., 0., 0.]
        ));
        // A shader at the top that wins for item 1 moves the line up: the
        // group, put just below that shader (above the Base Shader), counts
        // for item 1; for item 2 the Base Shader wins and it doesn't.
        let top = tree.add(NewLayer::Group, None).unwrap();
        tree.move_to(
            &[top],
            crate::shader_tree::Drop {
                parent: None,
                index: 0,
            },
        )
        .unwrap();
        set_layer(&mut tree, top, |n| {
            if let NodeKind::Group(g) = &mut n.kind {
                g.item = ItemMask::Item(1);
            }
        });
        tree.add(NewLayer::Shader, Some(top)).unwrap();
        tree.move_to(
            &[group],
            crate::shader_tree::Drop {
                parent: None,
                index: 1,
            },
        )
        .unwrap();
        // Order: [top group {shader}], [group {red}], outputs, Base Shader…
        assert!(near(
            value(&tree.evaluate(&ctx("Default")), Ch::DiffuseColor),
            [1., 0., 0.]
        ));
        let other = Context {
            item: 2,
            ..ctx("Default")
        };
        assert!(near(value(&tree.evaluate(&other), Ch::DiffuseColor), base));
    }

    // G-07: bump, normal maps and displacement.

    fn base_material(tree: &mut ShaderTree, f: impl FnOnce(&mut MaterialChannels)) {
        set_layer(tree, 4, |n| {
            if let NodeKind::Material(m) = &mut n.kind {
                f(m);
            }
        });
    }
    fn image_layer(
        tree: &mut ShaderTree,
        effect: TextureEffect,
        f: impl FnOnce(&mut ImageSettings, &mut Layer),
    ) -> NodeId {
        let id = tree.add(NewLayer::ImageMap, Some(4)).unwrap();
        set_layer(tree, id, |n| {
            if let NodeKind::Texture(t) = &mut n.kind
                && let TextureSource::ImageMap { image, settings } = &mut t.source
            {
                t.effect = effect;
                *image = Some(9);
                f(settings, &mut n.layer);
            }
        });
        id
    }
    /// A surface value where every image reads grey `v`.
    fn at(e: &Expr, v: f64) -> f64 {
        e.eval(&|_, _| [v, v, v, 1.])[0]
    }
    /// The bump height's change from a black to a white image: the tilt.
    fn tilt(p: &Principled) -> f64 {
        let b = p.bump.as_ref().expect("bump");
        at(b, 1.) - at(b, 0.)
    }
    fn surface(tree: &ShaderTree) -> Principled {
        tree.evaluate(&ctx("Default")).principled()
    }

    #[test]
    fn bump_layers_fold_into_one_height() {
        let e = TextureEffect::Bump;
        let mut tree = ShaderTree::default();
        constant(&mut tree, 4, e, [0.; 3], 0.3, layer(BlendMode::Normal, 1.));
        let top = constant(&mut tree, 4, e, [0.; 3], 0.2, layer(BlendMode::Add, 1.));
        // Add sums.
        assert!((value(&tree.evaluate(&ctx("Default")), Ch::Bump)[0] - 0.5).abs() < 1e-9);
        // Normal on top replaces; opacity scales toward the layer below.
        set_layer(&mut tree, top, |n| n.layer = layer(BlendMode::Normal, 0.5));
        assert!((value(&tree.evaluate(&ctx("Default")), Ch::Bump)[0] - 0.25).abs() < 1e-9);
        set_layer(&mut tree, top, |n| n.layer = layer(BlendMode::Normal, 1.));
        assert!((value(&tree.evaluate(&ctx("Default")), Ch::Bump)[0] - 0.2).abs() < 1e-9);
        // A Material layer above has no height of its own: the bump stays.
        tree.add(NewLayer::Material, Some(top)).unwrap();
        assert!((value(&tree.evaluate(&ctx("Default")), Ch::Bump)[0] - 0.2).abs() < 1e-9);
    }

    #[test]
    fn bump_tilt_scales_with_amplitude_high_value_and_opacity() {
        let mut tree = ShaderTree::default();
        let img = image_layer(&mut tree, TextureEffect::Bump, |_, _| {});
        let base = tilt(&surface(&tree));
        // The Base Material's 5 mm amplitude is the tilt from black to white.
        assert!((base - 0.005).abs() < 1e-12, "{base}");
        base_material(&mut tree, |m| m.bump_amplitude = 0.01);
        assert!((tilt(&surface(&tree)) - 2. * base).abs() < 1e-12);
        base_material(&mut tree, |m| m.bump_amplitude = 0.005);
        set_layer(&mut tree, img, |n| {
            if let NodeKind::Texture(t) = &mut n.kind
                && let TextureSource::ImageMap { settings, .. } = &mut t.source
            {
                settings.high_value = 2.;
            }
        });
        assert!((tilt(&surface(&tree)) - 2. * base).abs() < 1e-12);
        // Opacity scales the tilt; Invert flips it.
        set_layer(&mut tree, img, |n| {
            n.layer.opacity = 0.5;
            n.layer.invert = true;
        });
        assert!((tilt(&surface(&tree)) + base).abs() < 1e-12);
        // Amplitude 0: no bump at all.
        base_material(&mut tree, |m| m.bump_amplitude = 0.);
        assert_eq!(surface(&tree).bump, None);
    }

    #[test]
    fn flat_constant_bump_on_top_cancels_the_bump_below() {
        let mut tree = ShaderTree::default();
        image_layer(&mut tree, TextureEffect::Bump, |_, _| {});
        assert!(surface(&tree).bump.is_some());
        constant(
            &mut tree,
            4,
            TextureEffect::Bump,
            [0.; 3],
            0.4,
            layer(BlendMode::Normal, 1.),
        );
        assert_eq!(surface(&tree).bump, None);
    }

    #[test]
    fn displacement_clamps_the_folded_value() {
        let e = TextureEffect::Displacement;
        let moved = |v: f64| {
            let mut tree = ShaderTree::default();
            constant(&mut tree, 4, e, [0.; 3], v, layer(BlendMode::Normal, 1.));
            surface(&tree)
                .displacement
                .map(|d| d.constant().unwrap()[0])
        };
        // Base Material: 20 mm.
        for (v, expected) in [
            (1., 0.02),
            (0.5, 0.01),
            (-1., -0.02),
            (2., 0.02),
            (-2., -0.02),
        ] {
            let got = moved(v).unwrap();
            assert!((got - expected).abs() < 1e-12, "{v}: {got}");
        }
        assert_eq!(moved(0.), None);
        // 0.7 Add over 0.7 is 1.4, clipped to 1 (not each layer clipped).
        let mut tree = ShaderTree::default();
        constant(&mut tree, 4, e, [0.; 3], 0.7, layer(BlendMode::Normal, 1.));
        let top = constant(&mut tree, 4, e, [0.; 3], 0.7, layer(BlendMode::Add, 1.));
        let d = surface(&tree).displacement.unwrap().constant().unwrap()[0];
        assert!((d - 0.02).abs() < 1e-12, "{d}");
        // Opacity lerps toward the value below.
        set_layer(&mut tree, top, |n| n.layer = layer(BlendMode::Normal, 0.5));
        let d = surface(&tree).displacement.unwrap().constant().unwrap()[0];
        assert!((d - 0.014).abs() < 1e-12, "{d}");
        // Distance 0 or no displacement layer: the mesh stays as it is.
        base_material(&mut tree, |m| m.displacement_distance = 0.);
        assert_eq!(surface(&tree).displacement, None);
        assert_eq!(surface(&ShaderTree::default()).displacement, None);
    }

    #[test]
    fn image_displacement_uses_brightness_remapped_by_low_and_high() {
        let mut tree = ShaderTree::default();
        image_layer(&mut tree, TextureEffect::Displacement, |s, _| {
            s.low_value = -1.;
            s.high_value = 3.;
        });
        let d = surface(&tree).displacement.unwrap();
        // Black → -1 (full distance in); grey 0.25 → 0; white → 3, clipped to 1.
        assert!((at(&d, 0.) + 0.02).abs() < 1e-12);
        assert!(at(&d, 0.25).abs() < 1e-12);
        assert!((at(&d, 1.) - 0.02).abs() < 1e-12);
    }

    #[test]
    fn normal_maps_need_uvs_and_combine_with_bump_in_any_order() {
        let mut tree = ShaderTree::default();
        let normal = constant(
            &mut tree,
            4,
            TextureEffect::Normal,
            [0.6, 0.5, 0.9],
            0.,
            layer(BlendMode::Normal, 1.),
        );
        let p = surface(&tree);
        assert_eq!(p.normal_map, Some(Expr::Const([0.6, 0.5, 0.9])));
        assert_eq!(p.bump, None);
        // Without UVs a normal map alone does nothing: the clay again.
        let bare = p.clone().without_uvs().normalized();
        assert!(Principled::clay_surfaces().contains(&bare));
        // With a bump layer on top or below, both apply and the surface is
        // the same either way.
        let bump = image_layer(&mut tree, TextureEffect::Bump, |_, _| {});
        let over = surface(&tree);
        assert!(over.bump.is_some() && over.normal_map.is_some());
        let parent = tree.get(bump).unwrap().parent;
        let order =
            |tree: &ShaderTree| -> Vec<NodeId> { tree.children(parent).map(|n| n.id).collect() };
        let pos = |tree: &ShaderTree, id| order(tree).iter().position(|&n| n == id).unwrap();
        assert!(pos(&tree, bump) < pos(&tree, normal), "bump starts on top");
        let index = pos(&tree, bump);
        tree.move_to(&[normal], crate::shader_tree::Drop { parent, index })
            .unwrap();
        assert!(pos(&tree, normal) < pos(&tree, bump), "{:?}", order(&tree));
        assert_eq!(surface(&tree), over);
        // Without UVs the bump still applies.
        assert!(over.without_uvs().bump.is_some());
    }

    #[test]
    fn default_tree_has_no_relief() {
        let p = surface(&ShaderTree::default());
        assert_eq!((p.bump, p.normal_map, p.displacement), (None, None, None));
    }
}
