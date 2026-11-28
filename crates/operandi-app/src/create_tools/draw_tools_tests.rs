//! Create > Draw curve tools through viewport clicks (DRW-01, DRAW-00).
use super::*;
use crate::draw_tools::DrawKind;
use operandi_core::{CurveKind, PolygonRepresentation};

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
    ctx.options_mut(|o| o.max_passes = std::num::NonZeroUsize::MIN);
    let _ = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 900.))),
            focused: true,
            events,
            ..Default::default()
        },
        |ctx| {
            a.begin_input(ctx);
            a.compose(ctx);
            a.keep_utility_exclusive();
            a.keep_draw_valid();
            a.keyboard(ctx);
        },
    );
}
fn button(p: Pos2, pressed: bool, button: PointerButton, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: p,
        pressed,
        modifiers,
        button,
    }
}
fn click_with(a: &mut Operandi, ctx: &Context, p: Pos2, b: PointerButton, m: Modifiers) {
    frame(a, ctx, vec![Event::PointerMoved(p)]);
    frame(a, ctx, vec![button(p, true, b, m)]);
    frame(a, ctx, vec![button(p, false, b, m)]);
    frame(a, ctx, vec![]);
}
fn click(a: &mut Operandi, ctx: &Context, p: Pos2) {
    click_with(a, ctx, p, PointerButton::Primary, Modifiers::NONE);
}
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(
        a,
        ctx,
        vec![button(from, true, PointerButton::Primary, Modifiers::NONE)],
    );
    frame(
        a,
        ctx,
        vec![Event::PointerMoved((from + to.to_vec2()) / 2.)],
    );
    frame(a, ctx, vec![Event::PointerMoved(to)]);
    frame(
        a,
        ctx,
        vec![button(to, false, PointerButton::Primary, Modifiers::NONE)],
    );
    frame(a, ctx, vec![]);
}
fn app(kind: DrawKind) -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.history.undo.clear();
    a.category = toolbox::Category::Create;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    a.activate_draw(kind);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}
fn spot(a: &Operandi, dx: f32, dy: f32) -> Pos2 {
    a.viewport_rect.center() + vec2(dx, dy)
}
fn curves(a: &Operandi) -> Vec<(Vec<DVec3>, bool, CurveKind)> {
    let item = a.editor.scene.active().unwrap();
    let m = &item.mesh;
    (0..m.faces.len())
        .filter_map(|f| match m.attributes.representations.get(f) {
            Some(PolygonRepresentation::Polyline { closed, curve }) => Some((
                m.faces[f]
                    .vertices
                    .iter()
                    .map(|&v| m.vertices[v as usize])
                    .collect(),
                *closed,
                *curve,
            )),
            _ => None,
        })
        .collect()
}
fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}

#[test]
fn curve_clicks_add_points_one_undo_step_each() {
    let (mut a, ctx) = app(DrawKind::Curve);
    let p0 = spot(&a, -100., 0.);
    click(&mut a, &ctx, p0);
    assert!(curves(&a).is_empty(), "first point waits in the tool");
    let p0 = spot(&a, 0., -60.);
    click(&mut a, &ctx, p0);
    let p0 = spot(&a, 100., 0.);
    click(&mut a, &ctx, p0);
    let c = curves(&a);
    assert_eq!(c.len(), 1, "{}", a.status);
    assert_eq!(c[0].0.len(), 3);
    assert!(matches!(c[0].2, CurveKind::Curve { .. }));
    // On the construction plane through the origin.
    let normal = match a.actual_plane() {
        0 => 1,
        1 => 0,
        _ => 2,
    };
    assert!(
        c[0].0.iter().all(|p| p[normal].abs() < 1e-9),
        "{:?}",
        c[0].0
    );
    // In click order.
    let s: Vec<Pos2> = c[0].0.iter().map(|p| screen(&a, *p)).collect();
    assert!(
        s[0].x < s[1].x && s[1].x < s[2].x && s[1].y < s[0].y,
        "{s:?}"
    );
    assert_eq!(a.editor.history.undo.len(), 2);
    a.run(Command::Undo);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(curves(&a)[0].0.len(), 2);
    assert!(a.draw.is_some(), "the tool stays on after Undo");
}

#[test]
fn a_click_inserts_after_the_current_point() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let p0 = spot(&a, x, 0.);
        click(&mut a, &ctx, p0);
    }
    let first = curves(&a)[0].0[0];
    let p = screen(&a, first); // current = 0
    click(&mut a, &ctx, p);
    let p0 = spot(&a, 0., 80.);
    click(&mut a, &ctx, p0);
    let c = &curves(&a)[0].0;
    assert_eq!(c.len(), 3);
    assert!(
        c[1].z > c[0].z.max(c[2].z) || c[1].z < c[0].z.min(c[2].z) || c[1].x.abs() < 0.5,
        "inserted in the middle: {c:?}"
    );
    assert!((c[0] - first).length() < 1e-9);
}

