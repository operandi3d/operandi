//! Q drops any tool and Esc drops the selection (drop_keys.rs; Jason,
//! 2026-11-21, from the Model the Mug tutorial). The behaviour is shared,
//! so these run it across many tools and every selection mode.
use super::tests::{draw, key, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 1500.);

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    draw(a, ctx, SIZE, events);
}

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx
}

fn release(k: Key) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// A key press as the keyboard sends it: the key, the letter it types (if
/// any), then the release in the next frame (or the next press reads as a
/// repeat).
fn tap(a: &mut Operandi, ctx: &Context, k: Key) {
    let mut events = vec![key(k)];
    if k.name().len() == 1 {
        events.push(Event::Text(k.name().to_lowercase()));
    }
    frame(a, ctx, events);
    frame(a, ctx, vec![release(k)]);
    frame(a, ctx, vec![]);
}

fn move_to(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
}

fn over_view(a: &Operandi) -> Pos2 {
    a.viewport_rect.center() + vec2(137., 93.)
}

fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    move_to(a, ctx, at);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

/// Click a Tool Properties field and replace its text, without confirming.
fn type_text(a: &mut Operandi, ctx: &Context, field: &str, new: &str) {
    frame(a, ctx, vec![]);
    let at = *a
        .field_rects
        .get(field)
        .unwrap_or_else(|| panic!("{field} drawn: {:?}", a.field_rects.keys()));
    click_at(a, ctx, pos2(at.left() + 12., at.center().y));
    let old = a.fields.get(field).cloned().unwrap_or_default();
    let mut events = vec![key(Key::End)];
    events.extend((0..old.chars().count()).map(|_| key(Key::Backspace)));
    events.push(Event::Text(new.into()));
    frame(a, ctx, events);
}

/// The box scene with polygon 0 selected, in `mode`.
fn selected_box(mode: Mode) -> Operandi {
    let mut a = super::tests::box_app();
    a.run(Command::Mode(mode));
    let face = a.editor.scene.active().unwrap().mesh.faces[0]
        .vertices
        .clone();
    let s = &mut a.editor.scene.selection;
    s.faces = vec![0];
    s.vertices = face.clone();
    s.edges = vec![(face[0].min(face[1]), face[0].max(face[1]))];
    a
}

fn tool_on(a: &Operandi) -> bool {
    a.editor.session.has_active_task() || a.utility.is_some() || a.draw.is_some()
}

/// The tools a modelling session goes through, by their buttons' names.
const TOOLS: &[&str] = &[
    "Cube",
    "Sphere",
    "Cylinder",
    "Cone",
    "Torus",
    "Capsule",
    "Bevel",
    "Inset",
    "Extrude",
    "Thicken",
    "Smooth Shift",
    "Loop Slice",
    "Edge Bevel",
    "Move",
    "Rotate",
    "Bend",
];

/// Opens `name` on the box with a selection fit for it, or `None` when it
/// doesn't open there.
fn open_tool(name: &str, mode: Mode) -> Option<(Operandi, Context)> {
    let ctx = context();
    let mut a = selected_box(mode);
    frame(&mut a, &ctx, vec![]);
    a.execute(name);
    frame(&mut a, &ctx, vec![]);
    tool_on(&a).then_some((a, ctx))
}

/// SC-078 / UIC-24: Q drops every tool, with the pointer over a view or a
/// panel.
#[test]
fn q_drops_every_tool_wherever_the_pointer_is() {
    let mut opened = 0;
    for &name in TOOLS {
        for mode in [Mode::Polygon, Mode::Edge] {
            for over_panel in [false, true] {
                let Some((mut a, ctx)) = open_tool(name, mode) else {
                    continue;
                };
                opened += 1;
                let at = if over_panel {
                    a.field_rects
                        .values()
                        .next()
                        .map(|r| r.center() + vec2(0., 400.))
                        .unwrap_or(pos2(5., 5.))
                } else {
                    over_view(&a)
                };
                move_to(&mut a, &ctx, at);
                tap(&mut a, &ctx, Key::Q);
                assert!(
                    !tool_on(&a),
                    "Q drops {name} in {mode:?} (panel {over_panel}): {}",
                    a.status
                );
            }
        }
    }
    assert!(opened > 20, "only {opened} tool sessions opened");
}

