//! MaterialX in the app: File menu import and export, the commands, the
//! refusal leaving the scene alone, and the preset save option.
use super::file_chooser::Choose;
use super::*;
use operandi_core::preset::PresetSource;
use operandi_core::shader_tree::{NewLayer, NodeKind, TextureEffect};
use std::cell::RefCell;
use std::path::Path;

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

/// A folder with two small pictures and a .mtlx reading them.
fn material_folder(a: &Operandi, name: &str, extra_node: &str) -> PathBuf {
    let dir = a.environment.paths().data_root().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["Wall_Color.png", "Wall_Roughness.png"] {
        image::RgbImage::from_pixel(2, 2, image::Rgb([120, 90, 60]))
            .save(dir.join(f))
            .unwrap();
    }
    let rough = if extra_node.is_empty() {
        "<input name=\"specular_roughness\" type=\"float\" nodename=\"rough\" />"
    } else {
        "<input name=\"specular_roughness\" type=\"float\" nodename=\"noise\" />"
    };
    let text = format!(
        r#"<?xml version="1.0"?>
<materialx version="1.39">
  <image name="col" type="color3"><input name="file" type="filename" value="Wall_Color.png" colorspace="srgb_texture"/></image>
  <image name="rough" type="float"><input name="file" type="filename" value="Wall_Roughness.png"/></image>
  {extra_node}
  <open_pbr_surface name="s" type="surfaceshader">
    <input name="base_color" type="color3" nodename="col" />
    {rough}
    <input name="coat_weight" type="float" value="0.25" />
  </open_pbr_surface>
  <surfacematerial name="Wall" type="material"><input name="surfaceshader" type="surfaceshader" nodename="s"/></surfacematerial>
</materialx>"#
    );
    std::fs::write(dir.join("Wall.mtlx"), text).unwrap();
    dir
}

fn group(a: &mut Operandi) -> u64 {
    a.run(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }));
    let id = a.editor.scene.shader_tree.next_id - 1;
    a.shading.selected = vec![id];
    id
}

/// Import `file` at the top of the tree; the new material row's id.
fn import_top(a: &mut Operandi, file: &Path) -> u64 {
    a.execute(&format!(
        "operandi.materialx.import filename:\"{}\"",
        file.display()
    ));
    assert!(a.status.starts_with("Loaded MaterialX"), "{}", a.status);
    newest_group(a)
}

/// The newest top-level Material Group: where an import with no layer
/// selected goes (below the Base Shader, as Add Layer puts a group).
fn newest_group(a: &Operandi) -> u64 {
    a.editor
        .scene
        .shader_tree
        .children(None)
        .filter(|n| matches!(n.kind, NodeKind::Group(_)))
        .map(|n| n.id)
        .max()
        .unwrap()
}

fn effects(a: &Operandi) -> Vec<TextureEffect> {
    a.editor
        .scene
        .shader_tree
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Texture(t) => Some(t.effect),
            _ => None,
        })
        .collect()
}

#[test]
fn file_menu_imports_materialx_above_the_selected_layer_in_one_step() {
    let ctx = Context::default();
    let mut a = app();
    let dir = material_folder(&a, "Import", "");
    let g = group(&mut a);
    let steps = a.editor.undo.len();
    answer(Some(dir.join("Wall.mtlx")));
    a.menu_action(&ctx, app_menu::line_action("operandi.materialx.import"));
    assert!(
        matches!(&asked()[..], [Choose::MaterialX { .. }]),
        "{:?}",
        asked()
    );
    assert_eq!(
        effects(&a),
        [TextureEffect::DiffuseColor, TextureEffect::Roughness],
        "{}",
        a.status
    );
    assert!(a.status.starts_with("Loaded MaterialX"), "{}", a.status);
    assert_eq!(a.editor.undo.len(), steps + 1, "one undo step");
    // The menu passes the selected layer: the new material sits directly
    // above it, in its group (here the top level), and the group is kept.
    let tree = &a.editor.scene.shader_tree;
    let tops: Vec<_> = tree.children(None).map(|n| n.id).collect();
    let at = tops.iter().position(|&id| id == g).unwrap();
    assert!(at > 0, "{tops:?}");
    let new = tops[at - 1];
    assert!(tree.children(Some(g)).next().is_none(), "nothing replaced");
    assert!(
        tree.nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Texture(_)))
            .all(|n| n
                .parent
                .is_some_and(|p| p == new || tree.get(p).is_some_and(|q| q.parent == Some(new)))),
        "the layers came in their own group above the selected layer"
    );
    a.run(Command::Undo);
    assert!(effects(&a).is_empty());
    a.run(Command::Redo);
    assert_eq!(effects(&a).len(), 2);
}

