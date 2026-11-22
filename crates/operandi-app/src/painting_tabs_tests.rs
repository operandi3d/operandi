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

/// Astra P8-005: quitting from a clean scene still asks about changed
/// images in another, parked scene.
#[test]
fn quit_from_a_clean_scene_asks_about_images_in_other_scenes() {
    let mut a = app();
    let first = a.scenes.active_tab().id;
    new_image(&mut a, "TabsParked");
    a.request_file(FileAction::New); // clean, in front
    assert!(a.editor.changed_images().is_empty());

    a.begin_quit();
    assert!(
        a.painting.quit_prompt,
        "the parked scene's images are asked about"
    );
    assert_eq!(
        a.scenes.active_tab().id,
        first,
        "its scene comes to the front"
    );
    assert!(!a.allow_close);
    // Save: the image gets its file, then the quit goes on.
    a.quit_images_answer(true);
    assert!(a.editor.changed_images().is_empty(), "{}", a.status);
    assert!(scratch("TabsParked.png").is_file());
}

/// Astra P8-004: Don't Save, then Cancel at the scene question, and the
/// next quit asks about images again.
#[test]
fn dont_save_lasts_only_for_that_quit() {
    let mut a = app();
    new_image(&mut a, "TabsOnce");
    a.run(Command::NewMesh); // the scene is unsaved too
    a.begin_quit();
    assert!(a.painting.quit_prompt);
    a.quit_images_answer(false); // Don't Save
    assert!(matches!(a.pending, Some(FileAction::Close)));
    a.pending = None; // Cancel at the scene question
    assert!(!a.allow_close);

    a.begin_quit();
    assert!(a.painting.quit_prompt, "the images are asked about again");
    // Cancel on the images question also forgets the answers.
    a.quit_images_cancel();
    assert!(a.painting.quit_decided.is_empty());
}

/// Astra P8-001: Close Scene waits for the scene's image save panel; the
/// answer writes this scene's pixels under the chosen name, and Cancel
/// keeps the scene open.
#[test]
fn close_scene_waits_for_its_image_save_panels() {
    let answer: crate::file_chooser::HeldAnswer = Default::default();
    let ctx = Context::default();
    let mut a = app();
    a.painting.prefs.also_save_images = false;
    let red = new_image(&mut a, "TabsWaitRed");
    let red_pixels = a.editor.scene.paint.buffers[&red].clone();
    a.run(Command::NewMesh);
    let scene_file = scratch("tabs-wait.opscene");
    assert!(a.save(scene_file.clone()));
    a.painting.prefs.also_save_images = true;
    a.request_file(FileAction::New);
    // A different size, so the file shows whose pixels it holds.
    a.execute("operandi.image.new resolution:128 name:TabsWaitBlue");
    assert!(a.switch_scene(0));

    // Edit, then Close Scene saves the scene, whose image needs a panel.
    a.run(Command::NewMesh);
    a.held_panel = Some(answer.clone());
    a.prompt_save(FileAction::CloseScene);
    assert!(a.image_saves_pending());
    assert_eq!(
        a.scenes.tabs.len(),
        2,
        "the scene stays open while its panel is"
    );
    a.poll_choice(&ctx);
    assert_eq!(a.scenes.tabs.len(), 2);

    // Cancel on the panel: nothing closes, the image is still unsaved.
    *answer.borrow_mut() = Some(None);
    a.poll_choice(&ctx);
    assert_eq!(a.scenes.tabs.len(), 2);
    assert!(a.editor.image_changed(red));
    assert!(a.painting.after_images.is_none());

    // Again, answering with a file: the red image is written, then the
    // scene closes.
    a.run(Command::NewMesh);
    a.prompt_save(FileAction::CloseScene);
    assert!(a.image_saves_pending());
    let file = scratch("TabsWaitRed-chosen.png");
    *answer.borrow_mut() = Some(Some(file.clone()));
    a.poll_choice(&ctx);
    assert_eq!(a.scenes.tabs.len(), 1, "{}", a.status);
    let written = operandi_core::paint::pixels::io::load(&file).unwrap();
    assert_eq!(written.width, red_pixels.width, "the closed scene's pixels");
    assert_eq!(written.pixels, red_pixels.pixels);
}
