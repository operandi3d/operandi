// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `intern/slim/intern/geometry_data_retrieval.cpp`,
// `uv_initializer.cpp`, `least_squares_relocator.cpp`,
// `area_compensation.cpp`, `slim_parametrizer.cpp` and the SLIM and
// degenerate-triangle sections of
// `source/blender/geometry/intern/uv_parametrizer.cc`, at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22. The solver itself is in
// `solver.rs` (MPL-2.0).

//! Minimum stretch unwrapping (SLIM): set up each chart, start from a
//! mean-value-coordinates layout inside a circle, then run SLIM iterations.

mod solver;

use crate::chart::{FACE_DONE, Param, V, VERT_PIN, VERT_SELECT};
use crate::geom;
use crate::sparse::{Triplets, solve};
use glam::{DVec2, DVec3};
use solver::{SlimData, map_area, surface_area};

/// Options for minimum stretch (`ParamSlimOptions`).
#[derive(Clone, Copy, Debug)]
pub struct SlimOptions {
    /// Solver iterations; more is closer to the minimum.
    pub iterations: usize,
    /// How strongly point weights scale the result (0 = off).
    pub weight_influence: f64,
    /// Never mirror an island to match its pins.
    pub no_flip: bool,
    /// Start from the current UVs when they have no flipped triangles.
    pub skip_init: bool,
}

impl Default for SlimOptions {
    fn default() -> Self {
        Self {
            iterations: 10,
            weight_influence: 0.0,
            no_flip: false,
            skip_init: false,
        }
    }
}

/// One chart's data for SLIM (`MatrixTransferChart`).
struct SlimChart {
    v: Vec<DVec3>,
    uv: Vec<DVec2>,
    f: Vec<[usize; 3]>,
    /// Half-edges (start, end) with 3D lengths, for the initial layout.
    e: Vec<(usize, usize)>,
    el: Vec<f64>,
    boundary: usize,
    pins: Vec<usize>,
    pin_pos: Vec<DVec2>,
    weights: Vec<f64>,
    succeeded: bool,
}

const DOUBLED_VERT_THRESHOLD: f64 = 1.0e-5;

fn count_flips(f: &[[usize; 3]], uv: &[DVec2]) -> usize {
    f.iter()
        .filter(|t| (uv[t[1]] - uv[t[0]]).perp_dot(uv[t[2]] - uv[t[0]]) < 0.0)
        .count()
}

/// Mean value coordinates layout with the border fixed to `bnd_uv`
/// (`convex_border_parameterization` with `MVC`). Border vertices are
/// `0..boundary`.
fn mvc(ch: &SlimChart, bnd_uv: &[DVec2]) -> Option<Vec<DVec2>> {
    let n = ch.v.len();
    let nb = ch.boundary;
    // Angle at each vertex next to each half-edge: (vertex, 2*other [+1]).
    let mut angles: std::collections::HashMap<(usize, usize), f64> =
        std::collections::HashMap::new();
    for t in &ch.f {
        let a = [
            geom::angle_v3(ch.v[t[1]], ch.v[t[0]], ch.v[t[2]]),
            geom::angle_v3(ch.v[t[2]], ch.v[t[1]], ch.v[t[0]]),
            geom::angle_v3(ch.v[t[0]], ch.v[t[2]], ch.v[t[1]]),
        ];
        for k in 0..3 {
            let (v, next, prev) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
            *angles.entry((v, 2 * next)).or_default() += a[k];
            *angles.entry((v, 2 * prev + 1)).or_default() += a[k];
        }
    }
    let nu = n - nb;
    let mut aint = Triplets::new(nu, nu);
    let mut diag = vec![0.0; nu];
    let mut off: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nu];
    let mut bnd: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nu];
    for (k, &(a, b)) in ch.e.iter().enumerate() {
        if a < nb {
            continue;
        }
        let row = a - nb;
        let a1 = angles.get(&(a, 2 * b)).copied().unwrap_or(0.0);
        let a2 = angles.get(&(a, 2 * b + 1)).copied().unwrap_or(0.0);
        let w = ((a1 / 2.0).tan() + (a2 / 2.0).tan()) / ch.el[k];
        diag[row] += w;
        if b >= nb {
            off[row].push((b - nb, -w));
        } else {
            bnd[row].push((b, w));
        }
    }
    let mut rhs_u = vec![0.0; nu];
    let mut rhs_v = vec![0.0; nu];
    for row in 0..nu {
        let d = diag[row];
        aint.add(row, row, 1.0);
        for &(c, w) in &off[row] {
            aint.add(row, c, w / d);
        }
        for &(b, w) in &bnd[row] {
            rhs_u[row] += w / d * bnd_uv[b].x;
            rhs_v[row] += w / d * bnd_uv[b].y;
        }
    }
    let x = solve(&aint, &[rhs_u, rhs_v], false)?;
    let mut uv = bnd_uv.to_vec();
    uv.extend((0..nu).map(|i| DVec2::new(x[0][i], x[1][i])));
    Some(uv)
}

