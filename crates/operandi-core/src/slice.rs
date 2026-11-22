//! Tier 1A slicing tools: Loop Slice, Edge Slice and Polygon Slice (spec
//! toolbox-panels-2026-11-06, EDIT.md "Slice section"; starting values from
//! STARTING-VALUES-1A-1B.md section 1A).
//!
//! All three are live tools: their options and their placed cuts live in
//! [`SliceParameters`], and every preview cuts the session's base mesh
//! afresh with the shared engine in `slice_cut.rs`. A drag or a typed value
//! is one tweak (mid-tool undo); Drop records the whole session as one
//! step named after the tool.
//!
//! Provisional readings are marked in the code and logged in
//! docs/SPECIFICATION_QUESTIONS.md as SL-Q01...
use crate::slice_cut::{Chord, EdgeCut, InnerPoint, Node, cut, faces_of_edge};
use crate::topology::edge_key;
use crate::{Mesh, Mode, Result, Scene, Tool};
use glam::DVec3;
use std::collections::BTreeSet;

/// A slice's result: the mesh, the new edges the cut made and the polygons
/// it cut (their pieces).
pub type SliceResult = (Mesh, Vec<(u32, u32)>, Vec<u32>);

/// Edge Slice's chord being built: its start node, the polygons it may run
/// through, the inside points so far, and the start's edge.
type ChordStart = (Node, Vec<usize>, Vec<(usize, DVec3)>, (u32, u32));

/// Loop Slice's Mode: where the slices sit across the ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopSliceMode {
    /// Each slice anywhere.
    #[default]
    Free,
    /// Evenly spaced.
    Uniform,
    /// Mirrored about 50 %.
    Symmetry,
}

/// Loop Slice's Edit: what a press on the slider bar does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopSliceEdit {
    #[default]
    Move,
    Add,
    Remove,
}

/// Loop Slice options and its rings.
#[derive(Clone, Debug, PartialEq)]
pub struct LoopSlice {
    pub edit: LoopSliceEdit,
    pub mode: LoopSliceMode,
    /// The active slice, counted from 0 in the order the slices were made
    /// (measured 2026-11-16; Free mode doesn't re-sort them by position).
    pub current: usize,
    /// Every slice's place across the ring (0..1); its length is Count.
    /// Mode is applied whenever this changes (see `normalize`).
    pub positions: Vec<f64>,
    /// The on-screen slider bar: length and top-left offset in points.
    pub length: f64,
    pub slider_x: f64,
    pub slider_y: f64,
    pub slice_selected: bool,
    /// With Slice Selected: polygons inside the selection that take slice
    /// points without being cut across (the ring's ends) get extra edges
    /// so they stay quads where edges alone can do it (provisional,
    /// SL-Q04). Ignored with Slice Selected off.
    pub keep_quads: bool,
    pub slice_ngon: bool,
    pub preserve_curvature: bool,
    /// 1.0 = 100 %.
    pub tension: f64,
    pub select_new: bool,
    /// Inset (LS-41): how far every new loop point is pushed along the
    /// surface normal, in metres; positive pushes in (a groove). With no
    /// profile every slice gets the same push (provisional, LS-Q10); with
    /// one and Keep Aspect off it is the profile's deepest point.
    pub inset: f64,
    /// Split (LS-33): every slice leaves two separate edge loops.
    pub split: bool,
    /// With Split: close each side of a closed loop with one polygon.
    pub cap_sections: bool,
    /// With Split: the width (metres) between the two loops, centred on
    /// the slice.
    pub gap: f64,
    /// The router profile with Reverse Direction, Reverse Inset and Keep
    /// Aspect (LS-40..LS-43). With a shape picked, its points decide the
    /// loops: Count and Position are set aside.
    pub profile: crate::ProfilePick,
    /// The rings to cut: an edge of each, in the session's base mesh.
    /// Set from the selection at activation or by clicking an edge.
    pub rings: Vec<(u32, u32)>,
}
impl Default for LoopSlice {
    fn default() -> Self {
        Self {
            edit: LoopSliceEdit::Move,
            mode: LoopSliceMode::Free,
            current: 0,
            positions: vec![0.5],
            length: 200.,
            slider_x: 20.,
            slider_y: 50.,
            slice_selected: false,
            keep_quads: false,
            slice_ngon: false,
            preserve_curvature: false,
            tension: 1.,
            select_new: true,
            inset: 0.,
            split: false,
            cap_sections: false,
            gap: 0.,
            profile: crate::ProfilePick::default(),
            rings: Vec::new(),
        }
    }
}

/// Keep slices this far from the ring's edges (and each other's ends).
const POSITION_MIN: f64 = 0.001;

impl LoopSlice {
    pub fn count(&self) -> usize {
        self.positions.len()
    }
    /// Evenly spaced positions for `n` slices.
    fn uniform_positions(n: usize) -> Vec<f64> {
        (1..=n).map(|i| i as f64 / (n + 1) as f64).collect()
    }
    /// Count: changing it re-spaces the slices evenly (provisional, SL-Q02).
    pub fn set_count(&mut self, n: usize) {
        let n = n.clamp(1, 100);
        if n != self.count() {
            self.positions = Self::uniform_positions(n);
            self.current = self.current.min(n - 1);
        }
        self.normalize(self.current);
    }
    /// The Uniform button: re-space the slices evenly.
    pub fn respace(&mut self) {
        self.positions = Self::uniform_positions(self.count());
    }
    /// Apply Mode after `source` changed: Uniform re-spaces, Symmetry
    /// mirrors every slice's partner from the left half (or from `source`
    /// when it is on the right), Free keeps the slices in the order they
    /// were made (measured 2026-11-16). Keeps `current` on the slice that
    /// moved.
    pub fn normalize(&mut self, source: usize) {
        let n = self.count();
        if n == 0 {
            self.positions = vec![0.5];
        }
        let n = self.count();
        for p in &mut self.positions {
            *p = if p.is_finite() {
                p.clamp(POSITION_MIN, 1. - POSITION_MIN)
            } else {
                0.5
            };
        }
        match self.mode {
            LoopSliceMode::Uniform => self.respace(),
            LoopSliceMode::Symmetry => {
                let source = source.min(n - 1);
                // Mirror pairs (i, n-1-i); the source side wins.
                for i in 0..n / 2 {
                    let j = n - 1 - i;
                    if source == j {
                        self.positions[i] = 1. - self.positions[j];
                    } else {
                        self.positions[j] = 1. - self.positions[i];
                    }
                }
                if n % 2 == 1 {
                    self.positions[n / 2] = 0.5;
                }
                // A left slice dragged past the middle swaps sides.
                self.positions.sort_by(f64::total_cmp);
            }
            LoopSliceMode::Free => {}
        }
        self.current = self.current.min(n - 1);
    }
    /// Position field / Move drag: slice `index` to `t`.
    pub fn set_position(&mut self, index: usize, t: f64) {
        if let Some(p) = self.positions.get_mut(index) {
            *p = t;
            self.current = index;
            self.normalize(index);
        }
    }
    /// Edit Add: a new slice at `t`, made current.
    pub fn add(&mut self, t: f64) {
        if self.count() >= 100 {
            return;
        }
        self.positions.push(t);
        let index = self.count() - 1;
        self.current = index;
        if self.mode == LoopSliceMode::Free {
            self.normalize(index);
        } else {
            // Uniform and Symmetry place the new slice themselves.
            self.normalize(index);
        }
    }
    /// Edit Remove: take slice `index` away (the last one stays).
    pub fn remove(&mut self, index: usize) -> bool {
        if self.count() <= 1 || index >= self.count() {
            return false;
        }
        self.positions.remove(index);
        self.current = self.current.min(self.count() - 1);
        self.normalize(self.current);
        true
    }
}

/// A cut point placed with Edge Slice, in the session's base mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SlicePoint {
    /// On edge `a`–`b`, `t` (0..1) from `a`.
    Edge { a: u32, b: u32, t: f64 },
    /// Inside polygon `face` at mesh-local `at` (Inside Polygons).
    Inside { face: u32, at: DVec3 },
}

