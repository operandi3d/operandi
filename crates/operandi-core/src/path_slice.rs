//! Wave 2 path slicing (spec WAVE2-PATH-SLICE, PS-01..PS-45): one Slice
//! cutter shared by Pen Slice, Primitive Slice and Curve Slice.
//!
//! Each tool makes one or more paths (world-space points, open or closed);
//! [`path_cut`] projects them along the cutter's axis onto the active mesh
//! and cuts them in with the shared engine (`slice_cut.rs`). No mesh point
//! moves (PS-10).
//!
//! The exact cut rules are Provisional in the spec (PS-Q1..Q8). Each rule
//! sits in its own small function here so it can change once measured:
//!
//! - which polygons are cut: [`cuttable`] (selected, else all; Front Face
//!   Only; polygons seen edge-on along the axis are skipped);
//! - where an inner point lands: [`onto_plane`] (along the axis onto the
//!   polygon's plane, PS-Q1);
//! - open ends: a run that doesn't reach a second edge makes no chord but
//!   keeps its edge points (PS-12, PS-Q2);
//! - a closed outline inside one polygon: [`hole_bridges`] (PS-13, PS-Q3);
//! - which pieces are inside an outline: [`inside_outlines`] (Stencil, Core).
use crate::slice_cut::{Chord, EdgeCut, InnerPoint, Node, cut};
use crate::{Mesh, Result};
use glam::{DMat4, DVec2, DVec3};
use std::collections::{BTreeSet, HashMap};

/// The cutter's Operation (PS-14). Union, Intersect, Subtract and Add are
/// boolean types that wait for wave 4; Tunnel is not built yet either.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliceOperation {
    Core,
    Tunnel,
    Stencil,
    #[default]
    Slice,
    Union,
    Intersect,
    Subtract,
    Add,
}
impl SliceOperation {
    pub const ALL: [SliceOperation; 8] = [
        Self::Core,
        Self::Tunnel,
        Self::Stencil,
        Self::Slice,
        Self::Union,
        Self::Intersect,
        Self::Subtract,
        Self::Add,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Tunnel => "Tunnel",
            Self::Stencil => "Stencil",
            Self::Slice => "Slice",
            Self::Union => "Union",
            Self::Intersect => "Intersect",
            Self::Subtract => "Subtract",
            Self::Add => "Add",
        }
    }
    /// Built in this wave (the rest are greyed in the panel).
    pub fn is_built(self) -> bool {
        matches!(
            self,
            Self::Slice | Self::Stencil | Self::Core | Self::Tunnel
        )
    }
    /// Boolean types: Negative/Positive Depth and Preview Geometry apply.
    pub fn is_boolean(self) -> bool {
        matches!(
            self,
            Self::Union | Self::Intersect | Self::Subtract | Self::Add
        )
    }
}

/// The cutter's Axis: the direction the path is projected along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliceAxis {
    X,
    #[default]
    Y,
    Z,
    /// Vector X/Y/Z.
    Custom,
    /// The tool's own axis: the clicked face's normal for Primitive Slice,
    /// the work plane's normal for Pen and Curve Slice (provisional).
    ActionAxis,
}
impl SliceAxis {
    pub const ALL: [SliceAxis; 5] = [Self::X, Self::Y, Self::Z, Self::Custom, Self::ActionAxis];
    pub fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Custom => "Custom",
            Self::ActionAxis => "Action Axis",
        }
    }
}

/// The Slice cutter's options (spec section 2; starting values from the
/// attribute listing).
#[derive(Clone, Debug, PartialEq)]
pub struct PathCutter {
    pub operation: SliceOperation,
    pub axis: SliceAxis,
    /// Custom axis direction.
    pub vector: DVec3,
    pub front_face_only: bool,
    /// Stencil's material.
    pub material: String,
    /// Boolean types only (wave 4): stored and shown greyed.
    pub negative_depth: f64,
    pub positive_depth: f64,
    pub preview_geometry: bool,
    pub select_sliced_edges: bool,
    pub edge_set: String,
}
impl Default for PathCutter {
    fn default() -> Self {
        Self {
            operation: SliceOperation::Slice,
            axis: SliceAxis::Y,
            vector: DVec3::Y,
            front_face_only: false,
            material: "Default".into(),
            negative_depth: 0.,
            positive_depth: 1.,
            preview_geometry: true,
            select_sliced_edges: false,
            edge_set: "Sliced Edges".into(),
        }
    }
}
impl PathCutter {
    /// Primitive Slice starts on Action Axis.
    pub fn primitive() -> Self {
        Self {
            axis: SliceAxis::ActionAxis,
            ..Self::default()
        }
    }
    /// The world direction to project along; `action` is the tool's Action
    /// Axis. `None` for a zero Custom vector.
    pub fn direction(&self, action: DVec3) -> Option<DVec3> {
        let d = match self.axis {
            SliceAxis::X => DVec3::X,
            SliceAxis::Y => DVec3::Y,
            SliceAxis::Z => DVec3::Z,
            SliceAxis::Custom => self.vector,
            SliceAxis::ActionAxis => action,
        };
        (d.is_finite() && d.length() > 1e-12).then(|| d.normalize())
    }
}

/// One path to cut, in world space.
#[derive(Clone, Debug, PartialEq)]
pub struct CutPath {
    pub points: Vec<DVec3>,
    pub closed: bool,
}

/// What a path cut made.
#[derive(Clone, Debug, Default)]
pub struct PathCutResult {
    pub mesh: Mesh,
    /// The cut's new edges, each (low, high).
    pub new_edges: Vec<(u32, u32)>,
    /// Every piece of a polygon the path split.
    pub pieces: Vec<u32>,
    /// Polygons inside a closed outline (after Core's deletion).
    pub inside: Vec<u32>,
    /// The polygon selection after the cut: the selected polygons and their
    /// pieces (PS-17, provisional).
    pub selected: Vec<u32>,
}

/// A path flattened into the cut's 2D view (looking along the axis).
struct Flat {
    /// Mesh-local points.
    local: Vec<DVec3>,
    flat: Vec<DVec2>,
    closed: bool,
}
impl Flat {
    fn segments(&self) -> usize {
        if self.closed {
            self.flat.len()
        } else {
            self.flat.len().saturating_sub(1)
        }
    }
    /// The flat point at path parameter `m` (segment index + fraction).
    fn at(&self, m: f64) -> DVec2 {
        let n = self.flat.len();
        let k = (m.floor() as usize).min(self.segments().saturating_sub(1));
        let f = m - k as f64;
        self.flat[k].lerp(self.flat[(k + 1) % n], f)
    }
}

