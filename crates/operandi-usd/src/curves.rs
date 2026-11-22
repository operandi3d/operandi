//! Curves both ways (USD-IQ24, provisional): Operandi's line elements
//! (`PolygonRepresentation::Polyline`: polylines, Curves, B-Splines and
//! Beziers) as BasisCurves and NurbsCurves prims, and other programs'
//! curves as line elements.
//!
//! Export, exact for every kind:
//! - a polyline is a `linear` BasisCurves (`periodic` when closed);
//! - a Bezier is a cubic `bezier` BasisCurves: Operandi stores its points
//!   the way USD does (anchor, out, in, anchor...; closed adds the last
//!   out and the first in);
//! - a closed B-Spline is a cubic `bspline` `periodic` BasisCurves; an
//!   open one, clamped to its ends, is a cubic NurbsCurves with clamped
//!   uniform knots;
//! - a smooth Curve without Start/End Control (PF-09's chord-weighted
//!   tangents) is a cubic `bezier` BasisCurves of the same spans, kind
//!   `curve`, which the import turns back into the Curve;
//! - another Curve (Catmull-Rom) is a cubic `catmullRom` BasisCurves,
//!   `periodic` when closed. Open, USD's first and last points only shape
//!   the ends, as with Start/End Control; a natural end (no control) gets
//!   a made-up control point that gives the same end tangent, and the
//!   prim's customData "operandi" `kinds` says which, so the import drops
//!   it again.
//!
//! Prims of one item are grouped by schema, type, basis and wrap (USD
//! sets those per prim); each curve has its own points, so points shared
//! with polygons or other curves come back unwelded. A `widths` typed
//! map (weight1) becomes the curves' widths, and other programs' widths
//! come in as one.
//!
//! Import of other programs' curves:
//! - linear: polylines; bezier: Beziers;
//! - bspline: closed B-Splines when periodic; otherwise converted to the
//!   same curve as a Bezier (exact), `pinned` ends first given USD's
//!   made-up end points;
//! - catmullRom: Curves with Start and End Control (exact), `pinned` ends
//!   given USD's made-up end points;
//! - NurbsCurves: order 2 polylines; cubic, unweighted, clamped uniform
//!   knots B-Splines (exact); anything else is sampled into a polyline
//!   (reported);
//! - HermiteCurves: a polyline through the points (reported).
use glam::DVec3;
use operandi_core::curves::CurveKind;
use operandi_core::{Mesh, PolygonRepresentation};
use std::collections::BTreeMap;

/// Typed map holding curve widths (weight1, per point).
pub const WIDTHS_MAP: &str = "widths";

/// One BasisCurves or NurbsCurves prim.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurvesPrim {
    pub name: String,
    /// "BasisCurves" or "NurbsCurves".
    pub schema: &'static str,
    /// BasisCurves' type, basis and wrap ("" for none).
    pub ty: &'static str,
    pub basis: &'static str,
    pub wrap: &'static str,
    pub counts: Vec<i32>,
    pub points: Vec<[f32; 3]>,
    /// One per point, when the mesh has a `widths` map.
    pub widths: Option<Vec<f32>>,
    /// NurbsCurves' order per curve and knots.
    pub order: Vec<i32>,
    pub knots: Vec<f64>,
    /// The material bound to the prim (index into Plan::materials).
    pub material: Option<usize>,
    /// Operandi's kind of each curve ("line", "curve", "curve+start",
    /// "curve+end", "curve+start+end", "bspline", "bezier"), for the
    /// import.
    pub kinds: Vec<String>,
}

