//! Unit Cone, Torus and Capsule (Ctrl-click / Shift-click), provisional.
use glam::DVec3;
use operandi_core::*;

fn bounds(m: &Mesh) -> (DVec3, DVec3) {
    m.vertices.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), v| (lo.min(*v), hi.max(*v)),
    )
}

#[test]
fn unit_shapes_fit_a_metre_and_are_one_undo_step() {
    for shape in [
        UnitShape::Cone,
        UnitShape::Torus,
        UnitShape::Capsule,
        UnitShape::Plane,
    ] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        let steps = e.history.undo.len();
        e.dispatch(Command::InsertShapeUnit(
            shape,
            UnitDestination::ExistingMesh,
        ))
        .unwrap();
        let mesh = &e.scene.active().unwrap().mesh;
        assert!(!mesh.faces.is_empty(), "{shape:?}");
        let (lo, hi) = bounds(mesh);
        let size = hi - lo;
        // The tools' reset shapes (unit-primitive-2026-11-05): Cone and
        // Capsule 1.5 m tall, the Torus 1.5 m across and 0.5 m tall.
        let want = match shape {
            UnitShape::Cone => DVec3::new(1., 1.5, 1.),
            UnitShape::Capsule => DVec3::new(0.5, 1.5, 0.5),
            UnitShape::Torus => DVec3::new(1.5, 0.5, 1.5),
            // Plane Item's square (missing-tools-g2 MT5-02).
            UnitShape::Plane => DVec3::new(1., 0., 1.),
            UnitShape::Teapot => unreachable!("teapot_spec.rs checks the Teapot"),
        };
        assert!((size - want).length() < 1e-6, "{shape:?} {size}");
        assert!(((lo + hi) / 2.).length() < 1e-6, "{shape:?} centred");
        assert_eq!(e.history.undo.len(), steps + 1);
        e.dispatch(Command::Undo).unwrap();
        assert!(e.scene.active().unwrap().mesh.faces.is_empty());
    }
}

#[test]
fn shift_click_unit_makes_a_new_named_mesh() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let count = e.scene.items.len();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Torus,
        UnitDestination::NewMesh,
    ))
    .unwrap();
    assert_eq!(e.scene.items.len(), count + 1);
    assert_eq!(e.scene.active().unwrap().name, "Torus");
    // Undo removes the mesh and its geometry together.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items.len(), count);
}

#[test]
fn unit_drops_a_live_tool_and_accepts_a_moved_mesh() {
    // Unit insertion drops an open tool first, like Cube, Sphere and
    // Cylinder (tool-drop, Jason 2026-11-05).
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Cone,
        UnitDestination::ExistingMesh,
    ))
    .unwrap();
    assert!(e.session.cone.is_none());
    assert!(!e.scene.active().unwrap().mesh.faces.is_empty());
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().transform.translation = DVec3::X;
    // A moved mesh takes it too (primitive_moved_mesh.rs).
    e.dispatch(Command::InsertShapeUnit(
        UnitShape::Capsule,
        UnitDestination::ExistingMesh,
    ))
    .unwrap();
    assert_eq!(e.scene.active().unwrap().transform.translation, DVec3::X);
    assert!(!e.scene.active().unwrap().mesh.faces.is_empty());
}
