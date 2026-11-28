//! Constrain to Background (SPEC-BACKGROUND-CONSTRAINT v1, tutorial gaps
//! 2026-11-13): points a tool moves or creates in the foreground meshes
//! land on the surface of the background meshes.
//!
//! - Foreground: the active meshes (selected and shown). Background: every
//!   other shown mesh. Hidden polygons of a background mesh don't count
//!   (provisional, BG-Q1).
//! - The constraint is tool context, like the falloff: it lives on the
//!   `Editor`, is not saved and is not an undo step. `Editor::preview`
//!   applies it after a tool's own change on every frame, to the points
//!   the tool moved (`Editor::constrain_preview`); Pen applies it to each
//!   point it places.
//! - A SubD background is its displayed smooth surface, never its cage
//!   (BG-Q5, answered by Jason 2026-11-15): smooth polygons are replaced
//!   by the pieces `subdivision::display_surface` gives at the mesh's
//!   display levels, the same evaluation the viewport draws and picks;
//!   Face polygons stay as they are.
//! - The background is gathered once per drag: the world-space triangles
//!   and their box tree are cached by the background meshes' shared
//!   allocations, subdivision settings and transforms (`BackgroundCache`),
//!   so a drag frame only pays for the rays of the points that moved.
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, BG-Q1..Q10):
//! Screen Axis searches the whole line of sight through the point and takes the
//! hit nearest the viewer, so a point drawn on the work plane inside the
//! base mesh still lands on its front (Q4/Q6); Offset follows the surface
//! normal toward the viewer's side (Q7).
use crate::{Mode, Result, Scene};
use glam::{DMat4, DVec3};
use std::sync::{Arc, Mutex};

/// How a point finds the background (the spec's Geometry Constraint).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GeometryConstraint {
    /// In the tool setup but doing nothing.
    Off,
    /// Along the view direction of the view in use (perspective: along
    /// the line from the eye through the point).
    #[default]
    ScreenAxis,
    /// Along the direction the tool moved the point.
    Vector,
    /// To the nearest point of the background surface.
    Point,
}
impl GeometryConstraint {
    pub const ALL: [Self; 4] = [Self::Off, Self::ScreenAxis, Self::Vector, Self::Point];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::ScreenAxis => "Screen Axis",
            Self::Vector => "Vector",
            Self::Point => "Point",
        }
    }
    /// The command-line spelling.
    pub fn word(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::ScreenAxis => "screen",
            Self::Vector => "vector",
            Self::Point => "point",
        }
    }
    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|m| m.word().eq_ignore_ascii_case(word) || m.label().eq_ignore_ascii_case(word))
    }
}

/// The constraint's options (its Tool Properties and Mesh Constraint
/// popover rows). The app keeps the last ones in its preferences
/// (MC-06), all but Target Item, which names an item of one scene.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BackgroundConstraint {
    pub mode: GeometryConstraint,
    /// Distance off the surface, in metres, along its normal toward the
    /// side the search came from.
    pub offset: f64,
    /// Lets `offset` go below zero (points sit under the surface). On by
    /// default: it only unlocks negative values (MC-06).
    pub negative_offset: bool,
    /// Also constrain the handles of curve-drawing tools (MC-05, on by
    /// default). Stored only: the Draw tools' handles are the only curve
    /// handles, and Draw tools aren't constrained (BG-Q8).
    pub handle_constraint: bool,
    /// Background polygons count from both sides. Off: only polygons
    /// facing the search (provisional default off, BG-Q3).
    pub double_sided: bool,
    /// Target Item (spec v1.1): only this item counts as background.
    /// `None`, every background mesh (provisional, BG-Q12).
    #[serde(skip)]
    pub target: Option<u64>,
}
impl Default for BackgroundConstraint {
    fn default() -> Self {
        Self {
            mode: GeometryConstraint::ScreenAxis,
            offset: 0.,
            negative_offset: true,
            handle_constraint: true,
            double_sided: false,
            target: None,
        }
    }
}
impl BackgroundConstraint {
    pub fn validate(&self) -> Result<()> {
        if !self.offset.is_finite() || self.offset.abs() > 1e6 {
            return Err("Background Constraint Offset must be a finite distance".into());
        }
        if self.offset < 0. && !self.negative_offset {
            return Err("Turn on Negative Offset to put points under the surface".into());
        }
        Ok(())
    }
    /// The offset the points get (a negative one only with Negative
    /// Offset on).
    fn applied_offset(&self) -> f64 {
        if self.negative_offset {
            self.offset
        } else {
            self.offset.max(0.)
        }
    }
}

