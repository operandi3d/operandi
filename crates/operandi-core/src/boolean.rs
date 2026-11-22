//! Boolean (booleans/BOOLEANS.md, stage 1: BOOL-01..25): Union, Intersect,
//! Subtract and Add of the active mesh (the target) with driver meshes.
//!
//! The pipeline (plan from the boolean engine stress test, Jason
//! 2026-11-10: "I do favor stability"):
//!
//! 1. Pick the drivers (BOOL-06/07) and bring everything into the target's
//!    space, then into world space for the engines.
//! 2. Pre-flight every input (BOOL-19..21): closed, edge-manifold,
//!    consistently facing, not inside out, finite, not passing through
//!    itself. A failing input is refused with its name and problem edges;
//!    degenerate polygons are left out (BOOL-20).
//! 3. Move the inputs so their joint bounding box is centred on the origin.
//!    Both engines size their tolerance from the largest coordinate, so
//!    small parts far from the origin otherwise lose precision.
//! 4. Run Manifold, then a cheap check of its result (closed, finite,
//!    volume within what the operation allows).
//! 5. Always run boolmesh on the same inputs as a second opinion. When the
//!    two disagree on volume or extent, refuse and leave the mesh alone.
//! 6. Rebuild polygons from Manifold's per-polygon provenance: untouched
//!    faces come back as they were, cut faces as one n-gon per piece
//!    (BOOL-09), each piece keeping its source polygon's tags (BOOL-12) and
//!    corner data (BOOL-13, interpolated within the source polygon).
//!
//! Provisional (see docs/SPECIFICATION_QUESTIONS.md BOOL-Q*): the Last
//! Selected reading (Q1), no "Close holes and retry", no BOOL-30 named
//! result sets, and no progress bar or Esc cancel (BOOL-22) yet.
use crate::edge_commands::{CornerBlend, OutFace, rebuild};
use crate::topology::edge_key;
use crate::{Editor, Mesh, Mode, Result};
use glam::{DMat4, DVec3};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// BOOL-05: the Operation choice, in the command's numeric order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BooleanOp {
    #[default]
    Union,
    Intersect,
    Subtract,
    Add,
}

impl BooleanOp {
    pub const ALL: [BooleanOp; 4] = [Self::Union, Self::Intersect, Self::Subtract, Self::Add];
    pub fn label(self) -> &'static str {
        match self {
            Self::Union => "Union",
            Self::Intersect => "Intersect",
            Self::Subtract => "Subtract",
            Self::Add => "Add",
        }
    }
    /// `mode:<int>` or a readable word (union, intersect, subtract, add).
    pub fn parse(text: &str) -> Result<Self> {
        let t = text.trim();
        if let Ok(i) = t.parse::<usize>() {
            return Self::ALL
                .get(i)
                .copied()
                .ok_or_else(|| format!("Boolean mode must be 0 to 3, not {i}"));
        }
        Self::ALL
            .into_iter()
            .find(|o| o.label().eq_ignore_ascii_case(t))
            .ok_or_else(|| format!("Unknown Boolean mode: {t}"))
    }
}

/// BOOL-06: which meshes drive the boolean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DriveMesh {
    /// Every visible mesh item that isn't selected.
    #[default]
    Background,
    /// Within the target, the pieces holding selected polygons (Q1,
    /// provisional).
    LastSelected,
    /// A chosen mesh item, or a group's meshes.
    Item,
}

impl DriveMesh {
    pub const ALL: [DriveMesh; 3] = [Self::Background, Self::LastSelected, Self::Item];
    pub fn label(self) -> &'static str {
        match self {
            Self::Background => "Background",
            Self::LastSelected => "Last Selected",
            Self::Item => "Specific Surface Item",
        }
    }
    /// The readable word scripts may use instead of the number.
    pub fn word(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::LastSelected => "lastSelected",
            Self::Item => "item",
        }
    }
    /// `cutmesh:<int>` or a readable word (background, lastSelected, item).
    pub fn parse(text: &str) -> Result<Self> {
        let t = text.trim();
        if let Ok(i) = t.parse::<usize>() {
            return Self::ALL
                .get(i)
                .copied()
                .ok_or_else(|| format!("Boolean cutmesh must be 0 to 2, not {i}"));
        }
        Self::ALL
            .into_iter()
            .find(|d| d.word().eq_ignore_ascii_case(t))
            .ok_or_else(|| format!("Unknown Boolean cutmesh: {t}"))
    }
}

/// The dialog's and `poly.boolean`'s options (BOOL-23/25).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BooleanOptions {
    pub op: BooleanOp,
    pub drive: DriveMesh,
    /// The driver item for [`DriveMesh::Item`].
    pub item: Option<u64>,
    /// Add the drivers' child mesh items to the drivers.
    pub children: bool,
}

/// An input the pre-flight check refused (BOOL-19/21): which item, what's
/// wrong, and the edges to select so the user can see it. The edges are in
/// that item's own point numbering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BooleanProblem {
    pub item: u64,
    pub message: String,
    pub edges: Vec<(u32, u32)>,
}

// ── Inputs ──────────────────────────────────────────────────────────────

/// One solid going into the engines: a list of polygons of the combined
/// mesh (see [`Gathered`]).
pub(crate) struct Input {
    pub(crate) item: u64,
    pub(crate) name: String,
    pub(crate) faces: Vec<usize>,
    /// This input's first point in the combined mesh (its point numbers
    /// there minus this are its own).
    pub(crate) point_offset: u32,
}

/// The target mesh with every driver appended, all in the target's local
/// space, so result polygons can name any input polygon as their source.
pub(crate) struct Gathered {
    pub(crate) combined: Mesh,
    pub(crate) target: Input,
    pub(crate) drivers: Vec<Input>,
    /// Target local → world.
    pub(crate) to_world: DMat4,
}