impl CurvesPrim {
    /// The description `ou_define_curves` reads, and its buffers.
    pub fn json_and_buffers(
        &self,
        material: Option<&str>,
        visible: bool,
    ) -> (String, Vec<Vec<u8>>) {
        let mut buffers: Vec<Vec<u8>> = Vec::new();
        let mut add = |bytes: Vec<u8>| {
            buffers.push(bytes);
            buffers.len() - 1
        };
        let counts = add(self.counts.iter().flat_map(|c| c.to_ne_bytes()).collect());
        let points = add(self
            .points
            .iter()
            .flatten()
            .flat_map(|c| c.to_ne_bytes())
            .collect());
        let mut json = serde_json::json!({
            "name": self.name,
            "schema": self.schema,
            "counts": counts,
            "points": points,
            "visible": visible,
            "custom": { "kinds": self.kinds },
        });
        let o = json.as_object_mut().expect("an object");
        if self.schema == "BasisCurves" {
            o.insert("type".into(), self.ty.into());
            if !self.basis.is_empty() {
                o.insert("basis".into(), self.basis.into());
            }
            o.insert("wrap".into(), self.wrap.into());
        } else {
            let order = add(self.order.iter().flat_map(|c| c.to_ne_bytes()).collect());
            let knots = add(self.knots.iter().flat_map(|c| c.to_ne_bytes()).collect());
            o.insert("order".into(), order.into());
            o.insert("knots".into(), knots.into());
        }
        if let Some(w) = &self.widths {
            let widths = add(w.iter().flat_map(|c| c.to_ne_bytes()).collect());
            o.insert("widths".into(), widths.into());
        }
        if let Some(m) = material {
            o.insert("material".into(), m.into());
        }
        (json.to_string(), buffers)
    }
}

/// The prim a line element goes in: schema, type, basis, wrap.
type Group = (&'static str, &'static str, &'static str, &'static str);

/// Operandi's points for a Curve's open end without a control point: the
/// control point that gives USD's Catmull-Rom the same end tangent
/// (curves.rs `catmull_rom`'s natural end). `on` are the points the curve
/// passes through, `inner` the tangent at the neighbouring point.
fn natural_control(end: DVec3, next: DVec3, inner: Option<DVec3>) -> DVec3 {
    // Natural tangent t = (3 (next - end) - inner) / 2 against USD's
    // (next - control) / 2, so control = next - 2 t.
    let t = match inner {
        Some(inner) => ((next - end) * 3. - inner) * 0.5,
        // Two points: a straight span.
        None => next - end,
    };
    next - t * 2.
}

/// The points USD's Catmull-Rom needs for an open Operandi Curve: the
/// stored points with a made-up control point at each end that has none.
fn catmull_rom_points(points: &[DVec3], start: bool, end: bool) -> Vec<DVec3> {
    let n = points.len();
    let lo = usize::from(start);
    let hi = n - usize::from(end);
    let on = &points[lo..hi];
    let m = on.len();
    let inner = |i: usize| (on[i + 1] - on[i - 1]) * 0.5;
    let mut out = Vec::with_capacity(m + 2);
    if start {
        out.push(points[0]);
    } else {
        // Curves.rs reflects the start tangent off the next inner one.
        out.push(natural_control(on[0], on[1], (m > 2).then(|| inner(1))));
    }
    out.extend_from_slice(on);
    if end {
        out.push(points[n - 1]);
    } else {
        // Mirror image of the start: walk the curve backwards.
        let back = (m > 2).then(|| -inner(m - 2));
        out.push(natural_control(on[m - 1], on[m - 2], back));
    }
    out
}

/// Clamped uniform knots for an open cubic B-Spline of `n` points
/// (curves.rs `b_spline`): 0,0,0,0,1,…,k,k,k,k.
fn clamped_knots(n: usize) -> Vec<f64> {
    let spans = n - 3;
    let mut knots = vec![0.; 4];
    knots.extend((1..spans).map(|i| i as f64));
    knots.extend([spans as f64; 4]);
    knots
}

