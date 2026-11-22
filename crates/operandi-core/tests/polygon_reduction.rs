//! MP-T11 RED-001..004 core acceptance for basic Polygon Reduction.
//!
//! The 25-point/32-triangle grid, its common face values, the unrelated item
//! and the separately authored valid 24-triangle example come from the
//! admitted original `ORIGINAL_FIXTURE.json`. Results are judged by the
//! independent `reduction_oracle` (no reducer code, chart or tolerance), whose
//! sensitivity is checked first. Expected topology is never derived from the
//! reducer; only geometric validity, counts, data and history are asserted.
#[path = "support/reduction_oracle.rs"]
mod oracle;

use glam::DVec3;
use operandi_core::*;

#[test]
fn quadric_reduction_of_a_bumpy_quad_grid() {
    // 10 x 10 quads with a smooth bump; reduce to 60 polygons.
    let n = 11u32;
    let vertices: Vec<DVec3> = (0..n)
        .flat_map(|y| {
            (0..n).map(move |x| {
                let (fx, fy) = (x as f64 / 10. - 0.5, y as f64 / 10. - 0.5);
                DVec3::new(fx, fy, 0.3 * (-(fx * fx + fy * fy) * 8.).exp())
            })
        })
        .collect();
    let faces: Vec<Face> = (0..10u32)
        .flat_map(|y| (0..10u32).map(move |x| (x, y)))
        .map(|(x, y)| Face {
            vertices: vec![
                y * n + x,
                y * n + x + 1,
                (y + 1) * n + x + 1,
                (y + 1) * n + x,
            ],
            uv: None,
            material: "Grid".into(),
            material_label: None,
            smooth: true,
            part: None,
        })
        .collect();
    let mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    mesh.validate().unwrap();
    let selected: Vec<u32> = (0..100).collect();
    assert_eq!(reduction_source_count(&mesh, &selected).unwrap(), 100);
    let out = reduce_planar_disk(&mesh, &selected, 60).unwrap();
    out.mesh.validate().unwrap();
    assert!(out.achieved <= 60, "achieved {}", out.achieved);
    assert_eq!(out.mesh.faces.len() as u32, out.achieved);
    // Surviving points keep their positions; the four corners survive.
    for &corner in &[DVec3::new(-0.5, -0.5, 0.), DVec3::new(0.5, 0.5, 0.)] {
        assert!(
            out.mesh
                .vertices
                .iter()
                .any(|p| p.truncate().distance(corner.truncate()) < 1e-12)
        );
    }
    // Every face still faces up.
    for fi in 0..out.mesh.faces.len() {
        assert!(out.mesh.normal(fi).z > 0.);
    }
}

fn triangle_mesh(points: &[(f64, f64, f64)], triangles: &[[u32; 3]]) -> Mesh {
    Mesh {
        vertices: points
            .iter()
            .map(|&(x, y, z)| DVec3::new(x, y, z))
            .collect(),
        faces: triangles
            .iter()
            .map(|triangle| Face {
                vertices: triangle.to_vec(),
                uv: None,
                material: "Grid".into(),
                material_label: None,
                smooth: true,
                part: None,
            })
            .collect::<Vec<_>>()
            .into(),
        ..Default::default()
    }
}

