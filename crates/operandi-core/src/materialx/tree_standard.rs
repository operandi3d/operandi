//! A tree surface as standard MaterialX (materials architecture, stage 3):
//! the tree's own form (tree.rs) with the `operandi_*` node definitions
//! and implementations it uses (tree_library.rs) included, and the
//! surface as an `open_pbr_surface`, so any MaterialX reader (USD export,
//! Hydra, other apps) can draw it.
//!
//! The principled inputs go onto OpenPBR by the inverse of the OPM-03
//! table (`openpbr::cycles_target`). What OpenPBR has no input for here
//! (the Shader's visibility, alpha type and fog, bump heights,
//! displacement, rounded edges, a Node Material layer, the inside volume,
//! material-value outputs) is listed in the notes. A surface using a node
//! with no standard version yet (procedural patterns...) is refused,
//! naming it.
use super::doc::{self, Element, Port};
use super::{tree, tree_library, write};
use crate::Result;
use crate::Scene;
use crate::openpbr::{NANOMETRES_PER_MICROMETRE, STRENGTH_PER_NIT};
use crate::shader_eval::{Input, Principled, SeeThroughKind};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// How a principled input lands on OpenPBR: the input, whether it's a
/// colour, and the factor from the principled value to OpenPBR's.
const SURFACE_MAP: &[(&str, &str, bool, f64)] = &[
    ("base_color", "base_color", true, 1.),
    ("metallic", "base_metalness", false, 1.),
    ("roughness", "specular_roughness", false, 1.),
    ("ior", "specular_ior", false, 1.),
    ("alpha", "geometry_opacity", false, 1.),
    ("diffuse_roughness", "base_diffuse_roughness", false, 1.),
    ("subsurface_weight", "subsurface_weight", false, 1.),
    ("subsurface_radius", "subsurface_radius_scale", true, 1.),
    ("subsurface_scale", "subsurface_radius", false, 1.),
    // Specular IOR Level is weight / 2.
    ("specular_ior_level", "specular_weight", false, 2.),
    ("specular_tint", "specular_color", true, 1.),
    ("anisotropic", "specular_roughness_anisotropy", false, 1.),
    ("transmission_weight", "transmission_weight", false, 1.),
    ("sheen_weight", "fuzz_weight", false, 1.),
    ("coat_weight", "coat_weight", false, 1.),
    ("coat_roughness", "coat_roughness", false, 1.),
    ("emission_color", "emission_color", true, 1.),
    (
        "emission_strength",
        "emission_luminance",
        false,
        1. / STRENGTH_PER_NIT,
    ),
    (
        "subsurface_anisotropy",
        "subsurface_scatter_anisotropy",
        false,
        1.,
    ),
    ("coat_tint", "coat_color", true, 1.),
    ("coat_ior", "coat_ior", false, 1.),
    ("sheen_tint", "fuzz_color", true, 1.),
    ("sheen_roughness", "fuzz_roughness", false, 1.),
    (
        "thin_film_thickness",
        "thin_film_thickness",
        false,
        1. / NANOMETRES_PER_MICROMETRE,
    ),
    ("thin_film_ior", "thin_film_ior", false, 1.),
    (
        "dispersion_scale",
        "transmission_dispersion_scale",
        false,
        1.,
    ),
    (
        "dispersion_abbe",
        "transmission_dispersion_abbe_number",
        false,
        1.,
    ),
];

fn num(v: f64) -> String {
    format!("{v}")
}

/// The OpenPBR surface under construction, and the conversion nodes it
/// needs.
struct Surface<'a> {
    nodes: String,
    n: usize,
    inputs: Vec<(String, String, String)>,
    source: &'a Element,
}

