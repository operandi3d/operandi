//! Multiple active meshes, stage 1 (multi_active.rs): every selected mesh
//! item is active and keeps its own component selection.
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
/// Two cubes, `a` primary and `b` also selected, Polygon mode, nothing
/// selected.
fn two() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let a = add_cube(&mut e, 0.);
    let b = add_cube(&mut e, 5.);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    e.dispatch(Command::ClearSelection).unwrap();
    (e, a, b)
}
fn pick(e: &mut Editor, item: u64, hit: Component, replace: bool, remove: bool) {
    e.dispatch(Command::Select {
        item,
        hit,
        replace,
        remove,
    })
    .unwrap();
}

#[test]
fn active_meshes_are_selected_meshes_primary_first() {
    let (mut e, a, b) = two();
    let c = add_cube(&mut e, 10.);
    // A camera item: selected, but not a mesh.
    let mut camera = e.scene.items[0].clone();
    camera.id = e.scene.next_id;
    e.scene.next_id += 1;
    camera.name = "Camera".into();
    camera.mesh = Mesh::default().into();
    camera.camera = Some(Default::default());
    let cam = camera.id;
    e.scene.items.push(camera);
    e.scene.selection.item = Some(c);
    e.scene.selection.also = vec![cam, a, 9_999, a, b];
    assert_eq!(e.scene.active_meshes(), vec![c, a, b]);
    // A hidden mesh stays selected (keeping its components) but isn't
    // active: it neither draws nor picks components.
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == a)
        .unwrap()
        .visible = false;
    assert_eq!(e.scene.selected_meshes(), vec![c, a, b]);
    assert_eq!(e.scene.active_meshes(), vec![c, b]);
    e.scene
        .items
        .iter_mut()
        .find(|i| i.id == a)
        .unwrap()
        .visible = true;
    // A non-mesh primary: the meshes after it are still active.
    e.scene.selection.item = Some(cam);
    e.scene.selection.also = vec![b];
    assert_eq!(e.scene.active_meshes(), vec![b]);
    e.scene.selection.item = None;
    e.scene.selection.also.clear();
    assert!(e.scene.active_meshes().is_empty());
}

#[test]
fn clicks_route_to_each_mesh_and_keep_the_primary() {
    let (mut e, a, b) = two();
    pick(&mut e, b, Component::Polygon(0), true, false);
    let s = &e.scene.selection;
    assert_eq!(
        s.item,
        Some(a),
        "a click on another active mesh keeps the primary"
    );
    assert_eq!(s.also, vec![b]);
    assert!(s.faces.is_empty());
    assert_eq!(s.faces_of(b), [0]);
    // Shift-click adds on the primary; b keeps its polygon.
    pick(&mut e, a, Component::Polygon(1), false, false);
    assert_eq!(e.scene.selection.faces_of(a), [1]);
    assert_eq!(e.scene.selection.faces_of(b), [0]);
    pick(&mut e, b, Component::Polygon(3), false, false);
    assert_eq!(e.scene.selection.faces_of(b), [0, 3]);
    // Ctrl-click removes from that mesh only.
    pick(&mut e, b, Component::Polygon(0), false, true);
    assert_eq!(e.scene.selection.faces_of(b), [3]);
    assert_eq!(e.scene.selection.faces_of(a), [1]);
    // A plain click clears every active mesh, then selects.
    pick(&mut e, a, Component::Polygon(2), true, false);
    assert_eq!(e.scene.selection.faces_of(a), [2]);
    assert!(e.scene.selection.faces_of(b).is_empty());
    assert!(e.scene.selection.others.is_empty());
    // Clear Selection clears them all.
    pick(&mut e, b, Component::Polygon(4), false, false);
    e.dispatch(Command::ClearSelection).unwrap();
    assert!(e.scene.selection.faces.is_empty());
    assert!(e.scene.selection.others.is_empty());
    assert_eq!(e.scene.selected_items(), vec![a, b]);
}

