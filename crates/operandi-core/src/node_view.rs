//! Node editing for materials (A3, NE-01..NE-19; docs/NODE_EDITING_PLAN.md).
//!
//! A Material Group's node view shows the boxes the Shader Tree makes (its
//! layers, a mixing node per stacking layer, the Material Output) and the
//! group's **free nodes**: standard MaterialX nodes added in the node view
//! (NE-02). Only the free nodes, their wires, and where every box sits are
//! stored ([`GroupNodes`], one per Material Group in
//! `ShaderTree::node_views`, native version 56). Layer boxes are made from
//! the layers each time, so the tree stays the one source of truth
//! (UIC-244).
//!
//! A wire from a free node's output into a material layer's channel drives
//! that channel (NE-03): the layer's own value is kept but stops applying
//! until the wire goes (`shader_eval`). Every change is a
//! [`NodeEdit`] run through `ShaderTreeEdit::Nodes`, one undo step each
//! (NE-15, UIC-224..227).
use crate::shader_eval::Ch;
use crate::shader_tree::{NodeId, NodeKind, ShaderTree};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Free nodes a Material Group may hold (a sanity budget).
pub const MAX_FREE_NODES: usize = 512;

/// The view name of the Material Output box.
pub const OUTPUT: &str = "output";

/// The view name of layer `id`'s own box.
pub fn layer_name(id: NodeId) -> String {
    format!("layer:{id}")
}
/// The view name of layer `id`'s mixing node.
pub fn mix_name(id: NodeId) -> String {
    format!("mix:{id}")
}
/// The layer a view name belongs to, and whether it is the mixing node.
pub fn layer_of(name: &str) -> Option<(NodeId, bool)> {
    if let Some(id) = name.strip_prefix("layer:") {
        return id.parse().ok().map(|id| (id, false));
    }
    name.strip_prefix("mix:")
        .and_then(|id| id.parse().ok())
        .map(|id| (id, true))
}

/// One Material Group's node view: what isn't a layer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupNodes {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub free: Vec<FreeNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wires: Vec<Wire>,
    /// Where each box sits (view name to canvas point, NE-09). Boxes
    /// without a place are laid out by depth.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub places: BTreeMap<String, [f64; 2]>,
    /// Boxes hidden in the view only (NE-07).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub hidden: BTreeSet<String>,
}
impl GroupNodes {
    pub fn is_empty(&self) -> bool {
        self.free.is_empty()
            && self.wires.is_empty()
            && self.places.is_empty()
            && self.hidden.is_empty()
    }
    pub fn node(&self, name: &str) -> Option<&FreeNode> {
        self.free.iter().find(|n| n.name == name)
    }
    /// The wire into `to`, if any (an input takes one).
    pub fn wire_into(&self, to: &Target) -> Option<&Wire> {
        self.wires.iter().find(|w| &w.to == to)
    }
    /// Whether free node `name` feeds anything (NE-14 draws it dimmed
    /// when not).
    pub fn feeds(&self, name: &str) -> bool {
        self.wires.iter().any(|w| w.from == name)
    }
}

/// A standard MaterialX node added in the node view.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeNode {
    /// Unique in its group; a MaterialX name (letters, digits, `_`).
    pub name: String,
    /// The MaterialX category (`noise3d`) and output type (`color3`).
    pub category: String,
    pub ty: String,
    /// Its set input values, as MaterialX writes them ("0.5, 0.5, 0.5").
    /// A wired input keeps its value (NE-03).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<FreeInput>,
}
impl FreeNode {
    pub fn input(&self, name: &str) -> Option<&FreeInput> {
        self.inputs.iter().find(|i| i.name == name)
    }
    /// The catalog entry it was made from, for its label.
    pub fn kind(&self) -> Option<&'static Kind> {
        CATALOG
            .iter()
            .find(|k| k.category == self.category && k.ty == self.ty)
    }
    /// Every input its definition declares, with types (wired or not).
    pub fn declared(&self) -> Vec<(String, String)> {
        let def = self
            .kind()
            .and_then(|k| crate::materialx::stdlib::standard_def(k.nodedef));
        match def {
            Some(d) => d
                .inputs
                .iter()
                .map(|p| (p.name.clone(), p.ty.clone()))
                .collect(),
            None => self
                .inputs
                .iter()
                .map(|i| (i.name.clone(), i.ty.clone()))
                .collect(),
        }
    }
    pub fn input_type(&self, name: &str) -> Option<String> {
        self.declared()
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeInput {
    pub name: String,
    pub ty: String,
    pub value: String,
}

/// A wire from a free node's output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wire {
    pub from: String,
    #[serde(default = "out")]
    pub output: String,
    pub to: Target,
}
fn out() -> String {
    "out".into()
}