impl Surface<'_> {
    /// Sets OpenPBR input `to` from the tree surface's input `port`.
    fn set(&mut self, to: &str, port: &Port, colour: bool, factor: f64) {
        self.inputs.retain(|(n, _, _)| n != to);
        let (ty, attr) = if colour {
            ("color3", self.colour(port, factor))
        } else {
            ("float", self.float(port, factor))
        };
        self.inputs.push((to.into(), ty.into(), attr));
    }
    fn fresh(&mut self, what: &str) -> String {
        self.n += 1;
        format!("surface_{what}{}", self.n)
    }
    fn colour(&mut self, port: &Port, factor: f64) -> String {
        match (&port.nodename, &port.value) {
            (Some(n), _) if factor == 1. => format!("nodename=\"{n}\""),
            (Some(n), _) => {
                let name = self.fresh("scale");
                let _ = write!(
                    self.nodes,
                    "  <multiply name=\"{name}\" type=\"color3\">\n    <input name=\"in1\" type=\"color3\" nodename=\"{n}\" />\n    <input name=\"in2\" type=\"float\" value=\"{}\" />\n  </multiply>\n",
                    num(factor)
                );
                format!("nodename=\"{name}\"")
            }
            (None, Some(v)) => {
                let c = doc::numbers(v).unwrap_or_default();
                let c: Vec<String> = c.iter().map(|x| num(x * factor)).collect();
                format!("value=\"{}\"", c.join(", "))
            }
            (None, None) => "value=\"0, 0, 0\"".into(),
        }
    }
    /// A scalar input: the colour's first component (scalars are grey).
    fn float(&mut self, port: &Port, factor: f64) -> String {
        match (&port.nodename, &port.value) {
            (Some(n), _) => {
                let name = self.fresh("x");
                let _ = write!(
                    self.nodes,
                    "  <extract name=\"{name}\" type=\"float\">\n    <input name=\"in\" type=\"color3\" nodename=\"{n}\" />\n    <input name=\"index\" type=\"integer\" value=\"0\" />\n  </extract>\n"
                );
                if factor == 1. {
                    return format!("nodename=\"{name}\"");
                }
                let scaled = self.fresh("scale");
                let _ = write!(
                    self.nodes,
                    "  <multiply name=\"{scaled}\" type=\"float\">\n    <input name=\"in1\" type=\"float\" nodename=\"{name}\" />\n    <input name=\"in2\" type=\"float\" value=\"{}\" />\n  </multiply>\n",
                    num(factor)
                );
                format!("nodename=\"{scaled}\"")
            }
            (None, Some(v)) => {
                let c = doc::numbers(v).unwrap_or_default();
                format!(
                    "value=\"{}\"",
                    num(c.first().copied().unwrap_or(0.) * factor)
                )
            }
            (None, None) => "value=\"0\"".into(),
        }
    }
    fn port(&self, name: &str) -> Option<&Port> {
        self.source.input(name)
    }
}

/// The surface a Material Group gives, as the renderer evaluates it on
/// the first item whose polygons carry the group's material tag (with
/// that item's locators); on no item when none does. `None` for a node
/// that isn't a group.
pub fn group_surface(scene: &Scene, group: crate::shader_tree::NodeId) -> Option<Principled> {
    use crate::shader_tree::{NodeKind, TagMask, TagType};
    let node = scene.shader_tree.get(group)?;
    let NodeKind::Group(g) = &node.kind else {
        return None;
    };
    // A group tagged by material name follows that tag; any other group
    // is read on the default material.
    let tag = match (&g.tag, g.tag_type) {
        (TagMask::Name(t), TagType::Material) => t.as_str(),
        _ => crate::material::DEFAULT_MATERIAL,
    };
    let uses = |m: &str| m == tag || (m.is_empty() && tag == crate::material::DEFAULT_MATERIAL);
    let item = scene
        .items
        .iter()
        .find(|i| i.mesh.faces.iter().any(|f| uses(&f.material)));
    let id = item.map_or(0, |i| i.id);
    let ancestors = item.map_or_else(Vec::new, |i| scene.item_ancestors(i.id));
    let locators = item.map_or_else(Vec::new, |i| scene.locator_frames(i.id));
    let tree = scene.shading_tree();
    Some(
        tree.evaluate(&crate::shader_eval::Context {
            item: id,
            ancestors: &ancestors,
            material: tag,
            part: None,
            sets: &[],
            locators: &locators,
        })
        .principled(),
    )
}

