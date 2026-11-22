//! MN-06: number field gaps. Detents while dragging (UIC-54), the link
//! button on Item Transform's X/Y/Z groups (UIC-56), Enter with modifiers
//! (UIC-57) and click-to-step on every field (MN-06.4).
use super::tests::{box_app, draw, draw_held, pointer};
use super::*;
use crate::link_mode::LinkMode;

const SIZE: Vec2 = vec2(1440., 900.);

fn settle(a: &mut Operandi, ctx: &Context) {
    for _ in 0..2 {
        draw(a, ctx, SIZE, vec![]);
    }
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2, modifiers: Modifiers) {
    draw_held(a, ctx, SIZE, vec![Event::PointerMoved(at)], modifiers);
    draw_held(a, ctx, SIZE, vec![pointer(at, true, modifiers)], modifiers);
    draw_held(a, ctx, SIZE, vec![pointer(at, false, modifiers)], modifiers);
    draw(a, ctx, SIZE, vec![]);
}

/// Tool Properties field `key`'s mini-slider arrows (UIC-53).
fn arrows(a: &Operandi, key: &str) -> Rect {
    super::tests::arrows_rect(a, key)
}

fn moving() -> Operandi {
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Move);
    a
}

#[test]
fn clicking_a_distance_fields_arrows_steps_ten_drag_steps() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = moving();
    settle(&mut a, &ctx);
    let r = arrows(&a, "delta1");
    let right = pos2(r.right() - 2., r.center().y);
    let left = pos2(r.left() + 2., r.center().y);
    click(&mut a, &ctx, right, Modifiers::NONE);
    assert_eq!(a.fields["delta1"], workspace::number_text(0.1, " m"));
    click(&mut a, &ctx, right, Modifiers::SHIFT);
    assert_eq!(a.fields["delta1"], workspace::number_text(1.1, " m"));
    click(&mut a, &ctx, left, Modifiers::NONE);
    assert_eq!(a.fields["delta1"], workspace::number_text(1.0, " m"));
}

#[test]
fn click_amounts_follow_the_field_kind() {
    use scrub::{Kind, click_amount};
    let none = Modifiers::NONE;
    assert_eq!(click_amount(Kind::Distance, none, true), 0.1);
    assert_eq!(click_amount(Kind::Angle, none, true), 5.);
    assert_eq!(click_amount(Kind::Percent, none, false), -5.);
    assert_eq!(click_amount(Kind::Scalar, none, true), 0.1);
    assert_eq!(click_amount(Kind::Count, none, true), 1.);
    assert_eq!(click_amount(Kind::Count, Modifiers::SHIFT, false), -10.);
}

#[test]
fn dragging_past_one_metre_pauses_there() {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = moving();
    settle(&mut a, &ctx);
    let r = arrows(&a, "delta1");
    let at = r.center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    // Start the drag with no net change, free of the 0 detent.
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(at + vec2(20., 0.))],
    );
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    a.scrub.as_mut().unwrap().held = None;
    let mut x = 0.;
    let mut seen = Vec::new();
    // 1 cm per point: 1 m is a strong stop at 100 points.
    for _ in 0..27 {
        x += 4.;
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(at + vec2(x, 0.))],
        );
        seen.push(a.fields["delta1"].clone());
    }
    let one = workspace::number_text(1., " m");
    let held = seen.iter().filter(|t| **t == one).count();
    assert!(held >= 2, "1 m held over several frames: {seen:?}");
    // 108 points of travel, 8 of them held: 1 m.
    assert_eq!(seen.last().unwrap(), &one, "{seen:?}");
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(at + vec2(x, 0.), false, Modifiers::NONE)],
    );
}

/// Item Transform with one box selected.
fn item_app() -> (Operandi, Context, u64) {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Item));
    let id = a.editor.scene.selection.item.unwrap();
    let ctx = Context::default();
    theme::install(&ctx);
    settle(&mut a, &ctx);
    (a, ctx, id)
}

fn position(a: &Operandi, id: u64) -> DVec3 {
    a.editor.scene.item(id).unwrap().transform.translation
}

