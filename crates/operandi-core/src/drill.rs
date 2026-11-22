//! Solid Drill and Axis Drill (specifications/drills-2026-11-11, DRILL-01..27):
//! cut the target's own polygons with a driver and keep the inside pieces
//! (Core), the outside ones (Tunnel), or all of them (Stencil, with a
//! material on the inside pieces, and Slice). No driver faces are ever
//! added, so open and multi-piece targets are fine.
//!
//! Axis Drill is Solid Drill with the outline's prism as the driver
//! (DRILL-14): each driver polygon's shadow along the axis is extruded far
//! past the target both ways, and several shadows are unioned into one
//! solid, so one cutting core serves both tools.
//!
//! The cutting core, per run:
//!
//! 1. Every target edge is cut where it meets the driver surface. These
//!    points are made once per edge, so both polygons on an edge share them
//!    (DRILL-17); a cut within a hair of an existing point uses that point.
//! 2. Every target polygon near the driver is split in its own plane: the
//!    driver's crossing segments (exact-sign triangle–triangle tests against
//!    the polygon's triangles) and the polygon's boundary form a planar
//!    graph, whose faces are the pieces (DRILL-16). A piece with a hole is
//!    joined to its outer edge by two bridges, giving two hole-free
//!    polygons (Q1 default).
//! 3. Each piece is inside or outside (DRILL-15). A piece lying on the
//!    driver surface counts as inside (Q6). Otherwise two independent
//!    tests decide (DRILL-20): the generalized winding number, and a ray
//!    crossing count with exact predicates (Solid Drill) or a 2D
//!    point-in-outline test (Axis Drill). Uncut polygons that touch across
//!    uncut edges share one answer.
//! 4. Checks (DRILL-19/21): each cut polygon's pieces cover exactly its
//!    area, the open-edge length is unchanged (no gaps or T-junctions) and
//!    the rebuilt mesh validates. Any failure, or a panic, changes nothing.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md DRILL-Q*): no named result
//! sets (DRILL-22), no progress bar or Esc cancel (DRILL-09), per-point maps
//! on new points are not interpolated (as Boolean), and the procedural
//! versions (DRILL-30) are a later stage.
use crate::boolean::{
    BooleanOptions, BooleanProblem, DriveMesh, EngineOp, Gathered, Tri, bbox, corner_blend,
    guarded, manifold_run, orient, plural, preflight,
};
use crate::edge_commands::{OutFace, rebuild};
use crate::topology::edge_key;
use crate::{Editor, Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

// ── Options ─────────────────────────────────────────────────────────────

/// DRILL-10..13: the Operation, in the command's numeric order (Q2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrillOp {
    /// Q4 default.
    #[default]
    Core,
    Tunnel,
    Stencil,
    Slice,
}

impl DrillOp {
    /// `mode:<int>` order: 0 Core, 1 Tunnel, 2 Stencil, 3 Slice.
    pub const ALL: [DrillOp; 4] = [Self::Core, Self::Tunnel, Self::Stencil, Self::Slice];
    /// DRILL-07: the dialog lists the documentation's order.
    pub const DIALOG: [DrillOp; 4] = [Self::Core, Self::Stencil, Self::Tunnel, Self::Slice];
    pub fn label(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Tunnel => "Tunnel",
            Self::Stencil => "Stencil",
            Self::Slice => "Slice",
        }
    }
    /// `mode:<int>` or a readable word (core, tunnel, stencil, slice).
    pub fn parse(text: &str) -> Result<Self> {
        let t = text.trim();
        if let Ok(i) = t.parse::<usize>() {
            return Self::ALL
                .get(i)
                .copied()
                .ok_or_else(|| format!("Drill mode must be 0 to 3, not {i}"));
        }
        Self::ALL
            .into_iter()
            .find(|o| o.label().eq_ignore_ascii_case(t))
            .ok_or_else(|| format!("Unknown drill mode: {t}"))
    }
}

/// Axis Drill's direction of projection (Q3: Y by default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrillAxis {
    X,
    #[default]
    Y,
    Z,
}

impl DrillAxis {
    pub const ALL: [DrillAxis; 3] = [Self::X, Self::Y, Self::Z];
    pub fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
        }
    }
    pub fn vector(self) -> DVec3 {
        match self {
            Self::X => DVec3::X,
            Self::Y => DVec3::Y,
            Self::Z => DVec3::Z,
        }
    }
    /// `axis:<int>` (0 X, 1 Y, 2 Z) or x, y, z.
    pub fn parse(text: &str) -> Result<Self> {
        let t = text.trim();
        if let Ok(i) = t.parse::<usize>() {
            return Self::ALL
                .get(i)
                .copied()
                .ok_or_else(|| format!("Drill axis must be 0 to 2, not {i}"));
        }
        Self::ALL
            .into_iter()
            .find(|a| a.label().eq_ignore_ascii_case(t))
            .ok_or_else(|| format!("Unknown drill axis: {t}"))
    }
}

/// Which of the two tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrillKind {
    Solid,
    Axis,
}

impl DrillKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Solid Drill",
            Self::Axis => "Axis Drill",
        }
    }
    /// The script command (DRILL-06).
    pub fn command(self) -> &'static str {
        match self {
            Self::Solid => "poly.solidDrill",
            Self::Axis => "poly.drill",
        }
    }
}

/// Q5: the Material field starts as this, and an empty one uses it.
pub const STENCIL_MATERIAL: &str = "Stencil";

/// The dialogs' and commands' options (DRILL-05/06). `axis` is used by
/// Axis Drill only, `material` by Stencil only.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DrillOptions {
    pub op: DrillOp,
    pub axis: DrillAxis,
    pub material: String,
    pub drive: DriveMesh,
    /// The driver item for [`DriveMesh::Item`].
    pub item: Option<u64>,
    pub children: bool,
}

impl Default for DrillOptions {
    fn default() -> Self {
        Self {
            op: DrillOp::default(),
            axis: DrillAxis::default(),
            material: STENCIL_MATERIAL.into(),
            drive: DriveMesh::default(),
            item: None,
            children: false,
        }
    }
}

impl DrillOptions {
    fn drive_options(&self) -> BooleanOptions {
        BooleanOptions {
            drive: self.drive,
            item: self.item,
            children: self.children,
            ..Default::default()
        }
    }
    /// The material Stencil gives its inside pieces (Q5).
    pub fn stencil_material(&self) -> &str {
        let m = self.material.trim();
        if m.is_empty() { STENCIL_MATERIAL } else { m }
    }
}

const NO_DRIVER: &str = "No driver: show a background mesh or pick a Surface Item.";

// ── The cutter ──────────────────────────────────────────────────────────

/// A uniform grid over the driver's triangles, for "which triangles are
/// near this box" questions.
struct Grid {
    lo: DVec3,
    cell: DVec3,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
    boxes: Vec<(DVec3, DVec3)>,
}

