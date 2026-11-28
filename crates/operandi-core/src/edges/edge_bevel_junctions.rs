//! Edge Bevel / Edge Chamfer construction
//! (specifications/edge-bevel-chamfer-2026-11-02, EBC-J/P/B3), including ends
//! at interior points (new-point-uvs UVN-T-01) and on open borders
//! (EBC-B3-06).
//!
//! Every point touched by a selected edge is removed. Around it:
//! * each unselected edge that shares a polygon corner with a selected edge
//!   gets a point `width` along it (Inset) or far enough along it that the
//!   strip stays `width` from that selected edge (Chamfer, Parallel). Edge
//!   Bevel also slides along the other edges at the point; Edge Chamfer keeps
//!   the point there (EBC-B3-06's interior end);
//! * each polygon corner between two selected edges gets the point where the
//!   two offsets meet;
//! * polygons keep their corners there (a corner with no selected edge takes
//!   the points on both of its edges, with the profile between them when they
//!   are the two ends of one strip);
//! * each selected edge becomes a strip, rounded by `rows`;
//! * whatever opening is left around the point is closed: a corner patch of
//!   quads where strips with two or more rows meet (EBC-J-05, EBC-B3-04), one
//!   polygon otherwise (a triangle at three selected edges, an end triangle
//!   at an interior point). Openings on an open border stay open.
use crate::corner_uv::{UvRows, corner_uv, lerp, records_from_uvs};
use crate::edge_bevel::{
    CornerPattern, EdgeBevelMode, EdgeBevelResult, EdgeBevelSettings, EdgeBevelShape, EdgeEnd,
};
use crate::topology::edge_key;
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// A linear combination of points (weights may be negative).
type Comb = Vec<(u32, f64)>;

/// How a new point was made, for its UVs.
#[derive(Clone)]
enum Def {
    /// `t` of the way from `v` to `s` along an edge.
    Edge { v: u32, s: u32, t: f64 },
    /// Inside `face`: `v` + `ta`·(`a` − `v`) + `tb`·(`b` − `v`).
    Inside {
        face: usize,
        v: u32,
        a: u32,
        b: u32,
        ta: f64,
        tb: f64,
    },
    /// A profile row `t` of the way from `first` to `last`.
    Row { first: u32, last: u32, t: f64 },
    /// A corner-patch point: the same combination of other points' UVs as
    /// of their positions (EBC-B3-05: this reproduces the measured even
    /// patch centre).
    Blend(Comb),
}

fn uv_in(
    mesh: &Mesh,
    rows: &UvRows,
    defs: &BTreeMap<u32, Def>,
    face: usize,
    id: u32,
) -> Option<DVec2> {
    match defs.get(&id) {
        None => corner_uv(mesh, rows, face, id),
        Some(&Def::Edge { v, s, t }) => lerp(
            corner_uv(mesh, rows, face, v),
            corner_uv(mesh, rows, face, s),
            t,
        ),
        Some(&Def::Inside {
            face: f,
            v,
            a,
            b,
            ta,
            tb,
        }) => {
            if f != face {
                // A point placed inside one polygon that lands on one of
                // its edges (Loop Slide off, where the strip turns round a
                // fold of the solid) is also on the polygon across that
                // edge: read it there along the edge.
                const ON: f64 = 1e-9;
                return if tb.abs() < ON {
                    lerp(
                        corner_uv(mesh, rows, face, v),
                        corner_uv(mesh, rows, face, a),
                        ta,
                    )
                } else if ta.abs() < ON {
                    lerp(
                        corner_uv(mesh, rows, face, v),
                        corner_uv(mesh, rows, face, b),
                        tb,
                    )
                } else {
                    None
                };
            }
            let (o, pa, pb) = (
                corner_uv(mesh, rows, f, v)?,
                corner_uv(mesh, rows, f, a)?,
                corner_uv(mesh, rows, f, b)?,
            );
            Some(o + (pa - o) * ta + (pb - o) * tb)
        }
        Some(&Def::Row { first, last, t }) => lerp(
            uv_in(mesh, rows, defs, face, first),
            uv_in(mesh, rows, defs, face, last),
            t,
        ),
        Some(Def::Blend(comb)) => comb.iter().try_fold(DVec2::ZERO, |sum, &(id, w)| {
            Some(sum + uv_in(mesh, rows, defs, face, id)? * w)
        }),
    }
}

/// UV of a point on a new strip or end polygon: its value in a source
/// polygon it lies on (the one given first when it lies on several).
fn uv_on(
    mesh: &Mesh,
    rows: &UvRows,
    defs: &BTreeMap<u32, Def>,
    homes: &[usize],
    id: u32,
) -> Option<DVec2> {
    match defs.get(&id) {
        Some(&Def::Row { first, last, t }) => lerp(
            uv_on(mesh, rows, defs, homes, first),
            uv_on(mesh, rows, defs, homes, last),
            t,
        ),
        Some(Def::Blend(comb)) => comb.iter().try_fold(DVec2::ZERO, |sum, &(id, w)| {
            Some(sum + uv_on(mesh, rows, defs, homes, id)? * w)
        }),
        Some(&Def::Inside { face, .. }) => uv_in(mesh, rows, defs, face, id),
        Some(&Def::Edge { v, s, .. }) => {
            let on = |f: usize| {
                mesh.faces[f].vertices.contains(&v) && mesh.faces[f].vertices.contains(&s)
            };
            let face = homes
                .iter()
                .copied()
                .find(|&f| on(f))
                .or_else(|| (0..mesh.faces.len()).find(|&f| on(f)))?;
            uv_in(mesh, rows, defs, face, id)
        }
        None => homes.iter().find_map(|&f| corner_uv(mesh, rows, f, id)),
    }
}

/// UV of a strip point in the frame of the strip's side `f2`: its value in
/// `f2` where it lies on `f2`, otherwise its value in `f1` carried across the
/// selected edge `x`–`y` by the map that takes `f1`'s UVs of `x` and `y` to
/// `f2`'s (a shift at a plain seam; also turns and scales a rotated island).
fn strip_uv(
    mesh: &Mesh,
    rows: &UvRows,
    defs: &BTreeMap<u32, Def>,
    (x, y): (u32, u32),
    f1: usize,
    f2: usize,
    id: u32,
) -> Option<DVec2> {
    match defs.get(&id) {
        Some(&Def::Row { first, last, t }) => lerp(
            strip_uv(mesh, rows, defs, (x, y), f1, f2, first),
            strip_uv(mesh, rows, defs, (x, y), f1, f2, last),
            t,
        ),
        Some(Def::Blend(comb)) => comb.iter().try_fold(DVec2::ZERO, |sum, &(c, w)| {
            Some(sum + strip_uv(mesh, rows, defs, (x, y), f1, f2, c)? * w)
        }),
        _ => uv_in(mesh, rows, defs, f2, id).or_else(|| {
            let z = uv_in(mesh, rows, defs, f1, id)?;
            let (a1, b1) = (corner_uv(mesh, rows, f1, x)?, corner_uv(mesh, rows, f1, y)?);
            let (a2, b2) = (corner_uv(mesh, rows, f2, x)?, corner_uv(mesh, rows, f2, y)?);
            let (d1, d2) = (b1 - a1, b2 - a2);
            if d1.length_squared() < 1e-24 {
                return Some(z + (a2 - a1));
            }
            // Complex ratio d2 / d1: the turn and scale between the frames.
            let r = DVec2::new(d2.x * d1.x + d2.y * d1.y, d2.y * d1.x - d2.x * d1.y)
                / d1.length_squared();
            let q = z - a1;
            Some(a2 + DVec2::new(r.x * q.x - r.y * q.y, r.x * q.y + r.y * q.x))
        }),
    }
}

/// UV of a point on an Edge Bevel end polygon (UVN-T-01): a point slid
/// along an edge from removed point `v` keeps the UV interpolated along that
/// edge only when every polygon on the edge carries `v`'s continuous UV;
/// next to a seam it takes `v`'s continuous UV instead (measured: two of
/// the end triangle's corners). `None` for other points and points at a
/// fold, which follow [`uv_on`].
fn end_uv(
    mesh: &Mesh,
    rows: &UvRows,
    points: Option<&BTreeMap<u32, DVec2>>,
    defs: &BTreeMap<u32, Def>,
    corners: &BTreeMap<u32, Vec<Corner>>,
    flat_at: &dyn Fn(u32) -> bool,
    id: u32,
) -> Option<Option<DVec2>> {
    if let Some(&Def::Edge { v, s, .. }) = defs.get(&id)
        && let Some(fan) = corners.get(&v)
        && flat_at(v)
    {
        let at = |f: usize| corner_uv(mesh, rows, f, v);
        let continuous =
            crate::corner_uv::continuous_uv(points, v, fan.iter().map(|c| (c.face, at(c.face))));
        let sides = fan.iter().filter(|c| c.prev == s || c.next == s);
        if !sides
            .clone()
            .all(|c| crate::corner_uv::is_continuous(at(c.face), continuous))
        {
            return Some(continuous);
        }
    }
    None
}

/// Where the round profile between side points `a` and `b` turns: a point
/// on the selected edge from `v` toward `other`.
///
/// The forum's oval-corner report (2026-11-27): the corner used to sit
/// level with the midpoint of `a` and `b`. Where the two side points sit at
/// different places along the edge (Loop Slide sliding them along slanted
/// side edges, as at a hexagon's corner), it was then nearer one side than
/// the other and the profile came out as a lopsided oval. Now it is the
/// point on the edge as far from `a` as from `b` (at a plain end where both
/// slid the same distance, the old point itself), so the profile is a true
/// circular arc tangent to both sides; the arc's plane leans a little along
/// the edge. Where that point is far from the side points (they're level
/// but at unequal distances from the edge, so no such circle exists), it
/// keeps the midpoint level and [`profile`] draws the roundest curve that
/// fits, an elliptic arc.
///
/// `at_point` (Edge Bevel with Loop Slide on, EBC-Q-RND1 answered
/// 2026-11-28): both side points slid from `v` itself, and the profile
/// turns about `v`: the quarter circle stretched to the two distances
/// (round when they're equal), unless the surface is flat across the edge.
fn profile_corner(v: DVec3, other: DVec3, a: DVec3, b: DVec3, at_point: bool) -> DVec3 {
    let axis = (other - v).normalize_or_zero();
    let (ra, rb) = (a - v, b - v);
    let level = ((a + b) / 2. - v).dot(axis);
    let (sa, sb) = (ra.dot(axis), rb.dot(axis));
    // Side points level along the edge (a straight end, or where three
    // strips meet, EBC-B3-04) keep the measured corner level with them.
    if at_point && (sa - sb).abs() > 1e-9 * (ra.length() + rb.length()) {
        // The fold across the edge: the side points' directions seen along
        // the edge (a flat surface folds 0° however far along they slid).
        let across = |r: DVec3| (r - axis * r.dot(axis)).normalize_or_zero();
        let (ua, ub) = (across(ra), across(rb));
        let fold = std::f64::consts::PI - ua.dot(ub).clamp(-1., 1.).acos();
        if fold > 1e-3 {
            return v;
        }
    }
    // Distances from the edge line: how far the side points are from it.
    let (ha, hb) = ((ra - axis * sa).length(), (rb - axis * sb).length());
    let reach = ha.max(hb);
    let even = if (sa - sb).abs() > 1e-12 * reach.max(1e-300) {
        // |a - c| = |b - c| for c = v + axis s.
        Some((ra.length_squared() - rb.length_squared()) / (2. * (sa - sb)))
    } else {
        None
    };
    // Lean the profile at most half its reach from the side points' level:
    // beyond that the strip would twist more than it gains in roundness.
    // Less where the surface hardly folds across the edge, none where it is
    // flat: there the profile is the straight chord, and a leaning corner
    // would bend it sideways within the surface.
    let (ua, ub) = (
        (a - (v + axis * level)).normalize_or_zero(),
        (b - (v + axis * level)).normalize_or_zero(),
    );
    let fold = std::f64::consts::PI - ua.dot(ub).clamp(-1., 1.).acos();
    let lean = 0.5 * reach * (fold / std::f64::consts::FRAC_PI_2).min(1.);
    let s = match even {
        Some(s) if (s - level).abs() <= lean && fold > 1e-3 => s,
        _ => level,
    };
    v + axis * s
}

/// Profile rows strictly between `q1` and `q2` around `corner`: equal angles
/// on the arc tangent to both sides (EBC-P-01), offset from the chord scaled
/// by `depth` percent (EBC-P-03); a rational arc for unequal sides. With
/// `square` there is one row, at the corner (EBC-P-02, EBC-B3-08).
pub(crate) fn profile(
    corner: DVec3,
    q1: DVec3,
    q2: DVec3,
    rows: u32,
    depth: f64,
    square: bool,
) -> Vec<DVec3> {
    let toward = |round: DVec3| {
        let chord = q2 - q1;
        let along = (round - q1).dot(chord) / chord.length_squared().max(1e-300);
        let foot = q1 + chord * along;
        foot + (round - foot) * (depth / 100.)
    };
    if square {
        // EBS-03: Depth doesn't flatten Square or Sharp.
        return vec![corner];
    }
    let (u, w) = (
        (q1 - corner).normalize_or_zero(),
        (q2 - corner).normalize_or_zero(),
    );
    let (d1, d2) = (q1.distance(corner), q2.distance(corner));
    let half = u.dot(w).clamp(-1., 1.).acos() / 2.;
    let arc =
        ((d1 - d2).abs() <= 1e-9 * d1.max(d2) && half > 1e-6 && half.cos() > 1e-6).then(|| {
            let center = corner + (u + w).normalize() * (d1 / half.cos());
            (center, q1 - center, q2 - center)
        });
    // Unequal sides: no circle touches both sides at q1 and q2, so the
    // rows go on the same arc stretched to fit (an ellipse tangent to both
    // sides at q1 and q2), at the stretched arc's equal angles. A point at
    // angle φ on a circular arc of half-angle α (from -α at q1 to α at q2)
    // is corner + λ1 (q1 - corner) + λ2 (q2 - corner) with the λs below,
    // whatever the side lengths, so unequal sides give the stretched arc.
    // It used to be a rational curve weighted cos(θ/2) for the corner's
    // angle θ (right only at 90°) with rows at even curve steps, so other
    // corners came out pointed or flat (the forum's oval corners,
    // 2026-11-27).
    let alpha = std::f64::consts::FRAC_PI_2 - half;
    (1..rows)
        .map(|k| {
            let t = k as f64 / rows as f64;
            if let Some((center, a0, a1)) = arc {
                let axis = a0.cross(a1).normalize_or_zero();
                toward(center + glam::DQuat::from_axis_angle(axis, a0.angle_between(a1) * t) * a0)
            } else if alpha > 1e-6 && alpha < std::f64::consts::FRAC_PI_2 - 1e-6 {
                let phi = alpha * (2. * t - 1.);
                let (sa, ca) = alpha.sin_cos();
                let sum = (1. - phi.cos() * ca) / (sa * sa);
                let diff = phi.sin() / sa;
                let (l1, l2) = ((sum - diff) / 2., (sum + diff) / 2.);
                toward(corner + (q1 - corner) * l1 + (q2 - corner) * l2)
            } else {
                // Sides in line (or folded flat): the old rational curve,
                // which keeps its rows where they always were.
                let weight = ((1. + u.dot(w)).max(0.) * 0.5).sqrt();
                let s = 1. - t;
                let chord = q1.lerp(q2, t);
                let curve = (q1 * s * s + corner * (2. * weight * s * t) + q2 * t * t)
                    / (s * s + 2. * weight * s * t + t * t);
                chord.lerp(curve, depth / 100.)
            }
        })
        .collect()
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    /// Unequal sides at any corner angle: the rows are the circular arc
    /// stretched to fit, so in the frame of the two sides they lie on the
    /// unit circle about the far corner of the parallelogram, at equal
    /// angles.
    #[test]
    fn unequal_sides_give_the_stretched_arc() {
        for deg in [60f64, 90., 120., 150.] {
            let th = deg.to_radians();
            let (u, w) = (DVec3::X, DVec3::new(th.cos(), th.sin(), 0.));
            let (a, b) = (0.3, 0.1);
            let rows = 6;
            let pts = profile(DVec3::ZERO, u * a, w * b, rows, 100., false);
            let alpha = std::f64::consts::FRAC_PI_2 - th / 2.;
            for (k, p) in pts.iter().enumerate() {
                // Coordinates along the two sides.
                let det = u.x * w.y - u.y * w.x;
                let l1 = (p.x * w.y - p.y * w.x) / det / a;
                let l2 = (u.x * p.y - u.y * p.x) / det / b;
                // The same coordinates of the circular arc's point.
                let phi = alpha * (2. * (k + 1) as f64 / rows as f64 - 1.);
                let (sa, ca) = alpha.sin_cos();
                let sum = (1. - phi.cos() * ca) / (sa * sa);
                let diff = phi.sin() / sa;
                assert!((l1 - (sum - diff) / 2.).abs() < 1e-12, "{deg}° row {k}");
                assert!((l2 - (sum + diff) / 2.).abs() < 1e-12, "{deg}° row {k}");
            }
            // At 90° that is an ellipse quarter about (a, b).
            if deg == 90. {
                for p in &pts {
                    let e = ((p.x - a) / a).powi(2) + ((p.y - b) / b).powi(2);
                    assert!((e - 1.).abs() < 1e-12, "{p}");
                }
            }
        }
    }

    #[test]
    fn equal_sides_lean_onto_the_point_between_them() {
        // Side points slid the same distance along two edges that leave the
        // old point at different angles to the selected edge: the corner is
        // the old point, as far from both.
        let v = DVec3::ZERO;
        let a = DVec3::new(0.0866, 0., 0.05).normalize() * 0.1;
        let b = DVec3::new(0., 0.1, 0.);
        let c = profile_corner(v, DVec3::Z, a, b, false);
        assert!(c.length() < 1e-12, "{c}");
        // Level but unequal: no such point, the corner stays level.
        let c = profile_corner(v, DVec3::Z, DVec3::X * 0.3, DVec3::Y * 0.1, false);
        assert!(c.length() < 1e-12, "{c}");
    }
}

