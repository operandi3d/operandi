//! Topology Pen v2 through real pointer events (BEHAVIOR-TOPOLOGY-PEN
//! TP-04, TP-10..TP-30, 2026-11-20). Fixture: a 4 × 4 grid of 1 m quads
//! in the XY plane seen from the front, no background mesh.
use super::tests::{draw, face};
use super::*;
use operandi_core::PenMode;

const SIZE: Vec2 = vec2(1200., 800.);

fn grid_app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let mut mesh = operandi_core::Mesh::default();
    for j in 0..5 {
        for i in 0..5 {
            mesh.vertices
                .push(DVec3::new(i as f64 - 2., j as f64 - 2., 0.));
        }
    }
    for j in 0..4u32 {
        for i in 0..4u32 {
            let a = j * 5 + i;
            mesh.faces.push(face(vec![a, a + 1, a + 6, a + 5]));
        }
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.camera.view = View::Front;
    a.camera.target = DVec3::ZERO;
    a.camera.distance = 8.;
    a.run(Command::Mode(Mode::Vertex));
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut a, &ctx, SIZE, vec![]);
    a.execute("topology pen");
    assert!(a.topology_pen_open(), "{}", a.status);
    a.editor.history.undo.clear();
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

fn point(x: i32, y: i32) -> u32 {
    ((y + 2) * 5 + x + 2) as u32
}

fn screen(a: &Operandi, x: f64, y: f64) -> Pos2 {
    a.camera
        .project(DVec3::new(x, y, 0.), a.viewport_rect)
        .unwrap()
        .0
}

fn button(pos: Pos2, button: PointerButton, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    }
}

/// Hover, press, move in steps and release with `b` and `m` held.
fn drag_with(
    a: &mut Operandi,
    ctx: &Context,
    from: Pos2,
    to: Pos2,
    b: PointerButton,
    m: Modifiers,
) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![button(from, b, true, m)]);
    for k in 1..=4 {
        let p = from + (to - from) * (k as f32 / 4.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![button(to, b, false, m)]);
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2, m: Modifiers) {
    drag_with(a, ctx, at, at, PointerButton::Primary, m);
}

fn set_mode(a: &mut Operandi, mode: PenMode) {
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.topology_pen.mode = mode;
    a.run(Command::Preview(p));
}

fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}

const CTRL: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: false,
    mac_cmd: false,
    command: false,
};

#[test]
fn tp10_the_form_has_the_modes_and_the_values_stick() {
    let (mut a, ctx) = grid_app();
    let p = a.editor.session.live.as_ref().unwrap().params.clone();
    let mut f = crate::tool_form::ToolForm::<Parameters>::default();
    crate::topology_pen_ui::pen_form(&p, &mut f);
    let keys: Vec<String> = f.fields().into_iter().map(|f| f.row.key.clone()).collect();
    for key in [
        "pen_mode",
        "pen_edge_loop",
        "pen_keep_vertices",
        "pen_split_middle",
        "pen_quad_only",
        "pen_strength",
        "pen_radius",
        "pen_inner_snap",
        "pen_copy_uvs",
        "pen_edge_slide",
    ] {
        assert!(keys.iter().any(|k| k == key), "{key} in {keys:?}");
    }
    a.fields.insert("pen_radius".into(), "0.5 m".into());
    assert!(a.confirm_fields(), "{}", a.status);
    set_mode(&mut a, PenMode::Fill);
    a.run(Command::Drop);
    a.execute("topology pen");
    draw(&mut a, &ctx, SIZE, vec![]);
    let o = a.pen_options();
    assert_eq!(o.mode, PenMode::Fill);
    assert_eq!(o.radius, 0.5);
}

