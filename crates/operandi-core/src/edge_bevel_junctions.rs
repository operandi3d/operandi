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
    EdgeBevelMode, EdgeBevelResult, EdgeBevelSettings, EdgeBevelShape, EdgeEnd,
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
                return None;
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
    let weight = ((1. + u.dot(w)).max(0.) * 0.5).sqrt();
    let (d1, d2) = (q1.distance(corner), q2.distance(corner));
    let half = u.dot(w).clamp(-1., 1.).acos() / 2.;
    let arc =
        ((d1 - d2).abs() <= 1e-9 * d1.max(d2) && half > 1e-6 && half.cos() > 1e-6).then(|| {
            let center = corner + (u + w).normalize() * (d1 / half.cos());
            (center, q1 - center, q2 - center)
        });
    (1..rows)
        .map(|k| {
            let t = k as f64 / rows as f64;
            let s = 1. - t;
            if let Some((center, a0, a1)) = arc {
                let axis = a0.cross(a1).normalize_or_zero();
                toward(center + glam::DQuat::from_axis_angle(axis, a0.angle_between(a1) * t) * a0)
            } else {
                let chord = q1.lerp(q2, t);
                let curve = (q1 * s * s + corner * (2. * weight * s * t) + q2 * t * t)
                    / (s * s + 2. * weight * s * t + t * t);
                chord.lerp(curve, depth / 100.)
            }
        })
        .collect()
}

/// One polygon corner at a point: the polygon and the points before and
/// after it.
#[derive(Clone, Copy)]
struct Corner {
    face: usize,
    prev: u32,
    next: u32,
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
                let partner = partner(v, s);
                let distance = match partner {
                    // Chamfer keeps the strip parallel to the selected edge
                    // in this polygon.
                    Some(o) if parallel => {
                        let axis = (p(o) - p(v)).normalize_or_zero();
                        width * weight(v) * factor_of(v, o) / dir.cross(axis).length().max(1e-9)
                    }
                    Some(o) => width * weight(v) * factor_of(v, o),
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
                let scale = if parallel {
                    base / ua.cross(ub).length().max(1e-9)
                } else {
                    base
                };
                // A corner point has no point to stop on.
                if options.stop_at_edge && scale >= la.min(lb) - 1e-9 {
                    return Err(past_end());
                }
                let id = push(&mut out, p(v) + (ua + ub) * scale)?;
                defs.insert(
                    id,
                    Def::Inside {
                        face: c.face,
                        v,
                        a: c.prev,
                        b: c.next,
                        ta: scale / la,
                        tb: scale / lb,
                    },
                );
                inside.insert((c.face, v), id);
            }
        }
    }
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
                // The profile's corner is on the selected edge, level with the
                // two side points (the point itself at a plain end).
                let axis = (p(other) - p(v)).normalize_or_zero();
                let corner = p(v) + axis * ((a + b) / 2. - p(v)).dot(axis);
                let points = profile(corner, a, b, rows, depth, square);
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
            .unwrap_or_default()
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
            let quad = vec![rb[k], ra[k], ra[k + 1], rb[k + 1]];
            for i in 0..4 {
                strip_edges.insert(edge_key(quad[i], quad[(i + 1) % 4]));
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
            if n >= 2 && (!square || n == 2) && ring.len() % n == 0 && ring.len() / n >= 3 {
                // EBC-J-05 / EBC-B3-04: where strips with two or more rows
                // meet, a patch of quads.
                let start = ring.iter().position(|id| !is_row(&defs, id)).unwrap_or(0);
                ring.rotate_left(start);
                if ring
                    .iter()
                    .enumerate()
                    .all(|(i, id)| is_row(&defs, id) == (i % n != 0))
                {
                    // Square keeps the corner sharp: the patch meets at the
                    // original point (Depth ignored, EBS-03).
                    let center = square.then_some((v, 1.));
                    corner_patch(
                        &ring,
                        n,
                        center,
                        &mut out,
                        &mut defs,
                        &mut new_faces,
                        &homes,
                    )?;
                    continue;
                }
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
        let mut groups: Vec<(Vec<u32>, DVec3)> = Vec::new();
        let mut grouped = BTreeSet::new();
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
