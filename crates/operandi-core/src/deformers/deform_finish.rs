//! Deform finish (modelling parity batch 1): the Deform tab's point tools
//! Jitter, Quantize, Radial Align, Linear Align and the Relax Mesh command.
//! Spec: `specifications/deform-presets-2026-11-06` (v0) and the probe pass
//! DEFORM-PROBES (DP-xx, 2026-11-17). Open readings are DFF-Q01.. in
//! docs/SPECIFICATION_QUESTIONS.md.
//!
//! Every function here takes the mesh, the points it works on and an
//! optional falloff weight per point, and returns the moved mesh. Topology
//! never changes. The editor runs them as live tools (editor.rs) and the
//! Mesh Stack runs the same functions (mesh_stack_deform.rs).
use crate::{Mesh, Mode, Result, Scene};
use glam::{DMat3, DMat4, DVec3};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------- Jitter

/// Jitter's options (DP-00..04). Starting values from DP "Starting values":
/// Range 0, X/Y/Z on, the Rigid switches off. Seed: the Deform button's
/// 666, which every way of starting the tool in Operandi uses; the bare
/// tool's 137 is the MeshOp's starting value (DFF-Q01).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JitterOptions {
    /// Half-widths: each point moves within ±Range on each axis (DP-01).
    pub range: DVec3,
    pub seed: u32,
    pub enable: [bool; 3],
    pub rigid_translate: bool,
    pub rigid_rotate: bool,
    pub rigid_scale: bool,
    /// Set only while the tool runs on a UV proxy (UVT-40, UVT-135): Rigid
    /// Rotate turns each piece about the view axis only, by the angle
    /// Range X/Y allow, so a flat UV island never tilts out of the plane.
    pub planar: bool,
}
impl Default for JitterOptions {
    fn default() -> Self {
        Self {
            range: DVec3::ZERO,
            seed: JITTER_BUTTON_SEED,
            enable: [true; 3],
            rigid_translate: false,
            rigid_rotate: false,
            rigid_scale: false,
            planar: false,
        }
    }
}
/// A bare Jitter tool's Seed (DP starting values).
pub const JITTER_TOOL_SEED: u32 = 137;
/// The Deform tab button's Seed (DP starting values, preset).
pub const JITTER_BUTTON_SEED: u32 = 666;

impl JitterOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.range.is_finite() || self.range.min_element() < 0. {
            return Err("Jitter Range must be zero or more on every axis".into());
        }
        Ok(())
    }
    fn rigid(&self) -> bool {
        self.rigid_translate || self.rigid_rotate || self.rigid_scale
    }
}

/// Operandi's own generator (DP-02: the numbers needn't match the
/// reference's): splitmix64 over seed, key and stream, mapped to −1..1.
/// Each point's three draws depend only on the seed and the point, never on
/// which axes are on (DP-03).
pub fn jitter_random(seed: u32, key: u32, stream: u32) -> f64 {
    let mut z = (u64::from(seed) << 32 | u64::from(key))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(u64::from(stream).wrapping_mul(0xD1B5_4A32_D192_ED03));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    // 53 bits -> [0, 1) -> [-1, 1).
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2. - 1.
}

fn masked(v: DVec3, enable: [bool; 3]) -> DVec3 {
    DVec3::new(
        if enable[0] { v.x } else { 0. },
        if enable[1] { v.y } else { 0. },
        if enable[2] { v.z } else { 0. },
    )
}

/// The connected pieces of `mesh` (through polygons and polylines), as a
/// piece id per point: the lowest point index in the piece.
pub fn piece_ids(mesh: &Mesh) -> Vec<u32> {
    let n = mesh.vertices.len();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(p: &mut [u32], mut v: u32) -> u32 {
        while p[v as usize] != v {
            p[v as usize] = p[p[v as usize] as usize];
            v = p[v as usize];
        }
        v
    }
    for f in mesh.faces.iter() {
        let Some(&first) = f.vertices.first() else {
            continue;
        };
        for &v in &f.vertices[1..] {
            let (a, b) = (find(&mut parent, first), find(&mut parent, v));
            if a != b {
                // Keep the lower index as the root, so the id is the piece's
                // lowest point.
                let (lo, hi) = (a.min(b), a.max(b));
                parent[hi as usize] = lo;
            }
        }
    }
    (0..n as u32).map(|v| find(&mut parent, v)).collect()
}

