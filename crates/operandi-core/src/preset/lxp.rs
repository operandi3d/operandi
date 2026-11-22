//! Modo material presets (`.lxp`) as Operandi Material presets
//! (BEHAVIOR-LXP-MATERIALS v1.1, stage 3 of the Modo content folder).
//!
//! The preset's items come from the black-box reader
//! (`lxo::preset_items`). The material's channels go to OpenPBR values by
//! the spec's §3 rules and from there onto a Material through the shared
//! OpenPBR map (`openpbr::to_material`, OPM-01..04). Texture layers map by
//! effect (§4.1) and type (§7.2) onto Operandi's own layers, which share
//! Modo's two-zone model and setting names. Anything that can't map is
//! left out with a line in the import note; plug-in layers are never
//! approximated (LXP-Q7, §7.3).
//!
//! Provisional readings are LXP-B.. in docs/SPECIFICATION_QUESTIONS.md.
use super::{Preset, PresetHeader, PresetKind};
use crate::gradient::{Curve, Gradient, GradientInput, Key, KeyShape, Unit};
use crate::lxo::preset_items::{self, PresetItem, PresetItems};
use crate::openpbr::{self, Param, Value as Pv, Values};
use crate::procedural::{
    CellularType, CheckerType, DotsType, GridType, NoiseSettings, NoiseType, Pattern, Procedural,
    ProceduralKind, Variation,
};
use crate::process_occlusion::{Occlusion, OcclusionType, Process};
use crate::scene::Scene;
use crate::shader_tree::{
    BlendMode, GroupMask, ImageSettings, Layer, MaterialChannels, Node, NodeKind, Projection,
    ShaderTree, ShadingModel, Swizzle, TextureEffect, TextureLayer, TextureSource, TwoZone,
};
use crate::texture_locator::{TextureLocator, Tiling};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The preset file's extension.
pub const EXTENSION: &str = "lxp";

/// Material item types a preset may hold (§1).
pub const MATERIAL_TYPES: [&str; 3] = ["advancedMaterial", "unrealShader", "unityShader"];

/// Whether `DESC`'s item type names a material preset (LXP-01, MC-12).
pub fn is_material_type(item_type: &str) -> bool {
    MATERIAL_TYPES
        .iter()
        .any(|t| t.eq_ignore_ascii_case(item_type))
        || item_type.to_ascii_lowercase().contains("material")
        || item_type.to_ascii_lowercase().contains("shader")
}

/// LXP-04's tooltip.
pub const NO_MATERIAL: &str = "Modo preset with no material we can read.";

/// A converted preset.
#[derive(Clone, Debug)]
pub struct Converted {
    pub preset: Preset,
    /// The import note, one line each (§2 step 5).
    pub notes: Vec<String>,
    /// Whether anything that changes the look was dropped or approximated
    /// (LXP-02's info dot). Render-only settings don't count (LXP-B03).
    pub changed_look: bool,
    /// Texture layers the preset holds, and how many were left out.
    pub layers: usize,
    pub dropped: usize,
    /// LXP-05: the look depends mainly on dropped layers.
    pub partial: bool,
}

/// Where to look for images the preset names (§4.2).
#[derive(Clone, Debug, Default)]
pub struct ImageSearch {
    /// The preset's own folder.
    pub folder: PathBuf,
    /// The content folders the user added.
    pub roots: Vec<PathBuf>,
}

/// Read and convert `path`. `size_of` gives a picture's pixel size (the
/// core can't read pictures) and `None` when it can't be read.
pub fn material_preset(
    path: &Path,
    roots: &[PathBuf],
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> Result<Converted, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let items = preset_items::read(&bytes).map_err(|e| e.to_string())?;
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Modo Material".into());
    let search = ImageSearch {
        folder: path.parent().map(Path::to_path_buf).unwrap_or_default(),
        roots: roots.to_vec(),
    };
    convert(&items, &name, &search, size_of)
}

/// A note line, and whether it changes the look.
struct Notes {
    lines: Vec<String>,
    look: bool,
}
impl Notes {
    fn look(&mut self, line: impl Into<String>) {
        self.look = true;
        self.push(line);
    }
    fn quiet(&mut self, line: impl Into<String>) {
        self.push(line);
    }
    fn push(&mut self, line: impl Into<String>) {
        let line = line.into();
        if !self.lines.contains(&line) {
            self.lines.push(line);
        }
    }
}

// ------------------------------------------------------------ materials

/// A number channel or `default`.
fn num(m: &PresetItem, key: &str, default: f64) -> f64 {
    m.number(key).filter(|v| v.is_finite()).unwrap_or(default)
}
fn col(m: &PresetItem, key: &str, default: [f64; 3]) -> [f64; 3] {
    m.colour(key)
        .or_else(|| m.number(key).map(|v| [v; 3]))
        .unwrap_or(default)
}
fn flag(m: &PresetItem, key: &str, default: bool) -> bool {
    match m.channels.get(key) {
        Some(v) => match v.number() {
            Some(n) => n != 0.,
            None => matches!(
                v.text().map(str::to_ascii_lowercase).as_deref(),
                Some("on" | "true" | "yes" | "1")
            ),
        },
        None => default,
    }
}
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// The index of refraction giving straight-on reflectance `f0` (§3.1:
/// IOR = (1 + √F0) / (1 − √F0); 0.04 gives 1.5). Capped at 20 (F0 0.82),
/// past which Cycles' reflection is a metal's anyway.
pub fn ior_from_f0(f0: f64) -> f64 {
    let s = f0.clamp(0., 0.8186).sqrt();
    ((1. + s) / (1. - s)).clamp(1., 20.)
}
fn f0_of(ior: f64) -> f64 {
    let r = (ior - 1.) / (ior + 1.);
    r * r
}

/// The shading model a Modo `brdfType` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brdf {
    /// `gtr`, Physically Based.
    PhysicallyBased,
    Principled,
    /// `ashikhmin` (Energy Conserving) and Traditional (§3.3).
    Legacy,
}
pub fn brdf(m: &PresetItem) -> Brdf {
    match m.channels.get("brdfType") {
        Some(v) => match v.text().map(str::to_ascii_lowercase).as_deref() {
            Some("gtr" | "physical" | "physicallybased") => Brdf::PhysicallyBased,
            Some(t) if t.contains("principled") => Brdf::Principled,
            Some(_) => Brdf::Legacy,
            // A number: no public table says which is which (LXP-B04).
            None => Brdf::PhysicallyBased,
        },
        None => Brdf::PhysicallyBased,
    }
}

/// What the Material gets besides the OpenPBR values: Operandi channels
/// with no OpenPBR identity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    pub bump_amplitude: Option<f64>,
    pub displacement: Option<f64>,
    pub smoothing: Option<f64>,
    /// Degrees.
    pub smoothing_angle: Option<f64>,
    pub double_sided: Option<bool>,
    pub rounded_width: Option<f64>,
    /// Degrees.
    pub rounded_angle: Option<f64>,
    pub clearcoat_bump: Option<f64>,
    /// The base weight to keep as Diffuse Amount (Physically Based).
    pub base_weight: f64,
}

