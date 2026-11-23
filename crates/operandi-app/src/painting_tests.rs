//! 3D painting stage 1 in the app (painting.rs): commands, menus, the New
//! Image panel's line, the paint target, saving and the quit question.
use super::*;
use operandi_core::paint::{self, pixels};

/// A box with UVs, selected, and a Material Group to paint into.
fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.run(Command::InsertUnit {
        tool: Tool::Box,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    assert!(!a.editor.scene.active().unwrap().mesh.uv_maps.is_empty());
    a
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-painting-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn every_painting_command_is_registered_and_marked_operandi_only() {
    let a = Operandi::empty();
    for name in crate::painting::command_names() {
        let (c, rest) = a
            .commands
            .find_line(name)
            .unwrap_or_else(|| panic!("{name}"));
        assert!(rest.is_empty());
        assert!(c.entry.flags.operandi, "{name} is Operandi's own name");
        assert!(c.host.is_some(), "{name} runs");
    }
}

#[test]
fn bare_new_image_opens_the_panel_and_typed_lines_make_images() {
    let mut a = app();
    a.execute("operandi.image.new");
    assert!(a.painting_dialog_open());
    a.painting = Default::default();
    let undo = a.editor.undo.len();
    a.execute("operandi.image.new resolution:256 format:rgb float:true color:0.5,0.25,1 name:Skin");
    let id = a.editor.paint.last_image.expect(&a.status);
    let image = a.editor.scene.images.iter().find(|i| i.id == id).unwrap();
    assert_eq!(image.name, "Skin");
    let px = &a.editor.scene.paint.buffers[&id];
    assert_eq!(
        (px.width, px.format, px.depth),
        (256, pixels::Format::Rgb, pixels::Depth::F32)
    );
    assert_eq!(px.get(0), [0.5, 0.25, 1., 1.]);
    assert_eq!(a.editor.undo.len(), undo, "New Image isn't an undo step");
    // Non power of two: the one-time note.
    a.execute("operandi.image.new resolution:300");
    assert_eq!(a.status, pixels::SQUARE_POW2_NOTE);
    a.execute("operandi.image.new resolution:300");
    assert_ne!(a.status, pixels::SQUARE_POW2_NOTE);
    a.execute("operandi.image.new resolution:10");
    assert!(a.status.contains("64 to 8192"), "{}", a.status);
}

#[test]
fn add_color_texture_makes_the_paint_target_with_a_badge() {
    let mut a = app();
    a.execute("operandi.paint.addTexture kind:color");
    assert!(a.painting_dialog_open(), "bare kind opens New Image");
    a.painting = Default::default();
    let undo = a.editor.undo.len();
    a.execute("operandi.paint.addTexture kind:color resolution:64");
    assert_eq!(a.editor.undo.len(), undo + 1, "{}", a.status);
    let layer = a.paint_target().expect("the new layer is the target");
    assert_eq!(a.shading.selected, vec![layer]);
    let rows = a.shading_tree_rows();
    let row = rows
        .iter()
        .find(|r| r.key == crate::shading_panel::ShaderKey::Layer(layer))
        .unwrap();
    assert!(row.badges.contains(&crate::tree_list::Badge::PaintTarget));
    a.execute("operandi.paint.target");
    assert_eq!(a.status, "Ready to paint");
    // Undo keeps the image.
    let image = a.editor.paint.last_image.unwrap();
    a.execute("app.undo");
    assert!(a.editor.scene.shader_tree.get(layer).is_none());
    assert!(a.editor.scene.images.iter().any(|i| i.id == image));
}

#[test]
fn no_texture_offers_to_add_one_unless_auto_create_is_off() {
    let mut a = app();
    a.execute("operandi.paint.target");
    assert!(a.painting.ask_add, "Auto Create Textures offers to add one");
    a.painting.ask_add = false;
    a.execute("operandi.paint.autoCreate off");
    assert!(!a.painting.prefs.auto_create);
    a.execute("operandi.paint.target");
    assert!(!a.painting.ask_add);
    assert_eq!(a.status, paint::NO_TEXTURE);
}

#[test]
fn image_commands_grey_without_an_image_and_run_on_the_target() {
    let mut a = app();
    let state = a.line_state("operandi.image.invert").unwrap();
    assert_eq!(state.unavailable.as_deref(), Some(paint::NO_IMAGE_SELECTED));
    a.execute("operandi.paint.addTexture kind:color resolution:64 color:1,0,0");
    let image = a.current_image().unwrap();
    assert!(
        a.line_state("operandi.image.invert")
            .unwrap()
            .unavailable
            .is_none()
    );
    let undo = a.editor.undo.len();
    a.execute("operandi.image.invert");
    assert_eq!(
        a.editor.scene.paint.buffers[&image].get(0),
        [0., 1., 1., 1.]
    );
    a.execute("operandi.image.setResolution size:128");
    assert_eq!(a.editor.scene.paint.buffers[&image].width, 128);
    a.execute("operandi.image.expandSeams border:2");
    a.execute("operandi.image.clear");
    assert_eq!(a.editor.scene.paint.buffers[&image].get(0), [0.; 4]);
    assert!(a.editor.undo.len() >= undo + 3, "{}", a.status);
    // Not built yet: greyed with a reason.
    for line in [
        "operandi.image.newSequence",
        "operandi.image.toVectorDisplacement",
        "operandi.paint.addFurTexture",
    ] {
        assert_eq!(
            a.line_state(line).unwrap().unavailable.as_deref(),
            Some(crate::painting::NOT_YET)
        );
    }
}

#[test]
fn menus_hold_the_painting_entries_and_each_runs_a_command() {
    let a = app();
    let ctx = egui::Context::default();
    let menus = a.menu_model(&ctx);
    let titles = |id: &str| -> Vec<String> {
        menus
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .leaves()
            .iter()
            .map(|i| i.title.clone())
            .collect()
    };
    let file = titles("menu.file");
    for t in [
        "New Image…",
        "Save Image",
        "Save Image As…",
        "Save All Images",
    ] {
        assert!(file.iter().any(|f| f == t), "{t} in {file:?}");
    }
    let texture = titles("menu.texture");
    for t in [
        "Add Color Texture…",
        "Add Mask Texture…",
        "Add Displacement Texture…",
        "Flatten Layers",
    ] {
        assert!(texture.iter().any(|f| f == t), "{t} in {texture:?}");
    }
    for menu in &menus {
        for item in menu.leaves() {
            if let crate::app_menu::Action::Line(i) = item.action {
                let line = crate::app_menu::action_line(i).unwrap();
                assert!(a.line_state(&line).is_some(), "{line} is a command");
            }
        }
    }
}

#[test]
fn save_image_writes_the_file_and_clears_the_star() {
    fn stand_in(request: crate::file_chooser::Choose) -> Option<std::path::PathBuf> {
        let crate::file_chooser::Choose::SavePaintImage { name, .. } = request else {
            panic!("asked for {request:?}");
        };
        assert!(
            name.ends_with(".png"),
            "8-bit images save as PNG first: {name}"
        );
        Some(scratch(&name))
    }
    let mut a = app();
    a.chooser = Some(stand_in);
    a.execute("operandi.paint.addTexture kind:color resolution:64 name:Paint");
    let image = a.current_image().unwrap();
    assert!(a.editor.image_changed(image));
    assert!(
        a.line_state("operandi.image.saveAll")
            .unwrap()
            .unavailable
            .is_none()
    );
    a.execute("operandi.image.save");
    assert!(!a.editor.image_changed(image), "{}", a.status);
    let path = a
        .editor
        .scene
        .images
        .iter()
        .find(|i| i.id == image)
        .unwrap()
        .path
        .clone();
    assert!(path.ends_with("Paint.png") && std::path::Path::new(&path).is_file());
    assert!(
        a.line_state("operandi.image.saveAll")
            .unwrap()
            .unavailable
            .is_some()
    );
    // Save As with a file name: a float image defaults to EXR when the
    // name has no type.
    a.execute("operandi.image.new resolution:64 float:true name:Height");
    let height = a.editor.paint.last_image.unwrap();
    a.painting.selected_image = Some(height);
    let target = scratch("height");
    a.execute(&format!(
        "operandi.image.saveAs filename:{}",
        target.display()
    ));
    assert!(target.with_extension("exr").is_file(), "{}", a.status);
}

#[test]
fn quitting_with_changed_images_asks_first() {
    let mut a = app();
    a.begin_quit();
    assert!(!a.painting.quit_prompt, "nothing to ask");
    a.pending = None;
    a.execute("operandi.image.new resolution:64");
    a.begin_quit();
    assert!(a.painting.quit_prompt);
    assert!(a.pending.is_none(), "the scene question waits");
    a.quit_images_answer(false);
    assert!(!a.painting.quit_prompt, "Don't Save lets the quit go on");
    assert!(matches!(a.pending, Some(FileAction::Close)));
}

#[test]
fn auto_save_backs_up_changed_images() {
    let mut a = app();
    a.execute("operandi.image.new resolution:64 name:Spots");
    let backup = scratch("scene-backup.opscene");
    a.autosave_images(&backup);
    assert!(scratch("scene-backup Spots.png").is_file());
}

#[test]
fn flatten_needs_two_image_layers() {
    let mut a = app();
    assert!(
        a.line_state("operandi.image.flatten")
            .unwrap()
            .unavailable
            .is_some()
    );
    a.execute("operandi.paint.addTexture kind:color resolution:64 color:1,0,0");
    let first = a.paint_target().unwrap();
    a.execute("operandi.paint.addTexture kind:color resolution:64 color:0,0,1");
    let second = a.paint_target().unwrap();
    a.shading.selected = vec![first, second];
    assert!(
        a.line_state("operandi.image.flatten")
            .unwrap()
            .unavailable
            .is_none()
    );
    let images = a.editor.scene.images.len();
    a.execute("operandi.image.flatten");
    assert_eq!(a.editor.scene.images.len(), images + 1, "{}", a.status);
}

#[test]
fn painted_images_show_in_the_viewport_cache() {
    let mut px = pixels::PaintImage::new(pixels::Layout {
        width: 64,
        height: 64,
        format: pixels::Format::Rgba,
        depth: pixels::Depth::U8,
    })
    .unwrap();
    px.fill([1., 0., 0., 1.]);
    let data = crate::gpu_scene::painted_image(&px);
    assert_eq!((data.width, data.height), (64, 64));
    assert_eq!(&data.rgba[..4], &[255, 0, 0, 255]);
}

#[test]
fn photoshop_composite_asks_which_layer_then_paints_it() {
    let mut a = app();
    a.execute("operandi.paint.addTexture kind:color resolution:64 color:1,0,0 name:Skin");
    let layer = a.paint_target().unwrap();
    let image = a.current_image().unwrap();
    let path = scratch("skin-ui.psd");
    a.execute(&format!(
        "operandi.image.saveAs filename:{}",
        path.display()
    ));
    assert!(path.is_file(), "{}", a.status);
    // On the Composite, Invert asks which layer.
    a.execute("operandi.image.invert");
    assert_eq!(a.painting.which_layer.as_ref().map(|w| w.0), Some(layer));
    assert!(a.painting_dialog_open());
    // Picking Layer 1 points the map at it; the line runs again.
    a.painting.which_layer = None;
    a.execute(&format!(
        "operandi.paint.psdLayer layer:{layer} name:\"Layer 1\""
    ));
    assert_eq!(
        paint::psd_layer_of(&a.editor.scene, layer).as_deref(),
        Some("Layer 1")
    );
    a.execute("operandi.image.invert");
    assert_eq!(
        a.editor.layer_picture(layer).unwrap().get(0),
        [0., 1., 1., 1.]
    );
    assert!(a.editor.image_changed(image));
    // New Layer: Operandi Paint 1, and back to the Composite.
    a.execute(&format!("operandi.paint.newPsdLayer layer:{layer}"));
    assert_eq!(
        paint::psd_layer_of(&a.editor.scene, layer).as_deref(),
        Some("Operandi Paint 1")
    );
    a.execute(&format!("operandi.paint.psdLayer layer:{layer}"));
    assert_eq!(paint::psd_layer_of(&a.editor.scene, layer), None);
    a.execute("operandi.image.save");
    assert!(!a.editor.image_changed(image), "{}", a.status);
    let doc = pixels::psd::PsdDoc::read(&path).unwrap();
    let names: Vec<String> = doc.rows().into_iter().map(|r| r.name).collect();
    assert_eq!(names, ["Composite", "Operandi Paint 1", "Layer 1"]);
}
