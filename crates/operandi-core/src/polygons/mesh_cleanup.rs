//! CLEAN-001/002: remove only vertices not referenced by a polygon.
//!
//! This operation uses one explicit, order-preserving point correspondence for
//! topology, sparse point records, selections and stable component identities.
use crate::{Editor, Mesh, MeshIdentity, Mode, Result, Selection, Shared};
use std::collections::BTreeMap;

/// The target captured when the options dialog opened. Commit rechecks it.
#[derive(Clone, Debug, PartialEq)]
pub struct CleanupTarget {
    document: crate::DocumentToken,
    item: u64,
    selection: Selection,
    mesh: Shared<Mesh>,
}

impl CleanupTarget {
    pub(crate) fn mesh_vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }
}

/// An output mesh and the source point index of every output point.
#[derive(Clone, Debug, PartialEq)]
pub struct FloatingCleanup {
    pub mesh: Mesh,
    pub survivors: Vec<usize>,
}

fn remap_records<T>(records: &mut BTreeMap<u32, T>, remap: &[Option<u32>]) {
    *records = std::mem::take(records)
        .into_iter()
        .filter_map(|(old, value)| remap[old as usize].map(|new| (new, value)))
        .collect();
}

/// Remove unreferenced points without welding or changing face/corner payloads.
/// Full validation makes every old index used by the mapping safe to inspect.
pub fn remove_floating_vertices(source: &Mesh) -> Result<FloatingCleanup> {
    remove_unreferenced(source, None)
}

/// Remove unreferenced points; with `only`, just those of them in the set
/// (so an edit can drop the points it orphaned and keep pre-existing ones).
pub(crate) fn remove_unreferenced(
    source: &Mesh,
    only: Option<&std::collections::BTreeSet<u32>>,
) -> Result<FloatingCleanup> {
    source.validate()?;
    let mut used = vec![false; source.vertices.len()];
    for face in &source.faces {
        for &point in &face.vertices {
            used[point as usize] = true;
        }
    }
    if let Some(only) = only {
        for (i, used) in used.iter_mut().enumerate() {
            if !only.contains(&(i as u32)) {
                *used = true;
            }
        }
    }
    let survivors: Vec<usize> = used
        .iter()
        .enumerate()
        .filter_map(|(i, used)| used.then_some(i))
        .collect();
    Ok(FloatingCleanup {
        mesh: remap_points(source, &survivors)?,
        survivors,
    })
}

/// Renumber points: output point `i` is source point `survivors[i]`. Every
/// polygon must reference only kept points. Point records, vertex sets,
/// creases and exceptional edge records follow their point. Used by cleanup
/// (ascending survivors) and by M3 Reorder/Sync Index Order (permutations).
pub(crate) fn remap_points(source: &Mesh, survivors: &[usize]) -> Result<Mesh> {
    let mut remap = vec![None; source.vertices.len()];
    for (new, &old) in survivors.iter().enumerate() {
        remap[old] =
            Some(u32::try_from(new).map_err(|_| "Cleanup vertex index exceeds supported range")?);
    }
    let point = |old: u32| {
        remap[old as usize]
            .ok_or_else(|| "Cleanup encountered an unreferenced edge endpoint".to_string())
    };
    let mut output = source.clone();
    output.vertices = survivors.iter().map(|&i| source.vertices[i]).collect();
    for face in &mut output.faces {
        for vertex in &mut face.vertices {
            *vertex = point(*vertex)?;
        }
    }
    let attributes = &mut output.attributes;
    for map in attributes.point_weights.values_mut() {
        remap_records(map, &remap);
    }
    for map in attributes.point_colors.values_mut() {
        remap_records(map, &remap);
    }
    for map in attributes.point_uvs.values_mut() {
        remap_records(map, &remap);
    }
    for map in attributes.relative_morphs.values_mut() {
        remap_records(map, &remap);
    }
    for map in attributes.typed_maps.values_mut() {
        remap_records(&mut map.points, &remap);
    }
    for set in attributes.vertex_sets.values_mut() {
        *set = std::mem::take(set)
            .into_iter()
            .filter_map(|v| remap[v as usize])
            .collect();
    }
    attributes.edge_sets =
        crate::selection_sets::remap_edge_sets(&attributes.edge_sets, |v| remap[v as usize]);
    for crease in &mut attributes.creases {
        // Creases are stored with a < b; a permutation can swap the order.
        let (a, b) = (point(crease.a)?, point(crease.b)?);
        (crease.a, crease.b) = (a.min(b), a.max(b));
    }
    let mut exceptions = BTreeMap::new();
    for uses in attributes.extrude_edge_uses.values() {
        let mapped: Vec<[u32; 3]> = uses
            .iter()
            .map(|&[face, a, b]| Ok([face, point(a)?, point(b)?]))
            .collect::<Result<_>>()?;
        let [_, a, b] = *mapped
            .first()
            .ok_or("Cleanup encountered an empty exceptional edge")?;
        let key = format!("{}:{}", a.min(b), a.max(b));
        if exceptions.insert(key, mapped).is_some() {
            return Err("Cleanup encountered duplicate exceptional edges".into());
        }
    }
    attributes.extrude_edge_uses = exceptions;
    output.validate()?;
    Ok(output)
}