fn cross(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

/// Even-odd point-in-polygon test in the flat view.
fn inside_polygon(p: DVec2, ring: &[DVec2]) -> bool {
    let n = ring.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

fn signed_area(ring: &[DVec2]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| cross(ring[i], ring[(i + 1) % n]))
        .sum::<f64>()
        / 2.
}

/// PS-10/PS-Q1 (provisional): a path point inside polygon `face` lands on
/// the polygon's plane, moved along the axis `d`.
fn onto_plane(mesh: &Mesh, face: usize, p: DVec3, d: DVec3) -> DVec3 {
    let n = mesh.normal(face);
    let c = mesh.center(face);
    let denom = n.dot(d);
    if denom.abs() < 1e-12 {
        return p;
    }
    p + d * (n.dot(c - p) / denom)
}

/// The polygons a path cut works on: the selected polygons (all when none
/// are selected; PS-Q7), surface polygons only, facing the axis's positive
/// side with Front Face Only (PS-11/PS-Q4: provisional reading).
fn considered(mesh: &Mesh, d: DVec3, o: &PathCutter, selected: &[u32]) -> Vec<usize> {
    let all: Vec<usize> = if selected.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        let mut s: Vec<usize> = selected
            .iter()
            .map(|&f| f as usize)
            .filter(|&f| f < mesh.faces.len())
            .collect();
        s.sort_unstable();
        s.dedup();
        s
    };
    all.into_iter()
        .filter(|&f| !mesh.is_polyline(f))
        .filter(|&f| !o.front_face_only || mesh.normal(f).dot(d) > 1e-9)
        .collect()
}

/// Of `faces`, those overlapping the path's flat bounds, as (seen face-on,
/// seen edge-on along the axis). The ribbon cuts both kinds (PS-10): an
/// edge-on polygon (a box's side) is split where the ribbon passes through
/// it.
fn cuttable(
    faces: &[usize],
    mesh: &Mesh,
    flat: &[DVec2],
    bounds: (DVec2, DVec2),
) -> (Vec<usize>, Vec<usize>) {
    let mut face_on = Vec::new();
    let mut edge_on = Vec::new();
    for &f in faces {
        let vs = &mesh.faces[f].vertices;
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for &v in vs {
            lo = lo.min(flat[v as usize]);
            hi = hi.max(flat[v as usize]);
        }
        if hi.x < bounds.0.x || lo.x > bounds.1.x || hi.y < bounds.0.y || lo.y > bounds.1.y {
            continue;
        }
        let ring: Vec<DVec2> = vs.iter().map(|&v| flat[v as usize]).collect();
        let size = (hi - lo).max_element();
        if signed_area(&ring).abs() > 1e-9 * size * size.max(1e-9) {
            face_on.push(f);
        } else {
            edge_on.push(f);
        }
    }
    (face_on, edge_on)
}

/// A crossing of a path with a mesh edge: the path, its parameter there,
/// and where it lands on the edge.
#[derive(Clone, Copy, Debug)]
struct Crossing {
    path: usize,
    param: f64,
    node: Node,
}

/// Snap a crossing this close to an edge's end onto that end's point.
const END_SNAP: f64 = 1e-6;

/// The crossings of every path with edge `low`–`high` (flat view). New
/// edge points are appended to `cuts`.
fn edge_crossings(
    low: u32,
    high: u32,
    flat: &[DVec2],
    paths: &[Flat],
    cuts: &mut Vec<EdgeCut>,
) -> Vec<Crossing> {
    let (a, b) = (flat[low as usize], flat[high as usize]);
    let r = b - a;
    if r.length() < 1e-12 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (pi, path) in paths.iter().enumerate() {
        let n = path.flat.len();
        let segments = path.segments();
        for k in 0..segments {
            let (p, q) = (path.flat[k], path.flat[(k + 1) % n]);
            let s = q - p;
            let denom = cross(r, s);
            if denom.abs() < 1e-14 * r.length() * s.length().max(1e-12) {
                continue;
            }
            let t = cross(p - a, s) / denom;
            let w = cross(p - a, r) / denom;
            // Half-open per segment so a path point on an edge counts once;
            // the last segment of an open path keeps its end.
            let last = !path.closed && k + 1 == segments;
            if w < 0. || w > 1. || (w == 1. && !last) {
                continue;
            }
            if !(-END_SNAP..=1. + END_SNAP).contains(&t) {
                continue;
            }
            let node = if t <= END_SNAP {
                Node::Point(low)
            } else if t >= 1. - END_SNAP {
                Node::Point(high)
            } else {
                cuts.push(EdgeCut {
                    a: low,
                    b: high,
                    t,
                    at: None,
                });
                Node::Cut(cuts.len() - 1)
            };
            out.push(Crossing {
                path: pi,
                param: k as f64 + w,
                node,
            });
        }
    }
    out
}

/// PS-13 (provisional, PS-Q3): a closed outline inside one polygon is cut
/// as an inner polygon. Operandi's polygons can't have holes, so the ring
/// around it is split into two pieces by two bridges, from the outline
/// point nearest a corner and the one halfway round it to their nearest
/// corners. Returns (corner a, outline index j1, corner b, outline index j2).
fn hole_bridges(ring: &[(u32, DVec2)], outline: &[DVec2]) -> Option<(u32, usize, u32, usize)> {
    let n = outline.len();
    if n < 3 || ring.len() < 2 {
        return None;
    }
    let (mut best, mut a, mut j1) = (f64::MAX, ring[0].0, 0);
    for &(v, c) in ring {
        for (j, &p) in outline.iter().enumerate() {
            let d = c.distance_squared(p);
            if d < best {
                (best, a, j1) = (d, v, j);
            }
        }
    }
    let j2 = (j1 + n / 2) % n;
    let b = ring
        .iter()
        .filter(|(v, _)| *v != a)
        .min_by(|x, y| {
            x.1.distance_squared(outline[j2])
                .total_cmp(&y.1.distance_squared(outline[j2]))
        })?
        .0;
    Some((a, j1, b, j2))
}

