//! Torus, Capsule, Arc and Tube: the four numeric primitives of
//! `specifications/new-primitives-2026-11-03` (NPR-*). Each has its own
//! parameters (with the spec's Reset values as `Default`) and a generator
//! that returns new geometry; the shared numeric session in
//! `shape_editor.rs` appends it to the active mesh.
//!
//! Angles around an axis follow NPR's convention: measured from the first
//! perpendicular axis toward the second, starting at -90° and decreasing,
//! so the first point of a ring sits on the negative second axis.
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Which of the four primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Torus,
    Capsule,
    Arc,
    Tube,
    /// Create panel Text (text.rs); it shares this numeric session.
    Text,
    /// Teapot (teapot.rs, MT6); numeric too.
    Teapot,
}
impl ShapeKind {
    pub const ALL: [ShapeKind; 6] = [
        Self::Torus,
        Self::Capsule,
        Self::Arc,
        Self::Tube,
        Self::Text,
        Self::Teapot,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Torus => "Torus",
            Self::Capsule => "Capsule",
            Self::Arc => "Arc",
            Self::Tube => "Tube",
            Self::Text => "Text",
            Self::Teapot => "Teapot",
        }
    }
}

/// One primitive's settings.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeParameters {
    Torus(TorusParameters),
    Capsule(CapsuleParameters),
    Arc(ArcParameters),
    Tube(TubeParameters),
    Text(crate::TextParameters),
    Teapot(crate::teapot::TeapotParameters),
}
impl ShapeParameters {
    pub fn reset(kind: ShapeKind) -> Self {
        match kind {
            ShapeKind::Torus => Self::Torus(Default::default()),
            ShapeKind::Capsule => Self::Capsule(Default::default()),
            ShapeKind::Arc => Self::Arc(Default::default()),
            ShapeKind::Tube => Self::Tube(Default::default()),
            ShapeKind::Text => Self::Text(Default::default()),
            ShapeKind::Teapot => Self::Teapot(Default::default()),
        }
    }
    pub fn kind(&self) -> ShapeKind {
        match self {
            Self::Torus(_) => ShapeKind::Torus,
            Self::Capsule(_) => ShapeKind::Capsule,
            Self::Arc(_) => ShapeKind::Arc,
            Self::Tube(_) => ShapeKind::Tube,
            Self::Text(_) => ShapeKind::Text,
            Self::Teapot(_) => ShapeKind::Teapot,
        }
    }
    pub fn geometry(&self) -> Result<Mesh> {
        match self {
            Self::Torus(p) => torus_geometry(p),
            Self::Capsule(p) => capsule_geometry(p),
            Self::Arc(p) => arc_geometry(p),
            Self::Tube(p) => tube_geometry(p),
            Self::Text(p) => crate::text::text_geometry(p),
            Self::Teapot(p) => crate::teapot::teapot_geometry(p),
        }
    }
}

/// The last settings of each primitive, offered when it is used again.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShapePresets {
    pub torus: TorusParameters,
    pub capsule: CapsuleParameters,
    pub arc: ArcParameters,
    pub tube: TubeParameters,
    pub text: crate::TextParameters,
    pub teapot: crate::teapot::TeapotParameters,
}
impl ShapePresets {
    pub fn get(&self, kind: ShapeKind) -> ShapeParameters {
        match kind {
            ShapeKind::Torus => ShapeParameters::Torus(self.torus.clone()),
            ShapeKind::Capsule => ShapeParameters::Capsule(self.capsule.clone()),
            ShapeKind::Arc => ShapeParameters::Arc(self.arc.clone()),
            ShapeKind::Tube => ShapeParameters::Tube(self.tube.clone()),
            ShapeKind::Text => ShapeParameters::Text(self.text.clone()),
            ShapeKind::Teapot => ShapeParameters::Teapot(self.teapot.clone()),
        }
    }
    pub fn set(&mut self, p: ShapeParameters) {
        match p {
            ShapeParameters::Torus(p) => self.torus = p,
            ShapeParameters::Capsule(p) => self.capsule = p,
            ShapeParameters::Arc(p) => self.arc = p,
            ShapeParameters::Tube(p) => self.tube = p,
            ShapeParameters::Text(p) => self.text = p,
            ShapeParameters::Teapot(p) => self.teapot = p,
        }
    }
}

// Shared limits: the same coordinate budget and preview face budget as the
// other numeric primitives.
const COORDINATE_LIMIT: f64 = 1e9;
const SIZE_LIMIT: f64 = 1e8;
const FACE_BUDGET: u64 = 250_000;

fn check_budget(name: &str, faces: u64) -> Result<()> {
    if faces > FACE_BUDGET {
        return Err(format!(
            "{name} exceeds the 250,000-polygon preview budget; reduce Sides/Segments"
        ));
    }
    Ok(())
}
fn finite_size(name: &str, values: &[f64]) -> Result<()> {
    if values
        .iter()
        .any(|v| !v.is_finite() || v.abs() > SIZE_LIMIT)
    {
        return Err(format!(
            "{name} values must be finite and within the size budget"
        ));
    }
    Ok(())
}
fn finite_position(name: &str, p: DVec3) -> Result<()> {
    if !p.is_finite() || p.abs().max_element() > COORDINATE_LIMIT {
        return Err(format!(
            "{name} position must be finite and within the coordinate budget"
        ));
    }
    Ok(())
}

