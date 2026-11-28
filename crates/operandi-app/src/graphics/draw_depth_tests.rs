//! A point placed in a view without perspective keeps the depth of the
//! point it extends (`Operandi::placement_center`): a Solid Sketch leg
//! drawn in Front and continued in Left stays off the centre line, and so
//! does a curve. Perspective and the first point keep the work plane.
use super::*;
use crate::draw_tools::{DrawKind, DrawMode};
use operandi_core::CurveEdit;
use operandi_core::SketchEdit;
use operandi_core::curves::CurveKind;
use operandi_core::solid_sketch::SketchOptions;

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
fn click(a: &mut Operandi, ctx: &Context, p: Pos2) {
    let b = |pressed| Event::PointerButton {
        pos: p,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Primary,
    };
    frame(a, ctx, vec![Event::PointerMoved(p)]);
    frame(a, ctx, vec![b(true)]);
    frame(a, ctx, vec![b(false)]);
    frame(a, ctx, vec![]);
}
fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}
fn app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.camera.view = View::Front;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}
fn set_view(a: &mut Operandi, ctx: &Context, view: View) {
    a.camera.view = view;
    frame(a, ctx, vec![]);
}

/// A sketch with a hip at the origin and a heel off the centre line, the
/// heel current, as a leg drawn in the Front view leaves it.
fn sketch_leg(a: &mut Operandi, ctx: &Context, heel: DVec3) {
    assert!(a.run_ok(Command::SolidSketchStart {
        options: SketchOptions::ss_default(),
        resume: false,
    }));
    for at in [DVec3::ZERO, heel] {
        assert!(
            a.run_ok(Command::SolidSketch {
                edit: SketchEdit::Add {
                    at,
                    size: Some(0.1),
                    up: Some(DVec3::Y),
                },
                continuing: false,
            }),
            "{}",
            a.status
        );
    }
    frame(a, ctx, vec![]);
}
fn centres(a: &Operandi) -> Vec<DVec3> {
    a.editor
        .session
        .solid_sketch
        .as_ref()
        .unwrap()
        .world_centres()
}
fn near(a: DVec3, b: DVec3) -> bool {
    a.abs_diff_eq(b, 1e-6)
}

#[test]
fn sketch_node_clicked_in_left_view_keeps_the_heel_x() {
    let (mut a, ctx) = app();
    let heel = DVec3::new(0.3, -1., 0.);
    sketch_leg(&mut a, &ctx, heel);
    set_view(&mut a, &ctx, View::Left);
    // The toe: forward of the heel in Z, same height. X is the view's depth.
    let toe = DVec3::new(0.3, -1., 0.5);
    let p = screen(&a, toe);
    click(&mut a, &ctx, p);
    let c = centres(&a);
    assert_eq!(c.len(), 3, "{}", a.status);
    assert!(near(c[2], toe), "toe at {:?}, wanted {toe:?}", c[2]);
    let s = a.editor.session.solid_sketch.as_ref().unwrap();
    assert_eq!(
        s.sketch().nodes[2].parent,
        Some(1),
        "the toe extends the heel"
    );
}

#[test]
fn every_axis_view_keeps_the_depth_of_the_current_node() {
    // (view, a node off every axis, where the next click goes).
    let heel = DVec3::new(0.3, -0.7, 0.4);
    let cases = [
        (View::Left, DVec3::new(0.3, -1., 0.9)),
        (View::Right, DVec3::new(0.3, -1., 0.9)),
        (View::Front, DVec3::new(0.8, -1., 0.4)),
        (View::Back, DVec3::new(0.8, -1., 0.4)),
        (View::Top, DVec3::new(0.8, -0.7, 0.9)),
        (View::Bottom, DVec3::new(0.8, -0.7, 0.9)),
    ];
    for (view, want) in cases {
        let (mut a, ctx) = app();
        sketch_leg(&mut a, &ctx, heel);
        set_view(&mut a, &ctx, view);
        let p = screen(&a, want);
        click(&mut a, &ctx, p);
        let c = centres(&a);
        assert_eq!(c.len(), 3, "{view:?}: {}", a.status);
        assert!(near(c[2], want), "{view:?}: {:?}, wanted {want:?}", c[2]);
    }
}