/// Which of `faces` lie inside a closed outline (even-odd over every closed
/// path, tested at the polygon's centre in the flat view).
fn inside_outlines(
    mesh: &Mesh,
    faces: impl Iterator<Item = usize>,
    project: impl Fn(DVec3) -> DVec2,
    paths: &[Flat],
) -> Vec<u32> {
    let outlines: Vec<&Flat> = paths
        .iter()
        .filter(|p| p.closed && p.flat.len() >= 3)
        .collect();
    if outlines.is_empty() {
        return Vec::new();
    }
    let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for p in &outlines {
        for q in &p.flat {
            lo = lo.min(*q);
            hi = hi.max(*q);
        }
    }
    let eps = 1e-7 * (hi - lo).max_element().max(1.);
    let inside = |c: DVec2| {
        outlines
            .iter()
            .filter(|p| inside_polygon(c, &p.flat))
            .count()
            % 2
            == 1
    };
    // A point on an outline decides nothing: the cut runs through it.
    let on_outline = |c: DVec2| {
        outlines.iter().any(|p| {
            let n = p.flat.len();
            (0..n).any(|i| segment_distance(c, p.flat[i], p.flat[(i + 1) % n]) < eps)
        })
    };
    faces
        .filter(|&f| {
            // The cut follows the outlines, so a piece is wholly on one
            // side: its first point off every outline tells which.
            for &v in &mesh.faces[f].vertices {
                let c = project(mesh.vertices[v as usize]);
                if c.x < lo.x - eps || c.y < lo.y - eps || c.x > hi.x + eps || c.y > hi.y + eps {
                    return false;
                }
                if !on_outline(c) {
                    return inside(c);
                }
            }
            inside(project(mesh.center(f)))
        })
        .map(|f| f as u32)
        .collect()
}

/// Delete `removed` polygons from the result and the points only they
/// used; remap everything the result lists.
fn delete_faces(r: &mut PathCutResult, removed: &[u32]) -> Result<()> {
    let removal = crate::polygon_remove::remove_faces(&r.mesh, removed)?;
    let mut remap = vec![None; r.mesh.faces.len()];
    for (new, &old) in removal.survivors.iter().enumerate() {
        remap[old] = Some(new as u32);
    }
    let map =
        |list: &[u32]| -> Vec<u32> { list.iter().filter_map(|&f| remap[f as usize]).collect() };
    r.pieces = map(&r.pieces);
    r.selected = map(&r.selected);
    r.inside = map(&r.inside);
    // The points the deleted polygons used that nothing else uses now.
    let mut orphans: BTreeSet<u32> = BTreeSet::new();
    for &f in removed {
        orphans.extend(r.mesh.faces[f as usize].vertices.iter().copied());
    }
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&removal.mesh, Some(&orphans))?;
    let mut point = vec![None; removal.mesh.vertices.len()];
    for (new, &old) in cleaned.survivors.iter().enumerate() {
        point[old] = Some(new as u32);
    }
    r.new_edges = r
        .new_edges
        .iter()
        .filter_map(|&(a, b)| {
            let (a, b) = (point[a as usize]?, point[b as usize]?);
            Some((a.min(b), a.max(b)))
        })
        .collect();
    r.mesh = cleaned.mesh;
    let existing: BTreeSet<(u32, u32)> = r.mesh.edges().into_iter().collect();
    r.new_edges.retain(|e| existing.contains(e));
    Ok(())
}

fn segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared().max(1e-300)).clamp(0., 1.);
    p.distance(a + d * t)
}

