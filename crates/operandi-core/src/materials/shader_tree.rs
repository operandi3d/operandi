//! Shader tree, first slice (ST-1, specifications/shader-tree-2026-11-05).
//!
//! One tree per scene holds every surfacing, shading and render-output
//! layer. This slice is the data model only: item types, their channels,
//! order and parenting, the default tree, and the edits the Shading panel
//! makes. Nothing here renders yet; compiling the tree into Cycles shaders
//! is ST-4 (TODO in `operandi-cycles`).
//!
//! Layout: `nodes` is one flat list. A node's `parent` is `None` when it
//! sits directly under the Render root, else the id of a Group or Folder.
//! Siblings keep their list order, top row first; the future evaluator
//! reads them bottom to top, so order is preserved exactly.
//!
//! Materials: `Scene::materials` (material.rs) stays the table the viewport
//! and the Cycles bridge read. The tree mirrors it: each material tag gets
//! a Material Group holding a Material, and Set Material copies its colour,
//! Diffuse, Specular and Smoothing into that Material. Other tree channels
//! are stored only until ST-4.
use serde::{Deserialize, Serialize};

pub type NodeId = u64;

/// Most nodes a tree may hold (a sanity budget, like the item limit).
const MAX_SHADER_TREE_NODES: usize = 1_000_000;

pub(crate) fn check_node_count(count: usize) -> crate::Result<()> {
    if count > MAX_SHADER_TREE_NODES {
        return Err(format!(
            "A Shader Tree can have at most {MAX_SHADER_TREE_NODES} nodes"
        ));
    }
    Ok(())
}

macro_rules! labelled_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),+];
            /// The labels, in `ALL`'s order (channel enums).
            pub const LABELS: &[&str] = &[$($label),+];
            pub fn label(self) -> &'static str {
                match self { $($name::$variant => $label),+ }
            }
        }
    };
}

labelled_enum! {
    /// Blend modes, in menu order. A blend only combines
    /// layers of the same effect and only affects layers below it.
    BlendMode {
        Normal => "Normal",
        Add => "Add",
        Subtract => "Subtract",
        Difference => "Difference",
        NormalMultiply => "Normal Multiply",
        Divide => "Divide",
        Multiply => "Multiply",
        Screen => "Screen",
        Overlay => "Overlay",
        SoftLight => "Soft Light",
        HardLight => "Hard Light",
        Lighten => "Lighten",
        Darken => "Darken",
        ColorDodge => "Color Dodge",
        ColorBurn => "Color Burn",
        NormalMapBlend => "Normal Map Blend",
    }
}
// The enum comes from a macro, so the default is spelled out.
#[allow(clippy::derivable_impls)]
impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

labelled_enum! {
    /// What a Render Output writes (render outputs v1 §3, first batch; the
    /// later types aren't listed yet).
    OutputEffect {
        FinalColor => "Final Color",
        Alpha => "Alpha",
        Depth => "Depth",
        ShadingNormal => "Shading Normal",
        GeometricNormal => "Geometric Normal",
        DiffuseShadingTotal => "Diffuse Shading (Total)",
        DiffuseShadingDirect => "Diffuse Shading (Direct)",
        DiffuseShadingIndirect => "Diffuse Shading (Indirect)",
        SpecularShading => "Specular Shading",
        ReflectionShading => "Reflection Shading",
        TransparentShading => "Transparent Shading",
        SubsurfaceShading => "Subsurface Shading",
        LuminousShading => "Luminous Shading",
        DiffuseColor => "Diffuse Color",
        AmbientOcclusion => "Ambient Occlusion",
        ObjectCoordinates => "Object Coordinates",
        MotionVector => "Motion Vector",
        ShadingIncidence => "Shading Incidence",
        // Material values (material_values.rs, MV-Q01..Q08): a material's
        // own values at the first camera hit, not light.
        DiffuseAmount => "Diffuse Amount",
        DiffuseCoefficient => "Diffuse Coefficient",
        DiffuseEnergyConservation => "Diffuse Energy Conservation",
        DiffuseRoughness => "Diffuse Roughness",
        SpecularCoefficient => "Specular Coefficient",
        ReflectionCoefficient => "Reflection Coefficient",
        Roughness => "Roughness",
        SubsurfaceAmount => "Subsurface Amount",
        SubsurfaceColor => "Subsurface Color",
        TransparentAmount => "Transparent Amount",
        TransparentColor => "Transparent Color",
        Dpdu => "dPdu",
        Dpdv => "dPdv",
        // Cycles built-in outputs, group A (2026-11-20).
        IlluminationDirect => "Illumination (Direct)",
        IlluminationIndirect => "Illumination (Indirect)",
        IlluminationTotal => "Illumination (Total)",
        UvCoordinates => "UV Coordinates",
        SegmentId => "Segment ID",
        Cryptomatte => "Cryptomatte",
        // Render outputs group D (2026-11-20, RO-Q18..RO-Q31).
        IlluminationUnshadowed => "Illumination (Unshadowed)",
        DiffuseShadingUnshadowed => "Diffuse Shading (Unshadowed)",
        LightPathExpressions => "Light Path Expressions",
        ShadedAaSamples => "Shaded AA Samples",
    }
}

labelled_enum! {
    /// What a Shader passes on (§7).
    ShaderEffect {
        FullShading => "Full Shading",
        DiffuseShading => "Diffuse Shading",
        SpecularShading => "Specular Shading",
        ReflectionShading => "Reflection Shading",
        TransparentShading => "Transparent Shading",
        SubsurfaceShading => "Subsurface Shading",
        LuminousShading => "Luminous Shading",
        FogShading => "Fog Shading",
    }
}

labelled_enum! {
    /// The one channel a texture layer drives (§7, surface items).
    TextureEffect {
        DiffuseColor => "Diffuse Color",
        DiffuseAmount => "Diffuse Amount",
        DiffuseRoughness => "Diffuse Roughness",
        SpecularColor => "Specular Color",
        SpecularAmount => "Specular Amount",
        SpecularFresnel => "Specular Fresnel",
        ReflectionColor => "Reflection Color",
        ReflectionAmount => "Reflection Amount",
        ReflectionFresnel => "Reflection Fresnel",
        Roughness => "Roughness",
        ClearcoatAmount => "Clearcoat Amount",
        TransparentColor => "Transparent Color",
        TransparentAmount => "Transparent Amount",
        RefractionRoughness => "Refraction Roughness",
        SubsurfaceColor => "Subsurface Color",
        SubsurfaceAmount => "Subsurface Amount",
        LuminousColor => "Luminous Color",
        LuminousAmount => "Luminous Amount",
        Dissolve => "Dissolve",
        Flatness => "Flatness",
        Metallic => "Metallic",
        Sheen => "Sheen",
        SheenTint => "Sheen Tint",
        SpecularTint => "Specular Tint",
        DriverA => "Driver A",
        DriverB => "Driver B",
        DriverC => "Driver C",
        DriverD => "Driver D",
        GroupMask => "Group Mask",
        LayerMask => "Layer Mask",
        TextureOffset => "Texture Offset",
        AnisotropyDirection => "Anisotropy Direction",
        Rgba => "RGBA",
        Stencil => "Stencil",
        Bump => "Bump",
        Displacement => "Displacement",
        Normal => "Normal",
        ObjectNormal => "Object Normal",
        VectorDisplacement => "Vector Displacement",
        // Environments (environments-2026-11-09 §5): a texture layer under
        // an environment sets its colour.
        EnvironmentColor => "Environment Color",
    }
}
impl OutputEffect {
    /// A new output layer's name and its Add Layer entry.
    pub fn layer_name(self) -> &'static str {
        use OutputEffect::*;
        match self {
            FinalColor => "Final Color Output",
            Alpha => "Alpha Output",
            Depth => "Depth Output",
            ShadingNormal => "Shading Normal Output",
            GeometricNormal => "Geometric Normal Output",
            DiffuseShadingTotal => "Diffuse Shading (Total) Output",
            DiffuseShadingDirect => "Diffuse Shading (Direct) Output",
            DiffuseShadingIndirect => "Diffuse Shading (Indirect) Output",
            SpecularShading => "Specular Shading Output",
            ReflectionShading => "Reflection Shading Output",
            TransparentShading => "Transparent Shading Output",
            SubsurfaceShading => "Subsurface Shading Output",
            LuminousShading => "Luminous Shading Output",
            DiffuseColor => "Diffuse Color Output",
            AmbientOcclusion => "Ambient Occlusion Output",
            ObjectCoordinates => "Object Coordinates Output",
            MotionVector => "Motion Vector Output",
            ShadingIncidence => "Shading Incidence Output",
            DiffuseAmount => "Diffuse Amount Output",
            DiffuseCoefficient => "Diffuse Coefficient Output",
            DiffuseEnergyConservation => "Diffuse Energy Conservation Output",
            DiffuseRoughness => "Diffuse Roughness Output",
            SpecularCoefficient => "Specular Coefficient Output",
            ReflectionCoefficient => "Reflection Coefficient Output",
            Roughness => "Roughness Output",
            SubsurfaceAmount => "Subsurface Amount Output",
            SubsurfaceColor => "Subsurface Color Output",
            TransparentAmount => "Transparent Amount Output",
            TransparentColor => "Transparent Color Output",
            Dpdu => "dPdu Output",
            Dpdv => "dPdv Output",
            IlluminationDirect => "Illumination (Direct) Output",
            IlluminationIndirect => "Illumination (Indirect) Output",
            IlluminationTotal => "Illumination (Total) Output",
            UvCoordinates => "UV Coordinates Output",
            SegmentId => "Segment ID Output",
            Cryptomatte => "Cryptomatte Output",
            IlluminationUnshadowed => "Illumination (Unshadowed) Output",
            DiffuseShadingUnshadowed => "Diffuse Shading (Unshadowed) Output",
            LightPathExpressions => "Light Path Expressions Output",
            ShadedAaSamples => "Shaded AA Samples Output",
        }
    }
    /// The material value this output writes, `None` for the others.
    pub fn material_value(self) -> Option<crate::material_values::MaterialValue> {
        use crate::material_values::MaterialValue as V;
        use OutputEffect::*;
        Some(match self {
            DiffuseAmount => V::DiffuseAmount,
            DiffuseCoefficient => V::DiffuseCoefficient,
            DiffuseEnergyConservation => V::DiffuseEnergyConservation,
            DiffuseRoughness => V::DiffuseRoughness,
            SpecularCoefficient => V::SpecularCoefficient,
            ReflectionCoefficient => V::ReflectionCoefficient,
            Roughness => V::Roughness,
            SubsurfaceAmount => V::SubsurfaceAmount,
            SubsurfaceColor => V::SubsurfaceColor,
            TransparentAmount => V::TransparentAmount,
            TransparentColor => V::TransparentColor,
            Dpdu => V::Dpdu,
            Dpdv => V::Dpdv,
            _ => return None,
        })
    }
    /// The material-value outputs, in menu order (Add Layer ▸ Special ▸
    /// Render Output ▸ Material Values, UIC-10).
    pub const MATERIAL_VALUES: &[OutputEffect] = {
        use OutputEffect::*;
        &[
            DiffuseAmount,
            DiffuseCoefficient,
            DiffuseEnergyConservation,
            DiffuseRoughness,
            SpecularCoefficient,
            ReflectionCoefficient,
            Roughness,
            SubsurfaceAmount,
            SubsurfaceColor,
            TransparentAmount,
            TransparentColor,
            Dpdu,
            Dpdv,
        ]
    };
    /// One plain line on what a material-value output writes (UIC-04).
    pub fn material_value_tip(self) -> Option<&'static str> {
        use OutputEffect::*;
        Some(match self {
            DiffuseAmount => "Writes each surface's Diffuse Amount.",
            DiffuseCoefficient => "Writes Diffuse Color times Diffuse Amount.",
            DiffuseEnergyConservation => {
                "Writes the diffuse colour left after transparency, subsurface and metal."
            }
            DiffuseRoughness => "Writes each surface's Diffuse Roughness.",
            SpecularCoefficient => "Writes Specular Color times Specular Amount.",
            ReflectionCoefficient => {
                "Writes Reflection Color times Reflection Amount (specular with Match Specular)."
            }
            Roughness => "Writes each surface's Roughness.",
            SubsurfaceAmount => "Writes each surface's Subsurface Amount.",
            SubsurfaceColor => "Writes each surface's Subsurface Color.",
            TransparentAmount => "Writes each surface's Transparent Amount.",
            TransparentColor => "Writes each surface's Transparent Color.",
            Dpdu => "Writes the surface direction of increasing U, from the first UV map.",
            Dpdv => "Writes the surface direction of increasing V, from the first UV map.",
            _ => return None,
        })
    }
}
impl TextureEffect {
    /// The effect menu's sections, in the hand-off's order (Q-24).
    pub const MENU: &[(&str, &[TextureEffect])] = {
        use TextureEffect::*;
        &[
            (
                "Basic Channels",
                &[
                    DiffuseColor,
                    DiffuseAmount,
                    DiffuseRoughness,
                    SpecularColor,
                    SpecularAmount,
                    SpecularFresnel,
                    ReflectionColor,
                    ReflectionAmount,
                    ReflectionFresnel,
                    Roughness,
                    ClearcoatAmount,
                    TransparentColor,
                    TransparentAmount,
                    RefractionRoughness,
                    SubsurfaceColor,
                    SubsurfaceAmount,
                    LuminousColor,
                    LuminousAmount,
                    Dissolve,
                ],
            ),
            (
                "Principled",
                &[Flatness, Metallic, Sheen, SheenTint, SpecularTint],
            ),
            (
                "Shader Control",
                &[
                    DriverA,
                    DriverB,
                    DriverC,
                    DriverD,
                    GroupMask,
                    LayerMask,
                    TextureOffset,
                ],
            ),
            ("Special Effects", &[AnisotropyDirection, Rgba, Stencil]),
            (
                "Surface Shading",
                &[Bump, Displacement, Normal, ObjectNormal, VectorDisplacement],
            ),
        ]
    };
}

labelled_enum! {
    /// Which kind of polygon tag a Material Group masks by (Q-6).
    TagType {
        Material => "Material",
        Part => "Part",
        SelectionSet => "Selection Set",
    }
}

labelled_enum! {
    /// What a Material Group's mask covers (Q-23: Surface by default).
    Scope {
        Surface => "Surface",
        Fur => "Fur",
        All => "All",
    }
}

labelled_enum! {
    /// Texture projection, set on the layer's Texture Locator
    /// (texture_locator.rs). Stage 1 renders Solid, Planar and UV Map.
    Projection {
        Solid => "Solid",
        Planar => "Planar",
        Cylindrical => "Cylindrical",
        Spherical => "Spherical",
        Cubic => "Cubic",
        Box => "Box",
        Front => "Front",
        UvMap => "UV Map",
        LightProbe => "Light Probe",
        Triplanar => "Triplanar",
        ImplicitUv => "Implicit UV",
    }
}

labelled_enum! {
    /// Shader Alpha Type (§6.4).
    AlphaType {
        Opacity => "Opacity",
        Constant => "Constant",
        ShadowCatcher => "Shadow Catcher",
    }
}

labelled_enum! {
    /// Material shading model (translation map v0.1, 2026-11-06). Only
    /// Principled and Physically Based translate differently; Traditional
    /// and Energy Conserving render as Principled for now (provisional,
    /// ST-Q28).
    ShadingModel {
        Principled => "Principled",
        PhysicallyBased => "Physically Based",
        Traditional => "Traditional",
        EnergyConserving => "Energy Conserving",
    }
}

/// The common "Layer" block (§6.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// The eye column. A disabled layer stays in the tree and is saved.
    pub enable: bool,
    pub invert: bool,
    pub blend: BlendMode,
    /// 0 to 1 (the panel shows percent).
    pub opacity: f64,
    /// Editor Color (note 10): marks the row; no effect on rendering.
    /// Native version 31.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::editor_color::serde_name"
    )]
    pub editor_color: Option<crate::EditorColor>,
}
impl Default for Layer {
    fn default() -> Self {
        Self {
            enable: true,
            invert: false,
            blend: BlendMode::Normal,
            opacity: 1.,
            editor_color: None,
        }
    }
}

/// A Material Group's item mask (§6.2, v0.1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemMask {
    /// "(all)": no item limit.
    #[default]
    All,
    /// One mesh item.
    Item(u64),
    /// Every current member of a group of items, live (Q-43). Operandi has
    /// no item groups yet, so the panel doesn't offer this; the variant
    /// keeps room for it in saved files.
    ItemGroup(u64),
}

/// A Material Group's polygon tag (§6.2, v0.1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagMask {
    /// "(all)": no tag limit.
    #[default]
    All,
    /// "(none)": covers nothing. Changing the tag type resets to this.
    None,
    Name(String),
}
impl TagMask {
    pub fn label(&self) -> &str {
        match self {
            TagMask::All => "(all)",
            TagMask::None => "(none)",
            TagMask::Name(n) => n,
        }
    }
}

