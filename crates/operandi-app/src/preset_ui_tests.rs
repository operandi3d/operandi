//! Presets Stage 0 in the app: the File menu, the right-click routes and
//! the commands reach the same save and load (preset_ui.rs).
use super::file_chooser::Choose;
use super::*;
use operandi_core::preset::{PresetKind, PresetSource};
use operandi_core::shader_tree::{NewLayer, NodeKind};
use std::cell::RefCell;

thread_local! {
    static ANSWER: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static ASKED: RefCell<Vec<Choose>> = const { RefCell::new(Vec::new()) };
}

fn stand_in(request: Choose) -> Option<PathBuf> {
    ASKED.with(|a| a.borrow_mut().push(request));
    ANSWER.with(|a| a.borrow_mut().take())
}

fn answer(path: Option<PathBuf>) {
    ANSWER.with(|a| *a.borrow_mut() = path);
    ASKED.with(|a| a.borrow_mut().clear());
}

fn asked() -> Vec<Choose> {
    ASKED.with(|a| a.borrow().clone())
}

fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.chooser = Some(stand_in);
    a
}

fn add_layer(a: &mut Operandi, layer: NewLayer, selected: Option<u64>) -> u64 {
    a.run(Command::ShaderTree(ShaderTreeEdit::Add { layer, selected }));
    a.editor.scene.shader_tree.next_id - 1
}

#[test]
fn file_menu_saves_an_item_preset_in_the_presets_folder_and_loads_it() {
    let ctx = Context::default();
    let mut a = app();
    a.run(Command::NewMesh);
    let presets = a.environment.paths().presets();
    assert!(!presets.exists(), "made on first save, not before");
    let file = presets.join("Cube.oppreset");
    answer(Some(file.clone()));
    let before = a.editor.scene.clone();
    a.menu_action(&ctx, app_menu::line_action("preset.saveAs"));
    let [
        Choose::SavePreset {
            folder,
            name,
            title,
        },
    ] = &asked()[..]
    else {
        panic!("{:?}", asked());
    };
    assert_eq!(folder, &presets);
    assert!(name.ends_with(".oppreset"), "{name}");
    assert_eq!(title, "Save Item Preset");
    assert!(file.is_file(), "{}", a.status);
    assert_eq!(a.editor.scene, before, "saving changes nothing");
    assert!(a.status.starts_with("Saved Item preset"), "{}", a.status);

    // Load it into a new scene: one undo step.
    a.initialize_new_document();
    let (items, steps) = (a.editor.scene.items.len(), a.editor.undo.len());
    answer(Some(file.clone()));
    a.menu_action(&ctx, app_menu::line_action("preset.do"));
    assert_eq!(asked(), vec![Choose::LoadPreset { folder: presets }]);
    assert_eq!(a.editor.scene.items.len(), items + 1, "{}", a.status);
    assert_eq!(a.editor.undo.len(), steps + 1);
    // A cancelled panel does nothing.
    answer(None);
    a.menu_action(&ctx, app_menu::line_action("preset.do"));
    assert_eq!(a.editor.scene.items.len(), items + 1);
}

#[test]
fn file_menu_lists_the_preset_entries_with_ellipses() {
    let a = app();
    let ctx = Context::default();
    let titles: Vec<String> = a.menu_model(&ctx)[0]
        .leaves()
        .into_iter()
        .map(|i| i.title.clone())
        .collect();
    for title in [
        "Load Preset…",
        "Save Item Preset…",
        "Save Assembly Preset…",
        "Save Environment Preset…",
    ] {
        assert!(titles.iter().any(|t| t == title), "{title} in {titles:?}");
    }
}