/// A triangle soup in recentred world space, each triangle tagged with
/// the combined-mesh polygon it came from.
#[derive(Clone, Debug, Default)]
pub(crate) struct Tri {
    pub(crate) pos: Vec<DVec3>,
    pub(crate) tris: Vec<[u32; 3]>,
    pub(crate) owner: Vec<u32>,
}

impl Tri {
    pub(crate) fn volume(&self) -> f64 {
        let c = bbox_centre(&self.pos);
        self.tris
            .iter()
            .map(|t| {
                let [a, b, d] = t.map(|i| self.pos[i as usize] - c);
                a.dot(b.cross(d))
            })
            .sum::<f64>()
            / 6.
    }
}

pub(crate) fn bbox(points: &[DVec3]) -> (DVec3, DVec3) {
    points.iter().fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |(lo, hi), &p| (lo.min(p), hi.max(p)),
    )
}

fn bbox_centre(points: &[DVec3]) -> DVec3 {
    if points.is_empty() {
        return DVec3::ZERO;
    }
    let (lo, hi) = bbox(points);
    (lo + hi) * 0.5
}

pub(crate) fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

// ── Pre-flight (BOOL-19..21) ────────────────────────────────────────────

/// Check `input` and triangulate it into world space (not yet recentred).
/// Returns the triangles and how many degenerate polygons were left out.
pub(crate) fn preflight(
    g: &Gathered,
    input: &Input,
) -> std::result::Result<(Tri, usize), BooleanProblem> {
    let mesh = &g.combined;
    let own = |a: u32, b: u32| {
        let (a, b) = (a - input.point_offset, b - input.point_offset);
        edge_key(a, b)
    };
    let refuse = |message: String, edges: Vec<(u32, u32)>| BooleanProblem {
        item: input.item,
        message,
        edges,
    };
    if input.faces.iter().any(|&f| mesh.is_polyline(f)) {
        return Err(refuse(
            format!("{} has curves; Boolean works on polygons only", input.name),
            Vec::new(),
        ));
    }
    // BOOL-20: zero-area polygons are left out of the boolean.
    let (faces, degenerate): (Vec<usize>, Vec<usize>) = input
        .faces
        .iter()
        .partition(|&&f| mesh.normal(f) != DVec3::ZERO);
    if faces.is_empty() {
        return Err(refuse(
            format!("{} has no polygons", input.name),
            Vec::new(),
        ));
    }
    // Closed, edge-manifold and consistently facing, on the polygons.
    let mut uses: HashMap<(u32, u32), (u32, i32)> = HashMap::new();
    for &f in &faces {
        let v = &mesh.faces[f].vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            let e = uses.entry(edge_key(a, b)).or_default();
            e.0 += 1;
            e.1 += if a < b { 1 } else { -1 };
        }
    }
    let mut open = Vec::new();
    let mut crowded = Vec::new();
    let mut flipped = Vec::new();
    for (&(a, b), &(n, dir)) in &uses {
        match n {
            1 => open.push(own(a, b)),
            2 if dir != 0 => flipped.push(own(a, b)),
            2 => {}
            _ => crowded.push(own(a, b)),
        }
    }
    for list in [&mut open, &mut crowded, &mut flipped] {
        list.sort_unstable();
    }
    if !open.is_empty() {
        return Err(refuse(
            format!(
                "{} has {}, so it isn't a closed solid",
                input.name,
                plural(open.len(), "open edge", "open edges")
            ),
            open,
        ));
    }
    if !crowded.is_empty() {
        return Err(refuse(
            format!(
                "{} has {} shared by more than two polygons",
                input.name,
                plural(crowded.len(), "edge", "edges")
            ),
            crowded,
        ));
    }
    if !flipped.is_empty() {
        return Err(refuse(
            format!(
                "{} has polygons facing opposite ways along {}",
                input.name,
                plural(flipped.len(), "edge", "edges")
            ),
            flipped,
        ));
    }
    // Triangulate in world space.
    let mut tri = Tri::default();
    let mut index: HashMap<u32, u32> = HashMap::new();
    for &f in &faces {
        let v = &mesh.faces[f].vertices;
        let points: Vec<DVec3> = v.iter().map(|&i| mesh.vertices[i as usize]).collect();
        let triangles = if v.len() == 3 {
            vec![[0, 1, 2]]
        } else {
            crate::polygon_triple::triangulate_polygon(&points, |_, _| false).map_err(|e| {
                refuse(
                    format!(
                        "{} has a polygon Boolean can't split into triangles ({e})",
                        input.name
                    ),
                    Vec::new(),
                )
            })?
        };
        for t in triangles {
            let corners = t.map(|c| {
                let p = v[c];
                *index.entry(p).or_insert_with(|| {
                    tri.pos
                        .push(g.to_world.transform_point3(mesh.vertices[p as usize]));
                    tri.pos.len() as u32 - 1
                })
            });
            tri.tris.push(corners);
            tri.owner.push(f as u32);
        }
    }
    if tri
        .pos
        .iter()
        .any(|p| !p.is_finite() || p.abs().max_element() > 1e9)
    {
        return Err(refuse(
            format!("{} has points that aren't finite numbers", input.name),
            Vec::new(),
        ));
    }
    if tri.volume() <= 0. {
        return Err(refuse(
            format!(
                "{} is inside out: its polygons face inward (Flip them first)",
                input.name
            ),
            Vec::new(),
        ));
    }
    if let Some((x, y)) = self_intersection(&tri) {
        let edges_of = |t: usize| {
            let [a, b, c] = tri.tris[t];
            let back: HashMap<u32, u32> = index.iter().map(|(&k, &v)| (v, k)).collect();
            [(a, b), (b, c), (c, a)].map(|(p, q)| own(back[&p], back[&q]))
        };
        let mut edges: Vec<(u32, u32)> = edges_of(x).into_iter().chain(edges_of(y)).collect();
        // Triangulation edges aren't mesh edges; keep the real ones.
        let real: BTreeSet<(u32, u32)> = mesh
            .edges()
            .into_iter()
            .filter(|&(a, b)| a >= input.point_offset && b >= input.point_offset)
            .map(|(a, b)| own(a, b))
            .collect();
        edges.retain(|e| real.contains(e));
        edges.sort_unstable();
        edges.dedup();
        return Err(refuse(
            format!("{} passes through itself", input.name),
            edges,
        ));
    }
    Ok((tri, degenerate.len()))
}

