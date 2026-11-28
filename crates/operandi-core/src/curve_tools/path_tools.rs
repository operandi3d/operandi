//! The path tools (modelling-parity spec CURVE-EXTRUDE-PEN-CLONE v1, CX-xx;
//! SPEC-CURVE-CLONE v1): one path generator feeding a Sweep or a Clone
//! effector (CX-01). Curve Extrude, Bezier Extrude and B-Spline Extrude are
//! the Sweep tool with a curve kind; Curve Clone, Bezier Clone, B-Spline
//! Clone and Pen Clone are the Clone tool with one. Each button is a row in
//! the preset table (`factory-tool-presets.toml`, `path` values).
//!
//! The path is clicked out in world space (or drawn freehand and fitted,
//! curve_fit.rs). The generator turns it into **sections**: a position on
//! the path, the path's direction there and a turn (Align to Path).
//!
//! - Sweep (CX-10..16): the selected polygons are the first section; each
//!   later section is a copy of their outline moved so its centre sits on
//!   the path, joined to the one before by a ring of side quads. Cap Start
//!   keeps the original as the start cap, Cap End adds an end cap.
//! - Clone (CX-20..24): one copy of the source per later section, placed
//!   with its Scale Pivot (bottom centre) on the section's point.
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, CX-Q*): Size is
//! the scale reached at the last section, spread evenly along the path;
//! Invert on (the extrude presets) gives polygons facing out of the swept
//! solid; the source counts as the first Clone copy; a closed sweep path
//! returns to its start with its caps as set.
use crate::curves::{self, CurveKind};
use crate::duplicate::{Copies, WeldAt, copy_faces, reverse_corners, source_faces};
use crate::{Mesh, Mode, Result, Scene};
use glam::{DMat4, DQuat, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// More sections than this stops the preview (an allocation guard).
pub const MAX_SECTIONS: u32 = 10_000;

/// The path generator's curve kinds (CX-01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    /// A smooth curve through every point (Operandi's Curve drawing, CX-10).
    #[default]
    Curve,
    Bezier,
    BSpline,
    /// Straight segments; one section per point, no Path Steps (CX-20, CX-30).
    Pen,
}
impl PathKind {
    pub const ALL: [PathKind; 4] = [
        PathKind::Curve,
        PathKind::Bezier,
        PathKind::BSpline,
        PathKind::Pen,
    ];
    pub fn label(self) -> &'static str {
        match self {
            PathKind::Curve => "Curve",
            PathKind::Bezier => "Bezier",
            PathKind::BSpline => "B-Spline",
            PathKind::Pen => "Pen",
        }
    }
    /// Pen paths have one section per point; the others take Path Steps.
    pub fn uses_steps(self) -> bool {
        self != PathKind::Pen
    }
}

/// Path Steps (CX-02 starting values).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathSteps {
    /// Sweep: sections after the first. Clone: copies along the path, the
    /// source included (SPEC-CURVE-CLONE CC-01, Q3 provisional).
    pub number: u32,
    /// Sections where the curve bends more than Smoothing Angle (CX-11).
    pub automatic: bool,
    /// Degrees.
    pub smoothing_angle: f64,
    pub align_to_path: bool,
    /// The path is moved so its first point sits on the selection (CX-12).
    pub start_at_source: bool,
}
impl Default for PathSteps {
    fn default() -> Self {
        Self {
            number: 24,
            automatic: false,
            smoothing_angle: 5.,
            align_to_path: true,
            start_at_source: false,
        }
    }
}

/// The path itself and its own options (CX-02).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathCurve {
    pub kind: PathKind,
    /// World positions. Bezier stores anchors and handles in `curves`'
    /// order (a0, out0, in1, a1, …).
    pub points: Vec<DVec3>,
    pub closed: bool,
    /// Curve only: the first / last point shapes the curve without being on it.
    pub start_control: bool,
    pub end_control: bool,
    /// Sections evenly spaced along the curve's length instead of its
    /// parameter.
    pub by_length: bool,
    /// Leave the path behind as a curve polygon in the mesh.
    pub create_curve_polygon: bool,
    /// Degrees of roll for every section.
    pub bank: f64,
    /// Degrees of roll spread from the first section (0) to the last.
    pub twist: f64,
}
impl PathCurve {
    /// The curve kind `curves` draws this path with.
    pub fn curve_kind(&self) -> CurveKind {
        match self.kind {
            PathKind::Curve => CurveKind::Curve {
                start_control: self.start_control,
                end_control: self.end_control,
            },
            PathKind::Bezier => CurveKind::Bezier,
            PathKind::BSpline => CurveKind::BSpline,
            PathKind::Pen => CurveKind::Line,
        }
    }
    /// Fewest points that make a path.
    pub fn minimum_points(&self) -> usize {
        self.curve_kind().minimum_points()
    }
    /// The path drawn as a line, for the viewport overlay (world space).
    pub fn drawn(&self) -> Vec<DVec3> {
        let mut out = curves::sample(&self.points, self.curve_kind(), self.closed, 24);
        if self.closed && out.len() > 2 {
            out.push(out[0]);
        }
        out
    }
}

