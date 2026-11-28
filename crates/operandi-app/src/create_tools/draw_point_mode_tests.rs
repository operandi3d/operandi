//! SPEC-DRAW-POINT-MODE (Add / Insert / Edit / Delete on the draw tools,
//! Pen and Tube) and SPEC-DRAG-SNAP-POINTS (draw-tool points snap while
//! dragged), through viewport clicks in the Front view (work plane XY).
use super::*;
use crate::draw_tools::{DrawKind, DrawMode};
use operandi_core::curves::{self, CurveKind, bezier_anchors, bezier_points};
use operandi_core::{CurveEdit, PolygonRepresentation};

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
fn button(p: Pos2, pressed: bool, b: PointerButton, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: p,
        pressed,
        modifiers,
        button: b,
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
/// Press at `from`, move in steps to `to`, and leave the button down.
fn press_and_move(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2, m: Modifiers) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(a, ctx, vec![button(from, true, PointerButton::Primary, m)]);
    for s in 1..=4 {
        let p = from + (to - from) * (s as f32 / 4.);
        frame(a, ctx, vec![Event::PointerMoved(p)]);
    }
}
fn release(a: &mut Operandi, ctx: &Context, at: Pos2, m: Modifiers) {
    frame(a, ctx, vec![button(at, false, PointerButton::Primary, m)]);
    frame(a, ctx, vec![]);
}
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2, m: Modifiers) {
    press_and_move(a, ctx, from, to, m);
    release(a, ctx, to, m);
}
fn screen(a: &Operandi, p: DVec3) -> Pos2 {
    a.camera.project(p, a.viewport_rect).unwrap().0
}
/// The world point under screen position `p` on the XY plane.
fn world(a: &Operandi, p: Pos2) -> DVec3 {
    a.work_point(p, a.viewport_rect, 2, DVec3::ZERO).unwrap()
}
/// `px` screen pixels short of `target`, coming from `from`.
fn short_of(a: &Operandi, from: DVec3, target: DVec3, px: f32) -> Pos2 {
    let (s, t) = (screen(a, from), screen(a, target));
    t - (t - s).normalized() * px
}
fn empty_app() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::Mode(Mode::Polygon));
    a.camera.view = View::Front;
    a.category = toolbox::Category::Create;
    let ctx = Context::default();
    theme::install(&ctx);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}
fn add_curve(a: &mut Operandi, points: Vec<DVec3>, closed: bool, kind: CurveKind) -> u32 {
    assert!(
        a.run_ok(Command::SetCurve(Box::new(CurveEdit {
            item: None,
            face: None,
            points,
            closed,
            kind,
            label: "Test curve".into(),
        }))),
        "{}",
        a.status
    );
    a.editor.scene.active().unwrap().mesh.faces.len() as u32 - 1
}
/// Reopen curve `face` with the Draw tool `kind` (CV-20: Edit mode, point
/// 0 current), then switch to `mode`.
fn edit_curve(a: &mut Operandi, ctx: &Context, face: u32, kind: DrawKind, mode: DrawMode) {
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![face];
    a.editor.history.undo.clear();
    a.activate_draw(kind);
    let t = a.draw.as_mut().expect("draw tool on");
    assert_eq!(t.face, Some(face));
    t.mode = mode;
    frame(a, ctx, vec![]);
}
fn curve_points(a: &Operandi, face: u32) -> Vec<DVec3> {
    let m = &a.editor.scene.active().unwrap().mesh;
    m.faces[face as usize]
        .vertices
        .iter()
        .map(|&v| m.vertices[v as usize])
        .collect()
}
fn closed(a: &Operandi, face: u32) -> bool {
    matches!(
        a.editor
            .scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .representations[face as usize],
        PolygonRepresentation::Polyline { closed: true, .. }
    )
}
fn curve_kind() -> CurveKind {
    CurveKind::Curve {
        start_control: false,
        end_control: false,
    }
}
fn four() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(3., 1., 0.),
    ]
}
fn four_point_curve(mode: DrawMode) -> (Operandi, Context, u32) {
    let (mut a, ctx) = empty_app();
    let face = add_curve(&mut a, four(), false, curve_kind());
    edit_curve(&mut a, &ctx, face, DrawKind::Curve, mode);
    (a, ctx, face)
}
fn close(a: DVec3, b: DVec3) -> bool {
    a.abs_diff_eq(b, 1e-6)
}

