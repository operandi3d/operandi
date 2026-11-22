//! OpenPBR Surface 1.1.2 as Operandi's material model (OPM-01..04,
//! docs/specs/openpbr-materialx; the slot check there overrides the spec).
//!
//! Three tables live here, all from public documentation (the OpenPBR
//! specification and its MaterialX node definition, Apache-2.0, and
//! Cycles' Principled BSDF node definition):
//!
//! - [`Param`]: every OpenPBR input with its type and default.
//! - [`CHANNELS`]: each Material panel channel's OpenPBR identity, or
//!   "Operandi-only" (OPM-01). The UI names are unchanged.
//! - [`cycles_target`]: where each OpenPBR input lands on Cycles'
//!   Principled BSDF (OPM-03).
//!
//! The render path is unchanged: a Material's channels go to Cycles
//! through `shader_eval::Surface::principled`, which stays the reference
//! (OPM-01). [`from_material`] and [`to_material`] convert constants both
//! ways for MaterialX (Stage B), and a test holds the two routes to Cycles
//! to each other.
//!
//! Units in [`Values`]: lengths in metres (OpenPBR's default unit is the
//! centimetre; the MaterialX reader converts), the thin film in
//! micrometres as OpenPBR has it, emission in nits.
use crate::shader_eval::{Ch, Expr, Input};
use crate::shader_tree::{MaterialChannels, NodeId, NodeKind, ShadingModel, TextureEffect};
use std::collections::BTreeMap;

/// The OpenPBR version these tables follow (OPM-Q06).
pub const OPENPBR_VERSION: &str = "1.1.2";

/// Cycles Emission Strength per nit of emission_luminance. MaterialX's
/// reference implementation emits colour × luminance as radiance, so one
/// nit is one unit of Cycles' emission (OPM-Q09, decided by Jason
/// 2026-11-11: bright OpenPBR emitters such as 10 000-nit bulbs burn out
/// in a default studio, as they do in MaterialX's own viewer).
pub const STRENGTH_PER_NIT: f64 = 1.0;

/// OpenPBR's length unit when a file gives none: the centimetre.
pub const DEFAULT_LENGTH_IN_METRES: f64 = 0.01;

/// Cycles' Thin Film Thickness is in nanometres, OpenPBR's in micrometres.
pub const NANOMETRES_PER_MICROMETRE: f64 = 1000.;

/// The straight-on reflectance Cycles gives Specular IOR Level 0.5 at
/// IOR 1.5, the level every surface reflects with unless Use Refractive
/// Index is on (shader_eval `SPECULAR_IOR`).
fn f0(ior: f64) -> f64 {
    let r = (ior - 1.) / (ior + 1.);
    r * r
}

macro_rules! params {
    ($($variant:ident => $name:literal, $kind:ident, $default:expr;)+) => {
        /// The inputs of open_pbr_surface 1.1.2, in the node definition's order.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Param { $($variant),+ }
        impl Param {
            pub const ALL: &[Param] = &[$(Param::$variant),+];
            /// The MaterialX input name.
            pub fn name(self) -> &'static str {
                match self { $(Param::$variant => $name),+ }
            }
            pub fn kind(self) -> Kind {
                match self { $(Param::$variant => Kind::$kind),+ }
            }
            /// OpenPBR's default (geometry inputs: the surface's own).
            pub fn default(self) -> Value {
                match self { $(Param::$variant => $default),+ }
            }
        }
    };
}

/// An input's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Float,
    Color3,
    Boolean,
    /// A direction from the geometry (normal, tangent): only images
    /// drive these, through `normalmap`.
    Vector3,
}

/// A constant input value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Float(f64),
    Color([f64; 3]),
    Bool(bool),
    /// The geometry's own normal or tangent.
    Geometry,
}
impl Value {
    pub fn float(self) -> f64 {
        match self {
            Value::Float(v) => v,
            Value::Color(c) => c[0],
            Value::Bool(b) => f64::from(u8::from(b)),
            Value::Geometry => 0.,
        }
    }
    pub fn color(self) -> [f64; 3] {
        match self {
            Value::Color(c) => c,
            Value::Float(v) => [v; 3],
            Value::Bool(b) => [f64::from(u8::from(b)); 3],
            Value::Geometry => [0.; 3],
        }
    }
    pub fn bool(self) -> bool {
        match self {
            Value::Bool(b) => b,
            Value::Float(v) => v != 0.,
            Value::Color(c) => c.iter().any(|v| *v != 0.),
            Value::Geometry => false,
        }
    }
}

