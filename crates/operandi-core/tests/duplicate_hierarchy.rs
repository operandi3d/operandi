//! Duplicate Hierarchy (duplicate-panel-2026-11-20 DX-51). Acceptance
//! criteria:
//!
//! 1. A selected item is copied with its children at every level; the
//!    copies' links mirror the source's, and the copied root keeps the
//!    source's parent.
//! 2. A selected child of a selected item isn't copied twice.
//! 3. The copies of the branch roots are selected; names are "Name (2)".
//! 4. One undo step; Undo takes it back whole; nothing selected is refused
//!    without change.
//! 5. Plain Duplicate Item still copies the item alone (DXQ-4).
use glam::DVec3;
use operandi_core::*;

/// root > child > grandchild, plus a separate item; root selected.
fn tree() -> (Editor, [u64; 4]) {
    let mut e = Editor::default();
    for _ in 0..4 {
        e.dispatch(Command::NewMesh).unwrap();
    }
    for (item, name) in e
        .scene
        .items
        .iter_mut()
        .zip(["Root", "Child", "Grand", "Other"])
    {
        item.name = name.into();
    }
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    let [root, child, grand, other] = [ids[0], ids[1], ids[2], ids[3]];
    let at = |e: &mut Editor, id: u64| e.scene.item_index(id).unwrap();
    let c = at(&mut e, child);
    e.scene.items[c].parent = Some(root);
    let g = at(&mut e, grand);
    e.scene.items[g].parent = Some(child);
    e.scene.items[g].transform.translation = DVec3::new(1., 2., 3.);
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    e.scene.select_item(Some(root));
    (e, [root, child, grand, other])
}

#[test]
fn copies_the_whole_branch_with_its_links() {
    let (mut e, [root, child, grand, _]) = tree();
    let steps = e.undo.len();
    let before = e.scene.clone();
    e.dispatch(Command::DuplicateHierarchy).unwrap();
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.scene.items.len(), 7);
    let new: Vec<&Item> = e.scene.items[4..].iter().collect();
    let name = |id: u64| before.item(id).unwrap().name.clone();
    assert_eq!(new[0].name, format!("{} (2)", name(root)));
    assert_eq!(new[0].parent, None);
    assert_eq!(new[1].name, format!("{} (2)", name(child)));
    assert_eq!(new[1].parent, Some(new[0].id));
    assert_eq!(new[2].parent, Some(new[1].id));
    assert_eq!(
        new[2].transform,
        before.item(grand).unwrap().transform,
        "children keep their own transforms"
    );
    assert_eq!(e.scene.selected_items(), vec![new[0].id]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, before.items);
}

#[test]
fn a_selected_child_is_not_copied_twice_and_roots_keep_their_parent() {
    let (mut e, [root, child, _, other]) = tree();
    e.scene.select_item(Some(child));
    e.scene.selection.also = vec![root, other];
    e.dispatch(Command::DuplicateHierarchy).unwrap();
    // root's branch (3) and other (1).
    assert_eq!(e.scene.items.len(), 8);
    let sel = e.scene.selected_items();
    assert_eq!(sel.len(), 2);
    // Copy a child alone: its copy stays under the same parent.
    let (mut e, [root, child, _, _]) = tree();
    e.scene.select_item(Some(child));
    e.dispatch(Command::DuplicateHierarchy).unwrap();
    assert_eq!(e.scene.items.len(), 6);
    assert_eq!(e.scene.items[4].parent, Some(root));
    assert_eq!(e.scene.items[5].parent, Some(e.scene.items[4].id));
}

#[test]
fn nothing_selected_is_refused_and_plain_duplicate_copies_one_item() {
    let (mut e, [root, ..]) = tree();
    e.scene.select_item(None);
    let before = e.scene.clone();
    assert!(e.dispatch(Command::DuplicateHierarchy).is_err());
    assert_eq!(e.scene, before);
    e.scene.select_item(Some(root));
    e.dispatch(Command::DuplicateItem).unwrap();
    assert_eq!(
        e.scene.items.len(),
        5,
        "DXQ-4: Duplicate copies the item alone"
    );
}
