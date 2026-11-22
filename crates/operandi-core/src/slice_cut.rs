//! Shared cutting engine for the Slice tools (Loop Slice, Edge Slice,
//! Polygon Slice; spec toolbox-panels-2026-11-06, EDIT.md "Slice section").
//!
//! A cut is described as new points on edges ([`EdgeCut`]), optional new
//! points inside polygons ([`InnerPoint`]) and chords ([`Chord`]) that split
//! one polygon between two of its boundary points, passing through inner
//! points on the way. Any number of points may land on one edge; every
//! polygon using a cut edge takes its points, so no T-junction is left.
//!
//! Corner data follows Add Loop's rules (`edge_insert.rs`): UVs, UV maps,
//! corner weights and colours are interpolated when every source corner has
//! a value and left unassigned otherwise; corner normals, UV records and
//! typed corner records are unassigned on new corners. A polygon's pieces
//! keep its tags, part, material, smoothing label, representation and
//! polygon-set membership. A crease on a split edge carries to every piece
//! of that edge.
use crate::topology::edge_key;
use crate::{Mesh, MeshCrease, Result};
use glam::{DVec2, DVec3, DVec4};
use std::collections::{BTreeMap, BTreeSet};

/// A new point on edge `a`–`b` at `t` (0..1, measured from `a`), placed at
/// `at` instead when given (Preserve Curvature; corner data still
/// interpolates by `t`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EdgeCut {
    pub a: u32,
    pub b: u32,
    pub t: f64,
    pub at: Option<DVec3>,
}

/// A new point inside polygon `face` (mesh-local position).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InnerPoint {
    pub face: usize,
    pub at: DVec3,
}

/// A point a chord starts or ends at: an existing point or one of the cuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Node {
    Point(u32),
    Cut(usize),
}

/// Split polygon `face` from boundary node `from` to `to`, passing through
/// the inner points `through` (indices into the inner list) in order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Chord {
    pub face: usize,
    pub from: Node,
    pub to: Node,
    pub through: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CutResult {
    pub mesh: Mesh,
    /// The new point of every cut (cuts at the same place share one).
    pub cut_points: Vec<u32>,
    /// The new edges the chords made, each (low, high).
    pub new_edges: Vec<(u32, u32)>,
    /// Every polygon that is a piece of a chord-split polygon.
    pub pieces: Vec<u32>,
    /// The new point of every inner point, in the order given.
    pub inner_points: Vec<u32>,
    /// Every appended piece with the polygon it was cut from: (new, source).
    /// The first piece of a split polygon keeps the source's slot.
    pub piece_sources: Vec<(u32, u32)>,
}

/// The distinct cut places on each edge: (place from the low point, new
/// point, first cut there), ordered from the low point.
type Places = BTreeMap<(u32, u32), Vec<(f64, u32, usize)>>;

/// Where one output corner's data comes from, in terms of the source
/// polygon's corners.
#[derive(Clone, Debug, PartialEq)]
enum Source {
    Corner(usize),
    /// Between corner `i` and `j`, weight `w` toward `j`.
    Lerp(usize, usize, f64),
    /// A weighted blend of corners (an inner point).
    Blend(Vec<(usize, f64)>),
}

fn mix<T>(row: &[Option<T>], source: &Source) -> Option<T>
where
    T: Copy + std::ops::Mul<f64, Output = T> + std::ops::Add<Output = T>,
{
    match source {
        Source::Corner(i) => row.get(*i).copied().flatten(),
        Source::Lerp(i, j, w) => {
            Some(row.get(*i).copied()?? * (1. - w) + row.get(*j).copied()?? * *w)
        }
        Source::Blend(weights) => {
            let mut total: Option<T> = None;
            for &(i, w) in weights {
                let value = row.get(i).copied()?? * w;
                total = Some(match total {
                    Some(t) => t + value,
                    None => value,
                });
            }
            total
        }
    }
}

fn mix_plain(row: &[DVec2], source: &Source) -> DVec2 {
    match source {
        Source::Corner(i) => row[*i],
        Source::Lerp(i, j, w) => row[*i].lerp(row[*j], *w),
        Source::Blend(weights) => weights.iter().map(|&(i, w)| row[i] * w).sum(),
    }
}

fn keep<T: Clone>(row: &[Option<T>], source: &Source) -> Option<T> {
    match source {
        Source::Corner(i) => row.get(*i).cloned().flatten(),
        _ => None,
    }
}

