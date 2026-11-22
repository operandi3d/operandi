//! Curve elements (draw-tools DRAW-00): Curve, B-Spline and Bezier are line
//! elements (`PolygonRepresentation::Polyline`) whose points are control
//! points, drawn as a smooth curve. Everything that treats a polyline as a
//! line (selection, picking on its cage, saving) works on them unchanged.
//!
//! - **Curve** passes through every point: cubic Hermite spans, uniform in
//!   parameter, with chord-weighted tangents (PF-09, `smooth_tangents`);
//!   with Start/End Control or a repeated point, Catmull-Rom with a
//!   "natural" end tangent as before. Start/End Control make the first/last
//!   point a tangent handle only.
//! - **B-Spline**: uniform cubic, clamped to the first and last points
//!   (open) or periodic (closed). Four points give the cubic Bezier on them.
//! - **Bezier**: points stored as a0, out0, in1, a1, out1, …, in_n, a_n;
//!   closed adds out_n and in0 at the end. Spans are cubic.
//!
//! Freezing samples each span with [`FREEZE_STEPS`] segments.
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Segments for each span, by span index.
type Steps<'a> = &'a dyn Fn(usize) -> usize;

/// Segments per span when a curve is drawn or frozen (seen: 36).
pub const FREEZE_STEPS: usize = 36;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurveKind {
    /// Straight spans between the points: an ordinary polyline.
    #[default]
    Line,
    Curve {
        start_control: bool,
        end_control: bool,
    },
    BSpline,
    Bezier,
}
impl CurveKind {
    pub fn is_line(&self) -> bool {
        *self == CurveKind::Line
    }
    /// Fewest control points the curve needs (DRW-Q08: fewer is removed on
    /// Drop). Bezier counts stored points: two anchors with their handles.
    pub fn minimum_points(&self) -> usize {
        match self {
            CurveKind::Line => 2,
            CurveKind::Curve {
                start_control,
                end_control,
            } => 2 + usize::from(*start_control) + usize::from(*end_control),
            CurveKind::BSpline => 4,
            CurveKind::Bezier => 4,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            CurveKind::Line => "Polyline",
            CurveKind::Curve { .. } => "Curve",
            CurveKind::BSpline => "B-Spline",
            CurveKind::Bezier => "Bezier Curve",
        }
    }
}

/// The curve drawn through `points`, `steps` segments per span. A closed
/// curve's result does not repeat its first point. Too few points give the
/// points themselves.
pub fn sample(points: &[DVec3], kind: CurveKind, closed: bool, steps: usize) -> Vec<DVec3> {
    let steps = steps.max(1);
    match kind {
        CurveKind::Line => points.to_vec(),
        CurveKind::Curve {
            start_control,
            end_control,
        } => catmull_rom(points, closed, start_control, end_control, &|_| steps),
        CurveKind::BSpline => b_spline(points, closed, &|_| steps),
        CurveKind::Bezier => bezier(points, closed, &|_| steps),
    }
}

/// The curve drawn through `points` with `steps(i)` segments for span `i`
/// (Freeze with Fixed Spans off, FRZ-06). Same point order as [`sample`].
pub fn sample_spans(
    points: &[DVec3],
    kind: CurveKind,
    closed: bool,
    steps: &dyn Fn(usize) -> usize,
) -> Vec<DVec3> {
    match kind {
        CurveKind::Line => points.to_vec(),
        CurveKind::Curve {
            start_control,
            end_control,
        } => catmull_rom(points, closed, start_control, end_control, steps),
        CurveKind::BSpline => b_spline(points, closed, steps),
        CurveKind::Bezier => bezier(points, closed, steps),
    }
}

/// Spans of the drawn curve (0 when it has too few points to be one).
pub fn span_count(points: &[DVec3], kind: CurveKind, closed: bool) -> usize {
    if kind.is_line() {
        return 0;
    }
    let one = sample_spans(points, kind, closed, &|_| 1);
    let two = sample_spans(points, kind, closed, &|_| 2);
    // Too few points come back as the points themselves, whatever the steps.
    if one.len() == two.len() {
        return 0;
    }
    two.len() - one.len()
}

/// Sub-steps used to measure how far the curve turns inside one segment.
const TURN_PROBE: usize = 8;
/// Most segments one span gets when cut adaptively.
pub const ADAPTIVE_MAX_STEPS: usize = 1024;

