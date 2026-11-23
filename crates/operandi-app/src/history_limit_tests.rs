//! At Max Undo Level (100 by default, and every selection click is a step)
//! each new step trims the oldest one, so `undo.len()` stops growing. Code
//! that asked "did that record a step?" by comparing lengths got the wrong
//! answer in a normal session (Astra P4-003, P4-004). It asks the history's
//! mark now (`Editor::history_mark`, `recorded_since`, `steps_since`).
use super::tests::box_app;
use super::*;

/// A box in Polygon mode with one polygon selected and a full history of
/// `limit` steps.
fn full_history(limit: usize) -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![5];
    a.editor.history.max_undo_level = limit;
    while a.editor.undo.len() < limit {
        a.execute("poly.flip");
    }
    a.execute("poly.flip");
    assert_eq!(a.editor.undo.len(), limit, "full");
    a
}

#[test]
fn a_batch_at_the_history_limit_is_still_one_step() {
    let mut a = full_history(4);
    let before = a.editor.scene.clone();
    a.execute("cmds.batch label:Both c1:poly.flip c2:\"poly.setPart Road\"");
    // Two steps trimmed two old ones, then became one.
    assert_eq!(a.editor.undo.len(), 3);
    assert_eq!(a.editor.undo.last().unwrap().label, "Both");
    a.execute("Undo");
    assert_eq!(
        a.editor.scene, before,
        "one Undo takes the whole batch back"
    );
}

#[test]
fn moving_an_item_to_another_scene_at_the_history_limit_moves_it() {
    use scenes::SceneRowAction;
    let mut a = full_history(2);
    let from = a.scenes.tabs[0].id;
    let item = a.editor.scene.items[0].id;
    let count = a.editor.scene.items.len();
    a.request_file(FileAction::New);
    let to = a.scenes.tabs[1].id;
    a.editor.history.max_undo_level = 2;
    // Fill this scene's history too.
    for _ in 0..3 {
        a.run(Command::NewMesh);
    }
    assert_eq!(a.editor.undo.len(), 2);
    assert!(a.switch_scene(0));
    a.scene_row_action(SceneRowAction::Transfer {
        from,
        item,
        to,
        parent: None,
        moving: true,
    });
    assert!(a.status.starts_with("Moved 1 item"), "{}", a.status);
    assert!(a.switch_scene(0));
    assert!(
        a.editor.scene.items.iter().all(|i| i.id != item),
        "the source no longer has it"
    );
    assert!(a.editor.scene.items.len() < count);
}
