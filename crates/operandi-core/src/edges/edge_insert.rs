//! ME-T09 Add Point and ME-T08 Add Loop.
//!
//! Add Point adds a vertex on an edge, splitting it in
//! two, at Position (percent of the edge, 50 % = middle). Add Loop adds a
//! continuous edge through the band of polygons that crosses the target edge,
//! cutting each polygon at the same Position; the loop ends at a mesh
//! boundary or when it meets a triangle (which takes the new point but is not
//! cut), and closes when it comes back to the start. Polygons with more than
//! four sides also end the loop here (a centre cut through
//! n-gons is not built).
//!
//! Position is measured from the edge's first point `a` toward `b`. Each new
//! point is inserted into every polygon that uses the split edge, so no
//! T-junction is left. Corner data of a new corner (UV maps, legacy UVs,
//! corner weights/colours) is interpolated linearly when both end corners
//! have values and left unassigned otherwise; point maps get no record for
//! the new point; a crease on a split edge carries to both halves. Polygons
//! keep their tags; a cut polygon's two halves both keep them.
use crate::topology::edge_key;
use crate::{Mesh, MeshCrease, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// One oriented edge to split: new point at `a + (b - a) * t`, or at `at`
/// when given (Preserve Curvature; corner data still interpolates by `t`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cut {
    a: u32,
    b: u32,
    t: f64,
    at: Option<DVec3>,
}

/// Placement options shared by Add Loop and Add Point (tool-audit Tier 1,
/// provisional readings logged as TA1-Q01..Q03). Distances are in the
/// mesh's own units; the editor converts from world metres.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeInsertOptions {
    /// Add Loop only: a second loop mirrored about the middle (at `1 - t`,
    /// or Distance 2 from the far end with Absolute Distance).
    pub both_sides: bool,
    /// Place by distance from an edge end instead of Position.
    pub absolute: bool,
    /// Absolute Distance measures from the far (`b`) end.
    pub from_end: bool,
    pub distance1: f64,
    pub distance2: f64,
    /// New points follow a curve through the ring instead of the straight
    /// edge (see [`curve_point`]).
    pub preserve_curvature: bool,
    /// Curve tangent scale, 1.0 = 100 %.
    pub tension: f64,
}
impl Default for EdgeInsertOptions {
    fn default() -> Self {
        Self {
            both_sides: false,
            absolute: false,
            from_end: false,
            distance1: 0.,
            distance2: 0.,
            preserve_curvature: false,
            tension: 1.,
        }
    }
}

