//! Node graphs for the Node Material layer (Nodes N1, NM-01..09,
//! specifications/nodes-n1-2026-11-11).
//!
//! A graph is kept in the shared plain-text shape every Operandi tool reads
//! and writes ("Node graph JSON v1", `tools/blender-export/NODE-GRAPH-JSON.md`
//! on the Blender materials branch): node names, Blender node idnames,
//! their unlinked input values, props, links and exposed inputs. The scene
//! stores graphs in that same shape (NM-03) inside the shader tree's Nodes
//! section, so an import and a saved scene hold the same text. Anything this
//! build doesn't know (fields, props, node types) is kept as it was read.
//!
//! Rendering doesn't read Blender idnames: [`StoredGraph::flatten`] turns a
//! graph into Cycles nodes (type names and socket names from Cycles' own
//! node definitions, Apache-2.0), with groups, reroutes and frames resolved
//! away, ready for the C bridge's general node list (NM-30).
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The `format` string of a node graph file.
pub const FORMAT: &str = "operandi-node-graph";
/// The format version this build reads and writes.
pub const FORMAT_VERSION: u32 = 1;
/// Most nodes in one graph, groups inlined (a sanity budget).
const MAX_NODES: usize = 4096;
/// Most text one stored MaterialX graph may hold (a sanity budget).
const MAX_MATERIALX_BYTES: usize = 16 * 1024 * 1024;
/// Most nested group levels.
const MAX_GROUP_DEPTH: usize = 16;
/// Cycles' colour ramp table size (Blender bakes ramps to the same size).
pub const RAMP_TABLE_SIZE: usize = 256;

/// A socket value's type, as the file spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocketType {
    #[serde(rename = "VALUE")]
    Value,
    #[serde(rename = "RGBA")]
    Rgba,
    #[serde(rename = "VECTOR")]
    Vector,
    #[serde(rename = "INT")]
    Int,
    #[serde(rename = "BOOLEAN")]
    Boolean,
    #[serde(rename = "SHADER")]
    Shader,
    #[serde(rename = "STRING")]
    String,
}

/// One unlinked input's value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SocketValue {
    #[serde(rename = "type")]
    pub kind: SocketType,
    #[serde(default)]
    pub value: Value,
}
impl SocketValue {
    /// The value as up to four numbers (a bool is 0 or 1).
    pub fn numbers(&self) -> Vec<f64> {
        numbers(&self.value)
    }
}

/// Up to four numbers from a JSON number, bool or array of numbers.
pub fn numbers(v: &Value) -> Vec<f64> {
    match v {
        Value::Number(n) => n.as_f64().into_iter().collect(),
        Value::Bool(b) => vec![f64::from(u8::from(*b))],
        Value::Array(a) => a.iter().filter_map(Value::as_f64).take(4).collect(),
        _ => vec![],
    }
}

/// An image a node reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphImage {
    /// Relative to the file it was read from when possible; absolute once
    /// imported into a scene (the scene may live elsewhere).
    pub path: String,
    /// Blender's colour space name: "sRGB", "Non-Color", "Linear Rec.709"...
    #[serde(default = "srgb")]
    pub colorspace: String,
    #[serde(default)]
    pub packed: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
fn srgb() -> String {
    "sRGB".into()
}

/// One node as the file holds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub name: String,
    /// Blender's node idname (`ShaderNode...`).
    #[serde(rename = "type")]
    pub idname: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<GraphImage>,
    #[serde(default)]
    pub inputs: BTreeMap<String, SocketValue>,
    /// Anything else the writer put on the node (location, outputs...).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl GraphNode {
    /// Where the graph view draws it: the file's `location` when it has one.
    pub fn location(&self) -> Option<[f64; 2]> {
        let v = numbers(self.extra.get("location")?);
        (v.len() >= 2).then(|| [v[0], v[1]])
    }
    /// The group a group node instances: `props.group` (format v1), or
    /// `props.node_tree`.
    pub fn group_name(&self) -> Option<&str> {
        ["group", "node_tree"]
            .iter()
            .find_map(|k| self.props.get(*k).and_then(Value::as_str))
    }
}

/// `[node, socket name]` or `[node, socket name, identifier]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketRef(pub Vec<String>);
impl SocketRef {
    pub fn new(node: &str, socket: &str) -> Self {
        Self(vec![node.into(), socket.into()])
    }
    pub fn node(&self) -> &str {
        self.0.first().map_or("", String::as_str)
    }
    pub fn name(&self) -> &str {
        self.0.get(1).map_or("", String::as_str)
    }
    /// The identifier when given, else the name: what tells two sockets of
    /// the same name apart.
    pub fn key(&self) -> &str {
        self.0.get(2).map_or(self.name(), String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphLink {
    pub from: SocketRef,
    pub to: SocketRef,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// An exposed input (NM-02): a named control feeding one node socket.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exposed {
    pub name: String,
    pub to: SocketRef,
    #[serde(rename = "type")]
    pub kind: SocketType,
    #[serde(default)]
    pub default: Value,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}
impl Exposed {
    /// The value kind the panel edits.
    pub fn value_kind(&self) -> ValueKind {
        match self.kind {
            SocketType::Rgba => ValueKind::Color,
            SocketType::Vector => ValueKind::Vector,
            _ => ValueKind::Float,
        }
    }
    /// The default as numbers of the right count (1 or 3).
    pub fn default_value(&self) -> Vec<f64> {
        self.value_kind().fit(&numbers(&self.default))
    }
}

/// What an exposed input holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Float,
    Color,
    Vector,
}
impl ValueKind {
    pub fn count(self) -> usize {
        match self {
            ValueKind::Float => 1,
            ValueKind::Color | ValueKind::Vector => 3,
        }
    }
    /// `v` cut or padded to this kind's count (padding repeats a single
    /// number, else 0).
    pub fn fit(self, v: &[f64]) -> Vec<f64> {
        let n = self.count();
        (0..n)
            .map(|k| match (v.get(k), v.len()) {
                (Some(x), _) => *x,
                (None, 1) => v[0],
                _ => 0.,
            })
            .collect()
    }
}

/// A material: one graph with its output node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphMaterial {
    pub name: String,
    /// The Material Output node the render reads.
    #[serde(default = "material_output")]
    pub output: String,
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub links: Vec<GraphLink>,
    #[serde(default)]
    pub exposed: Vec<Exposed>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups_used: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
fn material_output() -> String {
    "Material Output".into()
}

/// One socket of a group's interface: {name, identifier?, type, default}
/// (format v1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupSocket {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// The file may spell it as a socket type ("VALUE") or as Blender's
    /// socket idname ("NodeSocketFloat"); either reads.
    #[serde(
        rename = "type",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "group_socket_type"
    )]
    pub kind: Option<SocketType>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub default: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
