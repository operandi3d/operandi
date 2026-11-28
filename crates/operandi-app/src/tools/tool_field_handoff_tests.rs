//! Viewport and Tool Properties take turns with any tool open
//! (field_handoff.rs): after a field edit the views' handles and keys work
//! again, and Frame All (A) frames what the tool is previewing (Jason,
//! 2026-11-21, from the Model the Mug tutorial). The behaviour is shared,
//! so these run it across every primitive and a modelling tool.
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

fn click_at(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
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
    // The text sits left of the mini-slider arrows (UIC-53).
    click_at(a, ctx, pos2(at.left() + 12., at.center().y));
    let old = a.fields.get(field).cloned().unwrap_or_default();
    let mut events = vec![key(Key::End)];
    events.extend((0..old.chars().count()).map(|_| key(Key::Backspace)));
    events.push(Event::Text(new.into()));
    frame(a, ctx, events);
}

/// The four ways a field edit ends (UIC-51, UIC-53).
#[derive(Clone, Copy, Debug)]
enum Edit {
    /// Typed, then Return.
    Enter,
    /// Typed, then Tab: the next field holds the keyboard.
    Tab,
    /// Typed and left pending: the press in the view confirms it.
    Pending,
    /// A mini-slider drag.
    Slider,
}

fn edit(a: &mut Operandi, ctx: &Context, how: Edit, field: &str, new: &str) {
    match how {
        Edit::Slider => {
            frame(a, ctx, vec![]);
            let grip = super::tests::arrows_rect(a, field).center();
            let before = a.fields[field].clone();
            drag(a, ctx, grip, grip + vec2(40., 0.));
            assert_ne!(a.fields[field], before, "{field} scrubbed");
        }
        _ => {
            type_text(a, ctx, field, new);
            match how {
                Edit::Enter => frame(a, ctx, vec![key(Key::Enter)]),
                Edit::Tab => frame(a, ctx, vec![key(Key::Tab)]),
                _ => {}
            }
            frame(a, ctx, vec![]);
        }
    }
    assert!(a.field_errors.is_empty(), "{field}: {:?}", a.field_errors);
}

fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(a, ctx, vec![pointer(from, true, Modifiers::NONE)]);
    frame(a, ctx, vec![Event::PointerMoved(from + (to - from) * 0.5)]);
    frame(a, ctx, vec![Event::PointerMoved(to)]);
    frame(a, ctx, vec![pointer(to, false, Modifiers::NONE)]);
    frame(a, ctx, vec![]);
}

/// Move the pointer over the view and press A (Frame All).
fn frame_all(a: &mut Operandi, ctx: &Context) {
    let over = a.viewport_rect.center() + vec2(37., 23.);
    frame(a, ctx, vec![Event::PointerMoved(over)]);
    frame(a, ctx, vec![key(Key::A)]);
    // Let go of A, or the next press reads as a key repeat.
    let release = Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(a, ctx, vec![release]);
}

/// A primitive drawn with a drag on the Top view's ground.
fn drawn(tool: Tool) -> (Operandi, Context) {
    let ctx = context();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.camera.view = View::Top;
    a.camera.distance = 12.;
    a.activate(tool);
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    let r = a.viewport_rect;
    let from = a.camera.project(DVec3::new(-0.5, 0., -0.5), r).unwrap().0;
    let to = a.camera.project(DVec3::new(0.5, 0., 0.5), r).unwrap().0;
    drag(&mut a, &ctx, from, to);
    assert!(a.editor.session.live.as_ref().expect("tool open").applied);
    (a, ctx)
}

fn position(a: &Operandi) -> DVec3 {
    a.editor.session.live.as_ref().unwrap().params.position
}

/// Drag the X move arrow 60 points right; the shape must follow.
fn drag_x_arrow(a: &mut Operandi, ctx: &Context, what: &str) {
    let r = a.viewport_rect;
    let handles = a.handles(r);
    let x = handles
        .iter()
        .find(|h| h.0 == 0)
        .unwrap_or_else(|| panic!("{what}: X arrow in {handles:?}"))
        .2;
    let before = position(a);
    drag(a, ctx, x, x + vec2(60., 0.));
    let after = position(a);
    assert!(
        after.x > before.x + 0.1,
        "{what}: X arrow {before:?} -> {after:?} ({})",
        a.status
    );
}