// ---- Mode: Add / Edit / Delete on Curve (DPM-02..04)

#[test]
fn dpm_t01_add_puts_the_new_point_after_the_current_one() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Add);
    a.draw.as_mut().unwrap().current = Some(1);
    let p = screen(&a, DVec3::new(1.5, 2., 0.));
    click(&mut a, &ctx, p);
    let c = curve_points(&a, face);
    assert_eq!(c.len(), 5, "{}", a.status);
    assert!(close(c[2], DVec3::new(1.5, 2., 0.)), "{c:?}");
    assert_eq!(a.draw.as_ref().unwrap().current, Some(2));
}

#[test]
fn dpm_t02_edit_click_in_empty_space_changes_nothing() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Edit);
    let p = screen(&a, DVec3::new(1.5, 2., 0.));
    click(&mut a, &ctx, p);
    assert_eq!(curve_points(&a, face), four());
    assert_eq!(a.editor.history.undo.len(), 0, "no undo step");
}

#[test]
fn dpm_t03_edit_drag_moves_a_point_in_one_step() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Edit);
    let (from, to) = (screen(&a, four()[1]), screen(&a, DVec3::new(1., 2., 0.)));
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    let c = curve_points(&a, face);
    assert_eq!(c.len(), 4);
    assert!(close(c[1], DVec3::new(1., 2., 0.)), "{c:?} ({})", a.status);
    assert_eq!(a.editor.history.undo.len(), 1);
}

#[test]
fn dpm_t04_delete_click_removes_the_point() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Delete);
    let p = screen(&a, four()[1]);
    click(&mut a, &ctx, p);
    let f = four();
    assert_eq!(
        curve_points(&a, face),
        vec![f[0], f[2], f[3]],
        "{}",
        a.status
    );
}

#[test]
fn dpm_t05_delete_keeps_a_closed_curve_closed() {
    let (mut a, ctx) = empty_app();
    let face = add_curve(&mut a, four(), true, curve_kind());
    edit_curve(&mut a, &ctx, face, DrawKind::Curve, DrawMode::Delete);
    let p = screen(&a, four()[0]);
    click(&mut a, &ctx, p);
    assert_eq!(curve_points(&a, face).len(), 3, "{}", a.status);
    assert!(closed(&a, face));
}

#[test]
fn dpm_t06_delete_click_in_empty_space_does_nothing() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Delete);
    let p = screen(&a, DVec3::new(1.5, 2., 0.));
    click(&mut a, &ctx, p);
    assert_eq!(curve_points(&a, face), four());
}

#[test]
fn dpm_t07_middle_click_deletes_in_any_mode() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Edit);
    let p = screen(&a, four()[2]);
    click_with(&mut a, &ctx, p, PointerButton::Middle, Modifiers::NONE);
    let f = four();
    assert_eq!(curve_points(&a, face), vec![f[0], f[1], f[3]]);
}

#[test]
fn dpm_t08_shift_middle_click_deletes_the_point_and_all_after() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Add);
    let p = screen(&a, four()[1]);
    click_with(&mut a, &ctx, p, PointerButton::Middle, Modifiers::SHIFT);
    // One point left: the curve waits in the tool for a second point.
    assert!(
        a.editor.scene.active().unwrap().mesh.faces.len() <= face as usize
            || curve_points(&a, face).len() <= 1
    );
    let t = a.draw.as_ref().unwrap();
    assert_eq!(t.pending, Some(four()[0]), "{}", a.status);
}

#[test]
fn dpm_t14_undo_brings_a_deleted_point_back_and_keeps_the_tool() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Delete);
    let p = screen(&a, four()[1]);
    click(&mut a, &ctx, p);
    a.run(Command::Undo);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(curve_points(&a, face), four());
    assert!(a.draw.is_some(), "the tool stays on");
}

