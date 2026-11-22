//! USD import of a MaterialX network as it was written (usd-gaps-3): for
//! materials Operandi wrote (customData operandi:materialxGraph), the
//! network's Shaders and NodeGraphs below the material, with their direct
//! connections, become the same MaterialX document the export started
//! from: NodeGraph prims are nodegraphs with their interface inputs and
//! outputs, wires to a NodeGraph's interface are `interfacename`, wires to
//! a NodeGraph output are `nodegraph`/`output`, multi-output nodes keep
//! their outputs, `ui:nodegraph:node:pos` is xpos/ypos, and nodes that
//! feed nothing still come. Other programs' networks keep the resolved
//! reading (`import_materials::document`), which is also the fallback
//! when this one can't be made.
use crate::import_materials::{Pictures, category, mx_colour_space, mx_type};
use crate::stage::{GraphPrim, MaterialData, ShaderInput, Source};
use operandi_core::materialx::doc::{Document, Element, Port};
use std::collections::BTreeMap;

/// The parent path of `path` ("" at the material's top level) and its
/// last part.
fn split(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", path),
    }
}

/// How a port in scope `scope` reads `source`: (nodename, nodegraph,
/// output, interfacename).
type Reading = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `material`'s network as a MaterialX document, or why it can't be one.
pub fn document(material: &MaterialData, pictures: &mut Pictures) -> Result<Document, String> {
    let prims: BTreeMap<&str, &GraphPrim> = material
        .graph
        .iter()
        .map(|p| (p.path.as_str(), p))
        .collect();
    let (terminal_scope, terminal) = split(&material.graph_terminal);
    if material.graph_terminal.is_empty() || !terminal_scope.is_empty() {
        return Err("the surface shader isn't at the material's top level".into());
    }
    let reading = |scope: &str, s: &Source| -> Result<Reading, String> {
        let src = prims
            .get(s.path.as_str())
            .ok_or_else(|| format!("a wire from {} outside the network", s.path))?;
        let (parent, name) = split(&s.path);
        if s.kind == "input" {
            // The enclosing NodeGraph's interface.
            if src.graph && s.path == scope {
                return Ok((None, None, None, Some(s.name.clone())));
            }
            return Err(format!("a wire from {}'s input {}", s.path, s.name));
        }
        if parent != scope {
            return Err(format!("a wire from {} across NodeGraphs", s.path));
        }
        if src.graph {
            return Ok((None, Some(name.into()), Some(s.name.clone()), None));
        }
        let output = (src.outputs.len() > 1 || s.name != "out").then(|| s.name.clone());
        Ok((Some(name.into()), None, output, None))
    };
    let port = |scope: &str,
                prim: &GraphPrim,
                input: &ShaderInput,
                pictures: &mut Pictures|
     -> Result<Option<Port>, String> {
        if input.unsupported.is_some() {
            return Ok(None);
        }
        let ty = mx_type(&input.ty).to_string();
        let mut p = Port {
            name: input.name.clone(),
            ty: ty.clone(),
            ..Port::default()
        };
        if let Some(s) = &input.source {
            (p.nodename, p.nodegraph, p.output, p.interfacename) = reading(scope, s)?;
        } else if ty == "filename" {
            p.value = Some(pictures.file(
                input.value.as_deref().unwrap_or_default(),
                input.resolved.as_deref().unwrap_or_default(),
                input.anchored.as_deref().unwrap_or_default(),
            ));
        } else {
            p.value = input.value.clone();
            if p.value.is_none() {
                return Ok(None);
            }
        }
        p.colorspace = input
            .colorspace
            .as_deref()
            .and_then(mx_colour_space)
            .or_else(|| mx_colour_space(&prim.colorspace));
        Ok(Some(p))
    };
    let place = |e: &mut Element, prim: &GraphPrim| {
        if let Some([x, y]) = prim.pos {
            e.attrs.insert("xpos".into(), format!("{}", x as f32));
            e.attrs.insert("ypos".into(), format!("{}", y as f32));
        }
    };
    let mut top: Vec<Element> = Vec::new();
    let mut graphs: BTreeMap<String, Element> = BTreeMap::new();
    for prim in &material.graph {
        let (scope, name) = split(&prim.path);
        if prim.graph {
            if !scope.is_empty() {
                return Err(format!("NodeGraph {} inside a NodeGraph", prim.path));
            }
            let mut e = Element {
                category: "nodegraph".into(),
                name: name.into(),
                ..Element::default()
            };
            place(&mut e, prim);
            for input in &prim.inputs {
                if let Some(p) = port("", prim, input, pictures)? {
                    e.inputs.push(p);
                }
            }
            for o in &prim.outputs {
                let mut p = Port {
                    name: o.name.clone(),
                    ty: mx_type(&o.ty).to_string(),
                    ..Port::default()
                };
                if let Some(s) = &o.source {
                    (p.nodename, p.nodegraph, p.output, p.interfacename) = reading(&prim.path, s)?;
                }
                e.outputs.push(p);
            }
            // Keep the children met before the graph itself.
            if let Some(old) = graphs.insert(name.into(), e) {
                graphs.get_mut(name).expect("just put").children = old.children;
            }
            continue;
        }
        let (cat, from_id) = category(&prim.id);
        let multi = prim.outputs.len() > 1;
        let ty = if multi {
            "multioutput".to_string()
        } else {
            // Shader outputs are tokens in USD: the id names the type.
            match prim.outputs.first().map(|o| mx_type(&o.ty)) {
                Some("string") | None if !from_id.is_empty() => from_id,
                Some(t) => t.to_string(),
                None => from_id,
            }
        };
        let mut e = Element {
            category: cat,
            name: name.into(),
            ty,
            colorspace: mx_colour_space(&prim.colorspace),
            ..Element::default()
        };
        place(&mut e, prim);
        for input in &prim.inputs {
            if let Some(p) = port(scope, prim, input, pictures)? {
                e.inputs.push(p);
            }
        }
        if multi {
            e.outputs = prim
                .outputs
                .iter()
                .map(|o| Port {
                    name: o.name.clone(),
                    ty: mx_type(&o.ty).to_string(),
                    ..Port::default()
                })
                .collect();
        }
        if scope.is_empty() {
            top.push(e);
        } else {
            if scope.contains('/') {
                return Err(format!("{} sits deeper than one NodeGraph", prim.path));
            }
            graphs
                .entry(scope.to_string())
                .or_insert_with(|| Element {
                    category: "nodegraph".into(),
                    name: scope.into(),
                    ..Element::default()
                })
                .children
                .push(e);
        }
    }
    if !top.iter().any(|e| e.name == terminal) {
        return Err(format!("no surface shader {terminal}"));
    }
    let mut elements: Vec<Element> = graphs.into_values().collect();
    elements.extend(top);
    elements.push(Element {
        category: "surfacematerial".into(),
        name: material.name.clone(),
        ty: "material".into(),
        inputs: vec![Port {
            name: "surfaceshader".into(),
            ty: "surfaceshader".into(),
            nodename: Some(terminal.into()),
            ..Port::default()
        }],
        ..Element::default()
    });
    Ok(Document {
        version: "1.39".into(),
        colorspace: Some("lin_rec709".into()),
        elements,
    })
}
