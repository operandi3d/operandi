//! Preset browser Stage 1 acceptance (PB-T10..T16, CD-T1..T5) in the app.
use super::*;
use operandi_core::shader_tree::{NewLayer, NodeKind};

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.load_preset_browser();
    a
}

fn scratch(a: &Operandi, name: &str) -> PathBuf {
    let dir = a.environment.paths().data_root().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Wait for the background listing of the shown folder.
fn listed(a: &mut Operandi) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut t = 0.;
    while a.preset_browser.listing_busy() || a.preset_browser.listing.is_none() {
        t += 2.;
        a.preset_browser.tick(t);
        assert!(
            std::time::Instant::now() < deadline,
            "listing never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn add_layer(a: &mut Operandi, layer: NewLayer, selected: Option<u64>) -> u64 {
    a.run(Command::ShaderTree(ShaderTreeEdit::Add { layer, selected }));
    a.editor.scene.shader_tree.next_id - 1
}

#[test]
fn pb_t10_roots_persist_and_removing_one_leaves_its_files() {
    let mut a = app();
    // PB-Q2: the Presets folder is the first root from the start.
    assert_eq!(
        a.preset_browser.roots[0].path,
        a.environment.paths().presets()
    );
    let lib = scratch(&a, "Library");
    std::fs::write(lib.join("keep.png"), b"x").unwrap();
    a.execute(&format!(
        "pbview.basePathAdd path:\"{}\" basePathKey:Textures",
        lib.display()
    ));
    assert!(
        a.preset_browser
            .roots
            .iter()
            .any(|r| r.path == lib && r.name() == "Textures")
    );
    a.save_preset_browser();
    // Relaunch: the root is still there.
    let mut b = Operandi::with_environment(a.environment.clone());
    b.load_preset_browser();
    assert!(b.preset_browser.roots.iter().any(|r| r.path == lib));
    b.execute(&format!("pbview.basePathRemove path:\"{}\"", lib.display()));
    assert!(!b.preset_browser.roots.iter().any(|r| r.path == lib));
    assert!(lib.join("keep.png").is_file(), "files on disk untouched");
    // The removed Presets root doesn't come back on the next launch.
    let presets = b.environment.paths().presets();
    b.preset_browser.remove_root(&presets);
    b.save_preset_browser();
    let mut c = Operandi::with_environment(b.environment.clone());
    c.load_preset_browser();
    assert!(!c.preset_browser.roots.iter().any(|r| r.path == presets));
}

#[test]
fn bq_06_roots_reorder_by_drag() {
    let mut a = app();
    let (x, y) = (scratch(&a, "X"), scratch(&a, "Y"));
    a.preset_browser.add_root(x.clone(), None);
    a.preset_browser.add_root(y.clone(), None);
    let presets = a.environment.paths().presets();
    let order = |a: &Operandi| {
        a.preset_browser
            .roots
            .iter()
            .map(|r| r.path.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&a), [presets.clone(), x.clone(), y.clone()]);
    a.preset_browser
        .reorder_roots(std::slice::from_ref(&y), Some((presets.clone(), false)));
    assert_eq!(order(&a), [y.clone(), presets.clone(), x.clone()]);
    // Dropped after a root's subfolder: after that root.
    a.preset_browser
        .reorder_roots(std::slice::from_ref(&y), Some((x.join("Sub"), true)));
    assert_eq!(order(&a), [presets, x, y]);
}

#[test]
fn pb_t11_the_window_state_and_view_settings_are_commands_and_persist() {
    let mut a = app();
    let ctx = Context::default();
    // Layout > Windows/Palettes > Preset Browser (LAY-55).
    let i = a
        .palettes
        .list
        .iter()
        .position(|p| p.hash == crate::palettes::PRESET_BROWSER)
        .unwrap();
    a.menu_action(
        &ctx,
        app_menu::line_action(&format!("layout.window hash:{}", a.palettes.list[i].hash)),
    );
    assert!(a.preset_browser.window_open);
    a.execute("layout.window hash:presetBrowser open:false");
    assert!(!a.preset_browser.window_open);
    a.execute("layout.window hash:presetBrowser");
    assert!(a.preset_browser.window_open);
    a.preset_browser.window_pos = Some([300., 200.]);
    a.execute("pbview.mode mode:list");
    a.execute("pbview.sortMode mode:size");
    a.execute("operandi.pbview.thumbnailSize size:500");
    assert_eq!(a.preset_browser.view, ViewMode::List);
    assert_eq!(a.preset_browser.sort, SortBy::Size);
    assert_eq!(a.preset_browser.thumb_size, THUMB_MAX);
    a.save_preset_browser();
    let mut b = Operandi::with_environment(a.environment.clone());
    b.load_preset_browser();
    assert!(b.preset_browser.window_open);
    assert_eq!(b.preset_browser.window_pos, Some([300., 200.]));
    assert_eq!(b.preset_browser.view, ViewMode::List);
    assert_eq!(b.preset_browser.sort, SortBy::Size);
    // F6 is the default key; its action toggles the floating window.
    assert_eq!(
        input::default_bindings()
            .get(POPOVER_ACTION)
            .map(String::as_str),
        Some("F6")
    );
    b.execute(POPOVER_ACTION);
    assert!(b.preset_browser.popover_toggle);
    // The next frame closes the open window; F6 again reopens it at its
    // remembered place.
    let ctx = Context::default();
    let _ = ctx.run(Default::default(), |ctx| b.preset_browser_frame(ctx));
    assert!(!b.preset_browser.window_open);
    b.execute(POPOVER_ACTION);
    let _ = ctx.run(Default::default(), |ctx| b.preset_browser_frame(ctx));
    assert!(b.preset_browser.window_open);
    assert_eq!(b.preset_browser.window_pos, Some([300., 200.]));
}

#[test]
fn pb_t13_a_material_preset_needs_a_group_and_is_one_undo_step() {
    let mut a = app();
    let folder = a.environment.paths().presets();
    std::fs::create_dir_all(&folder).unwrap();
    let group = add_layer(&mut a, NewLayer::Group, None);
    add_layer(&mut a, NewLayer::Material, Some(group));
    add_layer(&mut a, NewLayer::Constant, Some(group));
    let file = folder.join("Rust.oppreset");
    a.editor
        .save_preset(&file, PresetSource::Material(group))
        .unwrap();
    a.preset_browser.show_folder(folder.clone());
    listed(&mut a);
    let entry = a.preset_browser.entry(&file).expect("listed");
    assert_eq!(entry.kind, EntryKind::Preset(PresetKind::Material));
    // Nothing selected: a message, nothing changes.
    a.shading.selected.clear();
    let before = a.editor.scene.clone();
    a.browser_load(&entry, Target::Default);
    assert_eq!(a.editor.scene, before);
    assert!(!a.status.is_empty());
    // A group selected: applied, one undo removes it.
    let target = add_layer(&mut a, NewLayer::Group, None);
    a.shading.selected = vec![target];
    let before = a.editor.scene.clone();
    let undo = a.editor.undo.len();
    a.browser_load(&entry, Target::Default);
    assert_eq!(a.editor.undo.len(), undo + 1);
    assert!(
        a.editor
            .scene
            .shader_tree
            .children(Some(target))
            .any(|n| matches!(n.kind, NodeKind::Texture(_)))
    );
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}

#[test]
fn pb_t14_an_item_preset_lands_at_the_origin_in_one_step() {
    let mut a = app();
    a.run(Command::NewMesh);
    let folder = scratch(&a, "Items");
    let file = folder.join("Cube.oppreset");
    a.editor.save_preset(&file, PresetSource::Items).unwrap();
    let entry = a.entry_for_path(&file);
    assert_eq!(entry.kind, EntryKind::Preset(PresetKind::Item));
    let items = a.editor.scene.items.len();
    a.browser_load(&entry, Target::Default);
    assert_eq!(a.editor.scene.items.len(), items + 1);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items.len(), items);
}

#[test]
fn cd_t1_and_t3_texture_sets_and_hdrs_load_through_preset_do() {
    let mut a = app();
    let lib = scratch(&a, "Content");
    for name in [
        "Bricks01_1K-JPG_Color.jpg",
        "Bricks01_1K-JPG_NormalGL.jpg",
        "Bricks01_1K-JPG_NormalDX.jpg",
        "Bricks01_1K-JPG_Roughness.jpg",
        "Bricks01_1K-JPG_Displacement.jpg",
    ] {
        image::RgbImage::from_pixel(4, 4, image::Rgb([90, 40, 30]))
            .save(lib.join(name))
            .unwrap();
    }
    image::Rgb32FImage::from_pixel(8, 4, image::Rgb([2.0, 1.5, 1.0]))
        .save(lib.join("studio_4k.hdr"))
        .unwrap();
    a.preset_browser.add_root(lib.clone(), None);
    listed(&mut a);
    let shown = a.preset_browser.shown();
    let names: Vec<(&str, EntryKind)> = shown.iter().map(|e| (e.name.as_str(), e.kind)).collect();
    assert_eq!(
        names,
        [
            ("Bricks01", EntryKind::TextureSet),
            ("studio_4k", EntryKind::Environment)
        ]
    );
    // CD-T1 through the command: one material group with four layers.
    let group = add_layer(&mut a, NewLayer::Group, None);
    a.execute(&format!(
        "preset.do preset:\"{}\" target:\"{}\"",
        lib.join("Bricks01_1K-JPG_Color.jpg").display(),
        a.editor.scene.shader_tree.get(group).unwrap().name
    ));
    let layers = a.editor.scene.shader_tree.children(Some(group)).count();
    assert_eq!(
        layers, 5,
        "colour, roughness, GL normal, displacement and the material: {}",
        a.status
    );
    // CD-T3: the HDR becomes the environment image; one undo restores.
    let before = a.editor.scene.clone();
    a.execute(&format!(
        "preset.do preset:\"{}\"",
        lib.join("studio_4k.hdr").display()
    ));
    assert_ne!(a.editor.scene, before, "{}", a.status);
    assert!(
        a.editor
            .scene
            .images
            .iter()
            .any(|i| i.path.ends_with("studio_4k.hdr"))
    );
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
    // CD-T5: nothing was written into the content folder.
    assert_eq!(std::fs::read_dir(&lib).unwrap().count(), 6);
}

#[test]
fn pb_t16_file_actions_rename_duplicate_and_delete() {
    let mut a = app();
    let lib = scratch(&a, "Files");
    for name in ["wall_diff_2k.jpg", "wall_rough_2k.jpg", "photo.png"] {
        image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]))
            .save(lib.join(name))
            .unwrap();
    }
    a.preset_browser.add_root(lib.clone(), None);
    listed(&mut a);
    // A texture set renames as one: every map keeps its suffix.
    let set = a
        .preset_browser
        .shown()
        .into_iter()
        .find(|e| e.kind == EntryKind::TextureSet)
        .unwrap();
    a.execute(&format!(
        "file.rename newName:brick path:\"{}\"",
        set.path.display()
    ));
    assert!(
        lib.join("brick_diff_2k.jpg").is_file() && lib.join("brick_rough_2k.jpg").is_file(),
        "{}",
        a.status
    );
    a.preset_browser.highlight = Some(lib.join("photo.png"));
    a.execute("file.duplicate newName:copy");
    assert!(lib.join("copy.png").is_file() && lib.join("photo.png").is_file());
    // The menu's Delete asks first; Cancel keeps the file (the question is
    // the browser's modal).
    a.preset_browser.ask = Some(Ask::Delete(lib.join("copy.png")));
    assert!(a.has_modal());
    a.preset_browser.ask = None;
    assert!(lib.join("copy.png").is_file());
    a.execute(&format!(
        "file.delete path:\"{}\"",
        lib.join("copy.png").display()
    ));
    assert!(!lib.join("copy.png").exists());
}

