//! EA-001..009: Ellipsoid's admitted numeric pole/ring and corner-map contract.
//! Logical signed semiaxes, sampled points and evaluated surfaces are distinct.
use crate::{Face, Mesh, PolygonRepresentation, Result};
use glam::{DVec2, DVec3};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct EllipsoidParameters {
    pub position: DVec3,
    pub radius: DVec3,
    pub sides: u32,
    pub segments: u32,
    pub polygon: PolygonRepresentation,
    pub bulge_top: f64,
    pub bulge_side: f64,
    pub axis: usize,
    pub make_uvs: bool,
    pub compensation: bool,
    /// UI percentage: 7 means seven percent. The admitted clamp is 0..50.
    pub gaps: f64,
    pub selected_uv_map: Option<String>,
    /// EA-011's recorded crossing-lobe case only; no coincidence/merge policy.
    pub symmetry_axis: Option<usize>,
}
impl Default for EllipsoidParameters {
    /// Observed EA-001 Reset, not a globally reset factory-profile claim.
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            radius: DVec3::splat(0.5),
            sides: 24,
            segments: 12,
            polygon: crate::user_defaults::current().primitive_polygon,
            bulge_top: 0.,
            bulge_side: 0.,
            axis: 1,
            make_uvs: true,
            compensation: false,
            gaps: 0.,
            selected_uv_map: None,
            symmetry_axis: None,
        }
    }
}
impl EllipsoidParameters {
    pub fn min(&self) -> DVec3 {
        self.position - self.radius
    }
    pub fn max(&self) -> DVec3 {
        self.position + self.radius
    }
    pub fn set_min(&mut self, min: DVec3) {
        let max = self.max();
        self.position = (min + max) * 0.5;
        self.radius = (max - min) * 0.5;
    }
    pub fn set_max(&mut self, max: DVec3) {
        let min = self.min();
        self.position = (min + max) * 0.5;
        self.radius = (max - min) * 0.5;
    }
    pub fn normalized(mut self) -> Self {
        self.sides = self.sides.max(3);
        self.segments = self.segments.max(2);
        // Preserve nonfinite input for validation; f64::max would mask NaN.
        if self.bulge_top.is_finite() {
            self.bulge_top = self.bulge_top.max(-1.5);
        }
        if self.bulge_side.is_finite() {
            self.bulge_side = self.bulge_side.max(-1.5);
        }
        if self.gaps.is_finite() {
            self.gaps = self.gaps.clamp(0., 50.);
        }
        self
    }
    pub fn validate(&self) -> Result<()> {
        if matches!(self.polygon, PolygonRepresentation::Polyline { .. }) {
            return Err("This primitive cannot create Polyline elements".into());
        }
        if self.axis > 2
            || !self.position.is_finite()
            || !self.radius.is_finite()
            || !self.bulge_top.is_finite()
            || !self.bulge_side.is_finite()
            || !self.gaps.is_finite()
            || (self.position.abs() + self.radius.abs()).max_element() > 1e9
            || self.radius.abs().max_element() > 1e8
        {
            return Err("Ellipsoid fields must be finite and within the coordinate budget".into());
        }
        // Technical preview allocation bound, not a measured reference limit.
        if u64::from(self.sides.max(3)) * u64::from(self.segments.max(2)) > 250_000 {
            return Err("Ellipsoid exceeds the 250,000-face technical preview budget; reduce Sides/Segments".into());
        }
        if self.make_uvs && self.compensation && self.radius.min_element() <= 0. {
            // Spec: EA-Q02
            return Err("Compensated UVs aren't supported for zero or negative sizes; turn off Compensation or Make UVs".into());
        }
        if self
            .selected_uv_map
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("Select a nonempty UV map name of at most 1024 bytes".into());
        }
        if let Some(axis) = self.symmetry_axis
            && (axis > 2
                || self.position != DVec3::new(1., -2., 3.)
                || self.radius != DVec3::new(2., 3., 4.)
                || self.sides != 8
                || self.segments != 4
                || self.axis != 1
                || self.polygon != PolygonRepresentation::Face
                || self.bulge_top != 0.
                || self.bulge_side != 0.
                || self.make_uvs)
        {
            // Spec: EA-011 / EA-Q03
            return Err("Ellipsoid symmetry only works for one tested setup: Position 1, -2, 3; Radius 2, 3, 4; 8 Sides; 4 Segments; Axis Y; Face; no bulge; UVs off".into());
        }
        Ok(())
    }
}