#[test]
fn primitives_keep_handles_and_frame_all_after_every_kind_of_field_edit() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let size = if tool == Tool::Box {
            "size0"
        } else {
            "radius0"
        };
        for how in [Edit::Enter, Edit::Tab, Edit::Pending, Edit::Slider] {
            let what = format!("{tool:?} after {how:?}");
            let (mut a, ctx) = drawn(tool);
            // Field, then A, then handles, then the field again: the panel
            // and the view take turns as often as you like.
            // Typing not yet confirmed keeps the keys (see
            // `typed_text_and_tab_keep_the_field`); the press confirms it.
            let confirmed = !matches!(how, Edit::Pending);
            edit(&mut a, &ctx, how, size, "2");
            if confirmed {
                assert_frames_the_shape(&mut a, &ctx, &what);
            }
            drag_x_arrow(&mut a, &ctx, &what);
            // Y is the Top view's depth, so a pending value the press
            // confirms doesn't move the arrow out from under the pointer.
            edit(&mut a, &ctx, how, "position1", "1");
            drag_x_arrow(&mut a, &ctx, &what);
            edit(&mut a, &ctx, how, size, "3");
            if !confirmed {
                drag_x_arrow(&mut a, &ctx, &what);
            }
            assert_frames_the_shape(&mut a, &ctx, &what);
        }
    }
}

/// A over the view frames the shape wherever the view was looking.
fn assert_frames_the_shape(a: &mut Operandi, ctx: &Context, what: &str) {
    let at = position(a);
    a.camera.target = DVec3::splat(40.);
    frame_all(a, ctx);
    assert!(
        (a.camera.target - at).length() < 1.5,
        "{what}: framed {:?}, shape at {at:?} ({})",
        a.camera.target,
        a.status
    );
}

/// The numeric tools whose preview stands in for the scene until applied:
/// A frames the preview, not the empty mesh under it.
#[test]
fn frame_all_frames_a_numeric_tool_preview() {
    for (tool, position) in [
        ("Cone", "cone_position0"),
        ("Ellipsoid", "ell_position0"),
        ("Torus", "tor_position0"),
        ("Capsule", "cap_position0"),
    ] {
        let ctx = context();
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        a.camera.view = View::Top;
        a.execute(tool);
        frame(&mut a, &ctx, vec![]);
        edit(&mut a, &ctx, Edit::Enter, position, "25");
        assert!(a.editor.tool_preview_display().is_some(), "{tool} previews");
        a.camera.target = DVec3::ZERO;
        frame_all(&mut a, &ctx);
        assert!(
            (a.camera.target.x - 25.).abs() < 2.,
            "{tool}: framed {:?} ({})",
            a.camera.target,
            a.status
        );
    }
}

/// Not just primitives: a modelling tool's fields hand the keyboard back
/// the same way.
#[test]
fn a_modelling_tool_field_hands_the_keyboard_back_to_the_view() {
    let ctx = context();
    let mut a = super::tests::box_app();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = [0].into_iter().collect();
    a.activate(Tool::Move);
    frame(&mut a, &ctx, vec![]);
    let field = a
        .field_rects
        .keys()
        .find(|k| k.ends_with('0') && !k.ends_with("-link"))
        .cloned()
        .unwrap_or_else(|| panic!("a Move X field: {:?}", a.field_rects.keys()));
    edit(&mut a, &ctx, Edit::Tab, &field, "0.5");
    assert!(
        ctx.memory(|m| m.focused()).is_some(),
        "Tab left the next field focused"
    );
    a.camera.target = DVec3::splat(40.);
    frame_all(&mut a, &ctx);
    assert!(a.camera.target.length() < 3., "{:?}", a.camera.target);
}

/// Typing stays in its field when the pointer drifts over a view, so a
/// number is never split between the field and the shortcuts; and Tab
/// with the pointer resting on a view still moves between fields.
#[test]
fn typed_text_and_tab_keep_the_field() {
    let (mut a, ctx) = drawn(Tool::Box);
    type_text(&mut a, &ctx, "size0", "1");
    let field = ctx.memory(|m| m.focused());
    let target = a.camera.target;
    frame_all(&mut a, &ctx);
    assert_eq!(a.camera.target, target, "A stayed with the typing");
    assert_eq!(ctx.memory(|m| m.focused()), field);
    // Confirm with Tab while the pointer rests on the view.
    frame(&mut a, &ctx, vec![key(Key::Tab)]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.field_errors.is_empty(), "{:?}", a.field_errors);
    let focused = ctx.memory(|m| m.focused());
    assert!(focused.is_some(), "Tab moved to the next field");
    frame(&mut a, &ctx, vec![]);
    assert_eq!(ctx.memory(|m| m.focused()), focused, "no pointer move");
}