/// Edge Slice's Show readout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliceShow {
    None,
    /// Percent along the edge.
    #[default]
    Position,
    /// Distance to both edge ends.
    Distance,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeSlice {
    pub points: Vec<SlicePoint>,
    /// The path was closed on its first point.
    pub closed: bool,
    pub split_middle: bool,
    pub split_polygons: bool,
    pub show: SliceShow,
    /// Not built (SL-Q08): greyed.
    pub offset: f64,
    /// Placement step, 0.005 = 0.5 %; 0 = free.
    pub snap: f64,
    pub inside: bool,
    pub always_raycast: bool,
    pub multi_slice: bool,
}
impl Default for EdgeSlice {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            closed: false,
            split_middle: false,
            split_polygons: true,
            show: SliceShow::Position,
            offset: 0.,
            snap: 0.005,
            inside: true,
            always_raycast: true,
            multi_slice: false,
        }
    }
}
impl EdgeSlice {
    /// A placement `t` on an edge with Snap Value applied, kept off the ends.
    pub fn snapped(&self, t: f64) -> f64 {
        let t = if self.snap > 0. {
            (t / self.snap).round() * self.snap
        } else {
            t
        };
        let margin = if self.snap > 0. { self.snap } else { 0.001 }.min(0.25);
        t.clamp(margin, 1. - margin)
    }
}

/// Polygon Slice's Offset Side: where the Gap sits relative to the plane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OffsetSide {
    /// Centred on the plane (first choice; label provisional, PS-Q2).
    #[default]
    Center,
    Above,
    Below,
}

/// One Polygon Slice cut: the drawn line, in world space, and the work
/// plane's normal at the time. The cutting plane holds the line and stands
/// perpendicular to the work plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneCut {
    pub start: DVec3,
    pub end: DVec3,
    pub up: DVec3,
}
impl PlaneCut {
    /// The plane's unit normal, or `None` for a line too short to define it.
    pub fn normal(&self) -> Option<DVec3> {
        let n = (self.end - self.start).cross(self.up);
        (n.length() > 1e-12).then(|| n.normalize())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolygonSlice {
    /// Every cut so far; the last is the current one the fields edit.
    pub cuts: Vec<PlaneCut>,
    /// Start/End shown before the first cut (STARTING-VALUES 1A).
    pub start: DVec3,
    pub end: DVec3,
    pub fast_slice: bool,
    pub angle_snap: bool,
    /// Degrees.
    pub angle: f64,
    pub split: bool,
    pub cap_sections: bool,
    pub gap: f64,
    pub offset_side: OffsetSide,
    pub infinite: bool,
    /// Set by the viewport while a Fast Slice drag is held: the current cut
    /// is drawn but not made yet.
    pub held: bool,
}
impl Default for PolygonSlice {
    fn default() -> Self {
        Self {
            cuts: Vec::new(),
            start: DVec3::new(-1., 0., 0.),
            end: DVec3::new(1., 0., 0.),
            fast_slice: false,
            angle_snap: true,
            angle: 45.,
            split: false,
            cap_sections: false,
            gap: 0.,
            offset_side: OffsetSide::Center,
            infinite: false,
            held: false,
        }
    }
}

/// Options and placed cuts of the three Slice tools. Each tool reads its
/// own part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SliceParameters {
    pub loop_slice: LoopSlice,
    pub edge_slice: EdgeSlice,
    pub polygon_slice: PolygonSlice,
    /// Wave 2 path slicing (`path_slice.rs`). Boxed: `Parameters` sits on
    /// the stack in many places and these are large.
    pub pen_slice: Box<crate::PenSlice>,
    pub curve_slice: Box<crate::CurveSlice>,
    pub primitive_slice: Box<crate::PrimitiveSlice>,
    /// Wave 3 Polygon Axis Slice (`axis_slice.rs`).
    pub axis_slice: Box<crate::AxisSlice>,
}
impl SliceParameters {
    /// The options kept for the next session: everything but the placed
    /// rings, points and cuts.
    pub fn options_only(&self) -> Self {
        let mut p = self.clone();
        p.loop_slice.rings.clear();
        p.loop_slice.current = 0;
        p.edge_slice.points.clear();
        p.edge_slice.closed = false;
        p.polygon_slice.cuts.clear();
        p.polygon_slice.held = false;
        p.pen_slice.points.clear();
        p.curve_slice.points.clear();
        p.primitive_slice.place = None;
        p.primitive_slice.held = false;
        p
    }
}

impl Tool {
    /// The Edit tab slice tools (their card's family).
    pub fn is_slice(self) -> bool {
        self.card().family == crate::tool_card::Family::Slice
    }
    /// The wave 2 tools that cut a path with the shared cutter.
    pub fn is_path_slice(self) -> bool {
        matches!(
            self,
            Self::CurveSlice | Self::PenSlice | Self::PrimitiveSlice
        )
    }
}

/// The rings the selection names: each selected edge (Edge mode), or each
/// edge shared by two selected polygons (Polygon mode).
pub fn rings_from_selection(scene: &Scene) -> Vec<(u32, u32)> {
    let Some(item) = scene.active() else {
        return Vec::new();
    };
    let s = &scene.selection;
    match s.mode {
        Mode::Edge => s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect(),
        Mode::Polygon if s.faces.len() >= 2 => {
            let selected: BTreeSet<u32> = s.faces.iter().copied().collect();
            let point_faces = item.mesh.point_faces();
            let mut rings = Vec::new();
            let mut seen = BTreeSet::new();
            for &f in &s.faces {
                let Some(face) = item.mesh.faces.get(f as usize) else {
                    continue;
                };
                let n = face.vertices.len();
                for i in 0..n {
                    let e = edge_key(face.vertices[i], face.vertices[(i + 1) % n]);
                    if !seen.insert(e) {
                        continue;
                    }
                    let shared = faces_of_edge(&item.mesh, &point_faces, e.0, e.1)
                        .into_iter()
                        .filter(|&g| g as u32 != f && selected.contains(&(g as u32)))
                        .count();
                    if shared > 0 {
                        rings.push(e);
                    }
                }
            }
            rings
        }
        _ => Vec::new(),
    }
}

/// The polygon band crossing a ring edge: its edges, oriented so the first
/// point is on the start edge's `a` side, and each cut polygon with its two
/// ring edges (indices into `edges`).
struct Ring {
    edges: Vec<(u32, u32)>,
    splits: Vec<(usize, [usize; 2])>,
    /// The band came back to its start edge (a loop with no ends).
    closed: bool,
}

/// The edge across polygon `f` from `p`–`q` (adjacent corners), oriented so
/// the first point is on `p`'s side: the opposite side of a quad, or of an
/// even n-gon with Slice N-gon on (provisional, SL-Q03). `None` ends the
/// ring (triangles, odd n-gons, n-gons with Slice N-gon off).
fn across(f: &[u32], p: u32, q: u32, ngon: bool) -> Option<(u32, u32)> {
    let n = f.len();
    if n < 4 || n % 2 == 1 || (n > 4 && !ngon) {
        return None;
    }
    let ip = f.iter().position(|&v| v == p)?;
    let iq = f.iter().position(|&v| v == q)?;
    let forward = (ip + 1) % n == iq;
    if !forward && (iq + 1) % n != ip {
        return None;
    }
    let k = n / 2 - 1;
    let (np, nq) = if forward {
        ((ip + n - k) % n, (iq + k) % n)
    } else {
        ((ip + k) % n, (iq + n - k) % n)
    };
    Some((f[np], f[nq]))
}

/// Walk the band in both directions from edge `a`→`b`. Polygons outside
/// `allowed` end the walk like a triangle does (they take the points but
/// aren't cut).
fn walk_ring(
    mesh: &Mesh,
    a: u32,
    b: u32,
    allowed: Option<&BTreeSet<usize>>,
    ngon: bool,
) -> Result<Ring> {
    let point_faces = mesh.point_faces();
    let start_faces = faces_of_edge(mesh, &point_faces, a, b);
    if start_faces.len() > 2 {
        return Err("Loop Slice needs a manifold edge".into());
    }
    let mut edges = vec![(a, b)];
    let mut splits: Vec<(usize, [usize; 2])> = Vec::new();
    let mut seen_faces: BTreeSet<usize> = BTreeSet::new();
    let mut seen_edges: BTreeSet<(u32, u32)> = BTreeSet::from([edge_key(a, b)]);
    let mut closed = false;
    for &first in &start_faces {
        if closed {
            break;
        }
        let (mut p, mut q, mut face, mut from) = (a, b, first, 0usize);
        loop {
            if seen_faces.contains(&face) || allowed.is_some_and(|s| !s.contains(&face)) {
                break;
            }
            let Some((np, nq)) = across(&mesh.faces[face].vertices, p, q, ngon) else {
                break;
            };
            seen_faces.insert(face);
            if seen_edges.contains(&edge_key(np, nq)) {
                // Back at the start: a closed ring.
                splits.push((face, [from, 0]));
                closed = true;
                break;
            }
            seen_edges.insert(edge_key(np, nq));
            edges.push((np, nq));
            let next = edges.len() - 1;
            splits.push((face, [from, next]));
            from = next;
            let others = faces_of_edge(mesh, &point_faces, np, nq);
            if others.len() > 2 {
                break;
            }
            let Some(g) = others.into_iter().find(|&g| g != face) else {
                break;
            };
            (p, q, face) = (np, nq, g);
        }
    }
    if splits.is_empty() {
        return Err("Loop Slice needs a quad next to the ring's edge".into());
    }
    Ok(Ring {
        edges,
        splits,
        closed,
    })
}

/// One loop laid across a ring edge: `t` orders it on the edge (and
/// interpolates corner data), `x` places it across the edge, `depth`
/// pushes it into the surface (metres).
#[derive(Clone, Copy, Debug)]
struct LoopPlace {
    t: f64,
    x: f64,
    depth: f64,
}

/// With Split and no Gap, the two loops of a slice sit this far apart in
/// `t` (so the engine keeps them as separate points) and are then placed on
/// the same spot.
const SPLIT_EPS: f64 = 1e-6;

/// The loops a profile makes across one edge: one per profile point. `t`
/// runs evenly from the first point's `x` to the last one's, so the points
/// keep the profile's order along the edge even where the shape doubles
/// back (an undercut) and corner data still interpolates sensibly
/// (provisional, LS-Q10).
fn profile_places(points: &[crate::ProfilePoint]) -> Vec<LoopPlace> {
    let n = points.len();
    let (first, last) = (points[0].x, points[n - 1].x);
    points
        .iter()
        .enumerate()
        .map(|(k, q)| {
            let t = if (last - first).abs() > 1e-6 && n > 1 {
                first + (last - first) * k as f64 / (n - 1) as f64
            } else {
                (k + 1) as f64 / (n + 1) as f64
            };
            LoopPlace {
                t,
                x: q.x,
                depth: q.depth,
            }
        })
        .collect()
}

/// A ring's cuts: for each ring edge and loop, the cut on the slice's low
/// side and (with Split) on its high side.
struct RingCuts {
    ring: Ring,
    cut_at: Vec<Vec<[Option<usize>; 2]>>,
    loops: usize,
}

/// Keep Quads chords for polygon `face`, whose corners with the slice
/// points on its sides are `ext` (in walking order): cut quads off it, each
/// starting at a slice point, while it has an even number of sides above
/// four and a non-degenerate quad can be cut (provisional, SL-Q04).
fn quad_chords(face: usize, ext: &[(Node, DVec3)]) -> Vec<Chord> {
    let mut poly: Vec<usize> = (0..ext.len()).collect();
    let mut chords = Vec::new();
    let scale = ext
        .iter()
        .fold(0f64, |m, e| m.max(e.1.abs().max_element()))
        .max(1.);
    let flat = |a: DVec3, b: DVec3, c: DVec3| (b - a).cross(c - b).length() < 1e-9 * scale * scale;
    let proper = |pts: &[DVec3]| {
        let n = pts.len();
        (0..n).all(|i| !flat(pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]))
    };
    while poly.len() > 4 && poly.len().is_multiple_of(2) {
        let m = poly.len();
        let pick = (0..m).find(|&i| {
            if !matches!(ext[poly[i]].0, Node::Cut(_)) {
                return false;
            }
            let quad: Vec<DVec3> = (0..4).map(|k| ext[poly[(i + k) % m]].1).collect();
            let rest: Vec<DVec3> = (3..=m).map(|k| ext[poly[(i + k) % m]].1).collect();
            proper(&quad) && proper(&rest)
        });
        let Some(i) = pick else {
            break;
        };
        chords.push(Chord {
            face,
            from: ext[poly[i]].0,
            to: ext[poly[(i + 3) % m]].0,
            through: Vec::new(),
        });
        let (r1, r2) = ((i + 1) % m, (i + 2) % m);
        let (hi, lo) = (r1.max(r2), r1.min(r2));
        poly.remove(hi);
        poly.remove(lo);
    }
    chords
}

