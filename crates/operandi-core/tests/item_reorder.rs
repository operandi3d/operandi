//! Reordering and reparenting items by dragging rows in the Items list
//! (UI bible UIC-143): `Scene::move_items` / `Command::MoveItems`.
use glam::DVec3;
use operandi_core::item_common::{Choice, ItemCommonEdit};
use operandi_core::scene::Transform;
use operandi_core::*;

/// Four top-level meshes A, B, C, D, each at its own x.
fn four() -> (Editor, [u64; 4]) {
    let mut e = Editor::default();
    for x in 0..4 {
        e.dispatch(Command::NewMesh).unwrap();
        let id = e.scene.items.last().unwrap().id;
        e.dispatch(Command::ItemTransform {
            item: id,
            transform: Transform {
                translation: DVec3::new(x as f64, 0., 0.),
                ..Default::default()
            },
        })
        .unwrap();
    }
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    (e, [ids[0], ids[1], ids[2], ids[3]])
}

fn order(e: &Editor) -> Vec<u64> {
    e.scene.tree_order()
}

fn parent(e: &Editor, id: u64) -> Option<u64> {
    e.scene
        .items
        .iter()
        .find(|i| i.id == id)
        .and_then(|i| i.parent)
}

fn world(e: &Editor, id: u64) -> DVec3 {
    e.scene
        .world_matrix(id)
        .unwrap()
        .transform_point3(DVec3::ZERO)
}

fn mv(e: &mut Editor, items: &[u64], parent: Option<u64>, before: Option<u64>) -> Result<()> {
    e.dispatch(Command::MoveItems {
        items: items.to_vec(),
        parent,
        before,
    })
}

