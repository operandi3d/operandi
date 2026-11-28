//! Click to start in the app (tool-click-start-2026-11-16): an armed tool
//! shows its values and status but no handles and no change; the first
//! press in the view makes it live at the click and goes on as the tool's
//! own press (CS-15); Apply runs once and stays armed.
use super::tests::{box_app, draw, pointer};
use super::*;

const SIZE: Vec2 = vec2(1440., 1600.);

/// A 2 m box seen from the front, flags honoured as in the real app.
fn app(ctx: &Context, mode: Mode) -> Operandi {
    theme::install(ctx);
    let mut a = box_app();
    a.click_to_start = true;
    a.camera.view = View::Front;
    a.run(Command::Mode(mode));
    a.editor.history.undo.clear();
    for _ in 0..3 {
        draw(&mut a, ctx, SIZE, vec![]);
    }
    a
}
fn points(a: &Operandi) -> Vec<DVec3> {
    a.editor.scene.active().unwrap().mesh.vertices.clone()
}
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, by: Vec2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![pointer(from, true, Modifiers::NONE)]);
    for step in 1..=4 {
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(from + by * (step as f32 / 4.))],
        );
    }
    draw(
        a,
        ctx,
        SIZE,
        vec![pointer(from + by, false, Modifiers::NONE)],
    );
}
fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

/// CS-01/06: picking Bend arms it: no handles, no change, the status says
/// how to start. CS-02/10/11/14, BN-30a: the first press puts the centre
/// on the face under the pointer, Axis Z, spine fitted.
#[test]
fn bend_arms_then_the_first_press_starts_it_on_the_surface() {
    let ctx = Context::default();
    let mut a = app(&ctx, Mode::Vertex);
    let before = points(&a);
    a.execute("bend");
    assert_eq!(a.editor.armed_tool(), Some(Tool::Bend), "{}", a.status);
    assert_eq!(
        a.status,
        "Click in a view to start Bend, or set values and press Apply."
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.handles(a.viewport_rect).is_empty());
    let centre = screen(&a, DVec3::ZERO);
    assert_eq!(a.bend_hit(centre, a.viewport_rect), None, "no ring yet");
    assert_eq!(points(&a), before);
    // The front face is the one facing the eye.
    let (eye, ..) = a.camera.basis();
    let at = screen(&a, DVec3::new(0.5, 0.2, 0.));
    drag(&mut a, &ctx, at, vec2(0., 0.));
    assert!(!a.editor.is_armed(), "{}", a.status);
    let b = a.editor.session.live.as_ref().unwrap().params.bend;
    assert!(
        (b.base.z - eye.z.signum()).abs() < 1e-6,
        "on the face: {}",
        b.base
    );
    assert!(
        (b.base.x - 0.5).abs() < 0.05 && (b.base.y - 0.2).abs() < 0.05,
        "{}",
        b.base
    );
    assert_eq!(b.axis, 2, "the front plane's normal");
    assert!(
        b.spine.x < -1.4 && b.spine.y == 0.,
        "fitted -X: {}",
        b.spine
    );
    assert!(
        !a.handles(a.viewport_rect).is_empty()
            || a.bend_hit(screen(&a, b.base + b.spine), a.viewport_rect)
                .is_some()
    );
}

/// CS-21/22 through the panel's commands: Apply runs once and stays armed;
/// Drop of a never-clicked tool changes nothing.
#[test]
fn apply_runs_once_and_drop_changes_nothing() {
    let ctx = Context::default();
    let mut a = app(&ctx, Mode::Vertex);
    let before = points(&a);
    a.execute("bend");
    a.execute("tool.attr xfrm.bend angle 90");
    assert_eq!(points(&a), before, "armed: values only");
    a.run(Command::Drop);
    assert_eq!(points(&a), before);
    assert!(a.editor.history.undo.is_empty());
    a.execute("bend");
    a.execute("tool.attr xfrm.bend angle 90");
    a.run(Command::Apply);
    assert_ne!(points(&a), before);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert!(a.editor.is_armed());
}

/// Edge Bevel and Vertex Bevel start at once too (2026-11-23): picked on a
/// selection, each shows its gizmo with no click and leaves the mesh as it
/// was until a drag.
#[test]
fn edge_and_vertex_bevel_show_their_gizmo_at_once() {
    let ctx = Context::default();
    for (mode, tool) in [
        (Mode::Edge, Tool::EdgeBevel),
        (Mode::Vertex, Tool::VertexBevel),
    ] {
        let mut a = app(&ctx, mode);
        let edge = a.editor.scene.active().unwrap().mesh.edges()[0];
        match mode {
            Mode::Edge => a.editor.scene.selection.edges = vec![edge],
            _ => a.editor.scene.selection.vertices = vec![edge.0],
        }
        let before = points(&a);
        a.activate(tool);
        assert!(
            a.editor.session.live.is_some() && !a.editor.is_armed(),
            "{tool:?}: {}",
            a.status
        );
        draw(&mut a, &ctx, SIZE, vec![]);
        assert!(
            !a.handles(a.viewport_rect).is_empty(),
            "{tool:?}: the gizmo shows without a click"
        );
        assert_eq!(points(&a), before, "{tool:?}: picking it changed nothing");
    }
}

/// Polygon Bevel starts at once (CS-Q01 revisited 2026-11-23): picking it
/// shows its handles on the selection with no click and no change to the
/// mesh, and a drag anywhere then bevels as before.
#[test]
fn bevel_shows_its_handles_at_once_and_drags_the_bevel() {
    let ctx = Context::default();
    let mut a = app(&ctx, Mode::Polygon);
    a.editor.scene.selection.faces = [0].into_iter().collect();
    let before = points(&a);
    a.activate(Tool::Bevel);
    assert!(
        a.editor.session.live.is_some() && !a.editor.is_armed(),
        "{}",
        a.status
    );
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(
        !a.handles(a.viewport_rect).is_empty(),
        "the gizmo shows without a click"
    );
    assert_eq!(points(&a), before, "picking Bevel changed nothing yet");
    let from = a.viewport_rect.left_top() + vec2(120., 120.);
    drag(&mut a, &ctx, from, vec2(0., -60.));
    assert_eq!(a.editor.live_tool_label(), Some("Bevel"));
    assert_ne!(points(&a), before, "the drag bevelled: {}", a.status);
    // Vertex Extrude still starts at once.
    a.run(Command::Drop);
    a.run(Command::Mode(Mode::Vertex));
    a.editor.scene.selection.vertices = vec![0];
    a.activate(Tool::VertexExtrude);
    assert!(
        a.editor.session.live.is_some() && !a.editor.is_armed(),
        "{}",
        a.status
    );
}
