// SPDX-FileCopyrightText: 2023 Blender Authors
// SPDX-FileCopyrightText: 2001 softSurfer (http://www.softsurfer.com)
// SPDX-FileCopyrightText: 2026 Operandi contributors
// SPDX-License-Identifier: GPL-2.0-or-later
//
// Adapted from Blender `source/blender/blenlib/intern/convexhull_2d.cc` and
// the geometry helpers of `source/blender/geometry/intern/uv_parametrizer.cc`
// at commit b4eed6fd498e76a7e93404bac2326e9f84deba22. The polygon
// triangulation is Operandi's own (Blender uses its polyfill module).

//! Small 2D/3D geometry helpers shared by the unwrap and pack code.

use glam::{DVec2, DVec3};

/// Angle at `b` between `a` and `c`, in radians.
pub(crate) fn angle_v3(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    let u = (a - b).normalize_or_zero();
    let v = (c - b).normalize_or_zero();
    u.dot(v).clamp(-1.0, 1.0).acos()
}

/// Angle at `b` between `a` and `c` in 2D, in radians.
pub(crate) fn angle_v2(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    let u = (a - b).normalize_or_zero();
    let v = (c - b).normalize_or_zero();
    u.dot(v).clamp(-1.0, 1.0).acos()
}

/// Cosine of the angle at `b`.
pub(crate) fn cos_v3(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    let u = (a - b).normalize_or_zero();
    let v = (c - b).normalize_or_zero();
    u.dot(v)
}

/// Angles close to 180° make the solver rows zero; cap the largest angle at
/// 179° and give the rest to the other two (`fix_large_angle`).
fn fix_large_angle(v_fix: DVec3, v1: DVec3, v2: DVec3, fix: &mut f64, a1: &mut f64, a2: &mut f64) {
    let max_angle = 179.0_f64.to_radians();
    let fix_amount = *fix - max_angle;
    if fix_amount < 0.0 {
        return;
    }
    let d1 = v_fix.distance(v1);
    let d2 = v_fix.distance(v2);
    let sum = d1 + d2;
    let weight = if sum > 1e-20 { d2 / sum } else { 0.5 };
    *fix -= fix_amount;
    *a1 += fix_amount * weight;
    *a2 += fix_amount * (1.0 - weight);
}

/// The three corner angles of a triangle, guarded against degenerate shapes
/// (`p_triangle_angles`).
pub(crate) fn triangle_angles(v1: DVec3, v2: DVec3, v3: DVec3) -> [f64; 3] {
    let mut a1 = angle_v3(v3, v1, v2);
    let mut a2 = angle_v3(v1, v2, v3);
    let mut a3 = angle_v3(v2, v3, v1);
    fix_large_angle(v1, v2, v3, &mut a1, &mut a2, &mut a3);
    fix_large_angle(v2, v3, v1, &mut a2, &mut a3, &mut a1);
    fix_large_angle(v3, v1, v2, &mut a3, &mut a1, &mut a2);
    [a1.max(0.001), a2.max(0.001), a3.max(0.001)]
}

pub(crate) fn triangle_area(a: DVec3, b: DVec3, c: DVec3) -> f64 {
    0.5 * (b - a).cross(c - a).length()
}

/// Signed area of a 2D triangle (positive when counter-clockwise).
pub(crate) fn area_signed(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
}

/// Newell normal of a polygon; +Z when degenerate.
pub(crate) fn polygon_normal(co: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    let mut prev = co[co.len() - 1];
    for &cur in co {
        n.x += (prev.y - cur.y) * (prev.z + cur.z);
        n.y += (prev.z - cur.z) * (prev.x + cur.x);
        n.z += (prev.x - cur.x) * (prev.y + cur.y);
        prev = cur;
    }
    let len = n.length();
    if len > 0.0 && len.is_finite() {
        n / len
    } else {
        DVec3::Z
    }
}

/// Projects points onto the plane of `normal`, keeping counter-clockwise
/// winding positive.
pub(crate) fn project_to_plane(co: &[DVec3], normal: DVec3) -> Vec<DVec2> {
    // u × v = normal, so a polygon winding counter-clockwise about the
    // normal stays counter-clockwise in (u, v).
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    co.iter().map(|p| DVec2::new(p.dot(u), p.dot(v))).collect()
}

