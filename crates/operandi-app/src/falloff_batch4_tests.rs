//! Falloff batch 4 (falloff spec 2026-11-15): drawing Screen and Lasso and
//! painting Airbrush in the view (FO-43, FO-44, FO-52).
use super::tests::{box_app, draw};
use super::*;
use operandi_core::{AirbrushFalloff, LassoFalloff, ScreenFalloff};

const SIZE: Vec2 = vec2(1440., 900.);

fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    let button = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        modifiers: Modifiers::NONE,
        button: PointerButton::Primary,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![button(from, true)]);
    for step in 1..=6 {
        let t = step as f32 / 6.;
        draw(
            a,
            ctx,
            SIZE,
            vec![Event::PointerMoved(from + (to - from) * t)],
        );
    }
    draw(a, ctx, SIZE, vec![button(to, false)]);
}

fn stamp(a: &Operandi) -> Vec<f64> {
    let id = a.editor.scene.active().unwrap().id;
    a.editor
        .falloff_stamps
        .get(&(0, id))
        .map(|w| w.as_ref().clone())
        .unwrap_or_default()
}

/// FO-43: once armed, a press sets the centre and the drag the size; the
/// box's points under the circle get weight.
#[test]
fn screen_is_drawn_by_press_and_drag() {
    let ctx = Context::default();
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Screen(
        ScreenFalloff::default(),
    ))));
    a.falloff_draw_armed = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    drag(&mut a, &ctx, c, c + vec2(300., 0.));
    let Some(Falloff::Screen(s)) = a.editor.spatial_falloff else {
        panic!()
    };
    assert!(s.size > 250., "{s:?} {}", a.status);
    assert!(stamp(&a).iter().any(|&w| w > 0.), "{}", a.status);
    assert!(!a.falloff_draw_armed);
}

/// FO-52: a Rectangle drag over the whole view takes every front point at
/// full weight (and, projecting through, the back ones too).
#[test]
fn lasso_rectangle_stamps_inside_points() {
    let ctx = Context::default();
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Lasso(
        LassoFalloff::default(),
    ))));
    a.falloff_draw_armed = true;
    draw(&mut a, &ctx, SIZE, vec![]);
    let r = a.viewport_rect;
    drag(
        &mut a,
        &ctx,
        r.left_top() + vec2(5., 5.),
        r.right_bottom() - vec2(5., 5.),
    );
    let w = stamp(&a);
    assert!(
        !w.is_empty() && w.iter().all(|&w| w == 1.),
        "{w:?} {}",
        a.status
    );
}

/// FO-44: dragging over the box with Move open paints weight that builds
/// up; the brush moves with the pointer.
#[test]
fn airbrush_paints_on_drag() {
    let ctx = Context::default();
    let mut a = box_app();
    a.run(Command::SpatialFalloff(Some(Falloff::Airbrush(
        AirbrushFalloff {
            size: 400.,
            strength: 0.25,
            ..Default::default()
        },
    ))));
    draw(&mut a, &ctx, SIZE, vec![]);
    let c = a.viewport_rect.center();
    drag(&mut a, &ctx, c, c + vec2(5., 0.));
    let w = stamp(&a);
    let most = w.iter().cloned().fold(0., f64::max);
    assert!(most > 0.25 && most <= 1., "{most} {}", a.status);
}