/// Material Group mask and options (§6.2). The Scale block is not in this
/// slice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupMask {
    pub item: ItemMask,
    pub tag_type: TagType,
    pub tag: TagMask,
    pub scope: Scope,
    /// On an untagged group: its own layers apply only to surfaces a
    /// tagged group inside it matches (group extras §2).
    pub apply_to_sub_group: bool,
    /// Q-22 placeholder: on.
    pub apply_to_instances: bool,
    /// The Scale block (group extras §1; new in native v22).
    #[serde(default, skip_serializing_if = "GroupScale::is_default")]
    pub scale: GroupScale,
}
impl Default for GroupMask {
    fn default() -> Self {
        Self {
            item: ItemMask::All,
            tag_type: TagType::Material,
            tag: TagMask::All,
            scope: Scope::Surface,
            apply_to_sub_group: false,
            apply_to_instances: true,
            scale: GroupScale::default(),
        }
    }
}
impl GroupMask {
    /// No item and no polygon tag: the group covers everything.
    pub fn is_untagged(&self) -> bool {
        self.item == ItemMask::All && self.tag == TagMask::All
    }
}

/// A Material Group's Scale block (group extras §1): what the Scale
/// resizes, all on at start, and the Scale (1 = 100 %).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupScale {
    pub uv: bool,
    pub size: bool,
    pub bump_amplitude: bool,
    pub absorption_distance: bool,
    pub displacement_distance: bool,
    pub subsurface_distance: bool,
    pub scale: f64,
}
impl Default for GroupScale {
    fn default() -> Self {
        Self {
            uv: true,
            size: true,
            bump_amplitude: true,
            absorption_distance: true,
            displacement_distance: true,
            subsurface_distance: true,
            scale: 1.,
        }
    }
}
impl GroupScale {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Material channels (§6.3), Principled-style with friendly names.
/// Fractions are 0 to 1, colours linear RGB 0 to 1, distances in metres,
/// angles in degrees. Starting values are the reference Base Material's
/// (Tier 1 G-03); the four the material table has (colour, Diffuse,
/// Specular, Smoothing) match `Material::default`.
///
/// Fields added after native version 14 carry a serde default so older
/// files read; every other field must be present, so a stored value is
/// never replaced by a new default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialChannels {
    pub shading_model: ShadingModel,
    pub gl_display: bool,
    pub diffuse_color: [f64; 3],
    pub diffuse_amount: f64,
    pub diffuse_roughness: f64,
    pub specular_color: [f64; 3],
    /// An amount, not the reflectance (`material::specular_reflectance`).
    pub specular_amount: f64,
    /// Specular Fresnel.
    pub fresnel: f64,
    pub roughness: f64,
    pub anisotropy: f64,
    /// Match Specular: while on, Reflection Amount, Color and Fresnel have
    /// no render effect (translation map v0.1). New in version 17; older
    /// files load with it on.
    #[serde(default = "yes")]
    pub match_specular: bool,
    pub reflection_color: [f64; 3],
    pub reflection_amount: f64,
    /// G-02: 0 % reflects the same at every angle, 100 % reflects more at
    /// grazing angles. New in version 15; older files had none (0 %).
    #[serde(default)]
    pub reflection_fresnel: f64,
    pub clearcoat_amount: f64,
    pub clearcoat_roughness: f64,
    /// G-03: how much of the surface bump the clear coat follows. Stored
    /// only until bump renders. New in version 15 (100 % for older files).
    #[serde(default = "full")]
    pub clearcoat_bump: f64,
    pub metallic: f64,
    pub sheen: f64,
    pub sheen_tint: f64,
    pub specular_tint: f64,
    pub flatness: f64,
    pub transparent_color: [f64; 3],
    pub transparent_amount: f64,
    pub refractive_index: f64,
    pub refraction_roughness: f64,
    pub absorption_distance: f64,
    pub dissolve: f64,
    pub subsurface_color: [f64; 3],
    pub subsurface_amount: f64,
    pub subsurface_distance: f64,
    pub luminous_color: [f64; 3],
    pub luminous_intensity: f64,
    pub bump_amplitude: f64,
    pub displacement_distance: f64,
    pub smoothing: f64,
    pub smoothing_angle: f64,
    pub double_sided: bool,
    /// The rarer settings (group extras §3; new in native v22).
    #[serde(default, skip_serializing_if = "MaterialMore::is_default")]
    pub more: MaterialMore,
    /// The OpenPBR slots the older channels don't cover (OPM-02; new in
    /// native v41). Files without it read the values that render exactly
    /// as before ([`OpenPbrChannels::legacy`]); new materials start at
    /// OpenPBR's defaults (OPM-Q11, decided 2026-11-11).
    #[serde(
        default = "OpenPbrChannels::legacy",
        skip_serializing_if = "OpenPbrChannels::is_legacy"
    )]
    pub openpbr: OpenPbrChannels,
}

/// Material channels for OpenPBR Surface 1.1.2 slots with no older
/// Operandi channel (OPM-02, see `openpbr.rs`). New materials start at
/// OpenPBR's defaults (OPM-Q11, Jason 2026-11-11: coat IOR 1.6, the
/// subsurface colour its own 0.8 grey). Lengths are in metres except the
/// thin film, which is in micrometres as OpenPBR has it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OpenPbrChannels {
    /// coat_color: the coat's absorption tint (Cycles Coat Tint).
    pub coat_color: [f64; 3],
    /// coat_ior (Cycles Coat IOR).
    pub coat_ior: f64,
    /// coat_darkening. Stored: Cycles always darkens (OPM-Q10).
    pub coat_darkening: f64,
    /// coat_roughness_anisotropy. Stored: Cycles' coat is isotropic.
    pub coat_roughness_anisotropy: f64,
    /// fuzz_color (Cycles Sheen Tint); the weight is Sheen.
    pub fuzz_color: [f64; 3],
    /// fuzz_roughness (Cycles Sheen Roughness).
    pub fuzz_roughness: f64,
    pub thin_film_weight: f64,
    /// Micrometres (Cycles takes nanometres).
    pub thin_film_thickness: f64,
    pub thin_film_ior: f64,
    /// transmission_dispersion_abbe_number; the scale is Dispersion.
    pub dispersion_abbe_number: f64,
    /// transmission_scatter: the inside's scattering, as a multiple of
    /// 1 / Absorption Distance.
    pub transmission_scatter: [f64; 3],
    pub transmission_scatter_anisotropy: f64,
    /// geometry_thin_walled (Cycles Thin Wall).
    pub thin_walled: bool,
    /// subsurface_color: what the scattering reflects, used when
    /// [`Self::subsurface_uses_diffuse`] is off.
    pub subsurface_albedo: [f64; 3],
    pub subsurface_uses_diffuse: bool,
}
impl Default for OpenPbrChannels {
    fn default() -> Self {
        Self {
            coat_color: [1.; 3],
            coat_ior: 1.6,
            coat_darkening: 1.,
            coat_roughness_anisotropy: 0.,
            fuzz_color: [1.; 3],
            fuzz_roughness: 0.5,
            thin_film_weight: 0.,
            thin_film_thickness: 0.5,
            thin_film_ior: 1.4,
            dispersion_abbe_number: 20.,
            transmission_scatter: [0.; 3],
            transmission_scatter_anisotropy: 0.,
            thin_walled: false,
            subsurface_albedo: [0.8; 3],
            subsurface_uses_diffuse: false,
        }
    }
}
impl ShaderTree {
    /// Every Material without the OpenPBR values, as files before native
    /// v41 have them (for tests and tools that write older versions).
    pub fn legacy_openpbr(&mut self) {
        for node in &mut self.nodes {
            if let NodeKind::Material(m) = &mut node.kind {
                m.openpbr = OpenPbrChannels::legacy();
            }
        }
    }
}
impl OpenPbrChannels {
    /// What materials saved before native v41 had: OpenPBR's defaults
    /// except the two that would change their render, Coat IOR (Cycles
    /// used 1.5) and the subsurface colour (Cycles used Diffuse Color).
    pub fn legacy() -> Self {
        Self {
            coat_ior: 1.5,
            subsurface_uses_diffuse: true,
            ..Self::default()
        }
    }
    pub fn is_legacy(&self) -> bool {
        *self == Self::legacy()
    }
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

labelled_enum! {
    /// Material Reflection Type (stored only).
    ReflectionType {
        FullScene => "Full Scene",
        EnvironmentOnly => "Environment Only",
    }
}

/// Material settings added by group extras v1 §3, with the Base
/// Material's starting values. What renders: Use Refractive Index,
/// Subsurface Phase, Rounded Edge Width; the rest is stored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MaterialMore {
    pub exit_color: [f64; 3],
    pub dispersion: f64,
    pub subsurface_depth: f64,
    /// Raw 0..1; 0.5 scatters evenly (XQ-4).
    pub subsurface_phase: f64,
    /// Metres.
    pub rounded_edge_width: f64,
    pub rounded_edge_angle: f64,
    pub round_same_surface_only: bool,
    pub clip_enable: bool,
    pub matte: bool,
    pub matte_color: [f64; 3],
    pub clip_value: f64,
    pub importance: f64,
    pub reflection_rays: f64,
    pub refraction_rays: f64,
    pub subsurface_samples: f64,
    pub reflection_type: ReflectionType,
    pub blurry_reflection: bool,
    /// On: the reflections come from Refractive Index, not Specular Amount.
    pub use_refractive_index: bool,
    pub same_surface: bool,
    pub gl_color_override: bool,
    pub gl_color: [f64; 3],
    pub volume_scatter: f64,
    pub volume_absorb: f64,
    pub volume_density: f64,
    pub volume_level: f64,
    pub volume_luminous: f64,
}
impl Default for MaterialMore {
    fn default() -> Self {
        Self {
            exit_color: [0.; 3],
            dispersion: 0.,
            subsurface_depth: 1.,
            subsurface_phase: 0.5,
            rounded_edge_width: 0.,
            rounded_edge_angle: 40.,
            round_same_surface_only: false,
            clip_enable: true,
            matte: false,
            matte_color: [0.; 3],
            clip_value: 1.,
            importance: 1.,
            reflection_rays: 64.,
            refraction_rays: 64.,
            subsurface_samples: 64.,
            reflection_type: ReflectionType::FullScene,
            blurry_reflection: true,
            use_refractive_index: false,
            same_surface: false,
            gl_color_override: false,
            gl_color: [1.; 3],
            volume_scatter: 1.,
            volume_absorb: 1.,
            volume_density: 1.,
            volume_level: 0.,
            volume_luminous: 0.,
        }
    }
}
impl MaterialMore {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}
impl Default for MaterialChannels {
    fn default() -> Self {
        let m = crate::Material::default();
        Self {
            shading_model: ShadingModel::Principled,
            gl_display: true,
            diffuse_color: m.color,
            diffuse_amount: m.diffuse,
            diffuse_roughness: 0.,
            specular_color: [1.; 3],
            specular_amount: m.specular,
            fresnel: 1.,
            roughness: 0.4,
            anisotropy: 0.,
            match_specular: true,
            reflection_color: [1.; 3],
            reflection_amount: 0.,
            reflection_fresnel: 0.,
            clearcoat_amount: 0.,
            clearcoat_roughness: 0.,
            clearcoat_bump: 1.,
            metallic: 0.,
            sheen: 0.,
            sheen_tint: 0.,
            specular_tint: 0.,
            flatness: 0.,
            transparent_color: [1.; 3],
            transparent_amount: 0.,
            refractive_index: 1.,
            refraction_roughness: 0.,
            absorption_distance: 0.,
            dissolve: 0.,
            subsurface_color: [0.8; 3],
            subsurface_amount: 0.,
            subsurface_distance: 0.005,
            luminous_color: [1.; 3],
            luminous_intensity: 0.,
            bump_amplitude: 0.005,
            displacement_distance: 0.02,
            smoothing: if m.smoothing { 1. } else { 0. },
            smoothing_angle: 40.,
            double_sided: false,
            more: MaterialMore::default(),
            openpbr: OpenPbrChannels::default(),
        }
    }
}

impl MaterialChannels {
    /// The starting values before Tier 1 G-03 (version 14 and earlier),
    /// Specular already converted to an amount. Older files that never
    /// changed them keep rendering as the studio clay.
    pub fn before_tier1() -> Self {
        Self {
            diffuse_color: [0.8; 3],
            specular_amount: crate::material::specular_amount_for_reflectance(0.04),
            fresnel: 0.,
            refractive_index: 1.5,
            subsurface_color: [1.; 3],
            subsurface_distance: 0.01,
            openpbr: OpenPbrChannels::legacy(),
            ..Self::default()
        }
    }
}

fn full() -> f64 {
    1.
}

fn yes() -> bool {
    true
}

