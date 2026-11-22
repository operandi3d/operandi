//! Fitting smooth cubic Bezier spans to rough points: a hand-drawn path
//! (the path tools' Sketch mode, path_tools.rs) comes out as a few clean
//! spans instead of hundreds of pointer samples.
//!
//! Adapted from VectorCraft `crates/pathops/src/fit.rs` at commit
//! 36b5a5920c50d0818d1b3994527f5349b3484dbe (the least-squares cubic fit
//! with fixed end tangents, Newton reparameterisation and splitting near
//! the worst point, from the Graphics Gems description), moved from 2D
//! kurbo types to 3D glam points. The maths is unchanged.
//!
//! Copyright (c) 2026 ArtCraft Team and the VectorCraft contributors.
//! Licensed under the MIT licence (VectorCraft is MIT OR Apache-2.0; this
//! file is used under MIT):
//!
//! Permission is hereby granted, free of charge, to any person obtaining a
//! copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to permit
//! persons to whom the Software is furnished to do so, subject to the
//! following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! USE OR OTHER DEALINGS IN THE SOFTWARE.
use glam::DVec3;

/// One cubic span: start, two handles, end.
pub type Cubic = [DVec3; 4];

/// Maximum Newton reparameterisation rounds per fit.
const MAX_REPARAM: usize = 6;
/// Most pieces joined to the first piece of a fitted curve.
const MAX_JOINS: usize = 32;
/// Most samples of a fitted curve checked between two neighbouring points.
const MAX_BULGE_SAMPLES: f64 = 1024.0;
/// About how many points of a long piece its first fit uses.
const COARSE_POINTS: usize = 16;

fn unit(v: DVec3) -> Option<DVec3> {
    let l = v.length();
    (l > 1e-12 && l.is_finite()).then(|| v / l)
}

fn eval(c: &Cubic, t: f64) -> DVec3 {
    let s = 1. - t;
    c[0] * (s * s * s) + c[1] * (3. * s * s * t) + c[2] * (3. * s * t * t) + c[3] * (t * t * t)
}

fn deriv(c: &Cubic, t: f64) -> DVec3 {
    let s = 1. - t;
    (c[1] - c[0]) * (3. * s * s) + (c[2] - c[1]) * (6. * s * t) + (c[3] - c[2]) * (3. * t * t)
}

fn deriv2(c: &Cubic, t: f64) -> DVec3 {
    (c[2] - c[1] * 2. + c[0]) * (6. * (1. - t)) + (c[3] - c[2] * 2. + c[1]) * (6. * t)
}

/// Squared distance from `p` to the segment `a`–`b`.
fn segment_distance_sq(a: DVec3, b: DVec3, p: DVec3) -> f64 {
    let ab = b - a;
    let len = ab.length_squared();
    let t = if len > 0. {
        ((p - a).dot(ab) / len).clamp(0., 1.)
    } else {
        0.
    };
    (a + ab * t).distance_squared(p)
}

/// Fit a chain of cubics through `pts` within `tol` (a distance). `t0` is
/// the unit tangent leaving the first point, `t1` the one arriving at the
/// last (forward). Consecutive spans meet smoothly.
pub fn fit_cubics(pts: &[DVec3], t0: DVec3, t1: DVec3, tol: f64) -> Vec<Cubic> {
    let pts = dedup(pts);
    if pts.len() < 2 {
        return Vec::new();
    }
    let tol = tol.max(1e-9);
    let mut pieces = Vec::new();
    fit_rec(&pts, 0, pts.len() - 1, t0, t1, tol, &mut pieces);
    join_pieces(&pts, &pieces, t0, tol)
}

/// [`fit_cubics`] with the end tangents taken from the points themselves
/// (the first and last few points that are more than `tol` away).
pub fn fit_stroke(pts: &[DVec3], tol: f64) -> Vec<Cubic> {
    let pts = dedup(pts);
    if pts.len() < 2 {
        return Vec::new();
    }
    let first = pts[0];
    let last = pts[pts.len() - 1];
    let ahead = pts
        .iter()
        .find(|q| q.distance(first) > tol)
        .unwrap_or(&last);
    let behind = pts
        .iter()
        .rev()
        .find(|q| q.distance(last) > tol)
        .unwrap_or(&first);
    let t0 = unit(*ahead - first).unwrap_or(DVec3::X);
    let t1 = unit(last - *behind).unwrap_or(t0);
    fit_cubics(&pts, t0, t1, tol)
}

/// The fitted spans as a Bezier point list in `curves`' stored order
/// (a0, out0, in1, a1, …), ready for an open Bezier curve.
pub fn bezier_point_list(spans: &[Cubic]) -> Vec<DVec3> {
    let mut out = Vec::with_capacity(spans.len() * 3 + 1);
    for (i, c) in spans.iter().enumerate() {
        if i == 0 {
            out.push(c[0]);
        }
        out.extend([c[1], c[2], c[3]]);
    }
    out
}

