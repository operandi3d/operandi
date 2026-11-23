//! Issue #5: forward Delete of an edge that ends on the border (a loop cut
//! across an open surface) takes out the border points it leaves in a
//! straight line, as it already did for interior points. Border points at
//! a real corner stay, and Edge Remove (Shift+Backspace) keeps every point.
#![allow(clippy::field_reassign_with_default)] // meshes built field by field
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn mesh(points: &[(f64, f64)], faces: &[&[u32]]) -> Mesh {
    let mut m = Mesh::default();
    m.vertices = points.iter().map(|&(x, z)| DVec3::new(x, 0., z)).collect();
    m.faces = faces
        .iter()
        .map(|f| face(f.to_vec()))
        .collect::<Vec<_>>()
        .into();
    m.attributes.representations = vec![PolygonRepresentation::Face; faces.len()];
    m
}

/// A plane cut once down the middle: two quads sharing edge 1-4.
fn cut_plane() -> Mesh {
    mesh(
        &[(0., 0.), (1., 0.), (2., 0.), (2., 1.), (1., 1.), (0., 1.)],
        &[&[0, 1, 4, 5], &[1, 2, 3, 4]],
    )
}

#[test]
fn deleting_a_loop_cut_on_an_open_plane_takes_its_border_points() {
    let out = edge_delete_geometry(&cut_plane(), &[(1, 4)]).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices.len(), 4);
    assert_eq!(out.vertices.len(), 4);
    for gone in [DVec3::new(1., 0., 0.), DVec3::new(1., 0., 1.)] {
        assert!(out.vertices.iter().all(|v| v.distance(gone) > 1e-9));
    }
}

#[test]
fn deleting_a_loop_across_a_strip_takes_every_border_point() {
    // Three quads in a row; delete both inner edges.
    let m = mesh(
        &[
            (0., 0.),
            (1., 0.),
            (2., 0.),
            (3., 0.),
            (3., 1.),
            (2., 1.),
            (1., 1.),
            (0., 1.),
        ],
        &[&[0, 1, 6, 7], &[1, 2, 5, 6], &[2, 3, 4, 5]],
    );
    let out = edge_delete_geometry(&m, &[(1, 6), (2, 5)]).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices.len(), 4);
    assert_eq!(out.vertices.len(), 4);
}

#[test]
fn edge_remove_still_keeps_the_border_points() {
    let m = cut_plane();
    let (out, _) = edge_command_geometry(&m, &[(1, 4)], EdgeCommand::Remove).unwrap();
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices.len(), 6);
    assert_eq!(out.vertices.len(), 6);
}

#[test]
fn border_corners_stay() {
    // A quad split along its diagonal: both ends of the diagonal are
    // corners, so the quad comes back whole.
    let m = mesh(
        &[(0., 0.), (1., 0.), (1., 1.), (0., 1.)],
        &[&[0, 1, 2], &[0, 2, 3]],
    );
    let out = edge_delete_geometry(&m, &[(0, 2)]).unwrap();
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 1);
    assert_eq!(out.faces[0].vertices.len(), 4);
    assert_eq!(out.vertices.len(), 4);
    // A cut whose ends bend the outline: they are corners and stay.
    let bent = mesh(
        &[(0., 0.), (1., 0.5), (2., 0.), (2., 1.), (1., 1.5), (0., 1.)],
        &[&[0, 1, 4, 5], &[1, 2, 3, 4]],
    );
    let out = edge_delete_geometry(&bent, &[(1, 4)]).unwrap();
    assert_eq!(out.faces[0].vertices.len(), 6);
    assert_eq!(out.vertices.len(), 6);
}
