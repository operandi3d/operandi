//! ME-T05 Slide (edges), measured in specifications/add-loop-edge-slide-2026-11-02.
//!
//! Each connected chain of selected edges is oriented consistently; for a
//! positive amount every chain point moves along its "rail" (the connected
//! unselected edge) on the chain's left side, for a negative amount along the
//! rail on the right side (ES-02; the general side rule is open, ES-Q01). A
//! point with no rail on that side (a border) uses the other side (ES-06).
//! The amount is a fraction of each rail (Percent, ES-01) or a distance along
//! it (ES-03), and is not clamped: points may pass the far end (ES-05). A
//! point that lands exactly on its rail's far end merges with it (ES-04).
//!
//! Tool-audit Tier 1 options ([`SlideExtras`], shared with Vertex Slide; all
//! provisional, TA1-Q04..Q06): Stop Negative Slide, Loop Slide (Vertex Slide
//! only), Preserve Curvature with Tension, and Slip UVs.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// Slide options added by the tool audit (defaults match the reference's
/// recorded defaults, ES-D-01).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideExtras {
    /// Clamp the amount to 0..1 of each rail: negative slides do nothing
    /// and no point passes its rail's far end.
    pub stop_negative: bool,
    /// Vertex Slide: keep each point within its edge (±100 %). Off lets the
    /// point carry on past the end along the edge's line. Edge Slide follows
    /// ES-05 (never clamped) whatever this says.
    pub loop_slide: bool,
    /// Points follow a curve through the rail's neighbours instead of the
    /// straight rail (the same curve as Add Loop's).
    pub preserve_curvature: bool,
    /// Curve tangent scale, 1.0 = 100 %.
    pub tension: f64,
    /// Slid points take UVs interpolated along the rail so the texture
    /// stays put. Applies to every UV map of the mesh.
    pub slip_uvs: bool,
}
impl Default for SlideExtras {
    fn default() -> Self {
        Self {
            stop_negative: false,
            loop_slide: true,
            preserve_curvature: false,
            tension: 1.,
            slip_uvs: false,
        }
    }
}
impl SlideExtras {
    pub(crate) fn check(&self) -> Result<()> {
        if !self.tension.is_finite() || !(0. ..=10.).contains(&self.tension) {
            return Err("Tension must be between 0% and 1000%".into());
        }
        Ok(())
    }

    /// Where point `v` lands sliding `fraction` of the way to `target`.
    pub(crate) fn position(&self, mesh: &Mesh, v: u32, target: u32, fraction: f64) -> DVec3 {
        if self.preserve_curvature {
            crate::edge_insert::curve_point(mesh, v, target, fraction, self.tension)
        } else {
            let (p, q) = (mesh.vertices[v as usize], mesh.vertices[target as usize]);
            p + (q - p) * fraction
        }
    }
}

/// Slip UVs: give each slid point `(v, target, fraction)` the UVs the
/// surface had where it now sits. In a polygon that holds the rail the
/// corner moves along the rail's UVs; in a polygon on the other side it
/// carries on along the edge that points away from the rail, scaled by
/// the two edges' lengths. Polygons that hold neither keep their UVs.
pub(crate) fn slip_uvs(out: &mut Mesh, source: &Mesh, moves: &[(u32, u32, f64)]) {
    let point_faces = source.point_faces();
    for &(v, target, fraction) in moves {
        let pv = source.vertices[v as usize];
        let rail = source.vertices[target as usize] - pv;
        for &f in point_faces.of(v) {
            let f = f as usize;
            let vs = &source.faces[f].vertices;
            let n = vs.len();
            let Some(i) = vs.iter().position(|&x| x == v) else {
                continue;
            };
            // (corner to blend from, weight): uv_i + (uv_k - uv_i) * w.
            let blend = if let Some(j) = vs.iter().position(|&x| x == target) {
                Some((j, fraction))
            } else {
                [(i + 1) % n, (i + n - 1) % n]
                    .into_iter()
                    .map(|k| (k, source.vertices[vs[k] as usize] - pv))
                    .filter(|(_, d)| d.dot(rail) < 0.)
                    .min_by(|a, b| {
                        let cos = |d: DVec3| d.normalize_or_zero().dot(rail.normalize_or_zero());
                        cos(a.1).total_cmp(&cos(b.1))
                    })
                    .map(|(k, d)| {
                        (
                            k,
                            -fraction * rail.length() / d.length().max(f64::MIN_POSITIVE),
                        )
                    })
            };
            let Some((k, w)) = blend else {
                continue;
            };
            let mix = |a: DVec2, b: DVec2| a + (b - a) * w;
            if let (Some(uv), Some(old)) = (out.faces[f].uv.as_mut(), source.faces[f].uv.as_ref()) {
                uv[i] = mix(old[i], old[k]);
            }
            for (name, map) in &source.uv_maps {
                if let (Some(rows), Some(old)) = (out.uv_maps.get_mut(name), map.get(f))
                    && let (Some(a), Some(b)) = (old[i], old[k])
                {
                    rows[f][i] = Some(mix(a, b));
                }
            }
        }
    }
}

/// Slide the selected edges of `mesh` by `amount` of each rail (Percent).
pub fn edge_slide_geometry(mesh: &Mesh, edges: &[(u32, u32)], amount: f64) -> Result<Mesh> {
    edge_slide_with(mesh, edges, amount, false).map(|(m, _)| m)
}

