//! Edge Zip (Edit tab, Commands). Spec: missing-tools-g1
//! G1-SMALL-COMMANDS v1, MT3-01..06, cases EZ-A..E.
//!
//! Each selected edge `a–b` lies between two quads. Each of its points is
//! grouped with the point across each quad from it; groups that share a
//! point join, and each group becomes one point at the average of its
//! points (MT3-02/03). The two quads collapse away (MT3-04). Where a run
//! ends inside the mesh, the quads just past the end become triangles
//! fanned round the end point; those are merged back into one polygon
//! (MT3-05, EZ-B, EZ-E).
//!
//! Provisional (Q-EZ-1/Q-EZ-2): an open-border edge, or one with a
//! non-quad neighbour, is refused rather than welded.
use crate::edge_commands::{OutFace, rebuild};
use crate::topology::edge_key;
use crate::{Editor, Mesh, Mode, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// The refusal for edges without a quad on each side (Q-EZ-1/2).
pub const NEEDS_QUADS: &str = "Edge Zip needs edges with a quad on each side";

/// Zip `edges` of `mesh`. Refuses (no change) unless every edge has
/// exactly two neighbouring quads.
pub fn edge_zip_geometry(mesh: &Mesh, edges: &[(u32, u32)]) -> Result<Mesh> {
    if edges.is_empty() {
        return Err(NEEDS_QUADS.into());
    }
    let edge_faces = mesh.edge_faces();
    let n = mesh.vertices.len();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(parent: &mut [u32], v: u32) -> u32 {
        let mut r = v;
        while parent[r as usize] != r {
            r = parent[r as usize];
        }
        let mut v = v;
        while parent[v as usize] != r {
            let next = parent[v as usize];
            parent[v as usize] = r;
            v = next;
        }
        r
    }
    let join = |parent: &mut Vec<u32>, a: u32, b: u32| {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent[ra.max(rb) as usize] = ra.min(rb);
        }
    };
    let mut zipped = BTreeSet::new();
    let mut members = BTreeSet::new();
    for &(a, b) in edges {
        let faces = edge_faces.get(&edge_key(a, b)).ok_or(NEEDS_QUADS)?;
        if faces.len() != 2 || faces[0] == faces[1] {
            return Err(NEEDS_QUADS.into());
        }
        for &q in faces {
            let quad = &mesh.faces[q].vertices;
            if quad.len() != 4 || mesh.is_polyline(q) {
                return Err(NEEDS_QUADS.into());
            }
            zipped.insert(q);
            for (p, other) in [(a, b), (b, a)] {
                let across = across(quad, p, other).ok_or(NEEDS_QUADS)?;
                join(&mut parent, p, across);
                members.insert(p);
                members.insert(across);
            }
        }
    }
    // Groups: representative -> members; one point at their average.
    let mut groups: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &v in &members {
        let r = find(&mut parent, v);
        groups.entry(r).or_default().push(v);
    }
    let mut to: Vec<u32> = (0..n as u32).collect();
    let mut vertices = mesh.vertices.clone();
    let mut gone = BTreeSet::new();
    for (&keep, group) in &groups {
        let sum: DVec3 = group.iter().map(|&v| mesh.vertices[v as usize]).sum();
        vertices[keep as usize] = sum / group.len() as f64;
        for &v in group {
            to[v as usize] = keep;
            if v != keep {
                gone.insert(v);
            }
        }
    }
    // Each polygon with its points moved onto their groups; repeated
    // corners drop out, and anything under three corners goes (MT3-04).
    let mut faces: Vec<OutFace> = Vec::new();
    let mut fans: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if zipped.contains(&fi) {
            continue;
        }
        let polyline = mesh.is_polyline(fi);
        let mut vs: Vec<u32> = Vec::new();
        let mut donors = Vec::new();
        for (c, &v) in f.vertices.iter().enumerate() {
            let w = to[v as usize];
            if vs.last() != Some(&w) {
                vs.push(w);
                donors.push(Some((fi, c)));
            }
        }
        while vs.len() > 1 && !polyline && vs.first() == vs.last() {
            vs.pop();
            donors.pop();
        }
        let distinct: BTreeSet<_> = vs.iter().collect();
        if distinct.len() != vs.len() {
            return Err("Edge Zip would fold a polygon onto itself; nothing changed".into());
        }
        if vs.len() < if polyline { 2 } else { 3 } {
            continue;
        }
        // A quad squashed to a triangle round one group point: a fan
        // candidate at that point (MT3-05).
        if !polyline && f.vertices.len() == 4 && vs.len() == 3 {
            let lost: Vec<u32> = vs
                .iter()
                .copied()
                .filter(|&w| f.vertices.iter().filter(|&&v| to[v as usize] == w).count() == 2)
                .collect();
            if let [p] = lost[..] {
                fans.entry(p).or_default().push(faces.len());
            }
        }
        faces.push(OutFace {
            template: fi,
            vertices: vs,
            donors,
            blends: Vec::new(),
        });
    }
    let faces = merge_fans(faces, &fans);
    let out = rebuild(mesh, vertices, faces, &[], "Edge Zip")?;
    out.validate()
        .map_err(|e| format!("Edge Zip would leave an invalid mesh ({e}); nothing changed"))?;
    let out = crate::mesh_cleanup::remove_unreferenced(&out, Some(&gone))?.mesh;
    Ok(out)
}

