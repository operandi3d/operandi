//! MK-C01..03 Make: build one quad from two selected edges.
//!
//! Make creates a quadrangle from two selected edges: it
//! works out where the missing corner of the two edges goes, builds the new
//! polygon and merges it with the connecting edges.
//!
//! * Two edges sharing one point `s` with far ends `p` and `q`: when `p` and
//!   `q` are already joined through one further point `r` by boundary edges
//!   (a four-sided opening), the quad fills that opening with existing points.
//!   Otherwise a new point `r = p + q − s` completes the parallelogram, unless
//!   an existing point lies exactly there, in which case it is reused.
//! * Two separate edges: the quad joins their endpoints, pairing them so the
//!   quad does not cross itself.
//!
//! Winding follows the neighbouring polygons of the selected edges (the new
//! quad traverses each selected boundary edge opposite to its neighbour).
//! Each selected edge must have at most one neighbouring polygon. Nonplanar
//! quads are allowed; folded or degenerate ones are refused.
//!
//! Data: the polygon next to the first selected edge supplies material, part,
//! smoothing and display label (or plain defaults when it has none). Corner
//! records of the new quad are unassigned in every corner map; a new point
//! has no point-map records. Nothing else changes.
use crate::topology::edge_key;
use crate::{
    CornerId, EdgeId, Editor, Face, FaceId, Mesh, MeshIdentity, Mode, PolygonRepresentation,
    Result, VertexId,
};
use glam::DVec3;
use std::collections::BTreeMap;

/// Conservative relative margin for nonzero area, scaled by the square of the
/// longest quad edge.
const RELATIVE_TOLERANCE: f64 = 1e-9;

/// One corner of the planned quad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MakeCorner {
    Existing(u32),
    /// A new point at this position.
    New(DVec3),
}

/// Resolved quad: oriented corners and the source face for tags.
#[derive(Clone, Debug, PartialEq)]
pub struct MakePlan {
    pub item: u64,
    pub corners: [MakeCorner; 4],
    /// Face adjacent to the first selected edge (tag source), if any.
    pub source_face: Option<usize>,
}

/// Directed surface uses `(face, from, to)` per canonical undirected edge.
type EdgeUses = BTreeMap<(u32, u32), Vec<(usize, u32, u32)>>;