/// The curves prims of `mesh`'s line elements, named after `item`
/// (`has_polygons` when its polygons take the item's name).
/// `face_materials` gives each element's material (index into
/// Plan::materials). Elements with too few points are counted in
/// `skipped`.
pub fn curves_prims(
    item: &str,
    mesh: &Mesh,
    has_polygons: bool,
    face_materials: &[usize],
    skipped: &mut usize,
) -> Vec<CurvesPrim> {
    let widths = mesh
        .attributes
        .typed_maps
        .get(WIDTHS_MAP)
        .filter(|m| m.kind == "weight1");
    let mut groups: BTreeMap<Group, CurvesPrim> = BTreeMap::new();
    for f in 0..mesh.faces.len() {
        let Some(PolygonRepresentation::Polyline { closed, curve }) =
            mesh.attributes.representations.get(f).copied()
        else {
            continue;
        };
        let ids = &mesh.faces[f].vertices;
        let pts: Vec<DVec3> = ids.iter().map(|&v| mesh.vertices[v as usize]).collect();
        if pts.len() < curve.minimum_points() || (closed && curve.is_line() && pts.len() < 2) {
            *skipped += 1;
            continue;
        }
        // Points written, and the stored point each came from (for widths).
        let (group, kind, written, from): (Group, String, Vec<DVec3>, Vec<u32>) = match curve {
            CurveKind::Line => (
                (
                    "BasisCurves",
                    "linear",
                    "",
                    if closed { "periodic" } else { "nonperiodic" },
                ),
                "line".into(),
                pts.clone(),
                ids.clone(),
            ),
            CurveKind::Bezier => {
                let anchors = operandi_core::curves::bezier_anchors(&pts, closed);
                let p = operandi_core::curves::bezier_points(&anchors, closed);
                // The stored points, when they were already in that layout.
                let from = if p.len() == ids.len() {
                    ids.clone()
                } else {
                    vec![ids[0]; p.len()]
                };
                (
                    (
                        "BasisCurves",
                        "cubic",
                        "bezier",
                        if closed { "periodic" } else { "nonperiodic" },
                    ),
                    "bezier".into(),
                    p,
                    from,
                )
            }
            CurveKind::BSpline if closed => (
                ("BasisCurves", "cubic", "bspline", "periodic"),
                "bspline".into(),
                pts.clone(),
                ids.clone(),
            ),
            CurveKind::BSpline => (
                ("NurbsCurves", "", "", ""),
                "bspline".into(),
                pts.clone(),
                ids.clone(),
            ),
            // PF-09: a smooth Curve without Start/End Control has
            // chord-weighted tangents USD's Catmull-Rom can't draw: it
            // goes as the cubic Bezier of the same spans, kind "curve",
            // and the import takes its anchors back as the Curve.
            CurveKind::Curve {
                start_control: false,
                end_control: false,
            } if let Some(p) = operandi_core::curves::smooth_bezier(&pts, closed) => {
                // Each written point's stored point: an anchor's own, a
                // handle's nearest anchor.
                let from = (0..p.len())
                    .map(|k| ids[((k + 1) / 3) % ids.len()])
                    .collect();
                (
                    (
                        "BasisCurves",
                        "cubic",
                        "bezier",
                        if closed { "periodic" } else { "nonperiodic" },
                    ),
                    "curve".into(),
                    p,
                    from,
                )
            }
            CurveKind::Curve { .. } if closed => (
                ("BasisCurves", "cubic", "catmullRom", "periodic"),
                "curve+start+end".into(),
                pts.clone(),
                ids.clone(),
            ),
            CurveKind::Curve {
                start_control,
                end_control,
            } => {
                let mut kind = String::from("curve");
                if start_control {
                    kind += "+start";
                }
                if end_control {
                    kind += "+end";
                }
                let mut from = ids.clone();
                if !start_control {
                    from.insert(0, ids[0]);
                }
                if !end_control {
                    from.push(*ids.last().expect("enough points"));
                }
                (
                    ("BasisCurves", "cubic", "catmullRom", "nonperiodic"),
                    kind,
                    catmull_rom_points(&pts, start_control, end_control),
                    from,
                )
            }
        };
        let prim = groups.entry(group).or_insert_with(|| CurvesPrim {
            schema: group.0,
            ty: group.1,
            basis: group.2,
            wrap: group.3,
            material: face_materials.get(f).copied(),
            widths: widths.map(|_| Vec::new()),
            ..Default::default()
        });
        prim.counts.push(written.len() as i32);
        if group.0 == "NurbsCurves" {
            prim.order.push(4);
            prim.knots.extend(clamped_knots(written.len()));
        }
        prim.points
            .extend(written.iter().map(|p| p.as_vec3().to_array()));
        if let (Some(list), Some(map)) = (prim.widths.as_mut(), widths) {
            list.extend(from.iter().map(|v| {
                map.points
                    .get(v)
                    .and_then(|w| w.first())
                    .copied()
                    .unwrap_or(0.0) as f32
            }));
        }
        prim.kinds.push(kind);
    }
    let one = groups.len() == 1 && !has_polygons;
    groups
        .into_iter()
        .map(|(g, mut prim)| {
            prim.name = if one {
                item.to_string()
            } else {
                format!(
                    "{item}_{}{}",
                    match g {
                        (_, "linear", _, _) => "lines",
                        ("NurbsCurves", ..) => "nurbs",
                        (_, _, basis, _) => basis,
                    },
                    if g.3 == "periodic" { "_closed" } else { "" }
                )
            };
            prim
        })
        .collect()
}

