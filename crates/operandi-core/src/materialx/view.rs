//! A MaterialX material as boxes and links for the Node View panel (node
//! editor, A3, slice 1: a read-only view; docs/MATERIALX_GRAPHS.md).
//!
//! Every node the material reaches becomes a `GraphNode` whose `idname`
//! is `mtlx:<category>` (so it is never taken for one of the Cycles node
//! set's), with its set values as inputs and a link per connection.
//! Nodegraph nodes are named `<nodegraph>/<node>`; a nodegraph's own
//! inputs show on the nodes that read them. The material itself is the
//! last node, `<material>`, with its shader inputs.
use super::doc::{Document, Element, Port};
use crate::Result;
use crate::node_graph::{GraphLink, GraphMaterial, GraphNode, SocketRef, SocketType, SocketValue};
use std::collections::BTreeMap;

/// The prefix of a MaterialX node's `idname` in the view.
pub const PREFIX: &str = "mtlx:";

/// The category of a view node (`image` for `mtlx:image`), `None` for a
/// Cycles node.
pub fn category(idname: &str) -> Option<&str> {
    idname.strip_prefix(PREFIX)
}

fn kind(ty: &str) -> SocketType {
    match ty {
        "float" => SocketType::Value,
        "color3" | "color4" => SocketType::Rgba,
        "integer" => SocketType::Int,
        "boolean" => SocketType::Boolean,
        "surfaceshader" | "displacementshader" | "volumeshader" | "material" => SocketType::Shader,
        _ => SocketType::Vector,
    }
}

/// A port's value as the view shows it: numbers, or text.
fn value(p: &Port) -> Option<SocketValue> {
    let v = p.value.as_deref()?;
    let value = match super::doc::numbers(v) {
        Some(n) if !matches!(p.ty.as_str(), "string" | "filename") => serde_json::json!(n),
        _ if p.ty == "filename" => format!("{}{v}", p.fileprefix).into(),
        _ => v.to_owned().into(),
    };
    Some(SocketValue {
        kind: kind(&p.ty),
        value,
    })
}

/// Where a scope's names resolve: the top level, or a nodegraph.
#[derive(Clone, Copy)]
enum Scope<'a> {
    Top,
    Graph(&'a Element),
}

struct View<'a> {
    doc: &'a Document,
    nodes: Vec<GraphNode>,
    links: Vec<GraphLink>,
    done: BTreeMap<String, String>,
}

