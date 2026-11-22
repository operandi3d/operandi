//! Atlas and Atlas 2 projections (UVT-16/17): split the surface into charts
//! automatically, flatten each one, pack them in the tile keeping their
//! relative 3D sizes.
//!
//! Charting follows the clustering approach xatlas uses (D-Charts style
//! region growing, Julius, Kraevoy and Sheffer 2005, as xatlas's
//! `ClusteredCharts`): charts grow from seed polygons in order of a cost
//! that weighs how far a polygon turns from the chart's mean normal against
//! how round the chart stays, polygons over the cost limit start new
//! charts, and seeds then move to their chart's middle and the charts are
//! grown again (Lloyd iterations). This is Operandi's own implementation of
//! that method (written from the papers' description, no xatlas code), so
//! it carries no third-party notice. Flattening and packing reuse the
//! Blender port in `operandi-uv` (ENGINES.md).
//!
//! - **Atlas**: a tight normal limit and no target count: many small charts,
//!   "very broken up", for baking.
//! - **Atlas 2**: about Segments charts, regrown Iterations times; fewer,
//!   better shaped charts grouped by facing; Orientation is always on.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVA-P01..P04): the cost
//! weights and limits below, Segments 8 and Iterations 4 to start, and
//! Orientation turning each chart so world up runs up V.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use operandi_uv as uv;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// UVA-P01: Atlas's normal limit: a polygon may turn at most this far from
/// its chart's mean normal (cosine of about 35°).
pub const ATLAS_MIN_DOT: f64 = 0.82;
/// Atlas 2's looser limit (about 70°), as its charts are fewer.
pub const ATLAS2_MIN_DOT: f64 = 0.34;
/// The roundness term's weight against normal deviation.
pub const ROUNDNESS_WEIGHT: f64 = 0.15;

/// Atlas options (UVT §2.3 Atlas rows).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AtlasChartOptions {
    /// Atlas 2 (fewer charts, `segments` and `iterations` apply).
    pub fewer: bool,
    pub segment_by_material: bool,
    pub segment_by_part: bool,
    /// Turn each chart so world up runs up V, so charts of both sides read
    /// alike (always on for Atlas 2; UVA-P03).
    pub orientation: bool,
    /// Atlas 2: roughly how many charts.
    pub segments: u32,
    /// Atlas 2: how many times charts are regrown from their middles.
    pub iterations: u32,
    /// Relative Gap Size, as Unwrap's.
    pub relative_gap: f64,
}
impl Default for AtlasChartOptions {
    fn default() -> Self {
        Self {
            fewer: false,
            segment_by_material: false,
            segment_by_part: false,
            orientation: false,
            segments: 8,
            iterations: 4,
            relative_gap: 0.2,
        }
    }
}

