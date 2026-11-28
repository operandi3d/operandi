//! Solid Sketch in the app (spec solid-sketch v0.3, tests 13–25 that can
//! run headless): tool.set and the preset, panel order, clicks, drags,
//! handles, delete, the pick-up question and the strip command.
use super::tests::{draw, pointer};
use super::*;

fn app() -> (Operandi, Context, Vec2) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.camera.view = View::Top;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut a, &ctx, size, vec![]);
    (a, ctx, size)
}
fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}
fn click(a: &mut Operandi, ctx: &Context, size: Vec2, at: Pos2, m: Modifiers) {
    draw(
        a,
        ctx,
        size,
        vec![pointer(at, true, m), pointer(at, false, m)],
    );
}
fn button(at: Pos2, pressed: bool, button: PointerButton, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: at,
        pressed,
        modifiers,
        button,
    }
}
fn drag(
    a: &mut Operandi,
    ctx: &Context,
    size: Vec2,
    from: Pos2,
    to: Pos2,
    b: PointerButton,
    m: Modifiers,
) {
    draw(a, ctx, size, vec![button(from, true, b, m)]);
    let mid = from + (to - from) * 0.5;
    draw(a, ctx, size, vec![Event::PointerMoved(mid)]);
    draw(a, ctx, size, vec![Event::PointerMoved(to)]);
    draw(a, ctx, size, vec![button(to, false, b, m)]);
}
fn session(a: &Operandi) -> &operandi_core::SketchSession {
    a.editor.session.solid_sketch.as_ref().unwrap()
}
fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}

#[test]
fn tool_set_bare_is_level_2_and_the_button_preset_level_1() {
    let (mut a, ..) = app();
    a.execute("tool.set solid.sketch on");
    assert_eq!(
        session(&a).options().subdiv,
        operandi_core::solid_sketch::SubdivMode::Level2
    );
    a.execute("tool.set solid.sketch ?");
    assert_eq!(a.status, "1");
    a.execute("tool.set solid.sketch off");
    assert!(a.editor.session.solid_sketch.is_none());
    a.execute("tool.set \"SS Default\" on");
    assert_eq!(
        session(&a).options().subdiv,
        operandi_core::solid_sketch::SubdivMode::Level1
    );
    a.run(Command::Cancel);
    a.execute("Solid Sketch");
    assert_eq!(
        session(&a).options().subdiv,
        operandi_core::solid_sketch::SubdivMode::Level1
    );
}

#[test]
fn no_mesh_layer_refuses() {
    let mut a = Operandi::empty();
    a.execute("tool.set solid.sketch on");
    assert!(a.editor.session.solid_sketch.is_none());
    assert_eq!(
        a.status,
        "Solid Sketch needs a mesh layer. Add or select a mesh first."
    );
}

#[test]
fn panel_rows_follow_the_spec_order() {
    let (mut a, ..) = app();
    a.execute("Solid Sketch");
    let keys: Vec<String> = a.tool_attr_fields().into_iter().map(|(k, _)| k).collect();
    assert_eq!(
        keys,
        [
            "emod", "subdiv", "rotate", "current", "cenX", "cenY", "cenZ", "sizX", "sizY", "sizZ",
            "bank", "align", "merge", "negative"
        ]
    );
}

#[test]
fn clicks_draw_a_chain_and_a_click_on_a_node_branches() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    for x in [-1., 0., 1.] {
        let at = screen(&a, DVec3::new(x, 0., 0.));
        click(&mut a, &ctx, size, at, Modifiers::NONE);
    }
    assert_eq!(session(&a).sketch().len(), 3);
    assert_eq!(faces(&a), 22);
    // Plain click: 100 mm.
    assert!((session(&a).sketch().nodes[0].size.x - 0.1).abs() < 1e-9);
    // Click node 1, then empty space: the new node branches from node 1.
    let n1 = screen(&a, session(&a).world_centres()[1]);
    click(&mut a, &ctx, size, n1, Modifiers::NONE);
    assert_eq!(session(&a).current(), Some(1));
    let at = screen(&a, DVec3::new(0., 0., 1.));
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    assert_eq!(session(&a).sketch().nodes[3].parent, Some(1));
    assert_eq!(faces(&a), 30);
}

#[test]
fn changing_subdiv_mode_rebuilds_at_once() {
    // Spec 6 rule 4: Level 2 bakes one smoothing step (4x the quads);
    // switching mode rebuilds the mesh without waiting for another edit.
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    for x in [-1., 0., 1.] {
        let at = screen(&a, DVec3::new(x, 0., 0.));
        click(&mut a, &ctx, size, at, Modifiers::NONE);
    }
    assert_eq!(faces(&a), 22);
    a.execute("tool.attr solid.sketch subdiv level2");
    assert_eq!(faces(&a), 88, "{}", a.status);
    a.execute("tool.attr solid.sketch subdiv none");
    assert_eq!(faces(&a), 22);
    a.execute("tool.attr solid.sketch subdiv level1");
    assert_eq!(faces(&a), 22);
}

