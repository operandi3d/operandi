//! UIC-307 (Jason 2026-11-27, GitHub #36): tools aren't modal. A command
//! that used to refuse "Drop or Cancel the active tool first" runs with a
//! tool open: the tool's work settles as its own step, the command is the
//! next step, and the tool starts again. Original test data.
use glam::DVec3;
use operandi_core::*;

/// One 2-unit box at the origin in Polygon mode with its top selected.
fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let item = e.scene.items.last_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    let mesh = item.mesh.clone();
    let id = item.id;
    e.dispatch(Command::SelectItem(Some(id))).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let top = (0..mesh.faces.len())
        .find(|&f| mesh.normal(f).dot(DVec3::Y) > 0.9)
        .unwrap() as u32;
    e.scene.selection.faces = vec![top];
    e.history.undo.clear();
    e
}
fn tool(e: &Editor) -> Option<Tool> {
    e.session.live.as_ref().map(|l| l.tool)
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn meshes(e: &Editor) -> usize {
    e.scene.items.iter().filter(|i| i.is_mesh()).count()
}

/// The fixture in Item mode, the box selected.
fn item_fixture() -> Editor {
    let mut e = fixture();
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.history.undo.clear();
    e
}

#[test]
fn new_mesh_with_an_untouched_tool_keeps_it_on_the_new_mesh() {
    let mut e = item_fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let before = e.history.undo.len();
    e.dispatch(Command::NewMesh).unwrap();
    assert_eq!(meshes(&e), 2);
    assert_eq!(tool(&e), Some(Tool::Move));
    // Only the new mesh is a step: the untouched tool added none.
    assert_eq!(e.history.undo.len(), before + 1);
    let new = e.scene.items.last().unwrap().id;
    assert_eq!(e.scene.selection.item, Some(new));
}

#[test]
fn new_mesh_after_a_drag_settles_the_drag_first() {
    let mut e = item_fixture();
    let first = e.scene.active().unwrap().id;
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let start = e.history.undo.len();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::X;
    e.dispatch(Command::Preview(p)).unwrap();
    let moved = e.scene.item(first).unwrap().transform.clone();
    e.dispatch(Command::NewMesh).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.session.live.as_ref().unwrap().params.delta, DVec3::ZERO);
    // The move is one step, the new mesh the next.
    assert_eq!(e.history.undo.len(), start + 2);
    assert_eq!(e.scene.item(first).unwrap().transform, moved);
    // Undo removes the new mesh; the box stays moved.
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(meshes(&e), 1);
    assert_eq!(e.scene.item(first).unwrap().transform, moved);
}

#[test]
fn a_component_tool_settles_for_new_mesh_and_rests_on_the_empty_mesh() {
    // Move on polygons can't start on an empty mesh: the new mesh is made
    // and the tool stays dropped, the edit kept (provisional, UIC-307).
    let mut e = fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::X;
    e.dispatch(Command::Preview(p)).unwrap();
    let moved = points(&e);
    let first = e.scene.active().unwrap().id;
    e.dispatch(Command::NewMesh).unwrap();
    assert_eq!(meshes(&e), 2);
    assert_eq!(tool(&e), None);
    assert_eq!(e.scene.item(first).unwrap().mesh.vertices, moved);
}

#[test]
fn a_refusal_that_isnt_about_the_tool_still_stands() {
    let mut e = fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let before = e.clone();
    let r = e.dispatch(Command::NewNamedMesh("  ".into()));
    assert!(r.is_err());
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.history.undo.len(), before.history.undo.len());
    assert_eq!(e.scene.items.len(), before.scene.items.len());
}

#[test]
fn a_mesh_stack_value_changes_with_move_live_in_item_mode() {
    // #36: Move live in Item mode, a Mesh Stack op's check box clicked in
    // Properties. The value changes and Move stays on.
    use crate::mesh_stack_tools::{add, cube, with_base};
    let (mut e, id) = with_base(cube());
    let op = add(
        &mut e,
        id,
        "pmodel.materialTag.item",
        serde_json::json!({ "materialName": "Red" }),
    );
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let values = serde_json::json!({ "materialName": "Blue" })
        .as_object()
        .unwrap()
        .clone();
    e.dispatch(Command::MeshOpSet { op, values }).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    let stack = e.scene.item(id).unwrap().mesh_stack.as_ref().unwrap();
    let json = format!("{:?}", stack.ops.last().unwrap().kind);
    assert!(json.contains("Blue"), "{json}");
}
