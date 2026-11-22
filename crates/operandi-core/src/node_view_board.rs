//! A Material Group's node view as boxes and wires (node editing, NE-01..
//! NE-14; docs/NODE_EDITING_PLAN.md). Made from the layers and the group's
//! free nodes each time it is drawn, never stored (UIC-244): the app draws
//! a [`Board`] and turns gestures into `ShaderTreeEdit`s.
//!
//! - Each layer in the group is a **layer box** (`layer:<id>`); a Material
//!   layer's inputs are its channels, so editing one is the Properties edit.
//! - Each layer that stacks (material, texture, group, Node Material) has a
//!   **mixing node** (`mix:<id>`, NE-05): its opacity and blend mode, the
//!   stack below it and the layer itself. Mixing nodes are locked: their
//!   wires don't move and they can't be deleted here.
//! - The mixing nodes chain bottom to top into the **Material Output**,
//!   which also takes the topmost Shader.
//! - Free nodes come last, dimmed when they feed nothing (NE-14).
use crate::node_view::{self, CHANNELS, GroupNodes, OUTPUT, Target, channel_type};
use crate::shader_eval::{Ch, material_value};
use crate::shader_tree::{BlendMode, MaterialChannels, NodeId, NodeKind, ShaderTree};

/// What a box stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    Layer(NodeId),
    Mix(NodeId),
    Output,
    Free,
}