fn c3(p: DVec3) -> robust::Coord3D<f64> {
    robust::Coord3D {
        x: p.x,
        y: p.y,
        z: p.z,
    }
}

pub(crate) fn orient(a: DVec3, b: DVec3, c: DVec3, d: DVec3) -> f64 {
    robust::orient3d(c3(a), c3(b), c3(c), c3(d))
}

/// Does segment pq cross triangle abc (touching counts)? Exact predicates.
fn segment_hits_triangle(p: DVec3, q: DVec3, a: DVec3, b: DVec3, c: DVec3) -> bool {
    let s1 = orient(a, b, c, p);
    let s2 = orient(a, b, c, q);
    if s1 == 0. && s2 == 0. {
        return false; // coplanar: left to the edge checks
    }
    if (s1 > 0. && s2 > 0.) || (s1 < 0. && s2 < 0.) {
        return false;
    }
    let o1 = orient(p, q, a, b);
    let o2 = orient(p, q, b, c);
    let o3 = orient(p, q, c, a);
    (o1 >= 0. && o2 >= 0. && o3 >= 0.) || (o1 <= 0. && o2 <= 0. && o3 <= 0.)
}

/// The first pair of triangles that cross although they share no corner
/// (BOOL-21). Sweep along x over bounding boxes; exact pair test.
fn self_intersection(m: &Tri) -> Option<(usize, usize)> {
    let boxes: Vec<(DVec3, DVec3)> = m
        .tris
        .iter()
        .map(|t| bbox(&t.map(|i| m.pos[i as usize])))
        .collect();
    let mut order: Vec<usize> = (0..m.tris.len()).collect();
    order.sort_by(|&a, &b| boxes[a].0.x.total_cmp(&boxes[b].0.x));
    for (oi, &i) in order.iter().enumerate() {
        for &j in &order[oi + 1..] {
            if boxes[j].0.x > boxes[i].1.x {
                break;
            }
            let (bi, bj) = (boxes[i], boxes[j]);
            if bj.0.y > bi.1.y || bi.0.y > bj.1.y || bj.0.z > bi.1.z || bi.0.z > bj.1.z {
                continue;
            }
            let (ti, tj) = (m.tris[i], m.tris[j]);
            if ti.iter().any(|v| tj.contains(v)) {
                continue;
            }
            let pi = ti.map(|v| m.pos[v as usize]);
            let pj = tj.map(|v| m.pos[v as usize]);
            let hit = (0..3)
                .any(|k| segment_hits_triangle(pi[k], pi[(k + 1) % 3], pj[0], pj[1], pj[2]))
                || (0..3)
                    .any(|k| segment_hits_triangle(pj[k], pj[(k + 1) % 3], pi[0], pi[1], pi[2]));
            if hit {
                return Some((i, j));
            }
        }
    }
    None
}

// ── Engines ─────────────────────────────────────────────────────────────

/// The engine operations; Add is built from Union and Intersect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EngineOp {
    Union,
    Intersect,
    Subtract,
}

/// Run ids are the combined polygon index plus this, so every result
/// triangle names its source polygon. Manifold hands out its own ids
/// counting up from zero, far below this.
const RUN_BASE: u32 = 1 << 30;

fn manifold_solid(m: &Tri) -> Result<manifold_csg::Manifold> {
    let pos: Vec<f64> = m.pos.iter().flat_map(|p| p.to_array()).collect();
    let idx: Vec<u64> = m.tris.iter().flatten().map(|&i| i as u64).collect();
    // One run per source polygon (its triangles are consecutive).
    let mut starts = Vec::new();
    let mut ids = Vec::new();
    for (t, &o) in m.owner.iter().enumerate() {
        if t == 0 || m.owner[t - 1] != o {
            starts.push(3 * t as u64);
            ids.push(RUN_BASE + o);
        }
    }
    starts.push(idx.len() as u64);
    let gl = manifold_csg::MeshGL64::new_with_options(
        &pos,
        3,
        &idx,
        manifold_csg::MeshGL64Options::new().runs(&starts, &ids),
    )
    .map_err(|e| format!("the engine refused the input: {e}"))?;
    manifold_csg::Manifold::from_meshgl64(&gl)
        .map_err(|e| format!("the engine refused the input: {e}"))
}

/// Manifold: `target op (union of drivers)`. Every output triangle keeps
/// its source polygon (refused if the provenance doesn't add up).
pub(crate) fn manifold_run(target: &Tri, drivers: &[Tri], op: EngineOp) -> Result<Tri> {
    let a = manifold_solid(target)?;
    let solids = drivers
        .iter()
        .map(manifold_solid)
        .collect::<Result<Vec<_>>>()?;
    let b = if solids.len() == 1 {
        solids.into_iter().next().unwrap()
    } else {
        manifold_csg::Manifold::batch_union(&solids)
    };
    let out = match op {
        EngineOp::Union => a.union(&b),
        EngineOp::Intersect => a.intersection(&b),
        EngineOp::Subtract => a.difference(&b),
    };
    out.status()
        .map_err(|e| format!("the engine failed: {e}"))?;
    let gl = out.to_meshgl64();
    let props = gl.num_prop();
    let values = gl.vert_properties();
    let verts = gl.tri_verts();
    let starts = gl.run_index();
    let ids = gl.run_original_id();
    let mut tri = Tri {
        pos: values
            .chunks(props)
            .map(|c| DVec3::new(c[0], c[1], c[2]))
            .collect(),
        ..Default::default()
    };
    for (r, &id) in ids.iter().enumerate() {
        let (from, to) = (starts[r] as usize, starts[r + 1] as usize);
        if id < RUN_BASE {
            return Err("the engine lost track of where a result polygon came from".into());
        }
        for t in verts[from..to].chunks(3) {
            tri.tris.push([t[0] as u32, t[1] as u32, t[2] as u32]);
            tri.owner.push(id - RUN_BASE);
        }
    }
    if tri.tris.len() * 3 != verts.len() {
        return Err("the engine lost track of where a result polygon came from".into());
    }
    Ok(tri)
}

