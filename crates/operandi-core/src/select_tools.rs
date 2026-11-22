//! Select tab tools (modelling parity batch 4): Shortest Path, Sharp,
//! Coplanar Edges, Colinear Vertices, Diagonal, Coplanar Polygons and
//! Conditional Loop. Spec: modelling-parity-specs-2026-11-17 SELECTION-TOOLS
//! v1 (SL-xx) and selection-cases.json. Open readings: SL-Q1..Q5 in
//! docs/SPECIFICATION_QUESTIONS.md.
//!
//! Every command here is one undo step that changes only the selection
//! (`Editor::select_tool`). The functions take the mesh and return the
//! elements to select; the editor applies them and adds the mirrored
//! elements when Symmetry is on (as Select Symmetrical does).
use crate::{DetriangulateOptions, Mesh, Mode, Result, Scene};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// What one of the commands works out.
#[derive(Clone, Debug, PartialEq)]
pub enum SelectRequest {
    /// SL-01/02: add the shortest path between the first and last selected
    /// element.
    ShortestPath,
    /// SL-03 Shortest Path Mode: the path from the element selected before
    /// the last to the last, added to the selection.
    PathToLast,
    /// SL-10..12: edges whose polygons meet at a normal angle in
    /// `min..=max` degrees.
    Sharp { min: f64, max: f64 },
    /// SL-20..22: edges whose two polygons are flat within `flatness`.
    CoplanarEdges { flatness: f64 },
    /// SL-30/31: outline points on the line through their neighbours.
    ColinearVertices { distance: f64 },
    /// SL-40..44: the edges Detriangulate would remove.
    Diagonal(DetriangulateOptions),
    /// SL-50..53: polygons facing like the start polygons.
    CoplanarPolygons {
        start: Vec<u32>,
        options: CoplanarPolygonOptions,
    },
    /// SL-60..62: each selected edge grown into a loop with stop rules.
    ConditionalLoop(ConditionalLoopOptions),
}
impl SelectRequest {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ShortestPath | Self::PathToLast => "Shortest Path",
            Self::Sharp { .. } => "Select Sharp Edges",
            Self::CoplanarEdges { .. } => "Select Coplanar Edges",
            Self::ColinearVertices { .. } => "Select Colinear Vertices",
            Self::Diagonal(_) => "Select Diagonal Edges",
            Self::CoplanarPolygons { .. } => "Coplanar Polygons",
            Self::ConditionalLoop(_) => "Conditional Loop",
        }
    }
}

/// SL-11 starting values.
pub const SHARP_MIN: f64 = 89.55;
pub const SHARP_MAX: f64 = 180.;
/// SL-21.
pub const COPLANAR_FLATNESS: f64 = 0.005;
/// SL-31.
pub const COLINEAR_DISTANCE: f64 = 0.000_001;

/// Coplanar Polygons Tool options (SL-53; Angle 5° on the button, SL-Q3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoplanarPolygonOptions {
    /// Degrees.
    pub angle: f64,
    pub connectivity: bool,
    /// Metres, used with Connectivity off.
    pub range: f64,
}
impl Default for CoplanarPolygonOptions {
    fn default() -> Self {
        Self {
            angle: 5.,
            connectivity: true,
            range: 1.,
        }
    }
}

/// Conditional Loop's stop rules (SL-61 starting values; meanings SL-62,
/// provisional SL-Q4). Selection Set and Seam Map are kept for scripts but
/// not tested yet (Operandi has no edge sets or seam maps to stop at).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionalLoopOptions {
    pub both_directions: bool,
    /// 0 = no cap.
    pub max_loop: u32,
    pub stop_shared_polygon: bool,
    pub stop_sharp: bool,
    /// Degrees of turn at a corner that count as sharp (SL-Q4).
    pub sharp_angle: f64,
    pub stop_length: bool,
    /// 3 = 300%.
    pub length_tolerance: f64,
    pub stop_selection: bool,
    pub selection_count: u32,
}
impl Default for ConditionalLoopOptions {
    fn default() -> Self {
        Self {
            both_directions: true,
            max_loop: 0,
            stop_shared_polygon: true,
            stop_sharp: true,
            // SL-64: an octagonal ring (45° turns) closes, a 90° fold
            // stops; 60° sits between (provisional).
            sharp_angle: 60.,
            stop_length: false,
            length_tolerance: 3.,
            stop_selection: true,
            selection_count: 2,
        }
    }
}

fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// The elements to select after a request; `None` fields stay as they are.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picked {
    pub vertices: Option<Vec<u32>>,
    pub edges: Option<Vec<(u32, u32)>>,
    pub faces: Option<Vec<u32>>,
    pub mode: Option<Mode>,
}

// ---------------------------------------------------------- Shortest path

/// A graph over one kind of element: neighbours and positions.
struct Graph {
    adjacent: Vec<Vec<usize>>,
    position: Vec<DVec3>,
}
impl Graph {
    /// SL-02: breadth-first distances from `to`, then a walk from `from`
    /// that always steps one closer, going straight where it can (then the
    /// lowest index), so on a grid it runs along the row then the column.
    fn path(&self, from: usize, to: usize, start_dir: Option<DVec3>) -> Option<Vec<usize>> {
        let n = self.adjacent.len();
        let mut dist = vec![usize::MAX; n];
        dist[to] = 0;
        let mut queue = VecDeque::from([to]);
        while let Some(v) = queue.pop_front() {
            for &u in &self.adjacent[v] {
                if dist[u] == usize::MAX {
                    dist[u] = dist[v] + 1;
                    queue.push_back(u);
                }
            }
        }
        if dist[from] == usize::MAX {
            return None;
        }
        let mut path = vec![from];
        let mut at = from;
        let mut dir = start_dir;
        while at != to {
            let mut next: Vec<usize> = self.adjacent[at]
                .iter()
                .copied()
                .filter(|&u| dist[u] + 1 == dist[at])
                .collect();
            next.sort_unstable();
            let score = |u: usize| {
                dir.map_or(0., |d| {
                    (self.position[u] - self.position[at])
                        .normalize_or_zero()
                        .dot(d)
                })
            };
            let best = next
                .iter()
                .copied()
                .fold(None::<usize>, |best, u| match best {
                    Some(b) if score(b) >= score(u) - 1e-9 => Some(b),
                    _ => Some(u),
                })?;
            dir = Some((self.position[best] - self.position[at]).normalize_or_zero());
            path.push(best);
            at = best;
        }
        Some(path)
    }
}