#[test]
fn a_material_needing_nodes_is_refused_and_nothing_changes() {
    let mut a = app();
    let dir = material_folder(
        &a,
        "Refuse",
        // A node the MaterialX graph translation doesn't know yet
        // (materials slice 1 renders noise, so it no longer refuses).
        "<hextiledimage name=\"noise\" type=\"float\"><input name=\"file\" type=\"filename\" value=\"n.png\"/></hextiledimage>",
    );
    group(&mut a);
    let (before, steps) = (a.editor.scene.clone(), a.editor.undo.len());
    a.execute(&format!(
        "operandi.materialx.import filename:\"{}\"",
        dir.join("Wall.mtlx").display()
    ));
    assert!(
        a.status.contains(operandi_core::materialx::NEEDS_NODES) && a.status.contains("noise"),
        "{}",
        a.status
    );
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.editor.undo.len(), steps);
}

#[test]
fn export_writes_the_selected_material_and_reads_back() {
    let ctx = Context::default();
    let mut a = app();
    let dir = material_folder(&a, "Export", "");
    let m = import_top(&mut a, &dir.join("Wall.mtlx"));
    a.shading.selected = vec![m];
    let out = dir.join("Out.mtlx");
    answer(Some(out.clone()));
    a.menu_action(&ctx, app_menu::line_action("operandi.materialx.export"));
    assert!(
        matches!(&asked()[..], [Choose::SaveMaterialX { .. }]),
        "{:?}",
        asked()
    );
    assert!(out.is_file(), "{}", a.status);
    assert!(a.status.starts_with("Exported MaterialX"), "{}", a.status);
    // Picture paths are relative to the file's folder.
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("value=\"Wall_Color.png\""), "{text}");
    let (back, _) =
        operandi_core::materialx::material_preset(&out, None, &|p| image::image_dimensions(p).ok())
            .unwrap();
    assert_eq!(
        back.scene
            .shader_tree
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Texture(_)))
            .count(),
        2
    );
    // Nothing selected: the command says so.
    a.shading.selected.clear();
    a.execute("operandi.materialx.export filename:/tmp/x.mtlx");
    assert!(a.status.starts_with("Select a material"), "{}", a.status);
}

#[test]
fn saving_a_material_preset_can_write_materialx_beside_it() {
    let mut a = app();
    let dir = material_folder(&a, "Sidecar", "");
    let g = import_top(&mut a, &dir.join("Wall.mtlx"));
    let preset = dir.join("Saved.oppreset");
    a.save_preset_to(preset.clone(), PresetSource::Material(g), false);
    assert!(
        preset.is_file() && !dir.join("Saved.mtlx").exists(),
        "off by default"
    );
    a.save_preset_to(dir.join("Saved2.oppreset"), PresetSource::Material(g), true);
    assert!(dir.join("Saved2.mtlx").is_file(), "{}", a.status);
    assert!(a.status.contains("MaterialX written"), "{}", a.status);
}