#[derive(PartialEq)]
struct Candidate {
    cost: f64,
    face: usize,
    chart: usize,
}
impl Eq for Candidate {}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // A min-heap on cost.
        other
            .cost
            .total_cmp(&self.cost)
            .then(other.face.cmp(&self.face))
            .then(other.chart.cmp(&self.chart))
    }
}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The chart of each of `faces` (indices into `faces`).
pub fn charts(mesh: &Mesh, faces: &[u32], o: &AtlasChartOptions) -> Vec<usize> {
    let n = faces.len();
    if n == 0 {
        return Vec::new();
    }
    let normal: Vec<DVec3> = faces.iter().map(|&f| mesh.normal(f as usize)).collect();
    let centre: Vec<DVec3> = faces
        .iter()
        .map(|&f| {
            let v = &mesh.faces[f as usize].vertices;
            v.iter().map(|&p| mesh.vertices[p as usize]).sum::<DVec3>() / v.len() as f64
        })
        .collect();
    let area: Vec<f64> = faces
        .iter()
        .map(|&f| {
            let v: Vec<DVec3> = mesh.faces[f as usize]
                .vertices
                .iter()
                .map(|&p| mesh.vertices[p as usize])
                .collect();
            let mut s = DVec3::ZERO;
            for i in 1..v.len().saturating_sub(1) {
                s += (v[i] - v[0]).cross(v[i + 1] - v[0]);
            }
            (s.length() * 0.5).max(1e-18)
        })
        .collect();
    // Neighbours across edges, unless a material / part border blocks them.
    let mut by_edge: std::collections::BTreeMap<(u32, u32), Vec<usize>> = Default::default();
    for (k, &f) in faces.iter().enumerate() {
        let v = &mesh.faces[f as usize].vertices;
        for i in 0..v.len() {
            by_edge
                .entry(edge_key(v[i], v[(i + 1) % v.len()]))
                .or_default()
                .push(k);
        }
    }
    let mut next = vec![Vec::new(); n];
    for list in by_edge.values() {
        for &a in list {
            for &b in list {
                if a == b {
                    continue;
                }
                let (fa, fb) = (
                    &mesh.faces[faces[a] as usize],
                    &mesh.faces[faces[b] as usize],
                );
                if (o.segment_by_material && fa.material != fb.material)
                    || (o.segment_by_part && fa.part != fb.part)
                {
                    continue;
                }
                next[a].push(b);
            }
        }
    }
    let total: f64 = area.iter().sum();
    let min_dot = if o.fewer {
        ATLAS2_MIN_DOT
    } else {
        ATLAS_MIN_DOT
    };
    // Seeds: Atlas 2 starts from `segments` polygons spread by facing (the
    // largest polygon, then each next the one turning furthest from every
    // seed so far); Atlas starts from the largest polygon only.
    let largest = (0..n)
        .max_by(|&a, &b| area[a].total_cmp(&area[b]))
        .unwrap_or(0);
    let mut seeds = vec![largest];
    if o.fewer {
        while seeds.len() < (o.segments.max(1) as usize).min(n) {
            let far = (0..n).filter(|f| !seeds.contains(f)).min_by(|&a, &b| {
                let near = |f: usize| {
                    seeds
                        .iter()
                        .map(|&s| normal[f].dot(normal[s]))
                        .fold(f64::MIN, f64::max)
                };
                near(a).total_cmp(&near(b))
            });
            match far {
                Some(f) => seeds.push(f),
                None => break,
            }
        }
    }
    let rounds = if o.fewer { o.iterations.max(1) } else { 2 };
    let mut chart = vec![usize::MAX; n];
    for round in 0..rounds {
        chart.fill(usize::MAX);
        let mut sum_normal: Vec<DVec3> = Vec::new();
        let mut sum_area: Vec<f64> = Vec::new();
        let mut seed_centre: Vec<DVec3> = Vec::new();
        let mut heap = BinaryHeap::new();
        let start_chart = |s: usize,
                           chart: &mut [usize],
                           heap: &mut BinaryHeap<Candidate>,
                           sum_normal: &mut Vec<DVec3>,
                           sum_area: &mut Vec<f64>,
                           seed_centre: &mut Vec<DVec3>| {
            let c = sum_normal.len();
            chart[s] = c;
            sum_normal.push(normal[s] * area[s]);
            sum_area.push(area[s]);
            seed_centre.push(centre[s]);
            for &m in &next[s] {
                heap.push(Candidate {
                    cost: 0.,
                    face: m,
                    chart: c,
                });
            }
        };
        for &s in &seeds {
            if chart[s] == usize::MAX {
                start_chart(
                    s,
                    &mut chart,
                    &mut heap,
                    &mut sum_normal,
                    &mut sum_area,
                    &mut seed_centre,
                );
            }
        }
        loop {
            while let Some(Candidate { face, chart: c, .. }) = heap.pop() {
                if chart[face] != usize::MAX {
                    continue;
                }
                let mean = sum_normal[c].normalize_or_zero();
                let dot = normal[face].dot(mean);
                if dot < min_dot {
                    continue;
                }
                chart[face] = c;
                sum_normal[c] += normal[face] * area[face];
                sum_area[c] += area[face];
                let mean = sum_normal[c].normalize_or_zero();
                let radius = sum_area[c].sqrt().max(1e-9);
                for &m in &next[face] {
                    if chart[m] == usize::MAX {
                        let deviation = 1. - normal[m].dot(mean);
                        let spread = centre[m].distance(seed_centre[c]) / radius;
                        heap.push(Candidate {
                            cost: deviation + ROUNDNESS_WEIGHT * spread,
                            face: m,
                            chart: c,
                        });
                    }
                }
            }
            // Polygons no chart took start charts of their own.
            match (0..n)
                .filter(|&f| chart[f] == usize::MAX)
                .max_by(|&a, &b| area[a].total_cmp(&area[b]))
            {
                Some(s) => start_chart(
                    s,
                    &mut chart,
                    &mut heap,
                    &mut sum_normal,
                    &mut sum_area,
                    &mut seed_centre,
                ),
                None => break,
            }
        }
        if round + 1 == rounds {
            break;
        }
        // Lloyd: each chart's new seed is its polygon nearest the chart's
        // middle, facing most like it.
        let count = sum_normal.len();
        let mut middle = vec![DVec3::ZERO; count];
        for f in 0..n {
            middle[chart[f]] += centre[f] * area[f];
        }
        let mut best: Vec<(f64, usize)> = vec![(f64::MAX, 0); count];
        for f in 0..n {
            let c = chart[f];
            let m = middle[c] / sum_area[c];
            let score = centre[f].distance(m) / sum_area[c].sqrt().max(1e-9)
                + (1. - normal[f].dot(sum_normal[c].normalize_or_zero()));
            if score < best[c].0 {
                best[c] = (score, f);
            }
        }
        // Atlas 2 keeps to about `segments` charts: the largest ones seed
        // the next round.
        let mut order: Vec<usize> = (0..count).collect();
        order.sort_by(|&a, &b| sum_area[b].total_cmp(&sum_area[a]));
        if o.fewer {
            order.truncate((o.segments.max(1) as usize).max(1));
        }
        seeds = order.iter().map(|&c| best[c].1).collect();
        let _ = total;
    }
    chart
}

