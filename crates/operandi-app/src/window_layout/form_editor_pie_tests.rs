//! The Form Editor's pie ring (form_editor_pie.rs, UIC-308..313): pies
//! listed together, the ring drawn with every slot, click to select a
//! slot, drag one onto another to swap them, all through operandi.form.*
//! lines and the user's copy (Revert brings the shipped pie back).
use super::*;
use crate::layouts_tests::{data_root, frames, launch};
use crate::pie_menu;

fn app() -> (Operandi, Context) {
    let ctx = Context::default();
    let mut a = launch(&data_root(), &ctx);
    a.load_palettes();
    a.load_form_groups();
    frames(&mut a, &ctx, 2);
    (a, ctx)
}

fn labels(a: &Operandi, id: &str) -> Vec<Option<String>> {
    let pie = a.pie_named(id).unwrap();
    (0..pie_menu::SLOTS)
        .map(|i| pie.slot(i).map(|e| e.label.clone()))
        .collect()
}

#[test]
fn uic308_pies_sit_together_in_the_tree() {
    let (a, _) = app();
    let tree = a.form_tree();
    let pies = tree
        .iter()
        .find(|(g, _)| g == "Pies")
        .map(|(_, ids)| ids.clone())
        .expect("a Pies group");
    for id in pie_menu::ALL {
        assert!(pies.contains(&id.to_owned()), "{id}");
    }
}

#[test]
fn uic309_the_ring_shows_every_slot_and_a_click_selects_one() {
    let (mut a, ctx) = app();
    a.execute("operandi.form.select form:pie.view");
    frames(&mut a, &ctx, 3);
    for i in 0..pie_menu::SLOTS {
        assert!(
            a.form_editor.rects.contains_key(&format!("pie.view@{i}")),
            "slot {i} drawn"
        );
    }
    // The top slot is above the centre, the bottom one below (the ring
    // is laid out as the pie opens).
    let top = a.form_editor.rects["pie.view@0"].center();
    let bottom = a.form_editor.rects["pie.view@4"].center();
    let right = a.form_editor.rects["pie.view@2"].center();
    assert!(top.y < bottom.y && right.x > top.x);
    let size = crate::layouts_tests::SIZE;
    crate::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(bottom)]);
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![crate::tests::pointer(bottom, true, Modifiers::NONE)],
    );
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![crate::tests::pointer(bottom, false, Modifiers::NONE)],
    );
    assert_eq!(
        a.form_editor.selected,
        Some(("pie.view".to_owned(), Some(4)))
    );
}

#[test]
fn uic309_dragging_a_slot_onto_another_swaps_them() {
    let (mut a, ctx) = app();
    a.execute("operandi.form.select form:pie.view");
    frames(&mut a, &ctx, 3);
    let from = a.form_editor.rects["pie.view@0"].center();
    let to = a.form_editor.rects["pie.view@4"].center();
    let size = crate::layouts_tests::SIZE;
    crate::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(from)]);
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![crate::tests::pointer(from, true, Modifiers::NONE)],
    );
    for t in 1..=6 {
        let p = from + (to - from) * (t as f32 / 6.);
        crate::tests::draw(&mut a, &ctx, size, vec![Event::PointerMoved(p)]);
    }
    crate::tests::draw(
        &mut a,
        &ctx,
        size,
        vec![crate::tests::pointer(to, false, Modifiers::NONE)],
    );
    frames(&mut a, &ctx, 1);
    let now = labels(&a, pie_menu::VIEW);
    assert_eq!(now[0].as_deref(), Some("Bottom"), "{}", a.status);
    assert_eq!(now[4].as_deref(), Some("Top"));
    assert_eq!(now[2].as_deref(), Some("Right"), "the others stay");
    // The swap is the user's copy; Revert puts the shipped pie back.
    assert!(a.forms.is_changed(pie_menu::VIEW));
    a.execute("operandi.form.revert pie.view");
    assert_eq!(labels(&a, pie_menu::VIEW)[0].as_deref(), Some("Top"));
}

#[test]
fn uic160_new_pie_and_slot_lines_come_from_the_command_history() {
    let (mut a, ctx) = app();
    a.form_editor.new_form = "my.pie".into();
    a.execute("operandi.form.new id:my.pie style:pie");
    a.execute("Open Render Window");
    a.execute("operandi.form.select form:my.pie index:3");
    frames(&mut a, &ctx, 2);
    assert!(a.recent_lines().contains(&"Open Render Window".to_owned()));
    // What History ▾ runs when a line is picked: the slot keeps its place.
    a.execute("operandi.form.slot index:3 line:{Open Render Window} form:my.pie");
    assert_eq!(
        labels(&a, "my.pie")[3].as_deref(),
        Some("Open Render Window")
    );
    assert!(
        (0..pie_menu::SLOTS)
            .filter(|&i| i != 3)
            .all(|i| labels(&a, "my.pie")[i].is_none())
    );
    frames(&mut a, &ctx, 2);
    assert!(a.form_editor.rects.contains_key("my.pie@3"));
}