/// One editable channel, for the Properties panel and validation.
pub enum Channel<'a> {
    /// 0 to 1, shown as percent.
    Percent(&'a mut f64),
    /// A fraction shown as percent, in its own range (e.g. Brightness may
    /// go past 100 %).
    PercentIn(&'a mut f64, f64, f64),
    Color(&'a mut [f64; 3]),
    /// Metres, 0 or more.
    Distance(&'a mut f64),
    /// Degrees, 0 to 180.
    Angle(&'a mut f64),
    /// A plain number in a range.
    Number(&'a mut f64, f64, f64),
    Bool(&'a mut bool),
}
impl Channel<'_> {
    pub(crate) fn valid(&self) -> bool {
        let unit = |v: f64| v.is_finite() && (0.0..=1.0).contains(&v);
        match self {
            Channel::Percent(v) => unit(**v),
            Channel::PercentIn(v, lo, hi) => v.is_finite() && (*lo..=*hi).contains(*v),
            Channel::Color(c) => c.iter().all(|&v| unit(v)),
            Channel::Distance(v) => v.is_finite() && (0.0..=1e6).contains(*v),
            Channel::Angle(v) => v.is_finite() && (0.0..=180.0).contains(*v),
            Channel::Number(v, lo, hi) => v.is_finite() && (*lo..=*hi).contains(*v),
            Channel::Bool(_) => true,
        }
    }
    /// Brings an out-of-range value into range (NaN to the low end), for
    /// importers; whether it changed.
    pub(crate) fn clamp_into_range(&mut self) -> bool {
        if self.valid() {
            return false;
        }
        let fix = |v: &mut f64, lo: f64, hi: f64| {
            *v = if v.is_nan() { lo } else { v.clamp(lo, hi) };
        };
        match self {
            Channel::Percent(v) => fix(v, 0., 1.),
            Channel::PercentIn(v, lo, hi) => fix(v, *lo, *hi),
            Channel::Color(c) => c.iter_mut().for_each(|v| fix(v, 0., 1.)),
            Channel::Distance(v) => fix(v, 0., 1e6),
            Channel::Angle(v) => fix(v, 0., 180.),
            Channel::Number(v, lo, hi) => fix(v, *lo, *hi),
            Channel::Bool(_) => {}
        }
        true
    }
}

impl MaterialChannels {
    /// Every channel with its panel name, in panel order.
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        use Channel::*;
        vec![
            ("GL Display", Bool(&mut self.gl_display)),
            ("Diffuse Color", Color(&mut self.diffuse_color)),
            ("Diffuse Amount", Percent(&mut self.diffuse_amount)),
            ("Diffuse Roughness", Percent(&mut self.diffuse_roughness)),
            ("Specular Color", Color(&mut self.specular_color)),
            ("Specular Amount", Percent(&mut self.specular_amount)),
            ("Specular Fresnel", Percent(&mut self.fresnel)),
            ("Roughness", Percent(&mut self.roughness)),
            ("Anisotropy", Percent(&mut self.anisotropy)),
            ("Match Specular", Bool(&mut self.match_specular)),
            ("Reflection Color", Color(&mut self.reflection_color)),
            ("Reflection Amount", Percent(&mut self.reflection_amount)),
            ("Reflection Fresnel", Percent(&mut self.reflection_fresnel)),
            ("Clearcoat Amount", Percent(&mut self.clearcoat_amount)),
            (
                "Clearcoat Roughness",
                Percent(&mut self.clearcoat_roughness),
            ),
            ("Clearcoat Bump", Percent(&mut self.clearcoat_bump)),
            ("Coat Color", Color(&mut self.openpbr.coat_color)),
            ("Coat IOR", Number(&mut self.openpbr.coat_ior, 1., 10.)),
            ("Coat Darkening", Percent(&mut self.openpbr.coat_darkening)),
            (
                "Coat Anisotropy",
                Percent(&mut self.openpbr.coat_roughness_anisotropy),
            ),
            ("Metallic", Percent(&mut self.metallic)),
            ("Sheen", Percent(&mut self.sheen)),
            ("Sheen Tint", Percent(&mut self.sheen_tint)),
            ("Sheen Color", Color(&mut self.openpbr.fuzz_color)),
            ("Sheen Roughness", Percent(&mut self.openpbr.fuzz_roughness)),
            ("Specular Tint", Percent(&mut self.specular_tint)),
            ("Flatness", Percent(&mut self.flatness)),
            ("Thin Film", Percent(&mut self.openpbr.thin_film_weight)),
            (
                "Thin Film Thickness",
                Number(&mut self.openpbr.thin_film_thickness, 0., 100.),
            ),
            (
                "Thin Film IOR",
                Number(&mut self.openpbr.thin_film_ior, 1., 10.),
            ),
            ("Transparent Color", Color(&mut self.transparent_color)),
            ("Transparent Amount", Percent(&mut self.transparent_amount)),
            (
                "Refractive Index",
                Number(&mut self.refractive_index, 1., 10.),
            ),
            (
                "Refraction Roughness",
                Percent(&mut self.refraction_roughness),
            ),
            (
                "Absorption Distance",
                Distance(&mut self.absorption_distance),
            ),
            (
                "Transmission Scatter",
                Color(&mut self.openpbr.transmission_scatter),
            ),
            (
                "Scatter Anisotropy",
                Number(&mut self.openpbr.transmission_scatter_anisotropy, -1., 1.),
            ),
            ("Thin Walled", Bool(&mut self.openpbr.thin_walled)),
            ("Dissolve", Percent(&mut self.dissolve)),
            ("Subsurface Color", Color(&mut self.subsurface_color)),
            ("Subsurface Amount", Percent(&mut self.subsurface_amount)),
            (
                "Subsurface Distance",
                Distance(&mut self.subsurface_distance),
            ),
            (
                "Albedo from Diffuse",
                Bool(&mut self.openpbr.subsurface_uses_diffuse),
            ),
            (
                "Subsurface Albedo",
                Color(&mut self.openpbr.subsurface_albedo),
            ),
            ("Luminous Color", Color(&mut self.luminous_color)),
            (
                "Luminous Intensity",
                Number(&mut self.luminous_intensity, 0., 1e6),
            ),
            ("Bump Amplitude", Distance(&mut self.bump_amplitude)),
            (
                "Displacement Distance",
                Distance(&mut self.displacement_distance),
            ),
            ("Smoothing", Percent(&mut self.smoothing)),
            ("Smoothing Angle", Angle(&mut self.smoothing_angle)),
            ("Double Sided", Bool(&mut self.double_sided)),
            (
                "Use Refractive Index",
                Bool(&mut self.more.use_refractive_index),
            ),
            ("Exit Color", Color(&mut self.more.exit_color)),
            ("Dispersion", Percent(&mut self.more.dispersion)),
            (
                "Abbe Number",
                Number(&mut self.openpbr.dispersion_abbe_number, 1., 1000.),
            ),
            (
                "Subsurface Depth",
                Number(&mut self.more.subsurface_depth, 0., 1e3),
            ),
            ("Subsurface Phase", Percent(&mut self.more.subsurface_phase)),
            (
                "Rounded Edge Width",
                Distance(&mut self.more.rounded_edge_width),
            ),
            (
                "Rounded Edge Angle",
                Angle(&mut self.more.rounded_edge_angle),
            ),
            (
                "Round Same Surface Only",
                Bool(&mut self.more.round_same_surface_only),
            ),
            ("Clip Enable", Bool(&mut self.more.clip_enable)),
            ("Matte", Bool(&mut self.more.matte)),
            ("Matte Color", Color(&mut self.more.matte_color)),
            ("Clip Value", Percent(&mut self.more.clip_value)),
            ("Importance", PercentIn(&mut self.more.importance, 0., 100.)),
            (
                "Reflection Rays",
                Number(&mut self.more.reflection_rays, 1., 1e4),
            ),
            (
                "Refraction Rays",
                Number(&mut self.more.refraction_rays, 1., 1e4),
            ),
            (
                "Subsurface Samples",
                Number(&mut self.more.subsurface_samples, 1., 1e4),
            ),
            ("Blurry Reflection", Bool(&mut self.more.blurry_reflection)),
            ("Same Surface", Bool(&mut self.more.same_surface)),
            ("GL Color Override", Bool(&mut self.more.gl_color_override)),
            ("GL Color", Color(&mut self.more.gl_color)),
            (
                "Volume Scattering",
                Number(&mut self.more.volume_scatter, 0., 1e6),
            ),
            (
                "Volume Absorption",
                Number(&mut self.more.volume_absorb, 0., 1e6),
            ),
            (
                "Volume Density",
                Number(&mut self.more.volume_density, 0., 1e6),
            ),
            ("Volume Level", Number(&mut self.more.volume_level, 0., 1e6)),
            (
                "Volume Luminous",
                Number(&mut self.more.volume_luminous, 0., 1e6),
            ),
        ]
    }
    /// Copy the settings the material table has (material.rs).
    pub fn copy_from(&mut self, m: &crate::Material) {
        self.diffuse_color = m.color;
        self.diffuse_amount = m.diffuse;
        self.specular_amount = m.specular;
        self.smoothing = if m.smoothing { 1. } else { 0. };
    }
}

labelled_enum! {
    /// Shader Fog Type (shader item §8).
    FogType {
        None => "None",
        Linear => "Linear",
        Exponential => "Exponential",
        Layered => "Layered",
        Underwater => "Underwater",
    }
}

labelled_enum! {
    /// Shader Indirect Illum Type (shader item §1). Stored only: Cycles
    /// always path traces. Only the starting value is specified, so it is
    /// the one entry (provisional, SR-Q01).
    IndirectIllumType {
        IrradianceCaching => "Irradiance Caching",
    }
}

/// Shader channels (§6.4 and shader item v1, 2026-11-07). The fields up
/// to `alpha_value` are the first slice's; the rest live in `more` so
/// files without them read, and a shader that leaves them at their
/// starting values saves as before.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderChannels {
    pub effect: ShaderEffect,
    pub gl_display: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub visible_to_camera: bool,
    pub visible_to_indirect: bool,
    pub visible_to_reflection: bool,
    pub visible_to_refraction: bool,
    pub alpha_type: AlphaType,
    pub alpha_value: f64,
    #[serde(default, skip_serializing_if = "ShaderMore::is_default")]
    pub more: ShaderMore,
}
impl Default for ShaderChannels {
    fn default() -> Self {
        Self {
            effect: ShaderEffect::FullShading,
            gl_display: true,
            cast_shadows: true,
            receive_shadows: true,
            visible_to_camera: true,
            visible_to_indirect: true,
            visible_to_reflection: true,
            visible_to_refraction: true,
            alpha_type: AlphaType::Opacity,
            alpha_value: 1.,
            more: ShaderMore::default(),
        }
    }
}

/// The Shader settings added by the shader item spec v1 (§1), with their
/// starting values. Fractions are 0 to 1 (shown as percent), distances in
/// metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShaderMore {
    /// The five control switches (§3): off takes that section from the
    /// next applicable shader below (SH-1).
    pub compute_shading: bool,
    pub control_quality: bool,
    pub control_visibility: bool,
    pub control_light_linking: bool,
    pub control_fog: bool,
    /// Stored only.
    pub shading_rate: f64,
    /// §4: scales light arriving straight from lights.
    pub direct_multiplier: f64,
    /// §4: bounced light. Stored only (no per-surface Cycles control).
    pub indirect_multiplier: f64,
    /// Stored only.
    pub incoming_indirect_saturation: f64,
    pub outgoing_indirect_saturation: f64,
    pub indirect_type: IndirectIllumType,
    /// Stored only until the Occlusion texture and AO output exist.
    pub visible_to_occlusion: bool,
    /// §7: an item group of lights; `None` = no linking.
    pub light_group: Option<u64>,
    pub link_mode: crate::camera_light::LinkMode,
    /// §8.
    pub fog_type: FogType,
    pub fog_use_environment: bool,
    pub fog_color: [f64; 3],
    pub fog_start: f64,
    pub fog_end: f64,
    pub fog_density: f64,
}
impl Default for ShaderMore {
    fn default() -> Self {
        Self {
            compute_shading: true,
            control_quality: true,
            control_visibility: true,
            control_light_linking: true,
            control_fog: true,
            shading_rate: 1.,
            direct_multiplier: 1.,
            indirect_multiplier: 1.,
            incoming_indirect_saturation: 1.,
            outgoing_indirect_saturation: 1.,
            indirect_type: IndirectIllumType::IrradianceCaching,
            visible_to_occlusion: true,
            light_group: None,
            link_mode: crate::camera_light::LinkMode::Exclude,
            fog_type: FogType::None,
            fog_use_environment: false,
            fog_color: [0.5; 3],
            fog_start: 0.,
            fog_end: 10.,
            fog_density: 0.1,
        }
    }
}
impl ShaderMore {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl ShaderChannels {
    /// Every number and switch with its panel name, by panel section (the
    /// menus are drawn by the panel).
    pub fn sections(&mut self) -> Vec<(&'static str, Vec<(&'static str, Channel<'_>)>)> {
        use Channel::*;
        let m = &mut self.more;
        vec![
            (
                "Shading",
                vec![
                    ("GL Display", Bool(&mut self.gl_display)),
                    ("Compute Shading", Bool(&mut m.compute_shading)),
                ],
            ),
            (
                "Quality",
                vec![
                    ("Control Quality", Bool(&mut m.control_quality)),
                    ("Shading Rate", Number(&mut m.shading_rate, 0.01, 100.)),
                    (
                        "Direct Illum Multiplier",
                        PercentIn(&mut m.direct_multiplier, 0., 100.),
                    ),
                    (
                        "Indirect Illum Multiplier",
                        PercentIn(&mut m.indirect_multiplier, 0., 100.),
                    ),
                    (
                        "Incoming Indirect Saturation",
                        PercentIn(&mut m.incoming_indirect_saturation, 0., 100.),
                    ),
                    (
                        "Outgoing Indirect Saturation",
                        PercentIn(&mut m.outgoing_indirect_saturation, 0., 100.),
                    ),
                ],
            ),
            (
                "Visibility",
                vec![
                    ("Control Visibility", Bool(&mut m.control_visibility)),
                    ("Cast Shadows", Bool(&mut self.cast_shadows)),
                    ("Receive Shadows", Bool(&mut self.receive_shadows)),
                    ("Visible to Camera", Bool(&mut self.visible_to_camera)),
                    (
                        "Visible to Indirect Rays",
                        Bool(&mut self.visible_to_indirect),
                    ),
                    (
                        "Visible to Reflection Rays",
                        Bool(&mut self.visible_to_reflection),
                    ),
                    (
                        "Visible to Refraction Rays",
                        Bool(&mut self.visible_to_refraction),
                    ),
                    (
                        "Visible to Occlusion Rays",
                        Bool(&mut m.visible_to_occlusion),
                    ),
                    ("Alpha Value", Percent(&mut self.alpha_value)),
                ],
            ),
            (
                "Light Linking",
                vec![("Control Light Linking", Bool(&mut m.control_light_linking))],
            ),
            (
                "Fog",
                vec![
                    ("Control Fog", Bool(&mut m.control_fog)),
                    ("Use Environment Color", Bool(&mut m.fog_use_environment)),
                    ("Fog Color", Color(&mut m.fog_color)),
                    ("Fog Start Distance", Distance(&mut m.fog_start)),
                    ("Fog End Distance", Distance(&mut m.fog_end)),
                    ("Fog Density", PercentIn(&mut m.fog_density, 0., 100.)),
                ],
            ),
        ]
    }
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        self.sections().into_iter().flat_map(|(_, c)| c).collect()
    }

    /// This shader laid over the one below it (SH-1): each section whose
    /// control switch is off comes from `below` instead. GL Display has no
    /// switch and is always this shader's.
    pub fn over(&self, below: &ShaderChannels) -> ShaderChannels {
        let mut out = self.clone();
        let m = &self.more;
        if !m.compute_shading {
            out.effect = below.effect;
        }
        if !m.control_quality {
            out.more.shading_rate = below.more.shading_rate;
            out.more.direct_multiplier = below.more.direct_multiplier;
            out.more.indirect_multiplier = below.more.indirect_multiplier;
            out.more.incoming_indirect_saturation = below.more.incoming_indirect_saturation;
            out.more.outgoing_indirect_saturation = below.more.outgoing_indirect_saturation;
            out.more.indirect_type = below.more.indirect_type;
        }
        if !m.control_visibility {
            out.cast_shadows = below.cast_shadows;
            out.receive_shadows = below.receive_shadows;
            out.visible_to_camera = below.visible_to_camera;
            out.visible_to_indirect = below.visible_to_indirect;
            out.visible_to_reflection = below.visible_to_reflection;
            out.visible_to_refraction = below.visible_to_refraction;
            out.more.visible_to_occlusion = below.more.visible_to_occlusion;
            out.alpha_type = below.alpha_type;
            out.alpha_value = below.alpha_value;
        }
        if !m.control_light_linking {
            out.more.light_group = below.more.light_group;
            out.more.link_mode = below.more.link_mode;
        }
        if !m.control_fog {
            out.more.fog_type = below.more.fog_type;
            out.more.fog_use_environment = below.more.fog_use_environment;
            out.more.fog_color = below.more.fog_color;
            out.more.fog_start = below.more.fog_start;
            out.more.fog_end = below.more.fog_end;
            out.more.fog_density = below.more.fog_density;
        }
        out
    }

    /// The fog blend factor at `d` metres from the camera and `height`
    /// metres above Y 0 (§8, SH-4, SH-5): 0 keeps the surface, 1 is all
    /// fog colour.
    pub fn fog_factor(&self, d: f64, height: f64) -> f64 {
        let m = &self.more;
        let exponential = |density: f64| 1. - (-density * d.max(0.)).exp();
        match m.fog_type {
            FogType::None => 0.,
            FogType::Linear => {
                let span = m.fog_end - m.fog_start;
                if span <= 0. {
                    if d >= m.fog_end { 1. } else { 0. }
                } else {
                    ((d - m.fog_start) / span).clamp(0., 1.)
                }
            }
            FogType::Exponential | FogType::Underwater => exponential(m.fog_density),
            FogType::Layered => exponential(m.fog_density * (-height.max(0.)).exp()),
        }
    }
}

labelled_enum! {
    /// Image Map Swizzling (Tier 1 G-05): which image channels make the
    /// layer's colour. RGBA is "off".
    Swizzle {
        Rgba => "RGBA",
        Rgb => "RGB",
        Alpha => "Alpha",
        Red => "Red",
        Green => "Green",
        Blue => "Blue",
    }
}

labelled_enum! {
    /// Image Map Alpha (G-05): use the image's alpha as coverage, ignore it,
    /// or use only the alpha, as the layer's value.
    ImageAlpha {
        Use => "Use",
        Ignore => "Ignore",
        AlphaOnly => "Alpha Only",
    }
}

labelled_enum! {
    /// Image Map Filtering (G-05), Cycles' image interpolation.
    Filtering {
        Nearest => "Nearest",
        Bilinear => "Bilinear",
        Bicubic => "Bicubic",
    }
}

/// An Image Map's adjustments (Tier 1 G-05), with the reference starting
/// values. Fractions are 0 to 1 (shown as percent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSettings {
    /// Scalar effects: black maps to Low, white to High.
    pub low_value: f64,
    pub high_value: f64,
    pub invert_red: bool,
    pub invert_green: bool,
    pub invert_blue: bool,
    pub gamma: f64,
    pub brightness: f64,
    pub contrast: f64,
    pub clamp: bool,
    pub swizzle: Swizzle,
    pub alpha: ImageAlpha,
    pub filtering: Filtering,
    /// Stored only (provisional): Cycles filters by `filtering` alone.
    pub antialiasing: bool,
    pub antialiasing_strength: f64,
    pub minimum_spot: f64,
    /// PSD Layer (PNT-302): which layer of a Photoshop image the map shows,
    /// by name; `None` is the Composite. Absent from files without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub psd_layer: Option<String>,
}
impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            low_value: 0.,
            high_value: 1.,
            invert_red: false,
            invert_green: false,
            invert_blue: false,
            gamma: 1.,
            brightness: 1.,
            contrast: 1.,
            clamp: false,
            swizzle: Swizzle::Rgba,
            alpha: ImageAlpha::Use,
            filtering: Filtering::Bilinear,
            antialiasing: true,
            antialiasing_strength: 1.,
            minimum_spot: 1.,
            psd_layer: None,
        }
    }
}
impl ImageSettings {
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        use Channel::*;
        vec![
            ("Low Value", PercentIn(&mut self.low_value, -10., 10.)),
            ("High Value", PercentIn(&mut self.high_value, -10., 10.)),
            ("Invert Red", Bool(&mut self.invert_red)),
            ("Invert Green", Bool(&mut self.invert_green)),
            ("Invert Blue", Bool(&mut self.invert_blue)),
            ("Gamma", Number(&mut self.gamma, 0.01, 10.)),
            ("Brightness", PercentIn(&mut self.brightness, 0., 10.)),
            ("Contrast", PercentIn(&mut self.contrast, 0., 10.)),
            ("Clamp", Bool(&mut self.clamp)),
            ("Antialiasing", Bool(&mut self.antialiasing)),
            (
                "Antialiasing Strength",
                Percent(&mut self.antialiasing_strength),
            ),
            ("Minimum Spot", Number(&mut self.minimum_spot, 0.01, 1e3)),
        ]
    }
}

