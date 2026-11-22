//! Interactive Pen on non-empty and transformed meshes (fix pass: Pen adds
//! geometry to the mesh vertex by vertex).
use glam::DVec3;
use operandi_core::*;

fn editor_with_cube() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    e.scene.selection.mode = Mode::Polygon;
    e.synchronize_identities().unwrap();
    e
}

#[test]
fn pen_adds_a_polygon_to_an_existing_mesh_on_any_plane() {
    let mut e = editor_with_cube();
    e.dispatch(Command::PenStart).unwrap();
    // A triangle on the ground (XZ) plane, away from the cube.
    for p in [
        DVec3::new(2., 0., 0.),
        DVec3::new(3., 0., 0.),
        DVec3::new(2., 0., -1.),
    ] {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
    e.dispatch(Command::Drop).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 7, "cube kept, one polygon added");
    assert_eq!(e.scene.selection.faces, vec![6]);
    assert_eq!(m.vertices[8], DVec3::new(2., 0., 0.));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
}

#[test]
fn pen_points_map_into_a_transformed_item() {
    let mut e = editor_with_cube();
    e.scene.active_mut().unwrap().transform.translation = DVec3::new(10., 0., 0.);
    e.dispatch(Command::PenStart).unwrap();
    for p in [
        DVec3::new(12., 1., 0.),
        DVec3::new(13., 1., 0.),
        DVec3::new(13., 2., 0.),
    ] {
        e.dispatch(Command::PenCapture(p)).unwrap();
    }
    e.dispatch(Command::Drop).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(
        m.vertices[8],
        DVec3::new(2., 1., 0.),
        "stored in local space"
    );
}
