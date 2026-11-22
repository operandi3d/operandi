//! Curve Boolean (CURVE-BOOLEAN v1, CB-xx): Slice, Union, Intersect and
//! Subtract of the selected curves with the driver curves, in the curves'
//! plane. Only the selected curves change (CB-10); Union, Intersect and
//! Subtract results run the other way round (CB-15); Clip keeps only the
//! selected curve's own pieces, open (CB-16); Merge Curves off leaves the
//! pieces unjoined (CB-17).
//!
//! The method: every selected curve and every driver is cut where they
//! cross; each piece is inside or outside the other side (a point test on
//! its middle); the operation picks pieces (Union: both outsides;
//! Intersect: both insides; Subtract: the selected outside plus the driver
//! inside, turned round); Merge chains the pieces end to end into closed
//! outlines.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md CB-Q*): curves are cut as
//! their control polygons (straight segments) and the results are
//! polylines; open curves, Close / Trim, Self Intersection and the
//! Constraint Target aren't built; edges lying along each other aren't
//! split specially.
use crate::{Mesh, PolygonRepresentation, Result};
use glam::{DVec2, DVec3};

/// CB-01's Operation, in its order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CurveBooleanOp {
    #[default]
    Slice,
    Union,
    Intersect,
    Subtract,
}
impl CurveBooleanOp {
    pub const NAMES: [&'static str; 4] = ["Slice", "Union", "Intersect", "Subtract"];
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => Self::Union,
            2 => Self::Intersect,
            3 => Self::Subtract,
            _ => Self::Slice,
        }
    }
    pub fn named(word: &str) -> Result<Self> {
        match word.to_ascii_lowercase().as_str() {
            "slice" | "0" => Ok(Self::Slice),
            "union" | "1" => Ok(Self::Union),
            "intersect" | "2" => Ok(Self::Intersect),
            "subtract" | "3" => Ok(Self::Subtract),
            other => Err(format!("Curve Boolean has no operation {other}")),
        }
    }
}

/// CB-01 options (the button's values: Merge Curves on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurveBooleanOptions {
    pub op: CurveBooleanOp,
    /// 0 Auto, 1 X, 2 Y, 3 Z: the plane's normal.
    pub axis: u8,
    pub clip: bool,
    pub merge: bool,
}
impl Default for CurveBooleanOptions {
    fn default() -> Self {
        Self {
            op: CurveBooleanOp::Slice,
            axis: 0,
            clip: false,
            merge: true,
        }
    }
}

/// A closed loop of points, 3D with its 2D place in the cutting plane.
#[derive(Clone, Debug)]
struct Loop {
    p3: Vec<DVec3>,
    p2: Vec<DVec2>,
}

/// One cut piece: a straight segment of a selected curve or a driver.
#[derive(Clone, Copy, Debug)]
struct Piece {
    a3: DVec3,
    b3: DVec3,
    a2: DVec2,
    b2: DVec2,
}

const EPS: f64 = 1e-9;

fn signed_area(p: &[DVec2]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i].perp_dot(p[(i + 1) % n])).sum::<f64>() / 2.
}

/// Even-odd point in polygon.
fn inside(p: DVec2, poly: &[DVec2]) -> bool {
    let n = poly.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                c = !c;
            }
        }
    }
    c
}