/// Superellipse shaping shared with Ellipsoid's bulge: exponent 2/(2+bulge).
/// Bulge 0 is a plain circle (NPR-TOR-05, NPR-CAP-04 data).
fn signed_power(x: f64, exponent: f64) -> f64 {
    if x == 0. {
        0.
    } else {
        x.signum() * x.abs().powf(exponent)
    }
}
fn bulge_exponent(bulge: f64) -> f64 {
    2. / (2. + bulge)
}
fn clamp_bulge(b: f64) -> f64 {
    // Same lower bound as Ellipsoid so the exponent stays positive.
    if b.is_finite() { b.max(-1.5) } else { b }
}

/// Place a point given its height along `axis` and its two perpendicular
/// coordinates: `u` (cosine side) and `v` (sine side). For Axis Y, u is X
/// and v is Z; the mapping turns cyclically for X and Z (NPR-TOR-04).
fn on_axis(axis: usize, h: f64, u: f64, v: f64) -> DVec3 {
    let mut p = DVec3::ZERO;
    p[axis] = h;
    p[(axis + 1) % 3] = v;
    p[(axis + 2) % 3] = u;
    p
}
/// Ring angle of point `i` of `n`: -90° first, then decreasing.
fn ring_angle(i: u32, n: u32) -> f64 {
    -FRAC_PI_2 - TAU * f64::from(i) / f64::from(n)
}

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Validate a generated mesh. Polygons that collapse to zero area (a hole
/// closed to the axis, a 180° Arc of one segment) are kept as explicit
/// degenerate records rather than refused, so counts and order stay as the
/// settings say.
fn finish(mut mesh: Mesh, uvs: Option<Vec<Vec<Option<DVec2>>>>) -> Result<Mesh> {
    for fi in 0..mesh.faces.len() {
        if mesh.normal(fi) == DVec3::ZERO {
            mesh.attributes.degenerate_faces.insert(fi as u32);
        }
    }
    if let Some(uvs) = uvs {
        if uvs.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return Err("Primitive produced nonfinite UVs".into());
        }
        mesh.uv_maps.insert(crate::PRIMITIVE_UV_MAP.into(), uvs);
    }
    mesh.validate()?;
    Ok(mesh)
}

// ---------------------------------------------------------------- Torus

/// NPR Torus. Ring Radius, Cross Section and Height are the stored sizes;
/// Radius X/Y/Z and Hole Size are derived and linked (NPR-TOR-03).
#[derive(Clone, Debug, PartialEq)]
pub struct TorusParameters {
    pub position: DVec3,
    pub sides: u32,
    pub segments: u32,
    pub ring_radius: f64,
    pub cross_section: f64,
    pub height: f64,
    pub bulge_top: f64,
    pub bulge_side: f64,
    pub axis: usize,
    pub make_uvs: bool,
}
impl Default for TorusParameters {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            sides: 24,
            segments: 12,
            ring_radius: 0.5,
            cross_section: 0.25,
            height: 0.25,
            bulge_top: 0.,
            bulge_side: 0.,
            axis: 1,
            make_uvs: true,
        }
    }
}
impl TorusParameters {
    /// Radius along the axis is Height; the two across it are Cross Section.
    pub fn radius(&self) -> DVec3 {
        let mut r = DVec3::splat(self.cross_section);
        r[self.axis.min(2)] = self.height;
        r
    }
    /// Hole Size = Ring Radius ÷ Cross Section (0 when Cross Section is 0).
    pub fn hole(&self) -> f64 {
        if self.cross_section == 0. {
            0.
        } else {
            self.ring_radius / self.cross_section
        }
    }
    /// Setting Hole Size keeps Cross Section and moves Ring Radius.
    pub fn set_hole(&mut self, hole: f64) {
        self.ring_radius = hole * self.cross_section;
    }
    /// Setting a Radius component: along the axis it is Height; across it,
    /// Cross Section changes and Hole Size is kept, so Ring Radius follows
    /// (NPR-O-TOR-03-size1).
    pub fn set_radius(&mut self, component: usize, value: f64) {
        if component == self.axis {
            self.height = value;
        } else {
            let hole = self.hole();
            self.cross_section = value;
            self.ring_radius = hole * value;
        }
    }
    /// Bounding values shown as Min/Max: Position ± Radius, as the tool
    /// reports them (not the torus' outer extent).
    pub fn min(&self) -> DVec3 {
        self.position - self.radius()
    }
    pub fn max(&self) -> DVec3 {
        self.position + self.radius()
    }
    pub fn normalized(mut self) -> Self {
        self.sides = self.sides.max(3);
        self.segments = self.segments.max(3);
        self.axis = self.axis.min(2);
        self.bulge_top = clamp_bulge(self.bulge_top);
        self.bulge_side = clamp_bulge(self.bulge_side);
        self
    }
    pub fn validate(&self) -> Result<()> {
        finite_position("Torus", self.position)?;
        finite_size(
            "Torus",
            &[
                self.ring_radius,
                self.cross_section,
                self.height,
                self.bulge_top,
                self.bulge_side,
            ],
        )?;
        if self.ring_radius < 0. || self.cross_section < 0. || self.height < 0. {
            return Err("Torus sizes can't be negative".into());
        }
        if self.ring_radius == 0. && self.cross_section == 0. {
            return Err("Torus needs a Ring Radius or Cross Section above zero".into());
        }
        check_budget(
            "Torus",
            u64::from(self.sides.max(3)) * u64::from(self.segments.max(3)),
        )
    }
}

