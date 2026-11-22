//! 3MF import and export (the 3MF Consortium's core specification).
//!
//! Reading follows the structure of the 3MF reader in craft-libs
//! (`crates/mesh/src/threemf.rs`, MIT OR Apache-2.0, Copyright (c) 2026
//! ArtCraft Team and the craft-libs contributors; see
//! docs/licenses/craft-libs-LICENSE-MIT.txt), rewritten on roxmltree and
//! Operandi's own mesh types.
//!
//! Read: mesh objects (vertices and triangles), components and build items
//! with their transforms (baked into the points, one item per placed
//! object), the model's unit, and base materials (`displaycolor`) as
//! polygon tags with that colour. Colour groups, textures and other
//! extensions are passed over and named in a note.
//!
//! Written: one object per visible mesh item, triangulated as drawn, in
//! millimetres (what slicers expect), Z up, one base material per polygon
//! tag. Polylines are skipped.
use super::{RawFace, RawMesh, WorldMesh, zip};
use crate::Result;
use glam::{DMat4, DVec3};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

const CORE_NS: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const MODEL_REL: &str = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";
/// Deepest component nesting followed (a loop in the file stops here).
const MAX_DEPTH: usize = 32;

#[derive(Default)]
struct Object {
    name: String,
    vertices: Vec<DVec3>,
    /// Vertex indices and the material tag of each triangle.
    triangles: Vec<([u32; 3], String)>,
    components: Vec<(u32, DMat4)>,
}

/// 3MF's `transform`: twelve numbers, a 4×3 matrix for row vectors.
fn transform(s: Option<&str>) -> DMat4 {
    let Some(s) = s else { return DMat4::IDENTITY };
    let v: Vec<f64> = s
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    if v.len() != 12 || v.iter().any(|x| !x.is_finite()) {
        return DMat4::IDENTITY;
    }
    DMat4::from_cols_array(&[
        v[0], v[1], v[2], 0.0, v[3], v[4], v[5], 0.0, v[6], v[7], v[8], 0.0, v[9], v[10], v[11],
        1.0,
    ])
}

/// `#RRGGBB` or `#RRGGBBAA` (sRGB) as linear RGB.
fn color(s: Option<&str>) -> Option<[f64; 3]> {
    let hex = s?.trim().strip_prefix('#')?;
    if hex.len() != 6 && hex.len() != 8 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(
        [byte(0)?, byte(2)?, byte(4)?]
            .map(|b| crate::colour_math::srgb_to_linear(f64::from(b) / 255.0)),
    )
}

/// The model part's name: the package's 3D model relationship, else the
/// first `.model` part.
fn model_part(parts: &[(String, Vec<u8>)]) -> Option<usize> {
    let rels = parts.iter().find(|(n, _)| n == "_rels/.rels");
    if let Some((_, xml)) = rels
        && let Ok(text) = std::str::from_utf8(xml)
        && let Ok(doc) = roxmltree::Document::parse(text)
    {
        for rel in doc.descendants().filter(|n| n.has_tag_name("Relationship")) {
            if rel.attribute("Type") == Some(MODEL_REL)
                && let Some(target) = rel.attribute("Target")
            {
                let target = target.trim_start_matches('/');
                if let Some(i) = parts.iter().position(|(n, _)| n == target) {
                    return Some(i);
                }
            }
        }
    }
    parts
        .iter()
        .position(|(n, _)| n.to_ascii_lowercase().ends_with(".model"))
}