/// One input row.
#[derive(Clone, Debug, PartialEq)]
pub struct Socket {
    pub name: String,
    pub label: String,
    /// The MaterialX type it takes (`color3`, `float`, `vector3`...), empty
    /// for a stack link.
    pub ty: String,
    pub value: Value,
    /// What a wire into it would drive, `None` when it takes no wires
    /// (a mixing node's rows, NE-05).
    pub target: Option<Target>,
    /// A free node's wire drives it: its own value is kept but greyed
    /// (NE-03, UIC-92).
    pub driven: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Only a wire feeds it.
    None,
    Numbers(Vec<f64>),
    /// A choice (a blend mode): the options and the current one.
    Choice(Vec<&'static str>, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeBox {
    /// The view name (`layer:4`, `mix:4`, `output`, a free node's name).
    pub name: String,
    pub title: String,
    pub role: Role,
    pub inputs: Vec<Socket>,
    pub outputs: Vec<String>,
    /// NE-05: a mixing node.
    pub locked: bool,
    /// NE-14: a free node feeding nothing.
    pub dimmed: bool,
    pub hidden: bool,
    /// Where it sits, if placed (NE-09).
    pub at: Option<[f64; 2]>,
}
impl NodeBox {
    pub fn input(&self, name: &str) -> Option<&Socket> {
        self.inputs.iter().find(|s| s.name == name)
    }
}

/// A wire between two boxes.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub from: String,
    pub output: String,
    pub to: String,
    pub input: String,
    /// Made by the tree (the stack): it can't be moved (NE-05).
    pub locked: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Board {
    pub boxes: Vec<NodeBox>,
    pub links: Vec<Link>,
}
impl Board {
    pub fn get(&self, name: &str) -> Option<&NodeBox> {
        self.boxes.iter().find(|b| b.name == name)
    }
}

/// The channels a Material box shows: the common ones, plus any that
/// differ from a new material or are driven (provisional, NE-Q04).
const ALWAYS: [Ch; 6] = [
    Ch::DiffuseColor,
    Ch::DiffuseAmount,
    Ch::Roughness,
    Ch::Metallic,
    Ch::SpecularAmount,
    Ch::LuminousColor,
];

/// Sets a Material layer's own value for a drivable channel (the inverse
/// of `material_value` for `node_view::CHANNELS`).
pub fn set_channel(m: &mut MaterialChannels, ch: Ch, v: [f64; 3]) {
    let s = v[0];
    match ch {
        Ch::DiffuseColor => m.diffuse_color = v,
        Ch::DiffuseAmount => m.diffuse_amount = s,
        Ch::DiffuseRoughness => m.diffuse_roughness = s,
        Ch::SpecularColor => m.specular_color = v,
        Ch::SpecularAmount => m.specular_amount = s,
        Ch::Roughness => m.roughness = s,
        Ch::Anisotropy => m.anisotropy = s,
        Ch::Metallic => m.metallic = s,
        Ch::ReflectionColor => m.reflection_color = v,
        Ch::ReflectionAmount => m.reflection_amount = s,
        Ch::ClearcoatAmount => m.clearcoat_amount = s,
        Ch::ClearcoatRoughness => m.clearcoat_roughness = s,
        Ch::Sheen => m.sheen = s,
        Ch::SheenTint => m.sheen_tint = s,
        Ch::SpecularTint => m.specular_tint = s,
        Ch::Flatness => m.flatness = s,
        Ch::TransparentColor => m.transparent_color = v,
        Ch::TransparentAmount => m.transparent_amount = s,
        Ch::RefractionRoughness => m.refraction_roughness = s,
        Ch::SubsurfaceColor => m.subsurface_color = v,
        Ch::SubsurfaceAmount => m.subsurface_amount = s,
        Ch::LuminousColor => m.luminous_color = v,
        Ch::LuminousIntensity => m.luminous_intensity = s,
        Ch::Dissolve => m.dissolve = s,
        _ => {}
    }
}

fn numbers(text: &str) -> Vec<f64> {
    text.split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

impl ShaderTree {
    /// Material Group `group`'s node view, `None` when it isn't one.
    pub fn board(&self, group: NodeId) -> Option<Board> {
        let gnode = self.get(group)?;
        if !matches!(gnode.kind, NodeKind::Group(_)) {
            return None;
        }
        let empty = GroupNodes::default();
        let g = self.group_nodes(group).unwrap_or(&empty);
        let mut board = Board::default();
        let mut boxed = |b: NodeBox| board.boxes.push(b);
        let place = |name: &str| g.places.get(name).copied();
        let hidden = |name: &str| g.hidden.contains(name);
        // Top of the group first in the list; the stack runs bottom up.
        let layers: Vec<_> = self.children(Some(group)).collect();
        let mut below: Option<String> = None;
        let mut shader: Option<String> = None;
        let mut links = Vec::new();
        for node in layers.iter().rev() {
            let name = node_view::layer_name(node.id);
            let (inputs, stacks) = match &node.kind {
                NodeKind::Material(m) => (self.channel_rows(g, node.id, m), true),
                NodeKind::Texture(t) => (self.texture_rows(g, node.id, t), true),
                NodeKind::Group(_) | NodeKind::Folder | NodeKind::NodeMaterial(_) => {
                    (Vec::new(), true)
                }
                NodeKind::Shader(_) => (Vec::new(), false),
                NodeKind::RenderOutput(_) => continue,
            };
            let kind = self.effect_label(node);
            let title = if kind.is_empty() || kind == node.name {
                node.name.clone()
            } else {
                format!("{} ({kind})", node.name)
            };
            boxed(NodeBox {
                name: name.clone(),
                title,
                role: Role::Layer(node.id),
                inputs,
                outputs: vec!["out".into()],
                locked: false,
                dimmed: !node.layer.enable,
                hidden: hidden(&name),
                at: place(&name),
            });
            if !stacks {
                // Bottom up: the last one is the topmost Shader.
                shader = Some(name);
                continue;
            }
            let mix = node_view::mix_name(node.id);
            let blend = BlendMode::ALL
                .iter()
                .position(|b| *b == node.layer.blend)
                .unwrap_or(0);
            let mut inputs = Vec::new();
            if let Some(b) = &below {
                inputs.push(stack_row("below", "Below"));
                links.push(Link {
                    from: b.clone(),
                    output: "out".into(),
                    to: mix.clone(),
                    input: "below".into(),
                    locked: true,
                });
            }
            inputs.push(stack_row("layer", "Layer"));
            links.push(Link {
                from: name.clone(),
                output: "out".into(),
                to: mix.clone(),
                input: "layer".into(),
                locked: true,
            });
            inputs.push(Socket {
                name: "opacity".into(),
                label: "Opacity".into(),
                ty: "float".into(),
                value: Value::Numbers(vec![node.layer.opacity]),
                target: None,
                driven: false,
            });
            inputs.push(Socket {
                name: "blend".into(),
                label: "Blend".into(),
                ty: String::new(),
                value: Value::Choice(BlendMode::LABELS.to_vec(), blend),
                target: None,
                driven: false,
            });
            boxed(NodeBox {
                name: mix.clone(),
                title: format!("{} Mix", node.name),
                role: Role::Mix(node.id),
                inputs,
                outputs: vec!["out".into()],
                locked: true,
                dimmed: !node.layer.enable,
                hidden: hidden(&mix),
                at: place(&mix),
            });
            below = Some(mix);
        }
        let mut out_inputs = vec![stack_row("surface", "Surface")];
        if let Some(b) = &below {
            links.push(Link {
                from: b.clone(),
                output: "out".into(),
                to: OUTPUT.into(),
                input: "surface".into(),
                locked: true,
            });
        }
        out_inputs.push(stack_row("shader", "Shader"));
        if let Some(s) = &shader {
            links.push(Link {
                from: s.clone(),
                output: "out".into(),
                to: OUTPUT.into(),
                input: "shader".into(),
                locked: true,
            });
        }
        boxed(NodeBox {
            name: OUTPUT.into(),
            title: "Material Output".into(),
            role: Role::Output,
            inputs: out_inputs,
            outputs: Vec::new(),
            locked: false,
            dimmed: false,
            hidden: hidden(OUTPUT),
            at: place(OUTPUT),
        });
        for n in &g.free {
            let inputs = n
                .declared()
                .into_iter()
                .map(|(input, ty)| {
                    let to = Target::Free {
                        node: n.name.clone(),
                        input: input.clone(),
                    };
                    let value = match n.input(&input) {
                        Some(v) => Value::Numbers(numbers(&v.value)),
                        None => Value::None,
                    };
                    Socket {
                        label: input.clone(),
                        name: input,
                        ty,
                        value,
                        driven: g.wire_into(&to).is_some(),
                        target: Some(to),
                    }
                })
                .collect();
            boxed(NodeBox {
                name: n.name.clone(),
                title: n.kind().map_or(n.category.clone(), |k| k.label.to_string()),
                role: Role::Free,
                inputs,
                outputs: vec!["out".into()],
                locked: false,
                dimmed: !g.feeds(&n.name),
                hidden: hidden(&n.name),
                at: place(&n.name),
            });
        }
        for w in &g.wires {
            let (to, input) = match &w.to {
                Target::Free { node, input } => (node.clone(), input.clone()),
                Target::Layer { layer, channel } => {
                    (node_view::layer_name(*layer), channel.clone())
                }
            };
            links.push(Link {
                from: w.from.clone(),
                output: w.output.clone(),
                to,
                input,
                locked: false,
            });
        }
        board.links = links;
        Some(board)
    }

    /// A texture layer's plain colour or value settings (NE-03).
    fn texture_rows(
        &self,
        g: &GroupNodes,
        layer: NodeId,
        t: &crate::shader_tree::TextureLayer,
    ) -> Vec<Socket> {
        crate::node_view_settings::settings(t)
            .into_iter()
            .map(|(key, label, ty, v)| {
                let to = Target::Layer {
                    layer,
                    channel: key.to_string(),
                };
                Socket {
                    name: key.to_string(),
                    label: label.to_string(),
                    ty: ty.to_string(),
                    // A Gradient's Input has no number of its own.
                    value: if v.is_empty() {
                        Value::None
                    } else {
                        Value::Numbers(v)
                    },
                    driven: g.wire_into(&to).is_some(),
                    target: Some(to),
                }
            })
            .collect()
    }

    /// Layer `layer` with one of its box values changed (NE-01): a
    /// material channel, a texture setting, or (`opacity`) its mixing
    /// node's opacity (NE-05). `None` when nothing changes.
    pub fn edited_layer(
        &self,
        layer: NodeId,
        key: &str,
        v: &[f64],
    ) -> Option<crate::shader_tree::Node> {
        let mut node = self.get(layer)?.clone();
        let first = *v.first()?;
        if key == "opacity" {
            node.layer.opacity = first.clamp(0., 1.);
        } else {
            match &mut node.kind {
                NodeKind::Material(m) => {
                    let ch = node_view::channel(key)?;
                    let c = [0, 1, 2].map(|i| v.get(i).copied().unwrap_or(first));
                    set_channel(m, ch, c);
                }
                NodeKind::Texture(t) => {
                    if !crate::node_view_settings::set(t, key, v) {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        (node != *self.get(layer)?).then_some(node)
    }

    fn channel_rows(&self, g: &GroupNodes, layer: NodeId, m: &MaterialChannels) -> Vec<Socket> {
        let fresh = MaterialChannels::default();
        CHANNELS
            .iter()
            .filter_map(|(ch, key, label)| {
                let to = Target::Layer {
                    layer,
                    channel: (*key).to_string(),
                };
                let driven = g.wire_into(&to).is_some();
                let v = material_value(m, *ch);
                let show = ALWAYS.contains(ch) || driven || v != material_value(&fresh, *ch);
                show.then(|| Socket {
                    name: (*key).to_string(),
                    label: (*label).to_string(),
                    ty: channel_type(*ch).into(),
                    value: Value::Numbers(if ch.is_color() {
                        v.to_vec()
                    } else {
                        vec![v[0]]
                    }),
                    target: Some(to),
                    driven,
                })
            })
            .collect()
    }
}

fn stack_row(name: &str, label: &str) -> Socket {
    Socket {
        name: name.into(),
        label: label.into(),
        ty: String::new(),
        value: Value::None,
        target: None,
        driven: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::node_view::tests::{MATERIAL, add, diffuse, editor, nodes};
    use crate::node_view::{NodeEdit, layer_name, mix_name};

    #[test]
    fn a_material_group_shows_its_layers_mixing_nodes_and_output() {
        let (mut e, group) = editor();
        let board = e.scene.shader_tree.board(group).unwrap();
        let names: Vec<&str> = board.boxes.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(
            names,
            [layer_name(MATERIAL).as_str(), &mix_name(MATERIAL), "output"]
        );
        let mix = board.get(&mix_name(MATERIAL)).unwrap();
        assert!(mix.locked);
        assert!(
            mix.input("opacity").unwrap().target.is_none(),
            "no wires into a mixing node"
        );
        let m = board.get(&layer_name(MATERIAL)).unwrap();
        assert!(m.input("diffuse_color").is_some());
        assert!(board.links.iter().all(|l| l.locked));
        // A free node feeding nothing is dimmed; wired, it isn't, and the
        // channel shows as driven.
        let noise = add(&mut e, group, "noise3d", "color3");
        let board = e.scene.shader_tree.board(group).unwrap();
        assert!(board.get(&noise).unwrap().dimmed);
        nodes(
            &mut e,
            group,
            NodeEdit::Connect {
                from: noise.clone(),
                output: "out".into(),
                to: diffuse(MATERIAL),
            },
        )
        .unwrap();
        let board = e.scene.shader_tree.board(group).unwrap();
        assert!(!board.get(&noise).unwrap().dimmed);
        let m = board.get(&layer_name(MATERIAL)).unwrap();
        assert!(m.input("diffuse_color").unwrap().driven);
        assert!(
            board
                .links
                .iter()
                .any(|l| l.from == noise && l.to == layer_name(MATERIAL) && !l.locked)
        );
        // Only a Material Group has a board.
        assert!(e.scene.shader_tree.board(MATERIAL).is_none());
    }
}