#[test]
fn dragging_a_point_moves_it_in_one_step() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let p0 = spot(&a, x, 0.);
        click(&mut a, &ctx, p0);
    }
    let before = curves(&a)[0].0.clone();
    let undo = a.editor.history.undo.len();
    let p = screen(&a, before[1]);
    drag(&mut a, &ctx, p, p + vec2(0., 70.));
    let after = &curves(&a)[0].0;
    assert!((after[1] - before[1]).length() > 1e-3);
    assert!((after[0] - before[0]).length() < 1e-9);
    assert_eq!(a.editor.history.undo.len(), undo + 1);
}

#[test]
fn right_click_first_point_closes_middle_click_deletes() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for (x, y) in [(-150., 0.), (0., -80.), (150., 0.)] {
        let p0 = spot(&a, x, y);
        click(&mut a, &ctx, p0);
    }
    let first = screen(&a, curves(&a)[0].0[0]);
    click_with(
        &mut a,
        &ctx,
        first,
        PointerButton::Secondary,
        Modifiers::NONE,
    );
    assert!(curves(&a)[0].1, "closed: {}", a.status);
    let middle = screen(&a, curves(&a)[0].0[1]);
    click_with(&mut a, &ctx, middle, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(curves(&a)[0].0.len(), 2);
    assert!(curves(&a)[0].1, "deleting keeps it closed");
}

#[test]
fn shift_click_starts_a_separate_curve() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 0.] {
        let p0 = spot(&a, x, 0.);
        click(&mut a, &ctx, p0);
    }
    let p0 = spot(&a, 50., 100.);
    click_with(&mut a, &ctx, p0, PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(curves(&a).len(), 1, "Shift-click adds no point");
    for x in [60., 160.] {
        let p0 = spot(&a, x, 100.);
        click(&mut a, &ctx, p0);
    }
    assert_eq!(curves(&a).len(), 2);
}

#[test]
fn dropping_a_short_b_spline_removes_it() {
    let (mut a, ctx) = app(DrawKind::BSpline);
    for x in [-150., 0., 150.] {
        let p0 = spot(&a, x, 0.);
        click(&mut a, &ctx, p0);
    }
    assert_eq!(curves(&a).len(), 1);
    assert_eq!(curves(&a)[0].2, CurveKind::BSpline);
    a.drop_draw();
    assert!(
        curves(&a).is_empty(),
        "three points are too few for a B-Spline"
    );
    let (mut b, ctx) = app(DrawKind::BSpline);
    for x in [-150., -50., 50., 150.] {
        let p0 = spot(&b, x, 0.);
        click(&mut b, &ctx, p0);
    }
    b.drop_draw();
    assert_eq!(curves(&b).len(), 1);
}

#[test]
fn bezier_press_drag_pulls_out_handles_as_one_step() {
    let (mut a, ctx) = app(DrawKind::Bezier);
    let p0 = spot(&a, -150., 0.);
    click(&mut a, &ctx, p0);
    let undo = a.editor.history.undo.len();
    let p0 = spot(&a, 100., 0.);
    let p1 = spot(&a, 160., 40.);
    drag(&mut a, &ctx, p0, p1);
    assert_eq!(a.editor.history.undo.len(), undo + 1, "{}", a.status);
    let c = &curves(&a)[0];
    assert_eq!(c.2, CurveKind::Bezier);
    // a0, out0, in1, a1: the second anchor's in handle mirrors the drag.
    assert_eq!(c.0.len(), 4);
    let (inn, anchor) = (c.0[2], c.0[3]);
    assert!((inn - anchor).length() > 1e-3, "handles pulled out");
    // A plain click makes a sharp anchor (zero-length handles).
    let p0 = spot(&a, 0., 120.);
    click(&mut a, &ctx, p0);
    let c = &curves(&a)[0].0;
    assert_eq!(c.len(), 7);
    assert!((c[5] - c[6]).length() < 1e-9);
}

#[test]
fn a_curve_tool_turns_off_for_a_modeling_tool_and_resumes_on_a_selected_curve() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let p0 = spot(&a, x, 0.);
        click(&mut a, &ctx, p0);
    }
    a.run(Command::Activate(Tool::Move));
    frame(&mut a, &ctx, vec![]);
    assert!(a.draw.is_none());
    a.run(Command::Drop);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate_draw(DrawKind::Curve);
    assert_eq!(a.draw.as_ref().unwrap().face, Some(0), "{}", a.status);
    // CV-21: it opens in Edit mode; switching to Add adds points again.
    a.draw.as_mut().unwrap().mode = crate::draw_tools::DrawMode::Add;
    let p0 = spot(&a, 0., 90.);
    click(&mut a, &ctx, p0);
    assert_eq!(curves(&a).len(), 1);
    assert_eq!(curves(&a)[0].0.len(), 3);
}

