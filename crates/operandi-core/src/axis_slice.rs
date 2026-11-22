//! Wave 3 Polygon Axis Slice (spec WAVE3-AXIS-SLICE, AX-10..AX-14):
//! straight cuts across the selected polygons along planes perpendicular to
//! world X, Y and/or Z, spread over the selection's bounding box. It reuses
//! Polygon Slice's plane cut with Infinite on, once per plane. Nothing
//! moves; polygons are split.
//!
//! Spacing ([`axis_slice_planes`]) and the starting values are measured
//! (spec v1, AX-11..AX-14).
use crate::slice::{SliceResult, plane_cut};
use crate::{Mesh, Result};
use glam::{DMat4, DVec3};
use std::collections::BTreeSet;

/// Slice Mode: a count of cuts per axis, or a spacing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AxisSliceMode {
    #[default]
    Number,
    Size,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AxisSlice {
    pub mode: AxisSliceMode,
    /// Cuts per axis inside the box (Number mode); 0 = none on that axis.
    pub number: [u32; 3],
    /// Spacing between cuts per axis (Size mode); 0 = none on that axis.
    pub size: DVec3,
    /// Shifts every cut along each axis.
    pub offset: DVec3,
    /// Hold Slice: pause cutting while adjusting values (the planes are
    /// still drawn).
    pub hold: bool,
}
impl Default for AxisSlice {
    fn default() -> Self {
        Self {
            mode: AxisSliceMode::Number,
            number: [0, 0, 0],
            size: DVec3::ONE,
            offset: DVec3::ZERO,
            hold: false,
        }
    }
}

/// Most cuts on one axis (Size mode on a tiny spacing would otherwise make
/// millions).
pub const AXIS_SLICE_MAX_CUTS: usize = 1000;

/// The cut positions on each world axis for a box `min`..`max` (spec v1).
/// Number (AX-13): N cuts spaced s = L / (N + 1), at min + k·s + Offset
/// for k = 1..N, each wrapped round the box's length, so a cut slid past
/// one end comes back in at the other and the group keeps its spacing
/// (built from Jason's hands-on check; W3-AX-Q1). Size (AX-14): cuts at
/// min + Offset + k·Size for every whole k that lands strictly inside the
/// box. An axis with no extent gets none.
pub fn axis_slice_planes(min: DVec3, max: DVec3, o: &AxisSlice) -> [Vec<f64>; 3] {
    let mut out: [Vec<f64>; 3] = Default::default();
    for axis in 0..3 {
        let (lo, hi) = (min[axis], max[axis]);
        let length = hi - lo;
        if !(length.is_finite() && length > 1e-12) {
            continue;
        }
        let offset = o.offset[axis];
        if !offset.is_finite() {
            continue;
        }
        let eps = 1e-9 * length.max(1.);
        out[axis] = match o.mode {
            AxisSliceMode::Number => {
                let n = (o.number[axis] as usize).min(AXIS_SLICE_MAX_CUTS);
                if n == 0 {
                    continue;
                }
                let spacing = length / (n + 1) as f64;
                // Each cut slides with Offset and wraps at the box's ends,
                // so a cut leaving one side comes back in at the other and
                // the group keeps its spacing (Jason, 2026-11-08: in the
                // reference it "feels like wrapping"). One recorded case
                // disagrees (W3-AX-Q1).
                let mut cuts: Vec<f64> = (1..=n)
                    .map(|k| lo + (k as f64 * spacing + offset).rem_euclid(length))
                    .filter(|&x| x > lo + eps && x < hi - eps)
                    .collect();
                cuts.sort_by(f64::total_cmp);
                cuts
            }
            AxisSliceMode::Size => {
                let size = o.size[axis];
                if !(size.is_finite() && size > 1e-9) || length / size > AXIS_SLICE_MAX_CUTS as f64
                {
                    continue;
                }
                let first = lo + offset;
                let k0 = ((lo - first) / size).floor() as i64;
                let k1 = ((hi - first) / size).ceil() as i64;
                (k0..=k1)
                    .map(|k| first + k as f64 * size)
                    .filter(|&x| x > lo + eps && x < hi - eps)
                    .collect()
            }
        };
    }
    out
}