/// One polygon corner at a point: the polygon and the points before and
/// after it.
#[derive(Clone, Copy)]
struct Corner {
    face: usize,
    prev: u32,
    next: u32,
}

/// A crossing this close to a profile point (as a share of the profile
/// segment) uses that point.
const CROSS_SNAP: f64 = 1e-6;

/// Where the edge from `a` to `b` crosses the polyline `line`: the segment
/// index, the share along that segment and the share `t` along the edge
/// (0 < t ≤ 1). The nearest crossing to `a` when there are several; `None`
/// when the edge misses it (they don't meet in space, or it ends first).
fn crossing(a: DVec3, b: DVec3, line: &[DVec3]) -> Option<(usize, f64, f64)> {
    let d1 = b - a;
    let scale = d1.length().max(1e-12);
    let mut best: Option<(usize, f64, f64)> = None;
    for (seg, w) in line.windows(2).enumerate() {
        let d2 = w[1] - w[0];
        let r = a - w[0];
        let (aa, bb, ee) = (d1.dot(d1), d1.dot(d2), d2.dot(d2));
        let (c, f) = (d1.dot(r), d2.dot(r));
        let denom = aa * ee - bb * bb;
        if denom <= 1e-18 * aa * ee {
            continue; // parallel
        }
        let t = (bb * f - c * ee) / denom;
        let u = (aa * f - bb * c) / denom;
        if t <= CROSS_SNAP || t > 1. + CROSS_SNAP || !(-CROSS_SNAP..=1. + CROSS_SNAP).contains(&u) {
            continue;
        }
        if (a + d1 * t).distance(w[0] + d2 * u) > 1e-7 * scale.max(d2.length()) {
            continue; // skew: they pass without meeting
        }
        if best.is_none_or(|(_, _, bt)| t < bt) {
            best = Some((seg, u.clamp(0., 1.), t.min(1.)));
        }
    }
    best
}

/// Going around a point, the polygons between one selected edge `a` and the
/// next selected edge `b`: `face_a` holds `a`, `face_b` holds `b`, and
/// `between` lists the unselected edges crossed on the way, in order.
struct Gap {
    face_a: usize,
    a: u32,
    between: Vec<u32>,
    face_b: usize,
    b: u32,
}

/// Every [`Gap`] around point `v` from its polygon corners (none across an
/// open border, where the walk can't go round).
fn gaps_around(v: u32, fan: &[Corner], is_sel: &dyn Fn(u32, u32) -> bool) -> Vec<Gap> {
    let mut gaps = Vec::new();
    for start in fan
        .iter()
        .filter(|c| is_sel(v, c.prev) && !is_sel(v, c.next))
    {
        let mut between = vec![start.next];
        let mut at = start.next;
        let mut end = None;
        for _ in 0..fan.len() {
            let Some(c) = fan.iter().find(|c| c.prev == at) else {
                break;
            };
            if is_sel(v, c.next) {
                end = Some((c.face, c.next));
                break;
            }
            between.push(c.next);
            at = c.next;
        }
        if let Some((face_b, b)) = end
            && b != start.prev
        {
            gaps.push(Gap {
                face_a: start.face,
                a: start.prev,
                between,
                face_b,
                b,
            });
        }
    }
    gaps
}

/// Whether `q` lies inside polygon `face` (in its plane, seen along
/// `normal`).
fn within(mesh: &Mesh, face: usize, q: DVec3, normal: DVec3) -> bool {
    let r = &mesh.faces[face].vertices;
    let u = normal.any_orthonormal_vector();
    let w = normal.cross(u);
    let flat = |x: DVec3| (x.dot(u), x.dot(w));
    let (qx, qy) = flat(q);
    let mut inside = false;
    for i in 0..r.len() {
        let (ax, ay) = flat(mesh.vertices[r[i] as usize]);
        let (bx, by) = flat(mesh.vertices[r[(i + 1) % r.len()] as usize]);
        if (ay > qy) != (by > qy) && qx < ax + (qy - ay) / (by - ay) * (bx - ax) {
            inside = !inside;
        }
    }
    inside
}

/// How far along `offset` from point `v` the polygon `face` reaches before
/// one of its edges away from `v` (1 = the whole offset).
fn ray_room(mesh: &Mesh, face: usize, v: u32, offset: DVec3, normal: DVec3) -> f64 {
    let r = &mesh.faces[face].vertices;
    let o = mesh.vertices[v as usize];
    let mut room = f64::INFINITY;
    for i in 0..r.len() {
        let (ia, ib) = (r[i], r[(i + 1) % r.len()]);
        if ia == v || ib == v {
            continue;
        }
        let (a, b) = (mesh.vertices[ia as usize], mesh.vertices[ib as usize]);
        // o + offset t = a + (b - a) s, in the plane.
        let e = b - a;
        let den = offset.cross(e).dot(normal);
        if den.abs() < 1e-15 {
            continue;
        }
        let t = (a - o).cross(e).dot(normal) / den;
        let s = (a - o).cross(offset).dot(normal) / den;
        if t > 0. && (0. ..=1.).contains(&s) {
            room = room.min(t);
        }
    }
    room
}

/// Closes small rounds on a strip's side (see the call). In each polygon,
/// a run of corner points (each between two selected edges) lies on the
/// side lines of the run's edges, each parallel to its edge. Where an edge's
/// two points have passed each other (its side runs backwards), the edge
/// drops out and the side lines on either side meet; repeated until no side
/// runs backwards. Runs where nothing turned back are left exactly as they
/// were.
/// A strip side's outline along a run of edges: `lines` are the edges' side
/// lines (a point, a direction), `raw` the points where consecutive lines
/// meet (`lines.len() - 1` of them). Where an edge's side runs backwards
/// (its two points passed each other), the edge drops out and its
/// neighbours' lines meet instead, until none does. `None` when nothing ran
/// backwards; else the edges left and the points between them.
fn close_run(
    lines: &[(DVec3, DVec3)],
    raw: &[DVec3],
    normal: DVec3,
) -> Option<(Vec<usize>, Vec<DVec3>)> {
    let meet = |i: usize, j: usize| -> DVec3 {
        let ((ai, ui), (aj, uj)) = (lines[i], lines[j]);
        let den = ui.cross(uj).dot(normal);
        if den.abs() < 1e-9 {
            return raw[i..j].iter().sum::<DVec3>() / (j - i) as f64;
        }
        ai + ui * ((aj - ai).cross(uj).dot(normal) / den)
    };
    let place =
        |active: &[usize]| -> Vec<DVec3> { active.windows(2).map(|w| meet(w[0], w[1])).collect() };
    let mut active: Vec<usize> = (0..lines.len()).collect();
    let mut changed = false;
    while active.len() > 2 {
        let q = place(&active);
        // The inner edge whose side runs most backwards.
        let worst = (1..active.len() - 1)
            .map(|i| (i, (q[i] - q[i - 1]).dot(lines[active[i]].1)))
            .filter(|&(_, d)| d < 0.)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((i, _)) = worst else {
            break;
        };
        active.remove(i);
        changed = true;
    }
    if !changed {
        return None;
    }
    let q = place(&active);
    Some((active, q))
}

fn collapse_rounds(
    mesh: &Mesh,
    is_sel: &impl Fn(u32, u32) -> bool,
    inside: &BTreeMap<(usize, u32), u32>,
    out: &mut Mesh,
    defs: &mut BTreeMap<u32, Def>,
) -> Vec<(Vec<u32>, DVec3)> {
    let mut closed = Vec::new();
    let p = |v: u32| mesh.vertices[v as usize];
    let faces: BTreeSet<usize> = inside.keys().map(|&(f, _)| f).collect();
    for fi in faces {
        let r = &mesh.faces[fi].vertices;
        let n = r.len();
        let node = |k: usize| inside.get(&(fi, r[k % n])).copied();
        if (0..n).all(|k| node(k).is_some()) {
            continue;
        }
        let normal = newell(mesh, fi);
        for k0 in (0..n).filter(|&k| node(k).is_some() && node(k + n - 1).is_none()) {
            let m = (0..n).take_while(|&j| node(k0 + j).is_some()).count();
            if m < 2 {
                continue;
            }
            let ids: Vec<u32> = (0..m).map(|j| node(k0 + j).unwrap()).collect();
            let raw: Vec<DVec3> = ids.iter().map(|&i| out.vertices[i as usize]).collect();
            // Edge j runs from face corner k0+j-1 to k0+j; node j sits
            // between edges j and j+1. Each edge's side line: its direction
            // through one of its nodes.
            let lines: Vec<(DVec3, DVec3)> = (0..=m)
                .map(|j| {
                    let (a, b) = (r[(k0 + n + j - 1) % n], r[(k0 + j) % n]);
                    let u = (p(b) - p(a)).normalize_or_zero();
                    (raw[j.saturating_sub(1).min(m - 1)], u)
                })
                .collect();
            if (1..m).any(|j| !is_sel(r[(k0 + j - 1) % n], r[(k0 + j) % n])) {
                continue;
            }
            let Some((active, q)) = close_run(&lines, &raw, normal) else {
                continue;
            };
            // Nearly parallel side lines meet far away: leave such a run.
            let reach = (0..m)
                .map(|j| raw[j].distance(p(r[(k0 + j) % n])))
                .fold(0., f64::max);
            let placed: Vec<(u32, DVec3)> = active
                .windows(2)
                .zip(&q)
                .flat_map(|(w, &at)| (w[0]..w[1]).map(move |j| (j, at)))
                .map(|(j, at)| (ids[j], at))
                .collect();
            if placed
                .iter()
                .any(|&(i, at)| at.distance(out.vertices[i as usize]) > 4. * reach)
            {
                continue;
            }
            for w in active.windows(2).zip(&q) {
                let ((a, b), &at) = ((w.0[0], w.0[1]), w.1);
                if b - a > 1 {
                    closed.push(((a..b).map(|j| ids[j]).collect(), at));
                }
            }
            for (id, at) in placed {
                out.vertices[id as usize] = at;
                if let Some(Def::Inside {
                    v, a, b, ta, tb, ..
                }) = defs.get_mut(&id)
                {
                    let (x, ea, eb) = (at - p(*v), p(*a) - p(*v), p(*b) - p(*v));
                    let (aa, ab, bb) = (ea.dot(ea), ea.dot(eb), eb.dot(eb));
                    let det = aa * bb - ab * ab;
                    if det.abs() > 1e-12 * aa * bb {
                        *ta = (x.dot(ea) * bb - x.dot(eb) * ab) / det;
                        *tb = (x.dot(eb) * aa - x.dot(ea) * ab) / det;
                    }
                }
            }
        }
    }
    closed
}

fn newell(mesh: &Mesh, face: usize) -> DVec3 {
    let r = &mesh.faces[face].vertices;
    let mut n = DVec3::ZERO;
    for i in 0..r.len() {
        let (a, b) = (
            mesh.vertices[r[i] as usize],
            mesh.vertices[r[(i + 1) % r.len()] as usize],
        );
        n += a.cross(b);
    }
    n.normalize_or_zero()
}

pub(crate) fn general(
    mesh: &Mesh,
    selected: &BTreeSet<(u32, u32)>,
    settings: &EdgeBevelSettings,
    weights: Option<&[f64]>,
    what: &str,
) -> Result<EdgeBevelResult> {
    general_with(mesh, selected, settings, weights, what, None)
}