/// Resolve the oriented quad for `edges` (selection order).
pub fn make_plan(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<([MakeCorner; 4], Option<usize>)> {
    let [first, second] = edges else {
        return Err("Make requires exactly two selected edges".into());
    };
    let (first, second) = (edge_key(first.0, first.1), edge_key(second.0, second.1));
    if first == second {
        return Err("Make requires two distinct edges".into());
    }
    let mut uses = EdgeUses::new();
    for fi in 0..mesh.faces.len() {
        if mesh.is_polyline(fi) {
            continue;
        }
        for (a, b) in mesh.element_spans(fi) {
            uses.entry(edge_key(a, b)).or_default().push((fi, a, b));
        }
    }
    for e in [first, second] {
        if uses.get(&e).is_some_and(|u| u.len() > 1) {
            return Err(
                "Make requires boundary edges; a selected edge already has polygons on both sides (already filled), so no duplicate polygon is added"
                    .into(),
            );
        }
    }
    let boundary = |e: (u32, u32)| uses.get(&e).is_some_and(|u| u.len() == 1);
    let pos = |v: u32| mesh.vertices[v as usize];
    let shared: Vec<u32> = [first.0, first.1]
        .into_iter()
        .filter(|v| *v == second.0 || *v == second.1)
        .collect();
    // Unoriented loop first; winding is fixed below. `start` is the corner
    // the loop begins at (the far end of the first edge, as in the admitted
    // fixtures); only cyclic rotation depends on it.
    let mut start = None;
    let cycle: [MakeCorner; 4] = match shared[..] {
        [s] => {
            let p = if first.0 == s { first.1 } else { first.0 };
            let q = if second.0 == s { second.1 } else { second.0 };
            start = Some(p);
            if uses.contains_key(&edge_key(p, q)) {
                return Err(
                    "The selected edges already close a three-corner opening; Make is ambiguous"
                        .into(),
                );
            }
            // An existing four-sided opening wins; then an exactly coincident
            // point; otherwise a new point completes the parallelogram.
            let mut ring: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for &e in uses.keys() {
                if boundary(e) {
                    ring.entry(e.0).or_default().push(e.1);
                    ring.entry(e.1).or_default().push(e.0);
                }
            }
            let near = |v: u32| ring.get(&v).cloned().unwrap_or_default();
            let opening: Vec<u32> = near(p)
                .into_iter()
                .filter(|r| *r != s && *r != q && near(q).contains(r))
                .collect();
            let target = pos(p) + pos(q) - pos(s);
            let r = match opening[..] {
                [r] if uses.contains_key(&edge_key(s, r)) => {
                    return Err("The opening already has a diagonal edge; Make is ambiguous".into());
                }
                [r] => MakeCorner::Existing(r),
                [] => match (0..mesh.vertices.len() as u32).find(|&v| pos(v) == target) {
                    Some(v) if v != s && v != p && v != q => MakeCorner::Existing(v),
                    _ => MakeCorner::New(target),
                },
                _ => {
                    return Err(
                        "The selected edges border more than one possible opening; Make is ambiguous"
                            .into(),
                    );
                }
            };
            [
                MakeCorner::Existing(s),
                MakeCorner::Existing(p),
                r,
                MakeCorner::Existing(q),
            ]
        }
        [] => {
            // Separate edges (a, b) and (c, d): pair a with the nearer of c, d
            // so the quad's sides do not cross.
            let (a, b) = first;
            let (c, d) = second;
            let (c, d) = if pos(a).distance(pos(d)) <= pos(a).distance(pos(c)) {
                (c, d)
            } else {
                (d, c)
            };
            [a, b, c, d].map(MakeCorner::Existing)
        }
        _ => return Err("Make requires two distinct edges".into()),
    };
    let point = |c: MakeCorner| match c {
        MakeCorner::Existing(v) => pos(v),
        MakeCorner::New(p) => p,
    };
    let id = |c: MakeCorner| match c {
        MakeCorner::Existing(v) => Some(v),
        MakeCorner::New(_) => None,
    };
    // Winding: every existing boundary edge of the loop must be traversed
    // opposite to its one neighbour.
    let agrees = |loop_: [MakeCorner; 4]| {
        (0..4).all(|i| {
            let (Some(a), Some(b)) = (id(loop_[i]), id(loop_[(i + 1) % 4])) else {
                return true;
            };
            match uses.get(&edge_key(a, b)).map(|u| u[0]) {
                Some((_, from, to)) => (from, to) == (b, a),
                None => true,
            }
        })
    };
    let reversed = [cycle[0], cycle[3], cycle[2], cycle[1]];
    let oriented = match (agrees(cycle), agrees(reversed)) {
        (true, _) => cycle,
        (false, true) => reversed,
        (false, false) => {
            return Err(
                "The selected edges' neighbouring polygons have inconsistent winding".into(),
            );
        }
    };
    let mut oriented = oriented;
    if let Some(at) =
        start.and_then(|v| oriented.iter().position(|c| *c == MakeCorner::Existing(v)))
    {
        oriented.rotate_left(at);
    }
    let p = oriented.map(point);
    let scale = (0..4)
        .map(|i| p[i].distance(p[(i + 1) % 4]))
        .fold(0f64, f64::max);
    let area = (p[2] - p[0]).cross(p[3] - p[1]);
    if !scale.is_finite() || scale <= 0. || area.length() <= RELATIVE_TOLERANCE * scale * scale {
        return Err("Make refuses a degenerate quad".into());
    }
    let normal = area.normalize();
    if (0..4).any(|i| {
        let turn = (p[i] - p[(i + 3) % 4]).cross(p[(i + 1) % 4] - p[i]);
        turn.dot(normal) <= -RELATIVE_TOLERANCE * scale * scale
    }) {
        return Err("Make refuses a folded or self-intersecting quad".into());
    }
    let source_face = uses
        .get(&first)
        .map(|u| u[0].0)
        .or_else(|| uses.get(&second).map(|u| u[0].0));
    Ok((oriented, source_face))
}

/// Candidate mesh: new points then one quad appended; existing records
/// untouched; new corner records unassigned.
fn made_mesh(mesh: &Mesh, corners: [MakeCorner; 4], source: Option<usize>) -> Mesh {
    let mut out = mesh.clone();
    let vertices: Vec<u32> = corners
        .iter()
        .map(|c| match *c {
            MakeCorner::Existing(v) => v,
            MakeCorner::New(p) => {
                out.vertices.push(p);
                out.vertices.len() as u32 - 1
            }
        })
        .collect();
    let template = source.map(|f| mesh.faces[f].clone());
    out.faces.push(Face {
        vertices,
        uv: template
            .as_ref()
            .and_then(|f| f.uv.as_ref())
            .map(|_| vec![glam::DVec2::ZERO; 4]),
        material: template
            .as_ref()
            .map(|f| f.material.clone())
            .unwrap_or_default(),
        smooth: template.as_ref().is_some_and(|f| f.smooth),
        part: template.as_ref().and_then(|f| f.part.clone()),
        material_label: template.as_ref().and_then(|f| f.material_label.clone()),
    });
    for map in out.uv_maps.values_mut() {
        map.push(vec![None; 4]);
    }
    let attributes = &mut out.attributes;
    for map in attributes.uv_records.values_mut() {
        map.push(vec![None; 4]);
    }
    for map in attributes.corner_weights.values_mut() {
        map.push(vec![None; 4]);
    }
    for map in attributes.corner_colors.values_mut() {
        map.push(vec![None; 4]);
    }
    for map in attributes.corner_normals.values_mut() {
        map.push(vec![None; 4]);
    }
    for map in attributes.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            map.corners.push(vec![None; 4]);
        }
    }
    if !attributes.smoothing_labels.is_empty() {
        let label = source.and_then(|f| attributes.smoothing_labels[f].clone());
        attributes.smoothing_labels.push(label);
    }
    if !attributes.representations.is_empty() {
        attributes.representations.push(PolygonRepresentation::Face);
    }
    out
}

