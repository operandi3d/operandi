// SPDX-FileCopyrightText: 2013 Inigo Quilez
// SPDX-FileCopyrightText: 2011-2022 Blender Foundation
// SPDX-FileCopyrightText: 2026 Operandi contributors
//
// SPDX-License-Identifier: MIT AND Apache-2.0
//
// Adapted from Cycles (https://projects.blender.org/blender/cycles) at
// commit 3eb35f85996baa6c1b2e19bf297b22745c85e321: src/kernel/svm/voronoi.h
// (the Voronoi texture; its cell searches, smooth F1 and distance to edge
// are MIT, after Inigo Quilez, the rest Apache-2.0) and the PCG hashes in
// src/util/hash.h (Apache-2.0). The C++ is rewritten as Rust, Cycles' one
// function per dimension count folded into one taking the count (2-4D share
// one cell search; 1D keeps its own float hashes); the sums, scan windows,
// hashes and fractal steps are Cycles' own, in f32. `cycles_voronoi.wgsl`
// holds the same code as WGSL. See docs/DEPENDENCIES.md and
// docs/licenses/cycles/.

//! Cycles' Voronoi texture on the CPU, for graph layers drawn in the
//! viewport (MaterialX stage 5).
use crate::cycles_noise::{P4, hash_floats};

/// The Voronoi texture's features (`NodeVoronoiFeature`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VoronoiFeature {
    F1,
    F2,
    SmoothF1,
    DistanceToEdge,
    NSphereRadius,
}
impl VoronoiFeature {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "f1" => Self::F1,
            "f2" => Self::F2,
            "smooth_f1" => Self::SmoothF1,
            "distance_to_edge" => Self::DistanceToEdge,
            "n_sphere_radius" => Self::NSphereRadius,
            _ => return None,
        })
    }
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// The Voronoi texture's distance metrics (`NodeVoronoiDistanceMetric`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VoronoiMetric {
    Euclidean,
    Manhattan,
    Chebychev,
    Minkowski,
}
impl VoronoiMetric {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "euclidean" => Self::Euclidean,
            "manhattan" => Self::Manhattan,
            "chebychev" => Self::Chebychev,
            "minkowski" => Self::Minkowski,
            _ => return None,
        })
    }
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// Which output of the node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VoronoiOut {
    Distance,
    Color,
    Position,
    W,
    Radius,
}
impl VoronoiOut {
    pub fn from_socket(socket: &str) -> Option<Self> {
        Some(match socket {
            "Distance" => Self::Distance,
            "Color" => Self::Color,
            "Position" => Self::Position,
            "W" => Self::W,
            "Radius" => Self::Radius,
            _ => return None,
        })
    }
    pub fn code(self) -> u32 {
        self as u32
    }
}

/// The node's settings and inputs, before Cycles clamps them.
#[derive(Clone, Copy, Debug)]
pub struct Voronoi {
    pub dims: usize,
    pub feature: VoronoiFeature,
    pub metric: VoronoiMetric,
    pub normalize: bool,
    pub scale: f32,
    pub detail: f32,
    pub roughness: f32,
    pub lacunarity: f32,
    pub smoothness: f32,
    pub exponent: f32,
    pub randomness: f32,
}

/// What [`Voronoi::nearest_two`] finds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearestTwo {
    pub f1: f32,
    pub f2: f32,
    pub color: [f32; 3],
    pub point: [f32; 3],
}

#[derive(Clone, Copy, Default)]
struct Octave {
    distance: f32,
    color: [f32; 3],
    position: P4,
}

/// PCG hash of 2-4 ints (`hash_pcg{2,3,4}d_i`), Cycles' signed version.
fn pcg(k: [i32; 4], n: usize) -> [i32; 4] {
    let m = 1664525i32;
    let mut v = k.map(|x| x.wrapping_mul(m).wrapping_add(1013904223));
    let step = |v: &mut [i32; 4]| match n {
        2 => {
            v[0] = v[0].wrapping_add(v[1].wrapping_mul(m));
            v[1] = v[1].wrapping_add(v[0].wrapping_mul(m));
        }
        3 => {
            v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
            v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
            v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
        }
        _ => {
            v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[3]));
            v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
            v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
            v[3] = v[3].wrapping_add(v[1].wrapping_mul(v[2]));
        }
    };
    step(&mut v);
    for x in v.iter_mut().take(n) {
        *x ^= *x >> 16;
    }
    step(&mut v);
    v.map(|x| x & 0x7FFFFFFF)
}