/// What a wire drives: a free node's input, or a material layer's
/// channel (by its [`CHANNELS`] key).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Free { node: String, input: String },
    Layer { layer: NodeId, channel: String },
}

/// The material channels a wire can drive, with their keys (saved) and
/// labels (Properties' names). Provisional set (NE-Q01): the colour and
/// amount channels of a Material layer.
pub const CHANNELS: [(Ch, &str, &str); 24] = [
    (Ch::DiffuseColor, "diffuse_color", "Diffuse Color"),
    (Ch::DiffuseAmount, "diffuse_amount", "Diffuse Amount"),
    (
        Ch::DiffuseRoughness,
        "diffuse_roughness",
        "Diffuse Roughness",
    ),
    (Ch::SpecularColor, "specular_color", "Specular Color"),
    (Ch::SpecularAmount, "specular_amount", "Specular Amount"),
    (Ch::Roughness, "roughness", "Roughness"),
    (Ch::Anisotropy, "anisotropy", "Anisotropy"),
    (Ch::Metallic, "metallic", "Metallic"),
    (Ch::ReflectionColor, "reflection_color", "Reflection Color"),
    (
        Ch::ReflectionAmount,
        "reflection_amount",
        "Reflection Amount",
    ),
    (Ch::ClearcoatAmount, "clearcoat_amount", "Clearcoat Amount"),
    (
        Ch::ClearcoatRoughness,
        "clearcoat_roughness",
        "Clearcoat Roughness",
    ),
    (Ch::Sheen, "sheen", "Sheen"),
    (Ch::SheenTint, "sheen_tint", "Sheen Tint"),
    (Ch::SpecularTint, "specular_tint", "Specular Tint"),
    (Ch::Flatness, "flatness", "Flatness"),
    (
        Ch::TransparentColor,
        "transparent_color",
        "Transparent Color",
    ),
    (
        Ch::TransparentAmount,
        "transparent_amount",
        "Transparent Amount",
    ),
    (
        Ch::RefractionRoughness,
        "refraction_roughness",
        "Refraction Roughness",
    ),
    (Ch::SubsurfaceColor, "subsurface_color", "Subsurface Color"),
    (
        Ch::SubsurfaceAmount,
        "subsurface_amount",
        "Subsurface Amount",
    ),
    (Ch::LuminousColor, "luminous_color", "Luminous Color"),
    (
        Ch::LuminousIntensity,
        "luminous_intensity",
        "Luminous Intensity",
    ),
    (Ch::Dissolve, "dissolve", "Dissolve"),
];

/// A channel key's channel.
pub fn channel(key: &str) -> Option<Ch> {
    CHANNELS
        .iter()
        .find(|(_, k, _)| *k == key)
        .map(|(c, _, _)| *c)
}
/// A channel's key and label, if a wire can drive it.
pub fn channel_key(ch: Ch) -> Option<(&'static str, &'static str)> {
    CHANNELS
        .iter()
        .find(|(c, _, _)| *c == ch)
        .map(|(_, k, l)| (*k, *l))
}
/// A drivable channel's MaterialX type.
pub fn channel_type(ch: Ch) -> &'static str {
    if ch.is_color() { "color3" } else { "float" }
}