impl MeshIdentity {
    /// MK-C03: every existing point, face, corner and edge keeps its identity;
    /// a new point, the appended face, its corners and any new edges get fresh
    /// identities. All counts are checked before the first ID is taken; taken
    /// IDs are never returned or reused.
    fn with_made_face(&self, source: &Mesh, output: &Mesh, next: &mut u64) -> Result<Self> {
        self.validate(source)?;
        let face = output.faces.last().ok_or("Missing Make face")?;
        let added = output.vertices.len().checked_sub(source.vertices.len());
        if added.is_none_or(|n| n > 1)
            || output.vertices[..source.vertices.len()] != source.vertices[..]
            || output.faces.len() != source.faces.len() + 1
            || output.faces[..source.faces.len()] != source.faces[..]
        {
            return Err("Make identity topology mismatch".into());
        }
        let added = added.unwrap_or(0);
        let mut vertex_ids = self.vertices.clone();
        vertex_ids.extend((0..added).map(|_| VertexId(u64::MAX)));
        let new_edges = output
            .edges()
            .into_iter()
            .filter(|&(a, b)| {
                let (a, b) = (vertex_ids[a as usize], vertex_ids[b as usize]);
                a.0 == u64::MAX
                    || b.0 == u64::MAX
                    || !self.edges.contains_key(&(a.min(b), a.max(b)))
            })
            .count();
        let needed = (added + 1 + face.vertices.len() + new_edges) as u64;
        if next.checked_add(needed).is_none() {
            return Err("Session identity budget exhausted; Make reserved nothing".into());
        }
        let mut take = || {
            let id = *next;
            *next = next
                .checked_add(1)
                .ok_or("Session identity budget exhausted")?;
            Ok::<u64, String>(id)
        };
        let mut ids = self.clone();
        for _ in 0..added {
            ids.vertices.push(VertexId(take()?));
        }
        let id = FaceId(take()?);
        ids.faces.push(id);
        ids.logical_faces.push(id);
        ids.corners.push(
            face.vertices
                .iter()
                .map(|_| take().map(CornerId))
                .collect::<Result<_>>()?,
        );
        for (a, b) in output.edges() {
            let (a, b) = (ids.vertices[a as usize], ids.vertices[b as usize]);
            let k = if a < b { (a, b) } else { (b, a) };
            if let std::collections::btree_map::Entry::Vacant(entry) = ids.edges.entry(k) {
                entry.insert(EdgeId(take()?));
            }
        }
        ids.layout.push(face.vertices.clone());
        ids.validate(output)?;
        Ok(ids)
    }
}