#[test]
fn cv20_a_selected_curve_opens_in_edit_mode_on_its_first_point() {
    use crate::draw_tools::DrawMode;
    let (mut a, ctx) = app(DrawKind::Curve);
    for (x, y) in [(-150., 0.), (0., 0.), (150., 0.)] {
        let at = spot(&a, x, y);
        click(&mut a, &ctx, at);
    }
    a.run(Command::Drop);
    a.drop_draw();
    // A second curve, then select both: the first selected one opens.
    a.activate_draw(DrawKind::Curve);
    for x in [-150., 150.] {
        let at = spot(&a, x, -150.);
        click(&mut a, &ctx, at);
    }
    a.drop_draw();
    frame(&mut a, &ctx, vec![]);
    assert!(a.draw.is_none());
    assert_eq!(curves(&a).len(), 2);
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0, 1];
    a.activate_draw(DrawKind::Curve);
    frame(&mut a, &ctx, vec![]);
    let t = a.draw.as_ref().unwrap();
    assert_eq!(
        (t.face, t.mode, t.current),
        (Some(0), DrawMode::Edit, Some(0))
    );
    // Edit: clicking empty space adds nothing.
    // SetCurve re-creates the edited polygon last: follow the long one.
    let edited = |a: &Operandi| {
        curves(a)
            .into_iter()
            .map(|c| c.0)
            .max_by_key(Vec::len)
            .unwrap()
    };
    let empty = spot(&a, 0., 150.);
    click(&mut a, &ctx, empty);
    assert_eq!(edited(&a).len(), 3);
    // Dragging a point moves it.
    let before = edited(&a)[1];
    let from = screen(&a, before);
    drag(&mut a, &ctx, from, from + vec2(0., 60.));
    let after = edited(&a)[1];
    assert_eq!(edited(&a).len(), 3);
    assert!((after - before).length() > 1e-6, "{before} -> {after}");
    // Add after the current point (DRW-04): point 1 was current.
    a.draw.as_mut().unwrap().mode = DrawMode::Add;
    let at = spot(&a, 75., 120.);
    click(&mut a, &ctx, at);
    assert_eq!(edited(&a).len(), 4);
    assert_eq!(a.draw.as_ref().unwrap().current, Some(2));
    // A new curve (nothing selected) starts in Add mode.
    a.drop_draw();
    a.editor.scene.selection.faces.clear();
    a.activate_draw(DrawKind::Curve);
    assert_eq!(a.draw.as_ref().unwrap().mode, DrawMode::Add);
}

fn stroke(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(
        a,
        ctx,
        vec![button(from, true, PointerButton::Primary, Modifiers::NONE)],
    );
    for k in 1..=20 {
        let t = k as f32 / 20.;
        let p = from + (to - from) * t + vec2(0., (t * std::f32::consts::PI).sin() * 40.);
        frame(a, ctx, vec![Event::PointerMoved(p)]);
    }
    frame(
        a,
        ctx,
        vec![button(to, false, PointerButton::Primary, Modifiers::NONE)],
    );
    frame(a, ctx, vec![]);
}

#[test]
fn sketch_stroke_makes_a_curve_and_type_change_rebuilds_it() {
    let (mut a, ctx) = app(DrawKind::Sketch);
    let (from, to) = (spot(&a, -200., 0.), spot(&a, 200., 0.));
    stroke(&mut a, &ctx, from, to);
    let c = curves(&a);
    assert_eq!(c.len(), 1, "{}", a.status);
    assert!(matches!(c[0].2, CurveKind::Curve { .. }));
    assert!(
        c[0].0.len() >= 3,
        "fitted to a few points: {}",
        c[0].0.len()
    );
    let undo = a.editor.history.undo.len();
    // Type Quads rebuilds the same stroke as a strip, still one step.
    if let Some(t) = a.draw.as_mut() {
        t.sketch.kind = operandi_core::sketch::SketchType::Quads;
    }
    a.rebuild_sketch_for_test();
    assert_eq!(a.editor.history.undo.len(), undo);
    let m = &a.editor.scene.active().unwrap().mesh;
    assert!(curves(&a).is_empty());
    assert!(m.faces.len() > 5, "{} quads", m.faces.len());
    assert!(m.faces.iter().all(|f| f.vertices.len() == 4));
}

#[test]
fn draw_patch_fills_four_curves_that_meet() {
    let (mut a, ctx) = frozen_patch_app();
    // Four straight lines around a 2 m square on the ground.
    let corners = [
        DVec3::new(-1., 0., -1.),
        DVec3::new(1., 0., -1.),
        DVec3::new(1., 0., 1.),
        DVec3::new(-1., 0., 1.),
    ];
    for k in 0..4 {
        a.run(Command::SetCurve(Box::new(operandi_core::CurveEdit {
            item: None,
            face: None,
            points: vec![corners[k], corners[(k + 1) % 4]],
            closed: false,
            kind: CurveKind::Line,
            label: "Line".into(),
        })));
    }
    frame(&mut a, &ctx, vec![]);
    for k in [0usize, 1, 2, 3, 0] {
        let middle = (corners[k] + corners[(k + 1) % 4]) / 2.;
        let p = screen(&a, middle);
        click(&mut a, &ctx, p);
    }
    let m = &a.editor.scene.active().unwrap().mesh;
    assert_eq!(m.faces.len(), 4 + 100, "{}", a.status);
    // Faces the viewer (camera above the ground).
    assert!(m.normal(4).y > 0.9, "normal {}", m.normal(4));
}