pub(crate) fn remap_identities(
    source: &Mesh,
    output: &Mesh,
    old: &MeshIdentity,
    survivors: &[usize],
) -> Result<MeshIdentity> {
    old.validate(source)?;
    if survivors.windows(2).any(|w| w[0] >= w[1])
        || survivors.iter().any(|&i| i >= source.vertices.len())
        || survivors.len() != output.vertices.len()
        || survivors
            .iter()
            .zip(&output.vertices)
            .any(|(&i, p)| source.vertices[i] != *p)
        || source.faces.len() != output.faces.len()
    {
        return Err("Invalid Cleanup point correspondence".into());
    }
    let mut result = old.clone();
    result.vertices = survivors.iter().map(|&i| old.vertices[i]).collect();
    result.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
    // The operation preserves every face, corner and topological edge. Edge
    // keys use stable vertex IDs, not the remapped dense indices.
    result.validate(output)?;
    Ok(result)
}

impl Editor {
    /// Inspect the explicit idle Item target; does not mutate the document.
    pub fn cleanup_target(&self) -> Result<CleanupTarget> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Mesh Cleanup".into());
        }
        if self.session.pen_draft.is_some() {
            return Err("Finish or Cancel Pen before Mesh Cleanup".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Mesh Cleanup".into());
        }
        let item = self
            .scene
            .active()
            .ok_or("Select a mesh before Mesh Cleanup")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before Mesh Cleanup".into());
        }
        Ok(CleanupTarget {
            document: self.identities.document_token(),
            item: item.id,
            selection: self.scene.selection.clone(),
            mesh: item.mesh.clone().into(),
        })
    }

    // Read-only preflight must run before the identity transaction: even a
    // staged transaction can allocate IDs or complete old history checkpoints.
    pub(crate) fn cleanup_preflight(
        &self,
        expected: Option<&CleanupTarget>,
    ) -> Result<(CleanupTarget, FloatingCleanup)> {
        self.scene.validate()?;
        let target = self.cleanup_target()?;
        if expected.is_some_and(|expected| expected != &target) {
            return Err("Mesh Cleanup target changed; reopen the dialog".into());
        }
        let cleaned = remove_floating_vertices(&target.mesh)?;
        Ok((target, cleaned))
    }

    /// Mesh Cleanup with the dialog's twelve options (mesh-cleanup-2026-11-05).
    /// Scope: the whole mesh in Item mode and with nothing selected; only the
    /// selected polygons in Polygon mode; a Vertex or Edge mode selection does
    /// nothing (decided Q-MC-01). An already-clean mesh is a history no-op.
    pub(crate) fn mesh_cleanup(
        &mut self,
        flags: [bool; 12],
        expected: Option<CleanupTarget>,
    ) -> Result<()> {
        self.scene.validate()?;
        let target = self.cleanup_target()?;
        if expected.as_ref().is_some_and(|e| e != &target) {
            return Err("Mesh Cleanup target changed; reopen the dialog".into());
        }
        let selection = &self.scene.selection;
        let scope: Option<std::collections::BTreeSet<usize>> = match selection.mode {
            Mode::Vertex if !selection.vertices.is_empty() => return Ok(()),
            Mode::Edge if !selection.edges.is_empty() => return Ok(()),
            Mode::Polygon if !selection.faces.is_empty() => {
                Some(selection.faces.iter().map(|&f| f as usize).collect())
            }
            _ => None,
        };
        let result =
            crate::mesh_cleanup_ops::mesh_cleanup_geometry(&target.mesh, scope.as_ref(), flags)?;
        if result.mesh == *target.mesh {
            return Ok(());
        }
        let before = self.scene.clone();
        let old_ids = self.identities.current.clone();
        self.scene
            .active_mut()
            .ok_or("Select a mesh before Mesh Cleanup")?
            .mesh = result.mesh.into();
        let point = |v: u32| result.points.get(v as usize).copied().flatten();
        let selection = &mut self.scene.selection;
        // Removed polygons leave the selection; the others follow (rule 13).
        selection.faces = selection
            .faces
            .iter()
            .filter_map(|&f| result.faces.get(f as usize).copied().flatten())
            .collect();
        selection.vertices = selection
            .vertices
            .iter()
            .filter_map(|&v| point(v))
            .collect();
        selection.edges = selection
            .edges
            .iter()
            .filter_map(|&(a, b)| Some((point(a)?, point(b)?)))
            .collect();
        if let Err(error) = self
            .scene
            .validate()
            .and_then(|()| self.identities.invalidate_mesh(&self.scene, target.item))
        {
            self.scene = before;
            self.identities.current = old_ids;
            return Err(error);
        }
        self.record("Mesh Cleanup".into(), before);
        Ok(())
    }

    /// Commit a single atomic cleanup; an already-clean mesh is a history no-op.
    pub(crate) fn cleanup_floating(&mut self, expected: Option<CleanupTarget>) -> Result<()> {
        let (target, cleaned) = self.cleanup_preflight(expected.as_ref())?;
        if cleaned.survivors.len() == target.mesh.vertices.len() {
            return Ok(());
        }
        let old = self
            .identities
            .current
            .meshes
            .get(&target.item)
            .ok_or("Missing mesh identity owner")?;
        let identities = remap_identities(&target.mesh, &cleaned.mesh, old, &cleaned.survivors)?;
        let mut remap = vec![None; target.mesh.vertices.len()];
        for (new, &old) in cleaned.survivors.iter().enumerate() {
            remap[old] = Some(new as u32);
        }
        let map_point = |v: u32| remap.get(v as usize).copied().flatten();
        let before = self.scene.clone();
        let old_ids = self.identities.current.clone();
        self.scene
            .active_mut()
            .ok_or("Select a mesh before Mesh Cleanup")?
            .mesh = cleaned.mesh.into();
        self.scene.selection.vertices = self
            .scene
            .selection
            .vertices
            .iter()
            .filter_map(|&v| map_point(v))
            .collect();
        self.scene.selection.edges = self
            .scene
            .selection
            .edges
            .iter()
            .filter_map(|&(a, b)| Some((map_point(a)?, map_point(b)?)))
            .collect();
        self.identities
            .current
            .meshes
            .insert(target.item, identities.into());
        if let Err(error) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = old_ids;
            return Err(error);
        }
        self.record("Mesh Cleanup".into(), before);
        Ok(())
    }
}