/// FRZ-06 (Operandi choice): each span gets the fewest equal-parameter
/// segments such that the curve turns by at most `angle` (radians) inside
/// any segment and from one segment's direction to the next. A straight
/// span gets one segment.
pub fn sample_adaptive(points: &[DVec3], kind: CurveKind, closed: bool, angle: f64) -> Vec<DVec3> {
    let counts = adaptive_counts(points, kind, closed, angle);
    if counts.is_empty() {
        return sample(points, kind, closed, FREEZE_STEPS);
    }
    sample_spans(points, kind, closed, &|i| {
        counts.get(i).copied().unwrap_or(1)
    })
}

/// Segments per span for [`sample_adaptive`] (empty: too few points).
pub fn adaptive_counts(points: &[DVec3], kind: CurveKind, closed: bool, angle: f64) -> Vec<usize> {
    let angle = angle.max(1e-4);
    (0..span_count(points, kind, closed))
        .map(|i| adaptive_steps(points, kind, closed, i, angle))
        .collect()
}

/// Which stored point the curve passes through at the start of each span
/// (and, open, at the end): the samples Freeze reuses (FRZ-04 v2). Bezier
/// anchors and every on-curve Curve point; none for a B-Spline, which
/// doesn't pass through its points.
pub fn on_curve_points(
    len: usize,
    kind: CurveKind,
    closed: bool,
    spans: usize,
) -> Vec<Option<usize>> {
    let ends = spans + usize::from(!closed);
    let first = match kind {
        CurveKind::Curve { start_control, .. } => usize::from(start_control && !closed),
        _ => 0,
    };
    let stride = if kind == CurveKind::Bezier { 3 } else { 1 };
    match kind {
        CurveKind::Line | CurveKind::BSpline => vec![None; ends],
        _ => (0..ends)
            .map(|j| Some(first + stride * j).filter(|&i| i < len))
            .collect(),
    }
}

/// Points of span `i` cut in `n` steps (n + 1 points, both ends included).
fn span_run(points: &[DVec3], kind: CurveKind, closed: bool, i: usize, n: usize) -> Vec<DVec3> {
    let s = sample_spans(points, kind, closed, &|j| if j == i { n } else { 1 });
    // Every span before `i` gave one point, so span `i` starts at `i`.
    (0..=n).map(|k| s[(i + k) % s.len()]).collect()
}

/// Angles between neighbouring chords of a run (zero-length chords skipped).
fn turns(run: &[DVec3]) -> Vec<f64> {
    let dirs: Vec<DVec3> = run
        .windows(2)
        .filter_map(|w| (w[1] - w[0]).try_normalize())
        .collect();
    dirs.windows(2)
        .map(|d| d[0].dot(d[1]).clamp(-1., 1.).acos())
        .collect()
}

fn adaptive_steps(points: &[DVec3], kind: CurveKind, closed: bool, i: usize, angle: f64) -> usize {
    let total: f64 = turns(&span_run(points, kind, closed, i, 256)).iter().sum();
    let mut n = ((total / angle).ceil() as usize).clamp(1, ADAPTIVE_MAX_STEPS);
    loop {
        if n >= ADAPTIVE_MAX_STEPS || fits(points, kind, closed, i, n, angle) {
            return n.min(ADAPTIVE_MAX_STEPS);
        }
        n += (n / 8).max(1);
    }
}

/// `n` segments keep every turn within `angle` (with a little slack for
/// rounding on straight spans).
fn fits(points: &[DVec3], kind: CurveKind, closed: bool, i: usize, n: usize, angle: f64) -> bool {
    let limit = angle + 1e-9;
    let fine = span_run(points, kind, closed, i, n * TURN_PROBE);
    for k in 0..n {
        let inside: f64 = turns(&fine[k * TURN_PROBE..=(k + 1) * TURN_PROBE])
            .iter()
            .sum();
        if inside > limit {
            return false;
        }
    }
    let coarse: Vec<DVec3> = (0..=n).map(|k| fine[k * TURN_PROBE]).collect();
    turns(&coarse).iter().all(|&t| t <= limit)
}