/// Types `text` into Position `axis` and confirms with Enter and
/// `modifiers` held.
fn type_position(a: &mut Operandi, ctx: &Context, axis: usize, text: &str, modifiers: Modifiers) {
    let key_name = format!("translation{axis}");
    let field = a.field_rects[&format!("item-{key_name}")].center();
    click(a, ctx, field, Modifiers::NONE);
    a.item_fields.insert(key_name, text.into());
    let enter = Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    draw_held(a, ctx, SIZE, vec![enter], modifiers);
    settle(a, ctx);
}

#[test]
fn the_link_button_cycles_and_copy_sets_every_axis() {
    let (mut a, ctx, id) = item_app();
    let link = a.field_rects["item-translation-link"].center();
    let mode = |a: &Operandi| a.link_modes.get("translation").copied().unwrap_or_default();
    assert_eq!(mode(&a), LinkMode::Independent);
    let mut seen = vec![];
    for _ in 0..4 {
        click(&mut a, &ctx, link, Modifiers::NONE);
        seen.push(mode(&a));
    }
    assert_eq!(
        seen,
        [
            LinkMode::Copy,
            LinkMode::Proportional,
            LinkMode::Relative,
            LinkMode::Independent
        ]
    );
    type_position(&mut a, &ctx, 1, "2", Modifiers::NONE);
    assert_eq!(position(&a, id), DVec3::new(0., 2., 0.), "{}", a.status);
    click(&mut a, &ctx, link, Modifiers::NONE);
    assert_eq!(mode(&a), LinkMode::Copy);
    type_position(&mut a, &ctx, 0, "3", Modifiers::NONE);
    assert_eq!(position(&a, id), DVec3::splat(3.), "{}", a.status);
    // One undo step for the whole group.
    a.run(Command::Undo);
    assert_eq!(position(&a, id), DVec3::new(0., 2., 0.));
}

#[test]
fn enter_with_modifiers_picks_the_mode_for_one_entry() {
    let (mut a, ctx, id) = item_app();
    type_position(&mut a, &ctx, 1, "2", Modifiers::NONE);
    type_position(&mut a, &ctx, 2, "4", Modifiers::NONE);
    assert_eq!(position(&a, id), DVec3::new(0., 2., 4.));
    // Alt+Enter: Relative, +1 on every axis.
    type_position(&mut a, &ctx, 1, "3", Modifiers::ALT);
    assert_eq!(position(&a, id), DVec3::new(1., 3., 5.), "{}", a.status);
    // Ctrl+Enter (Cmd on a Mac): Proportional, ×2.
    type_position(&mut a, &ctx, 1, "6", Modifiers::COMMAND);
    assert_eq!(position(&a, id), DVec3::new(2., 6., 10.), "{}", a.status);
    // Ctrl+Alt+Enter: Copy.
    type_position(&mut a, &ctx, 0, "7", Modifiers::COMMAND | Modifiers::ALT);
    assert_eq!(position(&a, id), DVec3::splat(7.), "{}", a.status);
    // The button kept its own mode.
    assert_eq!(
        a.link_modes.get("translation").copied().unwrap_or_default(),
        LinkMode::Independent
    );
    // Plain Enter is the button's mode again.
    type_position(&mut a, &ctx, 0, "1", Modifiers::NONE);
    assert_eq!(position(&a, id), DVec3::new(1., 7., 7.));
}

#[test]
fn a_linked_drag_moves_every_axis() {
    let (mut a, ctx, id) = item_app();
    a.link_modes
        .insert("translation".into(), LinkMode::Relative);
    settle(&mut a, &ctx);
    let at = a.field_rects["item-translation0-arrows"].center();
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(&mut a, &ctx, SIZE, vec![pointer(at, true, Modifiers::NONE)]);
    for dx in [10., 20., 30., 40.] {
        draw(
            &mut a,
            &ctx,
            SIZE,
            vec![Event::PointerMoved(at + vec2(dx, 0.))],
        );
    }
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![pointer(at + vec2(40., 0.), false, Modifiers::NONE)],
    );
    let p = position(&a, id);
    assert!(p.x > 0.1, "{p}");
    assert!((p.y - p.x).abs() < 1e-9 && (p.z - p.x).abs() < 1e-9, "{p}");
}