/// The Atlas projection of `faces`: their UVs, one row per face.
pub fn atlas_uvs(mesh: &Mesh, faces: &[u32], o: &AtlasChartOptions) -> Result<Vec<Vec<DVec2>>> {
    let chart = charts(mesh, faces, o);
    // Seams: edges between different charts (and open borders).
    let mut owner: HashMap<(u32, u32), usize> = HashMap::new();
    let mut seams = Vec::new();
    for (k, &f) in faces.iter().enumerate() {
        let v = &mesh.faces[f as usize].vertices;
        for i in 0..v.len() {
            let e = edge_key(v[i], v[(i + 1) % v.len()]);
            match owner.get(&e) {
                Some(&other) if chart[other] != chart[k] => seams.push([e.0, e.1]),
                Some(_) => {}
                None => {
                    owner.insert(e, k);
                }
            }
        }
    }
    let input = uv::UvMesh {
        points: mesh.vertices.clone(),
        weights: None,
        faces: faces
            .iter()
            .map(|&f| {
                mesh.faces[f as usize]
                    .vertices
                    .iter()
                    .map(|&p| uv::Corner::new(p, DVec2::ZERO))
                    .collect()
            })
            .collect(),
        seams,
    };
    let solved = uv::unwrap(
        &input,
        &uv::UnwrapOptions {
            method: uv::UnwrapMethod::Conformal,
            average_scale: true,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    let mut rows = solved.uvs;
    // Orientation (UVA-P03): each chart is turned so world up (+Y; +Z for
    // charts facing up or down) runs up V, so charts of both sides of a
    // shape read the same way up. Charts are never mirrored (that would
    // flip them).
    if o.orientation || o.fewer {
        let mut members: HashMap<usize, Vec<usize>> = HashMap::new();
        for (k, &c) in chart.iter().enumerate() {
            members.entry(c).or_default().push(k);
        }
        for list in members.values() {
            let mut n = DVec3::ZERO;
            for &k in list {
                n += mesh.normal(faces[k] as usize);
            }
            let up = if n.normalize_or_zero().y.abs() > 0.9 {
                DVec3::Z
            } else {
                DVec3::Y
            };
            // The UV direction world-up grows along: the gradient of the
            // up coordinate over the chart's UV edges.
            let mut grad = DVec2::ZERO;
            for &k in list {
                let v = &mesh.faces[faces[k] as usize].vertices;
                let q = &rows[k];
                for i in 0..v.len() {
                    let j = (i + 1) % v.len();
                    let d = q[j] - q[i];
                    let h = (mesh.vertices[v[j] as usize] - mesh.vertices[v[i] as usize]).dot(up);
                    grad += d * h;
                }
            }
            if grad.length() < 1e-12 {
                continue;
            }
            let turn = std::f64::consts::FRAC_PI_2 - grad.y.atan2(grad.x);
            let (c, s) = (turn.cos(), turn.sin());
            for &k in list {
                for p in rows[k].iter_mut() {
                    *p = DVec2::new(c * p.x - s * p.y, s * p.x + c * p.y);
                }
            }
        }
    }
    // The solver places every chart from the origin, so charts that meet
    // in 3D could touch in UV at the same place and read as one island:
    // set each chart apart before packing.
    let mut lo: HashMap<usize, DVec2> = HashMap::new();
    for (k, row) in rows.iter().enumerate() {
        let e = lo.entry(chart[k]).or_insert(DVec2::MAX);
        for p in row {
            *e = e.min(*p);
        }
    }
    let span = rows
        .iter()
        .flatten()
        .fold(1f64, |m, p| m.max(p.x.abs()).max(p.y.abs()))
        * 4.;
    for (k, row) in rows.iter_mut().enumerate() {
        let shift = DVec2::new(chart[k] as f64 * span, 0.) - lo[&chart[k]];
        for p in row.iter_mut() {
            *p += shift;
        }
    }
    Ok(rows)
}
