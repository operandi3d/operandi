//! The Items-list eye hides children through their parent (GV-01..06,
//! 2026-11-10), clicked in the real list: with Auto Hide Child Items on
//! (the default) a parent's eye hides its subtree while the children keep
//! their own eyes, drawn faded; off, only the item. Ctrl+click is a plain
//! click. Alt+click solos an item with its subtree.
use super::tests::{box_app, draw, draw_held, pointer};
use super::*;
use crate::items_tree::{EYE, ItemKey};
use crate::tree_list::Cell;
use eframe::egui::PointerButton;

const SIZE: Vec2 = vec2(1440., 900.);

/// A parent mesh (the box) with a child mesh under it: (app, ctx, parent,
/// child).
fn family() -> (Operandi, Context, u64, u64) {
    let mut a = box_app();
    let parent = a.editor.scene.active().unwrap().id;
    a.run(Command::NewMesh);
    let child = a.editor.scene.active().unwrap().id;
    a.run(Command::SetParent {
        item: child,
        parent: Some(parent),
        keep_position: false,
    });
    assert_eq!(item(&a, child).parent, Some(parent), "{}", a.status);
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx, parent, child)
}

fn item(a: &Operandi, id: u64) -> &operandi_core::Item {
    a.editor.scene.item(id).unwrap()
}

fn shown(a: &Operandi, id: u64) -> bool {
    a.editor.scene.shows(item(a, id))
}

fn eye_at(a: &Operandi, id: u64) -> Pos2 {
    let here = a.scenes.active_tab().id;
    a.items_tree.cells[&(ItemKey::Item(here, id), EYE)].center()
}

/// Click item `id`'s eye with `modifiers` held.
fn click_eye(a: &mut Operandi, ctx: &Context, id: u64, modifiers: Modifiers) {
    let at = eye_at(a, id);
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers,
    };
    draw_held(a, ctx, SIZE, vec![Event::PointerMoved(at)], modifiers);
    draw_held(a, ctx, SIZE, vec![button(true), button(false)], modifiers);
    draw(a, ctx, SIZE, vec![]);
}

