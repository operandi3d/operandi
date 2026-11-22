//! E15 (BEHAVIOR-UV-TOOLS §15, UVT-120) and UVT-92: the current map's
//! edges as an SVG or EPS drawing of the 0-1 tile; Convert UVs to Mesh; and
//! Transfer UVs between meshes of one topology.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVE-P01..P03): the drawing
//! is `size` units square with black 1-unit lines on no background, each
//! UV edge drawn once; Convert makes the new mesh beside the old one,
//! named "<item> UVs", U to X and V to Y in metres; Transfer copies the
//! primary active mesh's map to the other active meshes whose polygons have
//! the same corner counts in the same order.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The drawing's format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LayoutFormat {
    Svg,
    Eps,
}

/// Each UV edge of `map` once, as (a, b) in UV.
pub fn uv_edges(mesh: &Mesh, map: &str) -> Vec<(DVec2, DVec2)> {
    let Some(rows) = mesh.uv_maps.get(map) else {
        return Vec::new();
    };
    let key = |p: DVec2| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (f, row) in rows.iter().enumerate() {
        if mesh.is_polyline(f) || row.iter().any(Option::is_none) {
            continue;
        }
        let n = row.len();
        for i in 0..n {
            let (a, b) = (row[i].unwrap(), row[(i + 1) % n].unwrap());
            let (ka, kb) = (key(a), key(b));
            if seen.insert(if ka < kb { (ka, kb) } else { (kb, ka) }) {
                out.push((a, b));
            }
        }
    }
    out
}

/// UVT-120: the map's edges as an SVG or EPS document, the 0-1 tile
/// `size` units square, V up.
pub fn uv_layout(mesh: &Mesh, map: &str, format: LayoutFormat, size: f64) -> Result<String> {
    if !(size.is_finite() && size > 0.) {
        return Err("The drawing's size must be more than zero".into());
    }
    if !mesh.uv_maps.contains_key(map) {
        return Err(format!("The mesh has no UV map called {map}"));
    }
    let edges = uv_edges(mesh, map);
    let mut s = String::new();
    match format {
        LayoutFormat::Svg => {
            let _ = writeln!(
                s,
                r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">
<title>UV layout: {map}</title>
<g fill="none" stroke="black" stroke-width="1" stroke-linecap="round">"#
            );
            for (a, b) in &edges {
                let _ = writeln!(
                    s,
                    r#"<line x1="{:.3}" y1="{:.3}" x2="{:.3}" y2="{:.3}"/>"#,
                    a.x * size,
                    (1. - a.y) * size,
                    b.x * size,
                    (1. - b.y) * size
                );
            }
            s.push_str("</g>\n</svg>\n");
        }
        LayoutFormat::Eps => {
            let _ = writeln!(
                s,
                "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 {} {}\n%%Title: UV layout: {map}\n%%Creator: Operandi\n%%EndComments\n1 setlinewidth 1 setlinecap",
                size.ceil() as i64,
                size.ceil() as i64
            );
            for (a, b) in &edges {
                let _ = writeln!(
                    s,
                    "newpath {:.3} {:.3} moveto {:.3} {:.3} lineto stroke",
                    a.x * size,
                    a.y * size,
                    b.x * size,
                    b.y * size
                );
            }
            s.push_str("showpage\n%%EOF\n");
        }
    }
    Ok(s)
}