/// Triangulates a simple polygon given in 3D (ear clipping in the polygon's
/// plane, then edge flips toward better-shaped triangles). Returns index
/// triples into `co`; always `co.len() - 2` triangles.
pub(crate) fn triangulate(co: &[DVec3]) -> Vec<[usize; 3]> {
    let n = co.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }
    let p = project_to_plane(co, polygon_normal(co));
    let mut tris = ear_clip(&p);
    beautify(&p, &mut tris);
    tris
}

fn point_in_triangle(p: DVec2, a: DVec2, b: DVec2, c: DVec2) -> bool {
    area_signed(a, b, p) >= 0.0 && area_signed(b, c, p) >= 0.0 && area_signed(c, a, p) >= 0.0
}

fn ear_clip(p: &[DVec2]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..p.len()).collect();
    let mut tris = Vec::with_capacity(p.len() - 2);
    let mut guard = 0;
    while idx.len() > 3 {
        let m = idx.len();
        let mut clipped = false;
        // Prefer the ear with the best smallest angle among valid ears.
        let mut best: Option<(usize, f64)> = None;
        for i in 0..m {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            if area_signed(p[a], p[b], p[c]) <= 0.0 {
                continue;
            }
            let blocked = idx
                .iter()
                .any(|&k| k != a && k != b && k != c && point_in_triangle(p[k], p[a], p[b], p[c]));
            if blocked {
                continue;
            }
            let q = min_angle_2d(p[a], p[b], p[c]);
            if best.is_none_or(|(_, bq)| q > bq) {
                best = Some((i, q));
            }
        }
        if let Some((i, _)) = best {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            tris.push([a, b, c]);
            idx.remove(i);
            clipped = true;
        }
        if !clipped {
            // Degenerate or self-touching polygon: cut a fan from what's left.
            for i in 1..idx.len() - 1 {
                tris.push([idx[0], idx[i], idx[i + 1]]);
            }
            return tris;
        }
        guard += 1;
        if guard > p.len() * 2 {
            break;
        }
    }
    if idx.len() == 3 {
        tris.push([idx[0], idx[1], idx[2]]);
    }
    tris
}

fn min_angle_2d(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    angle_v2(c, a, b)
        .min(angle_v2(a, b, c))
        .min(angle_v2(b, c, a))
}

/// Lawson flips: swap the diagonal of two neighbouring triangles when the
/// other diagonal gives a larger smallest angle and the quad is convex.
fn beautify(p: &[DVec2], tris: &mut [[usize; 3]]) {
    let n = p.len();
    for _ in 0..(n * n).max(4) {
        let mut flipped = false;
        for t in 0..tris.len() {
            for s in t + 1..tris.len() {
                if let Some((a, b, c, d)) = shared_edge(tris[t], tris[s]) {
                    // Triangles (a, b, c) and (b, a, d) share edge a-b.
                    if area_signed(p[c], p[d], p[b]) <= 1e-12
                        || area_signed(p[d], p[c], p[a]) <= 1e-12
                    {
                        continue;
                    }
                    let now = min_angle_2d(p[a], p[b], p[c]).min(min_angle_2d(p[b], p[a], p[d]));
                    let then = min_angle_2d(p[c], p[d], p[b]).min(min_angle_2d(p[d], p[c], p[a]));
                    if then > now + 1e-9 {
                        tris[t] = [c, d, b];
                        tris[s] = [d, c, a];
                        flipped = true;
                    }
                }
            }
        }
        if !flipped {
            break;
        }
    }
}

/// When two counter-clockwise triangles share an edge, returns (a, b, c, d)
/// with the first as (a, b, c) and the second as (b, a, d).
fn shared_edge(t: [usize; 3], s: [usize; 3]) -> Option<(usize, usize, usize, usize)> {
    for i in 0..3 {
        let (a, b, c) = (t[i], t[(i + 1) % 3], t[(i + 2) % 3]);
        for j in 0..3 {
            if s[j] == b && s[(j + 1) % 3] == a {
                return Some((a, b, c, s[(j + 2) % 3]));
            }
        }
    }
    None
}