/// Cut `paths` (world space) into `mesh` along world `direction` with the
/// cutter `o`. `world` maps the mesh to world space; `selected` are the
/// selected polygons (empty: all). Fails, changing nothing, when the paths
/// cross themselves inside one polygon or the result isn't a valid mesh.
pub fn path_cut(
    mesh: &Mesh,
    world: DMat4,
    paths: &[CutPath],
    direction: DVec3,
    o: &PathCutter,
    selected: &[u32],
) -> Result<PathCutResult> {
    let unchanged = || PathCutResult {
        mesh: mesh.clone(),
        selected: selected.to_vec(),
        ..Default::default()
    };
    if !o.operation.is_built() {
        return Err(format!("{} isn't built yet", o.operation.label()));
    }
    let local = world.inverse();
    let d = local.transform_vector3(direction);
    if !d.is_finite() || d.length() < 1e-12 {
        return Err("The slice axis has no direction".into());
    }
    let d = d.normalize();
    let u = d.any_orthonormal_vector();
    let v = d.cross(u);
    let project = |p: DVec3| DVec2::new(p.dot(u), p.dot(v));
    // The paths, mesh-local and flat, without repeated points.
    let mut flats: Vec<Flat> = Vec::new();
    for path in paths {
        let mut local_points: Vec<DVec3> = Vec::new();
        let mut flat_points: Vec<DVec2> = Vec::new();
        for &p in &path.points {
            if !p.is_finite() {
                return Err("A slice path point isn't a finite position".into());
            }
            let l = local.transform_point3(p);
            let f = project(l);
            if flat_points.last().is_some_and(|q| q.distance(f) < 1e-9) {
                continue;
            }
            local_points.push(l);
            flat_points.push(f);
        }
        let mut closed = path.closed;
        if closed
            && flat_points.len() >= 2
            && flat_points[0].distance(*flat_points.last().unwrap()) < 1e-9
        {
            flat_points.pop();
            local_points.pop();
        }
        if flat_points.len() < 3 {
            closed = false;
        }
        if flat_points.len() >= 2 {
            flats.push(Flat {
                local: local_points,
                flat: flat_points,
                closed,
            });
        }
    }
    if flats.is_empty() || mesh.faces.is_empty() {
        return Ok(unchanged());
    }
    let mut lo = DVec2::splat(f64::MAX);
    let mut hi = DVec2::splat(f64::MIN);
    for f in &flats {
        for p in &f.flat {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let pad = 1e-9 * (hi - lo).max_element().max(1.);
    let bounds = (lo - pad, hi + pad);
    let flat: Vec<DVec2> = mesh.vertices.iter().map(|&p| project(p)).collect();
    let faces = considered(mesh, d, o, selected);
    let (cut_faces, edge_on) = cuttable(&faces, mesh, &flat, bounds);

    // Crossings per edge, computed once so both polygons of an edge share
    // its points.
    let mut cuts: Vec<EdgeCut> = Vec::new();
    let mut by_edge: HashMap<(u32, u32), Vec<Crossing>> = HashMap::new();
    let mut inner: Vec<InnerPoint> = Vec::new();
    let mut chords: Vec<Chord> = Vec::new();
    // (face, path) of closed outlines lying wholly inside one polygon.
    let mut holes: Vec<(usize, usize)> = Vec::new();
    for &fi in &cut_faces {
        let vs = &mesh.faces[fi].vertices;
        let n = vs.len();
        let mut events: Vec<Vec<(f64, Node)>> = vec![Vec::new(); flats.len()];
        for i in 0..n {
            let (x, y) = (vs[i], vs[(i + 1) % n]);
            let key = (x.min(y), x.max(y));
            let list = by_edge
                .entry(key)
                .or_insert_with(|| edge_crossings(key.0, key.1, &flat, &flats, &mut cuts));
            for c in list.iter() {
                events[c.path].push((c.param, c.node));
            }
        }
        let ring: Vec<DVec2> = vs.iter().map(|&v| flat[v as usize]).collect();
        for (pi, path) in flats.iter().enumerate() {
            let list = &mut events[pi];
            if list.is_empty() {
                if path.closed && inside_polygon(path.flat[0], &ring) {
                    holes.push((fi, pi));
                }
                continue;
            }
            list.sort_by(|a, b| a.0.total_cmp(&b.0));
            // A path through a corner meets both of its edges there.
            list.dedup_by(|b, a| a.1 == b.1 && (a.0 - b.0).abs() < 1e-9);
            let count = list.len();
            let period = path.segments() as f64;
            let pairs = if path.closed { count } else { count - 1 };
            for k in 0..pairs {
                let (p1, n1) = list[k];
                let (mut p2, n2) = list[(k + 1) % count];
                if k + 1 == count {
                    p2 += period;
                }
                let mid = (p1 + p2) / 2.;
                let mid = if mid >= period { mid - period } else { mid };
                if !inside_polygon(path.at(mid), &ring) {
                    continue;
                }
                // The path's own points between the two crossings.
                let through: Vec<usize> = ((p1.floor() as i64 + 1)..=(p2.ceil() as i64 - 1))
                    .map(|j| j as f64)
                    .filter(|&j| j > p1 + 1e-9 && j < p2 - 1e-9)
                    .map(|j| {
                        let index = (j as usize) % path.local.len();
                        inner.push(InnerPoint {
                            face: fi,
                            at: onto_plane(mesh, fi, path.local[index], d),
                        });
                        inner.len() - 1
                    })
                    .collect();
                if n1 == n2 && through.is_empty() {
                    continue;
                }
                chords.push(Chord {
                    face: fi,
                    from: n1,
                    to: n2,
                    through,
                });
            }
            // PS-12: a run that starts or ends inside the polygon (open path)
            // makes no chord; its edge point stays (Operandi's polygons
            // can't hold the reference's slit).
        }
    }
    // Edge-on polygons: the ribbon passes through one where a path segment
    // crosses its flat line, meeting two of its edges at the same place on
    // the path; the cut joins those two points.
    for &fi in &edge_on {
        let vs = &mesh.faces[fi].vertices;
        let n = vs.len();
        let mut events: Vec<Vec<(f64, Node)>> = vec![Vec::new(); flats.len()];
        for i in 0..n {
            let (x, y) = (vs[i], vs[(i + 1) % n]);
            let key = (x.min(y), x.max(y));
            let list = by_edge
                .entry(key)
                .or_insert_with(|| edge_crossings(key.0, key.1, &flat, &flats, &mut cuts));
            for c in list.iter() {
                events[c.path].push((c.param, c.node));
            }
        }
        for list in &mut events {
            list.sort_by(|a, b| a.0.total_cmp(&b.0));
            list.dedup_by(|b, a| a.1 == b.1 && (a.0 - b.0).abs() < 1e-9);
            let mut k = 0;
            while k < list.len() {
                let mut j = k + 1;
                while j < list.len() && (list[j].0 - list[k].0).abs() < 1e-9 {
                    j += 1;
                }
                if j - k == 2 && list[k].1 != list[k + 1].1 {
                    chords.push(Chord {
                        face: fi,
                        from: list[k].1,
                        to: list[k + 1].1,
                        through: Vec::new(),
                    });
                }
                k = j;
            }
        }
    }
    if cuts.is_empty() && chords.is_empty() && holes.is_empty() {
        let mut r = unchanged();
        finish(&mut r, &faces, &project, &flats, o)?;
        return Ok(r);
    }
    let first = cut(mesh, &cuts, &inner, &chords)?;
    let mut out = first.mesh;
    let mut new_edges = first.new_edges;
    // Every output polygon's source polygon in `mesh`.
    let mut origin: Vec<u32> = (0..mesh.faces.len() as u32).collect();
    origin.resize(out.faces.len(), 0);
    for &(new, source) in &first.piece_sources {
        origin[new as usize] = source;
    }
    if !holes.is_empty() {
        let (mesh2, edges2) = cut_holes(
            &out,
            &holes,
            &flats,
            &flat_of(&out, &project),
            d,
            &mut origin,
        )?;
        out = mesh2;
        new_edges.extend(edges2);
    }
    let mut pieces: Vec<u32> = Vec::new();
    {
        // A polygon whose source is shared with another is a piece.
        let mut count: HashMap<u32, usize> = HashMap::new();
        for &s in &origin {
            *count.entry(s).or_default() += 1;
        }
        for (f, s) in origin.iter().enumerate() {
            if count[s] > 1 {
                pieces.push(f as u32);
            }
        }
    }
    let selected_set: BTreeSet<u32> = selected.iter().copied().collect();
    let considered_set: BTreeSet<u32> = faces.iter().map(|&f| f as u32).collect();
    let mut r = PathCutResult {
        mesh: out,
        new_edges,
        pieces,
        inside: Vec::new(),
        selected: origin
            .iter()
            .enumerate()
            .filter(|(_, s)| selected_set.contains(s))
            .map(|(f, _)| f as u32)
            .collect(),
    };
    let considered_out: Vec<usize> = origin
        .iter()
        .enumerate()
        .filter(|(_, s)| considered_set.contains(s))
        .map(|(f, _)| f)
        .collect();
    finish(&mut r, &considered_out, &project, &flats, o)?;
    Ok(r)
}

fn flat_of(mesh: &Mesh, project: &impl Fn(DVec3) -> DVec2) -> Vec<DVec2> {
    mesh.vertices.iter().map(|&p| project(p)).collect()
}

/// PS-13: cut every closed outline lying inside one polygon (see
/// [`hole_bridges`]) in two passes of the engine. Updates `origin`.
fn cut_holes(
    mesh: &Mesh,
    holes: &[(usize, usize)],
    paths: &[Flat],
    flat: &[DVec2],
    d: DVec3,
    origin: &mut Vec<u32>,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    // Pass 1: corner a -> outline j1..=j2 -> corner b, one chord per hole.
    let mut inner: Vec<InnerPoint> = Vec::new();
    let mut chords: Vec<Chord> = Vec::new();
    // Per hole: its face, path, j1, j2, the inner indices of j1 and j2, and
    // the first inner index of its outline points.
    let mut plans: Vec<(usize, usize, usize, usize, usize, usize, usize)> = Vec::new();
    for &(fi, pi) in holes {
        let path = &paths[pi];
        let ring: Vec<(u32, DVec2)> = mesh.faces[fi]
            .vertices
            .iter()
            .map(|&v| (v, flat[v as usize]))
            .collect();
        let Some((a, j1, b, j2)) = hole_bridges(&ring, &path.flat) else {
            continue;
        };
        let n = path.flat.len();
        let start = inner.len();
        let mut through = Vec::new();
        let mut j = j1;
        loop {
            inner.push(InnerPoint {
                face: fi,
                at: onto_plane(mesh, fi, path.local[j], d),
            });
            through.push(inner.len() - 1);
            if j == j2 {
                break;
            }
            j = (j + 1) % n;
        }
        chords.push(Chord {
            face: fi,
            from: Node::Point(a),
            to: Node::Point(b),
            through,
        });
        plans.push((fi, pi, j1, j2, start, inner.len() - 1, start));
    }
    if chords.is_empty() {
        return Ok((mesh.clone(), Vec::new()));
    }
    let first = cut(mesh, &[], &inner, &chords)?;
    origin.resize(first.mesh.faces.len(), 0);
    for &(new, source) in &first.piece_sources {
        origin[new as usize] = origin[source as usize];
    }
    let mut out = first.mesh;
    let mut edges = first.new_edges;
    // Pass 2: close each outline from j2 round to j1 in the piece holding
    // that half. The flat view of the new outline points comes from the
    // outlines themselves.
    let mut flat2 = flat.to_vec();
    flat2.resize(out.vertices.len(), DVec2::ZERO);
    for &(_, pi, j1, j2, _, _, start) in &plans {
        let path = &paths[pi];
        let n = path.flat.len();
        let (mut j, mut k) = (j1, start);
        loop {
            flat2[first.inner_points[k] as usize] = path.flat[j];
            if j == j2 {
                break;
            }
            j = (j + 1) % n;
            k += 1;
        }
    }
    let mut inner2: Vec<InnerPoint> = Vec::new();
    let mut chords2: Vec<Chord> = Vec::new();
    for &(fi, pi, j1, j2, i1, i2, _) in &plans {
        let path = &paths[pi];
        let n = path.flat.len();
        let (p1, p2) = (first.inner_points[i1], first.inner_points[i2]);
        // The other half's first point (or the middle of the closing span).
        let probe = if (j2 + 1) % n != j1 {
            path.flat[(j2 + 1) % n]
        } else {
            (path.flat[j2] + path.flat[j1]) / 2.
        };
        let candidates: Vec<usize> = (0..out.faces.len())
            .filter(|&g| g == fi || (g >= mesh.faces.len() && origin[g] == origin[fi]))
            .filter(|&g| {
                let vs = &out.faces[g].vertices;
                vs.contains(&p1) && vs.contains(&p2)
            })
            .collect();
        let Some(&g) = candidates.iter().find(|&&g| {
            let ring: Vec<DVec2> = out.faces[g]
                .vertices
                .iter()
                .map(|&v| flat2[v as usize])
                .collect();
            inside_polygon(probe, &ring)
        }) else {
            continue;
        };
        let mut through = Vec::new();
        let mut j = (j2 + 1) % n;
        while j != j1 {
            inner2.push(InnerPoint {
                face: g,
                at: onto_plane(&out, g, path.local[j], d),
            });
            through.push(inner2.len() - 1);
            j = (j + 1) % n;
        }
        chords2.push(Chord {
            face: g,
            from: Node::Point(p2),
            to: Node::Point(p1),
            through,
        });
    }
    if !chords2.is_empty() {
        let second = cut(&out, &[], &inner2, &chords2)?;
        origin.resize(second.mesh.faces.len(), 0);
        for &(new, source) in &second.piece_sources {
            origin[new as usize] = origin[source as usize];
        }
        out = second.mesh;
        edges.extend(second.new_edges);
    }
    // Edges an earlier pass made may have been split by a later one.
    let existing: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    edges.retain(|e| existing.contains(e));
    Ok((out, edges))
}

/// After the cut: Stencil, Core and Select Sliced Edges.
fn finish(
    r: &mut PathCutResult,
    considered: &[usize],
    project: &impl Fn(DVec3) -> DVec2,
    paths: &[Flat],
    o: &PathCutter,
) -> Result<()> {
    let inside = inside_outlines(&r.mesh, considered.iter().copied(), project, paths);
    match o.operation {
        SliceOperation::Stencil => {
            for &f in &inside {
                let face = &mut r.mesh.faces[f as usize];
                face.material = o.material.clone();
                face.material_label = Some(o.material.clone());
            }
            r.inside = inside;
        }
        SliceOperation::Core | SliceOperation::Tunnel if paths.iter().any(|p| p.closed) => {
            // Core keeps the inside, Tunnel deletes it (no side walls,
            // PS-14); points left unused go with them.
            let set: BTreeSet<u32> = inside.iter().copied().collect();
            let core = o.operation == SliceOperation::Core;
            let removed: Vec<u32> = considered
                .iter()
                .map(|&f| f as u32)
                .filter(|f| set.contains(f) != core)
                .collect();
            r.inside = if core { inside } else { Vec::new() };
            if !removed.is_empty() {
                delete_faces(r, &removed)?;
            }
        }
        _ => r.inside = inside,
    }
    r.new_edges.sort_unstable();
    r.new_edges.dedup();
    if o.select_sliced_edges && !r.new_edges.is_empty() {
        let name = if o.edge_set.trim().is_empty() {
            "Sliced Edges".to_string()
        } else {
            o.edge_set.clone()
        };
        r.mesh
            .attributes
            .edge_sets
            .entry(name)
            .or_default()
            .extend(r.new_edges.iter().copied());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The three path makers' state.

/// Pen Slice (PS-30/PS-31): clicked points, straight segments.
#[derive(Clone, Debug, PartialEq)]
pub struct PenSlice {
    /// World points in click order.
    pub points: Vec<DVec3>,
    /// Type: Closed (the starting value) or Open.
    pub closed: bool,
    /// The work plane's normal when the first point was placed (the Action
    /// Axis).
    pub plane_normal: DVec3,
    pub cutter: PathCutter,
}
impl Default for PenSlice {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            closed: true,
            plane_normal: DVec3::Y,
            cutter: PathCutter::default(),
        }
    }
}
impl PenSlice {
    pub fn paths(&self) -> Vec<CutPath> {
        vec![CutPath {
            points: self.points.clone(),
            closed: self.closed,
        }]
    }
}

/// Curve Slice (PS-20..PS-22): a curve drawn by clicking, sampled into
/// straight steps.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveSlice {
    /// Control points (world), the curve passes through them.
    pub points: Vec<DVec3>,
    pub closed: bool,
    /// Path Steps.
    pub steps: u32,
    /// Auto: pick the steps by angle.
    pub auto_steps: bool,
    /// Degrees, used by Auto.
    pub smooth_angle: f64,
    pub plane_normal: DVec3,
    pub cutter: PathCutter,
}
impl Default for CurveSlice {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            closed: false,
            steps: 24,
            auto_steps: false,
            smooth_angle: 5.,
            plane_normal: DVec3::Y,
            cutter: PathCutter::default(),
        }
    }
}
impl CurveSlice {
    /// The curve sampled into straight steps (PS-21, provisional: evenly
    /// spaced by curve parameter over the whole curve; the curve passes
    /// through its points; two points are a straight line).
    pub fn sampled(&self) -> Vec<DVec3> {
        let n = self.points.len();
        if n < 2 {
            return self.points.clone();
        }
        if n == 2 {
            // A straight line, but its step points still go on the cut
            // (PS-21).
            let (a, b) = (self.points[0], self.points[1]);
            let steps = self.steps.clamp(1, 4096) as usize;
            let count = if self.closed { steps } else { steps + 1 };
            return (0..count)
                .map(|i| a.lerp(b, i as f64 / steps as f64))
                .collect();
        }
        let kind = crate::curves::CurveKind::Curve {
            start_control: false,
            end_control: false,
        };
        let spans = if self.closed { n } else { n - 1 };
        if self.auto_steps {
            let dense = crate::curves::sample(&self.points, kind, self.closed, 64);
            return by_angle(&dense, self.closed, self.smooth_angle);
        }
        let steps = self.steps.clamp(1, 4096) as usize;
        // `steps` segments per span, then every `spans`-th sample: `steps`
        // even steps by parameter over the whole curve.
        let dense = crate::curves::sample(&self.points, kind, self.closed, steps);
        let count = if self.closed { steps } else { steps + 1 };
        (0..count)
            .filter_map(|i| dense.get(i * spans).copied())
            .collect()
    }
    pub fn paths(&self) -> Vec<CutPath> {
        vec![CutPath {
            points: self.sampled(),
            closed: self.closed,
        }]
    }
}