/// The view a Screen Axis search runs along: the view the tool is being
/// used in. Set by the app on each press in a view; MCP and command-line
/// edits with no view yet use the Front view (looking along −Z).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstraintView {
    /// The direction the view looks (unit, world space).
    pub forward: DVec3,
    /// A perspective view's eye; `None` for an orthographic view.
    pub eye: Option<DVec3>,
}
impl Default for ConstraintView {
    fn default() -> Self {
        Self {
            forward: -DVec3::Z,
            eye: None,
        }
    }
}
impl ConstraintView {
    /// The unit direction from the viewer through `p`.
    fn through(&self, p: DVec3) -> DVec3 {
        match self.eye {
            Some(eye) => (p - eye).try_normalize().unwrap_or(self.forward),
            None => self.forward,
        }
    }
}

/// Where the last view press was. Viewer context, not editor state: two
/// editors compare equal whatever view was last pressed in.
#[derive(Clone, Copy, Debug, Default)]
pub struct LastView(pub Option<ConstraintView>);
impl PartialEq for LastView {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// One background triangle: its corners in `Background::points` and its
/// polygon's (or smooth piece's) normal, unit, the side it faces. Indexed
/// so a smooth surface of millions of triangles stores each point once.
#[derive(Clone, Copy, Debug)]
struct Tri {
    v: [u32; 3],
    n: DVec3,
}

/// A box-tree node: `count == 0` is an inner node whose children are
/// `first` and `first + 1`; otherwise a leaf of `count` triangles from
/// `first` in `Background::order`.
#[derive(Clone, Copy, Debug)]
struct Node {
    min: DVec3,
    max: DVec3,
    first: u32,
    count: u32,
}

/// The background surface, in world space, ready for ray and nearest-point
/// searches.
#[derive(Debug, Default)]
pub struct Background {
    /// World-space points the triangles use.
    points: Vec<DVec3>,
    tris: Vec<Tri>,
    order: Vec<u32>,
    nodes: Vec<Node>,
}

/// A hit on the background: the point and the surface normal turned
/// toward the side the search came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub point: DVec3,
    pub normal: DVec3,
}

const LEAF: usize = 4;

