//! Item mode pre-highlights the whole item under the pointer, not only the
//! polygon there (Jason, 2026-11-27).
use super::tests::box_app;
use super::*;
use operandi_core::item_common::{Choice, ItemCommonEdit};

fn view_rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

#[test]
fn item_mode_rolls_over_the_whole_item() {
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    let r = view_rect();
    let at = a.camera.project(DVec3::ZERO, r).unwrap().0;
    // Polygon mode: just the polygon under the pointer.
    a.run(Command::Mode(Mode::Polygon));
    assert!(a.pick_component(at, r).is_some());
    assert_eq!(a.whole_item_rollover(mesh), None);
    // Item mode: the whole item.
    a.run(Command::Mode(Mode::Item));
    assert_eq!(a.whole_item_rollover(mesh), Some(mesh));
    // Back in Polygon mode, the polygon again.
    a.run(Command::Mode(Mode::Polygon));
    assert_eq!(a.whole_item_rollover(mesh), None);
}

#[test]
fn select_no_items_dont_roll_over() {
    // CP-A01: a click passes over it, so the pointer doesn't light it.
    let mut a = box_app();
    let mesh = a.editor.scene.items[0].id;
    a.run(Command::Mode(Mode::Item));
    a.run(Command::EditItemCommon {
        items: vec![mesh],
        edit: ItemCommonEdit::Select(Choice::No),
    });
    assert_eq!(a.whole_item_rollover(mesh), None);
}
