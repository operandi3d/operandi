//! Zoom to Surface (ZP-01..04): the perspective wheel zoom moves toward the
//! surface under the pointer instead of stalling at a fixed orbit centre.
//! fix-stuck-zoom: drag zoom (toward the press), the Zoom In/Out keys and
//! the header's Zoom button (toward the view centre) follow the same rule,
//! and Symmetry changes none of it.
use super::tests::box_app;
use super::*;
use crate::input::NavigationKind;

fn view() -> Rect {
    Rect::from_min_size(pos2(200., 50.), vec2(1200., 800.))
}

/// A perspective view of the 2 m box whose orbit centre hides 5 m behind
/// it, so zooming about the centre would pass through the box.
fn behind(mut a: Operandi) -> Operandi {
    a.camera.view = View::Perspective;
    a.viewport_rect = view();
    let (eye, _, _, forward) = a.camera.basis();
    a.camera.target = eye + forward * 12.;
    a.camera.distance = 12.;
    a
}

/// A wheel step and a drag delta that zoom in.
fn wheel_in(a: &Operandi) -> f32 {
    if a.input_preferences.options.wheel_zoom_factor(60.) < 1. {
        60.
    } else {
        -60.
    }
}
fn drag_in(a: &Operandi) -> Vec2 {
    [vec2(0., 20.), vec2(0., -20.), vec2(20., 0.), vec2(-20., 0.)]
        .into_iter()
        .find(|&d| a.input_preferences.options.drag_zoom_factor(d) < 1.)
        .unwrap()
}

fn depth(a: &Operandi, p: DVec3) -> f64 {
    let (eye, _, _, forward) = a.camera.basis();
    (p - eye).dot(forward)
}

#[test]
fn wheel_zoom_reaches_the_surface_under_the_pointer() {
    let mut a = behind(box_app());
    let r = view();
    let (start, _) = a.camera.project(DVec3::new(0.3, 0.4, 0.2), r).unwrap();
    let hit = a
        .zoom_surface_point(start, r)
        .expect("box under the pointer");
    let (at, _) = a.camera.project(hit, r).unwrap();
    let scroll = wheel_in(&a);
    for _ in 0..400 {
        a.wheel_zoom(Some(start), scroll, r);
        let (now, _) = a.camera.project(hit, r).expect("surface still drawn");
        assert!((now - at).length() < 0.05, "{at:?} -> {now:?}");
        assert!(depth(&a, hit) >= camera::MIN_SURFACE_DEPTH * 0.999);
    }
    // Past the old 3 cm stop, right up to the surface.
    assert!(
        depth(&a, hit) < 2. * camera::MIN_SURFACE_DEPTH,
        "{}",
        depth(&a, hit)
    );
    assert!(a.camera.near() < depth(&a, hit));
    // Frame puts the fixed near clip back.
    a.frame(false);
    assert_eq!(a.camera.near(), camera::NEAR_CLIP);
}

#[test]
fn wheel_zoom_at_cursor_off_heads_for_the_view_centre() {
    let mut a = behind(box_app());
    let r = view();
    a.input_preferences.options.wheel_zoom_at_cursor = false;
    let centre = a.zoom_surface_point(r.center(), r).unwrap();
    let side = r.center() + vec2(300., 200.);
    let scroll = wheel_in(&a);
    for _ in 0..400 {
        a.wheel_zoom(Some(side), scroll, r);
    }
    assert!(depth(&a, centre) < 2. * camera::MIN_SURFACE_DEPTH);
    let (now, _) = a.camera.project(centre, r).unwrap();
    assert!((now - r.center()).length() < 0.05);
}

/// The wheel, drag and key zooms on `a` and on `b`; their cameras must
/// match.
fn same_zooms(mut a: Operandi, mut b: Operandi) {
    let r = view();
    let p = r.center() + vec2(140., -90.);
    let (scroll, step) = (wheel_in(&a), drag_in(&a));
    for app in [&mut a, &mut b] {
        app.wheel_zoom(Some(p), scroll, r);
        app.input_preferences.options.wheel_zoom_at_cursor = false;
        app.wheel_zoom(Some(p), scroll, r);
        app.input_preferences.options.wheel_zoom_at_cursor = true;
        app.execute("Zoom In");
        app.execute("Zoom In");
        app.execute("Zoom Out");
        let mut last = p;
        for _ in 0..5 {
            app.navigate_gesture(NavigationKind::Zoom, p, last, last + step, r);
            last += step;
        }
    }
    assert_eq!(a.camera.placement(), b.camera.placement());
    assert_eq!(a.camera.near(), b.camera.near());
}

/// ZP-04: over empty space the wheel works as before, which is what it
/// does with the option off.
#[test]
fn empty_space_zooms_as_before() {
    let mut off = behind(Operandi::empty());
    off.input_preferences.options.zoom_to_surface = false;
    same_zooms(behind(Operandi::empty()), off);
}

