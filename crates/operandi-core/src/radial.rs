//! Independently constructed analytic Face primitives. The reviewed handoff
//! supplies measured invariants, not a recovered general mesh/UV algorithm.
//! World XYZ radii are semiaxes, including Cylinder Y (half the full height).
use crate::{Face, Mesh, Result};
use glam::DVec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadialParameters {
    pub radius: DVec3,
    pub sides: u32,
    pub segments: u32,
}

impl RadialParameters {
    /// Technical integrity limits, not reference-product limits. Flat drawing
    /// is separate transient display state; never persist coincident solid faces.
    pub fn validate(self, position: DVec3, globe: bool) -> Result<()> {
        if !position.is_finite()
            || position.abs().max_element() > 1e9
            || !self.radius.is_finite()
            || self.radius.min_element() <= 1e-8
            || self.radius.max_element() > 1e8
            || (position.abs() + self.radius).max_element() > 1e9
        {
            return Err("Solid Radius values must be finite, greater than 10 nm and within the coordinate budget; flat construction is not committed topology".into());
        }
        if self.sides < 3 || self.segments < if globe { 2 } else { 1 } {
            return Err(if globe {
                "Globe requires at least 3 Sides and 2 Segments"
            } else {
                "Cylinder requires at least 3 Sides and 1 Segment"
            }
            .into());
        }
        if (!globe && self.sides > 4096)
            || u64::from(self.sides) * u64::from(self.segments) > 249_998
        {
            return Err("Primitive exceeds the 250,000-face interactive allocation budget or 4096-corner cap budget; reduce resolution".into());
        }
        Ok(())
    }
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

/// A Globe along world `axis`, with the basic "Texture" UVs when asked. The
/// radius stays world-named, as for the Cylinder.
pub fn globe_geometry_with(
    position: DVec3,
    p: RadialParameters,
    axis: usize,
    make_uvs: bool,
) -> Result<Mesh> {
    if axis > 2 {
        return Err("Sphere Axis must be X, Y or Z".into());
    }
    let mut mesh = globe_geometry(
        DVec3::ZERO,
        RadialParameters {
            radius: to_local(axis, p.radius),
            ..p
        },
    )?;
    if make_uvs {
        crate::add_globe_uvs(&mut mesh, p.sides, p.segments)?;
    }
    for v in &mut mesh.vertices {
        *v = position + to_world(axis, *v);
    }
    p.validate(position, true)?;
    mesh.validate()?;
    Ok(mesh)
}

/// The primitives are built along local Y; these turn local Y onto world
/// `axis` and back (cyclic swaps, so windings still face outward).
fn to_world(axis: usize, v: DVec3) -> DVec3 {
    match axis {
        0 => DVec3::new(v.y, v.z, v.x),
        2 => DVec3::new(v.z, v.x, v.y),
        _ => v,
    }
}
fn to_local(axis: usize, v: DVec3) -> DVec3 {
    match axis {
        0 => DVec3::new(v.z, v.x, v.y),
        2 => DVec3::new(v.y, v.z, v.x),
        _ => v,
    }
}

/// Latitude rings with shared poles, triangles at the poles and quad bands.
/// Axis Y; exact reference angular phase is unresolved.
pub fn globe_geometry(position: DVec3, p: RadialParameters) -> Result<Mesh> {
    p.validate(position, true)?;
    let n = p.sides;
    let s = p.segments;
    let mut mesh = Mesh::default();
    mesh.vertices.push(position - DVec3::Y * p.radius.y);
    for ring in 1..s {
        let latitude = std::f64::consts::PI * ring as f64 / s as f64;
        for side in 0..n {
            let angle = std::f64::consts::TAU * side as f64 / n as f64;
            let point = DVec3::new(
                latitude.sin() * angle.cos(),
                -latitude.cos(),
                latitude.sin() * angle.sin(),
            );
            mesh.vertices.push(position + p.radius * point);
        }
    }
    let top = mesh.vertices.len() as u32;
    mesh.vertices.push(position + DVec3::Y * p.radius.y);
    for side in 0..n {
        let next = (side + 1) % n;
        mesh.faces.push(face(vec![0, 1 + side, 1 + next]));
        for band in 0..s - 2 {
            let a = 1 + band * n + side;
            let b = 1 + band * n + next;
            mesh.faces.push(face(vec![a, a + n, b + n, b]));
        }
        let last = 1 + (s - 2) * n;
        mesh.faces.push(face(vec![last + side, top, last + next]));
    }
    mesh.validate()?;
    Ok(mesh)
}

/// Shared rings along Y with one polygon at each end (Single caps).
pub fn cylinder_geometry(position: DVec3, p: RadialParameters) -> Result<Mesh> {
    cylinder_geometry_with(position, p, [CylinderCap::Single; 2], 1, false)
}

/// How a Cylinder end is closed (PRIM-Y-01).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CylinderCap {
    None,
    #[default]
    Single,
    /// An inner ring at half radius, quads between, and the inside split
    /// into quads around a centre point (even sides) or a triangle and quads
    /// (odd sides, measured for 5; others provisional).
    Quads,
    /// A centre point and a triangle fan.
    Radial,
}
impl CylinderCap {
    pub const ALL: [Self; 4] = [Self::None, Self::Single, Self::Quads, Self::Radial];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Single => "Single Polygon",
            Self::Quads => "Quads",
            Self::Radial => "Radial",
        }
    }
}