#[test]
fn draw_patch_refuses_a_curve_that_does_not_join() {
    let (mut a, ctx) = frozen_patch_app();
    for (p, q) in [
        (DVec3::new(-1., 0., -1.), DVec3::new(1., 0., -1.)),
        (DVec3::new(-1., 0., 1.), DVec3::new(1., 0., 1.)),
    ] {
        a.run(Command::SetCurve(Box::new(operandi_core::CurveEdit {
            item: None,
            face: None,
            points: vec![p, q],
            closed: false,
            kind: CurveKind::Line,
            label: "Line".into(),
        })));
    }
    frame(&mut a, &ctx, vec![]);
    let p = screen(&a, DVec3::new(0., 0., -1.));
    click(&mut a, &ctx, p);
    let p = screen(&a, DVec3::new(0., 0., 1.));
    click(&mut a, &ctx, p);
    assert_eq!(a.draw.as_ref().unwrap().picks.len(), 1);
    assert!(a.status.contains("shares an end"), "{}", a.status);
}

#[test]
fn pen_shift_click_ends_the_polygon_and_starts_the_next() {
    let (mut a, ctx) = app(DrawKind::Curve);
    a.drop_draw();
    a.run(Command::Mode(Mode::Polygon));
    a.execute("polygon pen");
    assert!(a.editor.session.pen_draft.is_some(), "{}", a.status);
    for (x, y) in [(-100., 0.), (100., 0.), (0., -100.)] {
        let p = spot(&a, x, y);
        click(&mut a, &ctx, p);
    }
    assert_eq!(
        a.editor.session.pen_draft.as_ref().unwrap().points().len(),
        3
    );
    let p = spot(&a, 200., 100.);
    click_with(&mut a, &ctx, p, PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        1,
        "{}",
        a.status
    );
    let draft = a
        .editor
        .session
        .pen_draft
        .as_ref()
        .expect("a new Pen polygon started");
    assert!(draft.points().is_empty(), "Shift-click adds no point");
}

#[test]
fn curve_points_snap_onto_other_curves_points() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 0.] {
        let p = spot(&a, x, 0.);
        click(&mut a, &ctx, p);
    }
    let end = curves(&a)[0].0[1];
    let p = spot(&a, 50., 100.);
    click_with(&mut a, &ctx, p, PointerButton::Primary, Modifiers::SHIFT);
    // Start the second curve 4 px from the first one's end: it snaps.
    let near = screen(&a, end) + vec2(3., 2.);
    click(&mut a, &ctx, near);
    let p = spot(&a, 150., 80.);
    click(&mut a, &ctx, p);
    let c = curves(&a);
    assert_eq!(c.len(), 2);
    assert!((c[1].0[0] - end).length() < 1e-12, "{} vs {end}", c[1].0[0]);
}

#[test]
fn torus_capsule_arc_draw_out_instead_of_appearing() {
    for (kind, name) in [
        (ShapeKind::Torus, "Torus"),
        (ShapeKind::Capsule, "Capsule"),
        (ShapeKind::Arc, "Arc"),
    ] {
        let (mut a, ctx) = app(DrawKind::Curve);
        a.drop_draw();
        a.arm_shape(kind);
        frame(&mut a, &ctx, vec![]);
        assert!(
            a.editor.session.shape.as_ref().unwrap().preview.is_none(),
            "{name}: nothing until drawn"
        );
        let (from, to) = (spot(&a, 0., 0.), spot(&a, 120., 0.));
        drag(&mut a, &ctx, from, to);
        assert!(
            a.editor.session.shape.as_ref().unwrap().preview.is_some(),
            "{name}: {}",
            a.status
        );
        a.run(Command::Drop);
        assert!(
            !a.editor.scene.active().unwrap().mesh.faces.is_empty(),
            "{name}"
        );
    }
}

#[test]
fn curve_points_current_and_hovered_are_active_others_handle_colour() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 0., 150.] {
        let at = spot(&a, x, 0.);
        click(&mut a, &ctx, at);
    }
    let active = theme::viewport::handle_active();
    let idle = theme::viewport::handle();
    let none = Color32::TRANSPARENT;
    let r = a.viewport_rect;
    let away = spot(&a, 0., 200.);
    // The last point placed is current: filled Active; the rest Handle.
    assert_eq!(
        a.curve_point_looks(Some(away), r),
        vec![(none, idle), (none, idle), (active, active)]
    );
    // Hovering the first point lights its outline.
    let first = screen(&a, curves(&a)[0].0[0]);
    assert_eq!(
        a.curve_point_looks(Some(first), r),
        vec![(none, active), (none, idle), (active, active)]
    );
    // Clicking it makes it current; the old current goes back to Handle.
    click(&mut a, &ctx, first);
    assert_eq!(
        a.curve_point_looks(Some(away), r),
        vec![(active, active), (none, idle), (none, idle)]
    );
}