fn pcg_unit(k: [i32; 4], n: usize) -> [f32; 4] {
    pcg(k, n).map(|x| x as f32 * (1.0 / 0x7FFFFFFFu32 as f32))
}

/// A cell's point offset (`hash_int{2,3,4}_to_float{2,3,4}`).
fn cell_point(cell: [i32; 4], dims: usize) -> [f32; 4] {
    let mut h = pcg_unit(cell, dims);
    for x in h.iter_mut().skip(dims) {
        *x = 0.0;
    }
    h
}

/// A cell's colour (`hash_int{2,3,4}_to_float3`; 2D hashes (x, y, 0)).
fn cell_color(cell: [i32; 4], dims: usize) -> [f32; 3] {
    let h = if dims == 2 {
        pcg_unit([cell[0], cell[1], 0, 0], 3)
    } else {
        pcg_unit(cell, dims)
    };
    [h[0], h[1], h[2]]
}

fn hash_float_to_float3(k: f32) -> [f32; 3] {
    [
        hash_floats(&[k]),
        hash_floats(&[k, 1.0]),
        hash_floats(&[k, 2.0]),
    ]
}

fn sub(a: P4, b: P4) -> P4 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}
fn add(a: P4, b: P4) -> P4 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}
fn dot(a: P4, b: P4, n: usize) -> f32 {
    (0..n).map(|k| a[k] * b[k]).sum()
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

impl Voronoi {
    fn distance(&self, a: P4, b: P4, n: usize) -> f32 {
        let d = sub(a, b);
        match self.metric {
            VoronoiMetric::Euclidean => dot(d, d, n).sqrt(),
            VoronoiMetric::Manhattan => (0..n).map(|k| d[k].abs()).sum(),
            VoronoiMetric::Chebychev => (0..n).map(|k| d[k].abs()).fold(0.0, f32::max),
            VoronoiMetric::Minkowski => (0..n)
                .map(|k| d[k].abs().powf(self.exponent))
                .sum::<f32>()
                .powf(1.0 / self.exponent),
        }
    }
    /// `voronoi_distance_bound`: orders points as `distance` does.
    fn bound(&self, a: P4, b: P4, n: usize) -> f32 {
        let d = sub(a, b);
        match self.metric {
            VoronoiMetric::Euclidean => dot(d, d, n),
            VoronoiMetric::Minkowski => (0..n).map(|k| d[k].abs().powf(self.exponent)).sum(),
            _ => self.distance(a, b, n),
        }
    }

    /// Every cell offset in −r..r per axis, x fastest (Cycles' loop order).
    fn offsets(n: usize, r: i32) -> impl Iterator<Item = [i32; 4]> {
        let side = (2 * r + 1) as usize;
        (0..side.pow(n as u32)).map(move |mut i| {
            let mut o = [0i32; 4];
            for c in o.iter_mut().take(n) {
                *c = (i % side) as i32 - r;
                i /= side;
            }
            o
        })
    }

    /// One octave of F1, F2 or Smooth F1 at `coord` (`n` ≥ 2).
    fn octave(&self, coord: P4, n: usize, feature: VoronoiFeature) -> Octave {
        let cell_f = coord.map(f32::floor);
        let local = sub(coord, cell_f);
        let cell = cell_f.map(|c| c as i32);
        let at = |o: [i32; 4]| {
            let c = [0, 1, 2, 3].map(|k| cell[k].wrapping_add(o[k]));
            let h = cell_point(c, n);
            let p: P4 = [0, 1, 2, 3].map(|k| o[k] as f32 + h[k] * self.randomness);
            (c, p)
        };
        let finish = |p: P4| {
            let mut q = add(p, cell_f);
            for x in q.iter_mut().skip(n) {
                *x = 0.0;
            }
            q
        };
        match feature {
            VoronoiFeature::SmoothF1 => {
                let (mut d, mut col, mut pos, mut h) = (0f32, [0f32; 3], [0f32; 4], -1f32);
                for o in Self::offsets(n, 2) {
                    let (c, p) = at(o);
                    let dp = self.distance(p, local, n);
                    h = if h == -1.0 {
                        1.0
                    } else {
                        smoothstep01(0.5 + 0.5 * (d - dp) / self.smoothness)
                    };
                    let mut corr = self.smoothness * h * (1.0 - h);
                    d = mix(d, dp, h) - corr;
                    corr /= 1.0 + 3.0 * self.smoothness;
                    let cc = cell_color(c, n);
                    col = [0, 1, 2].map(|k| mix(col[k], cc[k], h) - corr);
                    pos = [0, 1, 2, 3].map(|k| mix(pos[k], p[k], h) - corr);
                }
                Octave {
                    distance: d,
                    color: col,
                    position: finish(pos),
                }
            }
            VoronoiFeature::F2 => {
                let (mut d1, mut d2) = (f32::MAX, f32::MAX);
                let (mut c1, mut c2) = ([0i32; 4], [0i32; 4]);
                let (mut p1, mut p2) = ([0f32; 4], [0f32; 4]);
                for o in Self::offsets(n, 1) {
                    let (c, p) = at(o);
                    let dp = self.distance(p, local, n);
                    if dp < d1 {
                        d2 = d1;
                        d1 = dp;
                        c2 = c1;
                        c1 = c;
                        p2 = p1;
                        p1 = p;
                    } else if dp < d2 {
                        d2 = dp;
                        c2 = c;
                        p2 = p;
                    }
                }
                Octave {
                    distance: d2,
                    color: cell_color(c2, n),
                    position: finish(p2),
                }
            }
            _ => {
                let (mut best, mut bc, mut bp) = (f32::MAX, cell, [0f32; 4]);
                for o in Self::offsets(n, 1) {
                    let (c, p) = at(o);
                    let dp = self.bound(p, local, n);
                    if dp < best {
                        best = dp;
                        bc = c;
                        bp = p;
                    }
                }
                Octave {
                    distance: self.distance(bp, local, n),
                    color: cell_color(bc, n),
                    position: finish(bp),
                }
            }
        }
    }

    /// The 1D octave (float hashes, metric ignored as Cycles does).
    fn octave_1d(&self, x: f32, feature: VoronoiFeature) -> Octave {
        let cell = x.floor();
        let local = x - cell;
        let point = |o: f32| o + hash_floats(&[cell + o]) * self.randomness;
        let out = |d: f32, col: [f32; 3], p: f32| Octave {
            distance: d,
            color: col,
            position: [0.0, 0.0, 0.0, p],
        };
        match feature {
            VoronoiFeature::SmoothF1 => {
                let (mut d, mut col, mut pos, mut h) = (0f32, [0f32; 3], 0f32, -1f32);
                for i in -2..=2 {
                    let o = i as f32;
                    let p = point(o);
                    let dp = (local - p).abs();
                    h = if h == -1.0 {
                        1.0
                    } else {
                        smoothstep01(0.5 + 0.5 * (d - dp) / self.smoothness)
                    };
                    let mut corr = self.smoothness * h * (1.0 - h);
                    d = mix(d, dp, h) - corr;
                    corr /= 1.0 + 3.0 * self.smoothness;
                    let cc = hash_float_to_float3(cell + o);
                    col = [0, 1, 2].map(|k| mix(col[k], cc[k], h) - corr);
                    pos = mix(pos, p, h) - corr;
                }
                out(d, col, cell + pos)
            }
            VoronoiFeature::F2 => {
                let (mut d1, mut d2, mut o1, mut o2, mut p1, mut p2) =
                    (f32::MAX, f32::MAX, 0f32, 0f32, 0f32, 0f32);
                for i in -1..=1 {
                    let o = i as f32;
                    let p = point(o);
                    let dp = (local - p).abs();
                    if dp < d1 {
                        d2 = d1;
                        d1 = dp;
                        o2 = o1;
                        o1 = o;
                        p2 = p1;
                        p1 = p;
                    } else if dp < d2 {
                        d2 = dp;
                        o2 = o;
                        p2 = p;
                    }
                }
                out(d2, hash_float_to_float3(cell + o2), p2 + cell)
            }
            _ => {
                let (mut best, mut bo, mut bp) = (f32::MAX, 0f32, 0f32);
                for i in -1..=1 {
                    let o = i as f32;
                    let p = point(o);
                    let dp = (local - p).abs();
                    if dp < best {
                        best = dp;
                        bo = o;
                        bp = p;
                    }
                }
                out(best, hash_float_to_float3(cell + bo), bp + cell)
            }
        }
    }

    /// The plain Euclidean Voronoi nodes (Scale 1, no Detail) the shader
    /// tree's Cellular and Poisson Cells patterns render with, at `p` in
    /// 2D (x, y) or 3D: the F1 and F2 distances, the nearest cell's colour
    /// (its three randoms) and the nearest feature point.
    pub fn nearest_two(dims: usize, randomness: f32, p: [f32; 3]) -> NearestTwo {
        let n = dims.clamp(2, 3);
        let v = Voronoi {
            dims: n,
            feature: VoronoiFeature::F1,
            metric: VoronoiMetric::Euclidean,
            normalize: false,
            scale: 1.,
            detail: 0.,
            roughness: 0.5,
            lacunarity: 2.,
            smoothness: 0.5,
            exponent: 0.5,
            randomness: randomness.clamp(0., 1.),
        };
        let coord = [p[0], p[1], if n == 3 { p[2] } else { 0. }, 0.];
        let f1 = v.octave(coord, n, VoronoiFeature::F1);
        let f2 = v.octave(coord, n, VoronoiFeature::F2);
        NearestTwo {
            f1: f1.distance,
            f2: f2.distance,
            color: f1.color,
            point: [f1.position[0], f1.position[1], f1.position[2]],
        }
    }

    fn any_octave(&self, coord: P4, n: usize, feature: VoronoiFeature) -> Octave {
        if n == 1 {
            self.octave_1d(coord[3], feature)
        } else {
            self.octave(coord, n, feature)
        }
    }

    fn edge(&self, coord: P4, n: usize) -> f32 {
        if n == 1 {
            let x = coord[3];
            let cell = x.floor();
            let local = x - cell;
            let mid = hash_floats(&[cell]) * self.randomness;
            let left = -1.0 + hash_floats(&[cell - 1.0]) * self.randomness;
            let right = 1.0 + hash_floats(&[cell + 1.0]) * self.randomness;
            let a = ((mid + left) / 2.0 - local).abs();
            let b = ((mid + right) / 2.0 - local).abs();
            return a.min(b);
        }
        let cell_f = coord.map(f32::floor);
        let local = sub(coord, cell_f);
        let cell = cell_f.map(|c| c as i32);
        let to_point = |o: [i32; 4]| {
            let c = [0, 1, 2, 3].map(|k| cell[k].wrapping_add(o[k]));
            let h = cell_point(c, n);
            let p: P4 = [0, 1, 2, 3].map(|k| o[k] as f32 + h[k] * self.randomness);
            let mut v = sub(p, local);
            for x in v.iter_mut().skip(n) {
                *x = 0.0;
            }
            v
        };
        let (mut best, mut closest) = (f32::MAX, [0f32; 4]);
        for o in Self::offsets(n, 1) {
            let v = to_point(o);
            let d = dot(v, v, n);
            if d < best {
                best = d;
                closest = v;
            }
        }
        let mut edge = f32::MAX;
        for o in Self::offsets(n, 1) {
            let v = to_point(o);
            let perp = sub(v, closest);
            let l2 = dot(perp, perp, n);
            if l2 > 0.0001 {
                let mid = add(closest, v).map(|c| c / 2.0);
                let unit = perp.map(|c| c / l2.sqrt());
                edge = edge.min(dot(mid, unit, n));
            }
        }
        edge
    }

    fn n_sphere_radius(&self, coord: P4, n: usize) -> f32 {
        if n == 1 {
            let x = coord[3];
            let cell = x.floor();
            let local = x - cell;
            let point = |o: f32| o + hash_floats(&[cell + o]) * self.randomness;
            let (mut best, mut cp, mut co) = (f32::MAX, 0f32, 0f32);
            for i in -1..=1 {
                let o = i as f32;
                let p = point(o);
                let d = (p - local).abs();
                if d < best {
                    best = d;
                    cp = p;
                    co = o;
                }
            }
            let (mut best, mut cc) = (f32::MAX, 0f32);
            for i in [-1, 1] {
                let o = i as f32 + co;
                let p = point(o);
                let d = (cp - p).abs();
                if d < best {
                    best = d;
                    cc = p;
                }
            }
            return (cc - cp).abs() / 2.0;
        }
        let cell_f = coord.map(f32::floor);
        let local = sub(coord, cell_f);
        let cell = cell_f.map(|c| c as i32);
        let point = |o: [i32; 4]| {
            let c = [0, 1, 2, 3].map(|k| cell[k].wrapping_add(o[k]));
            let h = cell_point(c, n);
            let p: P4 = [0, 1, 2, 3].map(|k| o[k] as f32 + h[k] * self.randomness);
            p
        };
        let (mut best, mut cp, mut co) = (f32::MAX, [0f32; 4], [0i32; 4]);
        for o in Self::offsets(n, 1) {
            let p = point(o);
            let d = sub(p, local);
            let d = dot(d, d, n);
            if d < best {
                best = d;
                cp = p;
                co = o;
            }
        }
        let (mut best, mut cc) = (f32::MAX, [0f32; 4]);
        for o in Self::offsets(n, 1) {
            if o == [0; 4] {
                continue;
            }
            let p = point([0, 1, 2, 3].map(|k| o[k] + co[k]));
            let d = sub(cp, p);
            let d = dot(d, d, n);
            if d < best {
                best = d;
                cc = p;
            }
        }
        let d = sub(cc, cp);
        dot(d, d, n).sqrt() / 2.0
    }

    /// `svm_node_tex_voronoi`: one output at `vector` and `w` (before
    /// Scale), as a grey for the scalar outputs.
    pub fn eval(&self, out: VoronoiOut, vector: [f32; 3], w: f32) -> [f32; 3] {
        let n = self.dims.clamp(1, 4);
        let v = Voronoi {
            detail: self.detail.clamp(0.0, 15.0),
            roughness: self.roughness.clamp(0.0, 1.0),
            randomness: self.randomness.clamp(0.0, 1.0),
            smoothness: (self.smoothness / 2.0).clamp(0.0, 0.5),
            ..*self
        };
        let s = v.scale;
        let coord: P4 = match n {
            1 => [0.0, 0.0, 0.0, w * s],
            2 => [vector[0] * s, vector[1] * s, 0.0, 0.0],
            3 => [vector[0] * s, vector[1] * s, vector[2] * s, 0.0],
            _ => [vector[0] * s, vector[1] * s, vector[2] * s, w * s],
        };
        let reach = 0.5 + 0.5 * v.randomness;
        match v.feature {
            VoronoiFeature::DistanceToEdge => {
                if out != VoronoiOut::Distance {
                    return [0.0; 3];
                }
                [v.fractal_edge(coord, n, reach); 3]
            }
            VoronoiFeature::NSphereRadius => {
                if out != VoronoiOut::Radius {
                    return [0.0; 3];
                }
                [v.n_sphere_radius(coord, n); 3]
            }
            feature => {
                if out == VoronoiOut::Radius {
                    return [0.0; 3];
                }
                let f2 = if feature == VoronoiFeature::F2 {
                    2.0
                } else {
                    1.0
                };
                let max_distance = if n == 1 {
                    reach * f2
                } else {
                    let r = [reach, reach, reach, reach];
                    v.distance([0.0; 4], r, n) * f2
                };
                let o = v.fractal(coord, n, max_distance);
                match out {
                    VoronoiOut::Distance => [o.distance; 3],
                    VoronoiOut::Color => o.color,
                    VoronoiOut::Position => [o.position[0], o.position[1], o.position[2]],
                    _ => [o.position[3]; 3],
                }
            }
        }
    }

    /// `fractal_voronoi_x_fx`.
    fn fractal(&self, coord: P4, n: usize, max_distance: f32) -> Octave {
        let (mut amplitude, mut max_amplitude, mut scale) = (1f32, 0f32, 1f32);
        let mut out = Octave::default();
        let zero_input = self.detail == 0.0 || self.roughness == 0.0;
        let feature = match self.feature {
            VoronoiFeature::SmoothF1 if self.smoothness == 0.0 => VoronoiFeature::F1,
            f => f,
        };
        let mut i = 0;
        while i as f32 <= self.detail.ceil() {
            let oct = self.any_octave(coord.map(|c| c * scale), n, feature);
            if zero_input {
                max_amplitude = 1.0;
                out = oct;
                break;
            }
            if i as f32 <= self.detail {
                max_amplitude += amplitude;
                out.distance += oct.distance * amplitude;
                out.color = [0, 1, 2].map(|k| out.color[k] + oct.color[k] * amplitude);
                out.position =
                    [0, 1, 2, 3].map(|k| mix(out.position[k], oct.position[k] / scale, amplitude));
                scale *= self.lacunarity;
                amplitude *= self.roughness;
            } else {
                let rem = self.detail - self.detail.floor();
                if rem != 0.0 {
                    max_amplitude = mix(max_amplitude, max_amplitude + amplitude, rem);
                    out.distance = mix(out.distance, out.distance + oct.distance * amplitude, rem);
                    out.color = [0, 1, 2]
                        .map(|k| mix(out.color[k], out.color[k] + oct.color[k] * amplitude, rem));
                    out.position = [0, 1, 2, 3].map(|k| {
                        let p = out.position[k];
                        mix(p, mix(p, oct.position[k] / scale, amplitude), rem)
                    });
                }
            }
            i += 1;
        }
        if self.normalize {
            out.distance /= max_amplitude * max_distance;
            out.color = out.color.map(|c| c / max_amplitude);
        }
        out.position = out.position.map(|p| {
            if self.scale != 0.0 {
                p / self.scale
            } else {
                0.0
            }
        });
        out
    }

    /// `fractal_voronoi_distance_to_edge`.
    fn fractal_edge(&self, coord: P4, n: usize, max_distance: f32) -> f32 {
        let (mut amplitude, mut max_amplitude, mut scale, mut distance) =
            (1f32, max_distance, 1f32, 8f32);
        let zero_input = self.detail == 0.0 || self.roughness == 0.0;
        let mut i = 0;
        while i as f32 <= self.detail.ceil() {
            let od = self.edge(coord.map(|c| c * scale), n);
            if zero_input {
                distance = od;
                break;
            }
            if i as f32 <= self.detail {
                max_amplitude = mix(max_amplitude, max_distance / scale, amplitude);
                distance = mix(distance, distance.min(od / scale), amplitude);
                scale *= self.lacunarity;
                amplitude *= self.roughness;
            } else {
                let rem = self.detail - self.detail.floor();
                if rem != 0.0 {
                    let la = mix(max_amplitude, max_distance / scale, amplitude);
                    max_amplitude = mix(max_amplitude, la, rem);
                    let ld = mix(distance, distance.min(od / scale), amplitude);
                    distance = mix(distance, distance.min(ld), rem);
                }
            }
            i += 1;
        }
        if self.normalize {
            distance /= max_amplitude;
        }
        distance
    }
}

/// Cycles' `smoothstep(0, 1, x)`.
fn smoothstep01(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x >= 1.0 {
        1.0
    } else {
        (3.0 - 2.0 * x) * x * x
    }
}