/// The two-zone block every pattern texture shares (Tier 1 G-04): the
/// pattern's 0..1 value is shaped by Bias and Gain, then picks between
/// Zone 1 (at 0) and Zone 2 (at 1). Built ahead of the procedural
/// textures that will use it; no layer carries one yet (ST-Q13).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoZone {
    pub zone1_value: f64,
    pub zone1_color: [f64; 3],
    pub zone1_alpha: f64,
    pub zone2_value: f64,
    pub zone2_color: [f64; 3],
    pub zone2_alpha: f64,
    /// 0 to 1; 0.5 leaves the pattern alone.
    pub bias: f64,
    pub gain: f64,
}
impl Default for TwoZone {
    fn default() -> Self {
        Self {
            zone1_value: 0.,
            zone1_color: [0.; 3],
            zone1_alpha: 1.,
            zone2_value: 1.,
            zone2_color: [1.; 3],
            zone2_alpha: 1.,
            bias: 0.5,
            gain: 0.5,
        }
    }
}
impl TwoZone {
    /// The panel's rows.
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        use Channel::*;
        vec![
            ("Value 1", Number(&mut self.zone1_value, -1e6, 1e6)),
            ("Color 1", Color(&mut self.zone1_color)),
            ("Alpha 1", Percent(&mut self.zone1_alpha)),
            ("Value 2", Number(&mut self.zone2_value, -1e6, 1e6)),
            ("Color 2", Color(&mut self.zone2_color)),
            ("Alpha 2", Percent(&mut self.zone2_alpha)),
            ("Bias", Percent(&mut self.bias)),
            ("Gain", Percent(&mut self.gain)),
        ]
    }
    /// Perlin's bias curve: `t^(ln b / ln 0.5)`; 0.5 is the identity,
    /// higher values push the pattern toward Zone 2 (provisional).
    pub fn bias(b: f64, t: f64) -> f64 {
        let b = b.clamp(1e-6, 1. - 1e-6);
        let t = t.clamp(0., 1.);
        if t == 0. {
            0.
        } else {
            t.powf(b.ln() / 0.5f64.ln())
        }
    }
    /// Perlin's gain curve: bias applied symmetrically about 0.5; 0.5 is
    /// the identity, higher values sharpen the change between the zones
    /// (provisional).
    pub fn gain(g: f64, t: f64) -> f64 {
        let t = t.clamp(0., 1.);
        let g = 1. - g.clamp(1e-6, 1. - 1e-6);
        if t < 0.5 {
            Self::bias(g, 2. * t) / 2.
        } else {
            1. - Self::bias(g, 2. - 2. * t) / 2.
        }
    }
    /// The zone blend for a pattern value `t` (0 = Zone 1, 1 = Zone 2):
    /// value, colour and alpha.
    pub fn sample(&self, t: f64) -> (f64, [f64; 3], f64) {
        let w = Self::gain(self.gain, Self::bias(self.bias, t));
        let lerp = |a: f64, b: f64| a + (b - a) * w;
        (
            lerp(self.zone1_value, self.zone2_value),
            [0, 1, 2].map(|i| lerp(self.zone1_color[i], self.zone2_color[i])),
            lerp(self.zone1_alpha, self.zone2_alpha),
        )
    }
}

/// What a texture layer samples.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextureSource {
    /// One colour, or one value for scalar effects (§6.6). Keeps its single
    /// colour and value (provisional, ST-Q13).
    Constant { color: [f64; 3], value: f64 },
    /// A scene image (`Scene::images` id), none until one is chosen, with
    /// its adjustments (G-05; absent before version 15: the defaults).
    ImageMap {
        image: Option<u64>,
        #[serde(default)]
        settings: Box<ImageSettings>,
    },
    /// A procedural pattern with two zones (procedural.rs). New in native
    /// version 20.
    Procedural(Box<crate::procedural::Procedural>),
    /// An input looked up on a curve (gradient.rs, new in native v22).
    Gradient(Box<crate::gradient::Gradient>),
    /// A mesh's weight or colour map (vertex_map_texture.rs, v22). A
    /// weight goes through the two-zone block.
    VertexMap {
        map: Option<crate::vertex_map_texture::VertexMapRef>,
        #[serde(default)]
        zones: TwoZone,
    },
    /// A random number per element looked up on curves (v22).
    Variation(Box<crate::vertex_map_texture::Variation>),
    /// An adjustment of the texture layer directly below (v22).
    Process(Box<crate::process_occlusion::Process>),
    /// How open the point is, through the two-zone block (v22).
    Occlusion(Box<crate::process_occlusion::Occlusion>),
}
impl TextureSource {
    pub fn label(&self) -> &'static str {
        match self {
            TextureSource::Constant { .. } => "Constant",
            TextureSource::ImageMap { .. } => "Image Map",
            TextureSource::Procedural(p) => p.kind().label(),
            TextureSource::Gradient(_) => "Gradient",
            TextureSource::VertexMap { .. } => "Vertex Map Texture",
            TextureSource::Variation(_) => "Variation Texture",
            TextureSource::Process(_) => "Process",
            TextureSource::Occlusion(_) => "Occlusion",
        }
    }
}

/// A texture layer (§6.6). Where it lands on a surface comes from its
/// Texture Locator item (texture_locator.rs); several layers may share one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureLayer {
    pub source: TextureSource,
    pub effect: TextureEffect,
    /// The Texture Locator item (Locator setting). `None` only until the
    /// scene gives it one (older files, layers added outside the panel);
    /// such a layer, or one whose locator was deleted, renders as UV Map
    /// with repeat 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<u64>,
    /// Before native version 19 the projection was stored on the layer
    /// (and never rendered). Read only, to convert.
    #[serde(default, rename = "projection", skip_serializing)]
    pub legacy_projection: Option<Projection>,
    #[serde(default, rename = "axis", skip_serializing)]
    pub legacy_axis: Option<u8>,
    /// Ignore Scale Group (group extras §1): enclosing groups' Scale
    /// leaves this layer's size alone. New in native v22.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_scale_group: bool,
    /// NM-13: instead of `effect`, drive this exposed input of the Node
    /// Material below it (by exposed name). Native v38.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_input: Option<Box<String>>,
}

/// A Node Material layer (NM-10..16): a finished surface from a node graph
/// in the shader tree's Nodes section, blended over the result below by
/// its opacity. Native v38.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeMaterialLayer {
    /// The graph (`ShaderTree::node_graphs` id); `None` until one is picked.
    pub graph: Option<u64>,
    /// This layer's own values for exposed inputs, by exposed name; the
    /// others come from the graph's defaults (NM-04).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub overrides: std::collections::BTreeMap<String, Vec<f64>>,
}

/// Item types under Render (§2.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum NodeKind {
    RenderOutput(OutputEffect),
    Shader(ShaderChannels),
    Material(Box<MaterialChannels>),
    /// Material Group: container + mask + blend.
    Group(GroupMask),
    /// Pure organiser.
    Folder,
    Texture(TextureLayer),
    /// Node Material (NM-10).
    NodeMaterial(NodeMaterialLayer),
}

/// The types Add Layer offers in this slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewLayer {
    Folder,
    Group,
    Shader,
    RenderOutput(OutputEffect),
    Material,
    Constant,
    ImageMap,
    /// Add Layer > Textures.
    Procedural(crate::procedural::ProceduralKind),
    Gradient,
    VertexMap,
    Variation,
    Process,
    Occlusion,
    /// A Node Material with no graph yet (NM-10).
    NodeMaterial,
}
impl NewLayer {
    pub fn label(self) -> &'static str {
        match self {
            NewLayer::Folder => "Folder",
            NewLayer::Group => "Group",
            NewLayer::Shader => "Shader",
            NewLayer::RenderOutput(effect) => effect.layer_name(),
            NewLayer::Material => "Material",
            NewLayer::Constant => "Constant",
            NewLayer::ImageMap => "Image Map",
            NewLayer::Procedural(kind) => kind.label(),
            NewLayer::Gradient => "Gradient",
            NewLayer::VertexMap => "Vertex Map Texture",
            NewLayer::Variation => "Variation Texture",
            NewLayer::Process => "Process",
            NewLayer::Occlusion => "Occlusion",
            NewLayer::NodeMaterial => "Node Material",
        }
    }
    pub(crate) fn kind(self) -> NodeKind {
        // The layer's locator item is made by the editor (it needs the
        // scene); its projection depends on the source (texture_locator.rs).
        let texture = |source: TextureSource| {
            NodeKind::Texture(TextureLayer {
                source,
                effect: TextureEffect::DiffuseColor,
                locator: None,
                legacy_projection: None,
                legacy_axis: None,
                ignore_scale_group: false,
                node_input: None,
            })
        };
        match self {
            NewLayer::Folder => NodeKind::Folder,
            // G-20: a new Material Group covers All.
            NewLayer::Group => NodeKind::Group(GroupMask {
                scope: Scope::All,
                ..GroupMask::default()
            }),
            NewLayer::Shader => NodeKind::Shader(ShaderChannels::default()),
            NewLayer::RenderOutput(effect) => NodeKind::RenderOutput(effect),
            // G-20: as the Base Material, without bump or displacement.
            NewLayer::Material => NodeKind::Material(Box::new(MaterialChannels {
                bump_amplitude: 0.,
                displacement_distance: 0.,
                ..MaterialChannels::default()
            })),
            // G-20: black, value 100 %.
            NewLayer::Constant => texture(TextureSource::Constant {
                color: [0.; 3],
                value: 1.,
            }),
            NewLayer::ImageMap => texture(TextureSource::ImageMap {
                image: None,
                settings: Box::default(),
            }),
            NewLayer::Procedural(kind) => texture(TextureSource::Procedural(Box::new(
                crate::procedural::Procedural::new(kind),
            ))),
            NewLayer::Gradient => texture(TextureSource::Gradient(Box::default())),
            // [measured] no map, zones black 0 % to white 100 %, 50 / 50.
            NewLayer::VertexMap => texture(TextureSource::VertexMap {
                map: None,
                zones: TwoZone::default(),
            }),
            NewLayer::Variation => texture(TextureSource::Variation(Box::default())),
            NewLayer::Process => texture(TextureSource::Process(Box::default())),
            NewLayer::Occlusion => texture(TextureSource::Occlusion(Box::default())),
            NewLayer::NodeMaterial => NodeKind::NodeMaterial(NodeMaterialLayer::default()),
        }
    }
}

impl NodeKind {
    /// The starting values of a layer like this one (CH-Q1: what the
    /// channel status light compares with and Reset to Default restores).
    /// A texture keeps its source type, image or map; the Base Material
    /// (`base_material`) starts from the full material defaults, other
    /// materials as Add Layer makes them. `None` for a Folder.
    pub fn defaults(&self, base_material: bool) -> Option<NodeKind> {
        Some(match self {
            NodeKind::Folder => return None,
            NodeKind::RenderOutput(e) => NewLayer::RenderOutput(*e).kind(),
            NodeKind::Shader(_) => NewLayer::Shader.kind(),
            NodeKind::Material(_) if base_material => NodeKind::Material(Box::default()),
            NodeKind::Material(_) => NewLayer::Material.kind(),
            NodeKind::Group(_) => NodeKind::Group(GroupMask::default()),
            // Defaults keep the graph and drop the overrides.
            NodeKind::NodeMaterial(n) => NodeKind::NodeMaterial(NodeMaterialLayer {
                graph: n.graph,
                overrides: Default::default(),
            }),
            NodeKind::Texture(t) => {
                let fresh = match &t.source {
                    TextureSource::Constant { .. } => NewLayer::Constant.kind(),
                    TextureSource::ImageMap { image, .. } => NodeKind::Texture(TextureLayer {
                        source: TextureSource::ImageMap {
                            image: *image,
                            settings: Box::default(),
                        },
                        ..t.clone()
                    }),
                    TextureSource::Procedural(p) => NewLayer::Procedural(p.kind()).kind(),
                    TextureSource::Gradient(_) => NewLayer::Gradient.kind(),
                    TextureSource::VertexMap { map, .. } => NodeKind::Texture(TextureLayer {
                        source: TextureSource::VertexMap {
                            map: map.clone(),
                            zones: TwoZone::default(),
                        },
                        ..t.clone()
                    }),
                    TextureSource::Variation(_) => NewLayer::Variation.kind(),
                    TextureSource::Process(_) => NewLayer::Process.kind(),
                    TextureSource::Occlusion(_) => NewLayer::Occlusion.kind(),
                };
                match fresh {
                    NodeKind::Texture(f) => NodeKind::Texture(TextureLayer {
                        source: f.source,
                        ..t.clone()
                    }),
                    other => other,
                }
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: NodeId,
    /// `None`: directly under Render. Otherwise a Group or Folder.
    pub parent: Option<NodeId>,
    pub name: String,
    pub layer: Layer,
    pub kind: NodeKind,
    /// Editor colour and the other menu state (shader_layer_menu.rs).
    #[serde(
        default,
        skip_serializing_if = "crate::shader_layer_menu::LayerExtras::is_empty"
    )]
    pub extras: crate::shader_layer_menu::LayerExtras,
}
impl Node {
    /// Groups and Folders hold other layers. Materials, Shaders and Render
    /// Outputs don't (Q-42).
    pub fn is_container(&self) -> bool {
        matches!(self.kind, NodeKind::Group(_) | NodeKind::Folder)
            && self.extras.reference.is_none()
    }
    /// Whether `child` may sit inside this layer: anything in a Group or
    /// Folder; only textures (its layer masks, which may have masks of
    /// their own) inside a texture (§5.4, v0.1). Dropping a texture onto a
    /// texture to make a mask comes with ST-3.
    pub fn can_hold(&self, child: &Node) -> bool {
        self.is_container()
            || (matches!(self.kind, NodeKind::Texture(_))
                && matches!(child.kind, NodeKind::Texture(_)))
    }
    /// The type's name, for tooltips and the Properties header.
    pub fn type_label(&self) -> &'static str {
        match &self.kind {
            NodeKind::RenderOutput(_) => "Render Output",
            NodeKind::Shader(_) => "Shader",
            NodeKind::Material(_) => "Material",
            NodeKind::Group(_) => "Material Group",
            NodeKind::Folder if self.extras.reference.is_some() => "Reference",
            NodeKind::Folder => "Folder",
            NodeKind::Texture(t) => t.source.label(),
            NodeKind::NodeMaterial(_) => "Node Material",
        }
    }
    /// The panel's B column: blank for Normal (Q-30).
    pub fn blend_label(&self) -> &'static str {
        match self.layer.blend {
            BlendMode::Normal => "",
            mode => mode.label(),
        }
    }
    /// Every channel the Properties panel edits besides the Layer block.
    pub fn channels(&mut self) -> Vec<(&'static str, Channel<'_>)> {
        match &mut self.kind {
            NodeKind::Material(m) => m.channels(),
            NodeKind::Shader(s) => s.channels(),
            NodeKind::Group(g) => vec![
                (
                    "Apply to Sub-Group",
                    Channel::Bool(&mut g.apply_to_sub_group),
                ),
                (
                    "Apply to Item Instances",
                    Channel::Bool(&mut g.apply_to_instances),
                ),
                ("UV Horizontal and Vertical", Channel::Bool(&mut g.scale.uv)),
                ("Size", Channel::Bool(&mut g.scale.size)),
                ("Bump Amplitude", Channel::Bool(&mut g.scale.bump_amplitude)),
                (
                    "Absorption Distance",
                    Channel::Bool(&mut g.scale.absorption_distance),
                ),
                (
                    "Displacement Distance",
                    Channel::Bool(&mut g.scale.displacement_distance),
                ),
                (
                    "Subsurface Distance",
                    Channel::Bool(&mut g.scale.subsurface_distance),
                ),
                ("Scale", Channel::PercentIn(&mut g.scale.scale, 1e-4, 1e4)),
            ],
            NodeKind::Texture(t) => match &mut t.source {
                TextureSource::Constant { color, value } => vec![
                    ("Color", Channel::Color(color)),
                    ("Value", Channel::Number(value, -1e6, 1e6)),
                ],
                TextureSource::ImageMap { settings, .. } => settings.channels(),
                TextureSource::Procedural(p) => p.channels(),
                // Curves are checked by `Gradient::valid` and edited in the
                // panel's strips.
                TextureSource::Gradient(_) | TextureSource::Variation(_) => vec![],
                TextureSource::VertexMap { zones, .. } => zones.channels(),
                TextureSource::Process(p) => p.channels(),
                TextureSource::Occlusion(o) => o.channels(),
            },
            // Exposed inputs are edited through the graph (node_material.rs).
            NodeKind::RenderOutput(_) | NodeKind::Folder | NodeKind::NodeMaterial(_) => vec![],
        }
    }
    pub(crate) fn validate(&self) -> crate::Result<()> {
        if self.name.trim().is_empty()
            || self.name.len() > 1024
            || self.name.chars().any(char::is_control)
        {
            return Err("Invalid shader tree layer name".into());
        }
        let o = self.layer.opacity;
        if !o.is_finite() || !(0.0..=1.0).contains(&o) {
            return Err(format!(
                "{}: Opacity must be between 0% and 100%",
                self.name
            ));
        }
        match &self.kind {
            NodeKind::Texture(TextureLayer {
                source: TextureSource::Procedural(p),
                ..
            }) => p.validate().map_err(|e| format!("{}: {e}", self.name))?,
            NodeKind::Group(g) if matches!(&g.tag, TagMask::Name(t) if t.trim().is_empty()) => {
                return Err(format!("{}: empty polygon tag", self.name));
            }
            NodeKind::Texture(TextureLayer {
                source: TextureSource::Gradient(g),
                ..
            }) if !g.valid() => {
                return Err(format!("{}: invalid gradient keys", self.name));
            }
            NodeKind::Texture(TextureLayer {
                source: TextureSource::Variation(v),
                ..
            }) if !v.valid() => {
                return Err(format!("{}: invalid variation keys", self.name));
            }
            NodeKind::Texture(TextureLayer {
                source: TextureSource::Occlusion(o),
                ..
            }) if !o.valid() => {
                return Err(format!("{}: Occlusion Rays must be 1 to 1024", self.name));
            }
            _ => {}
        }
        let mut copy = self.clone();
        if let Some((name, _)) = copy.channels().into_iter().find(|(_, c)| !c.valid()) {
            return Err(format!("{}: {name} is out of range", self.name));
        }
        Ok(())
    }
}