/// RS-10: pieces for a curved span. Radial Sweep cuts by how far a span
/// turns, not by its length, so size never changes the result (RS-03).
pub const SWEEP_CURVED_PIECES: usize = 7;
/// RS-10: the most pieces a sharply bent span gets.
pub const SWEEP_SHARP_PIECES: usize = 10;
/// A span turning more than this (radians) starts getting extra pieces.
const SWEEP_BEND_START: f64 = std::f64::consts::FRAC_PI_4;
/// Each extra piece past [`SWEEP_BEND_START`] covers this much more turn.
const SWEEP_BEND_PER_PIECE: f64 = std::f64::consts::PI / 36.;
/// Relative distance from straight under which a span counts as straight.
const STRAIGHT: f64 = 1e-9;

/// The curve as Radial Sweep cuts it into straight pieces (RS-02, RS-10):
/// [`SWEEP_CURVED_PIECES`] per curved span, up to [`SWEEP_SHARP_PIECES`]
/// for a sharp bend, one per straight span, and straight runs across
/// several points merged into one piece. Same point order as [`sample`].
pub fn sweep_sample(points: &[DVec3], kind: CurveKind, closed: bool) -> Vec<DVec3> {
    let s = FREEZE_STEPS;
    let dense = sample(points, kind, closed, s);
    if kind.is_line() || dense.len() < 2 {
        return dense;
    }
    let spans = if closed {
        dense.len() / s
    } else {
        (dense.len() - 1) / s
    };
    // Too few points come back as the points themselves: no spans to cut.
    if spans == 0 || dense.len() != spans * s + usize::from(!closed) {
        let all = vec![true; dense.len()];
        return merge_straight(dense, &all, closed);
    }
    let pieces: Vec<usize> = (0..spans)
        .map(|i| {
            let run: Vec<DVec3> = (0..=s).map(|k| dense[(i * s + k) % dense.len()]).collect();
            span_pieces(&run)
        })
        .collect();
    let steps = |i: usize| pieces.get(i).copied().unwrap_or(SWEEP_CURVED_PIECES);
    let cut = match kind {
        CurveKind::Line => unreachable!(),
        CurveKind::Curve {
            start_control,
            end_control,
        } => catmull_rom(points, closed, start_control, end_control, &steps),
        CurveKind::BSpline => b_spline(points, closed, &steps),
        CurveKind::Bezier => bezier(points, closed, &steps),
    };
    // Only a point between two straight spans may merge away: inside a
    // curved span, an S-bend's middle can line up with its neighbours.
    let mut between_straight = vec![false; cut.len()];
    let mut at = 0;
    for i in 0..spans {
        let before = if i == 0 {
            closed.then(|| pieces[spans - 1])
        } else {
            Some(pieces[i - 1])
        };
        if at < cut.len() && before == Some(1) && pieces[i] == 1 {
            between_straight[at] = true;
        }
        at += pieces[i];
    }
    merge_straight(cut, &between_straight, closed)
}

/// Pieces for one span, from its densely sampled run of points.
fn span_pieces(run: &[DVec3]) -> usize {
    let (a, b) = (run[0], run[run.len() - 1]);
    let length: f64 = run.windows(2).map(|w| w[0].distance(w[1])).sum();
    if length <= f64::EPSILON * a.length().max(b.length()).max(1.) {
        return 1;
    }
    let chord = b - a;
    let off_line = |p: DVec3| {
        if chord.length_squared() == 0. {
            p.distance(a)
        } else {
            (p - a).cross(chord).length() / chord.length()
        }
    };
    if run.iter().all(|&p| off_line(p) <= STRAIGHT * length) {
        return 1;
    }
    let dirs: Vec<DVec3> = run
        .windows(2)
        .filter_map(|w| (w[1] - w[0]).try_normalize())
        .collect();
    let turn: f64 = dirs
        .windows(2)
        .map(|d| d[0].dot(d[1]).clamp(-1., 1.).acos())
        .sum();
    let extra = ((turn - SWEEP_BEND_START) / SWEEP_BEND_PER_PIECE)
        .ceil()
        .max(0.) as usize;
    (SWEEP_CURVED_PIECES + extra).min(SWEEP_SHARP_PIECES)
}

