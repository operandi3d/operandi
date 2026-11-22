//! RED-004 independent acceptance oracle for the original [0,4]×[0,4] grid.
//! It reads only positions and triangle indices and shares no code, chart or
//! tolerance with the reducer: exact XY plane, shoelace orientation, directed
//! edge pairing, one boundary loop on the square perimeter, explicit
//! vertex-on-edge (T-junction) and pairwise triangle-overlap tests, Euler
//! characteristic and edge connectivity. Its sensitivity is tested against
//! the separately authored valid 24-triangle example and corrupted variants.
#![allow(dead_code)]
use std::collections::{BTreeMap, BTreeSet};

pub const SIDE: f64 = 4.;
pub const AREA: f64 = 16.;
const EPS: f64 = 1e-9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskStats {
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
    pub boundary_edges: usize,
}

fn signed2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn on_side(p: [f64; 2]) -> Option<u8> {
    if p[0].abs() <= EPS {
        Some(0)
    } else if (p[0] - SIDE).abs() <= EPS {
        Some(1)
    } else if p[1].abs() <= EPS {
        Some(2)
    } else if (p[1] - SIDE).abs() <= EPS {
        Some(3)
    } else {
        None
    }
}

fn sides(p: [f64; 2]) -> BTreeSet<u8> {
    let mut s = BTreeSet::new();
    if p[0].abs() <= EPS {
        s.insert(0);
    }
    if (p[0] - SIDE).abs() <= EPS {
        s.insert(1);
    }
    if p[1].abs() <= EPS {
        s.insert(2);
    }
    if (p[1] - SIDE).abs() <= EPS {
        s.insert(3);
    }
    s
}

/// Strict interior overlap of two counter-clockwise triangles by separating
/// axes. Touching along an edge or at a point is not overlap.
fn interiors_overlap(t: [[f64; 2]; 3], u: [[f64; 2]; 3]) -> bool {
    for tri in [t, u] {
        for k in 0..3 {
            let (a, b) = (tri[k], tri[(k + 1) % 3]);
            let axis = [b[1] - a[1], a[0] - b[0]];
            let project = |p: [f64; 2]| p[0] * axis[0] + p[1] * axis[1];
            let (t0, t1) = t
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &p| {
                    (lo.min(project(p)), hi.max(project(p)))
                });
            let (u0, u1) = u
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &p| {
                    (lo.min(project(p)), hi.max(project(p)))
                });
            if t1 <= u0 + EPS || u1 <= t0 + EPS {
                return false;
            }
        }
    }
    true
}

