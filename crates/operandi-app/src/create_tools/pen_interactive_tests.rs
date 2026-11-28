use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;

#[test]
fn polygon_pen_command_click_capture_refine_and_drop() {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Front;
    app.editor.scene.selection.mode = Mode::Polygon;
    app.category = toolbox::Category::Polygon;
    app.execute("tool.set prim.pen on");
    assert!(app.editor.session.pen_draft.is_some());
    let before = app.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut app, &ctx, size, vec![]);
    for point in [
        DVec3::new(-1., -1., 0.),
        DVec3::new(1., -1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(-1., 1., 0.),
    ] {
        let pos = app.camera.project(point, app.viewport_rect).unwrap().0;
        draw(
            &mut app,
            &ctx,
            size,
            vec![
                pointer(pos, true, Modifiers::NONE),
                pointer(pos, false, Modifiers::NONE),
            ],
        );
    }
    assert_eq!(
        app.editor
            .session
            .pen_draft
            .as_ref()
            .unwrap()
            .points()
            .len(),
        4
    );
    assert_eq!(app.editor.scene, before);
    app.editor.dispatch(Command::PenSelect(0)).unwrap();
    app.clear_fields();
    draw(&mut app, &ctx, size, vec![]);
    app.fields.insert("pen_x".into(), "-1 m".into());
    app.fields.insert("pen_y".into(), "-1 m".into());
    assert!(app.confirm_fields());
    assert_eq!(
        app.editor.session.pen_draft.as_ref().unwrap().points()[0],
        DVec3::new(-1., -1., 0.)
    );
    app.execute("tool.drop");
    assert!(app.editor.session.pen_draft.is_none());
    assert_eq!(app.editor.scene.active().unwrap().mesh.faces.len(), 1);
    app.execute("undo");
    assert_eq!(app.editor.scene, before);
}

#[test]
fn panel_route_remap_and_text_focus_do_not_capture() {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Front;
    app.category = toolbox::Category::Polygon;
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut app, &ctx, size, vec![]);
    let pos = button_by_line(&app, "Polygon Pen").center();
    draw(
        &mut app,
        &ctx,
        size,
        vec![
            pointer(pos, true, Modifiers::NONE),
            pointer(pos, false, Modifiers::NONE),
        ],
    );
    assert!(app.editor.session.pen_draft.is_some());
    app.run(Command::Cancel);
    app.bindings.insert("Polygon Pen".into(), "C".into());
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(
        &mut app,
        &ctx,
        size,
        vec![key(Key::C), Event::Text("c".into())],
    );
    assert!(app.editor.session.pen_draft.is_none());
    assert_eq!(app.command, "c");
    ctx.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    draw(
        &mut app,
        &ctx,
        size,
        vec![Event::Key {
            key: Key::C,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let viewport = app.viewport_rect.center();
    draw(
        &mut app,
        &ctx,
        size,
        vec![Event::PointerMoved(viewport), key(Key::C)],
    );
    assert!(app.editor.session.pen_draft.is_some());
}

#[test]
fn pen_press_adds_drag_moves_and_drop_faces_the_viewer() {
    // A click adds a vertex; pressing on a vertex of this session
    // and dragging moves it. User report 2026-10-30: clicks with a little
    // pointer motion were lost and clockwise polygons vanished after Drop.
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Perspective;
    app.editor.scene.selection.mode = Mode::Polygon;
    app.execute("tool.set prim.pen on");
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut app, &ctx, size, vec![]);
    let r = app.viewport_rect;
    let count = |a: &Operandi| a.editor.session.pen_draft.as_ref().unwrap().points().len();
    // Clockwise on screen, each click with a 2-pixel wobble.
    let corners = [(-150., 120.), (150., 120.), (150., 260.), (-150., 260.)];
    let corners: Vec<Pos2> = corners
        .iter()
        .rev()
        .map(|&(x, y)| r.center() + vec2(x, y))
        .collect();
    for &p in &corners {
        draw(
            &mut app,
            &ctx,
            size,
            vec![
                pointer(p, true, Modifiers::NONE),
                Event::PointerMoved(p + vec2(2., 1.)),
                pointer(p + vec2(2., 1.), false, Modifiers::NONE),
            ],
        );
    }
    assert_eq!(count(&app), 4, "{}", app.status);
    // Press on an existing point and drag it: it moves, nothing is added.
    let before = app.editor.session.pen_draft.as_ref().unwrap().points()[1];
    let p1 = app.camera.project(before, r).unwrap().0;
    draw(
        &mut app,
        &ctx,
        size,
        vec![
            pointer(p1, true, Modifiers::NONE),
            Event::PointerMoved(p1 + vec2(0., 30.)),
            pointer(p1 + vec2(0., 30.), false, Modifiers::NONE),
        ],
    );
    assert_eq!(count(&app), 4);
    let moved = app.editor.session.pen_draft.as_ref().unwrap().points()[1];
    assert!(moved.distance(before) > 1e-3);
    // Orbiting mid-draft keeps later points on the first point's plane.
    app.camera.pitch = -0.2;
    let extra = r.center() + vec2(0., 300.);
    draw(
        &mut app,
        &ctx,
        size,
        vec![
            pointer(extra, true, Modifiers::NONE),
            pointer(extra, false, Modifiers::NONE),
        ],
    );
    let points = app
        .editor
        .session
        .pen_draft
        .as_ref()
        .unwrap()
        .points()
        .to_vec();
    let plane = app.pen_plane.unwrap().0;
    let axis = match plane {
        1 => 0,
        2 => 2,
        _ => 1,
    };
    assert!(
        points
            .iter()
            .all(|p| (p[axis] - points[0][axis]).abs() < 1e-9),
        "{points:?}"
    );
    app.run(Command::Undo);
    assert_eq!(count(&app), 4);
    // A field with text focus does not capture.
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    draw(
        &mut app,
        &ctx,
        size,
        vec![
            pointer(extra, true, Modifiers::NONE),
            pointer(extra, false, Modifiers::NONE),
        ],
    );
    assert_eq!(count(&app), 4);
    ctx.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    // Q drops; the polygon faces the camera.
    app.run(Command::Drop);
    assert!(app.editor.session.pen_draft.is_none(), "{}", app.status);
    let mesh = &app.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 1);
    let (eye, _, _, _) = app.camera.basis();
    assert!(mesh.normal(0).dot(eye - mesh.center(0)) > 0.);
}

