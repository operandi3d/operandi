//! Material conversion, "Lane A" (starter pack step 1): a texture-set
//! folder, optionally with the material's node graph exported from
//! Blender (node_graph.rs), becomes a Material preset: one material group
//! holding one Material, one Image Map layer per map (effect, strength and
//! colour space set) and a Texture Locator for the map's scale and
//! rotation, with the source's licence and credit in the preset header.
//!
//! - Without a graph the texture-set loader decides everything
//!   ([`TextureSet::material_preset`], CD-20..27).
//! - With a graph the material must be "simple": a tree into one
//!   Principled BSDF built only from Image Texture, Mapping, Texture
//!   Coordinate (UV/Generated), Normal Map, Bump, Displacement, Separate
//!   Color, Invert, Mix Color (fixed factor), Math Multiply, RGB and Value
//!   (reroutes pass through). Anything else, or a shape of those nodes
//!   that layers can't express, is refused whole: nothing is converted
//!   ([`Refusal`], "needs Node Material").
//! - Principled values come back through the Material-to-Cycles
//!   translation (`shader_eval::Surface::principled`) run in reverse: Base
//!   Color = Diffuse Color, Specular IOR Level = Specular Amount with IOR
//!   1.5, another IOR turns Use Refractive Index on, Alpha = 1 − Dissolve,
//!   and so on ([`material_channels`]).
//!
//! Provisional readings are marked and listed as BMC-Q.. in
//! docs/SPECIFICATION_QUESTIONS.md.
use super::texture_set::{MapKind, MapName, ResolutionChoice, TextureSet, parse_map_name};
use super::{Preset, PresetCredit, PresetHeader, PresetKind};
use crate::node_graph_reader::{GraphMaterial, GraphNode, SocketRef};
use crate::shader_eval::SPECULAR_IOR;
use crate::shader_tree::{
    BlendMode, Filtering, GroupMask, ImageSettings, Layer, MaterialChannels, NewLayer, Node,
    NodeKind, ShaderTree, Swizzle, TextureEffect, TextureLayer, TextureSource,
};
use crate::texture_locator::{TextureLocator, Tiling};
use crate::{Result, Scene};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A graph that isn't "simple": the whole material is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub material: String,
    pub reason: String,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: needs Node Material (not yet supported): {}",
            self.material, self.reason
        )
    }
}

/// A converted material and notes for the user (approximations, maps left
/// out).
#[derive(Debug)]
pub struct Converted {
    pub preset: Preset,
    pub notes: Vec<String>,
}

/// How a picture is read: as a colour (sRGB) or as plain numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColourSpace {
    Srgb,
    Linear,
}

/// The colour space Operandi's renderers read a picture driving `effect`
/// in: colour effects as sRGB, everything else (roughness, metal, height,
/// normals, opacity) as plain numbers.
pub fn colour_space(effect: TextureEffect) -> ColourSpace {
    match effect {
        TextureEffect::DiffuseColor
        | TextureEffect::SpecularColor
        | TextureEffect::ReflectionColor
        | TextureEffect::TransparentColor
        | TextureEffect::SubsurfaceColor
        | TextureEffect::LuminousColor => ColourSpace::Srgb,
        _ => ColourSpace::Linear,
    }
}

/// Convert a texture set, with `graph` when there is one. `size_of` gives
/// a picture's pixel size; `graph_dir` is the folder the graph's relative
/// image paths start from.
pub fn convert(
    set: &TextureSet,
    graph: Option<(&GraphMaterial, &Path)>,
    credit: Option<PresetCredit>,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> std::result::Result<Converted, ConvertError> {
    let (mut preset, notes) = match graph {
        None => {
            let (mut preset, notes) = set
                .material_preset(ResolutionChoice::Highest, size_of)
                .map_err(ConvertError::Failed)?;
            // A Principled BSDF's own defaults where no map drives them.
            for node in &mut preset.scene.shader_tree.nodes {
                if let NodeKind::Material(m) = &mut node.kind {
                    let distance = m.displacement_distance;
                    **m = material_channels(&GraphNode::default(), &mut Vec::new());
                    m.displacement_distance = distance;
                }
            }
            (preset, notes)
        }
        Some((material, dir)) => {
            let analysis = analyse(material).map_err(ConvertError::Refused)?;
            let (preset, mut notes) =
                build(&set.name, &analysis, dir, size_of).map_err(ConvertError::Failed)?;
            // The graph decides which maps are used; say which it skipped.
            let used: Vec<String> = analysis
                .layers
                .iter()
                .map(|l| file_name(&l.path).to_ascii_lowercase())
                .collect();
            for map in &set.maps {
                let name = map
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !used.contains(&name.to_ascii_lowercase()) {
                    notes.push(format!(
                        "{name} isn't used by the material's graph; left out"
                    ));
                }
            }
            (preset, notes)
        }
    };
    preset.header.source = credit;
    preset.scene.validate().map_err(ConvertError::Failed)?;
    Ok(Converted { preset, notes })
}

/// Why nothing was converted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConvertError {
    /// The graph isn't simple.
    Refused(Refusal),
    /// Something else (an unreadable picture, a bad file).
    Failed(String),
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertError::Refused(r) => r.fmt(f),
            ConvertError::Failed(e) => f.write_str(e),
        }
    }
}

// ------------------------------------------------------------ analysis