/// boolmesh, the second opinion: just the result's triangles.
fn boolmesh_run(target: &Tri, drivers: &[Tri], op: EngineOp) -> Result<Tri> {
    use boolmesh::prelude::*;
    let load = |m: &Tri| {
        let pos: Vec<f64> = m.pos.iter().flat_map(|p| p.to_array()).collect();
        let idx: Vec<usize> = m.tris.iter().flatten().map(|&i| i as usize).collect();
        Manifold::new(&pos, &idx).map_err(|e| format!("the check engine refused the input: {e}"))
    };
    let empty = |e: &str| e == "empty pos matrix" || e == "empty idx matrix";
    let to_tri = |out: &Manifold| Tri {
        pos: out.ps.iter().map(|p| DVec3::new(p.x, p.y, p.z)).collect(),
        tris: out
            .hs
            .chunks(3)
            .map(|h| [h[0].tail as u32, h[1].tail as u32, h[2].tail as u32])
            .collect(),
        owner: Vec::new(),
    };
    // Drivers union first, as Manifold does.
    let mut b = to_tri(&load(&drivers[0])?);
    for d in &drivers[1..] {
        b = match compute_boolean(&load(&b)?, &load(d)?, OpType::Add) {
            Ok(out) => to_tri(&out),
            Err(e) => return Err(format!("the check engine failed: {e}")),
        };
    }
    let kind = match op {
        EngineOp::Union => OpType::Add,
        EngineOp::Intersect => OpType::Intersect,
        EngineOp::Subtract => OpType::Subtract,
    };
    match compute_boolean(&load(target)?, &load(&b)?, kind) {
        Ok(out) => Ok(to_tri(&out)),
        // boolmesh reports an empty result (A - A, disjoint Intersect) as
        // an error; that is an empty mesh.
        Err(e) if empty(&e) => Ok(Tri::default()),
        Err(e) => Err(format!("the check engine failed: {e}")),
    }
}

/// Run an engine, turning a panic into an error (BOOL-22: never crash).
pub(crate) fn guarded(run: impl FnOnce() -> Result<Tri>) -> Result<Tri> {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(r) => r,
        Err(_) => Err("an engine stopped unexpectedly".into()),
    }
}

/// Closed (every directed edge has its reverse once), finite, indices in
/// range. Shared by the result check.
fn closed(m: &Tri) -> bool {
    let n = m.pos.len() as u32;
    if m.tris.iter().flatten().any(|&i| i >= n) || m.pos.iter().any(|p| !p.is_finite()) {
        return false;
    }
    let mut uses: HashMap<(u32, u32), u32> = HashMap::with_capacity(m.tris.len() * 3);
    for t in &m.tris {
        for k in 0..3 {
            *uses.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    uses.iter()
        .all(|(&(a, b), &count)| count == 1 && uses.get(&(b, a)) == Some(&1))
}

/// Step 4: the cheap check on Manifold's result. `va`/`vb` are the input
/// volumes (vb the drivers' union).
fn check_result(out: &Tri, op: EngineOp, va: f64, vb: f64) -> Result<()> {
    if !closed(out) {
        return Err("the result wasn't a closed solid".into());
    }
    let v = out.volume();
    let tol = 1e-6 * (va.abs() + vb.abs()) + 1e-300;
    let (lo, hi) = match op {
        EngineOp::Union => (va.max(vb), va + vb),
        EngineOp::Intersect => (0., va.min(vb)),
        EngineOp::Subtract => ((va - vb).max(0.), va),
    };
    if v < lo - tol || v > hi + tol {
        return Err("the result's volume was impossible for this operation".into());
    }
    Ok(())
}

/// Step 5: do the two engines agree? Same volume and the same extent, to
/// a millionth of the inputs' size.
fn engines_agree(a: &Tri, b: &Tri, va: f64, vb: f64, size: f64) -> bool {
    let tol = 1e-6 * (va.abs() + vb.abs()) + 1e-300;
    if (a.volume() - b.volume()).abs() > tol {
        return false;
    }
    match (a.tris.is_empty(), b.tris.is_empty()) {
        (true, true) => true,
        (false, false) => {
            let used = |m: &Tri| -> Vec<DVec3> {
                let mut seen = vec![false; m.pos.len()];
                for &i in m.tris.iter().flatten() {
                    seen[i as usize] = true;
                }
                m.pos
                    .iter()
                    .zip(seen)
                    .filter(|(_, s)| *s)
                    .map(|(p, _)| *p)
                    .collect()
            };
            let (la, ha) = bbox(&used(a));
            let (lb, hb) = bbox(&used(b));
            let t = 1e-6 * size + 1e-300;
            (la - lb).abs().max_element() <= t && (ha - hb).abs().max_element() <= t
        }
        _ => false,
    }
}

/// Steps 4–5 for one engine operation: Manifold's checked result, or why
/// nothing should change.
fn run_checked(target: &Tri, drivers: &[Tri], op: EngineOp, vb: f64, size: f64) -> Result<Tri> {
    let va = target.volume();
    let primary = guarded(|| manifold_run(target, drivers, op))?;
    check_result(&primary, op, va, vb)?;
    let second = guarded(|| boolmesh_run(target, drivers, op))?;
    if !engines_agree(&primary, &second, va, vb, size) {
        return Err(
            "the two geometry engines got different results here, so it can't be trusted".into(),
        );
    }
    Ok(primary)
}

// ── Rebuilding polygons (BOOL-09/12/13) ─────────────────────────────────

/// Points of the result, shared across Add's two engine results: source
/// points by their combined index, new points welded by position.
struct Points {
    base: usize,
    new: Vec<DVec3>,
    grid: HashMap<[i64; 3], Vec<u32>>,
    cell: f64,
}

impl Points {
    fn new(base: usize, cell: f64) -> Self {
        Self {
            base,
            new: Vec::new(),
            grid: HashMap::new(),
            cell,
        }
    }
    fn cell_of(&self, p: DVec3) -> [i64; 3] {
        (p / self.cell).floor().as_i64vec3().to_array()
    }
    /// The new point at `p` (within one cell), made if missing.
    fn at(&mut self, p: DVec3) -> u32 {
        let c = self.cell_of(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if let Some(list) = self.grid.get(&[c[0] + dx, c[1] + dy, c[2] + dz]) {
                        for &i in list {
                            if self.new[i as usize - self.base].distance(p) <= self.cell {
                                return i;
                            }
                        }
                    }
                }
            }
        }
        let i = (self.base + self.new.len()) as u32;
        self.new.push(p);
        self.grid.entry(c).or_default().push(i);
        i
    }
}