/// Loop Slice geometry: `positions` slices (or the picked profile's loops)
/// across every ring, with Split, Cap Sections, Gap and Keep Quads.
/// Returns the mesh, the new loop edges and the cut polygons' pieces.
pub fn loop_slice_geometry(
    mesh: &Mesh,
    o: &LoopSlice,
    selected_faces: &[u32],
) -> Result<SliceResult> {
    if !o.tension.is_finite() || !(-10. ..=10.).contains(&o.tension) {
        return Err("Tension must be between -1000% and 1000%".into());
    }
    if !o.inset.is_finite() {
        return Err("Inset must be a number".into());
    }
    if !o.gap.is_finite() || o.gap < 0. {
        return Err("Gap must be zero or more".into());
    }
    let point_faces = mesh.point_faces();
    let allowed: Option<BTreeSet<usize>> = (o.slice_selected && !selected_faces.is_empty())
        .then(|| selected_faces.iter().map(|&f| f as usize).collect());
    let mut cuts: Vec<EdgeCut> = Vec::new();
    let mut chords: Vec<Chord> = Vec::new();
    let mut covered: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut rings: Vec<RingCuts> = Vec::new();
    // Split with no Gap: where each cut point goes once the strip is gone.
    let mut snap: Vec<(usize, DVec3)> = Vec::new();
    let counts = mesh.edge_counts();
    for &(a, b) in &o.rings {
        if covered.contains(&edge_key(a, b)) {
            continue;
        }
        if !counts.contains_key(&edge_key(a, b)) {
            return Err("A ring edge is no longer in the mesh".into());
        }
        let ring = match walk_ring(mesh, a, b, allowed.as_ref(), o.slice_ngon) {
            Ok(ring) => ring,
            // A ring outside Slice Selected's polygons is skipped.
            Err(_) if allowed.is_some() => continue,
            Err(e) => return Err(e),
        };
        covered.extend(ring.edges.iter().map(|&(x, y)| edge_key(x, y)));
        // How many of the ring's cut polygons use each ring edge: an edge
        // with another polygon beside it is where the cut stops inside the
        // mesh (a closed end).
        let mut cut_by = vec![0usize; ring.edges.len()];
        for &(_, [i, j]) in &ring.splits {
            cut_by[i] += 1;
            cut_by[j] += 1;
        }
        let mut cut_at = Vec::with_capacity(ring.edges.len());
        let mut loops = 0;
        for (e, &(p, q)) in ring.edges.iter().enumerate() {
            let faces = faces_of_edge(mesh, &point_faces, p, q);
            // LS-41: depth pushes against the edge's surface normal (the
            // mean of its polygons' normals), so a positive depth digs in.
            let normal = faces
                .iter()
                .map(|&f| mesh.normal(f))
                .sum::<DVec3>()
                .normalize_or_zero();
            let (pp, pq) = (mesh.vertices[p as usize], mesh.vertices[q as usize]);
            let width = pp.distance(pq);
            let places: Vec<LoopPlace> = match o.profile.lay(width, o.inset) {
                Some(points) => profile_places(&points),
                None => o
                    .positions
                    .iter()
                    .map(|&t| LoopPlace {
                        t,
                        x: t,
                        depth: o.inset,
                    })
                    .collect(),
            };
            loops = places.len();
            let closed_end = faces.len() > cut_by[e];
            // Gap: each side moves half of it along the edge (LS-33,
            // measured: Gap 0.1 on a 1 m ring gives loops at 45 % and 55 %).
            let shift = if width > 0. { o.gap / 2. / width } else { 0. };
            let at = |x: f64, depth: f64| {
                let on_edge = if o.preserve_curvature {
                    crate::edge_insert::curve_point(mesh, p, q, x, o.tension)
                } else {
                    pp.lerp(pq, x)
                };
                on_edge - normal * depth
            };
            let mut row = Vec::with_capacity(places.len());
            for place in &places {
                let mut push = |t: f64, x: f64| -> Result<usize> {
                    if t <= 1e-9 || t >= 1. - 1e-9 {
                        return Err("The Gap is wider than the ring".into());
                    }
                    cuts.push(EdgeCut {
                        a: p,
                        b: q,
                        t,
                        at: Some(at(x, place.depth)),
                    });
                    Ok(cuts.len() - 1)
                };
                if !o.split {
                    row.push([Some(push(place.t, place.x)?), None]);
                } else if shift > 0. {
                    let low = push(place.t - shift, place.x - shift)?;
                    let high = push(place.t + shift, place.x + shift)?;
                    row.push([Some(low), Some(high)]);
                } else {
                    // No Gap: both loops on the same spot. They are cut a
                    // hair apart (the strip between needs an area to be a
                    // valid polygon until Split takes it away) and put back
                    // together afterwards. Where the cut stops inside the
                    // mesh the split closes there: the high side meets the
                    // low side's point.
                    let low = push(place.t - SPLIT_EPS, place.x - SPLIT_EPS)?;
                    let high = (!closed_end)
                        .then(|| push(place.t + SPLIT_EPS, place.x + SPLIT_EPS))
                        .transpose()?;
                    let spot = at(place.x, place.depth);
                    snap.extend(std::iter::once(low).chain(high).map(|c| (c, spot)));
                    row.push([Some(low), high]);
                }
            }
            cut_at.push(row);
        }
        let sides = if o.split { 2 } else { 1 };
        for &(face, [i, j]) in &ring.splits {
            for (start, finish) in cut_at[i].iter().zip(&cut_at[j]) {
                for side in 0..sides {
                    let end = |c: &[Option<usize>; 2]| c[side].or(c[0]);
                    if let (Some(from), Some(to)) = (end(start), end(finish)) {
                        chords.push(Chord {
                            face,
                            from: Node::Cut(from),
                            to: Node::Cut(to),
                            through: Vec::new(),
                        });
                    }
                }
            }
        }
        rings.push(RingCuts {
            ring,
            cut_at,
            loops,
        });
    }
    if cuts.is_empty() {
        return Ok((mesh.clone(), Vec::new(), Vec::new()));
    }
    if o.keep_quads
        && let Some(allowed) = &allowed
    {
        chords.extend(keep_quads_chords(
            mesh,
            &point_faces,
            &cuts,
            &rings,
            allowed,
        ));
    }
    let r = cut(mesh, &cuts, &[], &chords)?;
    let (mut out, mut pieces) = (r.mesh, r.pieces);
    if o.split {
        (out, pieces) = split_loops(out, pieces, &r.cut_points, &rings, &snap, o.cap_sections)?;
    }
    // Edges a later step took away (Split's gap strips) aren't new edges.
    let existing: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    let mut new_edges = r.new_edges;
    new_edges.retain(|e| existing.contains(e));
    Ok((out, new_edges, pieces))
}

