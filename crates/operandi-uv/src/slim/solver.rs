// SPDX-FileCopyrightText: 2016 Michael Rabinovich
// SPDX-FileCopyrightText: 2014 Alec Jacobson
// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: MPL-2.0
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from Blender `intern/slim/intern/slim.cpp`,
// `flip_avoiding_line_search.cpp` and `doublearea.cpp` at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22. Only the symmetric Dirichlet
// energy Blender uses is kept; Eigen matrices became per-face arrays.

//! Scalable Locally Injective Maps (Rabinovich et al. 2016): repeatedly
//! solves a reweighted, as-rigid-as-possible-like system whose weights come
//! from the symmetric Dirichlet energy, then moves toward its answer with a
//! line search that never lets a triangle flip.

use crate::sparse::{Triplets, solve};
use glam::{DVec2, DVec3};

/// The SLIM state for one chart (`SLIMData`).
pub(crate) struct SlimData {
    pub v: Vec<DVec3>,
    pub f: Vec<[usize; 3]>,
    /// Soft-constrained (pinned) vertices and their positions.
    pub b: Vec<usize>,
    pub bc: Vec<DVec2>,
    pub soft_const_p: f64,
    /// Current map.
    pub v_o: Vec<DVec2>,
    pub weight_per_face: Vec<f64>,
    pub with_weights: bool,
    pub energy: f64,
    pub expected_map_area: f64,
    pub skip_initialization: bool,
    pub reflection_mode: i32,
    // Internal.
    m: Vec<f64>,
    mesh_area: f64,
    proximal_p: f64,
    /// Surface gradient rows per face: coefficient for each of its three
    /// vertices (`Dx`, `Dy`).
    dx: Vec<[f64; 3]>,
    dy: Vec<[f64; 3]>,
    ji: Vec<[f64; 4]>,
    ri: Vec<[f64; 4]>,
    w: Vec<[f64; 4]>,
}

/// SLIM gave up (non-finite numbers).
#[derive(Debug)]
pub(crate) struct SlimFailed;

impl SlimData {
    pub(crate) fn new(v: Vec<DVec3>, f: Vec<[usize; 3]>, v_o: Vec<DVec2>) -> Self {
        Self {
            v,
            f,
            b: Vec::new(),
            bc: Vec::new(),
            soft_const_p: 0.0,
            v_o,
            weight_per_face: Vec::new(),
            with_weights: false,
            energy: 0.0,
            expected_map_area: 0.0,
            skip_initialization: false,
            reflection_mode: 0,
            m: Vec::new(),
            mesh_area: 0.0,
            proximal_p: 0.0001,
            dx: Vec::new(),
            dy: Vec::new(),
            ji: Vec::new(),
            ri: Vec::new(),
            w: Vec::new(),
        }
    }
}

/// Closed-form 2x2 SVD of `[[a, b], [c, d]]`: returns singular values
/// (descending, non-negative), the left singular vectors' rotation angle
/// and the angle of the closest rotation (`polar_svd`).
pub(crate) fn svd2(a: f64, b: f64, c: f64, d: f64) -> (f64, f64, f64, f64) {
    let e = (a + d) * 0.5;
    let f = (a - d) * 0.5;
    let g = (c + b) * 0.5;
    let h = (c - b) * 0.5;
    let q = (e * e + h * h).sqrt();
    let r = (f * f + g * g).sqrt();
    let a1 = g.atan2(f);
    let a2 = h.atan2(e);
    let phi = (a2 + a1) * 0.5;
    (q + r, (q - r).abs(), phi, a2)
}

/// Area of every triangle (`doublearea` / 2).
fn areas_3d(v: &[DVec3], f: &[[usize; 3]]) -> Vec<f64> {
    f.iter()
        .map(|t| 0.5 * (v[t[1]] - v[t[0]]).cross(v[t[2]] - v[t[0]]).length())
        .collect()
}