/// Jitter `points` (DP-01..04). `weights` scale each point's move (falloff).
/// Rigid modes move each connected piece as one body: Translate by one
/// shared offset (DP-04); Rotate and Scale about the piece's centre, by
/// amounts Range allows at the piece's size (DFF-Q02, provisional).
pub fn jitter_geometry(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    o: &JitterOptions,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    o.validate()?;
    let weight = |v: u32| weights.map_or(1., |w| w.get(v as usize).copied().unwrap_or(0.));
    let mut out = mesh.clone();
    if !o.rigid() {
        for &v in points {
            let i = v as usize;
            if i >= out.vertices.len() {
                continue;
            }
            let draw = DVec3::new(
                jitter_random(o.seed, v, 0),
                jitter_random(o.seed, v, 1),
                jitter_random(o.seed, v, 2),
            );
            out.vertices[i] += masked(draw * o.range, o.enable) * weight(v);
        }
        return Ok(out);
    }
    let ids = piece_ids(mesh);
    let mut pieces: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &v in points {
        if let Some(&id) = ids.get(v as usize) {
            pieces.entry(id).or_default().push(v);
        }
    }
    for (id, members) in pieces {
        let center = members
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .sum::<DVec3>()
            / members.len() as f64;
        let radius = members
            .iter()
            .map(|&v| mesh.vertices[v as usize].distance(center))
            .fold(0., f64::max);
        let draw = |stream: u32| {
            DVec3::new(
                jitter_random(o.seed, id, stream),
                jitter_random(o.seed, id, stream + 1),
                jitter_random(o.seed, id, stream + 2),
            )
        };
        let translate = if o.rigid_translate {
            masked(draw(0) * o.range, o.enable)
        } else {
            DVec3::ZERO
        };
        // Rotate: up to the angle whose arc at the piece's edge is Range.
        let rotation = if o.rigid_rotate && radius > 1e-12 && o.planar {
            let reach = masked(o.range, o.enable).truncate().max_element();
            let limit = (reach / radius).min(std::f64::consts::PI);
            DMat3::from_rotation_z(draw(3).z * limit)
        } else if o.rigid_rotate && radius > 1e-12 {
            let limit = (o.range / radius).min(DVec3::splat(std::f64::consts::PI));
            let a = masked(draw(3) * limit, o.enable);
            DMat3::from_rotation_z(a.z) * DMat3::from_rotation_y(a.y) * DMat3::from_rotation_x(a.x)
        } else {
            DMat3::IDENTITY
        };
        // Scale: up to the factor that grows the piece's edge by Range.
        let scale = if o.rigid_scale && radius > 1e-12 {
            let reach = masked(o.range, o.enable).max_element() / radius;
            (1. + jitter_random(o.seed, id, 6) * reach.min(0.99)).max(0.01)
        } else {
            1.
        };
        for v in members {
            let p = mesh.vertices[v as usize];
            let moved = center + rotation * ((p - center) * scale) + translate;
            out.vertices[v as usize] = p + (moved - p) * weight(v);
        }
    }
    Ok(out)
}

// -------------------------------------------------------------- Quantize

/// Quantize: Step X/Y/Z (DP-05); 0 leaves that axis alone.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuantizeOptions {
    pub step: DVec3,
}
impl QuantizeOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.step.is_finite() || self.step.min_element() < 0. {
            return Err("Quantize Step must be zero or more on every axis".into());
        }
        Ok(())
    }
}

/// DP-05: each point snaps to the nearest multiple of Step on each axis,
/// measured from the world origin (`world` places the mesh); exact halves
/// round away from zero (build guess). The weight blends start → snapped.
pub fn quantize_geometry(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    o: &QuantizeOptions,
    world: DMat4,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    o.validate()?;
    let inverse = world.inverse();
    if !inverse.is_finite() {
        return Err("Quantize can't work on a mesh scaled to nothing".into());
    }
    let mut out = mesh.clone();
    for &v in points {
        let i = v as usize;
        let Some(&p) = mesh.vertices.get(i) else {
            continue;
        };
        let mut w = world.transform_point3(p);
        for axis in 0..3 {
            let s = o.step[axis];
            if s > 0. {
                w[axis] = (w[axis] / s).round() * s;
            }
        }
        let snapped = inverse.transform_point3(w);
        let k = weights.map_or(1., |ws| ws.get(i).copied().unwrap_or(0.));
        out.vertices[i] = p + (snapped - p) * k;
    }
    Ok(out)
}

// ----------------------------------------------------------- Runs of points