/// §3.1–3.3: an `advancedMaterial` as OpenPBR values.
pub fn advanced_values(m: &PresetItem, notes: &mut Vec<(bool, String)>) -> (Values, Extras) {
    use Param::*;
    let mut v = Values::default();
    let model = brdf(m);
    let diff_amt = num(m, "diffAmt", 0.8);
    if diff_amt > 1. {
        notes.push((
            true,
            format!("Diffuse Amount {:.0} % clamped to 100 %", diff_amt * 100.),
        ));
    }
    let diff_amt = diff_amt.clamp(0., 1.);
    let diff_col = col(m, "diffCol", [0.8; 3]);
    let spec_amt = num(m, "specAmt", 0.04).clamp(0., 1.);
    let spec_col = col(m, "specCol", [1.; 3]);
    let refl_spec = flag(m, "reflSpec", true);
    let refl_amt = num(m, "reflAmt", 0.).clamp(0., 1.);
    // §3.3: a legacy model's separate reflection counts toward specular.
    let spec_weight = if model == Brdf::Legacy && !refl_spec {
        spec_amt.max(refl_amt)
    } else {
        spec_amt
    };
    if model == Brdf::Legacy {
        notes.push((true, "legacy shading model approximated".into()));
    }
    let metallic = num(m, "metallic", 0.).clamp(0., 1.);
    let use_ref_idx = flag(m, "useRefIdx", false);
    let ref_index = num(m, "refIndex", 1.5).max(1.);
    let tran_amt = num(m, "tranAmt", 0.).clamp(0., 1.);
    let mut extras = Extras {
        base_weight: diff_amt,
        ..Default::default()
    };
    // §3.2 (LXP-Q1): a metal made by hand.
    if diff_amt <= 0.05 && spec_weight >= 0.7 && metallic < 1. {
        v.set(BaseMetalness, Pv::Float(1.));
        v.set(BaseColor, Pv::Color(spec_col));
        v.set(BaseWeight, Pv::Float(1.));
        v.set(SpecularColor, Pv::Color([1.; 3]));
        extras.base_weight = 1.;
        notes.push((true, "converted to metal".into()));
    } else {
        v.set(BaseColor, Pv::Color(diff_col));
        v.set(BaseWeight, Pv::Float(diff_amt));
        v.set(BaseMetalness, Pv::Float(metallic));
        let tint = num(m, "specTint", 0.).clamp(0., 1.);
        let spec_colour = if model == Brdf::Principled && tint > 0. {
            mix([1.; 3], diff_col, tint)
        } else {
            spec_col
        };
        v.set(SpecularColor, Pv::Color(spec_colour));
        // §3.1: Use Refractive Index sets the index and Specular Amount
        // is ignored; else Specular Amount is F0.
        if use_ref_idx {
            v.set(SpecularIor, Pv::Float(ref_index));
        } else if tran_amt > 0. {
            // Refraction keeps the index; the reflection keeps F0
            // (OpenPBR's specular_weight scales it, as to_material reads).
            let ior = if flag(m, "specRefIdx", false) {
                1. + 0.8 * spec_amt
            } else {
                ref_index
            };
            v.set(SpecularIor, Pv::Float(ior));
            v.set(
                SpecularWeight,
                Pv::Float((spec_weight / f0_of(ior).max(1e-6)).min(2.)),
            );
        } else if spec_weight <= 0. {
            v.set(SpecularWeight, Pv::Float(0.));
        } else {
            v.set(SpecularIor, Pv::Float(ior_from_f0(spec_weight)));
        }
    }
    v.set(
        BaseDiffuseRoughness,
        Pv::Float(num(m, "diffRough", 0.).clamp(0., 1.)),
    );
    let spec_fres = num(m, "specFres", 1.);
    if (spec_fres - 1.).abs() > 1e-6 {
        notes.push((
            false,
            format!("Specular Fresnel {:.0} % left out", spec_fres * 100.),
        ));
    }
    let rough = num(m, "rough", 0.4).clamp(0., 1.);
    let tran_rough = num(m, "tranRough", 0.).clamp(0., 1.);
    if tran_amt > 0. && tran_rough > rough {
        v.set(SpecularRoughness, Pv::Float(tran_rough));
        notes.push((true, "Refraction Roughness used as the roughness".into()));
    } else {
        v.set(SpecularRoughness, Pv::Float(rough));
    }
    v.set(
        SpecularRoughnessAnisotropy,
        Pv::Float(num(m, "aniso", 0.).clamp(0., 1.)),
    );
    v.set(CoatWeight, Pv::Float(num(m, "coatAmt", 0.).clamp(0., 1.)));
    v.set(
        CoatRoughness,
        Pv::Float(num(m, "coatRough", 0.).clamp(0., 1.)),
    );
    extras.clearcoat_bump = m.number("coatBump");
    v.set(TransmissionWeight, Pv::Float(tran_amt));
    v.set(TransmissionColor, Pv::Color(col(m, "tranCol", [1.; 3])));
    v.set(TransmissionDepth, Pv::Float(num(m, "tranDist", 0.).max(0.)));
    if num(m, "disperse", 0.) > 0. {
        v.set(TransmissionDispersionScale, Pv::Float(1.));
        v.set(TransmissionDispersionAbbeNumber, Pv::Float(40.));
        notes.push((true, "Dispersion approximated (Abbe number 40)".into()));
    }
    v.set(
        GeometryOpacity,
        Pv::Float(1. - num(m, "dissAmt", 0.).clamp(0., 1.)),
    );
    v.set(
        SubsurfaceWeight,
        Pv::Float(num(m, "subsAmt", 0.).clamp(0., 1.)),
    );
    v.set(SubsurfaceColor, Pv::Color(col(m, "subsCol", [0.8; 3])));
    v.set(
        SubsurfaceRadius,
        Pv::Float(num(m, "subsDist", 0.005).max(0.)),
    );
    v.set(
        SubsurfaceScatterAnisotropy,
        Pv::Float((2. * num(m, "subsPhase", 0.5) - 1.).clamp(-1., 1.)),
    );
    // LXP-Q5 / LXP-B02b: Luminous Intensity is render-unit radiance, taken
    // 1:1 as emission_luminance (OPM-Q09: 1 nit = 1 render unit).
    let radiance = num(m, "radiance", 0.).max(0.);
    v.set(EmissionLuminance, Pv::Float(radiance));
    v.set(EmissionColor, Pv::Color(col(m, "lumiCol", [1.; 3])));
    let sheen = num(m, "sheen", 0.).clamp(0., 1.);
    if sheen > 0. {
        v.set(FuzzWeight, Pv::Float(sheen));
        let t = num(m, "sheenTint", 0.).clamp(0., 1.);
        v.set(FuzzColor, Pv::Color(mix([1.; 3], diff_col, t)));
    }
    if num(m, "flatness", 0.) > 0. {
        notes.push((true, "Flatness left out".into()));
    }
    extras.bump_amplitude = m.number("bumpAmp");
    extras.displacement = m.number("displace");
    extras.smoothing = m.number("smooth");
    extras.smoothing_angle = m.number("smAngle").map(f64::to_degrees);
    extras.double_sided = m
        .channels
        .contains_key("dblSided")
        .then(|| flag(m, "dblSided", false));
    extras.rounded_width = m.number("rndWidth");
    extras.rounded_angle = m.number("rndAngle").map(f64::to_degrees);
    let render_only: Vec<&str> = m
        .channels
        .keys()
        .map(String::as_str)
        .filter(|k| {
            let base = k.split('.').next().unwrap_or(k);
            matches!(
                base,
                "reflType"
                    | "reflRays"
                    | "tranRays"
                    | "importance"
                    | "exitCol"
                    | "density"
                    | "lpeLabel"
                    | "useBoolMtl"
            ) || base.starts_with("clip")
                || base.starts_with("scatter")
                || base.starts_with("absorb")
        })
        .map(|k| k.split('.').next().unwrap_or(k))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if !render_only.is_empty() {
        notes.push((
            false,
            format!("render-only settings left out: {}", render_only.join(", ")),
        ));
    }
    (v, extras)
}