#[test]
fn cv14_new_curve_line_is_grey_a_reopened_curve_uses_the_selection_colour() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let at = spot(&a, x, 0.);
        click(&mut a, &ctx, at);
    }
    assert_eq!(a.curve_line_colour(), theme::viewport::label());
    a.drop_draw();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate_draw(DrawKind::Curve);
    assert_eq!(a.curve_line_colour(), theme::viewport::selection());
    // Shift-click (in Add mode) starts a new curve: grey again.
    a.draw.as_mut().unwrap().mode = crate::draw_tools::DrawMode::Add;
    let at = spot(&a, 0., 120.);
    click_with(&mut a, &ctx, at, PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!(a.curve_line_colour(), theme::viewport::label());
}

fn press_key(a: &mut Operandi, ctx: &Context, key: Key) {
    let event = |pressed| Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(a, ctx, vec![event(true)]);
    frame(a, ctx, vec![event(false)]);
}

#[test]
fn cv01_q_and_escape_drop_every_draw_tool_and_keep_the_curve() {
    for key in [Key::Q, Key::Escape] {
        for kind in [
            DrawKind::Curve,
            DrawKind::BSpline,
            DrawKind::Bezier,
            DrawKind::Sketch,
            DrawKind::Patch,
        ] {
            let (mut a, ctx) = app(kind);
            if kind.edits_points() {
                for x in [-150., -50., 50., 150.] {
                    let at = spot(&a, x, 0.);
                    click(&mut a, &ctx, at);
                }
            }
            let before = curves(&a).len();
            let over = spot(&a, 0., 200.);
            frame(&mut a, &ctx, vec![Event::PointerMoved(over)]);
            press_key(&mut a, &ctx, key);
            assert!(a.draw.is_none(), "{key:?} drops {kind:?}");
            assert_eq!(curves(&a).len(), before, "{key:?} keeps the {kind:?} curve");
        }
    }
}

#[test]
fn cv03_space_opens_mini_properties_and_keeps_the_curve_tool() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 0., 150.] {
        let at = spot(&a, x, 0.);
        click(&mut a, &ctx, at);
    }
    let before = a.draw.clone();
    let over = spot(&a, 40., 60.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(over)]);
    press_key(&mut a, &ctx, Key::Space);
    // MP-01/05: open (and still open after the release), the tool as it was.
    assert_eq!(a.draw, before, "Space leaves the curve tool as it was");
    assert_eq!(a.mini_properties.map(|m| m.at), Some(over));
    assert!(a.mini_shows_tool(), "the curve tool's short view");
    // MP-07: it stays put when the pointer moves away (opened pinned).
    let away = spot(&a, -300., -200.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(away)]);
    assert_eq!(a.mini_properties.map(|m| m.at), Some(over));
    // MP-06: a click outside goes to the tool (adds a point) and the
    // pop-over stays.
    let points = curves(&a)[0].0.len();
    click(&mut a, &ctx, away);
    assert_eq!(curves(&a)[0].0.len(), points + 1);
    assert!(a.mini_properties.is_some());
    // MP-04: Esc drops the tool; the pop-over stays, now on the item.
    frame(&mut a, &ctx, vec![Event::PointerMoved(over)]);
    press_key(&mut a, &ctx, Key::Escape);
    assert!(a.draw.is_none());
    assert!(a.mini_properties.is_some());
    assert!(!a.mini_shows_tool());
    // MP-06: Space again closes it.
    press_key(&mut a, &ctx, Key::Space);
    assert!(a.mini_properties.is_none());
}

#[test]
fn n03_presses_on_the_mini_properties_title_bar_skip_the_tool() {
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let at = spot(&a, x, 0.);
        click(&mut a, &ctx, at);
    }
    let over = spot(&a, 40., 60.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(over)]);
    press_key(&mut a, &ctx, Key::Space);
    frame(&mut a, &ctx, vec![]);
    let rect = *a.overlay_rects.first().expect("the pop-over's place");
    assert!(
        spot(&a, 0., 0.).y > rect.top(),
        "the pop-over is over the view"
    );
    let points = curves(&a)[0].0.len();
    // The title bar: a click there is the pop-over's, not a new point.
    click(&mut a, &ctx, rect.center_top() + vec2(-40., 12.));
    assert_eq!(curves(&a)[0].0.len(), points);
    assert!(a.mini_properties.is_some());
    // Closed, nothing is held back any more.
    press_key(&mut a, &ctx, Key::Space);
    frame(&mut a, &ctx, vec![]);
    assert!(a.overlay_rects.is_empty());
}

#[test]
fn mp03_the_curve_short_view_is_mode_point_closed_and_controls() {
    use crate::tool_form::ToolFields;
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let at = spot(&a, x, 0.);
        click(&mut a, &ctx, at);
    }
    let p = crate::draw_tools::DrawFields::params(&a).unwrap();
    let form = crate::draw_tools::DrawFields::form(&a, &p);
    let mini: Vec<String> = form
        .fields()
        .into_iter()
        .filter(|f| f.mini)
        .map(|f| f.row.label.clone())
        .collect();
    assert_eq!(
        mini,
        [
            "Mode",
            "Position X",
            "Y",
            "Z",
            "Closed",
            "Start Control",
            "End Control"
        ],
        "{mini:?}"
    );
}