use Value::{Color as C, Float as F};
params! {
    BaseWeight => "base_weight", Float, F(1.);
    BaseColor => "base_color", Color3, C([0.8; 3]);
    BaseDiffuseRoughness => "base_diffuse_roughness", Float, F(0.);
    BaseMetalness => "base_metalness", Float, F(0.);
    SpecularWeight => "specular_weight", Float, F(1.);
    SpecularColor => "specular_color", Color3, C([1.; 3]);
    SpecularRoughness => "specular_roughness", Float, F(0.3);
    SpecularIor => "specular_ior", Float, F(1.5);
    SpecularRoughnessAnisotropy => "specular_roughness_anisotropy", Float, F(0.);
    TransmissionWeight => "transmission_weight", Float, F(0.);
    TransmissionColor => "transmission_color", Color3, C([1.; 3]);
    TransmissionDepth => "transmission_depth", Float, F(0.);
    TransmissionScatter => "transmission_scatter", Color3, C([0.; 3]);
    TransmissionScatterAnisotropy => "transmission_scatter_anisotropy", Float, F(0.);
    TransmissionDispersionScale => "transmission_dispersion_scale", Float, F(0.);
    TransmissionDispersionAbbeNumber => "transmission_dispersion_abbe_number", Float, F(20.);
    SubsurfaceWeight => "subsurface_weight", Float, F(0.);
    SubsurfaceColor => "subsurface_color", Color3, C([0.8; 3]);
    // 1 cm, in metres.
    SubsurfaceRadius => "subsurface_radius", Float, F(DEFAULT_LENGTH_IN_METRES);
    SubsurfaceRadiusScale => "subsurface_radius_scale", Color3, C([1., 0.5, 0.25]);
    SubsurfaceScatterAnisotropy => "subsurface_scatter_anisotropy", Float, F(0.);
    FuzzWeight => "fuzz_weight", Float, F(0.);
    FuzzColor => "fuzz_color", Color3, C([1.; 3]);
    FuzzRoughness => "fuzz_roughness", Float, F(0.5);
    CoatWeight => "coat_weight", Float, F(0.);
    CoatColor => "coat_color", Color3, C([1.; 3]);
    CoatRoughness => "coat_roughness", Float, F(0.);
    CoatRoughnessAnisotropy => "coat_roughness_anisotropy", Float, F(0.);
    CoatIor => "coat_ior", Float, F(1.6);
    CoatDarkening => "coat_darkening", Float, F(1.);
    ThinFilmWeight => "thin_film_weight", Float, F(0.);
    ThinFilmThickness => "thin_film_thickness", Float, F(0.5);
    ThinFilmIor => "thin_film_ior", Float, F(1.4);
    EmissionLuminance => "emission_luminance", Float, F(0.);
    EmissionColor => "emission_color", Color3, C([1.; 3]);
    GeometryOpacity => "geometry_opacity", Float, F(1.);
    GeometryThinWalled => "geometry_thin_walled", Boolean, Value::Bool(false);
    GeometryNormal => "geometry_normal", Vector3, Value::Geometry;
    GeometryCoatNormal => "geometry_coat_normal", Vector3, Value::Geometry;
    GeometryTangent => "geometry_tangent", Vector3, Value::Geometry;
    GeometryCoatTangent => "geometry_coat_tangent", Vector3, Value::Geometry;
}
impl Param {
    pub fn from_name(name: &str) -> Option<Param> {
        Param::ALL.iter().copied().find(|p| p.name() == name)
    }
    /// Lengths, which a file may give in its own unit.
    pub fn is_length(self) -> bool {
        matches!(self, Param::TransmissionDepth | Param::SubsurfaceRadius)
    }
}

/// Constant OpenPBR inputs; missing ones are OpenPBR's defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Values(pub BTreeMap<Param, Value>);
impl Values {
    pub fn get(&self, p: Param) -> Value {
        self.0.get(&p).copied().unwrap_or_else(|| p.default())
    }
    pub fn set(&mut self, p: Param, v: Value) {
        self.0.insert(p, v);
    }
    fn f(&self, p: Param) -> f64 {
        self.get(p).float()
    }
    fn c(&self, p: Param) -> [f64; 3] {
        self.get(p).color()
    }
}

/// A Material channel's OpenPBR identity (OPM-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Identity {
    /// The channel is this input, with the conversion [`from_material`]
    /// and [`to_material`] apply.
    OpenPbr(Param, Conversion),
    /// No OpenPBR input: exports drop it (with a note when it changes
    /// the render), imports leave it at its default.
    OperandiOnly,
}

/// How a channel's value becomes the input's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {
    Same,
    /// Specular Amount × 2 = specular_weight (Amount 50 % is OpenPBR's
    /// neutral 1); see [`from_material`] for the index of refraction.
    Double,
    /// Dissolve = 1 − geometry_opacity.
    OneMinus,
    /// Subsurface Phase (0..1, 0.5 even) = (anisotropy + 1) / 2.
    Phase,
    /// Luminous Intensity × [`STRENGTH_PER_NIT`]⁻¹ = emission_luminance.
    Nits,
    /// Clearcoat Bump 100 % = the coat follows geometry_normal (the coat
    /// normal is the surface's); below that it has no OpenPBR form.
    CoatFollowsNormal,
    /// The Refractive Index is specular_ior only for see-through
    /// surfaces and Use Refractive Index (see [`from_material`]).
    Ior,
}

