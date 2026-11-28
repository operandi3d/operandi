//! The shared drag layer in the viewport (loop-slice-2026-11-15): Loop
//! Slice's left, right and middle drags off the bar, the click that still
//! picks a ring, the trackpad stand-ins, and the other wired tools.
use super::tests::draw;
use super::*;
use operandi_core::haul::{self, Button, feel};

/// A strip of three unit quads in XZ (x 0..3, z 0..1) seen from the top.
/// Points 0..3 run along z = 0, points 4..7 along z = 1.
fn strip_app(ctx: &Context) -> Operandi {
    theme::install(ctx);
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..=1u32 {
        for i in 0..=3u32 {
            mesh.vertices.push(DVec3::new(i as f64, 0., j as f64));
        }
    }
    for i in 0..3u32 {
        mesh.faces
            .push(super::tests::face(vec![i, i + 4, i + 5, i + 1]));
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Top;
    a.frame(false);
    a
}

const SIZE: Vec2 = vec2(1440., 900.);

fn press(button: PointerButton, at: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: at,
        pressed,
        modifiers,
        button,
    }
}

/// A drag in small steps, as a mouse reports it.
fn haul(
    a: &mut Operandi,
    ctx: &Context,
    button: PointerButton,
    from: Pos2,
    to: Pos2,
    mods: Modifiers,
) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![press(button, from, true, mods)]);
    for k in 1..=8 {
        let at = from + (to - from) * (k as f32 / 8.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    }
    draw(a, ctx, SIZE, vec![press(button, to, false, mods)]);
}

fn loop_slice(a: &Operandi) -> &operandi_core::LoopSlice {
    &a.editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .slice
        .loop_slice
}

/// Loop Slice live on the ring of edge 0-4 (x = 0, z 0..1): it crosses all
/// three quads, so each slice at t puts points at z = t.
fn loop_slice_app(ctx: &Context) -> Operandi {
    let mut a = strip_app(ctx);
    a.run(Command::Mode(Mode::Edge));
    // Edge x = 0 from z 0 to 1: its ring crosses all three quads.
    a.editor.scene.selection.edges = vec![(0, 4)];
    draw(&mut a, ctx, SIZE, vec![]);
    a.activate(Tool::LoopSlice);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        6,
        "{}",
        a.status
    );
    draw(&mut a, ctx, SIZE, vec![]);
    a
}

/// A point in the view well away from the bar and the mesh.
fn open_space(a: &Operandi) -> Pos2 {
    a.viewport_rect.center() + vec2(0., 250.)
}

#[test]
fn left_drag_off_the_bar_moves_the_current_slice_one_bar_pixel_per_pixel() {
    let ctx = Context::default();
    let mut a = loop_slice_app(&ctx);
    let from = open_space(&a);
    haul(
        &mut a,
        &ctx,
        PointerButton::Primary,
        from,
        from + vec2(50., 3.),
        Modifiers::NONE,
    );
    // LS-Q02 provisional: 50 px on a 200 px bar = +25 %.
    let o = loop_slice(&a);
    assert!(
        (o.positions[0] - 0.75).abs() < 1e-6,
        "{:?} {}",
        o.positions,
        a.status
    );
    // The cut moved with it, live: points at 75 % across the ring.
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(
        m.vertices
            .iter()
            .any(|v| v.distance(DVec3::new(1., 0., 0.75)) < 1e-6)
    );
    // One undo step for the drag; the tool stays live.
    a.run(Command::Undo);
    assert!((loop_slice(&a).positions[0] - 0.5).abs() < 1e-9);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::LoopSlice)
    );
}

#[test]
fn the_bar_still_works_and_wins_over_the_drag_layer() {
    let ctx = Context::default();
    let mut a = loop_slice_app(&ctx);
    let r = a.viewport_rect;
    let triangle = r.left_top() + vec2(20. + 100., 50. + 4.);
    haul(
        &mut a,
        &ctx,
        PointerButton::Primary,
        triangle,
        triangle - vec2(60., 0.),
        Modifiers::NONE,
    );
    assert!(
        (loop_slice(&a).positions[0] - 0.2).abs() < 1e-6,
        "{:?}",
        loop_slice(&a).positions
    );
}

