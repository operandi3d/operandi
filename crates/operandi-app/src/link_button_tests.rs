//! MN-06b: one link button per Tool Properties X/Y/Z group (UIC-56), the
//! Enter keys that pick a link for one entry (UIC-57), and the button's
//! chain icons and plain-words tooltip.
use super::tests::{arrows_rect, box_app, draw, draw_held, pointer};
use super::*;
use crate::link_mode::LinkMode;

const SIZE: Vec2 = vec2(1440., 900.);

fn settle(a: &mut Operandi, ctx: &Context) {
    for _ in 0..2 {
        draw(a, ctx, SIZE, vec![]);
    }
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    let none = Modifiers::NONE;
    draw_held(a, ctx, SIZE, vec![Event::PointerMoved(at)], none);
    draw_held(a, ctx, SIZE, vec![pointer(at, true, none)], none);
    draw_held(a, ctx, SIZE, vec![pointer(at, false, none)], none);
    draw(a, ctx, SIZE, vec![]);
}

/// Focus Tool Properties field `key`, make its text `text` and press
/// Enter with `modifiers` held.
fn type_field(a: &mut Operandi, ctx: &Context, key: &str, text: &str, modifiers: Modifiers) {
    let at = a.field_rects[key].center();
    click(a, ctx, at);
    a.fields.insert(key.into(), text.into());
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

fn app(tool: Tool) -> (Operandi, Context) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![5];
    a.activate(tool);
    settle(&mut a, &ctx);
    (a, ctx)
}

fn shown(a: &Operandi, key: &str) -> String {
    a.confirmed_fields[key].clone()
}

#[test]
fn a_tool_group_has_one_link_button_on_its_first_row() {
    let (mut a, ctx) = app(Tool::Move);
    let link = a.field_rects["delta-link"];
    let [x, y, z] = ["delta0", "delta1", "delta2"].map(|k| a.field_rects[k]);
    // On the first row, between the field and the mini-slider arrows, as
    // tall as the arrows.
    assert!(link.center().y > x.top() && link.center().y < x.bottom());
    assert!(link.left() >= x.right());
    let arrows = arrows_rect(&a, "delta0");
    assert!(link.right() <= arrows.left() + 0.5, "{link:?} {arrows:?}");
    assert_eq!(link.height(), arrows.height());
    // The other rows draw none but keep the same field width.
    assert!((y.width() - x.width()).abs() < 0.5 && (z.width() - x.width()).abs() < 0.5);
    assert!(!a.field_rects.contains_key("delta1-link"));
    let mode = |a: &Operandi| a.link_modes.get("delta").copied().unwrap_or_default();
    assert_eq!(mode(&a), LinkMode::Independent);
    click(&mut a, &ctx, link.center());
    assert_eq!(mode(&a), LinkMode::Copy);
}

#[test]
fn the_buttons_mode_spreads_a_typed_value_over_the_group() {
    let (mut a, ctx) = app(Tool::Move);
    a.link_modes.insert("delta".into(), LinkMode::Copy);
    type_field(&mut a, &ctx, "delta0", "2", Modifiers::NONE);
    for k in ["delta0", "delta1", "delta2"] {
        assert_eq!(
            shown(&a, k),
            workspace::number_text(2., " m"),
            "{k}: {}",
            a.status
        );
    }
    // Independent again: only the typed row.
    a.link_modes.insert("delta".into(), LinkMode::Independent);
    type_field(&mut a, &ctx, "delta1", "5", Modifiers::NONE);
    assert_eq!(shown(&a, "delta0"), workspace::number_text(2., " m"));
    assert_eq!(shown(&a, "delta1"), workspace::number_text(5., " m"));
    assert_eq!(shown(&a, "delta2"), workspace::number_text(2., " m"));
}

#[test]
fn enter_keys_pick_the_link_for_one_entry() {
    let (mut a, ctx) = app(Tool::Move);
    type_field(&mut a, &ctx, "delta0", "1", Modifiers::NONE);
    type_field(&mut a, &ctx, "delta1", "2", Modifiers::NONE);
    type_field(&mut a, &ctx, "delta2", "4", Modifiers::NONE);
    let at = |a: &Operandi| ["delta0", "delta1", "delta2"].map(|k| shown(a, k));
    let m = |v: [f64; 3]| v.map(|x| workspace::number_text(x, " m"));
    // Alt+Enter: add together.
    type_field(&mut a, &ctx, "delta1", "3", Modifiers::ALT);
    assert_eq!(at(&a), m([2., 3., 5.]), "{}", a.status);
    // Ctrl+Enter (Cmd on a Mac): scale together.
    type_field(&mut a, &ctx, "delta1", "6", Modifiers::COMMAND);
    assert_eq!(at(&a), m([4., 6., 10.]), "{}", a.status);
    // Ctrl+Alt+Enter: same value.
    type_field(
        &mut a,
        &ctx,
        "delta2",
        "7",
        Modifiers::COMMAND | Modifiers::ALT,
    );
    assert_eq!(at(&a), m([7., 7., 7.]), "{}", a.status);
    // The button kept its own mode.
    assert_eq!(
        a.link_modes.get("delta").copied().unwrap_or_default(),
        LinkMode::Independent
    );
}

#[test]
fn whole_number_groups_round_what_they_take() {
    let (mut a, ctx) = app(Tool::Box);
    type_field(&mut a, &ctx, "segments0", "2", Modifiers::NONE);
    type_field(&mut a, &ctx, "segments1", "3", Modifiers::NONE);
    type_field(&mut a, &ctx, "segments2", "5", Modifiers::NONE);
    // ×1.5 by Ctrl+Enter: 3, 4.5 → 5 (rounded), 7.5 → 8.
    type_field(&mut a, &ctx, "segments0", "3", Modifiers::COMMAND);
    assert_eq!(
        ["segments0", "segments1", "segments2"].map(|k| shown(&a, k)),
        ["3", "5", "8"].map(|t| workspace::number_text(t.parse().unwrap(), "")),
        "{}",
        a.status
    );
}

#[test]
fn the_tooltip_names_the_state_in_plain_words() {
    let names: Vec<_> = [
        LinkMode::Independent,
        LinkMode::Copy,
        LinkMode::Proportional,
        LinkMode::Relative,
    ]
    .map(|m| m.tip())
    .into_iter()
    .collect();
    assert!(names[0].starts_with("Link: off"), "{}", names[0]);
    assert!(names[0].contains("Click: link same value"), "{}", names[0]);
    assert!(names[1].starts_with("Link: same value"));
    assert!(names[2].starts_with("Link: scale together"));
    assert!(names[3].starts_with("Link: add together"));
    assert!(names[3].contains("Click: link off"));
    for tip in &names {
        assert!(
            tip.contains("Alt+Enter") && tip.contains("Enter: scale together"),
            "{tip}"
        );
    }
}

#[test]
fn each_state_has_its_own_rendered_chain_icon() {
    let icons = [
        LinkMode::Independent,
        LinkMode::Copy,
        LinkMode::Proportional,
        LinkMode::Relative,
    ]
    .map(|m| m.icon());
    for (i, icon) in icons.iter().enumerate() {
        assert!(crate::rendered_icons::slug(*icon).is_some(), "{icon:?}");
        assert!(!icons[i + 1..].contains(icon));
    }
}