/// Drops the points marked `may_go` that sit on the straight line through
/// their neighbours, so a straight run is one piece (RS-02). Ends of an
/// open chain always stay.
fn merge_straight(points: Vec<DVec3>, may_go: &[bool], closed: bool) -> Vec<DVec3> {
    let n = points.len();
    let min = if closed { 3 } else { 2 };
    let mut keep = vec![true; n];
    let mut kept = n;
    let neighbour = |keep: &[bool], i: usize, forward: bool| -> Option<usize> {
        let mut j = i;
        for _ in 0..n {
            j = if forward {
                if j + 1 == n {
                    if !closed {
                        return None;
                    }
                    0
                } else {
                    j + 1
                }
            } else if j == 0 {
                if !closed {
                    return None;
                }
                n - 1
            } else {
                j - 1
            };
            if keep[j] && j != i {
                return Some(j);
            }
        }
        None
    };
    for i in 0..n {
        if kept <= min || !may_go.get(i).copied().unwrap_or(false) {
            continue;
        }
        let (Some(a), Some(c)) = (neighbour(&keep, i, false), neighbour(&keep, i, true)) else {
            continue;
        };
        if straight(points[a], points[i], points[c]) {
            keep[i] = false;
            kept -= 1;
        }
    }
    points
        .into_iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(p))
        .collect()
}

/// `b` is on the segment from `a` to `c`, going the same way.
fn straight(a: DVec3, b: DVec3, c: DVec3) -> bool {
    let (u, v) = (b - a, c - b);
    let scale = u.length() + v.length();
    scale > 0. && u.cross(v).length() <= STRAIGHT * scale * scale && u.dot(v) > 0.
}

/// PF-09: a smooth Curve with no Start/End Control. Inner tangents weigh
/// each neighbour chord by the other's length, m = (|b|·a + |a|·b) /
/// (|a| + |b|) with a = Pi − Pi−1, b = Pi+1 − Pi (chords wrap when
/// closed). An open end mirrors the first chord past the second: q = c0 +
/// max(L0, L1)·ĉ0 − c1, then m0 = (|c0|·q + |q|·c0) / (|q| + |c0|), the last
/// end the same from the other side, negated. Tangents are per unit of
/// the span's parameter. `None` (keep the older rule) for fewer than three
/// points or a zero-length chord.
fn smooth_tangents(points: &[DVec3], closed: bool) -> Option<Vec<DVec3>> {
    let n = points.len();
    if n < 3 {
        return None;
    }
    let blend = |a: DVec3, b: DVec3| {
        let (la, lb) = (a.length(), b.length());
        (a * lb + b * la) / (la + lb)
    };
    let chord_ok = |a: DVec3| a.length() > 1e-12;
    let at = |i: isize| points[i.rem_euclid(n as isize) as usize];
    if closed {
        let mut out = Vec::with_capacity(n);
        for i in 0..n as isize {
            let (a, b) = (at(i) - at(i - 1), at(i + 1) - at(i));
            if !chord_ok(a) || !chord_ok(b) {
                return None;
            }
            out.push(blend(a, b));
        }
        return Some(out);
    }
    let end = |p0: DVec3, p1: DVec3, p2: DVec3| -> Option<DVec3> {
        let (c0, c1) = (p1 - p0, p2 - p1);
        if !chord_ok(c0) || !chord_ok(c1) {
            return None;
        }
        let q = c0 + c0 / c0.length() * c0.length().max(c1.length()) - c1;
        if !chord_ok(q) {
            return None;
        }
        Some(blend(q, c0))
    };
    let mut out = vec![DVec3::ZERO; n];
    out[0] = end(points[0], points[1], points[2])?;
    out[n - 1] = -end(points[n - 1], points[n - 2], points[n - 3])?;
    for i in 1..n - 1 {
        let (a, b) = (points[i] - points[i - 1], points[i + 1] - points[i]);
        if !chord_ok(a) || !chord_ok(b) {
            return None;
        }
        out[i] = blend(a, b);
    }
    Some(out)
}

/// A smooth Curve with no Start/End Control as the cubic Bezier that draws
/// the same spans (PF-09): each Hermite span Pi..Pi+1 has the handles
/// Pi + mi/3 and Pi+1 − mi+1/3. Stored in [`CurveKind::Bezier`]'s order
/// (anchor, out, in, anchor...; closed ends with the last out and the
/// first in). `None` where the Curve keeps the older rule.
pub fn smooth_bezier(points: &[DVec3], closed: bool) -> Option<Vec<DVec3>> {
    let m = smooth_tangents(points, closed)?;
    let n = points.len();
    let spans = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(3 * spans + 1);
    for i in 0..spans {
        let j = (i + 1) % n;
        out.push(points[i]);
        out.push(points[i] + m[i] / 3.);
        out.push(points[j] - m[j] / 3.);
    }
    if !closed {
        out.push(points[n - 1]);
    }
    Some(out)
}

