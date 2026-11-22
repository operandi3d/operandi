//! Painted images in several open scenes (Astra P9-004, P9-005, P8-005,
//! P8-004, P8-001). Image ids are per scene, so two tabs here both use
//! image ids from the same start: each tab must keep its own saved-pixel
//! baseline, file times, save queue and quit answers.
use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-painting-tabs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// The save panel's stand-in: saves under the suggested name.
fn panel(request: crate::file_chooser::Choose) -> Option<PathBuf> {
    let crate::file_chooser::Choose::SavePaintImage { name, .. } = request else {
        panic!("asked for {request:?}");
    };
    Some(scratch(&name))
}

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.chooser = Some(panel);
    a
}

/// A new image named `name` in the open scene; returns its id.
fn new_image(a: &mut Operandi, name: &str) -> u64 {
    a.execute(&format!("operandi.image.new resolution:64 name:{name}"));
    let id = a.editor.paint.last_image.expect("an image was made");
    a.painting.selected_image = Some(id);
    id
}

#[test]
fn each_tab_keeps_its_own_saved_image_baseline_and_file_times() {
    let mut a = app();
    let red = new_image(&mut a, "TabsRed");
    a.execute("operandi.image.save");
    assert!(!a.editor.image_changed(red), "{}", a.status);
    let red_time = a.painting.doc.file_times.get(&red).copied();
    assert!(red_time.is_some());

    a.request_file(FileAction::New);
    let blue = new_image(&mut a, "TabsBlue");
    assert_eq!(blue, red, "both scenes number their images alike");
    a.execute("operandi.image.save");
    assert!(!a.editor.image_changed(blue), "{}", a.status);

    // Back in the first scene, its image is still saved: the second
    // scene's save didn't replace its baseline.
    assert!(a.switch_scene(0));
    assert!(
        !a.editor.image_changed(red),
        "the first scene's image looks unsaved after the other scene saved"
    );
    assert_eq!(a.painting.doc.file_times.get(&red).copied(), red_time);
    assert!(a.switch_scene(1));
    assert!(!a.editor.image_changed(blue));
}