/// An ordered run of points for the Align tools: a closed loop or an open
/// chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub points: Vec<u32>,
    pub closed: bool,
}

/// Split the graph `edges` into runs: chains between ends (points with one
/// edge, or a junction of three or more) and closed loops. Runs never cross:
/// a junction ends every run through it (DEFORM Linear: "runs must not
/// cross").
pub fn runs_from_edges(edges: &[(u32, u32)]) -> Vec<Run> {
    let mut adjacent: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut seen_edge: BTreeSet<(u32, u32)> = BTreeSet::new();
    for &(a, b) in edges {
        if a == b || !seen_edge.insert((a.min(b), a.max(b))) {
            continue;
        }
        adjacent.entry(a).or_default().push(b);
        adjacent.entry(b).or_default().push(a);
    }
    let mut used: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut runs = Vec::new();
    let degree = |v: u32| adjacent.get(&v).map_or(0, Vec::len);
    let walk = |start: u32, next: u32, used: &mut BTreeSet<(u32, u32)>| -> (Vec<u32>, bool) {
        let mut run = vec![start];
        let (mut prev, mut at) = (start, next);
        used.insert((start.min(next), start.max(next)));
        loop {
            if at == start {
                return (run, true);
            }
            run.push(at);
            if degree(at) != 2 {
                return (run, false);
            }
            let Some(&n) = adjacent[&at]
                .iter()
                .find(|&&n| n != prev && !used.contains(&(at.min(n), at.max(n))))
            else {
                return (run, false);
            };
            used.insert((at.min(n), at.max(n)));
            prev = at;
            at = n;
        }
    };
    // Open chains first, from every end.
    let ends: Vec<u32> = adjacent
        .keys()
        .copied()
        .filter(|&v| degree(v) != 2)
        .collect();
    for start in ends {
        for &next in adjacent[&start].clone().iter() {
            if used.contains(&(start.min(next), start.max(next))) {
                continue;
            }
            let (points, closed) = walk(start, next, &mut used);
            runs.push(Run { points, closed });
        }
    }
    // What is left are loops.
    let rest: Vec<u32> = adjacent.keys().copied().collect();
    for start in rest {
        let Some(&next) = adjacent[&start]
            .iter()
            .find(|&&n| !used.contains(&(start.min(n), start.max(n))))
        else {
            continue;
        };
        let (points, closed) = walk(start, next, &mut used);
        runs.push(Run { points, closed });
    }
    runs
}

/// The Align tools' runs from the selection, and in Polygon mode the inner
/// points of the selected polygons (DEFORM Radial / Linear):
/// - Edge mode: the selected edges joined into runs.
/// - Vertex mode: the selected points joined along the mesh's edges; points
///   that share no edge are one open run in selection order.
/// - Polygon mode: the border of the selected polygons; the points inside it
///   are returned separately.
pub fn align_runs(mesh: &Mesh, scene_selection: &crate::Selection) -> (Vec<Run>, Vec<u32>) {
    match scene_selection.mode {
        Mode::Edge => (runs_from_edges(&scene_selection.edges), Vec::new()),
        Mode::Vertex => {
            let chosen: BTreeSet<u32> = scene_selection.vertices.iter().copied().collect();
            let edges: Vec<(u32, u32)> = mesh
                .edges()
                .into_iter()
                .filter(|(a, b)| chosen.contains(a) && chosen.contains(b))
                .collect();
            if edges.is_empty() {
                let points = scene_selection.vertices.clone();
                return (
                    if points.len() >= 2 {
                        vec![Run {
                            points,
                            closed: false,
                        }]
                    } else {
                        Vec::new()
                    },
                    Vec::new(),
                );
            }
            (runs_from_edges(&edges), Vec::new())
        }
        Mode::Polygon => {
            let faces: BTreeSet<u32> = scene_selection.faces.iter().copied().collect();
            let mut count: BTreeMap<(u32, u32), u32> = BTreeMap::new();
            let mut inside: BTreeSet<u32> = BTreeSet::new();
            for &f in &faces {
                let Some(face) = mesh.faces.get(f as usize) else {
                    continue;
                };
                let n = face.vertices.len();
                for i in 0..n {
                    let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
                    *count.entry((a.min(b), a.max(b))).or_default() += 1;
                    inside.insert(a);
                }
            }
            let border: Vec<(u32, u32)> = count
                .into_iter()
                .filter(|&(_, c)| c == 1)
                .map(|(e, _)| e)
                .collect();
            let runs = runs_from_edges(&border);
            let on_border: BTreeSet<u32> = runs.iter().flat_map(|r| r.points.clone()).collect();
            let inner = inside.difference(&on_border).copied().collect();
            (runs, inner)
        }
        Mode::Item => (Vec::new(), Vec::new()),
    }
}