/// The Mug tutorial bug: a number typed into Tool Properties and not yet
/// confirmed held the keyboard, so Q typed into the field and the tool
/// stayed. Q now confirms the typing and drops the tool, with the pointer
/// on the panel or the view.
#[test]
fn q_in_a_number_field_confirms_the_typing_and_drops_the_tool() {
    for over_view_after in [false, true] {
        let ctx = context();
        let mut a = selected_box(Mode::Polygon);
        a.activate(Tool::Move);
        frame(&mut a, &ctx, vec![]);
        let field = a
            .field_rects
            .keys()
            .find(|k| k.ends_with('0') && !k.ends_with("-link"))
            .cloned()
            .expect("a Move X field");
        let before = a.editor.scene.active().unwrap().mesh.points()[0];
        type_text(&mut a, &ctx, &field, "0.5");
        assert!(
            ctx.memory(|m| m.focused()).is_some(),
            "the field has the keys"
        );
        if over_view_after {
            // Typed text keeps the field even with the pointer on a view.
            let at = over_view(&a);
            move_to(&mut a, &ctx, at);
        }
        tap(&mut a, &ctx, Key::Q);
        assert!(!tool_on(&a), "Q dropped Move: {}", a.status);
        assert!(
            !a.fields.values().any(|t| t.contains('q')),
            "{:?}",
            a.fields
        );
        let after = a.editor.scene.active().unwrap().mesh.points()[0];
        assert!(
            (after.x - before.x - 0.5).abs() < 1e-6,
            "the typed 0.5 was applied: {before:?} -> {after:?}"
        );
        // One undo takes the move back: the drop kept it as one step.
        a.run(Command::Undo);
        let undone = a.editor.scene.active().unwrap().mesh.points()[0];
        assert!((undone - before).length() < 1e-9, "{undone:?}");
    }
}

/// A value the field refuses keeps the field, its error and the tool.
#[test]
fn q_after_a_refused_value_keeps_the_field_and_the_tool() {
    let ctx = context();
    let mut a = selected_box(Mode::Polygon);
    a.activate(Tool::Move);
    frame(&mut a, &ctx, vec![]);
    let field = a
        .field_rects
        .keys()
        .find(|k| k.ends_with('0') && !k.ends_with("-link"))
        .cloned()
        .unwrap();
    type_text(&mut a, &ctx, &field, "1/0");
    tap(&mut a, &ctx, Key::Q);
    assert!(tool_on(&a), "the tool stays");
    assert!(!a.field_errors.is_empty(), "the field says why");
    assert_eq!(a.fields[&field], "1/0", "no q typed into it");
}

/// UIC-24 / UIC-66: Esc over a view drops every tool like Q, keeping its
/// edit (Create's scratch points are the one tool Esc cancels).
#[test]
fn escape_over_a_view_drops_every_tool() {
    let mut opened = 0;
    for &name in TOOLS {
        for mode in [Mode::Polygon, Mode::Edge] {
            let Some((mut a, ctx)) = open_tool(name, mode) else {
                continue;
            };
            opened += 1;
            let selection = a.editor.scene.selection.clone();
            let at = over_view(&a);
            move_to(&mut a, &ctx, at);
            tap(&mut a, &ctx, Key::Escape);
            assert!(!tool_on(&a), "Esc drops {name} in {mode:?}: {}", a.status);
            // One step per press: the selection is still there.
            assert_eq!(
                a.editor.scene.selection.faces, selection.faces,
                "{name}: Esc dropped the tool, not the selection"
            );
        }
    }
    assert!(opened > 10, "only {opened} tool sessions opened");
}

