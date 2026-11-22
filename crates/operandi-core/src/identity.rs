//! Session identities and immutable read adapters, separate from positional native
//! files. Only explicit construction/permutation maps preserve survivors. Other
//! mesh changes conservatively invalidate that mesh's handles. No coordinate match.
use crate::{EditCorrespondence, Mesh, Result, Scene};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
macro_rules! id {
    ($($t:ident),+) => {$(
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
        pub struct $t(pub u64);
    )+};
}
id!(VertexId, FaceId, CornerId, EdgeId);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct IdentityScope {
    pub process: u32,
    pub epoch: u64,
    pub item: u64,
    pub generation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MeshIdentity {
    pub scope: IdentityScope,
    pub vertices: Vec<VertexId>,
    pub faces: Vec<FaceId>,
    pub corners: Vec<Vec<CornerId>>,
    /// Sorted by stable endpoint identities, independent of dense edge order.
    #[serde(serialize_with = "serialize_edges")]
    pub edges: BTreeMap<(VertexId, VertexId), EdgeId>,
    pub logical_faces: Vec<FaceId>,
    pub layout: Vec<Vec<u32>>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct IdentityScene {
    pub meshes: BTreeMap<u64, crate::Shared<MeshIdentity>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct IdentityTicket {
    pub process: u32,
    pub epoch: u64,
    pub publication: u64,
    pub request: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FaceHandle {
    pub scope: IdentityScope,
    pub face: FaceId,
}
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IdentityRuntime {
    pub current: IdentityScene,
    pub publication: u64,
    pub request: u64,
    pub epoch: u64,
    allocation: AllocationBank,
    observed: Option<Scene>,
    pub(crate) record_before: Option<IdentityScene>,
}
/// One allocation owner per open document. Editor clones are private staging
/// copies of that owner, not new documents. Authored/history snapshots contain
/// IdentityScene only; allocation marks deliberately are never snapshotted.
#[derive(Clone, Debug)]
struct AllocationBank(Arc<Mutex<AllocationMarks>>);
#[derive(Debug)]
struct AllocationMarks {
    next: u64,
    generation: u64,
}
impl Default for AllocationBank {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(AllocationMarks {
            next: 1,
            generation: 0,
        })))
    }
}
impl PartialEq for AllocationBank {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
/// Opaque ownership token for the open document: equal only for the same
/// allocation owner, so staged Editor clones match and a newly opened,
/// recovered or replaced document does not, even with equal scene values.
/// Holding it keeps that owner alive, so its address is never reused.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentToken(AllocationBank);
impl IdentityRuntime {
    pub fn document_token(&self) -> DocumentToken {
        DocumentToken(self.allocation.clone())
    }
}
fn take(next: &mut u64) -> Result<u64> {
    let id = *next;
    *next = next
        .checked_add(1)
        .ok_or("Session identity budget exhausted")?;
    Ok(id)
}
fn key(a: VertexId, b: VertexId) -> (VertexId, VertexId) {
    if a < b { (a, b) } else { (b, a) }
}
impl MeshIdentity {
    /// Exactly what `bootstrap` returns for a mesh with this identity's
    /// layout, when this identity is itself an unedited bootstrap (one
    /// contiguous block of IDs in bootstrap order): the same structure with
    /// every ID moved to the new block. Lets a preview that only moves
    /// points (a drag) skip re-deriving and re-checking edges each frame.
    /// `None` when the layout differs or the IDs aren't such a block.
    fn rebootstrap(&self, mesh: &Mesh, scope: IdentityScope, next: &mut u64) -> Option<Self> {
        if self.layout.len() != mesh.faces.len()
            || self.vertices.len() != mesh.vertices.len()
            || self.logical_faces != self.faces
            || self
                .layout
                .iter()
                .zip(&mesh.faces)
                .any(|(l, f)| *l != f.vertices)
        {
            return None;
        }
        let first = self
            .vertices
            .first()
            .map(|v| v.0)
            .or(self.faces.first().map(|f| f.0))?;
        let mut expect = first;
        let mut follows = |id: u64| {
            let ok = id == expect;
            expect += 1;
            ok
        };
        if !self.vertices.iter().all(|v| follows(v.0))
            || !self.faces.iter().all(|f| follows(f.0))
            || !self.corners.iter().flatten().all(|c| follows(c.0))
        {
            return None;
        }
        // Bootstrap numbers edges in `Mesh::edges` order (by point index);
        // check that against this identity's edge IDs.
        // Point IDs are the block's start, so a point's index is its offset.
        let index = |v: VertexId| (v.0 - first) as u32;
        let mut by_points: Vec<((u32, u32), u64)> = self
            .edges
            .iter()
            .map(|(&(a, b), e)| {
                let (a, b) = (index(a), index(b));
                ((a.min(b), a.max(b)), e.0)
            })
            .collect();
        by_points.sort_unstable();
        if !by_points.iter().all(|&(_, e)| follows(e)) {
            return None;
        }
        // The same polygons can have different edges (a polygon turned into
        // an open line drops its closing span): the edge set must match too.
        if !by_points.iter().map(|&(k, _)| k).eq(mesh.edges()) {
            return None;
        }
        let count = expect - first;
        let start = *next;
        *next = next.checked_add(count)?;
        let shift = |id: u64| id - first + start;
        Some(Self {
            scope,
            vertices: self.vertices.iter().map(|v| VertexId(shift(v.0))).collect(),
            faces: self.faces.iter().map(|f| FaceId(shift(f.0))).collect(),
            logical_faces: self.faces.iter().map(|f| FaceId(shift(f.0))).collect(),
            corners: self
                .corners
                .iter()
                .map(|cs| cs.iter().map(|c| CornerId(shift(c.0))).collect())
                .collect(),
            // A common shift keeps the key order, so the map rebuilds in
            // order.
            edges: self
                .edges
                .iter()
                .map(|(&(a, b), e)| {
                    (
                        (VertexId(shift(a.0)), VertexId(shift(b.0))),
                        EdgeId(shift(e.0)),
                    )
                })
                .collect(),
            layout: self.layout.clone(),
        })
    }
    pub fn bootstrap(mesh: &Mesh, scope: IdentityScope, next: &mut u64) -> Result<Self> {
        mesh.validate()?;
        let vertices = mesh
            .vertices
            .iter()
            .map(|_| take(next).map(VertexId))
            .collect::<Result<Vec<_>>>()?;
        let faces = mesh
            .faces
            .iter()
            .map(|_| take(next).map(FaceId))
            .collect::<Result<Vec<_>>>()?;
        let corners = mesh
            .faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|_| take(next).map(CornerId))
                    .collect()
            })
            .collect::<Result<_>>()?;
        let mut edges = BTreeMap::new();
        for (a, b) in mesh.edges() {
            edges.insert(
                key(vertices[a as usize], vertices[b as usize]),
                EdgeId(take(next)?),
            );
        }
        let ids = Self {
            scope,
            vertices,
            logical_faces: faces.clone(),
            faces,
            corners,
            edges,
            layout: mesh.faces.iter().map(|f| f.vertices.clone()).collect(),
        };
        ids.validate(mesh)?;
        Ok(ids)
    }
    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        mesh.validate()?;
        self.validate_layout(mesh)
    }
    /// The identity-to-mesh correspondence part of `validate`, assuming the
    /// mesh itself already passed `Mesh::validate`.
    fn validate_layout(&self, mesh: &Mesh) -> Result<()> {
        if self.vertices.len() != mesh.vertices.len()
            || self.faces.len() != mesh.faces.len()
            || self.corners.len() != mesh.faces.len()
            || !self
                .layout
                .iter()
                .eq(mesh.faces.iter().map(|f| &f.vertices))
            || self
                .corners
                .iter()
                .zip(&mesh.faces)
                .any(|(cs, f)| cs.len() != f.vertices.len())
        {
            return Err("Identity layout mismatch".into());
        }
        let mut all: Vec<_> = self
            .vertices
            .iter()
            .map(|x| x.0)
            .chain(self.faces.iter().map(|x| x.0))
            .chain(self.corners.iter().flatten().map(|x| x.0))
            .chain(self.edges.values().map(|x| x.0))
            .collect();
        // Sorted and compared neighbour to neighbour: the same uniqueness
        // check as a set, without building one (runs every drag frame).
        all.sort_unstable();
        if all.first() == Some(&0)
            || all.windows(2).any(|w| w[0] == w[1])
            || self.logical_faces.len() != self.faces.len()
            || self.logical_faces.iter().copied().collect::<BTreeSet<_>>()
                != self.faces.iter().copied().collect()
        {
            return Err("Duplicate, zero or invalid logical identity".into());
        }
        let mut edges: Vec<_> = mesh
            .edges()
            .into_iter()
            .map(|(a, b)| key(self.vertices[a as usize], self.vertices[b as usize]))
            .collect();
        edges.sort_unstable();
        edges.dedup();
        if !edges.iter().eq(self.edges.keys()) {
            return Err("Identity edge-use coverage mismatch".into());
        }
        Ok(())
    }
    pub fn face_index(&self, handle: FaceHandle) -> Result<u32> {
        if handle.scope != self.scope {
            return Err("Foreign or expired mesh scope".into());
        }
        self.faces
            .iter()
            .position(|&id| id == handle.face)
            .map(|i| i as u32)
            .ok_or_else(|| "Retired face identity".into())
    }
    pub fn transported(
        &self,
        source: &Mesh,
        output: &Mesh,
        plan: &EditCorrespondence,
        next: &mut u64,
    ) -> Result<Self> {
        self.validate(source)?;
        plan.validate(source, output)?;
        let mut ids = self.clone();
        for _ in source.vertices.len()..output.vertices.len() {
            ids.vertices.push(VertexId(take(next)?));
        }
        for _ in source.faces.len()..output.faces.len() {
            ids.faces.push(FaceId(take(next)?));
        }
        // A surviving cap may reverse its winding (legacy positive Thick).
        // Retained corner identity follows the declared source occurrence,
        // independently of the legacy attribute payload's ordinal policy.
        for (f, face) in output.faces[..source.faces.len()].iter().enumerate() {
            ids.corners[f] = face
                .vertices
                .iter()
                .map(|&v| {
                    let original = plan.vertex_sources[v as usize];
                    let corner = source.faces[f]
                        .vertices
                        .iter()
                        .position(|&p| p == original)
                        .ok_or("Retained corner has no declared source occurrence")?;
                    Ok(self.corners[f][corner])
                })
                .collect::<Result<Vec<_>>>()?;
        }
        for face in &output.faces[source.faces.len()..] {
            ids.corners.push(
                face.vertices
                    .iter()
                    .map(|_| take(next).map(CornerId))
                    .collect::<Result<_>>()?,
            );
        }
        ids.edges.clear();
        let wanted: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        for k in wanted {
            ids.edges.insert(
                k,
                match self.edges.get(&k) {
                    Some(id) => *id,
                    None => EdgeId(take(next)?),
                },
            );
        }
        ids.logical_faces
            .extend_from_slice(&ids.faces[source.faces.len()..]);
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        ids.validate(output)?;
        Ok(ids)
    }
    /// Explicit split of source faces into pieces over the existing points
    /// (Triple). Points, retained faces and their corners keep identities. A
    /// split face's identity retires and each piece gets fresh face and corner
    /// identities, because one source corner can feed several pieces. Edges
    /// keep identities by stable endpoints; new diagonals are allocated. In
    /// logical order each retired face is replaced by its pieces.
    pub fn split_faces(
        &self,
        source: &Mesh,
        output: &Mesh,
        origins: &[crate::FaceSplitOrigin],
        next: &mut u64,
    ) -> Result<Self> {
        self.validate(source)?;
        output.validate()?;
        if source.vertices != output.vertices || origins.len() != output.faces.len() {
            return Err("Face split must keep points and declare every output face".into());
        }
        let mut retained = vec![false; source.faces.len()];
        let mut split = vec![false; source.faces.len()];
        for (o, (origin, face)) in origins.iter().zip(&output.faces).enumerate() {
            let src = source
                .faces
                .get(origin.face)
                .ok_or("Face split names a missing source face")?;
            match &origin.corners {
                None => {
                    if o != origin.face || face.vertices != src.vertices || retained[o] {
                        return Err("A retained face must keep its index and corners".into());
                    }
                    retained[o] = true;
                }
                Some(corners) => {
                    if corners.len() != face.vertices.len()
                        || corners
                            .iter()
                            .zip(&face.vertices)
                            .any(|(&c, v)| src.vertices.get(c) != Some(v))
                    {
                        return Err("Split piece corners must name source occurrences".into());
                    }
                    split[origin.face] = true;
                }
            }
        }
        if retained.iter().zip(&split).any(|(r, s)| r == s) {
            return Err("Every source face must be either retained or split".into());
        }
        let mut ids = self.clone();
        ids.faces = Vec::with_capacity(output.faces.len());
        ids.corners = Vec::with_capacity(output.faces.len());
        let mut pieces: BTreeMap<usize, Vec<FaceId>> = BTreeMap::new();
        for (origin, face) in origins.iter().zip(&output.faces) {
            if origin.corners.is_none() {
                ids.faces.push(self.faces[origin.face]);
                ids.corners.push(self.corners[origin.face].clone());
            } else {
                let id = FaceId(take(next)?);
                ids.faces.push(id);
                ids.corners.push(
                    face.vertices
                        .iter()
                        .map(|_| take(next).map(CornerId))
                        .collect::<Result<_>>()?,
                );
                pieces.entry(origin.face).or_default().push(id);
            }
        }
        let index: BTreeMap<FaceId, usize> = self
            .faces
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();
        ids.logical_faces = self
            .logical_faces
            .iter()
            .flat_map(|id| match pieces.get(&index[id]) {
                Some(new) => new.clone(),
                None => vec![*id],
            })
            .collect();
        ids.edges.clear();
        let wanted: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        for k in wanted {
            ids.edges.insert(
                k,
                match self.edges.get(&k) {
                    Some(id) => *id,
                    None => EdgeId(take(next)?),
                },
            );
        }
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        ids.validate(output)?;
        Ok(ids)
    }

    /// Replace two face owners and their corners, preserving point and boundary-edge identities.
    pub(crate) fn spin_faces(
        &self,
        source: &Mesh,
        output: &Mesh,
        faces: [usize; 2],
        next: &mut u64,
    ) -> Result<Self> {
        self.validate(source)?;
        output.validate()?;
        if source.vertices != output.vertices
            || source.faces.len() != output.faces.len()
            || faces[0] == faces[1]
            || faces.iter().any(|&i| i >= source.faces.len())
        {
            return Err("Invalid Spin identity correspondence".into());
        }
        let mut ids = self.clone();
        for i in 0..source.faces.len() {
            if faces.contains(&i) {
                let old = ids.faces[i];
                let fresh = FaceId(take(next)?);
                ids.faces[i] = fresh;
                for logical in &mut ids.logical_faces {
                    if *logical == old {
                        *logical = fresh;
                    }
                }
                ids.corners[i] = output.faces[i]
                    .vertices
                    .iter()
                    .map(|_| take(next).map(CornerId))
                    .collect::<Result<_>>()?;
            } else if source.faces[i].vertices != output.faces[i].vertices {
                return Err("Spin changed an unrelated face".into());
            }
        }
        let wanted: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        ids.edges.retain(|edge, _| wanted.contains(edge));
        for edge in wanted {
            if let std::collections::btree_map::Entry::Vacant(entry) = ids.edges.entry(edge) {
                entry.insert(EdgeId(take(next)?));
            }
        }
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        ids.validate(output)?;
        Ok(ids)
    }

    /// Explicit merge of selected source faces into one polygon over the
    /// existing points (MER-002). Points, retained faces and their corners
    /// keep identities. Every merged source face identity retires and the
    /// merged polygon gets a fresh face identity; each merged corner keeps the
    /// identity of the one source occurrence it was declared from, and the
    /// other source occurrences retire. Edges keep identities by stable
    /// endpoints; removed interior edges retire and no edge is allocated. In
    /// logical order the first retired face is replaced by the merged face.
    pub fn merge_faces(
        &self,
        source: &Mesh,
        output: &Mesh,
        origins: &[crate::FaceMergeOrigin],
        next: &mut u64,
    ) -> Result<Self> {
        use crate::FaceMergeOrigin;
        self.validate(source)?;
        output.validate()?;
        if source.vertices != output.vertices || origins.len() != output.faces.len() {
            return Err("Face merge must keep points and declare every output face".into());
        }
        let mut claimed = vec![false; source.faces.len()];
        let mut merged_sources = BTreeSet::new();
        let mut merged_count = 0;
        for (origin, face) in origins.iter().zip(&output.faces) {
            match origin {
                FaceMergeOrigin::Retained(f) => {
                    let src = source
                        .faces
                        .get(*f)
                        .ok_or("Face merge names a missing source face")?;
                    if src.vertices != face.vertices || std::mem::replace(&mut claimed[*f], true) {
                        return Err("A retained face must keep its corners exactly once".into());
                    }
                }
                FaceMergeOrigin::Merged { sources, corners } => {
                    merged_count += 1;
                    if sources.len() < 2 || corners.len() != face.vertices.len() {
                        return Err("A merged face must declare its sources and corners".into());
                    }
                    for &f in sources {
                        if f >= source.faces.len()
                            || std::mem::replace(&mut claimed[f], true)
                            || !merged_sources.insert(f)
                        {
                            return Err("Merged source faces must be distinct and unclaimed".into());
                        }
                    }
                    let mut used = BTreeSet::new();
                    for (&(f, c), v) in corners.iter().zip(&face.vertices) {
                        if !merged_sources.contains(&f)
                            || source.faces[f].vertices.get(c) != Some(v)
                            || !used.insert((f, c))
                        {
                            return Err(
                                "Merged corners must name distinct source occurrences".into()
                            );
                        }
                    }
                }
            }
        }
        if merged_count != 1 || claimed.iter().any(|c| !c) {
            return Err("Every source face must be either retained or merged once".into());
        }
        let merged_id = FaceId(take(next)?);
        let mut ids = self.clone();
        ids.faces = origins
            .iter()
            .map(|o| match o {
                FaceMergeOrigin::Retained(f) => self.faces[*f],
                FaceMergeOrigin::Merged { .. } => merged_id,
            })
            .collect();
        ids.corners = origins
            .iter()
            .map(|o| match o {
                FaceMergeOrigin::Retained(f) => self.corners[*f].clone(),
                FaceMergeOrigin::Merged { corners, .. } => {
                    corners.iter().map(|&(f, c)| self.corners[f][c]).collect()
                }
            })
            .collect();
        let retired: BTreeSet<FaceId> = merged_sources.iter().map(|&f| self.faces[f]).collect();
        let mut placed = false;
        ids.logical_faces = self
            .logical_faces
            .iter()
            .filter_map(|id| {
                if !retired.contains(id) {
                    Some(*id)
                } else if !std::mem::replace(&mut placed, true) {
                    Some(merged_id)
                } else {
                    None
                }
            })
            .collect();
        let needed: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        if needed.iter().any(|k| !self.edges.contains_key(k)) {
            return Err("Face merge cannot introduce edges".into());
        }
        ids.edges.retain(|k, _| needed.contains(k));
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        ids.validate(output)?;
        Ok(ids)
    }

    /// ALN-B01: preserve vertex/face/edge identities while moving each listed
    /// face's corner identities with the anchored reversal (`[a,b,c,d]` -> `[a,d,c,b]`).
    pub(crate) fn anchored_reversed_faces(
        &self,
        source: &Mesh,
        output: &Mesh,
        faces: &BTreeSet<usize>,
    ) -> Result<Self> {
        self.validate(source)?;
        if source.vertices != output.vertices
            || source.faces.len() != output.faces.len()
            || faces.iter().any(|&f| f >= source.faces.len())
        {
            return Err("Align identity topology mismatch".into());
        }
        let mut next = self.clone();
        for (fi, f) in source.faces.iter().enumerate() {
            let mut expected = f.vertices.clone();
            if faces.contains(&fi) {
                expected[1..].reverse();
                next.corners[fi][1..].reverse();
            }
            if output.faces[fi].vertices != expected {
                return Err("Align identity winding mismatch".into());
            }
            next.layout[fi] = expected;
        }
        next.validate(output)?;
        Ok(next)
    }
    /// Explicit Polygon Reduction correspondence (RED-003). Surviving points
    /// keep their identities and retired points drop out. An output face that
    /// is an unchanged source face keeps its face and corner identities; every
    /// reconnected face gets fresh ones. Edges keep identities by stable
    /// endpoints and new connections are allocated. Logical order keeps the
    /// retained faces in source logical order, then new faces in output order.
    pub fn reduced(
        &self,
        source: &Mesh,
        output: &Mesh,
        plan: &crate::ReductionCorrespondence,
        next: &mut u64,
    ) -> Result<Self> {
        self.validate(source)?;
        output.validate()?;
        plan.validate(source, output)?;
        let mut ids = self.clone();
        ids.vertices = plan
            .vertex_sources
            .iter()
            .map(|&v| self.vertices[v as usize])
            .collect();
        ids.faces = Vec::with_capacity(output.faces.len());
        ids.corners = Vec::with_capacity(output.faces.len());
        let mut fresh = Vec::new();
        for (face, origin) in output.faces.iter().zip(&plan.face_sources) {
            match origin {
                Some(f) => {
                    ids.faces.push(self.faces[*f as usize]);
                    ids.corners.push(self.corners[*f as usize].clone());
                }
                None => {
                    let id = FaceId(take(next)?);
                    ids.faces.push(id);
                    fresh.push(id);
                    ids.corners.push(
                        face.vertices
                            .iter()
                            .map(|_| take(next).map(CornerId))
                            .collect::<Result<_>>()?,
                    );
                }
            }
        }
        let kept: BTreeSet<FaceId> = ids.faces.iter().copied().collect();
        ids.logical_faces = self
            .logical_faces
            .iter()
            .copied()
            .filter(|f| kept.contains(f))
            .chain(fresh)
            .collect();
        ids.edges.clear();
        let wanted: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        for k in wanted {
            ids.edges.insert(
                k,
                match self.edges.get(&k) {
                    Some(id) => *id,
                    None => EdgeId(take(next)?),
                },
            );
        }
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        ids.validate(output)?;
        Ok(ids)
    }

    /// Explicit Polygon Collapse correspondence (COL-B01). Surviving points
    /// keep their identities and the new point gets a fresh one. Every
    /// repaired face is the same face: it keeps its face identity and each
    /// output corner keeps the identity of its declared source occurrence.
    /// The collapsed quad's face and corner identities retire. Edges keep
    /// identities by stable endpoints; edges to the new point are allocated.
    pub fn collapsed(
        &self,
        source: &Mesh,
        output: &Mesh,
        plan: &crate::CollapseCorrespondence,
        next: &mut u64,
    ) -> Result<Self> {
        self.validate(source)?;
        output.validate()?;
        plan.validate(source, output)?;
        let mut ids = self.clone();
        ids.vertices = plan
            .vertex_sources
            .iter()
            .map(|v| match v {
                Some(v) => Ok(self.vertices[*v as usize]),
                None => take(next).map(VertexId),
            })
            .collect::<Result<_>>()?;
        ids.faces = plan
            .face_sources
            .iter()
            .map(|&f| self.faces[f as usize])
            .collect();
        ids.corners = plan
            .face_sources
            .iter()
            .zip(&plan.corner_sources)
            .map(|(&f, cs)| cs.iter().map(|&c| self.corners[f as usize][c]).collect())
            .collect();
        let kept: BTreeSet<FaceId> = ids.faces.iter().copied().collect();
        ids.logical_faces.retain(|f| kept.contains(f));
        ids.edges.clear();
        let wanted: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        for k in wanted {
            ids.edges.insert(
                k,
                match self.edges.get(&k) {
                    Some(id) => *id,
                    None => EdgeId(take(next)?),
                },
            );
        }
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        ids.validate(output)?;
        Ok(ids)
    }

    /// Explicit source-face permutation/deletion only. Caller separately remaps
    /// payloads/selection; validation refuses stale or guessed geometry maps.
    pub fn remap_faces(&self, source: &Mesh, output: &Mesh, permutation: &[usize]) -> Result<Self> {
        self.validate(source)?;
        output.validate()?;
        if source.vertices != output.vertices
            || permutation.len() != output.faces.len()
            || permutation.iter().copied().collect::<BTreeSet<_>>().len() != permutation.len()
            || permutation
                .iter()
                .zip(&output.faces)
                .any(|(&i, f)| source.faces.get(i) != Some(f))
        {
            return Err("Face remap must declare exact surviving source faces".into());
        }
        let mut ids = self.clone();
        ids.faces = permutation.iter().map(|&i| self.faces[i]).collect();
        ids.corners = permutation
            .iter()
            .map(|&i| self.corners[i].clone())
            .collect();
        ids.logical_faces.retain(|f| ids.faces.contains(f));
        ids.layout = output.faces.iter().map(|f| f.vertices.clone()).collect();
        let needed: BTreeSet<_> = output
            .edges()
            .into_iter()
            .map(|(a, b)| key(ids.vertices[a as usize], ids.vertices[b as usize]))
            .collect();
        ids.edges.retain(|k, _| needed.contains(k));
        ids.validate(output)?;
        Ok(ids)
    }
}
impl crate::Shared<MeshIdentity> {
    /// `MeshIdentity::validate`, skipped when these exact shared identity and
    /// mesh allocations already passed it together (see `shared.rs`).
    pub fn validate_shared(&self, mesh: &crate::Shared<Mesh>) -> Result<()> {
        mesh.validate()?;
        // The layout check reads the polygon list, the point count and the
        // polygon kinds, never where points are. A mesh that shares those
        // with the one these identities last passed against (undo or redo
        // of a move, which lands on the other end of the drag) passes too.
        if let Some(previous) = self.checked_partner::<Mesh>()
            && crate::Shared::same(&previous.faces, &mesh.faces)
            && previous.vertices.len() == mesh.vertices.len()
            && (crate::Shared::same(&previous.attributes, &mesh.attributes)
                || previous.attributes.representations == mesh.attributes.representations)
        {
            self.record_checked_against(mesh);
            return Ok(());
        }
        self.checked_against(mesh, MeshIdentity::validate_layout)
    }
}
impl IdentityScene {
    pub fn validate(&self, scene: &Scene) -> Result<()> {
        if self.meshes.len() != scene.items.len() {
            return Err("Identity scene owner coverage mismatch".into());
        }
        for item in &scene.items {
            let ids = self
                .meshes
                .get(&item.id)
                .ok_or("Missing mesh identity owner")?;
            if ids.scope.item != item.id {
                return Err("Wrong identity item scope".into());
            }
            ids.validate_shared(item.mesh.shared())?;
        }
        Ok(())
    }
}
impl IdentityRuntime {
    fn marks(&self) -> Result<(u64, u64)> {
        let m = self
            .allocation
            .0
            .lock()
            .map_err(|_| "Identity allocator unavailable")?;
        Ok((m.next, m.generation))
    }
    pub fn next_id(&self) -> Result<u64> {
        self.marks().map(|m| m.0)
    }
    /// Reserve/burn a range without granting identities or rewinding ownership.
    pub fn reserve_through(&self, end: u64) -> Result<()> {
        let mut m = self
            .allocation
            .0
            .lock()
            .map_err(|_| "Identity allocator unavailable")?;
        if end < m.next {
            return Err("Cannot rewind identity reservations".into());
        }
        m.next = end;
        Ok(())
    }
    pub(crate) fn with_ids<T>(&self, f: impl FnOnce(&mut u64) -> Result<T>) -> Result<T> {
        let mut m = self
            .allocation
            .0
            .lock()
            .map_err(|_| "Identity allocator unavailable")?;
        // Direct mutation under the document lock retains partial reservations
        // even when validation/allocation returns Err. No unsafe delegation.
        f(&mut m.next)
    }
    fn allocate_mesh(
        &self,
        mesh: &crate::Shared<Mesh>,
        item: u64,
    ) -> Result<crate::Shared<MeshIdentity>> {
        let mut m = self
            .allocation
            .0
            .lock()
            .map_err(|_| "Identity allocator unavailable")?;
        m.generation = m
            .generation
            .checked_add(1)
            .ok_or("Mesh generation budget exhausted")?;
        let scope = IdentityScope {
            process: std::process::id(),
            epoch: self.epoch,
            item,
            generation: m.generation,
        };
        // A mesh whose polygons are unchanged from the item's current
        // bootstrap identity (points moved): same result, without re-deriving
        // and re-checking it. The scene's meshes were validated by the caller.
        if let Some(previous) = self.current.meshes.get(&item)
            && let Some(ids) = previous.rebootstrap(mesh, scope, &mut m.next)
        {
            // `rebootstrap` proved the same layout and edges as a previous
            // identity that passed against its mesh, with IDs shifted as one
            // block (still unique and nonzero): equivalent to
            // `validate_layout`, so the next validation skips it.
            let ids = crate::Shared::from(ids);
            ids.record_checked_against(mesh);
            return Ok(ids);
        }
        MeshIdentity::bootstrap(mesh, scope, &mut m.next).map(Into::into)
    }
    pub fn synchronize(&mut self, scene: &Scene) -> Result<()> {
        scene.validate()?;
        if self.epoch == 0 {
            self.epoch = NEXT_EPOCH
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
                .map_err(|_| "Document epoch budget exhausted")?;
        }
        if self.observed.as_ref() == Some(scene) {
            return self.validate_current(scene);
        }
        let changed_existing = self.observed.is_some();
        let mut meshes = BTreeMap::new();
        for item in &scene.items {
            let before = self.observed.as_ref().and_then(|s| s.item(item.id));
            // A mesh whose points only moved keeps its identities: a moved
            // point is the same point, and its polygons, corners and edges
            // are the same components (Jason, 2026-11-03). Topology changes
            // still issue new ones.
            let moved_only = before.is_some_and(|i| positions_only(&i.mesh, &item.mesh));
            let ids = if before.is_some_and(|i| i.mesh == item.mesh) || moved_only {
                let ids = self
                    .current
                    .meshes
                    .get(&item.id)
                    .cloned()
                    .ok_or("Missing unchanged identity")?;
                if moved_only {
                    // These identities passed against the previous mesh,
                    // which has the same polygons and edges as this one:
                    // equivalent to `validate_layout`, so a drag doesn't
                    // re-check every ID each frame.
                    ids.record_checked_against(item.mesh.shared());
                }
                ids
            } else {
                self.allocate_mesh(item.mesh.shared(), item.id)?
            };
            meshes.insert(item.id, ids);
        }
        self.current = IdentityScene { meshes };
        self.observed = Some(scene.clone());
        if changed_existing {
            self.advance()?;
        }
        self.validate_current(scene)
    }
    pub fn validate_current(&self, scene: &Scene) -> Result<()> {
        self.current.validate(scene)?;
        let (next, generation) = self.marks()?;
        for ids in self.current.meshes.values() {
            if ids.scope.process != std::process::id()
                || ids.scope.epoch != self.epoch
                || ids.scope.generation == 0
                || ids.scope.generation > generation
            {
                return Err("Expired identity owner scope".into());
            }
            // Remembered per identity allocation: a drag keeps the same
            // identities every frame, and this read every ID.
            let max = *ids.derived(|ids| {
                ids.vertices
                    .iter()
                    .map(|x| x.0)
                    .chain(ids.faces.iter().map(|x| x.0))
                    .chain(ids.corners.iter().flatten().map(|x| x.0))
                    .chain(ids.edges.values().map(|x| x.0))
                    .max()
                    .unwrap_or(0)
            });
            if next <= max {
                return Err("Identity allocation mark precedes live IDs".into());
            }
        }
        Ok(())
    }
    pub(crate) fn observe(&mut self, scene: &Scene) -> Result<()> {
        self.validate_current(scene)?;
        self.observed = Some(scene.clone());
        Ok(())
    }
    pub(crate) fn invalidate_mesh(&mut self, scene: &Scene, item: u64) -> Result<()> {
        if let Some(target) = scene.item(item) {
            let ids = self.allocate_mesh(target.mesh.shared(), item)?;
            self.current.meshes.insert(item, ids);
            self.observe(scene)?;
        }
        Ok(())
    }
    pub fn allocate_snapshot(&mut self, scene: &Scene) -> Result<IdentityScene> {
        scene.validate()?;
        let mut meshes = BTreeMap::new();
        for item in &scene.items {
            meshes.insert(item.id, self.allocate_mesh(item.mesh.shared(), item.id)?);
        }
        Ok(IdentityScene { meshes })
    }
    pub fn advance(&mut self) -> Result<()> {
        self.publication = self
            .publication
            .checked_add(1)
            .ok_or("Publication budget exhausted")?;
        self.request = self
            .request
            .checked_add(1)
            .ok_or("Request budget exhausted")?;
        Ok(())
    }
    pub fn ticket(&mut self) -> Result<IdentityTicket> {
        self.request = self
            .request
            .checked_add(1)
            .ok_or("Request budget exhausted")?;
        Ok(IdentityTicket {
            process: std::process::id(),
            epoch: self.epoch,
            publication: self.publication,
            request: self.request,
        })
    }
    pub fn check(&self, t: IdentityTicket) -> Result<()> {
        if t.process != std::process::id()
            || t.epoch != self.epoch
            || t.publication != self.publication
            || t.request != self.request
        {
            return Err("Stale document/publication/request ticket".into());
        }
        Ok(())
    }
}
/// Borrowed immutable authored view: effective queries never manufacture records.
pub struct MeshReadView<'a> {
    pub mesh: &'a Mesh,
    pub identities: &'a MeshIdentity,
    pub publication: u64,
}
impl MeshReadView<'_> {
    pub fn authored_uv(
        &self,
        map: &str,
        face: FaceHandle,
        corner: usize,
    ) -> Result<Option<glam::DVec2>> {
        let f = self.identities.face_index(face)? as usize;
        self.mesh
            .uv_maps
            .get(map)
            .and_then(|m| m.get(f))
            .and_then(|c| c.get(corner))
            .copied()
            .ok_or_else(|| "Missing map/corner".into())
    }
}