/// A node the Add menu offers (NE-02): a standard MaterialX nodedef the
/// renderer and the viewport both handle.
#[derive(Debug)]
pub struct Kind {
    pub label: &'static str,
    pub menu: &'static str,
    pub category: &'static str,
    pub ty: &'static str,
    pub nodedef: &'static str,
}
macro_rules! kind {
    ($label:expr, $menu:expr, $cat:expr, $ty:expr, $def:expr) => {
        Kind {
            label: $label,
            menu: $menu,
            category: $cat,
            ty: $ty,
            nodedef: $def,
        }
    };
}
/// The Add menu, in menu order (never sorted: it is what the user sees).
pub const CATALOG: &[Kind] = &[
    kind!("Noise", "Texture", "noise3d", "color3", "ND_noise3d_color3"),
    kind!(
        "Noise (Float)",
        "Texture",
        "noise3d",
        "float",
        "ND_noise3d_float"
    ),
    kind!(
        "Fractal Noise",
        "Texture",
        "fractal3d",
        "color3",
        "ND_fractal3d_color3"
    ),
    kind!(
        "Fractal Noise (Float)",
        "Texture",
        "fractal3d",
        "float",
        "ND_fractal3d_float"
    ),
    kind!(
        "Cell Noise",
        "Texture",
        "cellnoise3d",
        "float",
        "ND_cellnoise3d_float"
    ),
    kind!(
        "Checkerboard",
        "Texture",
        "checkerboard",
        "color3",
        "ND_checkerboard_color3"
    ),
    kind!(
        "Constant Color",
        "Input",
        "constant",
        "color3",
        "ND_constant_color3"
    ),
    kind!(
        "Constant",
        "Input",
        "constant",
        "float",
        "ND_constant_float"
    ),
    kind!(
        "Position",
        "Input",
        "position",
        "vector3",
        "ND_position_vector3"
    ),
    kind!(
        "Texture Coordinates",
        "Input",
        "texcoord",
        "vector2",
        "ND_texcoord_vector2"
    ),
    kind!("Add", "Math", "add", "color3", "ND_add_color3"),
    kind!(
        "Subtract",
        "Math",
        "subtract",
        "color3",
        "ND_subtract_color3"
    ),
    kind!(
        "Multiply",
        "Math",
        "multiply",
        "color3",
        "ND_multiply_color3"
    ),
    kind!(
        "Multiply (Float)",
        "Math",
        "multiply",
        "float",
        "ND_multiply_float"
    ),
    kind!("Mix", "Math", "mix", "color3", "ND_mix_color3"),
    kind!("Mix (Float)", "Math", "mix", "float", "ND_mix_float"),
    kind!("Clamp", "Math", "clamp", "float", "ND_clamp_float"),
    kind!("Power", "Math", "power", "float", "ND_power_float"),
    kind!("Invert", "Math", "invert", "float", "ND_invert_float"),
    kind!("Remap", "Adjust", "remap", "float", "ND_remap_float"),
    kind!(
        "Smooth Step",
        "Adjust",
        "smoothstep",
        "float",
        "ND_smoothstep_float"
    ),
    kind!(
        "Luminance",
        "Adjust",
        "luminance",
        "color3",
        "ND_luminance_color3"
    ),
];

/// The catalog entry by its MaterialX category and output type.
pub fn kind(category: &str, ty: &str) -> Option<&'static Kind> {
    CATALOG
        .iter()
        .find(|k| k.category == category && k.ty == ty)
}

/// Whether an output of type `from` may drive an input of type `to`
/// (NE-08): the same type, a float into any number, or colour and
/// vector of three into each other. Provisional (NE-Q02).
pub fn compatible(from: &str, to: &str) -> bool {
    let numeric = |t: &str| {
        matches!(
            t,
            "float" | "color3" | "color4" | "vector2" | "vector3" | "vector4"
        )
    };
    from == to
        || (from == "float" && numeric(to))
        || matches!((from, to), ("color3", "vector3") | ("vector3", "color3"))
}

/// One node-view change (NE-15: each is one undo step).
#[derive(Clone, Debug, PartialEq)]
pub enum NodeEdit {
    /// A free node from the Add menu at a canvas point (NE-02).
    Add {
        category: String,
        ty: String,
        at: [f64; 2],
    },
    /// Delete boxes by view name (NE-06): a free node goes with its wires;
    /// a layer's own box deletes the layer; a mixing node is refused.
    Delete(Vec<String>),
    /// Wire `from`'s `output` into `to` (NE-03), replacing a wire there.
    Connect {
        from: String,
        output: String,
        to: Target,
    },
    /// Remove the wire into `to`: its own value applies again.
    Disconnect(Target),
    /// A free node's input value (MaterialX text). `continuing` folds a
    /// drag into one step.
    SetValue {
        node: String,
        input: String,
        value: String,
        continuing: bool,
    },
    /// Where boxes sit (NE-16). `continuing` folds a drag into one step.
    Move {
        places: Vec<(String, [f64; 2])>,
        continuing: bool,
    },
    /// NE-07: hide boxes in the view, show some, or show all.
    Hide(Vec<String>),
    Show(Vec<String>),
    ShowAll,
}