#[test]
fn symmetric_leg_continued_in_left_view_mirrors_its_toe() {
    let (mut a, ctx) = app();
    a.run(Command::Symmetry(operandi_core::Symmetry::x()));
    let heel = DVec3::new(0.3, -1., 0.);
    sketch_leg(&mut a, &ctx, heel);
    // The heel has a twin on the other leg.
    assert_eq!(centres(&a).len(), 3, "{}", a.status);
    set_view(&mut a, &ctx, View::Left);
    let toe = DVec3::new(0.3, -1., 0.5);
    let p = screen(&a, toe);
    click(&mut a, &ctx, p);
    let c = centres(&a);
    assert_eq!(c.len(), 5, "toe and its twin: {}", a.status);
    // The seam faces the Left view, so its screen snap must not pull the
    // toe onto X = 0.
    assert!(c.iter().any(|&q| near(q, toe)), "no toe at {toe:?}: {c:?}");
    let mirror = DVec3::new(-0.3, -1., 0.5);
    assert!(
        c.iter().any(|&q| near(q, mirror)),
        "no mirrored toe at {mirror:?}: {c:?}"
    );
}

#[test]
fn perspective_and_first_node_stay_on_the_work_plane() {
    // First node in Left: nothing to extend, so the work plane (X = 0).
    let (mut a, ctx) = app();
    assert!(a.run_ok(Command::SolidSketchStart {
        options: SketchOptions::ss_default(),
        resume: false,
    }));
    set_view(&mut a, &ctx, View::Left);
    let want = DVec3::new(0., -1., 0.5);
    let p = screen(&a, want);
    click(&mut a, &ctx, p);
    let c = centres(&a);
    assert_eq!(c.len(), 1, "{}", a.status);
    assert!(near(c[0], want), "{:?}", c[0]);

    // Perspective keeps the work plane through the grid.
    let (mut a, ctx) = app();
    sketch_leg(&mut a, &ctx, DVec3::new(0.3, 0.6, 0.4));
    set_view(&mut a, &ctx, View::Perspective);
    let plane = a.actual_plane();
    let through = a.placement_center(Some(DVec3::new(0.3, 0.6, 0.4)));
    assert!(
        near(through, a.workplane_center()),
        "perspective plane {plane} through {through:?}"
    );
}

#[test]
fn fixed_work_plane_wins_over_the_previous_depth() {
    let (mut a, ctx) = app();
    a.editor.scene.workplane.fixed = true;
    a.editor.scene.workplane.plane = 1;
    set_view(&mut a, &ctx, View::Left);
    assert!(near(
        a.placement_center(Some(DVec3::new(0.3, 0., 0.))),
        a.workplane_center()
    ));
}

#[test]
fn curve_point_clicked_in_left_view_keeps_the_current_point_x() {
    let (mut a, ctx) = app();
    a.run(Command::Mode(Mode::Polygon));
    a.category = toolbox::Category::Create;
    let points = vec![DVec3::new(0.4, 0., 0.), DVec3::new(0.4, 1., 0.)];
    assert!(a.run_ok(Command::SetCurve(Box::new(CurveEdit {
        item: None,
        face: None,
        points,
        closed: false,
        kind: CurveKind::Curve {
            start_control: false,
            end_control: false,
        },
        label: "Test curve".into(),
    }))));
    let face = a.editor.scene.active().unwrap().mesh.faces.len() as u32 - 1;
    a.editor.scene.selection.faces = vec![face];
    a.activate_draw(DrawKind::Curve);
    {
        let t = a.draw.as_mut().unwrap();
        t.mode = DrawMode::Add;
        t.current = Some(1);
    }
    set_view(&mut a, &ctx, View::Left);
    let want = DVec3::new(0.4, 1., 1.);
    let p = screen(&a, want);
    click(&mut a, &ctx, p);
    let m = &a.editor.scene.active().unwrap().mesh;
    let c: Vec<DVec3> = m.faces[face as usize]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect();
    assert_eq!(c.len(), 3, "{}", a.status);
    assert!(near(c[2], want), "{c:?}");
}