/// Split the triangles of one source polygon into disk-shaped regions (a
/// region never wraps around a hole or touches itself at a point), and
/// return each region's boundary loop in winding order.
fn regions(tris: &[[u32; 3]]) -> Vec<Vec<u32>> {
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        for k in 0..3 {
            by_edge
                .entry(edge_key(t[k], t[(k + 1) % 3]))
                .or_default()
                .push(i);
        }
    }
    let mut region = vec![usize::MAX; tris.len()];
    let mut loops = Vec::new();
    for seed in 0..tris.len() {
        if region[seed] != usize::MAX {
            continue;
        }
        let r = loops.len();
        region[seed] = r;
        let mut members = vec![seed];
        let mut points: HashSet<u32> = tris[seed].into_iter().collect();
        let mut frontier = vec![seed];
        while let Some(t) = frontier.pop() {
            for k in 0..3 {
                let e = edge_key(tris[t][k], tris[t][(k + 1) % 3]);
                for &n in &by_edge[&e] {
                    if region[n] != usize::MAX {
                        continue;
                    }
                    // How many of n's edges already border the region.
                    let shared = (0..3)
                        .filter(|&j| {
                            by_edge[&edge_key(tris[n][j], tris[n][(j + 1) % 3])]
                                .iter()
                                .any(|&m| m != n && region[m] == r)
                        })
                        .count();
                    let fits = match shared {
                        // A new corner: only if it isn't on the region yet.
                        1 => tris[n].iter().filter(|v| points.contains(v)).count() == 2,
                        // Fills a notch.
                        2 => true,
                        _ => false,
                    };
                    if fits {
                        region[n] = r;
                        members.push(n);
                        points.extend(tris[n]);
                        frontier.push(n);
                    }
                }
            }
        }
        // Boundary: directed edges whose reverse isn't in the region.
        let inside: HashSet<(u32, u32)> = members
            .iter()
            .flat_map(|&t| (0..3).map(move |k| (tris[t][k], tris[t][(k + 1) % 3])))
            .collect();
        let next: HashMap<u32, u32> = inside
            .iter()
            .filter(|(a, b)| !inside.contains(&(*b, *a)))
            .map(|&(a, b)| (a, b))
            .collect();
        let start = *next.keys().min().unwrap();
        let mut ring = vec![start];
        let mut at = next[&start];
        while at != start && ring.len() <= next.len() {
            ring.push(at);
            at = next[&at];
        }
        loops.push(ring);
    }
    loops
}

/// Blend weights of `p` over polygon `face`'s corners: barycentric in the
/// polygon's triangle that holds it (nearest when it is just outside).
pub(crate) fn corner_blend(mesh: &Mesh, face: usize, p: DVec3) -> CornerBlend {
    let v = &mesh.faces[face].vertices;
    let points: Vec<DVec3> = v.iter().map(|&i| mesh.vertices[i as usize]).collect();
    let triangles = if v.len() == 3 {
        vec![[0, 1, 2]]
    } else {
        crate::polygon_triple::triangulate_polygon(&points, |_, _| false)
            .unwrap_or_else(|_| (1..v.len() - 1).map(|i| [0, i, i + 1]).collect())
    };
    let mut best: Option<(f64, [usize; 3], DVec3)> = None;
    for t in triangles {
        let [a, b, c] = t.map(|i| points[i]);
        let n = (b - a).cross(c - a);
        let area = n.length_squared();
        if area == 0. {
            continue;
        }
        let w = DVec3::new(
            (c - b).cross(p - b).dot(n),
            (a - c).cross(p - c).dot(n),
            (b - a).cross(p - a).dot(n),
        ) / area;
        let outside = (-w.min_element()).max(0.);
        if best.as_ref().is_none_or(|b| outside < b.0) {
            best = Some((outside, t, w));
        }
    }
    let Some((_, t, w)) = best else {
        return vec![((face, 0), 1.)];
    };
    // Clamp onto the triangle and renormalise so the weights sum to 1.
    let w = w.max(DVec3::ZERO);
    let w = w / w.element_sum().max(1e-300);
    (0..3)
        .filter(|&k| w[k] > 0.)
        .map(|k| ((face, t[k]), w[k]))
        .collect()
}