/// Sum of signed UV areas.
pub(crate) fn map_area(uv: &[DVec2], f: &[[usize; 3]]) -> f64 {
    f.iter()
        .map(|t| 0.5 * (uv[t[1]] - uv[t[0]]).perp_dot(uv[t[2]] - uv[t[0]]))
        .sum()
}

pub(crate) fn surface_area(v: &[DVec3], f: &[[usize; 3]]) -> f64 {
    areas_3d(v, f).iter().sum()
}

impl SlimData {
    /// `pre_calc`: surface gradient in each triangle's own 2D basis.
    fn pre_calc(&mut self) {
        let nf = self.f.len();
        self.dx = Vec::with_capacity(nf);
        self.dy = Vec::with_capacity(nf);
        for t in &self.f {
            let (p1, p2, p3) = (self.v[t[0]], self.v[t[1]], self.v[t[2]]);
            // grad(): edge vectors named after the opposite vertex.
            let v32 = p3 - p2;
            let v13 = p1 - p3;
            let v21 = p2 - p1;
            let n = v32.cross(v13);
            let dbl_a = n.length();
            let u = n / dbl_a;
            let eperp21 = u.cross(v21).normalize() * (v21.length() / dbl_a);
            let eperp13 = u.cross(v13).normalize() * (v13.length() / dbl_a);
            let g = [-(eperp13 + eperp21), eperp13, eperp21];
            // local_basis().
            let b1 = (p2 - p1).normalize();
            let b3 = b1.cross(p3 - p1).normalize();
            let b2 = -b1.cross(b3).normalize();
            self.dx.push(g.map(|gk| b1.dot(gk)));
            self.dy.push(g.map(|gk| b2.dot(gk)));
        }
        self.ji = vec![[0.0; 4]; nf];
        self.ri = vec![[0.0; 4]; nf];
        self.w = vec![[0.0; 4]; nf];
    }

    fn compute_jacobians(&self, uv: &[DVec2], out: &mut [[f64; 4]]) {
        for (i, t) in self.f.iter().enumerate() {
            let (dx, dy) = (self.dx[i], self.dy[i]);
            let mut j = [0.0; 4];
            for k in 0..3 {
                let p = uv[t[k]];
                j[0] += dx[k] * p.x;
                j[1] += dy[k] * p.x;
                j[2] += dx[k] * p.y;
                j[3] += dy[k] * p.y;
            }
            if self.with_weights {
                let w = self.weight_per_face[i];
                for x in &mut j {
                    *x *= w;
                }
            }
            out[i] = j;
        }
    }

    fn update_weights_and_closest_rotations(&mut self, uv: &[DVec2]) {
        let mut ji = std::mem::take(&mut self.ji);
        self.compute_jacobians(uv, &mut ji);
        let eps = 1e-8;
        for (i, j) in ji.iter().enumerate() {
            let (s1, s2, phi, rot) = svd2(j[0], j[1], j[2], j[3]);
            let weight = |s: f64| {
                if (s - 1.0).abs() < eps {
                    1.0
                } else {
                    let g = 2.0 * (s - s.powi(-3));
                    (g / (2.0 * (s - 1.0))).sqrt()
                }
            };
            let (m1, m2) = (weight(s1), weight(s2));
            // W = U diag(m) U^T with U a rotation by phi.
            let (sp, cp) = phi.sin_cos();
            let w11 = m1 * cp * cp + m2 * sp * sp;
            let w12 = (m1 - m2) * cp * sp;
            let w22 = m1 * sp * sp + m2 * cp * cp;
            self.w[i] = [w11, w12, w12, w22];
            // Closest rotation, stored column by column like Blender.
            let (sr, cr) = rot.sin_cos();
            self.ri[i] = [cr, sr, -sr, cr];
        }
        self.ji = ji;
    }

