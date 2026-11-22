//! Material editing items 5–7: image thumbnails in the Image Map's Image
//! list, the Image Viewer from the Image field and shader tree rows, and
//! material presets dropped on a material group row.
use super::*;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An app with the image-layers fixture (four scene images, Image Map
/// layers), image paths made absolute.
fn image_app() -> Operandi {
    let root = repo_root();
    let path = root.join("fixtures/viewport-image-textures/image-layers.opscene");
    let mut a = Operandi::empty();
    let mut scene = operandi_core::persistence::load(&path).unwrap();
    for image in &mut scene.images {
        if std::path::Path::new(&image.path).is_relative() {
            image.path = root.join(&image.path).to_string_lossy().into_owned();
        }
    }
    a.editor.scene = scene;
    a
}

#[test]
fn image_thumbnails_decode_once_in_the_background() {
    let mut a = image_app();
    let ctx = egui::Context::default();
    let path = a.editor.scene.images[0].path.clone();
    // First ask: not ready, queued (never decoded on the UI thread).
    assert!(a.image_thumbnail(&path).is_none());
    let start = std::time::Instant::now();
    let texture = loop {
        a.preset_browser.thumbs.poll(&ctx);
        if let Some(t) = a.image_thumbnail(&path) {
            break t;
        }
        assert!(start.elapsed().as_secs() < 20, "thumbnail never arrived");
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let size = texture.size();
    assert!(size[0] <= 192 && size[1] <= 192 && size[0] > 0);
    // Asked again: the same texture from memory.
    assert_eq!(a.image_thumbnail(&path).unwrap().id(), texture.id());
    assert!(a.image_thumbnail("").is_none());
}

#[test]
fn image_viewer_shows_a_scene_image_once() {
    let mut a = image_app();
    let path = std::path::PathBuf::from(&a.editor.scene.images[0].path);
    let before = a.render.history.slots.len();
    a.show_image_in_viewer(&path);
    assert!(a.render.open, "{}", a.status);
    assert_eq!(a.render.history.slots.len(), before + 1, "{}", a.status);
    let slot = a.render.history.slots.first().unwrap().clone();
    assert_eq!(slot.kind, crate::image_viewer::SlotKind::Loaded);
    assert_eq!(slot.saved.as_deref(), Some(path.as_path()));
    assert_eq!(a.render.viewer.primary, Some(slot.id));
    // Shown again: the same slot, no second copy.
    a.render.open = false;
    a.show_image_in_viewer(&path);
    assert!(a.render.open);
    assert_eq!(a.render.history.slots.len(), before + 1);
    assert_eq!(a.render.viewer.primary, Some(slot.id));
    // A missing file says so and adds nothing.
    a.show_image_in_viewer(std::path::Path::new("/no/such/picture.png"));
    assert_eq!(a.render.history.slots.len(), before + 1);
}

const SIZE: Vec2 = vec2(1440., 900.);

/// `image_app` with the Shading tab shown and drawn.
fn shading_image_app() -> (Operandi, egui::Context) {
    let mut a = image_app();
    let ctx = egui::Context::default();
    theme::install(&ctx);
    a.upper_frame.show_page(1);
    for _ in 0..3 {
        super::tests::draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

fn double_click(a: &mut Operandi, ctx: &egui::Context, at: Pos2) {
    use super::tests::{draw, pointer};
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    for _ in 0..2 {
        draw(
            a,
            ctx,
            SIZE,
            vec![
                pointer(at, true, Modifiers::NONE),
                pointer(at, false, Modifiers::NONE),
            ],
        );
    }
    draw(a, ctx, SIZE, vec![]);
}

#[test]
fn double_clicking_an_image_layer_row_opens_the_image_viewer() {
    let (mut a, ctx) = shading_image_app();
    let tree = &a.editor.scene.shader_tree;
    let layer = tree
        .nodes
        .iter()
        .map(|n| n.id)
        .find(|&id| tree.layer_image(id).is_some())
        .expect("the fixture has an Image Map with an image");
    let image = tree.layer_image(layer).unwrap();
    let path = a
        .editor
        .scene
        .images
        .iter()
        .find(|i| i.id == image)
        .unwrap()
        .path
        .clone();
    assert!(!a.render.open);
    let at = a.shading.row_rects[&layer][1].center();
    double_click(&mut a, &ctx, at);
    assert!(a.render.open, "{}", a.status);
    let slot = a.render.history.slots.first().unwrap();
    assert_eq!(
        slot.saved.as_deref(),
        Some(std::path::Path::new(&path)),
        "{}",
        a.status
    );
    // Not a rename (UIC-39).
    assert!(a.shading.tree.rename.is_none());
}

#[test]
fn double_clicking_a_group_row_still_selects_its_children() {
    let (mut a, ctx) = shading_image_app();
    let tree = &a.editor.scene.shader_tree;
    let group = tree
        .nodes
        .iter()
        .map(|n| n.id)
        .find(|&id| tree.has_children(id) && a.shading.row_rects.contains_key(&id))
        .expect("a parent row");
    let at = a.shading.row_rects[&group][1].center();
    double_click(&mut a, &ctx, at);
    assert!(!a.render.open, "a group shows no image");
    assert!(a.shading.selected.len() > 1, "UIC-38");
}

/// A material preset file (a group holding a Material and a Constant)
/// and its browser entry.
fn material_entry(a: &Operandi) -> operandi_core::preset::library::Entry {
    use operandi_core::preset::library::{Entry, EntryKind};
    use operandi_core::preset::{PresetKind, PresetSource};
    use operandi_core::shader_tree::NewLayer;
    let folder = a.environment.paths().data_root().join("tree-drop-presets");
    std::fs::create_dir_all(&folder).unwrap();
    let mut source = operandi_core::Editor::default();
    let tree = &mut source.scene.shader_tree;
    let group = tree.add(NewLayer::Group, None).unwrap();
    tree.add(NewLayer::Material, Some(group)).unwrap();
    tree.add(NewLayer::Constant, Some(group)).unwrap();
    let path = folder.join("Walnut.oppreset");
    source
        .save_preset(&path, PresetSource::Material(group))
        .unwrap();
    Entry {
        name: "Walnut".into(),
        path,
        kind: EntryKind::Preset(PresetKind::Material),
        modified: None,
        size: 0,
        texture_set: None,
        modo_kind: None,
    }
}

/// The shading app with a material group holding one Image Map, drawn.
fn group_rig() -> (Operandi, egui::Context, u64, u64) {
    use operandi_core::shader_tree::NewLayer;
    let (mut a, ctx) = shading_image_app();
    let group = a
        .editor
        .scene
        .shader_tree
        .add(NewLayer::Group, None)
        .unwrap();
    let inside = a
        .editor
        .scene
        .shader_tree
        .add(NewLayer::ImageMap, Some(group))
        .unwrap();
    a.editor.undo.clear();
    for _ in 0..2 {
        super::tests::draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx, group, inside)
}

#[test]
fn only_group_rows_take_a_dragged_material() {
    let (a, _ctx, group, inside) = group_rig();
    let row = |id: u64| a.shading.row_rects[&id][1].center();
    assert_eq!(a.tree_drop_target(row(group)), Some(group));
    // A non-group row and empty space are no target.
    assert_eq!(a.tree_drop_target(row(inside)), None);
    assert_eq!(a.tree_drop_target(pos2(-50., -50.)), None);
}

#[test]
fn hovering_a_group_row_outlines_it_in_the_drop_colour() {
    let (mut a, ctx, group, _) = group_rig();
    let entry = material_entry(&a);
    let before = a.editor.scene.clone();
    a.preset_browser.drag = Some(crate::preset_browser::Drag { entry });
    a.preset_browser.tree_drop = Some(group);
    let out = super::tests::draw(&mut a, &ctx, SIZE, vec![]);
    let colour = theme::workspace::drop_indicator();
    let row = a
        .shading
        .tree
        .rects
        .iter()
        .find(|(k, _)| *k == crate::shading_panel::ShaderKey::Layer(group));
    let row = row.unwrap().1;
    assert!(
        out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.stroke.color == colour && row.contains_rect(r.rect))),
        "no drop outline on the group row"
    );
    assert_eq!(a.editor.scene, before, "hovering changes nothing");
    a.cancel_browser_drag();
    assert!(a.preset_browser.tree_drop.is_none());
}

#[test]
fn dropping_on_a_group_row_replaces_its_contents_in_one_step() {
    use operandi_core::shader_tree::NodeKind;
    let (mut a, _ctx, group, inside) = group_rig();
    let entry = material_entry(&a);
    let before = a.editor.scene.clone();
    assert!(a.browser_load_into_group(&entry, group), "{}", a.status);
    assert_eq!(a.editor.undo.len(), 1);
    let tree = &a.editor.scene.shader_tree;
    assert!(tree.get(inside).is_none(), "the old contents went");
    let kids: Vec<_> = tree.children(Some(group)).collect();
    assert_eq!(kids.len(), 2);
    assert!(kids.iter().any(|n| matches!(n.kind, NodeKind::Material(_))));
    a.run(Command::Undo);
    assert_eq!(a.editor.scene, before);
}
