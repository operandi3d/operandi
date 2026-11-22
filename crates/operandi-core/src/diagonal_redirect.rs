//! Diagonal Re-Direct (Edit tab, Commands). Spec: missing-tools-g1
//! G1-SMALL-COMMANDS v1, MT4-01..07, cases DR-A..H.
//!
//! Two selected quads touching only at one corner C (diagonal
//! neighbours) and the two side quads T that share an edge with both form
//! a 2×2 block round C. The selected quads split into triangles along
//! their diagonal through C (MT4-04). Each side quad off the open border
//! gets a point M at its centre, and it and the two triangles beside it
//! become three quads, so the new diagonal is a clean edge loop (MT4-05).
//! A side quad touching the border is left, and its triangles stay
//! triangles (MT4-06).
//!
//! Provisional (Q-DR-1): side polygons that aren't quads, or a pole at C
//! (not exactly four polygons round it), refuse the whole command.
use crate::edge_commands::{OutFace, rebuild};
use crate::topology::edge_key;
use crate::{Editor, Mesh, Mode, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// The refusal for anything but diagonal quad pairs (MT4-01).
pub const SELECT_TWO: &str = "Select two diagonal polygons.";

/// The result: the new mesh, and the corners C of the pairs whose side
/// quads were both rebuilt (MT4-07).
pub struct Redirected {
    pub mesh: Mesh,
    pub corners: Vec<u32>,
}

/// One diagonal pair: selected quads `s` (two), their shared corner `c`
/// and the two side quads.
struct Pair {
    c: u32,
    sides: [usize; 2],
}

/// Re-Direct the diagonal pairs among `selected` polygons of `mesh`.
pub fn diagonal_redirect_geometry(mesh: &Mesh, selected: &[u32]) -> Result<Redirected> {
    let chosen: BTreeSet<usize> = selected.iter().map(|&f| f as usize).collect();
    if chosen.len() < 2 {
        return Err(SELECT_TWO.into());
    }
    let is_quad =
        |f: usize| mesh.faces.get(f).is_some_and(|x| x.vertices.len() == 4) && !mesh.is_polyline(f);
    if !chosen.iter().all(|&f| is_quad(f)) {
        return Err(SELECT_TWO.into());
    }
    let point_faces = mesh.point_faces();
    let edge_counts = mesh.edge_counts();
    let corners = |f: usize| &mesh.faces[f].vertices;
    // Diagonal pairs: two selected quads sharing exactly one point.
    let mut pairs = Vec::new();
    let mut paired = BTreeSet::new();
    // Each selected quad's diagonal: the corner it is split through.
    let mut diagonal: BTreeMap<usize, u32> = BTreeMap::new();
    let list: Vec<usize> = chosen.iter().copied().collect();
    for (i, &s1) in list.iter().enumerate() {
        for &s2 in &list[i + 1..] {
            let shared: Vec<u32> = corners(s1)
                .iter()
                .copied()
                .filter(|v| corners(s2).contains(v))
                .collect();
            let [c] = shared[..] else { continue };
            // A 2×2 block round C: exactly four polygons (Q-DR-1).
            let around: BTreeSet<usize> = point_faces.of(c).iter().map(|&f| f as usize).collect();
            if around.len() != 4 || point_faces.of(c).len() != 4 {
                return Err(SELECT_TWO.into());
            }
            let sides: Vec<usize> = around
                .iter()
                .copied()
                .filter(|&f| f != s1 && f != s2)
                .collect();
            let touches = |t: usize, s: usize| {
                let n = |q: usize| neighbours(corners(q), c);
                let (a, b) = n(t);
                let (x, y) = n(s);
                [a, b].iter().any(|v| *v == x || *v == y)
            };
            if sides.len() != 2
                || !sides.iter().all(|&t| {
                    is_quad(t) && !chosen.contains(&t) && touches(t, s1) && touches(t, s2)
                })
            {
                return Err(SELECT_TWO.into());
            }
            for s in [s1, s2] {
                // Split through C or its opposite corner: the same diagonal.
                let key = opposite(corners(s), c).min(c);
                if diagonal.insert(s, key).is_some_and(|k| k != key) {
                    return Err(SELECT_TWO.into());
                }
            }
            paired.insert(s1);
            paired.insert(s2);
            pairs.push(Pair {
                c,
                sides: [sides[0], sides[1]],
            });
        }
    }
    if pairs.is_empty() || paired.len() != chosen.len() {
        return Err(SELECT_TWO.into());
    }
    let on_border = |t: usize| {
        let f = corners(t);
        (0..4).any(|i| edge_counts.get(&edge_key(f[i], f[(i + 1) % 4])) == Some(&1))
    };
    let mut vertices = mesh.vertices.clone();
    // Rebuilt side quads: their three new quads. Consumed triangles are
    // (selected quad, its off-diagonal corner).
    let mut rebuilt: BTreeMap<usize, Vec<OutFace>> = BTreeMap::new();
    let mut consumed: BTreeSet<(usize, u32)> = BTreeSet::new();
    let mut done_corners = Vec::new();
    for pair in &pairs {
        let mut both = true;
        for &t in &pair.sides {
            if on_border(t) {
                both = false;
                continue;
            }
            if rebuilt.contains_key(&t) {
                return Err(SELECT_TWO.into());
            }
            let tv = corners(t);
            let k = tv.iter().position(|&v| v == pair.c).ok_or(SELECT_TWO)?;
            let (q, f, r) = (tv[(k + 1) % 4], tv[(k + 2) % 4], tv[(k + 3) % 4]);
            // The selected quads holding the sides C–q and C–r.
            let holder = |p: u32| {
                chosen.iter().copied().find(|&s| {
                    diagonal.contains_key(&s)
                        && corners(s).contains(&pair.c)
                        && corners(s).contains(&p)
                })
            };
            let (sq, sr) = (holder(q).ok_or(SELECT_TWO)?, holder(r).ok_or(SELECT_TWO)?);
            if !consumed.insert((sq, q)) || !consumed.insert((sr, r)) {
                return Err(SELECT_TWO.into());
            }
            let m = vertices.len() as u32;
            let centre: DVec3 = tv.iter().map(|&v| mesh.vertices[v as usize]).sum::<DVec3>() / 4.;
            vertices.push(centre);
            let at =
                |s: usize, v: u32| Some((s, corners(s).iter().position(|&w| w == v).unwrap_or(0)));
            let centre_blend: Vec<((usize, usize), f64)> = (0..4).map(|i| ((t, i), 0.25)).collect();
            let quad =
                |template: usize, vs: [u32; 4], donors: [Option<(usize, usize)>; 4]| OutFace {
                    template,
                    vertices: vs.to_vec(),
                    donors: vs
                        .iter()
                        .zip(donors)
                        .map(|(&v, d)| if v == m { None } else { d })
                        .collect(),
                    blends: vs
                        .iter()
                        .map(|&v| (v == m).then(|| centre_blend.clone()))
                        .collect(),
                };
            let dq = opposite(corners(sq), pair.c);
            let dr = opposite(corners(sr), pair.c);
            rebuilt.insert(
                t,
                vec![
                    quad(
                        sq,
                        [dq, q, m, pair.c],
                        [at(sq, dq), at(sq, q), None, at(sq, pair.c)],
                    ),
                    quad(
                        sr,
                        [pair.c, m, r, dr],
                        [at(sr, pair.c), None, at(sr, r), at(sr, dr)],
                    ),
                    quad(t, [f, r, m, q], [at(t, f), at(t, r), None, at(t, q)]),
                ],
            );
        }
        if both {
            done_corners.push(pair.c);
        }
    }
    let mut faces = Vec::with_capacity(mesh.faces.len() + 2 * chosen.len() + rebuilt.len());
    for (fi, f) in mesh.faces.iter().enumerate() {
        if let Some(&key) = diagonal.get(&fi) {
            // MT4-04: the two triangles along the diagonal through `key`.
            let k = f
                .vertices
                .iter()
                .position(|&v| v == key)
                .ok_or(SELECT_TWO)?;
            let c = |i: usize| (k + i) % 4;
            for tri in [[c(0), c(1), c(2)], [c(0), c(2), c(3)]] {
                // Each triangle is known by its off-diagonal corner.
                let off = if tri[1] == c(1) {
                    f.vertices[c(1)]
                } else {
                    f.vertices[c(3)]
                };
                if consumed.contains(&(fi, off)) {
                    continue;
                }
                faces.push(OutFace {
                    template: fi,
                    vertices: tri.iter().map(|&i| f.vertices[i]).collect(),
                    donors: tri.iter().map(|&i| Some((fi, i))).collect(),
                    blends: Vec::new(),
                });
            }
        } else if let Some(quads) = rebuilt.remove(&fi) {
            faces.extend(quads);
        } else {
            faces.push(OutFace::copy(mesh, fi, f.vertices.clone()));
        }
    }
    let out = rebuild(mesh, vertices, faces, &[], "Diagonal Re-Direct")?;
    out.validate().map_err(|e| {
        format!("Diagonal Re-Direct would leave an invalid mesh ({e}); nothing changed")
    })?;
    Ok(Redirected {
        mesh: out,
        corners: done_corners,
    })
}

/// The two neighbours of `c` in quad `q` (previous, next).
fn neighbours(q: &[u32], c: u32) -> (u32, u32) {
    let k = q.iter().position(|&v| v == c).unwrap_or(0);
    (q[(k + 3) % 4], q[(k + 1) % 4])
}

/// The corner of quad `q` opposite `c`.
fn opposite(q: &[u32], c: u32) -> u32 {
    let k = q.iter().position(|&v| v == c).unwrap_or(0);
    q[(k + 2) % 4]
}

impl Editor {
    /// Diagonal Re-Direct (MT4): one undo step; afterwards Vertex mode,
    /// with C selected for each pair whose side quads were both rebuilt
    /// (MT4-07).
    pub(crate) fn diagonal_redirect(&mut self) -> Result<()> {
        let item = self.edge_insert_target("Diagonal Re-Direct")?;
        if self.scene.selection.mode != Mode::Polygon {
            return Err(SELECT_TWO.into());
        }
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = diagonal_redirect_geometry(mesh, &self.scene.selection.faces)?;
        let before = self.scene.clone();
        let selection = &mut self.scene.selection;
        selection.mode = Mode::Vertex;
        selection.tag = None;
        selection.edges.clear();
        selection.vertices = out.corners;
        self.publish_polygon_edit_from(item, out.mesh, "Diagonal Re-Direct", Vec::new(), before)
    }
}