/// What a simple graph says, ready to become layers.
#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    pub material: String,
    /// The Principled BSDF node (its unlinked inputs are the values).
    pub principled: GraphNode,
    /// Image-driven layers, top of the layer list first.
    pub layers: Vec<GraphLayer>,
    pub mapping: UvMapping,
    /// Bump node Distance × Strength, when a bump layer exists.
    pub bump_amplitude: Option<f64>,
    /// Displacement node Scale, when a displacement layer exists.
    pub displacement_distance: Option<f64>,
    pub notes: Vec<String>,
}

/// One image layer a graph asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphLayer {
    pub effect: TextureEffect,
    /// As written in the graph (relative to the graph's folder, or
    /// absolute).
    pub path: String,
    /// The colour space Blender read it in.
    pub colorspace: String,
    /// Red, green, blue or alpha only (Separate Color, or the Alpha
    /// output).
    pub swizzle: Swizzle,
    pub invert: bool,
    /// Math Multiply by a constant (scalar effects).
    pub multiply: f64,
    pub blend: BlendMode,
    pub opacity: f64,
    /// Low/High values for displacement midlevel; `None` = 0/1.
    pub range: Option<(f64, f64)>,
    pub filtering: Filtering,
    pub tiling: Tiling,
}

/// The one UV placement every image shares (Mapping node).
#[derive(Clone, Debug, PartialEq)]
pub struct UvMapping {
    pub repeat: [f64; 2],
    /// Degrees, counter-clockwise.
    pub rotation: f64,
    /// Not representable yet on a UV Map locator (BMC-Q02): dropped with a
    /// note.
    pub offset: [f64; 2],
    /// Generated coordinates were asked for (read as UV, BMC-Q03).
    pub generated: bool,
}

impl Default for UvMapping {
    fn default() -> Self {
        Self {
            repeat: [1., 1.],
            rotation: 0.,
            offset: [0., 0.],
            generated: false,
        }
    }
}

/// The node types a simple material may use.
const ALLOWED: &[&str] = &[
    "ShaderNodeOutputMaterial",
    "ShaderNodeBsdfPrincipled",
    "ShaderNodeTexImage",
    "ShaderNodeMapping",
    "ShaderNodeTexCoord",
    "ShaderNodeNormalMap",
    "ShaderNodeBump",
    "ShaderNodeDisplacement",
    "ShaderNodeSeparateColor",
    "ShaderNodeSeparateRGB",
    "ShaderNodeInvert",
    "ShaderNodeMix",
    "ShaderNodeMixRGB",
    "ShaderNodeMath",
    "ShaderNodeRGB",
    "ShaderNodeValue",
    "NodeReroute",
];

/// Principled inputs a texture may drive, with the effect that carries
/// them. Alpha becomes Dissolve, inverted.
const PRINCIPLED_EFFECTS: &[(&str, TextureEffect)] = &[
    ("Base Color", TextureEffect::DiffuseColor),
    ("Metallic", TextureEffect::Metallic),
    ("Roughness", TextureEffect::Roughness),
    ("Alpha", TextureEffect::Dissolve),
    ("Emission Color", TextureEffect::LuminousColor),
    ("Emission Strength", TextureEffect::LuminousAmount),
    ("Specular IOR Level", TextureEffect::SpecularAmount),
    ("Specular Tint", TextureEffect::SpecularColor),
    ("Coat Weight", TextureEffect::ClearcoatAmount),
    ("Sheen Weight", TextureEffect::Sheen),
    ("Subsurface Weight", TextureEffect::SubsurfaceAmount),
    ("Transmission Weight", TextureEffect::TransparentAmount),
    ("Diffuse Roughness", TextureEffect::DiffuseRoughness),
];

/// What a traced socket gives.
#[derive(Clone, Debug, PartialEq)]
enum Traced {
    Const(Vec<f64>),
    /// Image layers, bottom first.
    Layers(Vec<GraphLayer>),
}

struct Tracer<'a> {
    m: &'a GraphMaterial,
    mapping: Option<UvMapping>,
    notes: Vec<String>,
}