/// Auto steps: keep a dense sample whenever the curve has turned by
/// `angle` degrees since the last kept one (provisional).
fn by_angle(dense: &[DVec3], closed: bool, angle: f64) -> Vec<DVec3> {
    if dense.len() <= 2 {
        return dense.to_vec();
    }
    let limit = angle.clamp(0.1, 90.).to_radians();
    let mut out = vec![dense[0]];
    let mut turned = 0.;
    for i in 1..dense.len() - 1 {
        let a = (dense[i] - dense[i - 1]).normalize_or_zero();
        let b = (dense[i + 1] - dense[i]).normalize_or_zero();
        turned += a.dot(b).clamp(-1., 1.).acos();
        if turned >= limit {
            out.push(dense[i]);
            turned = 0.;
        }
    }
    if !closed {
        out.push(*dense.last().unwrap());
    }
    out
}

/// Primitive Slice's Type (PS-40). Preset waits for the preset browser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PrimitiveShape {
    #[default]
    Rectangle,
    Circle,
    Ellipse,
    Preset,
}
impl PrimitiveShape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Circle => "Circle",
            Self::Ellipse => "Ellipse",
            Self::Preset => "Preset",
        }
    }
}

/// Where a Primitive Slice shape sits: its centre and frame (world). The
/// shape lies in the plane through `center` with normal `normal`; its X
/// runs along `x_axis`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapePlace {
    pub center: DVec3,
    pub normal: DVec3,
    pub x_axis: DVec3,
}
impl ShapePlace {
    /// The shape's in-plane Y direction.
    pub fn y_axis(&self) -> DVec3 {
        self.normal.cross(self.x_axis).normalize_or_zero()
    }
    /// A frame on a face with normal `normal` (see the body).
    pub fn on(center: DVec3, normal: DVec3) -> Self {
        let n = normal.normalize_or_zero();
        let n = if n.length() < 0.5 { DVec3::Y } else { n };
        // The shape's own right-handed frame (local X, local Y, axis), as
        // measured for Axis Y: (world Z, world X, world Y) (PS-40). Other
        // axes follow the same cycle: local X is the world axis after the
        // one closest to the normal (provisional for tilted faces).
        let k = (0..3)
            .max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs()))
            .unwrap_or(1);
        let mut next = DVec3::ZERO;
        next[(k + 1) % 3] = 1.;
        let x = next - n * next.dot(n);
        let x = if x.length() > 1e-6 {
            x.normalize()
        } else {
            n.any_orthonormal_vector()
        };
        Self {
            center,
            normal: n,
            x_axis: x,
        }
    }
}