#[test]
fn drag_sizes_a_new_node_and_moves_nodes() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    let from = screen(&a, DVec3::ZERO);
    let to = screen(&a, DVec3::new(0.5, 0., 0.));
    drag(
        &mut a,
        &ctx,
        size,
        from,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let s = session(&a).sketch().nodes[0].size;
    assert!((s.x - 0.5).abs() < 0.05 && s.x == s.y && s.y == s.z, "{s}");
    // One undo takes back the whole drag-created node.
    let at = screen(&a, DVec3::new(2., 0., 0.));
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    // Move node 1 by dragging its dot.
    let from = screen(&a, DVec3::new(2., 0., 0.));
    let to = screen(&a, DVec3::new(2., 0., 1.));
    drag(
        &mut a,
        &ctx,
        size,
        from,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let c = session(&a).sketch().nodes[1].center;
    assert!(
        c.distance(DVec3::new(2., 0., 1.)) < 0.05,
        "{c} {} {:?}",
        a.status,
        session(&a).current()
    );
    a.run(Command::Undo);
    let c = session(&a).sketch().nodes[1].center;
    assert!(c.distance(DVec3::new(2., 0., 0.)) < 1e-3, "{c}");
}

#[test]
fn right_drag_scales_and_middle_click_deletes() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    for x in [-1., 0., 1.] {
        let at = screen(&a, DVec3::new(x, 0., 0.));
        click(&mut a, &ctx, size, at, Modifiers::NONE);
    }
    let n2 = screen(&a, DVec3::new(1., 0., 0.));
    drag(
        &mut a,
        &ctx,
        size,
        n2,
        n2 + vec2(70., 0.),
        PointerButton::Secondary,
        Modifiers::NONE,
    );
    assert!(session(&a).sketch().nodes[2].size.x > 0.15);
    let n1 = screen(&a, DVec3::ZERO);
    draw(
        &mut a,
        &ctx,
        size,
        vec![
            button(n1, true, PointerButton::Middle, Modifiers::NONE),
            button(n1, false, PointerButton::Middle, Modifiers::NONE),
        ],
    );
    assert_eq!(session(&a).sketch().len(), 2);
    assert_eq!(session(&a).sketch().nodes[1].parent, Some(0));
}

#[test]
fn ctrl_click_on_a_link_inserts() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    for x in [-1., 1.] {
        let at = screen(&a, DVec3::new(x, 0., 0.));
        click(&mut a, &ctx, size, at, Modifiers::NONE);
    }
    let mid = screen(&a, DVec3::new(0.3, 0., 0.));
    click(&mut a, &ctx, size, mid, Modifiers::CTRL);
    let s = session(&a).sketch();
    assert_eq!(s.len(), 3);
    assert_eq!(s.nodes[1].parent, Some(0));
    assert_eq!(s.nodes[2].parent, Some(1));
    assert!(s.nodes[1].center.distance(DVec3::new(0.3, 0., 0.)) < 0.05);
    assert_eq!(session(&a).current(), Some(1));
}

#[test]
fn size_square_stretches_size_x_only() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    let at = screen(&a, DVec3::ZERO);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    // The red square sits at +Size X along the node's X (world X here).
    let red = screen(&a, DVec3::new(0.1, 0., 0.));
    let to = screen(&a, DVec3::new(0.4, 0., 0.));
    drag(
        &mut a,
        &ctx,
        size,
        red,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let s = session(&a).sketch().nodes[0].size;
    assert!((s.x - 0.4).abs() < 0.03, "{s}");
    assert!((s.y - 0.1).abs() < 1e-9 && (s.z - 0.1).abs() < 1e-9);
}

#[test]
fn destroy_mode_deletes_on_click() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    for x in [-1., 1.] {
        let at = screen(&a, DVec3::new(x, 0., 0.));
        click(&mut a, &ctx, size, at, Modifiers::NONE);
    }
    a.execute("tool.attr solid.sketch emod destroy");
    assert_eq!(
        session(&a).options().edit_mode,
        operandi_core::solid_sketch::EditMode::Destroy
    );
    let empty = screen(&a, DVec3::new(0., 0., 2.));
    click(&mut a, &ctx, size, empty, Modifiers::NONE);
    assert_eq!(session(&a).sketch().len(), 2);
    let n = screen(&a, DVec3::new(1., 0., 0.));
    click(&mut a, &ctx, size, n, Modifiers::NONE);
    assert_eq!(session(&a).sketch().len(), 1);
}