#[test]
fn import_args_parse_and_place_the_layers() {
    let mut a = app();
    let dir = material_folder(&a, "Args", "");
    let file = dir.join("Wall.mtlx");
    let g = group(&mut a);
    // Inside group g: a layer to import beside.
    a.run(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: Some(g),
    }));
    let inner = a.editor.scene.shader_tree.next_id - 1;
    assert_eq!(
        a.editor.scene.shader_tree.get(inner).unwrap().parent,
        Some(g)
    );
    let steps = a.editor.undo.len();
    a.execute(&format!(
        "operandi.materialx.import filename:\"{}\" layer:{inner}",
        file.display()
    ));
    assert_eq!(
        a.editor.undo.len(),
        steps + 1,
        "one undo step: {}",
        a.status
    );
    let tree = &a.editor.scene.shader_tree;
    let kids: Vec<_> = tree.children(Some(g)).map(|n| n.id).collect();
    assert_eq!(kids.len(), 2, "{kids:?}");
    assert_eq!(kids[1], inner, "new layers directly above layer:");
    // No layer: where Add Layer puts a group.
    a.execute(&format!(
        "operandi.materialx.import filename:\"{}\"",
        file.display()
    ));
    assert_eq!(a.editor.undo.len(), steps + 2, "{}", a.status);
    assert!(newest_group(&a) > inner, "a new top-level group");
    assert_eq!(effects(&a).len(), 4);
    // Undo takes each import back as one step.
    a.run(Command::Undo);
    a.run(Command::Undo);
    assert!(effects(&a).is_empty());
    // A layer that isn't there: a message, nothing changes.
    let (before, steps) = (a.editor.scene.clone(), a.editor.undo.len());
    a.execute(&format!(
        "operandi.materialx.import filename:\"{}\" layer:9999",
        file.display()
    ));
    assert!(
        a.status.contains("no shader tree layer 9999"),
        "{}",
        a.status
    );
    assert_eq!(
        (a.editor.scene.clone(), a.editor.undo.len()),
        (before, steps)
    );
    // No filename: the file panel, which keeps layer:.
    answer(None);
    a.execute(&format!("operandi.materialx.import layer:{g}"));
    assert!(
        matches!(&asked()[..], [Choose::MaterialX { .. }]),
        "{:?}",
        asked()
    );
}

#[test]
fn export_takes_an_item_and_is_not_undoable() {
    let mut a = app();
    let dir = material_folder(&a, "ExportItem", "");
    let g = import_top(&mut a, &dir.join("Wall.mtlx"));
    a.shading.selected.clear();
    let name = a.editor.scene.shader_tree.get(g).unwrap().name.clone();
    let steps = a.editor.undo.len();
    let by_id = dir.join("ById.mtlx");
    a.execute(&format!(
        "operandi.materialx.export filename:\"{}\" item:{g}",
        by_id.display()
    ));
    assert!(by_id.is_file(), "{}", a.status);
    let by_name = dir.join("ByName.mtlx");
    a.execute(&format!(
        "operandi.materialx.export filename:\"{}\" item:\"{name}\"",
        by_name.display()
    ));
    assert!(by_name.is_file(), "{}", a.status);
    assert_eq!(a.editor.undo.len(), steps, "export adds no undo step");
    // Not a material: readable text, no file.
    let none = dir.join("None.mtlx");
    a.execute(&format!(
        "operandi.materialx.export filename:\"{}\" item:Nope",
        none.display()
    ));
    assert!(
        a.status.contains("Nope isn't a material or material group"),
        "{}",
        a.status
    );
    assert!(!none.exists());
    // Nothing given and nothing selected.
    a.execute(&format!(
        "operandi.materialx.export filename:\"{}\"",
        none.display()
    ));
    assert!(a.status.starts_with("Select a material"), "{}", a.status);
    assert!(!none.exists());
}

#[test]
fn preset_save_as_mtlx_true_writes_a_mtlx() {
    let mut a = app();
    let dir = material_folder(&a, "SaveAsMtlx", "");
    let g = import_top(&mut a, &dir.join("Wall.mtlx"));
    a.shading.selected = vec![g];
    let off = dir.join("Off.oppreset");
    a.execute(&format!(
        "preset.saveAs kind:material filename:\"{}\"",
        off.display()
    ));
    assert!(off.is_file(), "{}", a.status);
    assert!(!dir.join("Off.mtlx").exists(), "mtlx: defaults to off");
    let on = dir.join("On.oppreset");
    a.execute(&format!(
        "preset.saveAs kind:material filename:\"{}\" mtlx:true",
        on.display()
    ));
    assert!(dir.join("On.mtlx").is_file(), "{}", a.status);
    assert!(a.status.contains("MaterialX written"), "{}", a.status);
    a.execute("preset.saveAs kind:material filename:/tmp/x.oppreset mtlx:maybe");
    assert!(
        a.status.contains("mtlx: takes true or false"),
        "{}",
        a.status
    );
    // The gear toggle puts mtlx:true on the menu's line.
    a.preset_browser.write_materialx = true;
    assert_eq!(
        a.preset_save_line("preset.saveAs"),
        "preset.saveAs mtlx:true"
    );
    a.preset_browser.write_materialx = false;
    assert_eq!(a.preset_save_line("preset.saveAs"), "preset.saveAs");
}