/// Inverse-distance weights of `at` over the corners of polygon `face`.
fn blend_weights(mesh: &Mesh, face: usize, at: DVec3) -> Vec<(usize, f64)> {
    let corners = &mesh.faces[face].vertices;
    let distances: Vec<f64> = corners
        .iter()
        .map(|&v| mesh.vertices[v as usize].distance(at))
        .collect();
    if let Some(i) = distances.iter().position(|&d| d < 1e-12) {
        return vec![(i, 1.)];
    }
    let inverse: Vec<f64> = distances.iter().map(|d| 1. / d).collect();
    let sum: f64 = inverse.iter().sum();
    inverse
        .iter()
        .enumerate()
        .map(|(i, w)| (i, w / sum))
        .collect()
}

/// One output corner of a piece: a position in the polygon's extended
/// corner list, or an inner point.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ref {
    Ext(usize),
    Inner(usize),
}

/// Cut `mesh` as described. Fails (and changes nothing) when a cut names a
/// missing edge or sits on an edge end, a chord's ends aren't on its
/// polygon, or the result isn't a valid mesh.
pub(crate) fn cut(
    mesh: &Mesh,
    cuts: &[EdgeCut],
    inner: &[InnerPoint],
    chords: &[Chord],
) -> Result<CutResult> {
    let counts = mesh.edge_counts();
    // Cuts by edge, ordered from the edge's low point; equal places share
    // one new point.
    let mut by_edge: BTreeMap<(u32, u32), Vec<(f64, usize)>> = BTreeMap::new();
    for (index, c) in cuts.iter().enumerate() {
        if c.a == c.b || !counts.contains_key(&edge_key(c.a, c.b)) {
            return Err("The slice crosses an edge that doesn't exist".into());
        }
        if !c.t.is_finite() || c.t <= 1e-9 || c.t >= 1. - 1e-9 {
            return Err("A slice point can't sit on an edge's end".into());
        }
        let from_low = if c.a < c.b { c.t } else { 1. - c.t };
        by_edge
            .entry(edge_key(c.a, c.b))
            .or_default()
            .push((from_low, index));
    }
    let mut out = mesh.clone();
    let mut cut_points = vec![0u32; cuts.len()];
    let next_index = |out: &Mesh| -> Result<u32> {
        u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded".into())
    };
    // Per edge: the distinct places (from the low point) and their points.
    let mut places: Places = BTreeMap::new();
    for (edge, list) in &mut by_edge {
        list.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut distinct: Vec<(f64, u32, usize)> = Vec::new();
        for &(t, index) in list.iter() {
            if let Some(last) = distinct.last()
                && (t - last.0).abs() < 1e-9
            {
                cut_points[index] = last.1;
                continue;
            }
            let c = cuts[index];
            let p = c.at.unwrap_or_else(|| {
                mesh.vertices[c.a as usize].lerp(mesh.vertices[c.b as usize], c.t)
            });
            let point = next_index(&out)?;
            out.vertices.push(p);
            cut_points[index] = point;
            distinct.push((t, point, index));
        }
        places.insert(*edge, distinct);
    }
    let mut inner_points = Vec::with_capacity(inner.len());
    for p in inner {
        if p.face >= mesh.faces.len() || !p.at.is_finite() {
            return Err("A slice point lies outside the mesh".into());
        }
        inner_points.push(next_index(&out)?);
        out.vertices.push(p.at);
    }
    // Polygons that take points: every polygon using a cut edge (found
    // through the points' polygons, not a scan of every polygon) and
    // every chord's polygon.
    let point_faces = mesh.point_faces();
    let mut affected: BTreeSet<usize> = BTreeSet::new();
    for &(a, b) in places.keys() {
        for &f in point_faces.of(a) {
            let vs = &mesh.faces[f as usize].vertices;
            let n = vs.len();
            if (0..n).any(|i| edge_key(vs[i], vs[(i + 1) % n]) == (a, b)) {
                affected.insert(f as usize);
            }
        }
    }
    let mut chords_of: BTreeMap<usize, Vec<&Chord>> = BTreeMap::new();
    for chord in chords {
        if chord.face >= mesh.faces.len() || mesh.is_polyline(chord.face) {
            return Err("A slice crosses a polygon that doesn't exist".into());
        }
        affected.insert(chord.face);
        chords_of.entry(chord.face).or_default().push(chord);
    }
    let mut new_edges: Vec<(u32, u32)> = Vec::new();
    let mut pieces_out: Vec<u32> = Vec::new();
    // Rows of every face appended, in order, written after the loop.
    struct Built {
        vertices: Vec<u32>,
        sources: Vec<Source>,
    }
    let mut appended: Vec<(usize, Built)> = Vec::new();
    let mut replaced: Vec<(usize, Built)> = Vec::new();
    for &fi in &affected {
        let face = &mesh.faces[fi];
        let n = face.vertices.len();
        // Extended corner list: each corner, then the points on the edge
        // that follows it, in walking order.
        let mut ext: Vec<(Source, u32)> = Vec::new();
        let mut node_at: BTreeMap<Node, usize> = BTreeMap::new();
        let mut cut_at: BTreeMap<u32, usize> = BTreeMap::new();
        for i in 0..n {
            let (x, y) = (face.vertices[i], face.vertices[(i + 1) % n]);
            node_at.entry(Node::Point(x)).or_insert(ext.len());
            ext.push((Source::Corner(i), x));
            if mesh.is_polyline(fi) {
                continue;
            }
            if let Some(list) = places.get(&edge_key(x, y)) {
                let forward = x < y;
                let mut ordered: Vec<&(f64, u32, usize)> = list.iter().collect();
                if !forward {
                    ordered.reverse();
                }
                for &&(from_low, point, _) in &ordered {
                    let w = if forward { from_low } else { 1. - from_low };
                    cut_at.entry(point).or_insert(ext.len());
                    ext.push((Source::Lerp(i, (i + 1) % n, w), point));
                }
            }
        }
        let resolve = |node: Node| -> Result<usize> {
            match node {
                Node::Point(_) => node_at.get(&node).copied(),
                Node::Cut(c) => cut_points.get(c).and_then(|p| cut_at.get(p).copied()),
            }
            .ok_or_else(|| "A slice chord doesn't touch its polygon".to_string())
        };
        let mut pieces: Vec<Vec<Ref>> = vec![(0..ext.len()).map(Ref::Ext).collect()];
        let mut split = false;
        for chord in chords_of.get(&fi).into_iter().flatten() {
            let (e1, e2) = (resolve(chord.from)?, resolve(chord.to)?);
            if e1 == e2 {
                continue;
            }
            let Some(pi) = pieces
                .iter()
                .position(|p| p.contains(&Ref::Ext(e1)) && p.contains(&Ref::Ext(e2)))
            else {
                // The chord crosses an earlier one: not a simple split.
                return Err("Slices can't cross inside one polygon".into());
            };
            let piece = &pieces[pi];
            let m = piece.len();
            let k1 = piece.iter().position(|r| *r == Ref::Ext(e1)).unwrap_or(0);
            let k2 = piece.iter().position(|r| *r == Ref::Ext(e2)).unwrap_or(0);
            let adjacent = (k1 + 1) % m == k2 || (k2 + 1) % m == k1;
            if adjacent && chord.through.is_empty() {
                // Along an existing side: nothing to split.
                continue;
            }
            let walk = |from: usize, to: usize| {
                let mut v = vec![piece[from]];
                let mut k = from;
                while k != to {
                    k = (k + 1) % m;
                    v.push(piece[k]);
                }
                v
            };
            let mut first = walk(k1, k2);
            first.extend(chord.through.iter().rev().map(|&i| Ref::Inner(i)));
            let mut second = walk(k2, k1);
            second.extend(chord.through.iter().map(|&i| Ref::Inner(i)));
            if first.len() < 3 || second.len() < 3 {
                continue;
            }
            pieces[pi] = first;
            pieces.push(second);
            split = true;
            // The chord's new edges.
            let point_of = |r: Ref| match r {
                Ref::Ext(e) => ext[e].1,
                Ref::Inner(i) => inner_points[i],
            };
            let mut path = vec![Ref::Ext(e1)];
            path.extend(chord.through.iter().map(|&i| Ref::Inner(i)));
            path.push(Ref::Ext(e2));
            for w in path.windows(2) {
                new_edges.push(edge_key(point_of(w[0]), point_of(w[1])));
            }
        }
        let inner_sources: BTreeMap<usize, Source> = chords_of
            .get(&fi)
            .into_iter()
            .flatten()
            .flat_map(|c| c.through.iter().copied())
            .map(|i| (i, Source::Blend(blend_weights(mesh, fi, inner[i].at))))
            .collect();
        let build = |piece: &Vec<Ref>| -> Built {
            let mut vertices = Vec::with_capacity(piece.len());
            let mut sources = Vec::with_capacity(piece.len());
            for r in piece {
                match *r {
                    Ref::Ext(e) => {
                        vertices.push(ext[e].1);
                        sources.push(ext[e].0.clone());
                    }
                    Ref::Inner(i) => {
                        vertices.push(inner_points[i]);
                        sources.push(inner_sources.get(&i).cloned().unwrap_or(Source::Corner(0)));
                    }
                }
            }
            Built { vertices, sources }
        };
        let mut built = pieces.iter().map(build);
        if let Some(first) = built.next() {
            replaced.push((fi, first));
        }
        for extra in built {
            appended.push((fi, extra));
        }
        if split {
            pieces_out.push(fi as u32);
        }
    }
    // Write rows: the piece kept in the polygon's slot, then each extra
    // piece appended with the same per-polygon data.
    let rows = |src: &Mesh, fi: usize, built: &Built, out: &mut Mesh, slot: Option<usize>| {
        let face = &src.faces[fi];
        let mut new_face = face.clone();
        new_face.vertices = built.vertices.clone();
        new_face.uv = face
            .uv
            .as_ref()
            .map(|uv| built.sources.iter().map(|s| mix_plain(uv, s)).collect());
        let at = |slot: Option<usize>, faces: usize| slot.unwrap_or(faces);
        let index = at(slot, out.faces.len());
        match slot {
            Some(i) => out.faces[i] = new_face,
            None => out.faces.push(new_face),
        }
        fn put<T>(rows: &mut Vec<Vec<T>>, slot: Option<usize>, row: Vec<T>) {
            match slot {
                Some(i) => rows[i] = row,
                None => rows.push(row),
            }
        }
        for (name, map) in &src.uv_maps {
            if let Some(target) = out.uv_maps.get_mut(name) {
                let row = built.sources.iter().map(|s| mix(&map[fi], s)).collect();
                put(target, slot, row);
            }
        }
        let a = &src.attributes;
        for (name, map) in &a.corner_weights {
            if let Some(target) = out.attributes.corner_weights.get_mut(name) {
                let row = built.sources.iter().map(|s| mix(&map[fi], s)).collect();
                put(target, slot, row);
            }
        }
        for (name, map) in &a.corner_colors {
            if let Some(target) = out.attributes.corner_colors.get_mut(name) {
                let row: Vec<Option<DVec4>> =
                    built.sources.iter().map(|s| mix(&map[fi], s)).collect();
                put(target, slot, row);
            }
        }
        for (name, map) in &a.corner_normals {
            if let Some(target) = out.attributes.corner_normals.get_mut(name) {
                let row = built.sources.iter().map(|s| keep(&map[fi], s)).collect();
                put(target, slot, row);
            }
        }
        for (name, map) in &a.uv_records {
            if let Some(target) = out.attributes.uv_records.get_mut(name) {
                let row = built.sources.iter().map(|s| keep(&map[fi], s)).collect();
                put(target, slot, row);
            }
        }
        for (name, map) in &a.typed_maps {
            if map.corners.is_empty() {
                continue;
            }
            if let Some(target) = out.attributes.typed_maps.get_mut(name) {
                let row = built
                    .sources
                    .iter()
                    .map(|s| keep(&map.corners[fi], s))
                    .collect();
                put(&mut target.corners, slot, row);
            }
        }
        if slot.is_none() {
            let at = &mut out.attributes;
            if !at.smoothing_labels.is_empty() {
                let label = a.smoothing_labels.get(fi).cloned().flatten();
                at.smoothing_labels.push(label);
            }
            if !at.representations.is_empty() {
                at.representations.push(a.representations[fi]);
            }
            for (name, set) in &a.polygon_sets {
                if set.contains(&(fi as u32))
                    && let Some(target) = at.polygon_sets.get_mut(name)
                {
                    target.insert(index as u32);
                }
            }
        }
        index
    };
    for (fi, built) in &replaced {
        rows(mesh, *fi, built, &mut out, Some(*fi));
    }
    let mut piece_sources = Vec::with_capacity(appended.len());
    for (fi, built) in &appended {
        let index = rows(mesh, *fi, built, &mut out, None);
        pieces_out.push(index as u32);
        piece_sources.push((index as u32, *fi as u32));
    }
    // Creases on split edges carry to every piece of the edge.
    if !places.is_empty() && !mesh.attributes.creases.is_empty() {
        let mut creases: Vec<MeshCrease> = Vec::new();
        for c in &mesh.attributes.creases {
            match places.get(&edge_key(c.a, c.b)) {
                Some(list) => {
                    let (low, high) = edge_key(c.a, c.b);
                    let mut chain = vec![low];
                    chain.extend(list.iter().map(|p| p.1));
                    chain.push(high);
                    for w in chain.windows(2) {
                        let (a, b) = edge_key(w[0], w[1]);
                        creases.push(MeshCrease {
                            a,
                            b,
                            weight: c.weight,
                        });
                    }
                }
                None => creases.push(c.clone()),
            }
        }
        out.attributes.creases = creases;
    }
    out.validate()
        .map_err(|e| format!("The slice produced an invalid mesh ({e}); nothing changed"))?;
    new_edges.sort_unstable();
    new_edges.dedup();
    pieces_out.sort_unstable();
    pieces_out.dedup();
    Ok(CutResult {
        mesh: out,
        cut_points,
        new_edges,
        pieces: pieces_out,
        inner_points,
        piece_sources,
    })
}