pub(super) fn read(
    bytes: &[u8],
    stem: &str,
    notes: &mut Vec<String>,
    materials: &mut BTreeMap<String, crate::Material>,
) -> Result<Vec<RawMesh>> {
    if !zip::is_zip(bytes) {
        return Err("Invalid 3MF: not a ZIP package".into());
    }
    let parts = zip::read(bytes).map_err(|e| format!("Invalid 3MF: {e}"))?;
    let index = model_part(&parts).ok_or("Invalid 3MF: no 3D model part")?;
    let text =
        std::str::from_utf8(&parts[index].1).map_err(|_| "Invalid 3MF: model part isn't UTF-8")?;
    let doc = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            ..Default::default()
        },
    )
    .map_err(|e| format!("Invalid 3MF: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "model" {
        return Err("Invalid 3MF: the model part's root isn't `model`".into());
    }
    let unit = match root.attribute("unit") {
        Some("micron") => 1e-6,
        Some("centimeter") => 0.01,
        Some("inch") => 0.0254,
        Some("foot") => 0.3048,
        Some("meter") => 1.0,
        _ => 0.001,
    };
    let id =
        |n: roxmltree::Node, k: &str| n.attribute(k).and_then(|s| s.trim().parse::<u32>().ok());
    let num = |n: roxmltree::Node, k: &str| {
        n.attribute(k)
            .and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)
    };
    // Base material groups: id -> tag names, in index order.
    let mut groups: HashMap<u32, Vec<String>> = HashMap::new();
    let mut skipped_extensions = 0;
    for g in root
        .descendants()
        .filter(|n| n.tag_name().name() == "basematerials")
    {
        let Some(gid) = id(g, "id") else { continue };
        let names = g
            .children()
            .filter(|n| n.tag_name().name() == "base")
            .enumerate()
            .map(|(i, b)| {
                let name = b
                    .attribute("name")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("3MF Material {gid}.{i}"));
                if let Some(c) = color(b.attribute("displaycolor")) {
                    materials.entry(name.clone()).or_insert(crate::Material {
                        color: c,
                        ..Default::default()
                    });
                }
                name
            })
            .collect();
        groups.insert(gid, names);
    }
    for kind in [
        "colorgroup",
        "texture2dgroup",
        "compositematerials",
        "multiproperties",
    ] {
        skipped_extensions += root
            .descendants()
            .filter(|n| n.tag_name().name() == kind)
            .count();
    }
    let material = |pid: Option<u32>, index: Option<u32>| -> String {
        pid.and_then(|p| groups.get(&p))
            .and_then(|g| g.get(index.unwrap_or(0) as usize))
            .cloned()
            .unwrap_or_default()
    };
    let mut objects: HashMap<u32, Object> = HashMap::new();
    let mut order = Vec::new();
    let mut bad_triangles = 0;
    let mut foreign = 0;
    for o in root
        .descendants()
        .filter(|n| n.tag_name().name() == "object")
    {
        let Some(oid) = id(o, "id") else { continue };
        let mut object = Object {
            name: o.attribute("name").unwrap_or("").trim().to_owned(),
            ..Default::default()
        };
        let (opid, oindex) = (id(o, "pid"), id(o, "pindex"));
        for n in o.descendants() {
            match n.tag_name().name() {
                "vertex" => object
                    .vertices
                    .push(DVec3::new(num(n, "x"), num(n, "y"), num(n, "z"))),
                "triangle" => {
                    let v = [id(n, "v1"), id(n, "v2"), id(n, "v3")];
                    let [Some(a), Some(b), Some(c)] = v else {
                        bad_triangles += 1;
                        continue;
                    };
                    let tag = match id(n, "pid") {
                        Some(pid) => material(Some(pid), id(n, "p1")),
                        None => material(opid, oindex),
                    };
                    object.triangles.push(([a, b, c], tag));
                }
                "component" => {
                    if n.attribute("path").is_some() {
                        foreign += 1;
                    } else if let Some(target) = id(n, "objectid") {
                        object
                            .components
                            .push((target, transform(n.attribute("transform"))));
                    }
                }
                _ => {}
            }
        }
        order.push(oid);
        objects.insert(oid, object);
    }
    // Build items, or every object once when the file has none.
    let mut placed: Vec<(u32, DMat4)> = root
        .descendants()
        .filter(|n| n.tag_name().name() == "item")
        .filter_map(|n| Some((id(n, "objectid")?, transform(n.attribute("transform")))))
        .collect();
    if placed.is_empty() {
        let used: std::collections::HashSet<u32> = objects
            .values()
            .flat_map(|o| o.components.iter().map(|c| c.0))
            .collect();
        placed = order
            .iter()
            .filter(|o| !used.contains(o))
            .map(|&o| (o, DMat4::IDENTITY))
            .collect();
    }
    // Z-up file units to Y-up metres.
    let to_scene = DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]) * DMat4::from_scale(DVec3::splat(unit));
    let mut meshes = Vec::new();
    let mut stack: Vec<(u32, DMat4, usize, String)> = placed
        .into_iter()
        .rev()
        .map(|(o, m)| (o, to_scene * m, 0, String::new()))
        .collect();
    while let Some((oid, matrix, depth, parent_name)) = stack.pop() {
        let Some(object) = objects.get(&oid) else {
            continue;
        };
        let name = if !object.name.is_empty() {
            object.name.clone()
        } else if !parent_name.is_empty() {
            parent_name.clone()
        } else {
            stem.to_owned()
        };
        if depth < MAX_DEPTH {
            for (child, local) in object.components.iter().rev() {
                stack.push((*child, matrix * *local, depth + 1, name.clone()));
            }
        }
        if object.triangles.is_empty() {
            continue;
        }
        let mirrored = matrix.determinant() < 0.0;
        let mut raw = RawMesh {
            name,
            points: object
                .vertices
                .iter()
                .map(|&p| matrix.transform_point3(p))
                .collect(),
            ..Default::default()
        };
        for (v, tag) in &object.triangles {
            let mut vertices = v.to_vec();
            if mirrored {
                vertices.swap(1, 2);
            }
            if vertices.iter().any(|&i| i as usize >= raw.points.len()) {
                bad_triangles += 1;
                continue;
            }
            raw.faces.push(RawFace {
                vertices,
                uv: None,
                material: tag.clone(),
                smooth: false,
            });
        }
        meshes.push(raw);
    }
    if bad_triangles > 0 {
        notes.push(format!(
            "{bad_triangles} 3MF triangle(s) named missing vertices and were left out"
        ));
    }
    if foreign > 0 {
        notes.push(format!(
            "{foreign} 3MF component(s) in other model parts not imported"
        ));
    }
    if skipped_extensions > 0 {
        notes.push(
            "3MF colour groups, textures and mixed materials not imported (base materials only)"
                .into(),
        );
    }
    Ok(meshes)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Linear RGB as `#RRGGBB` (sRGB).
