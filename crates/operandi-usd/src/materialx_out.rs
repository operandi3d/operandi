//! USD export, U6: each material's MaterialX network beside its
//! UsdPreviewSurface. The Material Group's own MaterialX (Operandi's
//! writer, `operandi_core::materialx::export`, open_pbr_surface with image
//! nodes) becomes Shader prims under the Material, wired to
//! `outputs:mtlx:surface`, as Blender and OpenUSD's MaterialX plugin
//! write them. Pictures are the same copied files the Preview Surface
//! reads.
//!
//! Graphs (usd-gaps-3): a MaterialX graph layer's nodegraphs become
//! UsdShade NodeGraph prims with their interface inputs and outputs, a
//! Material Group's free nodes (node editing) all go out with their
//! places, and a Blender Node Material layer goes out as the MaterialX
//! nodes its Cycles nodes map to (`cycles_mtlx`).
use operandi_core::Scene;
use operandi_core::shader_tree::{NodeId, NodeKind, TagMask, TagType};

/// A MaterialX network as USD shaders: top-level shaders, NodeGraph
/// prims holding their own shaders (MaterialX nodegraphs, with their
/// interface inputs and outputs), and the surface shader the material's
/// `outputs:mtlx:surface` reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Network {
    pub nodes: Vec<NetNode>,
    pub graphs: Vec<NetGraph>,
    /// The surface shader node `outputs:mtlx:surface` connects to.
    pub terminal: String,
}
impl Network {
    /// Every input: the nodes' and the NodeGraphs' interface inputs.
    pub fn inputs(&self) -> impl Iterator<Item = &NetInput> {
        self.nodes
            .iter()
            .flat_map(|n| n.inputs.iter())
            .chain(self.graphs.iter().flat_map(|g| g.inputs.iter()))
    }
    /// The same, to change.
    pub fn inputs_mut(&mut self) -> impl Iterator<Item = &mut NetInput> {
        self.nodes
            .iter_mut()
            .flat_map(|n| n.inputs.iter_mut())
            .chain(self.graphs.iter_mut().flat_map(|g| g.inputs.iter_mut()))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetNode {
    pub name: String,
    /// The MaterialX nodedef: info:id ("ND_image_color3").
    pub id: String,
    /// Its output type ("multioutput" for a node with named outputs).
    pub ty: String,
    pub inputs: Vec<NetInput>,
    /// A multi-output node's outputs; empty for the one `out`.
    pub outputs: Vec<NetOutput>,
    /// The NodeGraph it sits in, or `None` at the material's top level.
    pub graph: Option<String>,
    /// Its place in a node editor (MaterialX xpos/ypos), written as
    /// UsdUI's `ui:nodegraph:node:pos`.
    pub pos: Option<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetOutput {
    pub name: String,
    pub ty: String,
}

/// A NodeGraph prim: a MaterialX nodegraph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetGraph {
    pub name: String,
    /// Its interface inputs: values, or top-level nodes' outputs.
    pub inputs: Vec<NetInput>,
    /// Its outputs, each one of its own nodes (`Node`/`NodeOutput`) or
    /// one of its interface inputs passed through (`Interface`).
    pub outputs: Vec<NetGraphOutput>,
    pub pos: Option<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetGraphOutput {
    pub name: String,
    pub ty: String,
    pub value: NetValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetInput {
    pub name: String,
    pub ty: String,
    pub value: NetValue,
    pub colorspace: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NetValue {
    /// MaterialX's text ("0.5", "1, 0, 0").
    Text(String),
    /// A picture, by `Scene::images` id.
    Image(u64),
    /// Another node's `out`, in the same scope (top level or NodeGraph).
    Node(String),
    /// A multi-output node's named output, in the same scope.
    NodeOutput(String, String),
    /// The enclosing NodeGraph's interface input.
    Interface(String),
    /// A NodeGraph's output: (graph, output).
    Graph(String, String),
    /// A picture outside `Scene::images` (a MaterialX graph layer's),
    /// by its file path and the key the copied files are listed under
    /// (`materialx_graph_out::picture_key`).
    File { key: u64, path: String },
}

/// The nodedef name MaterialX gives a node of `category` with output
/// type `ty` (`in_ty`: its "in" input's type, for nodes named after it).
pub fn nodedef(category: &str, ty: &str, in_ty: Option<&str>) -> String {
    match (category, in_ty) {
        // Named after what they read.
        ("extract", Some(i)) => format!("ND_extract_{i}"),
        ("convert", Some(i)) => format!("ND_convert_{i}_{ty}"),
        // Named after the scale's type (MaterialX 1.39).
        ("normalmap", _) => "ND_normalmap_float".into(),
        // Multi-output nodes are named after what they split.
        (_, Some(i)) if ty == "multioutput" => format!("ND_{category}_{i}"),
        _ => format!("ND_{category}_{ty}"),
    }
}

/// The top-level Material Group for polygon tag `tag`.
pub fn group_for(scene: &Scene, tag: &str) -> Option<NodeId> {
    let mask = TagMask::Name(tag.to_owned());
    scene
        .shader_tree
        .nodes
        .iter()
        .find(|n| {
            n.parent.is_none()
                && matches!(&n.kind, NodeKind::Group(g) if g.tag_type == TagType::Material && g.tag == mask)
        })
        .map(|n| n.id)
}

/// Pictures that are scene images by their `Scene::images` id, so they
/// are copied once with the Preview Surface's.
fn scene_pictures(scene: &Scene, net: &mut Network) {
    for input in net.inputs_mut() {
        if let NetValue::File { path, .. } = &input.value
            && let Some(image) = scene.images.iter().find(|i| i.path == *path)
        {
            input.value = NetValue::Image(image.id);
        }
    }
}

/// A tree surface's network: its standard MaterialX form
/// (`materialx::tree_standard`: the tree's nodes through their standard
/// implementations onto an open_pbr_surface), with the Material Group's
/// free nodes (`free`: every one, wired or not, with its place) and each
/// picture by its `Scene::images` id where it is one. The notes list what
/// the form leaves out; an error names a node with no standard version
/// yet.
pub fn tree_network(
    scene: &Scene,
    surface: &operandi_core::shader_eval::Principled,
    name: &str,
    free: Option<&operandi_core::node_view::GroupNodes>,
) -> Result<(Network, Vec<String>), String> {
    let (text, notes) =
        operandi_core::materialx::tree_standard::document_with(surface, scene, name, free)?;
    let doc = operandi_core::materialx::doc::parse(&text)?;
    let material = doc
        .all("surfacematerial")
        .next()
        .ok_or("the tree form has no material")?
        .name
        .clone();
    let mut net =
        crate::materialx_graph_out::flat_network(&doc, &material, std::path::Path::new(""))?;
    scene_pictures(scene, &mut net);
    Ok((net, notes))
}

/// The network of the Material Group for polygon tag `tag` when the group
/// is shown by a graph layer: a MaterialX graph layer's graph as it is, or
/// a Blender Node Material layer's graph as the MaterialX nodes its nodes
/// map to (with notes on what doesn't map). `None` for any other group;
/// an error says why the graph couldn't be written.
pub fn graph_network(scene: &Scene, tag: &str) -> Result<Option<(Network, Vec<String>)>, String> {
    let Some(group) = group_for(scene, tag) else {
        return Ok(None);
    };
    let found = match crate::materialx_graph_out::group_network(scene, group)? {
        Some(net) => Some((net, Vec::new())),
        None => crate::cycles_mtlx::group_network(scene, group)?,
    };
    Ok(found.map(|(mut net, notes)| {
        scene_pictures(scene, &mut net);
        (net, notes)
    }))
}

/// The MaterialX network of the Material Group for polygon tag `tag`, or
/// `None` when the scene has no group for it: the group's Material and
/// Image Map layers (MX-05), with what that leaves out. An error says why
/// the group couldn't be written.
pub fn network(scene: &Scene, tag: &str) -> Result<Option<(Network, Vec<String>)>, String> {
    let Some(group) = group_for(scene, tag) else {
        return Ok(None);
    };
    let (text, losses) =
        operandi_core::materialx::export::material_x(scene, group, std::path::Path::new(""))?;
    let doc = operandi_core::materialx::doc::parse(&text)?;
    let material = doc
        .all("surfacematerial")
        .next()
        .ok_or("the MaterialX writer gave no material")?;
    let terminal = material
        .input("surfaceshader")
        .and_then(|p| p.nodename.clone())
        .ok_or("the MaterialX material has no surface shader")?;
    let mut nodes = Vec::new();
    for e in &doc.elements {
        if matches!(
            e.category.as_str(),
            "surfacematerial" | "nodedef" | "nodegraph"
        ) {
            continue;
        }
        let in_ty = e.input("in").map(|p| p.ty.as_str());
        let mut inputs = Vec::new();
        for p in &e.inputs {
            let value = if let Some(n) = &p.nodename {
                NetValue::Node(n.clone())
            } else if p.ty == "filename" {
                let file = format!("{}{}", p.fileprefix, p.value.clone().unwrap_or_default());
                match scene.images.iter().find(|i| i.path == file) {
                    Some(image) => NetValue::Image(image.id),
                    None => NetValue::Text(file),
                }
            } else {
                match &p.value {
                    Some(v) => NetValue::Text(v.clone()),
                    None => continue,
                }
            };
            inputs.push(NetInput {
                name: p.name.clone(),
                ty: p.ty.clone(),
                value,
                colorspace: p
                    .colorspace
                    .clone()
                    .filter(|c| Some(c) != doc.colorspace.as_ref()),
            });
        }
        nodes.push(NetNode {
            name: e.name.clone(),
            id: nodedef(&e.category, &e.ty, in_ty),
            ty: e.ty.clone(),
            inputs,
            ..NetNode::default()
        });
    }
    Ok(Some((
        Network {
            nodes,
            graphs: Vec::new(),
            terminal,
        },
        losses,
    )))
}

/// The network as the JSON `ou_define_network` reads, with pictures as
/// the asset paths `files` gives (their ids' copied files).
pub fn json(network: &Network, files: &std::collections::BTreeMap<u64, String>) -> String {
    use serde_json::{Value, json};
    let input = |i: &NetInput| -> Option<Value> {
        let mut v = json!({"name": i.name, "type": i.ty});
        match &i.value {
            NetValue::Text(t) => v["value"] = t.clone().into(),
            NetValue::Node(node) => v["connect"] = node.clone().into(),
            NetValue::NodeOutput(node, output) => {
                v["connect"] = node.clone().into();
                v["output"] = output.clone().into();
            }
            NetValue::Interface(name) => v["interface"] = name.clone().into(),
            NetValue::Graph(graph, output) => {
                v["graph"] = graph.clone().into();
                v["output"] = output.clone().into();
            }
            NetValue::Image(id) | NetValue::File { key: id, .. } => {
                v["value"] = files.get(id)?.clone().into()
            }
        }
        if let Some(c) = &i.colorspace {
            v["colorspace"] = c.clone().into();
        }
        Some(v)
    };
    let pos = |v: &mut Value, p: Option<[f64; 2]>| {
        if let Some([x, y]) = p {
            v["pos"] = json!([x, y]);
        }
    };
    let nodes: Vec<Value> = network
        .nodes
        .iter()
        .map(|n| {
            let inputs: Vec<Value> = n.inputs.iter().filter_map(input).collect();
            let mut v = json!({"name": n.name, "id": n.id, "type": n.ty, "inputs": inputs});
            if let Some(g) = &n.graph {
                v["graph"] = g.clone().into();
            }
            if !n.outputs.is_empty() {
                v["outputs"] = n
                    .outputs
                    .iter()
                    .map(|o| json!({"name": o.name, "type": o.ty}))
                    .collect();
            }
            pos(&mut v, n.pos);
            v
        })
        .collect();
    let graphs: Vec<Value> = network
        .graphs
        .iter()
        .map(|g| {
            let inputs: Vec<Value> = g.inputs.iter().filter_map(input).collect();
            let outputs: Vec<Value> = g
                .outputs
                .iter()
                .filter_map(|o| {
                    input(&NetInput {
                        name: o.name.clone(),
                        ty: o.ty.clone(),
                        value: o.value.clone(),
                        colorspace: None,
                    })
                })
                .collect();
            let mut v = json!({"name": g.name, "inputs": inputs, "outputs": outputs});
            pos(&mut v, g.pos);
            v
        })
        .collect();
    json!({"nodes": nodes, "graphs": graphs, "terminal": network.terminal}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodedefs_follow_materialx_names() {
        assert_eq!(nodedef("image", "color3", None), "ND_image_color3");
        assert_eq!(
            nodedef("open_pbr_surface", "surfaceshader", None),
            "ND_open_pbr_surface_surfaceshader"
        );
        assert_eq!(
            nodedef("extract", "float", Some("color3")),
            "ND_extract_color3"
        );
        assert_eq!(
            nodedef("convert", "color3", Some("float")),
            "ND_convert_float_color3"
        );
        assert_eq!(
            nodedef("normalmap", "vector3", Some("vector3")),
            "ND_normalmap_float"
        );
        assert_eq!(
            nodedef("separate3", "multioutput", Some("color3")),
            "ND_separate3_color3"
        );
    }
}