fn edit_pen_field(app: &mut Operandi, ctx: &Context, size: Vec2, field: &str, text: &str) {
    draw(app, ctx, size, vec![]);
    let old = app.fields[field].chars().count();
    let position = app.field_rects[field].center();
    draw(app, ctx, size, vec![Event::PointerMoved(position)]);
    draw(
        app,
        ctx,
        size,
        vec![pointer(position, true, Modifiers::NONE)],
    );
    draw(
        app,
        ctx,
        size,
        vec![pointer(position, false, Modifiers::NONE)],
    );
    let mut events = vec![key(Key::End)];
    events.extend((0..old).map(|_| key(Key::Backspace)));
    events.push(Event::Text(text.into()));
    draw(app, ctx, size, events);
    draw(app, ctx, size, vec![key(Key::Enter)]);
    draw(app, ctx, size, vec![]);
}

#[test]
fn actual_point_fields_refine_selected_point_and_cancel_invalid_text() {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Front;
    app.execute("tool.set prim.pen on");
    for point in [DVec3::ZERO, DVec3::X, DVec3::Y] {
        app.editor.dispatch(Command::PenCapture(point)).unwrap();
    }
    let original_scene = app.editor.scene.clone();
    let original_undo = app.editor.history.undo.clone();
    let original_redo = app.editor.history.redo.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    edit_pen_field(&mut app, &ctx, size, "pen_current", "0");
    assert_eq!(
        app.editor.session.pen_draft.as_ref().unwrap().current(),
        Some(0)
    );
    edit_pen_field(&mut app, &ctx, size, "pen_x", "-1 m");
    assert_eq!(
        app.editor.session.pen_draft.as_ref().unwrap().points(),
        &[DVec3::new(-1., 0., 0.), DVec3::X, DVec3::Y]
    );
    edit_pen_field(&mut app, &ctx, size, "pen_y", "-2 m");
    edit_pen_field(&mut app, &ctx, size, "pen_z", "0.1 m");
    assert_eq!(
        app.editor.session.pen_draft.as_ref().unwrap().points(),
        &[DVec3::new(-1., -2., 0.1), DVec3::X, DVec3::Y]
    );
    edit_pen_field(&mut app, &ctx, size, "pen_y", "invalid");
    assert!(app.field_errors.contains_key("pen_y"));
    app.execute("cancel");
    assert!(app.editor.session.pen_draft.is_none());
    assert_eq!(app.editor.scene, original_scene);
    assert!(app.editor.history.undo == original_undo && app.editor.history.redo == original_redo);
}

/// Pen panel options: Polyline clicked in the viewport keeps its click order
/// (no face-the-viewer reversal) and the panel draws for every Type.
#[test]
fn pen_panel_polyline_keeps_click_order() {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Front;
    app.editor.scene.selection.mode = Mode::Polygon;
    app.category = toolbox::Category::Polygon;
    app.execute("tool.set prim.pen on");
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    for output in [PenType::Vertices, PenType::Polygon, PenType::Polyline] {
        app.run(Command::PenOptions(PenOptions {
            output,
            ..Default::default()
        }));
        assert_eq!(
            app.editor
                .session
                .pen_draft
                .as_ref()
                .unwrap()
                .options()
                .output,
            output
        );
        draw(&mut app, &ctx, size, vec![]);
    }
    // Clockwise as seen from the front: a Polygon would be reversed on Drop.
    let points = [
        DVec3::new(-1., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(1., -1., 0.),
    ];
    for point in points {
        let pos = app.camera.project(point, app.viewport_rect).unwrap().0;
        draw(
            &mut app,
            &ctx,
            size,
            vec![
                pointer(pos, true, Modifiers::NONE),
                pointer(pos, false, Modifiers::NONE),
            ],
        );
    }
    app.execute("tool.drop");
    let mesh = &app.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.faces.len(), 1);
    assert_eq!(mesh.faces[0].vertices, vec![0, 1, 2]);
    assert!(
        mesh.vertices[0].distance(points[0]) < 1e-4,
        "click order kept"
    );
    assert_eq!(
        mesh.attributes.representations[0],
        PolygonRepresentation::Polyline {
            closed: false,
            curve: Default::default()
        }
    );
}

