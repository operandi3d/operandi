//! USD export of MaterialX graph layers (materials architecture, slice 2;
//! docs/MATERIALX_GRAPHS.md): a Material Group shown by a Node Material
//! layer whose graph is kept as MaterialX writes that graph as the
//! material's UsdShade MaterialX network, the form Hydra render delegates
//! and other apps read.
//!
//! The graph goes out as it is (usd-gaps-3): each MaterialX nodegraph is
//! a UsdShade NodeGraph prim with its interface inputs (`interfacename`
//! wires connect to them) and outputs, multi-output nodes keep their named
//! outputs, nodes that feed nothing still go (a node editor's spare
//! nodes), and every node's xpos/ypos is its UsdUI place. Only custom
//! nodes the document implements as nodegraphs (the shader tree's
//! `operandi_*` nodes, which no other program knows) are written as their
//! implementation's nodes, named `<node>_<inner node>`. A graph this can't
//! write (UsdPreviewSurface nodes, which go as Preview Surface anyway, a
//! nodegraph inside a nodegraph, custom nodes inside custom nodes) is left
//! out with the reason, and the material keeps its Preview Surface.
use crate::materialx_out::{
    NetGraph, NetGraphOutput, NetInput, NetNode, NetOutput, NetValue, Network, nodedef,
};
use operandi_core::Scene;
use operandi_core::materialx::doc::{Document, Element, Port};
use operandi_core::shader_tree::{NodeId, NodeKind};
use std::collections::BTreeSet;
use std::path::Path;

/// The key a graph picture's copied file is listed under: never a
/// `Scene::images` id (those count up from 1; the top bit is set here).
pub fn picture_key(path: &str) -> u64 {
    // FNV-1a: stable across runs and builds.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h | 1 << 63
}

/// The network of group `group` when its first enabled layer is a fully
/// opaque MaterialX graph layer; `None` for any other group.
pub fn group_network(scene: &Scene, group: NodeId) -> Result<Option<Network>, String> {
    let tree = &scene.shader_tree;
    let Some(top) = tree.children(Some(group)).find(|n| n.layer.enable) else {
        return Ok(None);
    };
    let NodeKind::NodeMaterial(m) = &top.kind else {
        return Ok(None);
    };
    let Some(mx) = m
        .graph
        .and_then(|g| tree.node_graphs.get(g))
        .and_then(|g| g.materialx.as_ref())
    else {
        return Ok(None);
    };
    if top.layer.opacity < 1. {
        return Err("a MaterialX graph layer below full opacity".into());
    }
    let doc = mx.document()?;
    flat_network(&doc, &mx.material, Path::new(&mx.folder)).map(Some)
}

/// Where a reference resolves: the top level, inside a nodegraph, or
/// inside the implementation of a custom node (`caller`, itself at the
/// top level or in a nodegraph, `outer`).
#[derive(Clone, Copy)]
enum Scope<'a> {
    Top,
    Graph(&'a Element),
    Impl {
        graph: &'a Element,
        def: &'a Element,
        caller: &'a Element,
        outer: Option<&'a Element>,
    },
}
impl<'a> Scope<'a> {
    fn outer(outer: Option<&'a Element>) -> Self {
        match outer {
            Some(g) => Scope::Graph(g),
            None => Scope::Top,
        }
    }
    /// The NodeGraph prim nodes of this scope sit in.
    fn graph_name(self) -> Option<String> {
        match self {
            Scope::Top => None,
            Scope::Graph(g) => Some(g.name.clone()),
            Scope::Impl { outer, .. } => outer.map(|g| g.name.clone()),
        }
    }
}

struct Flat<'a> {
    doc: &'a Document,
    folder: &'a Path,
    /// Nodes written, by (graph, name).
    done: BTreeSet<(Option<String>, String)>,
    graphs_done: BTreeSet<String>,
    nodes: Vec<NetNode>,
    graphs: Vec<NetGraph>,
    busy: Vec<String>,
}

/// An element's xpos/ypos.
fn place(el: &Element) -> Option<[f64; 2]> {
    let x = el.attrs.get("xpos")?.trim().parse().ok()?;
    let y = el.attrs.get("ypos")?.trim().parse().ok()?;
    Some([x, y])
}

/// Elements that aren't nodes.
const NOT_NODES: [&str; 7] = [
    "nodedef",
    "nodegraph",
    "surfacematerial",
    "volumematerial",
    "typedef",
    "look",
    "backdrop",
];