#[test]
fn a_click_that_doesnt_move_still_picks_a_ring() {
    let ctx = Context::default();
    let mut a = loop_slice_app(&ctx);
    // The edge across the strip at x = 2 (points 2 and 6) belongs to the
    // same ring; the edge along it from x 1 to 2 at z 0 starts another.
    let along = a
        .camera
        .project(DVec3::new(1.5, 0., 0.), a.viewport_rect)
        .unwrap()
        .0;
    haul(
        &mut a,
        &ctx,
        PointerButton::Primary,
        along,
        along + vec2(1., 1.),
        Modifiers::NONE,
    );
    assert_eq!(loop_slice(&a).rings, vec![(1, 2)], "{}", a.status);
    // Nothing moved: the slice stays at 50 %.
    assert!((loop_slice(&a).positions[0] - 0.5).abs() < 1e-9);
}

#[test]
fn right_drag_sideways_sets_count_in_whole_steps() {
    let ctx = Context::default();
    let mut a = loop_slice_app(&ctx);
    let from = open_space(&a);
    let px = feel::WHOLE_STEP_PX;
    haul(
        &mut a,
        &ctx,
        PointerButton::Secondary,
        from,
        from + vec2(2. * px + 3., 0.),
        Modifiers::NONE,
    );
    assert_eq!(loop_slice(&a).count(), 3, "{}", a.status);
    // Left is fewer; Count stops at 1.
    haul(
        &mut a,
        &ctx,
        PointerButton::Secondary,
        from,
        from - vec2(10. * px, 0.),
        Modifiers::NONE,
    );
    assert_eq!(loop_slice(&a).count(), 1);
    a.run(Command::Undo);
    assert_eq!(loop_slice(&a).count(), 3, "each drag is one undo step");
}

#[test]
fn middle_drag_sideways_picks_current_and_up_sets_inset() {
    let ctx = Context::default();
    let mut a = loop_slice_app(&ctx);
    a.editor
        .session
        .live
        .as_mut()
        .unwrap()
        .params
        .slice
        .loop_slice
        .set_count(3);
    let p = a.editor.session.live.as_ref().unwrap().params.clone();
    a.run(Command::Preview(p));
    let from = open_space(&a);
    let px = feel::WHOLE_STEP_PX;
    let current = loop_slice(&a).current;
    haul(
        &mut a,
        &ctx,
        PointerButton::Middle,
        from,
        from + vec2(px + 1., 2.),
        Modifiers::NONE,
    );
    assert_eq!(loop_slice(&a).current, current + 1, "{}", a.status);
    // Up/down: Inset, a distance that follows the view; up is more.
    haul(
        &mut a,
        &ctx,
        PointerButton::Middle,
        from,
        from - vec2(2., 40.),
        Modifiers::NONE,
    );
    let inset = loop_slice(&a).inset;
    assert!(inset > 0., "{inset} {}", a.status);
    // The axis locked to up/down: Current didn't change on the way.
    assert_eq!(loop_slice(&a).current, current + 1);
    // Positive Inset digs in: the new points sit below the surface (-Y,
    // the strip faces +Y from the top view).
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(
        m.vertices.iter().any(|v| (v.y + inset).abs() < 1e-9),
        "{inset}"
    );
}

#[test]
fn cmd_and_cmd_shift_drags_stand_in_for_middle_and_right() {
    let ctx = Context::default();
    let mut a = loop_slice_app(&ctx);
    let from = open_space(&a);
    let px = feel::WHOLE_STEP_PX;
    let cmd = Modifiers {
        mac_cmd: true,
        command: cfg!(target_os = "macos"),
        ..Modifiers::NONE
    };
    haul(
        &mut a,
        &ctx,
        PointerButton::Primary,
        from,
        from + vec2(2. * px + 1., 0.),
        cmd | Modifiers::SHIFT,
    );
    assert_eq!(loop_slice(&a).count(), 3, "Cmd+Shift = right: {}", a.status);
    let current = loop_slice(&a).current;
    haul(
        &mut a,
        &ctx,
        PointerButton::Primary,
        from,
        from + vec2(px + 1., 0.),
        cmd,
    );
    assert_eq!(
        loop_slice(&a).current,
        (current + 1).min(2),
        "Cmd = middle: {}",
        a.status
    );
    // Ctrl is left alone (BIBLE Keys F1).
    assert_eq!(crate::haul_view::trackpad_button(Modifiers::CTRL), None);
}