/// The point after `through` that continues the edge `from`–`through` in
/// a straight run of the mesh: the one neighbour of `through` that shares
/// no polygon with that edge. `None` at a border, a pole or an irregular
/// point (more or fewer than one candidate).
pub(crate) fn continuation(mesh: &Mesh, from: u32, through: u32) -> Option<u32> {
    let point_faces = mesh.point_faces();
    let mut candidates: Vec<u32> = Vec::new();
    let mut excluded: Vec<u32> = vec![from];
    for &f in point_faces.of(through) {
        let vs = &mesh.faces.get(f as usize)?.vertices;
        let n = vs.len();
        if n < 3 {
            continue;
        }
        let i = vs.iter().position(|&v| v == through)?;
        let (prev, next) = (vs[(i + n - 1) % n], vs[(i + 1) % n]);
        candidates.extend([prev, next]);
        if vs.contains(&from) {
            excluded.extend([prev, next]);
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    candidates.retain(|v| !excluded.contains(v));
    (candidates.len() == 1).then(|| candidates[0])
}

/// Preserve Curvature: the point at `t` (0..1) on a cubic Hermite curve
/// from point `p` to `q`. Each end tangent points along the neighbouring
/// edge that continues the run (see [`continuation`]) with the edge's own
/// length, scaled by `tension`; an end with no such neighbour uses the
/// straight edge. A straight, evenly spaced run therefore gives exactly the
/// straight cut. Outside 0..1 the straight line is used.
pub(crate) fn curve_point(mesh: &Mesh, p: u32, q: u32, t: f64, tension: f64) -> DVec3 {
    let (p0, p1) = (mesh.vertices[p as usize], mesh.vertices[q as usize]);
    if !(0. ..=1.).contains(&t) {
        return p0.lerp(p1, t);
    }
    let chord = p1 - p0;
    let length = chord.length();
    let tangent = |towards: Option<u32>, start: bool| -> DVec3 {
        let direction = towards
            .map(|w| {
                let w = mesh.vertices[w as usize];
                if start { p1 - w } else { w - p0 }
            })
            .map(|d| d.normalize_or_zero() * length)
            .filter(|d| *d != DVec3::ZERO)
            .unwrap_or(chord);
        direction * tension
    };
    let m0 = tangent(continuation(mesh, q, p), true);
    let m1 = tangent(continuation(mesh, p, q), false);
    let (t2, t3) = (t * t, t * t * t);
    p0 * (2. * t3 - 3. * t2 + 1.)
        + m0 * (t3 - 2. * t2 + t)
        + p1 * (-2. * t3 + 3. * t2)
        + m1 * (t3 - t2)
}

/// Keep a placement off the edge ends (as the click's 1 %..99 % steps do).
fn clamp_t(t: f64) -> f64 {
    t.clamp(0.01, 0.99)
}

/// First and (Both Sides) second placement on the oriented ring edge
/// `p`→`q`, where `p` is on the target edge's `a` side.
fn placements(mesh: &Mesh, p: u32, q: u32, t: f64, o: &EdgeInsertOptions) -> (f64, f64) {
    if !o.absolute {
        return (t, 1. - t);
    }
    let length = mesh.vertices[p as usize]
        .distance(mesh.vertices[q as usize])
        .max(f64::MIN_POSITIVE);
    let (d1, d2) = (o.distance1 / length, o.distance2 / length);
    if o.from_end {
        (clamp_t(1. - d1), clamp_t(d2))
    } else {
        (clamp_t(d1), clamp_t(1. - d2))
    }
}

fn place(mesh: &Mesh, p: u32, q: u32, t: f64, o: &EdgeInsertOptions) -> Cut {
    Cut {
        a: p,
        b: q,
        t,
        at: o
            .preserve_curvature
            .then(|| curve_point(mesh, p, q, t, o.tension)),
    }
}

fn check_options(o: &EdgeInsertOptions) -> Result<()> {
    if !o.tension.is_finite() || !(0. ..=10.).contains(&o.tension) {
        return Err("Tension must be between 0% and 1000%".into());
    }
    if o.absolute
        && !(o.distance1.is_finite()
            && o.distance2.is_finite()
            && o.distance1 >= 0.
            && o.distance2 >= 0.)
    {
        return Err("Distances must be zero or positive".into());
    }
    Ok(())
}

/// One output corner: source corner, optional interpolation toward another
/// source corner (index, weight), and the output point.
type OutCorner = (usize, Option<(usize, f64)>, u32);

fn check_position(t: f64) -> Result<()> {
    if !t.is_finite() || t <= 0. || t >= 1. {
        return Err("Position must be strictly between 0% and 100%".into());
    }
    Ok(())
}

fn require_edge(mesh: &Mesh, a: u32, b: u32) -> Result<()> {
    if a == b || !mesh.edges().contains(&edge_key(a, b)) {
        return Err("The target edge does not exist".into());
    }
    if mesh
        .attributes
        .extrude_edge_uses
        .contains_key(&format!("{}:{}", a.min(b), a.max(b)))
    {
        return Err("Cannot split an edge with inconsistent winding; use Align first".into());
    }
    Ok(())
}

/// Linear interpolation of an optional corner value pair.
fn mix<T: Copy + std::ops::Mul<f64, Output = T> + std::ops::Add<Output = T>>(
    x: Option<T>,
    y: Option<T>,
    t: f64,
) -> Option<T> {
    Some(x? * (1. - t) + y? * t)
}

/// Apply `cuts` (distinct edges), then cut each polygon listed in `splits`
/// between the two new points on its edges. Returns the mesh and the new
/// point index per cut.
fn apply(mesh: &Mesh, cuts: &[Cut], splits: &[(usize, [usize; 2])]) -> Result<(Mesh, Vec<u32>)> {
    let mut out = mesh.clone();
    let mut new_point: BTreeMap<(u32, u32), (u32, Cut)> = BTreeMap::new();
    for cut in cuts {
        let p = cut.at.unwrap_or_else(|| {
            mesh.vertices[cut.a as usize].lerp(mesh.vertices[cut.b as usize], cut.t)
        });
        let index = u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
        out.vertices.push(p);
        new_point.insert(edge_key(cut.a, cut.b), (index, *cut));
    }
    // Insert new points into every face using a cut edge; remember where.
    // inserted[f] = corner positions (in the new cycle) of inserted points.
    let mut inserted: BTreeMap<usize, Vec<(u32, usize)>> = BTreeMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        if !(0..n).any(|i| {
            new_point.contains_key(&edge_key(face.vertices[i], face.vertices[(i + 1) % n]))
        }) {
            continue;
        }
        // Source corner of every output corner, with interpolation weight.
        let mut corners: Vec<OutCorner> = Vec::new();
        for i in 0..n {
            let (x, y) = (face.vertices[i], face.vertices[(i + 1) % n]);
            corners.push((i, None, x));
            if let Some(&(np, cut)) = new_point.get(&edge_key(x, y)) {
                // Weight toward corner i+1, measured from x.
                let w = if cut.a == x { cut.t } else { 1. - cut.t };
                corners.push((i, Some(((i + 1) % n, w)), np));
                inserted
                    .entry(fi)
                    .or_default()
                    .push((np, corners.len() - 1));
            }
        }
        let out_face = &mut out.faces[fi];
        out_face.vertices = corners.iter().map(|c| c.2).collect();
        if let Some(uv) = &face.uv {
            out_face.uv = Some(
                corners
                    .iter()
                    .map(|&(i, w, _)| match w {
                        None => uv[i],
                        Some((j, w)) => uv[i].lerp(uv[j], w),
                    })
                    .collect(),
            );
        }
        let rows = |row: &Vec<Option<glam::DVec2>>| -> Vec<Option<glam::DVec2>> {
            corners
                .iter()
                .map(|&(i, w, _)| match w {
                    None => row[i],
                    Some((j, w)) => mix(row[i], row[j], w),
                })
                .collect()
        };
        for (name, map) in &mesh.uv_maps {
            out.uv_maps.get_mut(name).ok_or("Missing UV map")?[fi] = rows(&map[fi]);
        }
        for (name, map) in &mesh.attributes.corner_weights {
            out.attributes
                .corner_weights
                .get_mut(name)
                .ok_or("Missing map")?[fi] = corners
                .iter()
                .map(|&(i, w, _)| match w {
                    None => map[fi][i],
                    Some((j, w)) => mix(map[fi][i], map[fi][j], w),
                })
                .collect();
        }
        for (name, map) in &mesh.attributes.corner_colors {
            out.attributes
                .corner_colors
                .get_mut(name)
                .ok_or("Missing map")?[fi] = corners
                .iter()
                .map(|&(i, w, _)| match w {
                    None => map[fi][i],
                    Some((j, w)) => mix(map[fi][i], map[fi][j], w),
                })
                .collect();
        }
        // Classes without a meaningful interpolation stay unassigned.
        for map in out.attributes.corner_normals.values_mut() {
            let old = map[fi].clone();
            map[fi] = corners
                .iter()
                .map(|&(i, w, _)| w.map_or(old[i], |_| None))
                .collect();
        }
        for map in out.attributes.uv_records.values_mut() {
            let old = map[fi].clone();
            map[fi] = corners
                .iter()
                .map(|&(i, w, _)| w.map_or(old[i], |_| None))
                .collect();
        }
        for map in out.attributes.typed_maps.values_mut() {
            if !map.corners.is_empty() {
                let old = map.corners[fi].clone();
                map.corners[fi] = corners
                    .iter()
                    .map(|&(i, w, _)| w.map_or(old[i].clone(), |_| None))
                    .collect();
            }
        }
    }
    // Cut polygons between their two inserted points.
    for &(fi, [first, second]) in splits {
        let points = &inserted[&fi];
        let (pa, pb) = (
            new_point_of(cuts, first, &new_point)?,
            new_point_of(cuts, second, &new_point)?,
        );
        let ia = points
            .iter()
            .find(|p| p.0 == pa)
            .ok_or("Missing loop point")?
            .1;
        let ib = points
            .iter()
            .find(|p| p.0 == pb)
            .ok_or("Missing loop point")?
            .1;
        split_face(&mut out, fi, ia, ib)?;
    }
    // Creases on split edges carry to both halves.
    let mut creases: Vec<MeshCrease> = Vec::new();
    for c in &mesh.attributes.creases {
        if let Some(&(np, _)) = new_point.get(&edge_key(c.a, c.b)) {
            creases.push(MeshCrease {
                a: c.a.min(np),
                b: c.a.max(np),
                weight: c.weight,
            });
            creases.push(MeshCrease {
                a: c.b.min(np),
                b: c.b.max(np),
                weight: c.weight,
            });
        } else {
            creases.push(c.clone());
        }
    }
    out.attributes.creases = creases;
    out.validate()
        .map_err(|e| format!("The cut produced an invalid mesh ({e}); nothing changed"))?;
    let points = cuts
        .iter()
        .map(|c| new_point[&edge_key(c.a, c.b)].0)
        .collect();
    Ok((out, points))
}

fn new_point_of(
    cuts: &[Cut],
    index: usize,
    new_point: &BTreeMap<(u32, u32), (u32, Cut)>,
) -> Result<u32> {
    let c = cuts.get(index).ok_or("Missing loop cut")?;
    Ok(new_point[&edge_key(c.a, c.b)].0)
}

/// Split face `fi` of `mesh` along the chord between corner positions
/// `i` and `j`: the first piece keeps the slot, the second is appended.
fn split_face(mesh: &mut Mesh, fi: usize, i: usize, j: usize) -> Result<()> {
    let n = mesh.faces[fi].vertices.len();
    let walk = |from: usize, to: usize| {
        let mut v = vec![from];
        let mut k = from;
        while k != to {
            k = (k + 1) % n;
            v.push(k);
        }
        v
    };
    let (first, second) = (walk(i, j), walk(j, i));
    let pick_rows = |rows: &[Option<glam::DVec2>], cs: &[usize]| {
        cs.iter().map(|&c| rows[c]).collect::<Vec<_>>()
    };
    let source = mesh.faces[fi].clone();
    let mut a = source.clone();
    let mut b = source.clone();
    a.vertices = first.iter().map(|&c| source.vertices[c]).collect();
    b.vertices = second.iter().map(|&c| source.vertices[c]).collect();
    if let Some(uv) = &source.uv {
        a.uv = Some(first.iter().map(|&c| uv[c]).collect());
        b.uv = Some(second.iter().map(|&c| uv[c]).collect());
    }
    mesh.faces[fi] = a;
    mesh.faces.push(b);
    for map in mesh.uv_maps.values_mut() {
        let row = map[fi].clone();
        map[fi] = pick_rows(&row, &first);
        map.push(pick_rows(&row, &second));
    }
    fn split_rows<T: Clone>(rows: &mut Vec<Vec<T>>, fi: usize, first: &[usize], second: &[usize]) {
        let row = rows[fi].clone();
        rows[fi] = first.iter().map(|&c| row[c].clone()).collect();
        rows.push(second.iter().map(|&c| row[c].clone()).collect());
    }
    let at = &mut mesh.attributes;
    for rows in at.uv_records.values_mut() {
        split_rows(rows, fi, &first, &second);
    }
    for rows in at.corner_weights.values_mut() {
        split_rows(rows, fi, &first, &second);
    }
    for rows in at.corner_colors.values_mut() {
        split_rows(rows, fi, &first, &second);
    }
    for rows in at.corner_normals.values_mut() {
        split_rows(rows, fi, &first, &second);
    }
    for map in at.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            split_rows(&mut map.corners, fi, &first, &second);
        }
    }
    let appended = (mesh.faces.len() - 1) as u32;
    if !at.smoothing_labels.is_empty() {
        let label = at.smoothing_labels[fi].clone();
        at.smoothing_labels.push(label);
    }
    if !at.representations.is_empty() {
        let r = at.representations[fi];
        at.representations.push(r);
    }
    for set in at.polygon_sets.values_mut() {
        if set.contains(&(fi as u32)) {
            set.insert(appended);
        }
    }
    Ok(())
}

