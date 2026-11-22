// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/geometry/intern/uv_parametrizer.cc`
// (construction, chart splitting, boundary filling, loading and flushing)
// at commit b4eed6fd498e76a7e93404bac2326e9f84deba22. Pointers and linked
// lists became indices into arenas; the special-purpose hash became maps.

//! Charts: the half-edge surface the solvers work on, built from the input
//! polygons and cut apart at seams into connected pieces (islands).

use crate::geom;
use crate::{Corner, UvError, UvMesh};
use glam::{DVec2, DVec3};
use std::collections::HashMap;

pub(crate) type V = usize;
pub(crate) type E = usize;
pub(crate) type F = usize;

pub(crate) const VERT_PIN: u32 = 1;
pub(crate) const VERT_SELECT: u32 = 2;
pub(crate) const VERT_INTERIOR: u32 = 4;
pub(crate) const VERT_SPLIT: u32 = 16;

pub(crate) const EDGE_SEAM: u32 = 1;
pub(crate) const EDGE_VERTEX_SPLIT: u32 = 2;
pub(crate) const EDGE_PIN: u32 = 4;
pub(crate) const EDGE_SELECT: u32 = 8;
pub(crate) const EDGE_DONE: u32 = 16;
pub(crate) const EDGE_FILLED: u32 = 32;
/// Flags that belong to the corner (the edge's start vertex) and move with it
/// when a triangle is flipped. Blender moves only the pin flag; Operandi also
/// moves selection, which is per corner too.
const EDGE_VERTEX_FLAGS: u32 = EDGE_PIN | EDGE_SELECT;

pub(crate) const FACE_CONNECTED: u32 = 1;
pub(crate) const FACE_FILLED: u32 = 2;
pub(crate) const FACE_DONE: u32 = 8;