/// `stuck`: None lets Edge Bevel with Loop Slide off first push points in
/// its way (see below); then it runs again on the moved mesh, with the
/// points that reached the end of their edge without making enough room.
fn general_with(
    mesh: &Mesh,
    selected: &BTreeSet<(u32, u32)>,
    settings: &EdgeBevelSettings,
    weights: Option<&[f64]>,
    what: &str,
    stuck: Option<&BTreeSet<u32>>,
) -> Result<EdgeBevelResult> {
    let make_room = stuck.is_none();
    let stuck = stuck.cloned().unwrap_or_default();
    let options = &settings.options;
    // EBC-B3-07: with Edge End Triangle Fan, Edge Chamfer slides an interior
    // end point along the edges beyond (as Edge Bevel does) and fans
    // triangles from it; with Profile it keeps the point.
    let fan_ends = settings.chamfer && options.edge_end == EdgeEnd::TriangleFan;
    let (width, depth, chamfer) = (settings.width, settings.depth, settings.chamfer);
    let parallel = chamfer && options.parallel;
    // EBC-P-02 / EBC-B3-08 / EBS-03: Square and Sharp keep one row at the
    // original edge whatever the Level or Segments, Level 0 / Segments 1
    // included (EBC-Q12 v1).
    let square = options.shape != EdgeBevelShape::Round;
    let rows = if square { 2 } else { settings.rows };
    // EBC-P-02: Edge Chamfer's Square closes each end with its own triangle
    // instead of putting the profile into the end polygon.
    let profile_in_ends = !(chamfer && square);
    // Only the selected edges' points are read below, so only their
    // corners are collected (a drag rebuilds this every frame; a whole-mesh
    // map was most of its cost on large meshes).
    let mut at_touched = vec![false; mesh.vertices.len()];
    for &(a, b) in selected {
        for v in [a, b] {
            *at_touched
                .get_mut(v as usize)
                .ok_or("Edge selection is out of date")? = true;
        }
    }
    // Directed edge between selected edges' points -> the polygon
    // traversing it.
    let mut face_of: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    let mut corners: BTreeMap<u32, Vec<Corner>> = BTreeMap::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        let n = f.vertices.len();
        if !f.vertices.iter().any(|&v| at_touched[v as usize]) {
            continue;
        }
        if mesh.is_polyline(fi) {
            return Err(format!("{what} doesn't work next to polylines yet"));
        }
        for i in (0..n).filter(|&i| at_touched[f.vertices[i] as usize]) {
            let (p, v, q) = (
                f.vertices[(i + n - 1) % n],
                f.vertices[i],
                f.vertices[(i + 1) % n],
            );
            if at_touched[q as usize] {
                face_of.insert((v, q), fi);
            }
            corners.entry(v).or_default().push(Corner {
                face: fi,
                prev: p,
                next: q,
            });
        }
    }
    for &(a, b) in selected {
        if !face_of.contains_key(&(a, b)) || !face_of.contains_key(&(b, a)) {
            return Err(format!("{what} supports edges between two polygons"));
        }
    }
    let touched: BTreeSet<u32> = selected.iter().flat_map(|&(a, b)| [a, b]).collect();
    let is_sel = |a: u32, b: u32| selected.contains(&edge_key(a, b));
    let p = |v: u32| mesh.vertices[v as usize];
    // EBC-B3-01: in Width mode the distance each side moves follows the
    // angle θ between the edge's two polygons: 2·Width·sin(θ/2) for Edge
    // Bevel (fits 60/90/120/135°), Width·sin(θ/2) for Edge Chamfer (fits
    // the 90° and 135° samples; the capped 60° sample is open, EBC-Q02).
    let mut factor: BTreeMap<(u32, u32), f64> = BTreeMap::new();
    for &(x, y) in selected {
        let f = if options.mode == EdgeBevelMode::Width {
            let axis = (p(y) - p(x)).normalize_or_zero();
            // Into each polygon, across the edge.
            let in1 = newell(mesh, face_of[&(x, y)]).cross(axis);
            let in2 = newell(mesh, face_of[&(y, x)]).cross(-axis);
            let half = in1.angle_between(in2) / 2.;
            if chamfer { half.sin() } else { 2. * half.sin() }
        } else {
            1.
        };
        factor.insert((x, y), f);
    }
    let factor_of = |v: u32, o: u32| factor[&edge_key(v, o)];
    let mut at_point: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
    for (&(a, b), &f) in &factor {
        for v in [a, b] {
            let e = at_point.entry(v).or_default();
            *e = (e.0 + f, e.1 + 1.);
        }
    }
    let mean_factor = |v: u32| at_point.get(&v).map_or(1., |&(sum, n)| sum / n);
    let weight = |v: u32| weights.map_or(1., |w| w[v as usize]);
    // The tester's rounded hole (2026-11-24): "if I'm bevelling, it's
    // because I want to move that stuff out of my way". A polygon with one
    // point off the strips (the sliver between a rounded hole corner and the
    // old square corner) must leave room for the strips' sides: seen from
    // that point, the sides must run around it in order. If they don't, the
    // point slides out along its own edge away from the strips, just as far
    // as needed (so it moves smoothly as the width grows), at most 90% of
    // that edge, and the bevel runs on the moved mesh, the edges into it
    // bending to the sides' even points (as with Loop Slide off).
    // The tester (2026-11-24): with Loop Slide off the bevel comes first and
    // the surrounding edges adapt to it from the start; with Loop Slide on
    // the bevel adapts to them instead (nothing is pushed; see the squeeze
    // below).
    if make_room && !chamfer && !options.loop_slide {
        let mut moved = mesh.clone();
        let mut moved_points: BTreeSet<u32> = BTreeSet::new();
        let mut stuck_points: BTreeSet<u32> = BTreeSet::new();
        // Where point `t` of polygon `fi` (points `r`) slides to get out of
        // the way, going `away`: along its edge leading most directly away
        // (to a point not in this polygon), or straight across the polygon
        // beyond.
        let path = |fi: usize, r: &[u32], t: u32, away: DVec3| -> Option<DVec3> {
            let far = mesh
                .faces
                .iter()
                .filter_map(|g| {
                    let i = g.vertices.iter().position(|&x| x == t)?;
                    let m = g.vertices.len();
                    Some([g.vertices[(i + 1) % m], g.vertices[(i + m - 1) % m]])
                })
                .flatten()
                .filter(|x| !r.contains(x) && !touched.contains(x))
                .max_by(|&x, &y| {
                    let k = |z: u32| (p(z) - p(t)).normalize_or_zero().dot(away);
                    k(x).total_cmp(&k(y))
                });
            let along = |x: u32| (p(x) - p(t)).normalize_or_zero().dot(away);
            let from = p(t);
            // The tester's BevelStuck scene (2026-11-25): a corner that sits
            // on a straight line of the surrounding polygons has no edge
            // leading away (its edges run along the line), so it slides
            // straight away across the polygon beyond instead, short of that
            // polygon's far edges.
            match far {
                Some(far) if along(far) >= 0.7 => Some(p(far)),
                _ => {
                    let across = mesh.faces.iter().enumerate().find_map(|(gi, g)| {
                        if gi == fi || !g.vertices.contains(&t) {
                            return None;
                        }
                        let gn = newell(mesh, gi);
                        let dir = (away - gn * away.dot(gn)).normalize_or_zero();
                        let room = ray_room(mesh, gi, t, dir, gn);
                        (room.is_finite() && within(mesh, gi, from + dir * (room * 1e-3), gn))
                            .then(|| from + dir * room)
                    });
                    match (across, far) {
                        (Some(q), _) => Some(q),
                        (None, Some(far)) if along(far) > 0. => Some(p(far)),
                        _ => None,
                    }
                }
            }
        };
        // The least slide toward `to` that fits, found by halving, at most
        // 90% of the way; and whether even that didn't fit.
        let slide = |from: DVec3, to: DVec3, fits: &dyn Fn(DVec3) -> bool| -> (DVec3, bool) {
            let (mut lo, mut hi) = (0., 0.9);
            if !fits(from.lerp(to, hi)) {
                return (from.lerp(to, hi), true);
            }
            for _ in 0..40 {
                let m = (lo + hi) * 0.5;
                if fits(from.lerp(to, m)) {
                    hi = m;
                } else {
                    lo = m;
                }
            }
            (from.lerp(to, hi), false)
        };
        for (fi, f) in mesh.faces.iter().enumerate() {
            let r = &f.vertices;
            let n = r.len();
            let off: Vec<usize> = (0..n).filter(|&k| !touched.contains(&r[k])).collect();
            let [ti] = off[..] else {
                continue;
            };
            let t = r[ti];
            // A corner of this polygon between two selected edges.
            if !(0..n).any(|k| is_sel(r[k], r[(k + n - 1) % n]) && is_sel(r[k], r[(k + 1) % n])) {
                continue;
            }
            let normal = newell(mesh, fi);
            // Where each other corner's side point will roughly be.
            let side_of = |k: usize| -> DVec3 {
                let (x, a, b) = (r[k], r[(k + n - 1) % n], r[(k + 1) % n]);
                let base = |o: u32| width * weight(x) * factor_of(x, o);
                // Inside each selected edge of this polygon at x.
                let into = |o: u32, after: bool| {
                    let e = if after { p(o) - p(x) } else { p(x) - p(o) };
                    normal.cross(e).normalize_or_zero()
                };
                let (sa, sb) = (is_sel(x, a), is_sel(x, b));
                let d = match (sa, sb) {
                    (true, true) => {
                        let (na, nb) = (into(a, false), into(b, true));
                        let cos = na.dot(nb);
                        let w = (base(a) + base(b)) * 0.5;
                        if cos > -1. + 1e-9 {
                            (na + nb) * (w / (1. + cos))
                        } else {
                            DVec3::ZERO
                        }
                    }
                    (true, false) => into(a, false) * base(a),
                    (false, true) => into(b, true) * base(b),
                    _ => DVec3::ZERO,
                };
                p(x) + d * 1.1
            };
            let sides: Vec<DVec3> = (1..n).map(|k| side_of((ti + k) % n)).collect();
            // Every consecutive pair turns the polygon's way seen from `at`.
            // ... and keeps a little clear of the sides (half the width), so
            // the polygons around it don't turn to slivers.
            let clear = 0.5 * width * weight(t);
            let fits = |at: DVec3| {
                sides.windows(2).all(|w| {
                    let (a, b) = (w[0] - at, w[1] - at);
                    let e = w[1] - w[0];
                    let k = ((at - w[0]).dot(e) / e.length_squared().max(1e-300)).clamp(0., 1.);
                    a.cross(b).dot(normal) > 0.02 * a.length() * b.length()
                        && at.distance(w[0] + e * k) >= clear
                })
            };
            if fits(p(t)) {
                continue;
            }
            // Its edge leading most directly away from the polygon's sides.
            // (From the polygon's own points on the strips, which don't move
            // with the width: the sides may reach past the point.)
            let mid = (1..n).map(|k| p(r[(ti + k) % n])).sum::<DVec3>() / (n - 1) as f64;
            let away = (p(t) - mid).normalize_or_zero();
            let from = p(t);
            let Some(to) = path(fi, r, t, away) else {
                continue;
            };
            let (at, stuck) = slide(from, to, &fits);
            if stuck {
                stuck_points.insert(t);
            }
            // A point shared by two such polygons goes the farther.
            let now = moved.vertices[t as usize];
            if now.distance(from) < at.distance(from) {
                moved.vertices[t as usize] = at;
            }
            moved_points.insert(t);
        }
        // The tester's BevelStuck scene, wide (2026-11-25): "breaks when we
        // reach the vertices that are farther on the same plane". In any
        // polygon beside the strips, a point off them that the strip would
        // reach (closer to a selected edge of the polygon than the width,
        // plus half a width clear) slides away from that edge just as far as
        // needed. Only points on a flat stretch of surface: a corner of the
        // solid (where the polygons around it turn) stays, as sliding it
        // would change the shape.
        let seg = |q: DVec3, a: DVec3, b: DVec3| {
            let e = b - a;
            let k = ((q - a).dot(e) / e.length_squared().max(1e-300)).clamp(0., 1.);
            a + e * k
        };
        for (fi, f) in mesh.faces.iter().enumerate() {
            let r = &f.vertices;
            let n = r.len();
            let sel_at = |k: usize| is_sel(r[k % n], r[(k + 1) % n]);
            if !(0..n).any(sel_at) || (0..n).all(sel_at) {
                continue;
            }
            let normal = newell(mesh, fi);
            // Each run of selected edges in this polygon, and the strip's
            // side along it: the outline the bevel will take here (rounds
            // closed), from the run's first point's side to its last's.
            let mut outlines: Vec<(Vec<DVec3>, Vec<DVec3>)> = Vec::new();
            for k0 in (0..n).filter(|&k| sel_at(k) && !sel_at(k + n - 1)) {
                let m = (0..n).take_while(|&j| sel_at(k0 + j)).count();
                let lines: Vec<(DVec3, DVec3)> = (0..m)
                    .map(|j| {
                        let (a, b) = (r[(k0 + j) % n], r[(k0 + j + 1) % n]);
                        let u = (p(b) - p(a)).normalize_or_zero();
                        let reach = width * weight(a).max(weight(b)) * factor_of(a, b);
                        (p(a) + normal.cross(u) * reach, u)
                    })
                    .collect();
                let raw: Vec<DVec3> = lines
                    .windows(2)
                    .map(|w| {
                        let ((ai, ui), (aj, uj)) = (w[0], w[1]);
                        let den = ui.cross(uj).dot(normal);
                        if den.abs() < 1e-9 {
                            aj
                        } else {
                            ai + ui * ((aj - ai).cross(uj).dot(normal) / den)
                        }
                    })
                    .collect();
                let inner = match close_run(&lines, &raw, normal) {
                    Some((_, q)) => q,
                    None => raw,
                };
                let rim: Vec<DVec3> = (0..=m).map(|j| p(r[(k0 + j) % n])).collect();
                let last = lines[m - 1];
                let end = last.0 + last.1 * (p(r[(k0 + m) % n]) - p(r[(k0 + m - 1) % n])).length();
                let side: Vec<DVec3> = std::iter::once(lines[0].0)
                    .chain(inner)
                    .chain(std::iter::once(end))
                    .collect();
                outlines.push((rim, side));
            }
            let u = normal.any_orthonormal_vector();
            let w = normal.cross(u);
            let flat2 = |x: DVec3| (x.dot(u), x.dot(w));
            let off: Vec<u32> = r.iter().copied().filter(|x| !touched.contains(x)).collect();
            for &t in &off {
                let flat = mesh
                    .faces
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| g.vertices.contains(&t))
                    .all(|(gi, _)| newell(mesh, gi).dot(normal) > 0.9998);
                if !flat {
                    continue;
                }
                let clear = 0.5 * width * weight(t);
                // Outside each run's strip (between the run and its side
                // outline) and at least `clear` from the outline.
                let fits = |at: DVec3| {
                    outlines.iter().all(|(rim, side)| {
                        let ring: Vec<DVec3> =
                            rim.iter().chain(side.iter().rev()).copied().collect();
                        let (qx, qy) = flat2(at);
                        let mut inside = false;
                        for i in 0..ring.len() {
                            let (ax, ay) = flat2(ring[i]);
                            let (bx, by) = flat2(ring[(i + 1) % ring.len()]);
                            if (ay > qy) != (by > qy) && qx < ax + (qy - ay) / (by - ay) * (bx - ax)
                            {
                                inside = !inside;
                            }
                        }
                        !inside
                            && side
                                .windows(2)
                                .all(|s| at.distance(seg(at, s[0], s[1])) >= clear)
                    })
                };
                // From where the corner rule left it, if it moved.
                let from = moved.vertices[t as usize];
                if fits(from) {
                    continue;
                }
                let near = outlines
                    .iter()
                    .flat_map(|(rim, _)| rim.windows(2).map(|e| seg(from, e[0], e[1])))
                    .min_by(|x, y| x.distance(from).total_cmp(&y.distance(from)))
                    .unwrap_or(from);
                let away = from - near;
                let away = (away - normal * away.dot(normal)).normalize_or_zero();
                if away == DVec3::ZERO {
                    continue;
                }
                let Some(to) = path(fi, r, t, away) else {
                    continue;
                };
                if (to - from).dot(away) <= 0. {
                    continue;
                }
                let (at, stuck) = slide(from, to, &fits);
                if stuck {
                    stuck_points.insert(t);
                }
                moved.vertices[t as usize] = at;
                moved_points.insert(t);
            }
        }
        if !moved_points.is_empty() {
            return general_with(
                &moved,
                selected,
                settings,
                weights,
                what,
                Some(&stuck_points),
            );
        }
    }
    // The selected edge sharing a polygon corner with unselected (v, s).
    let partner = |v: u32, s: u32| -> Option<u32> {
        corners[&v].iter().find_map(|c| {
            if c.prev == s && is_sel(v, c.next) {
                Some(c.next)
            } else if c.next == s && is_sel(v, c.prev) {
                Some(c.prev)
            } else {
                None
            }
        })
    };
    // A side point that reaches the far end of its edge stops there (Stop
    // At Edge) or carries on along the line (provisional EBC-Q10). A stopped
    // point only makes sense when Auto Weld merges it with the point it
    // stopped on; otherwise the polygons fold, so it's refused (Jason's QA,
    // 2026-11-03).
    let past_end =
        || format!("{what} width reaches the far end of an edge; reduce it or tick Auto Weld");
    // Whether end point `v` may slide along unselected edge (v, s): the
    // polygons the slide runs over must face nearly the same way as the
    // strip's polygons at `v`. Provisional threshold (EBC-Q11, N05): within
    // 45° counts as one surface, so curved organic meshes bevel; the
    // reported refusal is a 90° box edge. (It used to need every polygon
    // around `v` within 20° of each other, which refused ends on ordinary
    // curved surfaces, Jason 2026-11-15.)
    let slide_ok = |v: u32, s: u32| {
        let normal = |c: &&Corner| newell(mesh, c.face);
        let strip: Vec<DVec3> = corners[&v]
            .iter()
            .filter(|c| is_sel(v, c.prev) || is_sel(v, c.next))
            .map(|c| normal(&c))
            .collect();
        let limit = 45f64.to_radians().cos();
        corners[&v]
            .iter()
            .filter(|c| c.prev == s || c.next == s)
            .map(|c| normal(&c))
            .all(|n| strip.iter().all(|m| n.dot(*m) > limit))
    };
    // N08 (Jason, 2026-11-16): an open strip's end point that can't slide
    // (a slide edge leads over a sharp fold) is kept instead of refusing:
    // the strip's two new edges close back onto it, as Edge Chamfer's kept
    // point does (EBC-B3-06). Decided for the whole point, so its two sides
    // agree.
    let keep_end = |v: u32| {
        corners[&v].iter().any(|c| {
            [c.next, c.prev]
                .into_iter()
                .any(|s| !is_sel(v, s) && partner(v, s).is_none() && !slide_ok(v, s))
        })
    };
    // Loop Slide off (Edge Bevel): where unselected (v, s) runs between two
    // selected edges, the point where the strips' two sides meet, each side
    // its strip's width from its edge in its own polygon (the midpoint of
    // the sides' closest approach when the polygons aren't flat together).
    // `None` when (v, s) doesn't run between two selected edges, or the
    // sides are parallel or meet far away; the point then slides as usual.
    let even_point = |v: u32, s: u32| -> Option<(DVec3, Def)> {
        // (Polygon, selected edge's far point, whether it comes after v.)
        let sides: Vec<(usize, u32, bool)> = corners[&v]
            .iter()
            .filter_map(|c| {
                if c.prev == s && is_sel(v, c.next) {
                    Some((c.face, c.next, true))
                } else if c.next == s && is_sel(v, c.prev) {
                    Some((c.face, c.prev, false))
                } else {
                    None
                }
            })
            .collect();
        let [(f1, o1, n1), (f2, o2, n2)] = sides[..] else {
            return None;
        };
        if o1 == o2 {
            return None;
        }
        let toward = p(s) - p(v);
        // Each side runs inside its polygon (left of the edge as the polygon
        // runs). Facing (v, s) went wrong when (v, s) lies in line with a
        // selected edge (the tester's rounded hole: the old corner's edge
        // continues the rim), and the side flipped into the hole.
        let side = |face: usize, o: u32, after: bool| -> Option<(DVec3, DVec3, f64)> {
            let u = (p(o) - p(v)).try_normalize()?;
            let w = newell(mesh, face).cross(u).try_normalize()?;
            let w = if after { w } else { -w };
            let d = width * weight(v) * factor_of(v, o);
            Some((p(v) + w * d, u, d))
        };
        let ((a1, u1, d1), (a2, u2, d2)) = (side(f1, o1, n1)?, side(f2, o2, n2)?);
        let b = u1.dot(u2);
        let denom = 1. - b * b;
        if denom < 1e-9 {
            return None;
        }
        let r = a1 - a2;
        let (c, f) = (u1.dot(r), u2.dot(r));
        let (t1, t2) = ((b * f - c) / denom, (f - b * c) / denom);
        let q = (a1 + u1 * t1 + a2 + u2 * t2) / 2.;
        if q.distance(p(v)) > 20. * d1.max(d2) {
            return None;
        }
        // With Loop Slide on the bevel adapts to the geometry: the point
        // stays at most half way to the far edges of its polygons (the
        // tester's BevelStuck scene, 2026-11-25: a rim point right next to
        // the inset's straight line put it past that line, and the big
        // polygon around the hole crossed itself and covered it). It
        // shrinks smoothly as the width grows past that. (With it off the
        // geometry in the way was pushed instead.)
        let x = q - p(v);
        let hit = if options.loop_slide {
            [f1, f2]
                .into_iter()
                .map(|f| ray_room(mesh, f, v, x, newell(mesh, f)))
                .fold(f64::INFINITY, f64::min)
        } else {
            f64::INFINITY
        };
        let x = if hit.is_finite() {
            x * (0.5 * hit).min(1.)
        } else {
            x
        };
        let q = p(v) + x;
        // The point in a polygon's frame (for UVs): v + ta·(o − v) + tb·(s − v),
        // in f1's, or f2's where (v, s) runs in line with f1's edge (the
        // tester's rounded hole), else along that edge alone.
        let frame = |f: usize, o: u32| -> Option<Def> {
            let (ea, eb) = (p(o) - p(v), toward);
            let (aa, ab, bb) = (ea.dot(ea), ea.dot(eb), eb.dot(eb));
            let det = aa * bb - ab * ab;
            if det.abs() <= 1e-12 * aa * bb {
                return None;
            }
            Some(Def::Inside {
                face: f,
                v,
                a: o,
                b: s,
                ta: (x.dot(ea) * bb - x.dot(eb) * ab) / det,
                tb: (x.dot(eb) * aa - x.dot(ea) * ab) / det,
            })
        };
        let def = frame(f1, o1).or_else(|| frame(f2, o2)).unwrap_or_else(|| {
            let ea = p(o1) - p(v);
            Def::Inside {
                face: f1,
                v,
                a: o1,
                b: s,
                ta: x.dot(ea) / ea.length_squared().max(1e-300),
                tb: 0.,
            }
        });
        Some((q, def))
    };
    let mut out = mesh.clone();
    let mut defs: BTreeMap<u32, Def> = BTreeMap::new();
    let push = |out: &mut Mesh, q: DVec3| -> Result<u32> {
        let i = u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
        out.vertices.push(q);
        Ok(i)
    };
    // Points on unselected edges, and at corners between two selected edges.
    let mut on_edge: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let mut inside: BTreeMap<(usize, u32), u32> = BTreeMap::new();
    // Touched points Edge Chamfer keeps, on edges away from the strips.
    let mut kept: BTreeSet<u32> = BTreeSet::new();
    // Points slid along edges away from the strips (fan apexes).
    let mut beyond: BTreeSet<u32> = BTreeSet::new();
    // Edge Bevel ends whose extra edges (no strip side) can't slide: where
    // the end's profile crosses them, they end on it and the point goes
    // (tester's report, 2026-11-24); see `crossing` below.
    let mut across: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let strips_at = |v: u32| selected.iter().filter(|&&(a, b)| a == v || b == v).count();
    // Mitred meeting points (Edge Bevel; the tester's L-shape, 2026-11-24).
    // Going around a point, where two or more unselected edges lie between
    // two selected edges (a corner whose surrounding polygons are split),
    // the two strips' sides meet at one point: each side its strip's width
    // from its edge in its own outer polygon, where they cross. Every edge
    // between them ends there, as a real bevel's corner does. (One
    // unselected edge between them still slides along it, Loop Slide.)
    // Before, each such edge took its own slid point, so the two strips
    // never met and their corner was left as a hole closed by one polygon.
    // Corner Pattern None keeps the two slid points instead and rounds the
    // outline between them (below).
    if !chamfer && !options.corner.rounds_outline() {
        for &v in &touched {
            if strips_at(v) < 2 {
                continue;
            }
            for gap in gaps_around(v, &corners[&v], &is_sel) {
                let Gap {
                    face_a,
                    a,
                    between,
                    face_b,
                    b,
                } = gap;
                if between.len() < 2 {
                    continue;
                }
                // Where each strip's side would slide along the outer split
                // line, and the middle of the rounded outline between those
                // two points around the old point: the corner's top point.
                // (Corner Pattern None keeps that whole rounded outline; the
                // other patterns close it to this one point. The two sides'
                // crossing lay far behind the corner and pinched the corner
                // into a thin wedge; the tester's Maya pictures put the top
                // point just inside the old corner.)
                // The tester's enlarged Maya pictures (2026-11-24): the band
                // on top keeps its full width into the corner, and the split
                // lines end where the two strips' sides cross. Each side is
                // the strip's width from its edge, toward the polygons
                // between (`face_a` / `face_b` hold the strips' other sides).
                let _ = (face_a, face_b, depth);
                let side = |o: u32, s: u32| -> Option<DVec3> {
                    let e = (p(o) - p(v)).try_normalize()?;
                    let t = p(s) - p(v);
                    (t - e * t.dot(e)).try_normalize()
                };
                let (Some(na), Some(nb)) =
                    (side(a, between[0]), side(b, between[between.len() - 1]))
                else {
                    continue;
                };
                let c = na.dot(nb);
                if c.abs() > 0.95 {
                    continue;
                }
                // q·na = wa and q·nb = wb, with q in the plane of na, nb.
                let (wa, wb) = (
                    width * weight(v) * factor_of(v, a),
                    width * weight(v) * factor_of(v, b),
                );
                let (al, be) = ((wa - c * wb) / (1. - c * c), (wb - c * wa) / (1. - c * c));
                let q = p(v) + na * al + nb * be;
                // In the frame of the first polygon between (for UVs).
                let (s1, s2) = (between[0], between[1]);
                let Some(middle) = corners[&v]
                    .iter()
                    .find(|c| (c.prev == s1 && c.next == s2) || (c.prev == s2 && c.next == s1))
                    .map(|c| c.face)
                else {
                    continue;
                };
                let (ea, eb, x) = (p(s1) - p(v), p(s2) - p(v), q - p(v));
                let (aa, ab, b2) = (ea.dot(ea), ea.dot(eb), eb.dot(eb));
                let det = aa * b2 - ab * ab;
                if det.abs() < 1e-18 {
                    continue;
                }
                let id = push(&mut out, q)?;
                defs.insert(
                    id,
                    Def::Inside {
                        face: middle,
                        v,
                        a: s1,
                        b: s2,
                        ta: (x.dot(ea) * b2 - x.dot(eb) * ab) / det,
                        tb: (x.dot(eb) * aa - x.dot(ea) * ab) / det,
                    },
                );
                for &s in &between {
                    on_edge.insert((v, s), id);
                }
            }
        }
    }
    for &v in &touched {
        let fan = &corners[&v];
        for c in fan {
            for s in [c.next, c.prev] {
                if is_sel(v, s) || on_edge.contains_key(&(v, s)) {
                    continue;
                }
                let along = p(s) - p(v);
                let len = along.length();
                let dir = along / len;
                // Loop Slide off: an unbevelled edge between two bevelled
                // ones ends where the two strips' sides meet instead of
                // sliding along it, so the width is even and it bends.
                // With Loop Slide on too, an edge running within 15° of its
                // strip's edge (the tester's rounded hole: the rim's edge
                // carries on to the old corner) gives the strip no width to
                // slide along, so it takes the even point there.
                let in_line = || {
                    partner(v, s).is_some_and(|o| {
                        let axis = (p(o) - p(v)).normalize_or_zero();
                        dir.cross(axis).length() < 15f64.to_radians().sin()
                    })
                };
                if !chamfer
                    && (!options.loop_slide || in_line())
                    && let Some((q, def)) = even_point(v, s)
                {
                    let id = push(&mut out, q)?;
                    defs.insert(id, def);
                    on_edge.insert((v, s), id);
                    continue;
                }
                let partner = partner(v, s);
                let distance = match partner {
                    // Chamfer keeps the strip parallel to the selected edge
                    // in this polygon.
                    Some(o) if parallel => {
                        let axis = (p(o) - p(v)).normalize_or_zero();
                        width * weight(v) * factor_of(v, o) / dir.cross(axis).length().max(1e-9)
                    }
                    Some(o) => width * weight(v) * factor_of(v, o),
                    // An Edge Bevel strip's single end whose extra edges lie
                    // across its end profile (the cap side): decided once
                    // the profile exists.
                    None if !chamfer && keep_end(v) && strips_at(v) == 1 => {
                        across.entry(v).or_default().push(s);
                        continue;
                    }
                    // Sliding the end point along an edge away from the
                    // strips is only allowed where the polygons it slides
                    // over face the strip's way (the measured grid cases,
                    // UVN-T-01 / EBC-B3-06; EBC-Q11). At a sharp fold the
                    // point stays put instead (N08).
                    None if (chamfer && !fan_ends) || keep_end(v) => {
                        kept.insert(v);
                        continue;
                    }
                    None => width * weight(v) * mean_factor(v),
                };
                let distance = if options.stop_at_edge && distance >= len - 1e-9 {
                    if !options.auto_weld || touched.contains(&s) {
                        return Err(past_end());
                    }
                    len
                } else {
                    distance
                };
                let id = push(&mut out, p(v) + dir * distance)?;
                defs.insert(
                    id,
                    Def::Edge {
                        v,
                        s,
                        t: distance / len,
                    },
                );
                on_edge.insert((v, s), id);
                if partner.is_none() {
                    beyond.insert(id);
                }
            }
            if is_sel(v, c.prev) && is_sel(v, c.next) {
                let (a, b) = (p(c.prev) - p(v), p(c.next) - p(v));
                let (la, lb) = (a.length(), b.length());
                let (ua, ub) = (a / la, b / lb);
                let base = width * weight(v) * (factor_of(v, c.prev) + factor_of(v, c.next)) * 0.5;
                // Edge Bevel (the tester's report, 2026-11-24: a chain that
                // bends 45° twice narrowed and kinked): the point is where
                // the strip's two sides cross, a full width from both
                // edges, so the strip keeps its width through the bend
                // (the same as before on square corners). Sliding the
                // width along both edges left it ~70% wide at 135°. On
                // very sharp corners it stays within three widths.
                // The tester's rounded hole (2026-11-24): where the corner
                // bends the other way (wider than 180° inside the polygon,
                // as along a rounded rim seen from the top), the point goes
                // to the polygon's side of both edges, not across them.
                // Each side line is `base` from its edge, toward the
                // polygon's inside (left of the edge as the polygon runs).
                let normal = newell(mesh, c.face);
                let (na, nb) = (ua.cross(normal), normal.cross(ub));
                // EBC-B3-03 (measured; 2026-11-26 fix): with Loop Slide on,
                // a corner where the polygon bends less than 180° slides the
                // width along both edges, as measured, so the strip narrows
                // at the bend. Loop Slide off keeps the full width (above).
                // EBC-Q-RND1 (answered 2026-11-28): the sum is |ua + ub| =
                // 2·cos(φ/2) Insets along the bisector for the polygon's
                // corner angle φ, floored at one Inset, so corners wider
                // than 120° (a many-sided cap's rim) move exactly the Inset,
                // as the wall side does, and their profiles are round.
                let convex = (ua + ub).dot(na + nb) > 1e-12;
                let offset = if chamfer && !parallel {
                    (ua + ub) * base
                } else if !chamfer && options.loop_slide && convex {
                    let sum = ua + ub;
                    sum * (base * (1. / sum.length().max(1e-12)).max(1.))
                } else {
                    let cos = na.dot(nb);
                    let d = if cos > -1. + 1e-9 {
                        (na + nb) * (base / (1. + cos))
                    } else {
                        DVec3::ZERO
                    };
                    let most = 3. * base;
                    if !chamfer && d.length() > most {
                        d * (most / d.length())
                    } else {
                        d
                    }
                };
                // Where the polygon is too thin for the width (the sliver
                // between a rounded hole corner and the old square corner),
                // the point would land outside it and the polygon would
                // flip. It slides toward the polygon's points off the
                // strips instead, as Loop Slide does along an edge, at most
                // half way.
                // With Loop Slide off the bevel keeps its width (the
                // polygon's other point was pushed out of the way, and its
                // edges bend to even points), unless that point ran out of
                // edge.
                let keep = !options.loop_slide
                    && !mesh.faces[c.face]
                        .vertices
                        .iter()
                        .any(|x| stuck.contains(x));
                let offset = if chamfer || keep || within(mesh, c.face, p(v) + offset, normal) {
                    offset
                } else {
                    let off: Vec<DVec3> = mesh.faces[c.face]
                        .vertices
                        .iter()
                        .filter(|&&x| strips_at(x) == 0)
                        .map(|&x| p(x))
                        .collect();
                    if off.is_empty() {
                        offset * 0.5 * ray_room(mesh, c.face, v, offset, normal).min(1.)
                    } else {
                        let target = off.iter().sum::<DVec3>() / off.len() as f64;
                        let to = target - p(v);
                        to * (base / to.length().max(1e-12)).min(0.5)
                    }
                };
                // The offset along each edge (negative on a corner that
                // bends the other way; zero when the edges are in line).
                let k = ua.dot(ub);
                let (sa, sb) = if 1. - k * k > 1e-9 {
                    (
                        (offset.dot(ua) - offset.dot(ub) * k) / (1. - k * k),
                        (offset.dot(ub) - offset.dot(ua) * k) / (1. - k * k),
                    )
                } else {
                    (0., 0.)
                };
                // A corner point has no point to stop on. It passes an
                // edge's far end when it lies beyond it both along that edge
                // and in the edges' own frame: on a wide corner sa and sb
                // are much longer than the distance along the edge (the
                // tester's BevelStuck scene, 2026-11-25, refused a 158°
                // corner at a third of the edge's length), on a sharp one
                // shorter.
                // Where the edge's far end is such a corner point too, in
                // the same polygon, the two close together instead (see
                // `collapse_rounds`).
                let corner_too = |o: u32| {
                    corners[&o]
                        .iter()
                        .any(|d| d.face == c.face && is_sel(o, d.prev) && is_sel(o, d.next))
                };
                let past = |s: f64, u: DVec3, l: f64, o: u32| {
                    s.min(offset.dot(u)) >= l - 1e-9 && (chamfer || !corner_too(o))
                };
                if options.stop_at_edge && (past(sa, ua, la, c.prev) || past(sb, ub, lb, c.next)) {
                    return Err(past_end());
                }
                let id = push(&mut out, p(v) + offset)?;
                defs.insert(
                    id,
                    Def::Inside {
                        face: c.face,
                        v,
                        a: c.prev,
                        b: c.next,
                        ta: sa / la,
                        tb: sb / lb,
                    },
                );
                inside.insert((c.face, v), id);
            }
        }
    }
    // Inside a small round (the tester's BevelStuck scene, 2026-11-25:
    // rounds about 0.025 across on a hole's rim), a width past the round's
    // radius takes the strip's side past itself: neighbouring corner points
    // pass each other and the polygon crosses itself (it then drew over the
    // hole). As an offset outline does, the edges whose side has turned
    // back vanish and their neighbours' side lines meet instead: the round
    // closes to a point.
    // The closed rounds' points are welded into one at the end.
    let closed = if chamfer {
        Vec::new()
    } else {
        collapse_rounds(mesh, &is_sel, &inside, &mut out, &mut defs)
    };
    // The point a polygon corner at a touched point uses next to `edge_to`.
    let corner_point = |c: &Corner, v: u32, toward: u32| -> u32 {
        if is_sel(v, c.prev) && is_sel(v, c.next) {
            inside[&(c.face, v)]
        } else if is_sel(v, toward) {
            let other = if toward == c.next { c.prev } else { c.next };
            on_edge[&(v, other)]
        } else {
            on_edge[&(v, toward)]
        }
    };
    // Strip ends: at each end of a selected edge, the F1-side and F2-side
    // points with the profile rows between them, shared by pair.
    let mut row_cache: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    let mut ends: BTreeMap<(u32, u32, u32), Vec<u32>> = BTreeMap::new();
    // Corner Pattern None (the tester's Maya picture, 2026-11-24): where two
    // split lines run into a point between two strips, the outline between
    // their two slid points is rounded around the old point, one segment
    // per row, so the corner can be an even grid: the rows wrap across and
    // the third strip's lines run straight up to that rounded outline. The
    // polygon between the split lines takes the rounded points.
    // No Mitering: the strips ending on a rounded outline end straight.
    let mut arch_ends: BTreeSet<u32> = BTreeSet::new();
    if !chamfer && !square && options.corner.rounds_outline() && rows >= 2 {
        for &v in &touched {
            if strips_at(v) < 2 {
                continue;
            }
            for gap in gaps_around(v, &corners[&v], &is_sel) {
                if gap.between.len() != 2 {
                    continue;
                }
                let (Some(&a), Some(&b)) = (
                    on_edge.get(&(v, gap.between[0])),
                    on_edge.get(&(v, gap.between[1])),
                ) else {
                    continue;
                };
                if a == b || row_cache.contains_key(&(a, b)) || row_cache.contains_key(&(b, a)) {
                    continue;
                }
                let (pa, pb) = (out.vertices[a as usize], out.vertices[b as usize]);
                // A round arch centred on the old point, bulging away from
                // it (the tester's picture: Maya's None outline is a clean
                // arch; one rounded toward the old point bent the wrong way).
                let (ua, ub) = (pa - p(v), pb - p(v));
                let (ra, rb) = (ua.length(), ub.length());
                let (Some(da), Some(db)) = (ua.try_normalize(), ub.try_normalize()) else {
                    continue;
                };
                let angle = da.dot(db).clamp(-1., 1.).acos();
                if angle < 1e-6 || angle > std::f64::consts::PI - 1e-6 {
                    continue;
                }
                let arch: Vec<DVec3> = (1..rows)
                    .map(|k| {
                        let t = k as f64 / rows as f64;
                        let dir =
                            (da * ((1. - t) * angle).sin() + db * (t * angle).sin()) / angle.sin();
                        p(v) + dir * (ra + (rb - ra) * t)
                    })
                    .collect();
                let _ = depth;
                let mut ids = Vec::new();
                for (k, q) in arch.into_iter().enumerate() {
                    let id = push(&mut out, q)?;
                    defs.insert(
                        id,
                        Def::Row {
                            first: a,
                            last: b,
                            t: (k + 1) as f64 / rows as f64,
                        },
                    );
                    ids.push(id);
                }
                row_cache.insert((a, b), ids);
                arch_ends.extend([a, b]);
            }
        }
    }
    for &(x, y) in selected {
        let f1 = face_of[&(x, y)];
        let f2 = face_of[&(y, x)];
        for v in [x, y] {
            let other = if v == x { y } else { x };
            let c1 = corners[&v]
                .iter()
                .find(|c| c.face == f1)
                .copied()
                .ok_or("Missing corner")?;
            let c2 = corners[&v]
                .iter()
                .find(|c| c.face == f2)
                .copied()
                .ok_or("Missing corner")?;
            let (q1, q2) = (corner_point(&c1, v, other), corner_point(&c2, v, other));
            let rows_between = if let Some(r) = row_cache.get(&(q1, q2)) {
                r.clone()
            } else if let Some(r) = row_cache.get(&(q2, q1)) {
                r.iter().rev().copied().collect()
            } else {
                let mut ids = Vec::new();
                let (a, b) = (out.vertices[q1 as usize], out.vertices[q2 as usize]);
                let corner = profile_corner(p(v), p(other), a, b, !chamfer && options.loop_slide);
                // No Mitering (the tester's report, 2026-11-24: straight was
                // good but flat): a strip ending on a rounded outline keeps
                // its round, but in the upright plane through its two end
                // points, so it runs straight up into the corner. Its
                // corner is level with the outline point, straight above the
                // other end along the third strip's edge.
                let upright = (options.corner == CornerPattern::NoMitering)
                    .then(|| {
                        let (s, t) = if arch_ends.contains(&q1) {
                            (a, q2)
                        } else if arch_ends.contains(&q2) {
                            (b, q1)
                        } else {
                            return None;
                        };
                        let Some(&Def::Inside { a: ea, b: eb, .. }) = defs.get(&t) else {
                            return None;
                        };
                        let w = if ea == other { eb } else { ea };
                        let e = (p(w) - p(v)).try_normalize()?;
                        let tp = out.vertices[t as usize];
                        Some(tp + e * (s - tp).dot(e))
                    })
                    .flatten();
                let points = profile(upright.unwrap_or(corner), a, b, rows, depth, square);
                for (k, q) in points.into_iter().enumerate() {
                    let id = push(&mut out, q)?;
                    defs.insert(
                        id,
                        Def::Row {
                            first: q1,
                            last: q2,
                            t: (k + 1) as f64 / rows as f64,
                        },
                    );
                    ids.push(id);
                }
                row_cache.insert((q1, q2), ids.clone());
                ids
            };
            let mut run = vec![q1];
            run.extend(rows_between);
            run.push(q2);
            ends.insert((x, y, v), run);
        }
    }
    // Extra edges into a strip's end (tester's report, 2026-11-24: the tip
    // of a shape whose top was split into triangles). At an end where more
    // polygons meet than the strip's two sides and one end polygon, the
    // edges between them used to keep the old point (N08's kept point), so
    // those polygons still reached it: spikes. The end's profile runs
    // across them instead, so each such edge ends where it crosses the
    // profile, the polygons between share the profile, and the point goes.
    // If any of the point's extra edges misses the profile (too short, or
    // leaving the surface), the point is kept as before.
    let mut ext_runs: Vec<Vec<u32>> = Vec::new();
    for (&v, edges) in &across {
        let Some(run) = ends.iter().find(|(k, _)| k.2 == v).map(|(_, r)| r.clone()) else {
            kept.insert(v);
            continue;
        };
        let line: Vec<DVec3> = run.iter().map(|&id| out.vertices[id as usize]).collect();
        let hits: Option<Vec<(u32, usize, f64, f64)>> = edges
            .iter()
            .map(|&s| crossing(p(v), p(s), &line).map(|(seg, u, t)| (s, seg, u, t)))
            .collect();
        let Some(mut hits) = hits else {
            kept.insert(v);
            continue;
        };
        hits.sort_by(|a, b| (a.1, a.2).partial_cmp(&(b.1, b.2)).unwrap());
        let mut ext = vec![run[0]];
        for seg in 0..run.len() - 1 {
            for &(s, _, u, t) in hits.iter().filter(|h| h.1 == seg) {
                let id = if u <= CROSS_SNAP {
                    run[seg]
                } else if u >= 1. - CROSS_SNAP {
                    run[seg + 1]
                } else {
                    let id = push(&mut out, p(v) + (p(s) - p(v)) * t)?;
                    defs.insert(id, Def::Edge { v, s, t });
                    ext.push(id);
                    id
                };
                on_edge.insert((v, s), id);
            }
            ext.push(run[seg + 1]);
        }
        ext.dedup();
        ext_runs.push(ext);
    }
    // The points of an extended end profile strictly between `a` and `b`,
    // in that order (none when no extended profile holds both).
    let ext_between = |a: u32, b: u32| -> Vec<u32> {
        ext_runs
            .iter()
            .find_map(|run| {
                let ia = run.iter().position(|&x| x == a)?;
                let ib = run.iter().position(|&x| x == b)?;
                Some(if ia < ib {
                    run[ia + 1..ib].to_vec()
                } else {
                    run[ib + 1..ia].iter().rev().copied().collect()
                })
            })
            .unwrap_or_default()
    };
    // Profile between two points around a touched point, if they end a strip.
    let between = |a: u32, b: u32| -> Vec<u32> {
        if !profile_in_ends {
            return Vec::new();
        }
        row_cache
            .get(&(a, b))
            .cloned()
            .or_else(|| {
                row_cache
                    .get(&(b, a))
                    .map(|r| r.iter().rev().copied().collect())
            })
            .unwrap_or_else(|| ext_between(a, b))
    };
    // Rebuild every polygon around touched points. `from_corner` keeps, for
    // each rebuilt polygon, the source corner each new corner came from, so
    // corner normal maps follow.
    let mut from_corner: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        if !face.vertices.iter().any(|v| touched.contains(v)) {
            continue;
        }
        let mut cycle = Vec::with_capacity(n + 4);
        let sources = from_corner.entry(fi).or_default();
        for i in 0..n {
            let v = face.vertices[i];
            // The corners added since the last step came from corner i - 1.
            sources.resize(cycle.len(), i.saturating_sub(1));
            if !touched.contains(&v) {
                cycle.push(v);
                continue;
            }
            let c = Corner {
                face: fi,
                prev: face.vertices[(i + n - 1) % n],
                next: face.vertices[(i + 1) % n],
            };
            let (sp, sn) = (is_sel(v, c.prev), is_sel(v, c.next));
            match (sp, sn) {
                (true, true) => cycle.push(inside[&(fi, v)]),
                (true, false) => cycle.push(on_edge[&(v, c.next)]),
                (false, true) => cycle.push(on_edge[&(v, c.prev)]),
                (false, false) => match (on_edge.get(&(v, c.prev)), on_edge.get(&(v, c.next))) {
                    // Both edges end at one mitred meeting point.
                    (Some(&a), Some(&b)) if a == b => cycle.push(a),
                    (Some(&a), Some(&b)) => {
                        cycle.push(a);
                        cycle.extend(between(a, b));
                        cycle.push(b);
                    }
                    // Edge Chamfer keeps the point on edges away from the
                    // strips.
                    (Some(&a), None) => cycle.extend([a, v]),
                    (None, Some(&b)) => cycle.extend([v, b]),
                    (None, None) => cycle.push(v),
                },
            }
        }
        sources.resize(cycle.len(), n - 1);
        out.faces[fi].vertices = cycle;
    }
    // Strips, rows from the F1 side (first) to the F2 side.
    let mut new_faces: Vec<(Vec<u32>, Vec<usize>)> = Vec::new();
    let mut strip_edges = BTreeSet::new();
    let mut through_corner = BTreeSet::new();
    // Per strip polygon (the first new polygons): its edge and two sides.
    let mut strips: Vec<(u32, u32, usize, usize)> = Vec::new();
    for &(x, y) in selected {
        let (f1, f2) = (face_of[&(x, y)], face_of[&(y, x)]);
        let (ra, rb) = (&ends[&(x, y, x)], &ends[&(x, y, y)]);
        for k in 0..rows as usize {
            // A row's end edge takes the points where extra edges cross the
            // profile there, so the end polygons meet it without gaps.
            let mut quad = vec![rb[k], ra[k]];
            quad.extend(ext_between(ra[k], ra[k + 1]));
            quad.extend([ra[k + 1], rb[k + 1]]);
            quad.extend(ext_between(rb[k + 1], rb[k]));
            for i in 0..quad.len() {
                strip_edges.insert(edge_key(quad[i], quad[(i + 1) % quad.len()]));
            }
            new_faces.push((quad, vec![f1, f2]));
            strips.push((x, y, f1, f2));
        }
        if square {
            through_corner.insert(edge_key(ra[1], rb[1]));
        }
    }
    // Close what is left open around each touched point: half-edges between
    // its new points used once, reversed and chained.
    let mut local: BTreeMap<u32, u32> = BTreeMap::new();
    for (&(v, _), &id) in &on_edge {
        local.insert(id, v);
    }
    for (&(_, v), &id) in &inside {
        local.insert(id, v);
    }
    for (&(a, _), ids) in &row_cache {
        let v = local[&a];
        for &id in ids {
            local.insert(id, v);
        }
    }
    for &v in &kept {
        local.insert(v, v);
    }
    // Only half-edges between new points are read below, so only those
    // are counted (a whole-mesh map here was most of a drag frame).
    let mut used: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for f in out
        .faces
        .iter()
        .map(|f| &f.vertices)
        .chain(new_faces.iter().map(|(v, _)| v))
    {
        for i in 0..f.len() {
            let (a, b) = (f[i], f[(i + 1) % f.len()]);
            if local.contains_key(&a) && local.contains_key(&b) {
                *used.entry((a, b)).or_default() += 1;
            }
        }
    }
    // EBC-B3-06: a point on an open border keeps its border open.
    let on_border = |v: u32| {
        let mut count: BTreeMap<u32, u32> = BTreeMap::new();
        for c in &corners[&v] {
            *count.entry(c.prev).or_default() += 1;
            *count.entry(c.next).or_default() += 1;
        }
        count.values().any(|&n| n == 1)
    };
    let is_row =
        |defs: &BTreeMap<u32, Def>, id: &u32| matches!(defs.get(id), Some(Def::Row { .. }));
    for &v in &touched {
        let mut next: BTreeMap<u32, u32> = BTreeMap::new();
        for (&(a, b), &count) in &used {
            if local.get(&a) == Some(&v)
                && local.get(&b) == Some(&v)
                && count == 1
                && !used.contains_key(&(b, a))
            {
                // The opening runs against the existing polygons.
                if next.insert(b, a).is_some() {
                    return Err(format!("{what} can't close the opening at a point here"));
                }
            }
        }
        let border = on_border(v);
        'rings: while let Some((&start, _)) = next.iter().next() {
            let mut ring = vec![start];
            let mut at = next.remove(&start).unwrap_or(start);
            while at != start {
                ring.push(at);
                match next.remove(&at) {
                    Some(n) => at = n,
                    None if border => continue 'rings,
                    None => return Err(format!("{what} left an open gap at a point")),
                }
            }
            let homes: Vec<usize> = corners[&v].iter().map(|c| c.face).collect();
            let n = rows as usize;
            // EBC-J-05 / EBC-B3-04: where strips with two or more rows meet,
            // a patch of quads. The opening runs strip corner, profile rows,
            // strip corner… An unselected edge between two strips (a split
            // line of the polygons around the point, the tester's L-shape,
            // 2026-11-24) adds a second corner point there; such a run of
            // corner points is laid out as one corner (their mean) and its
            // points go back into the polygon that takes that corner.
            // Before, such an opening was closed by one polygon whatever
            // the Corner Pattern.
            let layout = if n >= 2 && (!square || n == 2) {
                corner_layout(&ring, n, |id| is_row(&defs, id))
            } else {
                None
            };
            if let Some((ring, runs)) = layout {
                // One stand-in point per run of corner points.
                let mut layout_ring = ring.clone();
                let mut stand_ins: Vec<(u32, Vec<u32>)> = Vec::new();
                for (at, run) in runs.iter().rev() {
                    let w = 1. / run.len() as f64;
                    let comb: Comb = run.iter().map(|&id| (id, w)).collect();
                    let position = run
                        .iter()
                        .map(|&id| out.vertices[id as usize] * w)
                        .sum::<DVec3>();
                    let id = push(&mut out, position)?;
                    defs.insert(id, Def::Blend(comb));
                    layout_ring.splice(*at..*at + run.len(), [id]);
                    stand_ins.push((id, run.clone()));
                }
                let first_new = new_faces.len();
                // Corner Pattern (EBC-Q-CP1): Edge Bevel's other fills for a
                // Round corner; Patch is the measured one below.
                let three = layout_ring.len() == 3 * n && stand_ins.is_empty();
                let four = layout_ring.len() == 4 * n && stand_ins.is_empty();
                if !chamfer && !square && options.corner.rounds_outline() && four {
                    // None with its rounded outline: an even n × n grid.
                    corner_grid(&layout_ring, n, &mut out, &mut defs, &mut new_faces, &homes)?;
                } else if !chamfer
                    && !square
                    && three
                    && matches!(
                        options.corner,
                        CornerPattern::Radial
                            | CornerPattern::Uniform
                            | CornerPattern::Pinch
                            | CornerPattern::Patch
                    )
                    && (options.corner != CornerPattern::Patch || stand_ins.is_empty())
                {
                    // The pole: the strip corner across from the one strip
                    // whose edge turns the other way (the L's inner
                    // vertical edge among two outer top edges), else the
                    // first.
                    let convex: Vec<Option<bool>> = (0..3)
                        .map(|t| {
                            let (e0, e1) =
                                (layout_ring[t * n], layout_ring[((t + 1) * n) % (3 * n)]);
                            ends.iter()
                                .find(|(key, run)| {
                                    key.2 == v
                                        && ((run[0] == e0 && run[run.len() - 1] == e1)
                                            || (run[0] == e1 && run[run.len() - 1] == e0))
                                })
                                .map(|(&(x, y, _), _)| {
                                    let f2 = &mesh.faces[face_of[&(y, x)]].vertices;
                                    let c2 =
                                        f2.iter().map(|&i| p(i)).sum::<DVec3>() / f2.len() as f64;
                                    (c2 - p(x)).dot(newell(mesh, face_of[&(x, y)])) < 0.
                                })
                        })
                        .collect();
                    let odd = (0..3).find(|&t| {
                        convex[t].is_some()
                            && (0..3)
                                .filter(|&u| u != t)
                                .all(|u| convex[u].is_some() && convex[u] != convex[t])
                    });
                    let pole = odd.map_or(0, |t| (t + 2) % 3);
                    if options.corner == CornerPattern::Patch {
                        // Maya's Patch on an inner corner: the same wide
                        // sides as Radial, filled with the measured grid.
                        if odd.is_some()
                            && n.is_multiple_of(2)
                            && widen_sides(&layout_ring, n, pole, &mut out, |_| 1.)
                        {
                            corner_patch_wrapped(
                                &layout_ring,
                                n,
                                pole,
                                &mut out,
                                &mut defs,
                                &mut new_faces,
                                &homes,
                            )?;
                        } else {
                            corner_patch(
                                &layout_ring,
                                n,
                                None,
                                &mut out,
                                &mut defs,
                                &mut new_faces,
                                &homes,
                            )?;
                        }
                    } else {
                        corner_fill(
                            &layout_ring,
                            n,
                            pole,
                            odd.is_some(),
                            options.corner,
                            &mut out,
                            &mut defs,
                            &mut new_faces,
                            &homes,
                        )?;
                    }
                } else {
                    // Square keeps the corner sharp: the patch meets at the
                    // original point (Depth ignored, EBS-03).
                    let center = square.then_some((v, 1.));
                    corner_patch(
                        &layout_ring,
                        n,
                        center,
                        &mut out,
                        &mut defs,
                        &mut new_faces,
                        &homes,
                    )?;
                    // Each stand-in's polygon takes its run's points instead,
                    // in the opening's order.
                    for (id, run) in &stand_ins {
                        for (face, _) in new_faces[first_new..].iter_mut() {
                            let Some(i) = face.iter().position(|x| x == id) else {
                                continue;
                            };
                            let before = face[(i + face.len() - 1) % face.len()];
                            let ri = ring.iter().position(|x| *x == run[0]).unwrap_or(0);
                            let ring_before = ring[(ri + ring.len() - 1) % ring.len()];
                            let mut ids = run.clone();
                            if before != ring_before {
                                ids.reverse();
                            }
                            face.splice(i..=i, ids);
                        }
                    }
                }
                continue;
            }
            if let Some(apex) = ring.iter().position(|id| beyond.contains(id))
                && fan_ends
                && ring.len() > 3
            {
                ring.rotate_left(apex);
                for i in 1..ring.len() - 1 {
                    new_faces.push((vec![ring[0], ring[i], ring[i + 1]], homes.clone()));
                }
                continue;
            }
            if ring.len() >= 3 && !flat_opening(&out, &ring, &mut new_faces) {
                new_faces.push((ring, homes));
            }
        }
    }
    // Append the new polygons, copying attributes from their first home.
    for (vertices, homes) in &new_faces {
        let template = homes[0];
        let mut face = Face {
            vertices: vertices.clone(),
            uv: None,
            ..mesh.faces[template].clone()
        };
        if options.use_material {
            face.material = options.material.clone();
            face.material_label = Some(options.material.clone());
        }
        out.faces.push(face);
        if !mesh.attributes.smoothing_labels.is_empty() {
            out.attributes
                .smoothing_labels
                .push(mesh.attributes.smoothing_labels[template].clone());
        }
        if !mesh.attributes.representations.is_empty() {
            out.attributes
                .representations
                .push(mesh.attributes.representations[template]);
        }
    }
    let homes: Vec<Vec<usize>> = new_faces.into_iter().map(|(_, h)| h).collect();
    // The seam rules hold on flat surroundings, as measured (`corner_uv::flat`).
    let one_frame: Vec<bool> = strips
        .iter()
        .map(|&(_, _, f1, f2)| !chamfer && crate::corner_uv::flat(mesh, [f1, f2]))
        .collect();
    let flat_at = |v: u32| {
        corners
            .get(&v)
            .is_some_and(|fan| crate::corner_uv::flat(mesh, fan.iter().map(|c| c.face)))
    };
    for (name, map) in out.uv_maps.iter_mut() {
        let rows_in = &mesh.uv_maps[name];
        let points = mesh.attributes.point_uvs.get(name);
        *map = out
            .faces
            .iter()
            .enumerate()
            .map(|(fi, face)| {
                face.vertices
                    .iter()
                    .map(|&id| match fi.checked_sub(mesh.faces.len()) {
                        // UVN-T-01 / UVN-B2-02: Edge Bevel's strip takes one
                        // side's frame on all its corners, its F2 side (the
                        // measured right neighbour). Edge Chamfer keeps the
                        // per-corner rule (its seam case is open).
                        Some(new) if new < strips.len() && one_frame[new] => {
                            let (x, y, f1, f2) = strips[new];
                            strip_uv(mesh, rows_in, &defs, (x, y), f1, f2, id)
                                .or_else(|| uv_on(mesh, rows_in, &defs, &homes[new], id))
                        }
                        Some(new) if new >= strips.len() && !chamfer => {
                            end_uv(mesh, rows_in, points, &defs, &corners, &flat_at, id)
                                .unwrap_or_else(|| uv_on(mesh, rows_in, &defs, &homes[new], id))
                        }
                        Some(new) => uv_on(mesh, rows_in, &defs, &homes[new], id),
                        None => uv_in(mesh, rows_in, &defs, fi, id),
                    })
                    .collect()
            })
            .collect();
    }
    let shape: Vec<usize> = out.faces.iter().map(|f| f.vertices.len()).collect();
    let o = &mut out.attributes;
    for (name, map) in o.uv_records.iter_mut() {
        *map = records_from_uvs(&out.faces, &out.uv_maps[name]);
    }
    for map in o.typed_maps.values_mut() {
        if !map.corners.is_empty() {
            map.corners = shape.iter().map(|&n| vec![None; n]).collect();
        }
    }
    // Vertex normal maps: a rebuilt polygon's corners keep the normal of the
    // corner they came from; the new polygons' corners stay unassigned, as
    // Add Loop leaves its new corners (provisional, EBC-Q13).
    for (name, rows) in o.corner_normals.iter_mut() {
        let source = &mesh.attributes.corner_normals[name];
        *rows = shape
            .iter()
            .enumerate()
            .map(|(fi, &n)| match (source.get(fi), from_corner.get(&fi)) {
                (Some(old), Some(from)) => from.iter().map(|&c| old[c]).collect(),
                (Some(old), None) => old.clone(),
                (None, _) => vec![None; n],
            })
            .collect();
    }
    if !out.attributes.creases.is_empty() {
        let edges = out.edges();
        out.attributes
            .creases
            .retain(|c| edges.binary_search(&edge_key(c.a, c.b)).is_ok());
    }
    let invalid = |e: String| format!("{what} produced an invalid mesh ({e}); nothing changed");
    let renumber = |edges: &BTreeSet<(u32, u32)>, map: &dyn Fn(u32) -> Option<u32>| {
        edges
            .iter()
            .filter_map(|&(x, y)| Some((map(x)?, map(y)?)))
            .filter(|&(x, y)| x != y)
            .map(|(x, y)| edge_key(x, y))
            .collect::<BTreeSet<_>>()
    };
    let mut touched = touched;
    let mut groups = closed;
    if options.auto_weld {
        // New points that landed on another point (Stop At Edge, coincident
        // ends) merge with it; the lower index survives. Polygons that
        // collapse onto merged points are removed by the weld, so the mesh
        // is checked after it.
        let near: Vec<u32> = on_edge
            .keys()
            .map(|&(_, s)| s)
            .filter(|s| !touched.contains(s) || kept.contains(s))
            .chain(mesh.vertices.len() as u32..out.vertices.len() as u32)
            .collect::<BTreeSet<u32>>()
            .into_iter()
            .collect();
        let tolerance = 1e-9 * width.max(1.);
        // Sorted by x, so each point is only compared with its neighbours
        // along x (a drag can make thousands of new points).
        let mut near = near;
        let at = |v: u32| out.vertices[v as usize];
        near.sort_by(|&a, &b| at(a).x.total_cmp(&at(b).x));
        let mut grouped: BTreeSet<u32> = groups.iter().flat_map(|g| g.0.clone()).collect();
        for (i, &a) in near.iter().enumerate() {
            if grouped.contains(&a) {
                continue;
            }
            let pa = at(a);
            let mut members = vec![a];
            for &b in &near[i + 1..] {
                if at(b).x - pa.x > tolerance {
                    break;
                }
                if !grouped.contains(&b) && at(b).distance(pa) <= tolerance {
                    members.push(b);
                }
            }
            if members.len() > 1 {
                members.sort_unstable();
                grouped.extend(members.iter().copied());
                let position = at(members[0]);
                groups.push((members, position));
            }
        }
    }
    if !groups.is_empty() {
        // The weld keeps unused points, so the touched ones are
        // removed below under their new numbers.
        let (welded, remap) = crate::edge_commands::weld(&out, &groups, what)
            .map_err(|e| format!("{what} with Auto Weld: {e}"))?;
        let to_welded = |x: u32| remap[x as usize];
        strip_edges = renumber(&strip_edges, &to_welded);
        through_corner = renumber(&through_corner, &to_welded);
        touched = touched.iter().filter_map(|&v| remap[v as usize]).collect();
        out = welded;
    }
    out.validate().map_err(invalid)?;
    let cleaned = crate::mesh_cleanup::remove_unreferenced(&out, Some(&touched))?;
    let mut new_of_old = vec![None; out.vertices.len()];
    for (new, &old) in cleaned.survivors.iter().enumerate() {
        new_of_old[old] = Some(new as u32);
    }
    let to_cleaned = |x: u32| new_of_old[x as usize];
    let strip_edges = renumber(&strip_edges, &to_cleaned);
    let through_corner = renumber(&through_corner, &to_cleaned);
    let mesh_out = cleaned.mesh;
    let strip_edges: Vec<(u32, u32)> = strip_edges.into_iter().collect();
    let selected_after = if !chamfer && rows == 1 {
        // EBC-B3-03/06 (EBC-O-E3-03, EBC-O-E3-06): a one-row Edge Bevel
        // selects every edge between two new points: the strips, the
        // polygons closing their ends, and an end polygon's side that now
        // runs between two new points. With more rows only the strips'
        // outer sides are measured (EBC-O-E3-02/04), so those keep the
        // strip edges.
        let first_new = mesh.vertices.len();
        let is_new: Vec<bool> = cleaned
            .survivors
            .iter()
            .map(|&old| old >= first_new)
            .collect();
        // One pass over the corners (no whole-mesh edge map per drag frame).
        let mut between_new = BTreeSet::new();
        for f in &mesh_out.faces {
            let n = f.vertices.len();
            for i in 0..n {
                let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
                if is_new[a as usize] && is_new[b as usize] {
                    between_new.insert(edge_key(a, b));
                }
            }
        }
        between_new.into_iter().collect()
    } else if !chamfer {
        strip_edges.clone()
    } else {
        through_corner.into_iter().collect()
    };
    Ok(EdgeBevelResult {
        mesh: mesh_out,
        strip_edges,
        selected_after,
    })
}

