//! The Duplicate tab's Transform buttons (duplicate-panel-2026-11-20,
//! DX-20..26). Acceptance criteria:
//!
//! 1. Transform Array moves the selected items onto the grid's cells, one
//!    per cell, top of the item list first, and makes no new items
//!    (DX-20/21).
//! 2. More items than spots leave the rest where they are; fewer use the
//!    first spots (DX-22).
//! 3. Transform Radial Array turns each item round the axis on top of its
//!    own turn (DX-23).
//! 4. Locked items stay put; nothing selected is refused (DX-11/25).
//! 5. A child moves with its selected parent and isn't placed itself
//!    (DX-26).
//! 6. Drop is one undo step that Undo takes back whole; Cancel leaves the
//!    scene as it was (DX-25).
//! 7. The plain Array button after Transform Array makes copies again
//!    (the copier comes only from the open row).
//! 8. Transform Pen works in Item mode: items go to the path's points.
use glam::DVec3;
use operandi_core::*;

/// A scene of `n` cube items at x = 0, 3, 6 ... (item-list order), all
/// selected in Item mode, the last one made first-selected so selection
/// order and item-list order differ.
fn items(n: usize) -> Editor {
    let mut e = Editor::default();
    for k in 0..n {
        e.dispatch(Command::NewMesh).unwrap();
        let item = e.scene.active_mut().unwrap();
        item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
        item.transform.translation = DVec3::new(3. * k as f64, 0., 0.);
    }
    e.dispatch(Command::Mode(Mode::Item)).unwrap();
    let ids: Vec<u64> = e.scene.items.iter().map(|i| i.id).collect();
    e.scene.selection.item = ids.last().copied();
    e.scene.selection.also = ids[..ids.len() - 1].to_vec();
    e
}

fn ids(e: &Editor) -> Vec<u64> {
    e.scene.items.iter().map(|i| i.id).collect()
}

fn at(e: &Editor, id: u64) -> DVec3 {
    e.scene
        .world_matrix(id)
        .unwrap()
        .transform_point3(DVec3::ZERO)
}

/// Start row `row`, change its values, place it and drop.
fn run(e: &mut Editor, row: &str, set: impl FnOnce(&mut DuplicateParameters)) {
    e.dispatch(Command::ActivatePreset(row.into())).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    set(&mut p.duplicate);
    p.duplicate.placed = true;
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
}

#[test]
fn transform_array_moves_items_onto_cells_in_list_order() {
    let mut e = items(3);
    let before = ids(&e);
    run(&mut e, "TransformArray", |d| {
        d.array.count = [3, 1, 1];
        d.array.offset = DVec3::new(2., 0., 0.);
    });
    assert_eq!(ids(&e), before, "no new items (DX-20)");
    // The first item (top of the list) keeps its cell; the next ones step
    // along X from it, whatever their old place.
    for (k, id) in before.iter().enumerate() {
        assert!(
            at(&e, *id).abs_diff_eq(DVec3::new(2. * k as f64, 0., 0.), 1e-9),
            "{k}: {}",
            at(&e, *id)
        );
    }
}

#[test]
fn more_items_than_spots_leave_the_rest_and_fewer_use_the_first() {
    // DX-22.
    let mut e = items(3);
    let before = ids(&e);
    run(&mut e, "TransformArray", |d| {
        d.array.count = [2, 1, 1];
        d.array.offset = DVec3::new(1., 0., 0.);
    });
    assert!(at(&e, before[1]).abs_diff_eq(DVec3::new(1., 0., 0.), 1e-9));
    assert!(
        at(&e, before[2]).abs_diff_eq(DVec3::new(6., 0., 0.), 1e-9),
        "the extra item stays"
    );
    let mut e = items(2);
    let before = ids(&e);
    run(&mut e, "TransformArray", |d| {
        d.array.count = [5, 1, 1];
        d.array.offset = DVec3::new(1., 0., 0.);
    });
    assert_eq!(e.scene.items.len(), 2);
    assert!(at(&e, before[1]).abs_diff_eq(DVec3::new(1., 0., 0.), 1e-9));
    let p = e
        .presets
        .get::<operandi_core::duplicate::DuplicatePreset>()
        .array;
    assert_eq!(p.count, [5, 1, 1], "the placer's count isn't changed");
}

#[test]
fn transform_radial_array_turns_items_round_the_axis() {
    // DX-23: four items round Y through the origin, starting at x = 2.
    let mut e = items(4);
    let before = ids(&e);
    let first = e.scene.item_index(before[0]).unwrap();
    e.scene.items[first].transform.translation = DVec3::new(2., 0., 0.);
    e.scene.items[first].transform.rotation = DVec3::new(0., 0., 0.3);
    run(&mut e, "TransformRadialArray", |d| {
        d.radial.circle.count = 4;
        d.radial.circle.center = DVec3::ZERO;
    });
    let want = [
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 0., -2.),
        DVec3::new(-2., 0., 0.),
        DVec3::new(0., 0., 2.),
    ];
    for (id, w) in before.iter().zip(want) {
        assert!(at(&e, *id).abs_diff_eq(w, 1e-9), "{}", at(&e, *id));
    }
    // The second item turned a quarter round Y on top of its own (none):
    // its local X now points along world −Z... (Y-up, counter-clockwise).
    let w = e.scene.world_matrix(before[1]).unwrap();
    let x = w.transform_vector3(DVec3::X);
    assert!(x.abs_diff_eq(DVec3::new(0., 0., -1.), 1e-9), "{x}");
    // The first item kept its own turn.
    let w0 = e.scene.world_matrix(before[0]).unwrap();
    let x0 = w0.transform_vector3(DVec3::X);
    assert!(x0.abs_diff_eq(DVec3::new(0.3f64.cos(), 0.3f64.sin(), 0.), 1e-9));
}