#[test]
fn pen_walls_from_the_panel_right_drag_offset_and_drop() {
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Front;
    app.editor.scene.selection.mode = Mode::Polygon;
    app.category = toolbox::Category::Polygon;
    app.execute("tool.set prim.pen on");
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut app, &ctx, size, vec![]);
    // The panel turns walls on; Offset starts at 0.1 m.
    let mut options = app.editor.session.pen_draft.as_ref().unwrap().options();
    options.wall = WallMode::BothSides;
    options.offset = 0.1;
    app.run(Command::PenOptions(options));
    for point in [DVec3::new(-1., 0., 0.), DVec3::new(1., 0., 0.)] {
        let pos = app.camera.project(point, app.viewport_rect).unwrap().0;
        draw(
            &mut app,
            &ctx,
            size,
            vec![
                pointer(pos, true, Modifiers::NONE),
                pointer(pos, false, Modifiers::NONE),
            ],
        );
    }
    // The front view draws on XY, so the strip lies in XY.
    assert_eq!(
        app.editor
            .session
            .pen_draft
            .as_ref()
            .unwrap()
            .options()
            .plane_normal,
        DVec3::Z
    );
    // Right-drag to the right widens the wall.
    let at = app.viewport_rect.center();
    let right = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Secondary,
    };
    let end = at + vec2(60., 0.);
    draw(
        &mut app,
        &ctx,
        size,
        vec![right(at, true), Event::PointerMoved(end), right(end, false)],
    );
    let offset = app
        .editor
        .session
        .pen_draft
        .as_ref()
        .unwrap()
        .options()
        .offset;
    assert!(offset > 0.1, "{offset}");
    app.execute("tool.drop");
    let m = &app.editor.scene.active().unwrap().mesh;
    assert_eq!((m.vertices.len(), m.faces.len()), (4, 1));
    assert!(m.vertices.iter().all(|v| v.z.abs() < 1e-9));
    let width = m.vertices.iter().map(|v| v.y).fold(f64::MIN, f64::max)
        - m.vertices.iter().map(|v| v.y).fold(f64::MAX, f64::min);
    assert!((width - 2. * offset).abs() < 1e-9, "{width} vs {offset}");
}

#[test]
fn make_quads_clicks_ctrl_triangle_shift_new_strip_and_drop() {
    // SPEC-PEN-MAKE-QUADS: a starting edge, a quad per click, Ctrl+click
    // ends with a triangle, Shift+click starts the next strip.
    let mut app = Operandi::empty();
    app.run(Command::NewMesh);
    app.camera.view = View::Front;
    app.editor.scene.selection.mode = Mode::Polygon;
    app.execute("tool.set prim.pen on");
    let ctx = Context::default();
    theme::install(&ctx);
    let size = vec2(1440., 900.);
    draw(&mut app, &ctx, size, vec![]);
    let mut options = app.editor.session.pen_draft.as_ref().unwrap().options();
    options.make_quads = true;
    app.run(Command::PenOptions(options));
    let click = |app: &mut Operandi, x: f64, y: f64, modifiers: Modifiers| {
        let pos = app
            .camera
            .project(DVec3::new(x, y, 0.), app.viewport_rect)
            .unwrap()
            .0;
        draw(
            app,
            &ctx,
            size,
            vec![
                Event::PointerMoved(pos),
                pointer(pos, true, modifiers),
                pointer(pos, false, modifiers),
            ],
        );
    };
    // Starting edge drawn right to left, so the strip as clicked faces
    // away from the Front view; Drop turns it to the viewer.
    click(&mut app, 1., 0., Modifiers::NONE);
    click(&mut app, 0., 0., Modifiers::NONE);
    click(&mut app, 1., 1., Modifiers::NONE);
    click(&mut app, 0.5, 2., Modifiers::CTRL);
    let draft = app.editor.session.pen_draft.as_ref().unwrap();
    assert_eq!(draft.points().len(), 5, "{}", app.status);
    assert!(draft.strip_ended());
    // Shift+click drops this strip and starts the next with Make Quads on.
    click(&mut app, 3., 0., Modifiers::SHIFT);
    let m = &app.editor.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 2, "{}", app.status);
    assert!((0..2).all(|f| m.normal(f).z > 0.99), "faces the Front view");
    assert!(
        app.editor
            .session
            .pen_draft
            .as_ref()
            .is_some_and(|d| d.options().strip() && d.points().is_empty())
    );
    for (x, y) in [(3., 0.), (4., 0.), (3., 1.)] {
        click(&mut app, x, y, Modifiers::NONE);
    }
    app.execute("tool.drop");
    let m = &app.editor.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 3);
    assert_eq!(app.editor.scene.selection.faces, vec![2]);
}
