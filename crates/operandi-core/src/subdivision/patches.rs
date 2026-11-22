//! Modo Subdivision (SDV-09 v2) as patches for the graphics card to sample
//! (docs/SUBD_GPU_DESIGN.md).
//!
//! `modo_surface` samples every patch into polygons on the CPU, about a
//! kilobyte per drawn triangle once drawn. Here the surface stays as its
//! controls: one Catmull-Clark step of the cage, then per resulting quad
//! (one per cage corner) a biquadratic Bézier patch whose nine controls are
//! four corner controls (one per point of the step), four edge controls
//! (one per edge of the step) and its centre. Neighbouring patches share
//! corner and edge controls, so sampling them anywhere stays watertight.
//! The viewport samples them on an L × L grid in a vertex shader; the
//! samples equal `modo_surface`'s (tests).
//!
//! Everything is flat arrays indexed by structure instead of hash maps:
//! - step points: cage points `0..points`, then one per cage edge, then
//!   one per cage polygon;
//! - step edges: two halves per cage edge (`2e` from the low end, `2e + 1`
//!   from the high end), then one inner edge per cage corner (from the
//!   corner's edge point to its polygon's point);
//! - patches: one per cage corner, in cage polygon then corner order.
use super::{PolygonRepresentation, effective_kinds};
use crate::Mesh;
use glam::DVec3;
use std::sync::Arc;

/// Elements kept per copy-on-write chunk (`Chunks`).
pub const CHUNK: usize = 1 << 14;

/// A list kept in shared chunks: cloning it is cheap, and changing an
/// element copies only its chunk. A drag changes a few controls of a
/// surface with tens of millions; the copies the viewport and the cache
/// hold share every chunk the drag didn't touch, and the graphics card
/// re-sends only the chunks that differ (`same_chunk`).
#[derive(Clone, Debug, Default)]
pub struct Chunks<T> {
    parts: Vec<Arc<Vec<T>>>,
    len: usize,
}
impl<T: Clone> Chunks<T> {
    pub fn from_vec(all: Vec<T>) -> Self {
        let len = all.len();
        let parts = all.chunks(CHUNK).map(|c| Arc::new(c.to_vec())).collect();
        Self { parts, len }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn set(&mut self, i: usize, value: T) {
        Arc::make_mut(&mut self.parts[i / CHUNK])[i % CHUNK] = value;
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.parts.iter().flat_map(|p| p.iter())
    }
    /// The chunks, `CHUNK` elements each (the last may be shorter).
    pub fn chunks(&self) -> &[Arc<Vec<T>>] {
        &self.parts
    }
    /// Whether chunk `k` is the very same in `self` and `other`.
    pub fn same_chunk(&self, other: &Self, k: usize) -> bool {
        match (self.parts.get(k), other.parts.get(k)) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl<T> std::ops::Index<usize> for Chunks<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        &self.parts[i / CHUNK][i % CHUNK]
    }
}
impl<T: PartialEq> PartialEq for Chunks<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len
            && self
                .parts
                .iter()
                .zip(&other.parts)
                .all(|(a, b)| Arc::ptr_eq(a, b) || a == b)
    }
}

/// A Tab surface ready to sample: the controls and how patches use them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatchNet {
    /// Corner controls (one per step point), then edge controls (one per
    /// step edge), then centres (one per patch).
    pub controls: Chunks<[f32; 3]>,
    /// Per patch: its corner controls in order (cage corner, the corner's
    /// edge point, the polygon point, the previous edge point), then its
    /// side controls (sides 0→1, 1→2, 2→3, 3→0). Its centre is
    /// `centre_base + patch`.
    pub patches: Arc<Vec<[u32; 8]>>,
    pub centre_base: u32,
    /// The cage polygon (mesh numbering) each patch belongs to.
    pub owner: Arc<Vec<u32>>,
    /// Per cage polygon (mesh numbering), its patches; empty for polygons
    /// drawn some other way.
    pub blocks: Arc<Vec<std::ops::Range<u32>>>,
    /// The traced wireframe: each cage edge as two quadratic Béziers end
    /// to end (low corner control, low half's edge control, the edge
    /// point's corner control, high half's edge control, high corner
    /// control), then how many of the edge's polygons it stands for (1, or
    /// 2 for an inner edge: drawn once, blended as two strokes;
    /// `EdgeVertex::uses`).
    pub wire: Arc<Vec<[u32; 6]>>,
    /// The cage edge (mesh point numbers, low first) each wire record lies on.
    pub wire_edges: Arc<Vec<(u32, u32)>>,
    /// Samples per patch side (the display level, L).
    pub level: u8,
    /// Bounds and facing of each run of `fill_per_cluster` patches and of
    /// `wire_per_cluster` wire records, for culling those facing away.
    pub fill_clusters: Chunks<Cluster>,
    pub wire_clusters: Chunks<Cluster>,
    /// Per wire record, the surface's normal on each side of its cage edge
    /// (at the edge's middle, `pack4x8snorm`; zero for no polygon there, w
    /// set for an edge of more than two polygons), for Fade Wireframe on
    /// Small Polygons to keep outline and sharp edges.
    pub wire_sides: Chunks<[u32; 2]>,
    /// Every cage edge of the patches has two polygons: the surface is
    /// closed, so from outside it whatever faces away is hidden.
    pub closed: bool,
    /// The controls' bounds (the surface lies inside them; a drag only
    /// grows them).
    pub bounds: [[f32; 3]; 2],
    /// Which build this net's patch, owner and wire lists come from: a
    /// drag keeps them and changes only controls, clusters and sides.
    pub topology: u64,
}

/// A run of patches (or wire records): a sphere holding their controls
/// and a cone holding every normal they can have (from their controls'
/// differences, so it bounds the true normals). `cos` is the cosine of the
/// cone's half angle; below 0 the run faces every way and is never culled.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cluster {
    pub centre: [f32; 3],
    pub radius: f32,
    pub axis: [f32; 3],
    pub cos: f32,
}