fn group_socket_type<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<SocketType>, D::Error> {
    let Some(text) = Option::<String>::deserialize(d)? else {
        return Ok(None);
    };
    if let Ok(t) = serde_json::from_value::<SocketType>(Value::String(text.clone())) {
        return Ok(Some(t));
    }
    let base = text.strip_prefix("NodeSocket").unwrap_or(&text);
    Ok(Some(match base {
        b if b.starts_with("Float") => SocketType::Value,
        b if b.starts_with("Color") => SocketType::Rgba,
        b if b.starts_with("Vector") => SocketType::Vector,
        b if b.starts_with("Int") => SocketType::Int,
        b if b.starts_with("Bool") => SocketType::Boolean,
        b if b.starts_with("Shader") => SocketType::Shader,
        b if b.starts_with("String") => SocketType::String,
        _ => return Ok(None),
    }))
}

impl GroupSocket {
    fn key(&self) -> &str {
        self.identifier.as_deref().unwrap_or(&self.name)
    }
}

/// A node group (a graph used inside other graphs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphGroup {
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub links: Vec<GraphLink>,
    #[serde(default)]
    pub inputs: Vec<GroupSocket>,
    #[serde(default)]
    pub outputs: Vec<GroupSocket>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A whole node graph file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphFile {
    pub format: String,
    pub format_version: u32,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub source: Value,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub licence: Value,
    pub materials: Vec<GraphMaterial>,
    #[serde(default)]
    pub groups: Vec<GraphGroup>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One graph in the shader tree's Nodes section: a material with the
/// groups it uses, under a scene id that Node Material layers point at.
/// Several layers may share one graph (NM-04).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredGraph {
    pub id: u64,
    pub material: GraphMaterial,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GraphGroup>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub source: Value,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub licence: Value,
    /// A graph kept as MaterialX instead (materials architecture, slice
    /// 1): `material` then holds only its name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialx: Option<MaterialXGraph>,
}

/// One MaterialX material kept in the scene as MaterialX text
/// (docs/MATERIALX_GRAPHS.md): the material's own document, written by
/// `materialx::write::material`, rendered through
/// `materialx::cycles::translate`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialXGraph {
    /// The `surfacematerial` the text holds.
    pub material: String,
    /// The folder relative picture paths in the text resolve against.
    pub folder: String,
    pub text: String,
    /// What the translator refused, worked out once.
    #[serde(skip)]
    checked: std::sync::OnceLock<Vec<(String, String)>>,
}
impl PartialEq for MaterialXGraph {
    fn eq(&self, other: &Self) -> bool {
        self.material == other.material && self.folder == other.folder && self.text == other.text
    }
}
impl MaterialXGraph {
    pub fn new(material: &str, folder: &std::path::Path, text: String) -> Self {
        Self {
            material: material.to_string(),
            folder: folder.to_string_lossy().into_owned(),
            text,
            checked: Default::default(),
        }
    }

    pub fn document(&self) -> crate::Result<crate::materialx::doc::Document> {
        crate::materialx::doc::parse(&self.text)
    }

    /// The material as Cycles nodes.
    pub fn translate(&self) -> crate::Result<crate::materialx::cycles::Translated> {
        let doc = self.document()?;
        crate::materialx::cycles::translate(
            &doc,
            Some(&self.material),
            std::path::Path::new(&self.folder),
        )
        .map_err(|r| format!("{}: {r}", self.material))
    }

    /// The node it stops at and why (empty when it renders).
    pub fn unsupported(&self) -> &[(String, String)] {
        self.checked.get_or_init(|| match self.document() {
            Err(e) => vec![(self.material.clone(), e)],
            Ok(doc) => match crate::materialx::cycles::translate(
                &doc,
                Some(&self.material),
                std::path::Path::new(&self.folder),
            ) {
                Ok(_) => Vec::new(),
                Err(r) if r.node.is_empty() => vec![(self.material.clone(), r.reason)],
                Err(r) => vec![(r.node, r.category)],
            },
        })
    }

    /// How many MaterialX nodes the text holds (nodegraphs' contents
    /// counted, definitions not).
    pub fn node_count(&self) -> usize {
        fn count(els: &[crate::materialx::doc::Element]) -> usize {
            els.iter()
                .filter(|e| !matches!(e.category.as_str(), "nodedef" | "surfacematerial"))
                .map(|e| {
                    if e.category == "nodegraph" {
                        count(&e.children)
                    } else {
                        1
                    }
                })
                .sum()
        }
        self.document().map_or(0, |d| count(&d.elements))
    }
}