/// §3.4: Unreal and Unity materials as OpenPBR values.
pub fn engine_values(m: &PresetItem, notes: &mut Vec<(bool, String)>) -> (Values, Extras) {
    use Param::*;
    let mut v = Values::default();
    let mut extras = Extras {
        base_weight: 1.,
        ..Default::default()
    };
    let unity = m.kind.eq_ignore_ascii_case("unityShader");
    let base = if unity {
        col(m, "albedo", [1.; 3])
    } else {
        col(m, "base", [0.8; 3])
    };
    v.set(BaseColor, Pv::Color(base));
    v.set(
        BaseMetalness,
        Pv::Float(num(m, "metallic", 0.).clamp(0., 1.)),
    );
    let rough = if unity {
        1. - num(m, "smoothness", 0.5).clamp(0., 1.)
    } else {
        num(m, "roughness", 0.5).clamp(0., 1.)
    };
    v.set(SpecularRoughness, Pv::Float(rough));
    let f0 = if unity {
        num(m, "fresrel0", 0.04)
    } else {
        0.08 * num(m, "specular", 0.5)
    };
    if f0 <= 0. {
        v.set(SpecularWeight, Pv::Float(0.));
    } else {
        v.set(SpecularIor, Pv::Float(ior_from_f0(f0)));
    }
    let level = if unity {
        num(m, "emissivelevel", 1.)
    } else {
        1.
    };
    let emissive = col(m, "emissive", [0.; 3]).map(|c| c * level);
    let strength = emissive.iter().cloned().fold(0., f64::max);
    if strength > 0. {
        v.set(EmissionColor, Pv::Color(emissive.map(|c| c / strength)));
        v.set(EmissionLuminance, Pv::Float(strength));
    }
    if unity {
        extras.bump_amplitude = m.number("heightscale");
        if let Some(s) = m.number("normalscale")
            && (s - 1.).abs() > 1e-6
        {
            notes.push((true, format!("normal map strength {s:.2} read as 1")));
        }
    } else if let Some(mode) = m.text("shadeMode")
        && !mode.eq_ignore_ascii_case("standard")
        && !mode.eq_ignore_ascii_case("defaultLit")
    {
        notes.push((
            true,
            format!("Unreal shading model {mode} treated as standard"),
        ));
    }
    (v, extras)
}

/// The Material a preset's material item makes.
pub fn material_channels(m: &PresetItem, notes: &mut Vec<(bool, String)>) -> MaterialChannels {
    let advanced = m.kind.eq_ignore_ascii_case("advancedMaterial");
    let (values, extras) = if advanced {
        advanced_values(m, notes)
    } else {
        engine_values(m, notes)
    };
    let (mut mc, more) = openpbr::to_material(&values);
    notes.extend(more.into_iter().map(|n| (true, n)));
    if advanced {
        // Physically Based keeps Diffuse Amount apart from the colour, so
        // a Diffuse Amount layer multiplies the base as in Modo.
        mc.shading_model = ShadingModel::PhysicallyBased;
        let base = values.get(Param::BaseColor).color();
        mc.diffuse_color = base.map(|c| c.clamp(0., 1.));
        mc.diffuse_amount = extras.base_weight;
    }
    if let Some(a) = extras.bump_amplitude {
        mc.bump_amplitude = a.max(0.);
    }
    if let Some(d) = extras.displacement {
        mc.displacement_distance = d.max(0.);
    }
    if let Some(s) = extras.smoothing {
        mc.smoothing = s.clamp(0., 1.);
    }
    if let Some(a) = extras.smoothing_angle {
        mc.smoothing_angle = a.clamp(0., 180.);
    }
    if let Some(d) = extras.double_sided {
        mc.double_sided = d;
    }
    if let Some(w) = extras.rounded_width {
        mc.more.rounded_edge_width = w.max(0.);
    }
    if let Some(a) = extras.rounded_angle {
        mc.more.rounded_edge_angle = a.clamp(0., 180.);
    }
    if let Some(b) = extras.clearcoat_bump {
        mc.clearcoat_bump = b.clamp(0., 1.);
    }
    mc
}

// ------------------------------------------------------------ effects

/// Where a layer's effect goes (§4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// An Operandi effect; `invert` flips the value (smoothness, opacity).
    To { effect: TextureEffect, invert: bool },
    /// Left out, with the reason.
    Drop(&'static str),
}

/// The effect ids of §4.1 (and Modo's own channel effects, which share
/// Operandi's names) onto Operandi's effects.
pub const EFFECTS: &[(&str, TextureEffect, bool)] = {
    use TextureEffect::*;
    &[
        ("diffColor", DiffuseColor, false),
        ("baseUE", DiffuseColor, false),
        ("albedoUT", DiffuseColor, false),
        ("baseColorGLTF", DiffuseColor, false),
        ("diffAmount", DiffuseAmount, false),
        ("diffRough", DiffuseRoughness, false),
        ("specColor", SpecularColor, false),
        ("specAmount", SpecularAmount, false),
        ("specUE", SpecularAmount, false),
        ("specFresnel", SpecularFresnel, false),
        ("reflColor", ReflectionColor, false),
        ("reflAmount", ReflectionAmount, false),
        ("reflFresnel", ReflectionFresnel, false),
        ("rough", Roughness, false),
        ("roughUE", Roughness, false),
        ("roughnessGLTF", Roughness, false),
        ("smoothUT", Roughness, true),
        ("metallic", Metallic, false),
        ("metallicUE", Metallic, false),
        ("metallicUT", Metallic, false),
        ("metallicGLTF", Metallic, false),
        ("specularTint", SpecularTint, false),
        ("sheen", Sheen, false),
        ("sheenTint", SheenTint, false),
        ("coatAmount", ClearcoatAmount, false),
        ("ccUE", ClearcoatAmount, false),
        ("tranColor", TransparentColor, false),
        ("tranAmount", TransparentAmount, false),
        ("tranRough", RefractionRoughness, false),
        ("subsColor", SubsurfaceColor, false),
        ("sssUE", SubsurfaceColor, false),
        ("subsAmount", SubsurfaceAmount, false),
        ("lumiColor", LuminousColor, false),
        ("lumiAmount", LuminousAmount, false),
        ("emisUE", LuminousColor, false),
        ("emisUT", LuminousColor, false),
        ("emisGLTF", LuminousColor, false),
        ("dissolve", Dissolve, false),
        ("opacUE", Dissolve, true),
        ("stencil", Stencil, false),
        ("bump", Bump, false),
        ("bumpUE", Bump, false),
        ("bumpUT", Bump, false),
        ("normal", Normal, false),
        ("normalUE", Normal, false),
        ("normalUT", Normal, false),
        ("normalGLTF", Normal, false),
        ("normal2UT", Normal, false),
        ("displace", Displacement, false),
        ("layerMask", LayerMask, false),
        ("groupMask", GroupMask, false),
        ("driverA", DriverA, false),
        ("driverB", DriverB, false),
        ("driverC", DriverC, false),
        ("driverD", DriverD, false),
    ]
};

