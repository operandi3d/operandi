//! Light view: the view menu's Light entries look through a light, and
//! navigating moves the light, through the same path as Camera view.
use super::*;
use crate::camera::View;
use operandi_core::camera_light::{LightEdit, LightKind, item_frame};
use operandi_core::scene::Transform;

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a
}

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(900., 600.))
}

/// Add a light of `kind` at `at`, turned by `rotation` (radians), selected.
fn add_light(a: &mut Operandi, kind: LightKind, at: DVec3, rotation: DVec3) -> u64 {
    a.add_item(operandi_core::item_list::AddItemKind::Light(kind));
    let id = a.editor.scene.selection.item.unwrap();
    let item = a
        .editor
        .scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap();
    item.transform = Transform {
        translation: at,
        rotation,
        ..item.transform.clone()
    };
    id
}

fn select(a: &mut Operandi, id: u64) {
    a.editor.scene.selection.item = Some(id);
}

fn light(a: &Operandi, id: u64) -> operandi_core::Item {
    a.editor.scene.item(id).unwrap().clone()
}

#[test]
fn picking_a_light_looks_from_it_along_its_aim() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Directional,
        DVec3::new(2., 3., 4.),
        DVec3::new(-0.4, 0.6, 0.),
    );
    a.enter_light_view(Some(id));
    a.sync_camera_view(rect());
    assert!(a.camera.looks_through() && !a.camera.through);
    assert_eq!(a.view_title(), light(&a, id).name);
    let f = item_frame(&a.editor.scene, id).unwrap();
    let (eye, _, _, forward) = a.camera.basis();
    assert!(
        (eye - f.position).length() < 1e-9,
        "{eye} vs {}",
        f.position
    );
    assert!((forward - f.forward).length() < 1e-9);
    assert_eq!(a.camera.view, View::Perspective);
    // Leaving puts the ordinary lens back.
    a.choose_view_type(Some(View::Top));
    assert!(!a.camera.looks_through());
    assert_eq!(a.camera.view, View::Top);
    assert_eq!(a.camera.lens, crate::camera::default_lens());
}

#[test]
fn a_point_light_looks_down_its_own_axis() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(0., 1., 5.),
        DVec3::ZERO,
    );
    a.enter_light_view(Some(id));
    a.sync_camera_view(rect());
    let (eye, _, _, forward) = a.camera.basis();
    assert!((eye - DVec3::new(0., 1., 5.)).length() < 1e-9);
    // Unrotated, an item looks down its local -Z.
    assert!((forward - -DVec3::Z).length() < 1e-9, "{forward}");
}

#[test]
fn navigating_moves_the_light_once_per_press_and_undoes() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Spot,
        DVec3::new(0., 2., 6.),
        DVec3::new(-0.3, 0., 0.),
    );
    let r = rect();
    a.enter_light_view(Some(id));
    a.sync_camera_view(r);
    let before = light(&a, id);
    let undo = a.editor.undo.len();
    // A drag in progress: the view moves, the light waits.
    a.pointer_held = true;
    for _ in 0..3 {
        a.camera.yaw += 0.1;
        a.camera.distance *= 0.9;
        a.sync_camera_view(r);
    }
    assert_eq!(light(&a, id), before);
    let (eye, _, _, forward) = a.camera.basis();
    // Release: one undo step puts the light where the view is.
    a.pointer_held = false;
    a.sync_camera_view(r);
    assert_eq!(a.editor.undo.len(), undo + 1);
    assert_eq!(a.status, "Light moved");
    let f = item_frame(&a.editor.scene, id).unwrap();
    assert!((f.position - eye).length() < 1e-9);
    assert!((f.forward - forward).length() < 1e-9);
    // The view stays where the user left it.
    let (eye2, _, _, forward2) = a.camera.basis();
    assert!((eye2 - eye).length() < 1e-6 && (forward2 - forward).length() < 1e-6);
    // Only the transform changed: the light's own settings are kept.
    assert_eq!(light(&a, id).light, before.light);
    // Undo puts the light back, and the view follows it.
    a.run(Command::Undo);
    assert_eq!(light(&a, id), before);
    a.sync_camera_view(r);
    let f = item_frame(&a.editor.scene, id).unwrap();
    let (eye, _, _, forward) = a.camera.basis();
    assert!((eye - f.position).length() < 1e-9);
    assert!((forward - f.forward).length() < 1e-9);
    a.run(Command::Redo);
    a.sync_camera_view(r);
    assert!((item_frame(&a.editor.scene, id).unwrap().position - eye2).length() < 1e-9);
}