/// The plane normal of an ordered run (Newell's method), falling back to the
/// direction of least spread for a straight run.
fn run_normal(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        n += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    if n.length() > 1e-12 {
        return n.normalize();
    }
    let c = points.iter().sum::<DVec3>() / points.len().max(1) as f64;
    let along = points
        .iter()
        .map(|&p| p - c)
        .max_by(|a, b| a.length().total_cmp(&b.length()))
        .unwrap_or(DVec3::X)
        .normalize_or_zero();
    along.any_orthonormal_vector()
}

// --------------------------------------------------------- Radial Align

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadialMode {
    /// A full circle (DP-10).
    #[default]
    Circle,
    /// An arc of Central Angle from Start Angle (DP-10).
    Arc,
    /// A regular polygon of Sides corners in the circle (DP-13).
    NSided,
}
impl RadialMode {
    pub const ALL: [Self; 3] = [Self::Circle, Self::Arc, Self::NSided];
    pub fn label(self) -> &'static str {
        match self {
            Self::Circle => "Circle",
            Self::Arc => "Arc",
            Self::NSided => "N-Sided",
        }
    }
}

/// Radial Align's options; starting values from DP "Starting values".
/// Angles are degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadialAlignOptions {
    pub mode: RadialMode,
    pub sides: u32,
    /// N-Sided: turns the corners about the centre.
    pub rotate: f64,
    /// 0 = measured from the points (DP-11).
    pub radius: f64,
    pub start_angle: f64,
    /// Arc mode only (DP-10); 360° in use (RA-Q2 answered).
    pub central_angle: f64,
    /// 0..1: blend from the start (0) to the aligned position (1) (DP-14).
    pub weight: f64,
    /// Polygon mode: relax the inner points this many passes (DP-15).
    pub smoothing_iterations: u32,
    /// Polygon mode: put the inner points on the circle's plane first.
    pub flatten: bool,
    /// 0 = round; positive squares the circle off, negative pinches it.
    pub exponent: f64,
}
impl Default for RadialAlignOptions {
    fn default() -> Self {
        Self {
            mode: RadialMode::Circle,
            sides: 5,
            rotate: 0.,
            radius: 0.,
            start_angle: 0.,
            central_angle: 360.,
            weight: 1.,
            smoothing_iterations: 50,
            flatten: true,
            exponent: 0.,
        }
    }
}
impl RadialAlignOptions {
    pub fn validate(&self) -> Result<()> {
        let finite = [
            self.rotate,
            self.radius,
            self.start_angle,
            self.central_angle,
            self.weight,
            self.exponent,
        ];
        if finite.iter().any(|v| !v.is_finite()) {
            return Err("Radial Align values must be finite numbers".into());
        }
        if !(3..=1000).contains(&self.sides) {
            return Err("Radial Align Sides must be a whole number from 3 to 1000".into());
        }
        if self.radius < 0. {
            return Err("Radial Align Radius can't be negative (0 measures it)".into());
        }
        if self.smoothing_iterations > 10_000 {
            return Err("Smoothing Iterations must be 10000 or fewer".into());
        }
        if !(-4. ..=4.).contains(&self.exponent) {
            return Err("Exponent must be between -4 and 4".into());
        }
        Ok(())
    }
}

