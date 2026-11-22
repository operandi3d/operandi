//! Sketch Extrude along a drawn path against
//! `specifications/sketch-extrude-drag-2026-11-06` (E1): one section per
//! path step, and with Align to Path on (the default) the sections turn to
//! follow the path, so the end cap faces the path's last direction.
use glam::DVec3;
use operandi_core::*;

#[test]
fn ten_point_curve_makes_nine_turning_sections() {
    let cube = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    let top = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y > 0.5)
        .unwrap() as u32;
    // A quarter arc from the top centre, up and over to the right.
    let path: Vec<DVec3> = (0..10)
        .map(|i| {
            let t = std::f64::consts::FRAC_PI_2 * i as f64 / 9.;
            DVec3::new(0.5 + 1.5 * (1. - t.cos()), 0.5 + 1.5 * t.sin(), 0.)
                - DVec3::new(0.5, 0., 0.)
        })
        .collect();
    let (mesh, caps) = sketch_extrude_geometry(&cube, &[top], &path).unwrap();
    // 8/6 → 44/42: nine rings of four points with side walls.
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (44, 42));
    let cap = caps[0] as usize;
    let last = (path[9] - path[8]).normalize();
    let normal = mesh.normal(cap).normalize();
    assert!(normal.dot(last) > 0.99, "end cap faces the path: {normal}");
    // The cap is centred on the path's end and keeps the section's size.
    assert!((mesh.center(cap) - path[9] - DVec3::new(0., 0.5, 0.) + path[0]).length() < 1e-9);
    let ring = &mesh.faces[cap].vertices;
    for i in 0..4 {
        let e = mesh.vertices[ring[(i + 1) % 4] as usize] - mesh.vertices[ring[i] as usize];
        assert!((e.length() - 1.).abs() < 1e-9);
    }
}