/// Parameters along `a`–`b` where it crosses `c`–`d` (proper or touching).
fn cross(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> Option<(f64, f64)> {
    let r = b - a;
    let s = d - c;
    let den = r.perp_dot(s);
    if den.abs() < EPS {
        return None;
    }
    let t = (c - a).perp_dot(s) / den;
    let u = (c - a).perp_dot(r) / den;
    ((-EPS..=1. + EPS).contains(&t) && (-EPS..=1. + EPS).contains(&u))
        .then_some((t.clamp(0., 1.), u.clamp(0., 1.)))
}

/// `l`'s edges cut at every crossing with `others`, as pieces in order.
fn cut(l: &Loop, others: &[Loop]) -> Vec<Piece> {
    let n = l.p2.len();
    let mut out = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let (a2, b2, a3, b3) = (l.p2[i], l.p2[j], l.p3[i], l.p3[j]);
        let mut ts = vec![0., 1.];
        for o in others {
            let m = o.p2.len();
            for k in 0..m {
                if let Some((t, _)) = cross(a2, b2, o.p2[k], o.p2[(k + 1) % m]) {
                    ts.push(t);
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        for w in ts.windows(2) {
            out.push(Piece {
                a3: a3.lerp(b3, w[0]),
                b3: a3.lerp(b3, w[1]),
                a2: a2.lerp(b2, w[0]),
                b2: a2.lerp(b2, w[1]),
            });
        }
    }
    out
}

fn inside_any(p: DVec2, loops: &[Loop]) -> bool {
    loops.iter().any(|l| inside(p, &l.p2))
}

fn reversed(p: Piece) -> Piece {
    Piece {
        a3: p.b3,
        b3: p.a3,
        a2: p.b2,
        b2: p.a2,
    }
}

/// Runs of pieces from one source that follow each other in order, each
/// as a chain of points (open). `wrap`: the run may continue round the
/// end of the loop.
fn runs(pieces: &[Piece]) -> Vec<Vec<Piece>> {
    let mut out: Vec<Vec<Piece>> = Vec::new();
    for &p in pieces {
        match out.last_mut() {
            Some(run) if run.last().unwrap().b2.distance(p.a2) < 1e-7 => run.push(p),
            _ => out.push(vec![p]),
        }
    }
    // Join the last run onto the first when the loop wraps round.
    if out.len() > 1 {
        let first_start = out[0][0].a2;
        let last_end = out.last().unwrap().last().unwrap().b2;
        if last_end.distance(first_start) < 1e-7 {
            let mut last = out.pop().unwrap();
            last.append(&mut out[0]);
            out[0] = last;
        }
    }
    out
}

/// A run's points, its first piece's start to its last piece's end.
fn chain_points(run: &[Piece]) -> Vec<DVec3> {
    let mut pts = vec![run[0].a3];
    pts.extend(run.iter().map(|p| p.b3));
    pts
}

/// Join chains end to start into closed loops where they meet; chains
/// that don't close stay open. Returns (points, closed).
fn join(mut chains: Vec<(Vec<DVec3>, Vec<DVec2>)>) -> Vec<(Vec<DVec3>, bool)> {
    let mut out = Vec::new();
    while let Some((mut p3, mut p2)) = chains.pop() {
        loop {
            if p2.len() > 2 && p2[0].distance(*p2.last().unwrap()) < 1e-7 {
                p3.pop();
                out.push((p3, true));
                break;
            }
            let end = *p2.last().unwrap();
            let Some(k) = chains.iter().position(|c| c.1[0].distance(end) < 1e-7) else {
                out.push((p3, false));
                break;
            };
            let (n3, n2) = chains.remove(k);
            p3.extend_from_slice(&n3[1..]);
            p2.extend_from_slice(&n2[1..]);
        }
    }
    out
}

/// Drop points that sit on a straight line between their neighbours.
fn tidy(points: Vec<DVec3>, closed: bool) -> Vec<DVec3> {
    let n = points.len();
    if n < 3 {
        return points;
    }
    let keep = |i: usize| -> bool {
        if !closed && (i == 0 || i == n - 1) {
            return true;
        }
        let (a, b, c) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
        (b - a).cross(c - b).length() > 1e-9 * (b - a).length().max(1.) || (b - a).dot(c - b) < 0.
    };
    (0..n).filter(|&i| keep(i)).map(|i| points[i]).collect()
}

/// The 2D projection dropping the normal's axis.
fn project(p: DVec3, axis: usize) -> DVec2 {
    match axis {
        0 => DVec2::new(p.y, p.z),
        1 => DVec2::new(p.z, p.x),
        _ => DVec2::new(p.x, p.y),
    }
}

fn loop_of(mesh: &Mesh, f: usize, axis: usize) -> Loop {
    let p3: Vec<DVec3> = mesh.faces[f]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .collect();
    let p2 = p3.iter().map(|&p| project(p, axis)).collect();
    Loop { p3, p2 }
}

/// Is element `f` a closed curve (any kind)?
fn closed_curve(mesh: &Mesh, f: usize) -> bool {
    matches!(
        mesh.attributes.representations.get(f),
        Some(PolygonRepresentation::Polyline { closed: true, .. })
    ) && mesh.faces[f].vertices.len() >= 3
}

/// The result curves of one selected loop against the drivers:
/// (points, closed).
fn boolean_loop(sel: &Loop, drivers: &[Loop], o: &CurveBooleanOptions) -> Vec<(Vec<DVec3>, bool)> {
    let a_pieces = cut(sel, drivers);
    if o.op == CurveBooleanOp::Slice {
        let mut pts: Vec<DVec3> = a_pieces.iter().map(|p| p.a3).collect();
        pts.dedup_by(|x, y| x.distance(*y) < 1e-9);
        return vec![(pts, true)];
    }
    // Drivers run the same way round as the selected curve.
    let sel_ccw = signed_area(&sel.p2) > 0.;
    let drivers: Vec<Loop> = drivers
        .iter()
        .map(|d| {
            if (signed_area(&d.p2) > 0.) == sel_ccw {
                d.clone()
            } else {
                let mut d = d.clone();
                d.p2.reverse();
                d.p3.reverse();
                d
            }
        })
        .collect();
    let one = [sel.clone()];
    let mid = |p: &Piece| (p.a2 + p.b2) / 2.;
    let mut kept_a: Vec<Piece> = Vec::new();
    let mut kept_d: Vec<Vec<Piece>> = Vec::new();
    let want_a_inside = o.op == CurveBooleanOp::Intersect;
    for p in &a_pieces {
        if inside_any(mid(p), &drivers) == want_a_inside {
            kept_a.push(*p);
        }
    }
    if !o.clip {
        for (k, d) in drivers.iter().enumerate() {
            let others: Vec<Loop> = drivers
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != k)
                .map(|(_, l)| l.clone())
                .chain(one.iter().cloned())
                .collect();
            let pieces = cut(d, &others);
            let mut kept: Vec<Piece> = pieces
                .into_iter()
                .filter(|p| {
                    let m = mid(p);
                    let in_sel = inside(m, &sel.p2);
                    // Pieces inside another driver drop out of a union of
                    // drivers.
                    let in_other = drivers
                        .iter()
                        .enumerate()
                        .any(|(j, l)| j != k && inside(m, &l.p2));
                    match o.op {
                        CurveBooleanOp::Union => !in_sel && !in_other,
                        CurveBooleanOp::Intersect => in_sel,
                        CurveBooleanOp::Subtract => in_sel && !in_other,
                        CurveBooleanOp::Slice => false,
                    }
                })
                .collect();
            if o.op == CurveBooleanOp::Subtract {
                kept.reverse();
                kept = kept.into_iter().map(reversed).collect();
            }
            kept_d.push(kept);
        }
    }
    let mut chains: Vec<(Vec<DVec3>, Vec<DVec2>)> = Vec::new();
    let mut push_runs = |pieces: &[Piece]| {
        for run in runs(pieces) {
            let p3 = chain_points(&run);
            let mut p2 = vec![run[0].a2];
            p2.extend(run.iter().map(|p| p.b2));
            chains.push((p3, p2));
        }
    };
    push_runs(&kept_a);
    for d in &kept_d {
        push_runs(d);
    }
    let pieces: Vec<(Vec<DVec3>, bool)> = if o.merge && !o.clip {
        join(chains)
    } else {
        chains
            .into_iter()
            .map(|(p3, p2)| {
                let closed = p2.len() > 2 && p2[0].distance(*p2.last().unwrap()) < 1e-7;
                let mut p3 = p3;
                if closed {
                    p3.pop();
                }
                (p3, closed)
            })
            .collect()
    };
    // CB-15: the results run the other way round.
    pieces
        .into_iter()
        .map(|(mut p, closed)| {
            p.reverse();
            (tidy_keep(p, closed), closed)
        })
        .filter(|(p, _)| p.len() >= 2)
        .collect()
}

fn tidy_keep(p: Vec<DVec3>, closed: bool) -> Vec<DVec3> {
    // Slice keeps every inserted point; the other results drop points left
    // in the middle of a straight side (none in the spec's cases).
    tidy(p, closed)
}

/// The best-fit plane's normal axis for `loops` (Auto) or the chosen one.
fn plane_axis(mesh: &Mesh, faces: &[usize], axis: u8) -> usize {
    if (1..=3).contains(&axis) {
        return (axis - 1) as usize;
    }
    let mut n = DVec3::ZERO;
    for &f in faces {
        let p: Vec<DVec3> = mesh.faces[f]
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let m = p.len();
        for i in 0..m {
            let (a, b) = (p[i], p[(i + 1) % m]);
            n += DVec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            )
            .abs();
        }
    }
    if n.x >= n.y && n.x >= n.z {
        0
    } else if n.y >= n.z {
        1
    } else {
        2
    }
}

/// CB-10..19: the selected closed curves cut against the unselected
/// closed curves (or, with none, against the other selected curves, which
/// are used up: CB-18). Returns the new mesh.
pub fn curve_boolean(mesh: &Mesh, selected: &[u32], o: &CurveBooleanOptions) -> Result<Mesh> {
    let mut sel: Vec<usize> = selected
        .iter()
        .map(|&f| f as usize)
        .filter(|&f| f < mesh.faces.len() && closed_curve(mesh, f))
        .collect();
    sel.sort_unstable();
    sel.dedup();
    if sel.is_empty() {
        return Err("Curve Boolean: select closed curves (Polygon mode)".into());
    }
    let mut drivers: Vec<usize> = (0..mesh.faces.len())
        .filter(|f| !sel.contains(f) && closed_curve(mesh, *f))
        .collect();
    let mut used_up = Vec::new();
    if drivers.is_empty() {
        if sel.len() < 2 {
            return Err("Curve Boolean: there is no other curve to cut with".into());
        }
        // CB-18: the first selected curve against the others.
        drivers = sel.split_off(1);
        if o.op != CurveBooleanOp::Slice {
            used_up = drivers.clone();
        }
    }
    let all: Vec<usize> = sel.iter().chain(&drivers).copied().collect();
    let axis = plane_axis(mesh, &all, o.axis);
    let driver_loops: Vec<Loop> = drivers.iter().map(|&f| loop_of(mesh, f, axis)).collect();
    let mut out = mesh.clone();
    let mut remove: std::collections::BTreeSet<usize> = used_up.into_iter().collect();
    for &f in &sel {
        let results = boolean_loop(&loop_of(mesh, f, axis), &driver_loops, o);
        remove.insert(f);
        for (pts, closed) in results {
            let first = out.vertices.len() as u32;
            out.vertices.extend(pts.iter().copied());
            let ids: Vec<u32> = (first..out.vertices.len() as u32).collect();
            crate::vertex_commands::append_face(&mut out, ids, Some(f));
            let reps = &mut out.attributes.representations;
            reps.resize(out.faces.len() - 1, PolygonRepresentation::Face);
            reps.push(PolygonRepresentation::Polyline {
                closed,
                curve: crate::CurveKind::Line,
            });
        }
    }
    out.remove_faces(&remove)?;
    // Points only the removed curves used go too; points that were loose
    // before stay.
    let used_before = mesh.used_points();
    let used = out.used_points();
    let keep: Vec<usize> = (0..out.vertices.len())
        .filter(|&v| used[v] || (v < mesh.vertices.len() && !used_before[v]))
        .collect();
    if keep.len() < out.vertices.len() {
        out = crate::mesh_cleanup::remap_points(&out, &keep)?;
    }
    out.validate()?;
    Ok(out)
}

impl crate::Editor {
    /// Curve Boolean on the selected curves, one undo step; nothing is
    /// selected after (CB-19).
    pub(crate) fn curve_boolean_command(&mut self, o: CurveBooleanOptions) -> Result<()> {
        let item = self.edge_insert_target("Curve Boolean")?;
        if self.scene.selection.mode != crate::Mode::Polygon {
            return Err("Curve Boolean: select closed curves (Polygon mode)".into());
        }
        let selected = self.scene.selection.faces.clone();
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = curve_boolean(mesh, &selected, &o)?;
        self.publish_polygon_edit(item, out, "Curve Boolean", Vec::new())
    }
}