/// Effects §4.1 drops, with the note's reason.
pub const DROPPED_EFFECTS: &[(&str, &str)] = &[
    ("aoUE", "ambient occlusion maps"),
    ("aoUT", "ambient occlusion maps"),
    ("aoGLTF", "ambient occlusion maps"),
    ("detailMaskUT", "Unity detail masks"),
    ("albedo2UT", "Unity detail albedo"),
    ("flatness", "Flatness"),
    ("textureOffset", "Texture Offset"),
    ("ccrUE", "clear coat roughness maps"),
];

pub fn target(effect: &str) -> Target {
    if let Some(&(_, effect, invert)) = EFFECTS.iter().find(|(id, _, _)| *id == effect) {
        return Target::To { effect, invert };
    }
    if let Some((_, why)) = DROPPED_EFFECTS.iter().find(|(id, _)| *id == effect) {
        return Target::Drop(why);
    }
    Target::Drop("an effect Operandi doesn't have")
}

/// Whether an effect reads colour (its image is sRGB-encoded).
pub fn is_colour_effect(effect: TextureEffect) -> bool {
    use TextureEffect::*;
    matches!(
        effect,
        DiffuseColor
            | SpecularColor
            | ReflectionColor
            | TransparentColor
            | SubsurfaceColor
            | LuminousColor
            | EnvironmentColor
    )
}

/// §4.2's colour space rule: whether an image is read as linear data.
/// `(none)` → linear; `(default)` → sRGB for colour effects, linear for
/// scalar ones (roughness, metallic, bump, masks).
pub fn reads_linear(colour_space: Option<&str>, effect: TextureEffect) -> bool {
    match colour_space
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("(none)") | Some("linear") | Some("raw") => true,
        Some(s) if s.contains("srgb") => false,
        _ => !is_colour_effect(effect),
    }
}

fn blend_mode(m: &PresetItem) -> Option<BlendMode> {
    let v = m.channels.get("blend")?;
    let t = v.text()?.to_ascii_lowercase().replace([' ', '_'], "");
    Some(match t.as_str() {
        "normal" => BlendMode::Normal,
        "add" => BlendMode::Add,
        "subtract" | "sub" => BlendMode::Subtract,
        "difference" | "diff" => BlendMode::Difference,
        "normalmultiply" | "normmult" => BlendMode::NormalMultiply,
        "divide" | "div" => BlendMode::Divide,
        "multiply" | "mult" => BlendMode::Multiply,
        "screen" => BlendMode::Screen,
        "overlay" => BlendMode::Overlay,
        "softlight" => BlendMode::SoftLight,
        "hardlight" => BlendMode::HardLight,
        "lighten" => BlendMode::Lighten,
        "darken" => BlendMode::Darken,
        "colordodge" | "dodge" => BlendMode::ColorDodge,
        "colorburn" | "burn" => BlendMode::ColorBurn,
        "normalmapblend" | "normalmap" => BlendMode::NormalMapBlend,
        _ => return None,
    })
}

// ------------------------------------------------------------ layers

/// Layer types §7.2 maps.
const LAYER_TYPES: [&str; 11] = [
    "imageMap",
    "constant",
    "gradient",
    "noise",
    "cellular",
    "wood",
    "grid",
    "checker",
    "dots",
    "process",
    "occlusion",
];

/// Items that hold layers or shade the surface, never layers themselves:
/// masks drive "(all)", shaders "fullShade".
fn is_container(i: &PresetItem) -> bool {
    matches!(
        i.kind.as_str(),
        "mask" | "defaultShader" | "shaderFolder" | "group" | "groupLocator" | "txtrLocator"
    ) || matches!(i.text("effect"), Some("(all)" | "fullShade"))
}

/// Whether an item is a texture layer: it drives an effect.
fn is_layer(i: &PresetItem) -> bool {
    !is_container(i)
        && (i.channels.contains_key("effect") || LAYER_TYPES.contains(&i.kind.as_str()))
}

/// Whether the preset holds a material item (LXP-04).
pub fn has_material(p: &PresetItems) -> bool {
    p.items.values().any(is_material)
}

fn is_material(i: &PresetItem) -> bool {
    MATERIAL_TYPES
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&i.kind))
}

/// The two-zone settings (§7.2) a procedural layer stores.
fn zones(m: &PresetItem) -> TwoZone {
    let d = TwoZone::default();
    TwoZone {
        zone1_value: num(m, "value1", d.zone1_value),
        zone1_color: col(m, "color1", d.zone1_color),
        zone1_alpha: num(m, "alpha1", d.zone1_alpha),
        zone2_value: num(m, "value2", d.zone2_value),
        zone2_color: col(m, "color2", d.zone2_color),
        zone2_alpha: num(m, "alpha2", d.zone2_alpha),
        bias: num(m, "bias", d.bias),
        gain: num(m, "gain", d.gain),
    }
}

/// An enum channel's text, lower case without spaces.
fn word(m: &PresetItem, key: &str) -> Option<String> {
    m.text(key)
        .map(|t| t.to_ascii_lowercase().replace([' ', '_'], ""))
}

fn variation(m: &PresetItem) -> Variation {
    Variation {
        value: num(m, "variation", 0.),
        hue: num(m, "varyHue", 0.),
        saturation: num(m, "varySat", 0.),
        alpha: num(m, "varyAlpha", 0.),
    }
}

/// §7.2: a Modo procedural as an Operandi one, `None` when Operandi has no
/// such type.
pub fn procedural(m: &PresetItem) -> Option<Procedural> {
    let kind = match m.kind.as_str() {
        "noise" => ProceduralKind::Noise,
        "cellular" => ProceduralKind::Cellular,
        "wood" => ProceduralKind::Wood,
        "grid" => ProceduralKind::Grid,
        "checker" => ProceduralKind::Checker,
        "dots" => ProceduralKind::Dots,
        _ => return None,
    };
    let mut p = Procedural::new(kind);
    p.zones = zones(m);
    let octaves = |m: &PresetItem, d: u32| {
        num(m, "freqs", f64::from(d))
            .round()
            .clamp(1., f64::from(crate::procedural::MAX_OCTAVES)) as u32
    };
    p.pattern = match p.pattern {
        Pattern::Noise(d) => Pattern::Noise(NoiseSettings {
            kind: match word(m, "type").as_deref() {
                Some("simple") => NoiseType::Simple,
                Some("turbulence") => NoiseType::Turbulence,
                _ => NoiseType::Fractal,
            },
            frequencies: octaves(m, d.frequencies),
            frequency_ratio: num(m, "freqRatio", d.frequency_ratio),
            amplitude_ratio: num(m, "ampRatio", d.amplitude_ratio),
            smooth_tops: num(m, "smoothTops", d.smooth_tops),
            flow: flag(m, "flowNoise", d.flow),
            flow_phase: num(m, "angle", d.flow_phase),
        }),
        Pattern::Cellular {
            kind: _,
            cell_width,
            transition,
            frequencies,
            frequency_ratio,
            amplitude_ratio,
            smooth_tops,
            variation: _,
        } => Pattern::Cellular {
            kind: match word(m, "type").as_deref() {
                Some("round") => CellularType::Round,
                _ => CellularType::Angular,
            },
            cell_width: num(m, "cellWidth", cell_width),
            transition: num(m, "transWidth", transition),
            frequencies: octaves(m, frequencies),
            frequency_ratio: num(m, "freqRatio", frequency_ratio),
            amplitude_ratio: num(m, "ampRatio", amplitude_ratio),
            smooth_tops: num(m, "smoothTops", smooth_tops),
            variation: variation(m),
        },
        Pattern::Wood {
            ring_scale,
            waviness,
            wave_scale,
            graininess,
            grain_scale,
        } => Pattern::Wood {
            ring_scale: num(m, "ringScale", ring_scale),
            waviness: num(m, "wave", waviness),
            wave_scale: num(m, "waveScale", wave_scale),
            graininess: num(m, "grain", graininess),
            grain_scale: num(m, "grainScale", grain_scale),
        },
        Pattern::Grid {
            line_width,
            transition,
            ..
        } => Pattern::Grid {
            kind: match word(m, "type").as_deref() {
                Some("line") => GridType::Line,
                Some("triangle") => GridType::Triangle,
                Some("hexagon") => GridType::Hexagon,
                Some("cube") => GridType::Cube,
                _ => GridType::Square,
            },
            line_width: num(m, "lineWidth", line_width),
            transition: num(m, "transWidth", transition),
            variation: variation(m),
        },
        Pattern::Checker { transition, .. } => Pattern::Checker {
            kind: match word(m, "type").as_deref() {
                Some("cube") => CheckerType::Cube,
                _ => CheckerType::Square,
            },
            transition: num(m, "transWidth", transition),
            variation: variation(m),
        },
        Pattern::Dots {
            dot_width,
            transition,
            ..
        } => Pattern::Dots {
            kind: match word(m, "type").as_deref() {
                Some("hexagon") => DotsType::Hexagon,
                Some("cube") => DotsType::Cube,
                _ => DotsType::Square,
            },
            dot_width: num(m, "dotWidth", dot_width),
            transition: num(m, "transWidth", transition),
            variation: variation(m),
        },
        other => other,
    };
    Some(p)
}