/// An opening with no area (Edge Chamfer's kept interior point, EBC-B3-06
/// and B3-07) can't be a polygon: its points join the new polygon across its
/// longest side instead. The reference keeps zero-area polygons there; this
/// is a deliberate departure, since Operandi refuses degenerate polygons.
fn flat_opening(out: &Mesh, ring: &[u32], new_faces: &mut [(Vec<u32>, Vec<usize>)]) -> bool {
    let n = ring.len();
    let q = |i: usize| out.vertices[ring[i % n] as usize];
    let mut normal = DVec3::ZERO;
    let mut perimeter = 0.;
    for i in 0..n {
        normal += q(i).cross(q(i + 1));
        perimeter += q(i).distance(q(i + 1));
    }
    if normal.length() > 1e-9 * perimeter * perimeter {
        return false;
    }
    let long = (0..n)
        .max_by(|&i, &j| q(i).distance(q(i + 1)).total_cmp(&q(j).distance(q(j + 1))))
        .unwrap_or(0);
    let (x, y) = (ring[long], ring[(long + 1) % n]);
    for (face, _) in new_faces.iter_mut() {
        let m = face.len();
        if let Some(i) = (0..m).find(|&i| face[i] == y && face[(i + 1) % m] == x) {
            // y → (the rest of the ring) → x.
            let rest: Vec<u32> = (2..n).map(|k| ring[(long + k) % n]).collect();
            for (k, id) in rest.into_iter().enumerate() {
                face.insert(i + 1 + k, id);
            }
            return true;
        }
    }
    false
}

