//! MP-C11 Polygon Collapse: delete the selected polygons and heal the hole by
//! merging their points. Collapse removes the selection
//! without leaving a hole: neighbouring polygons close the gap.
//!
//! Rule: selected surface polygons are grouped into connected pieces (pieces
//! sharing a point are one piece). Every point of a piece is replaced by one
//! new point at the arithmetic mean of that piece's distinct points, and the
//! selected polygons are deleted. Neighbouring polygons are repaired: a run of
//! consecutive corners that became the same new point keeps only its first
//! corner in cyclic order. A neighbour left with fewer than three corners
//! (for example a triangle with an edge on the piece) is deleted, which is the
//! healing step. Selections on the mesh boundary are supported.
//!
//! Still refused, atomically: polylines or explicitly degenerate polygons in
//! or touching the selection; meshes with exceptional edge-use ledgers; a
//! repair that would pinch one polygon at a new point non-consecutively, or
//! fold/flatten a repaired polygon; differing point-map values across one
//! piece (point data is moved, never averaged); and any result that fails
//! full mesh validation (for example a nonmanifold edge).
//!
//! Data: faces keep their payload and relative order; deleted faces drop
//! theirs, as Remove does. A surviving corner keeps its own corner data; the
//! data of dropped corners in a collapsed run leaves with them. Creases
//! between two points of one piece disappear with their edge; creases to a
//! piece move to its new point (the first of several that land on one edge
//! wins); creases whose edge no longer exists are dropped.
//!
//! Output layout: surviving points keep source order and the new points are
//! appended in piece order (ordered by each piece's lowest source point);
//! faces keep source order. [`CollapseCorrespondence`] is the explicit source
//! association consumed by `MeshIdentity::collapsed`.
use crate::polygon_triple::TRIPLE_AREA_TOLERANCE;
use crate::{Editor, Face, Mesh, MeshCrease, Mode, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit source association of a Collapse result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CollapseCorrespondence {
    /// Source point of every output point; `None` only for new points.
    pub vertex_sources: Vec<Option<u32>>,
    /// Source face of every output face, strictly increasing.
    pub face_sources: Vec<u32>,
    /// Source corner ordinal of every output corner, per output face.
    pub corner_sources: Vec<Vec<usize>>,
}

/// A validated Collapse result for one mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct CollapseOutcome {
    pub mesh: Mesh,
    pub correspondence: CollapseCorrespondence,
    /// Source points of each collapsed piece, ascending, in piece order.
    pub pieces: Vec<Vec<u32>>,
    /// Output index of each piece's new point, in piece order.
    pub points: Vec<u32>,
    /// Number of selected polygons deleted.
    pub collapsed_faces: usize,
}

fn refuse(detail: impl std::fmt::Display) -> String {
    format!("Collapse refuses {detail}; nothing changed")
}

/// Collapse the single source face `face`; see [`collapse_faces`].
pub fn collapse_face(source: &Mesh, face: u32) -> Result<CollapseOutcome> {
    collapse_faces(source, &[face])
}