/// Slide by `amount`, a fraction of each rail or, with `by_distance`, a
/// distance along it. Returns the mesh and the selected edges after (they
/// are renumbered when points merge).
pub fn edge_slide_with(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    amount: f64,
    by_distance: bool,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    edge_slide_weighted(mesh, edges, amount, by_distance, None)
}

/// Slide with a falloff: each point's amount is scaled by its weight
/// (FAL-B3-04).
pub fn edge_slide_weighted(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    amount: f64,
    by_distance: bool,
    weights: Option<&[f64]>,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    edge_slide_extra(
        mesh,
        edges,
        amount,
        by_distance,
        weights,
        &SlideExtras::default(),
    )
}

/// Slide with the tool-audit options ([`SlideExtras`]).
pub fn edge_slide_extra(
    mesh: &Mesh,
    edges: &[(u32, u32)],
    amount: f64,
    by_distance: bool,
    weights: Option<&[f64]>,
    extras: &SlideExtras,
) -> Result<(Mesh, Vec<(u32, u32)>)> {
    mesh.validate()?;
    extras.check()?;
    if !amount.is_finite() || amount.abs() > 1e6 {
        return Err("Slide amount must be a finite number".into());
    }
    let selected: BTreeSet<(u32, u32)> = edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    if selected.is_empty() {
        return Err("Select edges to Slide".into());
    }
    let existing: BTreeSet<(u32, u32)> = mesh.edges().into_iter().collect();
    if !selected.is_subset(&existing) {
        return Err("A selected edge no longer exists".into());
    }
    if amount == 0. || (extras.stop_negative && amount < 0.) {
        return Ok((mesh.clone(), selected.into_iter().collect()));
    }
    // Directed edge -> face that traverses it.
    let mut face_of: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(fi) {
            continue;
        }
        let n = f.vertices.len();
        for i in 0..n {
            face_of.insert((f.vertices[i], f.vertices[(i + 1) % n]), fi);
        }
    }
    // Orient each chain: walk from an endpoint (or anywhere on a loop).
    let mut adjacency: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &(a, b) in &selected {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    let mut directed: Vec<(u32, u32)> = Vec::new();
    let mut used: BTreeSet<(u32, u32)> = BTreeSet::new();
    let starts: Vec<u32> = adjacency
        .iter()
        .filter(|(_, n)| n.len() != 2)
        .map(|(&v, _)| v)
        .chain(adjacency.keys().copied())
        .collect();
    for start in starts {
        let mut v = start;
        while let Some(&w) = adjacency[&v]
            .iter()
            .find(|&&w| !used.contains(&edge_key(v, w)))
        {
            used.insert(edge_key(v, w));
            directed.push((v, w));
            v = w;
        }
    }
    // For each chain point, the rail target on each side.
    let mut left: BTreeMap<u32, u32> = BTreeMap::new();
    let mut right: BTreeMap<u32, u32> = BTreeMap::new();
    let rail = |face: usize, v: u32, along: u32| -> Option<u32> {
        let f = &mesh.faces[face].vertices;
        let i = f.iter().position(|&x| x == v)?;
        let n = f.len();
        let (prev, next) = (f[(i + n - 1) % n], f[(i + 1) % n]);
        let other = if prev == along { next } else { prev };
        (!selected.contains(&edge_key(v, other))).then_some(other)
    };
    for &(u, w) in &directed {
        // Left face traverses u→w; right face traverses w→u.
        if let Some(&fl) = face_of.get(&(u, w)) {
            for (v, along) in [(u, w), (w, u)] {
                if let Some(t) = rail(fl, v, along) {
                    left.entry(v).or_insert(t);
                }
            }
        }
        if let Some(&fr) = face_of.get(&(w, u)) {
            for (v, along) in [(u, w), (w, u)] {
                if let Some(t) = rail(fr, v, along) {
                    right.entry(v).or_insert(t);
                }
            }
        }
    }
    let (side, other) = if amount > 0. {
        (&left, &right)
    } else {
        (&right, &left)
    };
    let points: BTreeSet<u32> = selected.iter().flat_map(|&(a, b)| [a, b]).collect();
    let mut out = mesh.clone();
    let mut merges = Vec::new();
    let mut moves = Vec::new();
    for &v in &points {
        let Some(&t) = side.get(&v).or_else(|| other.get(&v)) else {
            continue;
        };
        let (p, q) = (mesh.vertices[v as usize], mesh.vertices[t as usize]);
        let length = p.distance(q);
        let share = weights.map_or(1., |w| w[v as usize]);
        let mut fraction = share
            * if by_distance {
                amount.abs() / length.max(f64::MIN_POSITIVE)
            } else {
                amount.abs()
            };
        if extras.stop_negative {
            fraction = fraction.clamp(0., 1.);
        }
        out.vertices[v as usize] = extras.position(mesh, v, t, fraction);
        moves.push((v, t, fraction));
        if (fraction - 1.).abs() <= 1e-9 {
            merges.push((vec![v, t], q));
        }
    }
    if extras.slip_uvs {
        slip_uvs(&mut out, mesh, &moves);
    }
    if merges.is_empty() {
        out.validate()
            .map_err(|e| format!("Slide produced an invalid mesh ({e}); reduce the amount"))?;
        return Ok((out, selected.into_iter().collect()));
    }
    let (merged, remap) = crate::edge_commands::weld(&out, &merges, "Slide")?;
    let after: BTreeSet<(u32, u32)> = selected
        .iter()
        .filter_map(|&(a, b)| Some(edge_key(remap[a as usize]?, remap[b as usize]?)))
        .filter(|&(a, b)| a != b)
        .collect();
    Ok((merged, after.into_iter().collect()))
}