#[test]
fn tp12_remove_mode_click_deletes_a_polygon_in_one_step() {
    let (mut a, ctx) = grid_app();
    set_mode(&mut a, PenMode::Remove);
    let at = screen(&a, -1.5, -1.5);
    click(&mut a, &ctx, at, Modifiers::NONE);
    assert_eq!(faces(&a), 15, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert!(a.topology_pen_open(), "the pen stays up");
    // Nothing under the pointer: no change, no undo step.
    let away = screen(&a, 3.5, 3.5);
    click(&mut a, &ctx, away, Modifiers::NONE);
    assert_eq!(faces(&a), 15);
    assert_eq!(a.editor.history.undo.len(), 1);
}

#[test]
fn tp13_middle_drag_splits_a_quad() {
    let (mut a, ctx) = grid_app();
    let from = screen(&a, -2., -2.);
    let to = screen(&a, -1., -1.);
    drag_with(
        &mut a,
        &ctx,
        from,
        to,
        PointerButton::Middle,
        Modifiers::NONE,
    );
    assert_eq!(faces(&a), 17, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), 1);
    // Released on a neighbour: nothing, and why.
    let next = screen(&a, -1., -2.);
    drag_with(
        &mut a,
        &ctx,
        from,
        next,
        PointerButton::Middle,
        Modifiers::NONE,
    );
    assert_eq!(faces(&a), 17);
    assert!(a.status.contains("already"), "{}", a.status);
}

#[test]
fn tp11_shift_drag_pulls_a_border_edge_and_refuses_an_interior_one() {
    let (mut a, ctx) = grid_app();
    let shift = Modifiers::SHIFT;
    // Interior edge between (0, 0) and (0, 1).
    let inner = screen(&a, 0., 0.5);
    drag_with(
        &mut a,
        &ctx,
        inner,
        inner + vec2(40., 0.),
        PointerButton::Primary,
        shift,
    );
    assert_eq!(a.status, "Duplicate needs a border edge");
    assert_eq!(faces(&a), 16);
    // The right border edge between (2, 0) and (2, 1).
    let edge = screen(&a, 2., 0.5);
    drag_with(
        &mut a,
        &ctx,
        edge,
        edge + vec2(60., 0.),
        PointerButton::Primary,
        shift,
    );
    assert_eq!(faces(&a), 17, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), 1);
}

#[test]
fn tp20_ctrl_click_in_empty_space_makes_a_quad() {
    let (mut a, ctx) = grid_app();
    let at = screen(&a, 3.5, 0.);
    click(&mut a, &ctx, at, CTRL);
    assert_eq!(faces(&a), 17, "{}", a.status);
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(m.normal(16).z > 0.9, "faces the front view");
    assert_eq!(a.editor.history.undo.len(), 1);
}

#[test]
fn tp15_point_mode_click_in_empty_space_makes_a_point() {
    let (mut a, ctx) = grid_app();
    set_mode(&mut a, PenMode::Point);
    let at = screen(&a, 3.5, 1.);
    click(&mut a, &ctx, at, Modifiers::NONE);
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 17, "{}", a.status);
    assert_eq!(m.faces[16].vertices.len(), 1);
}

#[test]
fn tp30_dragging_a_border_point_onto_another_welds_on_release() {
    let (mut a, ctx) = grid_app();
    let before = a.editor.scene.active().unwrap().mesh.vertices.len();
    let from = screen(&a, -2., 2.);
    let to = screen(&a, -1., 2.) + vec2(3., 2.);
    drag_with(
        &mut a,
        &ctx,
        from,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(m.vertices.len(), before - 1, "{}", a.status);
    assert!(m.faces.iter().any(|f| f.vertices.len() == 3));
    assert_eq!(a.editor.history.undo.len(), 1, "move and weld are one step");
    let _ = point(0, 0);
}

#[test]
fn tp12_item_mode_says_to_pick_a_component_mode() {
    let (mut a, ctx) = grid_app();
    a.editor.scene.selection.mode = Mode::Item;
    let at = screen(&a, -1.5, -1.5);
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    let kind = a.topology_pen_press(PointerButton::Primary, Modifiers::NONE, at, a.viewport_rect);
    assert!(matches!(kind, input::GestureKind::Blocked));
    assert!(a.status.contains("component mode"), "{}", a.status);
}

#[test]
fn tp04_the_topology_tab_button_turns_snapping_off_until_the_pen_drops() {
    let (mut a, _ctx) = grid_app();
    a.run(Command::Drop);
    if !a.snapping.enabled {
        a.toggle_snapping();
    }
    let constraint = a.editor.background_constraint.is_some();
    a.execute("operandi.topology.tool \"Topology Pen\"");
    assert!(a.topology_pen_open(), "{}", a.status);
    assert!(!a.snapping.enabled, "snapping off while the pen is up");
    a.run(Command::Drop);
    a.settle_topology_pen();
    assert!(a.snapping.enabled, "snapping back on");
    assert_eq!(a.editor.background_constraint.is_some(), constraint);
}
