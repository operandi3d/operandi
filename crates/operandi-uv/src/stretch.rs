// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/geometry/intern/uv_parametrizer.cc`
// (the Stretch section) and `source/blender/blenlib/intern/rand.cc` at commit
// b4eed6fd498e76a7e93404bac2326e9f84deba22.

//! Iterative stretch minimisation (Blender's Minimize Stretch): each pass
//! nudges every free vertex in a random direction and keeps the position
//! with the lowest Sander stretch of its triangles. Borders stay put.

use crate::chart::{FACE_FILLED, Param, VERT_PIN, VERT_SELECT};
use glam::DVec2;

const STRETCH_ITER: usize = 20;

/// The 48-bit linear congruential generator Blender's `RNG` uses, so passes
/// are repeatable.
#[derive(Clone, Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u32) -> Self {
        Rng(((seed as u64) << 16) | 0x330E)
    }

    fn step(&mut self) -> u64 {
        self.0 = (self.0.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & ((1u64 << 48) - 1);
        self.0
    }

    pub(crate) fn float(&mut self) -> f64 {
        let i = (self.step() >> 17) & 0x7FFF_FFFF;
        i as f64 / 2147483648.0
    }
}

impl Param {
    /// `p_face_stretch`: Sander's L2 stretch of one triangle.
    fn face_stretch(&self, f: usize) -> f64 {
        let area = self.face_uv_area_signed(f);
        let [e1, e2, e3] = self.face_edges(f);
        if area <= 0.0 {
            // A flipped face: big penalty, with a slope toward unflipping.
            return 1e8
                * (1.0
                    + self.edge_uv_length(e1)
                    + self.edge_uv_length(e2)
                    + self.edge_uv_length(e3));
        }
        let [a, b, c] = self.face_verts(f).map(|v| &self.verts[v]);
        let w = 1.0 / (2.0 * area);
        let ps =
            (a.co * (b.uv.y - c.uv.y) + b.co * (c.uv.y - a.uv.y) + c.co * (a.uv.y - b.uv.y)) * w;
        let pt =
            (a.co * (c.uv.x - b.uv.x) + b.co * (a.uv.x - c.uv.x) + c.co * (b.uv.x - a.uv.x)) * w;
        let mut t = (0.5 * (ps.dot(ps) + pt.dot(pt))).sqrt();
        if self.faces[f].flag & FACE_FILLED != 0 {
            t *= 0.2;
        }
        t
    }

    fn vertex_stretch(&self, v: usize) -> f64 {
        self.wheel(v)
            .into_iter()
            .map(|e| self.face_stretch(self.edges[e].face))
            .sum()
    }

    fn stretch_minimize_chart(&mut self, c: usize) {
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            let flag = self.verts[v].flag;
            if flag & VERT_PIN != 0 || flag & VERT_SELECT == 0 {
                continue;
            }
            let orig_stretch = self.vertex_stretch(v);
            let orig_uv = self.verts[v].uv;
            let wheel = self.wheel(v);
            let trusted: f64 = wheel.iter().map(|&e| self.edge_uv_length(e)).sum::<f64>()
                / (2 * wheel.len()) as f64;
            let angle = self.rng.float() * std::f64::consts::TAU;
            let dir = DVec2::new(trusted * angle.cos(), trusted * angle.sin());

            let (mut low, mut high) = (0.0, 1.0);
            let mut stretch_low = orig_stretch;
            self.verts[v].uv = orig_uv + dir;
            let mut stretch_high = self.vertex_stretch(v);
            let mut stretch = stretch_high;
            for _ in 0..STRETCH_ITER {
                let mid = 0.5 * (low + high);
                self.verts[v].uv = orig_uv + dir * mid;
                stretch = self.vertex_stretch(v);
                if stretch_low < stretch_high {
                    high = mid;
                    stretch_high = stretch;
                } else {
                    low = mid;
                    stretch_low = stretch;
                }
            }
            if stretch >= orig_stretch {
                self.verts[v].uv = orig_uv;
            }
        }
    }

    /// Starts stretch minimisation (`uv_parametrizer_stretch_begin`): user
    /// pins are ignored and every chart's border is held instead.
    pub fn stretch_begin(&mut self) {
        self.rng = Rng::new(31415926);
        self.blend = 0.0;
        for c in 0..self.charts.len() {
            for i in 0..self.charts[c].verts.len() {
                let v = self.charts[c].verts[i];
                let border = self.edges[self.verts[v].edge].pair.is_none();
                if border {
                    self.verts[v].flag |= VERT_PIN;
                } else {
                    self.verts[v].flag &= !VERT_PIN;
                }
            }
            self.backup_uvs(c);
        }
    }

    /// Blend between the result (0) and the starting UVs (1) on flush.
    pub fn set_blend(&mut self, blend: f64) {
        self.blend = blend.clamp(0.0, 1.0);
    }

    /// One stretch pass over every chart (`uv_parametrizer_stretch_iter`).
    pub fn stretch_iter(&mut self) {
        for c in 0..self.charts.len() {
            self.stretch_minimize_chart(c);
        }
    }
}