fn serialize_edges<S: serde::Serializer>(
    edges: &BTreeMap<(VertexId, VertexId), EdgeId>,
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    edges
        .iter()
        .map(|((a, b), id)| (a, b, id))
        .collect::<Vec<_>>()
        .serialize(s)
}

/// True when `b` differs from `a` at most in point positions: the same
/// points, the same polygons with the same corners in the same order, and
/// the same element kinds (an open line has one edge fewer), so every point,
/// polygon, corner and edge identity names the same component.
fn positions_only(a: &Mesh, b: &Mesh) -> bool {
    a.vertices.len() == b.vertices.len()
        && a.faces.len() == b.faces.len()
        && a.attributes.representations == b.attributes.representations
        && (crate::Shared::same(&a.faces, &b.faces)
            || a.faces
                .iter()
                .zip(&b.faces)
                .all(|(x, y)| x.vertices == y.vertices))
}

#[cfg(test)]
#[path = "history_policy_identity_tests.rs"]
mod history_policy_identity_tests;

#[cfg(test)]
mod cleanup_noop_tests {
    use crate::{Command, Editor, Mode};

    #[test]
    fn clean_target_preserves_parked_redo_and_every_allocator_mark() {
        let mut editor = Editor::default();
        editor.dispatch(Command::NewMesh).unwrap();
        editor.scene.selection.mode = Mode::Item;
        editor.dispatch(Command::NewMesh).unwrap();
        editor.dispatch(Command::Undo).unwrap();
        assert!(!editor.redo.is_empty());
        assert_eq!(editor.scene.selection.mode, Mode::Item);
        // Exercise the review's conditional incomplete-checkpoint case as well
        // as the real parked redo produced by the public history commands.
        for incomplete in [false, true] {
            if incomplete {
                let parked = editor
                    .redo
                    .last_mut()
                    .map(std::sync::Arc::make_mut)
                    .unwrap();
                parked.identity_before = None;
                parked.identity_after = None;
            }
            let before = editor.clone();
            let marks = editor.identities.marks().unwrap();
            editor.dispatch(Command::CleanupFloating(None)).unwrap();
            assert!(
                editor == before,
                "scene, history and runtime must remain exact"
            );
            assert_eq!(editor.identities.marks().unwrap(), marks);
        }
    }
}