#[test]
fn cv26_shift_click_in_edit_mode_adds_nothing() {
    use crate::draw_tools::DrawMode;
    let (mut a, ctx) = app(DrawKind::Curve);
    for x in [-150., 150.] {
        let at = spot(&a, x, 0.);
        click(&mut a, &ctx, at);
    }
    a.draw.as_mut().unwrap().mode = DrawMode::Edit;
    let before = (curves(&a), a.draw.clone());
    let at = spot(&a, 0., 150.);
    click_with(&mut a, &ctx, at, PointerButton::Primary, Modifiers::SHIFT);
    assert_eq!((curves(&a), a.draw.clone()), before);
}

#[test]
fn cv27_a_one_point_curve_is_discarded_when_the_tool_drops() {
    for kind in [DrawKind::Curve, DrawKind::BSpline, DrawKind::Bezier] {
        let (mut a, ctx) = app(kind);
        let faces = a.editor.scene.active().unwrap().mesh.faces.len();
        let at = spot(&a, 0., 0.);
        click(&mut a, &ctx, at);
        press_key(&mut a, &ctx, Key::Q);
        assert!(a.draw.is_none());
        assert!(curves(&a).is_empty(), "{kind:?}");
        assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), faces);
    }
}

// ------------------------------------------------ G7 Patch Curves

/// Draw Patch with Freeze on (PC-1..PC-6): the patch is frozen polygons.
fn frozen_patch_app() -> (Operandi, Context) {
    let (mut a, ctx) = app(DrawKind::Patch);
    a.draw.as_mut().unwrap().patch.freeze = true;
    (a, ctx)
}

/// Adds one open curve through `points` to the active mesh.
fn add_curve(a: &mut Operandi, points: &[DVec3], kind: CurveKind) {
    a.run(Command::SetCurve(Box::new(operandi_core::CurveEdit {
        item: None,
        face: None,
        points: points.to_vec(),
        closed: false,
        kind,
        label: "Line".into(),
    })));
}

/// Clicks the middle handle of each listed curve (by its middle point).
fn pick(a: &mut Operandi, ctx: &Context, middles: &[DVec3]) {
    frame(a, ctx, vec![]);
    for m in middles {
        let p = screen(a, *m);
        click(a, ctx, p);
    }
}

fn line_loop(a: &mut Operandi, corners: &[DVec3]) -> Vec<DVec3> {
    let n = corners.len();
    for k in 0..n {
        add_curve(a, &[corners[k], corners[(k + 1) % n]], CurveKind::Line);
    }
    (0..n)
        .map(|k| (corners[k] + corners[(k + 1) % n]) / 2.)
        .collect()
}

fn mesh(a: &Operandi) -> &operandi_core::Mesh {
    &a.editor.scene.active().unwrap().mesh
}

/// PC-2 test 1: four picks build nothing; the fifth click, on the first
/// curve again, builds the patch as one undo step.
#[test]
fn patch_curves_build_on_the_closing_click() {
    let (mut a, ctx) = frozen_patch_app();
    let mids = line_loop(
        &mut a,
        &[
            DVec3::new(-1., 0., -1.),
            DVec3::new(1., 0., -1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(-1., 0., 1.),
        ],
    );
    let undo = a.editor.history.undo.len();
    pick(&mut a, &ctx, &mids);
    assert_eq!(mesh(&a).faces.len(), 4, "four picks alone build nothing");
    assert_eq!(a.draw.as_ref().unwrap().picks.len(), 4);
    assert!(a.status.contains("close the loop"), "{}", a.status);
    pick(&mut a, &ctx, &mids[..1]);
    assert_eq!(mesh(&a).faces.len(), 4 + 100, "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    assert!(a.draw.as_ref().unwrap().picks.is_empty());
    a.run(Command::Undo);
    assert_eq!(mesh(&a).faces.len(), 4);
}

/// PC-3 test 2: three curves make N x N cells, the row at the corner
/// opposite the first curve (PC-Q1) triangles, and that corner one point.
#[test]
fn patch_curves_three_curves_collapse_one_corner() {
    let (mut a, ctx) = frozen_patch_app();
    let (p0, p1, p2) = (
        DVec3::new(-1., 0., -1.),
        DVec3::new(1., 0., -1.),
        DVec3::new(0., 0., 1.),
    );
    let mids = line_loop(&mut a, &[p0, p1, p2]);
    if let Some(t) = a.draw.as_mut() {
        t.patch.parallel = 16;
        t.patch.perpendicular = 16;
    }
    pick(&mut a, &ctx, &[mids[0], mids[1], mids[2], mids[0]]);
    let m = mesh(&a);
    let cells: Vec<usize> = (3..m.faces.len()).collect();
    let tris = cells.iter().filter(|&&f| m.faces[f].vertices.len() == 3);
    let quads = cells.iter().filter(|&&f| m.faces[f].vertices.len() == 4);
    assert_eq!((quads.count(), tris.count()), (240, 16), "{}", a.status);
    // The apex is the curves' own point, shared by all 16 triangles.
    let apex = m
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, v)| (**v - p2).length() < 1e-9)
        .map(|(i, _)| i as u32)
        .collect::<Vec<_>>();
    let curve_points: Vec<u32> = (0..3).flat_map(|f| m.faces[f].vertices.clone()).collect();
    assert!(
        apex.iter().all(|v| curve_points.contains(v)),
        "no copy at the apex"
    );
    let at_apex = cells
        .iter()
        .filter(|&&f| m.faces[f].vertices.iter().any(|v| apex.contains(v)))
        .count();
    assert_eq!(at_apex, 16);
    for &f in &cells {
        assert!(m.normal(f).length() > 0.5, "polygon {f} has no area");
    }
}

