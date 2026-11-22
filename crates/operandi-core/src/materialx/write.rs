//! A MaterialX document back as text, for keeping a material's graph in
//! the scene (materials architecture, slice 1).
//!
//! [`material`] writes one material and only what it reads: the
//! `surfacematerial`, the nodes and nodegraphs it reaches, and every node
//! definition the document implements as a nodegraph (they are small, and
//! a custom node needs its own). The text is normalised as it goes:
//! inherited `fileprefix` folds into each filename and every colour or
//! filename input carries its effective colour space, so the stored text
//! reads the same with no context. Picture paths stay as the file had them
//! (relative to the document's folder, which the scene stores beside it).
use super::doc::{Document, Element, Port};
use std::collections::BTreeSet;
use std::fmt::Write as _;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn port(out: &mut String, tag: &str, p: &Port, indent: &str) {
    let _ = write!(out, "{indent}<{tag} name=\"{}\"", esc(&p.name));
    if !p.ty.is_empty() {
        let _ = write!(out, " type=\"{}\"", esc(&p.ty));
    }
    let mut attr = |k: &str, v: &Option<String>| {
        if let Some(v) = v {
            let _ = write!(out, " {k}=\"{}\"", esc(v));
        }
    };
    attr("nodename", &p.nodename);
    attr("nodegraph", &p.nodegraph);
    attr("output", &p.output);
    attr("interfacename", &p.interfacename);
    let value = match (&p.value, p.ty.as_str()) {
        (Some(v), "filename") => Some(format!("{}{v}", p.fileprefix)),
        (v, _) => v.clone(),
    };
    attr("value", &value);
    if matches!(p.ty.as_str(), "filename" | "color3" | "color4") {
        attr("colorspace", &p.colorspace);
    }
    attr("unit", &p.unit);
    attr("unittype", &p.unittype);
    out.push_str(" />\n");
}

/// One element as MaterialX text.
pub fn element(out: &mut String, e: &Element, indent: &str) {
    let _ = write!(out, "{indent}<{} name=\"{}\"", e.category, esc(&e.name));
    if !e.ty.is_empty() {
        let _ = write!(out, " type=\"{}\"", esc(&e.ty));
    }
    for (k, v) in &e.attrs {
        // Folded into the ports, or written above.
        if matches!(k.as_str(), "name" | "type" | "fileprefix") {
            continue;
        }
        let _ = write!(out, " {k}=\"{}\"", esc(v));
    }
    if e.inputs.is_empty() && e.outputs.is_empty() && e.children.is_empty() {
        out.push_str(" />\n");
        return;
    }
    out.push_str(">\n");
    let inner = format!("{indent}  ");
    for p in &e.inputs {
        port(out, "input", p, &inner);
    }
    for c in &e.children {
        element(out, c, &inner);
    }
    for p in &e.outputs {
        port(out, "output", p, &inner);
    }
    let _ = writeln!(out, "{indent}</{}>", e.category);
}

/// The top-level names `e` reads.
fn reads(e: &Element, out: &mut Vec<String>) {
    for p in e.inputs.iter().chain(&e.outputs) {
        out.extend(p.nodegraph.iter().cloned());
        // Inside a nodegraph a nodename means a sibling; only top-level
        // elements read top-level nodes by name.
        out.extend(p.nodename.iter().cloned());
    }
}

/// Material `material` of `doc` as a standalone document.
pub fn material(doc: &Document, material: &str) -> String {
    let mut keep: BTreeSet<&str> = BTreeSet::new();
    let mut todo = vec![material.to_string()];
    // A file of one material keeps its nodes that feed nothing too (node
    // editing, NE-10/NE-14: a free node survives export and re-import).
    if doc.all("surfacematerial").count() == 1 {
        todo.extend(dangling(doc));
    }
    while let Some(name) = todo.pop() {
        let Some(e) = doc.elements.iter().find(|e| e.name == name) else {
            continue;
        };
        if !keep.insert(e.name.as_str()) {
            continue;
        }
        let mut more = Vec::new();
        reads(e, &mut more);
        // A nodegraph's own inputs may read top-level nodes too.
        if e.category == "nodegraph" {
            for c in &e.children {
                for p in &c.inputs {
                    out_of_graph(e, p, &mut more);
                }
            }
        }
        todo.extend(more);
    }
    let mut out = String::from("<?xml version=\"1.0\"?>\n<materialx");
    let version = if doc.version.is_empty() {
        "1.39"
    } else {
        doc.version.as_str()
    };
    let _ = write!(out, " version=\"{}\"", esc(version));
    if let Some(cs) = &doc.colorspace {
        let _ = write!(out, " colorspace=\"{}\"", esc(cs));
    }
    out.push_str(">\n");
    for e in &doc.elements {
        let implementation = e.category == "nodedef"
            || (e.category == "nodegraph" && e.attrs.contains_key("nodedef"));
        if implementation || keep.contains(e.name.as_str()) {
            element(&mut out, e, "  ");
        }
    }
    out.push_str("</materialx>\n");
    out
}

/// Top-level nodes of a value type nothing reads.
pub(super) fn dangling(doc: &Document) -> Vec<String> {
    let mut read = Vec::new();
    for e in &doc.elements {
        reads(e, &mut read);
    }
    doc.elements
        .iter()
        .filter(|e| {
            !matches!(
                e.category.as_str(),
                "nodedef" | "nodegraph" | "surfacematerial" | "typedef" | "look" | "backdrop"
            ) && !matches!(
                e.ty.as_str(),
                "" | "surfaceshader" | "displacementshader" | "volumeshader" | "material"
            ) && !read.contains(&e.name)
        })
        .map(|e| e.name.clone())
        .collect()
}

/// A nodegraph child's input that names a node outside the graph (not a
/// sibling) reads the top level.
fn out_of_graph(graph: &Element, p: &Port, out: &mut Vec<String>) {
    if let Some(n) = &p.nodename
        && graph.child(n).is_none()
    {
        out.push(n.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materialx::doc;

    #[test]
    fn keeps_what_the_material_reads_and_folds_prefixes() {
        let text = r#"<?xml version="1.0"?>
<materialx version="1.39" fileprefix="tex/" colorspace="lin_rec709">
  <image name="img" type="color3" colorspace="srgb_texture">
    <input name="file" type="filename" value="a.png" />
  </image>
  <image name="unused" type="color3">
    <input name="file" type="filename" value="b.png" />
  </image>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="img" />
  </open_pbr_surface>
  <surfacematerial name="M" type="material">
    <input name="surfaceshader" type="surfaceshader" nodename="s" />
  </surfacematerial>
  <surfacematerial name="Other" type="material" />
</materialx>"#;
        let d = doc::parse(text).unwrap();
        let out = material(&d, "M");
        let back = doc::parse(&out).unwrap();
        assert!(back.element("unused").is_none() && back.element("Other").is_none());
        let file = back.element("img").unwrap().input("file").unwrap();
        assert_eq!(file.value.as_deref(), Some("tex/a.png"));
        assert_eq!(file.fileprefix, "");
        assert_eq!(file.colorspace.as_deref(), Some("srgb_texture"));
        assert_eq!(back.colorspace.as_deref(), Some("lin_rec709"));
        // Written again, the text is the same.
        assert_eq!(material(&back, "M"), out);
    }
}