/// A scene with a big cube far off at X 100, so Frame All of the whole
/// scene and Frame All of the tool's shape land in different places.
fn scene_with_far_cube() -> (Operandi, Context) {
    let ctx = context();
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.activate(Tool::Box);
    a.run(Command::Preview(Parameters {
        position: DVec3::new(100., 0., 0.),
        size: DVec3::splat(50.),
        ..Default::default()
    }));
    a.run(Command::Apply);
    a.run(Command::Drop);
    a.camera.view = View::Top;
    a.camera.distance = 12.;
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}

/// Frame All mid-creation frames the shape being made, not the scene; with
/// nothing drawn yet it frames a starting-size box at the tool's centre
/// instead of zooming out to everything.
#[test]
fn frame_all_with_a_creation_tool_frames_the_new_shape() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let (mut a, ctx) = scene_with_far_cube();
        a.activate(tool);
        frame(&mut a, &ctx, vec![]);
        // Nothing drawn: the tool's centre, close in.
        a.camera.target = DVec3::splat(40.);
        frame_all(&mut a, &ctx);
        assert!(
            a.camera.target.length() < 1e-6,
            "{tool:?}: {:?}",
            a.camera.target
        );
        let close = a.camera.distance;
        assert!(close < 5., "{tool:?}: distance {close}");
        // Drawn: the new shape, wherever it is.
        let r = a.viewport_rect;
        let from = a.camera.project(DVec3::new(1., 0., 1.), r).unwrap().0;
        let to = a.camera.project(DVec3::new(2., 0., 2.), r).unwrap().0;
        drag(&mut a, &ctx, from, to);
        // (Sphere and Cylinder draw out from their centre, the Cube from
        // a corner.)
        let at = position(&a);
        assert!(at.x > 0.9, "{tool:?} drawn at {at:?}");
        a.camera.target = DVec3::splat(40.);
        frame_all(&mut a, &ctx);
        let t = a.camera.target;
        assert!(
            (t - at).length() < 0.1,
            "{tool:?}: framed {t:?}, shape {at:?}"
        );
        // Dropped: Frame All is the whole scene again.
        a.run(Command::Drop);
        frame_all(&mut a, &ctx);
        assert!(a.camera.target.x > 40., "{tool:?}: {:?}", a.camera.target);
    }
    // A numeric preview frames only what it adds.
    let (mut a, ctx) = scene_with_far_cube();
    a.execute("Cone");
    frame(&mut a, &ctx, vec![]);
    edit(&mut a, &ctx, Edit::Enter, "cone_position0", "3");
    a.camera.target = DVec3::splat(40.);
    frame_all(&mut a, &ctx);
    assert!((a.camera.target.x - 3.).abs() < 1., "{:?}", a.camera.target);
}

/// Every move arrow of every primitive moves it, in Perspective too.
#[test]
fn every_move_arrow_moves_its_primitive_in_perspective() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let (mut a, ctx) = drawn(tool);
        a.camera.view = View::Perspective;
        frame(&mut a, &ctx, vec![]);
        frame_all(&mut a, &ctx);
        frame(&mut a, &ctx, vec![]);
        for axis in 0..3 {
            // A press on the tip and one on the shaft (any point clear of
            // the side squares drawn on it) each move it.
            for part in ["tip", "shaft"] {
                let r = a.viewport_rect;
                let handles = a.handles(r);
                let h = handles
                    .iter()
                    .find(|h| h.0 == axis)
                    .unwrap_or_else(|| panic!("{tool:?} arrow {axis}: {handles:?}"))
                    .clone();
                let grab = if part == "tip" {
                    h.2
                } else {
                    (2..9)
                        .map(|t| h.1 + (h.2 - h.1) * (t as f32 / 10.))
                        .find(|&p| a.handle_hit(p, r) == Some(axis))
                        .unwrap_or_else(|| panic!("{tool:?} arrow {axis}: no shaft hit {h:?}"))
                };
                let before = position(&a);
                drag(&mut a, &ctx, grab, grab + (h.2 - h.1).normalized() * 40.);
                let after = position(&a);
                assert!(
                    (after - before)[axis] > 0.05,
                    "{tool:?} arrow {axis} {part} {h:?}: {before:?} -> {after:?} ({})",
                    a.status
                );
            }
        }
    }
}