#[test]
fn dpm_t15_switching_modes_mid_shape_keeps_drawing_after_the_current_point() {
    let (mut a, ctx, face) = four_point_curve(DrawMode::Add);
    a.draw.as_mut().unwrap().current = Some(3);
    for mode in [DrawMode::Edit, DrawMode::Add] {
        a.draw.as_mut().unwrap().mode = mode;
        frame(&mut a, &ctx, vec![]);
    }
    let p = screen(&a, DVec3::new(4., 0., 0.));
    click(&mut a, &ctx, p);
    let c = curve_points(&a, face);
    assert_eq!(c.len(), 5, "{}", a.status);
    assert!(close(c[4], DVec3::new(4., 0., 0.)));
}

#[test]
fn dpm_11_mode_comes_back_as_last_used() {
    let (mut a, ctx) = empty_app();
    a.point_modes.remember(DrawKind::BSpline, DrawMode::Delete);
    a.activate_draw(DrawKind::BSpline);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(a.draw.as_ref().unwrap().mode, DrawMode::Delete);
    a.activate_draw(DrawKind::Curve);
    assert_eq!(a.draw.as_ref().unwrap().mode, DrawMode::Add, "per tool");
}

// ---- Bezier: Insert and Delete (DPM-04/05)

/// A straight Bezier from x = 0 to x = 3 with handles along it.
fn straight_bezier() -> Vec<DVec3> {
    bezier_points(
        &[
            [DVec3::ZERO, DVec3::ZERO, DVec3::new(1., 0., 0.)],
            [
                DVec3::new(2., 0., 0.),
                DVec3::new(3., 0., 0.),
                DVec3::new(3., 0., 0.),
            ],
        ],
        false,
    )
}

#[test]
fn dpm_t09_insert_splits_a_bezier_without_changing_its_shape() {
    let (mut a, ctx) = empty_app();
    let face = add_curve(&mut a, straight_bezier(), false, CurveKind::Bezier);
    edit_curve(&mut a, &ctx, face, DrawKind::Bezier, DrawMode::Insert);
    let before = curves::sample(&curve_points(&a, face), CurveKind::Bezier, false, 400);
    let p = screen(&a, DVec3::new(1.5, 0., 0.)) + vec2(0., 3.);
    click(&mut a, &ctx, p);
    let after_points = curve_points(&a, face);
    assert_eq!(
        bezier_anchors(&after_points, false).len(),
        3,
        "{}",
        a.status
    );
    // Every sample of the old shape lies on the new one.
    let after = curves::sample(&after_points, CurveKind::Bezier, false, 400);
    for p in &before {
        let d = after
            .iter()
            .map(|q| q.distance(*p))
            .fold(f64::INFINITY, f64::min);
        assert!(d < 1e-2, "{p} is {d} off the new curve");
    }
    // Exactly: the split curve's spans cover the old one's (de Casteljau).
    for p in curves::sample(&after_points, CurveKind::Bezier, false, 50) {
        assert!(p.y.abs() < 1e-9 && p.z.abs() < 1e-9, "{p}");
    }
}

#[test]
fn split_bezier_keeps_every_point_of_a_curved_span() {
    use crate::draw_tools::split_bezier;
    let anchors = [
        [DVec3::ZERO, DVec3::ZERO, DVec3::new(1., 2., 0.)],
        [
            DVec3::new(2., 2., 0.),
            DVec3::new(3., 0., 0.),
            DVec3::new(3., 0., 0.),
        ],
    ];
    let mut e: Vec<DVec3> = anchors.iter().flatten().copied().collect();
    let cubic = |c: [DVec3; 4], t: f64| {
        let u = 1. - t;
        c[0] * (u * u * u) + c[1] * (3. * u * u * t) + c[2] * (3. * u * t * t) + c[3] * (t * t * t)
    };
    let original = [e[1], e[2], e[3], e[4]];
    let at = split_bezier(&mut e, 0, 0.3);
    assert_eq!(at, 1);
    assert_eq!(e.len(), 9);
    for s in 0..=20 {
        let t = s as f64 / 20.;
        let left = cubic([e[1], e[2], e[3], e[4]], t);
        assert!(left.abs_diff_eq(cubic(original, 0.3 * t), 1e-12));
        let right = cubic([e[4], e[5], e[6], e[7]], t);
        assert!(right.abs_diff_eq(cubic(original, 0.3 + 0.7 * t), 1e-12));
    }
}