/// The shader tree's Nodes section (NM-03, NM-04): its own version number,
/// so the graph text can change apart from the scene format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeSection {
    pub format_version: u32,
    pub graphs: Vec<StoredGraph>,
}
impl Default for NodeSection {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            graphs: Vec::new(),
        }
    }
}
impl NodeSection {
    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }
    pub fn get(&self, id: u64) -> Option<&StoredGraph> {
        self.graphs.iter().find(|g| g.id == id)
    }
    pub fn get_mut(&mut self, id: u64) -> Option<&mut StoredGraph> {
        self.graphs.iter_mut().find(|g| g.id == id)
    }
    pub fn next_id(&self) -> u64 {
        self.graphs.iter().map(|g| g.id).max().unwrap_or(0) + 1
    }
    pub(crate) fn validate(&self) -> crate::Result<()> {
        if self.format_version == 0 || self.format_version > FORMAT_VERSION {
            return Err(format!(
                "Node graphs use format version {}; this build reads up to {FORMAT_VERSION}",
                self.format_version
            ));
        }
        let mut ids = std::collections::BTreeSet::new();
        for g in &self.graphs {
            if !ids.insert(g.id) {
                return Err(format!("Two node graphs share id {}", g.id));
            }
            if g.material.nodes.len() > MAX_NODES {
                return Err(format!("{}: too many nodes", g.material.name));
            }
            if g.materialx
                .as_ref()
                .is_some_and(|m| m.text.len() > MAX_MATERIALX_BYTES)
            {
                return Err(format!("{}: MaterialX text too large", g.material.name));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The node set (NM-20..29).

/// What a node is for the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// A Cycles node of this type name.
    Cycles(&'static str),
    /// The Material Output.
    Output,
    /// Resolved away before rendering: Group Input/Output, group nodes,
    /// reroutes.
    PassThrough,
    /// Draws in the graph view only (frames).
    Layout,
}

/// The N1 node set: Blender idname, label shown in messages, role. Plus
/// layout-only nodes (frames, reroutes) any real graph has.
const NODE_SET: &[(&str, &str, Role)] = &[
    // Shaders
    (
        "ShaderNodeBsdfPrincipled",
        "Principled BSDF",
        Role::Cycles("principled_bsdf"),
    ),
    (
        "ShaderNodeMixShader",
        "Mix Shader",
        Role::Cycles("mix_closure"),
    ),
    ("ShaderNodeEmission", "Emission", Role::Cycles("emission")),
    (
        "ShaderNodeBsdfTransparent",
        "Transparent BSDF",
        Role::Cycles("transparent_bsdf"),
    ),
    (
        "ShaderNodeBsdfGlass",
        "Glass BSDF",
        Role::Cycles("glass_bsdf"),
    ),
    // Textures
    (
        "ShaderNodeTexImage",
        "Image Texture",
        Role::Cycles("image_texture"),
    ),
    (
        "ShaderNodeTexNoise",
        "Noise Texture",
        Role::Cycles("noise_texture"),
    ),
    (
        "ShaderNodeTexVoronoi",
        "Voronoi Texture",
        Role::Cycles("voronoi_texture"),
    ),
    (
        "ShaderNodeTexWave",
        "Wave Texture",
        Role::Cycles("wave_texture"),
    ),
    (
        "ShaderNodeTexGradient",
        "Gradient Texture",
        Role::Cycles("gradient_texture"),
    ),
    // Input
    (
        "ShaderNodeTexCoord",
        "Texture Coordinate",
        Role::Cycles("texture_coordinate"),
    ),
    ("ShaderNodeMapping", "Mapping", Role::Cycles("mapping")),
    (
        "ShaderNodeLayerWeight",
        "Layer Weight",
        Role::Cycles("layer_weight"),
    ),
    ("ShaderNodeFresnel", "Fresnel", Role::Cycles("fresnel")),
    ("ShaderNodeRGB", "RGB", Role::Cycles("color")),
    ("ShaderNodeValue", "Value", Role::Cycles("value")),
    // Colour / convert. Mix (data_type picks the Cycles node) and the
    // older MixRGB both count as Mix Color.
    ("ShaderNodeMix", "Mix Color", Role::Cycles("mix_color")),
    ("ShaderNodeMixRGB", "Mix Color", Role::Cycles("mix")),
    ("ShaderNodeValToRGB", "Color Ramp", Role::Cycles("rgb_ramp")),
    ("ShaderNodeMath", "Math", Role::Cycles("math")),
    (
        "ShaderNodeSeparateColor",
        "Separate Color",
        Role::Cycles("separate_color"),
    ),
    (
        "ShaderNodeCombineColor",
        "Combine Color",
        Role::Cycles("combine_color"),
    ),
    ("ShaderNodeInvert", "Invert", Role::Cycles("invert")),
    // Vector
    (
        "ShaderNodeNormalMap",
        "Normal Map",
        Role::Cycles("normal_map"),
    ),
    ("ShaderNodeBump", "Bump", Role::Cycles("bump")),
    (
        "ShaderNodeDisplacement",
        "Displacement",
        Role::Cycles("displacement"),
    ),
    // Groups and output
    ("NodeGroupInput", "Group Input", Role::PassThrough),
    ("NodeGroupOutput", "Group Output", Role::PassThrough),
    ("ShaderNodeGroup", "Group", Role::PassThrough),
    ("ShaderNodeOutputMaterial", "Material Output", Role::Output),
    // Layout only (not render nodes).
    ("NodeReroute", "Reroute", Role::PassThrough),
    ("NodeFrame", "Frame", Role::Layout),
];

fn role(idname: &str) -> Option<Role> {
    NODE_SET
        .iter()
        .find(|(id, _, _)| *id == idname)
        .map(|(_, _, r)| *r)
}

/// The label of a known node type ("Noise Texture"), or the idname.
pub fn type_label(idname: &str) -> &str {
    NODE_SET
        .iter()
        .find(|(id, _, _)| *id == idname)
        .map_or(idname, |(_, label, _)| label)
}

/// Whether this build renders a node of this idname.
pub fn is_supported(idname: &str) -> bool {
    role(idname).is_some()
}

impl GraphMaterial {
    /// The nodes this build can't render, as (node name, idname).
    pub fn unsupported<'a>(&'a self, groups: &'a [GraphGroup]) -> Vec<(&'a str, &'a str)> {
        let mut out: Vec<(&str, &str)> = self
            .nodes
            .iter()
            .filter(|n| !is_supported(&n.idname))
            .map(|n| (n.name.as_str(), n.idname.as_str()))
            .collect();
        for g in groups {
            out.extend(
                g.nodes
                    .iter()
                    .filter(|n| !is_supported(&n.idname))
                    .map(|n| (n.name.as_str(), n.idname.as_str())),
            );
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Import (NM-05).

/// Width and height of a PNG or JPEG from its header, without decoding
/// (a headless import lists a graph's pictures in Clips with their size).
pub fn picture_size(path: &std::path::Path) -> crate::Result<(u32, u32)> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let be16 = |i: usize| u32::from(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
    if bytes.len() >= 24 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let be32 =
            |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        return Ok((be32(16), be32(20)));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = bytes[i + 1];
            // Start of Frame markers carry the size; C4, C8 and CC don't.
            if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
                return Ok((be16(i + 7), be16(i + 5)));
            }
            i += 2 + be16(i + 2) as usize;
        }
    }
    Err(format!(
        "{}: image needs a PNG or JPEG file",
        path.display()
    ))
}

/// Reads a node graph file. A material using a node outside the N1 node
/// set is refused whole, with a message naming the node (NM-05, NM-T4);
/// the file's other materials still import. `base` is the folder relative
/// image paths are read against; they come back absolute.
pub fn import(text: &str, base: Option<&std::path::Path>) -> crate::Result<Vec<StoredGraph>> {
    import_report(text, base).map(|(graphs, _)| graphs)
}

/// [`import`], with a message for each material it refused. An error when
/// it refuses them all (nothing to import).
pub fn import_report(
    text: &str,
    base: Option<&std::path::Path>,
) -> crate::Result<(Vec<StoredGraph>, Vec<String>)> {
    let file: GraphFile =
        serde_json::from_str(text).map_err(|e| format!("Not a node graph file: {e}"))?;
    if file.format != FORMAT {
        return Err(format!(
            "Not a node graph file (format \"{}\", expected \"{FORMAT}\")",
            file.format
        ));
    }
    if file.format_version == 0 || file.format_version > FORMAT_VERSION {
        return Err(format!(
            "Node graph format version {} is newer than this build reads ({FORMAT_VERSION})",
            file.format_version
        ));
    }
    if file.materials.is_empty() {
        return Err("The node graph file holds no materials".into());
    }
    let mut out = Vec::new();
    let mut refused = Vec::new();
    for m in &file.materials {
        match import_material(m, &file, base) {
            Ok(g) => out.push(g),
            Err(e) => refused.push(e),
        }
    }
    if out.is_empty() {
        let mut message = refused.join(" ");
        message.push_str(" Nothing was imported.");
        return Err(message);
    }
    Ok((out, refused))
}

fn import_material(
    m: &GraphMaterial,
    file: &GraphFile,
    base: Option<&std::path::Path>,
) -> crate::Result<StoredGraph> {
    let groups = groups_for(m, &file.groups)?;
    if let Some((node, idname)) = m.unsupported(&groups).first() {
        return Err(format!(
            "Can't import “{}”: node “{node}” is a {idname}, which Node Materials \
             can't render yet.",
            m.name
        ));
    }
    let mut m = m.clone();
    let mut groups = groups;
    if let Some(base) = base {
        for n in m
            .nodes
            .iter_mut()
            .chain(groups.iter_mut().flat_map(|g| g.nodes.iter_mut()))
        {
            if let Some(img) = &mut n.image {
                let p = std::path::Path::new(&img.path);
                if p.is_relative() && !img.path.is_empty() {
                    img.path = base.join(p).to_string_lossy().into_owned();
                }
            }
        }
    }
    let stored = StoredGraph {
        id: 0,
        material: m,
        groups,
        source: file.source.clone(),
        licence: file.licence.clone(),
        materialx: None,
    };
    // Catches broken links, loops and missing outputs before the scene
    // changes.
    stored.flatten(&Default::default())?;
    Ok(stored)
}

/// The groups a material uses, nested ones included, from the file's list.
fn groups_for(m: &GraphMaterial, all: &[GraphGroup]) -> crate::Result<Vec<GraphGroup>> {
    let mut want: Vec<String> = m.groups_used.clone();
    want.extend(
        m.nodes
            .iter()
            .filter_map(|n| n.group_name().map(String::from)),
    );
    let mut out: Vec<GraphGroup> = Vec::new();
    while let Some(name) = want.pop() {
        if out.iter().any(|g| g.name == name) {
            continue;
        }
        let g = all.iter().find(|g| g.name == name).ok_or_else(|| {
            format!(
                "“{}” uses node group “{name}”, which the file doesn't hold",
                m.name
            )
        })?;
        want.extend(
            g.nodes
                .iter()
                .filter_map(|n| n.group_name().map(String::from)),
        );
        out.push(g.clone());
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Flattening for the renderer (NM-30).

/// A value for one Cycles socket.
#[derive(Clone, Debug, PartialEq)]
pub enum FlatValue {
    Float(f64),
    Vec3([f64; 3]),
    Int(i64),
    Bool(bool),
    /// A string or enum socket.
    Text(String),
    /// A colour ramp table: `RAMP_TABLE_SIZE` RGBA entries (sets the
    /// "Ramp" and "Ramp Alpha" sockets).
    Ramp(Vec<[f64; 4]>),
    /// Fed by a shader tree layer above: index into the caller's drivers.
    Driven(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlatNode {
    /// Cycles node type name, or "output" for the Material Output.
    pub cycles: &'static str,
    /// The node's name (inside groups: "Group/Node").
    pub name: String,
    pub values: Vec<(String, FlatValue)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatLink {
    pub from: usize,
    pub from_socket: String,
    pub to: usize,
    pub to_socket: String,
}

/// A graph ready for Cycles: real nodes only, Cycles socket names.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlatGraph {
    pub nodes: Vec<FlatNode>,
    pub links: Vec<FlatLink>,
}
impl FlatGraph {
    /// The output node's index (always present after a flatten).
    pub fn output(&self) -> Option<usize> {
        self.nodes.iter().position(|n| n.cycles == "output")
    }
    /// What feeds the Material Output's `socket` ("Surface",
    /// "Displacement"), as (node, Cycles output socket).
    pub fn feeding(&self, socket: &str) -> Option<(usize, &str)> {
        let out = self.output()?;
        self.links
            .iter()
            .find(|l| l.to == out && l.to_socket == socket)
            .map(|l| (l.from, l.from_socket.as_str()))
    }
    /// A node's constant input value.
    pub fn value(&self, node: usize, socket: &str) -> Option<&FlatValue> {
        self.nodes
            .get(node)?
            .values
            .iter()
            .find(|(s, _)| s == socket)
            .map(|(_, v)| v)
    }
    /// Whether `socket` of `node` is linked.
    pub fn is_linked(&self, node: usize, socket: &str) -> bool {
        self.links
            .iter()
            .any(|l| l.to == node && l.to_socket == socket)
    }
}

/// Values for the exposed inputs when flattening: each by exposed name,
/// numbers (an override or a new default) or a driver index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExposedValues {
    pub values: BTreeMap<String, Vec<f64>>,
    pub drivers: BTreeMap<String, usize>,
}

/// One node during flattening: its full name, its definition and, for a
/// node inside a group, the group node it sits in.
struct Raw<'a> {
    name: String,
    node: &'a GraphNode,
    /// Index of the enclosing group node in `raws`, if any.
    parent: Option<usize>,
    /// For a group node: the full-name prefix of its contents.
    inner: Option<String>,
    /// For a group node: its group definition.
    group: Option<&'a GraphGroup>,
}

/// Where an input gets its value.
enum Source {
    Out(usize, String),
    Value(SocketValue),
    None,
}

impl StoredGraph {
    /// The exposed input called `name`.
    pub fn exposed(&self, name: &str) -> Option<&Exposed> {
        self.material.exposed.iter().find(|e| e.name == name)
    }

    /// The nodes this graph has that this build can't render (NM-03: kept
    /// and flagged).
    pub fn unsupported(&self) -> Vec<(String, String)> {
        if let Some(mx) = &self.materialx {
            return mx.unsupported().to_vec();
        }
        self.material
            .unsupported(&self.groups)
            .into_iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    /// Text identifying the graph's content, for caches (NM-40).
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let text = serde_json::to_string(&(&self.material, &self.groups, &self.materialx))
            .unwrap_or_default();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut h);
        h.finish()
    }

    /// The graph as Cycles nodes, with `exposed` applied.
    pub fn flatten(&self, exposed: &ExposedValues) -> crate::Result<FlatGraph> {
        if let Some(mx) = &self.materialx {
            return mx.translate().map(|t| t.graph);
        }
        let m = &self.material;
        if let Some((node, idname)) = self.unsupported().first() {
            return Err(format!(
                "{}: node “{node}” is a {idname}, which Node Materials can't render",
                m.name
            ));
        }
        // Every node, groups inlined.
        let mut raws: Vec<Raw> = Vec::new();
        self.collect(&m.nodes, "", None, 0, &mut raws)?;
        if raws.len() > MAX_NODES {
            return Err(format!("{}: too many nodes", m.name));
        }
        let find = |name: &str| raws.iter().position(|r| r.name == name);
        // Links by consumer (full names).
        let mut links: BTreeMap<(String, String), (String, String)> = BTreeMap::new();
        let mut add_links = |list: &[GraphLink], prefix: &str| -> crate::Result<()> {
            for l in list {
                let from = format!("{prefix}{}", l.from.node());
                let to = format!("{prefix}{}", l.to.node());
                links.insert((to, l.to.key().into()), (from, l.from.key().into()));
            }
            Ok(())
        };
        add_links(&m.links, "")?;
        for r in &raws {
            if let (Some(g), Some(inner)) = (r.group, &r.inner) {
                add_links(&g.links, inner)?;
            }
        }
        for ((to, _), (from, _)) in &links {
            if find(to).is_none() || find(from).is_none() {
                return Err(format!(
                    "{}: a link names a node that isn't there (“{}”)",
                    m.name,
                    if find(to).is_none() { to } else { from }
                ));
            }
        }
        // The output node.
        let output = find(&m.output)
            .filter(|&i| role(&raws[i].node.idname) == Some(Role::Output))
            .or_else(|| {
                raws.iter()
                    .position(|r| r.parent.is_none() && role(&r.node.idname) == Some(Role::Output))
            })
            .ok_or_else(|| format!("{}: no Material Output node", m.name))?;

        // Exposed sockets, by (full node name, socket key).
        let mut exposed_at: BTreeMap<(String, String), FlatValue> = BTreeMap::new();
        for e in &m.exposed {
            let at = (e.to.node().to_string(), e.to.key().to_string());
            let v = if let Some(&d) = exposed.drivers.get(&e.name) {
                FlatValue::Driven(d)
            } else {
                let n = exposed
                    .values
                    .get(&e.name)
                    .cloned()
                    .unwrap_or_else(|| e.default_value());
                match e.value_kind() {
                    ValueKind::Float => FlatValue::Float(n.first().copied().unwrap_or(0.)),
                    _ => {
                        let n = ValueKind::Color.fit(&n);
                        FlatValue::Vec3([n[0], n[1], n[2]])
                    }
                }
            };
            exposed_at.insert(at, v);
        }

        // Resolve a consumer input through pass-through nodes.
        let resolve = |node: usize, key: &str| -> Source {
            let mut at = (node, key.to_string());
            for _ in 0..=raws.len() * 2 + 2 {
                let r = &raws[at.0];
                let Some((from, fkey)) = links.get(&(r.name.clone(), at.1.clone())) else {
                    // Unlinked: the node's own value, if any.
                    return r
                        .node
                        .inputs
                        .get(&at.1)
                        .or_else(|| {
                            r.node
                                .inputs
                                .iter()
                                .find(|(k, _)| socket_base(k) == socket_base(&at.1))
                                .map(|(_, v)| v)
                        })
                        .cloned()
                        .map_or(Source::None, Source::Value);
                };
                let Some(fi) = find(from) else {
                    return Source::None;
                };
                let src = &raws[fi];
                match role(&src.node.idname) {
                    Some(Role::Cycles(_)) => return Source::Out(fi, fkey.clone()),
                    Some(Role::PassThrough) => match src.node.idname.as_str() {
                        "NodeReroute" => at = (fi, "Input".into()),
                        // A group node's output: what its Group Output receives.
                        "ShaderNodeGroup" => {
                            let Some(go) = raws.iter().position(|x| {
                                x.parent == Some(fi) && x.node.idname == "NodeGroupOutput"
                            }) else {
                                return Source::None;
                            };
                            at = (go, fkey.clone());
                        }
                        // A Group Input's socket: the group node's input.
                        "NodeGroupInput" => {
                            let Some(g) = src.parent else {
                                return Source::None;
                            };
                            let fallback = raws[g].group.and_then(|grp| {
                                grp.inputs.iter().find(|s| s.key() == fkey).and_then(|s| {
                                    s.kind.map(|kind| SocketValue {
                                        kind,
                                        value: s.default.clone(),
                                    })
                                })
                            });
                            if !links.contains_key(&(raws[g].name.clone(), fkey.clone()))
                                && !raws[g].node.inputs.contains_key(fkey)
                            {
                                return fallback.map_or(Source::None, Source::Value);
                            }
                            at = (g, fkey.clone());
                        }
                        _ => return Source::None,
                    },
                    _ => return Source::None,
                }
            }
            Source::None
        };

        // Build the flat graph.
        let mut flat = FlatGraph::default();
        let mut index: Vec<Option<usize>> = vec![None; raws.len()];
        for (i, r) in raws.iter().enumerate() {
            let cycles = match role(&r.node.idname) {
                Some(Role::Cycles(c)) => cycles_type(r.node, c),
                Some(Role::Output) if i == output => "output",
                _ => continue,
            };
            index[i] = Some(flat.nodes.len());
            flat.nodes.push(FlatNode {
                cycles,
                name: r.name.clone(),
                values: props(r.node, cycles),
            });
        }
        for (i, r) in raws.iter().enumerate() {
            let Some(to) = index[i] else { continue };
            // Every socket this node has a link or a value for.
            let mut keys: Vec<String> = r.node.inputs.keys().cloned().collect();
            for (to_name, key) in links.keys() {
                if *to_name == r.name && !keys.contains(key) {
                    keys.push(key.clone());
                }
            }
            for key in keys {
                let cycles = flat.nodes[to].cycles;
                let Some(socket) = cycles_input(&r.node.idname, cycles, &key) else {
                    continue;
                };
                if let Some(v) = exposed_at.get(&(r.name.clone(), key.clone())) {
                    flat.nodes[to].values.retain(|(s, _)| *s != socket);
                    flat.nodes[to].values.push((socket, v.clone()));
                    continue;
                }
                match resolve(i, &key) {
                    Source::Out(from, fkey) => {
                        let Some(from_flat) = index[from] else {
                            continue;
                        };
                        let fnode = raws[from].node;
                        let from_socket =
                            cycles_output(&fnode.idname, flat.nodes[from_flat].cycles, &fkey);
                        flat.links.push(FlatLink {
                            from: from_flat,
                            from_socket,
                            to,
                            to_socket: socket,
                        });
                    }
                    Source::Value(v) => {
                        if let Some(fv) = flat_value(&v) {
                            flat.nodes[to].values.retain(|(s, _)| *s != socket);
                            flat.nodes[to].values.push((socket, fv));
                        }
                    }
                    Source::None => {}
                }
            }
        }
        // Exposed inputs whose socket had neither link nor value.
        for ((node, key), v) in &exposed_at {
            let Some(i) = find(node) else {
                return Err(format!(
                    "{}: exposed input names a missing node “{node}”",
                    m.name
                ));
            };
            let Some(to) = index[i] else { continue };
            if let Some(socket) = cycles_input(&raws[i].node.idname, flat.nodes[to].cycles, key)
                && !flat.nodes[to].values.iter().any(|(s, _)| *s == socket)
            {
                flat.links
                    .retain(|l| !(l.to == to && l.to_socket == socket));
                flat.nodes[to].values.push((socket, v.clone()));
            }
        }
        // A driven socket takes no link from the graph.
        let driven: Vec<(usize, String)> = flat
            .nodes
            .iter()
            .enumerate()
            .flat_map(|(i, n)| {
                n.values
                    .iter()
                    .filter(|(_, v)| matches!(v, FlatValue::Driven(_)))
                    .map(move |(s, _)| (i, s.clone()))
            })
            .collect();
        flat.links
            .retain(|l| !driven.iter().any(|(i, s)| *i == l.to && *s == l.to_socket));
        check_acyclic(&flat).map_err(|e| format!("{}: {e}", m.name))?;
        if flat.feeding("Surface").is_none() && flat.feeding("Displacement").is_none() {
            return Err(format!(
                "{}: nothing is connected to the Material Output",
                m.name
            ));
        }
        Ok(flat)
    }

    /// Adds `nodes` (in a group when `parent` is set) and, recursively, the
    /// contents of their group nodes.
    fn collect<'a>(
        &'a self,
        nodes: &'a [GraphNode],
        prefix: &str,
        parent: Option<usize>,
        depth: usize,
        raws: &mut Vec<Raw<'a>>,
    ) -> crate::Result<()> {
        if depth > MAX_GROUP_DEPTH {
            return Err(format!(
                "{}: node groups nest too deeply",
                self.material.name
            ));
        }
        for n in nodes {
            let name = format!("{prefix}{}", n.name);
            if raws.iter().any(|r| r.name == name) {
                return Err(format!(
                    "{}: two nodes are called “{name}”",
                    self.material.name
                ));
            }
            let i = raws.len();
            let group = if n.idname == "ShaderNodeGroup" {
                let g = n.group_name().ok_or_else(|| {
                    format!("{}: group node “{name}” names no group", self.material.name)
                })?;
                Some(self.groups.iter().find(|x| x.name == g).ok_or_else(|| {
                    format!("{}: node group “{g}” is missing", self.material.name)
                })?)
            } else {
                None
            };
            let inner = group.map(|_| format!("{name}/"));
            raws.push(Raw {
                name,
                node: n,
                parent,
                inner: inner.clone(),
                group,
            });
            if let (Some(g), Some(inner)) = (group, inner) {
                self.collect(&g.nodes, &inner, Some(i), depth + 1, raws)?;
            }
        }
        Ok(())
    }
}

/// "Value_001" -> "Value"; "A_Color" -> "A".
fn socket_base(key: &str) -> &str {
    key.split('_').next().unwrap_or(key)
}

/// The Cycles node a Blender node becomes, when its props choose.
fn cycles_type(n: &GraphNode, default: &'static str) -> &'static str {
    if n.idname == "ShaderNodeMix" {
        return match n.props.get("data_type").and_then(Value::as_str) {
            Some("FLOAT") => "mix_float",
            Some("VECTOR") => match n.props.get("factor_mode").and_then(Value::as_str) {
                Some("NON_UNIFORM") => "mix_vector_non_uniform",
                _ => "mix_vector",
            },
            _ => "mix_color",
        };
    }
    default
}

/// The Cycles input socket for a Blender input key, or `None` to skip it.
fn cycles_input(idname: &str, cycles: &str, key: &str) -> Option<String> {
    let base = socket_base(key);
    Some(
        match (idname, key) {
            ("ShaderNodeMath", "Value") => "Value1",
            ("ShaderNodeMath", "Value_001") => "Value2",
            ("ShaderNodeMath", "Value_002") => "Value3",
            ("ShaderNodeMixShader", "Shader") => "Closure1",
            ("ShaderNodeMixShader", "Shader_001") => "Closure2",
            ("ShaderNodeMix", _) => {
                // Only the sockets of the node's data type count.
                let want = match cycles {
                    "mix_float" => "Float",
                    "mix_color" => "Color",
                    _ => "Vector",
                };
                if let Some((_, ty)) = key.split_once('_') {
                    let ok = ty == want
                        || (base == "Factor"
                            && ty == "Float"
                            && cycles != "mix_vector_non_uniform");
                    if !ok
                        && !(base == "Factor"
                            && ty == "Vector"
                            && cycles == "mix_vector_non_uniform")
                    {
                        return None;
                    }
                }
                base
            }
            // Blender 3.x principled names (renamed in 4.0).
            ("ShaderNodeBsdfPrincipled", "Emission") => "Emission Color",
            ("ShaderNodeBsdfPrincipled", "Subsurface") => "Subsurface Weight",
            ("ShaderNodeBsdfPrincipled", "Specular") => "Specular IOR Level",
            ("ShaderNodeBsdfPrincipled", "Transmission") => "Transmission Weight",
            ("ShaderNodeBsdfPrincipled", "Clearcoat") => "Coat Weight",
            ("ShaderNodeBsdfPrincipled", "Clearcoat Roughness") => "Coat Roughness",
            ("ShaderNodeBsdfPrincipled", "Clearcoat Normal") => "Coat Normal",
            ("ShaderNodeBsdfPrincipled", "Sheen") => "Sheen Weight",
            // Blender's hidden closure weight; Cycles sets its own.
            (_, "Weight") => return None,
            _ => key,
        }
        .to_string(),
    )
}

/// The Cycles output socket for a Blender output key.
fn cycles_output(idname: &str, cycles: &str, key: &str) -> String {
    match (idname, key) {
        ("ShaderNodeMixShader", _) => "Closure".into(),
        ("ShaderNodeMix", _) => "Result".into(),
        ("ShaderNodeRGB", _) if cycles == "color" => "Color".into(),
        _ => key.to_string(),
    }
}

/// A file value as a Cycles socket value (shaders have none).
fn flat_value(v: &SocketValue) -> Option<FlatValue> {
    let n = v.numbers();
    Some(match v.kind {
        SocketType::Value => FlatValue::Float(*n.first()?),
        SocketType::Rgba | SocketType::Vector => {
            let n = ValueKind::Color.fit(&n);
            FlatValue::Vec3([n[0], n[1], n[2]])
        }
        SocketType::Int => FlatValue::Int(*n.first()? as i64),
        SocketType::Boolean => FlatValue::Bool(n.first().is_some_and(|x| *x != 0.)),
        SocketType::String => FlatValue::Text(v.value.as_str()?.to_string()),
        SocketType::Shader => return None,
    })
}

fn prop_str<'a>(n: &'a GraphNode, key: &str) -> Option<&'a str> {
    n.props.get(key).and_then(Value::as_str)
}
fn prop_bool(n: &GraphNode, key: &str) -> Option<bool> {
    n.props.get(key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        Value::Number(x) => x.as_f64().map(|x| x != 0.),
        _ => None,
    })
}