#[test]
fn a_between_drop_reorders_siblings_without_parenting_and_undoes_in_one_step() {
    let (mut e, [a, b, c, d]) = four();
    let steps = e.undo.len();
    // D before B, at the top level.
    mv(&mut e, &[d], None, Some(b)).unwrap();
    assert_eq!(order(&e), [a, d, b, c]);
    assert!(
        e.scene.items.iter().all(|i| i.parent.is_none()),
        "nobody parented"
    );
    assert_eq!(e.undo.len(), steps + 1);
    assert_eq!(e.undo.last().unwrap().label, "Reorder");
    // A to the end.
    mv(&mut e, &[a], None, None).unwrap();
    assert_eq!(order(&e), [d, b, c, a]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(order(&e), [a, d, b, c]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(order(&e), [a, b, c, d]);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(order(&e), [a, d, b, c]);
}

#[test]
fn dropping_where_it_already_is_changes_nothing_and_adds_no_undo_step() {
    let (mut e, [a, b, c, _]) = four();
    let steps = e.undo.len();
    mv(&mut e, &[b], None, Some(c)).unwrap();
    mv(&mut e, &[b], None, Some(b)).unwrap();
    assert_eq!(order(&e)[..3], [a, b, c]);
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn a_between_drop_under_another_parent_reparents_keeping_the_world_position() {
    let (mut e, [a, b, c, d]) = four();
    mv(&mut e, &[b, c], Some(a), None).unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Parent");
    assert_eq!(order(&e), [a, b, c, d]);
    let before = world(&e, d);
    // D in between B and C, under A: reparented, same world place.
    mv(&mut e, &[d], Some(a), Some(c)).unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Parent");
    assert_eq!(parent(&e, d), Some(a));
    assert_eq!(order(&e), [a, b, d, c]);
    assert!((world(&e, d) - before).length() < 1e-9);
    // C out to the top level, first.
    mv(&mut e, &[c], None, Some(a)).unwrap();
    assert_eq!(e.undo.last().unwrap().label, "Unparent");
    assert_eq!(order(&e), [c, a, b, d]);
    assert!((world(&e, c) - DVec3::new(2., 0., 0.)).length() < 1e-9);
}

#[test]
fn a_multi_selection_keeps_its_list_order_whatever_the_selection_order() {
    let (mut e, [a, b, c, d]) = four();
    let steps = e.undo.len();
    // Picked D first, then A: they land as A, D (list order).
    mv(&mut e, &[d, a], None, Some(c)).unwrap();
    assert_eq!(order(&e), [b, a, d, c]);
    // Into B as children, in the same order.
    mv(&mut e, &[d, a], Some(b), None).unwrap();
    assert_eq!(order(&e), [b, a, d, c]);
    assert_eq!(parent(&e, a), Some(b));
    assert_eq!(parent(&e, d), Some(b));
    assert_eq!(e.undo.len(), steps + 2, "one step per drop");
}

#[test]
fn dropping_before_one_of_the_dragged_rows_lands_before_the_next_unmoved_sibling() {
    let (mut e, [a, b, c, d]) = four();
    // A and C dragged, dropped just above C: together, before D.
    mv(&mut e, &[a, c], None, Some(c)).unwrap();
    assert_eq!(order(&e), [b, a, c, d]);
}

#[test]
fn a_child_dragged_with_its_parent_stays_inside_it() {
    let (mut e, [a, b, c, d]) = four();
    mv(&mut e, &[b], Some(a), None).unwrap();
    mv(&mut e, &[a, b], None, None).unwrap();
    assert_eq!(parent(&e, b), Some(a), "B goes along inside A");
    assert_eq!(order(&e), [c, d, a, b]);
}

#[test]
fn loops_are_refused_and_change_nothing() {
    let (mut e, [a, b, c, _]) = four();
    mv(&mut e, &[b], Some(a), None).unwrap();
    mv(&mut e, &[c], Some(b), None).unwrap();
    let scene = e.scene.clone();
    let steps = e.undo.len();
    assert!(mv(&mut e, &[a], Some(a), None).is_err(), "onto itself");
    assert!(
        mv(&mut e, &[a], Some(c), None).is_err(),
        "into a grandchild"
    );
    assert!(
        mv(&mut e, &[a, c], Some(b), None).is_err(),
        "a multi-drag into one dragged row's branch"
    );
    assert_eq!(e.scene, scene);
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn a_before_row_under_a_different_parent_or_missing_items_are_refused() {
    let (mut e, [a, b, c, _]) = four();
    mv(&mut e, &[b], Some(a), None).unwrap();
    let scene = e.scene.clone();
    assert!(mv(&mut e, &[c], None, Some(b)).is_err());
    assert!(mv(&mut e, &[999], None, None).is_err());
    assert!(mv(&mut e, &[], None, None).is_err());
    assert_eq!(e.scene, scene);
}

#[test]
fn a_locked_item_can_be_reordered_but_not_reparented() {
    let (mut e, [a, b, c, _]) = four();
    e.dispatch(Command::EditItemCommon {
        items: vec![c],
        edit: ItemCommonEdit::Lock(Choice::Yes),
    })
    .unwrap();
    mv(&mut e, &[c], None, Some(a)).unwrap();
    assert_eq!(order(&e)[..3], [c, a, b]);
    let scene = e.scene.clone();
    // B and C into A: C is locked, so neither moves.
    assert!(mv(&mut e, &[b, c], Some(a), None).is_err());
    assert_eq!(e.scene, scene);
}

#[test]
fn the_order_survives_save_and_reopen() {
    let (mut e, [a, b, c, d]) = four();
    mv(&mut e, &[c], Some(a), None).unwrap();
    mv(&mut e, &[d], None, Some(a)).unwrap();
    mv(&mut e, &[b], Some(a), Some(c)).unwrap();
    assert_eq!(order(&e), [d, a, b, c]);
    let path =
        std::env::temp_dir().join(format!("operandi-reorder-{}.opscene", std::process::id()));
    persistence::save(&path, &e.scene).unwrap();
    let back = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(back.tree_order(), [d, a, b, c]);
}