/// The Sweep effector (CX-02).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepEffector {
    /// On: the result faces out of the swept solid (provisional, CX-Q5).
    pub invert: bool,
    pub cap_start: bool,
    pub cap_end: bool,
    /// Scale of the last section, 1 = 100%.
    pub size: f64,
}
impl Default for SweepEffector {
    fn default() -> Self {
        Self {
            invert: false,
            cap_start: true,
            cap_end: true,
            size: 1.,
        }
    }
}

/// Where a Clone copy is held when it is placed and turned (CX-21).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScalePivot {
    /// The bottom centre of the source's bounding box (lowest Y).
    #[default]
    Bottom,
    Center,
}

/// The Clone effector (CX-02).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloneEffector {
    pub copies: Copies,
    /// Scale of the last copy, 1 = 100%.
    pub size: f64,
    pub pivot: ScalePivot,
}
impl Default for CloneEffector {
    fn default() -> Self {
        Self {
            copies: Copies::default(),
            size: 1.,
            pivot: ScalePivot::Bottom,
        }
    }
}

/// One path tool's settings: generator plus both effectors (each tool
/// uses one).
#[derive(Clone, Debug, PartialEq)]
pub struct PathTool {
    pub curve: PathCurve,
    pub steps: PathSteps,
    pub sweep: SweepEffector,
    pub clone: CloneEffector,
    /// Pen: copies turn to follow the path (CX-22; on by default).
    pub pen_align: bool,
    /// Pen Extrude: an inner section widens across the bend so the walls
    /// keep their thickness at the corner (CX-31, build guess; on).
    pub corner_scale: bool,
}
impl Default for PathTool {
    fn default() -> Self {
        Self {
            curve: PathCurve::default(),
            steps: PathSteps::default(),
            sweep: SweepEffector::default(),
            clone: CloneEffector::default(),
            pen_align: true,
            corner_scale: true,
        }
    }
}
impl PathTool {
    /// The path tool's settings kept for the next session: the options,
    /// not the drawn path.
    pub fn options_only(&self) -> Self {
        let mut out = self.clone();
        out.curve.points.clear();
        out
    }
    /// Whether the path has enough points for a result.
    pub fn has_path(&self) -> bool {
        self.curve.points.len() >= self.curve.minimum_points()
    }
    fn aligned(&self) -> bool {
        if self.curve.kind == PathKind::Pen {
            self.pen_align
        } else {
            self.steps.align_to_path
        }
    }
}

/// A preset row's path values (`path = { kind = "bezier", ... }`).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathPreset {
    pub kind: PathKind,
    #[serde(default)]
    pub start_at_source: Option<bool>,
    #[serde(default)]
    pub cap_start: Option<bool>,
    #[serde(default)]
    pub invert: Option<bool>,
}
impl PathPreset {
    /// The row's values over `base` (TP-04).
    pub fn apply(&self, base: &mut PathTool) {
        if base.curve.kind != self.kind {
            base.curve.points.clear();
        }
        base.curve.kind = self.kind;
        if let Some(v) = self.start_at_source {
            base.steps.start_at_source = v;
        }
        if let Some(v) = self.cap_start {
            base.sweep.cap_start = v;
        }
        if let Some(v) = self.invert {
            base.sweep.invert = v;
        }
    }
}

/// One place along the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    pub position: DVec3,
    /// Unit direction of the path there.
    pub tangent: DVec3,
}

fn unit_or(v: DVec3, fallback: DVec3) -> DVec3 {
    v.try_normalize().unwrap_or(fallback)
}

