//! MP-T11 Polygon Reduction for general meshes.
//!
//! Polygon Reduce iteratively removes edges while keeping
//! as much of the original contouring as possible, using quadric error
//! matrices; Boundary Weight and Material Border Weight protect boundaries and
//! material borders. This module implements that with the public
//! Garland–Heckbert quadric error metric over half-edge collapses: a point
//! collapses onto one existing neighbour, so surviving points keep their
//! exact positions and the explicit [`ReductionCorrespondence`] stays valid.
//!
//! Domain and rules:
//! * The selected surface polygons are triangulated first (ear clipping for
//!   planar polygons, a fan otherwise); the reduced region is triangles, as
//!   with the reference tool's output.
//! * Points used by an unselected polygon or a nonmanifold edge never move,
//!   so the selection's border and the rest of the mesh are unchanged.
//! * A mesh-boundary point collapses only along its boundary. Boundary and
//!   material-border edges carry an extra constraint quadric, scaled by
//!   Boundary Weight and Material Border Weight.
//! * A collapse must pass the link condition (stays manifold, no duplicate
//!   triangles) and must not flip or flatten any remaining triangle.
//! * Collapses run cheapest first until the polygon count reaches the
//!   requested Number or no valid collapse remains (then `achieved` says so).
//! * Corner maps (UVs, corner weights/colours/normals, typed corner records)
//!   on the selected polygons are refused for now rather than guessed; point
//!   records of removed points leave with them.
use crate::polygon_triple::triangulate_polygon;
use crate::{Face, Mesh, MeshCrease, ReductionCorrespondence};
use crate::{ReductionOutcome, Result};
use glam::DVec3;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// Extra weight of boundary and material-border constraint planes.
const BORDER_WEIGHT: f64 = 1000.;

/// Symmetric 4×4 quadric, upper triangle row by row.
#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);
impl Quadric {
    fn plane(n: DVec3, d: f64, w: f64) -> Self {
        let (a, b, c) = (n.x, n.y, n.z);
        Self([
            w * a * a,
            w * a * b,
            w * a * c,
            w * a * d,
            w * b * b,
            w * b * c,
            w * b * d,
            w * c * c,
            w * c * d,
            w * d * d,
        ])
    }
    fn add(&mut self, o: &Self) {
        for (x, y) in self.0.iter_mut().zip(o.0) {
            *x += y;
        }
    }
    fn error(&self, p: DVec3) -> f64 {
        let q = &self.0;
        let (x, y, z) = (p.x, p.y, p.z);
        q[0] * x * x
            + 2. * q[1] * x * y
            + 2. * q[2] * x * z
            + 2. * q[3] * x
            + q[4] * y * y
            + 2. * q[5] * y * z
            + 2. * q[6] * y
            + q[7] * z * z
            + 2. * q[8] * z
            + q[9]
    }
}

