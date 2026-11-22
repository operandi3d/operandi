//! Create > Draw > Draw Patch, also Duplicate > Patch Curves
//! (draw-tools-2026-11-05, G7-PATCH-CURVES): a grid filling three or four
//! boundary curves that meet end to end. Operandi builds the surface as a
//! bilinear Coons patch; the result is frozen geometry, not a live patch
//! (PC-7 waits on PC-Q3).
use crate::{Face, Mesh, Result};
use glam::{DVec2, DVec3};

/// How divisions are spaced along a boundary (PC-6 Type 1 / Type 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PatchSpacing {
    /// Evenly by arc length.
    #[default]
    Length,
    /// Evenly in the curve's own parameter: each span between control
    /// points gets an equal share (PROVISIONAL, PC-Q2).
    Knot,
}

impl PatchSpacing {
    pub const ALL: [PatchSpacing; 2] = [PatchSpacing::Length, PatchSpacing::Knot];
    pub fn label(self) -> &'static str {
        match self {
            PatchSpacing::Length => "Length",
            PatchSpacing::Knot => "Knot",
        }
    }
}

/// `points` resampled to `count` points evenly spaced by arc length.
pub fn by_length(points: &[DVec3], count: usize) -> Vec<DVec3> {
    let count = count.max(2);
    if points.len() < 2 {
        return vec![points.first().copied().unwrap_or_default(); count];
    }
    let mut lengths = vec![0.];
    for w in points.windows(2) {
        lengths.push(lengths.last().unwrap() + (w[1] - w[0]).length());
    }
    let total = *lengths.last().unwrap();
    (0..count)
        .map(|i| {
            let target = total * i as f64 / (count - 1) as f64;
            let j = lengths
                .partition_point(|&l| l < target)
                .clamp(1, points.len() - 1);
            let span = lengths[j] - lengths[j - 1];
            let t = if span < 1e-15 {
                0.
            } else {
                (target - lengths[j - 1]) / span
            };
            points[j - 1].lerp(points[j], t)
        })
        .collect()
}

/// `points` (a curve sampled with the same number of steps per span)
/// resampled to `count` points evenly spaced in sample index, which is the
/// curve's own parameter.
pub fn by_knot(points: &[DVec3], count: usize) -> Vec<DVec3> {
    let count = count.max(2);
    if points.len() < 2 {
        return vec![points.first().copied().unwrap_or_default(); count];
    }
    let last = (points.len() - 1) as f64;
    (0..count)
        .map(|i| {
            let s = last * i as f64 / (count - 1) as f64;
            let j = (s.floor() as usize).min(points.len() - 2);
            points[j].lerp(points[j + 1], s - j as f64)
        })
        .collect()
}

fn resample(points: &[DVec3], count: usize, spacing: PatchSpacing) -> Vec<DVec3> {
    let mut out = match spacing {
        PatchSpacing::Length => by_length(points, count),
        PatchSpacing::Knot => by_knot(points, count),
    };
    // The ends are the curve's own ends exactly, so corners line up.
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        out[0] = *first;
        *out.last_mut().unwrap() = *last;
    }
    out
}

/// A built patch and where its loop corners are in it.
#[derive(Clone, Debug)]
pub struct PatchGrid {
    pub mesh: Mesh,
    /// The grid point at the start of each boundary curve, in pick order.
    pub corners: Vec<u32>,
}