/// PC-5 test 3: the 4 x 6 oracle.
#[test]
fn patch_curves_oracle_four_by_six() {
    let (mut a, ctx) = frozen_patch_app();
    let smooth = CurveKind::Curve {
        start_control: false,
        end_control: false,
    };
    let v = DVec3::new;
    let curves = [
        [v(-2., 0., -1.), v(-1.5, 0.3, -1.2), v(-1., 0., -1.)],
        [v(-1., 0., -1.), v(-0.8, 0.3, -0.5), v(-1., 0., 0.)],
        [v(-1., 0., 0.), v(-1.5, 0.3, 0.2), v(-2., 0., 0.)],
        [v(-2., 0., 0.), v(-2.2, 0.3, -0.5), v(-2., 0., -1.)],
    ];
    for c in &curves {
        add_curve(&mut a, c, smooth);
    }
    if let Some(t) = a.draw.as_mut() {
        t.patch.perpendicular = 4;
        t.patch.parallel = 6;
    }
    let before = mesh(&a).vertices.len();
    let mids: Vec<DVec3> = curves.iter().map(|c| c[1]).collect();
    pick(&mut a, &ctx, &[mids[0], mids[1], mids[2], mids[3], mids[0]]);
    let m = mesh(&a);
    assert_eq!(m.faces.len(), 4 + 24, "{}", a.status);
    assert!(m.faces[4..].iter().all(|f| f.vertices.len() == 4));
    assert_eq!(
        m.vertices.len() - before,
        31,
        "5 x 7 grid minus the corners"
    );
    // Corners reused: every corner of the curves is used by the patch.
    for c in &curves {
        let id = (0..before)
            .find(|&i| (m.vertices[i] - c[0]).length() < 1e-12)
            .unwrap() as u32;
        let used = m.faces[4..].iter().any(|f| f.vertices.contains(&id))
            || m.faces[4..].iter().any(|f| {
                f.vertices
                    .iter()
                    .any(|&w| w < before as u32 && (m.vertices[w as usize] - c[0]).length() < 1e-12)
            });
        assert!(used, "corner {:?} not reused", c[0]);
    }
    // Edge points are new, not the curves' middle points.
    for f in &m.faces[4..] {
        for &w in &f.vertices {
            assert!(
                w as usize >= before
                    || curves
                        .iter()
                        .any(|c| (m.vertices[w as usize] - c[0]).length() < 1e-12),
                "patch uses a curve's middle point"
            );
        }
    }
    let centre = v(-1.5, 0.6, -0.5);
    assert!(
        m.vertices[before..]
            .iter()
            .any(|p| (*p - centre).length() < 0.001),
        "no grid point at the Coons centre"
    );
}

/// PC-4 test 4: Parallel divisions run along the first picked curve.
#[test]
fn patch_curves_parallel_runs_along_the_first_pick() {
    for first in [0usize, 1] {
        let (mut a, ctx) = frozen_patch_app();
        let mids = line_loop(
            &mut a,
            &[
                DVec3::new(-1.5, 0., -0.5),
                DVec3::new(1.5, 0., -0.5),
                DVec3::new(1.5, 0., 0.5),
                DVec3::new(-1.5, 0., 0.5),
            ],
        );
        if let Some(t) = a.draw.as_mut() {
            t.patch.perpendicular = 4;
            t.patch.parallel = 6;
        }
        let order: Vec<DVec3> = (0..5).map(|k| mids[(first + k) % 4]).collect();
        pick(&mut a, &ctx, &order);
        let m = mesh(&a);
        assert_eq!(m.faces.len(), 4 + 24, "{}", a.status);
        let used: std::collections::BTreeSet<u32> = m.faces[4..]
            .iter()
            .flat_map(|f| f.vertices.clone())
            .collect();
        let on_first = used
            .iter()
            .filter(|&&w| {
                let p = m.vertices[w as usize];
                if first == 0 {
                    (p.z + 0.5).abs() < 1e-9
                } else {
                    (p.x - 1.5).abs() < 1e-9
                }
            })
            .count();
        assert_eq!(on_first, 7, "first pick {first}: 6 divisions along it");
    }
}