/// Keep Quads: the polygons inside the selection that take slice points
/// but aren't cut across (where the ring stops at a triangle or n-gon)
/// get chords that keep them quads.
fn keep_quads_chords(
    mesh: &Mesh,
    point_faces: &crate::topology_cache::PointFaces,
    cuts: &[EdgeCut],
    rings: &[RingCuts],
    allowed: &BTreeSet<usize>,
) -> Vec<Chord> {
    let split: BTreeSet<usize> = rings
        .iter()
        .flat_map(|r| r.ring.splits.iter().map(|s| s.0))
        .collect();
    // Each cut edge's cuts, from its low point, one per distinct place.
    let mut on_edge: std::collections::BTreeMap<(u32, u32), Vec<(f64, usize)>> = Default::default();
    for (i, c) in cuts.iter().enumerate() {
        let from_low = if c.a < c.b { c.t } else { 1. - c.t };
        on_edge
            .entry(edge_key(c.a, c.b))
            .or_default()
            .push((from_low, i));
    }
    for list in on_edge.values_mut() {
        list.sort_by(|x, y| x.0.total_cmp(&y.0));
        list.dedup_by(|x, y| (x.0 - y.0).abs() < 1e-9);
    }
    let mut ends: BTreeSet<usize> = BTreeSet::new();
    for &(a, b) in on_edge.keys() {
        for f in faces_of_edge(mesh, point_faces, a, b) {
            if allowed.contains(&f) && !split.contains(&f) {
                ends.insert(f);
            }
        }
    }
    let mut chords = Vec::new();
    for f in ends {
        let vs = &mesh.faces[f].vertices;
        let n = vs.len();
        let mut ext: Vec<(Node, DVec3)> = Vec::new();
        for i in 0..n {
            let (x, y) = (vs[i], vs[(i + 1) % n]);
            ext.push((Node::Point(x), mesh.vertices[x as usize]));
            if let Some(list) = on_edge.get(&edge_key(x, y)) {
                let mut ordered: Vec<&(f64, usize)> = list.iter().collect();
                if x > y {
                    ordered.reverse();
                }
                for &&(_, c) in &ordered {
                    let at = cuts[c].at.unwrap_or_else(|| {
                        let c = cuts[c];
                        mesh.vertices[c.a as usize].lerp(mesh.vertices[c.b as usize], c.t)
                    });
                    ext.push((Node::Cut(c), at));
                }
            }
        }
        chords.extend(quad_chords(f, &ext));
    }
    chords
}