/// Drag down the eye column from `from`'s eye to `to`'s.
fn sweep(a: &mut Operandi, ctx: &Context, from: u64, to: u64) {
    let (p0, p1) = (eye_at(a, from), eye_at(a, to));
    draw(a, ctx, SIZE, vec![Event::PointerMoved(p0)]);
    draw(a, ctx, SIZE, vec![pointer(p0, true, Modifiers::NONE)]);
    for k in 1..=6 {
        let p = p0 + (p1 - p0) * (k as f32 / 6.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![pointer(p1, false, Modifiers::NONE)]);
    draw(a, ctx, SIZE, vec![]);
}

/// Item `id`'s eye cell in the list: (on, dimmed).
fn eye_cell(a: &Operandi, id: u64) -> (bool, bool) {
    let here = a.scenes.active_tab().id;
    let row = a
        .item_rows()
        .into_iter()
        .find(|r| r.key == ItemKey::Item(here, id))
        .expect("a row");
    match &row.cells[EYE] {
        Cell::Toggle { on, dimmed, .. } => (*on, *dimmed),
        _ => panic!("no eye"),
    }
}

#[test]
fn gv01_02_05_a_parent_eye_hides_and_shows_its_children() {
    let (mut a, ctx, parent, child) = family();
    assert!(a.mesh_items_preferences.auto_hide_children, "on by default");
    let steps = a.editor.undo.len();
    click_eye(&mut a, &ctx, parent, Modifiers::NONE);
    assert!(!shown(&a, parent) && !shown(&a, child), "{}", a.status);
    assert!(item(&a, child).visible, "the child keeps its own eye");
    assert_eq!(eye_cell(&a, child), (true, true), "drawn faded (GV-05)");
    assert_eq!(eye_cell(&a, parent), (false, false));
    assert_eq!(a.editor.undo.len(), steps + 1, "one undo step");
    click_eye(&mut a, &ctx, parent, Modifiers::NONE);
    assert!(shown(&a, parent) && shown(&a, child), "both come back");
    assert_eq!(eye_cell(&a, child), (true, false));
}

#[test]
fn gv02_a_child_hidden_on_its_own_stays_hidden() {
    let (mut a, ctx, parent, child) = family();
    click_eye(&mut a, &ctx, child, Modifiers::NONE);
    assert!(!shown(&a, child) && shown(&a, parent));
    click_eye(&mut a, &ctx, parent, Modifiers::NONE);
    click_eye(&mut a, &ctx, parent, Modifiers::NONE);
    assert!(shown(&a, parent));
    assert!(!shown(&a, child), "still hidden on its own");
}

#[test]
fn gv03_with_the_preference_off_only_the_item_hides() {
    let (mut a, ctx, parent, child) = family();
    a.mesh_items_preferences.auto_hide_children = false;
    click_eye(&mut a, &ctx, parent, Modifiers::NONE);
    assert!(!shown(&a, parent), "{}", a.status);
    assert!(shown(&a, child), "the child stays visible");
    assert_eq!(eye_cell(&a, child), (true, false));
}

#[test]
fn gv04_ctrl_and_cmd_click_are_a_plain_click() {
    let (mut a, ctx, parent, child) = family();
    click_eye(&mut a, &ctx, parent, Modifiers::CTRL);
    assert!(!shown(&a, parent) && !shown(&a, child), "{}", a.status);
    assert!(item(&a, child).visible);
    click_eye(&mut a, &ctx, parent, Modifiers::COMMAND);
    assert!(shown(&a, parent) && shown(&a, child));
}

#[test]
fn gv06_a_group_locator_works_like_any_parent() {
    let (mut a, ctx, _, child) = family();
    a.run(Command::SelectItems(vec![child]));
    a.run(Command::ParentToGroupLocator);
    let group = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_group_locator())
        .expect("a Group Locator")
        .id;
    draw(&mut a, &ctx, SIZE, vec![]);
    click_eye(&mut a, &ctx, group, Modifiers::NONE);
    assert!(!shown(&a, group) && !shown(&a, child), "{}", a.status);
    assert!(item(&a, child).visible);
}

#[test]
fn soloing_keeps_the_subtree_and_works_under_a_hidden_parent() {
    let (mut a, ctx, parent, child) = family();
    a.run(Command::NewMesh);
    let other = a.editor.scene.active().unwrap().id;
    draw(&mut a, &ctx, SIZE, vec![]);
    click_eye(&mut a, &ctx, parent, Modifiers::ALT);
    assert!(shown(&a, parent) && shown(&a, child), "{}", a.status);
    assert!(!shown(&a, other), "everything else hides");
    click_eye(&mut a, &ctx, parent, Modifiers::ALT);
    assert!(shown(&a, other), "Alt+click again brings it back");
    // Solo of the child while its parent's eye hides it: the child shows,
    // the parent and the rest don't.
    click_eye(&mut a, &ctx, parent, Modifiers::NONE);
    assert!(!shown(&a, child));
    click_eye(&mut a, &ctx, child, Modifiers::ALT);
    assert!(shown(&a, child), "{}", a.status);
    assert!(!shown(&a, parent) && !shown(&a, other));
}

#[test]
fn an_eye_sweep_follows_the_preference() {
    for auto_hide in [true, false] {
        let (mut a, ctx, parent, child) = family();
        a.mesh_items_preferences.auto_hide_children = auto_hide;
        let steps = a.editor.undo.len();
        sweep(&mut a, &ctx, parent, child);
        assert!(!shown(&a, parent) && !shown(&a, child), "{}", a.status);
        assert_eq!(item(&a, parent).hides_children, auto_hide);
        assert_eq!(a.editor.undo.len(), steps + 1);
    }
}