/// Where dropped rows land: a parent (`None` = Render) and the position
/// among that parent's children before the move, 0 = top (so "below row
/// k" is k + 1 whether or not the moved rows were above it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drop {
    pub parent: Option<NodeId>,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderTree {
    pub nodes: Vec<Node>,
    pub next_id: NodeId,
    /// Render Output settings by layer id (render_outputs.rs; native v22).
    /// Layers without an entry use the starting values.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub outputs: std::collections::BTreeMap<NodeId, crate::render_outputs::OutputSettings>,
    /// The Nodes section (NM-03, NM-04): node graphs that Node Material
    /// layers use. Native v38.
    #[serde(
        default,
        skip_serializing_if = "crate::node_graph::NodeSection::is_empty"
    )]
    pub node_graphs: crate::node_graph::NodeSection,
    /// Material Groups' node views: free nodes, wires, box places
    /// (node_view.rs, NE-09/10; native v55).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub node_views: std::collections::BTreeMap<NodeId, crate::node_view::GroupNodes>,
}

impl Default for ShaderTree {
    /// The default new-scene tree (§3), top to bottom under Render.
    fn default() -> Self {
        let node = |id, name: &str, kind| Node {
            id,
            parent: None,
            name: name.into(),
            layer: Layer::default(),
            kind,
            extras: Default::default(),
        };
        Self {
            nodes: vec![
                node(
                    1,
                    "Alpha Output",
                    NodeKind::RenderOutput(OutputEffect::Alpha),
                ),
                node(
                    2,
                    "Final Color Output",
                    NodeKind::RenderOutput(OutputEffect::FinalColor),
                ),
                node(3, "Base Shader", NodeKind::Shader(Default::default())),
                node(4, "Base Material", NodeKind::Material(Box::default())),
            ],
            next_id: 5,
            outputs: Default::default(),
            node_graphs: Default::default(),
            node_views: Default::default(),
        }
    }
}

impl ShaderTree {
    /// The default layers. `next_id` doesn't count: a tree that only had
    /// layers added and deleted again saves like a new one. A Material
    /// with the pre-v41 OpenPBR values counts as untouched too: an older
    /// file's default tree stays the default tree (they render the same
    /// there: no coat or subsurface).
    pub fn is_default(&self) -> bool {
        if !self.outputs.is_empty() || !self.node_graphs.is_empty() || !self.node_views.is_empty() {
            return false;
        }
        let default = Self::default().nodes;
        self.nodes.len() == default.len()
            && self
                .nodes
                .iter()
                .zip(&default)
                .all(|(a, b)| match (&a.kind, &b.kind) {
                    (NodeKind::Material(m), NodeKind::Material(d)) if m.openpbr.is_legacy() => {
                        let mut m = (**m).clone();
                        m.openpbr = d.openpbr.clone();
                        Node {
                            kind: NodeKind::Material(Box::new(m)),
                            ..a.clone()
                        } == *b
                    }
                    _ => a == b,
                })
    }
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub(crate) fn index(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }
    /// A parent's children, top row first.
    pub fn children(&self, parent: Option<NodeId>) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(move |n| n.parent == parent)
    }
    pub fn has_children(&self, id: NodeId) -> bool {
        self.nodes.iter().any(|n| n.parent == Some(id))
    }
    /// The scene image (`Scene::images` id) layer `id` shows, if it is a
    /// layer with an image (an Image Map with one chosen).
    pub fn layer_image(&self, id: NodeId) -> Option<u64> {
        match &self.get(id)?.kind {
            NodeKind::Texture(TextureLayer {
                source: TextureSource::ImageMap { image, .. },
                ..
            }) => *image,
            _ => None,
        }
    }
    /// True when `id` is `ancestor` or lies inside it.
    pub fn is_within(&self, id: NodeId, ancestor: NodeId) -> bool {
        let mut at = Some(id);
        // The depth bound guards a corrupt parent loop.
        for _ in 0..=self.nodes.len() {
            match at {
                Some(a) if a == ancestor => return true,
                Some(a) => at = self.get(a).and_then(|n| n.parent),
                None => return false,
            }
        }
        false
    }
    /// The rows in panel order with their depth (0 = directly under
    /// Render), skipping the children of collapsed rows.
    pub fn rows(&self, collapsed: &dyn Fn(NodeId) -> bool) -> Vec<(&Node, usize)> {
        let mut out = Vec::new();
        let mut stack: Vec<(&Node, usize)> = self.children(None).map(|n| (n, 0)).collect();
        stack.reverse();
        while let Some((node, depth)) = stack.pop() {
            out.push((node, depth));
            if !collapsed(node.id) {
                let mut kids: Vec<_> = self
                    .children(Some(node.id))
                    .map(|n| (n, depth + 1))
                    .collect();
                kids.reverse();
                stack.extend(kids);
            }
        }
        out
    }
    /// The Effect column (§4.2, Q-29).
    pub fn effect_label(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::RenderOutput(e) => e.label().into(),
            NodeKind::Shader(s) => s.effect.label().into(),
            NodeKind::Texture(t) => match &t.node_input {
                Some(name) => name.to_string(),
                None => t.effect.label().into(),
            },
            NodeKind::NodeMaterial(m) => m
                .graph
                .and_then(|g| self.node_graphs.get(g))
                .map_or_else(|| "(no graph)".into(), |g| g.material.name.clone()),
            NodeKind::Folder => String::new(),
            NodeKind::Group(g) => g.tag.label().into(),
            // A material shows the mask of the group it ends up under.
            NodeKind::Material(_) => {
                let mut at = node.parent;
                while let Some(id) = at {
                    let Some(parent) = self.get(id) else { break };
                    if let NodeKind::Group(g) = &parent.kind
                        && g.tag != TagMask::All
                    {
                        return g.tag.label().into();
                    }
                    at = parent.parent;
                }
                "(all)".into()
            }
        }
    }
    pub fn validate(&self) -> crate::Result<()> {
        check_node_count(self.nodes.len())?;
        if self.next_id == 0 || self.next_id == u64::MAX {
            return Err("Shader tree is too large".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for n in &self.nodes {
            if n.id == 0 || n.id >= self.next_id || !ids.insert(n.id) {
                return Err("Invalid shader tree layer identity".into());
            }
        }
        for n in &self.nodes {
            if let Some(p) = n.parent {
                if !self.get(p).is_some_and(|parent| parent.can_hold(n)) {
                    return Err(format!("{}: that layer can't hold it", n.name));
                }
                if self.is_within(p, n.id) || !self.reaches_root(n.id) {
                    return Err("Shader tree parents form a loop".into());
                }
            }
            n.validate()?;
        }
        self.validate_library()?;
        for n in &self.nodes {
            if let Some(link) = &n.extras.instance {
                let ok = self
                    .get(link.source)
                    .is_some_and(|s| s.extras.instance.is_none() && s.id != n.id);
                if !ok || !crate::shader_instances::can_instance(n) {
                    return Err(format!("{}: invalid instance source", n.name));
                }
            }
        }
        for (id, settings) in &self.outputs {
            if !matches!(
                self.get(*id).map(|n| &n.kind),
                Some(NodeKind::RenderOutput(_))
            ) {
                return Err("Output settings for a missing Render Output".into());
            }
            if !settings.clone().valid() {
                return Err("A Render Output setting is out of range".into());
            }
        }
        self.validate_node_materials()?;
        self.validate_node_views()?;
        Ok(())
    }
    fn reaches_root(&self, id: NodeId) -> bool {
        let mut at = Some(id);
        for _ in 0..=self.nodes.len() {
            match at {
                None => return true,
                Some(a) => at = self.get(a).and_then(|n| n.parent),
            }
        }
        false
    }
    /// A name not used by another layer: `base`, else `base (2)`… (in the
    /// user's Item Index Style)
    pub(crate) fn unique_name(&self, base: &str) -> String {
        crate::user_defaults::free_name(base, |name| self.nodes.iter().any(|n| n.name == name))
    }
    /// Where Add Layer puts a new layer (§5.1, Q-2).
    pub(crate) fn add_target(&self, layer: NewLayer, selected: Option<NodeId>) -> Drop {
        // Render outputs go to the very top so they see the whole tree; so
        // does a Shader with nothing selected (G-20, above the outputs).
        if matches!(layer, NewLayer::RenderOutput(_))
            || (layer == NewLayer::Shader && selected.is_none())
        {
            return Drop {
                parent: None,
                index: 0,
            };
        }
        // The selected Group/Folder, else the nearest one around the
        // selected layer (past any texture a mask sits in).
        let mut parent = selected;
        while let Some(n) = parent.and_then(|id| self.get(id)) {
            if n.is_container() {
                break;
            }
            parent = n.parent;
        }
        // A selected id the tree doesn't have (a closed scene's selection)
        // is no parent: the new layer would take that id and parent itself.
        let parent = parent.filter(|&id| self.get(id).is_some());
        if parent.is_some() {
            return Drop { parent, index: 0 };
        }
        // Top level: directly below the Base Shader, i.e. the lowest shader
        // directly under Render; the top when there is none.
        let index = self
            .children(None)
            .enumerate()
            .filter(|(_, n)| matches!(n.kind, NodeKind::Shader(_)))
            .map(|(i, _)| i + 1)
            .last()
            .unwrap_or(0);
        Drop {
            parent: None,
            index,
        }
    }
    /// Index in `nodes` where a node lands to be child `index` of `parent`.
    pub(crate) fn slot(&self, drop: Drop) -> usize {
        let siblings: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent == drop.parent)
            .map(|(i, _)| i)
            .collect();
        match siblings.get(drop.index) {
            Some(&i) => i,
            // Below the last sibling (after its whole branch is fine: the
            // flat order only matters among siblings), else after the parent.
            None => siblings
                .last()
                .map(|&i| i + 1)
                .or_else(|| drop.parent.and_then(|p| self.index(p)).map(|i| i + 1))
                .unwrap_or(self.nodes.len()),
        }
    }
    /// Add Layer: returns the new layer's id.
    pub fn add(&mut self, layer: NewLayer, selected: Option<NodeId>) -> crate::Result<NodeId> {
        check_node_count(self.nodes.len() + 1)?;
        if self.next_id >= u64::MAX - 1 {
            return Err("Shader tree is full".into());
        }
        let drop = self.add_target(layer, selected);
        let id = self.next_id;
        let node = Node {
            id,
            parent: drop.parent,
            name: self.unique_name(layer.label()),
            layer: Layer::default(),
            kind: layer.kind(),
            extras: Default::default(),
        };
        let at = self.slot(drop);
        self.nodes.insert(at, node);
        self.next_id += 1;
        Ok(id)
    }
    /// Delete layers and everything inside them.
    pub fn remove(&mut self, ids: &[NodeId]) -> crate::Result<()> {
        if ids.is_empty() || ids.iter().any(|&id| self.get(id).is_none()) {
            return Err("Select shader tree layers to delete".into());
        }
        let gone: Vec<NodeId> = self
            .nodes
            .iter()
            .filter(|n| ids.iter().any(|&id| self.is_within(n.id, id)))
            .map(|n| n.id)
            .collect();
        self.nodes.retain(|n| !gone.contains(&n.id));
        self.outputs.retain(|id, _| !gone.contains(id));
        self.prune_node_views();
        Ok(())
    }
    /// Drag and drop: move layers (with their contents) to `drop`, keeping
    /// their relative order. Rows inside another moved row move with it.
    pub fn move_to(&mut self, ids: &[NodeId], drop: Drop) -> crate::Result<()> {
        if ids.is_empty() || ids.iter().any(|&id| self.get(id).is_none()) {
            return Err("Nothing to move".into());
        }
        if let Some(p) = drop.parent {
            let parent = self.get(p).ok_or("Drop target no longer exists")?;
            if let Some(id) = ids
                .iter()
                .find(|&&id| !parent.can_hold(self.get(id).expect("checked")))
            {
                let name = &self.get(*id).expect("checked").name;
                return Err(format!("{} can't hold {name}", parent.name));
            }
            if ids.iter().any(|&id| self.is_within(p, id)) {
                return Err("Can't move a group into itself".into());
            }
        }
        // Top-most moved rows only, in panel order.
        let order: Vec<NodeId> = self
            .rows(&|_| false)
            .into_iter()
            .map(|(n, _)| n.id)
            .filter(|&id| ids.contains(&id))
            .collect();
        let roots: Vec<NodeId> = order
            .iter()
            .copied()
            .filter(|&id| {
                !ids.iter()
                    .any(|&other| other != id && self.is_within(id, other))
            })
            .collect();
        // `drop.index` counts the parent's children as they are now; rows
        // that move out from above that spot don't count any more.
        let above = self
            .children(drop.parent)
            .take(drop.index)
            .filter(|n| roots.contains(&n.id))
            .count();
        let index = drop.index - above;
        let mut moved = Vec::new();
        for &id in &roots {
            let i = self.index(id).expect("checked above");
            let mut node = self.nodes.remove(i);
            node.parent = drop.parent;
            moved.push(node);
        }
        let count = self.children(drop.parent).count();
        let at = self.slot(Drop {
            parent: drop.parent,
            index: index.min(count),
        });
        self.nodes.splice(at..at, moved);
        Ok(())
    }
    /// Replace one layer's name, Layer block and channels. Its id, parent
    /// and type stay.
    pub fn replace(&mut self, node: Node) -> crate::Result<()> {
        let i = self
            .index(node.id)
            .ok_or("Shader tree layer no longer exists")?;
        let old = &self.nodes[i];
        if old.parent != node.parent
            || std::mem::discriminant(&old.kind) != std::mem::discriminant(&node.kind)
        {
            return Err("A layer edit can't change its type or parent".into());
        }
        node.validate()?;
        let mut node = node;
        node.name = node.name.trim().to_owned();
        self.nodes[i] = node;
        Ok(())
    }
    /// The Material a material tag shades with: the lowest Material
    /// directly under Render for `Default`, else the first Material inside
    /// the first Material Group masked by that tag.
    pub fn material_for_tag(&self, tag: &str) -> Option<NodeId> {
        if crate::material::is_default_name(tag) {
            return self
                .children(None)
                .filter(|n| matches!(n.kind, NodeKind::Material(_)))
                .last()
                .map(|n| n.id);
        }
        let group = self.nodes.iter().find(|n| {
            matches!(&n.kind, NodeKind::Group(g)
                if g.tag_type == TagType::Material && g.tag == TagMask::Name(tag.to_owned()))
                && !self.in_library(n.id)
        })?;
        self.children(Some(group.id))
            .find(|n| matches!(n.kind, NodeKind::Material(_)))
            .map(|n| n.id)
    }
    /// Make sure a material tag has its Material Group and Material,
    /// creating them from `material` when missing (provisional naming: the
    /// group is named after the tag, its material "<tag> Material").
    /// Returns the Material's id.
    pub fn ensure_material(
        &mut self,
        tag: &str,
        material: &crate::Material,
    ) -> crate::Result<NodeId> {
        if let Some(id) = self.material_for_tag(tag) {
            return Ok(id);
        }
        let (group, existing) = match self.nodes.iter().find(|n| {
            matches!(&n.kind, NodeKind::Group(g)
                if g.tag_type == TagType::Material && g.tag == TagMask::Name(tag.to_owned()))
                && !self.in_library(n.id)
        }) {
            Some(g) => (g.id, true),
            None if crate::material::is_default_name(tag) => {
                // No Base Material left: put one back under Render.
                (self.add(NewLayer::Material, None)?, false)
            }
            None => (self.add(NewLayer::Group, None)?, false),
        };
        if crate::material::is_default_name(tag) {
            let i = self.index(group).expect("just added");
            self.nodes[i].name = self.unique_name("Base Material");
            if let NodeKind::Material(m) = &mut self.nodes[i].kind {
                m.copy_from(material);
            }
            return Ok(group);
        }
        if !existing {
            let i = self.index(group).expect("just added");
            let name = self.unique_name(tag);
            self.nodes[i].name = name;
            self.nodes[i].kind = NodeKind::Group(GroupMask {
                tag: TagMask::Name(tag.to_owned()),
                ..Default::default()
            });
        }
        let id = self.add(NewLayer::Material, Some(group))?;
        let i = self.index(id).expect("just added");
        self.nodes[i].name = self.unique_name(&format!("{tag} Material"));
        if let NodeKind::Material(m) = &mut self.nodes[i].kind {
            m.copy_from(material);
        }
        Ok(id)
    }
    /// Auto Create Item Masks: a Group masked to mesh item `item`, named
    /// after it, at the top of the tree.
    pub(crate) fn add_item_mask(&mut self, item: u64, name: &str) -> crate::Result<NodeId> {
        let id = self.add(NewLayer::Group, None)?;
        let i = self.index(id).expect("just added");
        self.nodes[i].name = self.unique_name(name);
        self.nodes[i].kind = NodeKind::Group(GroupMask {
            item: ItemMask::Item(item),
            ..Default::default()
        });
        Ok(id)
    }
    /// Copy a material table entry into its tree Material (Set Material).
    pub fn mirror_material(&mut self, tag: &str, material: &crate::Material) -> crate::Result<()> {
        let id = self.ensure_material(tag, material)?;
        let i = self.index(id).expect("ensured");
        if let NodeKind::Material(m) = &mut self.nodes[i].kind {
            m.copy_from(material);
        }
        Ok(())
    }
}