fn square_fan() -> Mesh {
    triangle_mesh(
        &[
            (0., 0., 0.),
            (2., 0., 0.),
            (2., 2., 0.),
            (0., 2., 0.),
            (1., 1., 0.),
        ],
        &[[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
    )
}

#[test]
fn reduction_planar_disk_preserves_corners_area_and_correspondence() {
    let mesh = square_fan();
    let disk = validate_planar_disk(&mesh).unwrap();
    assert_eq!(disk.boundary, vec![0, 1, 2, 3]);
    assert!((disk.area - 4.).abs() < 1e-12);
    assert!(disk.normal.dot(DVec3::Z) > 0.999999);
    assert_eq!(reduction_source_count(&mesh, &[3, 1, 0, 2]).unwrap(), 4);

    let out = reduce_planar_disk(&mesh, &[0, 1, 2, 3], 2).unwrap();
    assert_eq!(out.requested, 2);
    assert_eq!(out.achieved, 2);
    assert_eq!(out.mesh.faces.len(), 2);
    out.mesh.validate().unwrap();
    out.correspondence.validate(&mesh, &out.mesh).unwrap();
    let result = validate_planar_disk(&out.mesh).unwrap();
    assert!((result.area - disk.area).abs() < 1e-12);
    for point in &mesh.vertices[..4] {
        assert!(out.mesh.vertices.contains(point), "lost corner {point:?}");
    }
}

#[test]
fn reduction_unreachable_target_reports_actual_count() {
    let mesh = square_fan();
    let out = reduce_polygons(&mesh, &[0, 1, 2, 3], 1, &ReductionParameters::default()).unwrap();
    assert_eq!(out.requested, 1);
    assert_eq!(out.achieved, 2);
    assert_eq!(out.mesh.faces.len(), 2);
    out.mesh.validate().unwrap();
    for point in &mesh.vertices[..4] {
        assert!(out.mesh.vertices.contains(point));
    }
}

#[test]
fn reduction_rejects_invalid_selection_and_requested_counts() {
    let mesh = square_fan();
    for (selection, message) in [
        (&[][..], "empty selection"),
        (&[0, 0][..], "Duplicate"),
        (&[99][..], "no longer exists"),
    ] {
        assert!(
            reduction_source_count(&mesh, selection)
                .unwrap_err()
                .contains(message)
        );
        assert!(
            reduce_planar_disk(&mesh, selection, 1)
                .unwrap_err()
                .contains(message)
        );
    }
    let selection = [0, 1, 2, 3];
    assert!(
        reduce_planar_disk(&mesh, &selection, 0)
            .unwrap_err()
            .contains("positive")
    );
    assert!(
        reduce_planar_disk(&mesh, &selection, 5)
            .unwrap_err()
            .contains("exceeds")
    );
    let params = ReductionParameters {
        boundary_weight: 50.,
        ..Default::default()
    };
    for count in [0, 5] {
        assert!(reduce_polygons(&mesh, &selection, count, &params).is_err());
    }
    let invalid = ReductionParameters {
        boundary_weight: f64::NAN,
        ..Default::default()
    };
    assert!(
        reduce_polygons(&mesh, &selection, 2, &invalid)
            .unwrap_err()
            .contains("Weight")
    );
    assert_eq!(mesh.faces.len(), 4);
    mesh.validate().unwrap();
}

#[test]
fn reduction_planar_validation_rejects_bad_geometry() {
    assert!(
        validate_planar_disk(&Mesh::default())
            .unwrap_err()
            .contains("no triangles")
    );
    let mut mesh = square_fan();
    mesh.vertices[4].z = 0.5;
    assert!(
        validate_planar_disk(&mesh)
            .unwrap_err()
            .contains("nonplanar")
    );
    assert_eq!(reduction_source_count(&mesh, &[0, 1, 2, 3]).unwrap(), 4);
    let out = reduce_planar_disk(&mesh, &[0, 1, 2, 3], 4).unwrap();
    assert_eq!(out.achieved, 4);
    out.mesh.validate().unwrap();

    let repeated = triangle_mesh(&[(0., 0., 0.), (1., 0., 0.)], &[[0, 1, 1]]);
    assert!(
        validate_planar_disk(&repeated)
            .unwrap_err()
            .contains("repeats")
    );
    let missing = triangle_mesh(&[(0., 0., 0.), (1., 0., 0.)], &[[0, 1, 2]]);
    assert!(
        validate_planar_disk(&missing)
            .unwrap_err()
            .contains("missing point")
    );
    let mut isolated = square_fan();
    isolated.vertices.push(DVec3::new(5., 5., 0.));
    assert!(
        validate_planar_disk(&isolated)
            .unwrap_err()
            .contains("isolated")
    );
}

#[test]
fn reduction_hole_is_not_a_planar_disk_but_general_path_keeps_boundary() {
    let mesh = triangle_mesh(
        &[
            (0., 0., 0.),
            (3., 0., 0.),
            (3., 3., 0.),
            (0., 3., 0.),
            (1., 1., 0.),
            (2., 1., 0.),
            (2., 2., 0.),
            (1., 2., 0.),
        ],
        &[
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    );
    mesh.validate().unwrap();
    assert!(
        validate_planar_disk(&mesh)
            .unwrap_err()
            .contains("boundary loop")
    );
    let selected: Vec<u32> = (0..8).collect();
    assert_eq!(reduction_source_count(&mesh, &selected).unwrap(), 8);
    let out = reduce_planar_disk(&mesh, &selected, 8).unwrap();
    out.mesh.validate().unwrap();
    assert_eq!(out.achieved, 8);
    assert_eq!(out.mesh.faces.len(), 8);
    for point in &mesh.vertices {
        assert!(
            out.mesh.vertices.contains(point),
            "lost boundary point {point:?}"
        );
    }
}