struct Piece {
    curve: Cubic,
    end: usize,
    t1: DVec3,
}

fn join_pieces(pts: &[DVec3], pieces: &[Piece], t0: DVec3, tol: f64) -> Vec<Cubic> {
    let mut out = Vec::new();
    let (mut start, mut leaving, mut next) = (0, t0, 0);
    while let Some(first) = pieces.get(next) {
        let (mut curve, mut end, mut arriving) = (first.curve, first.end, first.t1);
        next += 1;
        for piece in pieces.iter().skip(next).take(MAX_JOINS) {
            let Ok(joined) = fit_piece(&pts[start..=piece.end], leaving, piece.t1, tol) else {
                break;
            };
            (curve, end, arriving) = (joined, piece.end, piece.t1);
            next += 1;
        }
        out.push(curve);
        (start, leaving) = (end, arriving);
    }
    out
}

fn dedup(pts: &[DVec3]) -> Vec<DVec3> {
    let mut v: Vec<DVec3> = Vec::with_capacity(pts.len());
    for &p in pts {
        if p.is_finite() && v.last().is_none_or(|q| q.distance(p) > 1e-12) {
            v.push(p);
        }
    }
    v
}

fn fit_rec(
    all: &[DVec3],
    start: usize,
    end: usize,
    t0: DVec3,
    t1: DVec3,
    tol: f64,
    out: &mut Vec<Piece>,
) {
    let pts = &all[start..=end];
    let n = pts.len();
    if n == 2 {
        let d = (pts[0].distance(pts[1]) / 3.0).min(tol);
        out.push(Piece {
            curve: [pts[0], pts[0] + t0 * d, pts[1] - t1 * d, pts[1]],
            end,
            t1,
        });
        return;
    }
    let split = match fit_piece(pts, t0, t1, tol) {
        Ok(curve) => {
            out.push(Piece { curve, end, t1 });
            return;
        }
        Err(worst) => worst,
    };
    let margin = (n * 2 / 5).max(1);
    let split = split.clamp(margin, n - 1 - margin);
    let tc = center_tangent(pts, split, tol).unwrap_or(t0);
    fit_rec(all, start, start + split, t0, tc, tol, out);
    fit_rec(all, start + split, end, tc, t1, tol, out);
}

fn fit_piece(pts: &[DVec3], t0: DVec3, t1: DVec3, tol: f64) -> Result<Cubic, usize> {
    let give_up = tol * 8.0 + pts[0].distance(pts[pts.len() - 1]) * 0.05;
    if pts.len() >= COARSE_POINTS * 2 {
        let stride = pts.len() / COARSE_POINTS;
        let some: Vec<DVec3> = pts
            .iter()
            .step_by(stride)
            .chain(pts.last())
            .copied()
            .collect();
        let (_, err, worst, _) = fit_single_within(&some, chord_params(&some), t0, t1, give_up);
        if err > tol {
            return Err((worst * stride).min(pts.len() - 1));
        }
    }
    let (c, err, worst, u) = fit_single_within(pts, chord_params(pts), t0, t1, give_up);
    if err > tol {
        return Err(worst);
    }
    bulge(pts, &c, &u, tol).map_or(Ok(c), Err)
}

fn center_tangent(pts: &[DVec3], split: usize, tol: f64) -> Option<DVec3> {
    let p = pts[split];
    let before = pts[..split]
        .iter()
        .rev()
        .find(|q| q.distance(p) > tol)
        .unwrap_or(&pts[0]);
    let after = pts[split + 1..]
        .iter()
        .find(|q| q.distance(p) > tol)
        .unwrap_or(&pts[pts.len() - 1]);
    unit(*after - *before)
}

fn fit_single_within(
    pts: &[DVec3],
    mut u: Vec<f64>,
    t0: DVec3,
    t1: DVec3,
    give_up: f64,
) -> (Cubic, f64, usize, Vec<f64>) {
    let mut best = generate(pts, &u, t0, t1);
    let (mut best_err, mut best_split) = max_error(pts, &best, &u);
    if best_err > give_up {
        return (best, best_err, best_split, u);
    }
    let mut best_u = u.clone();
    for _ in 0..MAX_REPARAM {
        for (i, ui) in u.iter_mut().enumerate() {
            *ui = newton(&best, pts[i], *ui);
        }
        let c = generate(pts, &u, t0, t1);
        let (e, s) = max_error(pts, &c, &u);
        if e < best_err {
            best = c;
            best_err = e;
            best_split = s;
            best_u.copy_from_slice(&u);
        } else {
            break;
        }
    }
    (best, best_err, best_split, best_u)
}