#[test]
fn dpm_t10_deleting_a_bezier_anchor_takes_its_handles() {
    let (mut a, ctx) = empty_app();
    let anchors = [
        [DVec3::ZERO, DVec3::ZERO, DVec3::new(0.5, 0.5, 0.)],
        [
            DVec3::new(1.5, 1.5, 0.),
            DVec3::new(2., 1., 0.),
            DVec3::new(2.5, 0.5, 0.),
        ],
        [
            DVec3::new(3.5, 0.5, 0.),
            DVec3::new(4., 0., 0.),
            DVec3::new(4., 0., 0.),
        ],
    ];
    let face = add_curve(
        &mut a,
        bezier_points(&anchors, false),
        false,
        CurveKind::Bezier,
    );
    let stored = curve_points(&a, face).len();
    edit_curve(&mut a, &ctx, face, DrawKind::Bezier, DrawMode::Delete);
    let p = screen(&a, DVec3::new(2., 1., 0.));
    click(&mut a, &ctx, p);
    assert_eq!(curve_points(&a, face).len(), stored - 3, "{}", a.status);
}

// ---- Pen (DPM-10, D1 and D2)

fn pen(points: &[DVec3], quads: bool) -> (Operandi, Context) {
    let (mut a, ctx) = empty_app();
    a.start_interactive_pen();
    assert!(a.editor.session.pen_draft.is_some(), "{}", a.status);
    if quads {
        let mut o = a.editor.session.pen_draft.as_ref().unwrap().options();
        o.make_quads = true;
        assert!(a.run_ok(Command::PenOptions(o)), "{}", a.status);
    }
    for &p in points {
        assert!(a.run_ok(Command::PenCapture(p)), "{}", a.status);
    }
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}
fn pen_points(a: &Operandi) -> Vec<DVec3> {
    a.editor
        .session
        .pen_draft
        .as_ref()
        .unwrap()
        .points()
        .to_vec()
}

#[test]
fn dpm_t12_pen_delete_mode_removes_a_point_and_add_goes_after_current() {
    let (mut a, ctx) = pen(&four(), false);
    a.point_modes.pen = DrawMode::Delete;
    let p = screen(&a, four()[2]);
    click(&mut a, &ctx, p);
    let f = four();
    assert_eq!(pen_points(&a), vec![f[0], f[1], f[3]], "{}", a.status);
    assert_eq!(
        a.editor.session.pen_draft.as_ref().unwrap().current(),
        Some(1)
    );
    a.point_modes.pen = DrawMode::Add;
    let q = DVec3::new(1.5, -1., 0.);
    let s = screen(&a, q);
    click(&mut a, &ctx, s);
    let now = pen_points(&a);
    assert_eq!(now.len(), 4);
    assert!(close(now[2], q), "{now:?}");
}

#[test]
fn dpm_t13_pen_three_points_to_two_then_drop_leaves_two_points() {
    let (mut a, ctx) = pen(&four()[..3], false);
    a.point_modes.pen = DrawMode::Delete;
    let p = screen(&a, four()[1]);
    click(&mut a, &ctx, p);
    assert_eq!(pen_points(&a).len(), 2);
    let points = a.editor.scene.active().unwrap().mesh.vertices.len();
    assert!(a.run_ok(Command::Drop), "{}", a.status);
    // PEN-05: a two-point polygon drops as its two points.
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.vertices.len(),
        points + 2
    );
}