fn is_left(p0: DVec2, p1: DVec2, p2: DVec2) -> f64 {
    (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y)
}

/// Convex hull of 2D points (`BLI_convexhull_2d`): indices into `points`,
/// counter-clockwise.
pub(crate) fn convex_hull(points: &[DVec2]) -> Vec<usize> {
    let n = points.len();
    if n < 2 {
        return (0..n).collect();
    }
    // Blender sorts by ascending Y with X as the tie breaker and then runs the
    // softSurfer monotone chain on that order.
    let mut map: Vec<usize> = (0..n).collect();
    map.sort_by(|&a, &b| {
        let (pa, pb) = (points[a], points[b]);
        pa.y.total_cmp(&pb.y).then(pa.x.total_cmp(&pb.x))
    });
    let p: Vec<DVec2> = map.iter().map(|&i| points[i]).collect();
    let mut stack: Vec<usize> = Vec::with_capacity(n);
    let push = |stack: &mut Vec<usize>, i: usize| {
        while stack.len() > 1
            && is_left(p[stack[stack.len() - 2]], p[i], p[stack[stack.len() - 1]]) >= 0.0
        {
            stack.pop();
        }
        stack.push(i);
    };

    let minmin = 0;
    let maxmax = n - 1;
    let xmin = p[0].x;
    let mut i = 1;
    while i <= maxmax && p[i].x == xmin {
        i += 1;
    }
    let minmax = i - 1;
    if minmax == maxmax {
        push(&mut stack, minmin);
        if p[minmax].y != p[minmin].y {
            push(&mut stack, minmax);
        }
        return finish(&p, stack, &map);
    }
    let xmax = p[maxmax].x;
    let mut i = maxmax as isize - 1;
    while i >= 0 && p[i as usize].x == xmax {
        i -= 1;
    }
    let maxmin = (i + 1) as usize;

    push(&mut stack, minmin);
    let mut i = minmax;
    while {
        i += 1;
        i <= maxmin
    } {
        if i < maxmin && is_left(p[minmin], p[maxmin], p[i]) >= 0.0 {
            continue;
        }
        while stack.len() > 1 {
            let t = stack.len() - 1;
            if is_left(p[stack[t - 1]], p[stack[t]], p[i]) > 0.0 {
                break;
            }
            stack.pop();
        }
        push(&mut stack, i);
    }
    if maxmax != maxmin {
        push(&mut stack, maxmax);
    }
    let bot = stack.len() - 1;
    let mut i = maxmin as isize;
    loop {
        i -= 1;
        if i < minmax as isize {
            break;
        }
        let iu = i as usize;
        if iu > minmax && is_left(p[maxmax], p[minmax], p[iu]) >= 0.0 {
            continue;
        }
        while stack.len() - 1 > bot {
            let t = stack.len() - 1;
            if is_left(p[stack[t - 1]], p[stack[t]], p[iu]) > 0.0 {
                break;
            }
            stack.pop();
        }
        if p[iu] == p[stack[0]] {
            return finish(&p, stack, &map);
        }
        push(&mut stack, iu);
    }
    if minmax != minmin && stack[0] != minmin {
        push(&mut stack, minmin);
    }
    finish(&p, stack, &map)
}

fn finish(p: &[DVec2], stack: Vec<usize>, map: &[usize]) -> Vec<usize> {
    // `convexhull_2d_stack_finalize`: drop concave ends.
    let (mut bot, mut top) = (0usize, stack.len().saturating_sub(1));
    while top >= bot + 2 {
        if is_left(p[stack[top - 1]], p[stack[bot]], p[stack[top]]) >= 0.0 {
            top -= 1;
            continue;
        }
        if is_left(p[stack[top]], p[stack[bot + 1]], p[stack[bot]]) >= 0.0 {
            bot += 1;
            continue;
        }
        break;
    }
    if stack.is_empty() {
        return Vec::new();
    }
    stack[bot..=top].iter().map(|&i| map[i]).collect()
}