/// Collapse the selected source faces of `source`. Refuses every unsupported
/// input before producing a result; the result passes full mesh validation.
pub fn collapse_faces(source: &Mesh, selected: &[u32]) -> Result<CollapseOutcome> {
    source.validate()?;
    let selected: BTreeSet<u32> = selected.iter().copied().collect();
    if selected.is_empty() {
        return Err("Select polygons before Collapse".into());
    }
    for &f in &selected {
        if source.faces.get(f as usize).is_none() {
            return Err("Selected polygon no longer exists".into());
        }
        if source.is_polyline(f as usize) {
            return Err(refuse("a Polyline; select surface polygons"));
        }
        if source.attributes.degenerate_faces.contains(&f) {
            return Err(refuse("a degenerate polygon"));
        }
    }
    if !source.attributes.extrude_edge_uses.is_empty() {
        return Err(refuse("a mesh with exceptional edge uses"));
    }

    // Connected pieces of the selection, joined through shared points.
    let mut parent: BTreeMap<u32, u32> = BTreeMap::new();
    fn find(parent: &mut BTreeMap<u32, u32>, v: u32) -> u32 {
        let p = parent[&v];
        if p == v {
            return v;
        }
        let root = find(parent, p);
        parent.insert(v, root);
        root
    }
    for &f in &selected {
        let vs = &source.faces[f as usize].vertices;
        for &v in vs {
            parent.entry(v).or_insert(v);
        }
        for w in vs.windows(2) {
            let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
            if a != b {
                parent.insert(a.max(b), a.min(b));
            }
        }
    }
    let members: Vec<u32> = parent.keys().copied().collect();
    let mut by_root: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for v in members {
        let root = find(&mut parent, v);
        by_root.entry(root).or_default().push(v);
    }
    let mut pieces: Vec<Vec<u32>> = by_root.into_values().collect();
    pieces.sort_by_key(|p| p[0]);
    let mut piece_of: Vec<Option<usize>> = vec![None; source.vertices.len()];
    for (k, piece) in pieces.iter().enumerate() {
        for &v in piece {
            piece_of[v as usize] = Some(k);
        }
    }
    let is_source = |v: u32| piece_of[v as usize].is_some();

    // Nothing touching a piece may be a polyline or a degenerate polygon.
    for (f, poly) in source.faces.iter().enumerate() {
        if selected.contains(&(f as u32)) || !poly.vertices.iter().any(|&v| is_source(v)) {
            continue;
        }
        if source.is_polyline(f) {
            return Err(refuse("a selection touched by a Polyline"));
        }
        if source.attributes.degenerate_faces.contains(&(f as u32)) {
            return Err(refuse("a selection touched by a degenerate polygon"));
        }
    }

    // Point payload of each piece must agree exactly; it moves, never averages.
    let attributes = &source.attributes;
    for piece in &pieces {
        for (name, map) in &attributes.typed_maps {
            same_point(&map.points, piece, &format!("typed point map {name}"))?;
        }
        for (name, set) in &attributes.vertex_sets {
            if piece
                .iter()
                .any(|v| set.contains(v) != set.contains(&piece[0]))
            {
                return Err(refuse(format!(
                    "vertex set {name} membership that differs across a collapsed piece"
                )));
            }
        }
        for (name, map) in &attributes.point_uvs {
            same_point(map, piece, &format!("point UV map {name}"))?;
        }
        for (name, map) in &attributes.point_weights {
            same_point(map, piece, &format!("point weight map {name}"))?;
        }
        for (name, map) in &attributes.point_colors {
            same_point(map, piece, &format!("point color map {name}"))?;
        }
        for (name, map) in &attributes.relative_morphs {
            same_point(map, piece, &format!("morph {name}"))?;
        }
    }

    // Output points: survivors in source order, then one new point per piece.
    let mut vertex_remap: Vec<Option<u32>> = vec![None; source.vertices.len()];
    let mut vertex_sources = Vec::with_capacity(source.vertices.len());
    for v in 0..source.vertices.len() as u32 {
        if !is_source(v) {
            vertex_remap[v as usize] = Some(vertex_sources.len() as u32);
            vertex_sources.push(Some(v));
        }
    }
    let first_new = vertex_sources.len() as u32;
    let points: Vec<u32> = (0..pieces.len() as u32).map(|k| first_new + k).collect();
    vertex_sources.extend(std::iter::repeat_n(None, pieces.len()));
    let map_vertex = |v: u32| {
        vertex_remap[v as usize].unwrap_or_else(|| points[piece_of[v as usize].unwrap_or(0)])
    };

    // Repaired faces and their corner correspondence.
    let mut face_sources = Vec::with_capacity(source.faces.len());
    let mut corner_sources = Vec::with_capacity(source.faces.len());
    for (f, poly) in source.faces.iter().enumerate() {
        if selected.contains(&(f as u32)) {
            continue;
        }
        let n = poly.vertices.len();
        let mapped: Vec<u32> = poly.vertices.iter().map(|&v| map_vertex(v)).collect();
        let kept: Vec<usize> = if mapped.iter().all(|&m| m == mapped[0]) {
            Vec::new()
        } else {
            (0..n)
                .filter(|&i| mapped[i] != mapped[(i + n - 1) % n])
                .collect()
        };
        if kept.len() < 3 {
            // Healing: this neighbour closes up entirely.
            continue;
        }
        let distinct: BTreeSet<u32> = kept.iter().map(|&i| mapped[i]).collect();
        if distinct.len() != kept.len() {
            return Err(refuse(format!(
                "a repair that would pinch polygon {f} at a new point"
            )));
        }
        face_sources.push(f as u32);
        corner_sources.push(kept);
    }

    if face_sources.is_empty() {
        return Err(refuse(
            "a selection whose collapse would remove the whole mesh",
        ));
    }

    let mut vertices: Vec<DVec3> = vertex_sources
        .iter()
        .flatten()
        .map(|&v| source.vertices[v as usize])
        .collect();
    for piece in &pieces {
        let sum: DVec3 = piece.iter().map(|&v| source.vertices[v as usize]).sum();
        vertices.push(sum / piece.len() as f64);
    }
    let faces: Vec<Face> = face_sources
        .iter()
        .zip(&corner_sources)
        .map(|(&f, cs)| {
            let src = &source.faces[f as usize];
            Face {
                vertices: cs.iter().map(|&c| map_vertex(src.vertices[c])).collect(),
                uv: src
                    .uv
                    .as_ref()
                    .map(|uv| cs.iter().map(|&c| uv[c]).collect()),
                ..src.clone()
            }
        })
        .collect();
    let mut face_remap: Vec<Option<u32>> = vec![None; source.faces.len()];
    for (out, &f) in face_sources.iter().enumerate() {
        face_remap[f as usize] = Some(out as u32);
    }
    let remap_face = |f: &u32| face_remap[*f as usize];
    let representatives: Vec<(u32, u32)> = pieces
        .iter()
        .zip(&points)
        .map(|(piece, &p)| (piece[0], p))
        .collect();

    let mut out_attributes = attributes.clone();
    let at = &mut out_attributes;
    for map in at.typed_maps.values_mut() {
        map.points = move_point(&map.points, &vertex_remap, &representatives);
        if !map.corners.is_empty() {
            map.corners = pick(&map.corners, &face_sources, &corner_sources);
        }
    }
    for set in at.vertex_sets.values_mut() {
        let mut out: BTreeSet<u32> = set
            .iter()
            .filter_map(|&v| vertex_remap[v as usize])
            .collect();
        for &(rep, p) in &representatives {
            if set.contains(&rep) {
                out.insert(p);
            }
        }
        *set = out;
    }
    for set in at.polygon_sets.values_mut() {
        *set = set.iter().filter_map(remap_face).collect();
    }
    // An edge whose ends collapsed together leaves its sets.
    at.edge_sets = crate::selection_sets::remap_edge_sets(&at.edge_sets, |v| {
        vertex_remap[v as usize].or_else(|| piece_of[v as usize].map(|p| points[p]))
    });
    for map in at.point_uvs.values_mut() {
        *map = move_point(map, &vertex_remap, &representatives);
    }
    for map in at.point_weights.values_mut() {
        *map = move_point(map, &vertex_remap, &representatives);
    }
    for map in at.point_colors.values_mut() {
        *map = move_point(map, &vertex_remap, &representatives);
    }
    for map in at.relative_morphs.values_mut() {
        *map = move_point(map, &vertex_remap, &representatives);
    }
    for map in at.corner_weights.values_mut() {
        *map = pick(map, &face_sources, &corner_sources);
    }
    for map in at.corner_colors.values_mut() {
        *map = pick(map, &face_sources, &corner_sources);
    }
    for map in at.corner_normals.values_mut() {
        *map = pick(map, &face_sources, &corner_sources);
    }
    for records in at.uv_records.values_mut() {
        *records = pick(records, &face_sources, &corner_sources);
    }
    if !at.smoothing_labels.is_empty() {
        at.smoothing_labels = keep(&at.smoothing_labels, &face_sources);
    }
    if !at.representations.is_empty() {
        at.representations = keep(&at.representations, &face_sources);
    }
    at.degenerate_faces = at.degenerate_faces.iter().filter_map(remap_face).collect();
    let out_edges: BTreeSet<(u32, u32)> = faces
        .iter()
        .flat_map(|f| {
            let n = f.vertices.len();
            (0..n).map(move |i| {
                let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
                (a.min(b), a.max(b))
            })
        })
        .collect();
    let mut creases: Vec<MeshCrease> = Vec::with_capacity(at.creases.len());
    for crease in &attributes.creases {
        let (p, q) = (map_vertex(crease.a), map_vertex(crease.b));
        let (p, q) = (p.min(q), p.max(q));
        if p == q || !out_edges.contains(&(p, q)) || creases.iter().any(|c| (c.a, c.b) == (p, q)) {
            continue;
        }
        creases.push(MeshCrease {
            a: p,
            b: q,
            weight: crease.weight,
        });
    }
    at.creases = creases;

    let uv_maps = source
        .uv_maps
        .iter()
        .map(|(name, values)| (name.clone(), pick(values, &face_sources, &corner_sources)))
        .collect();
    let mesh = Mesh {
        attributes: out_attributes,
        vertices,
        faces: faces.into(),
        uv_maps,
    };

    // Repaired faces must keep area and orientation. The area floor scales
    // with each repaired face's own neighbourhood (its source corners plus
    // the pieces it touches), never with unrelated geometry elsewhere.
    for (out, &f) in face_sources.iter().enumerate() {
        let src = &source.faces[f as usize].vertices;
        if !src.iter().any(|&v| is_source(v)) {
            continue;
        }
        let touched: BTreeSet<usize> = src.iter().filter_map(|&v| piece_of[v as usize]).collect();
        let local = src
            .iter()
            .chain(touched.iter().flat_map(|&k| pieces[k].iter()))
            .map(|&v| source.vertices[v as usize]);
        let lo = local.clone().fold(DVec3::INFINITY, DVec3::min);
        let hi = local.fold(DVec3::NEG_INFINITY, DVec3::max);
        let extent = (hi - lo).length();
        let before = newell(&source.vertices, src);
        let after = newell(&mesh.vertices, &mesh.faces[out].vertices);
        if after.length() <= 2. * TRIPLE_AREA_TOLERANCE * extent * extent || after.dot(before) <= 0.
        {
            return Err(refuse(format!(
                "a result that would fold or flatten polygon {f}"
            )));
        }
    }
    mesh.validate()
        .map_err(|e| refuse(format!("an invalid result ({e})")))?;
    Ok(CollapseOutcome {
        mesh,
        correspondence: CollapseCorrespondence {
            vertex_sources,
            face_sources,
            corner_sources,
        },
        pieces,
        points,
        collapsed_faces: selected.len(),
    })
}

