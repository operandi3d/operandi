//! Items list header in the real UI (spec item-list-header): the Add Item
//! menu, disabled entries, Recent, the toggles, the view-only filter and
//! its layout round trip. Core behaviour is in
//! operandi-core/tests/item_list_header.rs.
use super::tests::{draw, pointer};
use super::*;
use operandi_core::camera_light::LightKind;
use operandi_core::item_list::AddItemKind;

const SIZE: Vec2 = vec2(1440., 900.);

fn setup() -> (Operandi, Context) {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let ctx = Context::default();
    theme::install(&ctx);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}
fn click(a: &mut Operandi, ctx: &Context, p: Pos2) {
    draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    draw(
        a,
        ctx,
        SIZE,
        vec![
            pointer(p, true, Modifiers::NONE),
            pointer(p, false, Modifiers::NONE),
        ],
    );
    draw(a, ctx, SIZE, vec![]);
}
fn hover(a: &mut Operandi, ctx: &Context, p: Pos2) {
    for _ in 0..3 {
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
}
fn rect(a: &Operandi, key: &str) -> Rect {
    *a.toolbox_rects
        .get(key)
        .unwrap_or_else(|| panic!("no {key} on screen"))
}
fn open_add_item(a: &mut Operandi, ctx: &Context) {
    a.toolbox_rects.retain(|k, _| !k.starts_with("Add Item"));
    let at = rect(a, "Item list Add Item arrow").center();
    click(a, ctx, at);
}

#[test]
fn header_has_add_item_select_filter_and_no_item_menu() {
    let (a, _) = setup();
    for key in [
        "Scenes Add Item",
        "Item list Add Item arrow",
        "Item list Select",
        "Item list Filter",
        "Item list Only Selected",
        "Item list Notes",
        "Item list Text",
    ] {
        rect(&a, key);
    }
    let labels: Vec<_> = a.toolbox_rects.keys().collect();
    assert!(
        !labels.iter().any(|k| k.contains("Delete Item")),
        "the old Item menu is gone"
    );
}

#[test]
fn add_item_mesh_goes_after_the_selection_and_into_recent() {
    let (mut a, ctx) = setup();
    a.run(Command::NewMesh);
    let first = a.editor.scene.items[0].id;
    a.run(Command::SelectItem(Some(first)));
    open_add_item(&mut a, &ctx);
    let mesh = rect(&a, "Add Item: Mesh").center();
    click(&mut a, &ctx, mesh);
    let names: Vec<_> = a
        .editor
        .scene
        .items
        .iter()
        .map(|i| i.name.as_str())
        .collect();
    assert_eq!(names, ["Mesh", "Mesh (3)", "Mesh (2)"]);
    assert_eq!(a.editor.scene.active().unwrap().name, "Mesh (3)");
    assert_eq!(a.item_list.prefs.recent_kinds(), [AddItemKind::Mesh]);
    // Saved for the next launch.
    let saved = item_list_ui::ItemListPreferences::load(&a.environment.paths().item_list());
    assert_eq!(saved.recent_kinds(), [AddItemKind::Mesh]);
}

#[test]
fn the_arrow_opens_the_same_menu() {
    let (mut a, ctx) = setup();
    a.toolbox_rects.retain(|k, _| !k.starts_with("Add Item"));
    let at = rect(&a, "Item list Add Item arrow").center();
    click(&mut a, &ctx, at);
    rect(&a, "Add Item: Camera");
    rect(&a, "Add Item menu: Lights");
}

#[test]
fn disabled_entries_do_nothing_and_empty_categories_are_hidden() {
    let (mut a, ctx) = setup();
    open_add_item(&mut a, &ctx);
    assert!(!a.toolbox_rects.contains_key("Add Item menu: Forces"));
    // Show unsupported brings the hidden categories back, greyed.
    a.item_list.prefs.show_unsupported = true;
    draw(&mut a, &ctx, SIZE, vec![super::tests::key(Key::Escape)]);
    draw(&mut a, &ctx, SIZE, vec![]);
    open_add_item(&mut a, &ctx);
    let forces = rect(&a, "Add Item menu: Forces").center();
    hover(&mut a, &ctx, forces);
    let vortex = rect(&a, "Add Item: Vortex Force").center();
    let before = a.editor.scene.clone();
    let undo = a.editor.undo.len();
    click(&mut a, &ctx, vortex);
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.editor.undo.len(), undo);
    assert!(a.item_list.prefs.recent.is_empty());
}

#[test]
fn recent_shows_none_when_empty_and_lists_newest_first() {
    let (mut a, ctx) = setup();
    open_add_item(&mut a, &ctx);
    let recent = rect(&a, "Add Item menu: Recent").center();
    hover(&mut a, &ctx, recent);
    rect(&a, "Add Item: (none)");
    a.add_item(AddItemKind::Light(LightKind::Spot));
    a.add_item(AddItemKind::Camera);
    assert_eq!(
        a.item_list.prefs.recent_kinds(),
        [AddItemKind::Camera, AddItemKind::Light(LightKind::Spot)]
    );
}

#[test]
fn command_line_adds_items_and_runs_patterns() {
    let (mut a, _) = setup();
    a.execute("item.add light.spot");
    assert_eq!(a.editor.scene.active().unwrap().name, "Spot Light");
    a.execute("item.pattern hide *light");
    assert!(!a.editor.scene.active().unwrap().visible);
    a.execute("item.add locator");
    assert!(a.status.contains("item.create <type>"), "{}", a.status);
}

#[test]
fn filters_change_only_the_view_and_save_with_the_layout() {
    let (mut a, ctx) = setup();
    a.run(Command::AddItem(AddItemKind::Camera));
    let before = a.editor.scene.clone();
    let mesh = a.editor.scene.items[0].id;
    a.item_list
        .filter
        .types
        .set(operandi_core::item_list::ItemType::Camera, false);
    a.item_list.filter.flatten = true;
    a.scene_rows.clear();
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    assert_eq!(a.scene_rows.keys().copied().collect::<Vec<_>>(), [mesh]);
    assert_eq!(a.editor.scene, before);

    let json = a.layout_json(&ctx);
    let mut b = Operandi::empty();
    let path = std::env::temp_dir().join(format!("operandi-il-layout-{}.json", std::process::id()));
    std::fs::write(&path, json).unwrap();
    b.load_layout_from(&ctx, &path);
    assert_eq!(b.item_list.filter, a.item_list.filter);
    std::fs::remove_file(path).ok();
}

#[test]
fn text_toggle_shows_the_search_row_and_off_clears_it() {
    let (mut a, ctx) = setup();
    let text = rect(&a, "Item list Text").center();
    click(&mut a, &ctx, text);
    assert_eq!(a.item_list.filter.text.as_deref(), Some(""));
    a.item_list.filter.text = Some("zzz".into());
    a.scene_rows.clear();
    draw(&mut a, &ctx, SIZE, vec![]);
    assert!(a.scene_rows.is_empty(), "nothing matches");
    click(&mut a, &ctx, text);
    assert_eq!(a.item_list.filter.text, None);
}