/// The quad's other neighbour of `p` (not `other`), when `p`–`other` is
/// one of its sides.
fn across(quad: &[u32], p: u32, other: u32) -> Option<u32> {
    let i = quad.iter().position(|&v| v == p)?;
    let (prev, next) = (quad[(i + 3) % 4], quad[(i + 1) % 4]);
    match (prev == other, next == other) {
        (true, false) => Some(next),
        (false, true) => Some(prev),
        _ => None,
    }
}

/// MT3-05: triangles fanned round one group point `p` that follow each
/// other (one's last side meets the next one's first) merge into one
/// polygon `p, x1, x2, ...`. A fan that closes all the way round stays.
fn merge_fans(mut faces: Vec<OutFace>, fans: &BTreeMap<u32, Vec<usize>>) -> Vec<OutFace> {
    let mut drop = BTreeSet::new();
    let mut merged = Vec::new();
    for (&p, tris) in fans {
        if tris.len() < 2 {
            continue;
        }
        // Each triangle as (p, x, y) in its winding: (corner of x, y).
        let rotated: Vec<(usize, usize)> = tris
            .iter()
            .filter_map(|&t| {
                let c = faces[t].vertices.iter().position(|&v| v == p)?;
                Some((t, c))
            })
            .collect();
        let x_of = |t: usize, c: usize, k: usize| faces[t].vertices[(c + k) % 3];
        let by_x: BTreeMap<u32, (usize, usize)> = rotated
            .iter()
            .map(|&(t, c)| (x_of(t, c, 1), (t, c)))
            .collect();
        let ys: BTreeSet<u32> = rotated.iter().map(|&(t, c)| x_of(t, c, 2)).collect();
        let mut visited = BTreeSet::new();
        // Chains start at a triangle whose first side isn't another's last.
        for &(t0, c0) in &rotated {
            if ys.contains(&x_of(t0, c0, 1)) || visited.contains(&t0) {
                continue;
            }
            let mut chain = vec![(t0, c0)];
            visited.insert(t0);
            let mut y = x_of(t0, c0, 2);
            while let Some(&(t, c)) = by_x.get(&y) {
                if !visited.insert(t) {
                    break;
                }
                chain.push((t, c));
                y = x_of(t, c, 2);
            }
            if chain.len() < 2 {
                continue;
            }
            let (ft, fc) = chain[0];
            let mut vertices = vec![p, x_of(ft, fc, 1)];
            let mut donors = vec![faces[ft].donors[fc], faces[ft].donors[(fc + 1) % 3]];
            for &(t, c) in &chain {
                vertices.push(x_of(t, c, 2));
                donors.push(faces[t].donors[(c + 2) % 3]);
            }
            let template = faces[ft].template;
            merged.push((
                ft,
                OutFace {
                    template,
                    vertices,
                    donors,
                    blends: Vec::new(),
                },
            ));
            drop.extend(chain[1..].iter().map(|&(t, _)| t));
        }
    }
    for (i, face) in merged {
        faces[i] = face;
    }
    faces
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !drop.contains(i))
        .map(|(_, f)| f)
        .collect()
}

impl Editor {
    /// Edge Zip (MT3): one undo step; Edge mode stays, nothing selected
    /// afterwards (MT3-06).
    pub(crate) fn edge_zip(&mut self) -> Result<()> {
        let item = self.edge_insert_target("Edge Zip")?;
        if self.scene.selection.mode != Mode::Edge {
            return Err(format!("{NEEDS_QUADS} (select edges in Edge mode)"));
        }
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = edge_zip_geometry(mesh, &self.scene.selection.edges)?;
        let before = self.scene.clone();
        let selection = &mut self.scene.selection;
        selection.vertices.clear();
        selection.edges.clear();
        self.publish_polygon_edit_from(item, out, "Edge Zip", Vec::new(), before)
    }
}