/// NPR-TOR-01/02 (shape, order), -04 (axis), -05 (bulge; formula
/// provisional, NPR-Q03).
pub fn torus_geometry(parameters: &TorusParameters) -> Result<Mesh> {
    // A first drag's flat base: no height is a flat ring.
    if parameters.height == 0. {
        let (r, c) = (parameters.ring_radius, parameters.cross_section);
        return crate::flat_annulus(
            parameters.position,
            parameters.axis.min(2),
            r - c,
            r + c,
            parameters.sides,
        );
    }
    let p = parameters.clone().normalized();
    p.validate()?;
    let (n, m) = (p.sides, p.segments);
    let et = bulge_exponent(p.bulge_top);
    let es = bulge_exponent(p.bulge_side);
    let mut mesh = Mesh::default();
    mesh.vertices.reserve((n * m) as usize);
    for k in 0..m {
        let phi = TAU * f64::from(k) / f64::from(m);
        let distance = p.ring_radius - p.cross_section * signed_power(phi.cos(), et);
        let h = -p.height * signed_power(phi.sin(), et);
        for i in 0..n {
            let theta = ring_angle(i, n);
            let u = distance * signed_power(theta.cos(), es);
            let v = distance * signed_power(theta.sin(), es);
            mesh.vertices.push(p.position + on_axis(p.axis, h, u, v));
        }
    }
    let at = |k: u32, i: u32| (k % m) * n + i % n;
    let mut uvs = p.make_uvs.then(Vec::new);
    for k in 0..m {
        for i in 0..n {
            mesh.faces.push(face(vec![
                at(k, i),
                at(k, i + 1),
                at(k + 1, i + 1),
                at(k + 1, i),
            ]));
            if let Some(uvs) = &mut uvs {
                // U around the ring, V along the cross-section; the seams
                // get their own corners at 1 (oracle NPR-O-TOR-01 UVs).
                let uv = |i: u32, k: u32| {
                    Some(DVec2::new(
                        f64::from(i) / f64::from(n),
                        f64::from(k) / f64::from(m),
                    ))
                };
                uvs.push(vec![uv(i, k), uv(i + 1, k), uv(i + 1, k + 1), uv(i, k + 1)]);
            }
        }
    }
    finish(mesh, uvs)
}

// ---------------------------------------------------------------- Capsule

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CapsuleEnds {
    /// Triangle fans to a pole point.
    #[default]
    Polar,
    /// Quads only: a grid patch inside the last end ring.
    QuadGrid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapsuleParameters {
    pub position: DVec3,
    /// Half-length along the axis and the radii across it.
    pub radius: DVec3,
    pub sides: u32,
    pub segments: u32,
    pub end_segments: u32,
    /// Height of each rounded end.
    pub end_size: f64,
    pub ends: CapsuleEnds,
    pub bulge: f64,
    pub axis: usize,
    pub make_uvs: bool,
}
impl Default for CapsuleParameters {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            radius: DVec3::new(0.25, 0.75, 0.25),
            sides: 24,
            segments: 1,
            end_segments: 5,
            end_size: 0.25,
            ends: CapsuleEnds::Polar,
            bulge: 0.,
            axis: 1,
            make_uvs: true,
        }
    }
}
impl CapsuleParameters {
    pub fn min(&self) -> DVec3 {
        self.position - self.radius
    }
    pub fn max(&self) -> DVec3 {
        self.position + self.radius
    }
    pub fn normalized(mut self) -> Self {
        self.sides = self.sides.max(3);
        self.segments = self.segments.max(1);
        self.end_segments = self.end_segments.max(1);
        self.axis = self.axis.min(2);
        self.bulge = clamp_bulge(self.bulge);
        self
    }
    /// Give a drawn base its height (`half` = half the total height along
    /// the axis; Jason, 2026-11-06). The rounded ends grow first: each end
    /// is as tall as the base's larger radius (a hemisphere, as on the unit
    /// capsule) and is squashed while the total height is under twice that,
    /// so a short drag gives a rounded, sphere-like shape with no straight
    /// sides. Past that the ends stay full and the straight sides make up
    /// the rest. End Size comes from the base every time, never from the
    /// previous value, so dragging down and up again can't shrink the ends.
    pub fn set_drawn_height(&mut self, half: f64) {
        let a = self.axis.min(2);
        let across = (0..3)
            .filter(|&i| i != a)
            .map(|i| self.radius[i])
            .fold(0., f64::max);
        self.radius[a] = half;
        self.end_size = across.min(half);
    }
    pub fn validate(&self) -> Result<()> {
        finite_position("Capsule", self.position)?;
        finite_size("Capsule", &[self.radius.x, self.radius.y, self.radius.z])?;
        finite_size("Capsule", &[self.end_size, self.bulge])?;
        if self.radius.min_element() <= 0. {
            return Err("Capsule radii must be above zero".into());
        }
        if self.end_size < 0. {
            return Err("Capsule End Size can't be negative".into());
        }
        let rings = u64::from(self.segments.max(1)) + 2 * u64::from(self.end_segments.max(1));
        check_budget("Capsule", u64::from(self.sides.max(3)) * rings)
    }
}