#[derive(Clone, Debug)]
pub(crate) struct Vert {
    pub key: u64,
    pub co: DVec3,
    pub uv: DVec2,
    pub edge: E,
    pub flag: u32,
    pub weight: f64,
    /// Scratch index for the solvers.
    pub id: usize,
    pub on_boundary: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Edge {
    pub vert: V,
    pub pair: Option<E>,
    pub next: E,
    pub face: F,
    /// Index of the input corner this half-edge starts at; `None` for edges
    /// of triangles added to fill holes.
    pub corner: Option<usize>,
    pub old_uv: DVec2,
    pub flag: u32,
    pub id: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Face {
    pub edge: E,
    pub flag: u32,
    /// Input polygon this triangle came from (`usize::MAX` for fill).
    pub source: usize,
    pub chart: usize,
    pub id: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Chart {
    pub verts: Vec<V>,
    pub edges: Vec<E>,
    pub faces: Vec<F>,
    pub nboundaries: usize,
    pub has_pins: bool,
    pub skip_flush: bool,
    pub area_uv: f64,
    pub area_3d: f64,
    pub origin: DVec2,
    pub single_pin: Option<V>,
    pub pin1: Option<V>,
    pub pin2: Option<V>,
    pub abf_alpha: Option<Vec<f64>>,
    /// LSCM set up for this chart (Blender's `chart->context`).
    pub lscm_ready: bool,
    pub orig_bounds: Option<(DVec2, DVec2, f64)>,
}

/// Options for building charts.
#[derive(Clone, Copy, Debug)]
pub struct ConstructOptions {
    /// Cap holes with temporary triangles while solving, so they keep their
    /// shape. No polygons are added to the mesh.
    pub fill_holes: bool,
    /// Cut wherever the current UVs are discontinuous, in addition to seams.
    pub topology_from_uvs: bool,
    /// Height / width of the texture; 1 for square.
    pub aspect_y: f64,
}

impl Default for ConstructOptions {
    fn default() -> Self {
        Self {
            fill_holes: true,
            topology_from_uvs: false,
            aspect_y: 1.0,
        }
    }
}

/// The parametrizer state (Blender's `ParamHandle`).
pub struct Param {
    pub(crate) verts: Vec<Vert>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) faces: Vec<Face>,
    pub(crate) charts: Vec<Chart>,
    /// Current UV of every input corner, read on load and written on flush.
    pub(crate) corner_uv: Vec<DVec2>,
    /// For each input face, the index of its first corner.
    pub(crate) face_first_corner: Vec<usize>,
    pub(crate) aspect_y: f64,
    pub(crate) blend: f64,
    /// Charts dropped while building (closed surfaces with no border).
    pub(crate) failed_charts: usize,
    pub(crate) rng: crate::stretch::Rng,
}

/// Key for an undirected edge between two vertex keys.
fn edge_key(a: u64, b: u64) -> (u64, u64) {
    if a < b { (a, b) } else { (b, a) }
}

struct Builder {
    vert_by_key: HashMap<u64, V>,
    edges_by_key: HashMap<(u64, u64), Vec<E>>,
}

impl Param {
    /// Builds charts from `mesh`. Faces need at least three corners and
    /// valid point indices.
    pub fn new(mesh: &UvMesh, options: ConstructOptions) -> Result<Self, UvError> {
        mesh.validate()?;
        let aspect_y = if options.aspect_y > 0.0 && options.aspect_y.is_finite() {
            options.aspect_y
        } else {
            1.0
        };
        let mut param = Param {
            verts: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
            charts: Vec::new(),
            corner_uv: Vec::new(),
            face_first_corner: Vec::new(),
            aspect_y,
            blend: 0.0,
            failed_charts: 0,
            rng: crate::stretch::Rng::new(31415926),
        };
        for face in &mesh.faces {
            param.face_first_corner.push(param.corner_uv.len());
            param.corner_uv.extend(face.iter().map(|c| c.uv));
        }

        let keys = pin_keys(mesh);
        let mut b = Builder {
            vert_by_key: HashMap::new(),
            edges_by_key: HashMap::new(),
        };
        for (fi, face) in mesh.faces.iter().enumerate() {
            let first = param.face_first_corner[fi];
            param.face_add(&mut b, mesh, fi, face, first, &keys);
        }
        // Seams: mark one half-edge of every seam edge (`edge_set_seam`).
        let seams: std::collections::HashSet<(u32, u32)> = mesh
            .seams
            .iter()
            .map(|&[a, b]| if a < b { (a, b) } else { (b, a) })
            .collect();
        if !seams.is_empty() {
            for (fi, face) in mesh.faces.iter().enumerate() {
                let first = param.face_first_corner[fi];
                for i in 0..face.len() {
                    let j = (i + 1) % face.len();
                    let (a, bb) = (face[i].point, face[j].point);
                    let k = if a < bb { (a, bb) } else { (bb, a) };
                    if seams.contains(&k) {
                        let ka = keys[first + i];
                        let kb = keys[first + j];
                        if let Some(e) = param.edge_lookup(&b, ka, kb) {
                            param.edges[e].flag |= EDGE_SEAM;
                        }
                    }
                }
            }
        }
        param.construct_end(&b, options);
        Ok(param)
    }

    pub(crate) fn wheel_next(&self, e: E) -> Option<E> {
        self.edges[self.edges[self.edges[e].next].next].pair
    }

    pub(crate) fn wheel_prev(&self, e: E) -> Option<E> {
        self.edges[e].pair.map(|p| self.edges[p].next)
    }

    pub(crate) fn boundary_next(&self, e: E) -> E {
        self.verts[self.edges[self.edges[e].next].vert].edge
    }

    pub(crate) fn boundary_prev(&self, e: E) -> E {
        let mut we = e;
        let mut last;
        loop {
            last = we;
            match self.wheel_next(we) {
                Some(n) if n != e => we = n,
                _ => break,
            }
        }
        self.edges[self.edges[last].next].next
    }

    pub(crate) fn vert_interior(&self, v: V) -> bool {
        self.edges[self.verts[v].edge].pair.is_some()
    }

    /// The edges around `v`, starting at `v.edge`.
    pub(crate) fn wheel(&self, v: V) -> Vec<E> {
        let start = self.verts[v].edge;
        let mut out = vec![start];
        let mut e = start;
        while let Some(n) = self.wheel_next(e) {
            if n == start || out.len() > self.edges.len() {
                break;
            }
            out.push(n);
            e = n;
        }
        out
    }

    pub(crate) fn face_verts(&self, f: F) -> [V; 3] {
        let e1 = self.faces[f].edge;
        let e2 = self.edges[e1].next;
        let e3 = self.edges[e2].next;
        [
            self.edges[e1].vert,
            self.edges[e2].vert,
            self.edges[e3].vert,
        ]
    }

    pub(crate) fn face_edges(&self, f: F) -> [E; 3] {
        let e1 = self.faces[f].edge;
        let e2 = self.edges[e1].next;
        [e1, e2, self.edges[e2].next]
    }

    pub(crate) fn edge_length(&self, e: E) -> f64 {
        let a = self.verts[self.edges[e].vert].co;
        let b = self.verts[self.edges[self.edges[e].next].vert].co;
        a.distance(b)
    }

    pub(crate) fn edge_uv_length(&self, e: E) -> f64 {
        let a = self.verts[self.edges[e].vert].uv;
        let b = self.verts[self.edges[self.edges[e].next].vert].uv;
        a.distance(b)
    }

    pub(crate) fn face_area(&self, f: F) -> f64 {
        let [a, b, c] = self.face_verts(f);
        geom::triangle_area(self.verts[a].co, self.verts[b].co, self.verts[c].co)
    }

    pub(crate) fn face_uv_area_signed(&self, f: F) -> f64 {
        let [a, b, c] = self.face_verts(f);
        geom::area_signed(self.verts[a].uv, self.verts[b].uv, self.verts[c].uv)
    }

    pub(crate) fn face_angles(&self, f: F) -> [f64; 3] {
        let [a, b, c] = self.face_verts(f);
        geom::triangle_angles(self.verts[a].co, self.verts[b].co, self.verts[c].co)
    }

    pub(crate) fn chart_uv_area(&self, c: usize) -> f64 {
        self.charts[c]
            .faces
            .iter()
            .map(|&f| self.face_uv_area_signed(f).abs())
            .sum()
    }

    pub(crate) fn chart_uv_bbox(&self, c: usize) -> (DVec2, DVec2) {
        let mut lo = DVec2::splat(f64::MAX);
        let mut hi = DVec2::splat(f64::MIN);
        for &v in &self.charts[c].verts {
            lo = lo.min(self.verts[v].uv);
            hi = hi.max(self.verts[v].uv);
        }
        (lo, hi)
    }

    pub(crate) fn chart_uv_scale(&mut self, c: usize, scale: f64) {
        if scale == 1.0 {
            return;
        }
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            self.verts[v].uv *= scale;
        }
    }

    pub(crate) fn chart_uv_translate(&mut self, c: usize, t: DVec2) {
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            self.verts[v].uv += t;
        }
    }