/// §7.2: Process (colour correct).
pub fn process(m: &PresetItem) -> Process {
    let d = Process::default();
    Process {
        bias: num(m, "bias", d.bias),
        gain: num(m, "gain", d.gain),
        hue: num(m, "hue", d.hue),
        saturation: num(m, "sat", d.saturation),
        value: num(m, "val", d.value),
        hue_shift: num(m, "hueShift", d.hue_shift),
    }
}

/// §7.2: Occlusion (Cycles AO).
pub fn occlusion(m: &PresetItem) -> Occlusion {
    let d = Occlusion::default();
    Occlusion {
        zones: zones(m),
        kind: match word(m, "type").as_deref() {
            Some("up") => OcclusionType::Up,
            Some("downslope") => OcclusionType::DownSlope,
            Some("reflection") => OcclusionType::Reflection,
            Some("concavity") => OcclusionType::Concavity,
            _ => OcclusionType::Uniform,
        },
        rays: num(m, "rays", f64::from(d.rays)).round().clamp(1., 1024.) as u32,
        distance: num(m, "dist", d.distance).max(0.),
        spread_angle: m
            .number("spread")
            .map(f64::to_degrees)
            .unwrap_or(d.spread_angle),
        same_surface_only: flag(m, "sameSurf", d.same_surface_only),
        ..d
    }
}

/// A gradient's input by its Modo channel name (§7.2).
pub fn gradient_input(param: &str) -> Option<GradientInput> {
    let key = param.to_ascii_lowercase().replace([' ', '_'], "");
    let named = match key.as_str() {
        "incidenceangle" | "incidence" => Some(GradientInput::Incidence),
        "driver" | "drivera" => Some(GradientInput::DriverA),
        "diffamount" => Some(GradientInput::DiffuseAmount),
        "incidencenobump" => Some(GradientInput::BumplessIncident),
        "driverb" => Some(GradientInput::DriverB),
        "driverc" => Some(GradientInput::DriverC),
        "driverd" => Some(GradientInput::DriverD),
        "slope" => Some(GradientInput::Slope),
        "texvalue" | "texturevalue" => Some(GradientInput::TextureValue),
        "camdistance" | "distancetocamera" | "cameradist" => Some(GradientInput::DistanceToCamera),
        _ => None,
    };
    // Otherwise by the label without spaces ("diffAmount" no, but
    // "diffuseamount" yes).
    named.or_else(|| {
        GradientInput::ALL
            .iter()
            .copied()
            .find(|i| i.label().to_ascii_lowercase().replace(' ', "") == key)
    })
}

/// A gradient record's interpolation flag as our key shape (LXP-B07:
/// 0 curve, 1 linear, 2 stepped; Modo's curve is our Smooth). `None`
/// for a flag our gradient has no key type for.
pub fn key_shape(flag: Option<u32>) -> Option<KeyShape> {
    match flag {
        None | Some(0) => Some(KeyShape::Smooth),
        Some(1) => Some(KeyShape::Linear),
        Some(2) => Some(KeyShape::Stepped),
        Some(_) => None,
    }
}

/// Envelope keys onto a curve of `shape`. Angles are stored in radians;
/// Operandi's keys are degrees.
fn curve<V: crate::gradient::CurveOut>(unit: Unit, keys: &[(f64, V)], shape: KeyShape) -> Curve<V> {
    let at = |t: f64| match unit {
        Unit::Degrees => t.to_degrees(),
        _ => t,
    };
    let mut c = Curve {
        keys: keys
            .iter()
            .take(crate::gradient::MAX_KEYS)
            .map(|(t, out)| Key {
                at: at(*t),
                out: *out,
                shape,
            })
            .collect(),
    };
    c.sort();
    c
}

/// The colour channels a gradient's records drive (LXP-B07).
const COLOUR_CHANNELS: [&str; 4] = ["color.R", "color.G", "color.B", "color.A"];

/// §7.2 / LXP-B07: a gradient layer, or why it can't map. Each gradient
/// record is one number curve keyed by channel name: `value` for number
/// effects, `color.R`/`.G`/`.B` (+ `.A`) for colour effects.
fn gradient(
    m: &PresetItem,
    items: &PresetItems,
    effect: TextureEffect,
    notes: &mut Notes,
    label: &str,
) -> Result<Gradient, String> {
    let param = m.text("param").unwrap_or("incidenceAngle");
    let input = gradient_input(param).ok_or_else(|| format!("gradient input {param}"))?;
    if input.is_stored_only() {
        return Err(format!("gradient input {}", input.label()));
    }
    if m.gradients.is_empty() {
        return Err("gradient with no curve".into());
    }
    let unit = input.unit();
    let mut approximated = false;
    // Each record's curve, by channel name; records without readable
    // keys are left out.
    let mut curves: Vec<(&str, Curve<f64>)> = Vec::new();
    for g in &m.gradients {
        // Values typed `percent` are stored as fractions like `float`
        // ones (provisional, LXP-B07c).
        if let Some((_, out)) = &g.types
            && !out.eq_ignore_ascii_case("float")
            && !out.eq_ignore_ascii_case("percent")
        {
            return Err(format!("gradient with {out} keys"));
        }
        let Some(keys) = items
            .envelopes
            .get(&g.envelope)
            .map(|e| &e.keys)
            .filter(|k| !k.is_empty())
        else {
            continue;
        };
        let shape = key_shape(g.interpolation).unwrap_or_else(|| {
            approximated = true;
            KeyShape::Smooth
        });
        curves.push((g.channel.as_str(), curve(unit, keys, shape)));
    }
    if curves.is_empty() {
        return Err("gradient with no readable keys".into());
    }
    let named = |name: &str| {
        curves
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(name))
            .map(|(_, c)| c)
    };
    // `value`, or the only record when it names no known channel.
    let value_curve = named("value").or_else(|| {
        let known = |c: &str| {
            c.eq_ignore_ascii_case("value")
                || COLOUR_CHANNELS.iter().any(|k| c.eq_ignore_ascii_case(k))
        };
        match curves.as_slice() {
            [(c, curve)] if !known(c) => Some(curve),
            _ => None,
        }
    });
    let d = Gradient::default();
    let g = if is_colour_effect(effect) {
        let parts: [Option<&Curve<f64>>; 4] = COLOUR_CHANNELS.map(named);
        let color = if parts[..3].iter().any(Option::is_some) {
            colour_curve(m, parts)
        } else if let Some(v) = value_curve.or(match curves.as_slice() {
            [(_, c)] => Some(c),
            _ => None,
        }) {
            notes.look(format!("{label}: colour gradient had only a value curve"));
            grey_curve(v)
        } else {
            return Err("colour gradient with no colour curves".into());
        };
        Gradient {
            input,
            value: d.value,
            color,
        }
    } else {
        let Some(value) = value_curve else {
            return Err("gradient with no value curve".into());
        };
        Gradient {
            input,
            value: value.clone(),
            color: d.color,
        }
    };
    if approximated {
        notes.look(format!("{label}: gradient curve approximated"));
    }
    if !g.valid() {
        return Err("gradient curve out of range".into());
    }
    Ok(g)
}

