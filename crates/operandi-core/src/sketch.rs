//! Create > Draw > Sketch (draw-tools-2026-11-05, Sketch): a free stroke
//! on the work plane, resampled to even spacing, built as a Curve, Face,
//! Lines, Quads, Bezier or B-Spline. All points are world space.
use crate::curve_edit::curve_mesh;
use crate::curves::{CurveKind, bezier_points};
use crate::{Face, Mesh, PolygonRepresentation, Result};
use glam::DVec3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SketchType {
    #[default]
    Curve,
    Face,
    Lines,
    Quads,
    Bezier,
    BSpline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchOptions {
    pub kind: SketchType,
    /// Sample spacing in screen pixels (seen default 30).
    pub precision: f64,
    /// Quads strip width, metres (default 0.1).
    pub width: f64,
    /// Quads divisions across the strip (default 1).
    pub slice: u32,
    /// 0 = keep every segment; otherwise cut to this many from the start.
    pub segments: u32,
}
impl Default for SketchOptions {
    fn default() -> Self {
        Self {
            kind: SketchType::Curve,
            precision: 30.,
            width: 0.1,
            slice: 1,
            segments: 0,
        }
    }
}

/// Evenly spaced samples along `stroke`, `spacing` apart, starting at its
/// first point and ending at its last.
pub fn resample(stroke: &[DVec3], spacing: f64) -> Vec<DVec3> {
    let Some(&first) = stroke.first() else {
        return Vec::new();
    };
    let spacing = spacing.max(1e-9);
    let mut out = vec![first];
    let mut carried = 0.;
    for pair in stroke.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let len = (b - a).length();
        if len < 1e-15 {
            continue;
        }
        let mut t = spacing - carried;
        while t <= len {
            out.push(a + (b - a) * (t / len));
            t += spacing;
        }
        carried = len - (t - spacing);
    }
    let last = *stroke.last().unwrap();
    if (last - *out.last().unwrap()).length() > spacing * 0.25 {
        out.push(last);
    } else if out.len() > 1 {
        *out.last_mut().unwrap() = last;
    }
    out
}

