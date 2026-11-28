//! Drag-to-scrub on Tool Properties numeric fields (scrub.rs).
use super::tests::{box_app, draw};
use super::*;

const SIZE: Vec2 = vec2(1440., 900.);

/// One frame with `events` and `modifiers` held.
fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>, modifiers: Modifiers) {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
        events,
        modifiers,
        focused: true,
        ..Default::default()
    };
    let _ = ctx.run(raw, |ctx| {
        a.begin_input(ctx);
        a.compose(ctx);
        a.keyboard(ctx);
    });
}

fn button(pos: Pos2, pressed: bool) -> Event {
    super::tests::pointer(pos, pressed, Modifiers::NONE)
}

/// Where `key`'s scrub handle is: the mini-slider arrows just right of its
/// text (UIC-53).
fn handle_at(a: &Operandi, key: &str) -> Pos2 {
    super::tests::arrows_rect(a, key).center()
}

/// Press on `key`'s handle and move far enough to start the drag; returns
/// the pointer position the measured drag continues from.
fn start_scrub(a: &mut Operandi, ctx: &Context, key: &str) -> Pos2 {
    for _ in 0..2 {
        let _ = draw(a, ctx, SIZE, vec![]);
    }
    let at = handle_at(a, key);
    frame(a, ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    frame(a, ctx, vec![button(at, true)], Modifiers::NONE);
    // Wiggle out and back so the drag starts with no net change.
    let out = at + vec2(20., 0.);
    frame(a, ctx, vec![Event::PointerMoved(out)], Modifiers::NONE);
    frame(a, ctx, vec![Event::PointerMoved(at)], Modifiers::NONE);
    assert!(
        a.scrub.as_ref().is_some_and(|s| s.key == key),
        "scrub started on {key}"
    );
    // Coming back lands on the 0 detent (UIC-54); start the measured drag
    // free of it.
    a.scrub.as_mut().unwrap().held = None;
    at
}

fn move_to(a: &mut Operandi, ctx: &Context, pos: Pos2, modifiers: Modifiers) {
    frame(a, ctx, vec![Event::PointerMoved(pos)], modifiers);
}

fn release(a: &mut Operandi, ctx: &Context, pos: Pos2) {
    frame(a, ctx, vec![button(pos, false)], Modifiers::NONE);
    let _ = draw(a, ctx, SIZE, vec![]);
}

fn top_y(a: &Operandi) -> f64 {
    let item = a.editor.scene.active().unwrap();
    item.mesh.faces[5]
        .vertices
        .iter()
        .map(|&v| item.mesh.vertices[v as usize].y)
        .sum::<f64>()
        / 4.
}

/// Every numeric field gets a handle; dragging Move's Y offset moves the
/// selection live, records nothing until release, then exactly one step,
/// and the tool stays live at zero like after a viewport handle drag.
#[test]
fn move_offset_scrubs_live_and_each_drag_undoes_alone() {
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Move);
    let before_y = top_y(&a);
    let history = a.editor.history.undo.len();
    let at = start_scrub(&mut a, &ctx, "delta1");
    let mut pos = at;
    for _ in 0..5 {
        pos += vec2(10., 0.);
        move_to(&mut a, &ctx, pos, Modifiers::NONE);
    }
    // 50 points at 1 cm each.
    assert_eq!(a.fields["delta1"], workspace::number_text(0.5, " m"));
    assert!((top_y(&a) - before_y - 0.5).abs() < 1e-9, "live preview");
    assert_eq!(
        a.editor.history.undo.len(),
        history,
        "nothing recorded mid-drag"
    );
    release(&mut a, &ctx, pos);
    // The scrub is one tweak of Move, recorded when Move drops.
    assert_eq!(a.editor.history.undo.len(), history);
    assert!(a.editor.session.tweaks.can_undo());
    assert!((top_y(&a) - before_y - 0.5).abs() < 1e-9);
    assert_eq!(a.editor.session.live.as_ref().unwrap().tool, Tool::Move);
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().params.delta,
        DVec3::ZERO
    );
    assert!(a.scrub.is_none());
    // A second drag is a second step; Undo takes back only that one.
    let at = start_scrub(&mut a, &ctx, "delta1");
    move_to(&mut a, &ctx, at + vec2(-20., 0.), Modifiers::NONE);
    release(&mut a, &ctx, at + vec2(-20., 0.));
    assert_eq!(a.editor.history.undo.len(), history);
    assert!(
        (top_y(&a) - before_y - 0.3).abs() < 1e-9,
        "{} {:?}",
        top_y(&a) - before_y,
        a.fields.get("delta1")
    );
    a.run(Command::Undo);
    assert!((top_y(&a) - before_y - 0.5).abs() < 1e-9);
}

