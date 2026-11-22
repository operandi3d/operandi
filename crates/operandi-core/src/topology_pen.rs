//! Topology Pen v2 (BEHAVIOR-TOPOLOGY-PEN, TP-xx, 2026-11-20): the pen's
//! modes and options, and the one-step edits behind them. The viewport
//! side (gestures, pre-highlight) is `operandi-app/src/topology_pen_ui.rs`.
//!
//! Every edit here is one undo step; a refusal changes nothing. Points
//! are in the mesh's own coordinates unless a name says world.
//!
//! Provisional readings (TPQ-1..7, docs/SPECIFICATION_QUESTIONS.md,
//! "Topology Pen v2 (TP-xx) 2026-11-20") are marked where they apply.
use crate::edge_commands::{OutFace, rebuild};
use crate::{Editor, Mesh, Mode, Result, TweakElement};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// What a plain left-drag does (TP-10). The order is the drop-down's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PenMode {
    #[default]
    Move,
    Duplicate,
    Remove,
    Split,
    AddLoop,
    Point,
    Fill,
    Smoothing,
}
impl PenMode {
    pub const ALL: [Self; 8] = [
        Self::Move,
        Self::Duplicate,
        Self::Remove,
        Self::Split,
        Self::AddLoop,
        Self::Point,
        Self::Fill,
        Self::Smoothing,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Duplicate => "Duplicate",
            Self::Remove => "Remove",
            Self::Split => "Split",
            Self::AddLoop => "Add Loop",
            Self::Point => "Point",
            Self::Fill => "Fill",
            Self::Smoothing => "Smoothing",
        }
    }
}

/// Topology Pen's Tool Properties. The values stick for the session (the
/// editor keeps a copy, `Editor::topology_pen_memory`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopologyPenOptions {
    pub mode: PenMode,
    /// Edges act on their whole loop (Move, Duplicate, Remove, Smoothing).
    pub edge_loop: bool,
    /// Remove (loop): keep the loop's end points even when they are left
    /// in only two edges.
    pub keep_vertices: bool,
    /// Add Loop: where the last loop went across its ring, 0..1.
    pub position: f64,
    /// Add Loop: always cut at 50 %.
    pub split_middle: bool,
    /// Fill: how far to look for points, in average edge lengths.
    pub fill_range: f64,
    /// Fill: only quads (off: a triangle when no quad fits).
    pub quad_only: bool,
    /// Smoothing.
    pub strength: f64,
    pub iterations: u32,
    pub lock_boundary: bool,
    pub lock_corner: bool,
    /// Falloff radius (world metres); 0 = the whole active mesh for
    /// Smoothing and just the element for Move.
    pub radius: f64,
    pub use_connectivity: bool,
    pub shape: crate::FalloffShape,
    pub show_vertex: bool,
    pub show_edge: bool,
    /// Smart snapping: interior points may weld too (off: border→border).
    pub inner_snap: bool,
    /// After a weld, the dragged point's UVs win (off: the target's).
    pub copy_uvs: bool,
    /// The last drag's marker offset (world); typing moves the last element.
    pub offset: DVec3,
    /// Plain drags slide along edges.
    pub edge_slide: bool,
    /// Elements facing away can be grabbed.
    pub backface: bool,
    /// Points land on inactive meshes too when there's no background.
    pub raycast_inactive: bool,
}
impl Default for TopologyPenOptions {
    fn default() -> Self {
        Self {
            mode: PenMode::Move,
            edge_loop: false,
            keep_vertices: false,
            position: 0.5,
            split_middle: false,
            fill_range: 1.5,
            quad_only: true,
            strength: 1.,
            iterations: 1,
            lock_boundary: false,
            lock_corner: true,
            radius: 0.,
            use_connectivity: false,
            shape: crate::FalloffShape::Linear,
            show_vertex: true,
            show_edge: true,
            inner_snap: false,
            copy_uvs: false,
            offset: DVec3::ZERO,
            edge_slide: false,
            backface: false,
            raycast_inactive: false,
        }
    }
}
impl TopologyPenOptions {
    /// The Tweak options a Move drag runs with: the pen's falloff.
    pub fn tweak_options(&self, edge_loop: bool) -> crate::TweakOptions {
        crate::TweakOptions {
            mode: crate::TweakMode::Auto,
            range: self.radius.max(0.),
            connect: if edge_loop {
                crate::TweakConnect::EdgeLoops
            } else if self.use_connectivity {
                crate::TweakConnect::Connectivity
            } else {
                crate::TweakConnect::Ignore
            },
            shape: self.shape,
        }
    }
}