#[test]
fn per_mesh_selection_survives_undo_and_redo() {
    let (mut e, a, b) = two();
    pick(&mut e, a, Component::Polygon(1), true, false);
    pick(&mut e, b, Component::Polygon(2), false, false);
    let both = e.scene.selection.clone();
    pick(&mut e, b, Component::Polygon(5), true, false);
    assert!(e.scene.selection.faces.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection, both);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.selection.faces_of(b), [5]);
    assert!(e.scene.selection.faces_of(a).is_empty());
    assert_eq!(e.scene.selection.item, Some(a));
}

#[test]
fn deselecting_an_item_drops_its_components() {
    let (mut e, a, b) = two();
    pick(&mut e, b, Component::Polygon(2), true, false);
    e.dispatch(Command::ToggleItemSelection(b)).unwrap();
    assert!(e.scene.selection.others.is_empty());
    assert_eq!(e.scene.active_meshes(), vec![a]);
    // Back in, it starts with nothing selected.
    e.dispatch(Command::ToggleItemSelection(b)).unwrap();
    assert!(e.scene.selection.faces_of(b).is_empty());
    // Taking the primary out makes the next mesh primary with its own
    // components.
    pick(&mut e, a, Component::Polygon(1), true, false);
    pick(&mut e, b, Component::Polygon(4), false, false);
    e.dispatch(Command::ToggleItemSelection(a)).unwrap();
    assert_eq!(e.scene.selection.item, Some(b));
    assert_eq!(e.scene.selection.faces, [4]);
    assert!(e.scene.selection.others.is_empty());
    // Selecting one item outright drops the rest.
    e.dispatch(Command::SelectItems(vec![b, a])).unwrap();
    pick(&mut e, a, Component::Polygon(0), true, false);
    e.dispatch(Command::SelectItem(Some(b))).unwrap();
    assert!(e.scene.selection.others.is_empty());
    // Deleting a selected mesh drops its components too.
    e.dispatch(Command::SelectItems(vec![a, b])).unwrap();
    pick(&mut e, b, Component::Polygon(0), true, false);
    e.scene.items.retain(|i| i.id != b);
    e.scene.prune_component_selections(None);
    assert!(e.scene.selection.others.is_empty());
}

#[test]
fn selection_commands_work_on_every_active_mesh() {
    let (mut e, a, b) = two();
    e.dispatch(Command::Selection(SelectionOperation::All))
        .unwrap();
    assert_eq!(e.scene.selection.faces_of(a).len(), 6);
    assert_eq!(e.scene.selection.faces_of(b).len(), 6);
    e.dispatch(Command::ClearSelection).unwrap();
    pick(&mut e, b, Component::Polygon(0), true, false);
    e.dispatch(Command::Selection(SelectionOperation::Invert))
        .unwrap();
    assert_eq!(e.scene.selection.faces_of(a).len(), 6);
    assert_eq!(e.scene.selection.faces_of(b).len(), 5);
    // Grow fails on nothing but works where something is selected.
    e.dispatch(Command::ClearSelection).unwrap();
    pick(&mut e, b, Component::Polygon(0), true, false);
    e.dispatch(Command::Selection(SelectionOperation::Grow))
        .unwrap();
    assert!(e.scene.selection.faces_of(a).is_empty());
    assert_eq!(e.scene.selection.faces_of(b).len(), 5);
    // Convert to Vertices: each mesh converts its own.
    e.dispatch(Command::ClearSelection).unwrap();
    pick(&mut e, a, Component::Polygon(0), true, false);
    pick(&mut e, b, Component::Polygon(0), false, false);
    e.dispatch(Command::Selection(SelectionOperation::Convert(
        Mode::Vertex,
    )))
    .unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Vertex);
    assert_eq!(e.scene.selection.vertices_of(a).len(), 4);
    assert_eq!(e.scene.selection.vertices_of(b).len(), 4);
    assert_eq!(e.scene.selection.item, Some(a));
    // One undo step for the whole command.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.mode, Mode::Polygon);
    assert!(e.scene.selection.vertices_of(b).is_empty());
    e.scene.validate().unwrap();
}