/// Every Material panel channel (`MaterialChannels::channels`) with its
/// OpenPBR identity. A test checks that the two lists match.
pub const CHANNELS: &[(&str, Identity)] = {
    use Conversion::*;
    use Identity::{OpenPbr as O, OperandiOnly as X};
    use Param::*;
    &[
        ("GL Display", X),
        ("Diffuse Color", O(BaseColor, Same)),
        // Principled ignores Diffuse Amount; Physically Based multiplies
        // the colour by it, which is base_weight.
        ("Diffuse Amount", O(BaseWeight, Same)),
        ("Diffuse Roughness", O(BaseDiffuseRoughness, Same)),
        ("Specular Color", O(SpecularColor, Same)),
        ("Specular Amount", O(SpecularWeight, Double)),
        ("Specular Fresnel", X),
        ("Roughness", O(SpecularRoughness, Same)),
        ("Anisotropy", O(SpecularRoughnessAnisotropy, Same)),
        ("Match Specular", X),
        ("Reflection Color", X),
        ("Reflection Amount", X),
        ("Reflection Fresnel", X),
        ("Clearcoat Amount", O(CoatWeight, Same)),
        ("Clearcoat Roughness", O(CoatRoughness, Same)),
        ("Clearcoat Bump", O(GeometryCoatNormal, CoatFollowsNormal)),
        ("Coat Color", O(CoatColor, Same)),
        ("Coat IOR", O(CoatIor, Same)),
        ("Coat Darkening", O(CoatDarkening, Same)),
        ("Coat Anisotropy", O(CoatRoughnessAnisotropy, Same)),
        ("Metallic", O(BaseMetalness, Same)),
        ("Sheen", O(FuzzWeight, Same)),
        ("Sheen Tint", X),
        ("Sheen Color", O(FuzzColor, Same)),
        ("Sheen Roughness", O(FuzzRoughness, Same)),
        ("Specular Tint", X),
        ("Flatness", X),
        ("Thin Film", O(ThinFilmWeight, Same)),
        ("Thin Film Thickness", O(ThinFilmThickness, Same)),
        ("Thin Film IOR", O(ThinFilmIor, Same)),
        ("Transparent Color", O(TransmissionColor, Same)),
        ("Transparent Amount", O(TransmissionWeight, Same)),
        ("Refractive Index", O(SpecularIor, Ior)),
        // OpenPBR refracts with specular_roughness.
        ("Refraction Roughness", X),
        ("Absorption Distance", O(TransmissionDepth, Same)),
        ("Transmission Scatter", O(TransmissionScatter, Same)),
        ("Scatter Anisotropy", O(TransmissionScatterAnisotropy, Same)),
        ("Thin Walled", O(GeometryThinWalled, Same)),
        ("Dissolve", O(GeometryOpacity, OneMinus)),
        // Cycles' Subsurface Radius colour: the per-channel scale.
        ("Subsurface Color", O(SubsurfaceRadiusScale, Same)),
        ("Subsurface Amount", O(SubsurfaceWeight, Same)),
        ("Subsurface Distance", O(SubsurfaceRadius, Same)),
        // On: subsurface_color is the Diffuse Color (older files).
        ("Albedo from Diffuse", X),
        ("Subsurface Albedo", O(SubsurfaceColor, Same)),
        ("Luminous Color", O(EmissionColor, Same)),
        ("Luminous Intensity", O(EmissionLuminance, Nits)),
        ("Bump Amplitude", X),
        ("Displacement Distance", X),
        ("Smoothing", X),
        ("Smoothing Angle", X),
        ("Double Sided", X),
        ("Use Refractive Index", X),
        ("Exit Color", X),
        ("Dispersion", O(TransmissionDispersionScale, Same)),
        ("Abbe Number", O(TransmissionDispersionAbbeNumber, Same)),
        ("Subsurface Depth", X),
        ("Subsurface Phase", O(SubsurfaceScatterAnisotropy, Phase)),
        ("Rounded Edge Width", X),
        ("Rounded Edge Angle", X),
        ("Round Same Surface Only", X),
        ("Clip Enable", X),
        ("Matte", X),
        ("Matte Color", X),
        ("Clip Value", X),
        ("Importance", X),
        ("Reflection Rays", X),
        ("Refraction Rays", X),
        ("Subsurface Samples", X),
        ("Blurry Reflection", X),
        ("Same Surface", X),
        ("GL Color Override", X),
        ("GL Color", X),
        ("Volume Scattering", X),
        ("Volume Absorption", X),
        ("Volume Density", X),
        ("Volume Level", X),
        ("Volume Luminous", X),
    ]
};

/// One UIC-92 dependency on the Material panel: while `off` holds, the
/// `rows` have no effect. `controls[0]` is the row shown as the reason and
/// sits above the rows (UIC-93); a texture layer driving any of `controls`
/// can turn them back on, so the rows stay live then (Jason, 2026-11-12).
pub struct Gate {
    pub controls: &'static [&'static str],
    pub rows: &'static [&'static str],
    pub off: fn(&MaterialChannels) -> bool,
    pub reason: &'static str,
}

/// The clear coat is off: no Clearcoat Amount and nothing from Reflection
/// either (with Match Specular off, Reflection Amount × Reflection Fresnel
/// adds to the coat; see `shader_eval`).
fn coat_off(m: &MaterialChannels) -> bool {
    m.clearcoat_amount == 0.
        && (m.match_specular || m.reflection_amount == 0. || m.reflection_fresnel == 0.)
}