/// Patches per fill cluster: about 192 grid samples, which is also one
/// instance of the viewport's fill draw (subd_gpu.rs).
pub fn fill_per_cluster(level: u8) -> u32 {
    let side = u32::from(level) + 1;
    (192 / (side * side)).max(1)
}

/// Wire records per wire cluster: about 192 stroke corners (2(2L+1) each).
pub fn wire_per_cluster(level: u8) -> u32 {
    (192 / (2 * (2 * u32::from(level) + 1))).max(1)
}

/// The controls' differences along each direction of a patch's 3 × 3 grid.
fn patch_normals(grid: &[[DVec3; 3]; 3], out: &mut Vec<DVec3>) {
    for a in 0..2 {
        for b in 0..3 {
            let du = grid[a + 1][b] - grid[a][b];
            for row in grid {
                for d in 0..2 {
                    let dv = row[d + 1] - row[d];
                    let n = du.cross(dv);
                    let l = n.length();
                    if l > 0.0 {
                        out.push(n / l);
                    }
                }
            }
        }
    }
}

/// The cluster holding `points` and `normals` (unit).
fn cluster(points: &[DVec3], normals: &[DVec3]) -> Cluster {
    let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for &p in points {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let centre = (lo + hi) / 2.0;
    let radius = points
        .iter()
        .map(|p| p.distance(centre))
        .fold(0.0, f64::max);
    let sum: DVec3 = normals.iter().copied().sum();
    let (axis, cos) = if normals.is_empty() || sum.length() < 1e-9 {
        (DVec3::Z, -2.0)
    } else {
        let axis = sum.normalize();
        let cos = normals.iter().map(|n| n.dot(axis)).fold(1.0, f64::min);
        // Float slack: widen the cone a little.
        (
            axis,
            if cos > 0.0 {
                (cos - 1e-4).max(0.0)
            } else {
                -2.0
            },
        )
    };
    let f = |v: DVec3| [v.x as f32, v.y as f32, v.z as f32];
    Cluster {
        centre: f(centre),
        // Slack for the f32 controls the card evaluates.
        radius: (radius * (1.0 + 1e-5) + 1e-6 * centre.abs().max_element()) as f32,
        axis: f(axis),
        cos: cos as f32,
    }
}

/// Bytes a `PatchNet` takes per patch on each side (CPU here, the copy on
/// the graphics card): controls (≈4 per patch × 12), patch ids (32), owner
/// (4), wire (≈½ per patch × 32), plus on the CPU the topology kept for
/// drags (`PatchState`, ≈40). For memory checks.
pub const BYTES_PER_PATCH: u64 = 4 * 12 + 32 + 4 + 16 + 40;

impl PatchNet {
    /// Triangles drawn: L² quads per patch, two triangles each.
    pub fn triangles(&self) -> u64 {
        self.patches.len() as u64 * 2 * u64::from(self.level).pow(2)
    }
    /// Bytes held (one side).
    pub fn bytes(&self) -> u64 {
        (self.controls.len() * 12
            + self.patches.len() * 36
            + self.wire.len() * 32
            + self.wire_edges.len() * 8
            + self.blocks.len() * 8
            + (self.fill_clusters.len() + self.wire_clusters.len()) * 32) as u64
    }
    fn control(&self, i: u32) -> DVec3 {
        let c = self.controls[i as usize];
        DVec3::new(c[0] as f64, c[1] as f64, c[2] as f64)
    }
    /// The 3 × 3 control grid of `patch`, `c[a][b]` with `a` along side 0
    /// (corner 0 → 1) and `b` along side 3 reversed (corner 0 → 3).
    pub fn grid(&self, patch: usize) -> [[DVec3; 3]; 3] {
        let p = &self.patches[patch];
        let k = |i: u32| self.control(i);
        [
            [k(p[0]), k(p[7]), k(p[3])],
            [k(p[4]), k(self.centre_base + patch as u32), k(p[6])],
            [k(p[1]), k(p[5]), k(p[2])],
        ]
    }
    /// The surface point of `patch` at (u, v) in 0..=1, u along side 0.
    pub fn evaluate(&self, patch: usize, u: f64, v: f64) -> DVec3 {
        let c = self.grid(patch);
        let (bu, bv) = (bernstein(u), bernstein(v));
        let mut p = DVec3::ZERO;
        for a in 0..3 {
            for b in 0..3 {
                p += c[a][b] * bu[a] * bv[b];
            }
        }
        p
    }
}

impl PatchNet {
    /// The unit normal of `patch` at (u, v) (from its slopes), or zero.
    pub fn normal(&self, patch: usize, u: f64, v: f64) -> DVec3 {
        let c = self.grid(patch);
        let (bu, bv, du, dv) = (bernstein(u), bernstein(v), slope(u), slope(v));
        let (mut tu, mut tv) = (DVec3::ZERO, DVec3::ZERO);
        for a in 0..3 {
            for b in 0..3 {
                tu += c[a][b] * du[a] * bv[b];
                tv += c[a][b] * bu[a] * dv[b];
            }
        }
        tu.cross(tv).normalize_or_zero()
    }
}

fn slope(t: f64) -> [f64; 3] {
    [-2.0 * (1.0 - t), 2.0 - 4.0 * t, 2.0 * t]
}

fn bernstein(t: f64) -> [f64; 3] {
    [(1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t]
}

/// Whether `mesh` draws as patches at `level`: it has Subdivision polygons,
/// no Catmull-Clark ones (after SDV-14 joins mixed regions), and the
/// level is above 0.
pub fn eligible(mesh: &Mesh, level: u8) -> bool {
    if level == 0 || !super::has_smooth(mesh) {
        return false;
    }
    let kinds = effective_kinds(mesh);
    let mut any = false;
    for (f, k) in kinds.iter().enumerate() {
        match k {
            PolygonRepresentation::CatmullClark if !mesh.is_polyline(f) => return false,
            PolygonRepresentation::Subdivs if !mesh.is_polyline(f) => any = true,
            _ => {}
        }
    }
    any
}

/// Patches (one per corner) that `mesh`'s Subdivision polygons become,
/// counted without building them.
pub fn patch_count(mesh: &Mesh) -> u64 {
    let kinds = effective_kinds(mesh);
    (0..mesh.faces.len())
        .filter(|&f| kinds[f] == PolygonRepresentation::Subdivs && !mesh.is_polyline(f))
        .map(|f| mesh.faces[f].vertices.len() as u64)
        .sum()
}

/// Run `work` over `0..n` split across the machine's cores, each part
/// writing its own slice of `out`.
fn parallel_fill<T: Send>(out: &mut [T], work: impl Fn(usize, &mut T) + Sync) {
    parallel_fill_from(4096, out, work);
}

/// `parallel_fill`, each thread taking at least `least` items.
fn parallel_fill_from<T: Send>(least: usize, out: &mut [T], work: impl Fn(usize, &mut T) + Sync) {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = out.len().div_ceil(threads).max(least.max(1));
    std::thread::scope(|s| {
        for (k, part) in out.chunks_mut(chunk).enumerate() {
            let work = &work;
            s.spawn(move || {
                for (i, slot) in part.iter_mut().enumerate() {
                    work(k * chunk + i, slot);
                }
            });
        }
    });
}

/// Build the patches of `mesh`'s Subdivision polygons at `level` (≥ 1).
/// `None` when the mesh isn't `eligible`.
pub fn build(mesh: &Mesh, level: u8) -> Option<PatchNet> {
    PatchState::new(mesh, level).map(|(_, net)| net)
}

/// What a drag needs to update a `PatchNet` near moved points: the
/// topology of the step and the patches, and the step's points.
#[derive(Clone, Debug)]
pub struct PatchState {
    topo: Arc<Topo>,
    /// Step points (f32): cage points' new places, then edge points, then
    /// polygon points.
    step: Vec<[f32; 3]>,
}

/// The step's and the patches' numbering (see the module comment).
#[derive(Debug)]
struct Topo {
    id: u64,
    level: u8,
    /// Mesh polygon of each local polygon; mesh point of each local point,
    /// and the reverse (`u32::MAX` for points no patch polygon uses).
    source: Vec<u32>,
    globals: Vec<u32>,
    local: Vec<u32>,
    face_start: Vec<u32>,
    corners: Vec<u32>,
    face_of_corner: Vec<u32>,
    /// Each corner's side (to the next corner) as an edge.
    corner_edge: Vec<u32>,
    edges: Vec<(u32, u32)>,
    /// Up to two (side corner) uses and the use count per edge.
    edge_uses: Vec<([u32; 2], u32)>,
    /// Crease sharpness (10 × weight) per edge.
    raw: Vec<f32>,
    /// Corners at each point.
    point_start: Vec<u32>,
    point_corners: Vec<u32>,
    /// Wire record of each edge, and edge of each record.
    edge_record: Vec<u32>,
    record_edge: Vec<u32>,
    n_points: usize,
    n_edges: usize,
    n_faces: usize,
    n_corners: usize,
    edge_base: u32,
    centre_base: u32,
}

fn v3(p: [f32; 3]) -> DVec3 {
    DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64)
}
fn f3(p: DVec3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

impl Topo {
    fn next(&self, c: usize) -> usize {
        let f = self.face_of_corner[c] as usize;
        if c + 1 == self.face_start[f + 1] as usize {
            self.face_start[f] as usize
        } else {
            c + 1
        }
    }
    fn prev(&self, c: usize) -> usize {
        let f = self.face_of_corner[c] as usize;
        if c == self.face_start[f] as usize {
            self.face_start[f + 1] as usize - 1
        } else {
            c - 1
        }
    }
    fn face_corners(&self, f: usize) -> std::ops::Range<usize> {
        self.face_start[f] as usize..self.face_start[f + 1] as usize
    }
    fn at_point(&self, v: usize) -> &[u32] {
        &self.point_corners[self.point_start[v] as usize..self.point_start[v + 1] as usize]
    }
    fn edge_sp(&self, e: u32) -> u32 {
        (self.n_points + e as usize) as u32
    }
    fn face_sp(&self, f: u32) -> u32 {
        (self.n_points + self.n_edges + f as usize) as u32
    }
    /// Patch `c`'s four step points.
    fn patch_points(&self, c: usize) -> [u32; 4] {
        [
            self.corners[c],
            self.edge_sp(self.corner_edge[c]),
            self.face_sp(self.face_of_corner[c]),
            self.edge_sp(self.corner_edge[self.prev(c)]),
        ]
    }
    /// Child sharpness on a half edge (SDV-07 s - 1; EW2-04 negative
    /// stays), read by the patch stage as "how much like an open edge".
    fn hard(&self, e: usize) -> f64 {
        let s = self.raw[e] as f64;
        let child = if s > 1.0 {
            s - 1.0
        } else if s < 0.0 {
            s
        } else {
            0.0
        };
        if child < 0.0 {
            child / 10.0
        } else {
            child.min(1.0)
        }
    }
    /// The edge control of each half edge, then of each inner edge.
    fn half(&self, e: u32, at: u32) -> u32 {
        self.edge_base + 2 * e + u32::from(at != self.edges[e as usize].0)
    }
    fn inner(&self, c: usize) -> u32 {
        self.edge_base + 2 * self.n_edges as u32 + c as u32
    }
    fn patch(&self, c: usize) -> [u32; 8] {
        let pp = self.patch_points(c);
        let v = self.corners[c];
        let prev = self.prev(c);
        [
            pp[0],
            pp[1],
            pp[2],
            pp[3],
            self.half(self.corner_edge[c], v),
            self.inner(c),
            self.inner(prev),
            self.half(self.corner_edge[prev], v),
        ]
    }

    // ---- One Catmull-Clark step (the curve rule on boundaries, negative
    // weights soften: `catmull_clark_step(.., Smooth, true)`).
    fn face_point(&self, cage: &[DVec3], f: usize) -> DVec3 {
        let r = self.face_corners(f);
        let n = r.len() as f64;
        self.corners[r]
            .iter()
            .map(|&v| cage[self.globals[v as usize] as usize])
            .sum::<DVec3>()
            / n
    }
    fn edge_point(&self, cage: &[DVec3], step: &[[f32; 3]], e: usize) -> DVec3 {
        let k = self.edges[e];
        let a = cage[self.globals[k.0 as usize] as usize];
        let b = cage[self.globals[k.1 as usize] as usize];
        let mid = (a + b) / 2.0;
        let (uses, count) = self.edge_uses[e];
        let r = self.raw[e] as f64;
        let s = if r < 0.0 { r / 10.0 } else { r };
        if count != 2 || s >= 1.0 {
            mid
        } else {
            let fp = |c: u32| v3(step[self.face_sp(self.face_of_corner[c as usize]) as usize]);
            let smooth = (a + b + fp(uses[0]) + fp(uses[1])) / 4.0;
            smooth + (mid - smooth) * s
        }
    }
    fn vertex_point(&self, cage: &[DVec3], step: &[[f32; 3]], v: usize) -> DVec3 {
        let point = |l: u32| cage[self.globals[l as usize] as usize];
        let p = point(v as u32);
        let around = self.at_point(v);
        let mut faces = DVec3::ZERO;
        let mut mids = DVec3::ZERO;
        let mut edge_count = 0u32;
        let mut boundary = [DVec3::ZERO; 2];
        let mut boundary_count = 0u32;
        let mut sharp = [DVec3::ZERO; 2];
        let mut sharp_count = 0u32;
        let mut sharp_sum = 0.0;
        // Each edge at v once: met from up to two corners at v.
        let mut seen: smallvec_lite::Small = Default::default();
        for &c in around {
            let c = c as usize;
            faces += v3(step[self.face_sp(self.face_of_corner[c]) as usize]);
            for e in [self.corner_edge[c], self.corner_edge[self.prev(c)]] {
                if !seen.insert(e) {
                    continue;
                }
                let k = self.edges[e as usize];
                let other = if k.0 as usize == v { k.1 } else { k.0 };
                let q = point(other);
                mids += (p + q) / 2.0;
                edge_count += 1;
                if self.edge_uses[e as usize].1 != 2 {
                    if boundary_count < 2 {
                        boundary[boundary_count as usize] = q;
                    }
                    boundary_count += 1;
                }
                let s = (self.raw[e as usize] as f64).max(0.0);
                if s > 0.0 {
                    if sharp_count < 2 {
                        sharp[sharp_count as usize] = q;
                    }
                    sharp_count += 1;
                    sharp_sum += s;
                }
            }
        }
        if edge_count == 0 {
            p
        } else if boundary_count > 0 {
            // The Smooth rule: every two-edge boundary point curves.
            if boundary_count == 2 {
                (boundary[0] + 6.0 * p + boundary[1]) / 8.0
            } else {
                p
            }
        } else {
            let n = edge_count as f64;
            let f = faces / around.len() as f64;
            let r = mids / n;
            let smooth = (f + 2.0 * r + (n - 3.0) * p) / n;
            let average = sharp_sum / sharp_count.max(1) as f64;
            let rule_point = match sharp_count {
                0 | 1 => None,
                2 => Some((sharp[0] + 6.0 * p + sharp[1]) / 8.0),
                _ => Some(p),
            };
            match rule_point {
                None => smooth,
                Some(q) if average >= 1.0 => q,
                Some(q) => smooth + (q - smooth) * average,
            }
        }
    }

    // ---- The patch stage (`modo_surface`): controls from the step.
    fn centre(&self, step: &[[f32; 3]], c: usize) -> DVec3 {
        self.patch_points(c)
            .iter()
            .map(|&i| v3(step[i as usize]))
            .sum::<DVec3>()
            / 4.0
    }
    /// The quad (patch) of use `c` of an edge that holds the half at `end`.
    fn quad_at(&self, c: u32, end: u32) -> usize {
        let c = c as usize;
        if self.corners[c] == end {
            c
        } else {
            self.next(c)
        }
    }
    fn half_control(&self, step: &[[f32; 3]], centre: &impl Fn(usize) -> DVec3, h: usize) -> DVec3 {
        let e = h / 2;
        let k = self.edges[e];
        let end = if h.is_multiple_of(2) { k.0 } else { k.1 };
        let (a, b) = (
            v3(step[end as usize]),
            v3(step[self.edge_sp(e as u32) as usize]),
        );
        let mid = (a + b) / 2.0;
        let (uses, count) = self.edge_uses[e];
        if count == 2 {
            let smooth =
                (centre(self.quad_at(uses[0], end)) + centre(self.quad_at(uses[1], end))) / 2.0;
            smooth + (mid - smooth) * self.hard(e)
        } else {
            mid
        }
    }
    fn inner_control(&self, centre: &impl Fn(usize) -> DVec3, c: usize) -> DVec3 {
        // From the corner's edge point to the polygon point: shared by
        // patches c and next(c), never creased.
        (centre(c) + centre(self.next(c))) / 2.0
    }
    /// The corner control of step point `sp`.
    fn corner_control(
        &self,
        step: &[[f32; 3]],
        centre: &impl Fn(usize) -> DVec3,
        sp: usize,
    ) -> DVec3 {
        let sv = |i: usize| v3(step[i]);
        if sp >= self.n_points + self.n_edges {
            let f = sp - self.n_points - self.n_edges;
            let r = self.face_corners(f);
            let n = r.len() as f64;
            return r.map(centre).sum::<DVec3>() / n;
        }
        if sp >= self.n_points {
            let e = sp - self.n_points;
            let k = self.edges[e];
            let (uses, count) = self.edge_uses[e];
            let p = sv(sp);
            if count != 2 {
                // Both halves are open: the average of their midpoints.
                let a = (p + sv(k.0 as usize)) / 2.0;
                let b = (p + sv(k.1 as usize)) / 2.0;
                return (a + b) / 2.0;
            }
            let quads = [
                uses[0] as usize,
                self.next(uses[0] as usize),
                uses[1] as usize,
                self.next(uses[1] as usize),
            ];
            let smooth = quads.iter().map(|&q| centre(q)).sum::<DVec3>() / 4.0;
            let h = self.hard(e);
            // Both halves carry the same sharpness; the inner edges none.
            return if h == 0.0 {
                smooth
            } else {
                let target = (p * 2.0 + sv(k.0 as usize) + sv(k.1 as usize)) / 4.0;
                smooth + (target - smooth) * h
            };
        }
        let v = sp;
        let around = self.at_point(v);
        let p = sv(v);
        // Open halves at v (each once), else the still-sharp ones.
        let mut seen: smallvec_lite::Small = Default::default();
        let mut open_sum = DVec3::ZERO;
        let mut open_count = 0u32;
        let mut sharp: [(u32, f64); 2] = [(0, 0.0); 2];
        let mut sharp_count = 0usize;
        let mut sharp_total = 0.0;
        let mut centre_sum = DVec3::ZERO;
        for &c in around {
            let c = c as usize;
            centre_sum += centre(c);
            for e in [self.corner_edge[c], self.corner_edge[self.prev(c)]] {
                if !seen.insert(e) {
                    continue;
                }
                let e = e as usize;
                let q = sv(self.edge_sp(e as u32) as usize);
                if self.edge_uses[e].1 != 2 {
                    open_sum += (p + q) / 2.0;
                    open_count += 1;
                } else {
                    let h = self.hard(e);
                    if h != 0.0 {
                        if sharp_count < 2 {
                            sharp[sharp_count] = (e as u32, h);
                        }
                        sharp_count += 1;
                        sharp_total += h;
                    }
                }
            }
        }
        if open_count > 0 {
            open_sum / open_count as f64
        } else {
            let smooth = centre_sum / around.len() as f64;
            match sharp_count {
                0 | 1 => smooth,
                2 => {
                    let target = (p * 2.0
                        + sv(self.edge_sp(sharp[0].0) as usize)
                        + sv(self.edge_sp(sharp[1].0) as usize))
                        / 4.0;
                    smooth + (target - smooth) * ((sharp[0].1 + sharp[1].1) / 2.0)
                }
                n => smooth + (p - smooth) * (sharp_total / n as f64),
            }
        }
    }
    /// The wire record's two side normals (see `PatchNet::wire_sides`).
    fn sides(&self, net: &PatchNet, r: usize) -> [u32; 2] {
        let e = self.record_edge[r] as usize;
        let (uses, count) = self.edge_uses[e];
        let pack = |n: DVec3, w: f32| {
            let b = |x: f32| ((x.clamp(-1.0, 1.0) * 127.0).round() as i8) as u8;
            u32::from_le_bytes([b(n.x as f32), b(n.y as f32), b(n.z as f32), b(w)])
        };
        if count > 2 {
            return [pack(DVec3::ZERO, 1.0), pack(DVec3::ZERO, 1.0)];
        }
        // The edge's middle is the corner (1, 0) of the use's patch.
        let side = |c: u32| pack(net.normal(c as usize, 1.0, 0.0), 0.0);
        [side(uses[0]), if count == 2 { side(uses[1]) } else { 0 }]
    }
}

/// Run `work` for each of `items` (in parallel when there are many),
/// collecting the results in order.
fn par_map<T: Send>(items: &[usize], work: impl Fn(usize) -> T + Sync) -> Vec<T> {
    par_map_from(8192, items, work)
}

/// `par_map` going parallel from `least` items (less for heavier work).
fn par_map_from<T: Send>(
    least: usize,
    items: &[usize],
    work: impl Fn(usize) -> T + Sync,
) -> Vec<T> {
    if items.len() < least {
        return items.iter().map(|&i| work(i)).collect();
    }
    let mut out: Vec<Option<T>> = (0..items.len()).map(|_| None).collect();
    parallel_fill_from(least.min(4096), &mut out, |k, slot| {
        *slot = Some(work(items[k]))
    });
    out.into_iter().map(|t| t.expect("filled")).collect()
}

/// Sorted, without repeats.
fn unique(mut v: Vec<usize>) -> Vec<usize> {
    v.sort_unstable();
    v.dedup();
    v
}

static TOPOLOGIES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl PatchState {
    /// Build the state and the patches of `mesh` at `level` (≥ 1). `None`
    /// when the mesh isn't `eligible`.
    pub fn new(mesh: &Mesh, level: u8) -> Option<(PatchState, PatchNet)> {
        if !eligible(mesh, level) {
            return None;
        }
        let topo = Arc::new(Topo::new(mesh, level));
        let t = &topo;
        let cage = &mesh.vertices;
        let mut step = vec![[0f32; 3]; t.n_points + t.n_edges + t.n_faces];
        {
            let (verts, rest) = step.split_at_mut(t.n_points);
            let (edge_pts, face_pts) = rest.split_at_mut(t.n_edges);
            parallel_fill(face_pts, |f, out| *out = f3(t.face_point(cage, f)));
            // Edge points read only polygon points: give them a view of
            // the whole list with polygon points filled.
            let mut whole = vec![[0f32; 3]; t.n_points + t.n_edges];
            whole.extend_from_slice(face_pts);
            parallel_fill(edge_pts, |e, out| *out = f3(t.edge_point(cage, &whole, e)));
            parallel_fill(verts, |v, out| *out = f3(t.vertex_point(cage, &whole, v)));
        }
        let mut controls = vec![[0f32; 3]; t.centre_base as usize + t.n_corners];
        {
            let (corner_ctrls, rest) = controls.split_at_mut(t.edge_base as usize);
            let (edge_ctrls, centres) = rest.split_at_mut(2 * t.n_edges + t.n_corners);
            parallel_fill(centres, |c, out| *out = f3(t.centre(&step, c)));
            let centre = |c: usize| v3(centres[c]);
            let centre = &centre;
            {
                let (halves, inner) = edge_ctrls.split_at_mut(2 * t.n_edges);
                parallel_fill(halves, |h, out| *out = f3(t.half_control(&step, centre, h)));
                parallel_fill(inner, |c, out| *out = f3(t.inner_control(centre, c)));
            }
            parallel_fill(corner_ctrls, |sp, out| {
                *out = f3(t.corner_control(&step, centre, sp));
            });
        }
        let mut patches = vec![[0u32; 8]; t.n_corners];
        parallel_fill(&mut patches, |c, out| *out = t.patch(c));
        let owner: Vec<u32> = t
            .face_of_corner
            .iter()
            .map(|&f| t.source[f as usize])
            .collect();
        let mut blocks = vec![0..0; mesh.faces.len()];
        for (f, &s) in t.source.iter().enumerate() {
            blocks[s as usize] = t.face_start[f]..t.face_start[f + 1];
        }
        let mut wire = Vec::with_capacity(t.n_edges);
        let mut wire_edges = Vec::with_capacity(t.n_edges);
        for &e in &t.record_edge {
            let k = t.edges[e as usize];
            let uses = t.edge_uses[e as usize].1.clamp(1, 2);
            wire.push([k.0, 2 * e, t.edge_sp(e), 2 * e + 1, k.1, uses]);
            wire_edges.push(crate::topology::edge_key(
                t.globals[k.0 as usize],
                t.globals[k.1 as usize],
            ));
        }
        for w in &mut wire {
            w[1] += t.edge_base;
            w[3] += t.edge_base;
        }
        let closed = t.edge_uses.iter().all(|u| u.1 == 2);
        let mut net = PatchNet {
            controls: Chunks::from_vec(controls),
            patches: Arc::new(patches),
            centre_base: t.centre_base,
            owner: Arc::new(owner),
            blocks: Arc::new(blocks),
            wire: Arc::new(wire),
            wire_edges: Arc::new(wire_edges),
            level,
            closed,
            topology: t.id,
            ..Default::default()
        };
        let n_fill = net.patches.len().div_ceil(fill_per_cluster(level) as usize);
        let n_wire = net.wire.len().div_ceil(wire_per_cluster(level) as usize);
        let all = |n: usize| (0..n).collect::<Vec<_>>();
        net.fill_clusters = Chunks::from_vec(par_map(&all(n_fill), |k| fill_cluster(&net, k)));
        net.wire_clusters = Chunks::from_vec(par_map(&all(n_wire), |k| wire_cluster(t, &net, k)));
        net.wire_sides = Chunks::from_vec(par_map(&all(net.wire.len()), |r| t.sides(&net, r)));
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for c in net.controls.iter() {
            for i in 0..3 {
                lo[i] = lo[i].min(c[i]);
                hi[i] = hi[i].max(c[i]);
            }
        }
        net.bounds = [lo, hi];
        Some((PatchState { topo, step }, net))
    }

    /// Whether `net` came from this state.
    pub fn made(&self, net: &PatchNet) -> bool {
        net.topology == self.topo.id
    }

    /// `net` (from this state) after the cage points `moved` (mesh
    /// numbering) of `mesh` moved, nothing else changing: only the
    /// controls, clusters and sides near them are worked out again, and
    /// the result shares every untouched chunk with `net`.
    pub fn update(&mut self, net: &PatchNet, mesh: &Mesh, moved: &[u32]) -> PatchNet {
        let t = Arc::clone(&self.topo);
        let cage = &mesh.vertices;
        let mut out = net.clone();
        let moved: Vec<usize> = moved
            .iter()
            .filter_map(|&g| t.local.get(g as usize).copied())
            .filter(|&l| l != u32::MAX)
            .map(|l| l as usize)
            .collect();
        if moved.is_empty() {
            return out;
        }
        let faces_at = |points: &[usize]| {
            unique(
                points
                    .iter()
                    .flat_map(|&v| {
                        t.at_point(v)
                            .iter()
                            .map(|&c| t.face_of_corner[c as usize] as usize)
                    })
                    .collect(),
            )
        };
        let points_of = |faces: &[usize]| {
            unique(
                faces
                    .iter()
                    .flat_map(|&f| t.face_corners(f).map(|c| t.corners[c] as usize))
                    .collect(),
            )
        };
        let edges_of = |faces: &[usize]| {
            unique(
                faces
                    .iter()
                    .flat_map(|&f| t.face_corners(f).map(|c| t.corner_edge[c] as usize))
                    .collect(),
            )
        };
        let corners_of = |faces: &[usize]| -> Vec<usize> {
            faces.iter().flat_map(|&f| t.face_corners(f)).collect()
        };
        let r0 = faces_at(&moved);
        let v0 = points_of(&r0);
        let e0 = edges_of(&r0);
        let r1 = faces_at(&v0);
        // The step near the moved points.
        for (&f, p) in r0.iter().zip(par_map(&r0, |f| t.face_point(cage, f))) {
            self.step[t.face_sp(f as u32) as usize] = f3(p);
        }
        let step = &self.step;
        let edge_pts = par_map(&e0, |e| t.edge_point(cage, step, e));
        let vert_pts = par_map(&v0, |v| t.vertex_point(cage, step, v));
        for (&e, p) in e0.iter().zip(edge_pts) {
            self.step[t.edge_sp(e as u32) as usize] = f3(p);
        }
        for (&v, p) in v0.iter().zip(vert_pts) {
            self.step[v] = f3(p);
        }
        let step = &self.step;
        let set = |out: &mut PatchNet, i: u32, p: DVec3| {
            let p = f3(p);
            out.controls.set(i as usize, p);
            for (k, &x) in p.iter().enumerate() {
                out.bounds[0][k] = out.bounds[0][k].min(x);
                out.bounds[1][k] = out.bounds[1][k].max(x);
            }
        };
        // Centres, then the edge and corner controls reading them.
        let c1 = corners_of(&r1);
        for (&c, p) in c1.iter().zip(par_map(&c1, |c| t.centre(step, c))) {
            set(&mut out, t.centre_base + c as u32, p);
        }
        let centre = |c: usize| v3(out.controls[t.centre_base as usize + c]);
        let inner = par_map(&c1, |c| t.inner_control(&centre, c));
        let e1 = edges_of(&r1);
        let halves: Vec<usize> = e1.iter().flat_map(|&e| [2 * e, 2 * e + 1]).collect();
        let half = par_map(&halves, |h| t.half_control(step, &centre, h));
        let v1 = points_of(&r1);
        let sps: Vec<usize> = v1
            .iter()
            .copied()
            .chain(e1.iter().map(|&e| t.edge_sp(e as u32) as usize))
            .chain(r1.iter().map(|&f| t.face_sp(f as u32) as usize))
            .collect();
        let corner = par_map(&sps, |sp| t.corner_control(step, &centre, sp));
        for (&c, p) in c1.iter().zip(inner) {
            set(&mut out, t.inner(c), p);
        }
        for (&h, p) in halves.iter().zip(half) {
            set(&mut out, t.edge_base + h as u32, p);
        }
        for (&sp, p) in sps.iter().zip(corner) {
            set(&mut out, sp as u32, p);
        }
        // Clusters and sides of every patch reading a changed control.
        let r2 = faces_at(&v1);
        let per = fill_per_cluster(t.level) as usize;
        let fill = unique(corners_of(&r2).iter().map(|&c| c / per).collect());
        for (&k, cl) in fill
            .iter()
            .zip(par_map_from(64, &fill, |k| fill_cluster(&out, k)))
        {
            out.fill_clusters.set(k, cl);
        }
        let records: Vec<usize> = edges_of(&r2)
            .iter()
            .map(|&e| t.edge_record[e] as usize)
            .collect();
        let per = wire_per_cluster(t.level) as usize;
        let wire = unique(records.iter().map(|&r| r / per).collect());
        for (&k, cl) in wire
            .iter()
            .zip(par_map_from(64, &wire, |k| wire_cluster(&t, &out, k)))
        {
            out.wire_clusters.set(k, cl);
        }
        for (&r, s) in records
            .iter()
            .zip(par_map_from(512, &records, |r| t.sides(&out, r)))
        {
            out.wire_sides.set(r, s);
        }
        out
    }
}

/// The cage points that moved from `old` to `new` (mesh numbering), when
/// nothing else that patches read changed (polygons, kinds, creases);
/// `None` otherwise. Cheap during a drag, which shares the polygon list.
pub fn moved(old: &Mesh, new: &Mesh) -> Option<Vec<u32>> {
    let same_faces = crate::Shared::same(&old.faces, &new.faces)
        || (old.faces.len() == new.faces.len()
            && old
                .faces
                .iter()
                .zip(new.faces.iter())
                .all(|(a, b)| a.vertices == b.vertices));
    let same_attributes = crate::Shared::same(&old.attributes, &new.attributes)
        || (old.attributes.representations == new.attributes.representations
            && old.attributes.creases == new.attributes.creases);
    (old.vertices.len() == new.vertices.len() && same_faces && same_attributes)
        .then(|| crate::mesh::moved_points(&old.vertices, &new.vertices))
}

/// Fill cluster `k`: its patches' controls and normals.
fn fill_cluster(net: &PatchNet, k: usize) -> Cluster {
    let per = fill_per_cluster(net.level) as usize;
    let (mut points, mut normals) = (Vec::new(), Vec::new());
    for p in k * per..((k + 1) * per).min(net.patches.len()) {
        let g = net.grid(p);
        points.extend(g.iter().flatten());
        patch_normals(&g, &mut normals);
    }
    cluster(&points, &normals)
}

/// Wire cluster `k`: each record's normals those of the patches beside
/// its edge (both polygons).
fn wire_cluster(t: &Topo, net: &PatchNet, k: usize) -> Cluster {
    let per = wire_per_cluster(net.level) as usize;
    let (mut points, mut normals) = (Vec::new(), Vec::new());
    for &e in &t.record_edge[k * per..((k + 1) * per).min(net.wire.len())] {
        let (uses, count) = t.edge_uses[e as usize];
        if count > 2 {
            normals.clear();
            break;
        }
        for &c in &uses[..count as usize] {
            for p in [c as usize, t.next(c as usize)] {
                let g = net.grid(p);
                points.extend(g.iter().flatten());
                patch_normals(&g, &mut normals);
            }
        }
    }
    if normals.is_empty() {
        Cluster {
            cos: -2.0,
            radius: f32::MAX,
            ..Default::default()
        }
    } else {
        cluster(&points, &normals)
    }
}

impl Topo {
    fn new(mesh: &Mesh, level: u8) -> Topo {
        let kinds = effective_kinds(mesh);
        let source: Vec<u32> = (0..mesh.faces.len())
            .filter(|&f| kinds[f] == PolygonRepresentation::Subdivs && !mesh.is_polyline(f))
            .map(|f| f as u32)
            .collect();
        // The local net: points used by the source polygons, numbered in
        // first use, as `surface` numbers them.
        let mut local = vec![u32::MAX; mesh.vertices.len()];
        let mut globals: Vec<u32> = Vec::new();
        let mut face_start: Vec<u32> = Vec::with_capacity(source.len() + 1);
        let mut corners: Vec<u32> = Vec::new();
        for &f in &source {
            face_start.push(corners.len() as u32);
            for &v in &mesh.faces[f as usize].vertices {
                let l = &mut local[v as usize];
                if *l == u32::MAX {
                    *l = globals.len() as u32;
                    globals.push(v);
                }
                corners.push(*l);
            }
        }
        face_start.push(corners.len() as u32);
        let n_points = globals.len();
        let n_faces = source.len();
        let n_corners = corners.len();
        let mut face_of_corner = vec![0u32; n_corners];
        for f in 0..n_faces {
            for c in face_start[f]..face_start[f + 1] {
                face_of_corner[c as usize] = f as u32;
            }
        }
        let mut t = Topo {
            id: TOPOLOGIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            level,
            source,
            globals,
            local,
            face_start,
            corners,
            face_of_corner,
            corner_edge: Vec::new(),
            edges: Vec::new(),
            edge_uses: Vec::new(),
            raw: Vec::new(),
            point_start: Vec::new(),
            point_corners: Vec::new(),
            edge_record: Vec::new(),
            record_edge: Vec::new(),
            n_points,
            n_edges: 0,
            n_faces,
            n_corners,
            edge_base: 0,
            centre_base: 0,
        };
        // Edges, found per low point (a bucket per point, short lists).
        let mut by_low = vec![0u32; n_points + 1];
        for c in 0..n_corners {
            let (a, b) = (t.corners[c], t.corners[t.next(c)]);
            by_low[a.min(b) as usize + 1] += 1;
        }
        for i in 0..n_points {
            by_low[i + 1] += by_low[i];
        }
        let mut bucket = vec![0u32; n_corners];
        {
            let mut fill = by_low.clone();
            for c in 0..n_corners {
                let (a, b) = (t.corners[c], t.corners[t.next(c)]);
                let lo = a.min(b) as usize;
                bucket[fill[lo] as usize] = c as u32;
                fill[lo] += 1;
            }
        }
        let mut corner_edge = vec![0u32; n_corners];
        let mut edges: Vec<(u32, u32)> = Vec::with_capacity(n_corners / 2 + 1);
        let mut edge_uses: Vec<([u32; 2], u32)> = Vec::with_capacity(n_corners / 2 + 1);
        for lo in 0..n_points {
            let first = edges.len();
            for &c in &bucket[by_low[lo] as usize..by_low[lo + 1] as usize] {
                let c = c as usize;
                let hi = t.corners[c].max(t.corners[t.next(c)]);
                let id = match edges[first..].iter().position(|e| e.1 == hi) {
                    Some(k) => first + k,
                    None => {
                        edges.push((lo as u32, hi));
                        edge_uses.push(([0; 2], 0));
                        edges.len() - 1
                    }
                };
                let u = &mut edge_uses[id];
                if u.1 < 2 {
                    u.0[u.1 as usize] = c as u32;
                }
                u.1 += 1;
                corner_edge[c] = id as u32;
            }
        }
        let n_edges = edges.len();
        // Creases: sharpness 10 × weight (SDV-07).
        let mut raw = vec![0f32; n_edges];
        if !mesh.attributes.creases.is_empty() {
            let mut lookup: crate::fxhash::FxHashMap<(u32, u32), f32> = Default::default();
            for c in &mesh.attributes.creases {
                let (a, b) = (t.local[c.a as usize], t.local[c.b as usize]);
                if a != u32::MAX && b != u32::MAX {
                    lookup.insert(
                        crate::topology::edge_key(a, b),
                        (10.0 * c.weight.clamp(-1.0, 1.0)) as f32,
                    );
                }
            }
            for (e, k) in edges.iter().enumerate() {
                if let Some(&s) = lookup.get(k) {
                    raw[e] = s;
                }
            }
        }
        // Point → corners using it.
        let mut point_start = vec![0u32; n_points + 1];
        for &v in &t.corners {
            point_start[v as usize + 1] += 1;
        }
        for i in 0..n_points {
            point_start[i + 1] += point_start[i];
        }
        let mut point_corners = vec![0u32; n_corners];
        {
            let mut fill = point_start.clone();
            for (c, &v) in t.corners.iter().enumerate() {
                point_corners[fill[v as usize] as usize] = c as u32;
                fill[v as usize] += 1;
            }
        }
        // Wire records in the order the polygons first use their edges
        // (neighbouring records lie near each other on the surface).
        let mut edge_record = vec![u32::MAX; n_edges];
        let mut record_edge = Vec::with_capacity(n_edges);
        for &e in &corner_edge {
            let r = &mut edge_record[e as usize];
            if *r == u32::MAX {
                *r = record_edge.len() as u32;
                record_edge.push(e);
            }
        }
        let edge_base = (n_points + n_edges + n_faces) as u32;
        t.centre_base = edge_base + (2 * n_edges + n_corners) as u32;
        t.edge_base = edge_base;
        t.corner_edge = corner_edge;
        t.edges = edges;
        t.edge_uses = edge_uses;
        t.raw = raw;
        t.point_start = point_start;
        t.point_corners = point_corners;
        t.edge_record = edge_record;
        t.record_edge = record_edge;
        t.n_edges = n_edges;
        t
    }
}

/// A tiny set of edge numbers for the handful of edges at one point.
mod smallvec_lite {
    #[derive(Default)]
    pub struct Small {
        inline: [u32; 16],
        len: usize,
        spill: Vec<u32>,
    }
    impl Small {
        /// False when `e` was already in.
        pub fn insert(&mut self, e: u32) -> bool {
            if self.inline[..self.len.min(16)].contains(&e) || self.spill.contains(&e) {
                return false;
            }
            if self.len < 16 {
                self.inline[self.len] = e;
            } else {
                self.spill.push(e);
            }
            self.len += 1;
            true
        }
    }
}

#[cfg(test)]
mod tests;