/// The opening `ring` laid out as strip corners each followed by `n − 1`
/// profile rows, at least three strips: the ring turned to start at a
/// corner point, and each run of two or more corner points in a row (an
/// unselected edge between two strips adds one) with its index in that
/// turned ring. `None` for any other opening.
type Layout = (Vec<u32>, Vec<(usize, Vec<u32>)>);
fn corner_layout(ring: &[u32], n: usize, is_row: impl Fn(&u32) -> bool) -> Option<Layout> {
    let len = ring.len();
    // Start at a corner point that follows a row.
    let start = (0..len).find(|&i| !is_row(&ring[i]) && is_row(&ring[(i + len - 1) % len]))?;
    let mut turned = ring.to_vec();
    turned.rotate_left(start);
    let mut runs = Vec::new();
    let mut corners = 0;
    let mut i = 0;
    while i < len {
        let run: Vec<u32> = turned[i..]
            .iter()
            .take_while(|id| !is_row(id))
            .copied()
            .collect();
        if run.is_empty() {
            return None;
        }
        if run.len() > 1 {
            runs.push((i, run.clone()));
        }
        i += run.len();
        let rows = turned[i..].iter().take_while(|id| is_row(id)).count();
        if rows != n - 1 {
            return None;
        }
        i += rows;
        corners += 1;
    }
    (corners >= 3).then_some((turned, runs))
}