/// SL-01: the shortest path between the first and last selected element of
/// the selection's mode, both kept. Fewer than two, or no path: an error
/// and nothing changes (SP3).
pub fn shortest_path(mesh: &Mesh, selection: &crate::Selection) -> Result<Picked> {
    let no_path = "Shortest Path: the two elements aren't connected";
    match selection.mode {
        Mode::Vertex => {
            let (Some(&a), Some(&b)) = (selection.vertices.first(), selection.vertices.last())
            else {
                return Err(TWO.into());
            };
            if selection.vertices.len() < 2 || a == b {
                return Err(TWO.into());
            }
            let n = mesh.vertices.len();
            let mut adjacent = vec![Vec::new(); n];
            for (x, y) in mesh.edges() {
                adjacent[x as usize].push(y as usize);
                adjacent[y as usize].push(x as usize);
            }
            let g = Graph {
                adjacent,
                position: mesh.vertices.clone(),
            };
            let path = g.path(a as usize, b as usize, None).ok_or(no_path)?;
            let mut out = selection.vertices.clone();
            add_missing(&mut out, path.into_iter().map(|v| v as u32));
            Ok(Picked {
                vertices: Some(out),
                ..Default::default()
            })
        }
        Mode::Edge => {
            if selection.edges.len() < 2 {
                return Err(TWO.into());
            }
            let first = selection.edges[0];
            let last = *selection.edges.last().unwrap();
            let edges = mesh.edges();
            let index: BTreeMap<(u32, u32), usize> =
                edges.iter().enumerate().map(|(i, &e)| (e, i)).collect();
            let at = |e: (u32, u32)| index.get(&key(e.0, e.1)).copied();
            let (Some(a), Some(b)) = (at(first), at(last)) else {
                return Err("Shortest Path: a selected edge isn't in the mesh".into());
            };
            if a == b {
                return Err(TWO.into());
            }
            let mut by_point: Vec<Vec<usize>> = vec![Vec::new(); mesh.vertices.len()];
            for (i, &(x, y)) in edges.iter().enumerate() {
                by_point[x as usize].push(i);
                by_point[y as usize].push(i);
            }
            let adjacent = edges
                .iter()
                .enumerate()
                .map(|(i, &(x, y))| {
                    by_point[x as usize]
                        .iter()
                        .chain(&by_point[y as usize])
                        .copied()
                        .filter(|&j| j != i)
                        .collect()
                })
                .collect();
            let mid =
                |e: (u32, u32)| (mesh.vertices[e.0 as usize] + mesh.vertices[e.1 as usize]) / 2.;
            let g = Graph {
                adjacent,
                position: edges.iter().map(|&e| mid(e)).collect(),
            };
            // The first edge's own direction counts as "straight on".
            let dir = (mesh.vertices[first.1 as usize] - mesh.vertices[first.0 as usize])
                .normalize_or_zero();
            let path = g.path(a, b, Some(dir)).ok_or(no_path)?;
            let mut out = selection.edges.clone();
            let have: BTreeSet<(u32, u32)> = out.iter().map(|&(x, y)| key(x, y)).collect();
            out.extend(
                path.into_iter()
                    .map(|i| edges[i])
                    .filter(|e| !have.contains(e)),
            );
            Ok(Picked {
                edges: Some(out),
                ..Default::default()
            })
        }
        Mode::Polygon => {
            if selection.faces.len() < 2 {
                return Err(TWO.into());
            }
            let (a, b) = (selection.faces[0], *selection.faces.last().unwrap());
            let n = mesh.faces.len();
            if a == b || a as usize >= n || b as usize >= n {
                return Err(TWO.into());
            }
            let mut adjacent = vec![Vec::new(); n];
            for faces in mesh.edge_faces().values() {
                for &f in faces {
                    for &g in faces {
                        if f != g && !adjacent[f].contains(&g) {
                            adjacent[f].push(g);
                        }
                    }
                }
            }
            let g = Graph {
                adjacent,
                position: (0..n).map(|f| centroid(mesh, f)).collect(),
            };
            let path = g.path(a as usize, b as usize, None).ok_or(no_path)?;
            let mut out = selection.faces.clone();
            add_missing(&mut out, path.into_iter().map(|f| f as u32));
            Ok(Picked {
                faces: Some(out),
                ..Default::default()
            })
        }
        Mode::Item => Err("Shortest Path works on vertices, edges or polygons".into()),
    }
}
const TWO: &str = "Shortest Path needs two selected elements: select a start and an end";

fn add_missing(out: &mut Vec<u32>, more: impl Iterator<Item = u32>) {
    let mut have: BTreeSet<u32> = out.iter().copied().collect();
    for v in more {
        if have.insert(v) {
            out.push(v);
        }
    }
}

fn centroid(mesh: &Mesh, f: usize) -> DVec3 {
    let v = &mesh.faces[f].vertices;
    v.iter().map(|&i| mesh.vertices[i as usize]).sum::<DVec3>() / v.len().max(1) as f64
}

// ------------------------------------------------ Sharp, Coplanar, Colinear