/// The polygons (surface polygons only) that use edge `a`–`b`, from the
/// mesh's point-polygon lists (fetch `mesh.point_faces()` once per walk:
/// each fetch compares the whole topology).
pub(crate) fn faces_of_edge(
    mesh: &Mesh,
    point_faces: &crate::topology_cache::PointFaces,
    a: u32,
    b: u32,
) -> Vec<usize> {
    let mut faces: Vec<usize> = point_faces
        .of(a)
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| {
            !mesh.is_polyline(f) && {
                let vs = &mesh.faces[f].vertices;
                let n = vs.len();
                (0..n).any(|i| edge_key(vs[i], vs[(i + 1) % n]) == edge_key(a, b))
            }
        })
        .collect();
    faces.dedup();
    faces
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn quad_strip(n: u32) -> Mesh {
        // n quads in a row along X: points 0..=n bottom, n+1..=2n+1 top.
        let mut mesh = Mesh::default();
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(i as f64, 0., 0.));
        }
        for i in 0..=n {
            mesh.vertices.push(DVec3::new(i as f64, 1., 0.));
        }
        for i in 0..n {
            mesh.faces.push(Face {
                vertices: vec![i, i + 1, n + 2 + i, n + 1 + i],
                uv: Some(vec![
                    DVec2::new(0., 0.),
                    DVec2::new(1., 0.),
                    DVec2::new(1., 1.),
                    DVec2::new(0., 1.),
                ]),
                material: "M".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
        mesh
    }

    #[test]
    fn two_points_on_one_edge_split_both_neighbours() {
        let mesh = quad_strip(2);
        // Edge 1–4 is shared by both quads.
        let cuts = [
            EdgeCut {
                a: 1,
                b: 4,
                t: 0.25,
                at: None,
            },
            EdgeCut {
                a: 4,
                b: 1,
                t: 0.25,
                at: None,
            },
        ];
        let r = cut(&mesh, &cuts, &[], &[]).unwrap();
        assert_eq!(r.mesh.vertices.len(), 8);
        assert_eq!(r.mesh.faces[0].vertices.len(), 6);
        assert_eq!(r.mesh.faces[1].vertices.len(), 6);
        assert_eq!(
            r.mesh.vertices[r.cut_points[0] as usize],
            DVec3::new(1., 0.25, 0.)
        );
        assert_eq!(
            r.mesh.vertices[r.cut_points[1] as usize],
            DVec3::new(1., 0.75, 0.)
        );
    }

    #[test]
    fn chord_through_an_inner_point_splits_and_interpolates_uvs() {
        let mesh = quad_strip(1);
        let cuts = [
            EdgeCut {
                a: 0,
                b: 1,
                t: 0.5,
                at: None,
            },
            EdgeCut {
                a: 3,
                b: 2,
                t: 0.5,
                at: None,
            },
        ];
        let inner = [InnerPoint {
            face: 0,
            at: DVec3::new(0.5, 0.5, 0.),
        }];
        let chords = [Chord {
            face: 0,
            from: Node::Cut(0),
            to: Node::Cut(1),
            through: vec![0],
        }];
        let r = cut(&mesh, &cuts, &inner, &chords).unwrap();
        assert_eq!(r.mesh.faces.len(), 2);
        assert_eq!(r.new_edges.len(), 2);
        let uv = r.mesh.faces[0].uv.as_ref().unwrap();
        assert!(
            uv.iter()
                .any(|u| (*u - DVec2::new(0.5, 0.)).length() < 1e-12)
        );
        assert_eq!(r.mesh.faces[1].material, "M");
    }

    #[test]
    fn missing_edge_is_refused() {
        let mesh = quad_strip(1);
        let cuts = [EdgeCut {
            a: 0,
            b: 3,
            t: 0.5,
            at: None,
        }];
        assert!(cut(&mesh, &cuts, &[], &[]).is_err());
    }
}