/// Douglas-Peucker: the fewest points that keep every sample within
/// `tolerance` of the simplified line.
pub fn simplify(points: &[DVec3], tolerance: f64) -> Vec<DVec3> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (points[a], points[b]);
        let d = pb - pa;
        let (mut far, mut at) = (0., a);
        for (i, p) in points.iter().enumerate().take(b).skip(a + 1) {
            let t = if d.length_squared() < 1e-24 {
                0.
            } else {
                ((*p - pa).dot(d) / d.length_squared()).clamp(0., 1.)
            };
            let dist = (*p - (pa + d * t)).length();
            if dist > far {
                far = dist;
                at = i;
            }
        }
        if far > tolerance {
            keep[at] = true;
            stack.push((a, at));
            stack.push((at, b));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

fn line(a: u32, b: u32) -> Face {
    Face {
        vertices: vec![a, b],
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// The geometry for one stroke. `samples` are already evenly spaced;
/// `normal` is the drawing plane's normal (Quads stand along it).
pub fn sketch_geometry(
    samples: &[DVec3],
    spacing: f64,
    options: &SketchOptions,
    normal: DVec3,
) -> Result<Mesh> {
    let mut samples = samples.to_vec();
    if options.segments > 0 {
        samples.truncate(options.segments as usize + 1);
    }
    if samples.len() < 2 {
        return Err("Sketch: the stroke is too short".into());
    }
    let fitted = || simplify(&samples, spacing * 0.5);
    let mesh = match options.kind {
        SketchType::Curve => curve_mesh(
            fitted(),
            false,
            CurveKind::Curve {
                start_control: false,
                end_control: false,
            },
        ),
        SketchType::BSpline => {
            let mut p = fitted();
            // A B-Spline needs four points: add midpoints until it has them.
            while p.len() < 4 {
                let i = (0..p.len() - 1)
                    .max_by(|&a, &b| {
                        (p[a + 1] - p[a])
                            .length()
                            .total_cmp(&(p[b + 1] - p[b]).length())
                    })
                    .unwrap();
                p.insert(i + 1, (p[i] + p[i + 1]) / 2.);
            }
            curve_mesh(p, false, CurveKind::BSpline)
        }
        SketchType::Bezier => {
            // Anchors at the fitted points, handles a third of the way to
            // the neighbours along the Catmull-Rom tangent.
            let p = fitted();
            let n = p.len();
            let anchors: Vec<[DVec3; 3]> = (0..n)
                .map(|i| {
                    let prev = p[i.saturating_sub(1)];
                    let next = p[(i + 1).min(n - 1)];
                    let tangent = (next - prev) / 6.;
                    [p[i] - tangent, p[i], p[i] + tangent]
                })
                .collect();
            curve_mesh(bezier_points(&anchors, false), false, CurveKind::Bezier)
        }
        SketchType::Face => {
            if samples.len() < 3 {
                return Err("Sketch Face needs a longer stroke".into());
            }
            let n = samples.len() as u32;
            let mut m = Mesh {
                vertices: samples.clone(),
                ..Default::default()
            };
            let mut face = line(0, 1);
            face.vertices = (0..n).collect();
            m.faces.push(face);
            m
        }
        SketchType::Lines => {
            let n = samples.len() as u32;
            let mut m = Mesh {
                vertices: samples.clone(),
                ..Default::default()
            };
            for i in 0..n - 1 {
                m.faces.push(line(i, i + 1));
                m.attributes
                    .representations
                    .push(PolygonRepresentation::Polyline {
                        closed: false,
                        curve: CurveKind::Line,
                    });
            }
            m
        }
        SketchType::Quads => {
            let slice = options.slice.max(1);
            let rows = slice + 1;
            let n = normal.normalize_or(DVec3::Y);
            let mut m = Mesh::default();
            for p in &samples {
                for k in 0..rows {
                    let s = k as f64 / slice as f64 - 0.5;
                    m.vertices.push(*p + n * (s * options.width));
                }
            }
            for i in 0..samples.len() as u32 - 1 {
                for k in 0..slice {
                    let a = i * rows + k;
                    let mut face = line(0, 0);
                    face.vertices = vec![a, a + rows, a + rows + 1, a + 1];
                    m.faces.push(face);
                }
            }
            m
        }
    };
    mesh.validate()?;
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resample_spaces_points_evenly() {
        let s = resample(&[DVec3::ZERO, DVec3::X * 10.], 1.);
        assert_eq!(s.len(), 11);
        assert!((s[3] - DVec3::X * 3.).length() < 1e-12);
    }
    #[test]
    fn segments_cut_from_the_start() {
        let s = resample(&[DVec3::ZERO, DVec3::X * 10.], 1.);
        let o = SketchOptions {
            kind: SketchType::Lines,
            segments: 4,
            ..Default::default()
        };
        let m = sketch_geometry(&s, 1., &o, DVec3::Y).unwrap();
        assert_eq!(m.faces.len(), 4);
    }
    #[test]
    fn quads_stand_across_the_plane() {
        let s = resample(&[DVec3::ZERO, DVec3::X * 2.], 1.);
        let o = SketchOptions {
            kind: SketchType::Quads,
            slice: 2,
            ..Default::default()
        };
        let m = sketch_geometry(&s, 1., &o, DVec3::Y).unwrap();
        assert_eq!(m.faces.len(), 4);
        let ys: Vec<f64> = m.vertices.iter().map(|v| v.y).collect();
        assert!((ys.iter().cloned().fold(f64::MIN, f64::max) - 0.05).abs() < 1e-12);
    }
    #[test]
    fn curve_fits_a_straight_stroke_with_two_points() {
        let s = resample(&[DVec3::ZERO, DVec3::X * 10.], 1.);
        let m = sketch_geometry(&s, 1., &SketchOptions::default(), DVec3::Y).unwrap();
        assert_eq!(m.vertices.len(), 2);
    }
}
