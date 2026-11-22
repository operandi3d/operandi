//! Basic UVs for Cube, Sphere and Cylinder (Jason decision 2026-11-01,
//! SP-Q13/CYL-Q16). Provisional original layouts, not a reference atlas:
//!
//! - Sphere: latitude/longitude over the whole 0–1 square, the layout the
//!   Ellipsoid amendment specifies (a seam copy per ring, each pole corner at
//!   the middle of its side). For 8 sides and 4 segments this gives the 43
//!   texture records the reference sample reports.
//! - Cylinder: the wall unrolled along the bottom half (U around, V up) and
//!   one round island per cap in the top half. For 8 sides and 3 segments
//!   this gives the 52 records the reference sample reports; the exact
//!   packing is not specified.
//! - Cube: six tiles, +X/+Y/+Z on the top row and -X/-Y/-Z below, each
//!   face seen from outside with Y up (Z for the top and bottom faces).
//!
//! All go into the map named "Texture", as Cone and Ellipsoid do. Inserting
//! into a mesh extends an existing map of that name (see `append_mesh`).
use crate::Mesh;
use glam::{DVec2, DVec3};
use std::f64::consts::TAU;

/// UV map name shared by every primitive's generated UVs.
pub const PRIMITIVE_UV_MAP: &str = "Texture";

type FaceUvs = Vec<Vec<Option<DVec2>>>;

fn insert(mesh: &mut Mesh, uvs: FaceUvs) -> crate::Result<()> {
    if uvs.len() != mesh.faces.len()
        || uvs
            .iter()
            .zip(&mesh.faces)
            .any(|(uv, f)| uv.len() != f.vertices.len())
    {
        return Err("Primitive UVs don't match the generated polygons".into());
    }
    mesh.uv_maps.insert(PRIMITIVE_UV_MAP.into(), uvs);
    mesh.validate()
}

/// UVs for `globe_geometry(_, sides, segments)`, in its face order.
pub fn add_globe_uvs(mesh: &mut Mesh, sides: u32, segments: u32) -> crate::Result<()> {
    let (n, s) = (f64::from(sides), f64::from(segments));
    // Sides run clockwise seen from outside, so U counts down from 1 to
    // keep the texture unmirrored.
    let at = |side: u32, ring: u32| Some(DVec2::new(1. - f64::from(side) / n, f64::from(ring) / s));
    let pole = |side: u32, ring: u32| {
        Some(DVec2::new(
            1. - (f64::from(side) + 0.5) / n,
            f64::from(ring) / s,
        ))
    };
    let mut uvs = Vec::with_capacity(mesh.faces.len());
    for side in 0..sides {
        // `side + 1` is the seam copy at U = 0 for the last side.
        let next = side + 1;
        uvs.push(vec![pole(side, 0), at(side, 1), at(next, 1)]);
        for band in 0..segments - 2 {
            uvs.push(vec![
                at(side, band + 1),
                at(side, band + 2),
                at(next, band + 2),
                at(next, band + 1),
            ]);
        }
        uvs.push(vec![
            at(side, segments - 1),
            pole(side, segments),
            at(next, segments - 1),
        ]);
    }
    insert(mesh, uvs)
}

/// UVs for `cylinder_geometry(_, sides, segments)`, in its face order.
pub fn add_cylinder_uvs(mesh: &mut Mesh, sides: u32, segments: u32) -> crate::Result<()> {
    let (n, s) = (f64::from(sides), f64::from(segments));
    // As for the sphere, U counts down so the wall isn't mirrored.
    let wall = |side: u32, ring: u32| {
        Some(DVec2::new(
            1. - f64::from(side) / n,
            0.5 * f64::from(ring) / s,
        ))
    };
    // Cap islands: radius 0.24, bottom on the left, top on the right, kept
    // just clear of the wall strip below. Each is seen from outside the
    // cylinder (top from +Y, bottom from -Y).
    let cap = |side: u32, center: DVec2, flip: f64| {
        let angle = TAU * f64::from(side) / n;
        Some(center + 0.24 * DVec2::new(angle.cos(), flip * angle.sin()))
    };
    let mut uvs = Vec::with_capacity(mesh.faces.len());
    for ring in 0..segments {
        for side in 0..sides {
            uvs.push(vec![
                wall(side, ring),
                wall(side, ring + 1),
                wall(side + 1, ring + 1),
                wall(side + 1, ring),
            ]);
        }
    }
    uvs.push(
        (0..sides)
            .map(|side| cap(side, DVec2::new(0.25, 0.75), 1.))
            .collect(),
    );
    uvs.push(
        (0..sides)
            .rev()
            .map(|side| cap(side, DVec2::new(0.75, 0.75), -1.))
            .collect(),
    );
    insert(mesh, uvs)
}

/// UVs for a Cube centered at `position` with `size`, any segmentation.
/// Faces are placed by their normal, so it also covers the flat draft.
pub fn add_cube_uvs(mesh: &mut Mesh, position: DVec3, size: DVec3) -> crate::Result<()> {
    let mut uvs = Vec::with_capacity(mesh.faces.len());
    for (fi, face) in mesh.faces.iter().enumerate() {
        let normal = mesh.normal(fi);
        let axis = normal.abs().max_position();
        let positive = normal[axis] > 0.;
        // (right, up) as seen from outside the face.
        let (right, up) = match (axis, positive) {
            (0, true) => (DVec3::NEG_Z, DVec3::Y),
            (0, false) => (DVec3::Z, DVec3::Y),
            (1, true) => (DVec3::X, DVec3::NEG_Z),
            (1, false) => (DVec3::X, DVec3::Z),
            (_, true) => (DVec3::X, DVec3::Y),
            (_, false) => (DVec3::NEG_X, DVec3::Y),
        };
        let tile = DVec2::new(axis as f64 / 3., if positive { 0.5 } else { 0. });
        let local = |p: DVec3, direction: DVec3| {
            (p - position).dot(direction) / size.dot(direction).abs() + 0.5
        };
        uvs.push(
            face.vertices
                .iter()
                .map(|&v| {
                    let p = mesh.vertices[v as usize];
                    let st = DVec2::new(local(p, right), local(p, up));
                    Some(tile + st * DVec2::new(1. / 3., 0.5))
                })
                .collect(),
        );
    }
    insert(mesh, uvs)
}