/// What [`ShaderTree::edit_nodes`] did, for the undo name and folding.
pub struct Applied {
    pub label: String,
    /// The layers a Delete removes (the caller deletes them as Delete
    /// Shader Layers does, locators and instances included).
    pub remove_layers: Vec<NodeId>,
    pub continuing: bool,
}

impl ShaderTree {
    /// The node view of Material Group `group`, if it has one.
    pub fn group_nodes(&self, group: NodeId) -> Option<&GroupNodes> {
        self.node_views.get(&group)
    }

    /// How many free nodes Material Group `group` holds (NE-11's row).
    pub fn free_node_count(&self, group: NodeId) -> usize {
        self.group_nodes(group).map_or(0, |g| g.free.len())
    }

    /// The Material Group a layer sits in (the nearest one above it).
    pub fn material_group_of(&self, layer: NodeId) -> Option<NodeId> {
        let mut at = self.get(layer)?.parent;
        for _ in 0..=self.nodes.len() {
            let node = self.get(at?)?;
            if matches!(node.kind, NodeKind::Group(_)) {
                return Some(node.id);
            }
            at = node.parent;
        }
        None
    }

    /// The wire driving layer `layer`'s channel `ch`, if any (NE-03/04).
    pub fn channel_driver(&self, layer: NodeId, ch: Ch) -> Option<(NodeId, &Wire)> {
        let (key, _) = channel_key(ch)?;
        self.setting_driver(layer, key)
    }

    /// The wire driving layer `layer`'s input `key` (a material channel
    /// key or a texture setting key, node_view_settings.rs), if any.
    pub fn setting_driver(&self, layer: NodeId, key: &str) -> Option<(NodeId, &Wire)> {
        let group = self.material_group_of(layer)?;
        let to = Target::Layer {
            layer,
            channel: key.into(),
        };
        self.group_nodes(group)?.wire_into(&to).map(|w| (group, w))
    }

    /// A box's label in undo names: a free node's name, a layer's name.
    pub fn view_label(&self, name: &str) -> String {
        if let Some((id, mix)) = layer_of(name) {
            let layer = self.get(id).map_or("layer", |n| n.name.as_str());
            return if mix {
                format!("{layer} Mix")
            } else {
                layer.to_string()
            };
        }
        if name == OUTPUT {
            return "Material Output".into();
        }
        name.to_string()
    }

    /// A wire target in undo names: "Diffuse Color", "Mix2 bg".
    pub fn target_label(to: &Target) -> String {
        match to {
            Target::Free { node, input } => format!("{node} {input}"),
            Target::Layer { channel: key, .. } => CHANNELS
                .iter()
                .find(|(_, k, _)| k == key)
                .map(|(_, _, l)| *l)
                .or_else(|| crate::node_view_settings::label(key))
                .map_or(key.clone(), str::to_string),
        }
    }

    /// The MaterialX type `to` takes, or why it can't be wired.
    fn target_type(&self, group: NodeId, g: &GroupNodes, to: &Target) -> crate::Result<String> {
        match to {
            Target::Free { node, input } => {
                let n = g
                    .node(node)
                    .ok_or_else(|| format!("No node named {node}"))?;
                n.input_type(input)
                    .ok_or_else(|| format!("{node} has no input {input}"))
            }
            Target::Layer {
                layer,
                channel: key,
            } => {
                let node = self.get(*layer).ok_or("That layer no longer exists")?;
                if self.material_group_of(*layer) != Some(group) {
                    // NE-08: no wires between materials.
                    return Err("Wires stay inside one material".into());
                }
                // A texture layer's plain colour or value settings take
                // wires too (NE-03, node_view_settings.rs).
                if let Some(ty) = crate::node_view_settings::setting_type(&node.kind, key) {
                    return Ok(ty.into());
                }
                if !matches!(node.kind, NodeKind::Material(_)) {
                    return Err(format!("{} has no input {key} that takes wires", node.name));
                }
                let ch = channel(key).ok_or_else(|| format!("{key} can't be driven"))?;
                Ok(channel_type(ch).into())
            }
        }
    }