/// LXP-B07: a colour curve from independent R, G, B (and A) curves, keyed
/// at the union of their key inputs. A missing component holds the
/// gradient item's static channel value, else 0 (alpha: else 1, so a
/// missing alpha curve stays opaque; provisional). Any stepped component
/// steps the whole colour; otherwise linear only when every component is.
fn colour_curve(m: &PresetItem, parts: [Option<&Curve<f64>>; 4]) -> Curve<[f64; 4]> {
    let mut at: Vec<f64> = parts
        .iter()
        .flatten()
        .flat_map(|c| c.keys.iter().map(|k| k.at))
        .collect();
    at.sort_by(f64::total_cmp);
    at.dedup();
    at.truncate(crate::gradient::MAX_KEYS);
    let shapes: Vec<KeyShape> = parts
        .iter()
        .flatten()
        .flat_map(|c| c.keys.iter().map(|k| k.shape))
        .collect();
    let shape = if shapes.contains(&KeyShape::Stepped) {
        KeyShape::Stepped
    } else if shapes.iter().all(|s| *s == KeyShape::Linear) {
        KeyShape::Linear
    } else {
        KeyShape::Smooth
    };
    let fallback: [f64; 4] = std::array::from_fn(|k| {
        m.number(COLOUR_CHANNELS[k])
            .unwrap_or(if k == 3 { 1. } else { 0. })
    });
    Curve {
        keys: at
            .into_iter()
            .map(|x| Key {
                at: x,
                out: std::array::from_fn(|k| parts[k].map_or(fallback[k], |c| c.sample(x))),
                shape,
            })
            .collect(),
    }
}

/// A value curve as grey colour keys (R = G = B, opaque).
fn grey_curve(v: &Curve<f64>) -> Curve<[f64; 4]> {
    Curve {
        keys: v
            .keys
            .iter()
            .map(|k| Key {
                at: k.at,
                out: [k.out, k.out, k.out, 1.],
                shape: k.shape,
            })
            .collect(),
    }
}

/// The items `id` links to or that link to it, of `kind`.
fn linked<'a>(items: &'a PresetItems, id: u32, kind: &str) -> Option<&'a PresetItem> {
    let me = items.items.get(&id)?;
    me.links
        .iter()
        .filter_map(|(_, t, _)| items.items.get(t))
        .find(|i| i.kind == kind)
        .or_else(|| {
            items
                .items
                .values()
                .find(|i| i.kind == kind && i.links.iter().any(|(_, t, _)| *t == id))
        })
}

/// §4.2 image path resolution.
pub fn resolve_image(
    name: &str,
    search: &ImageSearch,
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() && exists(p) {
        return Some(p.to_path_buf());
    }
    // Alias prefixes onto the content folders.
    let mut rest = name;
    for alias in ["kit_Modo_Content:", "content:", "asset:"] {
        if let Some(r) = name.strip_prefix(alias) {
            rest = r.trim_start_matches(['/', '\\']);
        }
    }
    let rel = Path::new(rest);
    // The content folders the user added, then the preset folder's own
    // ancestors (the content folder it sits in), each also with its
    // Assets folder (`asset:` names a path below it).
    let mut roots: Vec<PathBuf> = search.roots.clone();
    roots.extend(search.folder.ancestors().skip(1).map(Path::to_path_buf));
    let roots: Vec<PathBuf> = roots
        .into_iter()
        .flat_map(|r| [r.join("Assets"), r])
        .collect();
    let mut tries = Vec::new();
    if rel.is_relative() {
        tries.push(search.folder.join(rel));
        for root in &roots {
            tries.push(root.join(rel));
        }
    }
    // An absolute path from another machine: find its tail under a root
    // (".../Content/Assets/Images/x.png" → root/Assets/Images/x.png).
    let parts: Vec<&str> = rest.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    for start in 0..parts.len() {
        let tail: PathBuf = parts[start..].iter().collect();
        for root in &roots {
            tries.push(root.join(&tail));
        }
        if start == parts.len().saturating_sub(1) {
            tries.push(search.folder.join(&tail));
        }
    }
    tries.into_iter().find(|t| exists(t))
}

/// A converted layer: its source and the effect it drives.
struct Made {
    source: TextureSource,
    effect: TextureEffect,
    locator: Option<TextureLocator>,
    layer: Layer,
}

fn locator_of(
    items: &PresetItems,
    layer: &PresetItem,
    notes: &mut Notes,
) -> Option<TextureLocator> {
    let loc = linked(items, layer.id, "txtrLocator")?;
    let proj = word(loc, "projType").unwrap_or_default();
    let projection = match proj.as_str() {
        "uv" => Projection::UvMap,
        "" | "none" | "solid" => Projection::Solid,
        "planar" => Projection::Planar,
        "cylindrical" => Projection::Cylindrical,
        "spherical" => Projection::Spherical,
        "cubic" => Projection::Cubic,
        "box" => Projection::Box,
        "front" => Projection::Front,
        "triplanar" => Projection::Triplanar,
        other => {
            notes.look(format!("{other} projection read as UV"));
            Projection::UvMap
        }
    };
    let mut l = TextureLocator::new(projection);
    if let Some(axis) = loc.number("projAxis") {
        l.axis = (axis.round() as i64).clamp(0, 2) as u8;
    }
    l.uv_map = loc
        .text("uvMap")
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let tile = |key: &str| match word(loc, key).as_deref() {
        Some("reset") | Some("none") => Some(Tiling::Reset),
        Some("mirror") => Some(Tiling::Mirror),
        Some("edge") | Some("clamp") => Some(Tiling::Edge),
        _ => None,
    };
    if let Some(t) = tile("tileU") {
        l.tiling[0] = t;
    }
    if let Some(t) = tile("tileV") {
        l.tiling[1] = t;
    }
    l.repeat = [num(loc, "repeatU", 1.), num(loc, "repeatV", 1.)];
    Some(l)
}