/// The interior edges (two polygons) with their polygons, in edge order.
fn interior_edges(mesh: &Mesh) -> Vec<((u32, u32), usize, usize)> {
    mesh.edge_faces()
        .iter()
        .filter(|(_, f)| f.len() == 2)
        .map(|(&e, f)| (e, f[0], f[1]))
        .collect()
}

/// SL-10: the angle between the two face normals, inclusive bounds; border
/// edges never. Replaces the edge selection (SL-12, provisional).
pub fn sharp_edges(mesh: &Mesh, min: f64, max: f64) -> Result<Vec<(u32, u32)>> {
    if !(min.is_finite() && max.is_finite()) || min < 0. || max > 180. || min > max {
        return Err(
            "Sharp: Minimum and Maximum Angle must be from 0° to 180°, Minimum no more than Maximum"
                .into(),
        );
    }
    Ok(interior_edges(mesh)
        .into_iter()
        .filter(|&(_, f, g)| {
            let (a, b) = (mesh.normal(f), mesh.normal(g));
            if a == DVec3::ZERO || b == DVec3::ZERO {
                return false;
            }
            let angle = a.dot(b).clamp(-1., 1.).acos().to_degrees();
            // A little slack so 90.0000001° from rounding still counts.
            angle >= min - 1e-7 && angle <= max + 1e-7
        })
        .map(|(e, _, _)| e)
        .collect())
}

/// SL-20/21 (SL-Q1 build guess): the pair's points' largest distance from
/// the plane through the shared edge with the pair's average normal, over
/// the pair's size (its longest point-to-point span), at most Flatness.
pub fn coplanar_edges(
    mesh: &Mesh,
    flatness: f64,
    only: Option<&BTreeSet<u32>>,
) -> Result<Vec<(u32, u32)>> {
    if !flatness.is_finite() || flatness < 0. {
        return Err("Coplanar Edges: Flatness must be zero or more".into());
    }
    Ok(interior_edges(mesh)
        .into_iter()
        .filter(|&(_, f, g)| {
            only.is_none_or(|s| s.contains(&(f as u32)) || s.contains(&(g as u32)))
        })
        .filter(|&((a, _), f, g)| {
            let n = (mesh.normal(f) + mesh.normal(g)).normalize_or_zero();
            if n == DVec3::ZERO {
                return false;
            }
            let origin = mesh.vertices[a as usize];
            let points: Vec<DVec3> = mesh.faces[f]
                .vertices
                .iter()
                .chain(&mesh.faces[g].vertices)
                .map(|&v| mesh.vertices[v as usize])
                .collect();
            let size = points
                .iter()
                .flat_map(|p| points.iter().map(move |q| p.distance(*q)))
                .fold(0., f64::max);
            let off = points
                .iter()
                .map(|p| (*p - origin).dot(n).abs())
                .fold(0., f64::max);
            size > 0. && off / size <= flatness + 1e-12
        })
        .map(|(e, _, _)| e)
        .collect())
}

/// SL-30: points whose distance from the line through their two outline
/// neighbours is at most Distance, in every polygon that uses them, and
/// that lie between those neighbours (so corners never qualify).
pub fn colinear_vertices(mesh: &Mesh, distance: f64) -> Result<Vec<u32>> {
    if !distance.is_finite() || distance < 0. {
        return Err("Colinear Vertices: Distance must be zero or more".into());
    }
    let mut ok: BTreeMap<u32, bool> = BTreeMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let v = &face.vertices;
        let n = v.len();
        if n < 3 || mesh.is_polyline(fi) {
            continue;
        }
        for i in 0..n {
            let (a, p, b) = (
                mesh.vertices[v[(i + n - 1) % n] as usize],
                mesh.vertices[v[i] as usize],
                mesh.vertices[v[(i + 1) % n] as usize],
            );
            let d = b - a;
            let len2 = d.length_squared();
            let t = if len2 > 0. {
                (p - a).dot(d) / len2
            } else {
                -1.
            };
            let on = len2 > 0. && (0. ..=1.).contains(&t) && (a + d * t).distance(p) <= distance;
            let entry = ok.entry(v[i]).or_insert(true);
            *entry &= on;
        }
    }
    Ok(ok
        .into_iter()
        .filter(|&(_, on)| on)
        .map(|(v, _)| v)
        .collect())
}