/// Blender's enum identifier as Cycles spells it: lower case, with the
/// handful of names that differ.
fn cycles_enum(blender: &str) -> String {
    match blender {
        "FBM" => "fBM".into(),
        "SIN" => "sine".into(),
        "INVERSE_SQRT" => "inversesqrt".into(),
        "SMOOTH_MIN" => "smoothmin".into(),
        "SMOOTH_MAX" => "smoothmax".into(),
        "PINGPONG" => "pingpong".into(),
        "MIX" => "mix".into(),
        "REPEAT" => "periodic".into(),
        "EXTEND" => "clamp".into(),
        "CLIP" => "black".into(),
        other => other.to_ascii_lowercase(),
    }
}

/// Settings (Blender props) as Cycles socket values.
fn props(n: &GraphNode, cycles: &str) -> Vec<(String, FlatValue)> {
    let mut out: Vec<(String, FlatValue)> = Vec::new();
    let mut put = |socket: &str, v: FlatValue| out.push((socket.to_string(), v));
    let e = |key: &str| prop_str(n, key).map(|s| FlatValue::Text(cycles_enum(s)));
    let dims = |key: &str| {
        prop_str(n, key)
            .and_then(|s| s.trim_end_matches('D').parse::<i64>().ok())
            .map(FlatValue::Int)
    };
    match n.idname.as_str() {
        "ShaderNodeBsdfPrincipled" => {
            if let Some(v) = e("distribution") {
                put("Distribution", v);
            }
            if let Some(v) = e("subsurface_method") {
                put("Subsurface Method", v);
            }
        }
        "ShaderNodeBsdfGlass" => {
            if let Some(v) = e("distribution") {
                put("Distribution", v);
            }
        }
        "ShaderNodeTexImage" => {
            if let Some(img) = &n.image {
                put("Filename", FlatValue::Text(img.path.clone()));
                put(
                    "Colorspace",
                    FlatValue::Text(cycles_colorspace(&img.colorspace).into()),
                );
            }
            if let Some(v) = e("interpolation") {
                put("Interpolation", v);
            }
            if let Some(v) = e("extension") {
                put("Extension", v);
            }
            if let Some(v) = e("projection") {
                put("Projection", v);
            }
            if let Some(b) = n.props.get("projection_blend").and_then(Value::as_f64) {
                put("Projection Blend", FlatValue::Float(b));
            }
        }
        "ShaderNodeTexNoise" => {
            if let Some(v) = dims("noise_dimensions") {
                put("Dimensions", v);
            }
            if let Some(v) = e("noise_type") {
                put("Type", v);
            }
            if let Some(b) = prop_bool(n, "normalize") {
                put("Normalize", FlatValue::Bool(b));
            }
        }
        "ShaderNodeTexVoronoi" => {
            if let Some(v) = dims("voronoi_dimensions") {
                put("Dimensions", v);
            }
            if let Some(v) = e("feature") {
                put("Feature", v);
            }
            if let Some(v) = e("distance") {
                put("Distance Metric", v);
            }
            if let Some(b) = prop_bool(n, "normalize") {
                put("Normalize", FlatValue::Bool(b));
            }
        }
        "ShaderNodeTexWave" => {
            if let Some(v) = e("wave_type") {
                put("Type", v);
            }
            if let Some(v) = e("bands_direction") {
                put("Bands Direction", v);
            }
            if let Some(v) = e("rings_direction") {
                put("Rings Direction", v);
            }
            if let Some(v) = e("wave_profile") {
                put("Profile", v);
            }
        }
        "ShaderNodeTexGradient" => {
            if let Some(v) = e("gradient_type") {
                put("Type", v);
            }
        }
        "ShaderNodeMapping" => {
            if let Some(v) = e("vector_type") {
                put("Type", v);
            }
        }
        "ShaderNodeMath" => {
            if let Some(v) = e("operation") {
                put("Type", v);
            }
            if let Some(b) = prop_bool(n, "use_clamp") {
                put("Use Clamp", FlatValue::Bool(b));
            }
        }
        "ShaderNodeMix" => {
            if cycles == "mix_color"
                && let Some(v) = e("blend_type")
            {
                put("Type", v);
            }
            if let Some(b) = prop_bool(n, "clamp_factor") {
                put("Use Clamp", FlatValue::Bool(b));
            }
            if cycles == "mix_color"
                && let Some(b) = prop_bool(n, "clamp_result")
            {
                put("Use Clamp Result", FlatValue::Bool(b));
            }
        }
        "ShaderNodeMixRGB" => {
            if let Some(v) = e("blend_type") {
                put("Type", v);
            }
            if let Some(b) = prop_bool(n, "use_clamp") {
                put("Use Clamp", FlatValue::Bool(b));
            }
        }
        "ShaderNodeSeparateColor" | "ShaderNodeCombineColor" => {
            if let Some(v) = e("mode") {
                put("Type", v);
            }
        }
        "ShaderNodeNormalMap" => {
            if let Some(v) = e("space") {
                put("Space", v);
            }
            if let Some(uv) = prop_str(n, "uv_map").filter(|s| !s.is_empty()) {
                put("Attribute", FlatValue::Text(uv.into()));
            }
        }
        "ShaderNodeBump" => {
            if let Some(b) = prop_bool(n, "invert") {
                put("Invert", FlatValue::Bool(b));
            }
        }
        "ShaderNodeDisplacement" => {
            if let Some(v) = e("space") {
                put("Space", v);
            }
        }
        "ShaderNodeValToRGB" => {
            let (table, interpolate) = ramp_table(n);
            put("Ramp", FlatValue::Ramp(table));
            put("Interpolate", FlatValue::Bool(interpolate));
        }
        "ShaderNodeValue" => {
            let v = output_default(n, "Value").unwrap_or_default();
            put("Value", FlatValue::Float(v.first().copied().unwrap_or(0.)));
        }
        "ShaderNodeRGB" => {
            let v = ValueKind::Color.fit(&output_default(n, "Color").unwrap_or_default());
            put("Value", FlatValue::Vec3([v[0], v[1], v[2]]));
        }
        _ => {}
    }
    out
}

