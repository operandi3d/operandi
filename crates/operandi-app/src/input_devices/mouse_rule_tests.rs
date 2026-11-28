//! MN-03: mouse rule gaps. Primitive middle-drag copies (MN-03.1), a
//! transform tool's right-drag setting up the falloff (MN-03.2), the
//! transform rule for Bevel under the Screen falloff (MN-03.3) and the
//! Airbrush's Shift / Ctrl strokes (MN-03.4).
use super::tests::{box_app, draw};
use super::*;
use crate::viewport::falloff_screen_view::AirbrushStroke;
use operandi_core::{AirbrushFalloff, LinearFalloff, ScreenFalloff};

const SIZE: Vec2 = vec2(1440., 900.);

fn drag(
    a: &mut Operandi,
    ctx: &Context,
    from: Pos2,
    to: Pos2,
    button: PointerButton,
    modifiers: Modifiers,
) {
    let press = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers,
        button,
    };
    let held = |a: &mut Operandi, events| super::tests::draw_held(a, ctx, SIZE, events, modifiers);
    held(a, vec![Event::PointerMoved(from)]);
    held(a, vec![press(from, true)]);
    for step in 1..=6 {
        let t = step as f32 / 6.;
        held(a, vec![Event::PointerMoved(from + (to - from) * t)]);
    }
    held(a, vec![press(to, false)]);
    draw(a, ctx, SIZE, vec![]);
}

fn setup() -> (Operandi, Context) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = box_app();
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

// ---- MN-03.1

#[test]
fn middle_drag_copies_the_drawn_cube_and_adjusts_the_copy() {
    let (mut a, ctx) = setup();
    a.activate(Tool::Box);
    let first = Parameters {
        position: DVec3::new(3., 0., 0.),
        size: DVec3::new(1., 0.5, 2.),
        segments: [2, 3, 1],
        ..a.editor.session.live.as_ref().unwrap().params.clone()
    };
    a.run(Command::Preview(first.clone()));
    draw(&mut a, &ctx, SIZE, vec![]);
    let vertices = a.editor.scene.active().unwrap().mesh.vertices.len();
    let undo = a.editor.history.undo.len();
    let at = a.viewport_rect.center() + vec2(-150., 60.);
    // A middle click that doesn't move: a copy at the pointer.
    drag(&mut a, &ctx, at, at, PointerButton::Middle, Modifiers::NONE);
    let live = a.editor.session.live.as_ref().expect("the copy is live");
    assert_eq!(live.tool, Tool::Box);
    assert!(live.applied, "{}", a.status);
    assert_eq!(live.params.size, first.size, "same size");
    assert_eq!(live.params.segments, first.segments, "same segments");
    assert_ne!(
        live.params.position, first.position,
        "placed at the pointer"
    );
    assert_eq!(
        a.editor.history.undo.len(),
        undo + 1,
        "the first cube was dropped"
    );
    assert!(a.editor.scene.active().unwrap().mesh.vertices.len() > vertices);
    // A middle drag adjusts the copy it makes.
    let before = a.editor.session.live.as_ref().unwrap().params.clone();
    let at = a.viewport_rect.center() + vec2(150., -60.);
    drag(
        &mut a,
        &ctx,
        at,
        at + vec2(60., 0.),
        PointerButton::Middle,
        Modifiers::NONE,
    );
    let after = &a.editor.session.live.as_ref().unwrap().params;
    assert_ne!(after.position, before.position);
    assert_ne!(
        (after.size, after.position),
        (before.size, before.position),
        "{}",
        a.status
    );
}

