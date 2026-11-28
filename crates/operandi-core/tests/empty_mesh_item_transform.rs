//! Item-mode Move, Rotate and Scale work on a mesh item with no geometry
//! (GitHub issue #3): a fresh empty mesh, and a mesh whose only shape comes
//! from a Mesh Stack primitive on an empty Base Mesh. The pivot is the
//! item's own position.
use glam::DVec3;
use operandi_core::*;

fn empty_mesh() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
    e
}

fn mesh_stack_only() -> Editor {
    let mut e = empty_mesh();
    e.dispatch(Command::MeshOpCreate("prim.cube.item".into()))
        .unwrap();
    assert!(e.scene.active().unwrap().mesh.vertices.is_empty());
    e
}

/// Runs `tool` in Item mode with `delta` and returns the active item's
/// transform after the drop, plus the tool's handle centre.
fn transform_item(mut e: Editor, tool: Tool, delta: DVec3) -> (scene::Transform, DVec3) {
    let id = e.scene.active().unwrap().id;
    e.scene.items[0].transform.translation = DVec3::new(1., 2., 3.);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    e.dispatch(Command::Activate(tool)).unwrap();
    let center = Editor::extrude_frame(e.action_context, tool, &e.scene)
        .unwrap()
        .center;
    e.dispatch(Command::Preview(Parameters {
        delta,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let after = e.scene.active().unwrap().transform.clone();
    // One undo step puts it back.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(
        e.scene.active().unwrap().transform.translation,
        DVec3::new(1., 2., 3.)
    );
    (after, center)
}

fn check_all(make: fn() -> Editor) {
    let (t, center) = transform_item(make(), Tool::Move, DVec3::new(0.5, 0., 0.));
    assert!(
        (center - DVec3::new(1., 2., 3.)).length() < 1e-9,
        "{center:?}"
    );
    assert!((t.translation - DVec3::new(1.5, 2., 3.)).length() < 1e-9);
    let (t, _) = transform_item(make(), Tool::Rotate, DVec3::new(0., 30., 0.));
    assert!((t.rotation.y - 30f64.to_radians()).abs() < 1e-9, "{t:?}");
    assert!((t.translation - DVec3::new(1., 2., 3.)).length() < 1e-9);
    let (t, _) = transform_item(make(), Tool::Scale, DVec3::new(2., 2., 2.));
    assert!((t.scale - DVec3::splat(2.)).length() < 1e-9, "{t:?}");
    assert!((t.translation - DVec3::new(1., 2., 3.)).length() < 1e-9);
}

/// The Transform tool's numeric fields (move, rotate and scale at once).
#[test]
fn empty_mesh_item_takes_the_transform_tool() {
    let mut e = empty_mesh();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Transform)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0., 1., 0.),
        rotation_delta: DVec3::new(0., 0., 90.),
        scale_delta: DVec3::splat(3.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let t = &e.scene.active().unwrap().transform;
    assert!((t.translation - DVec3::Y).length() < 1e-9, "{t:?}");
    assert!((t.rotation.z - 90f64.to_radians()).abs() < 1e-9, "{t:?}");
    assert!((t.scale - DVec3::splat(3.)).length() < 1e-9, "{t:?}");
}

/// An empty mesh selected with a cube moves with it.
#[test]
fn empty_mesh_moves_with_other_selected_items() {
    let mut e = empty_mesh();
    let empty = e.scene.active().unwrap().id;
    e.dispatch(Command::NewMesh).unwrap();
    let cube = e.scene.active().unwrap().id;
    e.scene.items[1].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::SelectItems(vec![empty, cube])).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::new(0., 0., 2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    for item in &e.scene.items {
        assert!((item.transform.translation - DVec3::new(0., 0., 2.)).length() < 1e-9);
    }
}

#[test]
fn empty_mesh_item_moves_rotates_and_scales() {
    check_all(empty_mesh);
}

#[test]
fn mesh_stack_only_item_moves_rotates_and_scales() {
    check_all(mesh_stack_only);
}