impl Background {
    /// The shown, inactive meshes of `scene`, minus their hidden polygons.
    pub fn of(scene: &Scene, target: Option<u64>) -> Result<Self> {
        let mut points = Vec::new();
        let mut tris = Vec::new();
        for id in target_ids(scene, target) {
            let item = scene.item(id).ok_or("Missing item")?;
            let world = scene.world_matrix(id)?;
            let settings = item.subdivision.clone().unwrap_or_default();
            add_mesh(
                &mut points,
                &mut tris,
                &item.mesh,
                &settings,
                &item.elements,
                world,
            )?;
        }
        Ok(Self::build(points, tris))
    }
    /// One mesh's polygons placed by `m` (no subdivision): what the
    /// Surface Constraint op searches (generator_ops.rs).
    pub fn of_mesh(mesh: &crate::Mesh, m: DMat4) -> Result<Self> {
        let mut points = Vec::new();
        let mut tris = Vec::new();
        add_mesh(
            &mut points,
            &mut tris,
            mesh,
            &Default::default(),
            &Default::default(),
            m,
        )?;
        Ok(Self::build(points, tris))
    }
    /// The three corners of triangle `t`.
    fn corners(&self, t: usize) -> (DVec3, DVec3, DVec3) {
        let [a, b, c] = self.tris[t].v;
        (
            self.points[a as usize],
            self.points[b as usize],
            self.points[c as usize],
        )
    }
    fn build(points: Vec<DVec3>, tris: Vec<Tri>) -> Self {
        let mut order: Vec<u32> = (0..tris.len() as u32).collect();
        let nodes = if tris.is_empty() {
            Vec::new()
        } else {
            let corners = |t: u32| {
                let [a, b, c] = tris[t as usize].v;
                [a, b, c].map(|v| points[v as usize])
            };
            let centers: Vec<DVec3> = (0..tris.len() as u32)
                .map(|t| {
                    let [a, b, c] = corners(t);
                    (a + b + c) / 3.
                })
                .collect();
            // A smooth background can be millions of triangles: the top of
            // the tree splits across threads, each half built on its own.
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
            subtree(&corners, &centers, &mut order, 0, threads)
        };
        Self {
            points,
            tris,
            order,
            nodes,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }
    /// Half the diagonal of the background's box plus its centre, for
    /// starting a line search outside it.
    fn bounds(&self) -> Option<(DVec3, f64)> {
        let root = self.nodes.first()?;
        Some((
            (root.min + root.max) / 2.,
            (root.max - root.min).length() / 2.,
        ))
    }
    /// The first hit along the ray from `origin` in unit direction `dir`
    /// (distances above zero). With `double_sided` off only polygons
    /// facing against `dir` count.
    pub fn ray(&self, origin: DVec3, dir: DVec3, double_sided: bool) -> Option<(f64, Hit)> {
        let inv = dir.recip();
        let mut best: Option<(f64, Hit)> = None;
        let mut stack = vec![0usize];
        if self.nodes.is_empty() {
            return None;
        }
        while let Some(at) = stack.pop() {
            let node = self.nodes[at];
            let limit = best.map_or(f64::INFINITY, |b| b.0);
            if !slab(node.min, node.max, origin, inv, limit) {
                continue;
            }
            if node.count == 0 {
                stack.push(node.first as usize);
                stack.push(node.first as usize + 1);
                continue;
            }
            for &t in &self.order[node.first as usize..(node.first + node.count) as usize] {
                let tri = self.tris[t as usize];
                let facing = tri.n.dot(dir);
                if !double_sided && facing >= 0. {
                    continue;
                }
                let (a, b, c) = self.corners(t as usize);
                if let Some(d) = ray_triangle(origin, dir, a, b, c)
                    && d < best.map_or(f64::INFINITY, |b| b.0)
                {
                    let normal = if facing > 0. { -tri.n } else { tri.n };
                    best = Some((
                        d,
                        Hit {
                            point: origin + dir * d,
                            normal,
                        },
                    ));
                }
            }
        }
        best
    }
    /// The nearest point of the background surface to `p`, its normal
    /// turned toward `p`'s side.
    pub fn nearest(&self, p: DVec3) -> Option<Hit> {
        let mut best: Option<(f64, Hit)> = None;
        let mut stack = vec![0usize];
        if self.nodes.is_empty() {
            return None;
        }
        while let Some(at) = stack.pop() {
            let node = self.nodes[at];
            let d2 = (p.clamp(node.min, node.max) - p).length_squared();
            if best.is_some_and(|b| d2 > b.0) {
                continue;
            }
            if node.count == 0 {
                // Visit the nearer child first so the bound tightens early.
                let (l, r) = (node.first as usize, node.first as usize + 1);
                let dl = (p.clamp(self.nodes[l].min, self.nodes[l].max) - p).length_squared();
                let dr = (p.clamp(self.nodes[r].min, self.nodes[r].max) - p).length_squared();
                if dl < dr {
                    stack.push(r);
                    stack.push(l);
                } else {
                    stack.push(l);
                    stack.push(r);
                }
                continue;
            }
            for &t in &self.order[node.first as usize..(node.first + node.count) as usize] {
                let tri = self.tris[t as usize];
                let (a, b, c) = self.corners(t as usize);
                let q = closest_on_triangle(p, a, b, c);
                let d2 = (q - p).length_squared();
                if best.is_none_or(|b| d2 < b.0) {
                    let normal = if (p - q).dot(tri.n) < 0. {
                        -tri.n
                    } else {
                        tri.n
                    };
                    best = Some((d2, Hit { point: q, normal }));
                }
            }
        }
        best.map(|b| b.1)
    }
    /// Where `moved` (world) goes under `c`. `start` is where the point
    /// was before the tool moved it (`None` for a new point). `None`
    /// leaves the point where the tool put it (no hit, BG-03).
    pub fn constrain(
        &self,
        c: &BackgroundConstraint,
        view: &ConstraintView,
        start: Option<DVec3>,
        moved: DVec3,
    ) -> Option<DVec3> {
        let hit = match c.mode {
            GeometryConstraint::Off => return None,
            GeometryConstraint::ScreenAxis => {
                let dir = view.through(moved);
                let origin = match view.eye {
                    Some(eye) => eye,
                    None => self.line_start(moved, dir)?,
                };
                self.ray(origin, dir, c.double_sided)?.1
            }
            GeometryConstraint::Vector => {
                // Along the point's own motion, on the whole line through
                // it; the hit nearest where the tool put it (BG-Q9).
                let dir = (moved - start?).try_normalize()?;
                let origin = self.line_start(moved, dir)?;
                let mut from = origin;
                let mut nearest: Option<Hit> = None;
                while let Some((d, hit)) = self.ray(from, dir, c.double_sided) {
                    if nearest.is_none_or(|n| {
                        (hit.point - moved).length_squared() < (n.point - moved).length_squared()
                    }) {
                        nearest = Some(hit);
                    }
                    if (hit.point - origin).dot(dir) > (moved - origin).dot(dir) {
                        break;
                    }
                    from += dir * (d + 1e-9);
                }
                nearest?
            }
            GeometryConstraint::Point => self.nearest(moved)?,
        };
        Some(hit.point + hit.normal * c.applied_offset())
    }
    /// A point on the line through `p` along `dir`, before the background
    /// and before `p`.
    fn line_start(&self, p: DVec3, dir: DVec3) -> Option<DVec3> {
        let (center, radius) = self.bounds()?;
        let back = (p - center).length() + radius + 1.;
        Some(p - dir * back)
    }
}

/// The box of the triangles in `order`.
fn bounds_of(corners: &(impl Fn(u32) -> [DVec3; 3] + Sync), order: &[u32]) -> (DVec3, DVec3) {
    let (mut min, mut max) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for &t in order {
        for p in corners(t) {
            min = min.min(p);
            max = max.max(p);
        }
    }
    (min, max)
}

/// Sorts `order` about its median along the box's widest axis; returns
/// where the halves meet.
fn split(centers: &[DVec3], order: &mut [u32], min: DVec3, max: DVec3) -> usize {
    let extent = max - min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    let mid = order.len() / 2;
    order.select_nth_unstable_by(mid, |&p, &q| {
        centers[p as usize][axis].total_cmp(&centers[q as usize][axis])
    });
    mid
}

/// Triangles below which a subtree isn't worth a thread of its own.
const THREADED: usize = 65_536;

/// The box tree over `order` (which starts at `first` of the whole
/// order), root first. With more than one thread the two halves are built
/// at once and joined: root, both child roots, then the rest of each.
fn subtree(
    corners: &(impl Fn(u32) -> [DVec3; 3] + Sync),
    centers: &[DVec3],
    order: &mut [u32],
    first: u32,
    threads: usize,
) -> Vec<Node> {
    if threads <= 1 || order.len() < THREADED {
        return serial_tree(corners, centers, order, first);
    }
    let (min, max) = bounds_of(corners, order);
    let mid = split(centers, order, min, max);
    let (lo, hi) = order.split_at_mut(mid);
    let (left, right) = std::thread::scope(|s| {
        let left = s.spawn(|| subtree(corners, centers, lo, first, threads / 2));
        let right = subtree(
            corners,
            centers,
            hi,
            first + mid as u32,
            threads - threads / 2,
        );
        (left.join(), right)
    });
    let left = left.unwrap_or_else(|_| serial_tree(corners, centers, &mut order[..mid], first));
    // Where each half's nodes land in the joined list.
    let (l_rest, r_rest) = (3, 3 + left.len() - 1);
    let at_left = |i: u32| if i == 0 { 1 } else { l_rest + i as usize - 1 } as u32;
    let at_right = |i: u32| if i == 0 { 2 } else { r_rest + i as usize - 1 } as u32;
    let moved = |n: Node, at: &dyn Fn(u32) -> u32| Node {
        first: if n.count == 0 { at(n.first) } else { n.first },
        ..n
    };
    let mut nodes = Vec::with_capacity(1 + left.len() + right.len());
    nodes.push(Node {
        min,
        max,
        first: 1,
        count: 0,
    });
    nodes.push(moved(left[0], &at_left));
    nodes.push(moved(right[0], &at_right));
    nodes.extend(left[1..].iter().map(|&n| moved(n, &at_left)));
    nodes.extend(right[1..].iter().map(|&n| moved(n, &at_right)));
    nodes
}

/// `subtree` on one thread: split widest-axis at the median until leaves
/// are small.
fn serial_tree(
    corners: &(impl Fn(u32) -> [DVec3; 3] + Sync),
    centers: &[DVec3],
    order: &mut [u32],
    first: u32,
) -> Vec<Node> {
    let mut nodes = vec![Node {
        min: DVec3::ZERO,
        max: DVec3::ZERO,
        first: 0,
        count: order.len() as u32,
    }];
    let mut stack = vec![0usize];
    while let Some(at) = stack.pop() {
        let Node {
            first: start,
            count,
            ..
        } = nodes[at];
        let range = start as usize..(start + count) as usize;
        let (min, max) = bounds_of(corners, &order[range.clone()]);
        nodes[at].min = min;
        nodes[at].max = max;
        nodes[at].first = start + first;
        if range.len() <= LEAF {
            continue;
        }
        let mid = split(centers, &mut order[range], min, max) as u32;
        let left = nodes.len();
        for (s, c) in [(start, mid), (start + mid, count - mid)] {
            nodes.push(Node {
                min,
                max,
                first: s,
                count: c,
            });
        }
        nodes[at].first = left as u32;
        nodes[at].count = 0;
        stack.push(left);
        stack.push(left + 1);
    }
    nodes
}

/// Slab test: whether the ray reaches the box before `limit`.
fn slab(min: DVec3, max: DVec3, origin: DVec3, inv: DVec3, limit: f64) -> bool {
    let mut near = 0f64;
    let mut far = limit;
    for axis in 0..3 {
        let (o, i) = (origin[axis], inv[axis]);
        if i.is_infinite() {
            if o < min[axis] - 1e-9 || o > max[axis] + 1e-9 {
                return false;
            }
            continue;
        }
        let (mut t0, mut t1) = ((min[axis] - o) * i, (max[axis] - o) * i);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        near = near.max(t0 - 1e-9);
        far = far.min(t1 + 1e-9);
        if near > far {
            return false;
        }
    }
    true
}

/// Möller–Trumbore: the distance along `dir` (unit) of a hit in front of
/// `origin`, either side of the triangle.
fn ray_triangle(origin: DVec3, dir: DVec3, a: DVec3, b: DVec3, c: DVec3) -> Option<f64> {
    let e1 = b - a;
    let e2 = c - a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-18 {
        return None;
    }
    let inv = 1. / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(-1e-12..=1. + 1e-12).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < -1e-12 || u + v > 1. + 1e-12 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-12).then_some(t)
}

