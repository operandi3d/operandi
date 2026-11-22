//! Item editor colours (note 10): IC-01..IC-04.
use operandi_core::item_common::ItemCommonEdit;
use operandi_core::item_list::*;
use operandi_core::shader_tree::ShaderTreeEdit;
use operandi_core::*;
use std::collections::BTreeSet;

/// Root "Rig" with children "Arm" and "Leg"; "Prop" at the root.
fn rig() -> (Editor, [u64; 4]) {
    let mut e = Editor::default();
    for _ in 0..4 {
        e.scene.new_mesh().unwrap();
    }
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    for (item, name) in e.scene.items.iter_mut().zip(["Rig", "Arm", "Leg", "Prop"]) {
        item.name = name.into();
    }
    e.scene.items[1].parent = Some(ids[0]);
    e.scene.items[2].parent = Some(ids[0]);
    e.next_item_id = e.scene.next_id;
    (e, [ids[0], ids[1], ids[2], ids[3]])
}

fn color(e: &Editor, id: u64) -> Option<EditorColor> {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap()
        .common
        .editor_color
}

fn set(e: &mut Editor, items: Vec<u64>, c: Option<EditorColor>) {
    e.dispatch(Command::EditItemCommon {
        items,
        edit: ItemCommonEdit::EditorColor(c),
    })
    .unwrap();
}

#[test]
fn ic01_two_items_one_undo_step() {
    let (mut e, [_, arm, leg, _]) = rig();
    let steps = e.undo.len();
    set(&mut e, vec![arm, leg], Some(EditorColor::Orange));
    assert_eq!(color(&e, arm), Some(EditorColor::Orange));
    assert_eq!(color(&e, leg), Some(EditorColor::Orange));
    assert_eq!(e.undo.len(), steps + 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(color(&e, arm), None);
    assert_eq!(color(&e, leg), None);
}

#[test]
fn ic02_filter_by_color_keeps_parents_greyed() {
    let (mut e, [rig_id, arm, _, prop]) = rig();
    set(&mut e, vec![arm, prop], Some(EditorColor::Orange));
    let filter = ListFilter {
        by_color: true,
        color: Some(EditorColor::Orange),
        ..Default::default()
    };
    let rows = list_rows(&e.scene, &filter, &BTreeSet::new());
    let shown: Vec<(u64, bool)> = rows.iter().map(|r| (r.id, r.greyed)).collect();
    assert_eq!(shown, vec![(rig_id, true), (arm, false), (prop, false)]);
    // Off again: the whole tree.
    let all = list_rows(&e.scene, &ListFilter::default(), &BTreeSet::new());
    assert_eq!(all.len(), 4);
}

#[test]
fn ic03_pattern_select_by_color() {
    let (mut e, [_, arm, leg, prop]) = rig();
    set(&mut e, vec![arm, prop], Some(EditorColor::Orange));
    set(&mut e, vec![leg], Some(EditorColor::Blue));
    let query = PatternQuery {
        by_color: true,
        color: Some(EditorColor::Orange),
        ..Default::default()
    };
    e.dispatch(Command::ItemPattern {
        query,
        action: PatternAction::Select,
    })
    .unwrap();
    let mut selected = e.scene.selected_items();
    selected.sort_unstable();
    assert_eq!(selected, vec![arm, prop]);
}

#[test]
fn ic04_save_and_reopen_keep_colors() {
    let (mut e, [rig_id, _, leg, _]) = rig();
    set(&mut e, vec![leg], Some(EditorColor::LightBlue));
    let layer = e.scene.shader_tree.nodes[0].id;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::SetEditorColor {
        ids: vec![layer],
        color: Some(EditorColor::Pink),
    }))
    .unwrap();
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"lightblue\""), "stored by name");
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back, e.scene);
    let reopened = |id| {
        back.items
            .iter()
            .find(|i| i.id == id)
            .unwrap()
            .common
            .editor_color
    };
    assert_eq!(reopened(leg), Some(EditorColor::LightBlue));
    assert_eq!(reopened(rig_id), None);
    assert_eq!(
        back.shader_tree.get(layer).unwrap().layer.editor_color,
        Some(EditorColor::Pink)
    );
    // An unknown name reads as none.
    let odd = text.replace("\"lightblue\"", "\"chartreuse\"");
    let back = persistence::decode(odd.as_bytes()).unwrap();
    assert_eq!(
        back.items
            .iter()
            .find(|i| i.id == leg)
            .unwrap()
            .common
            .editor_color,
        None
    );
}

#[test]
fn colors_never_change_rendering_data() {
    let (mut e, [_, arm, _, _]) = rig();
    let mesh = e.scene.items[1].mesh.clone();
    set(&mut e, vec![arm], Some(EditorColor::Red));
    assert_eq!(e.scene.items[1].mesh, mesh);
}
