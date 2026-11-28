//! Numpad "." Perspective View, SC-054 "Perspective (press again:
//! camera)": Perspective, then the scene's camera, then back.
use super::tests::box_app;
use super::*;

#[test]
fn press_again_goes_to_the_camera_and_back() {
    let mut a = box_app();
    a.add_item(operandi_core::item_list::AddItemKind::Camera);
    a.camera.view = View::Top;
    a.execute("perspective view");
    assert_eq!(a.camera.view, View::Perspective);
    assert!(!a.camera.looks_through());
    a.execute("perspective view");
    assert!(a.camera.through, "second press: Camera view");
    a.execute("perspective view");
    assert!(
        !a.camera.looks_through(),
        "third press: back to Perspective"
    );
    assert_eq!(a.camera.view, View::Perspective);
}

#[test]
fn no_camera_stays_perspective_and_says_why() {
    let mut a = box_app();
    a.camera.view = View::Perspective;
    a.execute("perspective view");
    assert_eq!(a.camera.view, View::Perspective);
    assert!(!a.camera.looks_through());
    assert!(a.status.contains("no camera"), "{}", a.status);
}