/// Every greyed-row rule on the Material panel, older rows and OpenPBR
/// rows alike. Rows whose controlling row sits below them (Specular Color
/// under Specular Amount, Luminous Color under Luminous Intensity,
/// Transparent Color under Transparent Amount, Subsurface Color under
/// Subsurface Amount, Refractive Index and Specular Amount under Use
/// Refractive Index) are left live: UIC-93 needs the control above.
pub const GATES: &[Gate] = &[
    Gate {
        controls: &["Match Specular"],
        rows: &[
            "Reflection Color",
            "Reflection Amount",
            "Reflection Fresnel",
        ],
        off: |m| m.match_specular,
        reason: "No effect while Match Specular is on",
    },
    Gate {
        controls: &["Reflection Amount"],
        rows: &["Reflection Fresnel"],
        off: |m| m.reflection_amount == 0.,
        reason: "No effect while Reflection Amount is 0 %",
    },
    Gate {
        controls: &[
            "Clearcoat Amount",
            "Reflection Amount",
            "Reflection Fresnel",
        ],
        rows: &[
            "Clearcoat Roughness",
            "Clearcoat Bump",
            "Coat Color",
            "Coat IOR",
            "Coat Darkening",
            "Coat Anisotropy",
        ],
        off: coat_off,
        reason: "No effect while Clearcoat Amount is 0 %",
    },
    Gate {
        controls: &["Sheen"],
        rows: &["Sheen Tint", "Sheen Color", "Sheen Roughness"],
        off: |m| m.sheen == 0.,
        reason: "No effect while Sheen is 0 %",
    },
    Gate {
        controls: &["Thin Film"],
        rows: &["Thin Film Thickness", "Thin Film IOR"],
        off: |m| m.openpbr.thin_film_weight == 0.,
        reason: "No effect while Thin Film is 0 %",
    },
    Gate {
        controls: &["Transparent Amount"],
        rows: &[
            "Refraction Roughness",
            "Absorption Distance",
            "Transmission Scatter",
            "Scatter Anisotropy",
            "Exit Color",
            "Dispersion",
            "Abbe Number",
        ],
        off: |m| m.transparent_amount == 0.,
        reason: "No effect while Transparent Amount is 0 %",
    },
    Gate {
        controls: &["Transmission Scatter"],
        rows: &["Scatter Anisotropy"],
        off: |m| m.openpbr.transmission_scatter == [0.; 3],
        reason: "No effect while Transmission Scatter is black",
    },
    Gate {
        controls: &["Subsurface Amount"],
        rows: &[
            "Subsurface Distance",
            "Albedo from Diffuse",
            "Subsurface Albedo",
            "Subsurface Depth",
            "Subsurface Phase",
            "Subsurface Samples",
        ],
        off: |m| m.subsurface_amount == 0.,
        reason: "No effect while Subsurface Amount is 0 %",
    },
    Gate {
        controls: &["Albedo from Diffuse"],
        rows: &["Subsurface Albedo"],
        off: |m| m.openpbr.subsurface_uses_diffuse,
        reason: "No effect while Albedo from Diffuse is on",
    },
    Gate {
        controls: &["Smoothing"],
        rows: &["Smoothing Angle"],
        off: |m| m.smoothing == 0.,
        reason: "No effect while Smoothing is 0 %",
    },
    Gate {
        controls: &["Dispersion"],
        rows: &["Abbe Number"],
        off: |m| m.more.dispersion == 0.,
        reason: "No effect while Dispersion is 0 %",
    },
    Gate {
        controls: &["Rounded Edge Width"],
        rows: &["Rounded Edge Angle", "Round Same Surface Only"],
        off: |m| m.more.rounded_edge_width == 0.,
        reason: "No effect while Rounded Edge Width is 0",
    },
    Gate {
        controls: &["Clip Enable"],
        rows: &["Clip Value"],
        off: |m| !m.more.clip_enable,
        reason: "No effect while Clip Enable is off",
    },
    Gate {
        controls: &["Matte"],
        rows: &["Matte Color"],
        off: |m| !m.more.matte,
        reason: "No effect while Matte is off",
    },
    Gate {
        controls: &["GL Color Override"],
        rows: &["GL Color"],
        off: |m| !m.more.gl_color_override,
        reason: "No effect while GL Color Override is off",
    },
];

/// UIC-92: why Material panel row `name` has no effect at the moment (its
/// controlling row, above it, turns it off), `None` when it does. Greyed
/// rows stay where they are and keep their values.
pub fn inert_reason(m: &MaterialChannels, name: &str) -> Option<&'static str> {
    inert_reason_driven(m, name, &|_| false)
}

/// [`inert_reason`], where `driven(row)` says a texture layer drives that
/// row's channel. A driven row stays live, and so do the rows of a
/// controlling row that is driven: the texture can turn them on even while
/// the slider is 0 (Jason, 2026-11-12).
pub fn inert_reason_driven(
    m: &MaterialChannels,
    name: &str,
    driven: &dyn Fn(&str) -> bool,
) -> Option<&'static str> {
    if driven(name) {
        return None;
    }
    GATES
        .iter()
        .find(|g| g.rows.contains(&name) && (g.off)(m) && !g.controls.iter().any(|c| driven(c)))
        .map(|g| g.reason)
}

/// The Material panel rows that enabled texture layers drive for material
/// `node`: those in its group or any group around it (wherever they sit
/// in it, so a row never greys while a texture may be feeding it).
pub fn driven_rows(tree: &crate::shader_tree::ShaderTree, node: NodeId) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut parent = tree.get(node).map(|n| n.parent);
    while let Some(p) = parent {
        for n in tree.children(p) {
            if let NodeKind::Texture(t) = &n.kind
                && n.layer.enable
                && let Some(row) = effect_row(t.effect)
                && !out.contains(&row)
            {
                out.push(row);
            }
        }
        parent = p.and_then(|p| tree.get(p)).map(|n| n.parent);
    }
    out
}

/// The Material panel row a texture effect drives.
pub fn effect_row(effect: TextureEffect) -> Option<&'static str> {
    match effect {
        TextureEffect::LuminousAmount => Some("Luminous Intensity"),
        e => channel_identity(e.label()).map(|_| e.label()),
    }
}

/// A channel's identity by its panel name.
pub fn channel_identity(name: &str) -> Option<Identity> {
    CHANNELS.iter().find(|(n, _)| *n == name).map(|(_, i)| *i)
}