/// One Topology Pen edit (`Command::TopologyPen`), one undo step.
#[derive(Clone, Debug, PartialEq)]
pub enum PenEdit {
    /// TP-12 Remove: the element (or its loop), never a border edge.
    Remove {
        item: u64,
        element: TweakElement,
        edge_loop: bool,
        keep_vertices: bool,
    },
    /// TP-13 Split: cut the polygon both points are corners of.
    Split { item: u64, a: u32, b: u32 },
    /// TP-11 Duplicate a border vertex: a triangle with the neighbour
    /// whose edge is nearest the drag (provisional, TPQ-2). `offset` is
    /// world.
    DuplicateVertex { item: u64, v: u32, offset: DVec3 },
    /// TP-11 Duplicate the whole border loop through `a`–`b` as one row.
    DuplicateLoop {
        item: u64,
        a: u32,
        b: u32,
        offset: DVec3,
    },
    /// TP-15 Point: a one-point polygon at a world position.
    Point { world: DVec3 },
    /// TP-16 Fill the gap at `click` (world); `toward_viewer` is the world
    /// direction to the camera.
    Fill {
        item: u64,
        click: DVec3,
        toward_viewer: DVec3,
        range: f64,
        quad_only: bool,
    },
    /// TP-20 Ctrl+click empty space: a square quad facing the viewer.
    /// World centre, the view's right and up, and the side length.
    Quad {
        center: DVec3,
        right: DVec3,
        up: DVec3,
        side: f64,
    },
    /// TP-30 smart snapping: the drag's `moves` (local), then each
    /// `(from, to)` point welds, as one step.
    Weld {
        item: u64,
        pairs: Vec<(u32, u32)>,
        moves: Vec<(u32, DVec3)>,
        copy_uvs: bool,
    },
    /// Move/Slide/Smoothing results: these points to these positions
    /// (local), then nothing else. Locked and hidden points don't move.
    MovePoints {
        item: u64,
        points: Vec<(u32, DVec3)>,
        label: String,
    },
}

// ------------------------------------------------------------ loops

/// The edge loop through `a`–`b`, by the Select Loop rule (stops at a
/// triangle, n-gon, pole or border). At least the edge itself.
pub fn pen_edge_loop(scene: &crate::Scene, item: u64, a: u32, b: u32) -> Vec<(u32, u32)> {
    let mut probe = scene.clone();
    probe.selection.mode = Mode::Edge;
    match probe.select_edge_loop(item, (a, b), false) {
        Ok(_) => {
            let mut edges: Vec<(u32, u32)> = probe
                .selection
                .edges
                .iter()
                .map(|&(x, y)| key(x, y))
                .collect();
            edges.sort_unstable();
            edges.dedup();
            edges
        }
        Err(_) => vec![key(a, b)],
    }
}

/// Whether `a`–`b` is a border edge (one polygon).
pub fn is_border_edge(mesh: &Mesh, a: u32, b: u32) -> bool {
    mesh.edge_faces()
        .get(&key(a, b))
        .is_some_and(|f| f.len() == 1)
}

/// The border points `v` is joined to by border edges.
pub fn border_neighbours(mesh: &Mesh, v: u32) -> Vec<u32> {
    let faces = mesh.point_faces();
    let mut out = Vec::new();
    for &f in faces.of(v) {
        if mesh.is_polyline(f as usize) {
            continue;
        }
        let p = &mesh.faces[f as usize].vertices;
        let n = p.len();
        for i in 0..n {
            if p[i] == v {
                for u in [p[(i + n - 1) % n], p[(i + 1) % n]] {
                    if u != v && !out.contains(&u) && is_border_edge(mesh, v, u) {
                        out.push(u);
                    }
                }
            }
        }
    }
    out
}

/// The border loop through border edge `a`–`b`: the chain of border edges
/// walked from both ends until it closes or reaches a point where the
/// border doesn't continue one way.
pub fn border_loop(mesh: &Mesh, a: u32, b: u32) -> Vec<(u32, u32)> {
    let mut edges = vec![key(a, b)];
    if !is_border_edge(mesh, a, b) {
        return edges;
    }
    for (from, start) in [(a, b), (b, a)] {
        let (mut prev, mut at) = (from, start);
        loop {
            let next: Vec<u32> = border_neighbours(mesh, at)
                .into_iter()
                .filter(|&u| u != prev)
                .collect();
            let [next] = next[..] else { break };
            let e = key(at, next);
            if edges.contains(&e) {
                break;
            }
            edges.push(e);
            prev = at;
            at = next;
        }
    }
    edges
}

// ------------------------------------------------------------ duplicate

/// The polygon side `a`→`b` or `b`→`a` lies on: (face, corner of a,
/// corner of b, whether the side runs a→b).
fn side_of(mesh: &Mesh, a: u32, b: u32) -> Option<(usize, usize, usize, bool)> {
    mesh.point_faces().of(a).iter().find_map(|&fi| {
        let fi = fi as usize;
        if mesh.is_polyline(fi) {
            return None;
        }
        let v = &mesh.faces[fi].vertices;
        let k = v.len();
        (0..k).find_map(|i| {
            let (x, y) = (v[i], v[(i + 1) % k]);
            if (x, y) == (a, b) {
                Some((fi, i, (i + 1) % k, true))
            } else if (x, y) == (b, a) {
                Some((fi, (i + 1) % k, i, false))
            } else {
                None
            }
        })
    })
}