fn signed_power(x: f64, exponent: f64) -> f64 {
    if x == 0. {
        0.
    } else {
        x.signum() * x.abs().powf(exponent)
    }
}

/// Closed sampled ellipse perimeter fractions, EA-007. This intentionally is
/// not the generated meridian's arc length, and never depends on either bulge.
fn perimeter_knots(count: u32, a: f64, b: f64) -> Result<Vec<f64>> {
    let point = |i: u32| {
        let t = std::f64::consts::TAU * f64::from(i) / f64::from(count);
        DVec2::new(a * t.cos(), b * t.sin())
    };
    let mut sums = Vec::with_capacity(count as usize + 1);
    sums.push(0.);
    let mut previous = point(0);
    let mut sum = 0.;
    for i in 1..=count {
        let next = point(i);
        sum += (next - previous).length();
        sums.push(sum);
        previous = next;
    }
    if !sum.is_finite() || sum <= 0. {
        return Err("Ellipsoid compensated perimeter is not finite at this precision".into());
    }
    for s in &mut sums {
        *s /= sum;
    }
    Ok(sums)
}

fn ellipsoid_cage(parameters: &EllipsoidParameters) -> Result<Mesh> {
    let p = parameters.clone().normalized();
    p.validate()?;
    let (n, m) = (p.sides, p.segments);
    let ring = |j: u32, i: u32| 1 + (j - 1) * n + i % n;
    let top = 1 + n * (m - 1);
    let mut mesh = Mesh::default();
    mesh.vertices.reserve(top as usize + 1);
    let mut pole = p.position;
    pole[p.axis] -= p.radius[p.axis];
    mesh.vertices.push(pole);
    let et = 2. / (2. + p.bulge_top);
    let es = 2. / (2. + p.bulge_side);
    for j in 1..m {
        let eta = std::f64::consts::PI * f64::from(j) / f64::from(m) - std::f64::consts::FRAC_PI_2;
        let h = signed_power(eta.sin(), et);
        let r = signed_power(eta.cos(), et);
        for i in 0..n {
            let tau = std::f64::consts::TAU * f64::from(i) / f64::from(n) - std::f64::consts::PI;
            // Do not snap cardinal trig residuals before exponentiation: EA-O15/16.
            let u = r * signed_power(tau.cos(), es);
            let v = r * signed_power(tau.sin(), es);
            let q = match p.axis {
                0 => DVec3::new(h, u, v),
                1 => DVec3::new(v, h, u),
                _ => DVec3::new(u, v, h),
            };
            mesh.vertices.push(p.position + p.radius * q);
        }
    }
    pole[p.axis] = p.position[p.axis] + p.radius[p.axis];
    mesh.vertices.push(pole);
    let mut knots_u: Vec<_> = (0..=n).map(|i| f64::from(i) / f64::from(n)).collect();
    let mut knots_v: Vec<_> = (0..=m).map(|j| f64::from(j) / f64::from(m)).collect();
    if p.make_uvs && p.compensation {
        let a = p.radius[(p.axis + 1) % 3];
        let b = p.radius[(p.axis + 2) % 3];
        let c = p.radius[p.axis];
        knots_u = perimeter_knots(n, a, b)?
            .into_iter()
            .map(|u| a / a.max(b) * u)
            .collect();
        knots_v = perimeter_knots(m, (a + b) * 0.5, c)?
            .into_iter()
            .map(|v| b / a.max(b) * v)
            .collect();
    }
    let g = p.gaps / 100.;
    for k in knots_u.iter_mut().chain(&mut knots_v) {
        *k = g + (1. - 2. * g) * *k;
    }
    let mut uvs = Vec::new();
    let mut records = Vec::new();
    // Symbolic ownership, independent of equal coordinates (including Gaps50).
    // Ring seam copies stay distinct.
    let mut identities = BTreeMap::<(u32, u32, u32), u32>::new();
    let mut add = |vertices: Vec<u32>, indices: Vec<(u32, u32)>| {
        if p.make_uvs {
            let mut face_uv = Vec::new();
            let mut face_ids = Vec::new();
            for (&vertex, &(i, j)) in vertices.iter().zip(&indices) {
                let u = if vertex == 0 || vertex == top {
                    (knots_u[i as usize] + knots_u[i as usize + 1]) * 0.5
                } else {
                    knots_u[i as usize]
                };
                face_uv.push(Some(DVec2::new(u, knots_v[j as usize])));
                let next = identities.len() as u32;
                face_ids.push(Some(*identities.entry((vertex, i, j)).or_insert(next)));
            }
            uvs.push(face_uv);
            records.push(face_ids);
        }
        mesh.faces.push(Face {
            vertices,
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    };
    for i in 0..n {
        add(
            vec![0, ring(1, i + 1), ring(1, i)],
            vec![(i, 0), (i + 1, 1), (i, 1)],
        );
    }
    for j in 1..m - 1 {
        for i in 0..n {
            add(
                vec![
                    ring(j, i),
                    ring(j, i + 1),
                    ring(j + 1, i + 1),
                    ring(j + 1, i),
                ],
                vec![(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)],
            );
        }
    }
    for i in 0..n {
        // Decided (ellipsoid-2026-11-05 Q-EL-01): the top fan's ring corners
        // keep their own columns, like the bottom fan, not the reference's
        // swapped u.
        add(
            vec![top, ring(m - 1, i), ring(m - 1, i + 1)],
            vec![(i, m), (i, m - 1), (i + 1, m - 1)],
        );
    }
    mesh.attributes.representations = vec![p.polygon; mesh.faces.len()];
    if p.make_uvs {
        if uvs.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return Err("Ellipsoid produced nonfinite UVs".into());
        }
        let name = p
            .selected_uv_map
            .clone()
            .unwrap_or_else(|| "Texture".into());
        mesh.uv_maps.insert(name.clone(), uvs);
        mesh.attributes.uv_records.insert(name, records);
    }
    for fi in 0..mesh.faces.len() {
        if mesh.normal(fi) == DVec3::ZERO {
            if !p.radius.to_array().contains(&0.) {
                return Err("Ellipsoid collapsed at numerical coordinate precision; adjust Position/Radius/resolution or bulges".into());
            }
            // EA-004 explicitly retains finite zero-radius degenerate records.
            mesh.attributes.degenerate_faces.insert(fi as u32);
        }
    }
    mesh.validate()?;
    Ok(mesh)
}

pub fn ellipsoid_geometry(parameters: &EllipsoidParameters) -> Result<Mesh> {
    // A first drag's flat base: no size along the axis is one disc.
    let a = parameters.axis.min(2);
    if parameters.radius[a] == 0.
        && parameters.radius[(a + 1) % 3] != 0.
        && parameters.radius[(a + 2) % 3] != 0.
        && !(parameters.compensation && parameters.make_uvs)
    {
        return crate::flat_ngon(
            parameters.position,
            a,
            parameters.radius[(a + 1) % 3].abs(),
            parameters.radius[(a + 2) % 3].abs(),
            parameters.sides,
        );
    }
    let mut mesh = ellipsoid_cage(parameters)?;
    if let Some(axis) = parameters.symmetry_axis {
        let mut reflected = mesh.clone();
        for v in &mut reflected.vertices {
            v[axis] = -v[axis];
        }
        for f in &mut reflected.faces {
            f.vertices.reverse();
        }
        // This bounded case has UVs off, no creases or generated normals.
        crate::append_mesh(&mut mesh, reflected)?;
    }
    Ok(mesh)
}