#[test]
fn pen_middle_click_deletes_and_undo_puts_it_back() {
    let (mut a, ctx) = pen(&four(), false);
    let p = screen(&a, four()[1]);
    click_with(&mut a, &ctx, p, PointerButton::Middle, Modifiers::NONE);
    assert_eq!(pen_points(&a).len(), 3, "{}", a.status);
    a.run(Command::Undo);
    assert_eq!(pen_points(&a), four(), "back at its index");
    let p = screen(&a, four()[1]);
    click_with(&mut a, &ctx, p, PointerButton::Middle, Modifiers::SHIFT);
    assert_eq!(
        pen_points(&a),
        vec![four()[0]],
        "Shift: and every later one"
    );
}

#[test]
fn pen_edit_mode_adds_nothing_and_drag_is_one_undo_step() {
    let (mut a, ctx) = pen(&four(), false);
    a.point_modes.pen = DrawMode::Edit;
    let p = screen(&a, DVec3::new(1.5, 3., 0.));
    click(&mut a, &ctx, p);
    assert_eq!(pen_points(&a), four());
    let (from, to) = (screen(&a, four()[1]), screen(&a, DVec3::new(1., 2., 0.)));
    drag(&mut a, &ctx, from, to, Modifiers::NONE);
    assert!(close(pen_points(&a)[1], DVec3::new(1., 2., 0.)));
    a.run(Command::Undo);
    assert_eq!(pen_points(&a), four(), "the whole drag undone at once");
}

#[test]
fn d2_deleting_a_make_quads_point_removes_its_pair() {
    let (mut a, ctx) = pen(
        &[
            DVec3::new(0., 0., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
        ],
        true,
    );
    // Starting edge + two clicks: 2 + 2 + 2 points.
    let all = pen_points(&a);
    assert_eq!(all.len(), 6, "{}", a.status);
    a.point_modes.pen = DrawMode::Delete;
    let p = screen(&a, all[2]);
    click(&mut a, &ctx, p);
    let now = pen_points(&a);
    assert_eq!(now, vec![all[0], all[1], all[4], all[5]], "{}", a.status);
}

// ---- Tube (DPM-08)

fn tube(points: Vec<DVec3>) -> (Operandi, Context) {
    let (mut a, ctx) = empty_app();
    a.arm_shape(operandi_core::ShapeKind::Tube);
    let mut p = a.editor.session.shape.as_ref().unwrap().requested.clone();
    if let operandi_core::ShapeParameters::Tube(t) = &mut p {
        t.points = points;
    }
    assert!(a.run_ok(Command::ConfigureShape(p)), "{}", a.status);
    frame(&mut a, &ctx, vec![]);
    (a, ctx)
}
fn tube_params(a: &Operandi) -> operandi_core::TubeParameters {
    match &a.editor.session.shape.as_ref().unwrap().requested {
        operandi_core::ShapeParameters::Tube(t) => t.clone(),
        _ => panic!("not a tube"),
    }
}

#[test]
fn dpm_t11_tube_delete_mode_rebuilds_along_the_rest() {
    let pts = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 0., 0.),
    ];
    let (mut a, ctx) = tube(pts.clone());
    let (sides, radius) = (tube_params(&a).sides, tube_params(&a).radius);
    a.point_modes.tube = DrawMode::Delete;
    let p = screen(&a, pts[1]);
    click(&mut a, &ctx, p);
    let t = tube_params(&a);
    assert_eq!(t.points, vec![pts[0], pts[2]], "{}", a.status);
    assert_eq!((t.sides, t.radius), (sides, radius));
    a.run(Command::Undo);
    assert_eq!(tube_params(&a).points, pts, "one undo step");
}

#[test]
fn tube_add_goes_after_the_current_point() {
    let pts = vec![DVec3::new(0., 0., 0.), DVec3::new(2., 0., 0.)];
    let (mut a, ctx) = tube(pts.clone());
    // Pick point 0, then click between the two.
    let p = screen(&a, pts[0]);
    click(&mut a, &ctx, p);
    let q = DVec3::new(1., 1., 0.);
    let s = screen(&a, q);
    click(&mut a, &ctx, s);
    let t = tube_params(&a).points;
    assert_eq!(t.len(), 3, "{}", a.status);
    assert!(close(t[1], q), "{t:?}");
}