#[test]
fn edge_loop_double_click_on_another_mesh() {
    let (mut e, a, b) = two();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    pick(&mut e, a, Component::Edge(0, 1), true, false);
    let edge = e
        .scene
        .items
        .iter()
        .find(|i| i.id == b)
        .unwrap()
        .mesh
        .edges()[0];
    e.dispatch(Command::SelectEdgeLoop {
        item: b,
        a: edge.0,
        b: edge.1,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.item, Some(a));
    assert!(
        e.scene.selection.edges.is_empty(),
        "plain double-click clears the others"
    );
    // A cube's corners have three edges, so the loop stops at once.
    assert_eq!(e.scene.selection.edges_of(b), [edge]);
    // Select Loop on both meshes: each walks its own edges.
    pick(&mut e, a, Component::Edge(edge.0, edge.1), false, false);
    e.dispatch(Command::Selection(SelectionOperation::Ring))
        .unwrap();
    assert_eq!(e.scene.selection.edges_of(a).len(), 4);
    assert_eq!(e.scene.selection.edges_of(b).len(), 4);
}

#[test]
fn modes_remember_each_mesh_selection() {
    let (mut e, a, b) = two();
    pick(&mut e, b, Component::Polygon(3), true, false);
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    pick(&mut e, b, Component::Vertex(1), true, false);
    pick(&mut e, a, Component::Vertex(2), false, false);
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    assert_eq!(e.scene.selection.faces_of(b), [3]);
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    assert_eq!(e.scene.selection.vertices_of(b), [1]);
    assert_eq!(e.scene.selection.vertices_of(a), [2]);
}

#[test]
fn save_and_reopen_keeps_every_mesh_selection() {
    let (mut e, a, b) = two();
    pick(&mut e, a, Component::Polygon(1), true, false);
    pick(&mut e, b, Component::Polygon(2), false, false);
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 29"), "{}", &text[..120]);
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.selection, e.scene.selection);
    assert_eq!(back.selection.faces_of(b), [2]);
    // An older version can't carry the field.
    let older = text.replacen("\"version\": 29", "\"version\": 28", 1);
    assert!(persistence::decode(older.as_bytes()).is_err());
    // Without other-mesh components the file stays at the older version.
    pick(&mut e, a, Component::Polygon(1), true, false);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!text.contains("\"version\": 29"));
    assert!(!text.contains("others"));
}

#[test]
fn old_files_without_the_field_load() {
    let (mut e, a, _) = two();
    pick(&mut e, a, Component::Polygon(1), true, false);
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    let mut doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(doc["scene"]["selection"].get("others").is_none());
    doc["version"] = 28.into();
    let back = persistence::decode(doc.to_string().as_bytes()).unwrap();
    assert!(back.selection.others.is_empty());
    assert_eq!(back.selection.faces, [1]);
}

#[test]
fn tools_leave_an_empty_primary_alone() {
    // Stage 3 (multi_tools.rs): with nothing selected on the primary but a
    // polygon on b, Flip flips b's polygon and leaves the primary as it is
    // (it used to refuse rather than flip all of the primary).
    let (mut e, a, b) = two();
    pick(&mut e, b, Component::Polygon(0), true, false);
    let mesh = |e: &Editor, id: u64| {
        e.scene
            .items
            .iter()
            .find(|i| i.id == id)
            .unwrap()
            .mesh
            .clone()
    };
    let before = mesh(&e, a);
    let b_before = mesh(&e, b);
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_eq!(*mesh(&e, a), *before);
    assert_ne!(*mesh(&e, b), *b_before);
    // With a polygon on each, both flip and keep their selections.
    pick(&mut e, a, Component::Polygon(1), false, false);
    e.dispatch(Command::FlipPolygons).unwrap();
    assert_ne!(*mesh(&e, a), *before);
    assert_eq!(*mesh(&e, b), *b_before, "b flipped twice");
    assert_eq!(e.scene.selection.faces_of(b), [0]);
    e.scene.validate().unwrap();
}