impl ShaderTree {
    /// Older files (before native version 15) stored Specular Amount as the
    /// reflectance itself; turn every stored value into the amount that
    /// renders the same: Materials, and Constant layers driving Specular
    /// Amount. Image-driven Specular Amount can't be converted and reads
    /// on the new curve (ST-Q05).
    pub(crate) fn convert_specular_from_reflectance(&mut self) {
        for node in &mut self.nodes {
            match &mut node.kind {
                NodeKind::Material(m) => {
                    m.specular_amount =
                        crate::material::specular_amount_for_reflectance(m.specular_amount);
                }
                NodeKind::Texture(TextureLayer {
                    source: TextureSource::Constant { value, .. },
                    effect: TextureEffect::SpecularAmount,
                    ..
                }) => {
                    *value = crate::material::specular_amount_for_reflectance(*value);
                }
                _ => {}
            }
        }
    }
}

impl crate::Scene {
    /// See [`ShaderTree::convert_specular_from_reflectance`]; the material
    /// table too.
    pub(crate) fn convert_specular_from_reflectance(&mut self) {
        for m in self.materials.values_mut() {
            m.specular = crate::material::specular_amount_for_reflectance(m.specular);
        }
        // A default tree wasn't in the file (it holds today's values).
        if !self.shader_tree.is_default() {
            self.shader_tree.convert_specular_from_reflectance();
        }
    }
    /// Map the scene's materials into a tree that was never saved (older
    /// files, imports): every material tag gets its group, and a stored
    /// `Default` fills the Base Material. Trees that were saved stay as
    /// they are.
    pub fn map_materials_into_default_tree(&mut self) -> crate::Result<()> {
        if let Some(tree) = self.mapped_default_tree()? {
            self.shader_tree = tree;
        }
        Ok(())
    }
    /// The default tree with the scene's materials mapped in, or `None`
    /// when the tree was edited or saved (it is then used as it is).
    pub(crate) fn mapped_default_tree(&self) -> crate::Result<Option<ShaderTree>> {
        self.map_names_into_default_tree(self.material_names())
    }
    /// `mapped_default_tree` with only the stored materials mapped in: no
    /// pass over every polygon's tag, so the viewport can afford it. A tag
    /// with no stored material gets default values there, which the
    /// viewport shows as the plain fill without needing its group.
    pub fn stored_materials_tree(&self) -> crate::Result<Option<ShaderTree>> {
        let names = std::iter::once(crate::material::DEFAULT_MATERIAL.to_owned())
            .chain(
                self.materials
                    .keys()
                    .filter(|n| !crate::material::is_default_name(n))
                    .cloned(),
            )
            .collect();
        self.map_names_into_default_tree(names)
    }
    fn map_names_into_default_tree(&self, names: Vec<String>) -> crate::Result<Option<ShaderTree>> {
        if !self.shader_tree.is_default() {
            return Ok(None);
        }
        let mut tree = self.shader_tree.clone();
        for name in names {
            let material = self.material_or_default(&name);
            if crate::material::is_default_name(&name) {
                if self.material(&name).is_some() {
                    tree.mirror_material(&name, &material)?;
                }
            } else {
                tree.ensure_material(&name, &material)?;
            }
        }
        Ok(Some(tree))
    }
}

/// Edits the Shading panel makes; each is one undo step.
#[derive(Clone, Debug, PartialEq)]
pub enum ShaderTreeEdit {
    Add {
        layer: NewLayer,
        /// The selected row Add Layer places relative to.
        selected: Option<NodeId>,
    },
    Remove(Vec<NodeId>),
    Move {
        ids: Vec<NodeId>,
        drop: Drop,
    },
    /// Eye column; several rows when a click-drag sweeps across them.
    SetEnable {
        ids: Vec<NodeId>,
        enable: bool,
    },
    /// Editor Color of several layers, one undo step (note 10, IC-B3).
    SetEditorColor {
        ids: Vec<NodeId>,
        color: Option<crate::EditorColor>,
    },
    /// Any other single-layer edit (rename, blend, opacity, effect,
    /// channels, Locator setting). `continuing` folds it into the previous
    /// step of the same layer, so one opacity drag is one undo step.
    Replace {
        node: Node,
        continuing: bool,
    },
    /// MN-07.1: a picture dropped on a surface. A new Image Map layer
    /// showing it goes to the top of the material group masking `tag` (the
    /// polygon's material), or to the tree's top level when no group masks
    /// it. One step.
    AddImageForTag {
        tag: String,
        image: (String, u32, u32),
    },
    /// An Image Map layer's Image: a picture file (added to the scene's
    /// images if new) or `None` to clear it.
    SetImage {
        node: NodeId,
        image: Option<(String, u32, u32)>,
    },
    /// The Texture Locator sub-tab: a locator item's settings and transform
    /// (Position, Rotation and order, Size = scale). `continuing` folds a
    /// drag into one step.
    SetLocator {
        locator: u64,
        settings: Box<crate::texture_locator::TextureLocator>,
        transform: crate::scene::Transform,
        continuing: bool,
    },
    /// Auto Size: the locator of texture layer `layer` moves to the centre
    /// of the surfaces the layer applies to and takes their size.
    AutoSize {
        layer: NodeId,
    },
    /// Apply Scale (group extras §1, XQ-3): bakes Material Group `group`'s
    /// Scale into the materials and texture locators inside it and sets
    /// the Scale back to 100 %.
    ApplyScale {
        group: NodeId,
    },
    /// A Render Output's settings (render outputs v1 §2). `continuing`
    /// folds a drag into one step.
    SetOutput {
        node: NodeId,
        settings: Box<crate::render_outputs::OutputSettings>,
        continuing: bool,
    },
    /// Duplicate (layer menu §2): an independent copy directly above each
    /// top-most layer.
    Duplicate(Vec<NodeId>),
    /// Paste (layer menu §5): the clipboard's layers directly above
    /// `selected`.
    Paste {
        clip: Box<crate::shader_layer_menu::ShaderClipboard>,
        selected: Option<NodeId>,
    },
    /// Paste Values (layer menu §5): `from`'s settings onto layers of the
    /// same type.
    PasteValues {
        from: Box<Node>,
        targets: Vec<NodeId>,
    },
    /// Select Polygons (layer menu §7): the polygons the Material Group
    /// around the layer matches, in Polygon mode.
    SelectPolygons(NodeId),
    /// Change Type (layer menu §6): texture layer `node` becomes a layer
    /// of type `to` in place.
    ChangeType {
        node: NodeId,
        to: NewLayer,
    },
    /// Create Instance (layer menu §3): a linked clone directly above each
    /// top-most layer.
    CreateInstance(Vec<NodeId>),
    /// Drag onto the Library heading (layer menu §4): Material Groups
    /// become Library items.
    ToLibrary(Vec<NodeId>),
    /// Drag a Library item into the tree: a reference at `drop`.
    AddReference {
        library: NodeId,
        drop: Drop,
    },
    /// References ▸ De-Reference: real, unlinked copies in their place.
    DeReference(Vec<NodeId>),
    /// Delete Library items after turning their references into copies.
    DeReferenceAndRemove(Vec<NodeId>),
    /// Lock Preset / Unlock Preset on a Library item.
    SetLocked {
        node: NodeId,
        locked: bool,
    },
    /// Revert to Source: drop instance `node`'s overrides at `paths` (JSON
    /// pointers, shader_instances.rs), or all of them for `None`.
    RevertToSource {
        node: NodeId,
        paths: Option<Vec<String>>,
    },
    /// Import Node Material (NM-05): graphs read by
    /// `node_graph::import` join the Nodes section, each with a Node
    /// Material layer placed as Add Layer places one. `images` are the
    /// graphs' image files with their sizes, for Clips (NM-21).
    ImportNodeGraphs {
        graphs: Vec<crate::node_graph::StoredGraph>,
        images: Vec<(String, u32, u32)>,
        selected: Option<NodeId>,
    },
    /// A shared graph's exposed default (NM-04): every layer using the
    /// graph that doesn't override it follows. `continuing` folds a drag.
    SetNodeGraphDefault {
        graph: u64,
        input: String,
        value: Vec<f64>,
        continuing: bool,
    },
    /// A node-view change in Material Group `group` (node_view.rs,
    /// NE-02..NE-16): one undo step, named after it (NE-18).
    Nodes {
        group: NodeId,
        edit: crate::node_view::NodeEdit,
    },
}