/// Check a material is simple and read it ([`Analysis`]). Nodes nothing
/// reaches from the output are ignored.
pub fn analyse(m: &GraphMaterial) -> std::result::Result<Analysis, Refusal> {
    let refuse = |reason: String| Refusal {
        material: m.name.clone(),
        reason,
    };
    let output = m
        .output
        .as_deref()
        .and_then(|o| m.node(o))
        .ok_or_else(|| refuse("it has no Material Output".into()))?;
    if m.link_into(&output.name, "Volume").is_some() {
        return Err(refuse("its output has a Volume".into()));
    }
    let surface = m
        .link_into(&output.name, "Surface")
        .ok_or_else(|| refuse("nothing is connected to the Surface".into()))?;
    let principled = m
        .node(surface.from.node())
        .ok_or_else(|| refuse(format!("link from a missing node {}", surface.from.node())))?;
    if principled.kind != "ShaderNodeBsdfPrincipled" {
        return Err(refuse(format!(
            "its Surface is a {} node, not a Principled BSDF",
            short(&principled.kind)
        )));
    }
    let mut t = Tracer {
        m,
        mapping: None,
        notes: Vec::new(),
    };
    let mut layers: Vec<GraphLayer> = Vec::new();
    let mut bump_amplitude = None;
    let mut displacement_distance = None;
    // Every linked Principled input.
    let mut linked: Vec<&crate::node_graph_reader::GraphLink> = m
        .links
        .iter()
        .filter(|l| l.to.node() == principled.name)
        .collect();
    linked.sort_by(|a, b| a.to.socket().cmp(b.to.socket()));
    // Layers are gathered per effect, then put in the loader's order.
    let mut by_effect: Vec<(TextureEffect, Vec<GraphLayer>)> = Vec::new();
    for link in linked {
        let socket = link.to.socket();
        if socket == "Normal" {
            let (normal, bump) = t.normal(&link.from).map_err(refuse)?;
            if let Some(n) = normal {
                by_effect.push((TextureEffect::Normal, vec![n]));
            }
            if let Some((b, amplitude)) = bump {
                by_effect.push((TextureEffect::Bump, b));
                bump_amplitude = Some(amplitude);
            }
            continue;
        }
        let Some(&(_, effect)) = PRINCIPLED_EFFECTS.iter().find(|(s, _)| *s == socket) else {
            return Err(refuse(format!("its {socket} input is driven by nodes")));
        };
        match t.trace(&link.from, 0).map_err(refuse)? {
            Traced::Const(v) => t.notes.push(format!(
                "{socket} is a constant from nodes ({}); used as the material value",
                fmt_numbers(&v)
            )),
            Traced::Layers(mut ls) => {
                for l in &mut ls {
                    l.effect = effect;
                    // Dissolve is 1 − Alpha.
                    if socket == "Alpha" {
                        l.invert = !l.invert;
                    }
                }
                by_effect.push((effect, ls));
            }
        }
    }
    // Displacement into the output.
    if let Some(link) = m.link_into(&output.name, "Displacement") {
        let node = t.node(link.from.node()).map_err(refuse)?;
        if node.kind != "ShaderNodeDisplacement" {
            return Err(refuse(format!(
                "its Displacement comes from a {} node",
                short(&node.kind)
            )));
        }
        let height = m
            .link_into(&node.name, "Height")
            .ok_or_else(|| refuse("its Displacement node has no Height".into()))?;
        if m.link_into(&node.name, "Normal").is_some() {
            t.notes
                .push("The Displacement node's Normal input is left out".into());
        }
        let mid = node.scalar("Midlevel").unwrap_or(0.5);
        let scale = node.scalar("Scale").unwrap_or(1.);
        match t.trace(&height.from, 0).map_err(refuse)? {
            Traced::Const(_) => t
                .notes
                .push("A constant displacement does nothing; left out".into()),
            Traced::Layers(mut ls) => {
                for l in &mut ls {
                    l.effect = TextureEffect::Displacement;
                    // Blender moves (height − midlevel) × scale.
                    l.range = Some((-mid * l.multiply, (1. - mid) * l.multiply));
                    l.multiply = 1.;
                }
                by_effect.push((TextureEffect::Displacement, ls));
                displacement_distance = Some(scale);
            }
        }
    }
    // The loader's order, top first (AO multiply above the colour).
    let order = [
        TextureEffect::DiffuseColor,
        TextureEffect::Roughness,
        TextureEffect::Metallic,
        TextureEffect::SpecularAmount,
        TextureEffect::SpecularColor,
        TextureEffect::ClearcoatAmount,
        TextureEffect::Sheen,
        TextureEffect::SubsurfaceAmount,
        TextureEffect::DiffuseRoughness,
        TextureEffect::TransparentAmount,
        TextureEffect::Normal,
        TextureEffect::Bump,
        TextureEffect::Displacement,
        TextureEffect::Dissolve,
        TextureEffect::LuminousColor,
        TextureEffect::LuminousAmount,
    ];
    for effect in order {
        for (_, ls) in by_effect.iter().filter(|(e, _)| *e == effect) {
            // Traced bottom first; the list wants the top first.
            layers.extend(ls.iter().rev().cloned());
        }
    }
    let mapping = t.mapping.clone().unwrap_or_default();
    if mapping.offset != [0., 0.] {
        t.notes.push(format!(
            "Mapping Location {} is left out: a UV Map locator has no offset yet (BMC-Q02)",
            fmt_numbers(&mapping.offset)
        ));
    }
    if mapping.generated {
        t.notes
            .push("Generated coordinates are read as the mesh's UVs (provisional, BMC-Q03)".into());
    }
    for l in &layers {
        let wants = match colour_space(l.effect) {
            ColourSpace::Srgb => "sRGB",
            ColourSpace::Linear => "Non-Color",
        };
        let has = if l.colorspace.eq_ignore_ascii_case("sRGB") {
            "sRGB"
        } else {
            "Non-Color"
        };
        if !l.colorspace.is_empty() && has != wants {
            t.notes.push(format!(
                "{} is read as {wants} for {} (Blender read it as {})",
                file_name(&l.path),
                l.effect.label(),
                l.colorspace
            ));
        }
    }
    Ok(Analysis {
        material: m.name.clone(),
        principled: principled.clone(),
        layers,
        mapping,
        bump_amplitude,
        displacement_distance,
        notes: t.notes,
    })
}