/// Add one point on edge `a`–`b` at `t` (0..1 from `a`). Returns the mesh and
/// the new point index.
pub fn add_point(mesh: &Mesh, a: u32, b: u32, t: f64) -> Result<(Mesh, u32)> {
    add_point_with(mesh, a, b, t, &EdgeInsertOptions::default())
}

/// Add Point with placement options: Absolute Distance (`t` ignored) and
/// Preserve Curvature. Both Sides doesn't apply to a single point.
pub fn add_point_with(
    mesh: &Mesh,
    a: u32,
    b: u32,
    t: f64,
    options: &EdgeInsertOptions,
) -> Result<(Mesh, u32)> {
    mesh.validate()?;
    check_options(options)?;
    if !options.absolute {
        check_position(t)?;
    }
    require_edge(mesh, a, b)?;
    let (t, _) = placements(mesh, a, b, t, options);
    let (out, points) = apply(mesh, &[place(mesh, a, b, t, options)], &[])?;
    Ok((out, points[0]))
}

/// Add a loop through the polygon band crossing edge `a`–`b`, cutting each
/// edge of the band at `t` (0..1, measured from the `a` side of the band).
/// Returns the mesh and the new edges of the loop.
pub fn add_loop(mesh: &Mesh, a: u32, b: u32, t: f64) -> Result<(Mesh, Vec<(u32, u32)>)> {
    add_loop_with(mesh, a, b, t, &EdgeInsertOptions::default())
}