/// Turn one engine result into polygons of the combined mesh.
fn polygons(
    g: &Gathered,
    out: &Tri,
    shift: DVec3,
    tolerance: f64,
    points: &mut Points,
) -> Vec<OutFace> {
    let mesh = &g.combined;
    let to_local = g.to_world.inverse();
    let local: Vec<DVec3> = out
        .pos
        .iter()
        .map(|&p| to_local.transform_point3(p + shift))
        .collect();
    // Each result point becomes the source point at its place, preferring
    // the target's (lowest index), or one new point.
    let mut source: Vec<Option<u32>> = vec![None; out.pos.len()];
    for (t, tri) in out.tris.iter().enumerate() {
        let f = &mesh.faces[out.owner[t] as usize];
        for &r in tri {
            for &s in &f.vertices {
                if mesh.vertices[s as usize].distance(local[r as usize]) <= tolerance
                    && source[r as usize].is_none_or(|old| s < old)
                {
                    source[r as usize] = Some(s);
                }
            }
        }
    }
    let mut point_of: Vec<Option<u32>> = vec![None; out.pos.len()];
    let mut point = |r: u32, points: &mut Points| -> u32 {
        *point_of[r as usize].get_or_insert_with(|| {
            source[r as usize].unwrap_or_else(|| points.at(local[r as usize]))
        })
    };
    let mut by_owner: Vec<(u32, [u32; 3])> = out
        .owner
        .iter()
        .copied()
        .zip(out.tris.iter().copied())
        .collect();
    by_owner.sort_by_key(|(o, _)| *o);
    let mut faces = Vec::new();
    for group in by_owner.chunk_by(|a, b| a.0 == b.0) {
        let owner = group[0].0 as usize;
        let tris: Vec<[u32; 3]> = group.iter().map(|g| g.1).collect();
        let source_face = &mesh.faces[owner];
        for ring in regions(&tris) {
            let mut vertices: Vec<u32> = ring.iter().map(|&r| point(r, points)).collect();
            vertices.dedup();
            while vertices.len() > 1 && vertices.first() == vertices.last() {
                vertices.pop();
            }
            if vertices.len() < 3 {
                continue;
            }
            // BOOL-09: an untouched polygon keeps its corner order.
            let n = source_face.vertices.len();
            if vertices.len() == n
                && let Some(s) = vertices.iter().position(|&v| v == source_face.vertices[0])
                && (0..n).all(|i| vertices[(s + i) % n] == source_face.vertices[i])
            {
                vertices.clone_from(&source_face.vertices);
            }
            let mut donors = Vec::with_capacity(vertices.len());
            let mut blends = Vec::with_capacity(vertices.len());
            for &v in &vertices {
                let at = if (v as usize) < points.base {
                    mesh.vertices[v as usize]
                } else {
                    points.new[v as usize - points.base]
                };
                let donor = source_face
                    .vertices
                    .iter()
                    .position(|&s| s == v || mesh.vertices[s as usize].distance(at) <= tolerance);
                match donor {
                    Some(k) => {
                        donors.push(Some((owner, k)));
                        blends.push(None);
                    }
                    None => {
                        donors.push(None);
                        blends.push(Some(corner_blend(mesh, owner, at)));
                    }
                }
            }
            faces.push(OutFace {
                template: owner,
                vertices,
                donors,
                blends,
            });
        }
    }
    faces
}

// ── The command ─────────────────────────────────────────────────────────

/// What a successful Boolean wants to tell the user (status line).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BooleanReport {
    pub message: String,
}

impl Editor {
    /// BOOL-06/07: the target and its drivers, in the target's space.
    fn boolean_gather(&self, o: &BooleanOptions, what: &str) -> Result<Gathered> {
        self.drive_gather(o, what, None)
    }

    /// [`Self::boolean_gather`] with another tool's "no driver" message
    /// (Solid and Axis Drill, DRILL-23).
    pub(crate) fn drive_gather(
        &self,
        o: &BooleanOptions,
        what: &str,
        no_driver: Option<&str>,
    ) -> Result<Gathered> {
        let target_id = self.edge_insert_target(what)?;
        let scene = &self.scene;
        let item = scene.item(target_id).ok_or("Select a mesh")?;
        if !item.is_mesh() {
            return Err(format!("Select a mesh item before {what}"));
        }
        if !item.elements.is_empty() {
            return Err(format!(
                "{} has locked or hidden elements; unlock and unhide them before {what}",
                item.name
            ));
        }
        let to_world = scene.world_matrix(target_id)?;
        if to_world.determinant().abs() < 1e-15 {
            return Err(format!(
                "{}'s transform is flat; {what} can't use it",
                item.name
            ));
        }
        let mut combined: Mesh = (*item.mesh).clone();
        let mut target = Input {
            item: target_id,
            name: item.name.clone(),
            faces: (0..combined.faces.len()).collect(),
            point_offset: 0,
        };
        let mut drivers = Vec::new();
        let descendants = |root: u64| -> Vec<u64> {
            let mut out = Vec::new();
            let mut stack = vec![root];
            while let Some(p) = stack.pop() {
                for i in scene.items.iter().filter(|i| i.parent == Some(p)) {
                    if !out.contains(&i.id) && i.id != root {
                        out.push(i.id);
                        stack.push(i.id);
                    }
                }
            }
            out
        };
        let mut chosen: Vec<u64> = Vec::new();
        match o.drive {
            DriveMesh::LastSelected => {
                if scene.selection.mode != Mode::Polygon || scene.selection.faces.is_empty() {
                    return Err(format!(
                        "{what} with Last Selected needs selected polygons: the pieces they belong to cut the rest"
                    ));
                }
                let selected: HashSet<u32> = scene.selection.faces.iter().copied().collect();
                let (cut, kept): (Vec<Vec<usize>>, Vec<Vec<usize>>) =
                    crate::item_merge::pieces(&combined)
                        .into_iter()
                        .partition(|piece| piece.iter().any(|&f| selected.contains(&(f as u32))));
                target.faces = kept.into_iter().flatten().collect();
                target.faces.sort_unstable();
                if target.faces.is_empty() {
                    return Err(format!(
                        "{what}: every piece of {} is selected, so nothing is left to cut",
                        item.name
                    ));
                }
                let mut faces: Vec<usize> = cut.into_iter().flatten().collect();
                faces.sort_unstable();
                drivers.push(Input {
                    item: target_id,
                    name: format!("The selected part of {}", item.name),
                    faces,
                    point_offset: 0,
                });
            }
            DriveMesh::Background => {
                let selected = scene.selected_items();
                chosen = scene
                    .items
                    .iter()
                    .filter(|i| {
                        i.is_mesh()
                            && !selected.contains(&i.id)
                            && scene.drawn_in_viewport(i.id, true)
                    })
                    .map(|i| i.id)
                    .collect();
            }
            DriveMesh::Item => {
                let id = o
                    .item
                    .ok_or_else(|| format!("Choose the Surface Item that drives the {what}"))?;
                let it = scene
                    .item(id)
                    .ok_or_else(|| format!("The {what} Surface Item is gone"))?;
                if it.is_mesh() {
                    chosen.push(id);
                } else {
                    // A group or locator: its meshes drive.
                    chosen.extend(descendants(id));
                }
            }
        }
        if o.children && o.drive != DriveMesh::LastSelected {
            for id in chosen.clone() {
                for d in descendants(id) {
                    if !chosen.contains(&d) {
                        chosen.push(d);
                    }
                }
            }
        }
        let inverse = to_world.inverse();
        for id in chosen {
            let Some(d) = scene.item(id) else {
                continue;
            };
            if id == target_id || !d.is_mesh() || d.mesh.faces.is_empty() {
                continue;
            }
            // BOOL-08: the driver item itself is left alone; a copy of its
            // geometry, without its selection sets, comes along.
            let mut mesh: Mesh = (*d.mesh).clone();
            let to_target = inverse * scene.world_matrix(id)?;
            for v in &mut mesh.vertices {
                *v = to_target.transform_point3(*v);
            }
            let a = &mut mesh.attributes;
            a.vertex_sets.clear();
            a.polygon_sets.clear();
            a.edge_sets.clear();
            a.set_groups.clear();
            let offset = combined.vertices.len() as u32;
            let first = combined.faces.len();
            crate::append_mesh(&mut combined, mesh)?;
            drivers.push(Input {
                item: id,
                name: d.name.clone(),
                faces: (first..combined.faces.len()).collect(),
                point_offset: offset,
            });
        }
        if drivers.is_empty() {
            if let Some(message) = no_driver {
                return Err(message.to_owned());
            }
            return Err(match o.drive {
                DriveMesh::Background => format!(
                    "{what}: no driver. Show the meshes that should cut {}, and leave them unselected",
                    item.name
                ),
                _ => format!("{what}: the chosen Surface Item has no polygons to cut with"),
            });
        }
        Ok(Gathered {
            combined,
            target,
            drivers,
            to_world,
        })
    }

