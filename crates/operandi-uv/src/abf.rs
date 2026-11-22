// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/geometry/intern/uv_parametrizer.cc`
// (the ABF section) at commit b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Angle Based Flattening (ABF++, Sheffer et al. 2005): solves for the
//! triangle angles of a flat layout closest to the 3D angles. LSCM then lays
//! the triangles out from those angles.

use crate::chart::{Param, VERT_INTERIOR};
use crate::sparse::{Triplets, solve};
use std::f64::consts::PI;

const ABF_MAX_ITER: usize = 20;

struct System {
    ninterior: usize,
    alpha: Vec<f64>,
    beta: Vec<f64>,
    sine: Vec<f64>,
    cosine: Vec<f64>,
    weight: Vec<f64>,
    b_alpha: Vec<f64>,
    b_triangle: Vec<f64>,
    b_interior: Vec<f64>,
    lambda_triangle: Vec<f64>,
    lambda_planar: Vec<f64>,
    lambda_length: Vec<f64>,
    j2dt: Vec<[f64; 3]>,
    bstar: Vec<f64>,
    dstar: Vec<f64>,
}

impl System {
    fn compute_sines(&mut self) {
        for i in 0..self.alpha.len() {
            let (s, c) = self.alpha[i].sin_cos();
            self.sine[i] = s;
            self.cosine[i] = c;
        }
    }
}

impl Param {
    fn abf_sin_product(&self, sys: &System, v: usize, aid: Option<usize>) -> f64 {
        let (mut sin1, mut sin2) = (1.0, 1.0);
        for e in self.wheel(v) {
            let e1 = self.edges[e].next;
            let e2 = self.edges[e1].next;
            let (id1, id2) = (self.edges[e1].id, self.edges[e2].id);
            if aid == Some(id1) {
                sin1 *= sys.cosine[id1];
                sin2 = 0.0;
            } else {
                sin1 *= sys.sine[id1];
            }
            if aid == Some(id2) {
                sin1 = 0.0;
                sin2 *= sys.cosine[id2];
            } else {
                sin2 *= sys.sine[id2];
            }
        }
        sin1 - sin2
    }

    fn abf_grad_alpha(&self, sys: &System, f: usize, e: usize) -> f64 {
        let v = self.edges[e].vert;
        let v1 = self.edges[self.edges[e].next].vert;
        let v2 = self.edges[self.edges[self.edges[e].next].next].vert;
        let id = self.edges[e].id;
        let mut deriv = (sys.alpha[id] - sys.beta[id]) * sys.weight[id];
        deriv += sys.lambda_triangle[self.faces[f].id];
        if self.verts[v].flag & VERT_INTERIOR != 0 {
            deriv += sys.lambda_planar[self.verts[v].id];
        }
        for vv in [v1, v2] {
            if self.verts[vv].flag & VERT_INTERIOR != 0 {
                let product = self.abf_sin_product(sys, vv, Some(id));
                deriv += sys.lambda_length[self.verts[vv].id] * product;
            }
        }
        deriv
    }

    fn abf_gradient(&self, sys: &mut System, c: usize) -> f64 {
        let mut norm = 0.0;
        for &f in &self.charts[c].faces {
            let es = self.face_edges(f);
            let mut gt = -PI;
            for &e in &es {
                let g = self.abf_grad_alpha(sys, f, e);
                sys.b_alpha[self.edges[e].id] = -g;
                norm += g * g;
                gt += sys.alpha[self.edges[e].id];
            }
            sys.b_triangle[self.faces[f].id] = -gt;
            norm += gt * gt;
        }
        for &v in &self.charts[c].verts {
            if self.verts[v].flag & VERT_INTERIOR == 0 {
                continue;
            }
            let mut gplanar = -2.0 * PI;
            for e in self.wheel(v) {
                gplanar += sys.alpha[self.edges[e].id];
            }
            let id = self.verts[v].id;
            sys.b_interior[id] = -gplanar;
            norm += gplanar * gplanar;
            let glength = self.abf_sin_product(sys, v, None);
            sys.b_interior[sys.ninterior + id] = -glength;
            norm += glength * glength;
        }
        norm
    }