/// Corner Pattern None (the tester's Maya picture, 2026-11-24): fill a
/// four-sided opening (four strip corners, each side `n` segments: the
/// rounded outline and three strip ends) with an even `n` × `n` grid of
/// quads, a Coons patch of its four sides. Rows run from one side to the
/// opposite one, so a strip's rows wrap across the corner into the strip
/// opposite, and the lines between run straight up to the outline.
// The grid is indexed by row and line together.
#[allow(clippy::needless_range_loop)]
fn corner_grid(
    ring: &[u32],
    n: usize,
    out: &mut Mesh,
    defs: &mut BTreeMap<u32, Def>,
    new_faces: &mut Vec<(Vec<u32>, Vec<usize>)>,
    homes: &[usize],
) -> Result<()> {
    let len = ring.len();
    let at = |c: usize, t: usize| ring[(c * n + t) % len];
    // Sides in the opening's order: top 0 → 1, right 1 → 2, bottom 2 → 3,
    // left 3 → 0. Grid (i, j): i from the top side down, j left to right.
    let top = |j: usize| at(0, j);
    let right = |i: usize| at(1, i);
    let bottom = |j: usize| at(2, n - j);
    let left = |i: usize| at(3, n - i);
    let (c00, c0n, cn0, cnn) = (at(0, 0), at(1, 0), at(3, 0), at(2, 0));
    let mut grid = vec![vec![u32::MAX; n + 1]; n + 1];
    for i in 0..=n {
        for j in 0..=n {
            grid[i][j] = if i == 0 {
                top(j)
            } else if i == n {
                bottom(j)
            } else if j == 0 {
                left(i)
            } else if j == n {
                right(i)
            } else {
                let (s, t) = (j as f64 / n as f64, i as f64 / n as f64);
                let parts = [
                    (top(j), 1. - t),
                    (bottom(j), t),
                    (left(i), 1. - s),
                    (right(i), s),
                    (c00, -(1. - s) * (1. - t)),
                    (c0n, -s * (1. - t)),
                    (cn0, -(1. - s) * t),
                    (cnn, -s * t),
                ];
                let mut sum: BTreeMap<u32, f64> = BTreeMap::new();
                for (id, w) in parts {
                    *sum.entry(id).or_default() += w;
                }
                let comb: Comb = sum.into_iter().filter(|&(_, w)| w.abs() > 1e-15).collect();
                let position = comb
                    .iter()
                    .map(|&(id, w)| out.vertices[id as usize] * w)
                    .sum::<DVec3>();
                let id =
                    u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
                out.vertices.push(position);
                defs.insert(id, Def::Blend(comb));
                id
            };
        }
    }
    // Wound as the opening runs: along the top (j up), down the right
    // (i up), back along the bottom, up the left.
    for i in 0..n {
        for j in 0..n {
            new_faces.push((
                vec![
                    grid[i][j],
                    grid[i][j + 1],
                    grid[i + 1][j + 1],
                    grid[i + 1][j],
                ],
                homes.to_vec(),
            ));
        }
    }
    Ok(())
}