impl crate::Editor {
    pub(crate) fn edit_shader_tree(&mut self, edit: ShaderTreeEdit) -> crate::Result<()> {
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before editing the shader tree".into());
        }
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before editing the shader tree".into());
        }
        self.require_idle()?;
        let mut candidate = self.scene.clone();
        // An older file's materials join the tree on its first edit, so the
        // tree being edited is the one its renders already show.
        candidate.shader_tree = self.scene.render_shader_tree().into_owned();
        let tree_before = candidate.shader_tree.clone();
        let mut unlocking = false;
        let mut next = self.scene.next_id.max(self.next_item_id);
        let mut fold = None;
        let label = match edit {
            ShaderTreeEdit::Add { layer, selected } => {
                let id = candidate.shader_tree.add(layer, selected)?;
                // Every texture layer gets its own locator (spec §1).
                let node = candidate.shader_tree.get(id).expect("just added").clone();
                if let NodeKind::Texture(t) = &node.kind {
                    let settings = crate::Scene::starting_locator(&t.source);
                    let locator = candidate.add_texture_locator(&mut next, &node.name, settings)?;
                    let i = candidate.shader_tree.index(id).expect("just added");
                    if let NodeKind::Texture(t) = &mut candidate.shader_tree.nodes[i].kind {
                        t.locator = Some(locator);
                    }
                }
                format!("Add {}", layer.label())
            }
            ShaderTreeEdit::Remove(ids) => {
                // A source takes its instances with it, a Library item its
                // references (layer menu §3, §4).
                let ids = candidate.shader_tree.with_references(&ids);
                let ids = candidate.shader_tree.with_instances(&ids);
                let before: Vec<u64> = candidate.shader_tree.locators_in(&ids);
                candidate.shader_tree.remove(&ids)?;
                candidate.drop_unused_locators(&before);
                "Delete Shader Layers".into()
            }
            ShaderTreeEdit::Move { ids, drop } => {
                if ids.iter().any(|&id| {
                    candidate
                        .shader_tree
                        .get(id)
                        .is_some_and(|n| n.extras.library)
                }) {
                    return Err(
                        "Drag a Library item into the tree to drop a reference to it".into(),
                    );
                }
                candidate.shader_tree.move_to(&ids, drop)?;
                "Move Shader Layers".into()
            }
            ShaderTreeEdit::SetEnable { ids, enable } => {
                for id in ids {
                    let tree = &mut candidate.shader_tree;
                    let mut node = tree.get(id).ok_or("Missing shader tree layer")?.clone();
                    node.layer.enable = enable;
                    tree.replace(node)?;
                }
                "Shader Layer Visibility".into()
            }
            ShaderTreeEdit::SetEditorColor { ids, color } => {
                for id in ids {
                    let tree = &mut candidate.shader_tree;
                    let mut node = tree.get(id).ok_or("Missing shader tree layer")?.clone();
                    node.layer.editor_color = color;
                    tree.replace(node)?;
                }
                "Editor Color".into()
            }
            ShaderTreeEdit::Replace { node, continuing } => {
                let id = node.id;
                // The Locator setting may only point at a Texture Locator.
                let old = candidate
                    .shader_tree
                    .get(id)
                    .and_then(|n| match &n.kind {
                        NodeKind::Texture(t) => Some(t.locator),
                        _ => None,
                    })
                    .flatten();
                let new = match &node.kind {
                    NodeKind::Texture(t) => t.locator,
                    _ => None,
                };
                if new != old
                    && let Some(l) = new
                    && candidate.texture_locator(l).is_none()
                {
                    return Err("The Locator must be a Texture Locator item".into());
                }
                candidate.shader_tree.replace(node)?;
                // Pointing a layer at another locator leaves the old one
                // unused: it goes, as when the layer is deleted.
                if new != old
                    && let Some(old) = old
                {
                    candidate.drop_unused_locators(&[old]);
                }
                let label = format!(
                    "Edit {}",
                    candidate
                        .shader_tree
                        .get(id)
                        .map_or("Shader Layer", |n| n.name.as_str())
                );
                if continuing {
                    fold = Some(id);
                }
                label
            }
            ShaderTreeEdit::AddImageForTag { tag, image } => {
                let mask = TagMask::Name(tag);
                let group = candidate
                    .shader_tree
                    .nodes
                    .iter()
                    .find(|n| {
                        n.parent.is_none()
                            && matches!(&n.kind, NodeKind::Group(g)
                                if g.tag_type == TagType::Material && g.tag == mask)
                    })
                    .map(|n| n.id);
                let id = candidate.shader_tree.add(NewLayer::ImageMap, group)?;
                let mut node = candidate.shader_tree.get(id).expect("just added").clone();
                if let NodeKind::Texture(t) = &mut node.kind {
                    let settings = crate::Scene::starting_locator(&t.source);
                    t.locator =
                        Some(candidate.add_texture_locator(&mut next, &node.name, settings)?);
                    if let TextureSource::ImageMap { image: slot, .. } = &mut t.source {
                        let (path, width, height) = image;
                        *slot = Some(candidate.image_for_path(&path, width, height)?);
                    }
                }
                candidate.shader_tree.replace(node)?;
                "Add Image Map".into()
            }
            ShaderTreeEdit::SetImage { node, image } => {
                let tree = &mut candidate.shader_tree;
                let mut layer = tree.get(node).ok_or("Missing shader tree layer")?.clone();
                let NodeKind::Texture(TextureLayer {
                    source: TextureSource::ImageMap { image: slot, .. },
                    ..
                }) = &mut layer.kind
                else {
                    return Err("Only an Image Map layer has an Image".into());
                };
                *slot = match image {
                    Some((path, width, height)) => {
                        Some(candidate.image_for_path(&path, width, height)?)
                    }
                    None => None,
                };
                candidate.shader_tree.replace(layer)?;
                "Shader Image".into()
            }
            ShaderTreeEdit::SetLocator {
                locator,
                settings,
                transform,
                continuing,
            } => {
                settings.validate()?;
                let item = candidate
                    .items
                    .iter_mut()
                    .find(|i| i.id == locator && i.texture_locator().is_some())
                    .ok_or("Missing Texture Locator")?;
                item.texture = Some(crate::texture_locator::TextureItem::Locator(settings));
                item.transform = transform;
                if continuing {
                    fold = Some(locator);
                }
                format!("Edit {}", item.name)
            }
            ShaderTreeEdit::SetOutput {
                node,
                settings,
                continuing,
            } => {
                let name = match candidate.shader_tree.get(node) {
                    Some(n) if matches!(n.kind, NodeKind::RenderOutput(_)) => n.name.clone(),
                    _ => return Err("Select a Render Output".into()),
                };
                if settings.is_default() {
                    candidate.shader_tree.outputs.remove(&node);
                } else {
                    candidate.shader_tree.outputs.insert(node, *settings);
                }
                if continuing {
                    fold = Some(node);
                }
                format!("Edit {name}")
            }
            ShaderTreeEdit::Duplicate(ids) => {
                candidate.duplicate_shader_layers(&mut next, &ids)?;
                "Duplicate Shader Layers".into()
            }
            ShaderTreeEdit::Paste { clip, selected } => {
                candidate.paste_shader_layers(&mut next, &clip, selected)?;
                "Paste Shader Layers".into()
            }
            ShaderTreeEdit::PasteValues { from, targets } => {
                candidate.shader_tree.paste_values(&from, &targets)?;
                "Paste Values".into()
            }
            ShaderTreeEdit::ToLibrary(ids) => {
                candidate.shader_tree.move_to_library(&ids)?;
                "Add to Library".into()
            }
            ShaderTreeEdit::AddReference { library, drop } => {
                candidate.shader_tree.add_reference(library, drop)?;
                "Add Reference".into()
            }
            ShaderTreeEdit::DeReference(ids) => {
                candidate.dereference(&mut next, &ids)?;
                "De-Reference".into()
            }
            ShaderTreeEdit::DeReferenceAndRemove(ids) => {
                let refs = candidate.shader_tree.references_deleted_with(&ids);
                if !refs.is_empty() {
                    candidate.dereference(&mut next, &refs)?;
                }
                let ids = candidate.shader_tree.with_instances(&ids);
                let before: Vec<u64> = candidate.shader_tree.locators_in(&ids);
                candidate.shader_tree.remove(&ids)?;
                candidate.drop_unused_locators(&before);
                "Delete Shader Layers".into()
            }
            ShaderTreeEdit::SetLocked { node, locked } => {
                unlocking = true;
                candidate.shader_tree.set_locked(node, locked)?;
                if locked {
                    "Lock Preset"
                } else {
                    "Unlock Preset"
                }
                .into()
            }
            ShaderTreeEdit::CreateInstance(ids) => {
                candidate.shader_tree.create_instances(&ids)?;
                "Create Instance".into()
            }
            ShaderTreeEdit::RevertToSource { node, paths } => {
                candidate
                    .shader_tree
                    .revert_overrides(node, paths.as_deref())?;
                "Revert to Source".into()
            }
            ShaderTreeEdit::ChangeType { node, to } => {
                candidate.shader_tree.change_type(node, to)?;
                format!("Change Type to {}", to.label())
            }
            ShaderTreeEdit::SelectPolygons(id) => {
                candidate.select_shader_polygons(id)?;
                "Select Polygons".into()
            }
            ShaderTreeEdit::ApplyScale { group } => {
                let Some(NodeKind::Group(g)) =
                    candidate.shader_tree.get(group).map(|n| n.kind.clone())
                else {
                    return Err("Select a Material Group".into());
                };
                let sc = g.scale.clone();
                if sc.scale == 1. {
                    return Err("The group's Scale is already 100%".into());
                }
                let inside: Vec<NodeId> = candidate
                    .shader_tree
                    .nodes
                    .iter()
                    .filter(|n| n.id != group && candidate.shader_tree.is_within(n.id, group))
                    .map(|n| n.id)
                    .collect();
                let mut locators = std::collections::BTreeSet::new();
                for node in &mut candidate.shader_tree.nodes {
                    if !inside.contains(&node.id) {
                        continue;
                    }
                    match &mut node.kind {
                        NodeKind::Material(m) => {
                            let k = sc.scale;
                            if sc.bump_amplitude {
                                m.bump_amplitude *= k;
                            }
                            if sc.absorption_distance {
                                m.absorption_distance *= k;
                            }
                            if sc.displacement_distance {
                                m.displacement_distance *= k;
                            }
                            if sc.subsurface_distance {
                                m.subsurface_distance *= k;
                            }
                        }
                        NodeKind::Texture(t) if !t.ignore_scale_group => {
                            locators.extend(t.locator);
                        }
                        _ => {}
                    }
                }
                // Each locator once, even when several layers share it
                // (layers outside the group then change too; XQ-3).
                for item in &mut candidate.items {
                    if !locators.contains(&item.id) {
                        continue;
                    }
                    let Some(crate::texture_locator::TextureItem::Locator(l)) = &mut item.texture
                    else {
                        continue;
                    };
                    if l.reads_uvs() {
                        if sc.uv {
                            l.repeat = l.repeat.map(|r| r / sc.scale);
                        }
                    } else if sc.size {
                        item.transform.scale *= sc.scale;
                    }
                }
                if let Some(NodeKind::Group(g)) = candidate
                    .shader_tree
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == group)
                    .map(|n| &mut n.kind)
                {
                    g.scale.scale = 1.;
                }
                "Apply Scale".into()
            }
            ShaderTreeEdit::ImportNodeGraphs {
                graphs,
                images,
                selected,
            } => {
                for (path, w, h) in &images {
                    candidate.image_for_path(path, *w, *h)?;
                }
                let n = graphs.len();
                candidate.shader_tree.import_node_graphs(graphs, selected)?;
                if n == 1 {
                    "Import Node Material".into()
                } else {
                    "Import Node Materials".into()
                }
            }
            ShaderTreeEdit::SetNodeGraphDefault {
                graph,
                input,
                value,
                continuing,
            } => {
                candidate
                    .shader_tree
                    .set_node_graph_default(graph, &input, value)?;
                if continuing {
                    fold = Some(graph);
                }
                format!("Edit {input}")
            }
            ShaderTreeEdit::Nodes { group, edit } => {
                let applied = candidate.shader_tree.edit_nodes(group, edit)?;
                if !applied.remove_layers.is_empty() {
                    // NE-06/NE-19: deleting a layer's box is Delete Shader
                    // Layers, in the same step as the rest of the edit.
                    let ids = candidate
                        .shader_tree
                        .with_references(&applied.remove_layers);
                    let ids = candidate.shader_tree.with_instances(&ids);
                    let before: Vec<u64> = candidate.shader_tree.locators_in(&ids);
                    candidate.shader_tree.remove(&ids)?;
                    candidate.drop_unused_locators(&before);
                }
                if applied.continuing {
                    fold = Some(group);
                }
                applied.label
            }
            ShaderTreeEdit::AutoSize { layer } => {
                let locator = match candidate.shader_tree.get(layer).map(|n| &n.kind) {
                    Some(NodeKind::Texture(t)) => t.locator,
                    _ => return Err("Select a texture layer".into()),
                }
                .filter(|&l| candidate.texture_locator(l).is_some())
                .ok_or("That layer has no Texture Locator")?;
                let world = candidate
                    .texture_locator(locator)
                    .is_some_and(|l| l.world_coordinates);
                let (centre, size) = candidate
                    .auto_size_box(layer, world)
                    .ok_or("The layer applies to no polygons")?;
                // The locator's parent (the Texture Group) may be moved:
                // place it in the parent's space.
                let parent = candidate.item(locator).and_then(|i| i.parent);
                let to_parent = match parent {
                    Some(p) => candidate.world_matrix(p)?.inverse(),
                    None => glam::DMat4::IDENTITY,
                };
                let item = candidate.item_mut(locator).expect("checked");
                item.transform.translation = to_parent.transform_point3(centre);
                // A uniform Size stays uniform: all three take the largest
                // side (TL-Q19, chosen). Otherwise each axis takes its own
                // side, and a flat side keeps its size (TL-Q03).
                let scale = item.transform.scale;
                if scale.x == scale.y && scale.y == scale.z {
                    let largest = size.max_element();
                    if largest > 1e-9 {
                        item.transform.scale = glam::DVec3::splat(largest);
                    }
                } else {
                    for k in 0..3 {
                        if size[k] > 1e-9 {
                            item.transform.scale[k] = size[k];
                        }
                    }
                }
                format!("Auto Size {}", item.name)
            }
        };
        // Instances follow their sources; edits made on an instance become
        // its overrides (layer menu §3).
        candidate.shader_tree.sync_instances(Some(&tree_before))?;
        // Wires into layers that went or left their group go too (NE-13).
        candidate.shader_tree.prune_node_views();
        if !unlocking {
            candidate.shader_tree.check_locks(&tree_before)?;
        }
        candidate.shader_tree.validate()?;
        // Planar spec: Size clamps at 0.001 m (SetLocator, Apply Scale,
        // Auto Size).
        candidate.clamp_locator_sizes();
        // Items change only with locators; a full check costs a pass over
        // every mesh, so plain layer edits (opacity drags) skip it.
        if candidate.items.len() != self.scene.items.len()
            || candidate
                .items
                .iter()
                .zip(&self.scene.items)
                .any(|(a, b)| a.id != b.id || a.texture != b.texture || a.transform != b.transform)
        {
            candidate.validate()?;
        }
        if candidate == self.scene {
            return Ok(());
        }
        self.next_item_id = self.next_item_id.max(candidate.next_id);
        // A continuing edit replaces the previous step's result when that
        // step was this same layer's last edit and nothing came after it.
        if fold.is_some()
            && self.history.redo.is_empty()
            && let Some(last) = self.history.undo.last_mut().map(std::sync::Arc::make_mut)
            && last.label == label
            && last.after == self.scene
        {
            last.after = candidate.clone();
            self.scene = candidate;
            return Ok(());
        }
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record(label, before);
        Ok(())
    }
}

impl ShaderTree {
    /// The locators of the texture layers in (or inside) `ids`.
    pub fn locators_in(&self, ids: &[NodeId]) -> Vec<u64> {
        self.nodes
            .iter()
            .filter(|n| ids.iter().any(|&id| self.is_within(n.id, id)))
            .filter_map(|n| match &n.kind {
                NodeKind::Texture(t) => t.locator,
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_tree_node_count_boundary() {
        assert_eq!(MAX_SHADER_TREE_NODES, 1_000_000);
        assert!(check_node_count(MAX_SHADER_TREE_NODES - 1).is_ok());
        assert!(check_node_count(MAX_SHADER_TREE_NODES).is_ok());
        assert_eq!(
            check_node_count(MAX_SHADER_TREE_NODES + 1).unwrap_err(),
            "A Shader Tree can have at most 1000000 nodes"
        );
    }

    fn names(tree: &ShaderTree) -> Vec<(String, usize)> {
        tree.rows(&|_| false)
            .into_iter()
            .map(|(n, d)| (n.name.clone(), d))
            .collect()
    }

    #[test]
    fn default_tree_matches_the_new_scene() {
        let tree = ShaderTree::default();
        tree.validate().unwrap();
        let rows: Vec<_> = tree
            .rows(&|_| false)
            .into_iter()
            .map(|(n, _)| {
                (
                    n.name.as_str(),
                    tree.effect_label(n),
                    n.layer.opacity,
                    n.layer.enable,
                    n.blend_label(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("Alpha Output", "Alpha".to_string(), 1., true, ""),
                ("Final Color Output", "Final Color".into(), 1., true, ""),
                ("Base Shader", "Full Shading".into(), 1., true, ""),
                ("Base Material", "(all)".into(), 1., true, ""),
            ]
        );
    }

    #[test]
    fn add_layer_lands_below_the_base_shader_or_inside_the_group() {
        let mut tree = ShaderTree::default();
        let group = tree.add(NewLayer::Group, None).unwrap();
        let constant = tree.add(NewLayer::Constant, Some(group)).unwrap();
        // Q-2: a layer inside a group puts the next one at that group's top.
        let image = tree.add(NewLayer::ImageMap, Some(constant)).unwrap();
        let output = tree
            .add(NewLayer::RenderOutput(OutputEffect::Alpha), Some(group))
            .unwrap();
        assert_eq!(
            names(&tree),
            vec![
                ("Alpha Output (2)".into(), 0),
                ("Alpha Output".into(), 0),
                ("Final Color Output".into(), 0),
                ("Base Shader".into(), 0),
                ("Group".into(), 0),
                ("Image Map".into(), 1),
                ("Constant".into(), 1),
                ("Base Material".into(), 0),
            ]
        );
        assert_eq!(tree.get(output).unwrap().parent, None);
        assert_eq!(tree.get(image).unwrap().parent, Some(group));
        let NodeKind::Texture(t) = &tree.get(image).unwrap().kind else {
            panic!()
        };
        assert_eq!(t.effect, TextureEffect::DiffuseColor);
        // The editor gives it a locator; the bare tree leaves none.
        assert_eq!(t.locator, None);
        tree.validate().unwrap();
    }

    #[test]
    fn add_layer_starting_values_and_places() {
        // G-20, nothing selected.
        let mut tree = ShaderTree::default();
        let constant = tree.add(NewLayer::Constant, None).unwrap();
        let group = tree.add(NewLayer::Group, None).unwrap();
        let material = tree.add(NewLayer::Material, None).unwrap();
        let image = tree.add(NewLayer::ImageMap, None).unwrap();
        let shader = tree.add(NewLayer::Shader, None).unwrap();
        let NodeKind::Texture(t) = &tree.get(constant).unwrap().kind else {
            panic!()
        };
        assert_eq!(
            t.source,
            TextureSource::Constant {
                color: [0.; 3],
                value: 1.
            }
        );
        assert_eq!(t.effect, TextureEffect::DiffuseColor);
        let NodeKind::Texture(t) = &tree.get(image).unwrap().kind else {
            panic!()
        };
        assert_eq!(t.effect, TextureEffect::DiffuseColor);
        assert!(
            matches!(&tree.get(group).unwrap().kind, NodeKind::Group(g) if g.scope == Scope::All)
        );
        let NodeKind::Material(m) = &tree.get(material).unwrap().kind else {
            panic!()
        };
        assert_eq!((m.bump_amplitude, m.displacement_distance), (0., 0.));
        assert_eq!(
            **m,
            MaterialChannels {
                bump_amplitude: 0.,
                displacement_distance: 0.,
                ..Default::default()
            }
        );
        // The Shader goes to the very top, above both outputs; the others
        // directly below the Base Shader (newest first).
        let top: Vec<NodeId> = tree.children(None).map(|n| n.id).collect();
        assert_eq!(
            top,
            vec![shader, 1, 2, 3, image, material, group, constant, 4]
        );
        // With a group selected the Shader still lands in it (Q-2 unchanged).
        let inner = tree.add(NewLayer::Shader, Some(group)).unwrap();
        assert_eq!(tree.get(inner).unwrap().parent, Some(group));
    }

    #[test]
    fn move_reorders_parents_and_refuses_loops() {
        let mut tree = ShaderTree::default();
        let outer = tree.add(NewLayer::Group, None).unwrap();
        let inner = tree.add(NewLayer::Folder, Some(outer)).unwrap();
        // Into itself or its own child: refused, tree unchanged.
        let before = tree.clone();
        assert!(
            tree.move_to(
                &[outer],
                Drop {
                    parent: Some(inner),
                    index: 0
                }
            )
            .is_err()
        );
        assert!(
            tree.move_to(
                &[outer],
                Drop {
                    parent: Some(3),
                    index: 0
                }
            )
            .is_err()
        );
        assert_eq!(tree, before);
        // Base Material into the folder; then the outer group (with its
        // contents) to the bottom of Render.
        tree.move_to(
            &[4],
            Drop {
                parent: Some(inner),
                index: 0,
            },
        )
        .unwrap();
        tree.move_to(
            &[outer],
            Drop {
                parent: None,
                index: 99,
            },
        )
        .unwrap();
        assert_eq!(
            names(&tree),
            vec![
                ("Alpha Output".into(), 0),
                ("Final Color Output".into(), 0),
                ("Base Shader".into(), 0),
                ("Group".into(), 0),
                ("Folder".into(), 1),
                ("Base Material".into(), 2),
            ]
        );
        // Two rows keep their order; Alpha Output moves below Base Shader.
        // "Below Base Shader" is index 3 counted before the move.
        tree.move_to(
            &[2, 1],
            Drop {
                parent: None,
                index: 3,
            },
        )
        .unwrap();
        assert_eq!(
            names(&tree)[..3],
            [
                ("Base Shader".into(), 0),
                ("Alpha Output".into(), 0),
                ("Final Color Output".into(), 0),
            ]
        );
        tree.validate().unwrap();
    }

    #[test]
    fn remove_takes_contents_and_replace_keeps_type() {
        let mut tree = ShaderTree::default();
        let group = tree.add(NewLayer::Group, None).unwrap();
        tree.add(NewLayer::Material, Some(group)).unwrap();
        tree.remove(&[group]).unwrap();
        assert!(tree.is_default());
        let mut shader = tree.get(3).unwrap().clone();
        shader.layer.opacity = 1.5;
        assert!(tree.replace(shader.clone()).is_err());
        shader.layer.opacity = 0.5;
        shader.layer.blend = BlendMode::Multiply;
        tree.replace(shader.clone()).unwrap();
        assert_eq!(tree.get(3).unwrap().blend_label(), "Multiply");
        shader.kind = NodeKind::Folder;
        assert!(tree.replace(shader).is_err());
    }

    #[test]
    fn materials_map_into_groups() {
        let mut scene = crate::Scene::default();
        let red = crate::Material {
            color: [1., 0., 0.],
            ..Default::default()
        };
        scene.materials.insert("Red".into(), red.clone());
        scene.materials.insert(
            "Default".into(),
            crate::Material {
                specular: 0.5,
                ..Default::default()
            },
        );
        scene.map_materials_into_default_tree().unwrap();
        let tree = &scene.shader_tree;
        assert_eq!(
            names(tree),
            vec![
                ("Alpha Output".into(), 0),
                ("Final Color Output".into(), 0),
                ("Base Shader".into(), 0),
                ("Red".into(), 0),
                ("Red Material".into(), 1),
                ("Base Material".into(), 0),
            ]
        );
        let m = tree.get(tree.material_for_tag("Red").unwrap()).unwrap();
        assert_eq!(tree.effect_label(m), "Red");
        let NodeKind::Material(m) = &m.kind else {
            panic!()
        };
        assert_eq!(m.diffuse_color, [1., 0., 0.]);
        let NodeKind::Material(base) = &tree.get(4).unwrap().kind else {
            panic!()
        };
        assert_eq!(base.specular_amount, 0.5);
        // A saved tree is never rebuilt.
        let saved = scene.shader_tree.clone();
        scene.materials.insert("Blue".into(), red);
        scene.map_materials_into_default_tree().unwrap();
        assert_eq!(scene.shader_tree, saved);
    }
}

#[cfg(test)]
mod editor_tests {
    use super::*;
    use crate::{Command, Editor};

    fn edit(e: &mut Editor, edit: ShaderTreeEdit) {
        e.dispatch(Command::ShaderTree(edit)).unwrap();
    }

    #[test]
    fn set_image_adds_the_picture_in_one_undo_step() {
        let mut e = Editor::default();
        edit(
            &mut e,
            ShaderTreeEdit::Add {
                layer: NewLayer::ImageMap,
                selected: None,
            },
        );
        let map = e.scene.shader_tree.next_id - 1;
        let before = e.scene.clone();
        edit(
            &mut e,
            ShaderTreeEdit::SetImage {
                node: map,
                image: Some(("/pics/grid.png".into(), 8, 4)),
            },
        );
        assert_eq!(e.scene.images.len(), 1);
        let id = e.scene.images[0].id;
        assert!(matches!(
            &e.scene.shader_tree.get(map).unwrap().kind,
            NodeKind::Texture(TextureLayer { source: TextureSource::ImageMap { image: Some(i), .. }, .. }) if *i == id
        ));
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
        // Only Image Maps take an image.
        assert!(
            e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetImage {
                node: 4,
                image: None,
            }))
            .is_err()
        );
    }

    #[test]
    fn each_edit_is_one_undo_step_and_drags_fold() {
        let mut e = Editor::default();
        edit(
            &mut e,
            ShaderTreeEdit::Add {
                layer: NewLayer::Group,
                selected: None,
            },
        );
        let group = e.scene.shader_tree.next_id - 1;
        edit(
            &mut e,
            ShaderTreeEdit::SetEnable {
                ids: vec![group, 4],
                enable: false,
            },
        );
        // One opacity drag: three continuing values, one undo step.
        let mut node = e.scene.shader_tree.get(3).unwrap().clone();
        for (k, opacity) in [0.9, 0.7, 0.5].into_iter().enumerate() {
            node.layer.opacity = opacity;
            edit(
                &mut e,
                ShaderTreeEdit::Replace {
                    node: node.clone(),
                    continuing: k > 0,
                },
            );
        }
        assert_eq!(e.history.undo.len(), 3);
        assert_eq!(e.scene.shader_tree.get(3).unwrap().layer.opacity, 0.5);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.shader_tree.get(3).unwrap().layer.opacity, 1.);
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.shader_tree.get(4).unwrap().layer.enable);
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.shader_tree.is_default());
        e.dispatch(Command::Redo).unwrap();
        assert!(e.scene.shader_tree.get(group).is_some());
        // A refused edit changes nothing and records nothing.
        let undo = e.history.undo.len();
        assert!(
            e.dispatch(Command::ShaderTree(ShaderTreeEdit::Move {
                ids: vec![group],
                drop: Drop {
                    parent: Some(group),
                    index: 0,
                },
            }))
            .is_err()
        );
        assert_eq!(e.history.undo.len(), undo);
    }

