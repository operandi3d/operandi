//! MP-C10 basic Polygon Remove (REM-001..003): delete the explicitly selected
//! surface polygons of the active mesh as a single undoable edit.
//!
//! Supported context only: one visible active mesh, Polygon mode, a nonempty
//! explicit selection of valid surface faces, no live/armed tool, I2 stage or
//! selection gesture. Everything else is refused before mutation.
//!
//! Geometry contract: only the selected face slots are deleted. Every vertex
//! position and point payload is retained, including vertices that become
//! unused and pre-existing isolated points; the mesh item is kept even when
//! its last face is removed. Surviving faces keep their cycles, winding and
//! payload in their original relative order. There is no capping, welding,
//! cleanup or cascading deletion.
//!
//! Correspondence: output face `i` is source face `survivors[i]`. Every
//! face/corner-indexed attribute is rebuilt from that one map, and stable
//! identities use the same map through `MeshIdentity::remap_faces`. Payload
//! owned exclusively by deleted faces/corners/edges is dropped; nothing is
//! merged, renumbered (UV record IDs included) or invented.
use crate::{Editor, Mesh, Mode, Result};
use std::collections::BTreeSet;

/// The result of removing faces from one mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceRemoval {
    pub mesh: Mesh,
    /// Source face index for every output face, strictly increasing.
    pub survivors: Vec<usize>,
}

/// Remove exactly `removed` (source face indices, any order) from `source`.
/// Refuses empty, duplicate, out-of-range and Polyline targets, and any input
/// or output that fails full mesh validation, without partial results.
pub fn remove_faces(source: &Mesh, removed: &[u32]) -> Result<FaceRemoval> {
    source.validate()?;
    if removed.is_empty() {
        return Err("Select polygons before Remove".into());
    }
    let mut targets = BTreeSet::new();
    for &f in removed {
        if f as usize >= source.faces.len() {
            return Err("Selected polygon no longer exists".into());
        }
        if !targets.insert(f) {
            return Err("Duplicate polygon in Remove selection".into());
        }
        if source.is_polyline(f as usize) {
            return Err("Remove supports surface polygons only".into());
        }
    }
    let survivors: Vec<usize> = (0..source.faces.len())
        .filter(|&f| !targets.contains(&(f as u32)))
        .collect();
    // Source face index -> output face index, None for deleted slots.
    let mut remap = vec![None; source.faces.len()];
    for (out, &src) in survivors.iter().enumerate() {
        remap[src] = Some(out as u32);
    }
    let remap_face = |f: &u32| remap[*f as usize];

    let mut out = Mesh {
        vertices: source.vertices.clone(),
        faces: survivors.iter().map(|&f| source.faces[f].clone()).collect(),
        uv_maps: source
            .uv_maps
            .iter()
            .map(|(name, faces)| (name.clone(), keep(faces, &survivors)))
            .collect(),
        attributes: source.attributes.clone(),
    };
    let at = &mut out.attributes;
    // Point-indexed payload (positions, point maps/UVs/weights/colors/morphs,
    // vertex sets, typed point records) is unchanged: no vertex is removed.
    for map in at.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            map.corners = keep(&map.corners, &survivors);
        }
    }
    for set in at.polygon_sets.values_mut() {
        *set = set.iter().filter_map(remap_face).collect();
    }
    for map in at.corner_weights.values_mut() {
        *map = keep(map, &survivors);
    }
    for map in at.corner_colors.values_mut() {
        *map = keep(map, &survivors);
    }
    for map in at.corner_normals.values_mut() {
        *map = keep(map, &survivors);
    }
    for records in at.uv_records.values_mut() {
        *records = keep(records, &survivors);
    }
    if !at.smoothing_labels.is_empty() {
        at.smoothing_labels = keep(&at.smoothing_labels, &survivors);
    }
    if !at.representations.is_empty() {
        at.representations = keep(&at.representations, &survivors);
    }
    at.degenerate_faces = at.degenerate_faces.iter().filter_map(remap_face).collect();
    // Exact exceptional-incidence ledger: drop deleted uses, renumber the
    // survivors through the same map, and keep an entry only while its
    // remaining uses are still exceptional under `Mesh::validate`'s rule.
    at.extrude_edge_uses = std::mem::take(&mut at.extrude_edge_uses)
        .into_iter()
        .filter_map(|(edge, uses)| {
            let uses: Vec<[u32; 3]> = uses
                .iter()
                .filter_map(|&[f, a, b]| remap[f as usize].map(|f| [f, a, b]))
                .collect();
            (uses.len() > 2 || (uses.len() == 2 && uses[0][1..] == uses[1][1..]))
                .then_some((edge, uses))
        })
        .collect();
    // A crease belongs to an edge; keep it only while a surviving face still
    // bounds that edge. Surviving creases keep their order and weight.
    let edges: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    out.attributes
        .creases
        .retain(|c| edges.contains(&(c.a, c.b)));
    out.validate()?;
    Ok(FaceRemoval {
        mesh: out,
        survivors,
    })
}