struct Candidate {
    cost: f64,
    from: u32,
    to: u32,
}
impl PartialEq for Candidate {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Candidate {
    // Min-heap on cost, then on point indices for determinism.
    fn cmp(&self, o: &Self) -> Ordering {
        o.cost
            .total_cmp(&self.cost)
            .then(o.from.cmp(&self.from))
            .then(o.to.cmp(&self.to))
    }
}

struct Tri {
    v: [u32; 3],
    source: usize,
    alive: bool,
    /// Still exactly the source face's own corner cycle.
    original: bool,
}

fn normal(p: &[DVec3], t: [u32; 3]) -> DVec3 {
    let [a, b, c] = t.map(|v| p[v as usize]);
    (b - a).cross(c - a)
}

/// Whether reducing `selected` needs the general path (anything the exact
/// planar-disk profile does not cover) and is eligible for it.
pub(crate) fn general_reduction_count(mesh: &Mesh, selected: &[u32]) -> Result<u32> {
    mesh.validate()?;
    if selected.is_empty() {
        return Err("Select polygons for Polygon Reduction; an empty selection is not all".into());
    }
    let set: BTreeSet<u32> = selected.iter().copied().collect();
    if set.len() != selected.len() {
        return Err("Duplicate polygon selection".into());
    }
    if !mesh.attributes.extrude_edge_uses.is_empty() {
        return Err(
            "Polygon Reduction cannot yet reduce a mesh with inconsistent winding; use Align first"
                .into(),
        );
    }
    let at = &mesh.attributes;
    for &f in &set {
        let fi = f as usize;
        if fi >= mesh.faces.len() {
            return Err("Selected polygon no longer exists".into());
        }
        if mesh.is_polyline(fi) || at.degenerate_faces.contains(&f) {
            return Err("Polygon Reduction supports surface polygons only".into());
        }
        let corner_data = mesh.faces[fi].uv.is_some()
            || mesh
                .uv_maps
                .values()
                .any(|m| m[fi].iter().any(Option::is_some))
            || at
                .uv_records
                .values()
                .any(|m| m[fi].iter().any(Option::is_some))
            || at
                .corner_weights
                .values()
                .any(|m| m[fi].iter().any(Option::is_some))
            || at
                .corner_colors
                .values()
                .any(|m| m[fi].iter().any(Option::is_some))
            || at
                .corner_normals
                .values()
                .any(|m| m[fi].iter().any(Option::is_some))
            || at
                .typed_maps
                .values()
                .any(|m| !m.corners.is_empty() && m.corners[fi].iter().any(Option::is_some));
        if corner_data {
            return Err(
                "Polygon Reduction cannot yet remap UVs or corner maps on the selected polygons; nothing changed"
                    .into(),
            );
        }
    }
    u32::try_from(mesh.faces.len()).map_err(|_| "Mesh face budget exceeded".into())
}

/// Reduce the selected polygons of `mesh` toward `requested` total polygons.
/// `boundary_weight` and `material_weight` scale the open-border and material-border
/// constraints (1 = 100 %).
pub(crate) fn reduce_general(
    mesh: &Mesh,
    selected: &[u32],
    requested: u32,
    boundary_weight: f64,
    material_weight: f64,
) -> Result<ReductionOutcome> {
    let source_count = general_reduction_count(mesh, selected)?;
    let set: BTreeSet<usize> = selected.iter().map(|&f| f as usize).collect();
    let p = &mesh.vertices;
    let n = p.len();

    // Working triangles of the selection, in source face order.
    let mut tris: Vec<Tri> = Vec::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        if !set.contains(&fi) {
            continue;
        }
        let vs = &face.vertices;
        if vs.len() == 3 {
            tris.push(Tri {
                v: [vs[0], vs[1], vs[2]],
                source: fi,
                alive: true,
                original: true,
            });
            continue;
        }
        let points: Vec<DVec3> = vs.iter().map(|&v| p[v as usize]).collect();
        let corners = triangulate_polygon(&points, |_, _| false)
            .unwrap_or_else(|_| (1..vs.len() - 1).map(|i| [0, i, i + 1]).collect());
        for [a, b, c] in corners {
            tris.push(Tri {
                v: [vs[a], vs[b], vs[c]],
                source: fi,
                alive: true,
                original: false,
            });
        }
    }
    let unselected = mesh.faces.len() - set.len();
    let mut count = unselected + tris.len();

    // Fixed points: used by unselected faces or nonmanifold working edges.
    let mut fixed = vec![false; n];
    for (fi, face) in mesh.faces.iter().enumerate() {
        if !set.contains(&fi) {
            for &v in &face.vertices {
                fixed[v as usize] = true;
            }
        }
    }
    let mut edge_tris: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (ti, t) in tris.iter().enumerate() {
        for i in 0..3 {
            let (a, b) = (t.v[i], t.v[(i + 1) % 3]);
            edge_tris.entry((a.min(b), a.max(b))).or_default().push(ti);
        }
    }
    let mut boundary = vec![false; n];
    for (&(a, b), ts) in &edge_tris {
        if ts.len() > 2 {
            fixed[a as usize] = true;
            fixed[b as usize] = true;
        } else if ts.len() == 1 {
            boundary[a as usize] = true;
            boundary[b as usize] = true;
        }
    }