// ---- Snapping while dragging (SPEC-DRAG-SNAP-POINTS)

/// A loose two-point curve with an end at (3,1,0) to snap to, and the
/// curve being edited: (0,0,0) (1,1,0).
fn snap_rig(snapping: bool) -> (Operandi, Context, u32) {
    let (mut a, ctx) = empty_app();
    add_curve(
        &mut a,
        vec![DVec3::new(3., 1., 0.), DVec3::new(3., 3., 0.)],
        false,
        curve_kind(),
    );
    let face = add_curve(
        &mut a,
        vec![DVec3::ZERO, DVec3::new(1., 1., 0.)],
        false,
        curve_kind(),
    );
    a.snapping.enabled = snapping;
    edit_curve(&mut a, &ctx, face, DrawKind::Curve, DrawMode::Edit);
    (a, ctx, face)
}
const TARGET: DVec3 = DVec3::new(3., 1., 0.);
const B: DVec3 = DVec3::new(1., 1., 0.);
fn draft_point(a: &Operandi, i: usize) -> DVec3 {
    a.draw.as_ref().unwrap().draft.as_ref().expect("dragging")[i]
}

#[test]
fn dsn_t01_t02_point_shows_and_lands_on_the_target_inside_inner_range() {
    let (mut a, ctx, face) = snap_rig(true);
    let from = screen(&a, B);
    let to = short_of(&a, B, TARGET, 10.);
    press_and_move(&mut a, &ctx, from, to, Modifiers::NONE);
    assert_eq!(draft_point(&a, 1), TARGET, "shown snapped mid-drag");
    assert_eq!(a.snap_marker, Some((TARGET, true)));
    let end = short_of(&a, B, TARGET, 15.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(end)]);
    release(&mut a, &ctx, end, Modifiers::NONE);
    assert_eq!(curve_points(&a, face)[1], TARGET, "stored exactly");
    // DSN-T12: one drag, one undo step.
    a.run(Command::Undo);
    assert_eq!(curve_points(&a, face)[1], B);
}

#[test]
fn dsn_t03_t04_outside_inner_range_only_pre_highlights() {
    let (mut a, ctx, face) = snap_rig(true);
    let from = screen(&a, B);
    let between = short_of(&a, B, TARGET, 32.);
    press_and_move(&mut a, &ctx, from, between, Modifiers::NONE);
    assert_eq!(a.snap_marker, Some((TARGET, false)), "pre-highlighted");
    assert_ne!(draft_point(&a, 1), TARGET);
    let far = short_of(&a, B, TARGET, 60.);
    frame(&mut a, &ctx, vec![Event::PointerMoved(far)]);
    release(&mut a, &ctx, far, Modifiers::NONE);
    let p = curve_points(&a, face)[1];
    assert!(close(p, world(&a, far)), "follows the pointer: {p}");
}