/// Split: take away the strip between each slice's two loops (with no Gap
/// it has no area), then put the two loops of a slice with no Gap back on
/// one spot, and with Cap Sections close each side of a closed ring with a
/// polygon. Returns the mesh and the cut pieces (renumbered, with caps).
fn split_loops(
    mut mesh: Mesh,
    pieces: Vec<u32>,
    cut_points: &[u32],
    rings: &[RingCuts],
    snap: &[(usize, DVec3)],
    caps: bool,
) -> Result<(Mesh, Vec<u32>)> {
    // Every slice point: (ring, loop, side).
    let mut group: std::collections::BTreeMap<u32, (usize, usize, usize)> = Default::default();
    for (ri, rc) in rings.iter().enumerate() {
        for row in &rc.cut_at {
            for (k, sides) in row.iter().enumerate() {
                for (side, c) in sides.iter().enumerate() {
                    if let Some(c) = c {
                        group.insert(cut_points[*c], (ri, k, side));
                    }
                }
            }
        }
    }
    let strip: Vec<u32> = pieces
        .iter()
        .copied()
        .filter(|&f| {
            let vs = &mesh.faces[f as usize].vertices;
            let Some(&(ri, k, _)) = group.get(&vs[0]) else {
                return false;
            };
            let mut sides = [false; 2];
            for v in vs {
                match group.get(v) {
                    Some(&(r2, k2, s)) if (r2, k2) == (ri, k) => sides[s] = true,
                    _ => return false,
                }
            }
            sides[0] && sides[1]
        })
        .collect();
    let mut pieces = pieces;
    if !strip.is_empty() {
        let removal = crate::polygon_remove::remove_faces(&mesh, &strip)?;
        let mut remap = vec![None; mesh.faces.len()];
        for (new, &old) in removal.survivors.iter().enumerate() {
            remap[old] = Some(new as u32);
        }
        pieces = pieces.iter().filter_map(|&f| remap[f as usize]).collect();
        mesh = removal.mesh;
    }
    for &(c, at) in snap {
        mesh.vertices[cut_points[c] as usize] = at;
    }
    if caps {
        let edges_of = |mesh: &Mesh, f: u32| -> Vec<(u32, u32)> {
            let vs = &mesh.faces[f as usize].vertices;
            let n = vs.len();
            (0..n).map(|i| (vs[i], vs[(i + 1) % n])).collect()
        };
        for rc in rings.iter().filter(|rc| rc.ring.closed) {
            for k in 0..rc.loops {
                for side in 0..2 {
                    let mut ring_points: Vec<u32> = Vec::new();
                    for row in &rc.cut_at {
                        let Some(c) = row[k][side].or(row[k][0]) else {
                            continue;
                        };
                        let point = cut_points[c];
                        if ring_points.last() != Some(&point) {
                            ring_points.push(point);
                        }
                    }
                    if ring_points.len() < 3 {
                        continue;
                    }
                    // Wind the cap against the piece that shares its first
                    // side, so the surface stays consistently oriented.
                    let (u, v) = (ring_points[0], ring_points[1]);
                    let template = pieces.iter().copied().find(|&f| {
                        edges_of(&mesh, f)
                            .iter()
                            .any(|&e| e == (u, v) || e == (v, u))
                    });
                    if let Some(t) = template
                        && edges_of(&mesh, t).contains(&(u, v))
                    {
                        ring_points.reverse();
                    }
                    crate::vertex_commands::append_face(
                        &mut mesh,
                        ring_points,
                        template.map(|f| f as usize),
                    );
                    pieces.push(mesh.faces.len() as u32 - 1);
                }
            }
        }
    }
    mesh.validate()
        .map_err(|e| format!("Split produced an invalid mesh ({e}); nothing changed"))?;
    pieces.sort_unstable();
    pieces.dedup();
    Ok((mesh, pieces))
}

/// Edge Slice geometry for the placed `points` (base mesh). An inside point
/// joins the cut once the path leaves its polygon through an edge; until
/// then it changes nothing (provisional, SL-Q07).
pub fn edge_slice_geometry(mesh: &Mesh, o: &EdgeSlice) -> Result<(Mesh, Vec<(u32, u32)>)> {
    if o.points.is_empty() {
        return Ok((mesh.clone(), Vec::new()));
    }
    let mut cuts: Vec<EdgeCut> = Vec::new();
    let mut nodes: Vec<Option<Node>> = Vec::new();
    for p in &o.points {
        match *p {
            SlicePoint::Edge { a, b, t } => {
                nodes.push(Some(Node::Cut(cuts.len())));
                cuts.push(EdgeCut { a, b, t, at: None });
            }
            SlicePoint::Inside { face, .. } => {
                if face as usize >= mesh.faces.len() {
                    return Err("A slice point's polygon is no longer in the mesh".into());
                }
                nodes.push(None);
            }
        }
    }
    let mut inner: Vec<InnerPoint> = Vec::new();
    let mut chords: Vec<Chord> = Vec::new();
    if o.split_polygons {
        let point_faces = mesh.point_faces();
        let faces_of = |p: &SlicePoint| -> Vec<usize> {
            match *p {
                SlicePoint::Edge { a, b, .. } => faces_of_edge(mesh, &point_faces, a, b),
                SlicePoint::Inside { face, .. } => vec![face as usize],
            }
        };
        let mut order: Vec<usize> = (0..o.points.len()).collect();
        let close =
            o.closed && o.points.len() >= 3 && matches!(o.points[0], SlicePoint::Edge { .. });
        if close {
            order.push(0);
        }
        // The chord being built: start node, its candidate polygons and
        // the inside points so far.
        let mut start: Option<ChordStart> = None;
        for &i in &order {
            let point = o.points[i];
            let candidates = faces_of(&point);
            match (point, start.take()) {
                (SlicePoint::Inside { face, at }, Some((node, faces, mut through, edge))) => {
                    if !faces.contains(&(face as usize)) {
                        return Err(
                            "Each slice point must be on a polygon next to the last one".into()
                        );
                    }
                    through.push((face as usize, at));
                    start = Some((node, vec![face as usize], through, edge));
                }
                (SlicePoint::Inside { .. }, None) => {
                    // A path can't start inside a polygon; the point waits.
                }
                (SlicePoint::Edge { a, b, .. }, previous) => {
                    let node = nodes[i].ok_or("Missing slice point")?;
                    if let Some((from, faces, through, edge)) = previous {
                        let shared: Vec<usize> = faces
                            .iter()
                            .copied()
                            .filter(|f| candidates.contains(f))
                            .collect();
                        let Some(&face) = shared.first() else {
                            return Err(
                                "Each slice point must be on a polygon next to the last one".into(),
                            );
                        };
                        // Two points on one edge with nothing between: no cut.
                        if edge_key(a, b) != edge || !through.is_empty() {
                            let through_ids = through
                                .iter()
                                .map(|&(face, at)| {
                                    inner.push(InnerPoint { face, at });
                                    inner.len() - 1
                                })
                                .collect();
                            chords.push(Chord {
                                face,
                                from,
                                to: node,
                                through: through_ids,
                            });
                        }
                    }
                    start = Some((node, candidates, Vec::new(), edge_key(a, b)));
                }
            }
        }
    }
    let r = cut(mesh, &cuts, &inner, &chords)?;
    Ok((r.mesh, r.new_edges))
}

/// How far each side of a Split's cut moves away from the plane.
fn gap_shares(o: &PolygonSlice) -> (f64, f64) {
    let gap = o.gap.max(0.);
    match o.offset_side {
        OffsetSide::Center => (gap / 2., gap / 2.),
        OffsetSide::Above => (gap, 0.),
        OffsetSide::Below => (0., gap),
    }
}

