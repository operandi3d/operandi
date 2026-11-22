//! Path and Curvature falloffs (falloff spec 2026-11-15 batch 6): a tube
//! around a curve clicked in the view (FO-53) and weights from how sharply
//! the surface bends (FO-51). Both are worked out per point once per drag
//! (`falloff_reach::reach_weights`).
use crate::Mesh;
use crate::falloff::FalloffShape;
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// FO-53 Path: Smooth(distance to the curve / Size), 0 beyond.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathFalloff {
    /// Tube radius (scene units).
    pub size: f64,
    /// FO-35: control points are world positions when on, the mesh's own
    /// space when off.
    pub world: bool,
}
impl Default for PathFalloff {
    /// FO-53 (measured): Size 1 m, World off.
    fn default() -> Self {
        Self {
            size: 1.,
            world: false,
        }
    }
}
impl PathFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !(self.size.is_finite() && self.size >= 0.) {
            return Err("Path Size must be zero or more".into());
        }
        Ok(())
    }
}

/// FO-51 Curvature.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CurvatureFalloff {
    pub scale: f64,
}
impl Default for CurvatureFalloff {
    /// FO-51 (measured): Scale 1.
    fn default() -> Self {
        Self { scale: 1. }
    }
}
impl CurvatureFalloff {
    pub fn validate(self) -> crate::Result<()> {
        if !self.scale.is_finite() {
            return Err("Curvature Scale must be a number".into());
        }
        Ok(())
    }
}

/// Samples per span of the path curve.
const SPAN_SAMPLES: usize = 12;

/// The smooth curve through the control points (Catmull-Rom, ends
/// repeated), as a polyline. One point gives itself.
pub fn path_curve(points: &[DVec3]) -> Vec<DVec3> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let at = |i: isize| points[i.clamp(0, points.len() as isize - 1) as usize];
    let mut out = vec![points[0]];
    for i in 0..points.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for s in 1..=SPAN_SAMPLES {
            let t = s as f64 / SPAN_SAMPLES as f64;
            let (t2, t3) = (t * t, t * t * t);
            out.push(
                0.5 * ((2. * p1)
                    + (p2 - p0) * t
                    + (2. * p0 - 5. * p1 + 4. * p2 - p3) * t2
                    + (3. * p1 - p0 - 3. * p2 + p3) * t3),
            );
        }
    }
    out
}

/// FO-53: each point's weight from its distance to the curve through
/// `controls` (world positions with Use World Transforms, else the mesh's
/// own). No control points: every point weighs 0.
pub fn path_weights(mesh: &Mesh, world: DMat4, f: PathFalloff, controls: &[DVec3]) -> Vec<f64> {
    let curve = path_curve(controls);
    if curve.is_empty() || f.size <= 0. {
        return vec![0.; mesh.vertices.len()];
    }
    // A box around the curve grown by Size rejects far points first.
    let (lo, hi) = curve.iter().fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |(lo, hi), &c| (lo.min(c), hi.max(c)),
    );
    let (lo, hi) = (lo - DVec3::splat(f.size), hi + DVec3::splat(f.size));
    mesh.vertices
        .iter()
        .map(|&local| {
            let p = if f.world {
                world.transform_point3(local)
            } else {
                local
            };
            if p.cmplt(lo).any() || p.cmpgt(hi).any() {
                return 0.;
            }
            let d = if curve.len() == 1 {
                p.distance(curve[0])
            } else {
                curve
                    .windows(2)
                    .map(|s| segment_distance(p, s[0], s[1]))
                    .fold(f64::INFINITY, f64::min)
            };
            if d >= f.size {
                0.
            } else {
                FalloffShape::Smooth.weight(d / f.size)
            }
        })
        .collect()
}

fn segment_distance(p: DVec3, a: DVec3, b: DVec3) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared().max(1e-24)).clamp(0., 1.);
    p.distance(a + d * t)
}

/// FO-51 (formula provisional, FO-Q07): each point's signed bend, the
/// umbrella measure (how far its neighbours' average sits below it along
/// its normal, divided by their mean distance: positive on bumps, negative
/// in dips), times Scale, mapped to 0..1 around 0.5. Flat areas weigh 0.5;
/// a negative Scale swaps bumps and dips. Points on no polygon weigh 0.5.
pub fn curvature_weights(mesh: &Mesh, f: CurvatureFalloff) -> Vec<f64> {
    let n = mesh.vertices.len();
    let mut normal = vec![DVec3::ZERO; n];
    let mut sum = vec![DVec3::ZERO; n];
    let mut count = vec![0u32; n];
    let mut reach = vec![0f64; n];
    for (fi, face) in mesh.faces.iter().enumerate() {
        let c = &face.vertices;
        if c.len() < 3 || mesh.is_polyline(fi) {
            continue;
        }
        let pts: Vec<DVec3> = c.iter().map(|&v| mesh.vertices[v as usize]).collect();
        let mut fnorm = DVec3::ZERO;
        for i in 0..pts.len() {
            fnorm += pts[i].cross(pts[(i + 1) % pts.len()]);
        }
        for (i, &v) in c.iter().enumerate() {
            let v = v as usize;
            normal[v] += fnorm;
            for j in [(i + 1) % c.len(), (i + c.len() - 1) % c.len()] {
                // Each side is met from both polygons; the average doesn't
                // mind counting it twice.
                sum[v] += pts[j];
                count[v] += 1;
                reach[v] += pts[i].distance(pts[j]);
            }
        }
    }
    (0..n)
        .map(|v| {
            let nn = normal[v].normalize_or_zero();
            if count[v] == 0 || nn == DVec3::ZERO {
                return 0.5;
            }
            let avg = sum[v] / count[v] as f64;
            let len = reach[v] / count[v] as f64;
            if len <= 0. {
                return 0.5;
            }
            let bend = (mesh.vertices[v] - avg).dot(nn) / len;
            (0.5 + bend * f.scale).clamp(0., 1.)
        })
        .collect()
}
