//! GitHub #40: a dragged handle stays under the pointer. Axis arrows and
//! Free move used one screen-to-world scale taken at the orbit centre, so
//! a handle nearer or farther than that centre, or an arrow running into
//! the view, drifted away from the pointer (about half speed when the
//! handle sat at half the orbit distance).
use super::tests::box_app;
use super::*;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800., 600.))
}

/// Move on polygon 0 of the box, seen in perspective from an angle with
/// the orbit centre well behind the handle.
fn moving_app() -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate(Tool::Move);
    a.camera.view = View::Perspective;
    a.camera.yaw = 0.6;
    a.camera.pitch = 0.4;
    a.camera.distance = 6.;
    // Orbit centre 3 units behind the box: the handle is at about half the
    // orbit distance.
    let (_, _, _, forward) = a.camera.basis();
    a.camera.target = forward * 3.;
    a
}

fn handle_screen(a: &Operandi) -> Option<Pos2> {
    let centre = a.transform_handle_center();
    a.camera.project(centre, rect()).map(|(p, _)| p)
}

#[test]
fn move_arrow_tracks_the_pointer_in_perspective() {
    for axis in 0..3 {
        let mut a = moving_app();
        let handles = a.handles(rect());
        let Some(&(_, from, tip, _)) = handles.iter().find(|h| h.0 == axis) else {
            continue;
        };
        let start = handle_screen(&a).expect("handle on screen");
        let along = (tip - from).normalized();
        let to = start + along * 120.;
        let base = a.editor.session.live.as_ref().unwrap().params.clone();
        a.update_handle(axis, base, start, to, Tool::Move, rect());
        let now = handle_screen(&a).expect("handle still on screen");
        assert!(
            now.distance(to) < 1.5,
            "axis {axis}: handle at {now:?}, pointer at {to:?} ({})",
            a.status
        );
    }
}

#[test]
fn free_move_tracks_the_pointer_in_perspective() {
    let mut a = moving_app();
    let _ = a.handles(rect());
    let start = handle_screen(&a).expect("handle on screen");
    let to = start + vec2(90., -60.);
    let base = a.editor.session.live.as_ref().unwrap().params.clone();
    a.update_handle(viewport::FREE_HANDLE, base, start, to, Tool::Move, rect());
    let now = handle_screen(&a).expect("handle still on screen");
    assert!(
        now.distance(to) < 1.5,
        "handle at {now:?}, pointer at {to:?}"
    );
}

#[test]
fn end_on_arrow_falls_back_to_a_screen_drag() {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.activate(Tool::Move);
    let (_, _, _, forward) = a.camera.basis();
    let start = pos2(400., 300.);
    // A line straight into the view: no exact answer, so no number either.
    assert_eq!(
        a.camera
            .axis_drag(DVec3::ZERO, forward, start, start + vec2(0., 50.), rect()),
        None
    );
}