/// One plane cut of `mesh` (mesh-local plane through `origin` with unit
/// `normal`, the drawn line running `along` for `length`). Only polygons in
/// `considered` are cut. Returns the mesh, the new edges, every piece of a
/// cut polygon, and the points on the plane that the cut runs through.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plane_cut(
    mesh: &Mesh,
    origin: DVec3,
    normal: DVec3,
    along: DVec3,
    length: f64,
    infinite: bool,
    considered: &BTreeSet<usize>,
) -> Result<(SliceResult, Vec<u32>)> {
    let scale = mesh
        .vertices
        .iter()
        .fold(0f64, |m, v| m.max(v.abs().max_element()))
        .max(1.);
    let eps = 1e-9 * scale;
    let side = |v: u32| {
        let s = normal.dot(mesh.vertices[v as usize] - origin);
        if s.abs() < eps { 0. } else { s }
    };
    let mut cuts: Vec<EdgeCut> = Vec::new();
    let mut chords: Vec<Chord> = Vec::new();
    let mut on_plane: Vec<u32> = Vec::new();
    for &fi in considered {
        if mesh.is_polyline(fi) {
            continue;
        }
        let f = &mesh.faces[fi].vertices;
        let n = f.len();
        let s: Vec<f64> = f.iter().map(|&v| side(v)).collect();
        if s.iter().all(|&x| x >= 0.) || s.iter().all(|&x| x <= 0.) {
            continue;
        }
        // The nonzero side before and after each corner.
        let prev_sign = |i: usize| {
            (1..n)
                .map(|k| s[(i + n - k) % n])
                .find(|x| *x != 0.)
                .unwrap_or(0.)
        };
        let next_sign = |i: usize| {
            (1..n)
                .map(|k| s[(i + k) % n])
                .find(|x| *x != 0.)
                .unwrap_or(0.)
        };
        // Crossings in boundary order with their place along the line.
        let mut crossings: Vec<(f64, Node, DVec3)> = Vec::new();
        for i in 0..n {
            let (x, y) = (f[i], f[(i + 1) % n]);
            if s[i] == 0. {
                if prev_sign(i).signum() != next_sign(i).signum() {
                    let at = mesh.vertices[x as usize];
                    crossings.push((along.dot(at - origin), Node::Point(x), at));
                }
            } else if s[(i + 1) % n] != 0. && s[i].signum() != s[(i + 1) % n].signum() {
                // From the low point, so both polygons of the edge agree.
                let (low, high) = edge_key(x, y);
                let (sl, sh) = (side(low), side(high));
                let t = sl / (sl - sh);
                let at = mesh.vertices[low as usize].lerp(mesh.vertices[high as usize], t);
                let index = cuts.len();
                cuts.push(EdgeCut {
                    a: low,
                    b: high,
                    t,
                    at: None,
                });
                crossings.push((along.dot(at - origin), Node::Cut(index), at));
            }
        }
        if crossings.len() < 2 {
            continue;
        }
        crossings.sort_by(|x, y| x.0.total_cmp(&y.0));
        for pair in crossings.as_chunks::<2>().0 {
            // Infinite off: only where the drawn segment reaches (PS-Q1:
            // the segment's extent along the line, provisional).
            if !infinite && (pair[1].0 < -eps || pair[0].0 > length + eps) {
                continue;
            }
            for c in pair {
                if let Node::Point(v) = c.1 {
                    on_plane.push(v);
                }
            }
            chords.push(Chord {
                face: fi,
                from: pair[0].1,
                to: pair[1].1,
                through: Vec::new(),
            });
        }
    }
    // Cuts no chord uses would only add stray points: keep the used ones.
    let used: BTreeSet<usize> = chords
        .iter()
        .flat_map(|c| [c.from, c.to])
        .filter_map(|n| match n {
            Node::Cut(i) => Some(i),
            Node::Point(_) => None,
        })
        .collect();
    let mut kept: Vec<EdgeCut> = Vec::new();
    let mut new_index = vec![usize::MAX; cuts.len()];
    for (i, c) in cuts.iter().enumerate() {
        if used.contains(&i) {
            new_index[i] = kept.len();
            kept.push(*c);
        }
    }
    for c in &mut chords {
        for node in [&mut c.from, &mut c.to] {
            if let Node::Cut(i) = node {
                *i = new_index[*i];
            }
        }
    }
    if chords.is_empty() {
        return Ok(((mesh.clone(), Vec::new(), Vec::new()), Vec::new()));
    }
    let r = cut(mesh, &kept, &[], &chords)?;
    on_plane.extend(r.cut_points.iter().copied());
    on_plane.sort_unstable();
    on_plane.dedup();
    Ok(((r.mesh, r.new_edges, r.pieces), on_plane))
}

/// Split: give the polygons on the plane's positive side their own copies
/// of the points on the cut, then move each side's points away from the
/// plane by its Gap share along their edges (or the normal for points that
/// were already on the plane). With Cap Sections, close every loop of new
/// edges on each side with a polygon.
#[allow(clippy::too_many_arguments)]
fn split_sides(
    mesh: &mut Mesh,
    origin: DVec3,
    normal: DVec3,
    pieces: &[u32],
    on_plane: &[u32],
    new_edges: &[(u32, u32)],
    o: &PolygonSlice,
) -> Result<Vec<u32>> {
    let on: BTreeSet<u32> = on_plane.iter().copied().collect();
    let side_of = |mesh: &Mesh, f: usize| normal.dot(mesh.center(f) - origin);
    let mut copy: std::collections::BTreeMap<u32, u32> = Default::default();
    let positive: Vec<u32> = pieces
        .iter()
        .copied()
        .filter(|&f| side_of(mesh, f as usize) > 0.)
        .collect();
    for &f in &positive {
        let face = &mut mesh.faces[f as usize];
        for v in &mut face.vertices {
            if on.contains(v) {
                let next = mesh.vertices.len() as u32 + copy.len() as u32;
                *v = *copy.entry(*v).or_insert(next);
            }
        }
    }
    let originals: Vec<u32> = copy.keys().copied().collect();
    for &v in &originals {
        mesh.vertices.push(mesh.vertices[v as usize]);
    }
    let (above, below) = gap_shares(o);
    if above > 0. || below > 0. {
        // Each moved point slides along the side edge it sits on, toward
        // its own side, until it is its share away from the plane.
        let point_faces = crate::topology_cache::PointFaces::new(mesh);
        let copies: BTreeSet<u32> = copy.values().copied().collect();
        let neighbours = |mesh: &Mesh, v: u32| -> Vec<u32> {
            let mut out = Vec::new();
            for &f in point_faces.of(v) {
                let f = &mesh.faces[f as usize];
                let n = f.vertices.len();
                if let Some(i) = f.vertices.iter().position(|&x| x == v) {
                    out.push(f.vertices[(i + 1) % n]);
                    out.push(f.vertices[(i + n - 1) % n]);
                }
            }
            out
        };
        let mut moves: Vec<(u32, DVec3)> = Vec::new();
        for (&original, &duplicate) in &copy {
            for (v, sign, share) in [(original, -1., below), (duplicate, 1., above)] {
                if share <= 0. {
                    continue;
                }
                let at = mesh.vertices[v as usize];
                let target = neighbours(mesh, v)
                    .into_iter()
                    .filter(|&w| !on.contains(&w) && !copies.contains(&w))
                    .map(|w| mesh.vertices[w as usize])
                    .filter(|w| normal.dot(*w - origin) * sign > 1e-12)
                    .max_by(|x, y| {
                        let dx = (*x - at).normalize_or_zero().dot(normal * sign);
                        let dy = (*y - at).normalize_or_zero().dot(normal * sign);
                        dx.total_cmp(&dy)
                    });
                let moved = match target {
                    Some(w) => {
                        let rise = normal.dot(w - at) * sign;
                        at + (w - at) * (share / rise).min(1.)
                    }
                    None => at + normal * sign * share,
                };
                moves.push((v, moved));
            }
        }
        for (v, p) in moves {
            mesh.vertices[v as usize] = p;
        }
    }
    let mut caps: Vec<u32> = Vec::new();
    if o.cap_sections {
        // Loops of new edges on the negative side; the positive side's caps
        // use the copies and face the other way.
        let mut next: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
        for &(a, b) in new_edges {
            next.entry(a).or_default().push(b);
            next.entry(b).or_default().push(a);
        }
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let template = pieces.first().map(|&f| f as usize);
        for &startp in next.keys() {
            if seen.contains(&startp) || next[&startp].len() != 2 {
                continue;
            }
            let mut ring = vec![startp];
            let mut prev = startp;
            let mut at = next[&startp][0];
            let mut closed = false;
            while ring.len() <= next.len() {
                if at == startp {
                    closed = true;
                    break;
                }
                let Some(list) = next.get(&at).filter(|l| l.len() == 2) else {
                    break;
                };
                ring.push(at);
                let following = if list[0] == prev { list[1] } else { list[0] };
                prev = at;
                at = following;
            }
            seen.extend(ring.iter().copied());
            if !closed || ring.len() < 3 {
                continue;
            }
            // The negative side's cap faces +normal (out of its volume).
            let mut loop_points = ring.clone();
            let newell = |pts: &[u32], mesh: &Mesh| {
                let mut n = DVec3::ZERO;
                for i in 0..pts.len() {
                    let (p, q) = (
                        mesh.vertices[pts[i] as usize],
                        mesh.vertices[pts[(i + 1) % pts.len()] as usize],
                    );
                    n += p.cross(q);
                }
                n
            };
            if newell(&loop_points, mesh).dot(normal) < 0. {
                loop_points.reverse();
            }
            crate::vertex_commands::append_face(mesh, loop_points.clone(), template);
            caps.push(mesh.faces.len() as u32 - 1);
            if !copy.is_empty() {
                let mut other: Vec<u32> = loop_points
                    .iter()
                    .map(|v| copy.get(v).copied().unwrap_or(*v))
                    .collect();
                other.reverse();
                crate::vertex_commands::append_face(mesh, other, template);
                caps.push(mesh.faces.len() as u32 - 1);
            }
        }
    }
    mesh.validate()
        .map_err(|e| format!("Split produced an invalid mesh ({e}); nothing changed"))?;
    Ok(caps)
}