/// The point of triangle abc nearest `p` (Ericson, Real-Time Collision
/// Detection §5.1.5).
fn closest_on_triangle(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> DVec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1. / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// The background meshes of `scene`: shown, not display-hidden, and not
/// active; with a Target Item, only that one (none when it isn't such a
/// mesh: an active target doesn't constrain itself, BG-Q12).
pub fn target_ids(scene: &Scene, target: Option<u64>) -> Vec<u64> {
    let mut ids = background_ids(scene);
    if let Some(t) = target {
        ids.retain(|&id| id == t);
    }
    ids
}

/// Every background mesh of `scene` (no Target Item).
pub fn background_ids(scene: &Scene) -> Vec<u64> {
    let active = scene.active_meshes();
    let hidden = scene.display_hidden();
    scene
        .items
        .iter()
        .filter(|i| {
            i.is_mesh() && scene.shows(i) && !hidden.contains(&i.id) && !active.contains(&i.id)
        })
        .map(|i| i.id)
        .collect()
}

/// Newell's normal of the polygon through `points[face]`: the side it
/// faces as drawn, mirrored transforms included. `None` when degenerate.
fn newell(points: &[DVec3], face: &[u32]) -> Option<DVec3> {
    let mut n = DVec3::ZERO;
    for i in 0..face.len() {
        let p = points[face[i] as usize];
        let q = points[face[(i + 1) % face.len()] as usize];
        n += DVec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    n.try_normalize()
}

/// Adds one background mesh's surface, in world space: its displayed
/// smooth surface for smooth polygons, its own polygons for the rest.
fn add_mesh(
    points: &mut Vec<DVec3>,
    tris: &mut Vec<Tri>,
    mesh: &crate::Mesh,
    settings: &crate::subdivision::SubdivisionSettings,
    elements: &crate::element_state::ElementState,
    world: DMat4,
) -> Result<()> {
    // Only hidden polygons drop out; locked ones are still surface.
    let hidden_only = crate::element_state::ElementState {
        hidden: elements.hidden.clone(),
        ..Default::default()
    };
    let hidden = crate::element_state::Protection::of(mesh, &hidden_only).map(|p| p.face_hidden);
    let is_hidden = |f: usize| hidden.as_ref().is_some_and(|h| h[f]);
    // Smooth polygons: the pieces the viewport draws (BG-Q5), from the
    // shared evaluation. A piece of a hidden cage polygon is hidden with
    // it (BG-Q1).
    let smooth = crate::subdivision::has_smooth(mesh);
    if smooth {
        let (surface, sources) = crate::subdivision::display_surface(mesh, settings)?;
        let base = points.len() as u32;
        points.extend(surface.vertices.iter().map(|&v| world.transform_point3(v)));
        let world_points = &points[base as usize..];
        for (piece, face) in surface.faces.iter().enumerate() {
            let Some(&source) = sources.get(piece) else {
                continue;
            };
            if source >= mesh.faces.len()
                || is_hidden(source)
                || !crate::subdivision::is_smooth_polygon(mesh, source)
            {
                continue;
            }
            let v = &face.vertices;
            // The piece's own normal: the smooth surface's, as drawn.
            let Some(n) = newell(world_points, v) else {
                continue;
            };
            // Pieces are quads (or the polygon kept at level 0): a fan, as
            // the viewport picks them.
            for i in 1..v.len().saturating_sub(1) {
                tris.push(Tri {
                    v: [v[0], v[i], v[i + 1]].map(|k| base + k),
                    n,
                });
            }
        }
    }
    let base = points.len() as u32;
    points.extend(mesh.vertices.iter().map(|&v| world.transform_point3(v)));
    let world_points = &points[base as usize..];
    for f in 0..mesh.faces.len() {
        if is_hidden(f) || (smooth && crate::subdivision::is_smooth_polygon(mesh, f)) {
            continue;
        }
        // Curves and lines have no surface; broken polygons are skipped.
        let Ok(corners) = mesh.surface_triangles(f) else {
            continue;
        };
        let face = &mesh.faces[f].vertices;
        let Some(n) = newell(world_points, face) else {
            continue;
        };
        for [i, j, k] in corners {
            tris.push(Tri {
                v: [face[i], face[j], face[k]].map(|p| base + p),
                n,
            });
        }
    }
    Ok(())
}

/// The key a cached background was built for: each background mesh's
/// shared allocation (its polygons' SubD on/off kinds included),
/// subdivision settings (the display levels), transform and hidden
/// polygons.
type Key = Vec<(
    u64,
    crate::Shared<crate::Mesh>,
    crate::subdivision::SubdivisionSettings,
    DMat4,
    crate::element_state::ElementMask,
)>;

fn key_of(scene: &Scene, target: Option<u64>) -> Result<Key> {
    target_ids(scene, target)
        .into_iter()
        .map(|id| {
            let item = scene.item(id).ok_or("Missing item")?;
            Ok((
                id,
                item.mesh.shared().clone(),
                item.subdivision.clone().unwrap_or_default(),
                scene.world_matrix(id)?,
                item.elements.hidden.clone(),
            ))
        })
        .collect()
}

fn same_key(a: &Key, b: &Key) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.0 == y.0 && crate::Shared::same(&x.1, &y.1) && x.2 == y.2 && x.3 == y.3 && x.4 == y.4
        })
}