fn rotation(c: f64, s: f64) -> [[f64; 2]; 2] {
    [[c, -s], [s, c]]
}

fn apply(m: [[f64; 2]; 2], p: DVec2) -> DVec2 {
    DVec2::new(m[0][0] * p.x + m[0][1] * p.y, m[1][0] * p.x + m[1][1] * p.y)
}

/// Fit the initial layout to the pins (`transform_initialized_map`).
fn fit_to_pins(s: &mut SlimData) {
    match s.b.len() {
        0 => {}
        1 => {
            let t = s.bc[0] - s.v_o[s.b[0]];
            for p in &mut s.v_o {
                *p += t;
            }
        }
        2 => {
            let pd = s.bc[0] - s.bc[1];
            let id = s.v_o[s.b[0]] - s.v_o[s.b[1]];
            let scale = pd.length() / id.length();
            let (pn, inn) = (pd.normalize(), id.normalize());
            // Blender's version always turns one way (its TODO); the signed
            // angle here turns the right way.
            let c = inn.dot(pn);
            let sn = inn.perp_dot(pn);
            let m = rotation(c * scale, sn * scale);
            for p in &mut s.v_o {
                *p = apply(m, *p);
            }
            let t = s.bc[0] - s.v_o[s.b[0]];
            for p in &mut s.v_o {
                *p += t;
            }
        }
        _ => {
            let flip_allowed = s.reflection_mode == 0;
            let k = s.b.len() as f64;
            let ci = s.b.iter().map(|&i| s.v_o[i]).sum::<DVec2>() / k;
            let cp = s.bc.iter().sum::<DVec2>() / k;
            let init: Vec<DVec2> = s.b.iter().map(|&i| s.v_o[i] - ci).collect();
            let pins: Vec<DVec2> = s.bc.iter().map(|p| *p - cp).collect();
            // S = init^T * pins.
            let (mut a, mut b, mut c, mut d) = (0.0, 0.0, 0.0, 0.0);
            for (x, y) in init.iter().zip(&pins) {
                a += x.x * y.x;
                b += x.x * y.y;
                c += x.y * y.x;
                d += x.y * y.y;
            }
            // S = U diag V^T; the best rotation is V U^T. With S = R(phi)
            // diag(sx, sy) R(theta), V U^T = R(-theta) [diag(1,-1)] R(-phi)
            // where the reflection appears when sy < 0.
            let e = (a + d) * 0.5;
            let f = (a - d) * 0.5;
            let g = (c + b) * 0.5;
            let h = (c - b) * 0.5;
            let q = (e * e + h * h).sqrt();
            let r = (f * f + g * g).sqrt();
            let a1 = g.atan2(f);
            let a2 = h.atan2(e);
            let theta = (a2 - a1) * 0.5;
            let phi = (a2 + a1) * 0.5;
            let reflection = q - r < 0.0;
            let rot = |x: f64| rotation(x.cos(), x.sin());
            let mul = |m: [[f64; 2]; 2], n: [[f64; 2]; 2]| {
                let mut o = [[0.0; 2]; 2];
                for i in 0..2 {
                    for j in 0..2 {
                        o[i][j] = m[i][0] * n[0][j] + m[i][1] * n[1][j];
                    }
                }
                o
            };
            let mirror = [[1.0, 0.0], [0.0, -1.0]];
            let mut m = if reflection && flip_allowed {
                for t in &mut s.f {
                    t.swap(0, 2);
                }
                mul(mul(rot(-theta), mirror), rot(-phi))
            } else {
                rot(-(theta + phi))
            };
            // Least squares uniform scale.
            let (mut num, mut den) = (0.0, 0.0);
            for (x, y) in init.iter().zip(&pins) {
                let tx = apply(m, *x);
                num += tx.dot(*y);
                den += tx.length_squared();
            }
            let scale = if den > 0.0 { (num / den).abs() } else { 1.0 };
            for row in &mut m {
                for x in row.iter_mut() {
                    *x *= scale;
                }
            }
            let t = cp - apply(m, ci);
            for p in &mut s.v_o {
                *p = apply(m, *p) + t;
            }
        }
    }
}