/// Where each point of one run lands on the circle, polygon or arc.
fn radial_targets(points: &[DVec3], closed: bool, o: &RadialAlignOptions) -> Vec<DVec3> {
    let n = points.len();
    let center = points.iter().sum::<DVec3>() / n as f64;
    let measured = points.iter().map(|p| p.distance(center)).sum::<f64>() / n as f64;
    let radius = if o.radius > 0. { o.radius } else { measured };
    let normal = run_normal(points);
    // DP-12 / DP-Q1 (provisional): a loop keeps its point farthest from the
    // centre at its angle; an open run keeps its first point.
    let anchor = if closed {
        (0..n)
            .max_by(|&a, &b| {
                points[a]
                    .distance_squared(center)
                    .total_cmp(&points[b].distance_squared(center))
            })
            .unwrap_or(0)
    } else {
        0
    };
    let flat = |p: DVec3| (p - center) - normal * (p - center).dot(normal);
    let u = {
        let d = flat(points[anchor]).normalize_or_zero();
        if d == DVec3::ZERO {
            normal.any_orthonormal_vector()
        } else {
            d
        }
    };
    let v = normal.cross(u);
    // Keep the run's own winding (DP-12: points keep their order).
    let mut area = 0.;
    for i in 0..n {
        let (a, b) = (flat(points[i]), flat(points[(i + 1) % n]));
        area += a.dot(u) * b.dot(v) - a.dot(v) * b.dot(u);
    }
    let dir = if area < 0. { -1. } else { 1. };
    let tau = std::f64::consts::TAU;
    let start = o.start_angle.to_radians();
    let at = |theta: f64, r: f64| center + (u * theta.cos() + v * theta.sin()) * r * 1.;
    let shaped = |theta: f64| {
        if o.exponent == 0. {
            radius
        } else {
            // A superellipse: q = 2 is the circle.
            let q = 2. * 2f64.powf(o.exponent);
            radius / (theta.cos().abs().powf(q) + theta.sin().abs().powf(q)).powf(1. / q)
        }
    };
    let mut out = vec![DVec3::ZERO; n];
    match o.mode {
        RadialMode::Circle | RadialMode::Arc => {
            let step = if o.mode == RadialMode::Circle {
                tau / n as f64
            } else if closed {
                o.central_angle.to_radians() / n as f64
            } else {
                o.central_angle.to_radians() / (n - 1).max(1) as f64
            };
            for k in 0..n {
                let index = (anchor + k) % n;
                let theta = dir * (start + k as f64 * step);
                out[index] = at(theta, shaped(theta));
            }
        }
        RadialMode::NSided => {
            let sides = o.sides as usize;
            let corner = |j: usize| {
                let theta = dir * (start + o.rotate.to_radians() + j as f64 * tau / sides as f64);
                at(theta, radius)
            };
            let mut slots = Vec::with_capacity(n);
            if n >= sides {
                // DP-13: every corner gets a point; the extra points go on
                // the last sides from the anchor, evenly along each.
                let (base, extra) = (n / sides, n % sides);
                for side in 0..sides {
                    let count = base + usize::from(side >= sides - extra);
                    let (a, b) = (corner(side), corner(side + 1));
                    for i in 0..count {
                        slots.push(a + (b - a) * (i as f64 / count as f64));
                    }
                }
            } else {
                // Fewer points than sides: spread over the corners
                // (DFF-Q04, provisional).
                for k in 0..n {
                    slots.push(corner((k * sides + n / 2) / n % sides));
                }
            }
            for k in 0..n {
                out[(anchor + k) % n] = slots[k];
            }
        }
    }
    out
}

/// Radial Align (DP-10..15): each run onto its own circle, arc or polygon.
/// In Polygon mode `inner` points are flattened onto the plane and relaxed
/// between the aligned border. `weights` (falloff) scale Weight per point.
pub fn radial_align_geometry(
    mesh: &Mesh,
    runs: &[Run],
    inner: &[u32],
    o: &RadialAlignOptions,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    o.validate()?;
    if runs.iter().all(|r| r.points.len() < 3) {
        return Err(
            "Radial Align needs at least three connected points; select an edge loop, points or polygons"
                .into(),
        );
    }
    let k = |v: u32| o.weight * weights.map_or(1., |w| w.get(v as usize).copied().unwrap_or(0.));
    let mut out = mesh.clone();
    let mut fixed = vec![false; mesh.vertices.len()];
    let mut plane: Option<(DVec3, DVec3)> = None;
    for run in runs.iter().filter(|r| r.points.len() >= 3) {
        let pos: Vec<DVec3> = run
            .points
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let targets = radial_targets(&pos, run.closed, o);
        if plane.is_none() {
            let c = pos.iter().sum::<DVec3>() / pos.len() as f64;
            plane = Some((c, run_normal(&pos)));
        }
        for (&v, t) in run.points.iter().zip(targets) {
            let p = mesh.vertices[v as usize];
            out.vertices[v as usize] = p + (t - p) * k(v);
            fixed[v as usize] = true;
        }
    }
    if !inner.is_empty() {
        relax_inner(
            &mut out,
            mesh,
            inner,
            &fixed,
            o.flatten.then_some(plane).flatten(),
            o.smoothing_iterations,
            &k,
        );
    }
    Ok(out)
}