    /// Applies one node-view change to Material Group `group`.
    pub fn edit_nodes(&mut self, group: NodeId, edit: NodeEdit) -> crate::Result<Applied> {
        match self.get(group).map(|n| &n.kind) {
            Some(NodeKind::Group(_)) => {}
            Some(_) => return Err("Only a Material Group has a node view".into()),
            None => return Err("That Material Group no longer exists".into()),
        }
        let mut g = self.node_views.get(&group).cloned().unwrap_or_default();
        let mut remove_layers = Vec::new();
        let mut continuing = false;
        let label = match edit {
            NodeEdit::Add { category, ty, at } => {
                let k = kind(&category, &ty)
                    .ok_or_else(|| format!("No node {category} ({ty}) in the Add menu"))?;
                if g.free.len() >= MAX_FREE_NODES {
                    return Err("This material has too many nodes".into());
                }
                let def = crate::materialx::stdlib::standard_def(k.nodedef)
                    .ok_or_else(|| format!("No MaterialX definition {}", k.nodedef))?;
                let base: String = k
                    .label
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .collect();
                let name = (1..)
                    .map(|i| {
                        if i == 1 {
                            base.clone()
                        } else {
                            format!("{base}{i}")
                        }
                    })
                    .find(|n| g.node(n).is_none())
                    .expect("a free name");
                let inputs = def
                    .inputs
                    .iter()
                    .filter_map(|p| {
                        Some(FreeInput {
                            name: p.name.clone(),
                            ty: p.ty.clone(),
                            value: p.value.clone()?,
                        })
                    })
                    .collect();
                g.free.push(FreeNode {
                    name: name.clone(),
                    category,
                    ty,
                    inputs,
                });
                g.places.insert(name.clone(), at);
                format!("Add Node {name}")
            }
            NodeEdit::Delete(names) => {
                if names.is_empty() {
                    return Err("Select nodes to delete".into());
                }
                let mut deleted = Vec::new();
                for name in &names {
                    if let Some((id, mix)) = layer_of(name) {
                        let node = self.get(id).ok_or("That layer no longer exists")?;
                        if mix {
                            // NE-05: mixing nodes are the tree's to change.
                            return Err(format!(
                                "{}'s mixing node is locked: reorder or delete layers in the Shader Tree",
                                node.name
                            ));
                        }
                        remove_layers.push(id);
                        deleted.push(node.name.clone());
                    } else if name == OUTPUT {
                        return Err("The Material Output can't be deleted".into());
                    } else if g.node(name).is_some() {
                        g.free.retain(|n| &n.name != name);
                        g.wires.retain(|w| {
                            &w.from != name
                                && !matches!(&w.to, Target::Free { node, .. } if node == name)
                        });
                        g.places.remove(name);
                        g.hidden.remove(name);
                        deleted.push(name.clone());
                    } else {
                        return Err(format!("No node named {name}"));
                    }
                }
                match deleted.as_slice() {
                    [one] => format!("Delete Node {one}"),
                    _ => "Delete Nodes".into(),
                }
            }
            NodeEdit::Connect { from, output, to } => {
                let src = g
                    .node(&from)
                    .ok_or_else(|| format!("Only free nodes' outputs take wires yet ({from})"))?;
                if output != "out" {
                    return Err(format!("{from} has no output {output}"));
                }
                let want = self.target_type(group, &g, &to)?;
                if !compatible(&src.ty, &want) {
                    return Err(format!("A {} output can't drive a {want} input", src.ty));
                }
                if let Target::Free { node, .. } = &to
                    && (node == &from || reaches(&g, node, &from))
                {
                    return Err("That wire would make a loop".into());
                }
                let label = format!("Connect {from} > {}", Self::target_label(&to));
                g.wires.retain(|w| w.to != to);
                g.wires.push(Wire { from, output, to });
                label
            }
            NodeEdit::Disconnect(to) => {
                if g.wire_into(&to).is_none() {
                    return Err("Nothing is wired there".into());
                }
                g.wires.retain(|w| w.to != to);
                format!("Disconnect {}", Self::target_label(&to))
            }
            NodeEdit::SetValue {
                node,
                input,
                value,
                continuing: c,
            } => {
                let ty = g
                    .node(&node)
                    .ok_or_else(|| format!("No node named {node}"))?
                    .input_type(&input)
                    .ok_or_else(|| format!("{node} has no input {input}"))?;
                let value = check_value(&ty, &value)?;
                let n = g.free.iter_mut().find(|n| n.name == node).expect("checked");
                match n.inputs.iter_mut().find(|i| i.name == input) {
                    Some(i) => i.value = value,
                    None => n.inputs.push(FreeInput {
                        name: input.clone(),
                        ty,
                        value,
                    }),
                }
                continuing = c;
                format!("Edit {node} {input}")
            }
            NodeEdit::Move {
                places,
                continuing: c,
            } => {
                if places.is_empty() {
                    return Err("Nothing to move".into());
                }
                for (name, at) in places {
                    if !at.iter().all(|v| v.is_finite()) {
                        return Err("Node positions must be numbers".into());
                    }
                    self.check_view_name(group, &g, &name)?;
                    g.places.insert(name, at);
                }
                continuing = c;
                "Move Node".into()
            }
            NodeEdit::Hide(names) => {
                if names.is_empty() {
                    return Err("Select nodes to hide".into());
                }
                for name in names {
                    self.check_view_name(group, &g, &name)?;
                    g.hidden.insert(name);
                }
                "Hide Node".into()
            }
            NodeEdit::Show(names) => {
                for name in names {
                    g.hidden.remove(&name);
                }
                "Show Node".into()
            }
            NodeEdit::ShowAll => {
                if g.hidden.is_empty() {
                    return Err("No hidden nodes".into());
                }
                g.hidden.clear();
                "Show All Nodes".into()
            }
        };
        if g.is_empty() {
            self.node_views.remove(&group);
        } else {
            self.node_views.insert(group, g);
        }
        Ok(Applied {
            label,
            remove_layers,
            continuing,
        })
    }