/// Grid cells per side for Quad Grid ends; `None` falls back to Polar ends
/// (Sides not a multiple of 4; provisional, NPR-Q04).
fn quad_grid_cells(sides: u32) -> Option<u32> {
    sides.is_multiple_of(4).then_some(sides / 4)
}

/// Grid coordinate of boundary point `i` for `n` cells per side: point 0
/// is the middle of the low-v side, then the walk goes toward low u
/// (NPR-O-CAP-03-ends1 for 8 sides).
fn grid_boundary(i: u32, n: u32) -> (u32, u32) {
    let s = n.div_ceil(2);
    let mut steps = i;
    // Leg 1: (s,0) down to (0,0).
    if steps <= s {
        return (s - steps, 0);
    }
    steps -= s;
    if steps <= n {
        return (0, steps);
    }
    steps -= n;
    if steps <= n {
        return (steps, n);
    }
    steps -= n;
    if steps <= n {
        return (n, n - steps);
    }
    steps -= n;
    (n - steps, 0)
}

/// NPR-CAP-01..04. Body length 0 (End Size equal to the half-length)
/// collapses the body to one ring (NPR-O-CAP-04-axisX; provisional, NPR-Q09).
pub fn capsule_geometry(parameters: &CapsuleParameters) -> Result<Mesh> {
    // A first drag's flat base: no length along the axis is one disc.
    let a = parameters.axis.min(2);
    if parameters.radius[a] == 0. {
        return crate::flat_ngon(
            parameters.position,
            a,
            parameters.radius[(a + 1) % 3],
            parameters.radius[(a + 2) % 3],
            parameters.sides,
        );
    }
    let p = parameters.clone().normalized();
    p.validate()?;
    let n = p.sides;
    let a = p.axis;
    let half = p.radius[a];
    let ru = p.radius[(a + 2) % 3];
    let rv = p.radius[(a + 1) % 3];
    let end = p.end_size.min(half);
    let body = half - end;
    let eb = bulge_exponent(p.bulge);
    let grid = match p.ends {
        CapsuleEnds::QuadGrid => quad_grid_cells(n),
        CapsuleEnds::Polar => None,
    };
    // Ring levels from bottom to top as (height, scale across the axis).
    let mut rings: Vec<(f64, f64)> = Vec::new();
    let e = p.end_segments;
    let end_ring = |j: u32| {
        let angle = FRAC_PI_2 * f64::from(j) / f64::from(e);
        (body + end * angle.cos(), angle.sin())
    };
    for j in 1..e {
        let (h, s) = end_ring(j);
        rings.push((-h, s));
    }
    if body <= half * 1e-12 {
        rings.push((0., 1.));
    } else {
        for k in 0..=p.segments {
            let t = f64::from(k) / f64::from(p.segments);
            rings.push((-body + 2. * body * t, 1.));
        }
    }
    for j in (1..e).rev() {
        rings.push(end_ring(j));
    }
    let ring_point = |h: f64, s: f64, i: u32| {
        let theta = ring_angle(i, n);
        p.position
            + on_axis(
                a,
                h,
                s * ru * signed_power(theta.cos(), eb),
                s * rv * signed_power(theta.sin(), eb),
            )
    };
    // Interior grid points of one end, from the boundary ring by a Coons
    // patch, lifted onto the rounded end (provisional beyond 8 sides).
    let grid_interior = |cells: u32, ring: &[DVec3], top: bool| -> Vec<DVec3> {
        let local: Vec<DVec2> = ring
            .iter()
            .map(|q| {
                let d = *q - p.position;
                DVec2::new(d[(a + 2) % 3], d[(a + 1) % 3])
            })
            .collect();
        let mut boundary = std::collections::HashMap::new();
        for i in 0..4 * cells {
            boundary.insert(grid_boundary(i, cells), local[i as usize]);
        }
        let b = |x: u32, z: u32| boundary[&(x, z)];
        let nf = f64::from(cells);
        let mut out = Vec::new();
        for x in 1..cells {
            for z in 1..cells {
                let (s, t) = (f64::from(x) / nf, f64::from(z) / nf);
                let q = (1. - t) * b(x, 0) + t * b(x, cells) + (1. - s) * b(0, z) + s * b(cells, z)
                    - ((1. - s) * (1. - t) * b(0, 0)
                        + s * (1. - t) * b(cells, 0)
                        + (1. - s) * t * b(0, cells)
                        + s * t * b(cells, cells));
                let rho2 = ((q.x / ru).powi(2) + (q.y / rv).powi(2)).min(1.);
                let h = body + end * (1. - rho2).sqrt();
                out.push(p.position + on_axis(a, if top { h } else { -h }, q.x, q.y));
            }
        }
        out
    };
    let mut mesh = Mesh::default();
    let mut ring_start = Vec::with_capacity(rings.len());
    let mut bottom_interior = Vec::new();
    let mut top_interior = Vec::new();
    let pole_offset = |h: f64| p.position + on_axis(a, h, 0., 0.);
    let bottom_pole;
    let top_pole;
    match grid {
        None => {
            bottom_pole = Some(mesh.vertices.len() as u32);
            mesh.vertices.push(pole_offset(-half));
            for &(h, s) in &rings {
                ring_start.push(mesh.vertices.len() as u32);
                mesh.vertices.extend((0..n).map(|i| ring_point(h, s, i)));
            }
            top_pole = Some(mesh.vertices.len() as u32);
            mesh.vertices.push(pole_offset(half));
        }
        Some(cells) => {
            bottom_pole = None;
            top_pole = None;
            for (r, &(h, s)) in rings.iter().enumerate() {
                ring_start.push(mesh.vertices.len() as u32);
                let points: Vec<_> = (0..n).map(|i| ring_point(h, s, i)).collect();
                mesh.vertices.extend(points.iter().copied());
                if r == 0 {
                    // The bottom centre comes right after the first ring.
                    let first = mesh.vertices.len() as u32;
                    let inner = grid_interior(cells, &points, false);
                    bottom_interior = (first..first + inner.len() as u32).collect();
                    mesh.vertices.extend(inner);
                }
            }
            let last = rings.len() - 1;
            let points: Vec<_> = (0..n)
                .map(|i| mesh.vertices[(ring_start[last] + i) as usize])
                .collect();
            let first = mesh.vertices.len() as u32;
            let inner = grid_interior(cells, &points, true);
            top_interior = (first..first + inner.len() as u32).collect();
            mesh.vertices.extend(inner);
        }
    }
    let ring = |r: usize, i: u32| ring_start[r] + i % n;
    // UVs: U around (seam copy at 1), V by ring level with the poles at 0
    // and 1 (NPR-O-CAP-01 UVs). Grid ends have no published layout, so their
    // corners are left without UVs (provisional, NPR-Q02).
    let levels = rings.len() as f64 + 1.;
    let uv = |i: u32, level: usize| {
        Some(DVec2::new(
            f64::from(i) / f64::from(n),
            level as f64 / levels,
        ))
    };
    let pole_uv = |i: u32, level: usize| {
        Some(DVec2::new(
            (f64::from(i) + 0.5) / f64::from(n),
            level as f64 / levels,
        ))
    };
    let mut uvs: Vec<Vec<Option<DVec2>>> = Vec::new();
    let last = rings.len() - 1;
    let grid_cells = |mesh: &mut Mesh,
                      uvs: &mut Vec<Vec<Option<DVec2>>>,
                      cells: u32,
                      r: usize,
                      interior: &[u32],
                      top: bool| {
        let mut index = std::collections::HashMap::new();
        for i in 0..4 * cells {
            index.insert(grid_boundary(i, cells), ring(r, i));
        }
        let mut next = interior.iter();
        for x in 1..cells {
            for z in 1..cells {
                index.insert((x, z), *next.next().expect("interior point"));
            }
        }
        for cx in 0..cells {
            for cz in 0..cells {
                let corners = if top {
                    [(cx + 1, cz), (cx, cz), (cx, cz + 1), (cx + 1, cz + 1)]
                } else {
                    [(cx + 1, cz), (cx + 1, cz + 1), (cx, cz + 1), (cx, cz)]
                };
                mesh.faces
                    .push(face(corners.iter().map(|c| index[c]).collect()));
                uvs.push(vec![None; 4]);
            }
        }
    };
    match grid {
        None => {
            let pole = bottom_pole.expect("polar pole");
            for i in 0..n {
                mesh.faces
                    .push(face(vec![ring(0, i), pole, ring(0, i + 1)]));
                uvs.push(vec![uv(i, 1), pole_uv(i, 0), uv(i + 1, 1)]);
            }
        }
        Some(cells) => grid_cells(&mut mesh, &mut uvs, cells, 0, &bottom_interior, false),
    }
    for r in 0..last {
        for i in 0..n {
            mesh.faces.push(face(vec![
                ring(r + 1, i),
                ring(r, i),
                ring(r, i + 1),
                ring(r + 1, i + 1),
            ]));
            uvs.push(vec![
                uv(i, r + 2),
                uv(i, r + 1),
                uv(i + 1, r + 1),
                uv(i + 1, r + 2),
            ]);
        }
    }
    match grid {
        None => {
            let pole = top_pole.expect("polar pole");
            for i in 0..n {
                mesh.faces
                    .push(face(vec![ring(last, i), ring(last, i + 1), pole]));
                uvs.push(vec![
                    uv(i, last + 1),
                    uv(i + 1, last + 1),
                    pole_uv(i, last + 2),
                ]);
            }
        }
        Some(cells) => grid_cells(&mut mesh, &mut uvs, cells, last, &top_interior, true),
    }
    finish(mesh, p.make_uvs.then_some(uvs))
}