fn x_key(pressed: bool) -> Event {
    Event::Key {
        key: Key::X,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn dsn_t05_holding_x_turns_snapping_on_for_one_drag() {
    let (mut a, ctx, face) = snap_rig(false);
    let from = screen(&a, B);
    frame(&mut a, &ctx, vec![Event::PointerMoved(from), x_key(true)]);
    let end = short_of(&a, B, TARGET, 15.);
    press_and_move(&mut a, &ctx, from, end, Modifiers::NONE);
    release(&mut a, &ctx, end, Modifiers::NONE);
    frame(&mut a, &ctx, vec![x_key(false)]);
    frame(&mut a, &ctx, vec![]);
    assert_eq!(curve_points(&a, face)[1], TARGET, "{}", a.status);
    assert!(!a.snapping.enabled, "off again after the key");
}

#[test]
fn dsn_t06_holding_x_turns_snapping_off_for_one_drag() {
    let (mut a, ctx, face) = snap_rig(true);
    let from = screen(&a, B);
    frame(&mut a, &ctx, vec![Event::PointerMoved(from), x_key(true)]);
    let end = short_of(&a, B, TARGET, 15.);
    press_and_move(&mut a, &ctx, from, end, Modifiers::NONE);
    release(&mut a, &ctx, end, Modifiers::NONE);
    frame(&mut a, &ctx, vec![x_key(false)]);
    frame(&mut a, &ctx, vec![]);
    assert_ne!(curve_points(&a, face)[1], TARGET);
    assert!(a.snapping.enabled, "on again after the key");
}

#[test]
fn dsn_t09_bezier_handle_end_snaps_and_the_anchor_stays() {
    let (mut a, ctx) = empty_app();
    add_curve(
        &mut a,
        vec![TARGET, DVec3::new(3., 3., 0.)],
        false,
        curve_kind(),
    );
    let anchors = [
        [DVec3::ZERO, DVec3::ZERO, DVec3::new(1., 0., 0.)],
        [
            DVec3::new(1., -1., 0.),
            DVec3::new(2., -1., 0.),
            DVec3::new(2., -1., 0.),
        ],
    ];
    let face = add_curve(
        &mut a,
        bezier_points(&anchors, false),
        false,
        CurveKind::Bezier,
    );
    a.snapping.enabled = true;
    edit_curve(&mut a, &ctx, face, DrawKind::Bezier, DrawMode::Edit);
    let handle = DVec3::new(1., 0., 0.);
    let end = short_of(&a, handle, TARGET, 10.);
    let start = screen(&a, handle);
    drag(&mut a, &ctx, start, end, Modifiers::NONE);
    let after = bezier_anchors(&curve_points(&a, face), false);
    assert_eq!(
        after[0][2], TARGET,
        "handle end on the target ({})",
        a.status
    );
    assert_eq!(after[0][1], DVec3::ZERO, "anchor stays");
}

#[test]
fn dsn_t10_ctrl_drag_snaps_along_the_constrained_axis_only() {
    let (mut a, ctx) = empty_app();
    let target = DVec3::new(3., 1.2, 0.);
    add_curve(
        &mut a,
        vec![target, DVec3::new(3., 3., 0.)],
        false,
        curve_kind(),
    );
    let face = add_curve(&mut a, vec![DVec3::new(0., 0., 0.), B], false, curve_kind());
    a.snapping.enabled = true;
    edit_curve(&mut a, &ctx, face, DrawKind::Curve, DrawMode::Edit);
    let end = screen(&a, DVec3::new(2.9, 1., 0.));
    let start = screen(&a, B);
    drag(&mut a, &ctx, start, end, Modifiers::CTRL);
    let p = curve_points(&a, face)[1];
    assert!(close(p, DVec3::new(3., 1., 0.)), "{p} ({})", a.status);
}

#[test]
fn dsn_t11_own_points_are_targets() {
    let (mut a, ctx) = empty_app();
    let pts = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(4., 0., 0.),
    ];
    let face = add_curve(&mut a, pts.clone(), false, curve_kind());
    a.snapping.enabled = true;
    edit_curve(&mut a, &ctx, face, DrawKind::Curve, DrawMode::Edit);
    let end = short_of(&a, pts[1], pts[0], 10.);
    let start = screen(&a, pts[1]);
    drag(&mut a, &ctx, start, end, Modifiers::NONE);
    assert_eq!(curve_points(&a, face)[1], pts[0], "{}", a.status);
}

#[test]
fn dsn_grid_target_lands_on_a_grid_crossing() {
    let (mut a, ctx, face) = snap_rig(true);
    a.snapping.global.vertex = false;
    a.snapping.global.grid = true;
    let end = screen(&a, DVec3::new(0.63, 0.41, 0.));
    let start = screen(&a, B);
    drag(&mut a, &ctx, start, end, Modifiers::NONE);
    let p = curve_points(&a, face)[1];
    let r = a.viewport_rect;
    let step = crate::grid::spacing(&a.camera, r, 2, a.workplane_center());
    for axis in [0, 1] {
        let k = p[axis] / step;
        assert!((k - k.round()).abs() < 1e-9, "{p} on a {step} grid");
    }
}

#[test]
fn dsn_pen_point_snaps_while_dragged() {
    let (mut a, ctx) = empty_app();
    // Something to snap to: a mesh curve end.
    add_curve(
        &mut a,
        vec![DVec3::new(3., 3., 0.), DVec3::new(4., 3., 0.)],
        false,
        curve_kind(),
    );
    a.start_interactive_pen();
    for &p in &four()[..3] {
        assert!(a.run_ok(Command::PenCapture(p)), "{}", a.status);
    }
    a.snapping.enabled = true;
    frame(&mut a, &ctx, vec![]);
    let target = DVec3::new(3., 3., 0.);
    let end = short_of(&a, four()[1], target, 10.);
    let start = screen(&a, four()[1]);
    drag(&mut a, &ctx, start, end, Modifiers::NONE);
    assert_eq!(pen_points(&a)[1], target, "{}", a.status);
}

/// Press at `from`, move to each of `path`, release at the last.
fn drag_through(a: &mut Operandi, ctx: &Context, from: Pos2, path: &[Pos2]) {
    frame(a, ctx, vec![Event::PointerMoved(from)]);
    frame(
        a,
        ctx,
        vec![button(from, true, PointerButton::Primary, Modifiers::NONE)],
    );
    for &p in path {
        frame(a, ctx, vec![Event::PointerMoved(p)]);
    }
    let last = *path.last().unwrap();
    release(a, ctx, last, Modifiers::NONE);
}

/// A Pen point dragged onto a mesh point stays on it while the pointer
/// keeps moving within Inner Range (the point's own place, now the
/// target's, must not hide the target on the next frame).
#[test]
fn dsn_pen_point_stays_snapped_while_the_pointer_moves_within_range() {
    let (mut a, ctx) = empty_app();
    let target = DVec3::new(3., 3., 0.);
    add_curve(
        &mut a,
        vec![target, DVec3::new(4., 3., 0.)],
        false,
        curve_kind(),
    );
    a.start_interactive_pen();
    for &p in &four()[..3] {
        assert!(a.run_ok(Command::PenCapture(p)), "{}", a.status);
    }
    a.snapping.enabled = true;
    frame(&mut a, &ctx, vec![]);
    let start = screen(&a, four()[1]);
    let near = short_of(&a, four()[1], target, 10.);
    let nearer = short_of(&a, four()[1], target, 7.) + vec2(0., 2.);
    drag_through(&mut a, &ctx, start, &[near, nearer, nearer + vec2(1., 0.)]);
    assert_eq!(pen_points(&a)[1], target, "{}", a.status);
}

/// The same for a Tube path point.
#[test]
fn dsn_tube_point_stays_snapped_while_the_pointer_moves_within_range() {
    let pts = vec![DVec3::new(0., 0., 0.), DVec3::new(1., 1., 0.)];
    let target = DVec3::new(3., 3., 0.);
    let (mut a, ctx) = empty_app();
    add_curve(
        &mut a,
        vec![target, DVec3::new(4., 3., 0.)],
        false,
        curve_kind(),
    );
    a.arm_shape(operandi_core::ShapeKind::Tube);
    let mut p = a.editor.session.shape.as_ref().unwrap().requested.clone();
    if let operandi_core::ShapeParameters::Tube(t) = &mut p {
        t.points = pts.clone();
    }
    assert!(a.run_ok(Command::ConfigureShape(p)), "{}", a.status);
    a.snapping.enabled = true;
    frame(&mut a, &ctx, vec![]);
    let start = screen(&a, pts[1]);
    let near = short_of(&a, pts[1], target, 10.);
    let nearer = short_of(&a, pts[1], target, 7.) + vec2(0., 2.);
    drag_through(&mut a, &ctx, start, &[near, nearer, nearer + vec2(1., 0.)]);
    assert_eq!(tube_params(&a).points[1], target, "{}", a.status);
}