impl Grid {
    fn new(pos: &[DVec3], tris: &[[u32; 3]]) -> Self {
        let boxes: Vec<(DVec3, DVec3)> = tris
            .iter()
            .map(|t| bbox(&t.map(|i| pos[i as usize])))
            .collect();
        let (lo, hi) = bbox(pos);
        let span = (hi - lo).max(DVec3::splat(1e-300));
        let per = ((tris.len() as f64).cbrt() * 2.).clamp(1., 64.);
        let longest = span.max_element();
        let dims = [0, 1, 2].map(|k| ((span[k] / longest * per).ceil() as usize).clamp(1, 64));
        let cell = span / DVec3::new(dims[0] as f64, dims[1] as f64, dims[2] as f64);
        let mut grid = Self {
            lo,
            cell,
            dims,
            cells: vec![Vec::new(); dims[0] * dims[1] * dims[2]],
            boxes,
        };
        for t in 0..tris.len() {
            let (a, b) = grid.range(grid.boxes[t].0, grid.boxes[t].1);
            for x in a[0]..=b[0] {
                for y in a[1]..=b[1] {
                    for z in a[2]..=b[2] {
                        let i = grid.index(x, y, z);
                        grid.cells[i].push(t as u32);
                    }
                }
            }
        }
        grid
    }
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (x * self.dims[1] + y) * self.dims[2] + z
    }
    fn range(&self, lo: DVec3, hi: DVec3) -> ([usize; 3], [usize; 3]) {
        let at = |p: DVec3, k: usize| -> usize {
            let c = ((p[k] - self.lo[k]) / self.cell[k]).floor();
            (c.max(0.) as usize).min(self.dims[k] - 1)
        };
        ([0, 1, 2].map(|k| at(lo, k)), [0, 1, 2].map(|k| at(hi, k)))
    }
    /// Triangles whose box meets `lo..hi` (grown by `pad`).
    fn near(&self, lo: DVec3, hi: DVec3, pad: f64) -> Vec<u32> {
        let (lo, hi) = (lo - DVec3::splat(pad), hi + DVec3::splat(pad));
        let (a, b) = self.range(lo, hi);
        let mut out = BTreeSet::new();
        for x in a[0]..=b[0] {
            for y in a[1]..=b[1] {
                for z in a[2]..=b[2] {
                    for &t in &self.cells[self.index(x, y, z)] {
                        let (tl, th) = self.boxes[t as usize];
                        if (tl.cmple(hi) & th.cmpge(lo)).all() {
                            out.insert(t);
                        }
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

/// Axis Drill's second inside test: the driver polygons' shadows on the
/// plane at right angles to the axis.
struct Shadow {
    e1: DVec3,
    e2: DVec3,
    polygons: Vec<(Vec<DVec2>, DVec2, DVec2)>,
}

impl Shadow {
    fn flat(&self, p: DVec3) -> DVec2 {
        DVec2::new(p.dot(self.e1), p.dot(self.e2))
    }
    fn contains(&self, p: DVec3) -> bool {
        let q = self.flat(p);
        self.polygons
            .iter()
            .any(|(poly, lo, hi)| q.cmpge(*lo).all() && q.cmple(*hi).all() && inside_2d(poly, q))
    }
}

/// A closed driver surface in work space (the target's local space, moved
/// so the run is centred on the origin).
struct Cutter {
    pos: Vec<DVec3>,
    tris: Vec<[u32; 3]>,
    grid: Grid,
    shadow: Option<Shadow>,
    /// Distance under which a point is on the surface (Q6).
    on: f64,
    /// A length beyond everything in the run.
    far: f64,
}

impl Cutter {
    fn new(pos: Vec<DVec3>, tris: Vec<[u32; 3]>, shadow: Option<Shadow>, size: f64) -> Self {
        let grid = Grid::new(&pos, &tris);
        Self {
            pos,
            tris,
            grid,
            shadow,
            on: 1e-7 * size,
            far: 16. * size,
        }
    }
    fn tri(&self, t: u32) -> [DVec3; 3] {
        self.tris[t as usize].map(|i| self.pos[i as usize])
    }

    /// Q6: is `p` on the driver surface?
    fn on_surface(&self, p: DVec3) -> bool {
        self.grid
            .near(p, p, self.on)
            .into_iter()
            .any(|t| point_triangle_distance(p, self.tri(t)) <= self.on)
    }

    /// Test 1: the generalized winding number (|w| > ½ is inside).
    fn winding_inside(&self, p: DVec3) -> bool {
        let mut total = 0.;
        for t in 0..self.tris.len() as u32 {
            let [a, b, c] = self.tri(t).map(|v| v - p);
            let (la, lb, lc) = (a.length(), b.length(), c.length());
            let num = a.dot(b.cross(c));
            let den = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
            total += 2. * num.atan2(den);
        }
        (total / (4. * std::f64::consts::PI)).abs() > 0.5
    }

    /// Test 2 for Solid Drill: crossings of a ray from `p`, exact signs. A
    /// ray grazing an edge or corner is retried in another direction.
    fn ray_inside(&self, p: DVec3) -> Result<bool> {
        const DIRECTIONS: [[f64; 3]; 6] = [
            [0.5773, 0.6123, 0.5401],
            [-0.3124, 0.8812, 0.3547],
            [0.7072, -0.1213, -0.6966],
            [-0.6211, -0.5523, 0.5561],
            [0.1093, -0.9134, -0.3920],
            [0.8340, 0.3110, -0.4557],
        ];
        'direction: for d in DIRECTIONS {
            let q = p + DVec3::from_array(d).normalize() * self.far;
            let mut crossings = 0;
            for (t, &(lo, hi)) in self.grid.boxes.iter().enumerate() {
                let (sl, sh) = (p.min(q), p.max(q));
                if (hi.cmplt(sl) | lo.cmpgt(sh)).any() {
                    continue;
                }
                let [a, b, c] = self.tri(t as u32);
                let s1 = orient(a, b, c, p);
                let s2 = orient(a, b, c, q);
                if (s1 > 0. && s2 > 0.) || (s1 < 0. && s2 < 0.) {
                    continue;
                }
                if s1 == 0. || s2 == 0. {
                    continue 'direction;
                }
                let o = [orient(p, q, a, b), orient(p, q, b, c), orient(p, q, c, a)];
                if o.iter().all(|&x| x > 0.) || o.iter().all(|&x| x < 0.) {
                    crossings += 1;
                } else if o.contains(&0.)
                    && (o.iter().all(|&x| x >= 0.) || o.iter().all(|&x| x <= 0.))
                {
                    continue 'direction;
                }
            }
            return Ok(crossings % 2 == 1);
        }
        Err("the inside test couldn't find a clear direction".into())
    }

    /// DRILL-15/20: inside or outside, from two independent tests.
    fn inside(&self, p: DVec3) -> Result<bool> {
        if self.on_surface(p) {
            return Ok(true);
        }
        let first = self.winding_inside(p);
        let second = match &self.shadow {
            Some(shadow) => shadow.contains(p),
            None => self.ray_inside(p)?,
        };
        if first != second {
            return Err("the two inside tests disagreed about a piece".into());
        }
        Ok(first)
    }
}

fn point_triangle_distance(p: DVec3, [a, b, c]: [DVec3; 3]) -> f64 {
    // Closest point on a triangle (Ericson, Real-Time Collision Detection).
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0. && d2 <= 0. {
        return ap.length();
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0. && d4 <= d3 {
        return bp.length();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let v = d1 / (d1 - d3);
        return (p - (a + ab * v)).length();
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0. && d5 <= d6 {
        return cp.length();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let w = d2 / (d2 - d6);
        return (p - (a + ac * w)).length();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (p - (b + (c - b) * w)).length();
    }
    let denom = 1. / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    (p - (a + ab * v + ac * w)).length()
}

fn cross2(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn area_2d(poly: &[DVec2]) -> f64 {
    (0..poly.len())
        .map(|i| cross2(poly[i], poly[(i + 1) % poly.len()]))
        .sum::<f64>()
        * 0.5
}

/// Crossing-number point-in-polygon.
fn inside_2d(poly: &[DVec2], q: DVec2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > q.y) != (b.y > q.y) && q.x < (b.x - a.x) * (q.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

fn newell(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        n += a.cross(b);
    }
    n * 0.5
}

// ── Triangle–triangle crossing ──────────────────────────────────────────

/// Where `tri` meets the plane of `plane` (exact signs). Two points for an
/// edge or a crossing; fewer for a touch; None when `tri` lies in it.
fn plane_points(tri: [DVec3; 3], plane: [DVec3; 3]) -> Option<Vec<DVec3>> {
    let s = tri.map(|p| orient(plane[0], plane[1], plane[2], p));
    if s.iter().all(|&x| x == 0.) {
        return None;
    }
    if s.iter().all(|&x| x > 0.) || s.iter().all(|&x| x < 0.) {
        return Some(Vec::new());
    }
    let n = (plane[1] - plane[0]).cross(plane[2] - plane[0]);
    let d = tri.map(|p| n.dot(p - plane[0]));
    let mut out = Vec::new();
    for i in 0..3 {
        if s[i] == 0. {
            out.push(tri[i]);
        }
    }
    for i in 0..3 {
        let j = (i + 1) % 3;
        if (s[i] > 0. && s[j] < 0.) || (s[i] < 0. && s[j] > 0.) {
            let t = (d[i] / (d[i] - d[j])).clamp(0., 1.);
            out.push(tri[i] + (tri[j] - tri[i]) * t);
        }
    }
    Some(out)
}

/// The segment where target triangle `a` meets driver triangle `t`, or
/// None (apart, touching at a point, or coplanar: a coplanar driver face's
/// outline comes from its neighbours).
fn tri_tri(a: [DVec3; 3], t: [DVec3; 3]) -> Option<(DVec3, DVec3)> {
    let ta = plane_points(t, a)?;
    let at = plane_points(a, t)?;
    if ta.len() < 2 || at.len() < 2 {
        return None;
    }
    let line = (a[1] - a[0])
        .cross(a[2] - a[0])
        .cross((t[1] - t[0]).cross(t[2] - t[0]));
    if line.length_squared() == 0. {
        return None;
    }
    let span = |pts: &[DVec3]| {
        let mut lo = (f64::INFINITY, pts[0]);
        let mut hi = (f64::NEG_INFINITY, pts[0]);
        for &p in pts {
            let s = p.dot(line);
            if s < lo.0 {
                lo = (s, p);
            }
            if s > hi.0 {
                hi = (s, p);
            }
        }
        (lo, hi)
    };
    let (l1, h1) = span(&ta);
    let (l2, h2) = span(&at);
    let lo = if l1.0 > l2.0 { l1 } else { l2 };
    let hi = if h1.0 < h2.0 { h1 } else { h2 };
    if hi.0 - lo.0 <= 1e-12 * line.length() * (hi.1 - lo.1).length().max(1e-300) || hi.0 <= lo.0 {
        return None;
    }
    Some((lo.1, hi.1))
}

/// Points where segment pq meets driver triangle t, as parameters along
/// pq (a crossing, or the ends of an in-plane overlap).
fn edge_hits(p: DVec3, q: DVec3, t: [DVec3; 3]) -> Vec<f64> {
    let sp = orient(t[0], t[1], t[2], p);
    let sq = orient(t[0], t[1], t[2], q);
    if (sp > 0. && sq > 0.) || (sp < 0. && sq < 0.) {
        return Vec::new();
    }
    let n = (t[1] - t[0]).cross(t[2] - t[0]);
    if sp == 0. && sq == 0. {
        // In the triangle's plane: clip pq against its three sides.
        let (mut lo, mut hi) = (0f64, 1f64);
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            let side = n.cross(b - a); // points inward
            let (fp, fq) = (side.dot(p - a), side.dot(q - a));
            if fp < 0. && fq < 0. {
                return Vec::new();
            }
            if fp < 0. || fq < 0. {
                let s = fp / (fp - fq);
                if fp < 0. {
                    lo = lo.max(s);
                } else {
                    hi = hi.min(s);
                }
            }
        }
        if hi < lo {
            return Vec::new();
        }
        return vec![lo, hi];
    }
    let s = if sp == 0. {
        0.
    } else if sq == 0. {
        1.
    } else {
        let (dp, dq) = (n.dot(p - t[0]), n.dot(q - t[0]));
        (dp / (dp - dq)).clamp(0., 1.)
    };
    // Inside the triangle (edges included)?
    let o = [
        orient(p, q, t[0], t[1]),
        orient(p, q, t[1], t[2]),
        orient(p, q, t[2], t[0]),
    ];
    if o.iter().all(|&x| x >= 0.) || o.iter().all(|&x| x <= 0.) {
        vec![s]
    } else {
        Vec::new()
    }
}

// ── Cutting one polygon ─────────────────────────────────────────────────

/// A node of one polygon's planar graph.
#[derive(Clone, Copy)]
struct Node {
    p: DVec2,
    p3: DVec3,
    /// Its mesh point, for boundary nodes (corners and edge cuts).
    id: Option<u32>,
}

/// The pieces of one polygon: node lists, CCW in the polygon's frame.
struct Cut {
    nodes: Vec<Node>,
    pieces: Vec<Vec<usize>>,
}

/// Split one polygon. `boundary` is its outline with edge cuts, CCW;
/// `segments` the driver's crossings in 3D; `lift` maps a 2D point back
/// onto the polygon.
fn cut_polygon(
    boundary: Vec<Node>,
    segments: &[(DVec3, DVec3)],
    to_2d: &dyn Fn(DVec3) -> DVec2,
    lift: &dyn Fn(DVec2) -> DVec3,
    eps: f64,
) -> Result<Cut> {
    let nb = boundary.len();
    let mut nodes = boundary;
    let snap_far = 1e3 * eps;
    let dist_to_edge = |nodes: &[Node], k: usize, q: DVec2| -> (f64, f64) {
        let (a, b) = (nodes[k].p, nodes[(k + 1) % nb].p);
        let ab = b - a;
        let t = ((q - a).dot(ab) / ab.length_squared().max(1e-300)).clamp(0., 1.);
        ((a + ab * t).distance(q), t)
    };
    // Snap a segment end to a node: a boundary node when on the outline
    // (the edge cut made there), else an interior node.
    let node_at = |nodes: &mut Vec<Node>, p3: DVec3| -> Result<usize> {
        let q = to_2d(p3);
        let on_boundary = (0..nb).any(|k| dist_to_edge(nodes, k, q).0 <= eps);
        if on_boundary {
            let (best, d) = (0..nb)
                .map(|k| (k, nodes[k].p.distance(q)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            if d <= snap_far {
                return Ok(best);
            }
            return Err("a cut line met an edge where no cut point was made".into());
        }
        if let Some(k) = (nb..nodes.len()).find(|&k| nodes[k].p.distance(q) <= eps) {
            return Ok(k);
        }
        nodes.push(Node { p: q, p3, id: None });
        Ok(nodes.len() - 1)
    };
    let mut lines: Vec<(usize, usize)> = (0..nb).map(|k| (k, (k + 1) % nb)).collect();
    for &(a, b) in segments {
        let (i, j) = (node_at(&mut nodes, a)?, node_at(&mut nodes, b)?);
        if i != j {
            lines.push((i, j));
        }
    }
    // Crossings between cut lines (and with the outline, which must be at
    // an edge cut already).
    let n_lines = lines.len();
    for i in nb..n_lines {
        for j in 0..i {
            let ((a, b), (c, d)) = (lines[i], lines[j]);
            if a == c || a == d || b == c || b == d {
                continue;
            }
            let (pa, pb, pc, pd) = (nodes[a].p, nodes[b].p, nodes[c].p, nodes[d].p);
            let (r, s) = (pb - pa, pd - pc);
            let d1 = cross2(r, pc - pa);
            let d2 = cross2(r, pd - pa);
            let d3 = cross2(s, pa - pc);
            let d4 = cross2(s, pb - pc);
            let (lr, ls) = (r.length(), s.length());
            let strict = |x: f64, l: f64| x.abs() > eps * l;
            if !(strict(d1, lr) && strict(d2, lr) && strict(d3, ls) && strict(d4, ls)) {
                continue;
            }
            if (d1 > 0.) == (d2 > 0.) || (d3 > 0.) == (d4 > 0.) {
                continue;
            }
            let t = cross2(pc - pa, s) / cross2(r, s);
            let q = pa + r * t;
            if j < nb {
                let (best, dist) = (0..nb)
                    .map(|k| (k, nodes[k].p.distance(q)))
                    .min_by(|x, y| x.1.total_cmp(&y.1))
                    .unwrap();
                if dist > snap_far {
                    return Err("a cut line crossed an edge where no cut point was made".into());
                }
                let _ = best; // the split below joins it through the node-on-line pass
                continue;
            }
            if (nb..nodes.len()).all(|k| nodes[k].p.distance(q) > eps) {
                nodes.push(Node {
                    p: q,
                    p3: lift(q),
                    id: None,
                });
            }
        }
    }
    // Split every line at the nodes on it.
    let mut edges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut on_outline: HashSet<(usize, usize)> = HashSet::new();
    for (li, &(a, b)) in lines.iter().enumerate() {
        let (pa, pb) = (nodes[a].p, nodes[b].p);
        let ab = pb - pa;
        let len2 = ab.length_squared();
        let mut stops = vec![(0., a), (1., b)];
        for (k, n) in nodes.iter().enumerate() {
            if k == a || k == b {
                continue;
            }
            let t = (n.p - pa).dot(ab) / len2;
            if t <= 0. || t >= 1. {
                continue;
            }
            if (pa + ab * t).distance(n.p) <= eps {
                if li < nb && k >= nb {
                    return Err("a cut ended on an edge away from its cut points".into());
                }
                stops.push((t, k));
            }
        }
        stops.sort_by(|x, y| x.0.total_cmp(&y.0));
        for w in stops.windows(2) {
            let (u, v) = (w[0].1, w[1].1);
            if u != v {
                edges.insert(key2(u, v));
                if li < nb {
                    on_outline.insert(key2(u, v));
                }
            }
        }
    }
    // Drop dangling cut lines (they separate nothing), then straight-
    // through interior nodes (where a cut crossed a triangle diagonal).
    loop {
        let degree = degrees(&edges, nodes.len());
        let before = edges.len();
        edges.retain(|&(u, v)| on_outline.contains(&(u, v)) || (degree[u] > 1 && degree[v] > 1));
        if edges.len() == before {
            break;
        }
    }
    loop {
        let mut changed = false;
        let degree = degrees(&edges, nodes.len());
        for k in nb..nodes.len() {
            if degree[k] != 2 {
                continue;
            }
            let ends: Vec<usize> = edges
                .iter()
                .filter(|e| e.0 == k || e.1 == k)
                .map(|e| if e.0 == k { e.1 } else { e.0 })
                .collect();
            let (a, b) = (ends[0], ends[1]);
            let (pa, pb, pk) = (nodes[a].p, nodes[b].p, nodes[k].p);
            let ab = pb - pa;
            if cross2(ab, pk - pa).abs() <= eps * ab.length()
                && (pk - pa).dot(pk - pb) < 0.
                && !edges.contains(&key2(a, b))
            {
                edges.remove(&key2(a, k));
                edges.remove(&key2(b, k));
                edges.insert(key2(a, b));
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    // Faces; holes get two bridges each, then the faces are found again.
    for _round in 0..64 {
        let faces = extract_faces(&nodes, &edges);
        let holes = place_holes(&nodes, &edges, &faces, nb)?;
        if holes.is_empty() {
            let mut pieces = Vec::new();
            for (cycle, area) in faces {
                if area <= 0. {
                    continue;
                }
                let unique: HashSet<usize> = cycle.iter().copied().collect();
                if unique.len() != cycle.len() {
                    return Err("a piece touched itself at a point".into());
                }
                pieces.push(cycle);
            }
            return Ok(Cut { nodes, pieces });
        }
        for (face, hole) in holes {
            for bridge in bridges(&nodes, &edges, &faces, face, &hole, eps)? {
                edges.insert(key2(bridge.0, bridge.1));
            }
        }
    }
    Err("a piece's holes couldn't be joined to its outline".into())
}

fn key2(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

fn degrees(edges: &BTreeSet<(usize, usize)>, n: usize) -> Vec<usize> {
    let mut d = vec![0; n];
    for &(u, v) in edges {
        d[u] += 1;
        d[v] += 1;
    }
    d
}

/// Every face cycle of the graph with its signed area (bounded faces are
/// positive, each component's outside negative).
fn extract_faces(nodes: &[Node], edges: &BTreeSet<(usize, usize)>) -> Vec<(Vec<usize>, f64)> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for &(u, v) in edges {
        adj[u].push(v);
        adj[v].push(u);
    }
    for (k, list) in adj.iter_mut().enumerate() {
        let p = nodes[k].p;
        list.sort_by(|&a, &b| {
            let (da, db) = (nodes[a].p - p, nodes[b].p - p);
            da.y.atan2(da.x).total_cmp(&db.y.atan2(db.x))
        });
    }
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut faces = Vec::new();
    for &(u, v) in edges {
        for start in [(u, v), (v, u)] {
            if seen.contains(&start) {
                continue;
            }
            let mut cycle = Vec::new();
            let mut cur = start;
            loop {
                seen.insert(cur);
                cycle.push(cur.0);
                let list = &adj[cur.1];
                let i = list.iter().position(|&w| w == cur.0).unwrap();
                let next = (cur.1, list[(i + list.len() - 1) % list.len()]);
                if next == start || cycle.len() > 4 * edges.len() + 4 {
                    break;
                }
                cur = next;
            }
            let pts: Vec<DVec2> = cycle.iter().map(|&k| nodes[k].p).collect();
            faces.push((cycle, area_2d(&pts)));
        }
    }
    faces
}

/// The holes: each component that doesn't reach the outline sits inside
/// the smallest bounded face of another component around it. Returns
/// (face index, the hole's outside cycle).
fn place_holes(
    nodes: &[Node],
    edges: &BTreeSet<(usize, usize)>,
    faces: &[(Vec<usize>, f64)],
    nb: usize,
) -> Result<Vec<(usize, Vec<usize>)>> {
    let mut parent: Vec<usize> = (0..nodes.len()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for &(u, v) in edges {
        let (a, b) = (find(&mut parent, u), find(&mut parent, v));
        parent[a] = b;
    }
    let outline = find(&mut parent, 0);
    let mut out: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut done: HashSet<usize> = HashSet::new();
    for (cycle, area) in faces {
        if *area >= 0. {
            continue;
        }
        let comp = find(&mut parent, cycle[0]);
        if comp == outline || !done.insert(comp) {
            continue;
        }
        let probe = nodes[cycle[0]].p;
        let mut best: Option<(usize, f64)> = None;
        for (fi, (fc, fa)) in faces.iter().enumerate() {
            if *fa <= 0. || find(&mut parent, fc[0]) == comp {
                continue;
            }
            let poly: Vec<DVec2> = fc.iter().map(|&k| nodes[k].p).collect();
            if inside_2d(&poly, probe) && best.is_none_or(|b| *fa < b.1) {
                best = Some((fi, *fa));
            }
        }
        let (fi, _) = best.ok_or("a cut loop lay outside its polygon")?;
        // One hole per face per round; the rest wait for the next round.
        if out.iter().all(|(f, _)| *f != fi) {
            out.push((fi, cycle.clone()));
        }
    }
    let _ = nb;
    Ok(out)
}

/// Two bridges from `hole` to the rest of `face`'s boundary that cross
/// nothing, far apart on the hole (DRILL-16, Q1).
fn bridges(
    nodes: &[Node],
    edges: &BTreeSet<(usize, usize)>,
    faces: &[(Vec<usize>, f64)],
    face: usize,
    hole: &[usize],
    eps: f64,
) -> Result<Vec<(usize, usize)>> {
    let outer = &faces[face].0;
    let outer_poly: Vec<DVec2> = outer.iter().map(|&k| nodes[k].p).collect();
    let hole_set: HashSet<usize> = hole.iter().copied().collect();
    // Other holes already in this face (components inside it): any node
    // inside the face that isn't on its outline or this hole.
    let on_outer: HashSet<usize> = outer.iter().copied().collect();
    let targets: Vec<usize> = (0..nodes.len())
        .filter(|k| !hole_set.contains(k))
        .filter(|k| on_outer.contains(k) || inside_2d(&outer_poly, nodes[*k].p))
        .filter(|k| edges.iter().any(|e| e.0 == *k || e.1 == *k))
        .collect();
    let clear = |a: usize, b: usize, extra: &[(usize, usize)]| -> bool {
        let (pa, pb) = (nodes[a].p, nodes[b].p);
        let r = pb - pa;
        let lr = r.length();
        for &(u, v) in edges.iter().chain(extra) {
            if (u == a || u == b) && (v == a || v == b) {
                return false;
            }
            let (pu, pv) = (nodes[u].p, nodes[v].p);
            let s = pv - pu;
            let d1 = cross2(r, pu - pa);
            let d2 = cross2(r, pv - pa);
            let d3 = cross2(s, pa - pu);
            let d4 = cross2(s, pb - pu);
            let shares = u == a || u == b || v == a || v == b;
            if shares {
                // Only a fold back along the bridge is a problem.
                let other = if u == a || u == b { v } else { u };
                let po = nodes[other].p;
                let t = (po - pa).dot(r) / (lr * lr);
                if cross2(r, po - pa).abs() <= eps * lr && t > 0. && t < 1. {
                    return false;
                }
                continue;
            }
            let ls = s.length();
            let straddles_r =
                (d1 > eps * lr) != (d2 > eps * lr) || d1.abs() <= eps * lr || d2.abs() <= eps * lr;
            let straddles_s =
                (d3 > 0.) != (d4 > 0.) || d3.abs() <= eps * ls || d4.abs() <= eps * ls;
            if straddles_r && straddles_s {
                // Possible touch or crossing: check the overlap of boxes.
                let lo = pa.min(pb) - DVec2::splat(eps);
                let hi = pa.max(pb) + DVec2::splat(eps);
                let (ql, qh) = (pu.min(pv), pu.max(pv));
                if (qh.cmpge(lo) & ql.cmple(hi)).all() {
                    return false;
                }
            }
        }
        // Inside the face, outside the hole.
        let mid = (pa + pb) * 0.5;
        let hole_poly: Vec<DVec2> = hole.iter().map(|&k| nodes[k].p).collect();
        inside_2d(&outer_poly, mid) && !inside_2d(&hole_poly, mid)
    };
    let nearest = |h: usize, skip: Option<usize>, extra: &[(usize, usize)]| -> Option<usize> {
        let mut order: Vec<usize> = targets
            .iter()
            .copied()
            .filter(|&t| Some(t) != skip)
            .collect();
        order.sort_by(|&x, &y| {
            nodes[x]
                .p
                .distance(nodes[h].p)
                .total_cmp(&nodes[y].p.distance(nodes[h].p))
        });
        order.into_iter().find(|&t| clear(h, t, extra))
    };
    // Bridge one: the shortest clear one.
    let mut first: Option<(usize, usize, f64)> = None;
    for &h in hole {
        if let Some(t) = nearest(h, None, &[]) {
            let d = nodes[h].p.distance(nodes[t].p);
            if first.is_none_or(|f| d < f.2) {
                first = Some((h, t, d));
            }
        }
    }
    let (h1, t1, _) = first.ok_or("no clear bridge from a hole to its outline")?;
    // Bridge two: from the hole point farthest from the first.
    let mut others: Vec<usize> = hole.iter().copied().filter(|&h| h != h1).collect();
    others.sort_by(|&x, &y| {
        nodes[y]
            .p
            .distance(nodes[h1].p)
            .total_cmp(&nodes[x].p.distance(nodes[h1].p))
    });
    for h2 in others {
        if let Some(t2) = nearest(h2, Some(t1), &[(h1, t1)]) {
            return Ok(vec![(h1, t1), (h2, t2)]);
        }
    }
    Err("no second clear bridge from a hole to its outline".into())
}

// ── The run ─────────────────────────────────────────────────────────────

/// What one drill run produced before the operation picks pieces.
struct Pieces {
    /// Output polygons (all pieces), with whether each is inside.
    faces: Vec<(OutFace, bool)>,
    /// New points (target local space).
    new_points: Vec<DVec3>,
    /// Did the driver cut or touch any polygon?
    touched: bool,
}

/// One frame per polygon: 2D coordinates in its Newell plane, CCW.
struct Frame {
    o: DVec3,
    u: DVec3,
    v: DVec3,
}

impl Frame {
    fn new(points: &[DVec3]) -> Option<Self> {
        let n = newell(points);
        if n.length_squared() == 0. {
            return None;
        }
        let n = n.normalize();
        let u = n.any_orthonormal_vector();
        let v = n.cross(u);
        let o = points.iter().copied().sum::<DVec3>() / points.len() as f64;
        Some(Self { o, u, v })
    }
    fn flat(&self, p: DVec3) -> DVec2 {
        DVec2::new((p - self.o).dot(self.u), (p - self.o).dot(self.v))
    }
}

/// Cut the target's polygons (work space: local minus `centre`).
fn drill_pieces(g: &Gathered, cutter: &Cutter, centre: DVec3, size: f64) -> Result<Pieces> {
    let mesh = &g.combined;
    let work = |v: u32| mesh.vertices[v as usize] - centre;
    let eps = 1e-9 * size;
    let base = mesh.vertices.len() as u32;
    let mut new_points: Vec<DVec3> = Vec::new();
    let target: Vec<usize> = g
        .target
        .faces
        .iter()
        .copied()
        .filter(|&f| !mesh.is_polyline(f))
        .collect();

    // 1. Edge cuts, once per edge.
    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for &f in &target {
        let v = &mesh.faces[f].vertices;
        for i in 0..v.len() {
            edge_faces
                .entry(edge_key(v[i], v[(i + 1) % v.len()]))
                .or_default()
                .push(f);
        }
    }
    // Per edge (low, high): cut points as (parameter from low, mesh id).
    let mut edge_cuts: HashMap<(u32, u32), Vec<(f64, u32)>> = HashMap::new();
    for &(a, b) in edge_faces.keys() {
        let (p, q) = (work(a), work(b));
        let len = p.distance(q);
        if len == 0. {
            continue;
        }
        let (lo, hi) = bbox(&[p, q]);
        let mut ts: Vec<f64> = Vec::new();
        for t in cutter.grid.near(lo, hi, eps) {
            ts.extend(edge_hits(p, q, cutter.tri(t)));
        }
        ts.retain(|&t| t * len > eps && (1. - t) * len > eps);
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y) * len <= eps);
        if ts.is_empty() {
            continue;
        }
        let cuts = ts
            .into_iter()
            .map(|t| {
                new_points.push(p + (q - p) * t + centre);
                (t, base + new_points.len() as u32 - 1)
            })
            .collect();
        edge_cuts.insert((a, b), cuts);
    }

    // 2. Polygons near the driver get split.
    let mut faces: Vec<(OutFace, bool)> = Vec::new();
    let mut whole: Vec<usize> = Vec::new(); // polygons the driver doesn't touch
    let mut touched = !edge_cuts.is_empty();
    for &f in &target {
        let verts = &mesh.faces[f].vertices;
        let pts: Vec<DVec3> = verts.iter().map(|&v| work(v)).collect();
        let has_cuts = (0..verts.len())
            .any(|i| edge_cuts.contains_key(&edge_key(verts[i], verts[(i + 1) % verts.len()])));
        let Some(frame) = Frame::new(&pts) else {
            if has_cuts {
                return Err(format!(
                    "{} has a zero-area polygon where the drill cuts (Mesh Cleanup removes it)",
                    g.target.name
                ));
            }
            whole.push(f);
            continue;
        };
        let triangles: Vec<[usize; 3]> = if pts.len() == 3 {
            vec![[0, 1, 2]]
        } else {
            crate::polygon_triple::triangulate_polygon(&pts, |_, _| false)
                .unwrap_or_else(|_| (1..pts.len() - 1).map(|i| [0, i, i + 1]).collect())
        };
        let (lo, hi) = bbox(&pts);
        let near = cutter.grid.near(lo, hi, eps);
        let mut segments = Vec::new();
        for t in &near {
            let dt = cutter.tri(*t);
            for tri in &triangles {
                if let Some(s) = tri_tri(tri.map(|i| pts[i]), dt) {
                    segments.push(s);
                }
            }
        }
        if segments.is_empty() && !has_cuts {
            whole.push(f);
            continue;
        }
        touched = true;
        // The outline with its edge cuts, in corner order.
        let mut boundary: Vec<Node> = Vec::new();
        for i in 0..verts.len() {
            let (a, b) = (verts[i], verts[(i + 1) % verts.len()]);
            boundary.push(Node {
                p: frame.flat(pts[i]),
                p3: pts[i],
                id: Some(a),
            });
            if let Some(cuts) = edge_cuts.get(&edge_key(a, b)) {
                let mut list: Vec<(f64, u32)> = cuts
                    .iter()
                    .map(|&(t, id)| (if a < b { t } else { 1. - t }, id))
                    .collect();
                list.sort_by(|x, y| x.0.total_cmp(&y.0));
                for (_, id) in list {
                    let p3 = new_points[(id - base) as usize] - centre;
                    boundary.push(Node {
                        p: frame.flat(p3),
                        p3,
                        id: Some(id),
                    });
                }
            }
        }
        let flat: Vec<DVec2> = pts.iter().map(|&p| frame.flat(p)).collect();
        let lift = |q: DVec2| -> DVec3 {
            // Barycentric on the polygon's triangle holding q (nearest).
            let mut best: Option<(f64, DVec3)> = None;
            for tri in &triangles {
                let [a, b, c] = tri.map(|i| flat[i]);
                let den = cross2(b - a, c - a);
                if den == 0. {
                    continue;
                }
                let wb = cross2(q - a, c - a) / den;
                let wc = cross2(b - a, q - a) / den;
                let wa = 1. - wb - wc;
                let out = (-wa.min(wb).min(wc)).max(0.);
                let p = pts[tri[0]] * wa + pts[tri[1]] * wb + pts[tri[2]] * wc;
                if best.as_ref().is_none_or(|b| out < b.0) {
                    best = Some((out, p));
                }
            }
            best.map_or(frame.o, |b| b.1)
        };
        let to_2d = |p: DVec3| frame.flat(p);
        let cut = cut_polygon(boundary, &segments, &to_2d, &lift, eps)?;
        let nb_ids: Vec<Option<u32>> = cut.nodes.iter().map(|n| n.id).collect();
        let mut ids = nb_ids;
        let own_area = newell(&pts).length();
        let normal = newell(&pts).normalize();
        let mut covered = 0.;
        for piece in &cut.pieces {
            let mut vertices = Vec::with_capacity(piece.len());
            for &k in piece {
                let id = *ids[k].get_or_insert_with(|| {
                    new_points.push(cut.nodes[k].p3 + centre);
                    base + new_points.len() as u32 - 1
                });
                vertices.push(id);
            }
            let at = |id: u32| -> DVec3 {
                if id < base {
                    mesh.vertices[id as usize]
                } else {
                    new_points[(id - base) as usize]
                }
            };
            let p3: Vec<DVec3> = vertices.iter().map(|&v| at(v)).collect();
            let projected = newell(&p3).dot(normal);
            if projected <= 0. {
                return Err("a cut piece came out folded".into());
            }
            covered += projected;
            // Sample: the middle of the piece's largest triangle.
            let flat_piece: Vec<DVec3> = piece.iter().map(|&k| cut.nodes[k].p.extend(0.)).collect();
            let sample = sample_point(&flat_piece);
            let inside = cutter.inside(lift(sample.truncate()))?;
            // An untouched outline keeps its corner order (DRILL-16).
            if vertices.len() == verts.len()
                && let Some(s) = vertices.iter().position(|&v| v == verts[0])
                && (0..verts.len()).all(|i| vertices[(s + i) % verts.len()] == verts[i])
            {
                vertices.clone_from(verts);
            }
            faces.push((out_face(mesh, f, vertices, &new_points, base), inside));
        }
        // DRILL-19: the pieces cover the polygon exactly.
        if (covered - own_area).abs() > 1e-6 * own_area {
            return Err("the cut pieces didn't cover their polygon".into());
        }
    }

    // 3. Untouched polygons: one answer per patch joined by uncut edges.
    let whole_set: HashSet<usize> = whole.iter().copied().collect();
    let mut answer: HashMap<usize, bool> = HashMap::new();
    for &seed in &whole {
        if answer.contains_key(&seed) {
            continue;
        }
        let pts: Vec<DVec3> = mesh.faces[seed].vertices.iter().map(|&v| work(v)).collect();
        let inside = cutter.inside(sample_point(&pts))?;
        let mut stack = vec![seed];
        answer.insert(seed, inside);
        while let Some(f) = stack.pop() {
            let v = &mesh.faces[f].vertices;
            for i in 0..v.len() {
                for &n in &edge_faces[&edge_key(v[i], v[(i + 1) % v.len()])] {
                    if whole_set.contains(&n) && !answer.contains_key(&n) {
                        answer.insert(n, inside);
                        stack.push(n);
                    }
                }
            }
        }
    }
    for &f in &whole {
        let vertices = mesh.faces[f].vertices.clone();
        faces.push((out_face(mesh, f, vertices, &new_points, base), answer[&f]));
    }
    Ok(Pieces {
        faces,
        new_points,
        touched,
    })
}

/// A point well inside a (3D or flat) polygon: the middle of its largest
/// triangle.
fn sample_point(points: &[DVec3]) -> DVec3 {
    let triangles = if points.len() == 3 {
        vec![[0, 1, 2]]
    } else {
        crate::polygon_triple::triangulate_polygon(points, |_, _| false)
            .unwrap_or_else(|_| (1..points.len() - 1).map(|i| [0, i, i + 1]).collect())
    };
    triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| points[i]);
            ((b - a).cross(c - a).length(), (a + b + c) / 3.)
        })
        .max_by(|x, y| x.0.total_cmp(&y.0))
        .map_or(points[0], |x| x.1)
}

/// One output polygon from source polygon `f` (corner data from the
/// source's corners, blended for new points).
fn out_face(mesh: &Mesh, f: usize, vertices: Vec<u32>, new_points: &[DVec3], base: u32) -> OutFace {
    let source = &mesh.faces[f].vertices;
    let mut donors = Vec::with_capacity(vertices.len());
    let mut blends = Vec::with_capacity(vertices.len());
    for &v in &vertices {
        match source.iter().position(|&s| s == v) {
            Some(k) => {
                donors.push(Some((f, k)));
                blends.push(None);
            }
            None => {
                let at = if v < base {
                    mesh.vertices[v as usize]
                } else {
                    new_points[(v - base) as usize]
                };
                donors.push(None);
                blends.push(Some(corner_blend(mesh, f, at)));
            }
        }
    }
    OutFace {
        template: f,
        vertices,
        donors,
        blends,
    }
}

/// Total length of the edges used by exactly one polygon.
fn open_length(points: &dyn Fn(u32) -> DVec3, faces: &[&[u32]]) -> f64 {
    let mut uses: HashMap<(u32, u32), usize> = HashMap::new();
    for f in faces {
        for i in 0..f.len() {
            *uses
                .entry(edge_key(f[i], f[(i + 1) % f.len()]))
                .or_default() += 1;
        }
    }
    uses.iter()
        .filter(|(_, n)| **n == 1)
        .map(|(&(a, b), _)| points(a).distance(points(b)))
        .sum()
}

// ── Drivers ─────────────────────────────────────────────────────────────

/// Solid Drill: the drivers, pre-flighted (DRILL-03) and unioned when
/// there are several (DRILL-02, BOOL-07), in work space.
fn solid_cutter(g: &Gathered, what: &str, centre: DVec3, size: f64) -> Result<Cutter> {
    let to_local = g.to_world.inverse();
    let mut solids = Vec::new();
    for input in &g.drivers {
        let (mut tri, _) =
            preflight(g, input).map_err(|p| format!("{what}: {}. Nothing changed.", p.message))?;
        for p in &mut tri.pos {
            *p = to_local.transform_point3(*p) - centre;
        }
        tri.owner = vec![0; tri.tris.len()];
        solids.push(tri);
    }
    let solid = union(solids, what)?;
    Ok(Cutter::new(solid.pos, solid.tris, None, size))
}

fn union(mut solids: Vec<Tri>, what: &str) -> Result<Tri> {
    if solids.len() == 1 {
        return Ok(solids.pop().unwrap());
    }
    // Inverted solids (a mirrored target) are flipped for the engine.
    for s in &mut solids {
        if s.volume() < 0. {
            for t in &mut s.tris {
                t.swap(1, 2);
            }
        }
    }
    guarded(|| manifold_run(&solids[0], &solids[1..], EngineOp::Union))
        .map_err(|e| format!("{what}: joining the drivers failed ({e}). Nothing changed."))
}

/// Axis Drill: every driver polygon's shadow along `axis`, extruded far
/// both ways, unioned (DRILL-04/14).
fn prism_cutter(
    g: &Gathered,
    axis: DrillAxis,
    what: &str,
    centre: DVec3,
    size: f64,
) -> Result<Cutter> {
    let mesh = &g.combined;
    let to_local = g.to_world.inverse();
    let d = to_local.transform_vector3(axis.vector());
    if d.length_squared() == 0. || !d.is_finite() {
        return Err(format!(
            "{what}: the target's transform is flat along {}",
            axis.label()
        ));
    }
    let d = d.normalize();
    let e1 = d.any_orthonormal_vector();
    let e2 = d.cross(e1);
    let h = 4. * size;
    let mut solids = Vec::new();
    let mut polygons = Vec::new();
    for input in &g.drivers {
        for &f in &input.faces {
            if mesh.is_polyline(f) || mesh.faces[f].vertices.len() < 3 {
                continue;
            }
            let mut flat: Vec<DVec2> = mesh.faces[f]
                .vertices
                .iter()
                .map(|&v| {
                    let p = mesh.vertices[v as usize] - centre;
                    DVec2::new(p.dot(e1), p.dot(e2))
                })
                .collect();
            let area = area_2d(&flat);
            if area.abs() <= 1e-12 * size * size {
                continue;
            }
            if area < 0. {
                flat.reverse();
            }
            let pts3: Vec<DVec3> = flat.iter().map(|q| q.extend(0.)).collect();
            let caps = if flat.len() == 3 {
                vec![[0, 1, 2]]
            } else {
                crate::polygon_triple::triangulate_polygon(&pts3, |_, _| false).map_err(|_| {
                    format!(
                        "{what}: a driver polygon of {}'s outline crosses itself along {}",
                        input.name,
                        axis.label()
                    )
                })?
            };
            let n = flat.len() as u32;
            let at = |q: DVec2, s: f64| e1 * q.x + e2 * q.y + d * s;
            let mut tri = Tri::default();
            tri.pos.extend(flat.iter().map(|&q| at(q, -h)));
            tri.pos.extend(flat.iter().map(|&q| at(q, h)));
            for c in &caps {
                let c = c.map(|i| i as u32);
                tri.tris.push([c[0] + n, c[1] + n, c[2] + n]);
                tri.tris.push([c[0], c[2], c[1]]);
            }
            for i in 0..n {
                let j = (i + 1) % n;
                tri.tris.push([i, j, j + n]);
                tri.tris.push([i, j + n, i + n]);
            }
            tri.owner = vec![0; tri.tris.len()];
            solids.push(tri);
            let (lo, hi) = flat.iter().fold(
                (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)),
                |(l, hh), &q| (l.min(q), hh.max(q)),
            );
            polygons.push((flat, lo, hi));
        }
    }
    if solids.is_empty() {
        return Err(format!(
            "{what}: The driver has no outline along {}. Nothing changed.",
            axis.label()
        ));
    }
    let solid = union(solids, what)?;
    Ok(Cutter::new(
        solid.pos,
        solid.tris,
        Some(Shadow { e1, e2, polygons }),
        size,
    ))
}

// ── The command ─────────────────────────────────────────────────────────

impl Editor {
    /// The driver Solid Drill would refuse now (DRILL-03), for the notice's
    /// Select Problem Edges.
    pub fn drill_problem(&self, kind: DrillKind, o: &DrillOptions) -> Option<BooleanProblem> {
        if kind != DrillKind::Solid {
            return None;
        }
        let g = self
            .drive_gather(&o.drive_options(), kind.label(), Some(NO_DRIVER))
            .ok()?;
        g.drivers
            .iter()
            .find_map(|input| preflight(&g, input).err())
    }

    /// DRILL-01..21: the target's new mesh, the undo label and what to tell
    /// the user. Changes nothing in the scene.
    pub(crate) fn drill_result(
        &self,
        kind: DrillKind,
        o: &DrillOptions,
    ) -> Result<(u64, Mesh, String, String)> {
        let what = format!("{} {}", kind.label(), o.op.label());
        let g = self.drive_gather(&o.drive_options(), &what, Some(NO_DRIVER))?;
        match catch_unwind(AssertUnwindSafe(|| drill_mesh(&g, kind, o, &what))) {
            Ok(r) => r.map(|(mesh, message)| (g.target.item, mesh, what, message)),
            Err(_) => Err(format!(
                "{what}: the drill stopped unexpectedly. Nothing changed."
            )),
        }
    }

    /// `poly.solidDrill` / `poly.drill`: one undo step "<tool> <operation>"
    /// (DRILL-27). The drivers are untouched; nothing is left selected
    /// (DRILL-22). Stencil's material is created if missing (Q5).
    pub(crate) fn drill(&mut self, kind: DrillKind, o: DrillOptions) -> Result<()> {
        let (item, mesh, label, message) = self.drill_result(kind, &o)?;
        match kind {
            DrillKind::Solid => self.solid_drill_preset = o.clone(),
            DrillKind::Axis => self.axis_drill_preset = o.clone(),
        }
        let before = self.scene.clone();
        if o.op == DrillOp::Stencil {
            // The material and the cut are one undo step.
            let key = o.stencil_material().to_owned();
            let shown = self.scene.material_or_default(&key);
            let made = self.scene.map_materials_into_default_tree().and_then(|()| {
                self.scene
                    .shader_tree
                    .ensure_material(&key, &shown)
                    .map(|_| ())
            });
            if let Err(e) = made {
                self.scene = before;
                return Err(e);
            }
        }
        self.publish_polygon_edit_from(item, mesh, &label, Vec::new(), before)?;
        self.drill_note = Some(message);
        Ok(())
    }
}

/// The run itself, behind the panic guard.
pub(crate) fn drill_mesh(
    g: &Gathered,
    kind: DrillKind,
    o: &DrillOptions,
    what: &str,
) -> Result<(Mesh, String)> {
    let mesh = &g.combined;
    // Work space: target local, centred on everything in the run.
    let mut all: Vec<DVec3> = g
        .target
        .faces
        .iter()
        .flat_map(|&f| {
            mesh.faces[f]
                .vertices
                .iter()
                .map(|&v| mesh.vertices[v as usize])
        })
        .collect();
    if all.is_empty() {
        return Err(format!("{what}: {} has no polygons to cut", g.target.name));
    }
    for input in &g.drivers {
        for &f in &input.faces {
            all.extend(
                mesh.faces[f]
                    .vertices
                    .iter()
                    .map(|&v| mesh.vertices[v as usize]),
            );
        }
    }
    let (lo, hi) = bbox(&all);
    if !(lo.is_finite() && hi.is_finite()) {
        return Err(format!(
            "{what}: some points aren't finite numbers. Nothing changed."
        ));
    }
    let centre = (lo + hi) * 0.5;
    let size = (hi - lo).length().max(1e-300);
    let cutter = match kind {
        DrillKind::Solid => solid_cutter(g, what, centre, size)?,
        DrillKind::Axis => prism_cutter(g, o.axis, what, centre, size)?,
    };
    let pieces = drill_pieces(g, &cutter, centre, size)
        .map_err(|e| format!("{what}: {e}. Nothing changed."))?;
    let base = mesh.vertices.len() as u32;
    let point = |v: u32| -> DVec3 {
        if v < base {
            mesh.vertices[v as usize]
        } else {
            pieces.new_points[(v - base) as usize]
        }
    };
    // DRILL-19/21: the whole cut surface has the same open edges as the
    // target (no gaps, no T-junctions) before the operation picks pieces.
    let target_faces: Vec<&[u32]> = g
        .target
        .faces
        .iter()
        .filter(|&&f| !mesh.is_polyline(f))
        .map(|&f| mesh.faces[f].vertices.as_slice())
        .collect();
    let cut_faces: Vec<&[u32]> = pieces
        .faces
        .iter()
        .map(|(f, _)| f.vertices.as_slice())
        .collect();
    let (was, now) = (
        open_length(&point, &target_faces),
        open_length(&point, &cut_faces),
    );
    if (was - now).abs() > 1e-6 * size.max(was) {
        return Err(format!(
            "{what}: the cut left gaps between polygons, so it can't be trusted. Nothing changed."
        ));
    }
    let inside_count = pieces.faces.iter().filter(|f| f.1).count();
    let total = pieces.faces.len();
    if !pieces.touched {
        // DRILL-25: the driver doesn't reach the target.
        let reach = match (o.op, inside_count) {
            (DrillOp::Core, 0) => None,
            (DrillOp::Core, n) if n == total => {
                Some("the target is entirely inside the driver".to_string())
            }
            (DrillOp::Tunnel | DrillOp::Stencil | DrillOp::Slice, 0) => {
                Some("the driver doesn't reach the target".to_string())
            }
            (DrillOp::Slice, _) => Some("the driver doesn't cut the target".to_string()),
            _ => None,
        };
        if let Some(why) = reach {
            return Err(format!("{what}: {why}, so nothing changed."));
        }
    }
    // Pick the pieces (DRILL-10..13). Everything that isn't part of the
    // target (Last Selected's driver polygons, curves) comes through.
    let target_set: HashSet<usize> = g.target.faces.iter().copied().collect();
    let (own_faces, own_points) = own_counts(g);
    let mut out: Vec<OutFace> = Vec::new();
    let mut stencil: Vec<bool> = Vec::new();
    for f in 0..own_faces {
        if target_set.contains(&f) && !mesh.is_polyline(f) {
            continue;
        }
        let vertices = mesh.faces[f].vertices.clone();
        let n = vertices.len();
        out.push(OutFace {
            template: f,
            vertices,
            donors: (0..n).map(|k| Some((f, k))).collect(),
            blends: vec![None; n],
        });
        stencil.push(false);
    }
    for (face, inside) in pieces.faces {
        let keep = match o.op {
            DrillOp::Core => inside,
            DrillOp::Tunnel => !inside,
            DrillOp::Stencil | DrillOp::Slice => true,
        };
        if keep {
            stencil.push(inside && o.op == DrillOp::Stencil);
            out.push(face);
        }
    }
    // Polygons stay in the target's order, pieces where their source was.
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by_key(|&i| out[i].template);
    let stencil: Vec<bool> = order.iter().map(|&i| stencil[i]).collect();
    let mut slots: Vec<Option<OutFace>> = out.into_iter().map(Some).collect();
    let out: Vec<OutFace> = order.iter().map(|&i| slots[i].take().unwrap()).collect();
    let mut vertices = mesh.vertices.clone();
    vertices.extend_from_slice(&pieces.new_points);
    let mut result = rebuild(mesh, vertices, out, &[], what)?;
    let material = o.stencil_material().to_owned();
    for (face, s) in result.faces.iter_mut().zip(&stencil) {
        if *s {
            face.material = material.clone();
            face.material_label = Some(material.clone());
        }
    }
    let was_used: BTreeSet<u32> = mesh
        .used_points()
        .iter()
        .enumerate()
        .filter(|(_, u)| **u)
        .map(|(v, _)| v as u32)
        .chain(base..result.vertices.len() as u32)
        .collect();
    result = crate::mesh_cleanup::remove_unreferenced(&result, Some(&was_used))?.mesh;
    result
        .validate()
        .map_err(|e| format!("{what}: the result couldn't be rebuilt ({e}). Nothing changed."))?;
    // Unchanged (a Slice along existing edges): no step.
    if result.faces[..] == mesh.faces[..own_faces]
        && result.vertices[..] == mesh.vertices[..own_points]
    {
        return Err(format!(
            "{what}: the cut lines lie on existing edges, so nothing changed."
        ));
    }
    let cut = inside_count.min(total);
    let message = match (o.op, result.faces.is_empty()) {
        (DrillOp::Core, true) => {
            "Core: the driver doesn't reach the target, so nothing is left.".to_string()
        }
        (DrillOp::Stencil, _) => format!(
            "{what}: {}, {} with {material}",
            plural(result.faces.len(), "polygon", "polygons"),
            plural(cut, "piece", "pieces")
        ),
        _ => format!(
            "{what}: {}",
            plural(result.faces.len(), "polygon", "polygons")
        ),
    };
    Ok((result, message))
}

/// How many polygons and points the target item itself has (drivers from
/// other items are appended after them).
fn own_counts(g: &Gathered) -> (usize, usize) {
    g.drivers
        .iter()
        .filter(|d| d.point_offset > 0)
        .map(|d| (d.faces[0], d.point_offset as usize))
        .min()
        .unwrap_or((g.combined.faces.len(), g.combined.vertices.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse_from_numbers_and_words() {
        assert_eq!(DrillOp::parse("1").unwrap(), DrillOp::Tunnel);
        assert_eq!(DrillOp::parse("stencil").unwrap(), DrillOp::Stencil);
        assert!(DrillOp::parse("4").is_err());
        assert_eq!(DrillAxis::parse("2").unwrap(), DrillAxis::Z);
        assert_eq!(DrillAxis::parse("x").unwrap(), DrillAxis::X);
        assert!(DrillAxis::parse("w").is_err());
        assert_eq!(DrillOptions::default().stencil_material(), "Stencil");
    }

    #[test]
    fn crossing_segment_of_two_triangles() {
        let a = [
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(0., 1., 0.),
        ];
        let t = [
            DVec3::new(0., -2., -1.),
            DVec3::new(0., 2., -1.),
            DVec3::new(0., 0., 1.),
        ];
        let (p, q) = tri_tri(a, t).unwrap();
        assert!(p.x.abs() < 1e-12 && q.x.abs() < 1e-12);
        assert!(p.z.abs() < 1e-12 && q.z.abs() < 1e-12);
        let apart = [
            DVec3::new(5., 0., 0.),
            DVec3::new(6., 0., 0.),
            DVec3::new(5., 1., 0.),
        ];
        assert!(tri_tri(apart, t).is_none());
    }
}
