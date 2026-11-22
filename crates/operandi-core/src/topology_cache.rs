//! Per-thread memo of topology-only results that every drag frame used to
//! recompute: the sorted edge list and the edge-use exceptions that
//! `Mesh::validate` compares. A drag that only moves points keeps the same
//! polygons, so these become a lookup instead of a sort of every edge.
//!
//! Entries are keyed on everything the results depend on (each polygon's
//! point list and whether it is a surface, closed or open line), so a hit
//! always returns exactly what recomputing would. Point positions are not
//! part of the key because neither result reads them.
use crate::{Mesh, PolygonRepresentation};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub(crate) type EdgeUses = BTreeMap<String, Vec<[u32; 3]>>;
/// How many polygon sides use each edge, keyed (low point, high point).
pub type EdgeCounts = crate::fxhash::FxHashMap<(u32, u32), u32>;
/// The surface polygons on each edge, keyed (low point, high point), in
/// polygon order; a polygon using the edge twice is listed twice. Lines
/// are left out.
pub type EdgeFaces = BTreeMap<(u32, u32), Vec<usize>>;

/// The polygons using each point, packed: point `v`'s polygons are
/// `faces[offsets[v]..offsets[v + 1]]`, in polygon order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointFaces {
    offsets: Vec<u32>,
    faces: Vec<u32>,
}
impl PointFaces {
    pub fn new(mesh: &Mesh) -> Self {
        let mut offsets = vec![0u32; mesh.vertices.len() + 1];
        for f in &mesh.faces {
            for &v in &f.vertices {
                if let Some(o) = offsets.get_mut(v as usize + 1) {
                    *o += 1;
                }
            }
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut next = offsets.clone();
        let mut faces = vec![0u32; *offsets.last().unwrap_or(&0) as usize];
        for (fi, f) in mesh.faces.iter().enumerate() {
            for &v in &f.vertices {
                if let Some(n) = next.get_mut(v as usize) {
                    faces[*n as usize] = fi as u32;
                    *n += 1;
                }
            }
        }
        Self { offsets, faces }
    }
    /// The polygons using point `v` (a polygon using it twice is listed
    /// twice); empty for an unknown point.
    pub fn of(&self, v: u32) -> &[u32] {
        let v = v as usize;
        match (self.offsets.get(v), self.offsets.get(v + 1)) {
            (Some(&a), Some(&b)) => &self.faces[a as usize..b as usize],
            _ => &[],
        }
    }
}

/// For each polygon side (point `i` to point `i + 1`), the one other
/// polygon side running the opposite way between the same two points, when
/// there is exactly one such side and no other side uses that edge: the
/// two polygons share the edge the ordinary way. Lines have no sides here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SideMates {
    /// Where each polygon's sides start in `mates`.
    starts: Vec<u32>,
    /// (polygon, side) of the mate, or `u32::MAX` polygons for none.
    mates: Vec<(u32, u32)>,
}
impl SideMates {
    pub fn new(mesh: &Mesh) -> Self {
        let around = mesh.point_faces();
        let mut starts = Vec::with_capacity(mesh.faces.len() + 1);
        let mut mates = Vec::new();
        let none = (u32::MAX, u32::MAX);
        for (f, face) in mesh.faces.iter().enumerate() {
            starts.push(mates.len() as u32);
            let v = &face.vertices;
            let n = v.len();
            let line = mesh.is_polyline(f);
            for i in 0..n {
                if line {
                    mates.push(none);
                    continue;
                }
                let (a, b) = (v[i], v[(i + 1) % n]);
                // Every other side between a and b, either way round.
                let mut found = none;
                let mut others = 0;
                let mut checked: Option<u32> = None;
                for &g in around.of(a) {
                    // A polygon using `a` twice is listed twice.
                    if checked == Some(g) {
                        continue;
                    }
                    checked = Some(g);
                    if mesh.is_polyline(g as usize) {
                        continue;
                    }
                    let w = &mesh.faces[g as usize].vertices;
                    let m = w.len();
                    for j in 0..m {
                        if (g as usize, j) == (f, i) {
                            continue;
                        }
                        let (c, d) = (w[j], w[(j + 1) % m]);
                        if (c, d) == (b, a) {
                            others += 1;
                            found = (g, j as u32);
                        } else if (c, d) == (a, b) {
                            others += 2;
                        }
                    }
                }
                mates.push(if others == 1 { found } else { none });
            }
        }
        starts.push(mates.len() as u32);
        Self { starts, mates }
    }
    /// The mate of polygon `f`'s side `i`.
    pub fn of(&self, f: usize, i: usize) -> Option<(usize, usize)> {
        let at = *self.starts.get(f)? as usize + i;
        if at >= *self.starts.get(f + 1)? as usize {
            return None;
        }
        let (g, j) = self.mates[at];
        (g != u32::MAX).then_some((g as usize, j as usize))
    }
}

/// Below this many polygon corners recomputing is cheaper than keying.
const MIN_CORNERS: usize = 1024;
/// A drag alternates between its base and preview meshes; a few entries
/// keep both, plus the subdivided cage, without growing.
const ENTRIES: usize = 4;