#[test]
fn middle_drag_copies_a_sphere_with_its_radius() {
    let (mut a, ctx) = setup();
    a.activate(Tool::Sphere);
    let mut first = a.editor.session.live.as_ref().unwrap().params.clone();
    first.position = DVec3::new(-3., 0., 0.);
    first.radial.as_mut().unwrap().radius = DVec3::splat(0.75);
    a.run(Command::Preview(first.clone()));
    draw(&mut a, &ctx, SIZE, vec![]);
    let at = a.viewport_rect.center() + vec2(100., 100.);
    drag(&mut a, &ctx, at, at, PointerButton::Middle, Modifiers::NONE);
    let live = a.editor.session.live.as_ref().unwrap();
    assert_eq!(live.tool, Tool::Sphere, "{}", a.status);
    assert_eq!(live.params.radial.unwrap().radius, DVec3::splat(0.75));
    assert_ne!(live.params.position, first.position);
}

#[test]
fn middle_drag_before_drawing_explains_itself() {
    let (mut a, ctx) = setup();
    a.activate(Tool::Box);
    let at = a.viewport_rect.center();
    drag(&mut a, &ctx, at, at, PointerButton::Middle, Modifiers::NONE);
    assert!(a.status.contains("first"), "{}", a.status);
}

// ---- MN-03.2