/// The sections of `tool`'s path, in the path's own space. A Sweep gets
/// Number + 1 (the first is where the polygons start); a Clone gets Number
/// (CX-11, SPEC-CURVE-CLONE CC-01). Pen paths give one per point.
pub fn sections(tool: &PathTool, sweep: bool) -> Result<Vec<Section>> {
    let c = &tool.curve;
    if c.points.iter().any(|p| !p.is_finite()) {
        return Err("The path has a point that isn't a number".into());
    }
    if !tool.has_path() {
        return Err(format!(
            "Draw the path: click at least {} points",
            c.minimum_points()
        ));
    }
    if c.kind == PathKind::Pen {
        let mut out = pen_sections(&c.points, c.closed);
        // A closed sweep path returns to its start (the provisional reading
        // above): the last span runs from the last point back to the first,
        // as the smooth kinds' sections already do (#51).
        if sweep
            && c.closed
            && let Some(&first) = out.first()
        {
            out.push(first);
        }
        return Ok(out);
    }
    let kind = c.curve_kind();
    let spans = curves::span_count(&c.points, kind, c.closed);
    if spans == 0 {
        return Err("The path needs more points".into());
    }
    let s = &tool.steps;
    let params: Vec<f64> = if s.automatic {
        if !(s.smoothing_angle.is_finite() && s.smoothing_angle > 0.) {
            return Err("Smoothing Angle must be above 0°".into());
        }
        let counts =
            curves::adaptive_counts(&c.points, kind, c.closed, s.smoothing_angle.to_radians());
        let mut u = vec![0.];
        for (i, &n) in counts.iter().enumerate() {
            let n = n.max(1);
            u.extend((1..=n).map(|k| i as f64 + k as f64 / n as f64));
        }
        if !sweep && c.closed {
            u.pop();
        }
        u
    } else {
        if s.number == 0 || s.number > MAX_SECTIONS {
            return Err(format!("Steps must be 1 to {MAX_SECTIONS}"));
        }
        let n = s.number as usize;
        // A sweep's sections split the curve into Number steps; a clone's
        // copies split it into Number − 1 (both ends on), or Number round
        // a closed path.
        let intervals = if sweep || c.closed {
            n
        } else {
            n.saturating_sub(1)
        };
        let count = if sweep { n + 1 } else { n };
        let fractions: Vec<f64> = (0..count)
            .map(|k| {
                if intervals == 0 {
                    0.
                } else {
                    k as f64 / intervals as f64
                }
            })
            .collect();
        if c.by_length {
            by_length(&c.points, kind, c.closed, spans, &fractions)
        } else {
            fractions.iter().map(|f| f * spans as f64).collect()
        }
    };
    Ok(params
        .iter()
        .map(|&u| {
            let at = |u: f64| curves::point_at(&c.points, kind, c.closed, u).unwrap_or_default();
            let h = 1e-5;
            let (lo, hi) = if c.closed {
                (u - h, u + h)
            } else {
                ((u - h).max(0.), (u + h).min(spans as f64))
            };
            // A closed curve's parameter wraps round.
            let wrap = |u: f64| u.rem_euclid(spans as f64);
            let (a, b) = if c.closed {
                (at(wrap(lo)), at(wrap(hi)))
            } else {
                (at(lo), at(hi))
            };
            Section {
                position: at(if c.closed { wrap(u) } else { u }),
                tangent: unit_or(b - a, DVec3::Y),
            }
        })
        .collect())
}

/// Parameters for even length fractions along the curve (By Length).
fn by_length(
    points: &[DVec3],
    kind: CurveKind,
    closed: bool,
    spans: usize,
    fractions: &[f64],
) -> Vec<f64> {
    const FINE: usize = 64;
    let total = spans * FINE;
    let us: Vec<f64> = (0..=total).map(|k| k as f64 / FINE as f64).collect();
    let ps: Vec<DVec3> = us
        .iter()
        .map(|&u| curves::point_at(points, kind, closed, u).unwrap_or_default())
        .collect();
    let mut lengths = vec![0.];
    for w in ps.windows(2) {
        lengths.push(lengths.last().unwrap() + w[0].distance(w[1]));
    }
    let full = *lengths.last().unwrap();
    fractions
        .iter()
        .map(|&f| {
            let want = f * full;
            let k = lengths.partition_point(|&l| l < want).clamp(1, total);
            let (l0, l1) = (lengths[k - 1], lengths[k]);
            let t = if l1 > l0 { (want - l0) / (l1 - l0) } else { 0. };
            us[k - 1] + t * (us[k] - us[k - 1])
        })
        .collect()
}