/// Three or four boundary curves (points), each running on from the end of
/// the one before, around the loop. The first is the patch's "bottom":
/// `parallel` divisions run along it (PC-4) and `perpendicular` across.
/// With three curves the side opposite the first collapses to the corner
/// where the second and third meet (PROVISIONAL, PC-Q1): one shared point,
/// and the row of cells touching it is triangles (PC-3).
pub fn coons_patch(
    boundary: &[&[DVec3]],
    parallel: u32,
    perpendicular: u32,
    spacing_parallel: PatchSpacing,
    spacing_perpendicular: PatchSpacing,
    make_uvs: bool,
) -> Result<PatchGrid> {
    let three = match boundary.len() {
        3 => true,
        4 => false,
        _ => return Err("Patch Curves needs three or four curves".into()),
    };
    if boundary.iter().any(|b| b.len() < 2) {
        return Err("Patch Curves: a boundary curve has too few points".into());
    }
    let (nu, nv) = (parallel.max(1) as usize, perpendicular.max(1) as usize);
    if nu * nv > 1_000_000 {
        return Err("Patch Curves: too many polygons".into());
    }
    let bottom = resample(boundary[0], nu + 1, spacing_parallel);
    let right = resample(boundary[1], nv + 1, spacing_perpendicular);
    let (top, mut left) = if three {
        let apex = *boundary[1].last().unwrap();
        (
            vec![apex; nu + 1],
            resample(boundary[2], nv + 1, spacing_perpendicular),
        )
    } else {
        let mut top = resample(boundary[2], nu + 1, spacing_parallel);
        top.reverse();
        (top, resample(boundary[3], nv + 1, spacing_perpendicular))
    };
    left.reverse();
    let (p00, p10, p11, p01) = (bottom[0], bottom[nu], top[nu], top[0]);
    let row = nu as u32 + 1;
    let apex_index = nv as u32 * row;
    let index = |i: usize, j: usize| -> u32 {
        if three && j == nv {
            apex_index
        } else {
            j as u32 * row + i as u32
        }
    };
    let mut m = Mesh::default();
    for j in 0..=nv {
        let v = j as f64 / nv as f64;
        for i in 0..=nu {
            if three && j == nv && i > 0 {
                break;
            }
            let u = i as f64 / nu as f64;
            let ruled_u = bottom[i] * (1. - v) + top[i] * v;
            let ruled_v = left[j] * (1. - u) + right[j] * u;
            let corners = p00 * ((1. - u) * (1. - v))
                + p10 * (u * (1. - v))
                + p11 * (u * v)
                + p01 * ((1. - u) * v);
            m.vertices.push(ruled_u + ruled_v - corners);
        }
    }
    for j in 0..nv {
        for i in 0..nu {
            let mut corners: Vec<(u32, DVec2)> = Vec::with_capacity(4);
            for (x, y) in [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)] {
                let v = index(x, y);
                if corners.last().is_some_and(|c| c.0 == v)
                    || (corners.len() == 3 && corners[0].0 == v)
                {
                    continue;
                }
                let uv = DVec2::new(x as f64 / nu as f64, y as f64 / nv as f64);
                corners.push((v, uv));
            }
            m.faces.push(Face {
                vertices: corners.iter().map(|c| c.0).collect(),
                uv: make_uvs.then(|| corners.iter().map(|c| c.1).collect()),
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    m.validate()?;
    let corners = if three {
        vec![index(0, 0), index(nu, 0), apex_index]
    } else {
        vec![index(0, 0), index(nu, 0), index(nu, nv), index(0, nv)]
    };
    Ok(PatchGrid { mesh: m, corners })
}

#[cfg(test)]
mod tests {
    use super::*;
    use PatchSpacing::*;

    #[test]
    fn a_square_of_straight_edges_makes_a_flat_grid() {
        let (a, b, c, d) = (DVec3::ZERO, DVec3::X, DVec3::new(1., 0., 1.), DVec3::Z);
        let g = coons_patch(
            &[&[a, b], &[b, c], &[c, d], &[d, a]],
            10,
            10,
            Length,
            Length,
            true,
        )
        .unwrap();
        assert_eq!(g.mesh.faces.len(), 100);
        assert!(g.mesh.vertices.iter().all(|v| v.y.abs() < 1e-12));
        assert!((g.mesh.vertices[11 * 5 + 5] - DVec3::new(0.5, 0., 0.5)).length() < 1e-12);
        assert_eq!(g.corners, vec![0, 10, 120, 110]);
    }

    /// PC-4: Parallel runs along the first curve.
    #[test]
    fn parallel_counts_along_the_first_curve() {
        let (a, b, c, d) = (DVec3::ZERO, DVec3::X * 3., DVec3::new(3., 0., 1.), DVec3::Z);
        let g = coons_patch(
            &[&[a, b], &[b, c], &[c, d], &[d, a]],
            6,
            4,
            Length,
            Length,
            false,
        )
        .unwrap();
        assert_eq!(g.mesh.faces.len(), 24);
        // The first row of points runs along the first curve: 7 of them.
        let along: Vec<DVec3> = g.mesh.vertices[..7].to_vec();
        assert!(along.iter().all(|p| p.z.abs() < 1e-12));
        assert!((along[1].x - 0.5).abs() < 1e-12);
    }

    /// PC-3: three curves collapse the far side into one shared point.
    #[test]
    fn three_curves_make_one_row_of_triangles() {
        let (a, b, c) = (DVec3::ZERO, DVec3::X, DVec3::Z);
        let g = coons_patch(&[&[a, b], &[b, c], &[c, a]], 16, 16, Length, Length, false).unwrap();
        let tris = g
            .mesh
            .faces
            .iter()
            .filter(|f| f.vertices.len() == 3)
            .count();
        let quads = g
            .mesh
            .faces
            .iter()
            .filter(|f| f.vertices.len() == 4)
            .count();
        assert_eq!((quads, tris), (240, 16));
        assert_eq!(g.mesh.vertices.len(), 17 * 16 + 1);
        assert_eq!(g.corners.len(), 3);
        assert!((g.mesh.vertices[g.corners[2] as usize] - c).length() < 1e-12);
        for f in 0..g.mesh.faces.len() {
            assert!(g.mesh.normal(f).length() > 0.5, "polygon {f} has no area");
        }
    }

    /// PC-6: Knot spacing gives each span between control points an equal
    /// share, whatever its length.
    #[test]
    fn knot_spacing_splits_spans_evenly() {
        let pts = [DVec3::ZERO, DVec3::X, DVec3::X * 4.];
        let k = by_knot(&pts, 5);
        assert!((k[2] - DVec3::X).length() < 1e-12);
        let l = by_length(&pts, 5);
        assert!((l[2] - DVec3::X * 2.).length() < 1e-12);
    }
}