/// Crossed edges and the pairs of them new edges join.
pub type LoopRing = (Vec<(u32, u32)>, Vec<[usize; 2]>);

/// Add Loop's preview: the edges the loop through `a`–`b` crosses
/// (oriented from the `a` side) and which pairs of them a new edge joins.
pub fn add_loop_ring(mesh: &Mesh, a: u32, b: u32) -> Result<LoopRing> {
    let ring = walk_ring(mesh, a, b, None)?;
    Ok((ring.edges, ring.splits.iter().map(|s| s.1).collect()))
}

/// The quad band crossing an edge: its crossed edges, oriented so the first
/// point is on the start edge's `a` side, and the polygons to cut between
/// two of them (indices into `edges`).
struct Ring {
    edges: Vec<(u32, u32)>,
    splits: Vec<(usize, [usize; 2])>,
}

/// Walk the quad band in both directions from edge `a`→`b`. With `allowed`,
/// polygons outside it end the walk like a triangle does (they take the
/// point but aren't cut).
fn walk_ring(mesh: &Mesh, a: u32, b: u32, allowed: Option<&BTreeSet<usize>>) -> Result<Ring> {
    let faces_of = mesh.edge_faces();
    let mut edges = vec![(a, b)];
    let mut splits: Vec<(usize, [usize; 2])> = Vec::new();
    let mut seen_faces: BTreeSet<usize> = BTreeSet::new();
    let mut seen_edges: BTreeSet<(u32, u32)> = BTreeSet::from([edge_key(a, b)]);
    let start_faces = faces_of.get(&edge_key(a, b)).cloned().unwrap_or_default();
    if start_faces.len() > 2 {
        return Err("Add Loop needs a manifold edge".into());
    }
    let mut closed = false;
    for &first_face in &start_faces {
        if closed {
            break;
        }
        let mut cut_index = 0usize;
        let (mut p, mut q) = (a, b);
        let mut face = first_face;
        loop {
            if seen_faces.contains(&face) {
                break;
            }
            let f = &mesh.faces[face].vertices;
            if f.len() != 4 || allowed.is_some_and(|s| !s.contains(&face)) {
                // Triangles and n-gons take the point but end the loop.
                break;
            }
            seen_faces.insert(face);
            let n = 4;
            let ip = f.iter().position(|&v| v == p).ok_or("Broken loop")?;
            let iq = f.iter().position(|&v| v == q).ok_or("Broken loop")?;
            let other = |i: usize, not: usize| {
                let (x, y) = (f[(i + 1) % n], f[(i + n - 1) % n]);
                if x == f[not] { y } else { x }
            };
            let (np, nq) = (other(ip, iq), other(iq, ip));
            let next_key = edge_key(np, nq);
            if seen_edges.contains(&next_key) {
                // Came back to the start: a closed loop.
                splits.push((face, [cut_index, 0]));
                closed = true;
                break;
            }
            seen_edges.insert(next_key);
            edges.push((np, nq));
            let next_index = edges.len() - 1;
            splits.push((face, [cut_index, next_index]));
            cut_index = next_index;
            let Some(next_face) = faces_of[&next_key].iter().copied().find(|&g| g != face) else {
                break;
            };
            if faces_of[&next_key].len() > 2 {
                break;
            }
            p = np;
            q = nq;
            face = next_face;
        }
    }
    if splits.is_empty() {
        return Err("Add Loop needs a quad next to the target edge".into());
    }
    Ok(Ring { edges, splits })
}