    /// Builds and solves the global step (formulas 35 and 36 of the paper).
    fn solve_weighted_arap(&self) -> Result<Vec<DVec2>, SlimFailed> {
        let n = self.v.len();
        let mut l = Triplets::new(2 * n, 2 * n);
        let mut rhs = vec![0.0; 2 * n];
        for (i, t) in self.f.iter().enumerate() {
            let (dx, dy) = (self.dx[i], self.dy[i]);
            let wt = if self.with_weights {
                self.weight_per_face[i]
            } else {
                1.0
            };
            let [w11, w12, w21, w22] = self.w[i];
            let r = self.ri[i];
            let area = self.m[i];
            let rows: [([f64; 3], f64, f64, f64); 4] = [
                (dx, w11, w12, w11 * r[0] + w12 * r[1]),
                (dy, w11, w12, w11 * r[2] + w12 * r[3]),
                (dx, w21, w22, w21 * r[0] + w22 * r[1]),
                (dy, w21, w22, w21 * r[2] + w22 * r[3]),
            ];
            for (d, wu, wv, frhs) in rows {
                // Six entries: u of the three vertices, then v.
                let mut idx = [0usize; 6];
                let mut val = [0.0f64; 6];
                for k in 0..3 {
                    idx[k] = t[k];
                    idx[k + 3] = n + t[k];
                    val[k] = wt * d[k] * wu;
                    val[k + 3] = wt * d[k] * wv;
                }
                for a in 0..6 {
                    rhs[idx[a]] += area * val[a] * frhs;
                    for b in 0..6 {
                        l.add(idx[a], idx[b], area * val[a] * val[b]);
                    }
                }
            }
        }
        for k in 0..n {
            l.add(k, k, self.proximal_p);
            l.add(n + k, n + k, self.proximal_p);
            rhs[k] += self.proximal_p * self.v_o[k].x;
            rhs[n + k] += self.proximal_p * self.v_o[k].y;
        }
        for (i, &vi) in self.b.iter().enumerate() {
            rhs[vi] += self.soft_const_p * self.bc[i].x;
            rhs[n + vi] += self.soft_const_p * self.bc[i].y;
            l.add(vi, vi, self.soft_const_p);
            l.add(n + vi, n + vi, self.soft_const_p);
        }
        let x = solve(&l, &[rhs], true).ok_or(SlimFailed)?;
        let x = &x[0];
        Ok((0..n).map(|k| DVec2::new(x[k], x[n + k])).collect())
    }

    /// Energy of `uv`; also returns the singular values per face.
    fn compute_energy(&self, uv: &[DVec2], singular: Option<&mut Vec<(f64, f64)>>) -> f64 {
        let mut ji = vec![[0.0; 4]; self.f.len()];
        self.compute_jacobians(uv, &mut ji);
        let mut energy = 0.0;
        let mut sv = Vec::new();
        for (i, j) in ji.iter().enumerate() {
            let (s1, s2, _, _) = svd2(j[0], j[1], j[2], j[3]);
            energy += self.m[i] * (s1 * s1 + s1.powi(-2) + s2 * s2 + s2.powi(-2));
            sv.push((s1, s2));
        }
        for (i, &vi) in self.b.iter().enumerate() {
            energy += self.soft_const_p * (self.bc[i] - uv[vi]).length_squared();
        }
        if let Some(out) = singular {
            *out = sv;
        }
        energy
    }

    /// `slim_precompute`.
    pub(crate) fn precompute(&mut self) {
        self.m = areas_3d(&self.v, &self.f);
        self.mesh_area = self.m.iter().sum();
        self.proximal_p = 0.0001;
        if self.weight_per_face.len() != self.f.len() {
            self.weight_per_face = vec![1.0; self.f.len()];
        }
        self.pre_calc();
        let v_o = self.v_o.clone();
        self.energy = self.compute_energy(&v_o, None) / self.mesh_area;
    }