    fn check_view_name(&self, group: NodeId, g: &GroupNodes, name: &str) -> crate::Result<()> {
        let ok = match layer_of(name) {
            Some((id, _)) => self.material_group_of(id) == Some(group),
            None => name == OUTPUT || g.node(name).is_some(),
        };
        if ok {
            Ok(())
        } else {
            Err(format!("No node named {name} in this material"))
        }
    }

    /// Drops what refers to layers or groups that are gone (NE-13: a free
    /// node driving a deleted layer stays, unwired), and wires into layers
    /// that left the group. Run after every tree edit.
    pub(crate) fn prune_node_views(&mut self) {
        let groups: Vec<NodeId> = self.node_views.keys().copied().collect();
        for group in groups {
            if !matches!(self.get(group).map(|n| &n.kind), Some(NodeKind::Group(_))) {
                self.node_views.remove(&group);
                continue;
            }
            let inside = |id: NodeId| self.material_group_of(id) == Some(group);
            let material =
                |id: NodeId| matches!(self.get(id).map(|n| &n.kind), Some(NodeKind::Material(_)));
            let g = self.node_views.get(&group).expect("listed").clone();
            let mut next = g.clone();
            next.wires.retain(|w| match &w.to {
                Target::Layer { layer, channel } => {
                    inside(*layer)
                        && (material(*layer)
                            || self.get(*layer).is_some_and(|n| {
                                crate::node_view_settings::setting_type(&n.kind, channel).is_some()
                            }))
                }
                Target::Free { .. } => true,
            });
            let keep = |name: &String| match layer_of(name) {
                Some((id, _)) => inside(id),
                None => true,
            };
            next.places.retain(|n, _| keep(n));
            next.hidden.retain(keep);
            if next != g {
                if next.is_empty() {
                    self.node_views.remove(&group);
                } else {
                    self.node_views.insert(group, next);
                }
            }
        }
    }

    pub(crate) fn validate_node_views(&self) -> crate::Result<()> {
        for (group, g) in &self.node_views {
            if !matches!(self.get(*group).map(|n| &n.kind), Some(NodeKind::Group(_))) {
                return Err("Node view of a missing Material Group".into());
            }
            if g.free.len() > MAX_FREE_NODES {
                return Err("A material has too many nodes".into());
            }
            let mut names = BTreeSet::new();
            for n in &g.free {
                if !valid_name(&n.name) || !names.insert(n.name.as_str()) {
                    return Err(format!("Invalid or repeated node name '{}'", n.name));
                }
            }
            for w in &g.wires {
                if g.node(&w.from).is_none() {
                    return Err(format!("A wire from a missing node '{}'", w.from));
                }
                if let Target::Free { node, .. } = &w.to
                    && g.node(node).is_none()
                {
                    return Err(format!("A wire into a missing node '{node}'"));
                }
            }
        }
        Ok(())
    }
}