/// Polygon Slice geometry: every cut in turn, each on the selected polygons
/// (and the pieces earlier cuts made of them), or on every polygon when
/// none are selected. `world` maps the mesh to world space (the cuts are
/// world-space lines). Returns the mesh, the new edges and the cut pieces.
pub fn polygon_slice_geometry(
    mesh: &Mesh,
    world: glam::DMat4,
    o: &PolygonSlice,
    selected_faces: &[u32],
) -> Result<SliceResult> {
    if !o.gap.is_finite() || o.gap < 0. {
        return Err("Gap must be zero or positive".into());
    }
    let local = world.inverse();
    let mut out = mesh.clone();
    let mut considered: BTreeSet<usize> = if selected_faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        selected_faces.iter().map(|&f| f as usize).collect()
    };
    let mut all_edges: Vec<(u32, u32)> = Vec::new();
    let mut all_pieces: BTreeSet<u32> = BTreeSet::new();
    let count = if o.fast_slice && o.held {
        o.cuts.len().saturating_sub(1)
    } else {
        o.cuts.len()
    };
    for c in &o.cuts[..count] {
        let (start, end) = (
            local.transform_point3(c.start),
            local.transform_point3(c.end),
        );
        let up = local.transform_vector3(c.up);
        let line = end - start;
        let normal = line.cross(up);
        if line.length() < 1e-12 || normal.length() < 1e-12 {
            continue;
        }
        let normal = normal.normalize();
        let before = out.faces.len();
        let ((mesh, edges, pieces), on_plane) = plane_cut(
            &out,
            start,
            normal,
            line.normalize(),
            line.length(),
            o.infinite,
            &considered,
        )?;
        out = mesh;
        considered.extend(before..out.faces.len());
        if o.split && !pieces.is_empty() {
            let caps = split_sides(&mut out, start, normal, &pieces, &on_plane, &edges, o)?;
            all_pieces.extend(caps);
        }
        all_edges.extend(edges);
        all_pieces.extend(pieces);
    }
    // Edges an earlier cut made may have been split by a later one.
    let existing: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    all_edges.retain(|e| existing.contains(e));
    all_edges.sort_unstable();
    all_edges.dedup();
    Ok((out, all_edges, all_pieces.into_iter().collect()))
}

/// Remove selected edges and polygons that the cut renumbered away.
fn keep_valid_selection(scene: &mut Scene) {
    let Some(item) = scene.active() else {
        return;
    };
    let faces = item.mesh.faces.len() as u32;
    let points = item.mesh.vertices.len() as u32;
    let edges: BTreeSet<(u32, u32)> = if scene.selection.edges.is_empty() {
        BTreeSet::new()
    } else {
        item.mesh.edges().into_iter().collect()
    };
    let s = &mut scene.selection;
    s.edges.retain(|&(a, b)| edges.contains(&edge_key(a, b)));
    s.faces.retain(|&f| f < faces);
    s.vertices.retain(|&v| v < points);
}

/// Loop Slice rings plus, with Symmetry on, each ring edge's mirror edge
/// (oriented from the mirror of its first point) when it isn't one of the
/// rings already. A ring that is its own mirror finds its mirror edge in
/// its own band, which `loop_slice_geometry` then skips as covered.
fn mirrored_rings(mesh: &Mesh, rings: &[(u32, u32)], symmetry: crate::Symmetry) -> Vec<(u32, u32)> {
    let mut out = rings.to_vec();
    if symmetry.axis.is_none() || rings.is_empty() {
        return out;
    }
    let points: Vec<u32> = rings.iter().flat_map(|&(a, b)| [a, b]).collect();
    let partners = crate::symmetry::partners_of(mesh, symmetry, &points);
    let counts = mesh.edge_counts();
    let known: BTreeSet<(u32, u32)> = rings.iter().map(|&(a, b)| edge_key(a, b)).collect();
    for pair in partners.chunks(2) {
        if let [Some(a), Some(b)] = *pair
            && a != b
            && counts.contains_key(&edge_key(a, b))
            && !known.contains(&edge_key(a, b))
        {
            out.push((a, b));
        }
    }
    out
}

/// Whether two Polygon Slice cuts lie in the same plane.
fn same_plane(a: &PlaneCut, b: &PlaneCut, tolerance: f64) -> bool {
    match (a.normal(), b.normal()) {
        (Some(n), Some(m)) => {
            n.dot(m).abs() > 1. - 1e-9 && n.dot(b.start - a.start).abs() <= tolerance
        }
        _ => false,
    }
}

/// Edge Slice's path mirrored onto the other side, with its points named
/// in `after` (the mesh the first cut made), found by position. `None` with
/// Symmetry off, when the mirrored path lands on the drawn one (a path
/// along the plane), or when a mirrored point's edge or polygon was cut by
/// the first path (a path across the plane is cut once, provisional).
fn mirrored_edge_slice(
    base: &Mesh,
    after: &Mesh,
    o: &EdgeSlice,
    symmetry: crate::Symmetry,
) -> Option<EdgeSlice> {
    symmetry.axis?;
    if o.points.is_empty() {
        return None;
    }
    let tolerance = crate::symmetry::match_tolerance(base);
    let at = |v: u32| base.vertices.get(v as usize).copied();
    let mut cuts = Vec::new();
    for q in &o.points {
        cuts.push(match *q {
            SlicePoint::Edge { a, b, t } => at(a)?.lerp(at(b)?, t),
            SlicePoint::Inside { at, .. } => at,
        });
    }
    let mirrored: Vec<DVec3> = cuts.iter().map(|&q| symmetry.mirror(q)).collect();
    if crate::symmetry_sides::same_points(&cuts, &mirrored, tolerance) {
        return None;
    }
    let mut points = Vec::new();
    for q in &o.points {
        points.push(match *q {
            SlicePoint::Edge { a, b, t } => {
                let ends = [symmetry.mirror(at(a)?), symmetry.mirror(at(b)?)];
                let found = crate::symmetry_sides::points_at(after, &ends, tolerance);
                let (a, b) = (found[0]?, found[1]?);
                if !after.edge_counts().contains_key(&edge_key(a, b)) {
                    return None;
                }
                SlicePoint::Edge { a, b, t }
            }
            SlicePoint::Inside { face, at } => {
                let f = base.faces.get(face as usize)?;
                let c = symmetry.mirror(base.center(face as usize));
                let found =
                    crate::symmetry_sides::faces_at(after, &[(c, f.vertices.len())], tolerance);
                SlicePoint::Inside {
                    face: found[0]?,
                    at: symmetry.mirror(at),
                }
            }
        });
    }
    Some(EdgeSlice {
        points,
        ..o.clone()
    })
}