/// The rotation that makes the axis-aligned box of `points` smallest, as
/// (sin, cos) of the hull edge it aligns with
/// (`BLI_convexhull_aabb_fit_points_2d_as_sincos`). (0, 1) means no rotation.
pub(crate) fn aabb_fit_sincos(points: &[DVec2]) -> DVec2 {
    let hull: Vec<DVec2> = convex_hull(points).into_iter().map(|i| points[i]).collect();
    let mut best = DVec2::new(0.0, 1.0);
    if hull.len() < 2 {
        return best;
    }
    let mut best_area = f64::MAX;
    for i in 0..hull.len() {
        let d = hull[(i + 1) % hull.len()] - hull[i];
        let len = d.length();
        if len == 0.0 {
            continue;
        }
        let sc = d / len;
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for &p in &hull {
            let t = DVec2::new(sc.x * p.x + sc.y * p.y, sc.y * p.x - sc.x * p.y);
            lo = lo.min(t);
            hi = hi.max(t);
        }
        let area = (hi.x - lo.x) * (hi.y - lo.y);
        if area < best_area {
            best_area = area;
            best = sc;
        }
    }
    best
}

/// The rotation (radians, within ±45°) that turns `points` so the smallest
/// enclosing rectangle is axis aligned (`p_chart_minimum_area_angle`, found
/// here by testing each hull edge rather than with rotating calipers).
pub(crate) fn minimum_area_angle(points: &[DVec2]) -> f64 {
    let hull: Vec<DVec2> = convex_hull(points).into_iter().map(|i| points[i]).collect();
    if hull.len() < 3 {
        return 0.0;
    }
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut best = (f64::MAX, 0.0_f64);
    for i in 0..hull.len() {
        let d = hull[(i + 1) % hull.len()] - hull[i];
        if d.length_squared() == 0.0 {
            continue;
        }
        let mut angle = -d.y.atan2(d.x);
        angle = angle.rem_euclid(quarter);
        if angle > quarter / 2.0 {
            angle -= quarter;
        }
        let (s, c) = angle.sin_cos();
        let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
        for &p in &hull {
            let t = DVec2::new(c * p.x - s * p.y, s * p.x + c * p.y);
            lo = lo.min(t);
            hi = hi.max(t);
        }
        let area = (hi.x - lo.x) * (hi.y - lo.y);
        if area < best.0 * (1.0 - 1e-9) || (area <= best.0 && angle.abs() < best.1.abs()) {
            best = (area, angle);
        }
    }
    best.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hull_of_square_with_inside_point() {
        let pts = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.5, 0.5),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let mut h = convex_hull(&pts);
        assert_eq!(h.len(), 4);
        assert!(!h.contains(&2));
        // Counter-clockwise.
        let poly: Vec<DVec2> = h.iter().map(|&i| pts[i]).collect();
        let mut a = 0.0;
        for i in 0..poly.len() {
            a += poly[i].perp_dot(poly[(i + 1) % poly.len()]);
        }
        assert!(a > 0.0);
        h.sort();
        assert_eq!(h, vec![0, 1, 3, 4]);
    }

    #[test]
    fn rotated_rectangle_angle() {
        let a = 0.3_f64;
        let (s, c) = a.sin_cos();
        let rect = [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0)];
        let pts: Vec<DVec2> = rect
            .iter()
            .map(|&(x, y)| DVec2::new(c * x - s * y, s * x + c * y))
            .collect();
        let got = minimum_area_angle(&pts);
        assert!((got + a).abs() < 1e-9, "{got}");
    }

    #[test]
    fn triangulates_concave_polygon() {
        // An L shape.
        let co: Vec<DVec3> = [(0., 0.), (2., 0.), (2., 1.), (1., 1.), (1., 2.), (0., 2.)]
            .iter()
            .map(|&(x, y)| DVec3::new(x, y, 0.0))
            .collect();
        let tris = triangulate(&co);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris
            .iter()
            .map(|t| triangle_area(co[t[0]], co[t[1]], co[t[2]]))
            .sum();
        assert!((area - 3.0).abs() < 1e-12);
    }
}