impl Editor {
    /// Read-only resolution shared by the command and any UI affordance.
    pub fn make_target(&self) -> Result<MakePlan> {
        if self.i2_active() || self.session.pen_draft.is_some() {
            return Err("Finish or Cancel the active tool before Make".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Make".into());
        }
        if self.scene.selection.mode != Mode::Edge {
            return Err("Make requires Edge mode with two selected edges".into());
        }
        self.scene.validate()?;
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh for Make")?;
        let (corners, source_face) = make_plan(&item.mesh, &self.scene.selection.edges)?;
        Ok(MakePlan {
            item: item.id,
            corners,
            source_face,
        })
    }
    /// One atomic "Make" edit. Selection (mode, ordered and remembered
    /// components, item) is unchanged; the new face is not selected.
    pub(crate) fn make_polygon(&mut self, kind: crate::MakeType, close: bool) -> Result<()> {
        if self.session.has_active_task()
            || self.selection_start.is_some()
            || self.session.pen_draft.is_some()
        {
            return Err("Drop or Cancel the active tool before Make Polygon".into());
        }
        // make-split-2026-11-05: in Vertex mode, Make builds one polygon from
        // the ordered points.
        if self.scene.selection.mode == crate::Mode::Vertex {
            let points = self.scene.selection.vertices.clone();
            return self.make_polygon_from_vertices_as(points, kind, close);
        }
        // A closed loop of selected edges is filled with one polygon wound
        // like its neighbours (rule 7).
        if self.scene.selection.mode == crate::Mode::Edge
            && let Some(points) = edge_loop(&self.scene.selection.edges)
        {
            return self.make_polygon_from_vertices_as(points, kind, close);
        }
        let plan = self.make_target()?;
        let item = self.scene.active().ok_or("Missing Make target")?;
        // make-edges-2026-11-06 A1/A2: two bare line edges, or border edges
        // of separate pieces, make nothing. Which pairs qualify is open
        // (Q-ME-01); other pairs keep the two-edge quad.
        if !edges_on_one_surface_piece(&item.mesh, &self.scene.selection.edges) {
            return Err(
                "Make: these edges aren't on one connected surface; nothing was made".into(),
            );
        }
        let mesh = made_mesh(&item.mesh, plan.corners, plan.source_face);
        // Validate the whole candidate Scene first: predictable failures such
        // as the mesh face budget refuse before any identity is reserved.
        let mut candidate = self.scene.clone();
        candidate.active_mut().ok_or("Missing Make target")?.mesh = mesh.clone().into();
        candidate.validate()?;
        let next = self.identities.with_ids(|next| {
            self.identities
                .current
                .meshes
                .get(&plan.item)
                .ok_or("Missing mesh identity owner")?
                .with_made_face(&item.mesh, &mesh, next)
        })?;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.identities
            .current
            .meshes
            .insert(plan.item, next.into());
        if let Err(error) = self.identities.observe(&self.scene) {
            // The surrounding staged dispatch discards this editor on Err.
            self.scene = before;
            return Err(error);
        }
        self.record("Make".into(), before);
        Ok(())
    }
}

/// Whether every edge lies on a surface polygon and all of them are on one
/// connected piece of surface (pieces joined through shared points).
fn edges_on_one_surface_piece(mesh: &Mesh, edges: &[(u32, u32)]) -> bool {
    let mut parent: Vec<usize> = (0..mesh.vertices.len()).collect();
    fn root(parent: &mut [usize], mut v: usize) -> usize {
        while parent[v] != v {
            parent[v] = parent[parent[v]];
            v = parent[v];
        }
        v
    }
    let mut on_surface = std::collections::BTreeSet::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(f) {
            continue;
        }
        let n = face.vertices.len();
        for i in 0..n {
            let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
            on_surface.insert((a.min(b), a.max(b)));
            let (ra, rb) = (root(&mut parent, a as usize), root(&mut parent, b as usize));
            parent[ra] = rb;
        }
    }
    let mut pieces = std::collections::BTreeSet::new();
    for &(a, b) in edges {
        if !on_surface.contains(&(a.min(b), a.max(b))) {
            return false;
        }
        pieces.insert(root(&mut parent, a as usize));
    }
    pieces.len() <= 1
}

/// The points of `edges` in loop order when they form one closed loop of
/// three or more edges.
fn edge_loop(edges: &[(u32, u32)]) -> Option<Vec<u32>> {
    if edges.len() < 3 {
        return None;
    }
    let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in edges {
        next.entry(a).or_default().push(b);
        next.entry(b).or_default().push(a);
    }
    if next.len() != edges.len() || next.values().any(|n| n.len() != 2) {
        return None;
    }
    let start = edges[0].0;
    let mut out = vec![start];
    let (mut previous, mut at) = (start, edges[0].1);
    while at != start {
        out.push(at);
        let n = &next[&at];
        let step = if n[0] == previous { n[1] } else { n[0] };
        previous = at;
        at = step;
        if out.len() > edges.len() {
            return None;
        }
    }
    (out.len() == edges.len()).then_some(out)
}