    #[test]
    fn tree_saves_as_version_15_and_reopens() {
        let mut scene = crate::Scene::default();
        let plain = crate::persistence::encode(&scene).unwrap();
        assert!(String::from_utf8_lossy(&plain).contains("\"version\": 4"));
        let group = scene.shader_tree.add(NewLayer::Group, None).unwrap();
        let map = scene
            .shader_tree
            .add(NewLayer::ImageMap, Some(group))
            .unwrap();
        let mut node = scene.shader_tree.get(group).unwrap().clone();
        node.layer.blend = BlendMode::Screen;
        node.name = "Rust".into();
        if let NodeKind::Group(g) = &mut node.kind {
            g.tag_type = TagType::SelectionSet;
            g.tag = TagMask::Name("Top".into());
        }
        scene.shader_tree.replace(node).unwrap();
        // Image adjustments and the new Material channels round-trip.
        let mut node = scene.shader_tree.get(map).unwrap().clone();
        if let NodeKind::Texture(TextureLayer {
            source: TextureSource::ImageMap { settings, .. },
            ..
        }) = &mut node.kind
        {
            settings.gamma = 2.2;
            settings.swizzle = Swizzle::Red;
            settings.alpha = ImageAlpha::AlphaOnly;
            settings.filtering = Filtering::Bicubic;
            settings.invert_blue = true;
        }
        scene.shader_tree.replace(node).unwrap();
        let mut base = scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.reflection_fresnel = 0.3;
            m.clearcoat_bump = 0.7;
        }
        scene.shader_tree.replace(base).unwrap();
        // A pre-OpenPBR material, so the older version is what is tested.
        scene.shader_tree.legacy_openpbr();
        let bytes = crate::persistence::encode(&scene).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("\"version\": 15"));
        // A layer without a locator (added straight to the tree) gets one
        // when read (texture_locator.rs).
        let mut expected = scene.clone();
        let mut next = expected.next_id;
        expected.give_layers_locators(&mut next).unwrap();
        assert_eq!(crate::persistence::decode(&bytes).unwrap(), expected);
        // An older version number with a tree is refused.
        let old = text.replace("\"version\": 15", "\"version\": 13");
        assert!(crate::persistence::decode(old.as_bytes()).is_err());
    }

    #[test]
    fn shading_model_and_match_specular_need_version_17() {
        for (model, matching) in [
            (ShadingModel::PhysicallyBased, true),
            (ShadingModel::EnergyConserving, true),
            (ShadingModel::Principled, false),
        ] {
            let mut scene = crate::Scene::default();
            let mut base = scene.shader_tree.get(4).unwrap().clone();
            if let NodeKind::Material(m) = &mut base.kind {
                m.shading_model = model;
                m.match_specular = matching;
            }
            scene.shader_tree.replace(base).unwrap();
            // A pre-OpenPBR material, so the older version is what is tested.
            scene.shader_tree.legacy_openpbr();
            let bytes = crate::persistence::encode(&scene).unwrap();
            let text = String::from_utf8(bytes.clone()).unwrap();
            assert!(text.contains("\"version\": 17"), "{model:?}");
            assert_eq!(crate::persistence::decode(&bytes).unwrap(), scene);
            let old = text.replace("\"version\": 17", "\"version\": 16");
            assert!(crate::persistence::decode(old.as_bytes()).is_err());
        }
        // The defaults don't need the new version.
        let mut scene = crate::Scene::default();
        let mut base = scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.roughness = 0.1;
        }
        scene.shader_tree.replace(base).unwrap();
        scene.shader_tree.legacy_openpbr();
        let text = String::from_utf8(crate::persistence::encode(&scene).unwrap()).unwrap();
        assert!(text.contains("\"version\": 15"));
    }

    #[test]
    fn older_files_load_as_principled_with_match_specular_on() {
        // A version 16 file has no match_specular field.
        let mut scene = crate::Scene::default();
        let mut base = scene.shader_tree.get(4).unwrap().clone();
        if let NodeKind::Material(m) = &mut base.kind {
            m.reflection_amount = 1.;
        }
        scene.shader_tree.replace(base).unwrap();
        let bytes = crate::persistence::encode(&scene).unwrap();
        let mut doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        doc["version"] = 16.into();
        fn strip(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Object(map) => {
                    map.remove("match_specular");
                    map.values_mut().for_each(strip);
                }
                serde_json::Value::Array(list) => list.iter_mut().for_each(strip),
                _ => {}
            }
        }
        strip(&mut doc);
        let text = serde_json::to_string(&doc).unwrap();
        assert!(!text.contains("match_specular"));
        let read = crate::persistence::decode(text.as_bytes()).unwrap();
        let NodeKind::Material(m) = &read.shader_tree.get(4).unwrap().kind else {
            panic!()
        };
        assert_eq!(m.shading_model, ShadingModel::Principled);
        assert!(m.match_specular);
        assert_eq!(m.reflection_amount, 1.);
    }

    /// A version 14 document: `scene` encoded, then the fields version 15
    /// added taken out and the version number set back.
    fn as_version_14(scene: &crate::Scene) -> Vec<u8> {
        let bytes = crate::persistence::encode(scene).unwrap();
        let mut doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        doc["version"] = 14.into();
        fn strip(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Object(map) => {
                    for key in ["reflection_fresnel", "clearcoat_bump", "settings"] {
                        map.remove(key);
                    }
                    map.values_mut().for_each(strip);
                }
                serde_json::Value::Array(list) => list.iter_mut().for_each(strip),
                _ => {}
            }
        }
        strip(&mut doc);
        serde_json::to_vec(&doc).unwrap()
    }

    #[test]
    fn older_files_keep_their_values_and_their_look() {
        // A version 14 scene: a saved tree whose Base Material has the old
        // starting values (Specular stored as the reflectance 4 %), a red
        // material with 30 % reflectance, and an Image Map.
        let mut scene = crate::Scene::default();
        let red = crate::Material {
            color: [1., 0., 0.],
            specular: 0.3,
            ..Default::default()
        };
        scene.materials.insert("Red".into(), red);
        scene.map_materials_into_default_tree().unwrap();
        let old = MaterialChannels {
            specular_amount: 0.04,
            ..MaterialChannels::before_tier1()
        };
        let mut base = scene.shader_tree.get(4).unwrap().clone();
        base.kind = NodeKind::Material(Box::new(old.clone()));
        scene.shader_tree.replace(base).unwrap();
        scene.shader_tree.add(NewLayer::ImageMap, None).unwrap();
        let red_id = scene.shader_tree.material_for_tag("Red").unwrap();
        let mut red_node = scene.shader_tree.get(red_id).unwrap().clone();
        if let NodeKind::Material(m) = &mut red_node.kind {
            m.specular_amount = 0.3;
        }
        scene.shader_tree.replace(red_node).unwrap();

        let read = crate::persistence::decode(&as_version_14(&scene)).unwrap();
        // Stored values stay; only the genuinely new channels take their
        // defaults, and Specular becomes the amount that renders the same.
        let NodeKind::Material(m) = &read.shader_tree.get(4).unwrap().kind else {
            panic!()
        };
        assert_eq!(m.diffuse_color, [0.8; 3]);
        assert_eq!(m.fresnel, 0.);
        assert_eq!(m.refractive_index, 1.5);
        assert_eq!(m.subsurface_color, [1.; 3]);
        assert_eq!(m.subsurface_distance, 0.01);
        assert_eq!(m.reflection_fresnel, 0.);
        assert_eq!(m.clearcoat_bump, 1.);
        assert!((m.specular_amount - 0.5).abs() < 1e-12);
        let f0 = |s: f64| crate::material::specular_reflectance(s);
        assert!((f0(read.materials["Red"].specular) - 0.3).abs() < 1e-12);
        let NodeKind::Material(r) = &read.shader_tree.get(red_id).unwrap().kind else {
            panic!()
        };
        assert!((f0(r.specular_amount) - 0.3).abs() < 1e-12);
        assert!(read.shader_tree.nodes.iter().any(|n| matches!(
            &n.kind,
            NodeKind::Texture(TextureLayer { source: TextureSource::ImageMap { settings, .. }, .. })
                if **settings == ImageSettings::default()
        )));
        // The untouched old Base Material still renders as the clay.
        let ctx = crate::shader_eval::Context {
            item: 1,
            ancestors: &[],
            material: "Default",
            part: None,
            sets: &[],
            locators: &[],
        };
        let p = read.shader_tree.evaluate(&ctx).principled().normalized();
        assert!(crate::shader_eval::Principled::clay_surfaces().contains(&p));
        // Saving again writes version 15 and doesn't convert twice.
        let again =
            crate::persistence::decode(&crate::persistence::encode(&read).unwrap()).unwrap();
        assert_eq!(again, read);
    }

    #[test]
    fn older_material_table_converts_and_renders_the_same() {
        // A version 4 file with only a material table (no tree saved).
        let mut scene = crate::Scene::default();
        scene.materials.insert(
            "Default".into(),
            crate::Material {
                color: [0.8; 3],
                specular: 0.04,
                ..Default::default()
            },
        );
        let read = crate::persistence::decode(&as_version_14(&scene)).unwrap();
        assert!((read.materials["Default"].specular - 0.5).abs() < 1e-12);
        assert_eq!(read.materials["Default"].color, [0.8; 3]);
        // Mapped into the default tree it is the old default: clay.
        let tree = read.render_shader_tree();
        let ctx = crate::shader_eval::Context {
            item: 1,
            ancestors: &[],
            material: "Default",
            part: None,
            sets: &[],
            locators: &[],
        };
        let p = tree.evaluate(&ctx).principled().normalized();
        let clay = crate::shader_eval::Principled::clay_surfaces();
        assert!(clay.contains(&p));
    }

    #[test]
    fn set_material_adds_and_updates_the_tag_group() {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        let mesh = crate::Mesh {
            vertices: vec![glam::DVec3::ZERO, glam::DVec3::X, glam::DVec3::Y],
            faces: vec![crate::Face {
                vertices: vec![0, 1, 2],
                uv: None,
                material: String::new(),
                smooth: true,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        };
        e.scene.items[0].mesh = mesh.into();
        e.scene.selection.mode = crate::Mode::Polygon;
        e.scene.selection.faces = vec![0];
        let blue = crate::Material {
            color: [0., 0., 1.],
            ..Default::default()
        };
        e.dispatch(Command::SetMaterial(crate::MaterialAssignment {
            name: "Blue".into(),
            expected: None,
            material: Some(blue),
        }))
        .unwrap();
        let tree = &e.scene.shader_tree;
        let id = tree.material_for_tag("Blue").unwrap();
        let NodeKind::Material(m) = &tree.get(id).unwrap().kind else {
            panic!()
        };
        assert_eq!(m.diffuse_color, [0., 0., 1.]);
        // Undo takes the tag, the table entry and the group away together.
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.shader_tree.material_for_tag("Blue").is_none());
    }
}

#[cfg(test)]
mod mask_tests {
    use super::*;

    #[test]
    fn textures_may_hold_mask_textures_but_nothing_else() {
        let mut tree = ShaderTree::default();
        let outer = tree.add(NewLayer::Constant, None).unwrap();
        let mask = tree.add(NewLayer::ImageMap, None).unwrap();
        let inner = tree.add(NewLayer::Constant, None).unwrap();
        // A mask, and a mask on that mask (v0.1 nesting).
        tree.move_to(
            &[mask],
            Drop {
                parent: Some(outer),
                index: 0,
            },
        )
        .unwrap();
        tree.move_to(
            &[inner],
            Drop {
                parent: Some(mask),
                index: 0,
            },
        )
        .unwrap();
        tree.validate().unwrap();
        assert!(
            tree.move_to(
                &[4],
                Drop {
                    parent: Some(outer),
                    index: 0
                }
            )
            .is_err()
        );
        // Add Layer with a mask selected lands at the top level, not in it.
        let material = tree.add(NewLayer::Material, Some(inner)).unwrap();
        assert_eq!(tree.get(material).unwrap().parent, None);
    }

    #[test]
    fn none_tag_shows_in_the_effect_column() {
        let mut tree = ShaderTree::default();
        let group = tree.add(NewLayer::Group, None).unwrap();
        let mut node = tree.get(group).unwrap().clone();
        assert_eq!(tree.effect_label(&node), "(all)");
        if let NodeKind::Group(g) = &mut node.kind {
            g.tag = TagMask::None;
            g.item = ItemMask::ItemGroup(7);
        }
        tree.replace(node.clone()).unwrap();
        assert_eq!(tree.effect_label(&node), "(none)");
    }
}