#[test]
fn a_locked_light_stays_and_the_view_snaps_back() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(0., 1., 5.),
        DVec3::ZERO,
    );
    a.editor
        .scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap()
        .common
        .assembly
        .lock = operandi_core::item_common::Choice::Yes;
    let r = rect();
    a.enter_light_view(Some(id));
    a.sync_camera_view(r);
    let before = light(&a, id);
    let undo = a.editor.undo.len();
    a.camera.yaw += 0.3;
    a.sync_camera_view(r);
    // A move with nothing held is written once the view holds still.
    let (p, _) = a.camera.settling.expect("the move waits");
    a.camera.settling = Some((
        p,
        std::time::Instant::now() - crate::camera_light_ui::SETTLE,
    ));
    a.sync_camera_view(r);
    assert_eq!(light(&a, id), before);
    assert_eq!(a.editor.undo.len(), undo);
    assert!(a.status.contains("locked"), "{}", a.status);
    let (eye, ..) = a.camera.basis();
    assert!((eye - DVec3::new(0., 1., 5.)).length() < 1e-9);
}

#[test]
fn current_selection_follows_the_selected_light() {
    let mut a = app();
    let first = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(1., 1., 5.),
        DVec3::ZERO,
    );
    let second = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(-1., 1., 5.),
        DVec3::ZERO,
    );
    let mesh = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh())
        .unwrap()
        .id;
    // Nothing to follow: the view stays as it was.
    select(&mut a, mesh);
    a.enter_light_view(None);
    assert!(a.camera.light.is_none());
    assert_eq!(a.status, "Select a light to look through");
    select(&mut a, first);
    a.enter_light_view(None);
    a.sync_camera_view(rect());
    assert_eq!(a.through_light(), Some(first));
    select(&mut a, second);
    a.sync_camera_view(rect());
    assert_eq!(a.through_light(), Some(second));
    let (eye, ..) = a.camera.basis();
    assert!((eye - DVec3::new(-1., 1., 5.)).length() < 1e-9);
    // Something else selected: the last light stays.
    select(&mut a, mesh);
    a.sync_camera_view(rect());
    assert_eq!(a.through_light(), Some(second));
    // A light picked by name doesn't follow the selection.
    a.enter_light_view(Some(first));
    select(&mut a, second);
    a.sync_camera_view(rect());
    assert_eq!(a.through_light(), Some(first));
}

#[test]
fn deleting_the_light_leaves_light_view() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(0., 1., 5.),
        DVec3::ZERO,
    );
    a.enter_light_view(Some(id));
    a.sync_camera_view(rect());
    select(&mut a, id);
    a.run(Command::DeleteItem);
    a.sync_camera_view(rect());
    assert!(!a.camera.looks_through());
    assert_eq!(a.status, "Light view needs a light in the scene");
    assert_eq!(a.camera.view, View::Perspective);
}

#[test]
fn camera_and_light_view_replace_each_other() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(0., 1., 5.),
        DVec3::ZERO,
    );
    a.enter_light_view(Some(id));
    a.enter_camera_view();
    assert!(a.camera.through && a.camera.light.is_none());
    a.enter_light_view(Some(id));
    assert!(!a.camera.through && a.camera.light.is_some());
    // A non-light can't be looked through as a light.
    let mesh = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh())
        .unwrap()
        .id;
    a.enter_light_view(Some(mesh));
    assert_eq!(a.through_light(), Some(id));
}

#[test]
fn a_spot_view_shows_its_whole_cone() {
    let mut a = app();
    let id = add_light(&mut a, LightKind::Spot, DVec3::new(0., 0., 5.), DVec3::ZERO);
    let item = a
        .editor
        .scene
        .items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap();
    item.light
        .as_mut()
        .unwrap()
        .apply(LightEdit::ConeAngle(40f64.to_radians()))
        .unwrap();
    let r = rect();
    a.enter_light_view(Some(id));
    a.sync_camera_view(r);
    // The cone's edge, 3 m ahead, lands on the view's top edge (the
    // shorter side of a 900 x 600 view).
    let (eye, _, up, forward) = a.camera.basis();
    let edge = eye + forward * 3. + up * 3. * 20f64.to_radians().tan();
    let (p, _) = a.camera.project(edge, r).unwrap();
    assert!((p.y - r.top()).abs() < 1e-3, "{p:?}");
    // Other lights use a new view's lens.
    let point = add_light(&mut a, LightKind::Point, DVec3::ZERO, DVec3::ZERO);
    a.enter_light_view(Some(point));
    a.sync_camera_view(r);
    assert_eq!(a.camera.lens, crate::camera::default_lens());
}

#[test]
fn view_set_looks_through_a_light() {
    let mut a = app();
    let id = add_light(
        &mut a,
        LightKind::Point,
        DVec3::new(0., 1., 5.),
        DVec3::ZERO,
    );
    a.look_through(id).unwrap();
    assert_eq!(a.through_light(), Some(id));
    assert!(!a.camera.light.unwrap().follow_selection);
}