impl<'a> Tracer<'a> {
    fn node(&self, name: &str) -> std::result::Result<&'a GraphNode, String> {
        let node = self
            .m
            .node(name)
            .ok_or_else(|| format!("a link names a missing node {name}"))?;
        if !ALLOWED.contains(&node.kind.as_str()) {
            return Err(format!("it uses a {} node", short(&node.kind)));
        }
        Ok(node)
    }

    /// The Normal input: a Normal Map, a Bump, or a Bump over a Normal Map.
    #[allow(clippy::type_complexity)]
    fn normal(
        &mut self,
        from: &SocketRef,
    ) -> std::result::Result<(Option<GraphLayer>, Option<(Vec<GraphLayer>, f64)>), String> {
        let node = self.node(from.node())?;
        match node.kind.as_str() {
            "ShaderNodeNormalMap" => Ok((Some(self.normal_map(node)?), None)),
            "ShaderNodeBump" => {
                if ["Strength", "Distance"]
                    .iter()
                    .any(|s| self.m.link_into(&node.name, s).is_some())
                {
                    return Err("its Bump strength is driven by nodes".into());
                }
                let strength = node.scalar("Strength").unwrap_or(1.);
                let distance = node.scalar("Distance").unwrap_or(1.);
                let height = self
                    .m
                    .link_into(&node.name, "Height")
                    .ok_or("its Bump node has no Height")?;
                let mut layers = match self.trace(&height.from, 0)? {
                    Traced::Layers(ls) => ls,
                    Traced::Const(_) => Vec::new(),
                };
                let invert = node.props.get("invert").and_then(|v| v.as_bool()) == Some(true);
                for l in &mut layers {
                    l.effect = TextureEffect::Bump;
                    l.invert ^= invert;
                }
                if strength > 1. {
                    self.notes
                        .push(format!("Bump Strength {strength} is used as 1"));
                }
                let normal = match self.m.link_into(&node.name, "Normal") {
                    Some(link) => {
                        let inner = self.node(link.from.node())?;
                        if inner.kind != "ShaderNodeNormalMap" {
                            return Err(format!(
                                "its Bump's Normal comes from a {} node",
                                short(&inner.kind)
                            ));
                        }
                        Some(self.normal_map(inner)?)
                    }
                    None => None,
                };
                let bump = (!layers.is_empty()).then(|| (layers, distance * strength.min(1.)));
                Ok((normal, bump))
            }
            "NodeReroute" => {
                let input = self
                    .m
                    .link_into(&node.name, "Input")
                    .ok_or("a reroute leads nowhere")?;
                self.normal(&input.from.clone())
            }
            other => Err(format!("its Normal comes from a {} node", short(other))),
        }
    }

    fn normal_map(&mut self, node: &GraphNode) -> std::result::Result<GraphLayer, String> {
        if node.prop("space").is_some_and(|s| s != "TANGENT") {
            return Err(format!(
                "its Normal Map is in {} space",
                node.prop("space").unwrap_or("")
            ));
        }
        if self.m.link_into(&node.name, "Strength").is_some() {
            return Err("its Normal Map strength is driven by nodes".into());
        }
        let strength = node.scalar("Strength").unwrap_or(1.);
        let color = self
            .m
            .link_into(&node.name, "Color")
            .ok_or("its Normal Map has no colour input")?;
        let Traced::Layers(mut ls) = self.trace(&color.from, 0)? else {
            return Err("its Normal Map reads a constant".into());
        };
        if ls.len() != 1 {
            return Err("its Normal Map mixes several pictures".into());
        }
        let mut l = ls.remove(0);
        l.effect = TextureEffect::Normal;
        // Layer opacity fades the normal toward flat: strength up to 1.
        if strength > 1. {
            self.notes
                .push(format!("Normal Map Strength {strength} is used as 1"));
        }
        l.opacity *= strength.clamp(0., 1.);
        Ok(l)
    }

    /// Follow an output socket back to images or a constant.
    fn trace(&mut self, from: &SocketRef, depth: usize) -> std::result::Result<Traced, String> {
        if depth > 64 {
            return Err("its nodes loop or nest too deep".into());
        }
        let node = self.node(from.node())?;
        let input = |name: &str| self.m.link_into(&node.name, name).map(|l| l.from.clone());
        match node.kind.as_str() {
            "ShaderNodeTexImage" => {
                let image = node.image.as_ref().ok_or("an Image Texture has no image")?;
                if node.prop("projection").is_some_and(|p| p != "FLAT") {
                    return Err(format!(
                        "an Image Texture uses {} projection",
                        node.prop("projection").unwrap_or("")
                    ));
                }
                self.vector(node)?;
                let swizzle = if from.is("Alpha") {
                    Swizzle::Alpha
                } else {
                    Swizzle::Rgba
                };
                Ok(Traced::Layers(vec![GraphLayer {
                    effect: TextureEffect::DiffuseColor,
                    path: image.path.clone(),
                    colorspace: image.colorspace.clone(),
                    swizzle,
                    invert: false,
                    multiply: 1.,
                    blend: BlendMode::Normal,
                    opacity: 1.,
                    range: None,
                    filtering: match node.prop("interpolation") {
                        Some("Closest") => Filtering::Nearest,
                        Some("Cubic") | Some("Smart") => Filtering::Bicubic,
                        _ => Filtering::Bilinear,
                    },
                    tiling: match node.prop("extension") {
                        Some("EXTEND") => Tiling::Edge,
                        Some("CLIP") => Tiling::Reset,
                        Some("MIRROR") => Tiling::Mirror,
                        _ => Tiling::Repeat,
                    },
                }]))
            }
            "ShaderNodeRGB" | "ShaderNodeValue" => Ok(Traced::Const(
                node.output_value()
                    .ok_or("an RGB or Value node has no value")?,
            )),
            "NodeReroute" => {
                let from = input("Input").ok_or("a reroute leads nowhere")?;
                self.trace(&from, depth + 1)
            }
            "ShaderNodeSeparateColor" | "ShaderNodeSeparateRGB" => {
                if node.prop("mode").is_some_and(|m| m != "RGB") {
                    return Err(format!(
                        "a Separate Color node splits {}",
                        node.prop("mode").unwrap_or("")
                    ));
                }
                let channel = match from.socket() {
                    "Red" | "R" => Swizzle::Red,
                    "Green" | "G" => Swizzle::Green,
                    "Blue" | "B" => Swizzle::Blue,
                    s => return Err(format!("a Separate Color output {s}")),
                };
                let src = input("Color").or_else(|| input("Image"));
                let Some(src) = src else {
                    return Err("a Separate Color node reads a constant".into());
                };
                match self.trace(&src, depth + 1)? {
                    Traced::Layers(mut ls) if ls.len() == 1 && ls[0].swizzle == Swizzle::Rgba => {
                        ls[0].swizzle = channel;
                        Ok(Traced::Layers(ls))
                    }
                    Traced::Const(v) => {
                        let i = channel as usize - Swizzle::Red as usize;
                        Ok(Traced::Const(vec![v.get(i).copied().unwrap_or(0.)]))
                    }
                    _ => Err("a Separate Color node splits a mix of pictures".into()),
                }
            }
            "ShaderNodeInvert" => {
                if input("Fac").is_some() {
                    return Err("an Invert's Fac is driven by nodes".into());
                }
                let fac = node.scalar("Fac").unwrap_or(1.);
                if (fac - 1.).abs() > 1e-6 && fac.abs() > 1e-6 {
                    return Err(format!("an Invert node is only {}% on", fac * 100.));
                }
                let Some(src) = input("Color") else {
                    let v = node.number("Color").unwrap_or_default();
                    return Ok(Traced::Const(
                        v.iter()
                            .take(3)
                            .map(|c| if fac > 0.5 { 1. - c } else { *c })
                            .collect(),
                    ));
                };
                match self.trace(&src, depth + 1)? {
                    Traced::Layers(mut ls) if ls.len() == 1 => {
                        if fac > 0.5 {
                            ls[0].invert = !ls[0].invert;
                        }
                        Ok(Traced::Layers(ls))
                    }
                    Traced::Const(v) => Ok(Traced::Const(
                        v.iter()
                            .map(|c| if fac > 0.5 { 1. - c } else { *c })
                            .collect(),
                    )),
                    _ => Err("an Invert node inverts a mix of pictures".into()),
                }
            }
            "ShaderNodeMath" => {
                if node.prop("operation") != Some("MULTIPLY") {
                    return Err(format!(
                        "a Math node does {}",
                        node.prop("operation").unwrap_or("?")
                    ));
                }
                let a = self.operand(node, 0, depth)?;
                let b = self.operand(node, 1, depth)?;
                multiply(a, b)
            }
            "ShaderNodeMix" | "ShaderNodeMixRGB" => self.mix(node, from, depth),
            "ShaderNodeTexCoord" | "ShaderNodeMapping" => {
                Err("a coordinate feeds a material input directly".into())
            }
            other => Err(format!("a {} node feeds a material input", short(other))),
        }
    }

    /// One of Math's two inputs: linked (traced) or a constant.
    fn operand(
        &mut self,
        node: &GraphNode,
        index: usize,
        depth: usize,
    ) -> std::result::Result<Traced, String> {
        // Math's inputs share the name "Value"; links and unlinked inputs
        // name them by identifier.
        let id = ["Value", "Value_001"][index];
        let link = self
            .m
            .links
            .iter()
            .find(|l| l.to.node() == node.name && l.to.identifier() == Some(id));
        if let Some(link) = link {
            return self.trace(&link.from.clone(), depth + 1);
        }
        Ok(Traced::Const(node.number(id).unwrap_or(vec![0.])))
    }

    fn mix(
        &mut self,
        node: &GraphNode,
        from: &SocketRef,
        depth: usize,
    ) -> std::result::Result<Traced, String> {
        let legacy = node.kind == "ShaderNodeMixRGB";
        if !legacy && node.prop("data_type").is_some_and(|d| d != "RGBA") {
            return Err(format!(
                "a Mix node mixes {}",
                node.prop("data_type").unwrap_or("")
            ));
        }
        if !legacy && !from.is("Result") {
            return Err("a Mix node output isn't its Result".into());
        }
        let (fac_id, a_id, b_id) = if legacy {
            ("Fac", "Color1", "Color2")
        } else {
            ("Factor_Float", "A_Color", "B_Color")
        };
        let linked = |id: &str| {
            self.m
                .links
                .iter()
                .find(|l| {
                    l.to.node() == node.name
                        && (l.to.identifier() == Some(id) || l.to.socket() == id)
                })
                .map(|l| l.from.clone())
        };
        if linked(fac_id).is_some() || (!legacy && linked("Factor").is_some()) {
            return Err("a Mix Color factor is driven by nodes".into());
        }
        let fac = node
            .scalar(fac_id)
            .or_else(|| node.scalar("Factor"))
            .unwrap_or(0.5)
            .clamp(0., 1.);
        let blend = match node.prop("blend_type").unwrap_or("MIX") {
            "MIX" => BlendMode::Normal,
            "MULTIPLY" => BlendMode::Multiply,
            "ADD" => BlendMode::Add,
            "SUBTRACT" => BlendMode::Subtract,
            "SCREEN" => BlendMode::Screen,
            "OVERLAY" => BlendMode::Overlay,
            "DIFFERENCE" => BlendMode::Difference,
            "DIVIDE" => BlendMode::Divide,
            "DARKEN" => BlendMode::Darken,
            "LIGHTEN" => BlendMode::Lighten,
            "DODGE" => BlendMode::ColorDodge,
            "BURN" => BlendMode::ColorBurn,
            "SOFT_LIGHT" => BlendMode::SoftLight,
            other => return Err(format!("a Mix Color node uses {other}")),
        };
        let a = match linked(a_id).or_else(|| if legacy { None } else { linked("A") }) {
            Some(f) => self.trace(&f, depth + 1)?,
            None => Traced::Const(
                node.number(a_id)
                    .or_else(|| node.number("A"))
                    .unwrap_or_default(),
            ),
        };
        let b = match linked(b_id).or_else(|| if legacy { None } else { linked("B") }) {
            Some(f) => self.trace(&f, depth + 1)?,
            None => Traced::Const(
                node.number(b_id)
                    .or_else(|| node.number("B"))
                    .unwrap_or_default(),
            ),
        };
        match (a, b) {
            (Traced::Layers(mut below), Traced::Layers(above)) => {
                for mut l in above {
                    l.blend = blend;
                    l.opacity *= fac;
                    below.push(l);
                }
                Ok(Traced::Layers(below))
            }
            (a @ Traced::Layers(_), Traced::Const(_)) if fac == 0. => Ok(a),
            (Traced::Const(_), Traced::Layers(b)) if fac == 1. && blend == BlendMode::Normal => {
                Ok(Traced::Layers(b))
            }
            (Traced::Const(a), Traced::Const(_)) if fac == 0. => Ok(Traced::Const(a)),
            _ => Err("a Mix Color node mixes a picture with a constant colour".into()),
        }
    }

    /// An image's Vector input: unlinked (UV), Texture Coordinate UV or
    /// Generated, maybe through one Mapping. Every image must share it.
    fn vector(&mut self, image: &GraphNode) -> std::result::Result<(), String> {
        let mut mapping = UvMapping::default();
        let mut at = self
            .m
            .link_into(&image.name, "Vector")
            .map(|l| l.from.clone());
        let mut seen_mapping = false;
        for _ in 0..16 {
            let Some(from) = at.clone() else { break };
            let node = self.node(from.node())?;
            match node.kind.as_str() {
                "NodeReroute" => {
                    at = self
                        .m
                        .link_into(&node.name, "Input")
                        .map(|l| l.from.clone())
                }
                "ShaderNodeTexCoord" => {
                    match from.socket() {
                        "UV" => {}
                        "Generated" => mapping.generated = true,
                        s => return Err(format!("an image reads {s} coordinates")),
                    }
                    at = None;
                }
                "ShaderNodeMapping" if !seen_mapping => {
                    seen_mapping = true;
                    for s in ["Location", "Rotation", "Scale"] {
                        if self.m.link_into(&node.name, s).is_some() {
                            return Err(format!("a Mapping's {s} is driven by nodes"));
                        }
                    }
                    let kind = node.prop("vector_type").unwrap_or("POINT");
                    let loc = node.number("Location").unwrap_or(vec![0.; 3]);
                    let rot = node.number("Rotation").unwrap_or(vec![0.; 3]);
                    let scale = node.number("Scale").unwrap_or(vec![1.; 3]);
                    let g = |v: &Vec<f64>, i: usize, d: f64| v.get(i).copied().unwrap_or(d);
                    if g(&rot, 0, 0.).abs() > 1e-6 || g(&rot, 1, 0.).abs() > 1e-6 {
                        return Err("a Mapping tilts the texture out of the UV plane".into());
                    }
                    let degrees = g(&rot, 2, 0.).to_degrees();
                    let (sx, sy) = (g(&scale, 0, 1.), g(&scale, 1, 1.));
                    match kind {
                        "POINT" | "VECTOR" => {
                            mapping.repeat = [sx, sy];
                            mapping.rotation = degrees;
                            if kind == "POINT" {
                                mapping.offset = [g(&loc, 0, 0.), g(&loc, 1, 0.)];
                            }
                        }
                        "TEXTURE" => {
                            if sx == 0. || sy == 0. {
                                return Err("a Mapping scales to zero".into());
                            }
                            mapping.repeat = [1. / sx, 1. / sy];
                            mapping.rotation = -degrees;
                            mapping.offset = [-g(&loc, 0, 0.), -g(&loc, 1, 0.)];
                        }
                        other => return Err(format!("a Mapping of type {other}")),
                    }
                    at = self
                        .m
                        .link_into(&node.name, "Vector")
                        .map(|l| l.from.clone());
                }
                other => {
                    return Err(format!(
                        "an image's coordinates come from a {} node",
                        short(other)
                    ));
                }
            }
        }
        match &self.mapping {
            None => self.mapping = Some(mapping),
            Some(m) if *m == mapping => {}
            Some(_) => return Err("its images use different mappings".into()),
        }
        Ok(())
    }
}