fn hermite(p0: DVec3, m0: DVec3, p1: DVec3, m1: DVec3, t: f64) -> DVec3 {
    let (t2, t3) = (t * t, t * t * t);
    p0 * (2. * t3 - 3. * t2 + 1.)
        + m0 * (t3 - 2. * t2 + t)
        + p1 * (-2. * t3 + 3. * t2)
        + m1 * (t3 - t2)
}

fn catmull_rom(
    points: &[DVec3],
    closed: bool,
    start_control: bool,
    end_control: bool,
    steps: Steps,
) -> Vec<DVec3> {
    let n = points.len();
    if closed {
        if n < 3 {
            return points.to_vec();
        }
        let at = |i: isize| points[i.rem_euclid(n as isize) as usize];
        let smooth = smooth_tangents(points, true);
        let tangent = |i: isize| match &smooth {
            Some(m) => m[i.rem_euclid(n as isize) as usize],
            None => (at(i + 1) - at(i - 1)) * 0.5,
        };
        let mut out = Vec::with_capacity(n * steps(0));
        for i in 0..n as isize {
            let m0 = tangent(i);
            let m1 = tangent(i + 1);
            let steps = steps(i as usize).max(1);
            for s in 0..steps {
                out.push(hermite(at(i), m0, at(i + 1), m1, s as f64 / steps as f64));
            }
        }
        return out;
    }
    // Control ends shape the tangent but aren't on the curve.
    let (lo, hi) = (
        usize::from(start_control),
        n - usize::from(end_control).min(n),
    );
    if hi <= lo + 1 {
        return points[lo.min(n)..hi.max(lo).min(n)].to_vec();
    }
    let on = &points[lo..hi];
    let m = on.len();
    // Inner tangents are Catmull-Rom; ends use the control point when there
    // is one, otherwise the natural end (zero curvature).
    let mut tangents = vec![DVec3::ZERO; m];
    for i in 1..m - 1 {
        tangents[i] = (on[i + 1] - on[i - 1]) * 0.5;
    }
    if m == 2 {
        tangents[0] = on[1] - on[0];
        tangents[1] = on[1] - on[0];
    }
    if start_control {
        tangents[0] = (on[1] - points[0]) * 0.5;
    } else if m > 2 {
        tangents[0] = ((on[1] - on[0]) * 3. - tangents[1]) * 0.5;
    }
    if end_control {
        tangents[m - 1] = (points[n - 1] - on[m - 2]) * 0.5;
    } else if m > 2 {
        tangents[m - 1] = ((on[m - 1] - on[m - 2]) * 3. - tangents[m - 2]) * 0.5;
    }
    if !start_control
        && !end_control
        && let Some(smooth) = smooth_tangents(on, false)
    {
        tangents = smooth;
    }
    let mut out = Vec::with_capacity((m - 1) * steps(0) + 1);
    for i in 0..m - 1 {
        let steps = steps(i).max(1);
        for s in 0..steps {
            out.push(hermite(
                on[i],
                tangents[i],
                on[i + 1],
                tangents[i + 1],
                s as f64 / steps as f64,
            ));
        }
    }
    out.push(on[m - 1]);
    out
}

/// Uniform cubic B-spline basis on four control points.
fn b_span(p: [DVec3; 4], t: f64) -> DVec3 {
    let u = 1. - t;
    (p[0] * (u * u * u)
        + p[1] * (3. * t * t * t - 6. * t * t + 4.)
        + p[2] * (-3. * t * t * t + 3. * t * t + 3. * t + 1.)
        + p[3] * (t * t * t))
        / 6.
}

fn de_boor(knots: &[f64], ctrl: &[DVec3], t: f64) -> DVec3 {
    let p = 3;
    let n = ctrl.len();
    let mut k = p;
    while k + 1 < n && t >= knots[k + 1] {
        k += 1;
    }
    let mut d: Vec<DVec3> = (0..=p).map(|j| ctrl[j + k - p]).collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = j + k - p;
            let denom = knots[i + p + 1 - r] - knots[i];
            let a = if denom.abs() < 1e-15 {
                0.
            } else {
                (t - knots[i]) / denom
            };
            d[j] = d[j - 1] * (1. - a) + d[j] * a;
        }
    }
    d[p]
}