struct Entry {
    /// The polygon list this entry was last matched against: the same
    /// allocation has the same polygons, so a drag frame (whose mesh copy
    /// shares its polygons) matches without comparing them.
    faces: crate::shared::WeakShared<Vec<crate::Face>>,
    /// The attributes last matched: the same allocation has the same
    /// polygon kinds, so a subdivided mesh (whose kinds list every polygon)
    /// matches without reading them.
    attributes: crate::shared::WeakShared<crate::MeshAttributes>,
    /// No polygon is a line (`kinds` all zero).
    plain: bool,
    corners: Vec<u32>,
    sizes: Vec<u32>,
    kinds: Vec<u8>,
    edges: Option<Rc<Vec<(u32, u32)>>>,
    edge_uses: Option<Rc<EdgeUses>>,
    edge_counts: Option<Rc<EdgeCounts>>,
    point_faces: Option<Rc<PointFaces>>,
    side_mates: Option<Rc<SideMates>>,
    edge_faces: Option<Rc<EdgeFaces>>,
}

fn kind(mesh: &Mesh, face: usize) -> u8 {
    match mesh.attributes.representations.get(face) {
        Some(PolygonRepresentation::Polyline { closed: true, .. }) => 1,
        Some(PolygonRepresentation::Polyline { closed: false, .. }) => 2,
        _ => 0,
    }
}

impl Entry {
    fn new(mesh: &Mesh) -> Self {
        let kinds: Vec<u8> = (0..mesh.faces.len()).map(|f| kind(mesh, f)).collect();
        Self {
            faces: mesh.faces.downgrade(),
            attributes: mesh.attributes.downgrade(),
            plain: kinds.iter().all(|&k| k == 0),
            corners: mesh
                .faces
                .iter()
                .flat_map(|f| f.vertices.iter().copied())
                .collect(),
            sizes: mesh.faces.iter().map(|f| f.vertices.len() as u32).collect(),
            kinds,
            edges: None,
            edge_uses: None,
            edge_counts: None,
            point_faces: None,
            side_mates: None,
            edge_faces: None,
        }
    }
    /// `matches` without reading the polygons: the same polygon list
    /// allocation, and line kinds that are trivially the same.
    fn same(&self, mesh: &Mesh) -> bool {
        self.faces.is(&mesh.faces)
            && (self.attributes.is(&mesh.attributes)
                || if mesh.attributes.representations.is_empty() {
                    self.plain
                } else {
                    self.kinds
                        .iter()
                        .enumerate()
                        .all(|(f, &k)| k == kind(mesh, f))
                })
    }
    fn matches(&self, mesh: &Mesh) -> bool {
        self.sizes.len() == mesh.faces.len()
            && self
                .sizes
                .iter()
                .zip(&mesh.faces)
                .all(|(&n, f)| n as usize == f.vertices.len())
            && self
                .corners
                .iter()
                .eq(mesh.faces.iter().flat_map(|f| f.vertices.iter()))
            && self
                .kinds
                .iter()
                .enumerate()
                .all(|(f, &k)| k == kind(mesh, f))
    }
}

thread_local! {
    static CACHE: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
    static COMPUTED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many topology results this thread has computed on meshes large
/// enough to be remembered (misses, not lookups). Speed guards check that a
/// drag stops computing them after its first frames.
pub fn computed() -> u64 {
    COMPUTED.get()
}

/// Runs `compute` unless an entry for this mesh's topology already holds
/// the result; `slot` picks which result.
fn memo<T>(
    mesh: &Mesh,
    slot: fn(&mut Entry) -> &mut Option<Rc<T>>,
    compute: impl FnOnce() -> T,
) -> Rc<T> {
    let quick = CACHE.with_borrow(|cache| cache.iter().position(|e| e.same(mesh)));
    if quick.is_none() {
        let corners: usize = mesh.faces.iter().map(|f| f.vertices.len()).sum();
        if corners < MIN_CORNERS {
            return Rc::new(compute());
        }
    }
    let hit = CACHE.with_borrow_mut(|cache| {
        let at = quick.or_else(|| cache.iter().position(|e| e.matches(mesh)))?;
        cache[at].faces = mesh.faces.downgrade();
        cache[at].attributes = mesh.attributes.downgrade();
        // Most recent first, so eviction drops the least recently used.
        let entry = cache.remove(at);
        cache.insert(0, entry);
        Some(slot(&mut cache[0]).clone())
    });
    if let Some(Some(value)) = hit {
        return value;
    }
    COMPUTED.set(COMPUTED.get() + 1);
    let value = Rc::new(compute());
    CACHE.with_borrow_mut(|cache| {
        if hit.is_none() {
            cache.insert(0, Entry::new(mesh));
            cache.truncate(ENTRIES);
        }
        *slot(&mut cache[0]) = Some(value.clone());
    });
    value
}

pub(crate) fn edges(mesh: &Mesh, compute: impl FnOnce() -> Vec<(u32, u32)>) -> Rc<Vec<(u32, u32)>> {
    memo(mesh, |e| &mut e.edges, compute)
}

pub(crate) fn edge_uses(mesh: &Mesh, compute: impl FnOnce() -> EdgeUses) -> Rc<EdgeUses> {
    memo(mesh, |e| &mut e.edge_uses, compute)
}

pub(crate) fn edge_counts(mesh: &Mesh, compute: impl FnOnce() -> EdgeCounts) -> Rc<EdgeCounts> {
    memo(mesh, |e| &mut e.edge_counts, compute)
}

pub(crate) fn side_mates(mesh: &Mesh, compute: impl FnOnce() -> SideMates) -> Rc<SideMates> {
    memo(mesh, |e| &mut e.side_mates, compute)
}

pub(crate) fn edge_faces(mesh: &Mesh, compute: impl FnOnce() -> EdgeFaces) -> Rc<EdgeFaces> {
    memo(mesh, |e| &mut e.edge_faces, compute)
}

pub(crate) fn point_faces(mesh: &Mesh, compute: impl FnOnce() -> PointFaces) -> Rc<PointFaces> {
    memo(mesh, |e| &mut e.point_faces, compute)
}