/// SC-080: with no tool on, Esc over a view drops the selection of the
/// selection mode, as one undo step.
#[test]
fn escape_with_no_tool_drops_the_selection_in_every_mode() {
    for mode in [Mode::Vertex, Mode::Edge, Mode::Polygon, Mode::Item] {
        let ctx = context();
        let mut a = selected_box(mode);
        frame(&mut a, &ctx, vec![]);
        let before = a.editor.scene.selection.clone();
        let has = |a: &Operandi| match mode {
            Mode::Vertex => a.editor.scene.selection.component_count() > 0,
            Mode::Edge => a.editor.scene.selection.component_count() > 0,
            Mode::Polygon => !a.editor.scene.selection.faces.is_empty(),
            Mode::Item => a.editor.scene.selection.item.is_some(),
        };
        assert!(has(&a), "{mode:?} starts with a selection");
        let at = over_view(&a);
        move_to(&mut a, &ctx, at);
        tap(&mut a, &ctx, Key::Escape);
        assert!(!has(&a), "Esc dropped the {mode:?} selection: {}", a.status);
        a.run(Command::Undo);
        assert_eq!(
            a.editor.scene.selection, before,
            "{mode:?}: undo brings it back"
        );
    }
}

/// Esc acts on the view under the pointer (UIC-70): over a panel the
/// selection stays.
#[test]
fn escape_over_a_panel_keeps_the_selection() {
    let ctx = context();
    let mut a = selected_box(Mode::Polygon);
    frame(&mut a, &ctx, vec![]);
    let outside = a.viewport_rect.left_top() - vec2(20., -40.);
    assert!(!a.viewport_rect.contains(outside));
    move_to(&mut a, &ctx, outside);
    tap(&mut a, &ctx, Key::Escape);
    assert!(!a.editor.scene.selection.faces.is_empty());
}

/// UIC-51: Esc in a field cancels its edit first; the tool and the
/// selection stay for the next press.
#[test]
fn escape_in_a_field_cancels_the_edit_only() {
    let ctx = context();
    let mut a = selected_box(Mode::Polygon);
    a.activate(Tool::Move);
    frame(&mut a, &ctx, vec![]);
    let field = a
        .field_rects
        .keys()
        .find(|k| k.ends_with('0') && !k.ends_with("-link"))
        .cloned()
        .unwrap();
    let confirmed = a.fields[&field].clone();
    type_text(&mut a, &ctx, &field, "0.5");
    let at = over_view(&a);
    move_to(&mut a, &ctx, at);
    tap(&mut a, &ctx, Key::Escape);
    assert!(tool_on(&a), "the tool stays: {}", a.status);
    assert_eq!(a.fields[&field], confirmed, "the typing is cancelled");
    assert!(tool_on(&a), "the tool stays");
    assert!(!a.editor.scene.selection.faces.is_empty());
    // The next Esc drops the tool, the one after that the selection.
    tap(&mut a, &ctx, Key::Escape);
    assert!(!tool_on(&a), "{}", a.status);
    assert!(!a.editor.scene.selection.faces.is_empty());
    tap(&mut a, &ctx, Key::Escape);
    assert!(a.editor.scene.selection.faces.is_empty(), "{}", a.status);
}

/// Drop Selection is an input-map action: it can be moved off Esc, and
/// the command runs from a line too.
#[test]
fn drop_selection_is_rebindable_and_a_command() {
    let ctx = context();
    let mut a = selected_box(Mode::Polygon);
    a.bindings.insert("Drop Selection".into(), "Shift+K".into());
    frame(&mut a, &ctx, vec![]);
    let at = over_view(&a);
    move_to(&mut a, &ctx, at);
    tap(&mut a, &ctx, Key::Escape);
    assert!(
        !a.editor.scene.selection.faces.is_empty(),
        "Esc no longer drops it"
    );
    a.execute("select.drop");
    assert!(a.editor.scene.selection.faces.is_empty(), "{}", a.status);
}

/// The hint names what works: Q and Esc, not Space (Space opens Mini
/// Properties, UIC-66).
#[test]
fn the_exit_hint_names_q_and_escape() {
    let ctx = context();
    let mut a = selected_box(Mode::Polygon);
    a.activate(Tool::Bevel);
    frame(&mut a, &ctx, vec![]);
    let hint = a.tool_exit_hint().unwrap();
    assert!(hint.starts_with("Q / Escape: drop tool"), "{hint}");
}