/// The scale and position of a layer's locator, from its linked
/// transform items, for procedural feature sizes (§7.2: 1 m is 1 m).
fn locator_transform(items: &PresetItems, layer: &PresetItem) -> Option<([f64; 3], [f64; 3])> {
    let loc = linked(items, layer.id, "txtrLocator")?;
    let scale = linked(items, loc.id, "scale")
        .and_then(|s| Some([s.number("scl.X")?, s.number("scl.Y")?, s.number("scl.Z")?]));
    let pos = linked(items, loc.id, "translation")
        .and_then(|s| Some([s.number("pos.X")?, s.number("pos.Y")?, s.number("pos.Z")?]));
    if scale.is_none() && pos.is_none() {
        return None;
    }
    Some((scale.unwrap_or([1.; 3]), pos.unwrap_or([0.; 3])))
}

fn image_settings(m: &PresetItem, effect: TextureEffect) -> ImageSettings {
    let d = ImageSettings::default();
    let mut s = ImageSettings {
        low_value: num(m, "min", d.low_value),
        high_value: num(m, "max", d.high_value),
        gamma: num(m, "gamma", d.gamma),
        brightness: num(m, "brightness", d.brightness),
        contrast: num(m, "contrast", d.contrast),
        invert_red: flag(m, "redInv", false),
        invert_green: flag(m, "greenInv", false),
        invert_blue: flag(m, "blueInv", false),
        ..d
    };
    s.swizzle = match word(m, "rgba").as_deref() {
        Some("red") => Swizzle::Red,
        Some("green") => Swizzle::Green,
        Some("blue") => Swizzle::Blue,
        Some("alpha") => Swizzle::Alpha,
        Some("rgb") => Swizzle::Rgb,
        _ => s.swizzle,
    };
    // Scalar effects use a single channel of a coloured map as is.
    let _ = effect;
    s
}

