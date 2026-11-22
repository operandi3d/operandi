//! Poly Haul, Offset mode (the tool's default mode; spec
//! `poly-haul-2026-11-05`, v0: modes and defaults only).
//!
//! Offset pushes and pulls the selected polygons along
//! their normal axis; unlike Extrude it creates no new edge loops. Dragging
//! away from the model moves the polygons outwards and dragging toward it
//! moves them inwards, so the offset is signed.
//!
//! Rule: the selected surface polygons are grouped into connected pieces
//! (pieces that share a point are one piece). Each piece moves rigidly along
//! the normalized sum of its polygons' area vectors, so a flat patch moves
//! along its normal and neighbouring polygons stretch to follow. Topology,
//! tags and every attribute are unchanged; only point positions move.
//!
//! Provisional (Q-PH-01, every mode's geometry is open): the per-piece
//! average direction and the viewport drag gain. Reset sets Offset 0 (PH-D0).
//! The other eight modes, the mode list and the boolean union are not built.
use crate::{Mesh, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Offset the selected polygons of `source` by `offset` along their normal.
pub fn poly_haul_geometry(source: &Mesh, selected: &[u32], offset: f64) -> Result<Mesh> {
    source.validate()?;
    if !offset.is_finite() {
        return Err("Poly Haul Offset must be finite".into());
    }
    let faces: BTreeSet<u32> = selected.iter().copied().collect();
    if faces.is_empty() {
        return Err("Poly Haul requires selected polygons".into());
    }
    for &f in &faces {
        if f as usize >= source.faces.len() {
            return Err("Invalid Poly Haul selection".into());
        }
        if source.is_polyline(f as usize) {
            return Err("Poly Haul supports surface polygons only".into());
        }
    }
    if offset == 0. {
        return Ok(source.clone());
    }

    // Connected pieces through shared points (union-find over points).
    let mut parent: BTreeMap<u32, u32> = BTreeMap::new();
    fn root(parent: &mut BTreeMap<u32, u32>, v: u32) -> u32 {
        let p = parent[&v];
        if p == v {
            return v;
        }
        let r = root(parent, p);
        parent.insert(v, r);
        r
    }
    for &f in &faces {
        let vs = &source.faces[f as usize].vertices;
        for &v in vs {
            parent.entry(v).or_insert(v);
        }
        for w in vs.windows(2) {
            let (a, b) = (root(&mut parent, w[0]), root(&mut parent, w[1]));
            if a != b {
                parent.insert(a.max(b), a.min(b));
            }
        }
    }
    let mut direction: BTreeMap<u32, DVec3> = BTreeMap::new();
    for &f in &faces {
        let vs = &source.faces[f as usize].vertices;
        let r = root(&mut parent, vs[0]);
        let area = (0..vs.len()).fold(DVec3::ZERO, |n, i| {
            let (p, q) = (vs[i] as usize, vs[(i + 1) % vs.len()] as usize);
            n + source.vertices[p].cross(source.vertices[q])
        });
        *direction.entry(r).or_default() += area;
    }

    let mut out = source.clone();
    let points: Vec<u32> = parent.keys().copied().collect();
    for v in points {
        let r = root(&mut parent, v);
        let normal = direction[&r].normalize_or_zero();
        if normal == DVec3::ZERO {
            return Err(
                "Poly Haul cannot find a normal for a selected piece (it folds back on itself)"
                    .into(),
            );
        }
        let p = source.vertices[v as usize] + normal * offset;
        if !p.is_finite() {
            return Err("Poly Haul offset exceeds numeric range".into());
        }
        out.vertices[v as usize] = p;
    }
    out.validate()
        .map_err(|e| format!("Poly Haul result is invalid ({e}); try a smaller offset"))?;
    Ok(out)
}
