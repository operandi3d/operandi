//! Environment presets in the app (EP, HDRI stage 6): the save dialog,
//! the commands, and Replace / Add from the browser.
use super::*;
use crate::preset_browser::Target;
use operandi_core::preset::PresetKind;
use operandi_core::preset::library::{Entry, EntryKind};
use std::path::Path;

fn folder(a: &Operandi, name: &str) -> PathBuf {
    let f = a.environment.paths().data_root().join(name);
    std::fs::create_dir_all(&f).unwrap();
    f
}

/// A scene with one environment and one area light, saved as `name`.
fn saved_preset(a: &mut Operandi, name: &str) -> PathBuf {
    let dir = folder(a, "env-presets");
    let mut source = Operandi::empty();
    source.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Environment,
    ));
    source.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Light(operandi_core::camera_light::LightKind::Area),
    ));
    let path = dir.join(format!("{name}.openv"));
    source.run_command_line(&format!(
        "preset.saveAs kind:environment filename:{{{}}} lights:true",
        path.display()
    ));
    assert!(path.exists(), "{}", source.status);
    path
}

fn entry(path: &Path) -> Entry {
    Entry {
        name: path.file_stem().unwrap().to_string_lossy().into_owned(),
        path: path.to_path_buf(),
        kind: EntryKind::Preset(PresetKind::Environment),
        modified: None,
        size: 0,
        texture_set: None,
        modo_kind: None,
    }
}

#[test]
fn the_menu_opens_the_options_dialog_with_meshes_greyed_without_a_mesh() {
    let mut a = Operandi::empty();
    a.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Environment,
    ));
    a.execute("preset.saveAs kind:environment");
    let dialog = a.env_preset_dialog.as_ref().expect("the options dialog");
    assert_eq!(dialog.form.title, "Save Environment Preset");
    let labels: Vec<&str> = dialog.form.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Include Lights",
            "Include Selected Meshes",
            "Include Cameras",
            "Copy Images Next To Preset"
        ]
    );
    assert!(dialog.form.values.bool("lights") && dialog.form.values.bool("copy"));
    assert!(
        !dialog.form.values.bool("cameras"),
        "Include Cameras starts off"
    );
    let meshes = &dialog.form.rows[1];
    assert!(
        !meshes.when.holds(&dialog.form.values),
        "greyed, not hidden"
    );
    // With a mesh selected it is live.
    let mut a = super::tests::box_app();
    a.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Environment,
    ));
    let mesh = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.is_mesh() && i.in_item_list())
        .unwrap()
        .id;
    a.run(Command::SelectItem(Some(mesh)));
    a.execute("preset.saveAs kind:environment");
    let dialog = a.env_preset_dialog.as_ref().unwrap();
    assert!(dialog.form.rows[1].when.holds(&dialog.form.values));
}

#[test]
fn double_click_replaces_and_preset_do_mode_add_adds() {
    let mut a = Operandi::empty();
    let path = saved_preset(&mut a, "Studio");
    a.run(Command::AddItem(
        operandi_core::item_list::AddItemKind::Environment,
    ));
    let own = a.editor.scene.environments().next().unwrap().id;
    let steps = a.editor.undo.len();
    // Double-click: Replace, one step, the new top environment selected.
    assert!(a.browser_load(&entry(&path), Target::Default));
    assert_eq!(a.editor.undo.len(), steps + 1);
    assert!(a.editor.scene.items.iter().all(|i| i.id != own));
    let top = a.editor.scene.environments().next().unwrap().id;
    assert_eq!(
        a.shading.environment,
        Some(crate::environment_ui::EnvironmentRow::Item(top))
    );
    let light = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| i.light.is_some())
        .unwrap();
    // EP v0.2: the light is in the "Studio" item group, which is marked.
    let group = a
        .editor
        .scene
        .items
        .iter()
        .find(|i| Some(i.id) == light.parent)
        .unwrap();
    assert_eq!(group.name, "Studio");
    assert_eq!(group.common.from_preset.as_deref(), Some("Studio"));
    // preset.do mode:add puts a second copy on top.
    a.run_command_line(&format!("preset.do preset:{{{}}} mode:add", path.display()));
    assert_eq!(a.editor.scene.environments().count(), 2, "{}", a.status);
    assert_ne!(a.editor.scene.environments().next().unwrap().id, top);
    a.run_command_line(&format!(
        "preset.do preset:{{{}}} mode:sideways",
        path.display()
    ));
    assert!(a.status.contains("replace or add"), "{}", a.status);
}

#[test]
fn a_drop_on_a_view_asks_replace_or_add_and_gi_off_gives_a_hint_once() {
    let mut a = Operandi::empty();
    let path = saved_preset(&mut a, "Sky");
    let view = Rect::from_min_size(Pos2::ZERO, vec2(400., 300.));
    a.corner.areas.insert(a.active_view, view);
    let before = a.editor.scene.clone();
    assert!(a.browser_load(&entry(&path), Target::Drop(pos2(50., 50.))));
    assert_eq!(a.editor.scene, before, "nothing changes until a choice");
    assert!(a.env_drop_menu.is_some());
    a.env_drop_menu = None;
    a.preview.options.light_paths.diffuse = false;
    assert!(a.apply_environment_preset_from(&path, operandi_core::preset::ApplyMode::Add, false));
    assert!(
        a.status.contains(crate::environment_preset_ui::GI_HINT),
        "{}",
        a.status
    );
    assert!(a.apply_environment_preset_from(&path, operandi_core::preset::ApplyMode::Add, false));
    assert!(
        !a.status.contains(crate::environment_preset_ui::GI_HINT),
        "once"
    );
}

#[test]
fn a_missing_picture_loads_with_one_message() {
    let mut a = Operandi::empty();
    let dir = folder(&a, "env-missing");
    let picture = dir.join("gone.hdr");
    std::fs::write(&picture, b"stand-in").unwrap();
    let mut source = Operandi::empty();
    source.run(Command::SetEnvironmentImage {
        path: picture.to_string_lossy().into_owned(),
        width: 8,
        height: 4,
        item: None,
    });
    let elsewhere = folder(&a, "env-missing-presets");
    let path = elsewhere.join("Gone.openv");
    source.run_command_line(&format!(
        "preset.saveAs kind:environment filename:{{{}}}",
        path.display()
    ));
    std::fs::remove_file(&picture).unwrap();
    assert!(a.apply_environment_preset_from(
        &path,
        operandi_core::preset::ApplyMode::Replace,
        false
    ));
    assert!(a.status.contains("Missing pictures"), "{}", a.status);
    assert!(a.status.contains("gone.hdr"));
    let env = a.editor.scene.environments().next().unwrap().id;
    assert!(a.editor.scene.environment_image(env).is_some(), "path kept");
}
