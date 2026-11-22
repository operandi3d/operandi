// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/geometry/intern/uv_parametrizer.cc`
// (Least Squares Conformal Maps, pin choice, minimum-area rotation, average
// island scale, original bounds) at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Least Squares Conformal Maps (Lévy et al. 2002), in the angle-based form
//! Blender uses, so ABF++ angles can feed it.

use crate::chart::{Param, V, VERT_PIN, VERT_SELECT, VERT_SPLIT};
use crate::geom;
use crate::sparse::LeastSquares;
use glam::{DVec2, DVec3};

impl Param {
    /// `p_chart_pin_positions`: place two automatic pins from their 3D
    /// coordinates along the two longest axes between them.
    fn pin_positions(&mut self, c: usize, pin1: &mut Option<V>, pin2: &mut Option<V>) {
        match (*pin1, *pin2) {
            (Some(a), Some(b)) if a != b => {
                let d = (self.verts[a].co - self.verts[b].co).abs();
                let (dirx, diry) = if d.x > d.y && d.x > d.z {
                    (0, if d.y > d.z { 1 } else { 2 })
                } else if d.y > d.x && d.y > d.z {
                    (1, if d.x > d.z { 0 } else { 2 })
                } else {
                    (2, if d.x > d.y { 0 } else { 1 })
                };
                let (diru, dirv) = if dirx == 2 { (1, 0) } else { (0, 1) };
                for v in [a, b] {
                    let co = self.verts[v].co;
                    let mut uv = self.verts[v].uv;
                    uv[diru] = co[dirx];
                    uv[dirv] = co[diry];
                    self.verts[v].uv = uv;
                }
            }
            _ => {
                // Degenerate case.
                let f = self.charts[c].faces[0];
                let e = self.faces[f].edge;
                let a = self.edges[e].vert;
                let b = self.edges[self.edges[e].next].vert;
                self.verts[a].uv = DVec2::new(0.0, 0.5);
                self.verts[b].uv = DVec2::new(1.0, 0.5);
                *pin1 = Some(a);
                *pin2 = Some(b);
            }
        }
    }

    /// `p_chart_symmetry_pins`: on a chart cut along a seam, pin the middle
    /// of the seam and the point opposite it, which keeps symmetric charts
    /// symmetric.
    fn symmetry_pins(&mut self, c: usize, outer: usize) -> Option<(V, V)> {
        let split = |p: &Param, e: usize| p.verts[p.edges[e].vert].flag & VERT_SPLIT != 0;
        let mut be = outer;
        let mut lastbe = self.boundary_prev(be);
        let (mut maxlen, mut curlen, mut totlen, mut firstlen) = (0.0, 0.0, 0.0, 0.0);
        let (mut maxe1, mut maxe2) = (None, None);
        let (mut cure, mut firste1, mut firste2): (Option<usize>, Option<usize>, Option<usize>) =
            (None, None, None);
        let mut guard = 0;
        loop {
            totlen += self.edge_length(be);
            let nextbe = self.boundary_next(be);
            if split(self, be) || (split(self, lastbe) && split(self, nextbe)) {
                if cure.is_none() {
                    if be == outer {
                        firste1 = Some(be);
                    }
                    cure = Some(be);
                } else {
                    curlen += self.edge_length(lastbe);
                }
            } else if let Some(ce) = cure {
                if curlen > maxlen {
                    maxlen = curlen;
                    maxe1 = Some(ce);
                    maxe2 = Some(lastbe);
                }
                if firste1 == Some(ce) {
                    firstlen = curlen;
                    firste2 = Some(lastbe);
                }
                curlen = 0.0;
                cure = None;
            }
            lastbe = be;
            be = nextbe;
            guard += 1;
            if be == outer || guard > self.edges.len() {
                break;
            }
        }
        // Also count a run of splits over the starting point.
        if let Some(ce) = cure
            && ce != outer
        {
            firstlen += curlen + self.edge_length(be);
            if firstlen > maxlen {
                maxlen = firstlen;
                maxe1 = Some(ce);
                maxe2 = firste2;
            }
        }
        let (m1, m2) = (maxe1?, maxe2?);
        if maxlen < 0.5 * totlen {
            return None;
        }
        // pin1 in the middle of the split run.
        let (mut be1, mut be2) = (m1, m2);
        let (mut len1, mut len2) = (0.0, 0.0);
        guard = 0;
        while be1 != be2 && guard <= self.edges.len() {
            if len1 < len2 {
                len1 += self.edge_length(be1);
                be1 = self.boundary_next(be1);
            } else {
                be2 = self.boundary_prev(be2);
                len2 += self.edge_length(be2);
            }
            guard += 1;
        }
        let p1 = self.edges[be1].vert;
        // pin2 in the middle of the rest.
        let (mut be1, mut be2) = (m1, m2);
        let (mut len1, mut len2) = (0.0, 0.0);
        guard = 0;
        while be1 != be2 && guard <= self.edges.len() {
            if len1 < len2 {
                be1 = self.boundary_prev(be1);
                len1 += self.edge_length(be1);
            } else {
                len2 += self.edge_length(be2);
                be2 = self.boundary_next(be2);
            }
            guard += 1;
        }
        let p2 = self.edges[be1].vert;
        let (mut a, mut b) = (Some(p1), Some(p2));
        self.pin_positions(c, &mut a, &mut b);
        let (a, b) = (a?, b?);
        (self.verts[a].co != self.verts[b].co).then_some((a, b))
    }