/// Corner Pattern Radial and Uniform (the tester's Maya pictures,
/// 2026-11-24) where three strips of `n` rows meet: a grid with its pole at
/// one corner of the opening, `pole` (0, 1 or 2 strip corners along the
/// laid-out `ring`), on the corner's edge rather than inside it. Going out
/// from the pole, the two strips on either side of it (their profiles
/// pole→A and pole→B) give the grid's rows, which run across the corner
/// from one strip's row to the same row of the other; the third strip's
/// profile A→B is the far side, and its rows run as lines into the pole.
/// All quads, with thin triangles only at the pole.
/// * Radial: each row is shaped like the far profile, curved, so the rows
///   are rings around the pole.
/// * Uniform: each row is the far profile scaled in toward the pole, so
///   the rows nest as smooth arches and the lines close in on the pole
///   like a gothic arch (its sides widened by `uniform_bulge`).
/// * Pinch: each row is two straight runs meeting on the middle line, so
///   the rows are chevrons under a narrow spike.
// The grid is indexed by row and line together.
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
fn corner_fill(
    ring: &[u32],
    n: usize,
    pole: usize,
    wrap: bool,
    pattern: CornerPattern,
    out: &mut Mesh,
    defs: &mut BTreeMap<u32, Def>,
    new_faces: &mut Vec<(Vec<u32>, Vec<usize>)>,
    homes: &[usize],
) -> Result<()> {
    let len = ring.len();
    // Point `t` along the profile that starts at strip corner `c`.
    let at = |c: usize, t: usize| ring[(c * n + t) % len];
    let pole_id = at(pole, 0);
    // R: pole → A; O: the far profile from B (j = 0) to A (j = n); L: pole → B.
    let r = |i: usize| at(pole, i);
    let o = |j: usize| at(pole + 1, n - j);
    let l = |i: usize| at(pole + 2, n - i);
    let one = |id: u32, w: f64| (id, w);
    let add = |parts: Vec<(u32, f64)>| -> Comb {
        let mut sum: BTreeMap<u32, f64> = BTreeMap::new();
        for (id, w) in parts {
            *sum.entry(id).or_default() += w;
        }
        sum.into_iter().filter(|&(_, w)| w.abs() > 1e-15).collect()
    };
    // The Radial point at row i, line j: a blend of the two side profiles,
    // pushed out to land on the far profile at i = n.
    // Radial where one strip turns the other way (the tester's L, an inner
    // corner): every row wraps around the corner as an arc shaped like the
    // far profile; see `widen_sides`.
    // Uniform's arch bulges outward there: its sides go part of that way.
    // On an outer corner Uniform keeps its straight runs (its smooth
    // arches would be Radial's rows there).
    let inner = wrap;
    if wrap && pattern == CornerPattern::Uniform {
        widen_sides(ring, n, pole, out, uniform_bulge);
    }
    let wrap = wrap && pattern == CornerPattern::Radial && widen_sides(ring, n, pole, out, |_| 1.);
    let radial = |i: usize, s: f64, oj: Option<u32>| -> Comb {
        let f = if wrap { 1. } else { i as f64 / n as f64 };
        let mut parts = vec![one(l(i), 1. - s), one(r(i), s)];
        parts.push(one(l(n), -f * (1. - s)));
        parts.push(one(r(n), -f * s));
        match oj {
            Some(id) => parts.push(one(id, f)),
            // Between two far points (the middle line of an odd n).
            None => {
                let k = s * n as f64;
                let (j0, w) = (k.floor() as usize, k.fract());
                parts.push(one(o(j0), f * (1. - w)));
                parts.push(one(o((j0 + 1).min(n)), f * w));
            }
        }
        add(parts)
    };
    let mut grid = vec![vec![u32::MAX; n + 1]; n + 1];
    for i in 0..=n {
        for j in 0..=n {
            grid[i][j] = if i == 0 {
                pole_id
            } else if i == n {
                o(j)
            } else if j == 0 {
                l(i)
            } else if j == n {
                r(i)
            } else {
                let s = j as f64 / n as f64;
                // Uniform (the tester's Maya picture, 2026-11-25: "should
                // look smoother"): each row is the far profile's arch,
                // scaled in toward the pole, so the rows nest as smooth
                // arches and the lines bend gradually into the pole.
                // Pinch keeps the straight runs to the middle line, and so
                // does Uniform on an outer corner.
                let comb = if pattern == CornerPattern::Pinch
                    || (pattern == CornerPattern::Uniform && !inner)
                {
                    // Straight from the side row to the middle line.
                    let mid = radial(i, 0.5, None);
                    let (side, t) = if s <= 0.5 {
                        (l(i), 2. * s)
                    } else {
                        (r(i), 2. * (1. - s))
                    };
                    let mut parts: Vec<(u32, f64)> = vec![one(side, 1. - t)];
                    parts.extend(mid.into_iter().map(|(id, w)| (id, w * t)));
                    add(parts)
                } else {
                    radial(i, s, Some(o(j)))
                };
                let position = comb
                    .iter()
                    .map(|&(id, w)| out.vertices[id as usize] * w)
                    .sum::<DVec3>();
                let id =
                    u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
                out.vertices.push(position);
                defs.insert(id, Def::Blend(comb));
                id
            };
        }
    }
    // Wound as the opening runs: pole → A along j = n, A → B along i = n,
    // B → pole along j = 0.
    // On an inner corner Radial's first arc closes on the pole as one
    // polygon, as in Maya (the tester's picture, 2026-11-24), not a fan.
    if wrap {
        let mut top = vec![pole_id];
        top.extend((0..=n).rev().map(|j| grid[1][j]));
        new_faces.push((top, homes.to_vec()));
    }
    for i in 0..n {
        for j in 0..n {
            if wrap && i == 0 {
                continue;
            }
            let face = if i == 0 {
                vec![pole_id, grid[1][j + 1], grid[1][j]]
            } else {
                vec![
                    grid[i][j],
                    grid[i][j + 1],
                    grid[i + 1][j + 1],
                    grid[i + 1][j],
                ]
            };
            new_faces.push((face, homes.to_vec()));
        }
    }
    Ok(())
}

/// Uniform's arch (the tester's Maya picture, 2026-11-25): how far its
/// sides go toward the wrapped ones at fraction `t` from the pole (0) to
/// the far strip (1). Fuller near the strip, narrowing to the point, a
/// gothic arch whose sides run on out of the strip without a bend.
fn uniform_bulge(t: f64) -> f64 {
    t.sqrt()
}

/// An inner corner's sides (the tester's L and Maya's Radial and Patch
/// pictures, 2026-11-24): where one strip turns the other way, the rows of
/// the two side strips (pole → B and pole → A) don't stop on a mitre line
/// but wrap around the corner as arcs shaped like the far profile (the
/// turning strip's end), only the top line staying sharp at the pole. Each
/// side row's end slides back along its strip so that row i is the far
/// profile moved by one offset T(i), which differs from the side rows' ends
/// only along their strips. Returns false (nothing moved) if that can't be
/// solved.
fn widen_sides(
    ring: &[u32],
    n: usize,
    pole: usize,
    out: &mut Mesh,
    amount: impl Fn(f64) -> f64,
) -> bool {
    if n < 2 {
        return false;
    }
    let len = ring.len();
    let at = |c: usize, t: usize| ring[(c * n + t) % len];
    let r = |i: usize| at(pole, i);
    let o = |j: usize| at(pole + 1, n - j);
    let l = |i: usize| at(pole + 2, n - i);
    let q = |id: u32| out.vertices[id as usize];
    // The strips' directions: the far profile's tangents at its ends.
    let tangent = |a: u32, b: u32, c: u32| (q(b) * 4. - q(a) * 3. - q(c)).try_normalize();
    let (Some(dl), Some(dr)) = (tangent(o(0), o(1), o(2)), tangent(o(n), o(n - 1), o(n - 2)))
    else {
        return false;
    };
    let mut moved = Vec::new();
    for i in 1..n {
        let (ul, ur) = (q(l(i)) - q(l(n)), q(r(i)) - q(r(n)));
        // ul + a·dl = ur + b·dr, least squares.
        let d = ur - ul;
        let (aa, ab, bb) = (dl.dot(dl), -dl.dot(dr), dr.dot(dr));
        let det = aa * bb - ab * ab;
        if det.abs() < 1e-12 {
            return false;
        }
        let (x, y) = (dl.dot(d), -dr.dot(d));
        let t = ul + dl * ((x * bb - y * ab) / det);
        let (pl, pr) = (q(l(i)), q(r(i)));
        let amount = amount(i as f64 / n as f64);
        moved.push((
            l(i),
            pl + (q(o(0)) + t - pl) * amount,
            r(i),
            pr + (q(o(n)) + t - pr) * amount,
        ));
    }
    for (a, pa, b, pb) in moved {
        out.vertices[a as usize] = pa;
        out.vertices[b as usize] = pb;
    }
    true
}

