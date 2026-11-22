//! Cone primitive (spec `cone-2026-11-05`; Quad Grid caps also follow the
//! measured cases in `primitives-2026-11-02`).
//!
//! A base ring, `Segments − 1` intermediate rings shrinking linearly to the
//! apex, and one apex point. Point 0 of a ring lies on the negative side of
//! the axis after the cone's axis (X→Y→Z→X) and the ring runs toward the
//! negative side of the remaining axis. A negative Size along the axis flips
//! the cone; Size 0 along it makes only the base polygon, whatever the caps.
//!
//! Provisional readings (spec open questions): Quad Grid's general layout
//! (Q-CN-02) uses the grid below, which matches every measured case; Top
//! Cap Radial and top Quad Grid with odd sides are refused (Q-CN-03); Bulge
//! is not built (Q-CN-04); Distortion Compensation and Gaps keep their
//! current layouts (Q-CN-05, Q-CN-07); zero or negative radii are refused
//! (Q-CN-12).
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolygonRepresentation {
    #[default]
    Face,
    Subdivs,
    CatmullClark,
    /// Ordered line element; closure adds a span, never another point.
    /// `curve` says how the points are drawn: straight (a polyline) or as
    /// a smooth Curve, B-Spline or Bezier through or near them.
    Polyline {
        closed: bool,
        #[serde(default, skip_serializing_if = "crate::curves::CurveKind::is_line")]
        curve: crate::curves::CurveKind,
    },
    /// A live patch (G7 PC-7): the polygon lists only its 3 or 4 corner
    /// points; its surface is a Coons patch over the open curves joining
    /// each pair of neighbouring corners (patch.rs), tessellated when the
    /// scene is evaluated. Edits treat it as a plain polygon.
    Patch,
}

/// Fill of one end of the cone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConeCap {
    None,
    /// One polygon over the base ring. At the apex, a fan of triangles.
    Single,
    /// Quads (an inner ring and a grid). At the apex, the fan's triangles
    /// merged in pairs.
    QuadGrid,
    /// A centre point and a fan of triangles.
    Radial,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConeParameters {
    pub position: DVec3,
    /// Signed world-named half-extents, not full widths or rotated radii.
    pub size: DVec3,
    pub axis: usize,
    pub sides: u32,
    pub segments: u32,
    pub polygon: PolygonRepresentation,
    pub top_cap: ConeCap,
    pub bottom_cap: ConeCap,
    /// Percent; the base rim's subdivision weight with Subdivs or
    /// Catmull-Clark.
    pub edge_weight: f64,
    pub make_uvs: bool,
    pub compensation: bool,
    pub gaps: f64,
    pub selected_uv_map: Option<String>,
}
impl Default for ConeParameters {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            size: DVec3::new(0.5, 0.75, 0.5),
            axis: 1,
            sides: 24,
            segments: 1,
            polygon: crate::user_defaults::current().primitive_polygon,
            top_cap: ConeCap::Single,
            bottom_cap: ConeCap::Single,
            edge_weight: 100.,
            make_uvs: true,
            compensation: false,
            gaps: 0.,
            selected_uv_map: None,
        }
    }
}
impl ConeParameters {
    pub fn min(&self) -> DVec3 {
        self.position - self.size
    }
    pub fn max(&self) -> DVec3 {
        self.position + self.size
    }
    pub fn set_min(&mut self, min: DVec3) {
        let max = self.max();
        self.position = (min + max) * 0.5;
        self.size = (max - min) * 0.5;
    }
    pub fn set_max(&mut self, max: DVec3) {
        let min = self.min();
        self.position = (min + max) * 0.5;
        self.size = (max - min) * 0.5;
    }
    pub fn normalized(mut self) -> Self {
        self.sides = self.sides.max(3);
        self.segments = self.segments.max(1);
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
            || !self.size.is_finite()
            || !self.edge_weight.is_finite()
            || !self.gaps.is_finite()
            || (self.position.abs() + self.size.abs()).max_element() > 1e9
            || self.size.abs().max_element() > 1e8
        {
            return Err("Cone fields must be finite and within the coordinate budget".into());
        }
        let n = u64::from(self.sides.max(3));
        // A Quad Grid cap adds an inner ring of quads and its grid cells
        // (odd sides use the grid for one more side, cone_grid.rs).
        let grid = if self.bottom_cap == ConeCap::QuadGrid && n >= 5 {
            let m = n + n % 2;
            let p = m / 4;
            n + p * (p + u64::from(m % 4 == 2))
        } else {
            0
        };
        if n > 4096 || n * u64::from(self.segments.max(1)) * 2 + 2 * n + grid > 250_000 {
            return Err("Cone exceeds the 250,000-face or 4096-corner technical budget".into());
        }
        if (0..3).any(|i| i != self.axis && self.size[i] <= 0.) {
            return Err(
                "Cone needs a positive base radius; zero or negative radii aren't specified yet"
                    .into(),
            );
        }
        if self.size[self.axis] != 0. {
            if self.top_cap == ConeCap::Radial {
                return Err(
                    "Top Cap Radial isn't specified yet; use None, Single or Quad Grid".into(),
                );
            }
            if self.top_cap == ConeCap::QuadGrid && n % 2 == 1 {
                return Err("Top Cap Quad Grid needs an even number of Sides".into());
            }
        }
        if self
            .selected_uv_map
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 1024)
        {
            return Err("Select a nonempty UV map name of at most 1024 bytes".into());
        }
        if self.make_uvs {
            if self.bottom_cap != ConeCap::None && !self.compensation && self.gaps == 50. {
                return Err(
                    "Gaps: generated UVs would be non-finite; correct Gaps or turn Make UVs off"
                        .into(),
                );
            }
            if self.gaps >= 100. / 3. {
                return Err("UVs aren't supported for Gaps of one third or more".into());
            }
            if self.compensation && self.size[self.axis] <= 0. {
                return Err(
                    "Distortion Compensation isn't specified for zero or negative heights".into(),
                );
            }
        }
        Ok(())
    }
}