/// A Value or RGB node's number: `outputs.<socket>.value`, else
/// `props.value` / `props.color` (format v1).
fn output_default(n: &GraphNode, socket: &str) -> Option<Vec<f64>> {
    if let Some(v) = n
        .extra
        .get("outputs")
        .and_then(|o| o.get(socket))
        .and_then(|s| s.get("value").or(Some(s)))
    {
        let v = numbers(v);
        if !v.is_empty() {
            return Some(v);
        }
    }
    ["value", "color", "default_value"]
        .iter()
        .find_map(|k| n.props.get(*k).map(numbers).filter(|v| !v.is_empty()))
}

/// Blender colour space names as the Cycles bridge's colour spaces.
pub fn cycles_colorspace(blender: &str) -> &'static str {
    match blender {
        "Non-Color" | "Raw" | "Utility - Raw" | "data" => "data",
        "Linear" | "Linear Rec.709" | "Linear CIE-XYZ D65" | "scene_linear" => "scene_linear",
        _ => "__builtin_srgb",
    }
}

/// The ramp's stops (position, RGBA), sorted.
pub fn ramp_stops(n: &GraphNode) -> (Vec<(f64, [f64; 4])>, String) {
    let ramp = n.props.get("ramp");
    let interpolation = ramp
        .and_then(|r| r.get("interpolation"))
        .and_then(Value::as_str)
        .unwrap_or("LINEAR")
        .to_string();
    let mut stops: Vec<(f64, [f64; 4])> = ramp
        .and_then(|r| r.get("elements"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| {
                    let e = e.as_array()?;
                    let pos = e.first()?.as_f64()?;
                    let c = numbers(e.get(1)?);
                    let c = [
                        c.first().copied().unwrap_or(0.),
                        c.get(1).copied().unwrap_or(0.),
                        c.get(2).copied().unwrap_or(0.),
                        c.get(3).copied().unwrap_or(1.),
                    ];
                    Some((pos, c))
                })
                .collect()
        })
        .unwrap_or_default();
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    if stops.is_empty() {
        stops = vec![(0., [0., 0., 0., 1.]), (1., [1., 1., 1., 1.])];
    }
    (stops, interpolation)
}

