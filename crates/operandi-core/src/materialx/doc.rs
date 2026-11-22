//! A MaterialX document as plain data (MX-01): elements, their inputs and
//! outputs, nodegraphs, and the inherited `fileprefix` and `colorspace`.
//! Read with roxmltree; written by hand in `writer.rs`. Only what the
//! importer needs is kept; everything is still named, so a refusal can
//! say which node it stopped at.
use crate::Result;
use std::collections::BTreeMap;

/// One input or output of an element.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Port {
    pub name: String,
    pub ty: String,
    /// The `value` attribute, as written.
    pub value: Option<String>,
    pub nodename: Option<String>,
    pub nodegraph: Option<String>,
    /// The output of a multi-output node or of a nodegraph.
    pub output: Option<String>,
    pub interfacename: Option<String>,
    /// The effective colour space (the port's own, else inherited).
    pub colorspace: Option<String>,
    pub unit: Option<String>,
    pub unittype: Option<String>,
    /// The effective `fileprefix` for filename values.
    pub fileprefix: String,
}

/// A node, nodedef, nodegraph, material or other element.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Element {
    /// The XML tag: the node's category (`open_pbr_surface`, `image`...),
    /// or `nodegraph`, `nodedef`, `surfacematerial`...
    pub category: String,
    pub name: String,
    pub ty: String,
    pub attrs: BTreeMap<String, String>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    /// A nodegraph's nodes.
    pub children: Vec<Element>,
    /// The effective colour space at this element.
    pub colorspace: Option<String>,
}
impl Element {
    pub fn input(&self, name: &str) -> Option<&Port> {
        self.inputs.iter().find(|p| p.name == name)
    }
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|e| e.name == name)
    }
}

/// A whole `.mtlx` file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub version: String,
    pub colorspace: Option<String>,
    pub elements: Vec<Element>,
}
impl Document {
    pub fn element(&self, name: &str) -> Option<&Element> {
        self.elements.iter().find(|e| e.name == name)
    }
    /// Every element of a category, in file order.
    pub fn all<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.elements.iter().filter(move |e| e.category == category)
    }
}

/// Files bigger than this aren't MaterialX materials (a sanity budget).
const MAX_BYTES: usize = 16 * 1024 * 1024;

pub fn parse(text: &str) -> Result<Document> {
    if text.len() > MAX_BYTES {
        return Err("MaterialX file too large".into());
    }
    let xml = roxmltree::Document::parse(text).map_err(|e| format!("Invalid MaterialX: {e}"))?;
    let root = xml.root_element();
    if root.tag_name().name() != "materialx" {
        return Err("Not a MaterialX file (no <materialx> root)".into());
    }
    let colorspace = root.attribute("colorspace").map(str::to_owned);
    let prefix = root.attribute("fileprefix").unwrap_or_default().to_owned();
    let elements = root
        .children()
        .filter(|n| n.is_element())
        .map(|n| element(n, &colorspace, &prefix))
        .collect();
    Ok(Document {
        version: root.attribute("version").unwrap_or_default().to_owned(),
        colorspace,
        elements,
    })
}

fn element(n: roxmltree::Node, colorspace: &Option<String>, prefix: &str) -> Element {
    let colorspace = n
        .attribute("colorspace")
        .map(str::to_owned)
        .or_else(|| colorspace.clone());
    let prefix = format!("{prefix}{}", n.attribute("fileprefix").unwrap_or_default());
    let mut out = Element {
        category: n.tag_name().name().to_owned(),
        name: n.attribute("name").unwrap_or_default().to_owned(),
        ty: n.attribute("type").unwrap_or_default().to_owned(),
        attrs: n
            .attributes()
            .map(|a| (a.name().to_owned(), a.value().to_owned()))
            .collect(),
        colorspace: colorspace.clone(),
        ..Element::default()
    };
    for c in n.children().filter(|c| c.is_element()) {
        match c.tag_name().name() {
            "input" => out.inputs.push(port(c, &colorspace, &prefix)),
            "output" => out.outputs.push(port(c, &colorspace, &prefix)),
            _ => out.children.push(element(c, &colorspace, &prefix)),
        }
    }
    out
}

fn port(n: roxmltree::Node, colorspace: &Option<String>, prefix: &str) -> Port {
    let a = |k: &str| n.attribute(k).map(str::to_owned);
    Port {
        name: a("name").unwrap_or_default(),
        ty: a("type").unwrap_or_default(),
        value: a("value"),
        nodename: a("nodename"),
        nodegraph: a("nodegraph"),
        output: a("output"),
        interfacename: a("interfacename"),
        colorspace: a("colorspace").or_else(|| colorspace.clone()),
        unit: a("unit"),
        unittype: a("unittype"),
        fileprefix: format!("{prefix}{}", n.attribute("fileprefix").unwrap_or_default()),
    }
}

/// A `value` as numbers ("0.1, 0.2, 0.3"; booleans as 0/1).
pub fn numbers(value: &str) -> Option<Vec<f64>> {
    match value.trim() {
        "true" => return Some(vec![1.]),
        "false" => return Some(vec![0.]),
        _ => {}
    }
    value
        .split(',')
        .map(|s| s.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_elements_ports_and_inherited_attributes() {
        let doc = parse(
            r#"<?xml version="1.0"?>
<materialx version="1.39" colorspace="acescg" fileprefix="tex/">
  <image name="img" type="color3" colorspace="srgb_texture">
    <input name="file" type="filename" value="a.png" />
  </image>
  <nodegraph name="ng" fileprefix="sub/">
    <image name="inner" type="float">
      <input name="file" type="filename" value="b.png" colorspace="lin_rec709"/>
    </image>
    <output name="out" type="float" nodename="inner" />
  </nodegraph>
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" value="0.1, 0.2, 0.3" />
    <input name="specular_roughness" type="float" nodegraph="ng" output="out" />
  </open_pbr_surface>
</materialx>"#,
        )
        .unwrap();
        assert_eq!(doc.version, "1.39");
        let img = doc.element("img").unwrap();
        let file = img.input("file").unwrap();
        assert_eq!(file.fileprefix, "tex/");
        assert_eq!(file.colorspace.as_deref(), Some("srgb_texture"));
        let ng = doc.element("ng").unwrap();
        let inner = ng.child("inner").unwrap().input("file").unwrap();
        assert_eq!(inner.fileprefix, "tex/sub/");
        assert_eq!(inner.colorspace.as_deref(), Some("lin_rec709"));
        assert_eq!(ng.outputs[0].nodename.as_deref(), Some("inner"));
        let s = doc.element("s").unwrap();
        assert_eq!(
            s.input("base_color").unwrap().colorspace.as_deref(),
            Some("acescg")
        );
        assert_eq!(
            numbers(s.input("base_color").unwrap().value.as_deref().unwrap()),
            Some(vec![0.1, 0.2, 0.3])
        );
        assert_eq!(numbers("true"), Some(vec![1.]));
        assert!(parse("<notmaterialx/>").is_err());
        assert!(parse("<materialx").is_err());
    }
}