    /// `p_chart_extrema_verts`: pins at the two ends of the longest axis.
    fn extrema_verts(&mut self, c: usize) -> (V, V) {
        let mut minv = DVec3::splat(1e20);
        let mut maxv = DVec3::splat(-1e20);
        let mut minvert = [None; 3];
        let mut maxvert = [None; 3];
        for &v in &self.charts[c].verts {
            let co = self.verts[v].co;
            for i in 0..3 {
                if co[i] < minv[i] {
                    minv[i] = co[i];
                    minvert[i] = Some(v);
                }
                if co[i] > maxv[i] {
                    maxv[i] = co[i];
                    maxvert[i] = Some(v);
                }
            }
        }
        let mut dir = 0;
        let mut dirlen = -1.0;
        for i in 0..3 {
            if maxv[i] - minv[i] > dirlen {
                dir = i;
                dirlen = maxv[i] - minv[i];
            }
        }
        let (mut a, mut b) = (minvert[dir], maxvert[dir]);
        self.pin_positions(c, &mut a, &mut b);
        (a.unwrap(), b.unwrap())
    }

    /// `p_chart_lscm_begin`.
    pub(crate) fn lscm_begin(&mut self, c: usize, live: bool, abf: bool, original_bounds: bool) {
        if original_bounds {
            let pts = self.chart_uv_points(c);
            let angle = geom::minimum_area_angle(&pts);
            let (lo, hi) = self.chart_uv_bbox(c);
            self.charts[c].orig_bounds = Some((lo, hi, angle));
        }
        let (mut select, mut deselect, mut npins) = (false, false, 0);
        for &v in &self.charts[c].verts {
            let flag = self.verts[v].flag;
            if flag & VERT_PIN != 0 {
                npins += 1;
                if flag & VERT_SELECT != 0 {
                    select = true;
                }
            }
            if flag & VERT_SELECT == 0 {
                deselect = true;
            }
        }
        if live && (!select || !deselect) {
            self.charts[c].skip_flush = true;
            return;
        }
        if npins == 1 {
            self.charts[c].area_uv = self.chart_uv_area(c);
            self.charts[c].single_pin = self.charts[c]
                .verts
                .iter()
                .copied()
                .find(|&v| self.verts[v].flag & VERT_PIN != 0);
        }
        if abf && !self.abf_solve(c) {
            // ABF failed: fall back to plain LSCM.
            self.charts[c].abf_alpha = None;
        }
        for (i, &v) in self.charts[c].verts.clone().iter().enumerate() {
            self.verts[v].id = i;
        }
        if npins <= 1 {
            let (_, outer) = self.chart_boundaries(c);
            let pins = outer
                .and_then(|o| self.symmetry_pins(c, o))
                .unwrap_or_else(|| self.extrema_verts(c));
            self.charts[c].pin1 = Some(pins.0);
            self.charts[c].pin2 = Some(pins.1);
        }
        self.charts[c].lscm_ready = true;
    }