fn layer_of(m: &PresetItem) -> Layer {
    Layer {
        enable: flag(m, "enable", true),
        opacity: num(m, "opacity", 1.).clamp(0., 1.),
        invert: flag(m, "invert", false),
        ..Layer::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn make_layer(
    m: &PresetItem,
    items: &PresetItems,
    search: &ImageSearch,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
    exists: &dyn Fn(&Path) -> bool,
    scene: &mut Scene,
    notes: &mut Notes,
) -> Result<Made, String> {
    let label = m.label().to_owned();
    let effect_id = m.text("effect").unwrap_or("diffColor").to_owned();
    let (effect, flip) = match target(&effect_id) {
        Target::To { effect, invert } => (effect, invert),
        Target::Drop(why) => return Err(format!("{label}: {why} ({effect_id})")),
    };
    let mut layer = layer_of(m);
    layer.invert ^= flip;
    match blend_mode(m) {
        Some(b) => layer.blend = b,
        None if m.channels.contains_key("blend") => {
            notes.look(format!("{label}: blend mode read as Normal"));
        }
        None => {}
    }
    let locator = locator_of(items, m, notes);
    let source = match m.kind.as_str() {
        "imageMap" => {
            let clip = linked(items, m.id, "videoStill")
                .ok_or_else(|| format!("{label}: image map without an image"))?;
            let file = clip
                .text("filename")
                .or_else(|| clip.text("file"))
                .unwrap_or("")
                .to_owned();
            let space = clip.text("colorspace").or_else(|| clip.text("colorSpace"));
            if reads_linear(space, effect) == is_colour_effect(effect) {
                // Operandi reads colour maps as sRGB and data as linear.
                notes.look(format!(
                    "{label}: colour space {} read as {}",
                    space.unwrap_or("(default)"),
                    if is_colour_effect(effect) {
                        "sRGB"
                    } else {
                        "linear"
                    }
                ));
            }
            let found = resolve_image(&file, search, exists);
            let (path, size) = match found.as_ref().and_then(|p| Some((p, size_of(p)?))) {
                Some((p, s)) => (p.to_string_lossy().into_owned(), s),
                None => {
                    notes.look(format!("{label}: missing image {file}"));
                    // Kept with a placeholder size, so it reads once the
                    // file turns up (§4.2: a "missing image" tile).
                    (file.clone(), (1, 1))
                }
            };
            let image = if path.is_empty() {
                None
            } else {
                scene.image_for_path(&path, size.0, size.1).ok()
            };
            TextureSource::ImageMap {
                image,
                settings: Box::new(image_settings(m, effect)),
            }
        }
        "constant" => TextureSource::Constant {
            color: col(m, "color", [1.; 3]),
            value: num(m, "value", 1.),
        },
        "gradient" => TextureSource::Gradient(Box::new(
            gradient(m, items, effect, notes, &label).map_err(|e| format!("{label}: {e}"))?,
        )),
        "process" => TextureSource::Process(Box::new(process(m))),
        "occlusion" => TextureSource::Occlusion(Box::new(occlusion(m))),
        kind => match procedural(m) {
            Some(p) => TextureSource::Procedural(Box::new(p)),
            None => {
                return Err(if kind.ends_with(".RJJ") || kind.contains('.') {
                    format!("{label}: plug-in layer {kind}")
                } else {
                    format!("{label}: {kind} layers")
                });
            }
        },
    };
    Ok(Made {
        source,
        effect,
        locator,
        layer,
    })
}

/// The preset's texture layers, bottom to top (the material's end first).
/// Provisional (LXP-B05): siblings order by their `parent` link ordinal,
/// the way round that puts the material at the bottom.
fn ordered_layers(items: &PresetItems, material: u32) -> Vec<&PresetItem> {
    let parent_of = |i: &PresetItem| {
        i.links
            .iter()
            .find(|(g, _, _)| g == "parent")
            .map(|(_, t, o)| (*t, *o))
    };
    let mut layers: Vec<(&PresetItem, u32)> = items
        .items
        .values()
        .filter(|i| is_layer(i) && !is_material(i))
        .map(|i| (i, parent_of(i).map_or(i.id, |(_, o)| o)))
        .collect();
    layers.sort_by_key(|(i, o)| (*o, i.id));
    let material_ord = items
        .items
        .get(&material)
        .and_then(parent_of)
        .map(|(_, o)| o);
    let mut out: Vec<&PresetItem> = layers.into_iter().map(|(i, _)| i).collect();
    // If the material sorts last among its siblings, the ordinals run top
    // to bottom.
    if let Some(mo) = material_ord
        && out
            .iter()
            .filter_map(|i| parent_of(i).map(|(_, o)| o))
            .all(|o| o < mo)
        && !out.is_empty()
    {
        out.reverse();
    }
    out
}

/// Convert read preset items.
pub fn convert(
    items: &PresetItems,
    name: &str,
    search: &ImageSearch,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> Result<Converted, String> {
    convert_with(items, name, search, size_of, &|p: &Path| p.is_file())
}

/// [`convert`] with a file check (tests).
pub fn convert_with(
    items: &PresetItems,
    name: &str,
    search: &ImageSearch,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
    exists: &dyn Fn(&Path) -> bool,
) -> Result<Converted, String> {
    let material = items
        .items
        .values()
        .find(|i| is_material(i))
        .ok_or_else(|| NO_MATERIAL.to_string())?;
    let mut notes = Notes {
        lines: Vec::new(),
        look: false,
    };
    if items.items.values().filter(|i| is_material(i)).count() > 1 {
        notes.look("only the first material kept");
    }
    let mut channel_notes = Vec::new();
    let channels = material_channels(material, &mut channel_notes);
    for (look, line) in channel_notes {
        if look {
            notes.look(line);
        } else {
            notes.quiet(line);
        }
    }
    let mut scene = Scene {
        shader_tree: ShaderTree {
            node_views: Default::default(),
            nodes: Vec::new(),
            next_id: 1,
            outputs: Default::default(),
            node_graphs: Default::default(),
        },
        ..Scene::default()
    };
    let group_name = items
        .items
        .values()
        .find(|i| i.kind == "mask" && !i.name.is_empty())
        .map_or(name, |m| m.name.as_str())
        .to_owned();
    if items.items.values().any(|i| i.kind == "defaultShader") {
        notes.quiet("the preset's shader left out (the scene's own shades it)");
    }
    let mut nodes = vec![node(
        1,
        None,
        &group_name,
        NodeKind::Group(GroupMask::default()),
    )];
    let mut next = 2;
    let layers = ordered_layers(items, material.id);
    let mut made = Vec::new();
    let mut dropped = Vec::new();
    // Driver layers that are dropped take their gradients with them (§7.2).
    for l in &layers {
        match make_layer(
            l,
            items,
            search,
            size_of,
            &|p| exists(p),
            &mut scene,
            &mut notes,
        ) {
            Ok(m) => made.push((l, m)),
            Err(why) => dropped.push(((*l).clone(), why)),
        }
    }
    let lost_drivers: Vec<TextureEffect> = dropped
        .iter()
        .filter_map(|(l, _)| match target(l.text("effect").unwrap_or("")) {
            Target::To { effect, .. }
                if matches!(
                    effect,
                    TextureEffect::DriverA
                        | TextureEffect::DriverB
                        | TextureEffect::DriverC
                        | TextureEffect::DriverD
                ) =>
            {
                Some(effect)
            }
            _ => None,
        })
        .collect();
    let driver_input = |e: TextureEffect| match e {
        TextureEffect::DriverA => GradientInput::DriverA,
        TextureEffect::DriverB => GradientInput::DriverB,
        TextureEffect::DriverC => GradientInput::DriverC,
        _ => GradientInput::DriverD,
    };
    made.retain(|(l, m)| {
        let gone = matches!(&m.source, TextureSource::Gradient(g)
            if lost_drivers.iter().any(|d| driver_input(*d) == g.input));
        if gone {
            dropped.push((
                (**l).clone(),
                format!("{}: gradient on a dropped driver", l.label()),
            ));
        }
        !gone
    });
    for (_, why) in &dropped {
        notes.look(format!("left out {why}"));
    }
    let mut placements = Vec::new();
    // Top of the group first: Operandi lists the top layer first.
    for (l, m) in made.into_iter().rev() {
        let mut kind = NodeKind::Texture(TextureLayer {
            source: m.source,
            effect: m.effect,
            locator: None,
            legacy_projection: None,
            legacy_axis: None,
            ignore_scale_group: false,
            node_input: None,
        });
        if let NodeKind::Texture(t) = &mut kind
            && let TextureSource::Procedural(_) = t.source
            && m.locator.is_none()
        {
            // Procedurals without a locator: object space (§4.2 `none`).
        }
        let mut n = node(next, Some(1), l.label(), kind);
        n.layer = m.layer;
        placements.push((next, m.locator, locator_transform(items, l)));
        next += 1;
        nodes.push(n);
    }
    nodes.push(node(
        next,
        Some(1),
        material.label(),
        NodeKind::Material(Box::new(channels)),
    ));
    next += 1;
    scene.shader_tree = ShaderTree {
        node_views: Default::default(),
        nodes,
        next_id: next,
        outputs: Default::default(),
        node_graphs: Default::default(),
    };
    // Each layer's own locator, with the preset's projection and size.
    let mut next_item = scene.next_id;
    for (layer, settings, transform) in placements {
        let source = scene.shader_tree.nodes.iter().find_map(|n| match &n.kind {
            NodeKind::Texture(t) if n.id == layer => Some(t.source.clone()),
            _ => None,
        });
        let Some(source) = source else { continue };
        let settings = settings.unwrap_or_else(|| Scene::starting_locator(&source));
        let name = scene
            .shader_tree
            .get(layer)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        let id = scene
            .add_texture_locator(&mut next_item, &name, settings)
            .map_err(|e| e.to_string())?;
        let usable = transform.is_some_and(|(scale, pos)| {
            scale.iter().all(|v| v.is_finite() && v.abs() > 1e-9)
                && pos.iter().all(|v| v.is_finite())
        });
        if transform.is_some() && !usable {
            notes.look(format!(
                "{name}: texture size left at 1 m (the preset's can't be used)"
            ));
        }
        if let Some((scale, pos)) = transform.filter(|_| usable)
            && let Some(item) = scene.items.iter_mut().find(|i| i.id == id)
        {
            item.transform.scale = glam::DVec3::from_array(scale);
            item.transform.translation = glam::DVec3::from_array(pos);
        }
        if let Some(node) = scene.shader_tree.nodes.iter_mut().find(|n| n.id == layer)
            && let NodeKind::Texture(t) = &mut node.kind
        {
            t.locator = Some(id);
        }
    }
    scene.next_id = scene.next_id.max(next_item);
    // Values past Operandi's ranges come in at the nearest end.
    for n in &mut scene.shader_tree.nodes {
        let name = n.name.clone();
        let clamped: Vec<&str> = n
            .channels()
            .into_iter()
            .filter_map(|(label, mut c)| c.clamp_into_range().then_some(label))
            .collect();
        if !clamped.is_empty() {
            notes.look(format!("{name}: {} clamped to range", clamped.join(", ")));
        }
    }
    scene.validate().map_err(|e| e.to_string())?;
    let total = layers.len();
    let n_dropped = dropped.len();
    // LXP-05: more than half dropped, or every Bump layer, or every
    // Diffuse Color layer (provisional reading, LXP-B06).
    let effect_of = |l: &PresetItem| match target(l.text("effect").unwrap_or("diffColor")) {
        Target::To { effect, .. } => Some(effect),
        Target::Drop(_) => None,
    };
    let all_lost = |e: TextureEffect| {
        let total = layers.iter().filter(|l| effect_of(l) == Some(e)).count();
        let lost = dropped
            .iter()
            .filter(|(l, _)| effect_of(l) == Some(e))
            .count();
        total > 0 && lost == total
    };
    let partial = n_dropped * 2 > total
        || all_lost(TextureEffect::Bump)
        || all_lost(TextureEffect::DiffuseColor);
    if n_dropped > 0 {
        notes.look("some layers not supported");
    }
    Ok(Converted {
        preset: Preset {
            header: PresetHeader {
                kind: PresetKind::Material,
                thumbnail: None,
                source: None,
            },
            scene,
        },
        notes: notes.lines,
        changed_look: notes.look,
        layers: total,
        dropped: n_dropped,
        partial,
    })
}

fn node(id: u64, parent: Option<u64>, name: &str, kind: NodeKind) -> Node {
    Node {
        id,
        parent,
        name: name.into(),
        layer: Layer::default(),
        kind,
        extras: Default::default(),
    }
}

/// The note for a whole conversion, one line, for the status bar.
pub fn note_line(c: &Converted) -> String {
    c.notes.join(" · ")
}

/// For `BTreeMap` users: channel names of an item (tests, the survey).
pub fn channel_names(i: &PresetItem) -> Vec<&str> {
    i.channels.keys().map(String::as_str).collect()
}

#[allow(dead_code)]
fn _assert_types(_: BTreeMap<u32, PresetItem>) {}

#[cfg(test)]
mod tests;
