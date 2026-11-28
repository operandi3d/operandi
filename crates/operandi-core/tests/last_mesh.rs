//! VP-04 / VP-T8 (last_mesh.rs): with no mesh selected, the last selected
//! mesh stays active and stays the mesh being edited.
use glam::DVec3;
use operandi_core::*;

/// Adds a 2 m cube as a new mesh at `x`; returns its id.
fn add_cube(e: &mut Editor, x: f64) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        position: DVec3::new(x, 0., 0.),
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.active().unwrap().id
}
/// Cubes A and B, A selected last, then nothing selected (Item mode).
fn a_then_nothing() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let b = add_cube(&mut e, 5.);
    let a = add_cube(&mut e, 0.);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.dispatch(Command::ClearSelection).unwrap();
    assert_eq!(e.scene.selection.item, None, "nothing selected");
    (e, a, b)
}

#[test]
fn deselecting_everything_keeps_the_last_mesh_active() {
    // VP-T8.
    let (e, a, b) = a_then_nothing();
    assert!(e.scene.is_active_mesh(a));
    assert!(!e.scene.is_active_mesh(b));
    assert_eq!(e.scene.shown_active_meshes(), vec![a]);
    // It is not selected: the Item List shows nothing.
    assert!(e.scene.selected_items().is_empty());
    assert!(e.scene.active_meshes().is_empty());
}

#[test]
fn a_scene_where_no_mesh_was_selected_has_every_mesh_inactive() {
    // VPQ-10 (provisional): a scene just opened.
    let (e, a, b) = a_then_nothing();
    let reopened = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(reopened, e.scene, "not part of the document");
    assert!(!reopened.is_active_mesh(a) && !reopened.is_active_mesh(b));
    assert!(reopened.shown_active_meshes().is_empty());
}

#[test]
fn deleting_or_hiding_it_stops_it_being_active() {
    let (mut e, a, b) = a_then_nothing();
    e.dispatch(Command::Visibility(a)).unwrap();
    assert!(e.scene.shown_active_meshes().is_empty(), "hidden");
    assert_eq!(e.scene.selection.item, None, "hiding doesn't select it");
    e.dispatch(Command::Visibility(a)).unwrap();
    e.dispatch(Command::SelectItem(Some(a))).unwrap();
    e.dispatch(Command::DeleteItem).unwrap();
    assert!(!e.scene.is_active_mesh(a) && !e.scene.is_active_mesh(b));
    assert!(e.scene.shown_active_meshes().is_empty(), "deleted");
}

#[test]
fn selecting_another_mesh_moves_the_memory() {
    let (mut e, a, b) = a_then_nothing();
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert!(!e.scene.is_active_mesh(a));
    e.dispatch(Command::ClearSelection).unwrap();
    assert!(e.scene.is_active_mesh(b) && !e.scene.is_active_mesh(a));
}

#[test]
fn a_selected_camera_leaves_the_last_mesh_active() {
    let (mut e, a, _) = a_then_nothing();
    let mut camera = e.scene.items[0].clone();
    camera.id = e.scene.next_id;
    e.scene.next_id += 1;
    camera.mesh = Mesh::default().into();
    camera.camera = Some(Default::default());
    let cam = camera.id;
    e.scene.items.push(camera);
    e.dispatch(Command::SelectItem(Some(cam))).unwrap();
    assert!(e.scene.is_active_mesh(a), "no mesh selected");
    assert!(e.scene.is_active_mesh(cam), "selected");
}

#[test]
fn it_stays_the_mesh_being_edited() {
    // "As if it were still selected": Select All in Vertex mode works on A.
    let (mut e, a, b) = a_then_nothing();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    assert_eq!(
        e.scene.selection.item, None,
        "a mode switch selects nothing"
    );
    e.dispatch(Command::Selection(SelectionOperation::All))
        .unwrap();
    assert_eq!(e.scene.selection.item, Some(a));
    assert_eq!(e.scene.selection.vertices.len(), 8);
    assert!(e.scene.selection.others.is_empty() && !e.scene.is_active_mesh(b));
    // Undo goes back to the vertices unselected on A.
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.selection.vertices.is_empty());
}

#[test]
fn a_mesh_edit_with_nothing_selected_edits_it() {
    let (mut e, a, b) = a_then_nothing();
    let mesh_of =
        |e: &Editor, id: u64| (*e.scene.items.iter().find(|i| i.id == id).unwrap().mesh).clone();
    let (before_a, before_b) = (mesh_of(&e, a), mesh_of(&e, b));
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(e.scene.selection.item, Some(a), "edited, so selected again");
    assert_eq!(mesh_of(&e, b), before_b, "B untouched");
    assert_ne!(mesh_of(&e, a), before_a, "A flipped");
}

#[test]
fn a_component_click_on_it_selects_it_and_the_component() {
    let (mut e, a, _) = a_then_nothing();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.dispatch(Command::Select {
        item: a,
        hit: Component::Vertex(3),
        replace: true,
        remove: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.item, Some(a));
    assert_eq!(e.scene.selection.vertices, vec![3]);
}

#[test]
fn a_command_that_fails_leaves_nothing_selected() {
    let (mut e, _, _) = a_then_nothing();
    // Item mode: nothing to grow.
    let _ = e.dispatch(Command::Selection(SelectionOperation::More));
    assert_eq!(e.scene.selection.item, None);
}