/// PC-1 test 5: the Duplicate tab's Patch Curves button runs Draw Patch.
#[test]
fn patch_curves_button_on_the_duplicate_tab() {
    use crate::form::command_form::Control;
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    fn find(controls: &[Control]) -> Option<&Control> {
        controls.iter().find(|c| c.label == "Patch Curves")
    }
    let control = find(&a.forms.get("toolbox.duplicate").unwrap().controls).expect("button");
    assert_eq!(control.command.as_deref(), Some("Draw Patch"));
    assert_eq!(control.icon.as_deref(), Some("draw-patch"));
    a.execute("Draw Patch");
    assert_eq!(a.draw.as_ref().map(|t| t.kind), Some(DrawKind::Patch));
}

/// PC-7: Freeze is off by default and can be changed; with it off the
/// grid options are greyed.
#[test]
fn patch_curves_panel_greys_the_grid_while_live() {
    let (mut a, _ctx) = app(DrawKind::Patch);
    let form_of = |a: &Operandi| {
        let d = <crate::draw_tools::DrawFields as crate::tool_form::ToolFields>::params(a).unwrap();
        <crate::draw_tools::DrawFields as crate::tool_form::ToolFields>::form(a, &d)
    };
    let greyed = |a: &Operandi, key: &str| {
        form_of(a)
            .fields()
            .into_iter()
            .find(|f| f.row.key == key)
            .unwrap_or_else(|| panic!("{key}"))
            .disabled_tip
            .is_some()
    };
    assert!(
        !a.draw.as_ref().unwrap().patch.freeze,
        "Freeze is off by default"
    );
    assert!(!greyed(&a, "pch_freeze"));
    for k in ["pch_perp", "pch_par", "pch_type1", "pch_type2"] {
        assert!(greyed(&a, k), "{k} greyed while live");
    }
    a.draw.as_mut().unwrap().patch.freeze = true;
    for k in ["pch_perp", "pch_par", "pch_type1", "pch_type2"] {
        assert!(!greyed(&a, k), "{k} live once frozen");
    }
}

/// PC-7: with Freeze off, closing the loop leaves one live patch polygon on
/// the curves' corners, whose ends are joined; it draws 16 x 16 cells and
/// follows a moved corner.
#[test]
fn patch_curves_live_patch_is_one_polygon() {
    let (mut a, ctx) = app(DrawKind::Patch);
    let mids = line_loop(
        &mut a,
        &[
            DVec3::new(-1., 0., -1.),
            DVec3::new(1., 0., -1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(-1., 0., 1.),
        ],
    );
    let undo = a.editor.history.undo.len();
    pick(&mut a, &ctx, &[mids[0], mids[1], mids[2], mids[3], mids[0]]);
    assert_eq!(a.editor.history.undo.len(), undo + 1, "{}", a.status);
    let m = mesh(&a).clone();
    assert_eq!(m.faces.len(), 5);
    assert!(operandi_core::patch_live::is_patch(&m, 4));
    assert_eq!(m.vertices.len(), 4, "each corner's two curve ends joined");
    assert!(m.normal(4).y > 0.9, "faces the viewer");
    let shown = a
        .editor
        .evaluated(operandi_core::evaluate::Purpose::Display { base_edit: None });
    let s = &shown.items[0].mesh;
    let cells = (0..s.faces.len()).filter(|&f| !s.is_polyline(f)).count();
    assert_eq!(cells, 256);
    let before = s.vertices.clone();
    drop(shown);
    // Move a corner: both curves and the surface follow.
    let corner = m.faces[4].vertices[2] as usize;
    a.editor.scene.active_mut().unwrap().mesh.vertices[corner].y += 1.;
    let shown = a.editor.evaluated(operandi_core::evaluate::Purpose::Final);
    let s = &shown.items[0].mesh;
    let moved = before
        .iter()
        .zip(&s.vertices)
        .filter(|(p, q)| (**p - **q).length() > 1e-6)
        .count();
    assert!(moved > 20, "{moved} points moved");
}

/// PC-7: changing an option rebuilds the last patch in place.
#[test]
fn patch_curves_option_change_rebuilds_the_last_patch() {
    let (mut a, ctx) = frozen_patch_app();
    let mids = line_loop(
        &mut a,
        &[
            DVec3::new(-1., 0., -1.),
            DVec3::new(1., 0., -1.),
            DVec3::new(1., 0., 1.),
            DVec3::new(-1., 0., 1.),
        ],
    );
    pick(&mut a, &ctx, &[mids[0], mids[1], mids[2], mids[3], mids[0]]);
    let undo = a.editor.history.undo.len();
    a.draw.as_mut().unwrap().patch.parallel = 3;
    a.rebuild_patch_for_test();
    assert_eq!(a.editor.history.undo.len(), undo);
    assert_eq!(mesh(&a).faces.len(), 4 + 30);
}