    fn abf_matrix_invert(&self, sys: &mut System, c: usize) -> bool {
        let ninterior = sys.ninterior;
        let nvar = 2 * ninterior;
        let mut matrix = Triplets::new(nvar, nvar);
        let mut rhs = sys.b_interior.clone();

        for &f in &self.charts[c].faces {
            let [e1, e2, e3] = self.face_edges(f);
            let ids = [self.edges[e1].id, self.edges[e2].id, self.edges[e3].id];
            let vs = [
                self.edges[e1].vert,
                self.edges[e2].vert,
                self.edges[e3].vert,
            ];
            let wi = ids.map(|id| 1.0 / sys.weight[id]);

            // bstar1 = (J1 * dInv * bAlpha - bTriangle)
            let mut b = 0.0;
            for k in 0..3 {
                b += sys.b_alpha[ids[k]] * wi[k];
            }
            b -= sys.b_triangle[self.faces[f].id];
            // si = J1 * d * J1t
            let si = 1.0 / (wi[0] + wi[1] + wi[2]);
            // J1t * si * bstar1 - bAlpha
            let beta = ids.map(|id| b * si - sys.b_alpha[id]);
            sys.bstar[self.faces[f].id] = b;
            sys.dstar[self.faces[f].id] = si;

            let mut w = [[si; 3]; 3];
            for k in 0..3 {
                w[k][k] = si - sys.weight[ids[k]];
            }

            let mut vid = [usize::MAX; 6];
            let mut j2 = [[0.0f64; 3]; 3];
            let mut row1 = [0.0f64; 6];
            let mut row2 = [0.0f64; 6];
            let mut row3 = [0.0f64; 6];

            for k in 0..3 {
                let v = vs[k];
                if self.verts[v].flag & VERT_INTERIOR == 0 {
                    continue;
                }
                let vi = self.verts[v].id;
                vid[k] = vi;
                vid[k + 3] = ninterior + vi;
                for a in 0..3 {
                    j2[a][k] = if a == k {
                        wi[a]
                    } else {
                        self.abf_sin_product(sys, v, Some(ids[a])) * wi[a]
                    };
                    sys.j2dt[ids[a]][k] = j2[a][k];
                }
                rhs[vi] += j2[k][k] * beta[k];
                let (o1, o2) = ((k + 1) % 3, (k + 2) % 3);
                rhs[ninterior + vi] += j2[o1][k] * beta[o1] + j2[o2][k] * beta[o2];

                row1[k] = j2[k][k] * w[0][k];
                row2[k] = j2[k][k] * w[1][k];
                row3[k] = j2[k][k] * w[2][k];
                row1[k + 3] = j2[o1][k] * w[0][o1] + j2[o2][k] * w[0][o2];
                row2[k + 3] = j2[o1][k] * w[1][o1] + j2[o2][k] * w[1][o2];
                row3[k + 3] = j2[o1][k] * w[2][o1] + j2[o2][k] * w[2][o2];
            }

            for i in 0..3 {
                let r = vid[i];
                if r == usize::MAX {
                    continue;
                }
                for j in 0..6 {
                    let col = vid[j];
                    if col == usize::MAX {
                        continue;
                    }
                    let rows = [row1[j], row2[j], row3[j]];
                    for a in 0..3 {
                        let row = if a == i { r } else { r + ninterior };
                        matrix.add(row, col, j2[a][i] * rows[a]);
                    }
                }
            }
        }

        let Some(x) = solve(&matrix, &[rhs], false) else {
            return false;
        };
        let x = &x[0];
        for &f in &self.charts[c].faces {
            let [e1, e2, e3] = self.face_edges(f);
            let ids = [self.edges[e1].id, self.edges[e2].id, self.edges[e3].id];
            let vs = [
                self.edges[e1].vert,
                self.edges[e2].vert,
                self.edges[e3].vert,
            ];
            let mut pre = [0.0f64; 3];
            for k in 0..3 {
                let v = vs[k];
                if self.verts[v].flag & VERT_INTERIOR == 0 {
                    continue;
                }
                let vi = self.verts[v].id;
                let (x1, x2) = (x[vi], x[ninterior + vi]);
                for a in 0..3 {
                    pre[a] += sys.j2dt[ids[a]][k] * if a == k { x1 } else { x2 };
                }
            }
            let fid = self.faces[f].id;
            let dlambda1 = sys.dstar[fid] * (sys.bstar[fid] - (pre[0] + pre[1] + pre[2]));
            sys.lambda_triangle[fid] += dlambda1;
            for k in 0..3 {
                let id = ids[k];
                let dalpha = sys.b_alpha[id] - dlambda1;
                let alpha = sys.alpha[id] + dalpha / sys.weight[id] - pre[k];
                sys.alpha[id] = alpha.clamp(0.0, PI);
            }
        }
        for i in 0..ninterior {
            sys.lambda_planar[i] += x[i];
            sys.lambda_length[i] += x[ninterior + i];
        }
        true
    }

