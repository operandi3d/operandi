//! Edge Extrude (spec `extrude-2026-11-05`).
//!
//! Each selected edge is raised into a ridge. In every polygon on the edge,
//! the edge is replaced by a copy slid into that polygon by Width: each end
//! of the copy sits on the polygon's other edge at that corner, Width from
//! the corner. The original edge moves by Extrude along the normalized
//! average normal of its polygons and becomes the ridge; one quad per
//! polygon joins the slid copy to it. With nothing selected every edge is
//! processed; Width 0 changes nothing.
//!
//! Ends of the ridge:
//! * Where polygons without a selected edge meet the end (a cube corner),
//!   they take the slid end points in place of the corner, and one triangle
//!   per such polygon closes the gap to the ridge end (EE06). An edge there
//!   between two such polygons (a grid's interior end) slides too, so every
//!   one of them drops the corner (UVN-O-edge-extrude-cont). Where one
//!   still keeps the corner (it meets the mesh border there), a cap through
//!   the kept corner closes the gap instead (provisional).
//! * Where no such polygon is left (a mesh border), there is no cap and the
//!   original point is dropped.
//! * Connected selected edges in a line share their ridge point.
//!
//! Provisional readings (spec open questions):
//! * Border edges (one polygon) get one ridge quad, like one side of an
//!   interior edge (Q-EX-03, decided 2026-11-05: a proper ridge, not the
//!   reference's open cut).
//! * A polygon corner whose two edges are both selected moves to
//!   corner + Width along each of the two edges (the 2×2 grid's cross, EE07;
//!   other angles are Q-EX-05).
//! * Width reaching past a side edge is refused (Q-EX-04); edges with more
//!   than two polygons are refused (Q-EX-06).
//! * New polygons take their source polygon's tags (Q-EX-07); with Use
//!   Material only the ridge quads get the material (EE08).
//!
//! UV maps follow the shared rule in `corner_uv`: each new point's UV is
//! interpolated inside the polygon it serves. Meshes with other per-corner
//! data (corner weights, colours, normals, typed corner records) are
//! refused.
use crate::corner_uv::{UvRows, corner_uv, lerp, records_from_uvs};
use crate::topology::edge_key;
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// Where a new point comes from, for its UVs.
#[derive(Clone, Copy)]
enum Source {
    /// On edge `v`–`x`, a share `t` of the way from `v`.
    Slid { v: u32, x: u32, t: f64 },
    /// Corner `v` of polygon `face` moved `tu` toward `u` and `tx` toward `x`.
    Corner {
        face: usize,
        v: u32,
        u: u32,
        tu: f64,
        x: u32,
        tx: f64,
    },
    /// The ridge point standing on `v`.
    Ridge(u32),
}

fn push(out: &mut Mesh, sources: &mut BTreeMap<u32, Source>, q: DVec3, s: Source) -> Result<u32> {
    let i = u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
    out.vertices.push(q);
    sources.insert(i, s);
    Ok(i)
}

/// Extrude the selected edges of `mesh` by `extrude` with `width`. Returns
/// the mesh and the ridge edges. See [`edge_extrude_with`] for Use Material.
pub fn edge_extrude_geometry(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    extrude: f64,
    width: f64,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    edge_extrude_with(mesh, edges, extrude, width, None)
}

