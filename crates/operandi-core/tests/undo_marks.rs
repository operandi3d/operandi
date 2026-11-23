//! Astra P4-003: the history's mark tells whether something recorded and
//! which steps are new, also once Max Undo Level trims a step per new one.
use operandi_core::*;

#[test]
fn marks_count_steps_through_a_full_history() {
    let mut e = Editor::default();
    e.history.max_undo_level = 3;
    for _ in 0..5 {
        e.dispatch(Command::NewMesh).unwrap();
    }
    assert_eq!(e.undo.len(), 3);
    let mark = e.history_mark();
    assert!(!e.recorded_since(mark));
    assert_eq!(e.steps_since(mark), 0);
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::NewMesh).unwrap();
    assert_eq!(e.undo.len(), 3, "the length didn't change");
    assert!(e.recorded_since(mark));
    assert_eq!(e.steps_since(mark), 2);
    command_exec::fold_steps(&mut e, mark, "Two Meshes");
    assert_eq!(e.undo.last().unwrap().label, "Two Meshes");
    assert_eq!(e.undo.len(), 2);
}