    /// `uv = m * uv` with `m` given as columns.
    pub(crate) fn chart_uv_transform(&mut self, c: usize, col0: DVec2, col1: DVec2) {
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            let uv = self.verts[v].uv;
            self.verts[v].uv = col0 * uv.x + col1 * uv.y;
        }
    }

    /// Rotates about the chart's `origin`.
    pub(crate) fn chart_uv_rotate(&mut self, c: usize, angle: f64) {
        if angle == 0.0 {
            return;
        }
        let (s, co) = angle.sin_cos();
        let o = self.charts[c].origin;
        for i in 0..self.charts[c].verts.len() {
            let v = self.charts[c].verts[i];
            let d = self.verts[v].uv - o;
            self.verts[v].uv = o + DVec2::new(co * d.x - s * d.y, s * d.x + co * d.y);
        }
    }

    pub(crate) fn chart_uv_points(&self, c: usize) -> Vec<DVec2> {
        self.charts[c]
            .verts
            .iter()
            .map(|&v| self.verts[v].uv)
            .collect()
    }

    /// Number of charts (islands) built.
    pub fn chart_count(&self) -> usize {
        self.charts.len()
    }

    /// Charts dropped because they had no border to cut along.
    pub fn failed_chart_count(&self) -> usize {
        self.failed_charts
    }

    /// For each input face, the chart its triangles went to (`None` for faces
    /// in dropped charts or faces made only of degenerate triangles).
    pub fn face_charts(&self) -> Vec<Option<usize>> {
        let mut out = vec![None; self.face_first_corner.len()];
        for (ci, chart) in self.charts.iter().enumerate() {
            for &f in &chart.faces {
                let s = self.faces[f].source;
                if s != usize::MAX {
                    out[s] = Some(ci);
                }
            }
        }
        out
    }

    // ----------------------------------------------------------------------
    // Construction

    fn vert_lookup(&mut self, b: &mut Builder, key: u64, co: DVec3, weight: f64, e: E) -> V {
        if let Some(&v) = b.vert_by_key.get(&key) {
            return v;
        }
        let co = DVec3::new(
            if co.x.is_finite() { co.x } else { 0.0 },
            if co.y.is_finite() { co.y } else { 0.0 },
            if co.z.is_finite() { co.z } else { 0.0 },
        );
        let v = self.verts.len();
        self.verts.push(Vert {
            key,
            co,
            uv: DVec2::ZERO,
            edge: e,
            flag: 0,
            weight,
            id: 0,
            on_boundary: false,
        });
        b.vert_by_key.insert(key, v);
        v
    }

    fn edge_lookup(&self, b: &Builder, ka: u64, kb: u64) -> Option<E> {
        let list = b.edges_by_key.get(&edge_key(ka, kb))?;
        list.iter().copied().find(|&e| {
            let a = self.verts[self.edges[e].vert].key;
            let c = self.verts[self.edges[self.edges[e].next].vert].key;
            (a == ka && c == kb) || (a == kb && c == ka)
        })
    }

    fn face_exists(&self, b: &Builder, k1: u64, k2: u64, k3: u64) -> bool {
        let Some(list) = b.edges_by_key.get(&edge_key(k1, k2)) else {
            return false;
        };
        list.iter().any(|&e| {
            let a = self.verts[self.edges[e].vert].key;
            let c = self.verts[self.edges[self.edges[e].next].vert].key;
            let d = self.verts[self.edges[self.edges[self.edges[e].next].next].vert].key;
            ((a == k1 && c == k2) || (a == k2 && c == k1)) && d == k3
        })
    }

    fn new_face(&mut self, source: usize) -> (F, [E; 3]) {
        let f = self.faces.len();
        let e1 = self.edges.len();
        let (e2, e3) = (e1 + 1, e1 + 2);
        for (e, next) in [(e1, e2), (e2, e3), (e3, e1)] {
            debug_assert_eq!(e, self.edges.len());
            self.edges.push(Edge {
                vert: 0,
                pair: None,
                next,
                face: f,
                corner: None,
                old_uv: DVec2::ZERO,
                flag: 0,
                id: 0,
            });
        }
        self.faces.push(Face {
            edge: e1,
            flag: 0,
            source,
            chart: 0,
            id: 0,
        });
        (f, [e1, e2, e3])
    }

    /// `p_face_add_construct` for corners `idx` of one input face.
    fn add_triangle(
        &mut self,
        b: &mut Builder,
        mesh: &UvMesh,
        source: usize,
        corners: &[Corner],
        first: usize,
        keys: &[u64],
        idx: [usize; 3],
    ) {
        let (_, es) = self.new_face(source);
        for (k, &i) in idx.iter().enumerate() {
            let c = corners[i];
            let co = mesh.points[c.point as usize];
            let w = mesh.weights.as_ref().map_or(1.0, |w| w[c.point as usize]);
            let v = self.vert_lookup(b, keys[first + i], co, w, es[k]);
            let e = &mut self.edges[es[k]];
            e.vert = v;
            e.corner = Some(first + i);
            if c.pin {
                e.flag |= EDGE_PIN;
            }
            if c.select {
                e.flag |= EDGE_SELECT;
            }
        }
        for k in 0..3 {
            let ka = keys[first + idx[k]];
            let kb = keys[first + idx[(k + 1) % 3]];
            b.edges_by_key
                .entry(edge_key(ka, kb))
                .or_default()
                .push(es[k]);
        }
    }

    /// `uv_parametrizer_face_add`, including the guard against n-gons whose
    /// ears repeat an existing triangle.
    fn face_add(
        &mut self,
        b: &mut Builder,
        mesh: &UvMesh,
        source: usize,
        corners: &[Corner],
        first: usize,
        keys: &[u64],
    ) {
        let n = corners.len();
        let k = |i: usize| keys[first + i];
        if n == 3 {
            if !self.face_exists(b, k(0), k(1), k(2)) {
                self.add_triangle(b, mesh, source, corners, first, keys, [0, 1, 2]);
            }
            return;
        }
        let mut permute: Vec<usize> = (0..n).collect();
        let mut i = n as isize - 1;
        while i >= 0 {
            let pm = permute.len();
            let iu = i as usize;
            let (i0, i1, i2) = (permute[iu], permute[(iu + 1) % pm], permute[(iu + 2) % pm]);
            if !self.face_exists(b, k(i0), k(i1), k(i2)) {
                i -= 1;
                continue;
            }
            let iprev = permute[(iu + pm - 1) % pm];
            self.add_triangle(b, mesh, source, corners, first, keys, [iprev, i0, i1]);
            permute.remove(iu);
            if permute.len() == 3 {
                break;
            }
            i -= 1;
        }
        let co: Vec<DVec3> = permute
            .iter()
            .map(|&j| mesh.points[corners[j].point as usize])
            .collect();
        for t in geom::triangulate(&co) {
            self.add_triangle(
                b,
                mesh,
                source,
                corners,
                first,
                keys,
                [permute[t[0]], permute[t[1]], permute[t[2]]],
            );
        }
    }

    /// `p_face_flip`: reverse a triangle's winding, keeping corner data with
    /// its vertex.
    fn face_flip(&mut self, f: F) {
        let [e1, e2, e3] = self.face_edges(f);
        let (v1, v2, v3) = (
            self.edges[e1].vert,
            self.edges[e2].vert,
            self.edges[e3].vert,
        );
        let (f1, f2, f3) = (
            self.edges[e1].flag,
            self.edges[e2].flag,
            self.edges[e3].flag,
        );
        let (c1, c2, c3) = (
            self.edges[e1].corner,
            self.edges[e2].corner,
            self.edges[e3].corner,
        );
        let keep = !EDGE_VERTEX_FLAGS;
        let set = |e: &mut Edge, v, next, c, flag| {
            e.vert = v;
            e.next = next;
            e.corner = c;
            e.flag = flag;
        };
        set(
            &mut self.edges[e1],
            v2,
            e3,
            c2,
            (f1 & keep) | (f2 & EDGE_VERTEX_FLAGS),
        );
        set(
            &mut self.edges[e2],
            v3,
            e1,
            c3,
            (f2 & keep) | (f3 & EDGE_VERTEX_FLAGS),
        );
        set(
            &mut self.edges[e3],
            v1,
            e2,
            c1,
            (f3 & keep) | (f1 & EDGE_VERTEX_FLAGS),
        );
    }

    fn edge_implicit_seam(&mut self, e: E, ep: E) -> bool {
        let limit = 0.00001;
        let (Some(c1), Some(c2)) = (self.edges[e].corner, self.edges[self.edges[e].next].corner)
        else {
            return false;
        };
        let (uv1, uv2) = (self.corner_uv[c1], self.corner_uv[c2]);
        let epn = self.edges[ep].next;
        let (Some(p1), Some(p2)) = (self.edges[ep].corner, self.edges[epn].corner) else {
            return false;
        };
        let (uvp1, uvp2) =
            if self.verts[self.edges[e].vert].key == self.verts[self.edges[ep].vert].key {
                (self.corner_uv[p1], self.corner_uv[p2])
            } else {
                (self.corner_uv[p2], self.corner_uv[p1])
            };
        let differs = |a: DVec2, b: DVec2| (a.x - b.x).abs() > limit || (a.y - b.y).abs() > limit;
        if differs(uv1, uvp1) || differs(uv2, uvp2) {
            self.edges[e].flag |= EDGE_SEAM;
            self.edges[ep].flag |= EDGE_SEAM;
            return true;
        }
        false
    }

    fn edge_has_pair(&mut self, b: &Builder, e: E, topology_from_uvs: bool) -> Option<E> {
        if self.edges[e].flag & EDGE_SEAM != 0 {
            return None;
        }
        let k1 = self.verts[self.edges[e].vert].key;
        let k2 = self.verts[self.edges[self.edges[e].next].vert].key;
        let list = b.edges_by_key.get(&edge_key(k1, k2))?.clone();
        let mut pair: Option<E> = None;
        for pe in list {
            if pe == e {
                continue;
            }
            let a = self.verts[self.edges[pe].vert].key;
            let c = self.verts[self.edges[self.edges[pe].next].vert].key;
            if (a == k1 && c == k2) || (a == k2 && c == k1) {
                // Don't connect seams and T-junctions.
                if self.edges[pe].flag & EDGE_SEAM != 0
                    || pair.is_some()
                    || (topology_from_uvs && self.edge_implicit_seam(e, pe))
                {
                    return None;
                }
                pair = Some(pe);
            }
        }
        let p = pair?;
        if self.edges[e].vert == self.edges[p].vert {
            let pn = self.edges[p].next;
            if self.edges[pn].pair.is_some() || self.edges[self.edges[pn].next].pair.is_some() {
                // Not unfoldable (a Möbius strip or Klein bottle).
                return None;
            }
        }
        Some(p)
    }

    fn edge_connect_pair(
        &mut self,
        b: &Builder,
        e: E,
        topology_from_uvs: bool,
        stack: &mut Vec<E>,
    ) -> bool {
        if self.edges[e].pair.is_none()
            && let Some(p) = self.edge_has_pair(b, e, topology_from_uvs)
        {
            if self.edges[e].vert == self.edges[p].vert {
                let pf = self.edges[p].face;
                self.face_flip(pf);
            }
            self.edges[e].pair = Some(p);
            self.edges[p].pair = Some(e);
            let pf = self.edges[p].face;
            if self.faces[pf].flag & FACE_CONNECTED == 0 {
                stack.push(p);
            }
        }
        self.edges[e].pair.is_some()
    }

    /// `p_connect_pairs`: pair up half-edges and number the connected pieces.
    fn connect_pairs(&mut self, b: &Builder, topology_from_uvs: bool) -> usize {
        let mut ncharts = 0;
        let mut stack = Vec::new();
        for first in 0..self.faces.len() {
            if self.faces[first].flag & FACE_CONNECTED != 0 {
                continue;
            }
            stack.push(self.faces[first].edge);
            while let Some(e) = stack.pop() {
                let e1 = self.edges[e].next;
                let e2 = self.edges[e1].next;
                let f = self.edges[e].face;
                self.faces[f].flag |= FACE_CONNECTED;
                self.faces[f].chart = ncharts;
                for x in [e, e1, e2] {
                    if !self.edge_connect_pair(b, x, topology_from_uvs, &mut stack) {
                        let v = self.edges[x].vert;
                        self.verts[v].edge = x;
                    }
                }
            }
            ncharts += 1;
        }
        ncharts
    }

    /// `p_split_vert`: give each chart its own copy of a vertex whose wheel
    /// is cut apart by seams.
    fn split_vert(&mut self, chart: &mut Chart, e: E) {
        let mut v = self.edges[e].vert;
        if self.edges[e].flag & EDGE_PIN != 0 {
            chart.has_pins = true;
        }
        if self.edges[e].flag & EDGE_VERTEX_SPLIT != 0 {
            return;
        }
        // Rewind to the start of the wheel.
        let mut lastwe = e;
        let mut we = self.wheel_prev(e);
        let mut guard = 0;
        while let Some(w) = we {
            if w == e || guard > self.edges.len() {
                break;
            }
            lastwe = w;
            we = self.wheel_prev(w);
            guard += 1;
        }
        let mut copy = true;
        let mut we = Some(lastwe);
        guard = 0;
        while let Some(w) = we {
            if self.edges[w].flag & EDGE_VERTEX_SPLIT != 0 || guard > self.edges.len() {
                break;
            }
            self.edges[w].flag |= EDGE_VERTEX_SPLIT;
            if w == self.verts[v].edge {
                copy = false;
                chart.verts.push(v);
            }
            we = self.wheel_next(w);
            guard += 1;
        }
        if copy {
            self.verts[v].flag |= VERT_SPLIT;
            let mut nv = self.verts[v].clone();
            nv.flag |= VERT_SPLIT;
            nv.edge = lastwe;
            v = self.verts.len();
            self.verts.push(nv);
            chart.verts.push(v);
            let mut we = lastwe;
            guard = 0;
            loop {
                self.edges[we].vert = v;
                match self.wheel_next(we) {
                    Some(n) if n != lastwe && guard <= self.edges.len() => we = n,
                    _ => break,
                }
                guard += 1;
            }
        }
    }

    fn split_charts(&mut self, ncharts: usize) {
        let mut charts = vec![Chart::default(); ncharts];
        for f in 0..self.faces.len() {
            let c = self.faces[f].chart;
            let es = self.face_edges(f);
            let chart = &mut charts[c];
            chart.faces.push(f);
            chart.edges.extend(es);
            for e in es {
                let mut ch = std::mem::take(&mut charts[c]);
                self.split_vert(&mut ch, e);
                charts[c] = ch;
            }
        }
        self.charts = charts;
    }

    /// `p_chart_boundaries`: counts border loops and returns the start edge of
    /// the longest one.
    pub(crate) fn chart_boundaries(&mut self, c: usize) -> (usize, Option<E>) {
        let mut n = 0;
        let mut outer = None;
        let mut maxlen = -1.0;
        let edges = self.charts[c].edges.clone();
        for &e in &edges {
            if self.edges[e].pair.is_some() || self.edges[e].flag & EDGE_DONE != 0 {
                continue;
            }
            n += 1;
            let mut len = 0.0;
            let mut be = e;
            let mut guard = 0;
            loop {
                self.edges[be].flag |= EDGE_DONE;
                len += self.edge_length(be);
                be = self.boundary_next(be);
                guard += 1;
                if be == e || guard > edges.len() {
                    break;
                }
            }
            if len > maxlen {
                outer = Some(e);
                maxlen = len;
            }
        }
        for &e in &edges {
            self.edges[e].flag &= !EDGE_DONE;
        }
        self.charts[c].nboundaries = n;
        (n, outer)
    }

    fn add_fill_face(&mut self, c: usize, v1: V, v2: V, v3: V) -> (F, [E; 3]) {
        let (f, es) = self.new_face(usize::MAX);
        self.edges[es[0]].vert = v1;
        self.edges[es[1]].vert = v2;
        self.edges[es[2]].vert = v3;
        self.faces[f].chart = c;
        let chart = &mut self.charts[c];
        chart.faces.push(f);
        chart.edges.extend(es);
        (f, es)
    }

    /// `p_chart_fill_boundary`: cap one border loop with triangles.
    fn fill_boundary(&mut self, c: usize, border: &mut [E]) {
        let co: Vec<DVec3> = border
            .iter()
            .map(|&e| self.verts[self.edges[e].vert].co)
            .collect();
        // The fill must follow the loop's own winding.
        let mut tris = geom::triangulate(&co);
        let mut nedges = border.len();
        let mut filled = vec![false; tris.len()];
        while nedges >= 3 {
            let mut progress = false;
            for (ti, tri) in tris.iter_mut().enumerate() {
                if filled[ti] {
                    continue;
                }
                let (mut i1, mut i2, mut i3) = (tri[0], tri[1], tri[2]);
                let mut done = false;
                for _ in 0..3 {
                    let inner = border[i2];
                    let prev = self.boundary_prev(inner);
                    let next = self.boundary_next(inner);
                    if self.verts[self.edges[prev].vert].id != i1
                        || self.verts[self.edges[next].vert].id != i3
                    {
                        (i1, i2, i3) = (i3, i1, i2);
                        continue;
                    }
                    self.edges[inner].flag |= EDGE_FILLED;
                    self.edges[prev].flag |= EDGE_FILLED;
                    let (iv, pv, nv) = (
                        self.edges[inner].vert,
                        self.edges[prev].vert,
                        self.edges[next].vert,
                    );
                    let (f, es) = self.add_fill_face(c, iv, pv, nv);
                    self.faces[f].flag |= FACE_FILLED;
                    let (new_next, new_prev, new_inner) = (es[0], es[1], es[2]);
                    for x in es {
                        self.edges[x].flag = EDGE_FILLED;
                    }
                    self.edges[inner].pair = Some(new_inner);
                    self.edges[new_inner].pair = Some(inner);
                    self.edges[prev].pair = Some(new_next);
                    self.edges[new_next].pair = Some(prev);
                    if nedges == 3 {
                        self.edges[next].pair = Some(new_prev);
                        self.edges[new_prev].pair = Some(next);
                    } else {
                        filled[ti] = true;
                        let npv = self.edges[new_prev].vert;
                        self.verts[npv].edge = new_prev;
                        border[self.verts[npv].id] = new_prev;
                        let nbv = self.edges[next].vert;
                        border[self.verts[nbv].id] = next;
                    }
                    nedges -= 1;
                    done = true;
                    break;
                }
                if done {
                    progress = true;
                    break;
                }
            }
            if !progress {
                // The triangulation doesn't match the loop (a self-touching
                // border). Leave the rest of the hole open.
                break;
            }
        }
    }

    fn fill_boundaries(&mut self, c: usize, outer: Option<E>) {
        let original = self.charts[c].edges.len();
        for i in 0..original {
            let e = self.charts[c].edges[i];
            if self.edges[e].pair.is_some() || self.edges[e].flag & EDGE_FILLED != 0 {
                continue;
            }
            let mut border = Vec::new();
            let mut be = e;
            loop {
                border.push(be);
                self.edges[be].flag |= EDGE_FILLED;
                be = self.boundary_next(be);
                if be == e || border.len() > original {
                    break;
                }
            }
            if Some(e) == outer {
                continue;
            }
            if border.len() == 2 {
                // No real hole, an isolated seam.
                let be = self.boundary_next(e);
                self.edges[e].pair = Some(be);
                self.edges[be].pair = Some(e);
                continue;
            }
            for (k, &b) in border.iter().enumerate() {
                let v = self.edges[b].vert;
                self.verts[v].id = k;
            }
            self.fill_boundary(c, &mut border);
        }
    }

    /// `p_vert_load_pin_select_uvs`.
    pub(crate) fn vert_load_pin_select_uvs(&mut self, v: V) {
        let mut uv = DVec2::ZERO;
        let mut pin_uv = DVec2::ZERO;
        let (mut n, mut npins) = (0, 0);
        for e in self.wheel(v) {
            if let Some(c) = self.edges[e].corner {
                let cuv = self.corner_uv[c];
                let cuv = DVec2::new(cuv.x * self.aspect_y, cuv.y);
                if self.edges[e].flag & EDGE_SELECT != 0 {
                    self.verts[v].flag |= VERT_SELECT;
                }
                if self.edges[e].flag & EDGE_PIN != 0 {
                    pin_uv += cuv;
                    npins += 1;
                } else {
                    uv += cuv;
                }
                n += 1;
            }
        }
        if npins > 0 {
            self.verts[v].uv = pin_uv / npins as f64;
            self.verts[v].flag |= VERT_PIN;
        } else if n > 0 {
            self.verts[v].uv = uv / n as f64;
        } else {
            self.verts[v].uv = DVec2::ZERO;
        }
    }

    fn construct_end(&mut self, b: &Builder, options: ConstructOptions) {
        let n = self.connect_pairs(b, options.topology_from_uvs);
        self.split_charts(n);
        let charts = std::mem::take(&mut self.charts);
        let mut kept = Vec::with_capacity(charts.len());
        for chart in charts {
            self.charts.push(chart);
            let c = self.charts.len() - 1;
            let (nb, outer) = self.chart_boundaries(c);
            if !options.topology_from_uvs && nb == 0 {
                self.charts.pop();
                self.failed_charts += 1;
                continue;
            }
            if options.fill_holes && nb > 1 {
                self.fill_boundaries(c, outer);
            }
            let verts = self.charts[c].verts.clone();
            for v in verts {
                self.vert_load_pin_select_uvs(v);
            }
            kept.push(self.charts.pop().unwrap());
        }
        for (i, ch) in kept.iter().enumerate() {
            for &f in &ch.faces {
                self.faces[f].chart = i;
            }
        }
        self.charts = kept;
    }

    // ----------------------------------------------------------------------
    // Flushing

    pub(crate) fn backup_uvs(&mut self, c: usize) {
        for i in 0..self.charts[c].edges.len() {
            let e = self.charts[c].edges[i];
            if let Some(k) = self.edges[e].corner {
                self.edges[e].old_uv = self.corner_uv[k];
            }
        }
    }

    fn flush_chart(&mut self, c: usize) {
        let blend = self.blend;
        let inv = 1.0 - blend;
        let inv_x = inv / self.aspect_y;
        for i in 0..self.charts[c].edges.len() {
            let e = self.charts[c].edges[i];
            if let Some(k) = self.edges[e].corner {
                let old = self.edges[e].old_uv;
                let uv = self.verts[self.edges[e].vert].uv;
                self.corner_uv[k] =
                    DVec2::new(blend * old.x + inv_x * uv.x, blend * old.y + inv * uv.y);
            }
        }
    }

    /// Writes every chart's UVs back to its corners (`uv_parametrizer_flush`).
    pub fn flush(&mut self) {
        for c in 0..self.charts.len() {
            if !self.charts[c].skip_flush {
                self.flush_chart(c);
            }
        }
    }

    /// Puts back the UVs saved when solving started
    /// (`uv_parametrizer_flush_restore`).
    pub fn flush_restore(&mut self) {
        for c in 0..self.charts.len() {
            for i in 0..self.charts[c].edges.len() {
                let e = self.charts[c].edges[i];
                if let Some(k) = self.edges[e].corner {
                    self.corner_uv[k] = self.edges[e].old_uv;
                }
            }
        }
    }

    /// The UV of every input corner, face by face (as written by `flush`).
    pub fn corner_uvs(&self) -> Vec<Vec<DVec2>> {
        let n = self.face_first_corner.len();
        (0..n)
            .map(|f| {
                let start = self.face_first_corner[f];
                let end = self
                    .face_first_corner
                    .get(f + 1)
                    .copied()
                    .unwrap_or(self.corner_uv.len());
                self.corner_uv[start..end].to_vec()
            })
            .collect()
    }
}

/// Vertex keys for every corner (`uv_prepare_pin_index`/`uv_find_pin_index`):
/// a point's corners share its index as key, except that each distinct pinned
/// UV of a point gets its own key, and the point's other corners join the
/// nearest pinned UV.
fn pin_keys(mesh: &UvMesh) -> Vec<u64> {
    let mut pins: HashMap<u32, Vec<(DVec2, u64)>> = HashMap::new();
    let mut unique = 0u64;
    for face in &mesh.faces {
        for c in face {
            if !c.pin {
                continue;
            }
            let list = pins.entry(c.point).or_default();
            if !list.iter().any(|(uv, _)| *uv == c.uv) {
                list.push((c.uv, u64::MAX - unique));
                unique += 1;
            }
        }
    }
    let mut keys = Vec::new();
    for face in &mesh.faces {
        for c in face {
            let key = match pins.get(&c.point) {
                None => c.point as u64,
                Some(list) => {
                    let mut best = list[0];
                    for &cand in &list[1..] {
                        if cand.0.distance_squared(c.uv) < best.0.distance_squared(c.uv) {
                            best = cand;
                        }
                    }
                    best.1
                }
            };
            keys.push(key);
        }
    }
    keys
}