/// Add Loop with placement options. With Both Sides a second loop is cut
/// at the mirrored place (`1 - t`, or Distance 2 from the far end) through
/// the same band, in the same edit; at the middle the two coincide and one
/// loop is made (AL-05). Returns the mesh and the new edges of every loop.
pub fn add_loop_with(
    mesh: &Mesh,
    a: u32,
    b: u32,
    t: f64,
    options: &EdgeInsertOptions,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    mesh.validate()?;
    check_options(options)?;
    if !options.absolute {
        check_position(t)?;
    }
    require_edge(mesh, a, b)?;
    let ring = walk_ring(mesh, a, b, None)?;
    let places: Vec<(f64, f64)> = ring
        .edges
        .iter()
        .map(|&(p, q)| placements(mesh, p, q, t, options))
        .collect();
    let cuts: Vec<Cut> = ring
        .edges
        .iter()
        .zip(&places)
        .map(|(&(p, q), &(t1, _))| place(mesh, p, q, t1, options))
        .collect();
    let (first, points) = apply(mesh, &cuts, &ring.splits)?;
    let mut edges: Vec<(u32, u32)> = ring
        .splits
        .iter()
        .map(|&(_, [x, y])| edge_key(points[x], points[y]))
        .collect();
    let (t1, t2) = places[0];
    if !options.both_sides || (t1 - t2).abs() < 1e-9 {
        return Ok((first, edges));
    }
    // Second loop: walk the first loop's halves on the chosen side only.
    let far_side = t2 > t1;
    let index_of: BTreeMap<u32, usize> = points.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let mut halves: BTreeSet<usize> = ring.splits.iter().map(|s| s.0).collect();
    halves.extend(mesh.faces.len()..first.faces.len());
    let (start_a, start_b) = if far_side {
        (points[0], b)
    } else {
        (a, points[0])
    };
    let second = walk_ring(&first, start_a, start_b, Some(&halves))?;
    let mut cuts2 = Vec::with_capacity(second.edges.len());
    for &(x, y) in &second.edges {
        // Map each half-edge back to its source ring edge.
        let source = index_of
            .get(if far_side { &x } else { &y })
            .copied()
            .ok_or("Both Sides couldn't follow the first loop; nothing changed")?;
        let (t1, t2) = places[source];
        let local = if far_side {
            (t2 - t1) / (1. - t1)
        } else {
            t2 / t1
        };
        let (p, q) = ring.edges[source];
        // Where Absolute distances would cross on a short edge, the second
        // point hugs the first one on its own side instead.
        let inside = (1e-3..=1. - 1e-3).contains(&local);
        cuts2.push(Cut {
            a: x,
            b: y,
            t: local.clamp(1e-3, 1. - 1e-3),
            // The second loop lies on the same curve as the first, measured
            // on the source edge.
            at: (inside && options.preserve_curvature)
                .then(|| curve_point(mesh, p, q, t2, options.tension)),
        });
    }
    let (out, points2) = apply(&first, &cuts2, &second.splits)?;
    edges.extend(
        second
            .splits
            .iter()
            .map(|&(_, [x, y])| edge_key(points2[x], points2[y])),
    );
    Ok((out, edges))
}