/// Edge Extrude with an optional Use Material name for the ridge quads.
pub fn edge_extrude_with(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    extrude: f64,
    width: f64,
    material: Option<&str>,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    mesh.validate()?;
    if !extrude.is_finite() || !width.is_finite() || width < 0. {
        return Err("Edge Extrude needs a finite Extrude and a nonnegative Width".into());
    }
    // Nothing selected: every edge (EE07).
    let selected: BTreeSet<(u32, u32)> = if edges.is_empty() {
        mesh.edges().into_iter().collect()
    } else {
        edges.iter().map(|&(a, b)| edge_key(a, b)).collect()
    };
    if selected.is_empty() {
        return Err("Edge Extrude needs a mesh with edges".into());
    }
    // Width creates the geometry; Width 0 changes nothing (EE01, EE02).
    if width == 0. {
        return Ok((mesh.clone(), selected.into_iter().collect()));
    }
    if crate::corner_uv::has_uncarried_corner_data(mesh) {
        return Err(
            "Edge Extrude cannot yet remap corner weights, colors or normals; nothing changed"
                .into(),
        );
    }
    if !mesh.attributes.extrude_edge_uses.is_empty() {
        return Err("Edge Extrude needs consistent winding; use Align first".into());
    }
    let mut uses: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let n = f.vertices.len();
        for i in 0..n {
            let (x, y) = (f.vertices[i], f.vertices[(i + 1) % n]);
            uses.entry(edge_key(x, y)).or_default().push(fi);
        }
    }
    for e in &selected {
        match uses.get(e).map(Vec::len) {
            None => return Err("A selected edge no longer exists".into()),
            Some(1 | 2) => {}
            Some(_) => {
                return Err("Edge Extrude needs edges with one or two polygons".into());
            }
        }
    }
    let p = |v: u32| mesh.vertices[v as usize];
    let dir = |from: u32, to: u32| (p(to) - p(from)).normalize_or_zero();
    let mut out = mesh.clone();
    let mut sources: BTreeMap<u32, Source> = BTreeMap::new();
    let ends: BTreeSet<u32> = selected.iter().flat_map(|&(a, b)| [a, b]).collect();
    let ring = |f: usize| &mesh.faces[f].vertices;
    let corner = |f: usize, i: usize| {
        let r = ring(f);
        let n = r.len();
        (r[(i + n - 1) % n], r[i], r[(i + 1) % n])
    };
    // Slid points on unselected edges, keyed by (near end, far end).
    let mut slid: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    // Replacement of each polygon corner at a ridge end.
    let mut replace: BTreeMap<(usize, usize), Vec<u32>> = BTreeMap::new();
    // The one point a selected-edge corner becomes (for the ridge quads).
    let mut moved: BTreeMap<(usize, u32), u32> = BTreeMap::new();
    let slide = |out: &mut Mesh,
                 sources: &mut BTreeMap<u32, Source>,
                 slid: &mut BTreeMap<(u32, u32), u32>,
                 v: u32,
                 x: u32|
     -> Result<u32> {
        if let Some(&q) = slid.get(&(v, x)) {
            return Ok(q);
        }
        let length = p(v).distance(p(x));
        if width >= length - 1e-9 {
            return Err("Edge Extrude Width reaches the far end of a side edge; reduce it".into());
        }
        let q = push(
            out,
            sources,
            p(v) + dir(v, x) * width,
            Source::Slid {
                v,
                x,
                t: width / length,
            },
        )?;
        slid.insert((v, x), q);
        Ok(q)
    };
    // Pass 1: corners with a selected edge.
    for f in 0..mesh.faces.len() {
        for i in 0..ring(f).len() {
            let (u, v, x) = corner(f, i);
            let (incoming, outgoing) = (
                selected.contains(&edge_key(u, v)),
                selected.contains(&edge_key(v, x)),
            );
            let q = match (incoming, outgoing) {
                (false, false) => continue,
                (true, false) => slide(&mut out, &mut sources, &mut slid, v, x)?,
                (false, true) => slide(&mut out, &mut sources, &mut slid, v, u)?,
                (true, true) => {
                    let (lu, lx) = (p(v).distance(p(u)), p(v).distance(p(x)));
                    if width >= lu - 1e-9 || width >= lx - 1e-9 {
                        return Err(
                            "Edge Extrude Width reaches the far end of a side edge; reduce it"
                                .into(),
                        );
                    }
                    push(
                        &mut out,
                        &mut sources,
                        p(v) + (dir(v, u) + dir(v, x)) * width,
                        Source::Corner {
                            face: f,
                            v,
                            u,
                            tu: width / lu,
                            x,
                            tx: width / lx,
                        },
                    )?
                }
            };
            replace.insert((f, i), vec![q]);
            moved.insert((f, v), q);
        }
    }
    // Both ends slid along one edge: they must not cross.
    for &(v, x) in slid.keys() {
        if slid.contains_key(&(x, v)) && 2. * width >= p(v).distance(p(x)) - 1e-9 {
            return Err("Edge Extrude edges are too close together for this Width".into());
        }
    }
    // An edge at a ridge end between two other polygons (the grid's far
    // side of an interior end) slides too, so each of those polygons drops
    // the corner and gets its own end triangle (EE06's rule, one triangle
    // per other polygon; UVN-O-edge-extrude-cont).
    for f in 0..mesh.faces.len() {
        for i in 0..ring(f).len() {
            let (_, v, x) = corner(f, i);
            if !ends.contains(&v) || replace.contains_key(&(f, i)) || slid.contains_key(&(v, x)) {
                continue;
            }
            let between_others = uses[&edge_key(v, x)].len() == 2
                && uses[&edge_key(v, x)].iter().all(|&g| {
                    (0..ring(g).len()).all(|j| ring(g)[j] != v || !replace.contains_key(&(g, j)))
                });
            if between_others && !selected.contains(&edge_key(v, x)) {
                slide(&mut out, &mut sources, &mut slid, v, x)?;
            }
        }
    }
    // Pass 2: other polygons at a ridge end take the slid points on their
    // edges there, and drop the corner when both of its edges carry one.
    let mut others: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for f in 0..mesh.faces.len() {
        for i in 0..ring(f).len() {
            let (u, v, x) = corner(f, i);
            if !ends.contains(&v) || replace.contains_key(&(f, i)) {
                continue;
            }
            let (before, after) = (slid.get(&(v, u)).copied(), slid.get(&(v, x)).copied());
            let mut run: Vec<u32> = before.into_iter().collect();
            if before.is_none() || after.is_none() {
                run.push(v);
            }
            run.extend(after);
            replace.insert((f, i), run);
            others.entry(v).or_default().push(f);
        }
    }
    // Ridge points: each end moves along the average normal of its polygons
    // that hold a selected edge there.
    let mut ridge: BTreeMap<u32, u32> = BTreeMap::new();
    for &v in &ends {
        let faces: BTreeSet<usize> = moved
            .keys()
            .filter(|&&(_, w)| w == v)
            .map(|&(f, _)| f)
            .collect();
        let n = faces
            .iter()
            .map(|&f| mesh.normal(f))
            .sum::<DVec3>()
            .normalize_or_zero();
        if n == DVec3::ZERO && extrude != 0. {
            return Err("Edge Extrude needs neighbours that do not fold back on each other".into());
        }
        ridge.insert(
            v,
            push(&mut out, &mut sources, p(v) + n * extrude, Source::Ridge(v))?,
        );
    }
    // Rebuild every polygon.
    for (f, face) in mesh.faces.iter().enumerate() {
        let mut cycle = Vec::with_capacity(face.vertices.len() + 2);
        for (i, &v) in face.vertices.iter().enumerate() {
            match replace.get(&(f, i)) {
                Some(run) => cycle.extend(run),
                None => cycle.push(v),
            }
        }
        out.faces[f].vertices = cycle;
    }
    // Ridge quads: one per polygon on each selected edge, facing its side.
    let mut new_faces: Vec<(Vec<u32>, usize, bool)> = Vec::new();
    // Each ridge quad's selected edge, by its index in `new_faces`.
    let mut wall_edges: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
    let mut raised = Vec::new();
    for &(a, b) in &selected {
        for &f in &uses[&(a, b)] {
            let r = ring(f);
            let n = r.len();
            let i = r.iter().position(|&x| x == a).unwrap();
            // Orient along the polygon: `from` → `to`.
            let (from, to) = if r[(i + 1) % n] == b { (a, b) } else { (b, a) };
            wall_edges.insert(new_faces.len(), (a, b));
            new_faces.push((
                vec![moved[&(f, to)], moved[&(f, from)], ridge[&from], ridge[&to]],
                f,
                true,
            ));
        }
        raised.push(edge_key(ridge[&a], ridge[&b]));
    }
    // Caps: at each end with other polygons, close the hole between the
    // ridge quads and those polygons. Its edges are the half-edges near the
    // end that have no twin, reversed.
    let mut half: BTreeSet<(u32, u32)> = BTreeSet::new();
    let add_half = |vertices: &[u32], half: &mut BTreeSet<(u32, u32)>| {
        for i in 0..vertices.len() {
            half.insert((vertices[i], vertices[(i + 1) % vertices.len()]));
        }
    };
    for f in &out.faces {
        add_half(&f.vertices, &mut half);
    }
    for (vertices, ..) in &new_faces {
        add_half(vertices, &mut half);
    }
    for (&v, polygons) in &others {
        let mut near: BTreeSet<u32> = slid
            .iter()
            .filter(|&(&(w, _), _)| w == v)
            .map(|(_, &q)| q)
            .collect();
        near.insert(v);
        near.insert(ridge[&v]);
        let mut next: BTreeMap<u32, u32> = BTreeMap::new();
        for &(a, b) in &half {
            if near.contains(&a) && near.contains(&b) && !half.contains(&(b, a)) {
                // The cap runs the other way.
                if next.insert(b, a).is_some() {
                    return Err("Edge Extrude can't close the end of this ridge".into());
                }
            }
        }
        let mut seen = BTreeSet::new();
        let starts: Vec<u32> = {
            let targets: BTreeSet<u32> = next.values().copied().collect();
            let open: Vec<u32> = next
                .keys()
                .copied()
                .filter(|k| !targets.contains(k))
                .collect();
            if open.is_empty() {
                next.keys().copied().collect()
            } else {
                open
            }
        };
        for s in starts {
            if seen.contains(&s) {
                continue;
            }
            let mut cap = vec![s];
            seen.insert(s);
            let mut at = s;
            while let Some(&n) = next.get(&at) {
                if n == s || !seen.insert(n) {
                    break;
                }
                cap.push(n);
                at = n;
            }
            let r = ridge[&v];
            if cap.len() > 3 && !cap.contains(&v) && cap.contains(&r) {
                // The corner was dropped everywhere: one triangle per other
                // polygon, each joining its two slid points to the ridge end.
                let k = cap.iter().position(|&x| x == r).unwrap();
                cap.rotate_left(k);
                for w in cap[1..].windows(2) {
                    let home = polygons
                        .iter()
                        .copied()
                        .find(|&g| w.iter().all(|q| out.faces[g].vertices.contains(q)))
                        .unwrap_or(polygons[0]);
                    new_faces.push((vec![w[0], w[1], r], home, false));
                }
            } else if cap.len() >= 3 {
                new_faces.push((cap, polygons[0], false));
            }
        }
    }
    for (vertices, template, is_ridge) in &new_faces {
        let mut face = Face {
            vertices: vertices.clone(),
            uv: None,
            ..mesh.faces[*template].clone()
        };
        if let (true, Some(name)) = (*is_ridge, material) {
            face.material = name.to_string();
            face.material_label = Some(name.to_string());
        }
        out.faces.push(face);
        if !mesh.attributes.smoothing_labels.is_empty() {
            let l = mesh.attributes.smoothing_labels[*template].clone();
            out.attributes.smoothing_labels.push(l);
        }
        if !mesh.attributes.representations.is_empty() {
            let r = mesh.attributes.representations[*template];
            out.attributes.representations.push(r);
        }
    }
    // UVs: each corner's UV inside its home polygon (the rebuilt polygon
    // itself, a ridge quad's source polygon, a cap's first other polygon).
    let homes: Vec<usize> = (0..mesh.faces.len())
        .chain(new_faces.iter().map(|&(_, t, _)| t))
        .collect();
    let containing = |vs: &[u32]| {
        (0..mesh.faces.len()).find(|&f| vs.iter().all(|v| mesh.faces[f].vertices.contains(v)))
    };
    let uv_of = |rows: &UvRows, home: usize, id: u32| -> Option<DVec2> {
        let at = |vs: &[u32]| {
            if vs.iter().all(|v| mesh.faces[home].vertices.contains(v)) {
                Some(home)
            } else {
                containing(vs)
            }
        };
        match sources.get(&id) {
            None => corner_uv(mesh, rows, at(&[id])?, id),
            Some(&Source::Ridge(v)) => corner_uv(mesh, rows, at(&[v])?, v),
            Some(&Source::Slid { v, x, t }) => {
                let f = at(&[v, x])?;
                lerp(corner_uv(mesh, rows, f, v), corner_uv(mesh, rows, f, x), t)
            }
            Some(&Source::Corner {
                face,
                v,
                u,
                tu,
                x,
                tx,
            }) => {
                let c = corner_uv(mesh, rows, face, v)?;
                let cu = corner_uv(mesh, rows, face, u)?;
                let cx = corner_uv(mesh, rows, face, x)?;
                Some(c + (cu - c) * tu + (cx - c) * tx)
            }
        }
    };
    // UVN-O-edge-extrude-seam (provisional): on a flat selected edge where
    // the two polygons' UVs differ by one constant shift (a seam between a
    // lower- and a higher-numbered polygon's islands), both ridge quads take
    // the higher-numbered side's island, so the lower one's wall is shifted
    // across.
    let wall_shift = |rows: &UvRows, k: usize| -> Option<DVec2> {
        let &(a, b) = wall_edges.get(&k)?;
        let f = new_faces[k].1;
        let g = *uses[&(a, b)].iter().find(|&&g| g > f)?;
        if !crate::corner_uv::flat(mesh, [f, g]) {
            return None;
        }
        let sa = corner_uv(mesh, rows, g, a)? - corner_uv(mesh, rows, f, a)?;
        let sb = corner_uv(mesh, rows, g, b)? - corner_uv(mesh, rows, f, b)?;
        (sa.distance(sb) <= 1e-9 && sa.length() > 1e-9).then_some(sa)
    };
    let base = mesh.faces.len();
    for (name, map) in out.uv_maps.iter_mut() {
        let rows = &mesh.uv_maps[name];
        *map = out
            .faces
            .iter()
            .zip(&homes)
            .enumerate()
            .map(|(i, (face, &home))| {
                let shift = i
                    .checked_sub(base)
                    .and_then(|k| wall_shift(rows, k))
                    .unwrap_or(DVec2::ZERO);
                face.vertices
                    .iter()
                    .map(|&id| uv_of(rows, home, id).map(|uv| uv + shift))
                    .collect()
            })
            .collect();
    }
    let rows: Vec<usize> = out.faces.iter().map(|f| f.vertices.len()).collect();
    let at = &mut out.attributes;
    for (name, map) in at.uv_records.iter_mut() {
        *map = records_from_uvs(&out.faces, &out.uv_maps[name]);
    }
    for map in at.corner_weights.values_mut() {
        *map = rows.iter().map(|&n| vec![None; n]).collect();
    }
    for map in at.corner_colors.values_mut() {
        *map = rows.iter().map(|&n| vec![None; n]).collect();
    }
    for map in at.corner_normals.values_mut() {
        *map = rows.iter().map(|&n| vec![None; n]).collect();
    }
    for map in at.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            map.corners = rows.iter().map(|&n| vec![None; n]).collect();
        }
    }
    // The extruded edges no longer exist; creases on them are dropped.
    let kept: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    out.attributes
        .creases
        .retain(|c| kept.contains(&edge_key(c.a, c.b)));
    // Ends with nothing left around them drop their old point.
    let id = crate::smooth_shift::compact(mesh, &mut out);
    let id = |v: u32| id[v as usize].ok_or("Edge Extrude lost a ridge point");
    let raised = raised
        .into_iter()
        .map(|(a, b)| Ok(edge_key(id(a)?, id(b)?)))
        .collect::<Result<Vec<_>>>()?;
    out.validate()
        .map_err(|e| format!("Edge Extrude produced an invalid mesh ({e}); nothing changed"))?;
    Ok((out, raised))
}