/// Material `material` of `doc` as a UsdShade network (the name says
/// "flat" for history: only custom nodes are flattened now).
pub fn flat_network(doc: &Document, material: &str, folder: &Path) -> Result<Network, String> {
    let mat = doc
        .all("surfacematerial")
        .find(|m| m.name == material)
        .ok_or_else(|| format!("no material {material}"))?;
    let shader = mat
        .input("surfaceshader")
        .ok_or("the material has no surface shader")?;
    let mut f = Flat {
        doc,
        folder,
        done: BTreeSet::new(),
        graphs_done: BTreeSet::new(),
        nodes: Vec::new(),
        graphs: Vec::new(),
        busy: Vec::new(),
    };
    let terminal = match f.value(Scope::Top, shader)? {
        Some(NetValue::Node(n)) => n,
        _ => return Err("the material's surface shader isn't a node".into()),
    };
    // A document of one material keeps what feeds nothing too: a node
    // editor's spare nodes and graphs (NE-10, NE-14).
    if doc.all("surfacematerial").count() == 1 {
        let mut read = BTreeSet::new();
        for e in &doc.elements {
            reads(e, &mut read);
        }
        for e in &doc.elements {
            if read.contains(e.name.as_str()) {
                continue;
            }
            if e.category == "nodegraph" {
                // Implementations of custom nodes aren't graphs of the
                // material.
                if !e.attrs.contains_key("nodedef") {
                    f.graph(e)?;
                }
            } else if !NOT_NODES.contains(&e.category.as_str())
                && !matches!(
                    e.ty.as_str(),
                    "" | "surfaceshader" | "displacementshader" | "volumeshader" | "material"
                )
            {
                f.node(Scope::Top, &e.name, None)?;
            }
        }
    }
    Ok(Network {
        nodes: f.nodes,
        graphs: f.graphs,
        terminal,
    })
}

/// The top-level names `e` reads (its inputs' nodes and nodegraphs, and
/// its children's references out of their graph).
fn reads<'a>(e: &'a Element, out: &mut BTreeSet<&'a str>) {
    for p in e.inputs.iter().chain(&e.outputs) {
        if let Some(n) = &p.nodename {
            out.insert(n);
        }
        if let Some(g) = &p.nodegraph {
            out.insert(g);
        }
    }
    for c in &e.children {
        reads(c, out);
    }
}

impl<'a> Flat<'a> {
    /// What `p` gives in `scope`: a node of the network, a value, or
    /// nothing (left at the node's default).
    fn value(&mut self, scope: Scope<'a>, p: &'a Port) -> Result<Option<NetValue>, String> {
        if let Some(i) = &p.interfacename {
            return match scope {
                Scope::Top => Err(format!("{} reads an interface at the top level", p.name)),
                Scope::Graph(ng) => Ok(ng.input(i).map(|_| NetValue::Interface(i.clone()))),
                // The custom node's own input, else its definition's
                // default.
                Scope::Impl {
                    def, caller, outer, ..
                } => match caller.input(i).or_else(|| def.input(i)) {
                    Some(q) => self.value(Scope::outer(outer), q),
                    None => Ok(None),
                },
            };
        }
        if let Some(g) = &p.nodegraph {
            if !matches!(scope, Scope::Top) {
                return Err(format!("{}: a nodegraph inside a nodegraph", p.name));
            }
            let doc = self.doc;
            let ng = doc
                .elements
                .iter()
                .find(|e| e.category == "nodegraph" && &e.name == g)
                .ok_or_else(|| format!("no nodegraph {g}"))?;
            let out = match &p.output {
                Some(o) => ng.outputs.iter().find(|q| &q.name == o),
                None => ng.outputs.first(),
            }
            .ok_or_else(|| format!("nodegraph {g} has no such output"))?;
            self.graph(ng)?;
            return Ok(Some(NetValue::Graph(g.clone(), out.name.clone())));
        }
        if let Some(n) = &p.nodename {
            return self.node(scope, n, p.output.as_deref()).map(Some);
        }
        Ok(match &p.value {
            None => None,
            Some(v) if p.ty == "filename" => {
                if v.is_empty() {
                    None
                } else {
                    let path = self
                        .folder
                        .join(format!("{}{v}", p.fileprefix))
                        .to_string_lossy()
                        .into_owned();
                    Some(NetValue::File {
                        key: picture_key(&path),
                        path,
                    })
                }
            }
            Some(v) => Some(NetValue::Text(v.clone())),
        })
    }