impl crate::Editor {
    /// Shared preflight: idle editor, a visible active mesh.
    pub(crate) fn edge_insert_target(&self, what: &str) -> Result<u64> {
        if self.i2_active() || self.session.pen_draft.is_some() {
            return Err(format!("Finish or Cancel the active tool before {what}"));
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err(format!("Finish selection before {what}"));
        }
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or_else(|| format!("Select a visible mesh before {what}"))?;
        Ok(item.id)
    }

    /// Publish `mesh` for the active item as one edit labeled `label`.
    /// New topology gets fresh component identities.
    pub(crate) fn publish_edge_edit(
        &mut self,
        item: u64,
        mesh: Mesh,
        label: &str,
        loop_edges: Option<Vec<(u32, u32)>>,
    ) -> Result<()> {
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
        let edges: BTreeSet<(u32, u32)> = self
            .scene
            .active()
            .map(|i| i.mesh.edges().into_iter().collect())
            .unwrap_or_default();
        let selection = &mut self.scene.selection;
        selection
            .edges
            .retain(|&(x, y)| edges.contains(&edge_key(x, y)));
        if let Some(loop_edges) = loop_edges
            && selection.mode == crate::Mode::Edge
        {
            selection.edges = loop_edges;
        }
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.invalidate_mesh(&self.scene, item))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }

    /// Options with world-metre distances converted to the active item's
    /// own units (uniform scale assumed, as for the vertex tools).
    fn local_insert_options(&self, item: u64, options: &EdgeInsertOptions) -> EdgeInsertOptions {
        let scale = self
            .scene
            .world_matrix(item)
            .map(crate::scene::uniform_scale)
            .unwrap_or(1.0);
        EdgeInsertOptions {
            distance1: options.distance1 / scale,
            distance2: options.distance2 / scale,
            ..options.clone()
        }
    }

    pub(crate) fn add_edge_point(
        &mut self,
        a: u32,
        b: u32,
        t: f64,
        options: &EdgeInsertOptions,
    ) -> Result<()> {
        let item = self.edge_insert_target("Add Point")?;
        let options = self.local_insert_options(item, options);
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, _) = add_point_with(mesh, a, b, t, &options)?;
        self.publish_edge_edit(item, out, "Add Point", None)
    }

    pub(crate) fn add_edge_loop(
        &mut self,
        a: u32,
        b: u32,
        t: f64,
        options: &EdgeInsertOptions,
    ) -> Result<()> {
        let item = self.edge_insert_target("Add Loop")?;
        let options = self.local_insert_options(item, options);
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let (out, edges) = add_loop_with(mesh, a, b, t, &options)?;
        self.publish_edge_edit(item, out, "Add Loop", Some(edges))
    }
}