// ---------------------------------------------------------------- Arc

/// NPR Arc: a pie-wedge polygon. Stored as Center, the unit direction to
/// Start, the plane normal, Radius and Angle; Start and End are derived so
/// they always agree with Radius and Angle (NPR-ARC-01..05).
#[derive(Clone, Debug, PartialEq)]
pub struct ArcParameters {
    pub center: DVec3,
    pub start_direction: DVec3,
    pub normal: DVec3,
    pub radius: f64,
    /// Degrees, 0..=180 (NPR-ARC-03).
    pub angle: f64,
    pub segments: u32,
    pub reverse: bool,
}
impl Default for ArcParameters {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            start_direction: DVec3::X,
            normal: DVec3::Z,
            radius: 1.,
            angle: 90.,
            segments: 15,
            reverse: false,
        }
    }
}
/// Right-handed rotation of `v` about the unit `axis`.
fn rotate(v: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let (s, c) = angle.sin_cos();
    v * c + axis.cross(v) * s + axis * axis.dot(v) * (1. - c)
}
const ARC_MAX_ANGLE: f64 = 180.;
impl ArcParameters {
    pub fn start(&self) -> DVec3 {
        self.center + self.start_direction * self.radius
    }
    pub fn end(&self) -> DVec3 {
        self.center
            + rotate(self.start_direction, self.normal, self.angle.to_radians()) * self.radius
    }
    /// Angle entries above 180° become 180° (NPR-ARC-03); negative ones 0.
    pub fn set_angle(&mut self, degrees: f64) {
        self.angle = if degrees.is_finite() {
            degrees.clamp(0., ARC_MAX_ANGLE)
        } else {
            degrees
        };
    }
    /// Moving Center keeps Start and End's plane (the plane through the new
    /// Center and the old Start and End), points Start from the new Center
    /// toward the old Start, and keeps Radius and Angle, so End is rebuilt
    /// from them (NPR-O-ARC-05-center; NPR-Q06).
    pub fn set_center(&mut self, center: DVec3) {
        let (start, end) = (self.start(), self.end());
        let normal = (start - center).cross(end - center);
        if normal.length() > 1e-12 {
            self.normal = normal.normalize();
        }
        let direction = start - center;
        if direction.length() > 1e-12 {
            self.start_direction = direction.normalize();
            // Keep the start direction in the (possibly kept) plane.
            self.start_direction = (self.start_direction
                - self.normal * self.normal.dot(self.start_direction))
            .try_normalize()
            .unwrap_or(self.start_direction);
        }
        self.center = center;
    }
    /// Entering Start keeps End's direction: Radius becomes the distance
    /// from Center, and the plane and Angle follow the two directions.
    pub fn set_start(&mut self, start: DVec3) {
        let d = start - self.center;
        if d.length() <= 1e-12 || !d.is_finite() {
            return;
        }
        let end_direction = self.end() - self.center;
        self.radius = d.length();
        self.start_direction = d.normalize();
        self.reorient(end_direction);
    }
    /// Entering End keeps Start and Radius; the plane and Angle follow
    /// (NPR-O-ARC-06-start-end).
    pub fn set_end(&mut self, end: DVec3) {
        let d = end - self.center;
        if d.length() <= 1e-12 || !d.is_finite() {
            return;
        }
        self.reorient(d);
    }
    fn reorient(&mut self, end_direction: DVec3) {
        let normal = self.start_direction.cross(end_direction);
        if normal.length() > 1e-12 * end_direction.length() {
            self.normal = normal.normalize();
        } else {
            // Collinear: keep the plane, but make sure it still contains Start.
            let n = self.normal - self.start_direction * self.start_direction.dot(self.normal);
            self.normal = n
                .try_normalize()
                .unwrap_or_else(|| self.start_direction.any_orthonormal_vector());
        }
        let cos = self
            .start_direction
            .dot(end_direction.normalize())
            .clamp(-1., 1.);
        self.angle = cos.acos().to_degrees().min(ARC_MAX_ANGLE);
    }
    pub fn normalized(mut self) -> Self {
        self.segments = self.segments.max(1);
        if self.angle.is_finite() {
            self.angle = self.angle.clamp(0., ARC_MAX_ANGLE);
        }
        self
    }
    pub fn validate(&self) -> Result<()> {
        finite_position("Arc", self.center)?;
        finite_size("Arc", &[self.radius, self.angle])?;
        if !self.start_direction.is_finite()
            || !self.normal.is_finite()
            || (self.start_direction.length() - 1.).abs() > 1e-6
            || (self.normal.length() - 1.).abs() > 1e-6
        {
            return Err("Arc direction is invalid; use Reset".into());
        }
        if self.radius <= 0. {
            return Err("Arc Radius must be above zero".into());
        }
        if self.angle <= 0. && !self.reverse {
            return Err("Arc Angle must be above zero".into());
        }
        check_budget("Arc", 1)?;
        if self.segments > 100_000 {
            return Err("Arc has too many segments; use 100,000 or fewer".into());
        }
        Ok(())
    }
}