fn multiply(a: Traced, b: Traced) -> std::result::Result<Traced, String> {
    match (a, b) {
        (Traced::Const(x), Traced::Const(y)) => Ok(Traced::Const(vec![
            x.first().copied().unwrap_or(0.) * y.first().copied().unwrap_or(0.),
        ])),
        (Traced::Layers(mut ls), Traced::Const(k)) | (Traced::Const(k), Traced::Layers(mut ls))
            if ls.len() == 1 =>
        {
            ls[0].multiply *= k.first().copied().unwrap_or(0.);
            Ok(Traced::Layers(ls))
        }
        _ => Err("a Math node multiplies two pictures".into()),
    }
}

// ------------------------------------------------------------ building

/// Principled BSDF values (unlinked inputs) as material channels: the
/// Material-to-Cycles translation (`Surface::principled`) run backwards.
/// Inputs it can't carry are left out with a note.
pub fn material_channels(p: &GraphNode, notes: &mut Vec<String>) -> MaterialChannels {
    let mut m = match NewLayer::Material.kind() {
        NodeKind::Material(m) => *m,
        _ => unreachable!("a new material layer is a material"),
    };
    let s = |name: &str, default: f64| p.scalar(name).unwrap_or(default);
    let rgb = |name: &str, default: [f64; 3]| {
        p.number(name)
            .filter(|v| v.len() >= 3)
            .map_or(default, |v| [v[0], v[1], v[2]])
    };
    // Base Color = Diffuse Color (Principled model).
    m.shading_model = crate::shader_tree::ShadingModel::Principled;
    m.diffuse_color = rgb("Base Color", [0.8, 0.8, 0.8]);
    m.diffuse_amount = 1.;
    m.metallic = s("Metallic", 0.);
    m.roughness = s("Roughness", 0.5);
    m.diffuse_roughness = s("Diffuse Roughness", 0.);
    m.match_specular = true;
    // IOR 1.5: Specular IOR Level is Specular Amount. Another IOR: Use
    // Refractive Index, whose level is fixed at 0.5.
    let ior = s("IOR", SPECULAR_IOR);
    let level = s("Specular IOR Level", 0.5);
    if (ior - SPECULAR_IOR).abs() < 1e-4 {
        m.specular_amount = level;
        m.more.use_refractive_index = false;
    } else {
        m.more.use_refractive_index = true;
        m.refractive_index = ior;
        m.specular_amount = level;
        if (level - 0.5).abs() > 1e-4 {
            notes.push(format!(
                "Specular IOR Level {level} is left at 50%: with IOR {ior} the index sets the reflections"
            ));
        }
    }
    m.specular_color = rgb("Specular Tint", [1., 1., 1.]);
    m.anisotropy = s("Anisotropic", 0.);
    m.subsurface_amount = s("Subsurface Weight", 0.);
    m.subsurface_color = rgb("Subsurface Radius", [1., 0.2, 0.1]);
    m.subsurface_distance = s("Subsurface Scale", 0.05);
    m.sheen = s("Sheen Weight", 0.);
    m.clearcoat_amount = s("Coat Weight", 0.);
    m.clearcoat_roughness = s("Coat Roughness", 0.03);
    m.luminous_color = rgb("Emission Color", [1., 1., 1.]);
    m.luminous_intensity = s("Emission Strength", 0.);
    m.dissolve = 1. - s("Alpha", 1.).clamp(0., 1.);
    let transmission = s("Transmission Weight", 0.);
    if transmission > 0. {
        // Approximate (BMC-Q04): our see-through mix, tinted by the base.
        m.transparent_amount = transmission;
        m.transparent_color = m.diffuse_color;
        m.refractive_index = ior;
        m.refraction_roughness = m.roughness;
        notes.push("Transmission becomes Transparent Amount (approximate)".into());
    }
    for (name, default) in [
        ("Coat IOR", 1.5),
        ("Sheen Roughness", 0.5),
        ("Thin Film Thickness", 0.),
        ("Anisotropic Rotation", 0.),
    ] {
        if let Some(v) = p.scalar(name)
            && (v - default).abs() > 1e-4
        {
            notes.push(format!("{name} {v} is left out"));
        }
    }
    for name in ["Coat Tint", "Sheen Tint"] {
        if let Some(v) = p.number(name)
            && v.iter().take(3).any(|c| (c - 1.).abs() > 1e-4)
        {
            notes.push(format!("{name} {} is left out", fmt_numbers(&v)));
        }
    }
    m
}