/// Face-indexed values of the surviving faces, in output order.
/// The Delete key on points or polygons (Jason, 2026-11-10): `faces` go,
/// and so do the `points` and every point only those faces used, so no
/// stray points stay behind. Deleting a point shared by four quads takes
/// the four quads with it and leaves one hole along their outer boundary.
/// Points other polygons still use, and points that were already unused,
/// stay.
pub fn delete_geometry(source: &Mesh, faces: &[u32], points: &[u32]) -> Result<Mesh> {
    let mut orphans: std::collections::BTreeSet<u32> = points.iter().copied().collect();
    for &f in faces {
        let face = source
            .faces
            .get(f as usize)
            .ok_or("Selected polygon no longer exists")?;
        orphans.extend(face.vertices.iter().copied());
    }
    let kept = if faces.is_empty() {
        source.clone()
    } else {
        remove_faces(source, faces)?.mesh
    };
    Ok(crate::mesh_cleanup::remove_unreferenced(&kept, Some(&orphans))?.mesh)
}

fn keep<T: Clone>(values: &[T], survivors: &[usize]) -> Vec<T> {
    survivors.iter().map(|&f| values[f].clone()).collect()
}

impl Editor {
    /// Read-only Remove eligibility: the active item and its selected faces
    /// in selection order. Shared by every route before dispatch.
    pub fn remove_polygons_target(&self) -> Result<(u64, Vec<u32>)> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Remove".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Remove".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Remove requires Polygon selection mode".into());
        }
        let item = self.scene.active().ok_or("Select a mesh before Remove")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before Remove".into());
        }
        let faces = self.scene.selection.faces.clone();
        if faces.is_empty() {
            return Err("Select polygons before Remove".into());
        }
        for &f in &faces {
            if f as usize >= item.mesh.faces.len() {
                return Err("Selected polygon no longer exists".into());
            }
            if item.mesh.is_polyline(f as usize) {
                return Err("Remove supports surface polygons only".into());
            }
        }
        Ok((item.id, faces))
    }
    /// One atomic edit: the mesh, its stable identities and the selection
    /// change together or not at all. Removed faces leave the face selection;
    /// remembered edges that no longer bound any face leave the edge
    /// selection; vertex selection is untouched because no vertex is removed.
    pub(crate) fn remove_polygons(&mut self) -> Result<()> {
        let (item_id, faces) = self.remove_polygons_target()?;
        let item = self.scene.active().ok_or("Select a mesh before Remove")?;
        let source: &Mesh = &item.mesh;
        let removal = remove_faces(source, &faces)?;
        let identities = self
            .identities
            .current
            .meshes
            .get(&item_id)
            .ok_or("Missing mesh identity owner")?
            .remap_faces(source, &removal.mesh, &removal.survivors)?;

        let before = self.scene.clone();
        let previous_identities = self.identities.current.clone();
        let edges = removal.mesh.edges();
        let item = self
            .scene
            .active_mut()
            .ok_or("Select a mesh before Remove")?;
        item.mesh = removal.mesh.into();
        let selection = &mut self.scene.selection;
        selection.faces.clear();
        selection.edges.retain(|e| edges.binary_search(e).is_ok());
        self.identities
            .current
            .meshes
            .insert(item_id, identities.into());
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = previous_identities;
            return Err(e);
        }
        self.record("Remove".into(), before);
        Ok(())
    }

    /// `delete` (forward Delete) in a component mode as one undoable edit:
    /// the selected polygons, or every surface polygon using a selected
    /// point, go with the points they leave unused (`delete_geometry`);
    /// selected edges are removed with their leftover points
    /// (`edge_delete_geometry`).
    pub(crate) fn delete_components(&mut self) -> Result<()> {
        let item = self.edge_insert_target("Delete")?;
        let mesh = &self
            .scene
            .active()
            .ok_or("Select a mesh before Delete")?
            .mesh;
        let selection = &self.scene.selection;
        let (faces, points) = match selection.mode {
            Mode::Polygon => {
                if selection.faces.is_empty() {
                    return Err("Select polygons before Delete".into());
                }
                (selection.faces.clone(), Vec::new())
            }
            Mode::Vertex => {
                if selection.vertices.is_empty() {
                    return Err("Select points before Delete".into());
                }
                if selection
                    .vertices
                    .iter()
                    .any(|&v| v as usize >= mesh.vertices.len())
                {
                    return Err("A selected point no longer exists".into());
                }
                let chosen: std::collections::BTreeSet<u32> =
                    selection.vertices.iter().copied().collect();
                let faces = (0..mesh.faces.len())
                    .filter(|&f| !mesh.is_polyline(f))
                    .filter(|&f| mesh.faces[f].vertices.iter().any(|v| chosen.contains(v)))
                    .map(|f| f as u32)
                    .collect();
                (faces, selection.vertices.clone())
            }
            Mode::Edge => {
                let out = crate::edge_delete_geometry(mesh, &selection.edges)?;
                return self.publish_vertex_edit(item, out, "Delete", Vec::new());
            }
            _ => return Err("Delete removes points, edges or polygons here".into()),
        };
        let out = delete_geometry(mesh, &faces, &points)?;
        self.publish_vertex_edit(item, out, "Delete", Vec::new())
    }
}