pub fn cone_geometry(parameters: &ConeParameters) -> Result<Mesh> {
    let p = parameters.clone().normalized();
    p.validate()?;
    let n = p.sides;
    let m = p.segments;
    let axial = p.size[p.axis];
    let flat = axial == 0.;
    let mut mesh = Mesh::default();
    for j in 0..if flat { 1 } else { m } {
        let t = j as f64 / m as f64;
        for i in 0..n {
            let theta = std::f64::consts::TAU * i as f64 / n as f64;
            let (s, c) = theta.sin_cos();
            let mut q = match p.axis {
                0 => DVec3::new(0., -c, -s),
                1 => DVec3::new(-s, 0., -c),
                _ => DVec3::new(-c, -s, 0.),
            } * (1. - t);
            q[p.axis] = 2. * t - 1.;
            mesh.vertices.push(p.position + p.size * q);
        }
    }
    // The base's centre point (Radial), before the apex.
    let mut base_center = p.position;
    base_center[p.axis] -= axial;
    let radial = !flat && p.bottom_cap == ConeCap::Radial;
    let center_id = mesh.vertices.len() as u32;
    if radial {
        mesh.vertices.push(base_center);
    }
    let cap_face;
    let mut grid_cap = false;
    if flat {
        // Size 0 along the axis: only the base polygon, whatever the caps.
        mesh.faces.push(cone_face((0..n).collect()));
        cap_face = 0;
    } else {
        let mut tip = p.position;
        tip[p.axis] += axial;
        let tip_id = mesh.vertices.len() as u32;
        mesh.vertices.push(tip);
        for j in 0..m - 1 {
            for i in 0..n {
                let a = j * n + i;
                let b = j * n + (i + 1) % n;
                mesh.faces.push(cone_face(vec![a + n, a, b, b + n]));
            }
        }
        let top = (m - 1) * n;
        if p.top_cap == ConeCap::QuadGrid {
            // The apex fan's triangles merged in pairs (CN05c).
            for i in (0..n).step_by(2) {
                mesh.faces.push(cone_face(vec![
                    top + i,
                    top + (i + 1) % n,
                    top + (i + 2) % n,
                    tip_id,
                ]));
            }
        } else {
            for i in 0..n {
                mesh.faces
                    .push(cone_face(vec![top + i, top + (i + 1) % n, tip_id]));
            }
        }
        cap_face = mesh.faces.len();
        match p.bottom_cap {
            ConeCap::None => {}
            ConeCap::Single => mesh
                .faces
                .push(cone_face(std::iter::once(0).chain((1..n).rev()).collect())),
            ConeCap::Radial => {
                for i in 0..n {
                    mesh.faces.push(cone_face(vec![center_id, (i + 1) % n, i]));
                }
            }
            ConeCap::QuadGrid if n >= 5 => {
                grid_cap = true;
                crate::cone_grid::append_grid_cap(&mut mesh, &p);
            }
            ConeCap::QuadGrid => mesh
                .faces
                .push(cone_face(std::iter::once(0).chain((1..n).rev()).collect())),
        }
        if axial < 0. {
            for f in &mut mesh.faces {
                f.vertices.reverse();
            }
        }
    }
    mesh.attributes.representations = vec![p.polygon; mesh.faces.len()];
    if p.polygon != PolygonRepresentation::Face {
        for i in 0..n {
            mesh.attributes.creases.push(crate::MeshCrease {
                a: i.min((i + 1) % n),
                b: i.max((i + 1) % n),
                weight: p.edge_weight / 100.,
            });
        }
    }
    if p.make_uvs {
        let chart = Charts::new(&p);
        let uvs: Vec<_> = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(fi, f)| {
                f.vertices
                    .iter()
                    .map(|&v| {
                        let cap = fi >= cap_face;
                        let t = if flat {
                            0.
                        } else if v >= n * m {
                            1.
                        } else {
                            (v / n) as f64 / m as f64
                        };
                        let phi = std::f64::consts::TAU * (v % n) as f64 / n as f64
                            + std::f64::consts::PI / n as f64;
                        let uv = if cap && radial && v == center_id {
                            chart.cap
                        } else if cap && grid_cap {
                            let relative = mesh.vertices[v as usize] - base_center;
                            let projected = match p.axis {
                                0 => DVec2::new(relative.z / p.size.z, relative.y / p.size.y),
                                1 => DVec2::new(relative.x / p.size.x, relative.z / p.size.z),
                                _ => DVec2::new(relative.x / p.size.x, relative.y / p.size.y),
                            };
                            // A flipped cone mirrors its grid's layout
                            // (PR3-cone-y-0.75-sides12-quads).
                            let flip = if axial < 0. { -1. } else { 1. };
                            chart.cap + chart.cr * DVec2::new(projected.x, projected.y * flip)
                        } else if cap {
                            chart.cap
                                + DVec2::new(
                                    chart.cr.x * phi.cos(),
                                    chart.cr.y * phi.sin() * if axial <= 0. { 1. } else { -1. },
                                )
                        } else {
                            chart.side + (1. - t) * chart.sr * DVec2::new(phi.cos(), phi.sin())
                        };
                        Some(uv)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        if uvs.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return Err("Gaps: generated UVs would be non-finite".into());
        }
        let name = p
            .selected_uv_map
            .clone()
            .unwrap_or_else(|| "Texture".into());
        let mut next_cap_record = mesh.vertices.len() as u32;
        let records = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(fi, face)| {
                face.vertices
                    .iter()
                    .map(|&v| {
                        let id = if fi >= cap_face {
                            let id = next_cap_record;
                            next_cap_record += 1;
                            id
                        } else {
                            v
                        };
                        Some(id)
                    })
                    .collect()
            })
            .collect();
        mesh.attributes.uv_records.insert(name.clone(), records);
        mesh.uv_maps.insert(name, uvs);
    } else {
        mesh.uv_maps.insert(
            p.selected_uv_map
                .clone()
                .unwrap_or_else(|| "Texture".into()),
            mesh.faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect(),
        );
    }
    if (0..mesh.faces.len()).any(|fi| mesh.normal(fi) == DVec3::ZERO) {
        return Err("Cone collapses at numerical coordinate precision; increase Size or reduce Position magnitude".into());
    }
    mesh.validate()?;
    Ok(mesh)
}
pub(crate) fn cone_face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}
struct Charts {
    side: DVec2,
    cap: DVec2,
    sr: DVec2,
    cr: DVec2,
}
impl Charts {
    fn new(p: &ConeParameters) -> Self {
        let g = p.gaps / 100.;
        let capped = p.bottom_cap != ConeCap::None && p.size[p.axis] > 0.;
        if !p.compensation {
            if !capped {
                return Self {
                    side: DVec2::splat(0.5),
                    cap: DVec2::splat(0.5),
                    sr: DVec2::splat(0.5 - g),
                    cr: DVec2::splat(0.5 - g),
                };
            }
            let r = (1. - 3. * g).powi(2) / (4. * (1. - 2. * g));
            return Self {
                side: DVec2::new(g + r, 1. - g - r),
                cap: DVec2::new(2. * g + 3. * r, 1. - g - r),
                sr: DVec2::splat(r),
                cr: DVec2::splat(r),
            };
        }
        let (a, b, h) = match p.axis {
            0 => (p.size.y, p.size.z, p.size.x),
            1 => (p.size.z, p.size.x, p.size.y),
            _ => (p.size.x, p.size.y, p.size.z),
        };
        let r = (a + b) * 0.5;
        let l = r.hypot(h);
        let mut sr = DVec2::new(l * a / r, l * b / r);
        let mut cr = DVec2::new(a, b);
        let k = if capped {
            ((1. - 3. * g) / (2. * (sr.x + cr.x))).min((1. - 2. * g) / (2. * sr.y.max(cr.y)))
        } else {
            ((1. - 2. * g) / (2. * sr.x)).min((1. - 2. * g) / (2. * sr.y))
        };
        sr *= k;
        cr *= k;
        Self {
            side: if capped {
                DVec2::new(g + sr.x, 1. - g - sr.y)
            } else {
                DVec2::splat(g) + sr
            },
            cap: DVec2::new(2. * g + 2. * sr.x + cr.x, 1. - g - cr.y),
            sr,
            cr,
        }
    }
}