    // Quadrics: face planes (area weighted) plus border constraint planes.
    let mut quadric = vec![Quadric::default(); n];
    for t in &tris {
        let nn = normal(p, t.v);
        let area = nn.length();
        if area == 0. {
            continue;
        }
        let unit = nn / area;
        let q = Quadric::plane(unit, -unit.dot(p[t.v[0] as usize]), area * 0.5);
        for &v in &t.v {
            quadric[v as usize].add(&q);
        }
    }
    for (&(a, b), ts) in &edge_tris {
        let material_border = ts.len() == 2 && {
            let (x, y) = (
                &mesh.faces[tris[ts[0]].source],
                &mesh.faces[tris[ts[1]].source],
            );
            x.material != y.material || x.material_label != y.material_label
        };
        let weight = if ts.len() == 1 {
            boundary_weight
        } else if material_border {
            material_weight
        } else {
            continue;
        };
        if weight == 0. {
            continue;
        }
        let e = p[b as usize] - p[a as usize];
        let face_n = normal(p, tris[ts[0]].v).normalize_or_zero();
        let side = e.cross(face_n).normalize_or_zero();
        if side == DVec3::ZERO {
            continue;
        }
        let q = Quadric::plane(
            side,
            -side.dot(p[a as usize]),
            BORDER_WEIGHT * weight * e.length_squared(),
        );
        quadric[a as usize].add(&q);
        quadric[b as usize].add(&q);
    }

    // Incident working triangles per point.
    let mut incident: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for (ti, t) in tris.iter().enumerate() {
        for &v in &t.v {
            incident[v as usize].insert(ti);
        }
    }
    let neighbours = |incident: &[BTreeSet<usize>], tris: &[Tri], v: u32| -> BTreeSet<u32> {
        incident[v as usize]
            .iter()
            .flat_map(|&ti| tris[ti].v)
            .filter(|&w| w != v)
            .collect()
    };
    let mut removed = vec![false; n];
    let mut heap = BinaryHeap::new();
    let push = |heap: &mut BinaryHeap<Candidate>, quadric: &[Quadric], u: u32, v: u32| {
        let mut q = quadric[u as usize];
        q.add(&quadric[v as usize]);
        heap.push(Candidate {
            cost: q.error(p[v as usize]),
            from: u,
            to: v,
        });
    };
    for &(a, b) in edge_tris.keys() {
        push(&mut heap, &quadric, a, b);
        push(&mut heap, &quadric, b, a);
    }

    let target = requested as usize;
    while count > target {
        let Some(Candidate { from: u, to: v, .. }) = heap.pop() else {
            break;
        };
        if removed[u as usize] || removed[v as usize] || fixed[u as usize] {
            continue;
        }
        let shared: Vec<usize> = incident[u as usize]
            .intersection(&incident[v as usize])
            .copied()
            .collect();
        if shared.is_empty() || shared.len() > 2 {
            continue;
        }
        // Boundary points slide only along the boundary.
        if boundary[u as usize] && shared.len() != 1 {
            continue;
        }
        if !boundary[u as usize] && shared.len() == 1 {
            continue;
        }
        // Link condition.
        let opposite: BTreeSet<u32> = shared
            .iter()
            .flat_map(|&ti| tris[ti].v)
            .filter(|&w| w != u && w != v)
            .collect();
        let common: BTreeSet<u32> = neighbours(&incident, &tris, u)
            .intersection(&neighbours(&incident, &tris, v))
            .copied()
            .collect();
        if common != opposite {
            continue;
        }
        // No flipped or flattened triangle.
        let moved: Vec<usize> = incident[u as usize]
            .iter()
            .copied()
            .filter(|ti| !shared.contains(ti))
            .collect();
        let ok = moved.iter().all(|&ti| {
            let before = normal(p, tris[ti].v);
            let after = normal(p, tris[ti].v.map(|w| if w == u { v } else { w }));
            after.length() > 1e-12 * before.length().max(f64::MIN_POSITIVE)
                && after.dot(before) > 0.
        });
        if !ok {
            continue;
        }
        if count - shared.len() < target && count > target {
            // Never overshoot: an interior collapse removes two triangles.
            continue;
        }
        // Apply u -> v.
        for &ti in &shared {
            tris[ti].alive = false;
            for &w in &tris[ti].v {
                incident[w as usize].remove(&ti);
            }
        }
        for &ti in &moved {
            for w in &mut tris[ti].v {
                if *w == u {
                    *w = v;
                }
            }
            tris[ti].original = false;
            incident[v as usize].insert(ti);
        }
        incident[u as usize].clear();
        removed[u as usize] = true;
        count -= shared.len();
        let qu = quadric[u as usize];
        quadric[v as usize].add(&qu);
        for w in neighbours(&incident, &tris, v) {
            push(&mut heap, &quadric, w, v);
            push(&mut heap, &quadric, v, w);
        }
    }