fn setup(ch: &mut SlimChart, opts: &SlimOptions) -> Option<SlimData> {
    let mut s = SlimData::new(ch.v.clone(), ch.f.clone(), ch.uv.clone());
    s.reflection_mode = opts.no_flip as i32;
    s.skip_initialization = opts.skip_init && count_flips(&ch.f, &ch.uv) == 0;
    s.b = ch.pins.clone();
    s.bc = ch.pin_pos.clone();
    s.soft_const_p = 10.0e100;
    if opts.weight_influence > 0.0 {
        s.with_weights = true;
        let sign = if opts.weight_influence >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let max_factor = opts.weight_influence.abs() + 1.0;
        s.weight_per_face =
            ch.f.iter()
                .map(|t| {
                    let mean = (ch.weights[t[0]] + ch.weights[t[1]] + ch.weights[t[2]]) / 3.0;
                    max_factor.powf(sign * 2.0 * ((1.0 - mean) - 0.5))
                })
                .collect();
    }
    if !s.skip_initialization {
        // Border on a circle, inside by mean value coordinates.
        let nb = ch.boundary;
        let bnd: Vec<DVec2> = (0..nb)
            .map(|i| {
                let a = std::f64::consts::TAU / nb as f64 * i as f64;
                DVec2::new(a.cos() * 0.5 + 0.5, a.sin() * 0.5 + 0.5)
            })
            .collect();
        s.v_o = if nb == ch.v.len() {
            bnd
        } else {
            mvc(ch, &bnd)?
        };
        fit_to_pins(&mut s);
    }
    // Match the 3D size to the map's when the map's size matters.
    if s.skip_initialization || !s.b.is_empty() {
        let area = map_area(&s.v_o, &s.f).abs();
        if area != 0.0 {
            s.expected_map_area = area;
            let ratio = surface_area(&s.v, &s.f) / area;
            let k = ratio.sqrt();
            for p in &mut s.v {
                *p /= k;
            }
        }
    }
    s.precompute();
    Some(s)
}

fn correct_map_area(s: &mut SlimData) {
    if s.expected_map_area == 0.0 || !s.b.is_empty() {
        return;
    }
    let area = map_area(&s.v_o, &s.f);
    if area == 0.0 {
        return;
    }
    let k = (area / s.expected_map_area).sqrt();
    for p in &mut s.v_o {
        *p /= k;
    }
}

impl Param {
    fn edge_matrix(dir: DVec3) -> Option<[DVec3; 3]> {
        let eps = 1.0e-5;
        let len = dir.length();
        if len < eps {
            return None;
        }
        let d = dir / len;
        let mut n = d.cross(DVec3::Z);
        if n.length() < eps {
            n = d.cross(DVec3::Y);
            if n.length() < eps {
                return None;
            }
        }
        let n = n.normalize();
        let t = d.cross(n);
        Some([d, n, t])
    }

    fn mat_mul(m: [DVec3; 3], v: DVec3) -> DVec3 {
        m[0] * v.x + m[1] * v.y + m[2] * v.z
    }