fn hex(c: [f64; 3]) -> String {
    let b =
        c.map(|v| (crate::colour_math::linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8);
    format!("#{:02X}{:02X}{:02X}", b[0], b[1], b[2])
}

pub(super) fn write(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Vec<u8>> {
    // Every polygon tag across the meshes, one base material each.
    let mut tags: Vec<String> = Vec::new();
    for m in meshes {
        for f in m.surface_faces() {
            let tag = &m.mesh.faces[f].material;
            if !tags.contains(tag) {
                tags.push(tag.clone());
            }
        }
    }
    let mut xml = String::new();
    let _ = writeln!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        xml,
        r#"<model unit="millimeter" xml:lang="en-US" xmlns="{CORE_NS}">"#
    );
    let _ = writeln!(xml, r#" <metadata name="Application">Operandi</metadata>"#);
    let _ = writeln!(xml, " <resources>");
    let _ = writeln!(xml, r#"  <basematerials id="1">"#);
    let materials = meshes.first().map(|m| &m.materials);
    for tag in &tags {
        let settings = materials
            .and_then(|m| m.get(tag))
            .cloned()
            .unwrap_or_default();
        let label = if tag.is_empty() { "Default" } else { tag };
        let _ = writeln!(
            xml,
            r#"   <base name="{}" displaycolor="{}"/>"#,
            xml_escape(label),
            hex(settings.color)
        );
    }
    let _ = writeln!(xml, "  </basematerials>");
    let mut objects = Vec::new();
    let mut lines = 0;
    // Y-up metres to Z-up millimetres.
    let file = |p: DVec3| DVec3::new(p.x, -p.z, p.y) * 1000.0;
    for m in meshes {
        lines += m.polyline.iter().filter(|&&p| p).count();
        let faces: Vec<usize> = m.surface_faces().collect();
        if faces.is_empty() {
            continue;
        }
        let oid = objects.len() + 2;
        let _ = writeln!(
            xml,
            r#"  <object id="{oid}" name="{}" type="model" pid="1" pindex="0">"#,
            xml_escape(&m.name)
        );
        let _ = writeln!(xml, "   <mesh>\n    <vertices>");
        for &p in &m.mesh.vertices {
            let p = file(p);
            let _ = writeln!(
                xml,
                r#"     <vertex x="{}" y="{}" z="{}"/>"#,
                super::num(p.x),
                super::num(p.y),
                super::num(p.z)
            );
        }
        let _ = writeln!(xml, "    </vertices>\n    <triangles>");
        for f in faces {
            let face = &m.mesh.faces[f];
            let slot = tags.iter().position(|t| *t == face.material).unwrap_or(0);
            for [a, b, c] in m.triangles(f) {
                let v = |i: usize| face.vertices[i];
                let _ = writeln!(
                    xml,
                    r#"     <triangle v1="{}" v2="{}" v3="{}" pid="1" p1="{slot}"/>"#,
                    v(a),
                    v(b),
                    v(c)
                );
            }
        }
        let _ = writeln!(xml, "    </triangles>\n   </mesh>\n  </object>");
        objects.push(oid);
    }
    if objects.is_empty() {
        return Err("Nothing to export: no visible item has polygons".into());
    }
    let _ = writeln!(xml, " </resources>\n <build>");
    for oid in &objects {
        let _ = writeln!(xml, r#"  <item objectid="{oid}"/>"#);
    }
    let _ = writeln!(xml, " </build>\n</model>");
    if lines > 0 {
        notes.push(format!(
            "{lines} polyline(s) skipped: 3MF holds triangle meshes only"
        ));
    }
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
 <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
 <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
</Types>
"#;
    let rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="{MODEL_REL}"/>
</Relationships>
"#
    );
    zip::write(&[
        ("[Content_Types].xml", content_types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("3D/3dmodel.model", xml.as_bytes()),
    ])
}