/// The last background built, reused while the background meshes don't
/// change (a drag never changes them). Not editor state.
#[derive(Clone, Default)]
pub struct BackgroundCache(Arc<Mutex<Option<Built>>>, Arc<std::sync::atomic::AtomicU64>);
/// A background and the key it was built for.
type Built = (Key, Arc<Background>);
impl PartialEq for BackgroundCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl std::fmt::Debug for BackgroundCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BackgroundCache")
    }
}
impl BackgroundCache {
    /// The background of `scene`, built only when it changed.
    pub fn get(&self, scene: &Scene, target: Option<u64>) -> Result<Arc<Background>> {
        let key = key_of(scene, target)?;
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, bg)) = slot.as_ref()
            && same_key(k, &key)
        {
            return Ok(bg.clone());
        }
        let bg = Arc::new(Background::of(scene, target)?);
        self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *slot = Some((key, bg.clone()));
        Ok(bg)
    }
    /// Backgrounds built so far (cache tests).
    pub fn builds(&self) -> u64 {
        self.1.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl crate::Editor {
    /// The view a Screen Axis search uses.
    pub fn constraint_view(&self) -> ConstraintView {
        self.background_view.0.unwrap_or_default()
    }
    /// Records the view the next tool use happens in (a press in a view).
    pub fn set_constraint_view(&mut self, view: ConstraintView) {
        if view.forward.is_finite()
            && view.forward.length() > 1e-9
            && view.eye.is_none_or(|e| e.is_finite())
        {
            self.background_view = LastView(Some(ConstraintView {
                forward: view.forward.normalize(),
                ..view
            }));
        }
    }
    /// The constraint when it is on and does something.
    fn working_constraint(&self) -> Option<BackgroundConstraint> {
        self.background_constraint
            .filter(|c| c.mode != GeometryConstraint::Off)
    }
    /// Turn the constraint on (with these options) or off. A live tool
    /// stays active and re-applies (the spec: turning it on keeps the
    /// current tool).
    pub(crate) fn set_background_constraint(
        &mut self,
        c: Option<BackgroundConstraint>,
    ) -> Result<()> {
        if let Some(c) = &c {
            c.validate()?;
        }
        let mut next = self.clone();
        next.background_constraint = c;
        if let Some(c) = c {
            next.background_constraint_preset = c;
        }
        if let Some(l) = next.session.live.as_ref().filter(|l| l.applied) {
            next.dispatch(crate::Command::Preview(l.params.clone()))?;
        }
        *self = next;
        Ok(())
    }
    /// The tool chain's constraint layer (MC-08), after the tool's own
    /// change, so after snapping too (snapping steers the tool's input):
    /// every foreground point the tool moved goes onto the background.
    /// Tools whose card leaves the layer out, and tools that add or
    /// remove points (Extrude, Bevel, …), are left alone (provisional,
    /// BG-Q8), as is Item mode (item transforms aren't points).
    pub(crate) fn constrain_preview(&mut self) -> Result<()> {
        let Some(c) = self.working_constraint() else {
            return Ok(());
        };
        let Some(live) = self.session.live.as_ref() else {
            return Ok(());
        };
        // A tool in a UV view moves UVs, not points on a surface.
        if live.tool.card().layers.constraint != crate::tool_card::Constraint::MovedPoints
            || live.params.uv.is_some()
        {
            return Ok(());
        }
        if self.scene.selection.mode == Mode::Item {
            return Ok(());
        }
        // TC-46: Topology Pen's moves stay free; only the points it makes
        // land on the background.
        if self
            .session
            .preset
            .as_ref()
            .is_some_and(|p| p.row().actor.topology_pen)
        {
            return Ok(());
        }
        let base = &live.base;
        let foreground = base.active_meshes();
        let background = self.background_cache.get(base, c.target)?;
        if background.is_empty() {
            return Ok(());
        }
        let view = self.constraint_view();
        let mut scene = self.scene.clone();
        let mut changed = false;
        for id in foreground {
            let (Some(before), Some(after)) = (base.item(id), scene.item(id)) else {
                continue;
            };
            if before.mesh.vertices.len() != after.mesh.vertices.len()
                || crate::Shared::same(before.mesh.shared(), after.mesh.shared())
            {
                continue;
            }
            let moved = crate::mesh::moved_points(&before.mesh.vertices, &after.mesh.vertices);
            if moved.is_empty() {
                continue;
            }
            let world = scene.world_matrix(id)?;
            let local = world.inverse();
            let starts: Vec<DVec3> = moved
                .iter()
                .map(|&v| world.transform_point3(before.mesh.vertices[v as usize]))
                .collect();
            let item = scene.item_mut(id).ok_or("Missing item")?;
            for (&v, start) in moved.iter().zip(starts) {
                let p = &mut item.mesh.vertices[v as usize];
                if let Some(on) =
                    background.constrain(&c, &view, Some(start), world.transform_point3(*p))
                {
                    *p = local.transform_point3(on);
                    changed = true;
                }
            }
        }
        if changed {
            for item in &scene.items {
                if let Some(before) = base.item(item.id) {
                    item.mesh.shared().prove_points_moved(before.mesh.shared());
                }
            }
            scene.validate()?;
            self.scene = scene;
        }
        Ok(())
    }
    /// A new point (world space) placed by a creation tool (Pen), onto
    /// the background of the current scene. Unchanged when the constraint
    /// is off or nothing is hit.
    pub(crate) fn constrain_new_point(&self, p: DVec3) -> Result<DVec3> {
        let Some(c) = self.working_constraint() else {
            return Ok(p);
        };
        let background = self.background_cache.get(&self.scene, c.target)?;
        Ok(background
            .constrain(&c, &self.constraint_view(), None, p)
            .unwrap_or(p))
    }
    /// A Make Quads partner guessed at `partner` for the click at `point`
    /// (both world), onto the background (PMQ-Q5). The rung
    /// `point → partner` keeps its length: it's laid flat on the surface
    /// under `point`, then settled onto the surface nearest it. Pushing it
    /// along the view instead would stretch each rung over a curved
    /// background, and the next guess copies the rung, so a strip round an
    /// eye would grow until it fell off the horizon. Unchanged when the
    /// constraint is off; a background it can't reach falls back to the
    /// ordinary new-point rule.
    pub(crate) fn constrain_strip_partner(&self, point: DVec3, partner: DVec3) -> Result<DVec3> {
        let Some(c) = self.working_constraint() else {
            return Ok(partner);
        };
        let background = self.background_cache.get(&self.scene, c.target)?;
        let length = (partner - point).length();
        let fallback = || {
            background
                .constrain(&c, &self.constraint_view(), None, partner)
                .unwrap_or(partner)
        };
        let Some(under) = background.nearest(point) else {
            return Ok(fallback());
        };
        let normal = under.normal.normalize_or_zero();
        let rung = partner - point;
        let flat = (rung - normal * rung.dot(normal)).normalize_or_zero();
        if flat == DVec3::ZERO || length < 1e-12 {
            return Ok(fallback());
        }
        // A few settles: land on the surface, then hold the rung's length.
        let mut guess = point + flat * length;
        let mut landed = guess;
        for _ in 0..4 {
            let Some(hit) = background.nearest(guess) else {
                return Ok(fallback());
            };
            landed = hit.point + hit.normal * c.applied_offset();
            let along = (landed - point).normalize_or_zero();
            if along == DVec3::ZERO {
                break;
            }
            guess = point + along * length;
        }
        Ok(landed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(z: f64, half: f64) -> Background {
        // Facing +Z (counter-clockwise seen from +Z).
        let a = DVec3::new(-half, -half, z);
        let b = DVec3::new(half, -half, z);
        let c = DVec3::new(half, half, z);
        let d = DVec3::new(-half, half, z);
        Background::build(
            vec![a, b, c, d],
            vec![
                Tri {
                    v: [0, 1, 2],
                    n: DVec3::Z,
                },
                Tri {
                    v: [0, 2, 3],
                    n: DVec3::Z,
                },
            ],
        )
    }
    fn front() -> ConstraintView {
        ConstraintView::default()
    }

    #[test]
    fn screen_axis_lands_on_the_quad() {
        let bg = quad(0., 1.);
        let c = BackgroundConstraint::default();
        let p = bg
            .constrain(&c, &front(), None, DVec3::new(0.4, 0.2, 5.))
            .unwrap();
        assert!((p - DVec3::new(0.4, 0.2, 0.)).length() < 1e-12, "{p}");
        // From behind the surface too: the whole line of sight is searched.
        let p = bg
            .constrain(&c, &front(), None, DVec3::new(0.4, 0.2, -3.))
            .unwrap();
        assert!((p - DVec3::new(0.4, 0.2, 0.)).length() < 1e-12, "{p}");
    }

    #[test]
    fn single_sided_ignores_back_faces() {
        let bg = quad(0., 1.);
        let back = ConstraintView {
            forward: DVec3::Z,
            eye: None,
        };
        let c = BackgroundConstraint::default();
        assert_eq!(
            bg.constrain(&c, &back, None, DVec3::new(0.1, 0.1, 2.)),
            None
        );
        let both = BackgroundConstraint {
            double_sided: true,
            offset: 0.1,
            ..c
        };
        // Double Sided: the back counts, and Offset goes toward the viewer.
        let p = bg
            .constrain(&both, &back, None, DVec3::new(0.1, 0.1, 2.))
            .unwrap();
        assert!((p - DVec3::new(0.1, 0.1, -0.1)).length() < 1e-12, "{p}");
    }

    #[test]
    fn perspective_searches_from_the_eye() {
        let bg = quad(0., 10.);
        let view = ConstraintView {
            forward: -DVec3::Z,
            eye: Some(DVec3::new(0., 0., 10.)),
        };
        let p = bg
            .constrain(
                &BackgroundConstraint::default(),
                &view,
                None,
                DVec3::new(1., 0., 5.),
            )
            .unwrap();
        assert!((p - DVec3::new(2., 0., 0.)).length() < 1e-9, "{p}");
    }

    #[test]
    fn nearest_on_a_big_grid_matches_brute_force() {
        let mut points = Vec::new();
        let mut tris = Vec::new();
        let mut corners = Vec::new();
        for i in 0..40 {
            for j in 0..40 {
                let (x, y) = (i as f64 * 0.1, j as f64 * 0.1);
                let h = |x: f64, y: f64| (x * 3.).sin() * 0.2 + (y * 2.).cos() * 0.1;
                let a = DVec3::new(x, y, h(x, y));
                let b = DVec3::new(x + 0.1, y, h(x + 0.1, y));
                let c = DVec3::new(x + 0.1, y + 0.1, h(x + 0.1, y + 0.1));
                let at = points.len() as u32;
                points.extend([a, b, c]);
                tris.push(Tri {
                    v: [at, at + 1, at + 2],
                    n: (b - a).cross(c - a).normalize(),
                });
                corners.push((a, b, c));
            }
        }
        let bg = Background::build(points, tris);
        for p in [
            DVec3::new(1.3, 2.2, 1.),
            DVec3::new(-1., 5., -2.),
            DVec3::new(3.9, 0.05, 0.),
        ] {
            let got = bg.nearest(p).unwrap().point;
            let want = corners
                .iter()
                .map(|&(a, b, c)| closest_on_triangle(p, a, b, c))
                .min_by(|a, b| {
                    (*a - p)
                        .length_squared()
                        .total_cmp(&(*b - p).length_squared())
                })
                .unwrap();
            assert!(((got - p).length() - (want - p).length()).abs() < 1e-12);
        }
    }

    /// Big enough that the tree's top is built on several threads: rays
    /// and nearest points still match brute force.
    #[test]
    fn threaded_tree_matches_brute_force() {
        let n = 260usize;
        let h = |x: f64, y: f64| (x * 3.).sin() * 0.2 + (y * 2.).cos() * 0.1;
        let mut points = Vec::new();
        for i in 0..=n {
            for j in 0..=n {
                let (x, y) = (i as f64 * 0.02, j as f64 * 0.02);
                points.push(DVec3::new(x, y, h(x, y)));
            }
        }
        let at = |i: usize, j: usize| (i * (n + 1) + j) as u32;
        let mut tris = Vec::new();
        for i in 0..n {
            for j in 0..n {
                for v in [
                    [at(i, j), at(i + 1, j), at(i + 1, j + 1)],
                    [at(i, j), at(i + 1, j + 1), at(i, j + 1)],
                ] {
                    let [a, b, c] = v.map(|k| points[k as usize]);
                    tris.push(Tri {
                        v,
                        n: (b - a).cross(c - a).normalize(),
                    });
                }
            }
        }
        assert!(tris.len() > 2 * THREADED);
        let bg = Background::build(points.clone(), tris.clone());
        let corner = |t: &Tri| t.v.map(|k| points[k as usize]);
        for p in [
            DVec3::new(1.3, 2.2, 1.),
            DVec3::new(4.1, 0.7, -0.4),
            DVec3::new(5.1, 5.1, 0.),
        ] {
            let got = bg.nearest(p).unwrap().point;
            let want = tris
                .iter()
                .map(|t| {
                    let [a, b, c] = corner(t);
                    (closest_on_triangle(p, a, b, c) - p).length()
                })
                .fold(f64::INFINITY, f64::min);
            assert!(((got - p).length() - want).abs() < 1e-12);
            let down = bg.ray(DVec3::new(p.x, p.y, 5.), -DVec3::Z, false);
            let z = down.unwrap().1.point.z;
            assert!((z - h(p.x, p.y)).abs() < 0.01, "{z}");
        }
    }

    #[test]
    fn negative_offset_needs_its_switch() {
        // On by default (MC-06): it only unlocks negative values.
        assert!(BackgroundConstraint::default().negative_offset);
        let c = BackgroundConstraint {
            offset: -0.1,
            negative_offset: false,
            ..Default::default()
        };
        assert!(c.validate().is_err());
        assert!(
            BackgroundConstraint {
                negative_offset: true,
                ..c
            }
            .validate()
            .is_ok()
        );
    }
}