/// Pen sections: each point, facing the average of its two segments at an
/// inner point and its one segment at an end (CX-22).
fn pen_sections(points: &[DVec3], closed: bool) -> Vec<Section> {
    let n = points.len();
    let seg = |i: usize| unit_or(points[(i + 1) % n] - points[i], DVec3::Y);
    (0..n)
        .map(|i| {
            let tangent = if closed {
                unit_or(seg((i + n - 1) % n) + seg(i), seg(i))
            } else if i == 0 {
                seg(0)
            } else if i == n - 1 {
                seg(n - 2)
            } else {
                unit_or(seg(i - 1) + seg(i), seg(i))
            };
            Section {
                position: points[i],
                tangent,
            }
        })
        .collect()
}

/// Each section's turn: the identity at the first, then the previous turn
/// followed by the smallest rotation taking the previous section's
/// tangent onto this one's (CX-Q8); Bank and Twist roll it about the
/// tangent. Align off: only Bank/Twist about `start`, no following.
fn turns(tool: &PathTool, sections: &[Section], start: DVec3) -> Vec<DQuat> {
    let aligned = tool.aligned();
    let last = sections.len().saturating_sub(1).max(1) as f64;
    let mut carried = DQuat::IDENTITY;
    // CX-Q8 (answered 2026-11-20): the frame is carried from the source
    // polygon as it is, each step turned by the smallest rotation from the
    // previous section's tangent to this one's; the polygon isn't first
    // turned to face along the path.
    let mut dir = sections.first().map_or(start, |s| s.tangent);
    sections
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let roll_deg = tool.curve.bank + tool.curve.twist * k as f64 / last;
            if !aligned {
                return DQuat::from_axis_angle(start, roll_deg.to_radians());
            }
            if k > 0 {
                carried = DQuat::from_rotation_arc(dir, s.tangent) * carried;
                dir = s.tangent;
            }
            DQuat::from_axis_angle(dir, roll_deg.to_radians()) * carried
        })
        .collect()
}

/// CX-31 Corner Scale (Pen paths): at an inner point that bends by θ, the
/// section's width across the bend (in the bend's plane, square to the
/// path) grows by 1 / cos(θ / 2), so both walls keep their thickness.
/// `None` when nothing widens.
fn corner_widening(tool: &PathTool, path: &[Section], k: usize) -> Option<(DVec3, f64)> {
    if tool.curve.kind != PathKind::Pen || !tool.corner_scale || !tool.pen_align {
        return None;
    }
    let n = path.len();
    let (prev, next) = if tool.curve.closed {
        ((k + n - 1) % n, (k + 1) % n)
    } else if k == 0 || k + 1 >= n {
        return None;
    } else {
        (k - 1, k + 1)
    };
    let t_in = (path[k].position - path[prev].position).try_normalize()?;
    let t_out = (path[next].position - path[k].position).try_normalize()?;
    let bend = t_in.cross(t_out).try_normalize()?;
    let across = bend.cross(path[k].tangent).try_normalize()?;
    let half = t_in.angle_between(t_out) / 2.;
    (half.cos() > 1e-3).then(|| (across, 1. / half.cos()))
}

/// Section scale: 1 at the first section, Size at the last.
fn scale_at(size: f64, k: usize, count: usize) -> f64 {
    let last = count.saturating_sub(1).max(1) as f64;
    1. + (size - 1.) * k as f64 / last
}