    fn validate_corrected_point(
        &self,
        corr_e: usize,
        co1: DVec3,
        co2: DVec3,
        min_area: f64,
        min_cos: f64,
    ) -> bool {
        let corr_v = self.edges[corr_e].vert;
        let pair = self.edges[corr_e].pair;
        for e in self.wheel(corr_v) {
            if e == corr_e || self.faces[self.edges[e].face].flag & FACE_DONE == 0 {
                continue;
            }
            let n1 = self.edges[e].next;
            let n2 = self.edges[n1].next;
            let (a, b, c) = if Some(n2) == pair {
                (co1, co2, self.verts[self.edges[n2].vert].co)
            } else {
                (
                    co1,
                    self.verts[self.edges[n1].vert].co,
                    self.verts[self.edges[n2].vert].co,
                )
            };
            if geom::triangle_area(a, b, c) < min_area {
                return false;
            }
            if [
                geom::cos_v3(c, a, b),
                geom::cos_v3(a, b, c),
                geom::cos_v3(b, c, a),
            ]
            .iter()
            .any(|&x| x > min_cos)
            {
                return false;
            }
        }
        true
    }

    fn validate_corrected(&self, corr_e: usize, co: DVec3, min_area: f64, min_cos: f64) -> bool {
        let corr_v = self.edges[corr_e].vert;
        for e in self.wheel(corr_v) {
            if self.faces[self.edges[e].face].flag & FACE_DONE == 0 && e != corr_e {
                continue;
            }
            let n1 = self.edges[e].next;
            let o1 = self.verts[self.edges[n1].vert].co;
            let o2 = self.verts[self.edges[self.edges[n1].next].vert].co;
            if geom::triangle_area(co, o1, o2) < min_area {
                return false;
            }
            if [
                geom::cos_v3(o2, co, o1),
                geom::cos_v3(co, o1, o2),
                geom::cos_v3(o1, o2, co),
            ]
            .iter()
            .any(|&x| x > min_cos)
            {
                return false;
            }
        }
        true
    }