/// Whether free node `from` feeds `to`, directly or through others.
fn reaches(g: &GroupNodes, from: &str, to: &str) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from.to_string()];
    while let Some(n) = stack.pop() {
        if !seen.insert(n.clone()) {
            continue;
        }
        for w in g.wires.iter().filter(|w| w.from == n) {
            if let Target::Free { node, .. } = &w.to {
                if node == to {
                    return true;
                }
                stack.push(node.clone());
            }
        }
    }
    false
}

/// A MaterialX node name: a letter or `_`, then letters, digits, `_`.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.len() <= 64
}

/// A value for an input of MaterialX type `ty`, tidied ("0.5,1,1" to
/// "0.5, 1, 1"), or why it doesn't fit.
pub fn check_value(ty: &str, value: &str) -> crate::Result<String> {
    let count = match ty {
        "float" | "integer" => 1,
        "vector2" => 2,
        "color3" | "vector3" => 3,
        "color4" | "vector4" => 4,
        "boolean" => {
            return match value.trim() {
                "true" | "false" => Ok(value.trim().into()),
                other => Err(format!("'{other}' isn't true or false")),
            };
        }
        _ => return Ok(value.to_string()),
    };
    let parts: Vec<f64> = value
        .split(',')
        .map(|s| s.trim().parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| format!("'{value}' isn't {count} number(s)"))?;
    let parts = match (parts.len(), count) {
        (n, c) if n == c => parts,
        (1, c) => vec![parts[0]; c],
        _ => return Err(format!("'{value}' needs {count} number(s)")),
    };
    if parts.iter().any(|v| !v.is_finite()) {
        return Err("Values must be finite numbers".into());
    }
    Ok(parts
        .iter()
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join(", "))
}

// -- Wires driving channels (NE-03) -------------------------------------

use crate::materialx::doc::{Document, Element, Port};
use crate::node_graph::FlatGraph;
use crate::shader_eval::Expr;

/// A free node's output as one expression node (`Expr::Nodes`): the free
/// nodes it reads as MaterialX elements (dependencies first, the output
/// node last), their Cycles nodes for the renderer and their lowered
/// expression for the viewport. Two are equal when their MaterialX is:
/// the rest is made from it.
#[derive(Debug)]
pub struct NodeOut {
    pub elements: Vec<Element>,
    pub node: String,
    pub output: String,
    /// The MaterialX output type (`color3`, `float`).
    pub ty: String,
    pub flat: FlatGraph,
    /// The Cycles node and socket the value comes out of.
    pub at: (usize, String),
    /// What the viewport draws: the nodes lowered to tree expressions, or
    /// mid grey for nodes it can't draw (provisional, as MXG-Q07).
    pub lowered: Expr,
    key: String,
}
impl PartialEq for NodeOut {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

/// MaterialX text of elements, the identity of a [`NodeOut`].
fn key_of(elements: &[Element], node: &str, output: &str) -> String {
    let mut text = String::new();
    for e in elements {
        crate::materialx::write::element(&mut text, e, "");
    }
    format!("{text}{node}.{output}")
}

/// The elements free node `start` reads, dependencies first, by
/// `lookup` (each name once; a loop stops where it closes).
pub fn collect<'a>(start: &str, lookup: &dyn Fn(&str) -> Option<&'a Element>) -> Vec<Element> {
    fn visit<'a>(
        name: &str,
        lookup: &dyn Fn(&str) -> Option<&'a Element>,
        seen: &mut BTreeSet<String>,
        out: &mut Vec<Element>,
    ) {
        if !seen.insert(name.to_string()) {
            return;
        }
        let Some(el) = lookup(name) else {
            return;
        };
        for p in &el.inputs {
            if let Some(n) = &p.nodename {
                visit(n, lookup, seen, out);
            }
        }
        out.push(el.clone());
    }
    let mut out = Vec::new();
    visit(start, lookup, &mut BTreeSet::new(), &mut out);
    out
}