/// Closes `rim` (point ids in the cap polygon's winding) around `center`.
pub(crate) fn append_cap(mesh: &mut Mesh, rim: &[u32], center: DVec3, cap: CylinderCap) {
    let n = rim.len();
    let next = |i: usize| (i + 1) % n;
    match cap {
        CylinderCap::None => {}
        CylinderCap::Single => mesh.faces.push(face(rim.to_vec())),
        CylinderCap::Radial => {
            let c = mesh.vertices.len() as u32;
            mesh.vertices.push(center);
            for i in 0..n {
                mesh.faces.push(face(vec![rim[i], rim[next(i)], c]));
            }
        }
        CylinderCap::Quads => {
            let start = mesh.vertices.len() as u32;
            for &v in rim {
                let at = mesh.vertices[v as usize];
                mesh.vertices.push(center + 0.5 * (at - center));
            }
            let inner = |i: usize| start + (i % n) as u32;
            for i in 0..n {
                mesh.faces
                    .push(face(vec![rim[i], rim[next(i)], inner(i + 1), inner(i)]));
            }
            if n.is_multiple_of(2) {
                let c = mesh.vertices.len() as u32;
                mesh.vertices.push(center);
                for k in 0..n / 2 {
                    mesh.faces.push(face(vec![
                        inner(2 * k),
                        inner(2 * k + 1),
                        inner(2 * k + 2),
                        c,
                    ]));
                }
            } else {
                // PRIM-K-02: one triangle at the first point, quads fanned
                // from the last.
                mesh.faces
                    .push(face(vec![inner(0), inner(1), inner(n - 1)]));
                let mut k = 1;
                while k + 2 < n - 1 {
                    mesh.faces.push(face(vec![
                        inner(k),
                        inner(k + 1),
                        inner(k + 2),
                        inner(n - 1),
                    ]));
                    k += 2;
                }
            }
        }
    }
}