fn b_spline(points: &[DVec3], closed: bool, steps: Steps) -> Vec<DVec3> {
    let n = points.len();
    if closed {
        if n < 3 {
            return points.to_vec();
        }
        let at = |i: usize| points[i % n];
        let mut out = Vec::with_capacity(n * steps(0));
        for i in 0..n {
            let span = [at(i), at(i + 1), at(i + 2), at(i + 3)];
            let steps = steps(i).max(1);
            for s in 0..steps {
                out.push(b_span(span, s as f64 / steps as f64));
            }
        }
        return out;
    }
    if n < 4 {
        return points.to_vec();
    }
    // Clamped uniform knots: 0,0,0,0,1,2,…,k,k,k,k.
    let spans = n - 3;
    let mut knots = vec![0.; 4];
    knots.extend((1..spans).map(|i| i as f64));
    knots.extend([spans as f64; 4]);
    let mut out = Vec::with_capacity(spans * steps(0) + 1);
    for i in 0..spans {
        let steps = steps(i).max(1);
        for s in 0..steps {
            out.push(de_boor(&knots, points, i as f64 + s as f64 / steps as f64));
        }
    }
    out.push(points[n - 1]);
    out
}

fn cubic(p: [DVec3; 4], t: f64) -> DVec3 {
    let u = 1. - t;
    p[0] * (u * u * u) + p[1] * (3. * u * u * t) + p[2] * (3. * u * t * t) + p[3] * (t * t * t)
}

/// A Bezier curve's anchors, each as (in handle, anchor, out handle). Open
/// ends get a handle equal to their anchor on the missing side.
pub fn bezier_anchors(points: &[DVec3], closed: bool) -> Vec<[DVec3; 3]> {
    if closed {
        // a0, out0, (in, a, out)…, in_n, a_n, out_n, in0
        let n = points.len() / 3;
        if n == 0 {
            return Vec::new();
        }
        return (0..n)
            .map(|i| {
                let a = points[3 * i];
                let out = points[3 * i + 1];
                let inn = if i == 0 {
                    points[3 * n - 1]
                } else {
                    points[3 * i - 1]
                };
                [inn, a, out]
            })
            .collect();
    }
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() < 4 {
        return points.iter().map(|&p| [p, p, p]).collect();
    }
    let n = points.len().div_ceil(3);
    (0..n)
        .map(|i| {
            let a = points[3 * i];
            let inn = if i == 0 { a } else { points[3 * i - 1] };
            let out = if i == n - 1 { a } else { points[3 * i + 1] };
            [inn, a, out]
        })
        .collect()
}

/// The stored point list for `anchors` (the inverse of [`bezier_anchors`]).
pub fn bezier_points(anchors: &[[DVec3; 3]], closed: bool) -> Vec<DVec3> {
    let n = anchors.len();
    let mut out = Vec::with_capacity(3 * n);
    for (i, [inn, a, o]) in anchors.iter().enumerate() {
        if i > 0 {
            out.push(*inn);
        }
        out.push(*a);
        if i + 1 < n || closed {
            out.push(*o);
        }
    }
    if closed && n > 0 {
        out.push(anchors[0][0]);
    }
    out
}

fn bezier(points: &[DVec3], closed: bool, steps: Steps) -> Vec<DVec3> {
    let anchors = bezier_anchors(points, closed);
    let n = anchors.len();
    if n < 2 {
        return anchors.iter().map(|a| a[1]).collect();
    }
    let spans = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(spans * steps(0) + 1);
    for i in 0..spans {
        let (a, b) = (anchors[i], anchors[(i + 1) % n]);
        let steps = steps(i).max(1);
        for s in 0..steps {
            out.push(cubic([a[1], a[2], b[0], b[1]], s as f64 / steps as f64));
        }
    }
    if !closed {
        out.push(anchors[n - 1][1]);
    }
    out
}