/// Lets a mesh with the polygons Mesh Cleanup exists to fix pass
/// validation unchanged (the MCP test bridge's `scene_load`, BRQ-60):
/// 1- and 2-point polygons become open straight lines (the only form
/// Operandi keeps them in; their points and list position stay), and
/// polygons with a repeated point or no area are marked degenerate. Points
/// and polygon order are untouched; nothing is welded or removed.
pub fn keep_broken_polygons(mesh: &mut crate::Mesh) {
    use crate::PolygonRepresentation as P;
    let n = mesh.faces.len();
    let short: Vec<usize> = (0..n)
        .filter(|&f| (1..=2).contains(&mesh.faces[f].vertices.len()))
        .collect();
    if !short.is_empty() {
        let reps = &mut mesh.attributes.representations;
        if reps.is_empty() {
            *reps = vec![P::Face; n];
        }
        for f in short {
            reps[f] = P::Polyline {
                closed: false,
                curve: Default::default(),
            };
        }
    }
    for f in 0..n {
        if mesh.is_polyline(f) {
            continue;
        }
        let v = &mesh.faces[f].vertices;
        let repeats = {
            let mut seen = v.clone();
            seen.sort_unstable();
            seen.windows(2).any(|w| w[0] == w[1])
        };
        let in_range = v.iter().all(|&i| (i as usize) < mesh.vertices.len());
        if in_range && (repeats || mesh.normal(f) == glam::DVec3::ZERO) {
            mesh.attributes.degenerate_faces.insert(f as u32);
        }
    }
}