#[test]
fn tool_attr_reaches_fields_and_hidden_args() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    a.execute("tool.attr solid.sketch cenX 1");
    assert!(
        a.status.contains("no nodes") || a.status.contains("isn't available"),
        "{}",
        a.status
    );
    let at = screen(&a, DVec3::ZERO);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    a.execute("tool.attr solid.sketch cenX 1.5");
    assert!(
        (session(&a).sketch().nodes[0].center.x - 1.5).abs() < 1e-9,
        "{}",
        a.status
    );
    a.execute("tool.attr solid.sketch bank 0.5");
    assert!(
        (session(&a).sketch().nodes[0].twist - 0.5).abs() < 1e-9,
        "{}",
        a.status
    );
    a.execute("tool.attr solid.sketch subdiv none");
    assert_eq!(
        session(&a).options().subdiv,
        operandi_core::solid_sketch::SubdivMode::None
    );
    a.execute("tool.attr solid.sketch subdiv ?");
    assert_eq!(a.status, "none");
    for (attr, value, back) in [
        ("tension", "0.25", "0.25"),
        ("material", "Skin", "Skin"),
        ("uvProj", "default", "default"),
        ("cenU", "0.5", "0.5"),
        ("rotU", "0.5", "0.5"),
        ("negative", "true", "1"),
    ] {
        a.execute(&format!("tool.attr solid.sketch {attr} {value}"));
        a.execute(&format!("tool.attr solid.sketch {attr} ?"));
        assert_eq!(a.status, back, "{attr}");
    }
}

#[test]
fn reactivating_asks_and_strip_removes_the_data() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    let at = screen(&a, DVec3::ZERO);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    a.run(Command::Drop);
    a.execute("Solid Sketch");
    assert!(a.solid_sketch_ask.is_some());
    assert!(a.editor.session.solid_sketch.is_none());
    // Return answers Yes.
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![super::tests::key(Key::Enter)]);
    assert!(a.solid_sketch_ask.is_none());
    assert_eq!(session(&a).sketch().len(), 1);
    a.run(Command::Drop);
    a.execute("operandi.solidSketch.strip");
    assert!(
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .solid_sketch
            .is_none()
    );
    a.execute("Solid Sketch");
    assert!(a.solid_sketch_ask.is_none());
    assert!(a.editor.session.solid_sketch.is_some());
}

/// A background mesh with a point at (1, 0, 1), and an empty active mesh
/// for the sketch.
fn background_app() -> (Operandi, Context, Vec2) {
    let (mut a, ctx, size) = app();
    let mut mesh = operandi_core::Mesh {
        vertices: vec![
            DVec3::new(1., 0., 1.),
            DVec3::new(2., 0., 1.),
            DVec3::new(2., 0., 2.),
        ],
        ..Default::default()
    };
    mesh.faces.push(super::tests::face(vec![0, 1, 2]));
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.run(Command::NewMesh);
    draw(&mut a, &ctx, size, vec![]);
    (a, ctx, size)
}