/// Layers from an analysed graph: group, image layers (top first), the
/// material at the bottom, one shared Texture Locator.
fn build(
    name: &str,
    a: &Analysis,
    graph_dir: &Path,
    size_of: &dyn Fn(&Path) -> Option<(u32, u32)>,
) -> Result<(Preset, Vec<String>)> {
    let mut notes = a.notes.clone();
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
    let mut nodes = vec![node(1, None, name, NodeKind::Group(GroupMask::default()))];
    let mut next = 2;
    if a.layers.is_empty() {
        notes.push("The material uses no pictures".into());
    }
    for l in &a.layers {
        let path = resolve(graph_dir, &l.path);
        let (w, h) =
            size_of(&path).ok_or_else(|| format!("{} can't be read", path.to_string_lossy()))?;
        let image = scene.image_for_path(&path.to_string_lossy(), w, h)?;
        let mut settings = ImageSettings {
            swizzle: l.swizzle,
            filtering: l.filtering,
            ..ImageSettings::default()
        };
        if l.invert {
            settings.invert_red = true;
            settings.invert_green = true;
            settings.invert_blue = true;
        }
        if let Some((low, high)) = l.range {
            settings.low_value = low;
            settings.high_value = high;
        }
        if l.multiply != 1. {
            if colour_space(l.effect) == ColourSpace::Srgb {
                settings.brightness = l.multiply;
            } else {
                settings.high_value *= l.multiply;
                settings.low_value *= l.multiply;
            }
        }
        // CD-22: a DirectX normal has its green flipped, whatever the
        // source graph did with it.
        let kind = match parse_map_name(&stem(&path)) {
            Some(MapName::Map { kind, .. }) => Some(kind),
            _ => None,
        };
        if l.effect == TextureEffect::Normal && kind == Some(MapKind::NormalDx) {
            settings.invert_green = !settings.invert_green;
            notes.push(format!(
                "{} is a DirectX normal map: green flipped",
                file_name(&l.path)
            ));
        }
        let mut kind_layer = NewLayer::ImageMap.kind();
        if let NodeKind::Texture(TextureLayer { source, effect, .. }) = &mut kind_layer {
            *source = TextureSource::ImageMap {
                image: Some(image),
                settings: Box::new(settings),
            };
            *effect = l.effect;
        }
        let mut n = node(
            next,
            Some(1),
            &format!("{name} {}", l.effect.label()),
            kind_layer,
        );
        n.layer = Layer {
            blend: l.blend,
            opacity: l.opacity.clamp(0., 1.),
            ..Layer::default()
        };
        next += 1;
        nodes.push(n);
    }
    let mut channels = material_channels(&a.principled, &mut notes);
    channels.bump_amplitude = a.bump_amplitude.unwrap_or(0.);
    channels.displacement_distance = a.displacement_distance.unwrap_or(0.);
    nodes.push(node(
        next,
        Some(1),
        &format!("{name} Material"),
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
    // One locator per tiling (normally one for the whole material), with
    // the mapping's repeat and rotation.
    let mut next_item = scene.next_id.max(1);
    let mut locators: BTreeMap<String, u64> = BTreeMap::new();
    let layer_ids: Vec<(u64, Tiling)> = scene
        .shader_tree
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Texture(_)))
        .zip(&a.layers)
        .map(|(n, l)| (n.id, l.tiling))
        .collect();
    for (id, tiling) in layer_ids {
        let key = tiling.label().to_owned();
        let locator = match locators.get(&key) {
            Some(&l) => l,
            None => {
                let l = scene.add_texture_locator(
                    &mut next_item,
                    name,
                    TextureLocator {
                        repeat: a.mapping.repeat,
                        uv_rotation: a.mapping.rotation,
                        tiling: [tiling, tiling],
                        ..TextureLocator::new(crate::shader_tree::Projection::UvMap)
                    },
                )?;
                locators.insert(key, l);
                l
            }
        };
        if let Some(n) = scene.shader_tree.nodes.iter_mut().find(|n| n.id == id)
            && let NodeKind::Texture(t) = &mut n.kind
        {
            t.locator = Some(locator);
        }
    }
    scene.next_id = scene.next_id.max(next_item);
    scene.validate()?;
    Ok((
        Preset {
            header: PresetHeader {
                kind: PresetKind::Material,
                thumbnail: None,
                source: None,
            },
            scene,
        },
        notes,
    ))
}