/// The point at global parameter `u` (span `floor(u)`, `frac(u)` along
/// it; `0..=span_count`) on the drawn curve: the same curve [`sample`]
/// draws, evaluated anywhere (the path tools place sections between the
/// sampled points, path_tools.rs). `None` when the points make no span.
pub fn point_at(points: &[DVec3], kind: CurveKind, closed: bool, u: f64) -> Option<DVec3> {
    let spans = span_count(points, kind, closed);
    if spans == 0 || !u.is_finite() {
        return None;
    }
    let u = u.clamp(0., spans as f64);
    let i = (u.floor() as usize).min(spans - 1);
    let t = u - i as f64;
    match kind {
        CurveKind::Line => None,
        CurveKind::Curve {
            start_control,
            end_control,
        } => {
            let n = points.len();
            if closed {
                let at = |j: isize| points[j.rem_euclid(n as isize) as usize];
                let i = i as isize;
                if let Some(m) = smooth_tangents(points, true) {
                    let k = |j: isize| m[j.rem_euclid(n as isize) as usize];
                    return Some(hermite(at(i), k(i), at(i + 1), k(i + 1), t));
                }
                let m0 = (at(i + 1) - at(i - 1)) * 0.5;
                let m1 = (at(i + 2) - at(i)) * 0.5;
                return Some(hermite(at(i), m0, at(i + 1), m1, t));
            }
            // The same tangents as `catmull_rom`'s open case.
            let lo = usize::from(start_control);
            let hi = n - usize::from(end_control);
            let on = &points[lo..hi];
            let m = on.len();
            let inner = |j: usize| (on[j + 1] - on[j - 1]) * 0.5;
            let tangent = |j: usize| -> DVec3 {
                if m == 2 {
                    return on[1] - on[0];
                }
                if j == 0 {
                    if start_control {
                        (on[1] - points[0]) * 0.5
                    } else {
                        ((on[1] - on[0]) * 3. - inner(1)) * 0.5
                    }
                } else if j == m - 1 {
                    if end_control {
                        (points[n - 1] - on[m - 2]) * 0.5
                    } else {
                        ((on[m - 1] - on[m - 2]) * 3. - inner(m - 2)) * 0.5
                    }
                } else {
                    inner(j)
                }
            };
            if !start_control
                && !end_control
                && let Some(m) = smooth_tangents(on, false)
            {
                return Some(hermite(on[i], m[i], on[i + 1], m[i + 1], t));
            }
            Some(hermite(on[i], tangent(i), on[i + 1], tangent(i + 1), t))
        }
        CurveKind::BSpline => {
            let n = points.len();
            if closed {
                let at = |j: usize| points[j % n];
                return Some(b_span([at(i), at(i + 1), at(i + 2), at(i + 3)], t));
            }
            if u >= spans as f64 {
                return Some(points[n - 1]);
            }
            let mut knots = vec![0.; 4];
            knots.extend((1..spans).map(|k| k as f64));
            knots.extend([spans as f64; 4]);
            Some(de_boor(&knots, points, u))
        }
        CurveKind::Bezier => {
            let anchors = bezier_anchors(points, closed);
            let (a, b) = (anchors[i], anchors[(i + 1) % anchors.len()]);
            Some(cubic([a[1], a[2], b[0], b[1]], t))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn oracle() -> Vec<DVec3> {
        vec![
            DVec3::ZERO,
            DVec3::new(1., 1., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(3., 1., 0.),
        ]
    }
    #[test]
    fn b_spline_on_four_points_is_their_cubic_bezier() {
        let s = sample(&oracle(), CurveKind::BSpline, false, 36);
        assert!(
            (s[18] - DVec3::new(1.5, 0.5, 0.)).length() < 1e-9,
            "{}",
            s[18]
        );
        assert_eq!(s[0], DVec3::ZERO);
        assert_eq!(*s.last().unwrap(), DVec3::new(3., 1., 0.));
    }
    #[test]
    fn curve_passes_through_every_point() {
        let p = oracle();
        let kind = CurveKind::Curve {
            start_control: false,
            end_control: false,
        };
        let s = sample(&p, kind, false, 36);
        assert_eq!(s.len(), 3 * 36 + 1);
        for (i, q) in p.iter().enumerate() {
            assert!((s[i * 36] - *q).length() < 1e-12);
        }
        // Span 1 midpoint as seen (PF-09).
        assert!((s[18].x - 0.5).abs() < 1e-9);
        assert!((s[18].y - 0.702254).abs() < 1e-6, "{}", s[18]);
    }
    #[test]
    fn bezier_storage_round_trips() {
        let a = vec![
            [DVec3::ZERO, DVec3::ZERO, DVec3::X],
            [DVec3::Y, DVec3::new(1., 1., 0.), DVec3::new(2., 1., 0.)],
            [
                DVec3::new(2., 2., 0.),
                DVec3::new(3., 2., 0.),
                DVec3::new(3., 2., 0.),
            ],
        ];
        let open = bezier_points(&a, false);
        assert_eq!(open.len(), 7);
        assert_eq!(bezier_anchors(&open, false), a);
        let closed = bezier_points(&a, true);
        assert_eq!(closed.len(), 9);
        assert_eq!(bezier_anchors(&closed, true), a);
        // Four anchors make ten stored points (seen).
        let mut four = a.clone();
        four.push([DVec3::Z, DVec3::Z, DVec3::Z]);
        assert_eq!(bezier_points(&four, false).len(), 10);
    }

    const CURVE: CurveKind = CurveKind::Curve {
        start_control: false,
        end_control: false,
    };
    /// Points on an arc in the XY plane round (cx, 0), `turn` degrees.
    fn arc(points: usize, cx: f64, r: f64, turn: f64) -> Vec<DVec3> {
        (0..points)
            .map(|i| {
                let a = (-90. + turn * i as f64 / (points - 1) as f64).to_radians();
                DVec3::new(cx + r * a.cos(), r * a.sin(), 0.)
            })
            .collect()
    }
    fn pieces(points: &[DVec3], kind: CurveKind, closed: bool) -> usize {
        let s = sweep_sample(points, kind, closed);
        s.len() - usize::from(!closed)
    }
    #[test]
    fn sweep_cuts_curved_spans_in_seven() {
        // RS-06 table: half circle 9 and 17 points, quarter circle 3.
        assert_eq!(pieces(&arc(9, 2., 1., 180.), CURVE, false), 56);
        assert_eq!(pieces(&arc(17, 2., 1., 180.), CURVE, false), 112);
        assert_eq!(pieces(&arc(3, 2., 1., 90.), CURVE, false), 14);
    }
    #[test]
    fn sweep_cut_ignores_size() {
        // RS-03.
        let small = arc(9, 2., 1., 180.);
        let big: Vec<DVec3> = small.iter().map(|&p| p * 10.).collect();
        let tiny: Vec<DVec3> = small.iter().map(|&p| p * 0.001).collect();
        for p in [big, tiny] {
            assert_eq!(pieces(&p, CURVE, false), 56);
        }
    }
    #[test]
    fn sweep_straight_runs_are_one_piece() {
        // RS-06: a 3 or 6 point straight curve is one piece.
        for n in [3, 6] {
            let line: Vec<DVec3> = (0..n).map(|i| DVec3::new(1., i as f64 * 0.3, 0.)).collect();
            let s = sweep_sample(&line, CURVE, false);
            assert_eq!(s, vec![line[0], line[n - 1]]);
        }
    }
    #[test]
    fn sweep_sharp_bends_get_more_pieces() {
        // RS-06: a 5 point zig-zag gets 10 a span.
        let zig: Vec<DVec3> = (0..5)
            .map(|i| DVec3::new(1. + 2. * (i % 2) as f64, i as f64, 0.))
            .collect();
        assert_eq!(pieces(&zig, CURVE, false), 40);
    }
    #[test]
    fn sweep_keeps_the_ends_and_order() {
        let p = arc(9, 2., 1., 180.);
        for kind in [CURVE, CurveKind::BSpline] {
            let s = sweep_sample(&p, kind, false);
            assert_eq!(s[0], p[0]);
            assert_eq!(*s.last().unwrap(), p[8]);
        }
        // Closed: a ring of 8 curved spans, no repeated first point.
        let ring: Vec<DVec3> = (0..8)
            .map(|i| {
                let a = (i as f64 * 45.).to_radians();
                DVec3::new(2. + a.cos(), a.sin(), 0.)
            })
            .collect();
        assert_eq!(sweep_sample(&ring, CURVE, true).len(), 56);
    }
    #[test]
    fn sweep_merges_only_straight_runs() {
        // Curve points (1,1)..(1,3) make two straight spans in a row: one
        // piece. An S-bend's straight-looking middle stays (see zig-zag).
        let p = vec![
            DVec3::new(2., -1., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(1., 2., 0.),
            DVec3::new(1., 3., 0.),
            DVec3::new(1., 4., 0.),
            DVec3::new(2., 5., 0.),
        ];
        let s = sweep_sample(&p, CURVE, false);
        assert!(s.contains(&p[2]) && s.contains(&p[4]));
        assert!(!s.contains(&p[3]));
        // 7 + 7 curved spans either side, 1 straight piece, 7 + 7.
        assert_eq!(s.len() - 1, 7 * 4 + 1);
    }
}
