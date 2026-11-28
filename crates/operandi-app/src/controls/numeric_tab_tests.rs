//! Tab and Shift+Tab between Tool Properties numeric fields (UIC-51).
use super::tests::{box_app, draw, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

fn tab(shift: bool) -> Event {
    Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        },
    }
}

/// A Bevel session with its fields drawn, and its Tool Properties fields in
/// draw order (top to bottom; the last ones are scrolled out of sight).
fn bevel(ctx: &Context) -> (Operandi, Vec<String>) {
    theme::install(ctx);
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    a.run(Command::Preview(
        a.editor.session.live.as_ref().unwrap().params.clone(),
    ));
    for _ in 0..2 {
        let _ = draw(&mut a, ctx, SIZE, vec![]);
    }
    // The Tool Properties column; the Properties panel on the right has its
    // own fields.
    let mut keys: Vec<(String, Rect)> = a
        .field_rects
        .iter()
        .filter(|(_, r)| r.min.x < SIZE.x / 2.)
        .map(|(k, r)| (k.clone(), *r))
        .collect();
    keys.sort_by(|x, y| x.1.min.y.total_cmp(&y.1.min.y));
    let keys = keys.into_iter().map(|(k, _)| k).collect::<Vec<_>>();
    assert!(
        keys.iter().any(|k| k == "inset") && keys.iter().any(|k| k == "shift"),
        "{keys:?}"
    );
    (a, keys)
}

fn click(a: &mut Operandi, ctx: &Context, key: &str) {
    let pos = a.field_rects[key].center();
    let _ = draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(pos, true, Modifiers::NONE),
            pointer(pos, false, Modifiers::NONE),
        ],
    );
}

/// The Tool Properties field that has keyboard focus.
fn focused_field(a: &Operandi, ctx: &Context) -> Option<String> {
    let id = ctx.memory(|m| m.focused())?;
    let rect = ctx.read_response(id)?.rect;
    a.field_rects
        .iter()
        .find(|(_, r)| **r == rect)
        .map(|(k, _)| k.clone())
}

#[test]
fn tab_accepts_and_moves_to_next_field_shift_tab_back() {
    let ctx = Context::default();
    let (mut a, keys) = bevel(&ctx);
    let first = keys[0].clone();
    let second = keys[1].clone();
    click(&mut a, &ctx, &first);
    assert_eq!(focused_field(&a, &ctx).as_deref(), Some(first.as_str()));
    let history = a.editor.history.undo.len();
    a.fields.insert(first.clone(), "0.1 m".into());
    let _ = draw(&mut a, &ctx, SIZE, vec![tab(false)]);
    // The typed value is accepted, the same as clicking away.
    assert!(a.field_errors.is_empty(), "{:?}", a.field_errors);
    assert_eq!(a.confirmed_fields[&first], "0.1 m");
    assert_eq!(focused_field(&a, &ctx).as_deref(), Some(second.as_str()));
    // The tool stays live: Tab is not a tool shortcut or a drop here.
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Bevel);
    assert_eq!(a.editor.history.undo.len(), history);
    // The arriving field has its whole value selected, so typing replaces it.
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
    let id = ctx.memory(|m| m.focused()).unwrap();
    let range = TextEdit::load_state(&ctx, id)
        .and_then(|s| s.cursor.char_range())
        .unwrap();
    assert_eq!(
        range.primary.index.max(range.secondary.index),
        a.fields[&second].chars().count()
    );
    assert_eq!(range.primary.index.min(range.secondary.index), 0);

    let _ = draw(&mut a, &ctx, SIZE, vec![tab(true)]);
    assert_eq!(focused_field(&a, &ctx).as_deref(), Some(first.as_str()));
}

#[test]
fn tab_wraps_from_last_field_to_first() {
    let ctx = Context::default();
    let (mut a, keys) = bevel(&ctx);
    click(&mut a, &ctx, &keys[0]);
    // Shift+Tab from the first field goes round to the last enabled one,
    // below the panel's visible part (Weld Range is disabled until Weld is
    // on, so Tab skips it).
    let _ = draw(&mut a, &ctx, SIZE, vec![tab(true)]);
    let last = focused_field(&a, &ctx).unwrap();
    assert_eq!(last, "bevel_angle", "{keys:?}");
    // Tool Properties scrolls on its own below the divider now; the last
    // field is in that area, wherever its scroll has it.
    assert!(a.field_rects[&last].min.y > a.toolbox_rects["Tool Properties"].bottom());
    let _ = draw(&mut a, &ctx, SIZE, vec![tab(false)]);
    assert_eq!(focused_field(&a, &ctx).as_deref(), Some(keys[0].as_str()));
}

#[test]
fn refused_value_keeps_focus_with_its_error() {
    let ctx = Context::default();
    let (mut a, keys) = bevel(&ctx);
    let first = keys[0].clone();
    click(&mut a, &ctx, &first);
    a.fields.insert(first.clone(), "abc".into());
    for _ in 0..2 {
        let _ = draw(&mut a, &ctx, SIZE, vec![tab(false)]);
    }
    assert!(a.field_errors.contains_key(&first));
    assert_eq!(focused_field(&a, &ctx).as_deref(), Some(first.as_str()));
    assert_eq!(a.fields[&first], "abc");
}

#[test]
fn tab_without_a_numeric_field_is_left_alone() {
    let ctx = Context::default();
    let (mut a, _) = bevel(&ctx);
    ctx.memory_mut(|m| m.surrender_focus(m.focused().unwrap_or(Id::NULL)));
    let mut raw = RawInput {
        events: vec![tab(false)],
        ..Default::default()
    };
    assert!(!numeric_tab::take_tab(&ctx, &mut raw));
    assert_eq!(raw.events.len(), 1);
    let _ = draw(&mut a, &ctx, SIZE, vec![]);
}

#[test]
fn tool_properties_fields_stay_clear_of_the_scroll_bar() {
    // GitHub #31: Bevel's Tool Properties scrolls at this size; the bar
    // widens over the content near the pointer, so the fields (and their
    // slide arrows on the right) stop short of it.
    let ctx = Context::default();
    let (a, keys) = bevel(&ctx);
    let panel = a.toolbox_rects["Tool Properties"];
    let scroll = ctx.style().spacing.scroll;
    let room = scroll.bar_width + scroll.bar_outer_margin;
    for key in &keys {
        let field = a.field_rects[key];
        assert!(
            field.right() <= panel.right() - room + 0.5,
            "{key} ends at {} under a bar from {}",
            field.right(),
            panel.right() - room
        );
    }
}