/// Primitive Slice (PS-40..PS-45).
#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveSlice {
    pub shape: PrimitiveShape,
    /// `None` until a face is clicked.
    pub place: Option<ShapePlace>,
    pub half_x: f64,
    pub half_y: f64,
    pub corner_radius: f64,
    pub corner_segments: u32,
    /// Not built (its meaning isn't specified): greyed.
    pub exponent: f64,
    pub radius: f64,
    pub sides: u32,
    pub edit_from_center: bool,
    /// Not built: greyed (the fields below rotate the shape).
    pub show_rotate_handles: bool,
    /// Degrees.
    pub heading: f64,
    pub pitch: f64,
    pub bank: f64,
    pub keep_aspect: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub clone_x: u32,
    pub clone_y: u32,
    pub gap_x: f64,
    pub gap_y: f64,
    /// Center: the cloned group is centred on the shape's place.
    pub center_clones: bool,
    pub lazy_apply: bool,
    /// Set by the viewport while a Lazy Apply drag is held: the shape is
    /// drawn but not cut yet.
    pub held: bool,
    /// Preset only (not built): greyed.
    pub extend_open_edge: f64,
    pub cutter: PathCutter,
}
impl Default for PrimitiveSlice {
    fn default() -> Self {
        Self {
            shape: PrimitiveShape::Rectangle,
            place: None,
            half_x: 0.5,
            half_y: 0.5,
            corner_radius: 0.,
            corner_segments: 8,
            exponent: 0.,
            radius: 0.5,
            sides: 24,
            edit_from_center: false,
            show_rotate_handles: false,
            heading: 0.,
            pitch: 0.,
            bank: 0.,
            keep_aspect: false,
            flip_x: false,
            flip_y: false,
            clone_x: 1,
            clone_y: 1,
            gap_x: 0.01,
            gap_y: 0.01,
            center_clones: false,
            lazy_apply: true,
            held: false,
            extend_open_edge: 0.,
            cutter: PathCutter::primitive(),
        }
    }
}
impl PrimitiveSlice {
    /// The shape's half extents in its plane.
    pub fn half_size(&self) -> (f64, f64) {
        match self.shape {
            PrimitiveShape::Circle => (self.radius, self.radius),
            _ => (self.half_x, self.half_y),
        }
    }
    /// One shape's outline in its own plane (x, y), centred on the origin.
    pub fn outline_2d(&self) -> Vec<DVec2> {
        let mut points: Vec<DVec2> = match self.shape {
            PrimitiveShape::Rectangle => {
                let (hx, hy) = (self.half_x.abs(), self.half_y.abs());
                let r = self.corner_radius.clamp(0., hx.min(hy));
                if r <= 1e-12 {
                    vec![
                        DVec2::new(-hx, -hy),
                        DVec2::new(hx, -hy),
                        DVec2::new(hx, hy),
                        DVec2::new(-hx, hy),
                    ]
                } else {
                    let segments = self.corner_segments.clamp(1, 256);
                    let corners = [
                        (DVec2::new(hx - r, -hy + r), -90f64),
                        (DVec2::new(hx - r, hy - r), 0.),
                        (DVec2::new(-hx + r, hy - r), 90.),
                        (DVec2::new(-hx + r, -hy + r), 180.),
                    ];
                    let mut out = Vec::new();
                    for (c, start) in corners {
                        for s in 0..=segments {
                            let a = (start + 90. * s as f64 / segments as f64).to_radians();
                            out.push(c + DVec2::new(a.cos(), a.sin()) * r);
                        }
                    }
                    out
                }
            }
            PrimitiveShape::Circle | PrimitiveShape::Ellipse | PrimitiveShape::Preset => {
                let (hx, hy) = if self.shape == PrimitiveShape::Circle {
                    (self.radius.abs(), self.radius.abs())
                } else {
                    (self.half_x.abs(), self.half_y.abs())
                };
                let sides = self.sides.clamp(3, 1024);
                // The first corner on the shape's -X, which is world -Z for
                // a shape on Axis Y (PS-40, observed).
                (0..sides)
                    .map(|i| {
                        let a =
                            std::f64::consts::PI + std::f64::consts::TAU * i as f64 / sides as f64;
                        DVec2::new(a.cos() * hx, a.sin() * hy)
                    })
                    .collect()
            }
        };
        if self.flip_x {
            points.iter_mut().for_each(|p| p.x = -p.x);
        }
        if self.flip_y {
            points.iter_mut().for_each(|p| p.y = -p.y);
        }
        if self.flip_x != self.flip_y {
            points.reverse();
        }
        points
    }
    /// Every outline to cut (one per clone), in world space.
    pub fn paths(&self) -> Vec<CutPath> {
        let Some(place) = self.place else {
            return Vec::new();
        };
        let outline = self.outline_2d();
        let (hx, hy) = self.half_size();
        let (nx, ny) = (self.clone_x.clamp(1, 64), self.clone_y.clamp(1, 64));
        let step = DVec2::new(
            2. * hx.abs() + self.gap_x.max(0.),
            2. * hy.abs() + self.gap_y.max(0.),
        );
        let shift = if self.center_clones {
            DVec2::new((nx - 1) as f64, (ny - 1) as f64) * step / 2.
        } else {
            DVec2::ZERO
        };
        // Heading tilts about local X, then Pitch about local Y, then Bank
        // turns about the axis, all right-handed (PS-42; Heading and Pitch
        // signs provisional).
        let rotation = glam::DQuat::from_axis_angle(place.normal, self.bank.to_radians())
            * glam::DQuat::from_axis_angle(place.y_axis(), self.pitch.to_radians())
            * glam::DQuat::from_axis_angle(place.x_axis, self.heading.to_radians());
        let (x, y) = (place.x_axis, place.y_axis());
        let mut out = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let offset = DVec2::new(i as f64, j as f64) * step - shift;
                let points = outline
                    .iter()
                    .map(|p| {
                        let q = *p + offset;
                        place.center + rotation * (x * q.x + y * q.y)
                    })
                    .collect();
                out.push(CutPath {
                    points,
                    closed: true,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn face(vertices: Vec<u32>) -> Face {
        Face {
            vertices,
            uv: None,
            material: "M".into(),
            smooth: false,
            part: None,
            material_label: None,
        }
    }

    /// `n` x `n` unit quads in XZ (y = 0), facing +Y.
    fn grid(n: u32) -> Mesh {
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

    fn open(points: &[[f64; 3]]) -> Vec<CutPath> {
        vec![CutPath {
            points: points.iter().map(|&p| DVec3::from(p)).collect(),
            closed: false,
        }]
    }

    fn closed(points: &[[f64; 3]]) -> Vec<CutPath> {
        vec![CutPath {
            points: points.iter().map(|&p| DVec3::from(p)).collect(),
            closed: true,
        }]
    }

    fn run(mesh: &Mesh, paths: &[CutPath], o: &PathCutter) -> PathCutResult {
        path_cut(mesh, DMat4::IDENTITY, paths, DVec3::Y, o, &[]).unwrap()
    }

    #[test]
    fn grid_normal_faces_up() {
        assert!(grid(1).normal(0).y > 0.5);
    }

    #[test]
    fn straight_line_across_a_row_splits_each_polygon() {
        let mesh = grid(2);
        let r = run(
            &mesh,
            &open(&[[-1., 5., 0.5], [3., 5., 0.5]]),
            &PathCutter::default(),
        );
        assert_eq!(r.mesh.faces.len(), 6);
        assert_eq!(r.new_edges.len(), 2);
        assert_eq!(r.mesh.vertices.len(), mesh.vertices.len() + 3);
        // No point moved.
        assert_eq!(&r.mesh.vertices[..mesh.vertices.len()], &mesh.vertices[..]);
    }

    #[test]
    fn a_bend_inside_a_polygon_adds_an_inner_point_on_its_plane() {
        let mesh = grid(1);
        let r = run(
            &mesh,
            &open(&[[0.5, 3., -1.], [0.5, 3., 0.5], [2., 3., 0.5]]),
            &PathCutter::default(),
        );
        assert_eq!(r.mesh.faces.len(), 2);
        let inner = r
            .mesh
            .vertices
            .iter()
            .any(|p| (*p - DVec3::new(0.5, 0., 0.5)).length() < 1e-9);
        assert!(inner, "the bend lands on the polygon's plane");
    }

    #[test]
    fn open_end_inside_a_polygon_leaves_it_unsplit_but_keeps_the_edge_point() {
        let mesh = grid(1);
        let r = run(
            &mesh,
            &open(&[[0.5, 0., -1.], [0.5, 0., 0.5]]),
            &PathCutter::default(),
        );
        assert_eq!(r.mesh.faces.len(), 1);
        assert_eq!(r.mesh.vertices.len(), 5);
        assert!(r.new_edges.is_empty());
    }

    #[test]
    fn closed_outline_inside_one_polygon_cuts_an_inner_polygon() {
        let mut mesh = grid(1);
        for v in &mut mesh.vertices {
            *v *= 4.;
        }
        let r = run(
            &mesh,
            &closed(&[[1., 0., 1.], [3., 0., 1.], [3., 0., 3.], [1., 0., 3.]]),
            &PathCutter::default(),
        );
        r.mesh.validate().unwrap();
        // The inner square plus the ring in two pieces.
        assert_eq!(r.mesh.faces.len(), 3);
        let inner = r
            .mesh
            .faces
            .iter()
            .filter(|f| f.vertices.iter().all(|&v| v >= 4))
            .count();
        assert_eq!(inner, 1);
    }

    #[test]
    fn closed_rectangle_over_a_grid_cuts_every_crossed_polygon() {
        let mesh = grid(4);
        let r = run(
            &mesh,
            &closed(&[
                [0.5, 0., 0.5],
                [3.5, 0., 0.5],
                [3.5, 0., 3.5],
                [0.5, 0., 3.5],
            ]),
            &PathCutter::default(),
        );
        r.mesh.validate().unwrap();
        // 12 border polygons split in two, the 4 corners in two as well
        // (the corner point lands inside them).
        assert_eq!(r.mesh.faces.len(), 16 + 12);
    }

    #[test]
    fn front_face_only_cuts_only_polygons_facing_the_axis() {
        // Two stacked grids: the top faces +Y, the bottom faces -Y.
        let top = grid(1);
        let mut mesh = top.clone();
        let base = mesh.vertices.len() as u32;
        for v in &top.vertices {
            mesh.vertices.push(*v - DVec3::Y);
        }
        mesh.faces
            .push(face(vec![base, base + 1, base + 3, base + 2]));
        let line = open(&[[0.5, 5., -1.], [0.5, 5., 2.]]);
        let both = run(&mesh, &line, &PathCutter::default());
        assert_eq!(both.mesh.faces.len(), 4);
        let front = run(
            &mesh,
            &line,
            &PathCutter {
                front_face_only: true,
                ..Default::default()
            },
        );
        assert_eq!(front.mesh.faces.len(), 3);
    }

    #[test]
    fn stencil_tags_and_core_keeps_the_inside() {
        let mesh = grid(4);
        let square = closed(&[
            [0.5, 0., 0.5],
            [3.5, 0., 0.5],
            [3.5, 0., 3.5],
            [0.5, 0., 3.5],
        ]);
        let stencil = run(
            &mesh,
            &square,
            &PathCutter {
                operation: SliceOperation::Stencil,
                material: "Red".into(),
                ..Default::default()
            },
        );
        let red = stencil
            .mesh
            .faces
            .iter()
            .filter(|f| f.material == "Red")
            .count();
        assert_eq!(red, stencil.inside.len());
        assert_eq!(red, 4 + 12);
        let core = run(
            &mesh,
            &square,
            &PathCutter {
                operation: SliceOperation::Core,
                ..Default::default()
            },
        );
        assert_eq!(core.mesh.faces.len(), 16);
    }

    #[test]
    fn select_sliced_edges_fills_the_named_set() {
        let mesh = grid(2);
        let r = run(
            &mesh,
            &open(&[[-1., 0., 0.5], [3., 0., 0.5]]),
            &PathCutter {
                select_sliced_edges: true,
                ..Default::default()
            },
        );
        assert_eq!(r.mesh.attributes.edge_sets["Sliced Edges"].len(), 2);
    }

    #[test]
    fn selection_keeps_pieces_of_selected_polygons() {
        let mesh = grid(2);
        let r = path_cut(
            &mesh,
            DMat4::IDENTITY,
            &open(&[[-1., 0., 0.5], [3., 0., 0.5]]),
            DVec3::Y,
            &PathCutter::default(),
            &[0],
        )
        .unwrap();
        // Only polygon 0 is cut; it and its new piece stay selected.
        assert_eq!(r.mesh.faces.len(), 5);
        assert_eq!(r.selected.len(), 2);
    }

    #[test]
    fn unbuilt_operations_are_refused() {
        let mesh = grid(1);
        let o = PathCutter {
            operation: SliceOperation::Union,
            ..Default::default()
        };
        assert!(path_cut(&mesh, DMat4::IDENTITY, &[], DVec3::Y, &o, &[]).is_err());
    }

    #[test]
    fn path_through_a_corner_point_splits_cleanly() {
        let mesh = grid(2);
        // The diagonal through point (1, 0, 1), a shared corner.
        let r = run(
            &mesh,
            &open(&[[-0.5, 0., -0.5], [2.5, 0., 2.5]]),
            &PathCutter::default(),
        );
        r.mesh.validate().unwrap();
        // Polygons 0 and 3 are split along their diagonals.
        assert_eq!(r.mesh.faces.len(), 6);
        assert_eq!(r.mesh.vertices.len(), mesh.vertices.len());
    }

    #[test]
    fn curve_steps_are_even_by_parameter() {
        let c = CurveSlice {
            points: vec![DVec3::ZERO, DVec3::new(1., 0., 1.), DVec3::new(2., 0., 0.)],
            steps: 6,
            ..Default::default()
        };
        let s = c.sampled();
        assert_eq!(s.len(), 7);
        assert_eq!(s[0], DVec3::ZERO);
        assert!((s[3] - DVec3::new(1., 0., 1.)).length() < 1e-9);
        assert_eq!(*s.last().unwrap(), DVec3::new(2., 0., 0.));
        let two = CurveSlice {
            points: vec![DVec3::ZERO, DVec3::X],
            ..Default::default()
        };
        // A straight two-point curve still gets its step points (PS-21).
        let line = two.sampled();
        assert_eq!(line.len(), 25);
        assert!((line[12] - DVec3::new(0.5, 0., 0.)).length() < 1e-12);
    }

    #[test]
    fn primitive_rectangle_circle_and_clones() {
        let mut o = PrimitiveSlice {
            place: Some(ShapePlace::on(DVec3::ZERO, DVec3::Y)),
            ..Default::default()
        };
        let paths = o.paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].points.len(), 4);
        assert!(paths[0].points.iter().all(|p| p.y.abs() < 1e-12));
        o.corner_radius = 0.1;
        assert_eq!(o.paths()[0].points.len(), 4 * 9);
        o.shape = PrimitiveShape::Circle;
        assert_eq!(o.paths()[0].points.len(), 24);
        o.clone_x = 3;
        o.clone_y = 2;
        assert_eq!(o.paths().len(), 6);
    }
}
