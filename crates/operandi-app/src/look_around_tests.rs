//! NAV-03 look around (Alt+right drag, Jason 2026-11-22): the eye stays
//! put and the view turns about it, like turning on a swivel stool.
use super::*;
use crate::camera_light_ui::SETTLE;
use crate::input::NavigationKind;
use operandi_core::camera_light::item_frame;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(900., 600.))
}

/// One Alt+right drag step from the view's centre by `delta`.
fn look(a: &mut Operandi, delta: Vec2) {
    let c = rect().center();
    a.navigate_from(NavigationKind::Look, c, c + delta, rect());
}

fn eye(a: &Operandi) -> DVec3 {
    a.camera.basis().0
}

fn forward(a: &Operandi) -> DVec3 {
    a.camera.basis().3
}

#[test]
fn look_keeps_the_eye_and_turns_the_view_in_perspective() {
    let mut a = super::tests::box_app();
    a.camera.view = View::Perspective;
    let (e, f, d) = (eye(&a), forward(&a), a.camera.distance);
    look(&mut a, vec2(40., -25.));
    assert!((eye(&a) - e).length() < 1e-9, "{} vs {e}", eye(&a));
    assert!(forward(&a).dot(f) < 0.999, "the view turned");
    assert_eq!(a.camera.distance, d);
    assert_eq!(a.camera.view, View::Perspective);
}

#[test]
fn look_from_top_view_keeps_the_eye_and_frees_the_view() {
    let mut a = super::tests::box_app();
    a.camera.view = View::Top;
    let (e, f) = (eye(&a), forward(&a));
    look(&mut a, vec2(30., 10.));
    assert_eq!(a.camera.view, View::Orthographic);
    assert!((eye(&a) - e).length() < 1e-9, "{} vs {e}", eye(&a));
    assert!(forward(&a).dot(f) < 0.9999, "the view turned");
}

#[test]
fn dragging_right_looks_right_and_dragging_up_looks_up() {
    let mut a = super::tests::box_app();
    a.camera.view = View::Perspective;
    let (_, right, up, _) = a.camera.basis();
    let (to_right, above) = (a.camera.target + right, a.camera.target + up);
    let at = |a: &Operandi, p: DVec3| a.camera.project(p, rect()).unwrap().0;
    let c = rect().center();
    let (x0, y0) = (at(&a, to_right).x, at(&a, above).y);
    assert!(x0 > c.x && y0 < c.y);
    look(&mut a, vec2(5., 0.));
    let x1 = at(&a, to_right).x;
    assert!(x1 < x0 && x1 > c.x, "right of centre comes in: {x0} → {x1}");
    // Screen y grows downward: dragging up is a negative y.
    look(&mut a, vec2(0., -5.));
    let y1 = at(&a, above).y;
    assert!(y1 > y0 && y1 < c.y, "above centre comes down: {y0} → {y1}");
}

#[test]
fn look_stops_short_of_straight_up_and_down() {
    let mut a = super::tests::box_app();
    a.camera.view = View::Perspective;
    let e = eye(&a);
    for _ in 0..20 {
        look(&mut a, vec2(0., 500.));
    }
    assert_eq!(a.camera.pitch, crate::camera::ORBIT_PITCH);
    for _ in 0..40 {
        look(&mut a, vec2(0., -500.));
    }
    assert_eq!(a.camera.pitch, -crate::camera::ORBIT_PITCH);
    assert!((eye(&a) - e).length() < 1e-9);
}

#[test]
fn orbit_selected_and_trackball_leave_look_alone() {
    let mut a = super::tests::box_app();
    a.camera.view = View::Perspective;
    a.editor.dispatch(Command::Mode(Mode::Item)).unwrap();
    let id = a.editor.scene.items[0].id;
    a.editor.dispatch(Command::SelectItem(Some(id))).unwrap();
    let start = a.camera.clone();
    look(&mut a, vec2(60., 25.));
    let plain = a.camera.clone();
    a.camera = start;
    a.display.settings.drawing.orbit_selected = true;
    a.display.settings.drawing.trackball = Some(true);
    look(&mut a, vec2(60., 25.));
    assert_eq!(a.camera, plain);
}

#[test]
fn look_in_camera_view_turns_the_camera_in_place() {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    let id = a.editor.scene.render_camera().unwrap();
    a.enter_camera_view();
    a.sync_camera_view(rect());
    let before = item_frame(&a.editor.scene, id).unwrap();
    a.pointer_held = true;
    look(&mut a, vec2(50., 20.));
    a.sync_camera_view(rect());
    a.pointer_held = false;
    if let Some((p, _)) = a.camera.settling {
        a.camera.settling = Some((p, std::time::Instant::now() - SETTLE));
    }
    a.sync_camera_view(rect());
    let after = item_frame(&a.editor.scene, id).unwrap();
    assert!(
        (after.position - before.position).length() < 1e-6,
        "{} vs {}",
        after.position,
        before.position
    );
    assert!(
        after.forward.dot(before.forward) < 0.999,
        "the camera turned"
    );
}