#[test]
fn bevel_right_drag_sets_segments_and_left_keeps_its_own_drag() {
    let ctx = Context::default();
    let mut a = strip_app(&ctx);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![1];
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::Bevel);
    draw(&mut a, &ctx, SIZE, vec![]);
    let before = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .bevel
        .segments;
    let from = open_space(&a);
    haul(
        &mut a,
        &ctx,
        PointerButton::Secondary,
        from,
        from + vec2(feel::WHOLE_STEP_PX * 2. + 1., 0.),
        Modifiers::NONE,
    );
    let after = a
        .editor
        .session
        .live
        .as_ref()
        .unwrap()
        .params
        .bevel
        .segments;
    assert_eq!(after, before + 2, "{}", a.status);
    assert!(!haul::claims(Tool::Bevel, Button::Left), "DL-09");
}

/// The tester (2026-11-25): after Round Level became Segments, a right
/// drag in the view no longer changed Edge Bevel's segments (the table
/// still named the old field). Right/left adds or removes one per step,
/// down to 0, a flat strip.
#[test]
fn edge_bevel_right_drag_sets_segments() {
    let ctx = Context::default();
    let mut a = strip_app(&ctx);
    a.run(Command::Mode(Mode::Edge));
    a.editor.scene.selection.edges = vec![(1, 5)];
    draw(&mut a, &ctx, SIZE, vec![]);
    a.activate(Tool::EdgeBevel);
    draw(&mut a, &ctx, SIZE, vec![]);
    let rows = |a: &Operandi| {
        a.editor
            .session
            .live
            .as_ref()
            .unwrap()
            .params
            .chamfer
            .segments
    };
    let before = rows(&a);
    let from = open_space(&a);
    let step = feel::WHOLE_STEP_PX;
    haul(
        &mut a,
        &ctx,
        PointerButton::Secondary,
        from,
        from + vec2(step * 2. + 1., 0.),
        Modifiers::NONE,
    );
    assert_eq!(rows(&a), before + 2, "{}", a.status);
    haul(
        &mut a,
        &ctx,
        PointerButton::Secondary,
        from,
        from - vec2(step * 40., 0.),
        Modifiers::NONE,
    );
    assert_eq!(rows(&a), 1, "Segments 0, a flat strip: {}", a.status);
    // Up/down works too: up adds.
    haul(
        &mut a,
        &ctx,
        PointerButton::Secondary,
        from,
        from - vec2(0., step * 3. + 1.),
        Modifiers::NONE,
    );
    assert_eq!(rows(&a), 4, "up adds: {}", a.status);
}

#[test]
fn every_table_value_is_a_number_field_on_its_tools_panel() {
    // A haul writes through the tool's Tool Properties field; a typo in the
    // table would make that drag do nothing. Start each tool on a cube and
    // look the keys up in its form.
    let ctx = Context::default();
    for (tool, row) in haul::table() {
        let mut a = strip_app(&ctx);
        let mode = match tool {
            Tool::EdgeBevel | Tool::EdgeChamfer | Tool::LoopSlice => Mode::Edge,
            Tool::VertexBevel => Mode::Vertex,
            _ => Mode::Polygon,
        };
        a.run(Command::Mode(mode));
        match mode {
            Mode::Edge => a.editor.scene.selection.edges = vec![(1, 5)],
            Mode::Vertex => a.editor.scene.selection.vertices = vec![1],
            _ => a.editor.scene.selection.faces = vec![1],
        }
        draw(&mut a, &ctx, SIZE, vec![]);
        a.activate(*tool);
        assert_eq!(
            a.editor.session.live.as_ref().map(|l| l.tool),
            Some(*tool),
            "{tool:?}: {}",
            a.status
        );
        for button in [Button::Left, Button::Middle, Button::Right] {
            for axis in [haul::Axis::Sideways, haul::Axis::Up] {
                if let Some(key) = row.pair(button).get(axis) {
                    assert!(
                        a.haul_field_exists(*tool, key),
                        "{tool:?} {button:?} {axis:?}: no number field {key}"
                    );
                }
            }
        }
    }
}