/// `zoom_to_surface` off: the box is ignored and every zoom matches the
/// old behaviour (a scene with nothing to hit), in an orthographic view
/// too, which never zooms to the surface.
#[test]
fn switch_off_is_the_old_behaviour() {
    let mut off = behind(box_app());
    off.input_preferences.options.zoom_to_surface = false;
    same_zooms(off, behind(Operandi::empty()));
    // The old wheel zoom stops 3 cm from the orbit centre.
    let mut old = behind(box_app());
    old.input_preferences.options.zoom_to_surface = false;
    let r = view();
    let scroll = wheel_in(&old);
    for _ in 0..400 {
        old.wheel_zoom(Some(r.center()), scroll, r);
    }
    assert_eq!(old.camera.distance, camera::MIN_DISTANCE);
    assert_eq!(old.camera.near(), camera::NEAR_CLIP);

    let mut ortho = behind(box_app());
    ortho.camera.view = View::Orthographic;
    let mut ortho_empty = behind(Operandi::empty());
    ortho_empty.camera.view = View::Orthographic;
    same_zooms(ortho, ortho_empty);
}

/// The stall Jason hit: with the orbit centre 5 m behind the box, the old
/// drag zoom and keys stop 3 cm from that centre, far from the surface.
#[test]
fn old_drag_and_keys_stall_at_the_orbit_centre() {
    let mut old = behind(box_app());
    old.input_preferences.options.zoom_to_surface = false;
    let r = view();
    let p = r.center() + vec2(40., -30.);
    let step = drag_in(&old);
    let mut last = p;
    for _ in 0..400 {
        old.navigate_gesture(NavigationKind::Zoom, p, last, last + step, r);
        last += step;
    }
    for _ in 0..20 {
        old.execute("Zoom In");
    }
    assert_eq!(old.camera.distance, camera::MIN_DISTANCE);
}

/// fix-stuck-zoom: a drag zoom heads for the surface under the press,
/// keeps that spot under it, stops just short of it and never passes it;
/// the orbit centre ends up beside it, so orbiting turns about it.
#[test]
fn drag_zoom_reaches_the_surface_under_the_press() {
    let mut a = behind(box_app());
    let r = view();
    let p = r.center() + vec2(40., -30.);
    let hit = a.zoom_surface_point(p, r).expect("box under the press");
    let (at, _) = a.camera.project(hit, r).unwrap();
    let step = drag_in(&a);
    let mut last = p;
    for _ in 0..400 {
        a.navigate_gesture(NavigationKind::Zoom, p, last, last + step, r);
        last += step;
        let (now, _) = a.camera.project(hit, r).expect("surface still drawn");
        assert!((now - at).length() < 0.05, "{at:?} -> {now:?}");
        assert!(depth(&a, hit) >= camera::MIN_SURFACE_DEPTH * 0.999);
    }
    assert!(depth(&a, hit) < 2. * camera::MIN_SURFACE_DEPTH);
    assert!((a.camera.target - hit).length() < 1e-3, "orbit centre near");
    assert!(a.camera.near() < depth(&a, hit));
    // Dragging back out leaves the surface the same way.
    let mut last = p;
    for _ in 0..50 {
        a.navigate_gesture(NavigationKind::Zoom, p, last, last - step, r);
        last -= step;
    }
    assert!(depth(&a, hit) > 1e-2);
}

/// The Zoom In/Out keys and the header's Zoom button head for the surface
/// at the view centre.
#[test]
fn keys_and_zoom_button_reach_the_surface_at_the_view_centre() {
    for button in [false, true] {
        let mut a = behind(box_app());
        let r = view();
        let centre = a.zoom_surface_point(r.center(), r).unwrap();
        for _ in 0..60 {
            if button {
                let step = drag_in(&a) * 3.;
                a.navigate(NavigationKind::Zoom, step, r);
            } else {
                a.execute("Zoom In");
            }
            assert!(depth(&a, centre) >= camera::MIN_SURFACE_DEPTH * 0.999);
        }
        assert!(
            depth(&a, centre) < 2. * camera::MIN_SURFACE_DEPTH,
            "{button}"
        );
        let (now, _) = a.camera.project(centre, r).unwrap();
        assert!((now - r.center()).length() < 0.05);
        a.execute("Zoom Out");
        assert!(depth(&a, centre) > 1.5 * camera::MIN_SURFACE_DEPTH);
    }
}

/// However big a step, no zoom passes through the surface.
#[test]
fn a_huge_step_never_passes_through() {
    let mut a = behind(box_app());
    let r = view();
    let p = r.center() + vec2(40., -30.);
    let hit = a.zoom_surface_point(p, r).unwrap();
    let step = drag_in(&a) * 500.;
    a.navigate_gesture(NavigationKind::Zoom, p, p, p + step, r);
    assert!(depth(&a, hit) >= camera::MIN_SURFACE_DEPTH * 0.999);
    a.input_preferences.options.wheel_acceleration = 100_000;
    a.wheel_zoom(Some(p), wheel_in(&a) * 50., r);
    assert!(depth(&a, hit) >= camera::MIN_SURFACE_DEPTH * 0.999);
    // Still the box in front, not its far side.
    assert_eq!(
        a.zoom_surface_point(p, r)
            .map(|h| (h - hit).length() < 1e-6),
        Some(true)
    );
}