/// Patch on an inner corner whose sides `widen_sides` moved (even `n`; the
/// tester's Maya Patch picture, 2026-11-24): the measured three-sector
/// layout (each strip corner an m × m grid, m = n / 2, the three meeting at
/// one centre), but every point lies on the same wrapped surface as Radial,
/// so the side strips' rows run on into the corner as curves and cross near
/// the pole instead of meeting straight-sided panels. A point is placed by
/// its share of the pole (how far down the rows it lies) and the shares of
/// the two far corners (where across the rows).
#[allow(clippy::too_many_arguments)]
fn corner_patch_wrapped(
    ring: &[u32],
    n: usize,
    pole: usize,
    out: &mut Mesh,
    defs: &mut BTreeMap<u32, Def>,
    new_faces: &mut Vec<(Vec<u32>, Vec<usize>)>,
    homes: &[usize],
) -> Result<()> {
    let len = ring.len();
    let at = |c: usize, t: usize| ring[(c * n + t) % len];
    let pole_id = at(pole, 0);
    let r = |i: usize| at(pole, i);
    let o = |j: usize| at(pole + 1, n - j);
    let l = |i: usize| at(pole + 2, n - i);
    let m = n / 2;
    let nf = n as f64;
    // A point on ring i (1..=n) at a fraction across (0 at L, 1 at R): the
    // far profile moved by the ring's offset, as a blend of ring points.
    let on_ring = |i: usize, across: f64| -> Comb {
        let k = across * nf;
        let j0 = (k.floor().max(0.) as usize).min(n - 1);
        let w = k - j0 as f64;
        // Each side's offset from the far profile, blended across, so the
        // ring ends exactly on both sides however far each moved.
        vec![
            (l(i), 1. - across),
            (o(0), -(1. - across)),
            (r(i), across),
            (o(n), -across),
            (o(j0), 1. - w),
            (o(j0 + 1), w),
        ]
    };
    // The wrapped surface at `down` rows from the pole (0..=n) and a
    // fraction `across` (0 at L, 1 at R).
    let comb_at = |down: f64, across: f64| -> Comb {
        let i0 = (down.floor().max(0.) as usize).min(n - 1);
        let w = down - i0 as f64;
        let ring_at = |i: usize| -> Comb {
            if i == 0 {
                vec![(pole_id, 1.)]
            } else {
                on_ring(i, across)
            }
        };
        let mut sum: BTreeMap<u32, f64> = BTreeMap::new();
        for (id, x) in ring_at(i0) {
            *sum.entry(id).or_default() += x * (1. - w);
        }
        for (id, x) in ring_at(i0 + 1) {
            *sum.entry(id).or_default() += x * w;
        }
        sum.into_iter().filter(|&(_, x)| x.abs() > 1e-15).collect()
    };
    // Shares: [pole, L's far corner, R's far corner].
    let comb_of = |b: [f64; 3]| -> Comb {
        let side = b[1] + b[2];
        let across = if side < 1e-12 { 0.5 } else { b[2] / side };
        comb_at((1. - b[0]) * nf, across)
    };
    let key = |b: [f64; 3]| b.map(|x| (x * 1e9).round() as i64);
    // Points on the opening (one share zero) are the ring's own.
    let on_opening = |b: [f64; 3]| -> Option<u32> {
        let step = |x: f64| (x * nf).round() as usize;
        if b[2].abs() < 1e-9 {
            Some(l(step(b[1])))
        } else if b[1].abs() < 1e-9 {
            Some(r(step(b[2])))
        } else if b[0].abs() < 1e-9 {
            Some(o(step(b[2])))
        } else {
            None
        }
    };
    let mut made: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    // Each inside point's sector and place in it, for laying it out below.
    let mut cell: BTreeMap<u32, (usize, usize, usize)> = BTreeMap::new();
    for c in 0..3 {
        let (a, b) = ((c + 1) % 3, (c + 2) % 3);
        let mut corner = [0f64; 3];
        corner[c] = 1.;
        let mid = |x: usize| {
            let mut t = [0f64; 3];
            t[c] = 0.5;
            t[x] = 0.5;
            t
        };
        let (ma, mb, centre) = (mid(a), mid(b), [1. / 3.; 3]);
        // Sector at corner c: bilinear over c, mid(c, a), centre, mid(c, b).
        let point = |u: usize, v: usize| -> [f64; 3] {
            let (fu, fv) = (u as f64 / m as f64, v as f64 / m as f64);
            let mut w = [0f64; 3];
            for k in 0..3 {
                w[k] = corner[k] * (1. - fu) * (1. - fv)
                    + ma[k] * fu * (1. - fv)
                    + centre[k] * fu * fv
                    + mb[k] * (1. - fu) * fv;
            }
            w
        };
        for u in 0..m {
            for v in 0..m {
                let quad = [
                    point(u, v),
                    point(u + 1, v),
                    point(u + 1, v + 1),
                    point(u, v + 1),
                ];
                // Counter-clockwise in (share of R, 1 - share of pole), as
                // the pole grid's faces run.
                let xy = |b: [f64; 3]| (b[2], b[1] + b[2]);
                let mut area = 0.;
                for k in 0..4 {
                    let (p0, p1) = (xy(quad[k]), xy(quad[(k + 1) % 4]));
                    area += p0.0 * p1.1 - p1.0 * p0.1;
                }
                let mut face = Vec::with_capacity(4);
                let places = [(u, v), (u + 1, v), (u + 1, v + 1), (u, v + 1)];
                for (bary, (pu, pv)) in quad.into_iter().zip(places) {
                    let id = if let Some(id) = on_opening(bary) {
                        id
                    } else if let Some(&id) = made.get(&key(bary)) {
                        id
                    } else {
                        let comb = comb_of(bary);
                        let position = comb
                            .iter()
                            .map(|&(id, w)| out.vertices[id as usize] * w)
                            .sum::<DVec3>();
                        let id = u32::try_from(out.vertices.len())
                            .map_err(|_| "Mesh point budget exceeded")?;
                        out.vertices.push(position);
                        defs.insert(id, Def::Blend(comb));
                        made.insert(key(bary), id);
                        cell.insert(id, (c, pu, pv));
                        id
                    };
                    face.push(id);
                }
                if area < 0. {
                    face.reverse();
                }
                new_faces.push((face, homes.to_vec()));
            }
        }
    }
    // Lay each sector out as Maya's Patch does (the tester's pictures: the
    // grid lines turn like nested L's from one strip into the next, every
    // polygon inside that L): a Coons grid between the sector's real sides
    // (two runs of the opening and two straight lines to the centre), each
    // point then set onto the wrapped surface. (Spacing points by their
    // share of the pole bent every line toward it, a Pinch look.)
    let place = |c: &Comb, out: &Mesh| {
        c.iter()
            .map(|&(id, w)| out.vertices[id as usize] * w)
            .sum::<DVec3>()
    };
    let steps = 48usize;
    let mut samples: Vec<(f64, f64, DVec3)> = Vec::with_capacity((steps + 1) * (steps + 1));
    for a in 0..=steps {
        for b in 0..=steps {
            let (down, across) = (a as f64 / steps as f64 * nf, b as f64 / steps as f64);
            samples.push((down, across, place(&comb_at(down, across), out)));
        }
    }
    let onto = |target: DVec3, out: &Mesh| -> (f64, f64) {
        let mut best = samples
            .iter()
            .min_by(|x, y| {
                x.2.distance_squared(target)
                    .total_cmp(&y.2.distance_squared(target))
            })
            .map_or((0., 0.5), |s| (s.0, s.1));
        let mut step = (nf / steps as f64, 1. / steps as f64);
        for _ in 0..8 {
            step = (step.0 * 0.5, step.1 * 0.5);
            let mut pick = best;
            let mut d = place(&comb_at(best.0, best.1), out).distance_squared(target);
            for (da, db) in [(-1., 0.), (1., 0.), (0., -1.), (0., 1.)] {
                let c = (
                    (best.0 + da * step.0).clamp(0., nf),
                    (best.1 + db * step.1).clamp(0., 1.),
                );
                let e = place(&comb_at(c.0, c.1), out).distance_squared(target);
                if e < d {
                    (pick, d) = (c, e);
                }
            }
            best = pick;
        }
        best
    };
    let pos = |id: u32, out: &Mesh| out.vertices[id as usize];
    let corner_id = [pole_id, o(0), o(n)];
    // The opening's point `t` (0..=m) from corner c toward corner x.
    let side = |c: usize, x: usize, t: usize| -> u32 {
        let mut b = [0f64; 3];
        b[c] = 1. - t as f64 / n as f64;
        b[x] = t as f64 / n as f64;
        on_opening(b).unwrap_or(corner_id[c])
    };
    let centre_id = made.get(&key([1. / 3.; 3])).copied();
    let centre = {
        let mean =
            (pos(side(0, 1, m), out) + pos(side(1, 2, m), out) + pos(side(2, 0, m), out)) / 3.;
        let (down, across) = onto(mean, out);
        place(&comb_at(down, across), out)
    };
    let mut laid: Vec<(u32, Comb)> = Vec::new();
    for (&id, &(c, u, v)) in &cell {
        let (a, b) = ((c + 1) % 3, (c + 2) % 3);
        let (fs, ft) = (u as f64 / m as f64, v as f64 / m as f64);
        let cp = pos(corner_id[c], out);
        let (ma, mb) = (pos(side(c, a, m), out), pos(side(c, b, m), out));
        let along_a = |t: usize| pos(side(c, a, t), out);
        let along_b = |t: usize| pos(side(c, b, t), out);
        let target = if Some(id) == centre_id {
            centre
        } else {
            (1. - ft) * along_a(u)
                + ft * mb.lerp(centre, fs)
                + (1. - fs) * along_b(v)
                + fs * ma.lerp(centre, ft)
                - ((1. - fs) * (1. - ft) * cp
                    + fs * (1. - ft) * ma
                    + fs * ft * centre
                    + (1. - fs) * ft * mb)
        };
        let (down, across) = onto(target, out);
        laid.push((id, comb_at(down, across)));
    }
    for (id, comb) in laid {
        out.vertices[id as usize] = place(&comb, out);
        defs.insert(id, Def::Blend(comb));
    }
    Ok(())
}

/// EBC-J-05 / EBC-B3-04: fill the opening `ring` where strips of `n` rows
/// meet (`ring[0]`, `ring[n]`, … are strip corners, the rest profile rows,
/// wound against the polygons around it). Each corner gets an m × m grid of
/// quads (m = n / 2) bounded by its two half-profiles; with even `n` the
/// grids share one centre point, with odd `n` strips of quads run along each
/// profile's middle segment to a central polygon. This reproduces the
/// measured face layouts for 2, 3, 4 and 6 rows at three edges.
///
/// Positions (provisional, EBC-Q08): with even rows the grids' shared far
/// corner is the mean of the parallelograms A + B − C of each corner C and
/// the two profile points A, B m rows away (this gives the measured centre,
/// 2·mean(middle rows) − mean(corners)); with odd rows the central
/// polygon's corners sit 1/n of the way from that mean toward each corner.
/// Each grid's far sides are straight and its inside is a Coons patch. The
/// measured inner points of 3+ rows differ from these by up to about 0.1 of
/// the width.
// The grids are indexed by region, column and row together.
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
fn corner_patch(
    ring: &[u32],
    n: usize,
    square: Option<(u32, f64)>,
    out: &mut Mesh,
    defs: &mut BTreeMap<u32, Def>,
    new_faces: &mut Vec<(Vec<u32>, Vec<usize>)>,
    homes: &[usize],
) -> Result<()> {
    let len = ring.len();
    let k = len / n;
    let m = n / 2;
    let odd = n % 2 == 1;
    let r = |i: isize| ring[i.rem_euclid(len as isize) as usize];
    let scale = |c: &Comb, w: f64| -> Comb { c.iter().map(|&(id, x)| (id, x * w)).collect() };
    let add = |parts: &[Comb]| -> Comb {
        let mut sum: BTreeMap<u32, f64> = BTreeMap::new();
        for part in parts {
            for &(id, w) in part {
                *sum.entry(id).or_default() += w;
            }
        }
        sum.into_iter().filter(|&(_, w)| w != 0.).collect()
    };
    let lerp_comb = |a: &Comb, b: &Comb, t: f64| add(&[scale(a, 1. - t), scale(b, t)]);
    let mut make = |out: &mut Mesh, comb: Comb| -> Result<u32> {
        let position = comb
            .iter()
            .map(|&(id, w)| out.vertices[id as usize] * w)
            .sum::<DVec3>();
        let id = u32::try_from(out.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
        out.vertices.push(position);
        defs.insert(id, Def::Blend(comb));
        Ok(id)
    };
    let one = |id: u32| -> Comb { vec![(id, 1.)] };
    // Per region: its corner C, the profile points A(j) forward and B(j)
    // backward in the ring, and the far corner P.
    let a = |i: usize, j: usize| r((i * n + j) as isize);
    let b = |i: usize, j: usize| r(i as isize * n as isize - j as isize);
    let far: Vec<Comb> = (0..k)
        .map(|i| add(&[one(a(i, m)), one(b(i, m)), scale(&one(a(i, 0)), -1.)]))
        .collect();
    let mean = add(&far
        .iter()
        .map(|c| scale(c, 1. / k as f64))
        .collect::<Vec<_>>());
    let far: Vec<Comb> = if odd {
        // The central polygon: the corners' shape, 1/n the size, around
        // that mean (the parallelograms themselves coincide on a flat
        // patch, e.g. EBC-B3-07's planar T).
        (0..k)
            .map(|i| lerp_comb(&mean, &one(a(i, 0)), 1. / n as f64))
            .collect()
    } else if let Some((v, depth)) = square {
        vec![lerp_comb(&mean, &one(v), depth); k]
    } else {
        vec![mean; k]
    };
    let mut grid = vec![vec![vec![u32::MAX; m + 1]; m + 1]; k];
    let center = match square {
        // At full Depth the original corner point itself.
        Some((v, 1.)) => Some(v),
        _ if odd => None,
        _ => Some(make(out, far[0].clone())?),
    };
    for i in 0..k {
        for j in 0..=m {
            grid[i][j][0] = a(i, j);
            grid[i][0][j] = b(i, j);
        }
        grid[i][m][m] = match center {
            Some(c) => c,
            None => make(out, far[i].clone())?,
        };
    }
    // Far sides: A(m) → P (j = m) and B(m) → P (row m). With even rows a
    // region's A side is the next region's B side.
    for i in 0..k {
        for t in 1..m {
            let s = t as f64 / m as f64;
            grid[i][m][t] = make(out, lerp_comb(&one(a(i, m)), &far[i], s))?;
        }
    }
    for i in 0..k {
        for t in 1..m {
            grid[i][t][m] = if odd {
                let s = t as f64 / m as f64;
                make(out, lerp_comb(&one(b(i, m)), &far[i], s))?
            } else {
                grid[(i + k - 1) % k][m][t]
            };
        }
    }
    // Inside: a Coons patch over the four sides.
    for i in 0..k {
        let c = one(a(i, 0));
        for j in 1..m {
            for t in 1..m {
                let (u, v) = (j as f64 / m as f64, t as f64 / m as f64);
                let bottom = one(a(i, j));
                let left = one(b(i, t));
                let top = lerp_comb(&one(b(i, m)), &far[i], u);
                let right = lerp_comb(&one(a(i, m)), &far[i], v);
                let comb = add(&[
                    scale(&bottom, 1. - v),
                    scale(&top, v),
                    scale(&left, 1. - u),
                    scale(&right, u),
                    scale(&c, -(1. - u) * (1. - v)),
                    scale(&one(a(i, m)), -u * (1. - v)),
                    scale(&one(b(i, m)), -(1. - u) * v),
                    scale(&far[i], -u * v),
                ]);
                grid[i][j][t] = make(out, comb)?;
            }
        }
    }
    for g in &grid {
        for j in 0..m {
            for t in 0..m {
                new_faces.push((
                    vec![g[j][t + 1], g[j][t], g[j + 1][t], g[j + 1][t + 1]],
                    homes.to_vec(),
                ));
            }
        }
    }
    if odd {
        for i in 0..k {
            let (g, h) = (&grid[i], &grid[(i + 1) % k]);
            for t in 0..m {
                new_faces.push((
                    vec![g[m][t + 1], g[m][t], h[t][m], h[t + 1][m]],
                    homes.to_vec(),
                ));
            }
        }
        new_faces.push(((0..k).map(|i| grid[i][m][m]).collect(), homes.to_vec()));
    }
    Ok(())
}