/// TC-45 / TP-11: pull a row of quads out of border edges `edges`; each
/// edge point `p` gets a new point at `place(p)`, shared between edges
/// that meet. New polygons copy the source polygon's tags (material,
/// part) and face its way.
pub fn pull_out_edges(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    place: impl Fn(u32) -> DVec3,
) -> Result<Mesh> {
    let mut vertices = mesh.vertices.clone();
    let mut fresh: BTreeMap<u32, u32> = BTreeMap::new();
    let mut copies = Vec::new();
    let mut faces: Vec<OutFace> = (0..mesh.faces.len())
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect();
    for &(a, b) in edges {
        let Some((template, ca, cb, forward)) = side_of(mesh, a, b) else {
            return Err("Topology Pen: that edge isn't the side of a polygon".into());
        };
        let mut new_point = |p: u32| -> Result<u32> {
            if let Some(&n) = fresh.get(&p) {
                return Ok(n);
            }
            let n = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
            vertices.push(place(p));
            fresh.insert(p, n);
            copies.push((n, p));
            Ok(n)
        };
        let (ia, ib) = (new_point(a)?, new_point(b)?);
        let copy = |c: usize| Some(vec![((template, c), 1.)]);
        // The shared edge runs the other way in the new quad, so both
        // polygons face the same side.
        faces.push(if forward {
            OutFace {
                template,
                vertices: vec![b, a, ia, ib],
                donors: vec![Some((template, cb)), Some((template, ca)), None, None],
                blends: vec![None, None, copy(ca), copy(cb)],
            }
        } else {
            OutFace {
                template,
                vertices: vec![a, b, ib, ia],
                donors: vec![Some((template, ca)), Some((template, cb)), None, None],
                blends: vec![None, None, copy(cb), copy(ca)],
            }
        });
    }
    let out = rebuild(mesh, vertices, faces, &copies, "Topology Pen")?;
    out.validate()
        .map_err(|e| format!("Topology Pen result failed validation ({e}); nothing changed"))?;
    Ok(out)
}

/// TP-11 (provisional, TPQ-2): the border neighbour of `v` whose edge
/// points nearest `direction`.
pub fn duplicate_partner(mesh: &Mesh, v: u32, direction: DVec3) -> Option<u32> {
    let at = *mesh.vertices.get(v as usize)?;
    let d = direction.normalize_or_zero();
    border_neighbours(mesh, v).into_iter().max_by(|&x, &y| {
        let cx = (mesh.vertices[x as usize] - at).normalize_or_zero().dot(d);
        let cy = (mesh.vertices[y as usize] - at).normalize_or_zero().dot(d);
        cx.total_cmp(&cy)
    })
}

/// TP-11: a new triangle from border vertex `v`, its partner `u` and a new
/// point at `p`, facing the way the polygon on `v`–`u` does.
pub fn duplicate_vertex_geometry(mesh: &Mesh, v: u32, u: u32, p: DVec3) -> Result<Mesh> {
    let Some((template, cv, cu, forward)) = side_of(mesh, v, u) else {
        return Err("Topology Pen: that point has no border polygon".into());
    };
    let mut vertices = mesh.vertices.clone();
    let n = u32::try_from(vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
    vertices.push(p);
    let mut faces: Vec<OutFace> = (0..mesh.faces.len())
        .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
        .collect();
    let copy = |c: usize| Some(vec![((template, c), 1.)]);
    faces.push(if forward {
        OutFace {
            template,
            vertices: vec![u, v, n],
            donors: vec![Some((template, cu)), Some((template, cv)), None],
            blends: vec![None, None, copy(cv)],
        }
    } else {
        OutFace {
            template,
            vertices: vec![v, u, n],
            donors: vec![Some((template, cv)), Some((template, cu)), None],
            blends: vec![None, None, copy(cv)],
        }
    });
    let out = rebuild(mesh, vertices, faces, &[(n, v)], "Topology Pen")?;
    out.validate()
        .map_err(|e| format!("Topology Pen result failed validation ({e}); nothing changed"))?;
    Ok(out)
}

// ------------------------------------------------------------ fill

/// A basis of the plane facing `normal`.
fn plane_axes(normal: DVec3) -> (DVec3, DVec3) {
    let n = normal.normalize_or_zero();
    let helper = if n.x.abs() < 0.9 { DVec3::X } else { DVec3::Y };
    let u = n.cross(helper).normalize_or_zero();
    (u, n.cross(u))
}

fn cross2(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Whether segments p–q and r–s cross properly (not at an end).
fn segments_cross(p: (f64, f64), q: (f64, f64), r: (f64, f64), s: (f64, f64)) -> bool {
    let d1 = cross2(p, q, r);
    let d2 = cross2(p, q, s);
    let d3 = cross2(r, s, p);
    let d4 = cross2(r, s, q);
    d1 * d2 < 0. && d3 * d4 < 0.
}

/// Whether flat point `p` is inside flat polygon `ring`.
fn inside(ring: &[(f64, f64)], p: (f64, f64)) -> bool {
    let mut odd = false;
    let n = ring.len();
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0) {
            odd = !odd;
        }
    }
    odd
}