/// Pick the graph material that best matches `set`: the one using the
/// most of the set's pictures, a normal or bump breaking ties (ambientCG's
/// files hold a _NORMAL and a _DISPLACEMENT variant) [chosen, BMC-Q05].
pub fn pick_material<'a>(
    materials: &'a [GraphMaterial],
    set: &TextureSet,
) -> Option<&'a GraphMaterial> {
    let names: Vec<String> = set
        .maps
        .iter()
        .filter_map(|m| m.path.file_name())
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .collect();
    materials
        .iter()
        .filter(|m| m.output.is_some())
        .map(|m| {
            let used = m
                .nodes
                .iter()
                .filter_map(|n| n.image.as_ref())
                .filter(|i| names.contains(&file_name(&i.path).to_ascii_lowercase()))
                .count();
            let relief = m
                .nodes
                .iter()
                .any(|n| matches!(n.kind.as_str(), "ShaderNodeNormalMap" | "ShaderNodeBump"));
            ((used, relief), m)
        })
        .filter(|((used, _), _)| *used > 0)
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, m)| m)
}

fn resolve(dir: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        dir.join(p)
    }
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

/// "ShaderNodeTexNoise" as "TexNoise".
fn short(kind: &str) -> &str {
    kind.strip_prefix("ShaderNode").unwrap_or(kind)
}

fn fmt_numbers(v: &[f64]) -> String {
    let parts: Vec<String> = v.iter().map(|x| format!("{x:.3}")).collect();
    parts.join(", ")
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