/// Corner-indexed values of the output faces, in output order.
fn pick<T: Clone>(values: &[Vec<T>], faces: &[u32], corners: &[Vec<usize>]) -> Vec<Vec<T>> {
    faces
        .iter()
        .zip(corners)
        .map(|(&f, cs)| cs.iter().map(|&c| values[f as usize][c].clone()).collect())
        .collect()
}

/// Face-indexed values of the output faces, in output order.
fn keep<T: Clone>(values: &[T], faces: &[u32]) -> Vec<T> {
    faces.iter().map(|&f| values[f as usize].clone()).collect()
}

/// Newell area vector (twice the vector area) of one polygon loop.
fn newell(points: &[DVec3], cycle: &[u32]) -> DVec3 {
    let origin = points[cycle[0] as usize];
    (0..cycle.len()).fold(DVec3::ZERO, |n, i| {
        let p = points[cycle[i] as usize] - origin;
        let q = points[cycle[(i + 1) % cycle.len()] as usize] - origin;
        n + p.cross(q)
    })
}

/// Refuse unless every point of a piece has the same record (presence included).
fn same_point<T: PartialEq>(map: &BTreeMap<u32, T>, points: &[u32], what: &str) -> Result<()> {
    let first = map.get(&points[0]);
    if points.iter().any(|v| map.get(v) != first) {
        return Err(refuse(format!(
            "{what} values that differ across a collapsed piece"
        )));
    }
    Ok(())
}