/// Share of the shortest side edge that a fresh Edge or Vertex Extrude starts
/// its Width at. Provisional (EXW-Q01): the reference's starting Width isn't
/// measured, and Width 0 changes nothing, so a fresh tool needs one.
pub const START_WIDTH_SHARE: f64 = 0.1;

/// Starting Width for Edge Extrude on `edges`: a share of the shortest mesh
/// edge that touches a selected edge's end without being selected itself
/// (the side edges the Width slides along). `None` when there is none.
pub fn edge_extrude_start_width(mesh: &Mesh, edges: &[(u32, u32)]) -> Option<f64> {
    // Nothing selected extrudes every edge (EE07): no edge is a side edge,
    // so start from the shortest edge of all.
    if edges.is_empty() {
        let all = (0..mesh.vertices.len() as u32).collect();
        return shortest_touching(mesh, &all, &BTreeSet::new()).map(|l| l * START_WIDTH_SHARE);
    }
    let selected: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    let ends: BTreeSet<u32> = selected.iter().flat_map(|&(a, b)| [a, b]).collect();
    shortest_touching(mesh, &ends, &selected).map(|l| l * START_WIDTH_SHARE)
}

/// Starting Width for Vertex Extrude: a share of the shortest edge at a
/// selected point.
pub fn vertex_extrude_start_width(mesh: &Mesh, points: &[u32]) -> Option<f64> {
    // Nothing selected extrudes every point (VE08).
    let ends: BTreeSet<u32> = if points.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        points.iter().copied().collect()
    };
    shortest_touching(mesh, &ends, &BTreeSet::new()).map(|l| l * START_WIDTH_SHARE)
}

/// Shortest polygon edge with an end in `ends`, skipping `skip`. Walks the
/// polygons once; runs at tool activation, not per drag frame.
fn shortest_touching(
    mesh: &Mesh,
    ends: &BTreeSet<u32>,
    skip: &BTreeSet<(u32, u32)>,
) -> Option<f64> {
    let mut best: Option<f64> = None;
    for f in &mesh.faces {
        let n = f.vertices.len();
        for i in 0..n {
            let (x, y) = (f.vertices[i], f.vertices[(i + 1) % n]);
            if !(ends.contains(&x) || ends.contains(&y)) || skip.contains(&edge_key(x, y)) {
                continue;
            }
            let length = mesh.vertices[x as usize].distance(mesh.vertices[y as usize]);
            if length > 0. && best.is_none_or(|b| length < b) {
                best = Some(length);
            }
        }
    }
    best
}
