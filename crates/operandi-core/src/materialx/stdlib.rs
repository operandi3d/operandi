//! The MaterialX library files Operandi bundles (MX-01), Apache-2.0
//! (crates/operandi-core/materialx/, licences in docs/licenses): the
//! node definitions of the shaders it imports, for their defaults, and
//! the published standard_surface -> open_pbr_surface translation graph.
use super::doc::{self, Document, Element, Port};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const FILES: &[&str] = &[
    include_str!("../../materialx/open_pbr_surface.mtlx"),
    include_str!("../../materialx/standard_surface.mtlx"),
    include_str!("../../materialx/gltf_pbr.mtlx"),
    include_str!("../../materialx/usd_preview_surface.mtlx"),
    include_str!("../../materialx/standard_surface_to_open_pbr.mtlx"),
];

fn library() -> &'static Vec<Document> {
    static LIB: OnceLock<Vec<Document>> = OnceLock::new();
    LIB.get_or_init(|| {
        FILES
            .iter()
            .map(|f| doc::parse(f).expect("bundled MaterialX library parses"))
            .collect()
    })
}

/// A bundled node definition by its name.
pub(super) fn nodedef_named(name: &str) -> Option<&'static Element> {
    nodedef_by_name(name)
}

fn nodedef_by_name(name: &str) -> Option<&'static Element> {
    library()
        .iter()
        .flat_map(|d| d.all("nodedef"))
        .find(|e| e.name == name)
}

/// The default node definition for a node category (`node="..."`,
/// `isdefaultversion`, else the first), with `type` when several exist.
fn nodedef(category: &str, ty: Option<&str>) -> Option<&'static Element> {
    let all: Vec<&Element> = library()
        .iter()
        .flat_map(|d| d.all("nodedef"))
        .filter(|e| e.attrs.get("node").map(String::as_str) == Some(category))
        .filter(|e| ty.is_none_or(|t| e.outputs.iter().any(|o| o.ty == t)))
        .collect();
    all.iter()
        .find(|e| e.attrs.get("isdefaultversion").map(String::as_str) == Some("true"))
        .or(all.first())
        .copied()
}

/// The inputs a node category declares, with their defaults, following
/// `inherit` (standard_surface 1.0.1 inherits most from 1.0.0).
pub fn defaults(category: &str) -> BTreeMap<String, Port> {
    let mut out = BTreeMap::new();
    let mut at = nodedef(category, None);
    let mut guard = 0;
    while let Some(def) = at {
        for p in &def.inputs {
            out.entry(p.name.clone()).or_insert_with(|| p.clone());
        }
        at = def.attrs.get("inherit").and_then(|n| nodedef_by_name(n));
        guard += 1;
        if guard > 8 {
            break;
        }
    }
    out
}

/// MaterialX 1.39.4's standard library definitions (`stdlib_defs.mtlx`).
fn standard_defs() -> &'static Document {
    static DEFS: OnceLock<Document> = OnceLock::new();
    DEFS.get_or_init(|| {
        doc::parse(include_str!("../../materialx/stdlib_defs.mtlx"))
            .expect("bundled MaterialX standard library parses")
    })
}

/// A MaterialX 1.39.4 standard nodedef by its name (`ND_noise3d_color3`).
pub fn standard_def(name: &str) -> Option<&'static Element> {
    standard_defs().all("nodedef").find(|d| d.name == name)
}

/// The standard nodedef a node instance matches: its category and output
/// type, with every input it sets declared with that type
/// ("ND_multiply_color3FA" for a colour times a float). `None` for a
/// node that isn't a standard one.
pub fn standard_nodedef(el: &Element) -> Option<&'static str> {
    standard_defs()
        .all("nodedef")
        .find(|d| {
            d.attrs.get("node") == Some(&el.category)
                && d.outputs.iter().any(|o| o.ty == el.ty)
                && el
                    .inputs
                    .iter()
                    .all(|p| d.inputs.iter().any(|i| i.name == p.name && i.ty == p.ty))
        })
        .map(|d| d.name.as_str())
}