    /// The input Boolean with `o` would refuse now (BOOL-19/21), for the
    /// refusal's "Select Problem Edges". None when every input passes or
    /// the refusal isn't about one input's geometry.
    pub fn boolean_problem(&self, o: &BooleanOptions) -> Option<BooleanProblem> {
        let g = self.boolean_gather(o, "Boolean").ok()?;
        std::iter::once(&g.target)
            .chain(&g.drivers)
            .find_map(|input| preflight(&g, input).err())
    }

    /// BOOL-01..22: run the boolean and return the target's new mesh and
    /// what to tell the user. Changes nothing in the scene.
    pub(crate) fn boolean_result(&self, o: &BooleanOptions) -> Result<(u64, Mesh, String)> {
        let what = format!("Boolean {}", o.op.label());
        let g = self.boolean_gather(o, &what)?;
        let (mesh, message) = boolean_gathered(&g, o.op, &what)?;
        Ok((g.target.item, mesh, message))
    }

    /// `poly.boolean` (BOOL-24/25): one undo step "Boolean <op>". The
    /// drivers are untouched; the target's polygons are replaced and
    /// nothing is left selected (BOOL-14).
    pub(crate) fn boolean(&mut self, o: BooleanOptions) -> Result<()> {
        let (item, mesh, message) = self.boolean_result(&o)?;
        self.boolean_preset = o.clone();
        self.publish_polygon_edit(item, mesh, &format!("Boolean {}", o.op.label()), Vec::new())?;
        self.boolean_note = Some(message);
        Ok(())
    }
}