/// Polygon-mode inner points: flatten onto `plane`, then relax between the
/// fixed border for `iterations` passes; the weight blends the result.
fn relax_inner(
    out: &mut Mesh,
    base: &Mesh,
    inner: &[u32],
    fixed: &[bool],
    plane: Option<(DVec3, DVec3)>,
    iterations: u32,
    weight: &dyn Fn(u32) -> f64,
) {
    let mut work = out.vertices.clone();
    if let Some((c, n)) = plane {
        for &v in inner {
            let p = work[v as usize];
            work[v as usize] = p - n * (p - c).dot(n);
        }
    }
    let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let set: BTreeSet<u32> = inner.iter().copied().collect();
    for (a, b) in base.edges() {
        if set.contains(&a) {
            neighbours.entry(a).or_default().push(b);
        }
        if set.contains(&b) {
            neighbours.entry(b).or_default().push(a);
        }
    }
    for _ in 0..iterations {
        let before = work.clone();
        for (&v, ring) in &neighbours {
            if fixed[v as usize] {
                continue;
            }
            work[v as usize] =
                ring.iter().map(|&u| before[u as usize]).sum::<DVec3>() / ring.len() as f64;
        }
    }
    for &v in inner {
        let p = base.vertices[v as usize];
        out.vertices[v as usize] = p + (work[v as usize] - p) * weight(v);
    }
}

// --------------------------------------------------------- Linear Align

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinearMode {
    /// A straight line between the run's ends.
    #[default]
    Line,
    /// A smooth curve through the run (DFF-Q05, provisional).
    Curve,
}
impl LinearMode {
    pub const ALL: [Self; 2] = [Self::Line, Self::Curve];
    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "Line",
            Self::Curve => "Curve",
        }
    }
}

/// Linear Align's options; starting values from DP "Starting values".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearAlignOptions {
    pub mode: LinearMode,
    /// Even spacing (DP-16); off, each point moves to its nearest spot on
    /// the line (DP-17).
    pub uniform: bool,
    pub weight: f64,
    /// Curve mode: smoothing passes.
    pub smoothing_iterations: u32,
    pub flatten: bool,
    /// Resize every open run to `edge_length` (0: the last run's length).
    pub align_length: bool,
    pub edge_length: f64,
    /// Every run takes the first run's direction.
    pub align_first: bool,
    /// Straighten as seen along `axis`, keeping depth.
    pub screen_align: bool,
    pub axis: DVec3,
    /// Curve mode: keep the run's own shape, only respace along it.
    pub preserve_curve: bool,
}
impl Default for LinearAlignOptions {
    fn default() -> Self {
        Self {
            mode: LinearMode::Line,
            uniform: true,
            weight: 1.,
            smoothing_iterations: 50,
            flatten: true,
            align_length: false,
            edge_length: 0.,
            align_first: false,
            screen_align: false,
            axis: DVec3::Z,
            preserve_curve: false,
        }
    }
}
impl LinearAlignOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.weight.is_finite() || !self.edge_length.is_finite() || !self.axis.is_finite() {
            return Err("Linear Align values must be finite numbers".into());
        }
        if self.edge_length < 0. {
            return Err("Edge Length can't be negative".into());
        }
        if self.smoothing_iterations > 10_000 {
            return Err("Smoothing Iterations must be 10000 or fewer".into());
        }
        if self.screen_align && self.axis.length() < 1e-12 {
            return Err("Screen Align needs a non-zero Axis Vector".into());
        }
        Ok(())
    }
}

/// Points at equal distances along the polyline `p`, ends kept.
fn resample(p: &[DVec3]) -> Vec<DVec3> {
    let n = p.len();
    let mut lengths = vec![0.; n];
    for i in 1..n {
        lengths[i] = lengths[i - 1] + p[i].distance(p[i - 1]);
    }
    let total = lengths[n - 1];
    if total <= 1e-15 {
        return p.to_vec();
    }
    (0..n)
        .map(|i| {
            let s = total * i as f64 / (n - 1) as f64;
            let j = lengths.partition_point(|&l| l < s).clamp(1, n - 1);
            let span = lengths[j] - lengths[j - 1];
            let t = if span > 0. {
                (s - lengths[j - 1]) / span
            } else {
                0.
            };
            p[j - 1].lerp(p[j], t.clamp(0., 1.))
        })
        .collect()
}