    /// `p_chart_lscm_solve`.
    pub(crate) fn lscm_solve_chart(&mut self, c: usize) -> bool {
        let nverts = self.charts[c].verts.len();
        let nfaces = self.charts[c].faces.len();
        for i in 0..nverts {
            let v = self.charts[c].verts[i];
            if self.verts[v].flag & VERT_PIN != 0 {
                // Reload for live unwrap.
                self.vert_load_pin_select_uvs(v);
            }
        }
        if let Some(p) = self.charts[c].single_pin {
            self.charts[c].origin = self.verts[p].uv;
        }
        let mut ls = LeastSquares::new(2 * nfaces, 2 * nverts);
        if let (Some(p1), Some(p2)) = (self.charts[c].pin1, self.charts[c].pin2) {
            for p in [p1, p2] {
                let id = self.verts[p].id;
                ls.lock(2 * id, self.verts[p].uv.x);
                ls.lock(2 * id + 1, self.verts[p].uv.y);
            }
        } else {
            for &v in &self.charts[c].verts {
                if self.verts[v].flag & VERT_PIN != 0 {
                    let id = self.verts[v].id;
                    ls.lock(2 * id, self.verts[v].uv.x);
                    ls.lock(2 * id + 1, self.verts[v].uv.y);
                }
            }
        }

        // Detect "up" from the pinned triangles.
        let (mut up, mut down) = (0.0, 0.0);
        for &f in &self.charts[c].faces {
            let vs = self.face_verts(f);
            if vs.iter().all(|&v| self.verts[v].flag & VERT_PIN != 0) {
                let a = self.face_uv_area_signed(f);
                if a > 0.0 {
                    up += a;
                } else {
                    down -= a;
                }
            }
        }
        let flip_faces = down > up;

        let alpha = self.charts[c].abf_alpha.clone();
        for (row, &f) in self.charts[c].faces.iter().enumerate() {
            let [mut v1, mut v2, mut v3] = self.face_verts(f);
            let [mut a1, mut a2, mut a3] = match &alpha {
                Some(al) => [al[3 * row], al[3 * row + 1], al[3 * row + 2]],
                None => self.face_angles(f),
            };
            if flip_faces {
                std::mem::swap(&mut a2, &mut a3);
                std::mem::swap(&mut v2, &mut v3);
            }
            let (mut s1, mut s2, mut s3) = (a1.sin(), a2.sin(), a3.sin());
            let sinmax = s1.max(s2).max(s3);
            // Shift vertices to the most stable order.
            if s3 != sinmax {
                (v1, v2, v3) = (v3, v1, v2);
                (a1, a2, a3) = (a3, a1, a2);
                (s1, s2, s3) = (s3, s1, s2);
                if s2 == sinmax {
                    (v1, v2, v3) = (v3, v1, v2);
                    (a1, a2, a3) = (a3, a1, a2);
                    (s1, s2, s3) = (s3, s1, s2);
                }
            }
            let _ = (a2, a3);
            let ratio = if s3 == 0.0 { 1.0 } else { s2 / s3 };
            let cosine = a1.cos() * ratio;
            let sine = s1 * ratio;
            let (i1, i2, i3) = (self.verts[v1].id, self.verts[v2].id, self.verts[v3].id);
            let r = 2 * row;
            ls.add(r, 2 * i1, cosine - 1.0);
            ls.add(r, 2 * i1 + 1, -sine);
            ls.add(r, 2 * i2, -cosine);
            ls.add(r, 2 * i2 + 1, sine);
            ls.add(r, 2 * i3, 1.0);
            ls.add(r + 1, 2 * i1, sine);
            ls.add(r + 1, 2 * i1 + 1, cosine - 1.0);
            ls.add(r + 1, 2 * i2, -sine);
            ls.add(r + 1, 2 * i2 + 1, -cosine);
            ls.add(r + 1, 2 * i3 + 1, 1.0);
        }

        match ls.solve() {
            Some(x) => {
                for i in 0..nverts {
                    let v = self.charts[c].verts[i];
                    let id = self.verts[v].id;
                    self.verts[v].uv = DVec2::new(x[2 * id], x[2 * id + 1]);
                }
                true
            }
            None => {
                for i in 0..nverts {
                    let v = self.charts[c].verts[i];
                    self.verts[v].uv = DVec2::ZERO;
                }
                false
            }
        }
    }