#[test]
fn pb_122_saving_over_a_name_asks_and_keep_both_numbers_it() {
    let mut a = app();
    a.run(Command::NewMesh);
    let folder = scratch(&a, "Save");
    let path = folder.join("Mesh.oppreset");
    a.browser_save(path.clone(), PresetSource::Items, false);
    assert!(path.is_file());
    let first = std::fs::read(&path).unwrap();
    a.run(Command::NewMesh);
    a.browser_save(path.clone(), PresetSource::Items, false);
    assert!(matches!(a.preset_browser.ask, Some(Ask::Clash { .. })));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        first,
        "never overwritten silently"
    );
    let free = operandi_core::preset::library::free_path(&folder, "Mesh", "oppreset");
    assert_eq!(free, folder.join("Mesh 2.oppreset"));
    // PB-46: the saved file shows at once in the folder it went to.
    assert_eq!(a.preset_browser.folder.as_deref(), Some(folder.as_path()));
    assert_eq!(a.preset_browser.highlight.as_deref(), Some(path.as_path()));
}

#[test]
fn search_finds_entries_below_the_folder_and_the_body_draws() {
    let mut a = app();
    let lib = scratch(&a, "Deep");
    std::fs::create_dir_all(lib.join("Woods/Oak")).unwrap();
    image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]))
        .save(lib.join("Woods/Oak/oak_bark.png"))
        .unwrap();
    image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]))
        .save(lib.join("stone.png"))
        .unwrap();
    a.preset_browser.add_root(lib.clone(), None);
    listed(&mut a);
    a.execute("pbview.filterString filter:oak");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut t = 100.;
    while a.preset_browser.shown().is_empty() {
        t += 2.;
        a.preset_browser.tick(t);
        assert!(
            std::time::Instant::now() < deadline,
            "search never finished"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(a.preset_browser.shown()[0].name, "oak_bark");
    a.execute("pbview.filterStringEnable state:false");
    assert_eq!(
        a.preset_browser.shown().len(),
        1,
        "stone only, at the top level"
    );
    // Every view draws a frame without trouble.
    let ctx = Context::default();
    for view in ["vertical", "horizontal", "list"] {
        a.execute(&format!("pbview.mode mode:{view}"));
        let _ = ctx.run(Default::default(), |ctx| {
            CentralPanel::default().show(ctx, |ui| a.preset_browser_body(ui, false));
        });
    }
    // The folder tree has the root and (add path…).
    assert!(a.preset_browser.folder_rects.iter().any(|(p, _)| p == &lib));
}

/// pbview.mode's integers never change (scripts store them): 0 is
/// Vertical (the old "thumbnails"), 1 List, 2 Horizontal; `?` answers with
/// the name; a file saved with "thumbnails" opens in Vertical.
#[test]
fn pbview_mode_numbers_names_query_and_the_old_saved_name() {
    let mut a = app();
    for (given, want) in [
        ("0", ViewMode::Vertical),
        ("1", ViewMode::List),
        ("2", ViewMode::Horizontal),
        ("thumbnails", ViewMode::Vertical),
        ("Horizontal", ViewMode::Horizontal),
        ("list", ViewMode::List),
        ("vertical", ViewMode::Vertical),
    ] {
        a.execute(&format!("pbview.mode mode:{given}"));
        assert_eq!(a.preset_browser.view, want, "{given}");
    }
    a.execute("pbview.mode mode:2");
    a.execute("pbview.mode mode:?");
    assert_eq!(a.status, "horizontal");
    assert_eq!(
        a.preset_browser.view,
        ViewMode::Horizontal,
        "a query changes nothing"
    );
    a.execute("pbview.mode mode:3");
    assert!(a.status.contains("horizontal (2)"), "{}", a.status);
    assert_eq!(a.preset_browser.view, ViewMode::Horizontal);
    let old: ViewMode = serde_json::from_str("\"thumbnails\"").unwrap();
    assert_eq!(old, ViewMode::Vertical);
    assert_eq!(
        serde_json::to_string(&ViewMode::Vertical).unwrap(),
        "\"vertical\""
    );
}

/// Every bit of text a frame drew.
pub(super) fn drawn_text(output: &egui::FullOutput) -> String {
    fn walk(shape: &egui::Shape, out: &mut String) {
        match shape {
            egui::Shape::Text(t) => {
                out.push_str(&t.galley.job.text);
                out.push('\n');
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = String::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// A small pack: one CC0 material preset under Starter/ and its manifest.
fn install_pack(folder: &Path, version: &str) {
    use operandi_core::preset::pack::{Manifest, ManifestItem};
    let item = folder.join("Starter/Bricks");
    std::fs::create_dir_all(&item).unwrap();
    let (_, preset) = operandi_core::preset::basics::presets().remove(0);
    operandi_core::persistence::save_preset(&item.join("Bricks.oppreset"), &preset).unwrap();
    Manifest::new(
        "Operandi Library",
        version,
        vec![ManifestItem {
            path: "Starter/Bricks/Bricks.oppreset".into(),
            kind: "material".into(),
            licence: "CC0-1.0".into(),
            source_url: "https://ambientcg.com/a/Bricks113A".into(),
            credit: "ambientCG".into(),
            checked: "2026-11-11".into(),
            category: String::new(),
        }],
    )
    .write(folder)
    .unwrap();
}

#[test]
fn f1_basics_root_is_present_shown_first_and_read_only() {
    let mut a = app();
    let basics = a.preset_browser.basics.clone().expect("Basics root");
    assert_eq!(basics, a.environment.paths().basics());
    // A fresh browser shows the Basics, not an empty folder.
    assert_eq!(a.preset_browser.folder.as_ref(), Some(&basics));
    // It's not one of the user's roots (PB-Q2's Presets stays first).
    assert!(!a.preset_browser.roots.iter().any(|r| r.path == basics));
    a.preset_browser.start_listing(false);
    listed(&mut a);
    let names: Vec<String> = a
        .preset_browser
        .shown()
        .iter()
        .map(|e| e.name.clone())
        .collect();
    assert_eq!(
        names,
        ["Clay", "Emissive", "Glass", "Metal", "Plastic", "Rubber"]
    );
    assert!(
        a.preset_browser
            .shown()
            .iter()
            .all(|e| e.kind == EntryKind::Preset(PresetKind::Material))
    );
    // Loading one is the usual one-step material load.
    let group = add_layer(&mut a, NewLayer::Group, None);
    a.shading.selected = vec![group];
    let plastic = a.entry_for_path(&basics.join("Plastic.oppreset"));
    assert!(a.browser_load(&plastic, crate::preset_browser::Target::Default));
    // Nothing can write there: rename, duplicate, delete, save, remove.
    let file = basics.join("Metal.oppreset");
    let before = std::fs::read(&file).unwrap();
    a.execute(&format!("file.delete path:\"{}\"", file.display()));
    assert_eq!(a.status, crate::preset_browser::READ_ONLY_NOTE);
    a.execute(&format!(
        "file.rename newName:Steel path:\"{}\"",
        file.display()
    ));
    a.execute(&format!(
        "file.duplicate newName:Steel path:\"{}\"",
        file.display()
    ));
    assert!(!basics.join("Steel.oppreset").exists());
    a.browser_save(
        basics.join("Mine.oppreset"),
        PresetSource::RenderSettings,
        true,
    );
    assert!(!basics.join("Mine.oppreset").exists());
    a.execute(&format!(
        "pbview.basePathRemove path:\"{}\"",
        basics.display()
    ));
    assert!(a.status.contains("read-only"), "{}", a.status);
    assert_eq!(std::fs::read(&file).unwrap(), before);
    // A tampered file is put back on the next start.
    std::fs::write(&file, b"broken").unwrap();
    let mut b = Operandi::with_environment(a.environment.clone());
    b.load_preset_browser();
    assert_eq!(std::fs::read(&file).unwrap(), before);
    // Its tooltip says what it is.
    let tip = b.entry_tip(&b.entry_for_path(&file));
    assert!(
        tip.contains("GPL-3.0-or-later") && tip.contains("read-only"),
        "{tip}"
    );
}

#[test]
fn f1_empty_state_offers_the_library_until_a_pack_is_installed() {
    let mut a = app();
    let library = a.environment.paths().library().to_path_buf();
    assert!(!a.preset_browser.has_pack());
    let ctx = Context::default();
    let frame = |a: &mut Operandi| {
        let out = ctx.run(Default::default(), |ctx| {
            CentralPanel::default().show(ctx, |ui| a.preset_browser_body(ui, false));
        });
        drawn_text(&out)
    };
    let text = frame(&mut a);
    assert!(text.contains("Get the free Operandi Library…"), "{text}");
    assert!(text.contains("Locate Library Folder…"), "{text}");
    // Get… opens the page (in tests: says it would), downloads nothing.
    a.execute("operandi.pbview.libraryGet");
    assert!(
        a.status.contains(crate::preset_browser::LIBRARY_URL),
        "{}",
        a.status
    );
    assert!(!library.exists(), "nothing was written");
    // A pack unzipped into the library folder shows on the next check,
    // titled by its manifest, and the offer goes away.
    install_pack(&library.join("Operandi Library"), "2026.11.0");
    a.preset_browser.tick(1000.);
    let pack = library.join("Operandi Library");
    let root = a
        .preset_browser
        .roots
        .iter()
        .find(|r| r.path == pack)
        .cloned();
    let root = root.expect("pack added as a root");
    assert!(a.preset_browser.has_pack());
    assert_eq!(
        a.preset_browser.root_title(&root),
        "Operandi Library 2026.11.0"
    );
    let text = frame(&mut a);
    assert!(!text.contains("Get the free Operandi Library…"), "{text}");
    assert!(text.contains("Operandi Library 2026.11.0"), "{text}");
    // Each entry's tooltip carries its licence, credit and source.
    let entry = a.entry_for_path(&pack.join("Starter/Bricks/Bricks.oppreset"));
    let tip = a.entry_tip(&entry);
    for want in ["CC0-1.0", "ambientCG", "https://ambientcg.com/a/Bricks113A"] {
        assert!(tip.contains(want), "{want} missing from {tip}");
    }
    // Removed, it stays removed (it isn't re-added on the next check).
    a.preset_browser.remove_root(&pack);
    a.preset_browser.tick(2000.);
    assert!(!a.preset_browser.roots.iter().any(|r| r.path == pack));
    assert!(!a.preset_browser.has_pack());
    // Locate Library Folder… (pbview.basePathAdd) adds it back, from the
    // library folder that holds it.
    a.execute(&format!(
        "pbview.basePathAdd path:\"{}\"",
        library.display()
    ));
    assert!(a.preset_browser.roots.iter().any(|r| r.path == pack));
    assert!(
        a.status.contains("Operandi Library 2026.11.0"),
        "{}",
        a.status
    );
}

#[test]
fn f1_two_installed_versions_note_the_newer_one() {
    let mut a = app();
    let library = a.environment.paths().library().to_path_buf();
    let (old, new) = (
        library.join("Operandi Library"),
        library.join("Operandi Library 2026.12.0"),
    );
    install_pack(&old, "2026.11.0");
    install_pack(&new, "2026.12.0");
    a.preset_browser.tick(1000.);
    assert_eq!(
        a.preset_browser.newer_version(&old),
        Some(("2026.12.0".into(), new.clone()))
    );
    assert_eq!(a.preset_browser.newer_version(&new), None);
    let ctx = Context::default();
    let out = ctx.run(Default::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| a.preset_browser_body(ui, false));
    });
    let text = drawn_text(&out);
    assert!(
        text.contains("Operandi Library 2026.11.0 (newer: 2026.12.0)"),
        "{text}"
    );
    // A folder without a manifest is added as a plain folder.
    let plain = scratch(&a, "Plain");
    a.execute(&format!("pbview.basePathAdd path:\"{}\"", plain.display()));
    assert!(a.status.starts_with("Added "), "{}", a.status);
    assert!(a.preset_browser.roots.iter().any(|r| r.path == plain));
}

/// Registry ruling (2026-11-11): libraryLocate is gone (Locate… runs
/// pbview.basePathAdd), and libraryGet takes an optional url: and only
/// opens it, leaving the scene and its undo alone.
#[test]
fn f1_library_commands_follow_the_registry_ruling() {
    let mut a = app();
    let undo_before = a.editor.undo.len();
    a.execute("operandi.pbview.libraryGet url:https://example.com/lib");
    assert!(a.status.contains("https://example.com/lib"), "{}", a.status);
    a.execute("operandi.pbview.libraryGet");
    assert!(
        a.status.contains(crate::preset_browser::LIBRARY_URL),
        "{}",
        a.status
    );
    assert_eq!(a.editor.undo.len(), undo_before, "not undoable");
    a.execute("help operandi.pbview.libraryGet");
    assert!(a.status.contains("url:"), "{}", a.status);
    a.execute("operandi.pbview.libraryLocate");
    assert!(
        !a.status.starts_with("Added"),
        "libraryLocate is no command: {}",
        a.status
    );
    // A pack added with a label keeps the label; its packs are known, so
    // the library folder's scan doesn't add it twice.
    let pack = scratch(&a, "Labelled");
    install_pack(&pack, "2026.11.0");
    a.execute(&format!(
        "pbview.basePathAdd path:\"{}\" basePathKey:Mine",
        pack.display()
    ));
    let root = a.preset_browser.roots.iter().find(|r| r.path == pack);
    assert_eq!(root.and_then(|r| r.label.as_deref()), Some("Mine"));
}

/// Opt-in check against a real content folder: lists every folder below
/// `OPERANDI_LIBRARY` and renders a few texture sets on the ball into
/// `OPERANDI_LIBRARY_OUT` (needs `--features cycles` for the renders).
#[test]
#[ignore]
fn real_library_listing() {
    let Some(root) = std::env::var_os("OPERANDI_LIBRARY").map(PathBuf::from) else {
        return;
    };
    let out = std::env::var_os("OPERANDI_LIBRARY_OUT").map(PathBuf::from);
    let kind = |p: &Path| {
        operandi_core::persistence::read_preset_header(p)
            .ok()
            .map(|h| h.kind)
    };
    let mut todo = vec![root];
    let mut rendered = 0;
    while let Some(dir) = todo.pop() {
        let listing =
            operandi_core::preset::library::list_folder(&dir, Default::default(), &kind, &|| false)
                .unwrap();
        println!("{}", dir.display());
        for e in &listing.entries {
            let maps = e
                .texture_set
                .as_ref()
                .map(|s| {
                    s.maps
                        .iter()
                        .map(|m| {
                            format!(
                                "{:?}{}",
                                m.kind,
                                m.resolution.map(|r| format!("@{r}k")).unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            println!("    {:<40} {:<22} {maps}", e.name, e.kind.label());
            if let (Some(out), Some(set)) = (&out, &e.texture_set)
                && rendered < 4
                && let Ok((preset, _)) =
                    set.material_preset(Default::default(), &|p| image::image_dimensions(p).ok())
                && let Some(picture) = crate::preset_thumbs::render_preview(&preset)
            {
                rendered += 1;
                std::fs::write(
                    out.join(format!("{}.png", e.name)),
                    crate::preset_thumbs::png_bytes(&picture).unwrap(),
                )
                .unwrap();
            }
        }
        todo.extend(listing.folders);
    }
}

// ------------------------------------------------------------ matcaps

/// A Matcaps folder in a scratch root holding `names` (PNG pictures).
fn matcap_folder(a: &Operandi, names: &[&str]) -> PathBuf {
    let dir = scratch(a, "MatcapRoot").join("Matcaps").join("Clay");
    std::fs::create_dir_all(&dir).unwrap();
    for (i, name) in names.iter().enumerate() {
        image::RgbImage::from_pixel(8, 8, image::Rgb([40 * i as u8, 100, 160]))
            .save(dir.join(format!("{name}.png")))
            .unwrap();
    }
    dir
}

/// MC-01, MC-02, MC-04: a picture in a Matcaps folder lists as a Matcap;
/// loading it puts the active view in the Matcap style with it, and the
/// scene and its history don't change.
#[test]
fn mc_double_click_uses_the_matcap_in_the_active_view() {
    use crate::view_display::Style;
    let mut a = app();
    let dir = matcap_folder(&a, &["Jade", "Chrome"]);
    a.preset_browser.show_folder(dir.clone());
    listed(&mut a);
    let jade = a
        .preset_browser
        .entry(&dir.join("Jade.png"))
        .expect("listed");
    assert_eq!(jade.kind, EntryKind::Matcap);
    let before = a.editor.scene.clone();
    let undo = a.editor.undo.len();
    assert!(a.browser_load(&jade, Target::Default));
    assert_eq!(a.display.settings.active.style, Style::Reflection);
    assert_eq!(
        a.display.settings.drawing.matcap.path().as_deref(),
        Some(dir.join("Jade.png").as_path())
    );
    assert_eq!(a.status, "Matcap: Jade");
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.editor.undo.len(), undo);
    // The tile is round: its corners are see-through, its middle isn't.
    let tile = crate::preset_thumbs::matcap_picture(&jade.path).unwrap();
    assert_eq!(tile.pixels[0].a(), 0);
    let mid = tile.size[1] / 2 * tile.size[0] + tile.size[0] / 2;
    assert_eq!(tile.pixels[mid].a(), 255);
}

/// MC-03, MC-06: dropped on a view, only that view changes; Use in All
/// 3D Views changes every one.
#[test]
fn mc_a_drop_changes_only_its_view_and_use_in_all_changes_every_view() {
    use crate::view_display::Style;
    let mut a = crate::tests::box_app();
    a.load_preset_browser();
    let ctx = Context::default();
    for _ in 0..2 {
        crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    a.split_view();
    crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    a.activate_view(0);
    crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    let dir = matcap_folder(&a, &["Jade", "Chrome"]);
    let chrome = a.entry_for_path(&dir.join("Chrome.png"));
    assert_eq!(chrome.kind, EntryKind::Matcap);
    let at = a.corner.areas[&1].center();
    let before = a.editor.scene.clone();
    assert!(a.browser_load(&chrome, Target::Drop(at)));
    assert_eq!(a.active_view, 1);
    assert_eq!(a.display.settings.active.style, Style::Reflection);
    assert_eq!(
        a.display.settings.drawing.matcap.path().as_deref(),
        Some(chrome.path.as_path())
    );
    a.activate_view(0);
    assert_ne!(a.display.settings.active.style, Style::Reflection);
    assert_eq!(
        a.display.settings.drawing.matcap,
        crate::matcap::Choice::default()
    );
    assert_eq!(a.editor.scene, before);
    // MC-06.
    let jade = dir.join("Jade.png");
    assert!(a.use_matcap(&jade, None, true));
    a.activate_view(1);
    for _ in 0..2 {
        assert_eq!(a.display.settings.active.style, Style::Reflection);
        assert_eq!(
            a.display.settings.drawing.matcap.path().as_deref(),
            Some(jade.as_path())
        );
        a.activate_view(0);
    }
    assert_eq!(a.status, "Matcap Jade in all 3D views");
}

/// MC-08: a view whose matcap file is gone draws Clay Studio and the
/// status says so once.
#[test]
fn mc_a_missing_matcap_falls_back_to_clay_studio() {
    let mut a = app();
    let gone = scratch(&a, "Gone").join("Matcaps").join("Lost One.png");
    a.display.settings.drawing.matcap = crate::matcap::Choice::custom(&gone);
    a.note_missing_matcap();
    assert_eq!(a.status, "Matcap not found: Lost One");
    a.status = "Ready".into();
    a.note_missing_matcap();
    assert_eq!(a.status, "Ready");
    let clay = crate::matcap::BUILT_IN
        .iter()
        .position(|(k, ..)| *k == crate::matcap::MISSING)
        .unwrap();
    assert_eq!(
        a.display.settings.drawing.matcap.layer(),
        crate::matcap::Choice::BuiltIn(clay as u8).layer()
    );
}

/// MC-05: the library's matcaps are found under the roots in the
/// background.
#[test]
fn mc_the_library_matcaps_are_found_under_the_roots() {
    let mut a = app();
    let dir = matcap_folder(&a, &["Jade"]);
    let root = dir.parent().unwrap().parent().unwrap().to_path_buf();
    a.execute(&format!("pbview.basePathAdd path:\"{}\"", root.display()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut t = 100.;
    loop {
        t += 1.;
        a.preset_browser.scan_matcaps(t, false);
        if crate::matcap::library()
            .iter()
            .any(|m| m.path == dir.join("Jade.png"))
        {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "never found");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let found = crate::matcap::library();
    let jade = found
        .iter()
        .find(|m| m.path == dir.join("Jade.png"))
        .unwrap();
    assert_eq!((jade.group.as_str(), jade.name.as_str()), ("Clay", "Jade"));
}

/// PG-01, MC-09: a fresh Presets folder holds the five empty starter
/// folders, and an empty Meshes folder shows its hint.
#[test]
fn pg_01_a_fresh_presets_folder_has_the_starter_folders() {
    let a = app();
    let presets = a.environment.paths().presets();
    for name in operandi_core::preset::library::STARTER_FOLDERS {
        assert!(presets.join(name).is_dir(), "{name}");
    }
    assert!(crate::preset_browser::is_meshes_folder(
        &presets.join("Meshes")
    ));
    assert!(!crate::preset_browser::is_meshes_folder(
        &presets.join("Matcaps")
    ));
}

// ------------------------------------------------------------ meshes

fn obj_file(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(
        &path,
        "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nv 0 0 1\nv 1 0 1\nv 1 1 1\nv 0 1 1\n\
         f 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 4 8 5 1\n",
    )
    .unwrap();
    path
}

fn added_bounds(a: &Operandi, before: usize) -> (glam::DVec3, glam::DVec3) {
    let mut scene = a.editor.scene.clone();
    let kept: Vec<u64> = scene.items[before..].iter().map(|i| i.id).collect();
    scene.items.retain(|i| kept.contains(&i.id));
    operandi_core::preset::placement::mesh_bounds(&scene).unwrap()
}

/// PG-06: a model double-clicked or dragged in lands at the origin in one
/// step; Shift held at the drop rests it on what's under the pointer; a
/// scene dragged in joins this one, and with Cmd/Ctrl opens on its own.
#[test]
fn pg_06_models_land_at_the_origin_or_on_the_surface_with_shift() {
    let mut a = crate::tests::box_app();
    a.load_preset_browser();
    let ctx = Context::default();
    for _ in 0..2 {
        crate::tests::draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
    }
    let dir = scratch(&a, "Models");
    let obj = obj_file(&dir, "Crate.obj");
    let entry = a.entry_for_path(&obj);
    assert_eq!(entry.kind, EntryKind::Model);
    assert!(entry.kind.has_picture(), "PG-05: models get a thumbnail");
    let n = a.editor.scene.items.len();
    let undo = a.editor.undo.len();
    assert!(a.browser_load(&entry, Target::Default), "{}", a.status);
    assert_eq!(a.editor.undo.len(), undo + 1);
    assert_eq!(a.editor.scene.items.len(), n + 1);
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.items.len(), n);
    // Shift-drop onto the middle of the view.
    let at = a.corner.areas[&a.active_view].center();
    let point = a.drop_point(at).expect("something under the pointer");
    a.preset_browser.drop_modifiers = egui::Modifiers::SHIFT;
    assert!(a.browser_load(&entry, Target::Drop(at)), "{}", a.status);
    let (lo, hi) = added_bounds(&a, n);
    let base = glam::DVec3::new((lo.x + hi.x) / 2., lo.y, (lo.z + hi.z) / 2.);
    assert!((base - point).length() < 1e-6, "{base} vs {point}");
    a.run(Command::Undo);
    // A scene dragged in joins this one.
    let scene_file = dir.join("Other.opscene");
    persistence::save(&scene_file, &a.editor.scene).unwrap();
    let scene_entry = a.entry_for_path(&scene_file);
    a.preset_browser.drop_modifiers = egui::Modifiers::NONE;
    let count = a.editor.scene.items.len();
    assert!(a.browser_load(&scene_entry, Target::Drop(at)));
    assert!(a.editor.scene.items.len() > count);
}

/// PG-02..04: dropped selected meshes save as a Mesh Preset; Centre on
/// Ground saves the copy centred on X/Z and resting on Y = 0, and the
/// scene keeps its transform.
#[test]
fn pg_04_a_mesh_preset_can_be_saved_centred_on_the_ground() {
    let mut a = crate::tests::box_app();
    a.load_preset_browser();
    assert!(a.selection_is_meshes() || a.editor.scene.selection.item.is_none());
    let id = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh() && !i.mesh.vertices.is_empty())
        .unwrap()
        .id;
    a.editor.scene.select_item(Some(id));
    a.editor.scene.selection.mode = operandi_core::Mode::Item;
    assert!(a.selection_is_meshes());
    a.editor.scene.item_mut(id).unwrap().transform.translation = glam::DVec3::new(2., 5., 0.);
    let dir = a.environment.paths().presets().join("Meshes");
    let file = dir.join("Box.oppreset");
    a.execute(&format!(
        "preset.saveAs kind:item filename:\"{}\" centerOnGround:true",
        file.display()
    ));
    assert!(file.is_file(), "{}", a.status);
    let saved = persistence::load_preset(&file).unwrap();
    let (lo, hi) = operandi_core::preset::placement::mesh_bounds(&saved.scene).unwrap();
    assert!(lo.y.abs() < 1e-9, "{lo}");
    assert!((lo.x + hi.x).abs() < 1e-9 && (lo.z + hi.z).abs() < 1e-9);
    // The scene's item still sits where it was.
    let t = a.editor.scene.item(id).unwrap().transform.translation;
    assert_eq!(t, glam::DVec3::new(2., 5., 0.));
    // The option is off unless asked for, and a bad value is refused.
    a.execute(&format!(
        "preset.saveAs kind:item filename:\"{}\" centerOnGround:sometimes",
        dir.join("Bad.oppreset").display()
    ));
    assert!(!dir.join("Bad.oppreset").exists());
    assert!(a.status.contains("centerOnGround"), "{}", a.status);
    assert!(!a.preset_browser.save_centered);
}

/// Every folder below `root`, `root` first.
fn all_folders(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![root.to_path_buf()];
    let mut i = 0;
    while i < out.len() {
        let mut subs: Vec<PathBuf> = std::fs::read_dir(&out[i])
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        subs.sort();
        out.extend(subs);
        i += 1;
    }
    out
}

/// The library pack check (scripts/build-library-pack.sh): point the
/// browser at an unzipped pack wherever it was put and load everything it
/// lists. Run with
/// `OPERANDI_LIBRARY_PACK=<folder>/"Operandi Library" cargo test --release
/// -p operandi-app library_pack_loads_from_its_own_folder -- --ignored --nocapture`.
#[test]
#[ignore = "needs an unzipped library pack in OPERANDI_LIBRARY_PACK"]
fn library_pack_loads_from_its_own_folder() {
    let pack =
        PathBuf::from(std::env::var("OPERANDI_LIBRARY_PACK").expect("OPERANDI_LIBRARY_PACK"));
    let mut a = app();
    a.preset_browser.add_root(pack.clone(), None);
    a.preset_browser.read_manifests();
    assert!(
        a.preset_browser.manifest(&pack).is_some(),
        "the pack's manifest reads"
    );
    let (mut materials, mut matcaps, mut environments, mut meshes) = (0, 0, 0, 0);
    let mut tried_category = std::collections::HashSet::new();
    for folder in all_folders(&pack) {
        a.preset_browser.show_folder(folder.clone());
        listed(&mut a);
        for e in a.preset_browser.shown() {
            let credited = a
                .preset_browser
                .pack_item(&e.path)
                .map(|(_, i)| i.licence.clone());
            match e.kind {
                EntryKind::Preset(PresetKind::Material) => {
                    materials += 1;
                    assert_eq!(credited.as_deref(), Some("CC0-1.0"), "{}", e.path.display());
                    let preset = operandi_core::persistence::load_preset(&e.path).unwrap();
                    for image in &preset.scene.images {
                        let p = Path::new(&image.path);
                        assert!(
                            p.starts_with(&pack),
                            "{} names {}",
                            e.path.display(),
                            image.path
                        );
                        assert!(p.is_file(), "{} misses {}", e.path.display(), image.path);
                    }
                    // One material per category goes through preset.do.
                    if tried_category.insert(folder.clone()) {
                        let group = add_layer(&mut a, NewLayer::Group, None);
                        let name = a.editor.scene.shader_tree.get(group).unwrap().name.clone();
                        a.execute(&format!(
                            "preset.do preset:\"{}\" target:\"{name}\"",
                            e.path.display()
                        ));
                        let layers = a.editor.scene.shader_tree.children(Some(group)).count();
                        assert!(layers > 0, "{}: {}", e.path.display(), a.status);
                    }
                }
                EntryKind::Matcap => {
                    matcaps += 1;
                    assert_eq!(credited.as_deref(), Some("CC0-1.0"), "{}", e.path.display());
                    assert!(
                        crate::preset_thumbs::matcap_picture(&e.path).is_some(),
                        "{} decodes",
                        e.path.display()
                    );
                }
                EntryKind::Environment => {
                    environments += 1;
                    assert_eq!(credited.as_deref(), Some("CC0-1.0"), "{}", e.path.display());
                    assert!(
                        crate::preset_thumbs::picture_file(&e.path).is_some(),
                        "{} decodes",
                        e.path.display()
                    );
                    let before = a.editor.scene.clone();
                    a.execute(&format!("preset.do preset:\"{}\"", e.path.display()));
                    assert!(
                        a.editor
                            .scene
                            .images
                            .iter()
                            .any(|i| Path::new(&i.path) == e.path),
                        "{}: {}",
                        e.path.display(),
                        a.status
                    );
                    a.run(Command::Undo);
                    assert!(a.editor.scene == before, "one undo restores");
                }
                EntryKind::Preset(PresetKind::Item) => {
                    // 2026.11.5: our own CC0 shapes, thumbnail inside, and
                    // each drops into the scene as one undo step.
                    meshes += 1;
                    assert_eq!(credited.as_deref(), Some("CC0-1.0"), "{}", e.path.display());
                    let preset = operandi_core::persistence::load_preset(&e.path).unwrap();
                    assert!(
                        operandi_core::persistence::preset_thumbnail(&preset.header).is_some(),
                        "{} has its thumbnail",
                        e.path.display()
                    );
                    let before = a.editor.scene.clone();
                    a.execute(&format!("preset.do preset:\"{}\"", e.path.display()));
                    assert_eq!(
                        a.editor.scene.items.len(),
                        before.items.len() + 1,
                        "{}: {}",
                        e.path.display(),
                        a.status
                    );
                    a.run(Command::Undo);
                    assert!(a.editor.scene == before, "one undo restores");
                }
                EntryKind::Preset(PresetKind::Assembly)
                | EntryKind::Model
                | EntryKind::ModoMeshPreset => meshes += 1,
                _ => {}
            }
        }
    }
    // MC-05: the view's Matcap menu finds the pack's matcaps too.
    a.preset_browser.scan_matcaps(0., true);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !a.preset_browser.scan_matcaps(1., false) {
        assert!(
            std::time::Instant::now() < deadline,
            "matcap scan never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let in_menu = crate::matcap::library()
        .iter()
        .filter(|m| m.path.starts_with(&pack))
        .count();
    println!(
        "pack {}: {materials} materials, {matcaps} matcaps ({in_menu} in the Matcap menu), \
         {environments} environments, {meshes} meshes",
        pack.display()
    );
    assert!(materials > 0 && matcaps > 0 && environments > 0);
    assert_eq!(in_menu, matcaps);
}