impl<'a> View<'a> {
    fn element(&self, scope: Scope<'a>, name: &str) -> Option<&'a Element> {
        match scope {
            Scope::Top => self.doc.element(name),
            Scope::Graph(g) => g.child(name),
        }
    }

    /// The node and output port `p` reads, written once; `None` for a
    /// plain value.
    fn source(&mut self, scope: Scope<'a>, p: &'a Port) -> Option<(String, String)> {
        if let Some(i) = &p.interfacename
            && let Scope::Graph(g) = scope
        {
            return self.source(Scope::Top, g.input(i)?);
        }
        if let Some(ng) = &p.nodegraph {
            let g = self
                .doc
                .elements
                .iter()
                .find(|e| e.category == "nodegraph" && &e.name == ng)?;
            let out = match &p.output {
                Some(o) => g.outputs.iter().find(|q| &q.name == o),
                None => g.outputs.first(),
            }?;
            return self.source(Scope::Graph(g), out);
        }
        let n = p.nodename.as_deref()?;
        let name = self.node(scope, n)?;
        Some((name, p.output.clone().unwrap_or_else(|| "out".into())))
    }

    /// Node `name` of `scope` in the view; its view name.
    fn node(&mut self, scope: Scope<'a>, name: &str) -> Option<String> {
        let view_name = match scope {
            Scope::Top => name.to_owned(),
            Scope::Graph(g) => format!("{}/{name}", g.name),
        };
        if let Some(n) = self.done.get(&view_name) {
            return Some(n.clone());
        }
        let el = self.element(scope, name)?;
        // Marked before its inputs, so a loop stops here.
        self.done.insert(view_name.clone(), view_name.clone());
        let node = self.box_of(scope, el, view_name.clone());
        self.nodes.push(node);
        Some(view_name)
    }

    fn box_of(&mut self, scope: Scope<'a>, el: &'a Element, name: String) -> GraphNode {
        let mut inputs = BTreeMap::new();
        for p in &el.inputs {
            match self.source(scope, p) {
                Some((from, output)) => self.links.push(GraphLink {
                    from: SocketRef::new(&from, &output),
                    to: SocketRef::new(&name, &p.name),
                    extra: Default::default(),
                }),
                None => {
                    // A nodegraph input's value shows on the node reading it.
                    let port = match (&p.interfacename, scope) {
                        (Some(i), Scope::Graph(g)) => g.input(i).unwrap_or(p),
                        _ => p,
                    };
                    if let Some(v) = value(port) {
                        inputs.insert(p.name.clone(), v);
                    }
                }
            }
        }
        let mut props = BTreeMap::new();
        if !el.ty.is_empty() {
            props.insert("type".into(), el.ty.clone().into());
        }
        GraphNode {
            name,
            idname: format!("{PREFIX}{}", el.category),
            props,
            image: None,
            inputs,
            extra: Default::default(),
        }
    }
}

/// Material `material` of `doc` (its first when `None`) as a view graph.
pub fn graph(doc: &Document, material: Option<&str>) -> Result<GraphMaterial> {
    let mat = doc
        .all("surfacematerial")
        .find(|m| material.is_none_or(|n| m.name == n))
        .ok_or("The MaterialX document has no such material")?;
    let mut v = View {
        doc,
        nodes: Vec::new(),
        links: Vec::new(),
        done: BTreeMap::new(),
    };
    let out = v.box_of(Scope::Top, mat, mat.name.clone());
    v.nodes.push(out);
    // Nodes that feed nothing still show (NE-14), when the file holds
    // only this material.
    if doc.all("surfacematerial").count() == 1 {
        for name in super::write::dangling(doc) {
            v.node(Scope::Top, &name);
        }
    }
    Ok(GraphMaterial {
        name: mat.name.clone(),
        output: mat.name.clone(),
        nodes: v.nodes,
        links: v.links,
        exposed: Vec::new(),
        groups_used: Vec::new(),
        extra: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materialx::doc;

    #[test]
    fn nodes_links_and_nodegraphs_become_boxes() {
        let d = doc::parse(
            r#"<materialx version="1.39">
  <nodegraph name="NG">
    <input name="tint" type="color3" value="0.5, 0.5, 0.5" />
    <image name="img" type="color3">
      <input name="file" type="filename" value="a.png" />
    </image>
    <multiply name="m" type="color3">
      <input name="in1" type="color3" nodename="img" />
      <input name="in2" type="color3" interfacename="tint" />
    </multiply>
    <output name="out" type="color3" nodename="m" />
  </nodegraph>
  <noise3d name="unused" type="float" />
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodegraph="NG" output="out" />
    <input name="specular_roughness" type="float" value="0.3" />
  </open_pbr_surface>
  <surfacematerial name="M" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
</materialx>"#,
        )
        .unwrap();
        let g = graph(&d, None).unwrap();
        let names: Vec<&str> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        // A node feeding nothing shows too, after the material (NE-14).
        assert_eq!(names, ["NG/img", "NG/m", "s", "M", "unused"]);
        assert_eq!(category(&g.nodes[1].idname), Some("multiply"));
        // The nodegraph's input shows on the node that reads it.
        assert_eq!(g.nodes[1].inputs["in2"].numbers(), vec![0.5, 0.5, 0.5]);
        assert_eq!(g.nodes[0].inputs["file"].value, "a.png");
        assert_eq!(g.nodes[2].inputs["specular_roughness"].numbers(), vec![0.3]);
        let link = |from: &str, to: &str| {
            g.links
                .iter()
                .any(|l| l.from.node() == from && l.to.node() == to)
        };
        assert!(link("NG/img", "NG/m") && link("NG/m", "s") && link("s", "M"));
        assert_eq!(g.links.len(), 3);
    }

    #[test]
    fn a_tree_material_shows_its_operandi_nodes() {
        use crate::shader_eval::{Expr, ImageLookup, Input, Principled};
        use crate::shader_tree::Filtering;
        let image = Expr::ImageColor(Box::new(ImageLookup::uv(3, Filtering::Bilinear)));
        let p = Principled {
            inputs: vec![
                (Input::BaseColor, image.clone()),
                (Input::Roughness, Expr::Luma(Box::new(image))),
            ],
            shader: Default::default(),
            bump: None,
            normal_map: None,
            displacement: None,
            coat_bump: 0.,
            see_through: None,
            rounded_edge: None,
            absorption: None,
            scatter: None,
            thin_walled: false,
            glass_principled: false,
            node: None,
            values: Vec::new(),
        };
        let text = super::super::tree::write(&p, &crate::Scene::default(), "Wood");
        let g = graph(&doc::parse(&text).unwrap(), None).unwrap();
        let cats: Vec<&str> = g.nodes.iter().filter_map(|n| category(&n.idname)).collect();
        for want in [
            "operandi_texcoord",
            "operandi_image",
            "operandi_luma",
            "operandi_surface",
        ] {
            assert!(cats.contains(&want), "{want}: {cats:?}");
        }
        // The shared picture is one box feeding two.
        assert_eq!(cats.iter().filter(|c| **c == "operandi_image").count(), 1);
    }

    #[test]
    fn a_loop_or_missing_node_doesnt_hang() {
        let d = doc::parse(
            r#"<materialx version="1.39">
  <add name="a" type="float"><input name="in1" type="float" nodename="b" /></add>
  <add name="b" type="float"><input name="in1" type="float" nodename="a" /><input name="in2" type="float" nodename="gone" /></add>
  <open_pbr_surface name="s" type="surfaceshader"><input name="base_weight" type="float" nodename="a" /></open_pbr_surface>
  <surfacematerial name="M" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s" /></surfacematerial>
</materialx>"#,
        )
        .unwrap();
        let g = graph(&d, Some("M")).unwrap();
        assert_eq!(g.nodes.len(), 4);
        assert!(graph(&d, Some("Nope")).is_err());
    }
}