#[cfg(test)]
mod rebootstrap_tests {
    use super::*;
    use glam::DVec3;

    fn scope(generation: u64) -> IdentityScope {
        IdentityScope {
            process: 1,
            epoch: 1,
            item: 1,
            generation,
        }
    }

    #[test]
    fn moved_points_give_exactly_a_fresh_bootstrap() {
        let mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let mut next = 7;
        let first = MeshIdentity::bootstrap(&mesh, scope(1), &mut next).unwrap();
        let mut moved = mesh.clone();
        moved.vertices[3] += DVec3::new(0.1, 0.2, 0.3);
        let (mut a, mut b) = (next + 5, next + 5);
        let fast = first.rebootstrap(&moved, scope(2), &mut a).unwrap();
        let fresh = MeshIdentity::bootstrap(&moved, scope(2), &mut b).unwrap();
        assert_eq!(fast, fresh);
        assert_eq!(a, b);
        // And again from the fast result.
        let again = fast.rebootstrap(&moved, scope(3), &mut a).unwrap();
        assert_eq!(
            again,
            MeshIdentity::bootstrap(&moved, scope(3), &mut b).unwrap()
        );
    }

    #[test]
    fn changed_polygons_or_edited_ids_take_the_full_path() {
        let mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let mut next = 1;
        let ids = MeshIdentity::bootstrap(&mesh, scope(1), &mut next).unwrap();
        let mut fewer = mesh.clone();
        fewer.faces[0].vertices.reverse();
        assert!(ids.rebootstrap(&fewer, scope(2), &mut next).is_none());
        let mut edited = ids.clone();
        edited.faces.swap(0, 1);
        edited.logical_faces.swap(0, 1);
        assert!(edited.rebootstrap(&mesh, scope(2), &mut next).is_none());
    }
}

#[cfg(test)]
mod rebootstrap_edge_tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn same_polygons_with_different_edges_take_the_full_path() {
        let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let scope = IdentityScope {
            process: 1,
            epoch: 1,
            item: 1,
            generation: 1,
        };
        let mut next = 1;
        let ids = MeshIdentity::bootstrap(&mesh, scope, &mut next).unwrap();
        // Same layout, but polygon 0 as an open line: one span fewer.
        mesh.attributes.representations = vec![crate::PolygonRepresentation::Face; 6];
        mesh.attributes.representations[0] = crate::PolygonRepresentation::Polyline {
            closed: false,
            curve: Default::default(),
        };
        if mesh.edges().len() != ids.edges.len() {
            assert!(ids.rebootstrap(&mesh, scope, &mut next).is_none());
        }
    }
}