/// Convert UVs to Mesh (UVT-120): a flat mesh with a point per UV position
/// (U to X, V to Y) and a polygon per mapped polygon, which keeps the map.
pub fn uvs_to_mesh(mesh: &Mesh, map: &str) -> Result<Mesh> {
    let rows = mesh
        .uv_maps
        .get(map)
        .ok_or_else(|| format!("The mesh has no UV map called {map}"))?;
    let key = |v: u32, p: DVec2| (v, (p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let mut ids = std::collections::HashMap::new();
    let mut out = Mesh::default();
    let mut new_rows = Vec::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        let Some(row) = rows.get(f).filter(|r| r.iter().all(Option::is_some)) else {
            continue;
        };
        if mesh.is_polyline(f) || row.len() != face.vertices.len() {
            continue;
        }
        let vertices = face
            .vertices
            .iter()
            .zip(row)
            .map(|(&v, p)| {
                let p = p.unwrap();
                *ids.entry(key(v, p)).or_insert_with(|| {
                    out.vertices.push(DVec3::new(p.x, p.y, 0.));
                    out.vertices.len() as u32 - 1
                })
            })
            .collect();
        out.faces.push(crate::Face {
            vertices,
            uv: None,
            ..face.clone()
        });
        new_rows.push(row.clone());
    }
    if out.faces.is_empty() {
        return Err("The map has no mapped polygons to convert".into());
    }
    out.uv_maps.insert(map.to_owned(), new_rows);
    out.attributes.current_uv_map = Some(map.to_owned());
    out.validate()?;
    Ok(out)
}

/// Whether two meshes share a topology for Transfer: the same polygons'
/// corner counts in the same order and the same edges.
fn same_topology(a: &Mesh, b: &Mesh) -> bool {
    a.faces.len() == b.faces.len()
        && a.faces
            .iter()
            .zip(b.faces.iter())
            .all(|(x, y)| x.vertices.len() == y.vertices.len())
        && {
            let edges = |m: &Mesh| -> BTreeSet<(u32, u32)> {
                m.faces
                    .iter()
                    .flat_map(|f| {
                        let v = &f.vertices;
                        (0..v.len()).map(move |i| edge_key(v[i], v[(i + 1) % v.len()]))
                    })
                    .collect()
            };
            edges(a) == edges(b)
        }
}

impl crate::Editor {
    /// `Command::UvToMesh`: a new mesh item from the active mesh's map,
    /// beside it; one undo step.
    pub(crate) fn uv_to_mesh(&mut self) -> Result<()> {
        self.require_idle()?;
        let item = self.scene.active().ok_or("Select a mesh to convert")?;
        let map = crate::uv_maps::current_map(&item.mesh).ok_or("The mesh has no UV map yet")?;
        let flat = uvs_to_mesh(&item.mesh, &map)?;
        let name = format!("{} UVs", item.name);
        let before = self.scene.clone();
        self.scene.new_mesh()?;
        let made = self.scene.items.last_mut().ok_or("Missing mesh")?;
        made.name = name;
        made.mesh = flat.into();
        let id = made.id;
        self.scene.selection = crate::Selection {
            item: Some(id),
            ..Default::default()
        };
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record("Convert UVs to Mesh".into(), before);
        Ok(())
    }

    /// `Command::UvTransfer` (UVT-92, UVE-P03): the primary mesh's map
    /// copied to the other active meshes of the same topology; one undo
    /// step.
    pub(crate) fn uv_transfer(&mut self) -> Result<()> {
        self.require_idle()?;
        let source = self.scene.active().ok_or("Select the source mesh first")?;
        let map =
            crate::uv_maps::current_map(&source.mesh).ok_or("The source mesh has no UV map")?;
        let rows = source.mesh.uv_maps[&map].clone();
        let source_mesh = source.mesh.clone();
        let source_id = source.id;
        let targets: Vec<u64> = self
            .scene
            .active_meshes()
            .into_iter()
            .filter(|&id| id != source_id)
            .collect();
        if targets.is_empty() {
            return Err("Transfer UVs: select the target meshes too (the source first)".into());
        }
        let before = self.scene.clone();
        let mut done = 0;
        for id in targets {
            let item = self.scene.item_mut(id).ok_or("Missing mesh")?;
            if !same_topology(&source_mesh, &item.mesh) {
                continue;
            }
            let mut mesh: Mesh = (*item.mesh).clone();
            mesh.uv_maps.insert(map.clone(), rows.clone());
            mesh.attributes.point_uvs.remove(&map);
            mesh.attributes.uv_records.remove(&map);
            if mesh.attributes.current_uv_map.is_none() {
                mesh.attributes.current_uv_map = Some(map.clone());
            }
            item.mesh = mesh.into();
            done += 1;
        }
        if done == 0 {
            self.scene = before;
            return Err("Transfer UVs: no target has the source's topology".into());
        }
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record("Transfer UVs".into(), before);
        Ok(())
    }
}
