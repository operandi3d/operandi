//! ME-T06 Edge Relax.
//!
//! Edge Relax changes the radius of bevelled edges after
//! the fact: the user selects the bevel's edges, including an edge on either
//! side, and drags Offset. Basic Operandi profile: the selected edges are the
//! parallel rows of a bevel strip. Every chain of points running across the
//! strip (from one outer selected edge to the other) is rebuilt: its two ends
//! slide along their side edges so the outer edges move Offset in parallel
//! (positive widens), and the points between are spread evenly along a round
//! arc whose corner is where the two sides meet. Only positions change.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Relax the bevel described by the selected row `edges` by `offset`.
pub fn edge_relax_geometry(mesh: &Mesh, edges: &[(u32, u32)], offset: f64) -> Result<Mesh> {
    mesh.validate()?;
    if !offset.is_finite() {
        return Err("Edge Relax Offset must be finite".into());
    }
    let chosen: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    // Strip quads: two opposite edges selected; the others cross the strip.
    let mut strips = BTreeSet::new();
    let mut cross: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let v = &f.vertices;
        if v.len() != 4 || mesh.is_polyline(fi) {
            continue;
        }
        for s in 0..2 {
            let e = |k: usize| (v[(s + k) % 4], v[(s + k + 1) % 4]);
            let (r0, c0, r1, c1) = (e(0), e(1), e(2), e(3));
            if chosen.contains(&edge_key(r0.0, r0.1))
                && chosen.contains(&edge_key(r1.0, r1.1))
                && !chosen.contains(&edge_key(c0.0, c0.1))
                && !chosen.contains(&edge_key(c1.0, c1.1))
            {
                strips.insert(fi);
                for (x, y) in [c0, c1] {
                    let list = cross.entry(x).or_default();
                    if !list.contains(&y) {
                        list.push(y);
                    }
                    let list = cross.entry(y).or_default();
                    if !list.contains(&x) {
                        list.push(x);
                    }
                }
            }
        }
    }
    if strips.is_empty() {
        return Err("Edge Relax: select a bevel's edges, including the edge on either side".into());
    }
    if cross.values().any(|n| n.len() > 2) {
        return Err("Edge Relax basic supports simple strips".into());
    }
    let p = |v: u32| mesh.vertices[v as usize];
    let in_strip = |x: u32, y: u32| {
        strips.iter().any(|&f| {
            let vs = &mesh.faces[f].vertices;
            (0..4).any(|i| edge_key(vs[i], vs[(i + 1) % 4]) == edge_key(x, y))
        })
    };
    // Side edge leaving an outer point; returns (direction, along-row axis).
    let side = |u: u32| -> Result<(DVec3, DVec3, f64)> {
        let mut found = None;
        let mut row = None;
        for (a, b) in mesh.edges() {
            let w = if a == u {
                b
            } else if b == u {
                a
            } else {
                continue;
            };
            if chosen.contains(&edge_key(u, w)) {
                row = Some(w);
            } else if !in_strip(u, w) {
                if found.is_some_and(|x| x != w) {
                    return Err("Edge Relax needs one side edge at each outer point".to_string());
                }
                found = Some(w);
            }
        }
        let w = found.ok_or("Edge Relax needs one side edge at each outer point")?;
        let r = row.ok_or("Edge Relax needs one side edge at each outer point")?;
        let along = p(w) - p(u);
        let len = along.length();
        let dir = along / len;
        let sine = dir.cross((p(r) - p(u)).normalize()).length();
        if len < 1e-12 || sine < 1e-6 {
            return Err("Degenerate Edge Relax side".into());
        }
        Ok((dir, dir / sine, len))
    };
    let mut out = mesh.clone();
    let mut seen = BTreeSet::new();
    let ends: Vec<u32> = cross
        .iter()
        .filter(|(_, n)| n.len() == 1)
        .map(|(&v, _)| v)
        .collect();
    for start in ends {
        if seen.contains(&start) {
            continue;
        }
        let mut chain = vec![start];
        let mut prev = None;
        let mut v = start;
        while let Some(&w) = cross[&v].iter().find(|&&w| Some(w) != prev) {
            chain.push(w);
            prev = Some(v);
            v = w;
            if cross[&v].len() == 1 {
                break;
            }
        }
        seen.extend(chain.iter().copied());
        let (first, last) = (chain[0], *chain.last().unwrap());
        let (d0, m0, l0) = side(first)?;
        let (d1, m1, l1) = side(last)?;
        if offset > 0. && (offset * m0.length() >= l0 - 1e-9 || offset * m1.length() >= l1 - 1e-9) {
            return Err("Edge Relax Offset reaches the far end of a side edge".into());
        }
        let q0 = p(first) + m0 * offset;
        let q1 = p(last) + m1 * offset;
        if (q1 - q0).dot(p(last) - p(first)) <= 0. {
            return Err("Edge Relax Offset closes the bevel".into());
        }
        // Corner: where the two sides, extended past the bevel, meet.
        let (u, w) = (-d0, -d1);
        let r = q0 - q1;
        let b = u.dot(w);
        let den = 1. - b * b;
        let control = if den > 1e-9 {
            let t = (b * w.dot(r) - u.dot(r)) / den;
            let s = (w.dot(r) - b * u.dot(r)) / den;
            Some((q0 + u * t + q1 + w * s) * 0.5)
        } else {
            None
        };
        let n = chain.len() - 1;
        out.vertices[first as usize] = q0;
        out.vertices[last as usize] = q1;
        for (k, &v) in chain.iter().enumerate().take(n).skip(1) {
            let t = k as f64 / n as f64;
            let s = 1. - t;
            out.vertices[v as usize] = match control {
                Some(c) => {
                    let (a, b) = ((q0 - c).normalize_or_zero(), (q1 - c).normalize_or_zero());
                    let weight = ((1. + a.dot(b)).max(0.) * 0.5).sqrt();
                    (q0 * s * s + c * (2. * weight * s * t) + q1 * t * t)
                        / (s * s + 2. * weight * s * t + t * t)
                }
                None => q0.lerp(q1, t),
            };
        }
    }
    out.validate()
        .map_err(|e| format!("Edge Relax produced an invalid mesh ({e}); nothing changed"))?;
    Ok(out)
}