    /// `slim_solve`: runs `iterations` steps.
    pub(crate) fn solve(&mut self, iterations: usize) -> Result<(), SlimFailed> {
        let pins = !self.b.is_empty();
        let mut singular: Vec<(f64, f64)> = Vec::new();
        if pins {
            let v_o = self.v_o.clone();
            self.energy = self.compute_energy(&v_o, Some(&mut singular)) / self.mesh_area;
        }
        for _ in 0..iterations {
            let dest = self.v_o.clone();
            self.update_weights_and_closest_rotations(&dest);
            let dest = self.solve_weighted_arap()?;
            if dest.iter().any(|p| !p.is_finite()) {
                return Err(SlimFailed);
            }
            let start_energy = self.energy * self.mesh_area;
            let mut cur = std::mem::take(&mut self.v_o);
            let e = {
                let this = &*self;
                flip_avoiding_line_search(&this.f, &mut cur, &dest, start_energy, |x| {
                    this.compute_energy(x, if pins { Some(&mut singular) } else { None })
                })?
            };
            self.v_o = cur;
            self.energy = e / self.mesh_area;
            if pins {
                let factor = global_scale_invariance_factor(&singular, &self.m);
                for d in self.dx.iter_mut().chain(self.dy.iter_mut()) {
                    for x in d.iter_mut() {
                        *x /= factor;
                    }
                }
                let v_o = self.v_o.clone();
                self.energy = self.compute_energy(&v_o, Some(&mut singular)) / self.mesh_area;
            }
        }
        Ok(())
    }
}

/// The scale `1 / x` that minimises `a x^2 + b / x^2` over the map (see the
/// derivation in Blender's `computeGlobalScaleInvarianceFactor`).
fn global_scale_invariance_factor(singular: &[(f64, f64)], areas: &[f64]) -> f64 {
    let (mut a, mut b) = (0.0, 0.0);
    for (&(s1, s2), &area) in singular.iter().zip(areas) {
        a += area * (s1 * s1 + s2 * s2);
        b += area * (1.0 / (s1 * s1) + 1.0 / (s2 * s2));
    }
    let x = (b / a).sqrt().sqrt();
    1.0 / x
}

fn smallest_pos_quad_zero(a: f64, b: f64, c: f64) -> Result<f64, SlimFailed> {
    let (t1, t2);
    if a != 0.0 {
        let delta_in = b * b - 4.0 * a * c;
        if delta_in < 0.0 {
            return Ok(f64::INFINITY);
        }
        let delta = delta_in.sqrt();
        t1 = (-b + delta) / (2.0 * a);
        t2 = (-b - delta) / (2.0 * a);
    } else {
        // Blender divides -b by c here; kept as is.
        t1 = -b / c;
        t2 = t1;
    }
    if !t1.is_finite() || !t2.is_finite() {
        return Err(SlimFailed);
    }
    let (hi, lo) = (t1.max(t2), t1.min(t2));
    if hi == lo {
        return Ok(f64::INFINITY);
    }
    if hi > 0.0 {
        return Ok(if lo > 0.0 { lo } else { hi });
    }
    Ok(f64::INFINITY)
}

/// The smallest step along `d` at which a triangle collapses.
fn max_step_from_singularities(
    uv: &[DVec2],
    f: &[[usize; 3]],
    d: &[DVec2],
) -> Result<f64, SlimFailed> {
    let mut max_step = f64::INFINITY;
    for t in f {
        let (u1, u2, u3) = (uv[t[0]], uv[t[1]], uv[t[2]]);
        let (v1, v2, v3) = (d[t[0]], d[t[1]], d[t[2]]);
        let a = v1.x * v2.y - v1.y * v2.x - v1.x * v3.y + v1.y * v3.x + v2.x * v3.y - v2.y * v3.x;
        let b = u1.x * v2.y - u1.y * v2.x - u2.x * v1.y + u2.y * v1.x - u1.x * v3.y
            + u1.y * v3.x
            + u3.x * v1.y
            - u3.y * v1.x
            + u2.x * v3.y
            - u2.y * v3.x
            - u3.x * v2.y
            + u3.y * v2.x;
        let c = u1.x * u2.y - u1.y * u2.x - u1.x * u3.y + u1.y * u3.x + u2.x * u3.y - u2.y * u3.x;
        max_step = max_step.min(smallest_pos_quad_zero(a, b, c)?);
    }
    Ok(max_step)
}