/// Index of the first point after which the curve strays over 99% of `tol`
/// from the line to the next point (see the VectorCraft notes on the
/// sample spacing).
fn bulge(pts: &[DVec3], c: &Cubic, u: &[f64], tol: f64) -> Option<usize> {
    let step = (0.01 * tol * 8.0 / (6.0 * largest_second_difference(c)).max(1e-300)).sqrt();
    let limit = tol * 0.99;
    pts.windows(2).zip(u.windows(2)).position(|(p, t)| {
        let samples = ((t[1] - t[0]).abs() / step).ceil().max(1.0);
        if samples > MAX_BULGE_SAMPLES {
            return true;
        }
        let samples = samples as usize;
        (1..=samples).any(|k| {
            let at = t[0] + (t[1] - t[0]) * k as f64 / (samples + 1) as f64;
            segment_distance_sq(p[0], p[1], eval(c, at)) > limit * limit
        })
    })
}

fn largest_second_difference(c: &Cubic) -> f64 {
    (c[2] - c[1] - (c[1] - c[0]))
        .length()
        .max((c[3] - c[2] - (c[2] - c[1])).length())
}

fn chord_params(pts: &[DVec3]) -> Vec<f64> {
    let mut u = Vec::with_capacity(pts.len());
    let mut acc = 0.0;
    u.push(0.0);
    for w in pts.windows(2) {
        acc += w[0].distance(w[1]);
        u.push(acc);
    }
    if acc > 0.0 {
        for x in &mut u {
            *x /= acc;
        }
    }
    u
}

fn bern(t: f64) -> [f64; 4] {
    let s = 1.0 - t;
    [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t]
}

/// Solve the 2×2 normal equations for the handle lengths along the fixed
/// tangents.
fn generate(pts: &[DVec3], u: &[f64], t0: DVec3, t1: DVec3) -> Cubic {
    let (Some(&p0), Some(&p3)) = (pts.first(), pts.last()) else {
        return [DVec3::ZERO; 4];
    };
    let dir_in = -t1;
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (p, &t) in pts.iter().zip(u) {
        let b = bern(t);
        let a0 = t0 * b[1];
        let a1 = dir_in * b[2];
        c00 += a0.dot(a0);
        c01 += a0.dot(a1);
        c11 += a1.dot(a1);
        let tmp = *p - (p0 * (b[0] + b[1]) + p3 * (b[2] + b[3]));
        x0 += a0.dot(tmp);
        x1 += a1.dot(tmp);
    }
    let det = c00 * c11 - c01 * c01;
    let seg = p0.distance(p3);
    let eps = 1e-6 * seg;
    let (mut al, mut ar) = if det.abs() > 1e-12 {
        ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det)
    } else {
        (seg / 3.0, seg / 3.0)
    };
    if !al.is_finite() || !ar.is_finite() || al < eps || ar < eps {
        al = seg / 3.0;
        ar = seg / 3.0;
    }
    let cap = seg * 4.0 + 1e-9;
    al = al.min(cap);
    ar = ar.min(cap);
    [p0, p0 + t0 * al, p3 + dir_in * ar, p3]
}

fn max_error(pts: &[DVec3], c: &Cubic, u: &[f64]) -> (f64, usize) {
    let mut e = 0.0;
    let mut idx = pts.len() / 2;
    for (i, (p, &t)) in pts.iter().zip(u).enumerate() {
        let d = eval(c, t).distance(*p);
        if d > e {
            e = d;
            idx = i;
        }
    }
    (e, idx)
}

fn newton(c: &Cubic, p: DVec3, t: f64) -> f64 {
    let q = eval(c, t) - p;
    let q1 = deriv(c, t);
    let q2 = deriv2(c, t);
    let num = q.dot(q1);
    let den = q1.dot(q1) + q.dot(q2);
    if den.abs() < 1e-12 {
        return t;
    }
    let nt = t - num / den;
    if nt.is_finite() {
        nt.clamp(0.0, 1.0)
    } else {
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_noisy_arc_fits_a_few_spans_within_tolerance() {
        // A quarter circle of radius 1 with small wobble, 200 samples.
        let pts: Vec<DVec3> = (0..200)
            .map(|i| {
                let a = i as f64 / 199. * std::f64::consts::FRAC_PI_2;
                let r = 1. + 0.002 * (i as f64 * 1.7).sin();
                DVec3::new(r * a.cos(), 0., r * a.sin())
            })
            .collect();
        let spans = fit_stroke(&pts, 0.01);
        assert!(!spans.is_empty() && spans.len() <= 4, "{}", spans.len());
        // Every sample is near the fitted chain.
        for p in &pts {
            let near = spans
                .iter()
                .flat_map(|c| (0..=64).map(move |k| eval(c, k as f64 / 64.)))
                .map(|q| q.distance(*p))
                .fold(f64::INFINITY, f64::min);
            assert!(near < 0.02, "{near}");
        }
        let list = bezier_point_list(&spans);
        assert_eq!(list.len(), spans.len() * 3 + 1);
        assert!(list[0].distance(pts[0]) < 1e-9);
    }

    #[test]
    fn a_straight_stroke_is_one_span() {
        let pts: Vec<DVec3> = (0..50)
            .map(|i| DVec3::new(i as f64 * 0.1, 0., 0.))
            .collect();
        assert_eq!(fit_stroke(&pts, 0.01).len(), 1);
    }
}