/// The ramp baked to Cycles' table, and whether Cycles interpolates
/// between entries (not for Constant). Linear and Ease follow Blender's
/// colour band; Cardinal and B-Spline are read as Linear (provisional,
/// NM-Q04).
fn ramp_table(n: &GraphNode) -> (Vec<[f64; 4]>, bool) {
    let (stops, interpolation) = ramp_stops(n);
    let table = (0..RAMP_TABLE_SIZE)
        .map(|k| {
            ramp_eval(
                &stops,
                &interpolation,
                k as f64 / (RAMP_TABLE_SIZE - 1) as f64,
            )
        })
        .collect();
    (table, interpolation != "CONSTANT")
}

/// A colour band read at `t`.
pub fn ramp_eval(stops: &[(f64, [f64; 4])], interpolation: &str, t: f64) -> [f64; 4] {
    let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
        return [0., 0., 0., 1.];
    };
    if t <= first.0 {
        return first.1;
    }
    if t >= last.0 {
        return last.1;
    }
    let k = stops
        .iter()
        .position(|s| s.0 > t)
        .unwrap_or(stops.len() - 1);
    let (a, b) = (stops[k - 1], stops[k]);
    if interpolation == "CONSTANT" {
        return a.1;
    }
    let span = b.0 - a.0;
    let mut f = if span > 0. { (t - a.0) / span } else { 0. };
    if interpolation == "EASE" {
        f = f * f * (3. - 2. * f);
    }
    std::array::from_fn(|c| a.1[c] + (b.1[c] - a.1[c]) * f)
}

/// Refuses a graph whose links loop.
fn check_acyclic(g: &FlatGraph) -> Result<(), String> {
    let n = g.nodes.len();
    let mut state = vec![0u8; n];
    fn visit(i: usize, g: &FlatGraph, state: &mut [u8]) -> bool {
        match state[i] {
            1 => return false,
            2 => return true,
            _ => {}
        }
        state[i] = 1;
        for l in g.links.iter().filter(|l| l.to == i) {
            if !visit(l.from, g, state) {
                return false;
            }
        }
        state[i] = 2;
        true
    }
    for i in 0..n {
        if !visit(i, g, &mut state) {
            return Err("the node links form a loop".into());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "node_graph_tests.rs"]
mod tests;