    /// Nodegraph `ng` as a NodeGraph prim, written once: its interface
    /// inputs, every node in it and its outputs.
    fn graph(&mut self, ng: &'a Element) -> Result<(), String> {
        if !self.graphs_done.insert(ng.name.clone()) {
            return Ok(());
        }
        let at = self.graphs.len();
        self.graphs.push(NetGraph {
            name: ng.name.clone(),
            pos: place(ng),
            ..NetGraph::default()
        });
        let mut inputs = Vec::new();
        for q in &ng.inputs {
            let Some(value) = self.value(Scope::Top, q)? else {
                continue;
            };
            inputs.push(self.input(q, value, None));
        }
        let mut outputs = Vec::new();
        for o in &ng.outputs {
            let Some(value) = self.value(Scope::Graph(ng), o)? else {
                continue;
            };
            outputs.push(NetGraphOutput {
                name: o.name.clone(),
                ty: o.ty.clone(),
                value,
            });
        }
        // Its nodes that no output reads still belong to it.
        for c in &ng.children {
            if !NOT_NODES.contains(&c.category.as_str()) {
                self.node(Scope::Graph(ng), &c.name, None)?;
            }
        }
        self.graphs[at].inputs = inputs;
        self.graphs[at].outputs = outputs;
        Ok(())
    }

    /// An input of the network: `p` with `value`, keeping its colour
    /// space where it differs from the document's (`space` overrides it).
    fn input(&self, p: &Port, value: NetValue, space: Option<Option<String>>) -> NetInput {
        let space = space.unwrap_or_else(|| p.colorspace.clone());
        NetInput {
            name: p.name.clone(),
            ty: p.ty.clone(),
            value,
            colorspace: space
                .filter(|_| matches!(p.ty.as_str(), "filename" | "color3" | "color4"))
                .filter(|c| Some(c) != self.doc.colorspace.as_ref()),
        }
    }