/// Renumber surviving point records and give each new point the common
/// record of its piece (already proven identical), if any.
fn move_point<T: Clone>(
    map: &BTreeMap<u32, T>,
    remap: &[Option<u32>],
    representatives: &[(u32, u32)],
) -> BTreeMap<u32, T> {
    let mut out: BTreeMap<u32, T> = map
        .iter()
        .filter_map(|(&v, value)| remap[v as usize].map(|o| (o, value.clone())))
        .collect();
    for &(representative, point) in representatives {
        if let Some(value) = map.get(&representative) {
            out.insert(point, value.clone());
        }
    }
    out
}

impl CollapseCorrespondence {
    /// Exact description of `source` → `output`; refuses guessed maps.
    pub fn validate(&self, source: &Mesh, output: &Mesh) -> Result<()> {
        let bad = || Err("Collapse correspondence does not describe this edit".to_string());
        if self.vertex_sources.len() != output.vertices.len()
            || self.vertex_sources.iter().all(Option::is_some)
            || self.face_sources.len() != output.faces.len()
            || self.corner_sources.len() != output.faces.len()
            || self.face_sources.windows(2).any(|w| w[0] >= w[1])
        {
            return bad();
        }
        let mut seen = BTreeSet::new();
        for (v, p) in self.vertex_sources.iter().zip(&output.vertices) {
            if let Some(v) = v
                && (source.vertices.get(*v as usize) != Some(p) || !seen.insert(*v))
            {
                return bad();
            }
        }
        for ((&f, cs), face) in self
            .face_sources
            .iter()
            .zip(&self.corner_sources)
            .zip(&output.faces)
        {
            let Some(src) = source.faces.get(f as usize) else {
                return bad();
            };
            if cs.len() != face.vertices.len() || cs.windows(2).any(|w| w[0] == w[1]) {
                return bad();
            }
            for (&c, &v) in cs.iter().zip(&face.vertices) {
                let Some(&sv) = src.vertices.get(c) else {
                    return bad();
                };
                match self.vertex_sources[v as usize] {
                    Some(origin) if origin != sv => return bad(),
                    None if seen.contains(&sv) => return bad(),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

impl Editor {
    /// Read-only Collapse eligibility: the active item and its selected
    /// faces. Shared by every route before dispatch.
    pub fn collapse_polygon_target(&self) -> Result<(u64, Vec<u32>)> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Collapse".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Collapse".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Collapse requires Polygon selection mode".into());
        }
        let item = self.scene.active().ok_or("Select a mesh before Collapse")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before Collapse".into());
        }
        let faces = &self.scene.selection.faces;
        if faces.is_empty() {
            return Err("Select polygons before Collapse".into());
        }
        if faces.iter().any(|&f| f as usize >= item.mesh.faces.len()) {
            return Err("Selected polygon no longer exists".into());
        }
        Ok((item.id, faces.clone()))
    }
    /// One atomic edit: mesh, stable identities and selection change together
    /// or not at all. The polygon selection is cleared (COL-B01). Remembered
    /// points and edges follow surviving points; collapsed ones drop out.
    pub(crate) fn collapse_polygon(&mut self) -> Result<()> {
        let (item_id, faces) = self.collapse_polygon_target()?;
        let source: Mesh = (*self.scene.active().ok_or("Select a mesh")?.mesh).clone();
        let outcome = collapse_faces(&source, &faces)?;
        let ids = self
            .identities
            .current
            .meshes
            .get(&item_id)
            .ok_or("Missing mesh identity owner")?
            .clone();
        let collapsed_ids = self.identities.with_ids(|next| {
            ids.collapsed(&source, &outcome.mesh, &outcome.correspondence, next)
        })?;
        let before = self.scene.clone();
        let identities_before = self.identities.current.clone();
        let mut point_remap = vec![None; source.vertices.len()];
        for (out, origin) in outcome.correspondence.vertex_sources.iter().enumerate() {
            if let Some(v) = origin {
                point_remap[*v as usize] = Some(out as u32);
            }
        }
        let remap = |v: u32| point_remap.get(v as usize).copied().flatten();
        let edges: BTreeSet<(u32, u32)> = outcome.mesh.edges().into_iter().collect();
        let selection = &mut self.scene.selection;
        selection.faces.clear();
        selection.vertices = selection
            .vertices
            .iter()
            .filter_map(|&v| remap(v))
            .collect();
        selection.edges = selection
            .edges
            .iter()
            .filter_map(|&(a, b)| {
                let (a, b) = (remap(a)?, remap(b)?);
                edges.contains(&(a.min(b), a.max(b))).then_some((a, b))
            })
            .collect();
        self.scene
            .active_mut()
            .ok_or("Select a mesh before Collapse")?
            .mesh = outcome.mesh.into();
        self.identities
            .current
            .meshes
            .insert(item_id, collapsed_ids.into());
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = identities_before;
            return Err(e);
        }
        self.record("Collapse".into(), before);
        Ok(())
    }
}