impl TextureEffect {
    /// The OpenPBR input a texture layer with this effect drives
    /// (OPM-04), `None` for Operandi-only effects (drivers, masks,
    /// relief...). Geometry normal and opacity count: a Normal layer is
    /// geometry_normal, Dissolve is 1 − geometry_opacity.
    pub fn openpbr_name(self) -> Option<&'static str> {
        use TextureEffect as E;
        let p = match self {
            E::DiffuseColor => Param::BaseColor,
            E::DiffuseAmount => Param::BaseWeight,
            E::DiffuseRoughness => Param::BaseDiffuseRoughness,
            E::SpecularColor => Param::SpecularColor,
            E::SpecularAmount => Param::SpecularWeight,
            E::Roughness => Param::SpecularRoughness,
            E::ClearcoatAmount => Param::CoatWeight,
            E::TransparentColor => Param::TransmissionColor,
            E::TransparentAmount => Param::TransmissionWeight,
            E::SubsurfaceColor => Param::SubsurfaceRadiusScale,
            E::SubsurfaceAmount => Param::SubsurfaceWeight,
            E::LuminousColor => Param::EmissionColor,
            E::LuminousAmount => Param::EmissionLuminance,
            E::Dissolve => Param::GeometryOpacity,
            E::Metallic => Param::BaseMetalness,
            E::Sheen => Param::FuzzWeight,
            E::Normal => Param::GeometryNormal,
            E::AnisotropyDirection => Param::GeometryTangent,
            _ => return None,
        };
        Some(p.name())
    }
}

