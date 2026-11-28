//! Flip Polygons (spec `poly-flip-2026-11-05`): reverses each polygon's
//! facing by keeping its first corner and reversing the rest. In Polygon
//! mode it flips the selected polygons, or every polygon when none is
//! selected; in Item mode every polygon of the mesh. Vertex and Edge mode
//! refuse. Winding-only; the exact incidence ledger remains validated.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

impl Editor {
    pub(crate) fn flip_selected_polygons(&mut self) -> Result<()> {
        if self.session.live.is_some()
            || self.session.bridge.is_some()
            || self.session.spike.is_some()
            || self.session.cone.is_some()
            || self.session.ellipsoid.is_some()
            || self.session.shape.is_some()
            || self.session.reduction.is_some()
            || self.selection_start.is_some()
        {
            return Err("Drop or Cancel the active task before Flip".into());
        }
        let candidate = flip_winding(&self.scene)?;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Flip".into(), before);
        Ok(())
    }
}

/// `scene` with the polygons Flip works on turned to face the other way:
/// the selected polygons in Polygon mode, every polygon when none is
/// selected or in Item mode. Vertex and Edge mode refuse.
pub(crate) fn flip_winding(scene: &Scene) -> Result<Scene> {
    scene.validate()?;
    if matches!(scene.selection.mode, Mode::Vertex | Mode::Edge) {
        return Err("Need to directly select one or more polygons.".into());
    }
    let item = scene
        .active()
        .filter(|i| scene.shows(i))
        .ok_or("Select a visible mesh for Flip")?;
    let mesh = &item.mesh;
    // Nothing selected, or Item mode: every polygon (FL05, FL06).
    let all = scene.selection.mode != Mode::Polygon || scene.selection.faces.is_empty();
    let chosen: Vec<u32> = if all {
        (0..mesh.faces.len() as u32)
            .filter(|&f| !mesh.is_polyline(f as usize))
            .collect()
    } else {
        scene.selection.faces.clone()
    };
    let selected: BTreeSet<usize> = chosen.iter().map(|&f| f as usize).collect();
    if selected.len() != chosen.len() || selected.iter().any(|&f| f >= mesh.faces.len()) {
        return Err("Invalid or duplicate Flip selection".into());
    }
    let mut affected = BTreeSet::new();
    for &fi in &selected {
        // Sub-D (Subdivs / Catmull-Clark) faces are ordinary polygons
        // drawn smoothed, so their winding flips like any face. Only
        // curves and polylines have no facing to flip.
        if mesh.is_polyline(fi) {
            return Err("Flip works on polygons, not curves".into());
        }
        let face = &mesh.faces[fi];
        if mesh
            .attributes
            .corner_normals
            .values()
            .any(|map| map[fi].iter().any(Option::is_some))
            || mesh.attributes.typed_maps.values().any(|map| {
                map.kind == "normal3"
                    && ((!map.corners.is_empty() && map.corners[fi].iter().any(Option::is_some))
                        || face.vertices.iter().any(|v| map.points.contains_key(v)))
            })
        {
            return Err("Flip of authored normals is not supported".into());
        }
        for i in 0..face.vertices.len() {
            let a = face.vertices[i];
            let b = face.vertices[(i + 1) % face.vertices.len()];
            affected.insert((a.min(b), a.max(b)));
        }
    }
    let mut uses: BTreeMap<(u32, u32), Vec<[u32; 3]>> = BTreeMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) {
            continue;
        }
        for i in 0..face.vertices.len() {
            let a = face.vertices[i];
            let b = face.vertices[(i + 1) % face.vertices.len()];
            let k = (a.min(b), a.max(b));
            if affected.contains(&k) {
                uses.entry(k).or_default().push([fi as u32, a, b]);
            }
        }
    }
    if uses.values().any(|v| v.len() > 2) {
        return Err("Flip on nonmanifold edge is unsupported".into());
    }
    let mut candidate = scene.clone();
    let out = &mut candidate.active_mut().ok_or("Missing Flip target")?.mesh;
    // The first corner stays first; the rest are reversed (rule 1).
    fn flip<T>(row: &mut [T]) {
        if row.len() > 1 {
            row[1..].reverse();
        }
    }
    for &fi in &selected {
        flip(&mut out.faces[fi].vertices);
        if let Some(uv) = &mut out.faces[fi].uv {
            flip(uv);
        }
        for map in out.uv_maps.values_mut() {
            flip(&mut map[fi]);
        }
        for map in out.attributes.uv_records.values_mut() {
            flip(&mut map[fi]);
        }
        for map in out.attributes.corner_weights.values_mut() {
            flip(&mut map[fi]);
        }
        for map in out.attributes.corner_colors.values_mut() {
            flip(&mut map[fi]);
        }
        for map in out.attributes.corner_normals.values_mut() {
            flip(&mut map[fi]);
        }
        for map in out.attributes.typed_maps.values_mut() {
            if !map.corners.is_empty() {
                flip(&mut map.corners[fi]);
            }
        }
    }
    for (edge, records) in &mut uses {
        for record in records.iter_mut() {
            if selected.contains(&(record[0] as usize)) {
                record.swap(1, 2);
            }
        }
        let key = format!("{}:{}", edge.0, edge.1);
        if records.len() == 2 && records[0][1..] == records[1][1..] {
            out.attributes
                .extrude_edge_uses
                .insert(key, records.clone());
        } else {
            out.attributes.extrude_edge_uses.remove(&key);
        }
    }
    candidate.validate()?;
    Ok(candidate)
}
