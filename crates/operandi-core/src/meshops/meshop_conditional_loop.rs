//! Select Conditional Loop (`selop.loopConditional.item`, MESHOPS-TIDY
//! TB-10..20): a selection op that grows each edge coming into it (the
//! selection so far) into an edge loop, stopping at a border, a closed
//! loop or one of its stop rules.
//!
//! Provisional (MOQ-69): an edge's "first" point is the lower point number
//! (`Mesh::edges` pairs, TB-12); seeds of another kind are converted to
//! edges first (a polygon's edges, an edge whose two points are picked); a
//! walk that reaches a border point from inside stops there, and a walk
//! along a border edge continues only along border edges.
use crate::Mesh;
use crate::uv_seams::SEAM_PREFIX;
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// How the next edge at a point is chosen (TB-13).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoopMethod {
    /// The opposite edge at a 4-edge point; stops anywhere else.
    Middle,
    /// The straightest edge, at any number of edges.
    #[default]
    Angle,
    /// As Angle, scored by the dot product (the same pick).
    Dot,
}
impl LoopMethod {
    pub const ALL: [(LoopMethod, &'static str); 3] = [
        (Self::Middle, "Middle"),
        (Self::Angle, "Angle"),
        (Self::Dot, "Dot Product"),
    ];
}

/// The op's channels (TB table, reference names and defaults).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LoopRules {
    pub loop_both_dirs: bool,
    #[serde(rename = "directionAB")]
    pub direction_ab: bool,
    pub method: LoopMethod,
    /// Edges per direction, counting the seed; 0 = no limit.
    pub cap_loop_count: u32,
    pub stop_at_selection: bool,
    pub stop_at_selection_count: u32,
    pub stop_at_shared_polygon: bool,
    pub stop_at_sharp_angles: bool,
    pub stop_at_selection_set: bool,
    pub stop_at_set_name: String,
    pub stop_at_set_count: u32,
    pub stop_at_seam_map: bool,
    pub stop_at_seam_name: String,
    pub stop_at_seam_count: u32,
    pub stop_at_length: bool,
    /// A fraction: 3 = 300% longer than the seed.
    pub stop_at_length_tolerance: f64,
}
impl Default for LoopRules {
    fn default() -> Self {
        Self {
            loop_both_dirs: true,
            direction_ab: true,
            method: LoopMethod::Angle,
            cap_loop_count: 0,
            stop_at_selection: true,
            stop_at_selection_count: 2,
            stop_at_shared_polygon: true,
            stop_at_sharp_angles: true,
            stop_at_selection_set: true,
            stop_at_set_name: String::new(),
            stop_at_set_count: 2,
            stop_at_seam_map: true,
            stop_at_seam_name: "UV Seam".into(),
            stop_at_seam_count: 2,
            stop_at_length: false,
            stop_at_length_tolerance: 3.0,
        }
    }
}