    fn lscm_transform_single_pin(&mut self, c: usize) {
        let Some(pin) = self.charts[c].single_pin else {
            return;
        };
        // Keep the UV area the same.
        let new_area = self.chart_uv_area(c);
        if new_area > 0.0 {
            let scale = self.charts[c].area_uv / new_area;
            if scale > 0.0 {
                self.chart_uv_scale(c, scale.sqrt());
            }
        }
        // Keep the pinned UV in place.
        let offset = self.charts[c].origin - self.verts[pin].uv;
        self.chart_uv_translate(c, offset);
    }

    fn lscm_end_chart(&mut self, c: usize) {
        let ch = &mut self.charts[c];
        ch.lscm_ready = false;
        ch.abf_alpha = None;
        ch.pin1 = None;
        ch.pin2 = None;
        ch.single_pin = None;
    }

    /// `p_chart_rotate_minimum_area`.
    pub(crate) fn rotate_minimum_area(&mut self, c: usize) {
        let angle = geom::minimum_area_angle(&self.chart_uv_points(c));
        let (s, co) = angle.sin_cos();
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            let uv = self.verts[v].uv;
            self.verts[v].uv = DVec2::new(co * uv.x - s * uv.y, s * uv.x + co * uv.y);
        }
    }

    /// `p_chart_rotate_fit_aabb`.
    fn rotate_fit_aabb(&mut self, c: usize) {
        let sc = geom::aabb_fit_sincos(&self.chart_uv_points(c));
        if sc != DVec2::new(0.0, 1.0) {
            // Columns of Blender's {{cos, sin}, {-sin, cos}} with (sin, cos) = sc.
            self.chart_uv_transform(c, DVec2::new(sc.y, sc.x), DVec2::new(-sc.x, sc.y));
        }
    }

    /// Prepares every chart for LSCM (`uv_parametrizer_lscm_begin`). `abf`
    /// first flattens angles with ABF++ (Angle Based); otherwise plain
    /// conformal LSCM.
    pub fn lscm_begin_all(&mut self, live: bool, abf: bool, original_bounds: bool) {
        for c in 0..self.charts.len() {
            self.backup_uvs(c);
            self.lscm_begin(c, live, abf, original_bounds);
        }
    }

    /// Solves every prepared chart (`uv_parametrizer_lscm_solve`). Returns
    /// (charts changed, charts failed).
    pub fn lscm_solve(&mut self) -> (usize, usize) {
        let (mut changed, mut failed) = (0, 0);
        for c in 0..self.charts.len() {
            if !self.charts[c].lscm_ready {
                continue;
            }
            let ok = self.lscm_solve_chart(c);
            if ok && !self.charts[c].has_pins {
                self.rotate_minimum_area(c);
            } else if ok && self.charts[c].single_pin.is_some() {
                self.rotate_fit_aabb(c);
                self.lscm_transform_single_pin(c);
            }
            if !ok || !self.charts[c].has_pins {
                self.lscm_end_chart(c);
            }
            if ok {
                changed += 1;
            } else {
                failed += 1;
            }
        }
        (changed, failed)
    }

    /// `uv_parametrizer_lscm_end`.
    pub fn lscm_end(&mut self) {
        for c in 0..self.charts.len() {
            self.lscm_end_chart(c);
        }
    }

    /// Scales charts so their UV areas match their 3D areas relative to each
    /// other, keeping each chart's centre (`uv_parametrizer_average`).
    pub fn average(&mut self, ignore_pinned: bool, scale_uv: bool, shear: bool) {
        if self.charts.is_empty() {
            return;
        }
        let (mut tot3d, mut totuv) = (0.0, 0.0);
        for c in 0..self.charts.len() {
            if ignore_pinned && self.charts[c].has_pins {
                continue;
            }
            let (lo, hi) = self.chart_uv_bbox(c);
            self.charts[c].origin = (lo + hi) * 0.5;
            if scale_uv || shear {
                for _ in 0..10 {
                    let (mut cou_s, mut cov_s, mut cross, mut wsum) = (0.0, 0.0, 0.0, 0.0);
                    for &f in &self.charts[c].faces {
                        let [va, vb, vc] = self.face_verts(f).map(|v| &self.verts[v]);
                        let s0 = va.uv - vc.uv;
                        let s1 = vb.uv - vc.uv;
                        let det = s0.x * s1.y - s1.x * s0.y;
                        if det == 0.0 {
                            continue;
                        }
                        // Inverse of the column matrix [s0 s1] (Blender's
                        // `invert_m2_m2` on m[col][row]).
                        let m00 = s1.y / det;
                        let m01 = -s0.y / det;
                        let m10 = -s1.x / det;
                        let m11 = s0.x / det;
                        let da = va.co - vc.co;
                        let db = vb.co - vc.co;
                        let mut cou = da * m00 + db * m01;
                        let mut cov = da * m10 + db * m11;
                        let w = self.face_area(f);
                        cou_s += cou.length() * w;
                        cov_s += cov.length() * w;
                        if shear {
                            cou = cou.normalize_or_zero();
                            cov = cov.normalize_or_zero();
                            cross += cou.dot(cov) * w;
                        }
                        wsum += w;
                    }
                    if cou_s * cov_s < 1e-10 {
                        break;
                    }
                    let su = if scale_uv {
                        (cou_s / cov_s).sqrt()
                    } else {
                        1.0
                    };
                    let t10 = (cross / wsum).clamp(-0.5, 0.5);
                    // t = [[su, 0], [t10, 1/su]] as columns t[0], t[1].
                    self.chart_uv_transform(c, DVec2::new(su, 0.0), DVec2::new(t10, 1.0 / su));
                    let err = (su - 1.0).abs() + t10.abs() + (1.0 / su - 1.0).abs();
                    if err < 1e-6 {
                        break;
                    }
                }
            }
            let a3: f64 = self.charts[c]
                .faces
                .iter()
                .map(|&f| self.face_area(f))
                .sum();
            let au = self.chart_uv_area(c);
            self.charts[c].area_3d = a3;
            self.charts[c].area_uv = au;
            tot3d += a3;
            totuv += au;
        }
        if tot3d == 0.0 || totuv == 0.0 {
            return;
        }
        let tot_fac = tot3d / totuv;
        for c in 0..self.charts.len() {
            if ignore_pinned && self.charts[c].has_pins {
                continue;
            }
            let (a3, au) = (self.charts[c].area_3d, self.charts[c].area_uv);
            if a3 != 0.0 && au != 0.0 {
                self.chart_uv_scale(c, (a3 / au / tot_fac).sqrt());
                let (lo, hi) = self.chart_uv_bbox(c);
                let t = self.charts[c].origin - (lo + hi) * 0.5;
                self.chart_uv_translate(c, t);
            }
        }
    }

    /// Fits each fully selected chart back into the box and orientation it
    /// had before unwrapping (`uv_parametrizer_original_bounds`).
    pub fn original_bounds(&mut self) {
        for c in 0..self.charts.len() {
            if self.charts[c]
                .verts
                .iter()
                .any(|&v| self.verts[v].flag & VERT_SELECT == 0)
            {
                continue;
            }
            let Some((olo, ohi, oangle)) = self.charts[c].orig_bounds else {
                continue;
            };
            let new_angle = geom::minimum_area_angle(&self.chart_uv_points(c));
            self.chart_uv_rotate(c, oangle - new_angle);
            let (lo, hi) = self.chart_uv_bbox(c);
            let new_size = hi - lo;
            let orig = ohi - olo;
            let size = orig.x.max(orig.y);
            let denom = new_size.x.max(new_size.y);
            let scale = if denom != 0.0 { size / denom } else { 0.0 };
            self.chart_uv_scale(c, scale);
            let (lo, hi) = self.chart_uv_bbox(c);
            let t = (olo + ohi) * 0.5 - (lo + hi) * 0.5;
            self.chart_uv_translate(c, t);
        }
    }
}
