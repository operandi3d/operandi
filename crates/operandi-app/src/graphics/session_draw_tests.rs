//! Drag creation for Cone, Ellipsoid, Torus and Capsule (Jason's bug
//! report 2026-11-06, spec drag-creation-2026-11-06): no Ctrl gives a flat
//! base centred on the press; the centre handle then gives height; Ctrl
//! keeps the uniform drag.
use super::tests::box_app;
use super::*;
use operandi_core::shapes::ShapeParameters;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

/// The session's (size along the normal, an in-plane size, preview built).
fn state(a: &Operandi) -> (f64, f64, bool) {
    if let Some(s) = &a.editor.session.cone {
        let p = &s.requested;
        return (p.size[p.axis], p.size.x, s.preview.is_some());
    }
    if let Some(s) = &a.editor.session.ellipsoid {
        let p = &s.requested;
        return (p.radius[p.axis], p.radius.x, s.preview.is_some());
    }
    let s = a.editor.session.shape.as_ref().unwrap();
    let built = s.preview.is_some();
    match &s.requested {
        ShapeParameters::Torus(t) => (t.height, t.ring_radius, built),
        ShapeParameters::Capsule(c) => (c.radius[c.axis], c.radius.x, built),
        _ => unreachable!(),
    }
}

/// Press at (2, 0, 0), drag to (3, 0, 1) in Top view.
fn draw(a: &mut Operandi, which: usize, uniform: bool) {
    let r = rect();
    let plane = a.actual_plane();
    let from = a.camera.project(DVec3::new(2., 0., 0.), r).unwrap().0;
    let to = a.camera.project(DVec3::new(3., 0., 1.), r).unwrap().0;
    if which < 2 {
        let cone = which == 0;
        let input::GestureKind::SolidDraw { anchor, .. } = a.begin_solid_draw(cone, from, r) else {
            panic!("no draw: {}", a.status);
        };
        a.solid_draw(cone, plane, anchor, to, r, uniform);
    } else {
        let input::GestureKind::Numeric(drag) = a.begin_numeric_drag(from, r) else {
            panic!("no draw: {}", a.status);
        };
        a.numeric_drag(&drag, from, to, r, uniform);
    }
}

fn armed(which: usize) -> Operandi {
    let mut a = box_app();
    a.run(match which {
        0 => Command::ArmCone,
        1 => Command::ArmEllipsoid,
        2 => Command::ArmShape(ShapeKind::Torus),
        _ => Command::ArmShape(ShapeKind::Capsule),
    });
    a
}

#[test]
fn first_drag_is_flat_and_the_centre_handle_adds_height() {
    for which in 0..4 {
        let mut a = armed(which);
        draw(&mut a, which, false);
        let (height, across, built) = state(&a);
        assert_eq!(height, 0., "shape {which} starts flat: {}", a.status);
        assert!(
            across > 0.9 && built,
            "shape {which}: {across} {}",
            a.status
        );
        // The handle sits on the base centre: drag it up.
        let r = rect();
        let handle = a.session_height_handle(r).expect("height handle");
        let Some(input::GestureKind::SessionHeight { base, axis }) =
            a.begin_session_height(handle, r)
        else {
            panic!("shape {which}: the handle didn't grab");
        };
        a.session_height(base, axis, handle, handle - vec2(0., 60.), r);
        let (height, _, built) = state(&a);
        assert!(
            height > 0. && built,
            "shape {which} got height: {}",
            a.status
        );
    }
}

#[test]
fn ctrl_keeps_the_uniform_drag() {
    for which in 0..4 {
        let mut a = armed(which);
        draw(&mut a, which, true);
        let (height, _, _) = state(&a);
        assert!(height > 0., "shape {which}: Ctrl drags a solid at once");
    }
}

/// Capsule height (Jason, 2026-11-06): the rounded ends grow first, then the
/// straight sides. The drag used to clamp End Size to its value on the
/// previous frame, so the first small step left the ends flat for good.
#[test]
fn capsule_height_drag_grows_rounded_ends_then_sides() {
    let mut a = armed(3);
    draw(&mut a, 3, false);
    let r = rect();
    let handle = a.session_height_handle(r).expect("height handle");
    let Some(input::GestureKind::SessionHeight { base, axis }) = a.begin_session_height(handle, r)
    else {
        panic!("the handle didn't grab");
    };
    let capsule = |a: &Operandi| match &a.editor.session.shape.as_ref().unwrap().requested {
        ShapeParameters::Capsule(c) => c.clone(),
        _ => unreachable!(),
    };
    let across = {
        let c = capsule(&a);
        (0..3)
            .filter(|&i| i != c.axis)
            .map(|i| c.radius[i])
            .fold(0., f64::max)
    };
    // One pixel up: a squashed, fully rounded shape.
    a.session_height(base, axis, handle, handle - vec2(0., 1.), r);
    let c = capsule(&a);
    assert!(c.radius[axis] < across, "{c:?}");
    assert_eq!(c.end_size, c.radius[axis], "ends fill the height");
    // Keep pulling: full hemispheres plus straight sides.
    a.session_height(base, axis, handle, handle - vec2(0., 400.), r);
    let c = capsule(&a);
    assert!(c.radius[axis] > across, "{c:?}");
    assert_eq!(c.end_size, across, "ends grew back to hemispheres");
    assert!(a.editor.session.shape.as_ref().unwrap().preview.is_some());
}
