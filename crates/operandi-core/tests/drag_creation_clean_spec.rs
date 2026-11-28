//! Drag creation's flat base against `specifications/drag-creation-2026-11-06`:
//! the first drag gives a flat shape that Drop keeps (Cube 4/1, Sphere and
//! Cylinder a 24-sided disc). The height stage is open (Q-DC-01).
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.synchronize_identities().unwrap();
    e
}

fn counts(e: &Editor) -> (usize, usize) {
    let m = &e.scene.active().unwrap().mesh;
    (m.vertices.len(), m.faces.len())
}

#[test]
fn cube_base_is_one_rectangle_and_drop_keeps_it() {
    // D1/D2: centre (0.409, −0.09, 0), size 0.822 × 0.168 × 0.
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.position = DVec3::new(0.409, -0.09, 0.);
    p.size = DVec3::new(0.822, 0.168, 0.);
    p.segments = [1, 1, 1];
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(counts(&e), (4, 1));
}

#[test]
fn sphere_and_cylinder_bases_are_24_sided_discs() {
    for (tool, radius) in [
        (Tool::Sphere, DVec3::new(0.516, 0.086, 0.)),
        (Tool::Cylinder, DVec3::new(0.822, 0.168, 0.)),
    ] {
        let mut e = editor();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.position = DVec3::ZERO;
        let radial = p.radial.as_mut().unwrap();
        radial.radius = radius;
        assert_eq!(radial.sides, 24, "{tool:?} default Sides");
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(counts(&e), (24, 1), "{tool:?}");
        let m = &e.scene.active().unwrap().mesh;
        // Flat in XY, centred on the drag start, facing +Z.
        assert!(m.vertices.iter().all(|v| v.z == 0.));
        assert!(m.normal(0).z > 0.);
        let max_x = m.vertices.iter().map(|v| v.x).fold(f64::MIN, f64::max);
        assert!((max_x - radius.x).abs() < 1e-12);
    }
}