fn moving() -> (Operandi, Context) {
    let (mut a, ctx) = setup();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate(Tool::Move);
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

#[test]
fn transform_right_drag_sets_up_a_linear_falloff() {
    let (mut a, ctx) = moving();
    let base = LinearFalloff::default();
    a.run(Command::SpatialFalloff(Some(Falloff::Linear(base))));
    let c = a.viewport_rect.center();
    drag(
        &mut a,
        &ctx,
        c,
        c + vec2(200., 0.),
        PointerButton::Secondary,
        Modifiers::NONE,
    );
    let Some(Falloff::Linear(f)) = a.editor.spatial_falloff else {
        panic!("{:?}", a.editor.spatial_falloff)
    };
    assert_ne!((f.start, f.end), (base.start, base.end), "{}", a.status);
    assert!(f.start.distance(f.end) > 0.1);
    assert_eq!(
        a.editor.session.live.as_ref().map(|l| l.tool),
        Some(Tool::Move)
    );
}

#[test]
fn transform_right_drag_sizes_a_screen_falloff() {
    let (mut a, ctx) = moving();
    a.run(Command::SpatialFalloff(Some(Falloff::Screen(
        ScreenFalloff::default(),
    ))));
    let c = a.viewport_rect.center();
    drag(
        &mut a,
        &ctx,
        c,
        c + vec2(150., 0.),
        PointerButton::Secondary,
        Modifiers::NONE,
    );
    let Some(Falloff::Screen(s)) = a.editor.spatial_falloff else {
        panic!()
    };
    assert!(s.size > 100., "{s:?} {}", a.status);
}

#[test]
fn without_a_falloff_right_drag_is_unchanged() {
    let (mut a, ctx) = moving();
    let scene = a.editor.scene.clone();
    let c = a.viewport_rect.center();
    drag(
        &mut a,
        &ctx,
        c,
        c + vec2(150., 0.),
        PointerButton::Secondary,
        Modifiers::NONE,
    );
    assert_eq!(a.editor.spatial_falloff, None);
    assert_eq!(a.editor.scene.items, scene.items);
}

// ---- MN-03.3

fn bevel_with(screen: bool) -> (Operandi, Context) {
    let (mut a, ctx) = setup();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    if screen {
        let r = a.viewport_rect;
        a.run(Command::SpatialFalloff(Some(Falloff::Screen(
            ScreenFalloff {
                center: glam::DVec2::new(r.width() as f64 / 2., r.height() as f64 / 2.),
                size: 2000.,
                transparent: true,
            },
        ))));
        a.restamp_screen_falloffs();
    }
    a.activate(Tool::Bevel);
    draw(&mut a, &ctx, SIZE, vec![]);
    (a, ctx)
}

fn faces(a: &Operandi) -> usize {
    a.editor.scene.active().unwrap().mesh.faces.len()
}

/// Three drags off the handles: plain, plain, Shift. The polygon count
/// after each.
fn three_drags(a: &mut Operandi, ctx: &Context) -> [usize; 3] {
    let corner = a.viewport_rect.left_top() + vec2(80., 80.);
    let mut out = [0; 3];
    for (i, m) in [Modifiers::NONE, Modifiers::NONE, Modifiers::SHIFT]
        .into_iter()
        .enumerate()
    {
        drag(
            a,
            ctx,
            corner,
            corner + vec2(0., -40.),
            PointerButton::Primary,
            m,
        );
        out[i] = faces(a);
    }
    out
}

#[test]
fn bevel_under_the_screen_falloff_starts_fresh_on_a_plain_drag() {
    let (mut a, ctx) = bevel_with(true);
    let start = faces(&a);
    let [one, two, three] = three_drags(&mut a, &ctx);
    assert!(one > start, "the first drag bevels: {}", a.status);
    assert!(two > one, "a plain drag starts a fresh bevel");
    assert_eq!(three, two, "Shift keeps adjusting the same one");
}

#[test]
fn bevel_without_it_keeps_todays_rule() {
    let (mut a, ctx) = bevel_with(false);
    let [one, two, _] = three_drags(&mut a, &ctx);
    assert_eq!(two, one, "a plain re-drag adjusts the same bevel");
}

// ---- MN-03.4

#[test]
fn airbrush_strokes_add_lower_and_erase() {
    use AirbrushStroke::*;
    assert_eq!(AirbrushStroke::for_press(Modifiers::NONE), Some(Add));
    assert_eq!(AirbrushStroke::for_press(Modifiers::SHIFT), Some(Lower));
    assert_eq!(AirbrushStroke::for_press(Modifiers::COMMAND), Some(Erase));
    assert_eq!(AirbrushStroke::for_press(Modifiers::ALT), None);
    assert_eq!(Add.apply(0.5, 0.25), 0.75);
    assert_eq!(Add.apply(0.9, 0.25), 1.);
    assert_eq!(Lower.apply(0.5, 0.25), 0.25);
    assert_eq!(Lower.apply(0.1, 0.25), 0.);
    assert_eq!(Erase.apply(0.7, 0.01), 0.);
    assert_eq!(Erase.apply(0.7, 0.), 0.7);
}

fn stamp(a: &Operandi) -> Vec<f64> {
    let id = a.editor.scene.active().unwrap().id;
    a.editor
        .falloff_stamps
        .get(&(0, id))
        .map(|w| w.as_ref().clone())
        .unwrap_or_default()
}

fn most(w: &[f64]) -> f64 {
    w.iter().cloned().fold(0., f64::max)
}

#[test]
fn airbrush_shift_lowers_and_ctrl_erases_with_any_tool() {
    let (mut a, ctx) = moving();
    a.run(Command::SpatialFalloff(Some(Falloff::Airbrush(
        AirbrushFalloff {
            size: 400.,
            strength: 0.25,
            ..Default::default()
        },
    ))));
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    let paint = |a: &mut Operandi, m| {
        drag(a, &ctx, c, c + vec2(5., 0.), PointerButton::Primary, m);
        most(&stamp(a))
    };
    let built = paint(&mut a, Modifiers::NONE);
    assert!(built > 0.25, "{built} {}", a.status);
    let lowered = paint(&mut a, Modifiers::SHIFT);
    assert!(lowered < built, "{lowered} < {built}: {}", a.status);
    paint(&mut a, Modifiers::NONE);
    let erased = paint(&mut a, Modifiers::COMMAND);
    assert_eq!(erased, 0., "{}", a.status);
    // A middle drag is still the tool's own (Move repeats its last drag,
    // HM-04): it paints nothing.
    paint(&mut a, Modifiers::NONE);
    let painted = stamp(&a);
    drag(
        &mut a,
        &ctx,
        c,
        c + vec2(30., 0.),
        PointerButton::Middle,
        Modifiers::NONE,
    );
    assert_eq!(stamp(&a), painted);
}