/// Where an OpenPBR input goes on Cycles' Principled BSDF (OPM-03),
/// checked against the pinned Cycles' node definition (OPM-Q07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cycles {
    /// Straight onto this socket.
    Socket(&'static str),
    /// Onto a socket after a conversion (described).
    Converted(&'static str, &'static str),
    /// Outside the principled BSDF: the see-through mix, the interior
    /// volume, the relief.
    Elsewhere(&'static str),
    /// No Cycles form: stored, and noted on import.
    Unsupported(&'static str),
}

pub fn cycles_target(p: Param) -> Cycles {
    use Cycles::*;
    use Param::*;
    match p {
        BaseWeight => Converted("Base Color", "multiplies base_color"),
        BaseColor => Socket("Base Color"),
        BaseDiffuseRoughness => Socket("Diffuse Roughness"),
        BaseMetalness => Socket("Metallic"),
        SpecularWeight => Converted("Specular IOR Level", "weight / 2"),
        SpecularColor => Socket("Specular Tint"),
        SpecularRoughness => Socket("Roughness"),
        SpecularIor => Socket("IOR"),
        SpecularRoughnessAnisotropy => Socket("Anisotropic"),
        TransmissionWeight => Elsewhere("Mix Shader factor with a Glass BSDF"),
        TransmissionColor => Elsewhere("the Glass BSDF's Color"),
        TransmissionDepth => Elsewhere("Volume Absorption density -ln(colour) / depth"),
        TransmissionScatter => Elsewhere("Volume Scatter density scatter / depth"),
        TransmissionScatterAnisotropy => Elsewhere("Volume Scatter Anisotropy"),
        TransmissionDispersionScale => Socket("Transmission Dispersion Scale"),
        TransmissionDispersionAbbeNumber => Socket("Transmission Dispersion Abbe Number"),
        SubsurfaceWeight => Socket("Subsurface Weight"),
        SubsurfaceColor => Converted("Base Color", "mixed in by subsurface_weight"),
        SubsurfaceRadius => Socket("Subsurface Scale"),
        SubsurfaceRadiusScale => Socket("Subsurface Radius"),
        SubsurfaceScatterAnisotropy => Socket("Subsurface Anisotropy"),
        FuzzWeight => Socket("Sheen Weight"),
        FuzzColor => Socket("Sheen Tint"),
        FuzzRoughness => Socket("Sheen Roughness"),
        CoatWeight => Socket("Coat Weight"),
        CoatColor => Socket("Coat Tint"),
        CoatRoughness => Socket("Coat Roughness"),
        CoatRoughnessAnisotropy => Unsupported("Cycles' coat is isotropic"),
        CoatIor => Socket("Coat IOR"),
        CoatDarkening => Unsupported("Cycles always darkens under the coat"),
        ThinFilmWeight => Converted("Thin Film Thickness", "0 when the weight is 0"),
        ThinFilmThickness => Converted("Thin Film Thickness", "micrometres x 1000"),
        ThinFilmIor => Socket("Thin Film IOR"),
        EmissionLuminance => Converted("Emission Strength", "nits x STRENGTH_PER_NIT"),
        EmissionColor => Socket("Emission Color"),
        GeometryOpacity => Socket("Alpha"),
        GeometryThinWalled => Socket("Thin Wall"),
        GeometryNormal => Socket("Normal"),
        GeometryCoatNormal => Socket("Coat Normal"),
        GeometryTangent => Socket("Tangent"),
        GeometryCoatTangent => Unsupported("Cycles' coat has no tangent"),
    }
}

/// The OpenPBR inputs the older translation lacks, added to a surface's
/// principled inputs (called by `Surface::principled`). Each is given
/// only when it differs from Cycles' default, so a surface that leaves
/// them alone translates exactly as before (OPM-T1).
pub(crate) fn cycles_extras(c: &impl Fn(Ch) -> Expr, inputs: &mut Vec<(Input, Expr)>) {
    let mut add = |input: Input, e: Expr, cycles_default: f64| {
        if e != Expr::scalar(cycles_default) {
            inputs.push((input, e));
        }
    };
    add(Input::CoatTint, c(Ch::CoatColor), 1.);
    add(Input::CoatIor, c(Ch::CoatIor), 1.5);
    add(Input::SheenTint, c(Ch::FuzzColor), 1.);
    add(Input::SheenRoughness, c(Ch::FuzzRoughness), 0.5);
    // Cycles has no film coverage: any weight above 0 is the whole film
    // (provisional, OPM-Q08).
    let weight = c(Ch::ThinFilmWeight);
    let thickness = Expr::times(
        c(Ch::ThinFilmThickness),
        Expr::scalar(NANOMETRES_PER_MICROMETRE),
    );
    let film = match weight.constant() {
        Some(w) if w[0] > 0. => thickness,
        Some(_) => Expr::scalar(0.),
        None => Expr::times(thickness, Expr::step_above(weight, 0.)),
    };
    let has_film = film != Expr::scalar(0.);
    add(Input::ThinFilmThickness, film, 0.);
    if has_film {
        add(Input::ThinFilmIor, c(Ch::ThinFilmIor), 1.33);
    }
    let scale = c(Ch::Dispersion);
    if scale != Expr::scalar(0.) {
        add(Input::DispersionScale, scale, 0.);
        add(Input::DispersionAbbe, c(Ch::DispersionAbbe), 20.);
    }
}

/// The inside of a see-through Glass from Transparent Color `t` after
/// `depth` metres and the OpenPBR scatter colour (OpenPBR §transmission):
/// extinction mu_t = -ln(t) / depth, scattering mu_s = scatter / depth,
/// absorption mu_a = mu_t - mu_s, shifted up by grey if any part is
/// negative. Returned as Cycles' Volume Absorption (colour, density:
/// absorbs density × (1 − colour)) and Volume Scatter (colour, density:
/// scatters density × colour, anisotropy). With no scatter this is the
/// older absorption exactly.
#[allow(clippy::type_complexity)]
pub(crate) fn interior_volume(
    t: [f64; 3],
    scatter: [f64; 3],
    anisotropy: f64,
    depth: f64,
) -> (Option<([f64; 3], f64)>, Option<([f64; 3], f64, f64)>) {
    let mu_t = t.map(|v| -(v.clamp(1e-4, 1.)).ln() / depth);
    let mu_s = scatter.map(|v| v.max(0.) / depth);
    let mut mu_a = [0., 1., 2.].map(|i: f64| mu_t[i as usize] - mu_s[i as usize]);
    let lowest = mu_a.iter().copied().fold(f64::MAX, f64::min);
    if lowest < 0. {
        mu_a = mu_a.map(|v| v - lowest);
    }
    let density = mu_a.iter().copied().fold(0., f64::max);
    let absorption = (density > 0.).then(|| (mu_a.map(|s| 1. - s / density), density));
    let s_density = mu_s.iter().copied().fold(0., f64::max);
    let scattering = (s_density > 0.).then(|| {
        (
            mu_s.map(|s| s / s_density),
            s_density,
            anisotropy.clamp(-1., 1.),
        )
    });
    (absorption, scattering)
}

/// Is this surface see-through as OpenPBR sees it, so its index of
/// refraction is specular_ior?
fn see_through(m: &MaterialChannels) -> bool {
    m.transparent_amount > 0.
}

/// A Material's channels as OpenPBR constants, with notes on what
/// renders in Operandi but has no OpenPBR form (the export's losses).
pub fn from_material(m: &MaterialChannels) -> (Values, Vec<String>) {
    use Param::*;
    let mut v = Values::default();
    let mut losses = Vec::new();
    let o = &m.openpbr;
    let (base_weight, base_color) = match m.shading_model {
        ShadingModel::PhysicallyBased => (m.diffuse_amount, m.diffuse_color),
        _ => (1., m.diffuse_color),
    };
    v.set(BaseWeight, F(base_weight));
    v.set(BaseColor, C(base_color));
    v.set(BaseDiffuseRoughness, F(m.diffuse_roughness));
    v.set(BaseMetalness, F(m.metallic));
    // The index and Specular Amount (see the module notes in
    // `to_material`): Cycles reflects with IOR 1.5 at level Amount, or
    // with the Refractive Index at level 0.5 when Use Refractive Index is
    // on; a see-through surface refracts with the Refractive Index.
    let (ior, weight) = if m.more.use_refractive_index {
        (m.refractive_index, 1.)
    } else if see_through(m) && m.refractive_index > 1. {
        // Same reflectance straight on: weight × F0(n) = 2 Amount × F0(1.5).
        (
            m.refractive_index,
            2. * m.specular_amount * f0(1.5) / f0(m.refractive_index),
        )
    } else {
        (1.5, 2. * m.specular_amount)
    };
    v.set(SpecularIor, F(ior));
    v.set(SpecularWeight, F(weight));
    v.set(SpecularColor, C(m.specular_color));
    v.set(SpecularRoughness, F(m.roughness));
    v.set(SpecularRoughnessAnisotropy, F(m.anisotropy));
    v.set(TransmissionWeight, F(m.transparent_amount));
    v.set(TransmissionColor, C(m.transparent_color));
    v.set(TransmissionDepth, F(m.absorption_distance));
    v.set(TransmissionScatter, C(o.transmission_scatter));
    v.set(
        TransmissionScatterAnisotropy,
        F(o.transmission_scatter_anisotropy),
    );
    v.set(TransmissionDispersionScale, F(m.more.dispersion));
    v.set(
        TransmissionDispersionAbbeNumber,
        F(o.dispersion_abbe_number),
    );
    v.set(SubsurfaceWeight, F(m.subsurface_amount));
    v.set(
        SubsurfaceColor,
        C(if o.subsurface_uses_diffuse {
            base_color
        } else {
            o.subsurface_albedo
        }),
    );
    v.set(SubsurfaceRadius, F(m.subsurface_distance));
    v.set(SubsurfaceRadiusScale, C(m.subsurface_color));
    v.set(
        SubsurfaceScatterAnisotropy,
        F(2. * (m.more.subsurface_phase - 0.5)),
    );
    v.set(FuzzWeight, F(m.sheen));
    v.set(FuzzColor, C(o.fuzz_color));
    v.set(FuzzRoughness, F(o.fuzz_roughness));
    v.set(CoatWeight, F(m.clearcoat_amount));
    v.set(CoatColor, C(o.coat_color));
    v.set(CoatRoughness, F(m.clearcoat_roughness));
    v.set(CoatRoughnessAnisotropy, F(o.coat_roughness_anisotropy));
    v.set(CoatIor, F(o.coat_ior));
    v.set(CoatDarkening, F(o.coat_darkening));
    v.set(ThinFilmWeight, F(o.thin_film_weight));
    v.set(ThinFilmThickness, F(o.thin_film_thickness));
    v.set(ThinFilmIor, F(o.thin_film_ior));
    v.set(
        EmissionLuminance,
        F(m.luminous_intensity / STRENGTH_PER_NIT),
    );
    v.set(EmissionColor, C(m.luminous_color));
    v.set(GeometryOpacity, F(1. - m.dissolve));
    v.set(GeometryThinWalled, Value::Bool(o.thin_walled));

    if !m.match_specular && m.reflection_amount > 0. {
        losses.push("Reflection Amount (Match Specular off)".into());
    }
    if see_through(m) && (m.refraction_roughness - m.roughness).abs() > 1e-9 {
        losses.push("Refraction Roughness (OpenPBR refracts with the Roughness)".into());
    }
    if m.clearcoat_amount > 0. && m.clearcoat_bump < 1. {
        losses.push("Clearcoat Bump below 100 %".into());
    }
    if m.more.rounded_edge_width > 0. {
        losses.push("Rounded Edge Width".into());
    }
    (v, losses)
}

/// OpenPBR constants as Material channels (an import's material), with
/// notes on what Cycles can't show. Starts from a new material, so
/// Operandi-only channels keep their defaults.
///
/// The index of refraction: Cycles' principled reflection takes IOR 1.5
/// at level Specular Amount, or the Refractive Index at level 0.5 with
/// Use Refractive Index on. specular_ior 1.5 sets Specular Amount =
/// weight / 2; another index turns Use Refractive Index on (exact for
/// weight 1). A see-through surface with a weight other than 1 refracts
/// with the index and reflects at the level giving the same reflectance
/// straight on, as [`from_material`] writes it, unless that's more than
/// IOR 1.5 can reflect (a diamond).
pub fn to_material(v: &Values) -> (MaterialChannels, Vec<String>) {
    use Param::*;
    let mut m = MaterialChannels::default();
    let mut notes = Vec::new();
    m.shading_model = ShadingModel::Principled;
    // base_weight folds into the colour: Principled ignores Diffuse
    // Amount, so it is 100 %.
    m.diffuse_color = v.c(BaseColor).map(|c| (c * v.f(BaseWeight)).clamp(0., 1.));
    m.diffuse_amount = 1.;
    m.diffuse_roughness = unit(v.f(BaseDiffuseRoughness));
    m.metallic = unit(v.f(BaseMetalness));
    let ior = v.f(SpecularIor).max(1.);
    let weight = v.f(SpecularWeight).max(0.);
    m.refractive_index = ior;
    let level = weight / 2. * f0(ior) / f0(1.5);
    if v.f(TransmissionWeight) > 0. && (weight - 1.).abs() > 1e-6 && level <= 1. {
        m.specular_amount = unit(level);
    } else if v.f(TransmissionWeight) > 0. && (ior - 1.5).abs() >= 1e-6 {
        // Weight 1, or more than IOR 1.5 can reflect: the index itself.
        m.more.use_refractive_index = true;
        if (weight - 1.).abs() > 1e-6 {
            notes.push(format!(
                "specular_weight {weight:.3} with specular_ior {ior:.3}: Cycles reflects at weight 1"
            ));
        }
    } else if v.f(TransmissionWeight) > 0. {
        m.specular_amount = unit(level);
    } else if (ior - 1.5).abs() < 1e-6 {
        m.specular_amount = unit(weight / 2.);
        if weight > 2. {
            notes.push("specular_weight above 2 capped".into());
        }
    } else {
        m.more.use_refractive_index = true;
        if (weight - 1.).abs() > 1e-6 {
            notes.push(format!(
                "specular_weight {weight:.3} with specular_ior {ior:.3}: Cycles reflects at weight 1"
            ));
        }
    }
    m.specular_color = v.c(SpecularColor).map(unit);
    m.roughness = unit(v.f(SpecularRoughness));
    m.refraction_roughness = m.roughness;
    m.anisotropy = unit(v.f(SpecularRoughnessAnisotropy));
    m.transparent_amount = unit(v.f(TransmissionWeight));
    m.transparent_color = v.c(TransmissionColor).map(unit);
    m.absorption_distance = v.f(TransmissionDepth).max(0.);
    let o = &mut m.openpbr;
    o.transmission_scatter = v.c(TransmissionScatter).map(unit);
    o.transmission_scatter_anisotropy = v.f(TransmissionScatterAnisotropy).clamp(-1., 1.);
    m.more.dispersion = unit(v.f(TransmissionDispersionScale));
    o.dispersion_abbe_number = v.f(TransmissionDispersionAbbeNumber).clamp(1., 1000.);
    m.subsurface_amount = unit(v.f(SubsurfaceWeight));
    o.subsurface_albedo = v.c(SubsurfaceColor).map(unit);
    o.subsurface_uses_diffuse = false;
    m.subsurface_distance = v.f(SubsurfaceRadius).max(0.);
    m.subsurface_color = v.c(SubsurfaceRadiusScale).map(unit);
    m.more.subsurface_phase = (v.f(SubsurfaceScatterAnisotropy).clamp(-1., 1.) + 1.) / 2.;
    m.sheen = unit(v.f(FuzzWeight));
    o.fuzz_color = v.c(FuzzColor).map(unit);
    o.fuzz_roughness = unit(v.f(FuzzRoughness));
    m.clearcoat_amount = unit(v.f(CoatWeight));
    o.coat_color = v.c(CoatColor).map(unit);
    m.clearcoat_roughness = unit(v.f(CoatRoughness));
    o.coat_roughness_anisotropy = unit(v.f(CoatRoughnessAnisotropy));
    o.coat_ior = v.f(CoatIor).clamp(1., 10.);
    o.coat_darkening = unit(v.f(CoatDarkening));
    o.thin_film_weight = unit(v.f(ThinFilmWeight));
    o.thin_film_thickness = v.f(ThinFilmThickness).clamp(0., 100.);
    o.thin_film_ior = v.f(ThinFilmIor).clamp(1., 10.);
    m.luminous_intensity = (v.f(EmissionLuminance) * STRENGTH_PER_NIT).clamp(0., 1e6);
    m.luminous_color = v.c(EmissionColor).map(unit);
    m.dissolve = unit(1. - v.f(GeometryOpacity));
    o.thin_walled = v.get(GeometryThinWalled).bool();

    if m.clearcoat_amount > 0. && o.coat_darkening < 1. {
        notes.push("coat_darkening below 1: Cycles always darkens under the coat".into());
    }
    if m.clearcoat_amount > 0. && o.coat_roughness_anisotropy > 0. {
        notes.push("coat_roughness_anisotropy: Cycles' coat is isotropic".into());
    }
    if o.thin_film_weight > 0. && o.thin_film_weight < 1. {
        notes.push("thin_film_weight between 0 and 1 renders as a whole film".into());
    }
    let colour_over = |p: Param| v.c(p).iter().any(|c| *c > 1. + 1e-3);
    if [BaseColor, EmissionColor, SpecularColor]
        .into_iter()
        .any(colour_over)
    {
        notes.push("colours above 1 clamped".into());
    }
    (m, notes)
}

fn unit(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0., 1.) } else { 0. }
}