/// Symmetry doesn't change navigation: every zoom, then an orbit, lands
/// on the same camera with it on or off.
#[test]
fn symmetry_changes_no_zoom() {
    let r = view();
    let p = r.center() + vec2(40., -30.);
    let run = |symmetry: bool| {
        let mut a = behind(box_app());
        a.execute("polygons");
        if symmetry {
            a.execute("symmetry x");
            assert!(a.editor.symmetry.axis.is_some());
        }
        let (scroll, step) = (wheel_in(&a), drag_in(&a));
        for _ in 0..30 {
            a.wheel_zoom(Some(p), scroll, r);
        }
        let mut last = p;
        for _ in 0..30 {
            a.navigate_gesture(NavigationKind::Zoom, p, last, last + step, r);
            last += step;
        }
        a.execute("Zoom In");
        a.navigate_gesture(NavigationKind::Orbit, p, p, p + vec2(30., 10.), r);
        a.camera.placement()
    };
    assert_eq!(run(false), run(true));
}

/// One frame of the whole app with `events`.
fn app_frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) {
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

/// Real wheel events through the viewport, a Sub-D sphere framed with
/// Symmetry X on and the pointer on the seam and beside it: the zoom gets
/// to the surface every time.
#[test]
fn wheel_events_reach_the_surface_with_symmetry_on() {
    for (symmetry, offset) in [
        (false, vec2(0., 0.)),
        (true, vec2(0., 0.)),
        (true, vec2(25., 10.)),
    ] {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let sphere = operandi_core::radial::globe_geometry(
            DVec3::ZERO,
            operandi_core::radial::RadialParameters {
                radius: DVec3::ONE,
                sides: 24,
                segments: 12,
            },
        )
        .unwrap();
        a.editor.scene.active_mut().unwrap().mesh = sphere.into();
        a.execute("polygons");
        a.execute("toggle subdivision");
        a.camera.view = View::Perspective;
        a.camera.yaw = 0.;
        a.camera.pitch = 0.;
        if symmetry {
            a.execute("symmetry x");
        }
        let ctx = Context::default();
        for _ in 0..3 {
            app_frame(&mut a, &ctx, vec![]);
        }
        a.frame(false);
        app_frame(&mut a, &ctx, vec![]);
        let r = a.viewport_rect;
        let p = r.center() + offset;
        let hit = a
            .zoom_surface_point(p, r)
            .expect("sphere under the pointer");
        let sign = wheel_in(&a).signum();
        for _ in 0..300 {
            app_frame(
                &mut a,
                &ctx,
                vec![
                    Event::PointerMoved(p),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0., 60. * sign),
                        modifiers: Modifiers::NONE,
                    },
                ],
            );
        }
        let d = depth(&a, hit);
        assert!(
            (camera::MIN_SURFACE_DEPTH * 0.999..2. * camera::MIN_SURFACE_DEPTH).contains(&d),
            "symmetry {symmetry} {offset:?}: {d}"
        );
    }
}

/// Wheel Zoom at Mouse Cursor in Front, Right and Top views (and drag
/// zoom, which anchors at the press the same way): the world point under
/// the pointer stays under it. The anchor was the ground plane, edge-on
/// in Front, Back and side views, so those zoomed about the view centre.
/// Box zoom keeps the orbit centre on the plane it was on.
#[test]
fn ortho_zoom_at_cursor_keeps_the_point_under_the_pointer() {
    let r = view();
    let p = r.center() + vec2(310., -170.);
    for view in [View::Front, View::Right, View::Top] {
        let mut a = box_app();
        a.camera.view = view;
        a.camera.target = DVec3::new(0.4, -0.3, 0.2);
        a.viewport_rect = r;
        a.input_preferences.options.wheel_zoom_at_cursor = true;
        let held = a.camera.view_plane_point(p, r).unwrap();
        let close = |a: &Operandi, what: &str| {
            let now = a.camera.view_plane_point(p, r).unwrap();
            assert!(
                (now - held).length() <= 1e-6 * held.length().max(1.),
                "{view:?} {what}: {held} -> {now}"
            );
            let (at, _) = a.camera.project(held, r).unwrap();
            assert!((at - p).length() < 1e-3, "{view:?} {what}: {at:?}");
        };
        let scroll = wheel_in(&a);
        let before = a.camera.distance;
        for _ in 0..6 {
            a.wheel_zoom(Some(p), scroll, r);
            close(&a, "wheel");
        }
        assert!(a.camera.distance < before, "{view:?} zoomed in");
        let (step, mut last) = (drag_in(&a), r.center());
        for _ in 0..5 {
            a.navigate_gesture(NavigationKind::Zoom, p, last, last + step, r);
            last += step;
            close(&a, "drag");
        }
        let (_, _, _, forward) = a.camera.basis();
        let plane = a.camera.target.dot(forward);
        a.box_zoom(Rect::from_center_size(p, vec2(200., 120.)), r);
        assert!(
            (a.camera.target.dot(forward) - plane).abs() < 1e-9,
            "{view:?} box zoom moved the orbit centre off its plane"
        );
    }
}