/// A Cylinder along world `axis` with the given bottom and top caps. The
/// radius stays world-named (PRIM-Y-01 Axis X keeps Y 0.75 and Z 0.5 as the
/// cross-section). With `make_uvs`, the basic "Texture" layout: the wall
/// strip and two cap islands seen from outside.
pub fn cylinder_geometry_with(
    position: DVec3,
    p: RadialParameters,
    caps: [CylinderCap; 2],
    axis: usize,
    make_uvs: bool,
) -> Result<Mesh> {
    p.validate(position, false)?;
    if axis > 2 {
        return Err("Cylinder Axis must be X, Y or Z".into());
    }
    let r = to_local(axis, p.radius);
    let n = p.sides;
    let s = p.segments;
    let mut mesh = Mesh::default();
    for ring in 0..=s {
        for side in 0..n {
            let angle = std::f64::consts::TAU * side as f64 / n as f64;
            let point = DVec3::new(angle.cos(), 2. * ring as f64 / s as f64 - 1., angle.sin());
            mesh.vertices.push(r * point);
        }
    }
    for ring in 0..s {
        for side in 0..n {
            let a = ring * n + side;
            let b = ring * n + (side + 1) % n;
            mesh.faces.push(face(vec![a, a + n, b + n, b]));
        }
    }
    let walls = mesh.faces.len();
    let bottom: Vec<u32> = (0..n).collect();
    // The top starts at side 0 too, so both caps pair their inner quads the
    // same way round (PRIM-Y-01 Quads).
    let top: Vec<u32> = std::iter::once(0)
        .chain((1..n).rev())
        .map(|i| s * n + i)
        .collect();
    append_cap(&mut mesh, &bottom, -DVec3::Y * r.y, caps[0]);
    append_cap(&mut mesh, &top, DVec3::Y * r.y, caps[1]);
    if make_uvs {
        let (nf, sf) = (f64::from(n), f64::from(s));
        // As for the sphere, U counts down so the wall isn't mirrored.
        let wall = |side: u32, ring: u32| {
            Some(glam::DVec2::new(
                1. - f64::from(side) / nf,
                0.5 * f64::from(ring) / sf,
            ))
        };
        let uvs = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(fi, f)| {
                if fi < walls {
                    let (side, ring) = (fi as u32 % n, fi as u32 / n);
                    return vec![
                        wall(side, ring),
                        wall(side, ring + 1),
                        wall(side + 1, ring + 1),
                        wall(side + 1, ring),
                    ];
                }
                // Cap islands: radius 0.24, bottom on the left, top on the
                // right, just clear of the wall strip.
                let lower = mesh.vertices[f.vertices[0] as usize].y < 0.;
                let (center, flip) = if lower {
                    (glam::DVec2::new(0.25, 0.75), 1.)
                } else {
                    (glam::DVec2::new(0.75, 0.75), -1.)
                };
                f.vertices
                    .iter()
                    .map(|&v| {
                        let q = mesh.vertices[v as usize];
                        Some(center + 0.24 * glam::DVec2::new(q.x / r.x, flip * q.z / r.z))
                    })
                    .collect()
            })
            .collect();
        mesh.uv_maps.insert(crate::PRIMITIVE_UV_MAP.into(), uvs);
    }
    for v in &mut mesh.vertices {
        *v = position + to_world(axis, *v);
    }
    mesh.validate()?;
    Ok(mesh)
}