/// SOLID-SKETCH-SPEC 5.3: with vertex snapping on, a new node and a moved
/// node snap to background geometry; activating the tool again (the
/// pick-up question, Yes) and clicking doesn't crash and still snaps.
#[test]
fn nodes_snap_on_create_and_move_and_reactivating_is_safe() {
    let (mut a, ctx, size) = background_app();
    a.snapping.enabled = true;
    a.execute("Solid Sketch");
    let target = DVec3::new(1., 0., 1.);
    let at = screen(&a, target) + vec2(6., -5.);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    let c = session(&a).world_centres()[0];
    assert!(c.distance(target) < 1e-9, "{c} ({})", a.status);
    // A second node, then moved onto the point at (2, 0, 2).
    let free = screen(&a, DVec3::new(0., 0., 2.));
    click(&mut a, &ctx, size, free, Modifiers::NONE);
    let from = screen(&a, session(&a).world_centres()[1]);
    let to = screen(&a, DVec3::new(2., 0., 2.)) + vec2(-6., 5.);
    drag(
        &mut a,
        &ctx,
        size,
        from,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let c = session(&a).world_centres()[1];
    assert!(
        c.distance(DVec3::new(2., 0., 2.)) < 1e-9,
        "{c} ({})",
        a.status
    );
    a.run(Command::Drop);
    a.execute("Solid Sketch");
    draw(&mut a, &ctx, size, vec![]);
    draw(&mut a, &ctx, size, vec![super::tests::key(Key::Enter)]);
    // The question's window was on screen in the Enter frame; a press
    // comes after a frame without it (a press on a window above a view
    // stays in that window, bug report #5).
    draw(&mut a, &ctx, size, vec![]);
    assert!(a.editor.session.solid_sketch.is_some(), "{}", a.status);
    let at = screen(&a, DVec3::new(2., 0., 1.)) + vec2(6., -5.);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    let centres = session(&a).world_centres();
    assert!(
        centres
            .iter()
            .any(|c| c.distance(DVec3::new(2., 0., 1.)) < 1e-9),
        "{centres:?} ({})",
        a.status
    );
}

/// SNH-Q02 (answered): with Symmetry X on, a new node near both the seam
/// and a snap target goes to the nearer on screen.
#[test]
fn a_new_node_takes_the_nearer_of_seam_and_target() {
    for (click_off_seam, target_off_click, seam_wins) in [(3., 6., true), (6., 2., false)] {
        let (mut a, ctx, size) = app();
        let r = a.viewport_rect;
        let seam = screen(&a, DVec3::new(0., 0., 1.));
        let at = seam + vec2(click_off_seam, 0.);
        let target_at = at + vec2(target_off_click, 0.);
        let target = a.work_point(target_at, r, 0, DVec3::ZERO).unwrap();
        let mut mesh = operandi_core::Mesh {
            vertices: vec![target, target + DVec3::X, target + DVec3::Z],
            ..Default::default()
        };
        mesh.faces.push(super::tests::face(vec![0, 1, 2]));
        a.editor.scene.active_mut().unwrap().mesh = mesh.into();
        a.run(Command::NewMesh);
        a.run(Command::Symmetry(operandi_core::Symmetry::x()));
        a.snapping.enabled = true;
        draw(&mut a, &ctx, size, vec![]);
        a.execute("Solid Sketch");
        // The first node always lands on the plane (SYM-B3-10); the
        // second is the one placed by the rule.
        let root = screen(&a, DVec3::new(0.5, 0., -1.));
        click(&mut a, &ctx, size, root, Modifiers::NONE);
        click(&mut a, &ctx, size, at, Modifiers::NONE);
        let c = session(&a).world_centres()[1];
        if seam_wins {
            assert!(c.x.abs() < 1e-9, "{c} ({})", a.status);
        } else {
            assert!(c.distance(target) < 1e-9, "{c} vs {target} ({})", a.status);
        }
    }
}

/// #45 (Jason, 2026-11-27): red, green and blue all size the node (X, Y,
/// Z); green stretches Size Y only.
#[test]
fn green_square_stretches_size_y_only() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    let at = screen(&a, DVec3::ZERO);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    // From the front the node's Y (world Y here) points up the screen.
    a.camera.view = View::Front;
    draw(&mut a, &ctx, size, vec![]);
    let (_, c, axes, s) = a.sketch_current_frame().unwrap();
    let green = screen(&a, c + axes[1] * s.y);
    let to = screen(&a, c + axes[1] * 0.4);
    drag(
        &mut a,
        &ctx,
        size,
        green,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let node = &session(&a).sketch().nodes[0];
    assert!((node.size.y - 0.4).abs() < 0.03, "{}", node.size);
    assert!((node.size.x - 0.1).abs() < 1e-9 && (node.size.z - 0.1).abs() < 1e-9);
    assert_eq!(node.twist, 0., "green no longer twists");
}

/// #45: Twist has its own dark green handle beyond the green square;
/// dragging it right turns the node one way, left the other, and leaves
/// the size alone.
#[test]
fn twist_handle_turns_with_left_right_drags() {
    let (mut a, ctx, size) = app();
    a.execute("Solid Sketch");
    let at = screen(&a, DVec3::ZERO);
    click(&mut a, &ctx, size, at, Modifiers::NONE);
    a.camera.view = View::Front;
    draw(&mut a, &ctx, size, vec![]);
    let r = a.viewport_rect;
    let handle = a.sketch_twist_handle(r).expect("a twist handle");
    let (_, c, axes, s) = a.sketch_current_frame().unwrap();
    let green = screen(&a, c + axes[1] * s.y);
    assert!(handle.distance(green) > 8., "apart from the green square");
    drag(
        &mut a,
        &ctx,
        size,
        handle,
        handle + vec2(60., 0.),
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let node = &session(&a).sketch().nodes[0];
    let right = node.twist;
    assert!(right.abs() > 0.1, "twisted: {right}");
    assert!(
        (node.size - DVec3::splat(0.1)).length() < 1e-9,
        "{}",
        node.size
    );
    let handle = a.sketch_twist_handle(r).unwrap();
    drag(
        &mut a,
        &ctx,
        size,
        handle,
        handle + vec2(-120., 0.),
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let left = session(&a).sketch().nodes[0].twist;
    assert!(left.signum() != right.signum(), "{right} then {left}");
}