impl GroupNodes {
    /// Free node `name` as a MaterialX element: wired inputs read their
    /// node, the others their kept values. No positions: they never
    /// change a render (NE-16).
    pub fn element(&self, name: &str) -> Option<Element> {
        let n = self.node(name)?;
        let mut inputs = Vec::new();
        for (input, ty) in n.declared() {
            let to = Target::Free {
                node: name.to_string(),
                input: input.clone(),
            };
            let port = match (self.wire_into(&to), n.input(&input)) {
                (Some(w), _) => Port {
                    name: input,
                    ty,
                    nodename: Some(w.from.clone()),
                    ..Port::default()
                },
                (None, Some(v)) => Port {
                    name: input,
                    ty,
                    value: Some(v.value.clone()),
                    ..Port::default()
                },
                (None, None) => continue,
            };
            inputs.push(port);
        }
        Some(Element {
            category: n.category.clone(),
            name: n.name.clone(),
            ty: n.ty.clone(),
            inputs,
            ..Element::default()
        })
    }

    /// Every free node as an element, in list order.
    pub fn elements(&self) -> Vec<Element> {
        self.free
            .iter()
            .filter_map(|n| self.element(&n.name))
            .collect()
    }
}

/// Output `output` of free node `node` among `elements` (as
/// [`collect`] gives them) as an expression: `Expr::Nodes`, a constant
/// when it folds, `None` when the renderer can't make it (the channel then
/// keeps its own value).
pub fn node_out(elements: Vec<Element>, node: &str, output: &str) -> Option<Expr> {
    let key = key_of(&elements, node, output);
    thread_local! {
        static MEMO: std::cell::RefCell<std::collections::HashMap<String, Option<Expr>>> =
            Default::default();
    }
    if let Some(hit) = MEMO.with(|m| m.borrow().get(&key).cloned()) {
        return hit;
    }
    let made = make_node_out(elements, node, output, key.clone());
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        // A small cache: drags make a new key per frame.
        if m.len() > 256 {
            m.clear();
        }
        m.insert(key, made.clone());
    });
    made
}

fn make_node_out(elements: Vec<Element>, node: &str, output: &str, key: String) -> Option<Expr> {
    use crate::materialx::cycles::{self, Output};
    let ty = elements.iter().find(|e| e.name == node)?.ty.clone();
    let colour = ty != "float";
    let doc = Document {
        version: "1.39".into(),
        colorspace: None,
        elements: elements.clone(),
    };
    let (flat, at) =
        match cycles::translate_output(&doc, node, output, std::path::Path::new("")).ok()? {
            (_, Output::Unset) => return None,
            (_, Output::Const(c)) => {
                let v = match c.len() {
                    0 => [0.; 3],
                    1 | 2 => [c[0]; 3],
                    _ => [c[0], c[1], c[2]],
                };
                return Some(Expr::Const(v));
            }
            (flat, Output::Socket { node, socket }) => (flat, (node, socket)),
        };
    let lowered = crate::graph_view::output_expr(
        &flat,
        at.0,
        &at.1,
        colour,
        &crate::graph_view::GraphSpace::default(),
    )
    .unwrap_or(Expr::scalar(0.5));
    Some(Expr::Nodes(std::sync::Arc::new(NodeOut {
        elements,
        node: node.to_string(),
        output: output.to_string(),
        ty,
        flat,
        at,
        lowered,
        key,
    })))
}

impl ShaderTree {
    /// What a wire into layer `layer`'s channel `ch` makes it, `None`
    /// when nothing drives it (or the renderer can't make the nodes).
    pub fn driven_value(&self, layer: NodeId, ch: Ch) -> Option<Expr> {
        if self.node_views.is_empty() {
            return None;
        }
        let (key, _) = channel_key(ch)?;
        self.driven_setting(layer, key)
    }

    /// [`Self::driven_value`] for any wire target key of a layer (a
    /// texture setting's too).
    pub fn driven_setting(&self, layer: NodeId, key: &str) -> Option<Expr> {
        if self.node_views.is_empty() {
            return None;
        }
        let (group, wire) = self.setting_driver(layer, key)?;
        let g = self.group_nodes(group)?;
        let all = g.elements();
        let elements = collect(&wire.from, &|n| all.iter().find(|e| e.name == n));
        node_out(elements, &wire.from, &wire.output)
    }
}

#[cfg(test)]
#[path = "node_view_tests.rs"]
pub(crate) mod tests;