/// Bisection line search from `x` toward `dest`, starting no further than
/// 80% of the way to the first triangle flip. Returns the new energy.
fn flip_avoiding_line_search(
    f: &[[usize; 3]],
    x: &mut Vec<DVec2>,
    dest: &[DVec2],
    cur_energy: f64,
    mut energy: impl FnMut(&[DVec2]) -> f64,
) -> Result<f64, SlimFailed> {
    let d: Vec<DVec2> = dest.iter().zip(x.iter()).map(|(a, b)| *a - *b).collect();
    let min_step = max_step_from_singularities(x, f, &d)?;
    let mut step = (min_step * 0.8).min(1.0);
    let old_energy = if cur_energy > 0.0 {
        cur_energy
    } else {
        energy(x)
    };
    let mut new_energy = old_energy;
    let mut iter = 0;
    while new_energy >= old_energy && iter < 12 {
        let new_x: Vec<DVec2> = x.iter().zip(&d).map(|(p, dp)| *p + *dp * step).collect();
        let e = energy(&new_x);
        if e >= old_energy {
            step /= 2.0;
        } else {
            *x = new_x;
            new_energy = e;
        }
        iter += 1;
    }
    Ok(new_energy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratic_step_selects_the_first_positive_root() {
        assert!((smallest_pos_quad_zero(1.0, -5.0, 6.0).unwrap() - 2.0).abs() < 1e-12);
        assert!((smallest_pos_quad_zero(1.0, 1.0, -2.0).unwrap() - 1.0).abs() < 1e-12);
        for (a, b, c) in [(1.0, 0.0, 1.0), (1.0, 5.0, 6.0), (1.0, -2.0, 1.0)] {
            assert_eq!(smallest_pos_quad_zero(a, b, c).unwrap(), f64::INFINITY);
        }
        assert!(smallest_pos_quad_zero(0.0, 1.0, 0.0).is_err());
    }

    #[test]
    fn line_search_halves_a_step_that_would_raise_energy() {
        let faces = [[0, 1, 2]];
        let mut uv = vec![DVec2::ZERO, DVec2::X, DVec2::Y];
        let dest = [DVec2::ZERO, 2.0 * DVec2::X, -DVec2::Y];
        let energy = |uv: &[DVec2]| (uv[2].y - 0.75).powi(2);
        let initial_energy = energy(&uv);

        // The triangle collapses at step 0.5. The capped step (0.4)
        // raises the energy, so the search accepts the next step (0.2).
        let result =
            flip_avoiding_line_search(&faces, &mut uv, &dest, initial_energy, energy).unwrap();
        assert!((result - 0.0225).abs() < 1e-12);
        assert!((uv[1] - 1.2 * DVec2::X).length() < 1e-12);
        assert!((uv[2] - 0.6 * DVec2::Y).length() < 1e-12);
        assert!((map_area(&uv, &faces) - 0.36).abs() < 1e-12);
    }

    #[test]
    fn svd2_matches_definition() {
        for &(a, b, c, d) in &[
            (2.0, 0.5, -0.3, 1.0),
            (0.0, 1.0, 1.0, 0.0),
            (1.0, 2.0, 3.0, 4.0),
        ] {
            let (s1, s2, phi, rot) = svd2(a, b, c, d);
            assert!(s1 >= s2 && s2 >= 0.0);
            // Singular values: s1 s2 = |det|, s1^2 + s2^2 = |A|_F^2.
            let det: f64 = a * d - b * c;
            assert!((s1 * s2 - det.abs()).abs() < 1e-12);
            assert!((s1 * s1 + s2 * s2 - (a * a + b * b + c * c + d * d)).abs() < 1e-12);
            // U's first column is the direction A stretches most.
            let u = DVec2::new(phi.cos(), phi.sin());
            let at_u = DVec2::new(a * u.x + c * u.y, b * u.x + d * u.y); // A^T u
            assert!((at_u.length() - s1).abs() < 1e-9);
            let _ = rot;
        }
    }
}