// ------------------------------------------------------- Coplanar polygons

/// SL-50..52: polygons whose normal is within Angle of a start polygon's;
/// Connectivity on: reached from the start through such polygons; off: any,
/// with its plane within Range of a start polygon's plane.
pub fn coplanar_polygons(
    mesh: &Mesh,
    start: &[u32],
    o: &CoplanarPolygonOptions,
) -> Result<Vec<u32>> {
    if !(o.angle.is_finite() && (0. ..=180.).contains(&o.angle)) {
        return Err("Coplanar Polygons: Angle must be from 0° to 180°".into());
    }
    if !o.range.is_finite() || o.range < 0. {
        return Err("Coplanar Polygons: Height Range must be zero or more".into());
    }
    let n = mesh.faces.len();
    let start: Vec<usize> = start
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| f < n)
        .collect();
    if start.is_empty() {
        return Err("Coplanar Polygons: select a polygon to start from".into());
    }
    let cos = o.angle.to_radians().cos();
    let planes: Vec<(DVec3, DVec3)> = start
        .iter()
        .map(|&f| (mesh.normal(f), centroid(mesh, f)))
        .collect();
    let facing = |f: usize| {
        let nf = mesh.normal(f);
        nf != DVec3::ZERO && planes.iter().any(|(ns, _)| ns.dot(nf) >= cos - 1e-12)
    };
    let mut chosen: BTreeSet<usize> = start.iter().copied().collect();
    if o.connectivity {
        let edge_faces = mesh.edge_faces();
        let mut by_face: Vec<Vec<usize>> = vec![Vec::new(); n];
        for faces in edge_faces.values() {
            for &f in faces {
                by_face[f].extend(faces.iter().copied().filter(|&g| g != f));
            }
        }
        let mut queue: VecDeque<usize> = start.iter().copied().collect();
        while let Some(f) = queue.pop_front() {
            for &g in &by_face[f] {
                if !chosen.contains(&g) && facing(g) {
                    chosen.insert(g);
                    queue.push_back(g);
                }
            }
        }
    } else {
        for f in 0..n {
            if mesh.is_polyline(f) || !facing(f) {
                continue;
            }
            let c = centroid(mesh, f);
            if planes
                .iter()
                .any(|(ns, p)| (c - *p).dot(*ns).abs() <= o.range + 1e-12)
            {
                chosen.insert(f);
            }
        }
    }
    Ok(chosen.into_iter().map(|f| f as u32).collect())
}

// ---------------------------------------------------------- Conditional loop