/// NPR-ARC-01..04: corners from Start along the sweep, then Center.
/// Reverse sweeps the rest of the disc the other way round.
pub fn arc_geometry(parameters: &ArcParameters) -> Result<Mesh> {
    let p = parameters.clone().normalized();
    p.validate()?;
    let sweep = if p.reverse {
        -(360. - p.angle)
    } else {
        p.angle
    }
    .to_radians();
    let mut mesh = Mesh::default();
    for k in 0..=p.segments {
        let t = f64::from(k) / f64::from(p.segments);
        mesh.vertices
            .push(p.center + rotate(p.start_direction, p.normal, sweep * t) * p.radius);
    }
    mesh.vertices.push(p.center);
    mesh.faces
        .push(face((0..mesh.vertices.len() as u32).collect()));
    finish(mesh, None)
}

// ---------------------------------------------------------------- Tube

#[derive(Clone, Debug, PartialEq)]
pub struct TubeParameters {
    /// Control points in order; the curve passes through each.
    pub points: Vec<DVec3>,
    pub sides: u32,
    /// Steps per span between consecutive control points.
    pub segments: u32,
    pub radius: f64,
    pub caps: bool,
    pub closed: bool,
    pub by_length: bool,
    /// Degrees of turn from the first ring to the last.
    pub twist: f64,
    pub make_uvs: bool,
}
impl Default for TubeParameters {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            sides: 12,
            segments: 10,
            radius: 0.2,
            caps: true,
            closed: false,
            by_length: false,
            twist: 0.,
            make_uvs: true,
        }
    }
}
impl TubeParameters {
    pub fn normalized(mut self) -> Self {
        self.sides = self.sides.max(3);
        self.segments = self.segments.max(1);
        // Closed tubes have no ends to cap (NPR-O-TUB-04 values).
        if self.closed {
            self.caps = false;
        }
        self
    }
    fn spans(&self) -> usize {
        if self.closed {
            self.points.len()
        } else {
            self.points.len().saturating_sub(1)
        }
    }
    pub fn validate(&self) -> Result<()> {
        let needed = if self.closed { 3 } else { 2 };
        if self.points.len() < needed {
            return Err(if self.closed {
                "Closed Tube needs at least 3 control points".into()
            } else {
                "Tube needs at least 2 control points; add points".into()
            });
        }
        if self.points.len() > 10_000 {
            return Err("Tube has too many control points".into());
        }
        for &q in &self.points {
            finite_position("Tube point", q)?;
        }
        let n = self.points.len();
        for i in 0..self.spans() {
            if (self.points[(i + 1) % n] - self.points[i]).length() <= 1e-12 {
                return Err(
                    "Tube control points next to each other can't be in the same place".into(),
                );
            }
        }
        finite_size("Tube", &[self.radius, self.twist])?;
        if self.radius <= 0. {
            return Err("Tube Radius must be above zero".into());
        }
        let rings = self.spans() as u64 * u64::from(self.segments.max(1)) + 1;
        check_budget("Tube", rings * u64::from(self.sides.max(3)))
    }
}