    /// `p_chart_abf_solve`: on success stores the flattened angles in the
    /// chart (three per face, in face edge order).
    pub(crate) fn abf_solve(&mut self, c: usize) -> bool {
        let limit = if self.charts[c].faces.len() > 100 {
            1.0
        } else {
            0.001
        };
        let mut ninterior = 0;
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            if self.vert_interior(v) {
                self.verts[v].flag |= VERT_INTERIOR;
                self.verts[v].id = ninterior;
                ninterior += 1;
            } else {
                self.verts[v].flag &= !VERT_INTERIOR;
            }
        }
        let mut nangles = 0;
        for (fi, &f) in self.charts[c].faces.clone().iter().enumerate() {
            self.faces[f].id = fi;
            for e in self.face_edges(f) {
                self.edges[e].id = nangles;
                nangles += 1;
            }
        }
        let nfaces = self.charts[c].faces.len();
        let mut sys = System {
            ninterior,
            alpha: vec![0.0; nangles],
            beta: vec![0.0; nangles],
            sine: vec![0.0; nangles],
            cosine: vec![0.0; nangles],
            weight: vec![0.0; nangles],
            b_alpha: vec![0.0; nangles],
            b_triangle: vec![0.0; nfaces],
            b_interior: vec![0.0; 2 * ninterior],
            lambda_triangle: vec![0.0; nfaces],
            lambda_planar: vec![0.0; ninterior],
            lambda_length: vec![1.0; ninterior],
            j2dt: vec![[0.0; 3]; nangles],
            bstar: vec![0.0; nfaces],
            dstar: vec![0.0; nfaces],
        };
        for &f in &self.charts[c].faces {
            let angles = self.face_angles(f);
            for (k, e) in self.face_edges(f).into_iter().enumerate() {
                let id = self.edges[e].id;
                sys.alpha[id] = angles[k];
                sys.beta[id] = angles[k];
                sys.weight[id] = 2.0 / (angles[k] * angles[k]);
            }
        }
        for &v in &self.charts[c].verts {
            if self.verts[v].flag & VERT_INTERIOR == 0 {
                continue;
            }
            let wheel = self.wheel(v);
            let sum: f64 = wheel.iter().map(|&e| sys.beta[self.edges[e].id]).sum();
            let scale = if sum == 0.0 { 0.0 } else { 2.0 * PI / sum };
            for e in wheel {
                let id = self.edges[e].id;
                sys.beta[id] *= scale;
                sys.alpha[id] = sys.beta[id];
            }
        }
        if ninterior > 0 {
            sys.compute_sines();
            let mut converged = false;
            for _ in 0..ABF_MAX_ITER {
                let norm = self.abf_gradient(&mut sys, c);
                if norm < limit {
                    converged = true;
                    break;
                }
                if !self.abf_matrix_invert(&mut sys, c) {
                    return false;
                }
                sys.compute_sines();
            }
            if !converged {
                return false;
            }
        }
        self.charts[c].abf_alpha = Some(sys.alpha);
        true
    }
}
