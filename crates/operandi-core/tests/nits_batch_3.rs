//! Nits batch 3 (Jason, 2026-11-16): small fixes with their regression
//! tests.
use operandi_core::*;

/// Three meshes, the first and third selected (Item mode).
fn three_items() -> (Editor, [u64; 3]) {
    let mut e = Editor::default();
    let mut ids = [0; 3];
    for id in &mut ids {
        e.dispatch(Command::NewMesh).unwrap();
        *id = e.scene.selection.item.unwrap();
    }
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.scene.select_item(Some(ids[0]));
    e.scene.toggle_item(ids[2]);
    assert_eq!(e.scene.picked_items(), vec![ids[0], ids[2]]);
    (e, ids)
}

/// N10: the Delete key's `delete` takes every selected item, in one undo.
#[test]
fn n10_delete_removes_every_selected_item() {
    let (mut e, ids) = three_items();
    let before = e.scene.clone();
    command_exec::run(&mut e, "delete", &Default::default()).unwrap();
    let left: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    assert!(left.contains(&ids[1]));
    assert!(!left.contains(&ids[0]) && !left.contains(&ids[2]));
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
}