#[test]
fn shift_click_adds_items_and_ctrl_click_removes_them() {
    let (mut e, a, b) = two();
    let c = add_cube(&mut e, 10.);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    pick(&mut e, a, Component::Item, true, false);
    assert_eq!(e.scene.picked_items(), vec![a]);
    pick(&mut e, b, Component::Item, false, false);
    pick(&mut e, c, Component::Item, false, false);
    assert_eq!(e.scene.picked_items(), vec![a, b, c]);
    // The primary (last picked) comes first.
    assert_eq!(e.scene.active_meshes(), vec![c, a, b]);
    // Shift on an item already selected leaves it.
    pick(&mut e, b, Component::Item, false, false);
    assert_eq!(e.scene.picked_items(), vec![a, b, c]);
    pick(&mut e, b, Component::Item, false, true);
    assert_eq!(e.scene.picked_items(), vec![a, c]);
    // Ctrl toggles: a second Ctrl-click puts it back.
    pick(&mut e, b, Component::Item, false, true);
    assert_eq!(e.scene.picked_items(), vec![a, c, b]);
    pick(&mut e, b, Component::Item, false, true);
    assert_eq!(e.scene.picked_items(), vec![a, c]);
    // A plain click selects only the clicked item.
    pick(&mut e, c, Component::Item, true, false);
    assert_eq!(e.scene.picked_items(), vec![c]);
    // One undo step per click.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.picked_items(), vec![a, c]);
}

#[test]
fn polygon_connected_double_click_on_another_mesh() {
    let (mut e, a, b) = two();
    pick(&mut e, a, Component::Polygon(0), true, false);
    e.dispatch(Command::SelectConnectedPolygons {
        item: b,
        face: 2,
        add: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.item, Some(a));
    assert!(
        e.scene.selection.faces.is_empty(),
        "plain double-click clears the others"
    );
    assert_eq!(e.scene.selection.faces_of(b).len(), 6, "the whole cube");
    // Shift adds a's cube back without touching b.
    e.dispatch(Command::SelectConnectedPolygons {
        item: a,
        face: 1,
        add: true,
    })
    .unwrap();
    assert_eq!(e.scene.selection.faces_of(a).len(), 6);
    assert_eq!(e.scene.selection.faces_of(b).len(), 6);
}

#[test]
fn replace_visible_pick_preserves_hidden_mesh_components() {
    for (mode, prior, replacement) in [
        (Mode::Vertex, Component::Vertex(0), Component::Vertex(1)),
        (Mode::Edge, Component::Edge(0, 1), Component::Edge(1, 2)),
        (Mode::Polygon, Component::Polygon(0), Component::Polygon(1)),
    ] {
        let (mut e, a, b) = two();
        e.dispatch(Command::Mode(mode)).unwrap();
        pick(&mut e, a, prior, true, false);
        pick(&mut e, b, replacement, false, false);
        let remembered = e.scene.selection.others[&b].clone();
        e.dispatch(Command::Visibility(b)).unwrap();
        let before = e.scene.selection.clone();
        pick(&mut e, a, replacement, true, false);
        let after = e.scene.selection.clone();
        assert_eq!(after.others.get(&b), Some(&remembered), "{mode:?}");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene.selection, before);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene.selection, after);
        e.dispatch(Command::Visibility(b)).unwrap();
        assert_eq!(e.scene.active_meshes(), [a, b]);
        assert_eq!(e.scene.selection.others[&b], remembered);
    }
}
