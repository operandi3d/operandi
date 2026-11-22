//! Camera view navigation without a press (wheel, trackpad, 3D mouse):
//! the camera item moves once the view holds still, as one undo step, and
//! moving a camera keeps the picking surface. Writing the camera every
//! frame made Camera view navigation crawl on heavy scenes.
use super::*;
use crate::camera_light_ui::SETTLE;
use operandi_core::camera_light::item_frame;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(900., 600.))
}

/// A new document (it has a render camera) looking through its camera.
fn app() -> (Operandi, u64) {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    let id = a.editor.scene.render_camera().unwrap();
    a.enter_camera_view();
    a.sync_camera_view(rect());
    (a, id)
}

fn camera_position(a: &Operandi, id: u64) -> DVec3 {
    item_frame(&a.editor.scene, id).unwrap().position
}

/// One wheel-zoom frame: the view moves, nothing is held.
fn wheel_frame(a: &mut Operandi) {
    a.camera.distance *= 0.9;
    a.sync_camera_view(rect());
}

/// Let the waiting move's quiet time pass.
fn wait_out(a: &mut Operandi) {
    let (p, _) = a.camera.settling.expect("a move waits");
    a.camera.settling = Some((p, std::time::Instant::now() - SETTLE));
}

#[test]
fn a_wheel_gesture_moves_the_camera_once_it_holds_still() {
    let (mut a, id) = app();
    let before = camera_position(&a, id);
    let undo = a.editor.undo.len();
    for _ in 0..5 {
        wheel_frame(&mut a);
    }
    // Still moving (and then just stopped): nothing written yet, and a
    // repaint is asked for so the write happens even if the app idles.
    assert_eq!(camera_position(&a, id), before);
    assert_eq!(a.editor.undo.len(), undo);
    a.sync_camera_view(rect());
    assert_eq!(a.editor.undo.len(), undo);
    assert!(a.camera_view_settle_in().is_some());
    let (eye, ..) = a.camera.basis();
    wait_out(&mut a);
    a.sync_camera_view(rect());
    assert_eq!(a.editor.undo.len(), undo + 1, "one step for the gesture");
    assert!((camera_position(&a, id) - eye).length() < 1e-9);
    assert!(a.camera.settling.is_none() && a.camera_view_settle_in().is_none());
    // The view stays put after the write, and later frames write nothing.
    let (eye2, ..) = a.camera.basis();
    assert!((eye2 - eye).length() < 1e-6);
    a.sync_camera_view(rect());
    assert_eq!(a.editor.undo.len(), undo + 1);
}

#[test]
fn a_drag_still_moves_the_camera_on_release_without_waiting() {
    let (mut a, id) = app();
    let undo = a.editor.undo.len();
    a.pointer_held = true;
    for _ in 0..3 {
        a.camera.yaw += 0.1;
        a.sync_camera_view(rect());
    }
    assert_eq!(a.editor.undo.len(), undo);
    assert!(
        a.camera_view_settle_in().is_none(),
        "no repaints while held"
    );
    let (eye, ..) = a.camera.basis();
    a.pointer_held = false;
    a.sync_camera_view(rect());
    assert_eq!(a.editor.undo.len(), undo + 1);
    assert!((camera_position(&a, id) - eye).length() < 1e-9);
}

#[test]
fn undo_during_the_wait_undoes_the_waiting_move() {
    let (mut a, id) = app();
    let before = camera_position(&a, id);
    let undo = a.editor.undo.len();
    wheel_frame(&mut a);
    wheel_frame(&mut a);
    // Undo straight away: the move is written first, then undone, so the
    // camera and the view are back where they were and Redo brings it.
    a.run(Command::Undo);
    assert_eq!(a.editor.undo.len(), undo);
    assert!((camera_position(&a, id) - before).length() < 1e-9);
    a.sync_camera_view(rect());
    let (eye, ..) = a.camera.basis();
    assert!((eye - before).length() < 1e-9);
    a.run(Command::Redo);
    assert_ne!(camera_position(&a, id), before);
}

#[test]
fn leaving_camera_view_keeps_a_waiting_move() {
    let (mut a, id) = app();
    let before = camera_position(&a, id);
    wheel_frame(&mut a);
    let (eye, ..) = a.camera.basis();
    a.leave_camera_view();
    assert_ne!(camera_position(&a, id), before);
    assert!((camera_position(&a, id) - eye).length() < 1e-9);
}

#[test]
fn moving_a_camera_keeps_the_picking_surface() {
    let (mut a, id) = app();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::primitive::cube_geometry(DVec3::ZERO, DVec3::ONE)
            .unwrap()
            .into();
    a.pick_surface();
    let builds = a.surface_cache.builds();
    wheel_frame(&mut a);
    wait_out(&mut a);
    a.sync_camera_view(rect());
    assert!(camera_position(&a, id).is_finite());
    a.pick_surface();
    assert_eq!(
        a.surface_cache.builds(),
        builds,
        "a camera move rebuilt picking"
    );
    // A mesh under the camera moves with it: then picking does rebuild.
    let mesh = a.editor.scene.active().unwrap().id;
    a.editor
        .scene
        .items
        .iter_mut()
        .find(|i| i.id == mesh)
        .unwrap()
        .parent = Some(id);
    a.pick_surface();
    let builds = a.surface_cache.builds();
    wheel_frame(&mut a);
    wait_out(&mut a);
    a.sync_camera_view(rect());
    a.pick_surface();
    assert_eq!(a.surface_cache.builds(), builds + 1);
}