/// SL-60..62: each selected edge grown both ways into an edge loop (across
/// four-edge points) until a stop rule says stop.
pub fn conditional_loop(
    mesh: &Mesh,
    seeds: &[(u32, u32)],
    already: &BTreeSet<(u32, u32)>,
    o: &ConditionalLoopOptions,
) -> Result<Vec<(u32, u32)>> {
    if seeds.is_empty() {
        return Err("Conditional Loop: select an edge to grow from".into());
    }
    if !o.length_tolerance.is_finite() || o.length_tolerance < 0. {
        return Err("Conditional Loop: Length Tolerance must be zero or more".into());
    }
    let edge_faces = mesh.edge_faces();
    let mut around: Vec<Vec<u32>> = vec![Vec::new(); mesh.vertices.len()];
    for (a, b) in mesh.edges() {
        around[a as usize].push(b);
        around[b as usize].push(a);
    }
    let faces_of = |e: (u32, u32)| edge_faces.get(&e).cloned().unwrap_or_default();
    let mut out: Vec<(u32, u32)> = Vec::new();
    let mut have: BTreeSet<(u32, u32)> = BTreeSet::new();
    fn push(e: (u32, u32), out: &mut Vec<(u32, u32)>, have: &mut BTreeSet<(u32, u32)>) {
        if have.insert(e) {
            out.push(e);
        }
    }
    for &seed in seeds {
        let seed = key(seed.0, seed.1);
        push(seed, &mut out, &mut have);
        let directions: &[(u32, u32)] = if o.both_directions {
            &[(seed.0, seed.1), (seed.1, seed.0)]
        } else {
            &[(seed.0, seed.1)]
        };
        let mut count = 1u32;
        for &(from, to) in directions {
            let mut passed: BTreeSet<usize> = faces_of(seed).into_iter().collect();
            let mut crossed_selected = 0u32;
            let (mut prev, mut at) = (from, to);
            loop {
                if o.max_loop > 0 && count >= o.max_loop {
                    break;
                }
                // The edge straight across `at`: its point has four edges,
                // and the next edge shares no polygon with this one.
                if around[at as usize].len() != 4 {
                    break;
                }
                let here = faces_of(key(prev, at));
                let Some(&next) = around[at as usize].iter().find(|&&n| {
                    n != prev && {
                        let f = faces_of(key(at, n));
                        !f.iter().any(|x| here.contains(x))
                    }
                }) else {
                    break;
                };
                let e = key(at, next);
                if have.contains(&e) {
                    break;
                }
                let next_faces = faces_of(e);
                if o.stop_shared_polygon && next_faces.iter().any(|f| passed.contains(f)) {
                    break;
                }
                let (p0, p1, p2) = (
                    mesh.vertices[prev as usize],
                    mesh.vertices[at as usize],
                    mesh.vertices[next as usize],
                );
                if o.stop_sharp {
                    let turn = (p1 - p0)
                        .normalize_or_zero()
                        .dot((p2 - p1).normalize_or_zero())
                        .clamp(-1., 1.)
                        .acos()
                        .to_degrees();
                    if turn > o.sharp_angle {
                        break;
                    }
                }
                if o.stop_length {
                    let (l0, l1) = (p0.distance(p1), p1.distance(p2));
                    let ratio = if l0.min(l1) > 0. {
                        l0.max(l1) / l0.min(l1)
                    } else {
                        f64::INFINITY
                    };
                    if ratio - 1. > o.length_tolerance {
                        break;
                    }
                }
                if o.stop_selection && already.contains(&e) {
                    crossed_selected += 1;
                    if crossed_selected >= o.selection_count {
                        break;
                    }
                }
                push(e, &mut out, &mut have);
                count += 1;
                passed.extend(next_faces);
                prev = at;
                at = next;
            }
        }
    }
    Ok(out)
}

// ------------------------------------------------------------- The command

impl crate::Editor {
    /// Run `request` on the active mesh: one undo step that changes only
    /// the selection. Symmetry adds the mirrored elements.
    pub(crate) fn select_tool(&mut self, request: SelectRequest) -> Result<()> {
        self.require_idle()?;
        let label = request.label();
        let picked = picked_for(&self.scene, &request)?;
        let before = self.scene.clone();
        let s = &mut self.scene.selection;
        if let Some(mode) = picked.mode {
            s.mode = mode;
        }
        if let Some(v) = picked.vertices {
            s.vertices = v;
        }
        if let Some(e) = picked.edges {
            s.edges = e;
        }
        if let Some(f) = picked.faces {
            s.faces = f;
        }
        let symmetry = self.active_symmetry();
        if symmetry.axis.is_some() {
            self.scene.select_symmetrical(symmetry)?;
        }
        if self.scene != before {
            self.record_selection(label.into(), before);
        }
        Ok(())
    }
}