/// The selected polygons in groups that share edges (each sweeps from
/// where it sits, CX-16).
fn regions(mesh: &Mesh, faces: &[u32]) -> Vec<Vec<u32>> {
    let set: BTreeSet<u32> = faces.iter().copied().collect();
    let mut edge_owner: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for &f in &set {
        let v = &mesh.faces[f as usize].vertices;
        for i in 0..v.len() {
            let (a, b) = (v[i], v[(i + 1) % v.len()]);
            edge_owner.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for &f in &set {
        if !seen.insert(f) {
            continue;
        }
        let mut group = vec![f];
        let mut i = 0;
        while i < group.len() {
            let g = group[i];
            i += 1;
            let v = &mesh.faces[g as usize].vertices;
            for j in 0..v.len() {
                let (a, b) = (v[j], v[(j + 1) % v.len()]);
                for &h in &edge_owner[&(a.min(b), a.max(b))] {
                    if seen.insert(h) {
                        group.push(h);
                    }
                }
            }
        }
        group.sort_unstable();
        out.push(group);
    }
    out
}

/// What a sweep made: the mesh and the caps selected after it: every
/// start and end cap it made (CX-15, 2026-11-20).
#[derive(Clone, Debug, PartialEq)]
pub struct SweepResult {
    pub mesh: Mesh,
    pub end_caps: Vec<u32>,
}

/// Sweep the polygons `selected` of `mesh` along `path` (both local
/// space): CX-10..16.
pub fn sweep_geometry(mesh: &Mesh, selected: &[u32], tool: &PathTool) -> Result<SweepResult> {
    let mut picked: Vec<u32> = selected
        .iter()
        .copied()
        .filter(|&f| (f as usize) < mesh.faces.len() && !mesh.is_polyline(f as usize))
        .collect();
    picked.sort_unstable();
    picked.dedup();
    if picked.is_empty() {
        return Err("Curve Extrude: select the polygons to extrude".into());
    }
    let size = tool.sweep.size;
    if !(size.is_finite() && size > 0.) {
        return Err("Size must be above 0%".into());
    }
    let path = sections(tool, true)?;
    if path.len() < 2 {
        return Err("The path needs at least two sections".into());
    }
    let mut out = mesh.clone();
    let mut new_faces: Vec<u32> = Vec::new();
    let mut end_caps: Vec<u32> = Vec::new();
    let groups = regions(mesh, &picked);
    let all_centre = centre_of(mesh, &picked);
    let mut reversed: Vec<usize> = Vec::new();
    let mut removed: BTreeSet<usize> = BTreeSet::new();
    for group in &groups {
        let c0 = centre_of(mesh, group);
        let normal = group
            .iter()
            .map(|&f| mesh.normal(f as usize))
            .sum::<DVec3>()
            .try_normalize()
            .unwrap_or(path[0].tangent);
        // The path's start sits on this group (Start at Source), or the
        // path keeps its place and each group keeps its offset from the
        // selection's centre.
        let offset = if tool.steps.start_at_source {
            c0 - path[0].position
        } else {
            c0 - all_centre
        };
        // The polygons' facing, pointing along the path (the side the
        // sweep leaves from); the turns carry it unchanged from the start.
        let start = if normal.dot(path[0].tangent) < 0. {
            -normal
        } else {
            normal
        };
        let turns = turns(tool, &path, start);
        // Boundary edges, in each polygon's winding.
        let mut uses: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for &f in group {
            let v = &mesh.faces[f as usize].vertices;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                *uses.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let mut boundary: Vec<(u32, u32, u32)> = Vec::new();
        for &f in group {
            let v = &mesh.faces[f as usize].vertices;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                if uses[&(a.min(b), a.max(b))] == 1 {
                    boundary.push((a, b, f));
                }
            }
        }
        let mut points: Vec<u32> = group
            .iter()
            .flat_map(|&f| mesh.faces[f as usize].vertices.iter().copied())
            .collect();
        points.sort_unstable();
        points.dedup();
        let on_boundary: BTreeSet<u32> = boundary.iter().flat_map(|&(a, b, _)| [a, b]).collect();
        let last = path.len() - 1;
        // ring[k][point] for k = 1..=last.
        let mut rings: Vec<BTreeMap<u32, u32>> = vec![BTreeMap::new(); path.len()];
        rings[0] = points.iter().map(|&v| (v, v)).collect();
        for k in 1..=last {
            let widen = corner_widening(tool, &path, k);
            let place = |q: DVec3| {
                let r = turns[k] * ((q - c0) * scale_at(size, k, path.len()));
                let r = widen.map_or(r, |(w, f)| r + w * (r.dot(w) * (f - 1.)));
                path[k].position + offset + r
            };
            for &v in &points {
                // Inner points are only needed for the end cap.
                if k < last && !on_boundary.contains(&v) {
                    continue;
                }
                let id = u32::try_from(out.vertices.len()).map_err(|_| "Too many points")?;
                out.vertices.push(place(mesh.vertices[v as usize]));
                rings[k].insert(v, id);
            }
        }
        let first_new = out.faces.len();
        for k in 1..=last {
            for &(a, b, f) in &boundary {
                let quad = vec![
                    rings[k - 1][&a],
                    rings[k - 1][&b],
                    rings[k][&b],
                    rings[k][&a],
                ];
                crate::vertex_commands::append_face(&mut out, quad, Some(f as usize));
                new_faces.push((out.faces.len() - 1) as u32);
            }
        }
        if tool.sweep.cap_end {
            for &f in group {
                let cap = mesh.faces[f as usize]
                    .vertices
                    .iter()
                    .map(|v| rings[last][v])
                    .collect();
                crate::vertex_commands::append_face(&mut out, cap, Some(f as usize));
                end_caps.push((out.faces.len() - 1) as u32);
                new_faces.push((out.faces.len() - 1) as u32);
            }
        }
        // A group joined to other polygons keeps the mesh's winding and
        // has no start cap: its walls start from the hole it leaves.
        let counts = mesh.edge_counts();
        let attached = boundary
            .iter()
            .any(|&(a, b, _)| counts.get(&(a.min(b), a.max(b))).is_some_and(|&n| n > 1));
        // The walls and end cap follow the polygons' winding. Invert on
        // (the presets) keeps that; off reverses a free-standing group
        // (CX-Q5).
        let flip = !tool.sweep.invert && !attached;
        if flip {
            reversed.extend(first_new..out.faces.len());
        }
        // Cap Start keeps the original polygon, turned to face back along
        // the walls.
        if tool.sweep.cap_start && !attached {
            if !flip {
                reversed.extend(group.iter().map(|&f| f as usize));
            }
            // Every cap made is selected afterwards (2026-11-20): the
            // start cap with the end cap (CX-15).
            end_caps.extend(group.iter().copied());
        } else {
            removed.extend(group.iter().map(|&f| f as usize));
        }
    }
    for &f in &reversed {
        reverse_corners(&mut out, f);
    }
    if tool.curve.create_curve_polygon {
        add_curve_polygon(&mut out, &tool.curve)?;
    }
    if !removed.is_empty() {
        out.remove_faces(&removed)?;
        let shift = |f: u32| f - removed.iter().filter(|&&p| p < f as usize).count() as u32;
        end_caps = end_caps.into_iter().map(shift).collect();
    }
    out.validate()?;
    Ok(SweepResult {
        mesh: out,
        end_caps,
    })
}

/// The centre of the bounding box of `faces`' points.
fn centre_of(mesh: &Mesh, faces: &[u32]) -> DVec3 {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for &f in faces {
        for &v in &mesh.faces[f as usize].vertices {
            let p = mesh.vertices[v as usize];
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    if lo.is_finite() {
        (lo + hi) / 2.
    } else {
        DVec3::ZERO
    }
}

/// Create Curve Polygon: the path (in the mesh's space, already in
/// `curve.points`) left behind as a curve element.
fn add_curve_polygon(mesh: &mut Mesh, curve: &PathCurve) -> Result<()> {
    let first = u32::try_from(mesh.vertices.len()).map_err(|_| "Too many points")?;
    mesh.vertices.extend(curve.points.iter().copied());
    let ids: Vec<u32> = (first..first + curve.points.len() as u32).collect();
    let representation = crate::PolygonRepresentation::Polyline {
        closed: curve.closed,
        curve: curve.curve_kind(),
    };
    let faces_before = mesh.faces.len();
    crate::vertex_commands::append_face(mesh, ids, None);
    let reps = &mut mesh.attributes.representations;
    if reps.len() < faces_before + 1 {
        reps.resize(faces_before, crate::PolygonRepresentation::Face);
        reps.push(representation);
    } else {
        reps[faces_before] = representation;
    }
    Ok(())
}

/// The Clone effector's transforms (local space) for `source` polygons:
/// one per section after the first, plus the first when Replace Source
/// moves the original onto it (CX-20..24).
pub fn clone_transforms(mesh: &Mesh, source: &[usize], tool: &PathTool) -> Result<Vec<DMat4>> {
    let size = tool.clone.size;
    if !(size.is_finite() && size > 0.) {
        return Err("Size must be above 0%".into());
    }
    let path = sections(tool, false)?;
    let faces: Vec<u32> = source.iter().map(|&f| f as u32).collect();
    let (lo, hi) = bounds_of(mesh, &faces).ok_or("Nothing to copy")?;
    let pivot = match tool.clone.pivot {
        ScalePivot::Bottom => DVec3::new((lo.x + hi.x) / 2., lo.y, (lo.z + hi.z) / 2.),
        ScalePivot::Center => (lo + hi) / 2.,
    };
    // Pen paths and Start at Source off use the path where it was drawn.
    let offset = if tool.curve.kind.uses_steps() && tool.steps.start_at_source {
        pivot - path[0].position
    } else {
        DVec3::ZERO
    };
    let turns = turns(tool, &path, path[0].tangent);
    let first = usize::from(!tool.clone.copies.replace_source);
    Ok((first..path.len())
        .map(|k| {
            let s = scale_at(size, k, path.len());
            DMat4::from_translation(path[k].position + offset)
                * DMat4::from_quat(turns[k])
                * DMat4::from_scale(DVec3::splat(s))
                * DMat4::from_translation(-pivot)
        })
        .collect())
}

fn bounds_of(mesh: &Mesh, faces: &[u32]) -> Option<(DVec3, DVec3)> {
    let mut lo = DVec3::splat(f64::INFINITY);
    let mut hi = DVec3::splat(f64::NEG_INFINITY);
    for &f in faces {
        for &v in &mesh.faces.get(f as usize)?.vertices {
            let p = mesh.vertices[v as usize];
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    lo.is_finite().then_some((lo, hi))
}

/// `tool` with its path moved into the mesh's own space (and mirrored for
/// the other side of Symmetry).
pub(crate) fn local_tool(
    tool: &PathTool,
    local: DMat4,
    mirror: Option<crate::Symmetry>,
) -> PathTool {
    let mut t = tool.clone();
    for p in &mut t.curve.points {
        let q = local.transform_point3(*p);
        *p = mirror.map_or(q, |s| s.mirror(q));
    }
    t
}

/// The live preview of Curve Extrude (Sweep) or Curve Clone on the base
/// scene. Nothing changes until the path has enough points.
pub(crate) fn preview(
    sweep: bool,
    scene: &mut Scene,
    tool: &PathTool,
    symmetry: crate::Symmetry,
) -> Result<()> {
    if !tool.has_path() {
        return Ok(());
    }
    if scene.selection.mode == Mode::Item {
        return Err("Path tools work on polygons; pick a component mode".into());
    }
    let item = scene.active().ok_or("Select a mesh")?;
    let world = scene.world_matrix(item.id)?;
    if world.determinant() == 0. {
        return Err("Path tools need an invertible item transform".into());
    }
    let local = world.inverse();
    if sweep {
        if scene.selection.mode != Mode::Polygon || scene.selection.faces.is_empty() {
            return Err("Curve Extrude: select polygons in Polygon mode".into());
        }
        // SYM3-12: the other side's polygons follow the mirrored path.
        let mut made_curve = false;
        crate::symmetry_sides::two_pass(scene, symmetry, |scene, mirrored| {
            let selected = scene.selection.faces.clone();
            let item = scene.active().ok_or("Select a mesh")?;
            let mut t = local_tool(tool, local, mirrored.then_some(symmetry));
            // One curve polygon is enough.
            t.curve.create_curve_polygon &= !std::mem::replace(&mut made_curve, true);
            let result = sweep_geometry(&item.mesh, &selected, &t)?;
            scene.active_mut().ok_or("Select a mesh")?.mesh = result.mesh.into();
            scene.selection.faces = result.end_caps;
            scene.selection.vertices.clear();
            scene.selection.edges.clear();
            Ok(())
        })?;
        return Ok(());
    }
    if source_faces(scene).is_empty() {
        return Err(crate::duplicate::NOTHING_TO_COPY.into());
    }
    let scale = crate::scene::uniform_scale(world);
    let mut made_curve = false;
    crate::symmetry_sides::two_pass(scene, symmetry, |scene, mirrored| {
        let source = source_faces(scene);
        if source.is_empty() {
            return Ok(());
        }
        let item = scene.active().ok_or("Select a mesh")?;
        let mut t = local_tool(tool, local, mirrored.then_some(symmetry));
        let placed = clone_transforms(&item.mesh, &source, &t)?;
        let copies: Copies = t.clone.copies;
        let outcome = copy_faces(
            &item.mesh,
            &source,
            &placed,
            &copies,
            WeldAt::Original,
            copies.distance / scale,
        )?;
        let mut mesh = outcome.mesh.clone();
        t.curve.create_curve_polygon &= !std::mem::replace(&mut made_curve, true);
        if t.curve.create_curve_polygon {
            add_curve_polygon(&mut mesh, &t.curve)?;
        }
        scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
        crate::duplicate::select_after(scene, &outcome, copies.replace_source);
        Ok(())
    })?;
    Ok(())
}