/// One curve as Operandi stores it: its points, closed, and its kind.
#[derive(Clone, Debug, PartialEq)]
pub struct LineElement {
    /// The prim's curve it came from.
    pub index: usize,
    pub points: Vec<DVec3>,
    /// The file point each of `points` came from (for widths), `None` for
    /// made-up ones.
    pub from: Vec<Option<usize>>,
    pub closed: bool,
    pub curve: CurveKind,
}

/// A curves prim as the bridge's reader describes it (stage.rs).
pub struct CurvesIn<'a> {
    pub schema: &'a str,
    pub ty: &'a str,
    pub basis: &'a str,
    pub wrap: &'a str,
    pub counts: &'a [i32],
    pub points: &'a [DVec3],
    pub order: &'a [i32],
    pub knots: &'a [f64],
    pub weights: &'a [f64],
    /// What Operandi wrote: each curve's kind.
    pub kinds: Option<Vec<String>>,
}

/// USD's made-up end point for a `pinned` end: its neighbour reflected
/// through it, 2·P0 − P1.
fn pinned_end(end: DVec3, next: DVec3) -> DVec3 {
    end * 2. - next
}

/// An open uniform cubic B-Spline's spans as one Bezier, in Operandi's
/// stored order (anchor, out, in, anchor...).
fn b_spline_as_bezier(p: &[DVec3]) -> Vec<DVec3> {
    let mut out = Vec::new();
    for i in 0..p.len().saturating_sub(3) {
        let (a, b, c, d) = (p[i], p[i + 1], p[i + 2], p[i + 3]);
        if i == 0 {
            out.push((a + b * 4. + c) / 6.);
        }
        out.push((b * 2. + c) / 3.);
        out.push((b + c * 2.) / 3.);
        out.push((b + c * 4. + d) / 6.);
    }
    out
}

/// Evaluates a NURBS curve (order `k`, `knots`, optional weights) at `t`.
fn nurbs_point(points: &[DVec3], weights: &[f64], knots: &[f64], k: usize, t: f64) -> DVec3 {
    let n = points.len();
    let p = k - 1;
    let mut span = p;
    while span + 1 < n && t >= knots[span + 1] {
        span += 1;
    }
    // Homogeneous de Boor.
    let mut d: Vec<(DVec3, f64)> = (0..=p)
        .map(|j| {
            let i = j + span - p;
            let w = weights.get(i).copied().unwrap_or(1.0);
            (points[i] * w, w)
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = j + span - p;
            let denom = knots[i + p + 1 - r] - knots[i];
            let a = if denom.abs() < 1e-15 {
                0.
            } else {
                (t - knots[i]) / denom
            };
            d[j] = (
                d[j - 1].0 * (1. - a) + d[j].0 * a,
                d[j - 1].1 * (1. - a) + d[j].1 * a,
            );
        }
    }
    let (pt, w) = d[p];
    if w.abs() < 1e-15 { pt } else { pt / w }
}

/// Whether `knots` are clamped uniform for `n` points of order 4, up to
/// an offset and scale.
fn is_clamped_uniform(knots: &[f64], n: usize) -> bool {
    if n < 4 || knots.len() != n + 4 {
        return false;
    }
    let (lo, hi) = (knots[0], knots[n + 3]);
    let spans = (n - 3) as f64;
    if hi <= lo {
        return false;
    }
    clamped_knots(n)
        .iter()
        .zip(knots)
        .all(|(want, got)| ((got - lo) / (hi - lo) * spans - want).abs() < 1e-6)
}