/// The colour picture nodes (`operandi_image`, not alpha) node `name`
/// reads, directly or through other nodes.
fn pictures_under(d: &doc::Document, name: &str, out: &mut BTreeSet<String>) {
    let mut todo = vec![name.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(n) = todo.pop() {
        if !seen.insert(n.clone()) {
            continue;
        }
        let Some(el) = d.element(&n) else { continue };
        if el.category == "operandi_image" {
            out.insert(n.clone());
        }
        todo.extend(el.inputs.iter().filter_map(|p| p.nodename.clone()));
    }
}

/// Surface `p` as a standalone standard MaterialX document named `name`,
/// with notes on what it leaves out.
pub fn document(p: &Principled, scene: &Scene, name: &str) -> Result<(String, Vec<String>)> {
    document_with(p, scene, name, None)
}

/// [`document`] with Material Group `free`'s free nodes too: every one,
/// wired or not, with its box place as `xpos`/`ypos` (NE-09, NE-10).
pub fn document_with(
    p: &Principled,
    scene: &Scene,
    name: &str,
    free: Option<&crate::node_view::GroupNodes>,
) -> Result<(String, Vec<String>)> {
    let text = tree::write(p, scene, name);
    let d = doc::parse(&text)?;
    let source = d
        .all(tree::SURFACE)
        .next()
        .ok_or("the tree form has no surface")?;
    // Pictures are read as sRGB where they drive a colour, as plain
    // numbers otherwise (as Cycles reads them: RenderShader::compile).
    let colour_inputs: Vec<&str> = SURFACE_MAP
        .iter()
        .filter(|(_, _, colour, _)| *colour)
        .map(|(from, _, _, _)| *from)
        .chain(["see_through_color"])
        .collect();
    let mut colour_pictures = BTreeSet::new();
    let mut number_pictures = BTreeSet::new();
    for port in &source.inputs {
        let into = if colour_inputs.contains(&port.name.as_str()) {
            &mut colour_pictures
        } else {
            &mut number_pictures
        };
        if let Some(n) = &port.nodename {
            pictures_under(&d, n, into);
        }
    }
    let mut notes = Vec::new();
    if colour_pictures
        .intersection(&number_pictures)
        .next()
        .is_some()
    {
        notes.push("pictures used as both colours and numbers are read as colours".into());
    }
    let mut used = BTreeSet::new();
    let mut body = String::new();
    // Free nodes the surface doesn't read still go in, and every free
    // node carries its place.
    let mut elements = d.elements.clone();
    if let Some(g) = free {
        for el in g.elements() {
            if !elements.iter().any(|e| e.name == el.name) {
                elements.push(el);
            }
        }
        for el in &mut elements {
            if let Some([x, y]) = g.places.get(&el.name) {
                // MaterialX places in node-width units; the view's are
                // canvas points (provisional: 1 unit = 100 points).
                el.attrs.insert("xpos".into(), format!("{}", x / 100.));
                el.attrs.insert("ypos".into(), format!("{}", y / 100.));
            }
        }
    }
    for el in &elements {
        match el.category.as_str() {
            tree::SURFACE | tree::NODE_MATERIAL | "surfacematerial" => continue,
            // A free node is a standard MaterialX node already.
            cat if !cat.starts_with("operandi_") => {
                write::element(&mut body, el, "  ");
            }
            cat => {
                if !tree_library::implemented(cat) {
                    return Err(format!("{cat} has no standard MaterialX version yet"));
                }
                used.insert(cat.to_owned());
                let mut el = el.clone();
                if colour_pictures.contains(&el.name) {
                    for p in el.inputs.iter_mut().filter(|p| p.ty == "filename") {
                        let ext = std::path::Path::new(p.value.as_deref().unwrap_or_default())
                            .extension()
                            .map(|e| e.to_string_lossy().to_ascii_lowercase())
                            .unwrap_or_default();
                        if !matches!(ext.as_str(), "exr" | "hdr") {
                            p.colorspace = Some("srgb_texture".into());
                        }
                    }
                }
                write::element(&mut body, &el, "  ");
            }
        }
    }
    let mut s = Surface {
        nodes: String::new(),
        n: 0,
        inputs: Vec::new(),
        source,
    };
    for (from, to, colour, factor) in SURFACE_MAP {
        if let Some(port) = s.port(from).cloned() {
            s.set(to, &port, *colour, *factor);
        }
    }
    // Cycles' subsurface takes the base colour; OpenPBR's has its own.
    if let Some(port) = s.port("base_color").cloned() {
        s.set("subsurface_color", &port, true, 1.);
    }
    if s.port("thin_film_thickness").is_some() {
        s.inputs.push((
            "thin_film_weight".into(),
            "float".into(),
            "value=\"1\"".into(),
        ));
    }
    if let Some(st) = &p.see_through {
        for (from, to, colour) in [
            ("see_through_amount", "transmission_weight", false),
            ("see_through_color", "transmission_color", true),
        ] {
            if let Some(port) = s.port(from).cloned() {
                s.set(to, &port, colour, 1.);
            }
        }
        if st.kind == SeeThroughKind::Glass {
            if let Some(port) = s.port("see_through_ior").cloned() {
                s.set("specular_ior", &port, false, 1.);
            }
            let rough = p
                .inputs
                .iter()
                .find(|(i, _)| *i == Input::Roughness)
                .map(|(_, e)| e);
            if rough != Some(&st.roughness) {
                notes.push(
                    "the see-through part's own roughness (OpenPBR shares specular_roughness)"
                        .into(),
                );
            }
        }
    }
    if p.thin_walled {
        s.inputs.push((
            "geometry_thin_walled".into(),
            "boolean".into(),
            "value=\"true\"".into(),
        ));
    }
    if let Some(port) = s.port("normal_map").cloned() {
        let v = s.fresh("normal_in");
        let map = s.fresh("normalmap");
        let src = s.colour(&port, 1.);
        let _ = write!(
            s.nodes,
            "  <convert name=\"{v}\" type=\"vector3\">\n    <input name=\"in\" type=\"color3\" {src} />\n  </convert>\n  <normalmap name=\"{map}\" type=\"vector3\">\n    <input name=\"in\" type=\"vector3\" nodename=\"{v}\" />\n  </normalmap>\n"
        );
        s.inputs.push((
            "geometry_normal".into(),
            "vector3".into(),
            format!("nodename=\"{map}\""),
        ));
    }
    let left_out = [
        (p.bump.is_some(), "bump heights"),
        (p.displacement.is_some(), "displacement"),
        (p.rounded_edge.is_some(), "rounded edges"),
        (
            p.absorption.is_some() || p.scatter.is_some(),
            "the see-through inside's absorption and scattering",
        ),
        (
            p.node.is_some(),
            "a Node Material layer mixed over the surface",
        ),
        (!p.values.is_empty(), "material-value outputs"),
        (
            p.shader != Default::default(),
            "the Shader's settings (visibility, alpha type, fog)",
        ),
    ];
    notes.extend(
        left_out
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, what)| (*what).to_string()),
    );
    let mut out = String::from("<?xml version=\"1.0\"?>\n<materialx version=\"1.39\">\n");
    // The definitions and implementations of the operandi_* nodes used.
    for cat in &used {
        out.push_str(&tree_library::definition(cat).unwrap_or_default());
    }
    out.push_str(&body);
    out.push_str(&s.nodes);
    let _ = writeln!(
        out,
        "  <open_pbr_surface name=\"surface\" type=\"surfaceshader\">"
    );
    for (input, ty, attr) in &s.inputs {
        let _ = writeln!(out, "    <input name=\"{input}\" type=\"{ty}\" {attr} />");
    }
    let _ = writeln!(out, "  </open_pbr_surface>");
    let material = super::export::mx_name(name);
    let _ = writeln!(
        out,
        "  <surfacematerial name=\"{material}\" type=\"material\">\n    <input name=\"surfaceshader\" type=\"surfaceshader\" nodename=\"surface\" />\n  </surfacematerial>\n</materialx>"
    );
    Ok((out, notes))
}

#[cfg(test)]
#[path = "tree_standard_tests.rs"]
mod tests;