/// The live preview of a Slice tool: cut the base scene's active mesh.
pub(crate) fn preview(
    tool: Tool,
    scene: &mut Scene,
    p: &SliceParameters,
    symmetry: crate::Symmetry,
) -> Result<()> {
    let item = scene.active().ok_or("Select a mesh to slice")?;
    let mode = scene.selection.mode;
    let selected_faces = if mode == Mode::Polygon {
        scene.selection.faces.clone()
    } else {
        Vec::new()
    };
    match tool {
        Tool::LoopSlice => {
            // SYM3-10 / class D: each ring's mirror ring is cut too, started
            // from the mirrored edge so the cut lands at the mirror position.
            let mut o = p.loop_slice.clone();
            o.rings = mirrored_rings(&item.mesh, &o.rings, symmetry);
            let (mesh, edges, pieces) = loop_slice_geometry(&item.mesh, &o, &selected_faces)?;
            scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
            keep_valid_selection(scene);
            if p.loop_slice.select_new && !edges.is_empty() {
                match mode {
                    Mode::Edge => scene.selection.edges = edges,
                    Mode::Polygon => scene.selection.faces = pieces,
                    _ => {}
                }
            }
        }
        Tool::EdgeSlice => {
            let (mut mesh, _) = edge_slice_geometry(&item.mesh, &p.edge_slice)?;
            // SYM3-10 / class D: the mirrored path is cut too, found again
            // in the first cut's result by position.
            if let Some(o) = mirrored_edge_slice(&item.mesh, &mesh, &p.edge_slice, symmetry) {
                mesh = edge_slice_geometry(&mesh, &o)?.0;
            }
            scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
            keep_valid_selection(scene);
        }
        Tool::PolygonSlice => {
            let world = scene.world_matrix(item.id)?;
            // SYM3-10 / class D: each cut's mirror image is cut as well,
            // unless it is the same plane.
            let mut o = p.polygon_slice.clone();
            if let Some(m) = crate::symmetry_sides::WorldMirror::new(symmetry, world) {
                let tolerance = crate::symmetry_sides::world_tolerance(&item.mesh, world);
                for c in p.polygon_slice.cuts.clone() {
                    let mirrored = PlaneCut {
                        start: m.point(c.start),
                        end: m.point(c.end),
                        up: m.vector(c.up),
                    };
                    if !o.cuts.iter().any(|x| same_plane(x, &mirrored, tolerance)) {
                        o.cuts.insert(o.cuts.len().saturating_sub(1), mirrored);
                    }
                }
            }
            let (mesh, _, _) = polygon_slice_geometry(&item.mesh, world, &o, &selected_faces)?;
            scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
            keep_valid_selection(scene);
        }
        Tool::PenSlice | Tool::CurveSlice | Tool::PrimitiveSlice => {
            let (paths, cutter, action, held) = match tool {
                Tool::PenSlice => {
                    let o = &p.pen_slice;
                    (o.paths(), &o.cutter, o.plane_normal, false)
                }
                Tool::CurveSlice => {
                    let o = &p.curve_slice;
                    (o.paths(), &o.cutter, o.plane_normal, false)
                }
                _ => {
                    let o = &p.primitive_slice;
                    let action = o.place.map_or(DVec3::Y, |s| s.normal);
                    (o.paths(), &o.cutter, action, o.lazy_apply && o.held)
                }
            };
            if held {
                // Lazy Apply: the shape is drawn while the drag is held;
                // the cut comes on release.
                return Ok(());
            }
            let direction = cutter
                .direction(action)
                .ok_or("The Custom axis needs a direction")?;
            let world = scene.world_matrix(item.id)?;
            let mut r = crate::path_slice::path_cut(
                &item.mesh,
                world,
                &paths,
                direction,
                cutter,
                &selected_faces,
            )?;
            // SYM3-10 / class D: the mirrored path cuts the other side, in
            // the mirrored direction, unless it lands on the drawn path.
            if let Some(m) = crate::symmetry_sides::WorldMirror::new(symmetry, world) {
                let tolerance = crate::symmetry_sides::world_tolerance(&item.mesh, world);
                let mirrored: Vec<crate::path_slice::CutPath> = paths
                    .iter()
                    .map(|c| crate::path_slice::CutPath {
                        points: c.points.iter().map(|&q| m.point(q)).collect(),
                        closed: c.closed,
                    })
                    .collect();
                let flat = |v: &[crate::path_slice::CutPath]| -> Vec<DVec3> {
                    v.iter().flat_map(|c| c.points.iter().copied()).collect()
                };
                if !crate::symmetry_sides::same_points(&flat(&paths), &flat(&mirrored), tolerance) {
                    let faces = if selected_faces.is_empty() {
                        Vec::new()
                    } else {
                        r.selected.clone()
                    };
                    r = crate::path_slice::path_cut(
                        &r.mesh,
                        world,
                        &mirrored,
                        m.vector(direction),
                        cutter,
                        &faces,
                    )?;
                }
            }
            scene.active_mut().ok_or("Select a mesh")?.mesh = r.mesh.into();
            if mode == Mode::Polygon {
                scene.selection.faces = r.selected;
            }
            keep_valid_selection(scene);
        }
        Tool::AxisSlice => {
            let world = scene.world_matrix(item.id)?;
            let (mut mesh, _, mut pieces) = crate::axis_slice::axis_slice_geometry(
                &item.mesh,
                world,
                &p.axis_slice,
                &selected_faces,
            )?;
            // SYM3-10 / class D: an Offset across the plane is cut once more
            // with the Offset mirrored, so the cuts come in mirror pairs
            // (provisional: assumes the box is centred on the plane, as it
            // is for a symmetric selection).
            if let Some(m) = crate::symmetry_sides::WorldMirror::new(symmetry, world) {
                let o = &p.axis_slice;
                let offset = m.vector(o.offset);
                if !o.hold && offset.distance(o.offset) > 1e-12 {
                    let mut faces = selected_faces.clone();
                    if !faces.is_empty() {
                        faces.extend(&pieces);
                    }
                    let second = crate::AxisSlice {
                        offset,
                        ..o.as_ref().clone()
                    };
                    let (after, _, more) =
                        crate::axis_slice::axis_slice_geometry(&mesh, world, &second, &faces)?;
                    mesh = after;
                    pieces.extend(more);
                }
            }
            scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
            // The selected polygons' pieces stay selected (provisional).
            if mode == Mode::Polygon && !selected_faces.is_empty() {
                let mut faces = selected_faces.clone();
                faces.extend(pieces);
                faces.sort_unstable();
                faces.dedup();
                scene.selection.faces = faces;
            }
            keep_valid_selection(scene);
        }
        _ => return Err("Not a Slice tool".into()),
    }
    Ok(())
}

/// Starting parameters of a Slice session: the kept options, with Loop
/// Slice's rings from the selection.
pub(crate) fn start(tool: Tool, scene: &Scene, preset: &SliceParameters) -> SliceParameters {
    let mut p = preset.options_only();
    if tool == Tool::LoopSlice {
        p.loop_slice.rings = rings_from_selection(scene);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn face(vertices: Vec<u32>) -> Face {
        Face {
            vertices,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        }
    }

    /// `n` x `n` unit grid of quads in XZ (y = 0), points row by row.
    pub(crate) fn grid(n: u32) -> Mesh {
        let mut mesh = Mesh::default();
        for j in 0..=n {
            for i in 0..=n {
                mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
            }
        }
        for j in 0..n {
            for i in 0..n {
                let a = j * (n + 1) + i;
                mesh.faces.push(face(vec![a, a + n + 1, a + n + 2, a + 1]));
            }
        }
        mesh
    }

    #[test]
    fn across_finds_the_opposite_side_of_quads_and_even_ngons() {
        assert_eq!(across(&[0, 1, 2, 3], 0, 1, false), Some((3, 2)));
        assert_eq!(across(&[0, 1, 2, 3], 1, 0, false), Some((2, 3)));
        assert_eq!(across(&[0, 1, 2, 3, 4, 5], 0, 1, true), Some((4, 3)));
        assert_eq!(across(&[0, 1, 2, 3, 4, 5], 0, 1, false), None);
        assert_eq!(across(&[0, 1, 2, 3, 4], 0, 1, true), None);
        assert_eq!(across(&[0, 1, 2], 0, 1, true), None);
    }

    #[test]
    fn symmetry_mirrors_and_uniform_spaces() {
        let mut o = LoopSlice {
            mode: LoopSliceMode::Symmetry,
            ..Default::default()
        };
        o.set_count(2);
        o.set_position(0, 0.2);
        assert_eq!(o.positions, vec![0.2, 0.8]);
        o.mode = LoopSliceMode::Uniform;
        o.normalize(0);
        assert_eq!(o.positions.len(), 2);
        assert!((o.positions[0] - 1. / 3.).abs() < 1e-12);
    }

    #[test]
    fn plane_cut_splits_every_polygon_it_crosses() {
        let mesh = grid(2);
        let o = PolygonSlice {
            cuts: vec![PlaneCut {
                start: DVec3::new(-1., 0., 0.5),
                end: DVec3::new(3., 0., 0.5),
                up: DVec3::Y,
            }],
            ..Default::default()
        };
        let (out, edges, _) =
            polygon_slice_geometry(&mesh, glam::DMat4::IDENTITY, &o, &[]).unwrap();
        assert_eq!(out.faces.len(), 6);
        assert_eq!(edges.len(), 2);
    }
}
