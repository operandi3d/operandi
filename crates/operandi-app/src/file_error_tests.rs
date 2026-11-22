//! A failed Save or Open is loud: a dialog with the reason, the scene stays
//! dirty, and its name and path don't change.
use super::tests::{box_app, draw, key};
use super::*;

#[test]
fn a_failed_save_shows_a_dialog_and_keeps_the_scene_dirty() {
    let mut a = box_app();
    assert!(a.dirty());
    let label = a.scene_label();
    let bad = std::env::temp_dir().join("operandi-no-such-directory-big-save/Guitar.opscene");
    assert!(!a.save(bad.clone()));
    assert!(a.dirty(), "title keeps its *");
    assert_eq!(a.path, None);
    assert_eq!(a.scene_label(), label);
    assert!(!a.status.contains("saved"), "{}", a.status);
    let (title, message) = *a.file_error.clone().expect("a dialog");
    assert_eq!(title, "Save");
    assert!(message.contains("not saved"), "{message}");
    assert!(
        message.contains("Cannot create save temporary"),
        "{message}"
    );
    assert!(!bad.exists());
    // The dialog is modal and OK closes it.
    let ctx = Context::default();
    theme::install(&ctx);
    draw(&mut a, &ctx, vec2(1200., 800.), vec![]);
    assert!(a.has_modal());
    draw(&mut a, &ctx, vec2(1200., 800.), vec![]);
    draw(&mut a, &ctx, vec2(1200., 800.), vec![key(Key::Enter)]);
    assert!(a.file_error.is_none());
    assert!(a.dirty());
}

#[test]
fn a_failed_open_shows_a_dialog_and_keeps_the_current_scene() {
    let mut a = box_app();
    let before = a.editor.scene.clone();
    let dir = std::env::temp_dir().join(format!("operandi-open-error-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Broken.opscene");
    std::fs::write(
        &path,
        b"{\"format\": \"Operandi\", \"version\": 34, \"scene\": 1}",
    )
    .unwrap();
    a.open(path.clone());
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.path, None);
    let (title, message) = *a.file_error.clone().expect("a dialog");
    assert_eq!(title, "Open");
    assert!(message.contains("Broken.opscene"), "{message}");
    std::fs::remove_dir_all(&dir).unwrap();
}
