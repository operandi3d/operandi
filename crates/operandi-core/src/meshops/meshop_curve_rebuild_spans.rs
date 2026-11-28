//! Curve Rebuild Spans as a Mesh Stack op (`curve.rebuildSpan`,
//! MESHOPS-TIDY TB-30..37): each chosen span of a curve is split into
//! Count spans (or one per Distance), at equal steps of the curve's
//! parameter; the new knots can be pushed by Offset Amount along a vector,
//! the curve's own normal or an Alignment Mesh's normal, and snapped onto
//! that mesh first (Base Position = Mesh Position). Every curve it touches
//! comes out as a Bezier with its shape kept.
//!
//! Provisional (MOQ-69): a span is chosen when a chosen edge joins two of
//! its stored points; a cast along an axis looks the Flip way first, then
//! the other (TB-35); Broken Tangents on leaves a moved knot's handles
//! where they were, off moves them with it (TB-36). Span Reduction and
//! Falloff By Distance are kept and saved but do nothing (TB-37). Choice
//! values are stored by their place in the list.
use crate::curves::CurveKind;
use crate::mesh_stack_builtin::{check, choice, count, distance};
use crate::meshop_curve_common::{drop_faces, kind_of, points_of, push_curve};
use crate::meshop_types::{Field, FieldKind, OpContext, OpDef, SelElement, Typed, TypedOp};
use crate::{Mesh, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const REBUILD_SPANS_ID: &str = "curve.rebuildSpan";
/// Most spans one span may be split into.
const MAX_SPLIT: u32 = 10_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RebuildSpansOp {
    /// The Alignment Mesh item (0 = none); Operandi's own key for the
    /// reference's link.
    pub alignment_mesh: u64,
    pub apply: f64,
    /// 0 Curve Position, 1 Mesh Position.
    pub push_root_mode: u32,
    pub effect_all_points: bool,
    /// 0 Count, 1 Distance.
    pub rebuild_mode: u32,
    pub break_count: u32,
    pub break_dist: f64,
    pub push_amount: f64,
    /// 0 Mesh Normal, 1 Vector, 2 Curve Normal.
    pub push_mode: u32,
    #[serde(rename = "pushDirection.X")]
    pub push_x: f64,
    #[serde(rename = "pushDirection.Y")]
    pub push_y: f64,
    #[serde(rename = "pushDirection.Z")]
    pub push_z: f64,
    pub break_tangents: bool,
    /// Kept, no effect (TB-37): 0 None, 1 Averaged, 2 Longest.
    pub reduce_mode: u32,
    /// Kept, no effect (TB-37).
    #[serde(rename = "reduceMaxD")]
    pub reduce_max_d: f64,
    /// 0 Closest, 1 X, 2 Y, 3 Z, 4 User Vector, 5 Curve Normal.
    pub search_mode: u32,
    #[serde(rename = "searchVec.X")]
    pub search_x: f64,
    #[serde(rename = "searchVec.Y")]
    pub search_y: f64,
    #[serde(rename = "searchVec.Z")]
    pub search_z: f64,
    pub search_vec_flip: bool,
    /// 0 = no limit.
    #[serde(rename = "searchMaxD")]
    pub search_max_d: f64,
    pub falloff_param: bool,
    /// Kept, no effect (TB-37).
    pub falloff_distance: bool,
    /// 0 Linear, 1 Ease In, 2 Ease Out, 3 Smooth.
    pub falloff_mode: u32,
}
impl Default for RebuildSpansOp {
    fn default() -> Self {
        Self {
            alignment_mesh: 0,
            apply: 1.,
            push_root_mode: 0,
            effect_all_points: false,
            rebuild_mode: 0,
            break_count: 3,
            break_dist: 0.1,
            push_amount: 0.,
            push_mode: 0,
            push_x: 0.,
            push_y: 1.,
            push_z: 0.,
            break_tangents: false,
            reduce_mode: 0,
            reduce_max_d: 0.,
            search_mode: 0,
            search_x: 0.,
            search_y: 1.,
            search_z: 0.,
            search_vec_flip: false,
            search_max_d: 0.,
            falloff_param: false,
            falloff_distance: false,
            falloff_mode: 0,
        }
    }
}

type Cubic = [DVec3; 4];
fn at(c: &Cubic, t: f64) -> DVec3 {
    let s = 1. - t;
    c[0] * (s * s * s) + c[1] * (3. * s * s * t) + c[2] * (3. * s * t * t) + c[3] * (t * t * t)
}
fn tangent(c: &Cubic, t: f64) -> DVec3 {
    let s = 1. - t;
    (c[1] - c[0]) * (3. * s * s) + (c[2] - c[1]) * (6. * s * t) + (c[3] - c[2]) * (3. * t * t)
}
fn second(c: &Cubic, t: f64) -> DVec3 {
    (c[2] - c[1] * 2. + c[0]) * (6. * (1. - t)) + (c[3] - c[2] * 2. + c[1]) * (6. * t)
}
/// The piece of `c` from `t0` to `t1`, exactly.
fn piece(c: &Cubic, t0: f64, t1: f64) -> Cubic {
    let k = (t1 - t0) / 3.;
    let (a, b) = (at(c, t0), at(c, t1));
    [a, a + tangent(c, t0) * k, b - tangent(c, t1) * k, b]
}
/// The curve's normal (towards the inside of its bend) at `t`, if it
/// bends there.
fn curve_normal(c: &Cubic, t: f64) -> Option<DVec3> {
    let d = tangent(c, t);
    let a = second(c, t);
    let len2 = d.length_squared();
    if len2 < 1e-24 {
        return None;
    }
    (a - d * (a.dot(d) / len2)).try_normalize()
}
fn length(c: &Cubic) -> f64 {
    (0..32)
        .map(|i| at(c, i as f64 / 32.).distance(at(c, (i + 1) as f64 / 32.)))
        .sum()
}

/// Each span of the drawn curve as a cubic, and the stored point numbers
/// (places in the element's point list) that span uses.
fn spans(points: &[DVec3], kind: CurveKind, closed: bool) -> Vec<(Cubic, Vec<usize>)> {
    let n = points.len();
    if kind.is_line() {
        let count = if closed { n } else { n.saturating_sub(1) };
        return (0..count)
            .map(|i| {
                let (a, b) = (points[i], points[(i + 1) % n]);
                (
                    [a, a.lerp(b, 1. / 3.), a.lerp(b, 2. / 3.), b],
                    vec![i, (i + 1) % n],
                )
            })
            .collect();
    }
    let cubics = crate::curve_convert::span_cubics(points, kind, closed);
    let used = |i: usize| -> Vec<usize> {
        let w = |k: usize| k % n.max(1);
        match kind {
            CurveKind::Bezier => (0..4).map(|k| w(3 * i + k)).collect(),
            CurveKind::BSpline => (0..4).map(|k| w(i + k)).collect(),
            CurveKind::Curve { start_control, .. } => {
                let off = usize::from(start_control);
                vec![w(off + i), w(off + i + 1)]
            }
            CurveKind::Line => Vec::new(),
        }
    };
    cubics
        .into_iter()
        .enumerate()
        .map(|(i, c)| (c, used(i)))
        .collect()
}

/// Möller–Trumbore: the distance along `dir` from `o` to the triangle.
fn ray_hit(o: DVec3, dir: DVec3, [a, b, c]: [DVec3; 3]) -> Option<f64> {
    let (e1, e2) = (b - a, c - a);
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-18 {
        return None;
    }
    let inv = 1. / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    let eps = 1e-9;
    if u < -eps || v < -eps || u + v > 1. + eps {
        return None;
    }
    let t = e2.dot(q) * inv;
    t.is_finite().then_some(t)
}

/// The Alignment Mesh in this item's space, split into triangles.
struct Guide {
    mesh: Mesh,
    tris: Vec<(usize, [usize; 3])>,
}
impl Guide {
    fn corners(&self, f: usize, t: [usize; 3]) -> [DVec3; 3] {
        t.map(|c| self.mesh.vertices[self.mesh.faces[f].vertices[c] as usize])
    }
    /// The nearest surface point and its normal.
    fn closest(&self, p: DVec3) -> Option<(DVec3, DVec3)> {
        let (f, t, w) = crate::generator_ops::nearest_on(&self.mesh, &self.tris, p)?;
        let [a, b, c] = self.corners(f, t);
        Some((a * w.x + b * w.y + c * w.z, self.mesh.normal(f)))
    }
    /// The nearest hit along `dir` (only forwards).
    fn cast(&self, p: DVec3, dir: DVec3) -> Option<(f64, DVec3, DVec3)> {
        self.tris
            .iter()
            .filter_map(|&(f, t)| {
                let d = ray_hit(p, dir, self.corners(f, t))?;
                (d >= -1e-12).then(|| (d, p + dir * d, self.mesh.normal(f)))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }
}

/// Falloff By Parameter's shaping (TB-34) of `x` in 0..1.
fn shaped(mode: u32, x: f64) -> f64 {
    match mode {
        1 => 1. - (1. - x) * (1. - x),
        2 => x * x,
        3 => 3. * x * x - 2. * x * x * x,
        _ => x,
    }
}

/// One knot of the rebuilt curve: where it is, its two handles, whether
/// it is new, its place in its original span and that span.
struct Knot {
    anchor: [DVec3; 3],
    new: bool,
    t: f64,
    span: Cubic,
}

impl RebuildSpansOp {
    fn check(&self) -> Result<()> {
        let numbers = [
            self.apply,
            self.break_dist,
            self.push_amount,
            self.push_x,
            self.push_y,
            self.push_z,
            self.search_x,
            self.search_y,
            self.search_z,
            self.search_max_d,
            self.reduce_max_d,
        ];
        if numbers.iter().any(|v| !v.is_finite()) {
            return Err("Curve Rebuild Spans values must be numbers".into());
        }
        let lists = [
            (self.push_root_mode, 1, "Base Position"),
            (self.rebuild_mode, 1, "Mode"),
            (self.push_mode, 2, "Offset Mode"),
            (self.reduce_mode, 2, "Reduction Mode"),
            (self.search_mode, 5, "Normal Method"),
            (self.falloff_mode, 3, "Falloff Mode"),
        ];
        for (v, max, what) in lists {
            if v > max {
                return Err(format!("{what} must be 0 to {max}"));
            }
        }
        if self.rebuild_mode == 1 && self.break_dist <= 0. {
            return Err("Distance must be more than 0".into());
        }
        Ok(())
    }

    /// How many spans span `c` becomes (TB-31, TB-32, TB-Q3).
    fn pieces(&self, c: &Cubic) -> Result<u32> {
        let k = if self.rebuild_mode == 0 {
            self.break_count.max(1)
        } else {
            ((length(c) / self.break_dist + 1e-6).floor() as u32).max(1)
        };
        if k > MAX_SPLIT {
            return Err(format!("A span can be split into at most {MAX_SPLIT}"));
        }
        Ok(k)
    }

    /// The surface point for `p` by Normal Method (TB-35), and its normal.
    fn find(&self, guide: &Guide, p: DVec3, curve_n: Option<DVec3>) -> Option<(DVec3, DVec3)> {
        let limit =
            |q: DVec3| self.search_max_d <= 0. || q.distance(p) <= self.search_max_d + 1e-12;
        let dir = match self.search_mode {
            0 => return guide.closest(p).filter(|(q, _)| limit(*q)),
            1 => DVec3::X,
            2 => DVec3::Y,
            3 => DVec3::Z,
            4 => DVec3::new(self.search_x, self.search_y, self.search_z).try_normalize()?,
            _ => curve_n?,
        };
        let dir = if self.search_vec_flip { -dir } else { dir };
        guide
            .cast(p, dir)
            .or_else(|| guide.cast(p, -dir))
            .map(|(_, q, n)| (q, n))
            .filter(|(q, _)| limit(*q))
    }
}

impl TypedOp for RebuildSpansOp {
    fn run(&self, ctx: &OpContext) -> Result<Mesh> {
        self.check()?;
        let mesh = ctx.input;
        let guide = match ctx.sources()?.into_iter().next() {
            Some(s) => {
                let mut m = (*s.mesh).clone();
                m.vertices
                    .iter_mut()
                    .for_each(|p| *p = s.to_local.transform_point3(*p));
                let mut tris = Vec::new();
                for f in 0..m.faces.len() {
                    if !m.is_polyline(f) {
                        tris.extend(m.triangles(f)?.into_iter().map(|t| (f, t)));
                    }
                }
                Some(Guide { mesh: m, tris })
            }
            None => None,
        };
        // The chosen edges (all with no Selection).
        let edges = mesh.edges();
        let chosen: Option<std::collections::HashSet<(u32, u32)>> = ctx.selection.map(|s| {
            s.iter()
                .filter_map(|&e| edges.get(e as usize).copied())
                .collect()
        });
        let mut out = mesh.clone();
        let mut gone = std::collections::BTreeSet::new();
        for f in 0..mesh.faces.len() {
            if !mesh.is_polyline(f) {
                continue;
            }
            let (kind, closed) = kind_of(mesh, f);
            let points = points_of(mesh, f);
            let ids = &mesh.faces[f].vertices;
            let all = spans(&points, kind, closed);
            if all.is_empty() {
                continue;
            }
            let picked = |used: &[usize]| match &chosen {
                None => true,
                Some(set) => used.iter().any(|&a| {
                    used.iter().any(|&b| {
                        let (p, q) = (ids[a], ids[b]);
                        p != q && set.contains(&(p.min(q), p.max(q)))
                    })
                }),
            };
            let wanted: Vec<bool> = all.iter().map(|(_, used)| picked(used)).collect();
            if !wanted.iter().any(|w| *w) {
                continue;
            }
            // The rebuilt pieces, then a knot at each piece's start.
            let mut knots: Vec<Knot> = Vec::new();
            let mut last: Option<Cubic> = None;
            for ((c, _), want) in all.iter().zip(&wanted) {
                let k = if *want { self.pieces(c)? } else { 1 };
                for j in 0..k {
                    let (t0, t1) = (j as f64 / k as f64, (j + 1) as f64 / k as f64);
                    let p = piece(c, t0, t1);
                    let inn = last.map_or(p[0], |l| l[2]);
                    knots.push(Knot {
                        anchor: [inn, p[0], p[1]],
                        new: j > 0,
                        t: t0,
                        span: *c,
                    });
                    last = Some(p);
                }
            }
            let end = last.expect("at least one piece");
            if closed {
                knots[0].anchor[0] = end[2];
            } else {
                knots.push(Knot {
                    anchor: [end[2], end[3], end[3]],
                    new: false,
                    t: 1.,
                    span: all[all.len() - 1].0,
                });
            }
            // TB-33..35: move the new knots (all with Effect All Points).
            for k in knots.iter_mut().filter(|k| k.new || self.effect_all_points) {
                let p = k.anchor[1];
                let weight = if self.falloff_param {
                    shaped(self.falloff_mode, 1. - (2. * k.t - 1.).abs())
                } else {
                    1.
                };
                let bend = curve_normal(&k.span, k.t);
                let found = guide.as_ref().and_then(|g| self.find(g, p, bend));
                let base = match (self.push_root_mode, found) {
                    (1, Some((q, _))) => p.lerp(q, self.apply),
                    _ => p,
                };
                let dir = match self.push_mode {
                    0 => found.map_or(DVec3::ZERO, |(_, n)| n),
                    1 => DVec3::new(self.push_x, self.push_y, self.push_z),
                    _ => bend.unwrap_or(DVec3::ZERO),
                };
                let to = base + dir * (self.apply * weight * self.push_amount);
                let d = to - p;
                k.anchor[1] = to;
                if !self.break_tangents {
                    k.anchor[0] += d;
                    k.anchor[2] += d;
                }
            }
            let anchors: Vec<[DVec3; 3]> = knots.iter().map(|k| k.anchor).collect();
            let stored = crate::curves::bezier_points(&anchors, closed);
            push_curve(&mut out, &stored, CurveKind::Bezier, closed, Some(f));
            gone.insert(f);
        }
        let out = drop_faces(&out, &gone)?;
        out.validate()?;
        Ok(out)
    }
}

pub(crate) fn entries() -> Vec<Arc<dyn OpDef>> {
    crate::meshop_types::with_element(
        vec![Arc::new(Typed::<RebuildSpansOp>::new(
            REBUILD_SPANS_ID,
            "Curve Rebuild Spans",
            "Curves",
            true,
            vec![
                Field::new("alignmentMesh", "Alignment Mesh", FieldKind::Item),
                check("effectAllPoints", "Effect All Points"),
                Field::new("apply", "Apply", FieldKind::Percent),
                choice(
                    "pushRootMode",
                    "Base Position",
                    &["Curve Position", "Mesh Position"],
                ),
                choice("rebuildMode", "Mode", &["Count", "Distance"]),
                count("breakCount", "Count", 1, MAX_SPLIT),
                distance("breakDist", "Distance"),
                distance("pushAmount", "Offset Amount"),
                choice(
                    "pushMode",
                    "Offset Mode",
                    &["Mesh Normal", "Vector", "Curve Normal"],
                ),
                Field::new("pushDirection.X", "Offset Vector X", FieldKind::Number),
                Field::new("pushDirection.Y", "Offset Vector Y", FieldKind::Number),
                Field::new("pushDirection.Z", "Offset Vector Z", FieldKind::Number),
                check("breakTangents", "Broken Tangents"),
                Field::new("reduceMode", "Reduction Mode", FieldKind::Hidden),
                Field::new("reduceMaxD", "Min Collapse Distance", FieldKind::Hidden),
                choice(
                    "searchMode",
                    "Normal Method",
                    &[
                        "Closest",
                        "X Axis",
                        "Y Axis",
                        "Z Axis",
                        "User Vector",
                        "Curve Normal",
                    ],
                ),
                Field::new("searchVec.X", "Normal Search Vector X", FieldKind::Number),
                Field::new("searchVec.Y", "Normal Search Vector Y", FieldKind::Number),
                Field::new("searchVec.Z", "Normal Search Vector Z", FieldKind::Number),
                check("searchVecFlip", "Flip Search Vector"),
                distance("searchMaxD", "Max Search Distance"),
                check("falloffParam", "Falloff By Parameter"),
                Field::new("falloffDistance", "Falloff By Distance", FieldKind::Hidden),
                choice(
                    "falloffMode",
                    "Falloff Mode",
                    &["Linear", "Ease In", "Ease Out", "Smooth"],
                ),
            ],
        ))],
        SelElement::Edge,
    )
}