/// OpenPBR constants straight onto Cycles' principled sockets
/// (OPM-03), by Cycles socket name: the documented table applied
/// without the Material channels. Tests hold it to the render path.
pub fn principled_values(v: &Values) -> Vec<(&'static str, [f64; 3])> {
    use Param::*;
    let s = |x: f64| [x; 3];
    let mut base = v.c(BaseColor).map(|c| c * v.f(BaseWeight));
    let ssw = v.f(SubsurfaceWeight);
    let albedo = v.c(SubsurfaceColor);
    base = [0, 1, 2].map(|i| base[i] + (albedo[i] - base[i]) * ssw);
    let mut out = vec![
        ("Base Color", base),
        ("Metallic", s(v.f(BaseMetalness))),
        ("Diffuse Roughness", s(v.f(BaseDiffuseRoughness))),
        ("Roughness", s(v.f(SpecularRoughness))),
        ("Specular Tint", v.c(SpecularColor)),
        ("Anisotropic", s(v.f(SpecularRoughnessAnisotropy))),
        ("Subsurface Weight", s(ssw)),
        ("Subsurface Radius", v.c(SubsurfaceRadiusScale)),
        ("Subsurface Scale", s(v.f(SubsurfaceRadius))),
        ("Sheen Weight", s(v.f(FuzzWeight))),
        ("Sheen Tint", v.c(FuzzColor)),
        ("Sheen Roughness", s(v.f(FuzzRoughness))),
        ("Coat Weight", s(v.f(CoatWeight))),
        ("Coat Tint", v.c(CoatColor)),
        ("Coat Roughness", s(v.f(CoatRoughness))),
        ("Coat IOR", s(v.f(CoatIor))),
        (
            "Emission Strength",
            s(v.f(EmissionLuminance) * STRENGTH_PER_NIT),
        ),
        ("Emission Color", v.c(EmissionColor)),
        ("Alpha", s(v.f(GeometryOpacity))),
        (
            "Thin Film Thickness",
            s(if v.f(ThinFilmWeight) > 0. {
                v.f(ThinFilmThickness) * NANOMETRES_PER_MICROMETRE
            } else {
                0.
            }),
        ),
        ("Thin Film IOR", s(v.f(ThinFilmIor))),
        (
            "Transmission Dispersion Scale",
            s(v.f(TransmissionDispersionScale)),
        ),
        (
            "Transmission Dispersion Abbe Number",
            s(v.f(TransmissionDispersionAbbeNumber)),
        ),
    ];
    let ior = v.f(SpecularIor);
    let weight = v.f(SpecularWeight);
    let see_through = v.f(TransmissionWeight) > 0.;
    let level = weight / 2. * f0(ior) / f0(1.5);
    let own_index =
        (ior - 1.5).abs() > 1e-6 && (!see_through || (weight - 1.).abs() <= 1e-6 || level > 1.);
    if own_index {
        out.push(("IOR", s(ior)));
        out.push(("Specular IOR Level", s(0.5)));
    } else {
        out.push(("IOR", s(1.5)));
        out.push((
            "Specular IOR Level",
            s(if see_through { level } else { weight / 2. }),
        ));
    }
    out
}

#[cfg(test)]
mod tests;