/// The Tessellated sphere (PRIM-S-02): an icosahedron with poles on local Y,
/// the upper ring starting at +Z and the lower ring half a step round, each
/// `level` splitting every triangle into four on the surface (Level 0 → 12
/// points / 20 triangles, Level 1 → 42 / 80). World-named radius and axis as
/// for the Globe. UVs are a spherical projection (provisional: none measured).
pub fn tessellated_sphere(
    position: DVec3,
    radius: DVec3,
    level: u32,
    axis: usize,
    make_uvs: bool,
) -> Result<Mesh> {
    RadialParameters {
        radius,
        sides: 3,
        segments: 2,
    }
    .validate(position, true)?;
    if axis > 2 {
        return Err("Sphere Axis must be X, Y or Z".into());
    }
    if level > 6 {
        return Err("Tessellated Level goes up to 6 (81,920 triangles)".into());
    }
    let tau = std::f64::consts::TAU;
    let (y, ring) = (1. / 5f64.sqrt(), 2. / 5f64.sqrt());
    let mut points = vec![DVec3::Y];
    for k in 0..5 {
        let a = tau * k as f64 / 5.;
        points.push(DVec3::new(ring * a.sin(), y, ring * a.cos()));
    }
    points.push(-DVec3::Y);
    for k in 0..5 {
        let a = tau * (k as f64 + 0.5) / 5.;
        points.push(DVec3::new(ring * a.sin(), -y, ring * a.cos()));
    }
    let mut triangles = Vec::new();
    for k in 0..5u32 {
        let (u, un) = (1 + k, 1 + (k + 1) % 5);
        let (l, ln) = (7 + k, 7 + (k + 1) % 5);
        triangles.extend([[0, u, un], [u, l, un], [un, l, ln], [6, ln, l]]);
    }
    for _ in 0..level {
        let mut middle = std::collections::BTreeMap::new();
        let mut split = |a: u32, b: u32, points: &mut Vec<DVec3>| -> u32 {
            *middle.entry((a.min(b), a.max(b))).or_insert_with(|| {
                points.push((points[a as usize] + points[b as usize]).normalize());
                points.len() as u32 - 1
            })
        };
        let mut next = Vec::with_capacity(triangles.len() * 4);
        for [a, b, c] in triangles {
            let ab = split(a, b, &mut points);
            let bc = split(b, c, &mut points);
            let ca = split(c, a, &mut points);
            next.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        }
        triangles = next;
    }
    // Every triangle faces outward.
    for t in &mut triangles {
        let [a, b, c] = t.map(|v| points[v as usize]);
        if (b - a).cross(c - a).dot(a + b + c) < 0. {
            t.swap(1, 2);
        }
    }
    let mut mesh = Mesh::default();
    if make_uvs {
        let uv = |p: DVec3| {
            glam::DVec2::new(
                0.5 + p.x.atan2(p.z) / tau,
                0.5 + p.y.clamp(-1., 1.).asin() / std::f64::consts::PI,
            )
        };
        let uvs = triangles
            .iter()
            .map(|t| {
                let mut c = t.map(|v| uv(points[v as usize]));
                // Keep a triangle across the seam on one side of it, and give
                // a pole the U of its neighbours.
                let max = c.iter().map(|c| c.x).fold(f64::MIN, f64::max);
                for c in &mut c {
                    if max - c.x > 0.5 {
                        c.x += 1.;
                    }
                }
                for i in 0..3 {
                    if points[t[i] as usize].y.abs() > 1. - 1e-12 {
                        c[i].x = (c[(i + 1) % 3].x + c[(i + 2) % 3].x) / 2.;
                    }
                }
                c.map(Some).to_vec()
            })
            .collect();
        mesh.uv_maps.insert(crate::PRIMITIVE_UV_MAP.into(), uvs);
    }
    let r = to_local(axis, radius);
    mesh.vertices = points
        .into_iter()
        .map(|p| position + to_world(axis, r * p))
        .collect();
    mesh.faces = triangles.into_iter().map(|t| face(t.to_vec())).collect();
    mesh.validate()?;
    Ok(mesh)
}

/// A flat `sides`-gon in the plane across world `axis`, facing +axis, with
/// radii `ru` and `rv` along the next two axes: the flat base a first drag
/// draws (spec drag-creation-2026-11-06 rule 1).
pub fn flat_ngon(center: DVec3, axis: usize, ru: f64, rv: f64, sides: u32) -> Result<Mesh> {
    if !(3..=4096).contains(&sides) || !center.is_finite() || !(ru > 1e-8 && rv > 1e-8) {
        return Err("A flat base needs a size in both directions and 3–4096 Sides".into());
    }
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let vertices = (0..sides)
        .map(|i| {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(sides);
            let mut p = center;
            p[u] += ru * t.cos();
            p[v] += rv * t.sin();
            p
        })
        .collect();
    let mesh = Mesh {
        vertices,
        faces: vec![face((0..sides).collect())].into(),
        ..Default::default()
    };
    mesh.validate()?;
    Ok(mesh)
}

/// A flat ring of `sides` quads between radii `inner` and `outer` across
/// `axis`, facing +axis: a Torus drawn with no height.
pub fn flat_annulus(
    center: DVec3,
    axis: usize,
    inner: f64,
    outer: f64,
    sides: u32,
) -> Result<Mesh> {
    if !(3..=4096).contains(&sides) || !center.is_finite() || !(inner > 1e-8 && outer > inner) {
        return Err("A flat ring needs an inner and a larger outer radius".into());
    }
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let mut vertices = Vec::with_capacity(2 * sides as usize);
    for r in [outer, inner] {
        for i in 0..sides {
            let t = std::f64::consts::TAU * f64::from(i) / f64::from(sides);
            let mut p = center;
            p[u] += r * t.cos();
            p[v] += r * t.sin();
            vertices.push(p);
        }
    }
    let faces = (0..sides)
        .map(|i| {
            let j = (i + 1) % sides;
            face(vec![i, j, sides + j, sides + i])
        })
        .collect();
    let mesh = Mesh {
        vertices,
        faces,
        ..Default::default()
    };
    mesh.validate()?;
    Ok(mesh)
}