#[test]
fn nothing_to_save_is_said_before_any_panel_opens() {
    let ctx = Context::default();
    let mut a = app();
    a.editor
        .scene
        .items
        .retain(|i| i.light.is_none() && i.environment.is_none());
    a.editor.scene.select_item(None);
    answer(Some(std::env::temp_dir().join("never.oppreset")));
    a.menu_action(
        &ctx,
        app_menu::line_action("preset.saveAs kind:environment"),
    );
    assert!(asked().is_empty());
    assert!(a.status.contains("no environment to save"), "{}", a.status);
    a.menu_action(&ctx, app_menu::line_action("preset.saveAs"));
    assert!(asked().is_empty());
    assert!(
        a.status.contains("Select one or more items"),
        "{}",
        a.status
    );
}

#[test]
fn save_item_preset_with_the_shading_tab_saves_the_material_group() {
    let ctx = Context::default();
    let mut a = app();
    a.run(Command::NewMesh);
    let group = add_layer(&mut a, NewLayer::Group, None);
    add_layer(&mut a, NewLayer::Constant, Some(group));
    add_layer(&mut a, NewLayer::Constant, Some(group));
    a.shading.selected = vec![group];
    // Scenes tab in front: the selected item is saved.
    assert_eq!(a.item_preset_source(), PresetSource::Items);
    a.upper_frame.show_page(1);
    assert!(a.shading_tab());
    assert_eq!(a.item_preset_source(), PresetSource::Material(group));
    let file = a.environment.paths().presets().join("Layers.oppreset");
    answer(Some(file.clone()));
    a.menu_action(&ctx, app_menu::line_action("preset.saveAs"));
    let preset = operandi_core::persistence::load_preset(&file).unwrap();
    assert_eq!(preset.header.kind, PresetKind::Material);

    // PB-T1 through the app: a new scene, a material group selected.
    a.initialize_new_document();
    let into = add_layer(&mut a, NewLayer::Group, None);
    // Nothing selected in the shader tree: refused, nothing changes.
    a.shading.selected.clear();
    let before = a.editor.scene.clone();
    a.load_preset_from(file.clone(), a.preset_target_group(), false);
    assert_eq!(a.editor.scene, before);
    assert!(a.status.contains("material group"), "{}", a.status);
    a.shading.selected = vec![into];
    answer(Some(file));
    a.menu_action(&ctx, app_menu::line_action("preset.do"));
    let textures = a
        .editor
        .scene
        .shader_tree
        .children(Some(into))
        .filter(|n| matches!(n.kind, NodeKind::Texture(_)))
        .count();
    assert_eq!(textures, 2, "{}", a.status);
}

#[test]
fn commands_save_and_load_by_path() {
    let mut a = app();
    a.run(Command::NewMesh);
    a.run(Command::NewLight(
        operandi_core::camera_light::LightKind::Spot,
    ));
    let dir = a.environment.paths().data_root().join("typed");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("lights");
    a.execute(&format!(
        "preset.save kind:environment path:\"{}\" lights:true",
        file.display()
    ));
    // Environment presets are .openv (EP); lights: is off by default.
    let saved = dir.join("lights.openv");
    assert!(saved.is_file(), "the extension is added: {}", a.status);
    a.initialize_new_document();
    a.execute(&format!("preset.load path:\"{}\"", saved.display()));
    assert!(
        a.editor.scene.items.iter().any(|i| i.light.is_some()),
        "{}",
        a.status
    );
    a.execute("preset.save kind:nonsense");
    assert!(a.status.contains("preset.saveAs"), "{}", a.status);
    a.execute("help preset.load");
    assert!(a.status.starts_with("Help: preset.do"), "{}", a.status);
}

#[test]
fn opening_a_preset_file_loads_it_into_the_current_scene() {
    let mut a = app();
    a.run(Command::NewMesh);
    let file = a.environment.paths().data_root().join("drop.oppreset");
    a.editor.save_preset(&file, PresetSource::Items).unwrap();
    a.initialize_new_document();
    let items = a.editor.scene.items.len();
    let scenes_before = a.scene_menu_entries().len();
    a.open(file);
    assert_eq!(a.editor.scene.items.len(), items + 1, "{}", a.status);
    assert_eq!(a.scene_menu_entries().len(), scenes_before, "no new scene");
}