    fn correct_degenerate_point(&mut self, f: usize, min_area: f64, min_cos: f64) -> bool {
        const REF: [[f64; 3]; 12] = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 0.5, 1.0],
            [0.5, 0.0, 1.0],
            [0.5, 1.0, 0.0],
            [0.0, 1.0, 0.5],
            [1.0, 0.0, 0.5],
            [1.0, 0.5, 0.0],
        ];
        let corr_len = 2.0 * ((min_area + 1.0e-10) / 3.0 / 3.0_f64.sqrt()).sqrt();
        let es = self.face_edges(f);
        for m in 0..3 {
            for r in REF {
                let Some(mat) = Self::edge_matrix(DVec3::from_array(r)) else {
                    continue;
                };
                let mut co = [DVec3::ZERO; 3];
                for i in 0..3 {
                    let a = i as f64 / 3.0 * std::f64::consts::TAU;
                    let dir = Self::mat_mul(mat, DVec3::new(0.0, a.cos(), a.sin()))
                        * (corr_len * (m + 1) as f64);
                    co[i] = self.verts[self.edges[es[i]].vert].co + dir;
                }
                for i in 0..3 {
                    if !self.validate_corrected_point(
                        es[i],
                        co[i],
                        co[(i + 1) % 3],
                        min_area,
                        min_cos,
                    ) {
                        return false;
                    }
                }
                for i in 0..3 {
                    let v = self.edges[es[i]].vert;
                    self.verts[v].co = co[i];
                }
                self.faces[f].flag |= FACE_DONE;
                return true;
            }
        }
        false
    }

    /// `p_chart_correct_degenerate_triangles`: nudge points of slivers and
    /// collapsed triangles so every triangle has some area and no tiny
    /// angle (SLIM needs that). Returns whether all were fixed.
    fn correct_degenerate_triangles(&mut self, c: usize, min_area: f64, min_angle: f64) -> bool {
        let eps = 1.0e-6;
        let min_cos = min_angle.cos();
        let min_sin = (min_angle + 1.0e-10).sin();
        let faces = self.charts[c].faces.clone();
        for &f in &faces {
            if self.faces[f].flag & FACE_DONE != 0 {
                continue;
            }
            let area = self.face_area(f);
            let es = self.face_edges(f);
            let (mut max_e, mut max_len) = (None, f64::NEG_INFINITY);
            let (mut min_e, mut min_len) = (None, f64::INFINITY);
            let mut mid_e = None;
            for &e in &es {
                let len = self.edge_length(e);
                if len > max_len {
                    max_e = Some(e);
                    max_len = len;
                    let me = max_e.unwrap();
                    mid_e = Some(if Some(self.edges[me].next) == min_e {
                        self.edges[min_e.unwrap()].next
                    } else {
                        self.edges[me].next
                    });
                }
                if len < min_len {
                    min_e = Some(e);
                    min_len = len;
                    let ne = min_e.unwrap();
                    mid_e = Some(if Some(self.edges[ne].next) == max_e {
                        self.edges[max_e.unwrap()].next
                    } else {
                        self.edges[ne].next
                    });
                }
            }
            let (max_e, min_e, mid_e) = (max_e.unwrap(), min_e.unwrap(), mid_e.unwrap());
            if max_len < eps || (area <= min_area && min_e == max_e) {
                self.correct_degenerate_point(f, min_area, min_cos);
                continue;
            }
            if min_e == max_e {
                self.faces[f].flag |= FACE_DONE;
                continue;
            }
            let Some(m) = Self::edge_matrix(
                self.verts[self.edges[self.edges[max_e].next].vert].co
                    - self.verts[self.edges[max_e].vert].co,
            ) else {
                continue;
            };
            let co = |e: usize| self.verts[self.edges[e].vert].co;
            let max_face_cos = if self.edges[mid_e].next == max_e {
                geom::cos_v3(co(mid_e), co(max_e), co(min_e))
            } else {
                geom::cos_v3(co(max_e), co(mid_e), co(min_e))
            };
            let angle_corr_len = if max_face_cos > min_cos {
                self.edge_length(mid_e) * min_sin
            } else {
                0.0
            };
            if area > min_area && angle_corr_len == 0.0 {
                self.faces[f].flag |= FACE_DONE;
                continue;
            }
            let corr_len = ((min_area + 1.0e-10) * 2.0 / max_len).max(angle_corr_len);
            let corr_e = self.edges[self.edges[max_e].next].next;
            let corr_v = self.edges[corr_e].vert;
            let mut found = None;
            'search: for mult in 0..2 {
                for d in 0..16 {
                    let a = d as f64 / 16.0 * std::f64::consts::TAU;
                    let dir = Self::mat_mul(m, DVec3::new(0.0, a.cos(), a.sin()))
                        * (corr_len * (mult + 1) as f64);
                    let cand = self.verts[corr_v].co + dir;
                    if self.validate_corrected(corr_e, cand, min_area, min_cos) {
                        found = Some(cand);
                        break 'search;
                    }
                }
            }
            if let Some(cand) = found {
                self.faces[f].flag |= FACE_DONE;
                self.verts[corr_v].co = cand;
            }
        }
        let mut all = true;
        for &f in &faces {
            if self.faces[f].flag & FACE_DONE == 0 {
                all = false;
            }
            self.faces[f].flag &= !FACE_DONE;
        }
        all
    }

    fn slim_chart_data(
        &mut self,
        c: usize,
        opts: &SlimOptions,
        minimize_stretch: bool,
    ) -> SlimChart {
        let mut ch = SlimChart {
            v: Vec::new(),
            uv: Vec::new(),
            f: Vec::new(),
            e: Vec::new(),
            el: Vec::new(),
            boundary: 0,
            pins: Vec::new(),
            pin_pos: Vec::new(),
            weights: Vec::new(),
            succeeded: true,
        };
        let verts = self.charts[c].verts.clone();
        for &v in &verts {
            self.verts[v].on_boundary = false;
        }
        // Border vertices first, in border order.
        let (_, outer) = self.chart_boundaries(c);
        let mut order: Vec<V> = Vec::new();
        if let Some(outer) = outer {
            let mut be = outer;
            loop {
                let v = self.edges[be].vert;
                self.verts[v].on_boundary = true;
                self.verts[v].id = order.len();
                order.push(v);
                be = self.boundary_next(be);
                if be == outer || order.len() > verts.len() {
                    break;
                }
            }
        } else {
            ch.succeeded = false;
        }
        ch.boundary = order.len();
        for &v in &verts {
            if !self.verts[v].on_boundary {
                self.verts[v].id = order.len();
                order.push(v);
            }
        }
        for &v in &order {
            let vert = &self.verts[v];
            ch.v.push(vert.co);
            ch.uv.push(vert.uv);
            ch.weights.push(vert.weight);
            let pinned =
                vert.flag & VERT_PIN != 0 || (minimize_stretch && vert.flag & VERT_SELECT == 0);
            if pinned {
                ch.pins.push(vert.id);
                ch.pin_pos.push(vert.uv);
            }
        }
        let _ = opts;
        for &e in &self.charts[c].edges {
            let len = self.edge_length(e);
            if len < DOUBLED_VERT_THRESHOLD {
                ch.succeeded = false;
            }
            let a = self.verts[self.edges[e].vert].id;
            let b = self.verts[self.edges[self.edges[e].next].vert].id;
            ch.e.push((a, b));
            ch.el.push(len);
        }
        for &f in &self.charts[c].faces {
            let [a, b, cc] = self.face_verts(f);
            ch.f.push([self.verts[a].id, self.verts[b].id, self.verts[cc].id]);
        }
        ch
    }

    /// Minimum stretch unwrap of every chart (`uv_parametrizer_slim_solve`).
    /// Returns (charts changed, charts failed). Charts that fail get zero UVs,
    /// as in Blender.
    pub fn slim_solve(&mut self, opts: &SlimOptions, original_bounds: bool) -> (usize, usize) {
        self.slim_run(opts, original_bounds, false)
    }

    /// SLIM iterations on the current UVs, moving only selected, unpinned
    /// vertices (Blender's minimize-stretch path with SLIM).
    pub fn slim_relax(&mut self, opts: &SlimOptions) -> (usize, usize) {
        let opts = SlimOptions {
            skip_init: true,
            ..*opts
        };
        self.slim_run(&opts, false, true)
    }

    fn slim_run(
        &mut self,
        opts: &SlimOptions,
        original_bounds: bool,
        minimize_stretch: bool,
    ) -> (usize, usize) {
        let (mut changed, mut failed) = (0, 0);
        for c in 0..self.charts.len() {
            self.backup_uvs(c);
            if original_bounds {
                let pts = self.chart_uv_points(c);
                let angle = geom::minimum_area_angle(&pts);
                let (lo, hi) = self.chart_uv_bbox(c);
                self.charts[c].orig_bounds = Some((lo, hi, angle));
            }
            self.correct_degenerate_triangles(c, 1.0e-8, 1.0_f64.to_radians());
            let mut ch = self.slim_chart_data(c, opts, minimize_stretch);
            let mut result = None;
            if ch.succeeded
                && let Some(mut s) = setup(&mut ch, opts)
                && s.solve(opts.iterations).is_ok()
            {
                correct_map_area(&mut s);
                if s.v_o.iter().all(|p| p.is_finite()) {
                    result = Some(s.v_o);
                }
            }
            let verts = self.charts[c].verts.clone();
            match result {
                Some(uv) => {
                    changed += 1;
                    for v in verts {
                        if self.verts[v].flag & VERT_PIN == 0 {
                            self.verts[v].uv = uv[self.verts[v].id];
                        }
                    }
                }
                None => {
                    failed += 1;
                    if !minimize_stretch {
                        for v in verts {
                            self.verts[v].uv = DVec2::ZERO;
                        }
                    }
                }
            }
        }
        (changed, failed)
    }
}