/// Steps 2-6 on gathered inputs: the target's new mesh and what to tell
/// the user. The direct tool and the Boolean MeshOp share it (tool
/// families: one engine).
pub(crate) fn boolean_gathered(g: &Gathered, op: BooleanOp, what: &str) -> Result<(Mesh, String)> {
    let o = BooleanOpOnly { op };
    let mut notes = Vec::new();
    let mut dropped = 0;
    let mut check = |input: &Input| match preflight(g, input) {
        Ok((tri, degenerate)) => {
            dropped += degenerate;
            Ok(tri)
        }
        Err(problem) => Err(format!("{what}: {}. Nothing changed.", problem.message)),
    };
    let mut target = check(&g.target)?;
    let mut drivers = g
        .drivers
        .iter()
        .map(&mut check)
        .collect::<Result<Vec<_>>>()?;
    if dropped > 0 {
        notes.push(format!(
            "left out {}",
            plural(dropped, "zero-area polygon", "zero-area polygons")
        ));
    }
    // Step 3: centre the inputs on the origin.
    let all: Vec<DVec3> = target
        .pos
        .iter()
        .chain(drivers.iter().flat_map(|d| &d.pos))
        .copied()
        .collect();
    let centre = bbox_centre(&all);
    let (lo, hi) = bbox(&all);
    let size = (hi - lo).length().max(1e-300);
    for m in std::iter::once(&mut target).chain(drivers.iter_mut()) {
        for p in &mut m.pos {
            *p -= centre;
        }
    }
    // Drivers act as one solid (BOOL-07): its volume bounds the result.
    let vb = if drivers.len() == 1 {
        drivers[0].volume()
    } else {
        guarded(|| manifold_run(&drivers[0], &drivers[1..], EngineOp::Union))
            .map_err(|e| format!("{what}: {e}. Nothing changed."))?
            .volume()
    };
    let engine_ops: &[EngineOp] = match o.op {
        BooleanOp::Union => &[EngineOp::Union],
        BooleanOp::Intersect => &[EngineOp::Intersect],
        BooleanOp::Subtract => &[EngineOp::Subtract],
        // BOOL-04: both whole surfaces, split where they cross = the
        // union's faces plus the intersection's.
        BooleanOp::Add => &[EngineOp::Union, EngineOp::Intersect],
    };
    let mut results = Vec::new();
    for &op in engine_ops {
        results.push(
            run_checked(&target, &drivers, op, vb, size)
                .map_err(|e| format!("{what}: {e}. Nothing changed."))?,
        );
    }
    // Step 6: polygons.
    let to_local_scale = {
        let m = g.to_world.inverse();
        m.x_axis
            .length()
            .max(m.y_axis.length())
            .max(m.z_axis.length())
    };
    let tolerance = 1e-9 * size * to_local_scale.max(1.);
    let mut points = Points::new(g.combined.vertices.len(), tolerance);
    let mut faces = Vec::new();
    for r in &results {
        faces.extend(polygons(g, r, centre, tolerance, &mut points));
    }
    let mut vertices = g.combined.vertices.clone();
    vertices.extend_from_slice(&points.new);
    let rebuilt = rebuild(&g.combined, vertices, faces, &[], what)?;
    let mut mesh = rebuilt;
    mesh.attributes.extrude_edge_uses.clear();
    if o.op == BooleanOp::Add {
        // Deliberately non-manifold along the crossing line (BOOL-04).
        mesh.attributes.extrude_edge_uses = mesh.edge_uses();
    }
    // Points the inputs used and the result doesn't are removed; lone
    // points the target had stay (BOOL-20).
    let was_used: BTreeSet<u32> = g
        .combined
        .used_points()
        .iter()
        .enumerate()
        .filter(|(_, u)| **u)
        .map(|(v, _)| v as u32)
        .chain(g.combined.vertices.len() as u32..mesh.vertices.len() as u32)
        .collect();
    let mesh = crate::mesh_cleanup::remove_unreferenced(&mesh, Some(&was_used))?.mesh;
    mesh.validate()
        .map_err(|e| format!("{what}: the result couldn't be rebuilt ({e}). Nothing changed."))?;
    // A last look at the polygons: closed again (Add: two or four
    // polygons on every edge).
    let mut uses: HashMap<(u32, u32), usize> = HashMap::new();
    for f in &mesh.faces {
        for i in 0..f.vertices.len() {
            *uses
                .entry(edge_key(
                    f.vertices[i],
                    f.vertices[(i + 1) % f.vertices.len()],
                ))
                .or_default() += 1;
        }
    }
    let fine = |n: usize| n == 2 || (o.op == BooleanOp::Add && n == 4);
    if !uses.values().all(|&n| fine(n)) {
        return Err(format!(
            "{what}: the rebuilt polygons didn't close up, so nothing changed"
        ));
    }
    if mesh.faces.is_empty() {
        notes.push(match o.op {
            BooleanOp::Intersect => "the meshes don't overlap, so the result is empty".into(),
            _ => "the result is empty".into(),
        });
    }
    let message = if notes.is_empty() {
        format!(
            "{what}: {}",
            plural(mesh.faces.len(), "polygon", "polygons")
        )
    } else {
        format!("{what}: {}", notes.join("; "))
    };
    Ok((mesh, message))
}
/// The one option [`boolean_gathered`] reads, named like the dialog's so
/// the shared code reads the same.
struct BooleanOpOnly {
    op: BooleanOp,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(lo: f64, hi: f64) -> Mesh {
        let c = (lo + hi) * 0.5;
        Mesh::box_mesh(DVec3::splat(c), DVec3::splat(hi - lo), 0).unwrap()
    }

    fn tri_of(m: &Mesh) -> Tri {
        let mut t = Tri {
            pos: m.vertices.clone(),
            ..Default::default()
        };
        for (fi, f) in m.faces.iter().enumerate() {
            for i in 1..f.vertices.len() - 1 {
                t.tris
                    .push([f.vertices[0], f.vertices[i], f.vertices[i + 1]]);
                t.owner.push(fi as u32);
            }
        }
        t
    }

    #[test]
    fn engines_agree_on_the_reference_cubes() {
        let a = tri_of(&cube(-1., 1.));
        let b = tri_of(&cube(0., 2.));
        for (op, v) in [
            (EngineOp::Union, 15.),
            (EngineOp::Intersect, 1.),
            (EngineOp::Subtract, 7.),
        ] {
            let out = run_checked(&a, std::slice::from_ref(&b), op, 8., 3.).unwrap();
            assert!((out.volume() - v).abs() < 1e-9, "{op:?}");
        }
    }

    #[test]
    fn disagreement_is_caught() {
        let a = tri_of(&cube(-1., 1.));
        let b = tri_of(&cube(0., 2.));
        assert!(!engines_agree(&a, &b, 8., 8., 3.));
        assert!(engines_agree(&a, &a.clone(), 8., 8., 3.));
        assert!(!engines_agree(&a, &Tri::default(), 8., 8., 3.));
    }

    #[test]
    fn self_intersection_found() {
        let mut m = tri_of(&cube(-1., 1.));
        assert!(self_intersection(&m).is_none());
        let b = tri_of(&cube(0., 2.));
        let off = m.pos.len() as u32;
        m.pos.extend(b.pos);
        m.tris.extend(b.tris.iter().map(|t| t.map(|i| i + off)));
        assert!(self_intersection(&m).is_some());
    }

    #[test]
    fn modes_parse_from_numbers_and_words() {
        assert_eq!(BooleanOp::parse("2").unwrap(), BooleanOp::Subtract);
        assert_eq!(BooleanOp::parse("add").unwrap(), BooleanOp::Add);
        assert!(BooleanOp::parse("4").is_err());
        assert_eq!(
            DriveMesh::parse("lastSelected").unwrap(),
            DriveMesh::LastSelected
        );
        assert_eq!(DriveMesh::parse("2").unwrap(), DriveMesh::Item);
        assert!(DriveMesh::parse("fish").is_err());
    }
}