/// Segments per knot span when a NURBS curve has to be sampled.
const NURBS_STEPS: usize = 12;

/// The line elements of one curves prim, and notes on what was
/// approximated or left out.
pub fn line_elements(c: &CurvesIn, notes: &mut Vec<String>) -> Vec<LineElement> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut knot_start = 0usize;
    let periodic = c.wrap == "periodic";
    let pinned = c.wrap == "pinned";
    for (k, &count) in c.counts.iter().enumerate() {
        let n = count.max(0) as usize;
        let range = start..(start + n).min(c.points.len());
        start += n;
        let p = &c.points[range.clone()];
        let from: Vec<Option<usize>> = range.clone().map(Some).collect();
        let kind = c
            .kinds
            .as_ref()
            .and_then(|k_| k_.get(k))
            .map(String::as_str);
        let element =
            |points: Vec<DVec3>, from: Vec<Option<usize>>, closed, curve_kind| LineElement {
                index: k,
                points,
                from,
                closed,
                curve: curve_kind,
            };
        if c.schema == "NurbsCurves" {
            let order = c.order.get(k).copied().unwrap_or(4).max(2) as usize;
            let knots = c
                .knots
                .get(knot_start..knot_start + n + order)
                .unwrap_or(&[]);
            knot_start += n + order;
            let weights = c.weights.get(range.clone()).unwrap_or(&[]);
            let unweighted = weights.iter().all(|&w| (w - 1.0).abs() < 1e-9);
            if n < order || knots.len() != n + order {
                notes.push(format!("{n}-point NURBS curve with bad knots left out"));
                continue;
            }
            if order == 2 && unweighted {
                out.push(element(p.to_vec(), from, false, CurveKind::Line));
            } else if order == 4 && unweighted && is_clamped_uniform(knots, n) {
                out.push(element(p.to_vec(), from, false, CurveKind::BSpline));
            } else {
                let (t0, t1) = (knots[order - 1], knots[n]);
                let spans = n + 1 - order;
                let steps = spans * NURBS_STEPS;
                let pts = (0..=steps)
                    .map(|s| {
                        let t = t0 + (t1 - t0) * s as f64 / steps as f64;
                        nurbs_point(p, weights, knots, order, t.min(t1 - 1e-12 * (t1 - t0)))
                    })
                    .collect::<Vec<_>>();
                let mut pts = pts;
                if let Some(last) = pts.last_mut() {
                    *last = nurbs_point(p, weights, knots, order, t1 - 1e-9 * (t1 - t0).max(1e-9));
                }
                notes.push(format!(
                    "NURBS curve(s) of order {order}{} sampled into polylines",
                    if unweighted { "" } else { " with weights" }
                ));
                let len = pts.len();
                out.push(element(pts, vec![None; len], false, CurveKind::Line));
            }
            continue;
        }
        if c.schema != "BasisCurves" {
            notes.push(format!(
                "{} read as polylines through their points",
                c.schema
            ));
            out.push(element(p.to_vec(), from, periodic, CurveKind::Line));
            continue;
        }
        if c.ty != "cubic" {
            if n >= 2 {
                out.push(element(
                    p.to_vec(),
                    from,
                    periodic && n >= 3,
                    CurveKind::Line,
                ));
            }
            continue;
        }
        match c.basis {
            "bspline" if periodic => {
                if n >= 3 {
                    out.push(element(p.to_vec(), from, true, CurveKind::BSpline));
                }
            }
            "bspline" => {
                let mut q = p.to_vec();
                if pinned && n >= 2 {
                    q.insert(0, pinned_end(p[0], p[1]));
                    q.push(pinned_end(p[n - 1], p[n - 2]));
                }
                if q.len() < 4 {
                    continue;
                }
                let b = b_spline_as_bezier(&q);
                let len = b.len();
                out.push(element(b, vec![None; len], false, CurveKind::Bezier));
            }
            "catmullRom" if periodic => {
                if n >= 3 {
                    out.push(element(
                        p.to_vec(),
                        from,
                        true,
                        CurveKind::Curve {
                            start_control: false,
                            end_control: false,
                        },
                    ));
                }
            }
            "catmullRom" => {
                // Operandi's own: drop the made-up end points again.
                let (start_control, end_control) = match kind {
                    Some(k) if k.starts_with("curve") => (k.contains("+start"), k.contains("+end")),
                    _ => (true, true),
                };
                let (mut q, mut f) = (p.to_vec(), from);
                if pinned && n >= 2 {
                    q.insert(0, pinned_end(p[0], p[1]));
                    q.push(pinned_end(p[n - 1], p[n - 2]));
                    f.insert(0, None);
                    f.push(None);
                }
                if kind.is_some() && !pinned {
                    if !start_control && !q.is_empty() {
                        q.remove(0);
                        f.remove(0);
                    }
                    if !end_control {
                        q.pop();
                        f.pop();
                    }
                }
                let curve = CurveKind::Curve {
                    start_control: start_control || kind.is_none() || pinned,
                    end_control: end_control || kind.is_none() || pinned,
                };
                if q.len() < curve.minimum_points() {
                    notes.push(format!(
                        "{n}-point Catmull-Rom curve left out: too few points"
                    ));
                    continue;
                }
                out.push(element(q, f, false, curve));
            }
            // "bezier", and USD's default basis.
            _ => {
                let usable = if periodic {
                    n - n % 3
                } else {
                    (n.max(1) - 1) / 3 * 3 + 1
                };
                if usable < n {
                    notes.push(format!(
                        "{} point(s) at the end of a Bezier curve left out (not a whole span)",
                        n - usable
                    ));
                }
                if usable < if periodic { 6 } else { 4 } {
                    continue;
                }
                // PF-09: Operandi's own smooth Curve written as a Bezier
                // comes back as the Curve through its anchors.
                if kind == Some("curve") {
                    let anchors: Vec<usize> = (0..usable).step_by(3).collect();
                    out.push(element(
                        anchors.iter().map(|&i| p[i]).collect(),
                        anchors.iter().map(|&i| from[i]).collect(),
                        periodic,
                        CurveKind::Curve {
                            start_control: false,
                            end_control: false,
                        },
                    ));
                    continue;
                }
                out.push(element(
                    p[..usable].to_vec(),
                    from[..usable].to_vec(),
                    periodic,
                    CurveKind::Bezier,
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use operandi_core::curves::sample;

    fn close(a: &[DVec3], b: &[DVec3]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.distance(*y) < 1e-9)
    }

    fn wiggle(n: usize) -> Vec<DVec3> {
        (0..n)
            .map(|i| DVec3::new(i as f64, ((i * 7) % 5) as f64 * 0.3, (i % 2) as f64))
            .collect()
    }

    /// USD's uniform Catmull-Rom on the written points, `steps` per span
    /// between the second and the second-to-last.
    fn usd_catmull_rom(p: &[DVec3], steps: usize) -> Vec<DVec3> {
        let mut out = Vec::new();
        for i in 1..p.len() - 2 {
            let (m0, m1) = ((p[i + 1] - p[i - 1]) * 0.5, (p[i + 2] - p[i]) * 0.5);
            for s in 0..steps {
                let t = s as f64 / steps as f64;
                let (t2, t3) = (t * t, t * t * t);
                out.push(
                    p[i] * (2. * t3 - 3. * t2 + 1.)
                        + m0 * (t3 - 2. * t2 + t)
                        + p[i + 1] * (-2. * t3 + 3. * t2)
                        + m1 * (t3 - t2),
                );
            }
        }
        out.push(p[p.len() - 2]);
        out
    }

    /// Curves with a Start or End Control (and two-point ones) keep the
    /// Catmull-Rom rule USD draws.
    #[test]
    fn catmull_rom_with_made_up_ends_is_the_same_curve() {
        for n in [2usize, 3, 4, 7] {
            for (s, e) in [(false, false), (true, false), (false, true), (true, true)] {
                if !s && !e && n > 2 {
                    continue; // PF-09: written as a Bezier (next test).
                }
                let pts = wiggle(n + usize::from(s) + usize::from(e));
                let kind = CurveKind::Curve {
                    start_control: s,
                    end_control: e,
                };
                let ours = sample(&pts, kind, false, 8);
                let usd = usd_catmull_rom(&catmull_rom_points(&pts, s, e), 8);
                assert!(close(&ours, &usd), "n {n} start {s} end {e}");
            }
        }
    }

    /// PF-09: a smooth Curve's Bezier draws it sample for sample, open and
    /// closed, and reads back as the same Curve.
    #[test]
    fn a_smooth_curve_goes_as_the_bezier_of_its_spans_and_comes_back() {
        let smooth = CurveKind::Curve {
            start_control: false,
            end_control: false,
        };
        for n in [3usize, 4, 7] {
            for closed in [false, true] {
                let pts = wiggle(n);
                let bez = operandi_core::curves::smooth_bezier(&pts, closed).unwrap();
                let ours = sample(&pts, smooth, closed, 36);
                let drawn = sample(&bez, CurveKind::Bezier, closed, 36);
                assert_eq!(ours.len(), drawn.len(), "n {n} closed {closed}");
                for (a, b) in ours.iter().zip(&drawn) {
                    assert!(a.distance(*b) < 1e-6, "n {n} closed {closed}: {a} {b}");
                }
                // Through the reader, with Operandi's kind.
                let counts = [bez.len() as i32];
                let input = CurvesIn {
                    schema: "BasisCurves",
                    ty: "cubic",
                    basis: "bezier",
                    wrap: if closed { "periodic" } else { "nonperiodic" },
                    counts: &counts,
                    points: &bez,
                    order: &[],
                    knots: &[],
                    weights: &[],
                    kinds: Some(vec!["curve".into()]),
                };
                let mut notes = Vec::new();
                let back = line_elements(&input, &mut notes);
                assert!(notes.is_empty(), "{notes:?}");
                assert_eq!(back[0].curve, smooth);
                assert_eq!(back[0].closed, closed);
                assert!(close(&back[0].points, &pts), "n {n} closed {closed}");
            }
        }
    }

    #[test]
    fn a_b_spline_as_bezier_is_the_same_curve() {
        // A clamped B-Spline's own Bezier would differ; an unclamped
        // (USD) one: compare against direct uniform evaluation.
        let p = wiggle(7);
        let bez = b_spline_as_bezier(&p);
        let ours = sample(&bez, CurveKind::Bezier, false, 4);
        let closed = sample(&p, CurveKind::BSpline, true, 4);
        // The closed B-Spline's first n-3 spans are the open, unclamped one.
        assert!(close(&ours[..ours.len() - 1], &closed[..4 * 4]));
    }

    #[test]
    fn clamped_nurbs_is_a_b_spline_and_others_are_sampled() {
        let p = wiggle(6);
        let knots = clamped_knots(6);
        let mut notes = Vec::new();
        let input = CurvesIn {
            schema: "NurbsCurves",
            ty: "",
            basis: "",
            wrap: "",
            counts: &[6],
            points: &p,
            order: &[4],
            knots: &knots,
            weights: &[],
            kinds: None,
        };
        let e = line_elements(&input, &mut notes);
        assert_eq!(e[0].curve, CurveKind::BSpline);
        assert!(notes.is_empty());
        // The sampled path of a weighted one ends on the clamped end points.
        let weights = [1.0, 2.0, 1.0, 1.0, 0.5, 1.0];
        let e = line_elements(
            &CurvesIn {
                weights: &weights,
                ..input
            },
            &mut notes,
        );
        assert_eq!(e[0].curve, CurveKind::Line);
        assert!(e[0].points[0].distance(p[0]) < 1e-9);
        assert!(e[0].points.last().unwrap().distance(p[5]) < 1e-6);
        assert_eq!(notes.len(), 1);
        // Unweighted, the sampled points lie on Operandi's B-Spline.
        let ours = sample(&p, CurveKind::BSpline, false, 6);
        let theirs: Vec<DVec3> = (0..=18)
            .map(|s| nurbs_point(&p, &[], &knots, 4, (s as f64 / 6.).min(3. - 1e-12)))
            .collect();
        assert!(close(&ours[..18], &theirs[..18]));
    }
}
