//! Command feedback still reports edits when a new step trims an old one.
use super::tests::box_app;
use super::*;

fn full_history() -> Operandi {
    let mut a = box_app();
    a.editor.history.max_undo_level = 2;
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![5];
    a.execute("poly.flip");
    a.execute("poly.flip");
    assert_eq!(a.editor.undo.len(), 2, "history is full");
    a
}

#[test]
fn polygon_remove_reports_success_at_the_history_limit() {
    let mut a = full_history();
    let before = a.editor.scene.clone();
    a.execute("poly.remove");
    assert_eq!(a.status, "Removed 1 polygon");
    assert_eq!(a.editor.undo.len(), 2);
    assert_ne!(a.editor.scene, before);
    a.execute("Undo");
    assert_eq!(a.editor.scene, before);
}

#[test]
fn polygon_collapse_reports_success_at_the_history_limit() {
    let mut a = full_history();
    let before = a.editor.scene.clone();
    a.execute("poly.collapse");
    assert_eq!(a.status, "Collapse: 1 polygon collapsed");
    assert_eq!(a.editor.undo.len(), 2);
    assert_ne!(a.editor.scene, before);
    a.execute("Undo");
    assert_eq!(a.editor.scene, before);
}

#[test]
fn vertex_split_reports_success_at_the_history_limit() {
    let mut a = full_history();
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    assert_eq!(a.editor.undo.len(), 2, "history is still full");
    let before = a.editor.scene.clone();
    a.execute("vert.split");
    assert_eq!(a.status, "Split done");
    assert_eq!(a.editor.undo.len(), 2);
    assert_ne!(a.editor.scene, before);
    a.execute("Undo");
    assert_eq!(a.editor.scene, before);
}