#[test]
fn locked_items_stay_and_nothing_selected_is_refused() {
    // DX-25: the locked item stays put and doesn't use up a spot.
    let mut e = items(3);
    let before = ids(&e);
    e.dispatch(Command::EditItemCommon {
        items: vec![before[1]],
        edit: item_common::ItemCommonEdit::Lock(item_common::Choice::Yes),
    })
    .unwrap();
    assert_eq!(duplicate_transform::locked_count(&e.scene), 1);
    run(&mut e, "TransformArray", |d| {
        d.array.count = [3, 1, 1];
        d.array.offset = DVec3::new(1., 0., 0.);
    });
    assert!(at(&e, before[1]).abs_diff_eq(DVec3::new(3., 0., 0.), 1e-9));
    assert!(at(&e, before[2]).abs_diff_eq(DVec3::new(1., 0., 0.), 1e-9));
    // DX-11: nothing selected.
    let mut e = items(2);
    e.scene.selection.item = None;
    e.scene.selection.also.clear();
    assert!(duplicate_transform::nothing_to_move(&e.scene));
    let start = e.scene.clone();
    assert!(
        e.dispatch(Command::ActivatePreset("TransformArray".into()))
            .is_err()
    );
    assert_eq!(e.scene, start, "a refusal changes nothing");
}

#[test]
fn a_child_moves_with_its_selected_parent() {
    // DX-26: item 1 is item 0's child; only item 0 and item 2 are placed.
    let mut e = items(3);
    let before = ids(&e);
    let child = e.scene.item_index(before[1]).unwrap();
    e.scene.items[child].parent = Some(before[0]);
    // Item 1 now sits at (3, 0, 0) under item 0 at the origin.
    run(&mut e, "TransformArray", |d| {
        d.array.count = [2, 1, 1];
        d.array.offset = DVec3::new(5., 0., 0.);
    });
    assert!(at(&e, before[0]).abs_diff_eq(DVec3::ZERO, 1e-9));
    assert!(at(&e, before[2]).abs_diff_eq(DVec3::new(5., 0., 0.), 1e-9));
    assert!(at(&e, before[1]).abs_diff_eq(DVec3::new(3., 0., 0.), 1e-9));
}

#[test]
fn drop_is_one_undo_step_and_cancel_changes_nothing() {
    let mut e = items(2);
    let start = e.scene.clone();
    let steps = e.history.undo.len();
    run(&mut e, "TransformArray", |d| {
        d.array.count = [2, 1, 1];
        d.array.offset = DVec3::new(4., 0., 0.);
    });
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_ne!(e.scene.items, start.items);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.items, start.items);
    // Cancel.
    e.dispatch(Command::ActivatePreset("TransformArray".into()))
        .unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.duplicate.placed = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.items, start.items);
}

#[test]
fn the_plain_array_button_makes_copies_after_transform_array() {
    let mut e = items(1);
    run(&mut e, "TransformArray", |d| {
        d.array.count = [2, 1, 1];
    });
    assert_eq!(e.scene.items.len(), 1);
    // Straight from the open row to the plain button.
    e.dispatch(Command::ActivatePreset("TransformArray".into()))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Array)).unwrap();
    let live = e.session.live.as_ref().unwrap();
    assert_eq!(live.params.duplicate.copier, None);
    let mut p = live.params.clone();
    p.duplicate.placed = true;
    p.duplicate.array.count = [2, 1, 1];
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene.items.len(), 2, "plain Array makes a copy");
}

#[test]
fn transform_pen_puts_items_on_the_path_points() {
    let mut e = items(3);
    let before = ids(&e);
    e.dispatch(Command::ActivatePreset("PenTransform".into()))
        .unwrap();
    assert_eq!(e.live_tool_label(), Some("Transform Pen"));
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    assert_eq!(
        p.duplicate.curve_clone.curve.kind,
        path_tools::PathKind::Pen
    );
    p.duplicate.curve_clone.pen_align = false;
    p.duplicate.curve_clone.curve.points = vec![
        DVec3::new(0., 1., 0.),
        DVec3::new(0., 2., 0.),
        DVec3::new(0., 3., 0.),
    ];
    e.session.tweak_held = true;
    e.dispatch(Command::Preview(p)).unwrap();
    e.end_tweak();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(ids(&e), before);
    for (k, id) in before.iter().enumerate() {
        let want = DVec3::new(0., 1. + k as f64, 0.);
        assert!(at(&e, *id).abs_diff_eq(want, 1e-9), "{k}: {}", at(&e, *id));
    }
}