    if count >= source_count as usize {
        // Triangulation alone would not lower the count: leave the mesh.
        return Ok(ReductionOutcome {
            mesh: mesh.clone(),
            correspondence: ReductionCorrespondence {
                vertex_sources: (0..n as u32).collect(),
                face_sources: (0..mesh.faces.len() as u32).map(Some).collect(),
                retired_vertices: Vec::new(),
                retired_faces: Vec::new(),
            },
            requested,
            achieved: source_count,
        });
    }
    build(mesh, &set, &tris, &removed, requested, count)
}

/// Assemble the output mesh and its correspondence.
fn build(
    mesh: &Mesh,
    set: &BTreeSet<usize>,
    tris: &[Tri],
    removed: &[bool],
    requested: u32,
    count: usize,
) -> Result<ReductionOutcome> {
    let vertex_sources: Vec<u32> = (0..mesh.vertices.len() as u32)
        .filter(|&v| !removed[v as usize])
        .collect();
    let retired_vertices: Vec<u32> = (0..mesh.vertices.len() as u32)
        .filter(|&v| removed[v as usize])
        .collect();
    let mut remap = vec![None; mesh.vertices.len()];
    for (i, &v) in vertex_sources.iter().enumerate() {
        remap[v as usize] = Some(i as u32);
    }
    let map =
        |v: u32| remap[v as usize].ok_or_else(|| "Reduction left a removed point".to_string());

    // (output face, source face, unchanged?)
    let mut faces: Vec<Face> = Vec::new();
    let mut owners: Vec<usize> = Vec::new();
    let mut face_sources: Vec<Option<u32>> = Vec::new();
    let mut by_source: BTreeMap<usize, Vec<&Tri>> = BTreeMap::new();
    for t in tris.iter().filter(|t| t.alive) {
        by_source.entry(t.source).or_default().push(t);
    }
    for (fi, face) in mesh.faces.iter().enumerate() {
        if !set.contains(&fi) {
            let mut f = face.clone();
            f.vertices = face
                .vertices
                .iter()
                .map(|&v| map(v))
                .collect::<Result<_>>()?;
            faces.push(f);
            owners.push(fi);
            face_sources.push(Some(fi as u32));
            continue;
        }
        for t in by_source.get(&fi).into_iter().flatten() {
            let mut f = face.clone();
            f.vertices = t.v.iter().map(|&v| map(v)).collect::<Result<_>>()?;
            f.uv = None;
            faces.push(f);
            owners.push(fi);
            face_sources.push(t.original.then_some(fi as u32));
        }
    }
    let retained: BTreeSet<u32> = face_sources.iter().flatten().copied().collect();
    let retired_faces: Vec<u32> = (0..mesh.faces.len() as u32)
        .filter(|f| !retained.contains(f))
        .collect();

    let mut out = mesh.clone();
    out.vertices = vertex_sources
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    let corner_rows = |rows: &Vec<Vec<Option<glam::DVec2>>>| -> Vec<Vec<Option<glam::DVec2>>> {
        faces
            .iter()
            .zip(&face_sources)
            .map(|(f, s)| match s {
                Some(src) if !set.contains(&(*src as usize)) => rows[*src as usize].clone(),
                _ => vec![None; f.vertices.len()],
            })
            .collect()
    };
    out.uv_maps = mesh
        .uv_maps
        .iter()
        .map(|(k, rows)| (k.clone(), corner_rows(rows)))
        .collect();
    fn rows_for<T: Clone>(
        rows: &[Vec<Option<T>>],
        faces: &[Face],
        owners: &[usize],
        set: &BTreeSet<usize>,
    ) -> Vec<Vec<Option<T>>> {
        faces
            .iter()
            .zip(owners)
            .map(|(f, &o)| {
                if set.contains(&o) {
                    vec![None; f.vertices.len()]
                } else {
                    rows[o].clone()
                }
            })
            .collect()
    }
    let mut attrs = mesh.attributes.clone();
    let at = &mut attrs;
    let src = &mesh.attributes;
    for (k, rows) in &src.uv_records {
        at.uv_records
            .insert(k.clone(), rows_for(rows, &faces, &owners, set));
    }
    for (k, rows) in &src.corner_weights {
        at.corner_weights
            .insert(k.clone(), rows_for(rows, &faces, &owners, set));
    }
    for (k, rows) in &src.corner_colors {
        at.corner_colors
            .insert(k.clone(), rows_for(rows, &faces, &owners, set));
    }
    for (k, rows) in &src.corner_normals {
        at.corner_normals
            .insert(k.clone(), rows_for(rows, &faces, &owners, set));
    }
    for (k, map) in &src.typed_maps {
        let entry = at.typed_maps.get_mut(k).ok_or("Missing typed map")?;
        if !map.corners.is_empty() {
            entry.corners = rows_for(&map.corners, &faces, &owners, set);
        }
        entry.points = map
            .points
            .iter()
            .filter_map(|(&v, x)| remap[v as usize].map(|w| (w, x.clone())))
            .collect();
    }
    fn remap_points<T: Clone>(m: &BTreeMap<u32, T>, remap: &[Option<u32>]) -> BTreeMap<u32, T> {
        m.iter()
            .filter_map(|(&v, x)| remap[v as usize].map(|w| (w, x.clone())))
            .collect()
    }
    for (k, m) in &src.point_uvs {
        at.point_uvs.insert(k.clone(), remap_points(m, &remap));
    }
    for (k, m) in &src.point_weights {
        at.point_weights.insert(k.clone(), remap_points(m, &remap));
    }
    for (k, m) in &src.point_colors {
        at.point_colors.insert(k.clone(), remap_points(m, &remap));
    }
    for (k, m) in &src.relative_morphs {
        at.relative_morphs
            .insert(k.clone(), remap_points(m, &remap));
    }
    for (k, s) in &src.vertex_sets {
        at.vertex_sets.insert(
            k.clone(),
            s.iter().filter_map(|&v| remap[v as usize]).collect(),
        );
    }
    at.edge_sets = crate::selection_sets::remap_edge_sets(&src.edge_sets, |v| remap[v as usize]);
    for (k, s) in &src.polygon_sets {
        at.polygon_sets.insert(
            k.clone(),
            owners
                .iter()
                .enumerate()
                .filter(|(_, o)| s.contains(&(**o as u32)))
                .map(|(i, _)| i as u32)
                .collect(),
        );
    }
    if !src.smoothing_labels.is_empty() {
        at.smoothing_labels = owners
            .iter()
            .map(|&o| src.smoothing_labels[o].clone())
            .collect();
    }
    if !src.representations.is_empty() {
        at.representations = owners.iter().map(|&o| src.representations[o]).collect();
    }
    at.degenerate_faces = owners
        .iter()
        .enumerate()
        .filter(|(_, o)| src.degenerate_faces.contains(&(**o as u32)) && !set.contains(o))
        .map(|(i, _)| i as u32)
        .collect();
    out.faces = faces.into();
    let edges: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    at.creases = src
        .creases
        .iter()
        .filter_map(|c| {
            let (a, b) = (remap[c.a as usize]?, remap[c.b as usize]?);
            edges.contains(&(a.min(b), a.max(b))).then_some(MeshCrease {
                a,
                b,
                weight: c.weight,
            })
        })
        .collect();
    out.attributes = attrs;
    out.validate().map_err(|e| {
        format!("Polygon Reduction result failed validation ({e}); nothing changed")
    })?;
    let correspondence = ReductionCorrespondence {
        vertex_sources,
        face_sources,
        retired_vertices,
        retired_faces,
    };
    correspondence.validate(mesh, &out)?;
    Ok(ReductionOutcome {
        mesh: out,
        correspondence,
        requested,
        achieved: count as u32,
    })
}