/// The world bounding box of `faces` (all surface polygons when empty).
pub fn axis_slice_bounds(mesh: &Mesh, world: DMat4, faces: &[u32]) -> Option<(DVec3, DVec3)> {
    let mut used = vec![false; mesh.vertices.len()];
    let mut any = false;
    let mut mark = |f: usize| {
        if f < mesh.faces.len() && !mesh.is_polyline(f) {
            for &v in &mesh.faces[f].vertices {
                used[v as usize] = true;
                any = true;
            }
        }
    };
    if faces.is_empty() {
        (0..mesh.faces.len()).for_each(&mut mark);
    } else {
        faces.iter().for_each(|&f| mark(f as usize));
    }
    if !any {
        return None;
    }
    let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
    for (v, &u) in used.iter().enumerate() {
        if u {
            let p = world.transform_point3(mesh.vertices[v]);
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    Some((lo, hi))
}

/// Polygon Axis Slice geometry: every plane in turn, on `selected` (all
/// polygons when empty) and the pieces earlier planes made of them.
/// Returns the mesh, the new edges and the cut pieces.
pub fn axis_slice_geometry(
    mesh: &Mesh,
    world: DMat4,
    o: &AxisSlice,
    selected: &[u32],
) -> Result<SliceResult> {
    if o.size.is_nan() || o.offset.is_nan() || !o.offset.is_finite() {
        return Err("Axis Slice values must be finite numbers".into());
    }
    if o.hold {
        return Ok((mesh.clone(), Vec::new(), Vec::new()));
    }
    let Some((min, max)) = axis_slice_bounds(mesh, world, selected) else {
        return Ok((mesh.clone(), Vec::new(), Vec::new()));
    };
    let planes = axis_slice_planes(min, max, o);
    let local = world.inverse();
    let mut out = mesh.clone();
    let mut considered: BTreeSet<usize> = if selected.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        selected.iter().map(|&f| f as usize).collect()
    };
    let mut edges: Vec<(u32, u32)> = Vec::new();
    let mut pieces: BTreeSet<u32> = BTreeSet::new();
    let axes = [DVec3::X, DVec3::Y, DVec3::Z];
    for (axis, list) in planes.iter().enumerate() {
        for &x in list {
            let mut origin = DVec3::ZERO;
            origin[axis] = x;
            // The plane in mesh-local space: through the mapped point, its
            // normal mapped by the transpose of the mesh's linear part.
            let local_origin = local.transform_point3(origin);
            let normal = world.transpose().transform_vector3(axes[axis]);
            if normal.length() < 1e-12 {
                continue;
            }
            let normal = normal.normalize();
            let along = normal.any_orthonormal_vector();
            let before = out.faces.len();
            let ((mesh, new_edges, new_pieces), _) =
                plane_cut(&out, local_origin, normal, along, 0., true, &considered)?;
            out = mesh;
            considered.extend(before..out.faces.len());
            edges.extend(new_edges);
            pieces.extend(new_pieces);
        }
    }
    let existing: BTreeSet<(u32, u32)> = out.edges().into_iter().collect();
    edges.retain(|e| existing.contains(e));
    edges.sort_unstable();
    edges.dedup();
    Ok((out, edges, pieces.into_iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn grid(n: u32) -> Mesh {
        let mut mesh = Mesh::default();
        for j in 0..=n {
            for i in 0..=n {
                mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
            }
        }
        for j in 0..n {
            for i in 0..n {
                let a = j * (n + 1) + i;
                mesh.faces.push(Face {
                    vertices: vec![a, a + n + 1, a + n + 2, a + 1],
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
            }
        }
        mesh
    }

    fn close(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
    }

    /// The spec's examples on a quad x -2..2 (AX-13, AX-14).
    fn x_cuts(o: AxisSlice) -> Vec<f64> {
        axis_slice_planes(DVec3::new(-2., 0., -1.), DVec3::new(2., 0., 1.), &o)[0].clone()
    }

    #[test]
    fn number_mode_spacing_and_wrapped_offset() {
        let n = |x: u32, off: f64| AxisSlice {
            number: [x, 0, 0],
            offset: DVec3::new(off, 0., 0.),
            ..Default::default()
        };
        assert!(
            x_cuts(AxisSlice::default()).is_empty(),
            "Numbers start at 0"
        );
        assert!(close(&x_cuts(n(1, 0.)), &[0.]));
        assert!(close(&x_cuts(n(2, 0.)), &[-0.667, 0.667]));
        assert!(close(&x_cuts(n(3, 0.)), &[-1., 0., 1.]));
        assert!(close(&x_cuts(n(2, 0.3)), &[-0.367, 0.967]));
        // Offset 2.5 on one cut: wraps at the box's end to -1.5 (the
        // recorded case says 0.5: W3-AX-Q1).
        assert!(close(&x_cuts(n(1, 2.5)), &[-1.5]));
        // Through the middle and on to the edge without jumping, then in
        // again at the other side (Jason's report).
        assert!(close(&x_cuts(n(1, -0.01)), &[-0.01]));
        assert!(close(&x_cuts(n(1, -1.9)), &[-1.9]));
        assert!(close(&x_cuts(n(1, -2.1)), &[1.9]));
        // Three cuts: the left one leaves and comes back on the right,
        // keeping the 1 m spacing.
        assert!(close(&x_cuts(n(3, -1.2)), &[-1.2, -0.2, 1.8]));
        // No extent on Y: no cuts there.
        let o = AxisSlice {
            number: [0, 3, 0],
            ..Default::default()
        };
        let p = axis_slice_planes(DVec3::new(-2., 0., -1.), DVec3::new(2., 0., 1.), &o);
        assert!(p[1].is_empty());
    }

    #[test]
    fn size_mode_from_the_box_start() {
        let s = |size: f64, off: f64| AxisSlice {
            mode: AxisSliceMode::Size,
            size: DVec3::new(size, 100., 100.),
            offset: DVec3::new(off, 0., 0.),
            ..Default::default()
        };
        assert!(close(&x_cuts(s(1., 0.)), &[-1., 0., 1.]));
        assert!(close(&x_cuts(s(1.5, 0.)), &[-0.5, 1.]));
        assert!(close(&x_cuts(s(0.7, 0.)), &[-1.3, -0.6, 0.1, 0.8, 1.5]));
        assert!(close(&x_cuts(s(1., 0.3)), &[-1.7, -0.7, 0.3, 1.3]));
        let off = axis_slice_planes(DVec3::new(1., 0., 0.), DVec3::new(5., 0., 1.), &s(1., 0.));
        assert!(close(&off[0], &[2., 3., 4.]));
        // The starting values: 1 m on every axis.
        let o = AxisSlice {
            mode: AxisSliceMode::Size,
            ..Default::default()
        };
        let p = axis_slice_planes(DVec3::new(-2., 0., -1.), DVec3::new(2., 0., 1.), &o);
        assert!(close(&p[0], &[-1., 0., 1.]) && close(&p[2], &[0.]) && p[1].is_empty());
    }

    #[test]
    fn cuts_split_the_selected_polygons_only() {
        let mesh = grid(2);
        let o = AxisSlice {
            number: [1, 0, 0],
            ..Default::default()
        };
        // Polygon 0 spans x 0..1: the one cut at x = 0.5.
        let (out, edges, pieces) = axis_slice_geometry(&mesh, DMat4::IDENTITY, &o, &[0]).unwrap();
        assert_eq!(out.faces.len(), 5);
        assert_eq!(edges.len(), 1);
        assert_eq!(pieces.len(), 2);
        // No selection: the whole grid, two cuts per axis at 2/3 and 4/3,
        // each through a row of two polygons.
        let o = AxisSlice {
            number: [2, 0, 2],
            ..Default::default()
        };
        let (out, _, _) = axis_slice_geometry(&mesh, DMat4::IDENTITY, &o, &[]).unwrap();
        assert_eq!(out.faces.len(), 16);
        out.validate().unwrap();
    }

    #[test]
    fn hold_slice_changes_nothing() {
        let mesh = grid(2);
        let o = AxisSlice {
            hold: true,
            ..Default::default()
        };
        let (out, _, _) = axis_slice_geometry(&mesh, DMat4::IDENTITY, &o, &[]).unwrap();
        assert_eq!(out.faces.len(), 4);
    }

    #[test]
    fn a_moved_item_cuts_in_world_space() {
        let mesh = grid(1);
        let world = DMat4::from_translation(DVec3::new(10., 0., 0.));
        let o = AxisSlice {
            number: [1, 0, 0],
            ..Default::default()
        };
        let (out, _, _) = axis_slice_geometry(&mesh, world, &o, &[]).unwrap();
        assert_eq!(out.faces.len(), 2);
        let mid = out
            .vertices
            .iter()
            .filter(|p| (p.x - 0.5).abs() < 1e-9)
            .count();
        assert_eq!(mid, 2);
    }
}