/// Smooth curve through the control points: one cubic Hermite piece per
/// span. A point's tangent blends its two chords weighted by the opposite
/// chord's length; open ends extrapolate a parabola through the first (or
/// last) three points. Matches NPR-O-TUB-02 and -04 exactly; provisional
/// (NPR-Q08).
struct TubeCurve {
    points: Vec<DVec3>,
    tangents: Vec<DVec3>,
    closed: bool,
}
impl TubeCurve {
    fn new(points: &[DVec3], closed: bool) -> Self {
        let n = points.len();
        let tangents = (0..n)
            .map(|i| {
                let (before, after) = if closed {
                    (points[(i + n - 1) % n], points[(i + 1) % n])
                } else if i == 0 {
                    let before = if n > 2 {
                        3. * points[0] - 3. * points[1] + points[2]
                    } else {
                        2. * points[0] - points[1]
                    };
                    (before, points[1])
                } else if i == n - 1 {
                    let after = if n > 2 {
                        3. * points[n - 1] - 3. * points[n - 2] + points[n - 3]
                    } else {
                        2. * points[n - 1] - points[n - 2]
                    };
                    (points[n - 2], after)
                } else {
                    (points[i - 1], points[i + 1])
                };
                let c_in = points[i] - before;
                let c_out = after - points[i];
                let (l_in, l_out) = (c_in.length(), c_out.length());
                if l_in + l_out == 0. {
                    DVec3::ZERO
                } else {
                    (c_in * l_out + c_out * l_in) / (l_in + l_out)
                }
            })
            .collect();
        Self {
            points: points.to_vec(),
            tangents,
            closed,
        }
    }
    fn spans(&self) -> usize {
        if self.closed {
            self.points.len()
        } else {
            self.points.len() - 1
        }
    }
    /// Position and derivative at curve parameter `u` in 0..=spans.
    fn eval(&self, u: f64) -> (DVec3, DVec3) {
        let n = self.points.len();
        let span = (u.floor() as usize).min(self.spans() - 1);
        let t = u - span as f64;
        let (p0, p1) = (self.points[span], self.points[(span + 1) % n]);
        let (m0, m1) = (self.tangents[span], self.tangents[(span + 1) % n]);
        let (t2, t3) = (t * t, t * t * t);
        let position = p0 * (2. * t3 - 3. * t2 + 1.)
            + m0 * (t3 - 2. * t2 + t)
            + p1 * (-2. * t3 + 3. * t2)
            + m1 * (t3 - t2);
        let derivative = p0 * (6. * t2 - 6. * t)
            + m0 * (3. * t2 - 4. * t + 1.)
            + p1 * (-6. * t2 + 6. * t)
            + m1 * (3. * t2 - 2. * t);
        (position, derivative)
    }
    fn direction(&self, u: f64) -> DVec3 {
        let (p, d) = self.eval(u);
        if let Some(d) = d.try_normalize() {
            return d;
        }
        // A zero tangent (a control point whose chords cancel): look a
        // little along the curve instead.
        let step = 1e-4;
        let end = self.spans() as f64;
        let (a, b) = if u + step <= end {
            (p, self.eval(u + step).0)
        } else {
            (self.eval(u - step).0, p)
        };
        (b - a).try_normalize().unwrap_or(DVec3::Y)
    }
}

