//! The shared node graph JSON, format "operandi-node-graph" version 1
//! (tools/blender-export/NODE-GRAPH-JSON.md). The Blender exporter writes
//! it; the material converter (preset/convert.rs) and the node material
//! builder read it. Only reading lives here (the Node Material layer keeps
//! graphs, and renders them, through `node_graph`, which holds the same
//! shape in full); unknown fields are kept out of
//! the way (`serde(default)`) so a newer exporter's extras don't break an
//! older reader of the same `format_version`.
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// The format name every file carries.
pub const FORMAT: &str = "operandi-node-graph";
/// The newest `format_version` this build reads.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphFile {
    pub format: String,
    pub format_version: u32,
    #[serde(default)]
    pub source: Option<GraphSource>,
    #[serde(default)]
    pub licence: Option<GraphLicence>,
    #[serde(default)]
    pub materials: Vec<GraphMaterial>,
    #[serde(default)]
    pub groups: Vec<GraphGroup>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphSource {
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub file: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphLicence {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub credit: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphMaterial {
    pub name: String,
    /// The Material Output node's name (`None`: the material has none).
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub links: Vec<GraphLink>,
    #[serde(default)]
    pub exposed: Vec<Value>,
    #[serde(default)]
    pub groups_used: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphGroup {
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub links: Vec<GraphLink>,
    #[serde(default)]
    pub inputs: Vec<InterfaceSocket>,
    #[serde(default)]
    pub outputs: Vec<InterfaceSocket>,
}

/// A group interface socket.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct InterfaceSocket {
    pub name: String,
    /// Only when it differs from `name`.
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub default: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphNode {
    pub name: String,
    /// Blender's idname, e.g. `ShaderNodeBsdfPrincipled`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub props: BTreeMap<String, Value>,
    #[serde(default)]
    pub image: Option<GraphImage>,
    /// Unlinked inputs only, keyed by socket name (or identifier where a
    /// name repeats on the node).
    #[serde(default)]
    pub inputs: BTreeMap<String, GraphInput>,
    /// Output values, written only where a node keeps its value on an
    /// output socket (Value: `"Value"`, RGB: `"Color"`).
    #[serde(default)]
    pub outputs: BTreeMap<String, GraphInput>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphImage {
    /// Relative to the JSON file when it could be.
    pub path: String,
    #[serde(default)]
    pub colorspace: String,
    #[serde(default)]
    pub packed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphInput {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub value: Value,
}

/// One end of a link: node name, socket name, and the socket identifier
/// when the name repeats on that node.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct SocketRef(pub Vec<String>);

impl SocketRef {
    pub fn node(&self) -> &str {
        self.0.first().map_or("", String::as_str)
    }
    pub fn socket(&self) -> &str {
        self.0.get(1).map_or("", String::as_str)
    }
    pub fn identifier(&self) -> Option<&str> {
        self.0.get(2).map(String::as_str)
    }
    /// Whether this end is socket `name` (or identifier `name`).
    pub fn is(&self, name: &str) -> bool {
        self.socket() == name || self.identifier() == Some(name)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct GraphLink {
    pub from: SocketRef,
    pub to: SocketRef,
}

impl GraphFile {
    /// Read a file's text, checking its format and version.
    pub fn parse(text: &str) -> crate::Result<GraphFile> {
        let file: GraphFile =
            serde_json::from_str(text).map_err(|e| format!("Invalid node graph JSON: {e}"))?;
        if file.format != FORMAT {
            return Err(format!("Not an {FORMAT} file (format \"{}\")", file.format));
        }
        if file.format_version == 0 || file.format_version > FORMAT_VERSION {
            return Err(format!(
                "Node graph format version {} is newer than this build reads ({FORMAT_VERSION})",
                file.format_version
            ));
        }
        Ok(file)
    }
}

impl GraphMaterial {
    pub fn node(&self, name: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.name == name)
    }
    /// The link into `node`'s input `socket`, if it is linked.
    pub fn link_into(&self, node: &str, socket: &str) -> Option<&GraphLink> {
        self.links
            .iter()
            .find(|l| l.to.node() == node && l.to.is(socket))
    }
}

impl GraphNode {
    /// An unlinked input's value as numbers (a scalar is one number).
    pub fn number(&self, socket: &str) -> Option<Vec<f64>> {
        numbers(&self.inputs.get(socket)?.value)
    }
    /// An unlinked scalar input.
    pub fn scalar(&self, socket: &str) -> Option<f64> {
        self.number(socket).and_then(|v| v.first().copied())
    }
    /// A Value or RGB node's number: `outputs` first, then the older
    /// `props.value` / `props.color`.
    pub fn output_value(&self) -> Option<Vec<f64>> {
        self.outputs
            .values()
            .find_map(|o| numbers(&o.value))
            .or_else(|| self.props.get("value").and_then(numbers))
            .or_else(|| self.props.get("color").and_then(numbers))
    }
    /// The group a Group node uses: `props.group`, then `props.node_tree`
    /// (files written another way).
    pub fn group_name(&self) -> Option<&str> {
        self.prop("group").or_else(|| self.prop("node_tree"))
    }
    /// A text property (`props`), e.g. `blend_type`.
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props.get(key).and_then(Value::as_str)
    }
}

/// A JSON number or list of numbers.
pub fn numbers(value: &Value) -> Option<Vec<f64>> {
    match value {
        Value::Number(n) => n.as_f64().map(|v| vec![v]),
        Value::Bool(b) => Some(vec![f64::from(u8::from(*b))]),
        Value::Array(items) => items.iter().map(Value::as_f64).collect(),
        _ => None,
    }
}