/// What `request` selects in `scene`.
pub fn picked_for(scene: &Scene, request: &SelectRequest) -> Result<Picked> {
    let item = scene
        .active()
        .filter(|i| i.is_mesh())
        .ok_or("Select a mesh first")?;
    let mesh = &item.mesh;
    let s = &scene.selection;
    if s.mode == Mode::Item {
        return Err(format!(
            "{} works on vertices, edges or polygons; pick a component mode first",
            request.label()
        ));
    }
    Ok(match request {
        SelectRequest::ShortestPath => shortest_path(mesh, s)?,
        SelectRequest::PathToLast => {
            let mut probe = s.clone();
            fn last_two<T: Copy>(v: &mut Vec<T>) {
                let n = v.len();
                if n >= 2 {
                    *v = vec![v[n - 2], v[n - 1]];
                }
            }
            last_two(&mut probe.vertices);
            last_two(&mut probe.edges);
            last_two(&mut probe.faces);
            let path = shortest_path(mesh, &probe)?;
            let mut out = Picked::default();
            if let Some(v) = path.vertices {
                let mut all = s.vertices.clone();
                add_missing(&mut all, v.into_iter());
                out.vertices = Some(all);
            }
            if let Some(e) = path.edges {
                let mut all = s.edges.clone();
                let have: BTreeSet<(u32, u32)> = all.iter().map(|&(a, b)| key(a, b)).collect();
                all.extend(e.into_iter().filter(|x| !have.contains(&key(x.0, x.1))));
                out.edges = Some(all);
            }
            if let Some(f) = path.faces {
                let mut all = s.faces.clone();
                add_missing(&mut all, f.into_iter());
                out.faces = Some(all);
            }
            out
        }
        SelectRequest::Sharp { min, max } => Picked {
            edges: Some(sharp_edges(mesh, *min, *max)?),
            mode: Some(Mode::Edge),
            ..Default::default()
        },
        SelectRequest::CoplanarEdges { flatness } => {
            // SL-22: from the selected polygons, or the whole mesh.
            let only: Option<BTreeSet<u32>> = (s.mode == Mode::Polygon && !s.faces.is_empty())
                .then(|| s.faces.iter().copied().collect());
            Picked {
                edges: Some(coplanar_edges(mesh, *flatness, only.as_ref())?),
                mode: Some(Mode::Edge),
                ..Default::default()
            }
        }
        SelectRequest::ColinearVertices { distance } => Picked {
            vertices: Some(colinear_vertices(mesh, *distance)?),
            mode: Some(Mode::Vertex),
            ..Default::default()
        },
        SelectRequest::Diagonal(o) => {
            o.validate()?;
            let mut edges = crate::detriangulate::detriangulate_removed_edges(mesh, &[], o)?;
            edges.sort_unstable();
            Picked {
                edges: Some(edges),
                mode: Some(Mode::Edge),
                ..Default::default()
            }
        }
        SelectRequest::CoplanarPolygons { start, options } => Picked {
            faces: Some(coplanar_polygons(mesh, start, options)?),
            mode: Some(Mode::Polygon),
            ..Default::default()
        },
        SelectRequest::ConditionalLoop(o) => {
            if s.mode != Mode::Edge {
                return Err("Conditional Loop works in Edge mode: select an edge".into());
            }
            // Loop From: Last (SL-60): grow from the last selected edge.
            let seed = s.edges.last().copied().into_iter().collect::<Vec<_>>();
            let already: BTreeSet<(u32, u32)> = s.edges.iter().map(|&(a, b)| key(a, b)).collect();
            let grown = conditional_loop(mesh, &seed, &already, o)?;
            let mut edges = s.edges.clone();
            let have: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| key(a, b)).collect();
            edges.extend(grown.into_iter().filter(|e| !have.contains(e)));
            Picked {
                edges: Some(edges),
                ..Default::default()
            }
        }
    })
}