/// The loops grown from `seeds` (edge flags, numbered as `edges`), seeds
/// included.
pub fn grow(rules: &LoopRules, mesh: &Mesh, edges: &[(u32, u32)], seeds: &[bool]) -> Vec<bool> {
    let index: HashMap<(u32, u32), usize> =
        edges.iter().enumerate().map(|(i, &e)| (e, i)).collect();
    let at = |a: u32, b: u32| index.get(&(a.min(b), a.max(b))).copied();
    // Each point's edges, and each edge's polygons.
    let mut point_edges: Vec<Vec<usize>> = vec![Vec::new(); mesh.vertices.len()];
    for (i, &(a, b)) in edges.iter().enumerate() {
        point_edges[a as usize].push(i);
        point_edges[b as usize].push(i);
    }
    let mut edge_faces: Vec<Vec<u32>> = vec![Vec::new(); edges.len()];
    for f in 0..mesh.faces.len() {
        for (a, b) in mesh.element_spans(f) {
            if let Some(e) = at(a, b) {
                edge_faces[e].push(f as u32);
            }
        }
    }
    let border = |e: usize| edge_faces[e].len() == 1;
    let on_border: Vec<bool> = (0..mesh.vertices.len())
        .map(|v| point_edges[v].iter().any(|&e| border(e)))
        .collect();
    let share = |e: usize, f: usize| edge_faces[e].iter().any(|p| edge_faces[f].contains(p));
    // Edges that count for the stop rules (TB-15, TB-18).
    let named = |name: &str| -> Option<HashSet<usize>> {
        let set = mesh.attributes.edge_sets.get(name)?;
        Some(set.iter().filter_map(|&(a, b)| at(a, b)).collect())
    };
    let set_edges = (rules.stop_at_selection_set && !rules.stop_at_set_name.is_empty())
        .then(|| named(&rules.stop_at_set_name))
        .flatten();
    let seam_edges = (rules.stop_at_seam_map && !rules.stop_at_seam_name.is_empty())
        .then(|| named(&format!("{SEAM_PREFIX}{}", rules.stop_at_seam_name)))
        .flatten();
    let counted = |v: u32, arrived: usize, hit: &dyn Fn(usize) -> bool| {
        point_edges[v as usize]
            .iter()
            .filter(|&&e| e != arrived && hit(e))
            .count() as u32
    };
    let stops_at = |v: u32, arrived: usize| {
        (rules.stop_at_selection
            && counted(v, arrived, &|e| seeds[e]) >= rules.stop_at_selection_count.max(1))
            || set_edges.as_ref().is_some_and(|s| {
                counted(v, arrived, &|e| s.contains(&e)) >= rules.stop_at_set_count.max(1)
            })
            || seam_edges.as_ref().is_some_and(|s| {
                counted(v, arrived, &|e| s.contains(&e)) >= rules.stop_at_seam_count.max(1)
            })
    };
    let p = |v: u32| mesh.vertices[v as usize];
    let dir = |e: usize, from: u32| -> DVec3 {
        let (a, b) = edges[e];
        let to = if a == from { b } else { a };
        (p(to) - p(from)).normalize_or_zero()
    };
    let other = |e: usize, v: u32| {
        let (a, b) = edges[e];
        if a == v { b } else { a }
    };
    // The next edge after arriving at `v` along `e`, if the walk goes on.
    let next = |e: usize, v: u32, seed_len: f64| -> Option<usize> {
        let came = -dir(e, v); // the way the walk was going
        let along_border = border(e);
        if !along_border && on_border[v as usize] {
            return None;
        }
        let pick = match rules.method {
            LoopMethod::Middle => {
                let es = &point_edges[v as usize];
                if es.len() != 4 || on_border[v as usize] {
                    return None;
                }
                let mut opposite = es.iter().copied().filter(|&f| f != e && !share(e, f));
                let f = opposite.next()?;
                if opposite.next().is_some() {
                    return None;
                }
                f
            }
            LoopMethod::Angle | LoopMethod::Dot => {
                // A loop crosses to the far side of the point: edges on
                // the arriving edge's own polygons come last (only at
                // triangle and n-gon corners, where Shared Polygon stops).
                let straightest = |apart: bool| {
                    point_edges[v as usize]
                        .iter()
                        .copied()
                        .filter(|&f| f != e && (!along_border || border(f)))
                        .filter(|&f| !share(e, f) == apart)
                        .max_by(|&f, &g| came.dot(dir(f, v)).total_cmp(&came.dot(dir(g, v))))
                };
                straightest(true).or_else(|| straightest(false))?
            }
        };
        let turn = came.dot(dir(pick, v)).clamp(-1., 1.).acos();
        if rules.stop_at_sharp_angles && turn >= std::f64::consts::FRAC_PI_2 - 1e-9 {
            return None;
        }
        if rules.stop_at_shared_polygon && share(e, pick) {
            return None;
        }
        if rules.stop_at_length {
            let (a, b) = edges[pick];
            let len = p(a).distance(p(b));
            if len > seed_len * (1. + rules.stop_at_length_tolerance) + 1e-12 {
                return None;
            }
        }
        Some(pick)
    };
    let mut out = seeds.to_vec();
    let limit = match rules.cap_loop_count {
        0 => usize::MAX,
        n => n as usize - 1,
    };
    for (seed, _) in seeds.iter().enumerate().filter(|(_, s)| **s) {
        let (a, b) = edges[seed];
        let seed_len = p(a).distance(p(b));
        // TB-12: Start Direction on walks out of the first point.
        let starts: Vec<u32> = match (rules.loop_both_dirs, rules.direction_ab) {
            (true, _) => vec![a, b],
            (false, true) => vec![a],
            (false, false) => vec![b],
        };
        let mut walked: HashSet<usize> = HashSet::from([seed]);
        for start in starts {
            let (mut e, mut v) = (seed, start);
            let mut added = 0;
            while added < limit && !stops_at(v, e) {
                let Some(f) = next(e, v, seed_len) else {
                    break;
                };
                if !walked.insert(f) {
                    break; // back at the start: a closed loop
                }
                out[f] = true;
                added += 1;
                v = other(f, v);
                e = f;
            }
        }
    }
    out
}