/// TP-16 Fill: the points of the polygon a click at `click` would make,
/// ordered to face `toward_viewer`, or `None` when nothing fits. Points
/// within `range` × the average edge length near the click count; the
/// nearest four that make a convex ring around the click win, crossing no
/// existing edge and covering no existing polygon (Quad Only off: a
/// triangle when no quad fits). Provisional (TPQ-5): with no edges near
/// the click, the average spacing of the nearest points stands in for the
/// edge length.
pub fn fill_ring(
    mesh: &Mesh,
    click: DVec3,
    toward_viewer: DVec3,
    range: f64,
    quad_only: bool,
) -> Option<Vec<u32>> {
    let used = mesh.used_points();
    let mut near: Vec<(f64, u32)> = (0..mesh.vertices.len() as u32)
        .filter(|&v| used[v as usize])
        .map(|v| (mesh.vertices[v as usize].distance(click), v))
        .collect();
    near.sort_by(|x, y| x.0.total_cmp(&y.0));
    near.truncate(8);
    if near.len() < 3 {
        return None;
    }
    let nearest: BTreeSet<u32> = near.iter().map(|&(_, v)| v).collect();
    let edges = mesh.edges();
    let lengths: Vec<f64> = edges
        .iter()
        .filter(|(a, b)| nearest.contains(a) || nearest.contains(b))
        .map(|&(a, b)| mesh.vertices[a as usize].distance(mesh.vertices[b as usize]))
        .collect();
    let average = if lengths.is_empty() {
        let spacing: Vec<f64> = near
            .iter()
            .map(|&(_, v)| {
                near.iter()
                    .filter(|&&(_, u)| u != v)
                    .map(|&(_, u)| mesh.vertices[v as usize].distance(mesh.vertices[u as usize]))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect();
        spacing.iter().sum::<f64>() / spacing.len() as f64
    } else {
        lengths.iter().sum::<f64>() / lengths.len() as f64
    };
    let reach = range.max(0.) * average;
    let pool: Vec<u32> = near
        .iter()
        .filter(|&&(d, _)| d <= reach)
        .map(|&(_, v)| v)
        .collect();
    let (u, w) = plane_axes(toward_viewer);
    let flat = |p: DVec3| ((p - click).dot(u), (p - click).dot(w));
    let origin = (0., 0.);
    // The click must be in a gap, not on an existing polygon.
    for (fi, f) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) || f.vertices.len() < 3 {
            continue;
        }
        let ring: Vec<(f64, f64)> = f
            .vertices
            .iter()
            .map(|&v| flat(mesh.vertices[v as usize]))
            .collect();
        if inside(&ring, origin) {
            return None;
        }
    }
    let edge_counts = mesh.edge_counts();
    let try_size = |k: usize| -> Option<Vec<u32>> {
        let mut best: Option<(f64, Vec<u32>)> = None;
        let n = pool.len();
        let mut pick = |chosen: &[u32]| {
            let mut ring: Vec<u32> = chosen.to_vec();
            ring.sort_by(|&x, &y| {
                let (px, py) = flat(mesh.vertices[x as usize]);
                let (qx, qy) = flat(mesh.vertices[y as usize]);
                py.atan2(px).total_cmp(&qy.atan2(qx))
            });
            let pts: Vec<(f64, f64)> = ring
                .iter()
                .map(|&v| flat(mesh.vertices[v as usize]))
                .collect();
            // Convex, counter-clockwise seen from the viewer, around the
            // click.
            let convex = (0..k).all(|i| cross2(pts[i], pts[(i + 1) % k], pts[(i + 2) % k]) > 1e-12);
            let around = (0..k).all(|i| cross2(pts[i], pts[(i + 1) % k], origin) > 0.);
            if !convex || !around {
                return;
            }
            // No side may cross an existing edge or already have two
            // polygons.
            for i in 0..k {
                let (a, b) = (ring[i], ring[(i + 1) % k]);
                if edge_counts.get(&key(a, b)).is_some_and(|&c| c >= 2) {
                    return;
                }
                for &(x, y) in &edges {
                    if [x, y].contains(&a) || [x, y].contains(&b) {
                        continue;
                    }
                    if segments_cross(
                        pts[i],
                        pts[(i + 1) % k],
                        flat(mesh.vertices[x as usize]),
                        flat(mesh.vertices[y as usize]),
                    ) {
                        return;
                    }
                }
            }
            // No existing polygon inside it (an existing point inside
            // means it would cover geometry).
            if mesh
                .vertices
                .iter()
                .enumerate()
                .any(|(v, &p)| used[v] && !ring.contains(&(v as u32)) && inside(&pts, flat(p)))
            {
                return;
            }
            let score: f64 = ring
                .iter()
                .map(|&v| mesh.vertices[v as usize].distance(click))
                .sum();
            if best.as_ref().is_none_or(|(s, _)| score < *s) {
                best = Some((score, ring));
            }
        };
        let mut idx: Vec<usize> = (0..k).collect();
        if n < k {
            return None;
        }
        loop {
            let chosen: Vec<u32> = idx.iter().map(|&i| pool[i]).collect();
            pick(&chosen);
            // Next combination.
            let mut i = k;
            while i > 0 && idx[i - 1] == n - k + i - 1 {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            idx[i - 1] += 1;
            for j in i..k {
                idx[j] = idx[j - 1] + 1;
            }
        }
        best.map(|(_, r)| r)
    };
    try_size(4).or_else(|| if quad_only { None } else { try_size(3) })
}

// ------------------------------------------------------------ smoothing

/// TP-17 Smoothing on the press-time mesh: each point of `weights` moves
/// toward its neighbours' average by Strength × weight, `iterations`
/// times. Border points slide along the border (both neighbours on it);
/// Lock Boundary keeps them still, Lock Corner keeps border points of
/// only one polygon still. `along` (an edge loop) limits the neighbours to
/// the loop's, so points slide along it; its open ends stay. Returns the
/// new positions of the points that moved.
pub fn smooth_points(
    mesh: &Mesh,
    weights: &[(u32, f64)],
    strength: f64,
    iterations: u32,
    lock_boundary: bool,
    lock_corner: bool,
    along: Option<&[(u32, u32)]>,
) -> Vec<(u32, DVec3)> {
    let n = mesh.vertices.len();
    let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut border_ring: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let edge_faces = mesh.edge_faces();
    let wanted: BTreeSet<u32> = weights.iter().map(|&(v, _)| v).collect();
    let edges: Vec<(u32, u32)> = match along {
        Some(l) => l.to_vec(),
        None => mesh.edges(),
    };
    for &(a, b) in &edges {
        let border = edge_faces.get(&key(a, b)).is_some_and(|f| f.len() == 1);
        for (x, y) in [(a, b), (b, a)] {
            if wanted.contains(&x) {
                neighbours.entry(x).or_default().push(y);
                if border {
                    border_ring.entry(x).or_default().push(y);
                }
            }
        }
    }
    let faces = mesh.point_faces();
    let mut out = mesh.vertices.clone();
    let strength = strength.clamp(0., 1.);
    for _ in 0..iterations.max(1) {
        let before = out.clone();
        for &(v, w) in weights {
            let vi = v as usize;
            if vi >= n || w <= 0. {
                continue;
            }
            let ring = match border_ring.get(&v) {
                Some(b) if along.is_none() => {
                    if lock_boundary {
                        continue;
                    }
                    let polys = faces
                        .of(v)
                        .iter()
                        .filter(|&&f| !mesh.is_polyline(f as usize))
                        .count();
                    if lock_corner && polys <= 1 {
                        continue;
                    }
                    b
                }
                _ => match neighbours.get(&v) {
                    Some(r) => r,
                    None => continue,
                },
            };
            // An open loop's ends (one loop neighbour) stay put.
            if ring.len() < 2 {
                continue;
            }
            let average =
                ring.iter().map(|&u| before[u as usize]).sum::<DVec3>() / ring.len() as f64;
            out[vi] = before[vi] + (average - before[vi]) * strength * w.min(1.);
        }
    }
    weights
        .iter()
        .filter(|&&(v, _)| (v as usize) < n && out[v as usize] != mesh.vertices[v as usize])
        .map(|&(v, _)| (v, out[v as usize]))
        .collect()
}

/// The Smoothing falloff: every point of the mesh at Radius 0, else the
/// points within `radius` of `center` weighted by `shape` (straight-line
/// distance, TP-21).
pub fn falloff_weights(
    mesh: &Mesh,
    center: DVec3,
    radius: f64,
    shape: crate::FalloffShape,
) -> Vec<(u32, f64)> {
    let used = mesh.used_points();
    (0..mesh.vertices.len() as u32)
        .filter(|&v| used[v as usize])
        .filter_map(|v| {
            if radius <= 0. {
                return Some((v, 1.));
            }
            let d = mesh.vertices[v as usize].distance(center);
            (d < radius).then(|| (v, shape.weight(d / radius)))
        })
        .collect()
}

// ------------------------------------------------------------ weld

/// TP-30: weld each `(from, to)` point pair: `from` merges into `to`.
/// Polygons drop repeated points and go when they have fewer than three
/// left. `copy_uvs` (Copy Discontinuous UVs): the dragged point's UVs win;
/// off, the target's (welding keeps the target's corner data already).
pub fn weld_geometry(mesh: &Mesh, pairs: &[(u32, u32)], copy_uvs: bool) -> Result<Mesh> {
    let n = mesh.vertices.len() as u32;
    let mut map: Vec<u32> = (0..n).collect();
    for &(from, to) in pairs {
        if from >= n || to >= n || from == to {
            return Err("Topology Pen: that weld point no longer exists".into());
        }
        map[from as usize] = to;
    }
    // Chains (a→b, b→c) end at their last target.
    for v in 0..n as usize {
        let mut t = map[v];
        let mut guard = 0;
        while map[t as usize] != t && guard < pairs.len() + 1 {
            t = map[t as usize];
            guard += 1;
        }
        map[v] = t;
    }
    let mut vertices = mesh.vertices.clone();
    if copy_uvs {
        // The dragged point's position is the target's either way; only
        // corner data differs, and donors below carry it.
    }
    for &(from, to) in pairs {
        vertices[from as usize] = mesh.vertices[map[to as usize] as usize];
    }
    let mut faces: Vec<OutFace> = Vec::with_capacity(mesh.faces.len());
    for (fi, f) in mesh.faces.iter().enumerate() {
        let mut verts: Vec<u32> = Vec::new();
        let mut donors = Vec::new();
        let k = f.vertices.len();
        for c in 0..k {
            let v = map[f.vertices[c] as usize];
            if verts.last() == Some(&v) || (c == k - 1 && verts.first() == Some(&v)) {
                // A repeated point: the corner kept is the dragged one's
                // with Copy Discontinuous UVs, else the target's.
                if copy_uvs && f.vertices[c] != v {
                    let last = donors.len() - 1;
                    donors[last] = Some((fi, c));
                }
                continue;
            }
            verts.push(v);
            donors.push(Some((fi, c)));
        }
        let min = if mesh.is_polyline(fi) { 1 } else { 3 };
        if verts.len() < min {
            continue;
        }
        faces.push(OutFace {
            template: fi,
            vertices: verts,
            donors,
            blends: Vec::new(),
        });
    }
    let out = rebuild(mesh, vertices, faces, &[], "Topology Pen")?;
    let keep: BTreeSet<u32> = pairs.iter().map(|&(from, _)| from).collect();
    let out = crate::mesh_cleanup::remove_unreferenced(&out, Some(&keep))?.mesh;
    out.validate()
        .map_err(|e| format!("Topology Pen weld failed validation ({e}); nothing changed"))?;
    Ok(out)
}

// ------------------------------------------------------------ editor

impl Editor {
    /// The first background point nearest `world`, or `world` itself.
    pub fn land_on_background(&self, world: DVec3) -> DVec3 {
        self.background_cache
            .get(&self.scene, None)
            .ok()
            .and_then(|b| b.nearest(world))
            .map_or(world, |hit| hit.point)
    }

    /// Whether there is a background mesh to put points on.
    pub fn has_background(&self) -> bool {
        self.background_cache
            .get(&self.scene, None)
            .is_ok_and(|b| !b.is_empty())
    }

    fn pen_target(&self, item: u64) -> Result<(u64, Mesh, glam::DMat4)> {
        let id = self.edge_insert_target("Topology Pen")?;
        if self.scene.selection.mode == Mode::Item {
            return Err("Topology Pen: pick a component mode (Vertex, Edge or Polygon)".into());
        }
        if id != item {
            return Err("Topology Pen works on the active mesh".into());
        }
        let active = self.scene.active().ok_or("Select a mesh")?;
        Ok((id, (*active.mesh).clone(), self.scene.world_matrix(id)?))
    }

    fn pinned(&self) -> Vec<bool> {
        self.scene
            .active()
            .and_then(|i| crate::Protection::of(&i.mesh, &i.elements).map(|p| p.pinned))
            .unwrap_or_default()
    }

    /// `Command::TopologyPen`.
    pub(crate) fn topology_pen(&mut self, edit: PenEdit) -> Result<()> {
        match edit {
            PenEdit::Remove {
                item,
                element,
                edge_loop,
                keep_vertices,
            } => self.pen_remove(item, element, edge_loop, keep_vertices),
            PenEdit::Split { item, a, b } => self.pen_split(item, a, b),
            PenEdit::DuplicateVertex { item, v, offset } => {
                let (id, mesh, world) = self.pen_target(item)?;
                if border_neighbours(&mesh, v).is_empty() {
                    return Err("Duplicate needs a border edge".into());
                }
                let local = world.inverse();
                let dir = local.transform_vector3(offset);
                let u = duplicate_partner(&mesh, v, dir).ok_or("Duplicate needs a border edge")?;
                let p = self
                    .land_on_background(world.transform_point3(mesh.vertices[v as usize]) + offset);
                let out = duplicate_vertex_geometry(&mesh, v, u, local.transform_point3(p))?;
                self.publish_vertex_edit(id, out, "Topology Pen Duplicate", Vec::new())
            }
            PenEdit::DuplicateLoop { item, a, b, offset } => {
                let (id, mesh, world) = self.pen_target(item)?;
                if !is_border_edge(&mesh, a, b) {
                    return Err("Duplicate needs a border edge".into());
                }
                let edges = border_loop(&mesh, a, b);
                let local = world.inverse();
                let out = pull_out_edges(&mesh, &edges, |p| {
                    local.transform_point3(self.land_on_background(
                        world.transform_point3(mesh.vertices[p as usize]) + offset,
                    ))
                })?;
                self.publish_vertex_edit(id, out, "Topology Pen Duplicate", Vec::new())
            }
            PenEdit::Point { world: at } => {
                let id = self.edge_insert_target("Topology Pen")?;
                if self.scene.selection.mode == Mode::Item {
                    return Err(
                        "Topology Pen: pick a component mode (Vertex, Edge or Polygon)".into(),
                    );
                }
                let active = self.scene.active().ok_or("Select a mesh")?;
                let local = self.scene.world_matrix(id)?.inverse().transform_point3(at);
                let (out, ids) = crate::add_points(&active.mesh, &[local])?;
                let (out, _) = crate::make_polygon_typed(&out, &ids, crate::MakeType::Auto, false)?;
                self.publish_vertex_edit(id, out, "Topology Pen Point", Vec::new())
            }
            PenEdit::Fill {
                item,
                click,
                toward_viewer,
                range,
                quad_only,
            } => {
                let (id, mesh, world) = self.pen_target(item)?;
                let local = world.inverse();
                let ring = fill_ring(
                    &mesh,
                    local.transform_point3(click),
                    local.transform_vector3(toward_viewer),
                    range,
                    quad_only,
                )
                .ok_or(if quad_only {
                    "Fill: no four points around the click make a quad (Quad Only)"
                } else {
                    "Fill: no points around the click make a polygon"
                })?;
                let out = fill_geometry(&mesh, &ring)?;
                self.publish_vertex_edit(id, out, "Topology Pen Fill", Vec::new())
            }
            PenEdit::Quad {
                center,
                right,
                up,
                side,
            } => {
                let id = self.edge_insert_target("Topology Pen")?;
                if !(side.is_finite() && side > 0.) {
                    return Err("Topology Pen: the quad needs a size".into());
                }
                let active = self.scene.active().ok_or("Select a mesh")?;
                let local = self.scene.world_matrix(id)?.inverse();
                let h = side / 2.;
                let (r, u) = (right.normalize_or_zero() * h, up.normalize_or_zero() * h);
                let corners: Vec<DVec3> = [-r - u, r - u, r + u, -r + u]
                    .iter()
                    .map(|&c| local.transform_point3(center + c))
                    .collect();
                let (out, ids) = crate::add_points(&active.mesh, &corners)?;
                let (out, _) = crate::make_polygon_typed(&out, &ids, crate::MakeType::Face, true)?;
                self.publish_vertex_edit(id, out, "Topology Pen Quad", Vec::new())
            }
            PenEdit::Weld {
                item,
                pairs,
                moves,
                copy_uvs,
            } => {
                let (id, mut mesh, _) = self.pen_target(item)?;
                for (v, p) in moves {
                    if let Some(at) = mesh.vertices.get_mut(v as usize)
                        && p.is_finite()
                    {
                        *at = p;
                    }
                }
                let pinned = self.pinned();
                if pairs.iter().any(|&(f, t)| {
                    pinned.get(f as usize).copied().unwrap_or(false)
                        || pinned.get(t as usize).copied().unwrap_or(false)
                }) {
                    return Err("Topology Pen: locked or hidden points don't weld".into());
                }
                let out = weld_geometry(&mesh, &pairs, copy_uvs)?;
                self.publish_vertex_edit(id, out, "Topology Pen Weld", Vec::new())
            }
            PenEdit::MovePoints {
                item,
                points,
                label,
            } => {
                let (id, mut mesh, _) = self.pen_target(item)?;
                let pinned = self.pinned();
                let mut changed = false;
                for (v, p) in points {
                    let Some(at) = mesh.vertices.get_mut(v as usize) else {
                        return Err("Topology Pen: a moved point no longer exists".into());
                    };
                    if !p.is_finite() || p.abs().max_element() > 1e9 {
                        return Err("Topology Pen: points must stay within the scene range".into());
                    }
                    if pinned.get(v as usize).copied().unwrap_or(false) || *at == p {
                        continue;
                    }
                    *at = p;
                    changed = true;
                }
                if !changed {
                    return Err("Topology Pen: nothing moved".into());
                }
                let keep = self.scene.selection.vertices.clone();
                self.publish_vertex_edit(id, mesh, &label, keep)
            }
        }
    }

    fn pen_remove(
        &mut self,
        item: u64,
        element: TweakElement,
        edge_loop: bool,
        keep_vertices: bool,
    ) -> Result<()> {
        let (id, mesh, _) = self.pen_target(item)?;
        let out = match element {
            TweakElement::Polygon(f) => {
                if f as usize >= mesh.faces.len() {
                    return Err("Topology Pen: that polygon no longer exists".into());
                }
                crate::polygon_remove::delete_geometry(&mesh, &[f], &[])?
            }
            TweakElement::Edge(a, b) => {
                let mut edges = if edge_loop {
                    pen_edge_loop(&self.scene, id, a, b)
                } else {
                    vec![key(a, b)]
                };
                // TP-12: border edges are never removed.
                edges.retain(|&(x, y)| !is_border_edge(&mesh, x, y));
                if edges.is_empty() {
                    return Err("Topology Pen: border edges can't be removed".into());
                }
                remove_edges(&mesh, &edges, keep_vertices)?
            }
            TweakElement::Vertex(v) => {
                let edges: Vec<(u32, u32)> = mesh
                    .edges()
                    .into_iter()
                    .filter(|&(x, y)| x == v || y == v)
                    .collect();
                if edges.is_empty() {
                    // A lone point (a one-point polygon): it goes.
                    let faces: Vec<u32> = mesh.point_faces().of(v).to_vec();
                    crate::polygon_remove::delete_geometry(&mesh, &faces, &[v])?
                } else if edges.iter().any(|&(x, y)| is_border_edge(&mesh, x, y)) {
                    // Provisional (TPQ-3): a border point isn't dissolved.
                    return Err("Topology Pen: border points can't be removed".into());
                } else {
                    crate::edge_delete_geometry(&mesh, &edges)?
                }
            }
        };
        self.publish_vertex_edit(id, out, "Topology Pen Remove", Vec::new())
    }

    fn pen_split(&mut self, item: u64, a: u32, b: u32) -> Result<()> {
        let (_, mesh, _) = self.pen_target(item)?;
        if a == b {
            return Err("Split: release on another point of the same polygon".into());
        }
        if mesh.edges().contains(&key(a, b)) {
            return Err("Split: those points are already joined by an edge".into());
        }
        let faces = mesh.point_faces();
        let shared = faces
            .of(a)
            .iter()
            .filter(|&&f| {
                !mesh.is_polyline(f as usize) && mesh.faces[f as usize].vertices.contains(&b)
            })
            .count();
        if shared == 0 {
            return Err("Split: those points aren't on the same polygon".into());
        }
        let keep_mode = self.scene.selection.mode;
        let keep_vertices = std::mem::replace(&mut self.scene.selection.vertices, vec![a, b]);
        self.scene.selection.mode = Mode::Vertex;
        let faces_before = mesh.faces.len();
        let done = self.split_polygon();
        self.scene.selection.mode = keep_mode;
        self.scene.selection.vertices = keep_vertices;
        done?;
        if self.scene.active().map_or(0, |i| i.mesh.faces.len()) == faces_before {
            return Err("Split: nothing to cut there".into());
        }
        Ok(())
    }
}

/// TP-12: dissolve interior `edges`; with Keep Vertices off, the points
/// they leave in only two edges go too.
pub fn remove_edges(mesh: &Mesh, edges: &[(u32, u32)], keep_vertices: bool) -> Result<Mesh> {
    if keep_vertices {
        Ok(crate::edge_command_geometry(mesh, edges, crate::EdgeCommand::Remove)?.0)
    } else {
        crate::edge_delete_geometry(mesh, edges)
    }
}

/// TP-16: the Fill polygon from `ring` (ordered), and the one-point
/// polygons on its points go.
pub fn fill_geometry(mesh: &Mesh, ring: &[u32]) -> Result<Mesh> {
    let singles: Vec<u32> = (0..mesh.faces.len())
        .filter(|&f| mesh.faces[f].vertices.len() == 1 && ring.contains(&mesh.faces[f].vertices[0]))
        .map(|f| f as u32)
        .collect();
    let base = if singles.is_empty() {
        mesh.clone()
    } else {
        let faces: Vec<OutFace> = (0..mesh.faces.len())
            .filter(|f| !singles.contains(&(*f as u32)))
            .map(|f| OutFace::copy(mesh, f, mesh.faces[f].vertices.clone()))
            .collect();
        rebuild(mesh, mesh.vertices.clone(), faces, &[], "Fill")?
    };
    let (out, _) = crate::make_polygon_typed(&base, ring, crate::MakeType::Face, true)?;
    Ok(out)
}