/// First ring's reference direction for a tangent. For Y it is +X and for
/// X it is -Z; in between it turns with the tangent's lean (NPR-O-TUB-01,
/// -02, -04, -05). Provisional for tangents with a Z component (NPR-Q08).
fn tube_reference(t: DVec3) -> DVec3 {
    let a = t.cross(DVec3::Z);
    let b = t.cross(a);
    (a * t.y + b * t.x).try_normalize().unwrap_or_else(|| {
        (DVec3::X - t * t.dot(DVec3::X))
            .try_normalize()
            .unwrap_or(DVec3::X)
    })
}

/// NPR-TUB-01..03. By Length spaces rings evenly along the curve; Twist
/// turns ring k by Twist·k/(rings−1) about the curve (provisional, NPR-Q08).
pub fn tube_geometry(parameters: &TubeParameters) -> Result<Mesh> {
    let p = parameters.clone().normalized();
    p.validate()?;
    let curve = TubeCurve::new(&p.points, p.closed);
    let spans = curve.spans();
    let steps = spans * p.segments as usize;
    let rings = if p.closed { steps } else { steps + 1 };
    let mut params: Vec<f64> = (0..rings)
        .map(|k| k as f64 / f64::from(p.segments))
        .collect();
    if p.by_length {
        // Dense samples per span, then equal arc-length steps.
        const SAMPLES: usize = 64;
        let total = spans * SAMPLES;
        let us: Vec<f64> = (0..=total).map(|k| k as f64 / SAMPLES as f64).collect();
        let mut lengths = vec![0.];
        let mut previous = curve.eval(0.).0;
        for &u in &us[1..] {
            let q = curve.eval(u).0;
            lengths.push(lengths.last().unwrap() + (q - previous).length());
            previous = q;
        }
        let full = *lengths.last().unwrap();
        for (k, value) in params.iter_mut().enumerate() {
            let s = full * k as f64 / steps as f64;
            let j = lengths
                .partition_point(|&l| l <= s)
                .saturating_sub(1)
                .min(total - 1);
            let span = lengths[j + 1] - lengths[j];
            let f = if span > 0. {
                (s - lengths[j]) / span
            } else {
                0.
            };
            *value = us[j] + f * (us[j + 1] - us[j]);
        }
    }
    let centres: Vec<DVec3> = params.iter().map(|&u| curve.eval(u).0).collect();
    let tangents: Vec<DVec3> = params.iter().map(|&u| curve.direction(u)).collect();
    // Rotation-minimizing frames from the first ring's reference.
    let mut reference = tube_reference(tangents[0]);
    let n = p.sides;
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(rings * n as usize);
    let twist = p.twist.to_radians();
    let twist_steps = if p.closed { rings } else { rings - 1 }.max(1) as f64;
    for k in 0..rings {
        let t = tangents[k];
        if k > 0 {
            let previous = tangents[k - 1];
            let axis = previous.cross(t);
            if let Some(axis) = axis.try_normalize() {
                let angle = previous.dot(t).clamp(-1., 1.).acos();
                reference = rotate(reference, axis, angle);
            }
            reference = (reference - t * t.dot(reference))
                .try_normalize()
                .unwrap_or_else(|| t.any_orthonormal_vector());
        }
        let turn = twist * k as f64 / twist_steps;
        for i in 0..n {
            let angle = TAU * f64::from(i) / f64::from(n) + turn;
            mesh.vertices
                .push(centres[k] + rotate(reference, t, angle) * p.radius);
        }
    }
    let at = |k: usize, i: u32| (k % rings) as u32 * n + i % n;
    let mut uvs = Vec::new();
    let v_of = |k: usize| k as f64 / if p.closed { rings } else { rings - 1 } as f64;
    let wall_uv = |i: u32, k: usize| Some(DVec2::new(f64::from(i) / f64::from(n), v_of(k)));
    if p.caps {
        mesh.faces
            .push(face((0..n).map(|i| at(0, n - i)).collect()));
        uvs.push(vec![None; n as usize]);
    }
    let bands = if p.closed { rings } else { rings - 1 };
    for k in 0..bands {
        for i in 0..n {
            mesh.faces.push(face(vec![
                at(k, i),
                at(k, i + 1),
                at(k + 1, i + 1),
                at(k + 1, i),
            ]));
            uvs.push(vec![
                wall_uv(i, k),
                wall_uv(i + 1, k),
                wall_uv(i + 1, k + 1),
                wall_uv(i, k + 1),
            ]);
        }
    }
    if p.caps {
        mesh.faces
            .push(face((0..n).map(|i| at(rings - 1, i)).collect()));
        uvs.push(vec![None; n as usize]);
    }
    // Make UVs: walls unrolled (U around, V along); caps have no published
    // layout and are left without UVs (provisional).
    finish(mesh, p.make_uvs.then_some(uvs))
}