/// Linear Align (DP-16..18): each open run's points onto the line (or curve)
/// between its two end points. Closed runs have no ends and are refused
/// (DFF-Q06, provisional).
pub fn linear_align_geometry(
    mesh: &Mesh,
    runs: &[Run],
    o: &LinearAlignOptions,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    o.validate()?;
    let open: Vec<&Run> = runs
        .iter()
        .filter(|r| !r.closed && r.points.len() >= 2)
        .collect();
    if open.is_empty() {
        return Err(if runs.iter().any(|r| r.closed) {
            "Linear Align lines up runs between their two ends; a closed loop has none. Select an open run of edges or points".into()
        } else {
            "Linear Align needs a run of at least two connected points".into()
        });
    }
    let k = |v: u32| o.weight * weights.map_or(1., |w| w.get(v as usize).copied().unwrap_or(0.));
    let ends = |r: &Run| {
        (
            mesh.vertices[r.points[0] as usize],
            mesh.vertices[*r.points.last().unwrap() as usize],
        )
    };
    let first_dir = {
        let (a, b) = ends(open[0]);
        (b - a).normalize_or_zero()
    };
    let target_length = if o.edge_length > 0. {
        o.edge_length
    } else {
        let (a, b) = ends(open[open.len() - 1]);
        a.distance(b)
    };
    let axis = o.axis.normalize_or_zero();
    let mut out = mesh.clone();
    for run in open {
        let pos: Vec<DVec3> = run
            .points
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let n = pos.len();
        let (a, b) = (pos[0], pos[n - 1]);
        let mut targets: Vec<DVec3> = match o.mode {
            LinearMode::Line => {
                let d = b - a;
                let len2 = d.length_squared();
                (0..n)
                    .map(|i| {
                        if o.uniform || len2 <= 1e-24 {
                            a + d * (i as f64 / (n - 1) as f64)
                        } else {
                            a + d * ((pos[i] - a).dot(d) / len2)
                        }
                    })
                    .collect()
            }
            LinearMode::Curve => {
                let mut c = pos.clone();
                if !o.preserve_curve {
                    for _ in 0..o.smoothing_iterations {
                        let before = c.clone();
                        for i in 1..n - 1 {
                            c[i] = before[i] * 0.5 + (before[i - 1] + before[i + 1]) * 0.25;
                        }
                    }
                }
                if o.uniform { resample(&c) } else { c }
            }
        };
        // Direction and length options act on the aligned run about A.
        let span = b - a;
        if (o.align_first || o.align_length) && span.length() > 1e-12 {
            let dir = if o.align_first {
                first_dir
            } else {
                span.normalize()
            };
            let length = if o.align_length {
                target_length
            } else {
                span.length()
            };
            let rot = glam::DQuat::from_rotation_arc(span.normalize(), dir);
            let scale = length / span.length();
            for t in &mut targets {
                *t = a + rot * ((*t - a) * scale);
            }
        }
        for (i, &v) in run.points.iter().enumerate() {
            let p = pos[i];
            let mut d = targets[i] - p;
            if o.screen_align && axis != DVec3::ZERO {
                d -= axis * d.dot(axis);
            }
            out.vertices[v as usize] = p + d * k(v);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------ Relax Mesh

/// Relax Mesh... (DP-20..22): Iterations 5, Constrain off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelaxMeshOptions {
    pub iterations: u32,
    pub constrain: bool,
}
impl Default for RelaxMeshOptions {
    fn default() -> Self {
        Self {
            iterations: 5,
            constrain: false,
        }
    }
}
impl RelaxMeshOptions {
    pub fn validate(&self) -> Result<()> {
        if !(1..=1000).contains(&self.iterations) {
            return Err("Relax Mesh Iterations must be a whole number from 1 to 1000".into());
        }
        Ok(())
    }
}

/// Constrain off: each pass moves a point a fifth of the way to its
/// neighbours' average (fitted to DP-RM-1). Constrain on: the points settle
/// against the fixed border (DP-21, RM-Q1 provisional): full passes until
/// nothing moves, up to 200 per Iteration.
const RELAX_GENTLE: f64 = 0.2;
const RELAX_SOLVE_PASSES: u32 = 200;

/// Relax `points` (DP-20: open-border points never move); one result.
pub fn relax_mesh_geometry(
    mesh: &Mesh,
    points: &BTreeSet<u32>,
    o: &RelaxMeshOptions,
) -> Result<Mesh> {
    o.validate()?;
    let n = mesh.vertices.len();
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (a, b) in mesh.edges() {
        neighbours[a as usize].push(b);
        neighbours[b as usize].push(a);
    }
    let mut border = vec![false; n];
    for (&(a, b), faces) in mesh.edge_faces().iter() {
        if faces.len() == 1 {
            border[a as usize] = true;
            border[b as usize] = true;
        }
    }
    let free: Vec<usize> = points
        .iter()
        .map(|&v| v as usize)
        .filter(|&v| v < n && !border[v] && !neighbours[v].is_empty())
        .collect();
    let mut out = mesh.clone();
    let (lambda, passes) = if o.constrain {
        (1., o.iterations * RELAX_SOLVE_PASSES)
    } else {
        (RELAX_GENTLE, o.iterations)
    };
    for _ in 0..passes {
        let before = out.vertices.clone();
        let mut moved = 0f64;
        for &v in &free {
            let ring = &neighbours[v];
            let average =
                ring.iter().map(|&u| before[u as usize]).sum::<DVec3>() / ring.len() as f64;
            let next = before[v] + (average - before[v]) * lambda;
            moved = moved.max(next.distance_squared(before[v]));
            out.vertices[v] = next;
        }
        if o.constrain && moved < 1e-24 {
            break;
        }
    }
    Ok(out)
}

/// The points Relax Mesh works on: the selection's, or the whole mesh.
pub fn relax_points(scene: &Scene) -> Result<BTreeSet<u32>> {
    crate::transform_vertices(scene)
}

/// One preview of a Deform finish tool on the scene's active mesh: the
/// selection's points (whole mesh when nothing is selected, DP-00) or runs.
pub fn tool_geometry(
    tool: crate::Tool,
    scene: &Scene,
    p: &crate::Parameters,
    weights: Option<&[f64]>,
) -> Result<Mesh> {
    use crate::Tool;
    let item = scene.active().ok_or("Select a mesh")?;
    let mesh = &item.mesh;
    match tool {
        Tool::Jitter => {
            jitter_geometry(mesh, &crate::transform_vertices(scene)?, &p.jitter, weights)
        }
        Tool::Quantize => quantize_geometry(
            mesh,
            &crate::transform_vertices(scene)?,
            &p.quantize,
            scene.world_matrix(item.id)?,
            weights,
        ),
        Tool::RadialAlign | Tool::LinearAlign => {
            if scene.selection.mode == Mode::Item {
                return Err(format!(
                    "{} works on points, edges or polygons; pick a component mode first",
                    tool.label()
                ));
            }
            let (runs, inner) = align_runs(mesh, &scene.selection);
            if runs.is_empty() {
                return Err(format!(
                    "{}: select an edge loop or run, connected points, or polygons first",
                    tool.label()
                ));
            }
            if tool == Tool::RadialAlign {
                radial_align_geometry(mesh, &runs, &inner, &p.radial_align, weights)
            } else {
                linear_align_geometry(mesh, &runs, &p.linear_align, weights)
            }
        }
        _ => Err(format!("{} isn't a Deform finish tool", tool.label())),
    }
}

impl crate::Editor {
    /// Relax Mesh... (DP-22): one undo step; refused in Item mode or with
    /// no mesh, and changes nothing when nothing would move.
    pub(crate) fn relax_mesh(&mut self, options: RelaxMeshOptions) -> Result<()> {
        options.validate()?;
        if self.session.live.is_some() {
            return Err("Drop the active tool before Relax Mesh".into());
        }
        if self.scene.selection.mode == Mode::Item {
            return Err(
                "Relax Mesh works on points, edges or polygons; pick a component mode first".into(),
            );
        }
        *self
            .presets
            .get_mut::<crate::deform_finish::RelaxMeshPreset>() = options;
        let points = relax_points(&self.scene)?;
        let item = self.scene.active().ok_or("Select a mesh for Relax Mesh")?;
        let id = item.id;
        let mut mesh = relax_mesh_geometry(&item.mesh, &points, &options)?;
        let symmetry = self.active_symmetry();
        let primary = self.symmetry_primary(&self.scene);
        symmetry.mirror_moved(&item.mesh, &mut mesh, primary);
        if mesh == *item.mesh {
            return Ok(());
        }
        let before = self.scene.clone();
        self.scene.item_mut(id).ok_or("Missing mesh")?.mesh = mesh.into();
        self.record("Relax Mesh".into(), before);
        Ok(())
    }
}

/// Relax Mesh's last-used options (UIC-95).
pub enum RelaxMeshPreset {}
impl crate::PresetKey for RelaxMeshPreset {
    type Value = RelaxMeshOptions;
}