/// MaterialX 1.39.4's nodegraph implementations of standard nodes
/// (`stdlib_ng.mtlx`).
fn standard_graphs() -> &'static Document {
    static NG: OnceLock<Document> = OnceLock::new();
    NG.get_or_init(|| {
        doc::parse(include_str!("../../materialx/stdlib_ng.mtlx"))
            .expect("bundled MaterialX standard nodegraphs parse")
    })
}

/// The bundled definition and nodegraph implementation of a node with
/// no implementation in its own document: MaterialX's standard nodes
/// implemented as nodegraphs (place2d, ramp4, triplanarprojection...) and
/// the shader libraries' helpers (gltf_colorimage...).
pub fn implementation(el: &Element) -> Option<(&'static Element, &'static Element)> {
    let defs = library()
        .iter()
        .chain(std::iter::once(standard_defs()))
        .flat_map(|d| d.all("nodedef"));
    let mut graphs = library()
        .iter()
        .chain(std::iter::once(standard_graphs()))
        .flat_map(|d| d.all("nodegraph"));
    let fits = |d: &Element| {
        d.attrs.get("node") == Some(&el.category)
            && (el.ty.is_empty()
                || d.outputs.iter().any(|o| o.ty == el.ty)
                || (el.ty == "multioutput" && d.outputs.len() > 1))
            && el.inputs.iter().all(|p| {
                d.inputs
                    .iter()
                    .any(|i| i.name == p.name && (p.ty.is_empty() || i.ty == p.ty))
            })
    };
    let candidates: Vec<&'static Element> = defs.filter(|d| fits(d)).collect();
    let def = candidates
        .iter()
        .find(|d| d.attrs.get("isdefaultversion").map(String::as_str) == Some("true"))
        .or(candidates.first())
        .copied()?;
    let graph = graphs.find(|g| g.attrs.get("nodedef") == Some(&def.name))?;
    Some((def, graph))
}

/// The translation nodegraph standard_surface -> open_pbr_surface.
pub fn standard_to_openpbr() -> &'static Element {
    library()
        .iter()
        .flat_map(|d| d.all("nodegraph"))
        .find(|e| e.name == "NG_standard_surface_to_open_pbr_surface")
        .expect("bundled translation graph")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openpbr::{Param, Value};

    #[test]
    fn bundled_openpbr_defaults_match_the_table() {
        let defs = defaults("open_pbr_surface");
        let def = nodedef("open_pbr_surface", None).unwrap();
        assert_eq!(def.attrs.get("version").map(String::as_str), Some("1.1.2"));
        assert_eq!(defs.len(), Param::ALL.len());
        for p in Param::ALL {
            let port = &defs[p.name()];
            let Some(v) = port.value.as_deref().and_then(doc::numbers) else {
                assert!(matches!(p.default(), Value::Geometry), "{p:?}");
                continue;
            };
            let want = match p.default() {
                // The table keeps lengths in metres; the file in cm.
                Value::Float(f) if p.is_length() => vec![f * 100.],
                Value::Float(f) => vec![f],
                Value::Color(c) => c.to_vec(),
                Value::Bool(b) => vec![f64::from(u8::from(b))],
                Value::Geometry => unreachable!(),
            };
            assert_eq!(v.len(), want.len(), "{p:?}");
            assert!(
                v.iter().zip(&want).all(|(a, b)| (a - b).abs() < 1e-9),
                "{p:?}: {v:?} vs {want:?}"
            );
        }
    }

    #[test]
    fn other_shaders_have_their_defaults() {
        let std = defaults("standard_surface");
        assert!(std.contains_key("specular_roughness") && std.contains_key("coat_color"));
        assert_eq!(std["base"].value.as_deref(), Some("1.0"));
        let gltf = defaults("gltf_pbr");
        assert!(gltf.contains_key("metallic") && gltf.contains_key("emissive_strength"));
        let usd = defaults("UsdPreviewSurface");
        assert_eq!(
            usd["diffuseColor"].value.as_deref(),
            Some("0.18, 0.18, 0.18")
        );
        assert!(
            !standard_to_openpbr().outputs.is_empty() || !standard_to_openpbr().children.is_empty()
        );
    }
}