/// A press and release on the handle without moving records nothing.
#[test]
fn a_click_on_the_handle_changes_nothing() {
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Move);
    let before = a.editor.scene.clone();
    let history = a.editor.history.undo.len();
    let at = start_scrub(&mut a, &ctx, "delta0");
    release(&mut a, &ctx, at);
    assert!(a.editor.scene == before);
    assert_eq!(a.editor.history.undo.len(), history);
}

/// Shift is ten times coarser and Option ten times finer.
#[test]
fn shift_and_option_change_the_step() {
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Move);
    let at = start_scrub(&mut a, &ctx, "delta0");
    move_to(&mut a, &ctx, at + vec2(3., 0.), Modifiers::SHIFT);
    assert_eq!(a.fields["delta0"], workspace::number_text(0.3, " m"));
    move_to(&mut a, &ctx, at + vec2(8., 0.), Modifiers::ALT);
    assert_eq!(a.fields["delta0"], workspace::number_text(0.305, " m"));
    release(&mut a, &ctx, at + vec2(8., 0.));
}

/// Escape mid-drag puts the value back and leaves the tool and history as
/// they were; the rest of the drag does nothing.
#[test]
fn escape_puts_the_value_back() {
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    a.run(Command::Preview(
        a.editor.session.live.as_ref().unwrap().params.clone(),
    ));
    let start = a.editor.session.live.as_ref().unwrap().params.shift;
    let history = a.editor.history.undo.len();
    let at = start_scrub(&mut a, &ctx, "shift");
    move_to(&mut a, &ctx, at + vec2(25., 0.), Modifiers::NONE);
    assert!((a.editor.session.live.as_ref().unwrap().params.shift - start - 0.25).abs() < 1e-9);
    frame(
        &mut a,
        &ctx,
        vec![super::tests::key(Key::Escape)],
        Modifiers::NONE,
    );
    assert_eq!(
        a.editor.session.live.as_ref().unwrap().tool,
        Tool::Bevel,
        "Escape didn't cancel the tool"
    );
    assert_eq!(a.editor.session.live.as_ref().unwrap().params.shift, start);
    move_to(&mut a, &ctx, at + vec2(60., 0.), Modifiers::NONE);
    assert_eq!(a.editor.session.live.as_ref().unwrap().params.shift, start);
    release(&mut a, &ctx, at + vec2(60., 0.));
    assert_eq!(a.editor.history.undo.len(), history);
    assert_eq!(a.fields["shift"], workspace::number_text(start, " m"));
}

/// Counts move in whole steps and stop at the tool's limit with no error
/// left on the field; dragging back responds at once. Dropping the tool
/// records the whole session as one step, as typing does.
#[test]
fn counts_step_whole_numbers_and_stop_at_the_limit() {
    let ctx = Context::default();
    let mut a = box_app();
    a.editor.scene.selection.faces = vec![5];
    a.activate(Tool::Bevel);
    a.run(Command::Preview(
        a.editor.session.live.as_ref().unwrap().params.clone(),
    ));
    a.fields.insert("inset".into(), "0.1 m".into());
    assert!(a.confirm_fields());
    let history = a.editor.history.undo.len();
    let segments = || "bevel_segments";
    let at = start_scrub(&mut a, &ctx, segments());
    move_to(&mut a, &ctx, at + vec2(10., 0.), Modifiers::NONE);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .bevel
            .segments,
        3
    );
    assert_eq!(a.fields[segments()], "3");
    move_to(&mut a, &ctx, at + vec2(-200., 0.), Modifiers::NONE);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .bevel
            .segments,
        1
    );
    assert_eq!(a.fields[segments()], "1");
    assert!(a.field_errors.is_empty(), "{:?}", a.field_errors);
    move_to(&mut a, &ctx, at + vec2(-195., 0.), Modifiers::NONE);
    assert_eq!(
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .bevel
            .segments,
        2
    );
    release(&mut a, &ctx, at + vec2(-195., 0.));
    assert_eq!(a.editor.history.undo.len(), history);
    a.run(Command::Drop);
    assert_eq!(
        a.editor.history.undo.len(),
        history + 1,
        "{} live={:?}",
        a.status,
        a.editor.session.live.as_ref().map(|l| l.tool)
    );
}