/// Accept only one valid triangulated disk exactly covering the square.
pub fn check_square_disk(positions: &[[f64; 3]], faces: &[Vec<u32>]) -> Result<DiskStats, String> {
    if faces.is_empty() {
        return Err("no faces".into());
    }
    if positions.iter().any(|p| p[2] != 0.) {
        return Err("a point leaves the Z=0 plane".into());
    }
    let p = |v: u32| [positions[v as usize][0], positions[v as usize][1]];
    let mut used = vec![false; positions.len()];
    let mut area = 0.;
    let mut directed = BTreeSet::new();
    for (fi, f) in faces.iter().enumerate() {
        if f.len() != 3 || f.iter().any(|&v| v as usize >= positions.len()) {
            return Err(format!("face {fi} is not a valid triangle"));
        }
        if f[0] == f[1] || f[1] == f[2] || f[0] == f[2] {
            return Err(format!("face {fi} repeats a point"));
        }
        let s = signed2(p(f[0]), p(f[1]), p(f[2]));
        if s <= EPS {
            return Err(format!("face {fi} is inverted or has zero area about +Z"));
        }
        area += s / 2.;
        for k in 0..3 {
            used[f[k] as usize] = true;
            if !directed.insert((f[k], f[(k + 1) % 3])) {
                return Err("duplicate directed edge (nonmanifold or inconsistent)".into());
            }
        }
    }
    if used.iter().any(|u| !u) {
        return Err("unused point".into());
    }
    if (area - AREA).abs() > 1e-9 {
        return Err(format!("covered area {area} != {AREA}"));
    }
    for v in 0..positions.len() {
        let q = p(v as u32);
        if q[0] < -EPS || q[0] > SIDE + EPS || q[1] < -EPS || q[1] > SIDE + EPS {
            return Err("point outside the square".into());
        }
    }
    for corner in [[0., 0.], [SIDE, 0.], [SIDE, SIDE], [0., SIDE]] {
        if !(0..positions.len()).any(|v| p(v as u32) == corner) {
            return Err(format!("corner {corner:?} missing"));
        }
    }
    let boundary: Vec<(u32, u32)> = directed
        .iter()
        .copied()
        .filter(|&(a, b)| !directed.contains(&(b, a)))
        .collect();
    let mut perimeter = 0.;
    let mut next = BTreeMap::new();
    for &(a, b) in &boundary {
        let common: BTreeSet<u8> = sides(p(a)).intersection(&sides(p(b))).copied().collect();
        if common.is_empty() || on_side(p(a)).is_none() {
            return Err("boundary edge leaves the square perimeter (crack or hole)".into());
        }
        let d = [p(b)[0] - p(a)[0], p(b)[1] - p(a)[1]];
        perimeter += (d[0] * d[0] + d[1] * d[1]).sqrt();
        if next.insert(a, b).is_some() {
            return Err("boundary pinch".into());
        }
    }
    if (perimeter - 4. * SIDE).abs() > 1e-9 {
        return Err(format!("boundary length {perimeter} != perimeter"));
    }
    let start = boundary[0].0;
    let mut loop_len = 1;
    let mut v = next[&start];
    while v != start {
        v = *next.get(&v).ok_or("open boundary")?;
        loop_len += 1;
        if loop_len > boundary.len() {
            return Err("malformed boundary".into());
        }
    }
    if loop_len != boundary.len() {
        return Err("more than one boundary loop".into());
    }
    let edges: BTreeSet<(u32, u32)> = directed
        .iter()
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    // T-junction: no point strictly inside an edge that does not use it.
    for &(a, b) in &edges {
        let (pa, pb) = (p(a), p(b));
        for v in 0..positions.len() as u32 {
            if v == a || v == b {
                continue;
            }
            let q = p(v);
            let cross = signed2(pa, pb, q);
            let len2 = (pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2);
            let t = ((q[0] - pa[0]) * (pb[0] - pa[0]) + (q[1] - pa[1]) * (pb[1] - pa[1])) / len2;
            if cross.abs() <= EPS * len2.sqrt() && t > EPS && t < 1. - EPS {
                return Err(format!("point {v} lies inside edge {a}-{b} (T-junction)"));
            }
        }
    }
    for i in 0..faces.len() {
        for j in i + 1..faces.len() {
            let t = [p(faces[i][0]), p(faces[i][1]), p(faces[i][2])];
            let u = [p(faces[j][0]), p(faces[j][1]), p(faces[j][2])];
            if interiors_overlap(t, u) {
                return Err(format!("faces {i} and {j} overlap"));
            }
        }
    }
    // Edge-connected faces.
    let mut by_edge: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (fi, f) in faces.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(fi);
        }
    }
    let mut seen = vec![false; faces.len()];
    let mut pending = vec![0usize];
    seen[0] = true;
    while let Some(fi) = pending.pop() {
        for k in 0..3 {
            let (a, b) = (faces[fi][k], faces[fi][(k + 1) % 3]);
            for &g in &by_edge[&(a.min(b), a.max(b))] {
                if !seen[g] {
                    seen[g] = true;
                    pending.push(g);
                }
            }
        }
    }
    if seen.iter().any(|s| !s) {
        return Err("faces are not edge-connected".into());
    }
    let euler = positions.len() as i64 - edges.len() as i64 + faces.len() as i64;
    if euler != 1 {
        return Err(format!("Euler characteristic {euler} != 1"));
    }
    Ok(DiskStats {
        vertices: positions.len(),
        edges: edges.len(),
        faces: faces.len(),
        boundary_edges: boundary.len(),
    })
}