    /// Node `name` of `scope` written once: what reads its `output`
    /// (`None`: its only one).
    fn node(
        &mut self,
        scope: Scope<'a>,
        name: &str,
        output: Option<&str>,
    ) -> Result<NetValue, String> {
        let (elements, net_name) = match scope {
            Scope::Top => (&self.doc.elements[..], name.to_string()),
            Scope::Graph(ng) => (&ng.children[..], name.to_string()),
            // An implementation's nodes are named after the node using it.
            Scope::Impl { graph, caller, .. } => {
                (&graph.children[..], format!("{}_{name}", caller.name))
            }
        };
        let el = elements
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("no node {name}"))?;
        let multi = el.outputs.len() > 1 || el.ty == "multioutput";
        let reading = |n: String| match output {
            Some(o) if multi => NetValue::NodeOutput(n, o.to_string()),
            _ => NetValue::Node(n),
        };
        let key = (scope.graph_name(), net_name.clone());
        if self.done.contains(&key) {
            return Ok(reading(net_name));
        }
        if self.busy.contains(&net_name) || self.busy.len() > 256 {
            return Err(format!("node {name} feeds itself"));
        }
        if el.category.starts_with("Usd") {
            return Err("its nodes are UsdPreviewSurface ones".into());
        }
        if el.category == "nodegraph" {
            return Err(format!("{name} is a nodegraph used as a node"));
        }
        // A custom node the document implements as a nodegraph (the shader
        // tree's operandi_* nodes) is written as its implementation's nodes.
        let doc = self.doc;
        if let Some(def) = doc.all("nodedef").find(|d| {
            d.attrs.get("node").map(String::as_str) == Some(el.category.as_str())
                && d.outputs.iter().any(|o| o.ty == el.ty)
        }) {
            let graph = doc
                .all("nodegraph")
                .find(|g| g.attrs.get("nodedef") == Some(&def.name))
                .ok_or_else(|| format!("{name} is a custom node with no implementation"))?;
            let outer = match scope {
                Scope::Top => None,
                Scope::Graph(g) => Some(g),
                Scope::Impl { .. } => {
                    return Err(format!("{name}: custom nodes inside custom nodes"));
                }
            };
            let out = match output {
                Some(o) => graph.outputs.iter().find(|q| q.name == o),
                None => graph.outputs.first(),
            }
            .ok_or_else(|| format!("{}'s implementation has no such output", el.category))?;
            self.busy.push(net_name.clone());
            let inner = Scope::Impl {
                graph,
                def,
                caller: el,
                outer,
            };
            let v = self.value(inner, out)?;
            self.busy.pop();
            return match v {
                Some(v @ (NetValue::Node(_) | NetValue::NodeOutput(..))) => Ok(v),
                _ => Err(format!(
                    "{}'s implementation doesn't end in a node",
                    el.category
                )),
            };
        }
        self.busy.push(net_name.clone());
        let mut inputs = Vec::new();
        for p in &el.inputs {
            let Some(value) = self.value(scope, p)? else {
                continue;
            };
            // A custom node's picture keeps the colour space set on the
            // node using it.
            let space = match (scope, &p.interfacename) {
                (Scope::Impl { caller, .. }, Some(i)) => Some(
                    caller
                        .input(i)
                        .and_then(|q| q.colorspace.clone())
                        .or_else(|| p.colorspace.clone()),
                ),
                _ => None,
            };
            inputs.push(self.input(p, value, space));
        }
        self.busy.pop();
        let in_ty = el.input("in").map(|p| p.ty.as_str());
        let id = operandi_core::materialx::stdlib::standard_nodedef(el)
            .map_or_else(|| nodedef(&el.category, &el.ty, in_ty), str::to_owned);
        self.nodes.push(NetNode {
            name: net_name.clone(),
            id,
            ty: if multi {
                "multioutput".into()
            } else {
                el.ty.clone()
            },
            inputs,
            outputs: if multi {
                el.outputs
                    .iter()
                    .map(|o| NetOutput {
                        name: o.name.clone(),
                        ty: o.ty.clone(),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            graph: scope.graph_name(),
            // Implementation nodes have no place of their own.
            pos: match scope {
                Scope::Impl { .. } => None,
                _ => place(el),
            },
        });
        self.done.insert(key);
        Ok(reading(net_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::materialx::doc;

    #[test]
    fn nodegraphs_become_node_graphs_with_their_interface() {
        let d = doc::parse(
            r#"<materialx version="1.39" colorspace="lin_rec709">
  <nodegraph name="NG" xpos="2" ypos="3">
    <input name="tint" type="color3" value="0.5, 0.5, 0.5" />
    <image name="img" type="color3">
      <input name="file" type="filename" value="tex/a.png" colorspace="srgb_texture" />
    </image>
    <multiply name="m" type="color3">
      <input name="in1" type="color3" nodename="img" />
      <input name="in2" type="color3" interfacename="tint" />
    </multiply>
    <output name="out" type="color3" nodename="m" />
  </nodegraph>
  <noise3d name="spare" type="float" xpos="1.5" ypos="-2" />
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodegraph="NG" output="out" />
  </open_pbr_surface>
  <surfacematerial name="M" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#,
        )
        .unwrap();
        let net = flat_network(&d, "M", Path::new("/lib")).unwrap();
        let names: Vec<(&str, Option<&str>)> = net
            .nodes
            .iter()
            .map(|n| (n.name.as_str(), n.graph.as_deref()))
            .collect();
        assert_eq!(
            names,
            [
                ("img", Some("NG")),
                ("m", Some("NG")),
                ("s", None),
                ("spare", None)
            ]
        );
        assert_eq!(net.terminal, "s");
        let m = &net.nodes[1];
        assert_eq!(m.id, "ND_multiply_color3");
        assert_eq!(m.inputs[0].value, NetValue::Node("img".into()));
        assert_eq!(m.inputs[1].value, NetValue::Interface("tint".into()));
        let g = &net.graphs[0];
        assert_eq!(g.name, "NG");
        assert_eq!(g.pos, Some([2., 3.]));
        assert_eq!(g.inputs[0].value, NetValue::Text("0.5, 0.5, 0.5".into()));
        assert_eq!(g.outputs[0].value, NetValue::Node("m".into()));
        assert_eq!(
            net.nodes[2].inputs[0].value,
            NetValue::Graph("NG".into(), "out".into())
        );
        assert_eq!(net.nodes[3].pos, Some([1.5, -2.]));
        let file = &net.nodes[0].inputs[0];
        assert_eq!(
            file.value,
            NetValue::File {
                key: picture_key("/lib/tex/a.png"),
                path: "/lib/tex/a.png".into()
            }
        );
        assert_eq!(file.colorspace.as_deref(), Some("srgb_texture"));
        assert!(picture_key("/lib/tex/a.png") > 1 << 62);
    }

    #[test]
    fn multi_output_nodes_keep_their_outputs() {
        let d = doc::parse(
            r#"<materialx version="1.39">
  <separate3 name="sep" type="multioutput"><input name="in" type="color3" value="1, 0, 0" /><output name="outr" type="float" /><output name="outg" type="float" /></separate3>
  <open_pbr_surface name="s" type="surfaceshader"><input name="base_metalness" type="float" nodename="sep" output="outr" /></open_pbr_surface>
  <surfacematerial name="M" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s" /></surfacematerial>
</materialx>"#,
        )
        .unwrap();
        let net = flat_network(&d, "M", Path::new("")).unwrap();
        let sep = &net.nodes[0];
        assert_eq!(sep.id, "ND_separate3_color3");
        assert_eq!(sep.ty, "multioutput");
        assert_eq!(sep.outputs.len(), 2);
        assert_eq!(
            net.nodes[1].inputs[0].value,
            NetValue::NodeOutput("sep".into(), "outr".into())
        );
    }
}
