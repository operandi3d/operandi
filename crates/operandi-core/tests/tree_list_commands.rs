//! Core commands behind the shared tree list (UI bible UIC-31/38/40/138):
//! selecting several items in order, showing or hiding several in one
//! step, and the Items search's pattern and Match Case options.
use operandi_core::item_common::{Choice, ItemCommonEdit};
use operandi_core::item_list::{ListFilter, list_rows, wildcard_match};
use operandi_core::*;
use std::collections::BTreeSet;

fn three_meshes() -> (Editor, Vec<u64>) {
    let mut e = Editor::default();
    for _ in 0..3 {
        e.dispatch(Command::NewMesh).unwrap();
    }
    let ids = e.scene.items.iter().map(|i| i.id).collect();
    (e, ids)
}

#[test]
fn select_items_keeps_the_given_order_with_the_first_primary() {
    let (mut e, ids) = three_meshes();
    let steps = e.undo.len();
    e.dispatch(Command::SelectItems(vec![ids[2], ids[0], ids[2]]))
        .unwrap();
    assert_eq!(e.scene.selection.item, Some(ids[2]));
    assert_eq!(e.scene.selection.also, [ids[0]], "duplicates dropped");
    assert_eq!(e.undo.len(), steps, "selecting isn't an undo step");
    assert!(e.dispatch(Command::SelectItems(vec![999])).is_err());
    e.dispatch(Command::SelectItems(vec![])).unwrap();
    assert_eq!(e.scene.selection.item, None);
}

#[test]
fn set_items_visible_is_one_step_and_skips_locked_items() {
    let (mut e, ids) = three_meshes();
    e.dispatch(Command::EditItemCommon {
        items: vec![ids[1]],
        edit: ItemCommonEdit::Lock(Choice::Yes),
    })
    .unwrap();
    let steps = e.undo.len();
    e.dispatch(Command::SetItemsVisible {
        items: ids.clone(),
        visible: false,
        hides_children: false,
    })
    .unwrap();
    let visible: Vec<bool> = e.scene.items.iter().map(|i| i.visible).collect();
    assert_eq!(visible, [false, true, false]);
    assert_eq!(e.undo.len(), steps + 1);
    assert!(
        e.dispatch(Command::SetItemsVisible {
            items: vec![ids[1]],
            visible: false,
            hides_children: false,
        })
        .is_err(),
        "only locked items: refused"
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.items.iter().all(|i| i.visible));
}

#[test]
fn search_pattern_and_match_case() {
    let (e, ids) = three_meshes();
    let names: Vec<String> = e.scene.items.iter().map(|i| i.name.clone()).collect();
    let rows = |f: &ListFilter| -> Vec<u64> {
        list_rows(&e.scene, f, &BTreeSet::new())
            .into_iter()
            .map(|r| r.id)
            .collect()
    };
    // Plain text: the name contains it, any case.
    let mut f = ListFilter {
        text: Some(names[0].to_uppercase()),
        ..Default::default()
    };
    assert!(rows(&f).contains(&ids[0]));
    // Pattern: the whole name, `*` and `?`.
    f.text_pattern = true;
    f.text = Some(names[0].to_lowercase());
    assert_eq!(rows(&f), [ids[0]]);
    f.text = Some("*".into());
    assert_eq!(rows(&f), ids);
    // Match Case: case matters.
    f.text_case = true;
    f.text = Some(names[0].to_uppercase());
    if names[0] != names[0].to_uppercase() {
        assert!(rows(&f).is_empty());
    }
    assert!(wildcard_match("m?sh*", "mesh 2"));
    assert!(!wildcard_match("m?sh", "mesh 2"));
    assert!(wildcard_match("é?", "éa"), "characters, not bytes");
}
