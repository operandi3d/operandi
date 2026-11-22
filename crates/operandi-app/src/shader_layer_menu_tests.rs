//! The shader layer right-click menu through the real Shading tab: the
//! entries in order with the ones that don't apply left out (UIC-07), and
//! Duplicate, Copy / Paste and Editor Color from the menu.
use super::tests::{box_app, draw, key};
use super::*;
use operandi_core::shader_tree::{NewLayer, NodeId, ShaderTreeEdit};

const SIZE: Vec2 = vec2(1440., 900.);

fn shading_app() -> (Operandi, Context) {
    let mut a = box_app();
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    a.upper_frame.show_page(1);
    for _ in 0..2 {
        draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

fn click(pos: Pos2, button: PointerButton) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]
}

fn add(a: &mut Operandi, layer: NewLayer) -> NodeId {
    let id = a.editor.scene.shader_tree.next_id;
    a.run(Command::ShaderTree(ShaderTreeEdit::Add {
        layer,
        selected: None,
    }));
    id
}

/// Right-click layer `id`; the menu's button labels, top to bottom.
fn open_menu(a: &mut Operandi, ctx: &Context, id: NodeId) -> Vec<(String, Pos2)> {
    draw(a, ctx, SIZE, vec![]);
    // Right of the name, clear of a texture's attributes "+".
    let row = a.shading.row_rects[&id][1];
    let at = pos2(row.right() - 8., row.center().y);
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, click(at, PointerButton::Secondary));
    draw(a, ctx, SIZE, vec![]);
    let out = draw(a, ctx, SIZE, vec![]);
    let nodes = out.platform_output.accesskit_update.unwrap().nodes;
    let all: Vec<(String, accesskit::Rect)> = nodes
        .iter()
        .map(|(_, n)| n)
        .filter(|n| n.role() == accesskit::Role::Button)
        .filter_map(|n| Some((n.label()?.to_owned(), n.bounds()?)))
        .collect();
    // The menu's column: the buttons lined up under its first entry.
    let first = all
        .iter()
        .find(|(l, _)| l == "Rename…")
        .map(|(_, b)| *b)
        .expect("the layer menu opened");
    let mut buttons: Vec<(String, Pos2)> = all
        .into_iter()
        .filter(|(_, b)| (b.x0 - first.x0).abs() < 1. && b.y0 >= first.y0)
        .map(|(l, b)| {
            let at = pos2(((b.x0 + b.x1) / 2.) as f32, ((b.y0 + b.y1) / 2.) as f32);
            (l.trim_end_matches(" ⏵").to_owned(), at)
        })
        .collect();
    buttons.sort_by(|x, y| x.1.y.total_cmp(&y.1.y));
    buttons
}

fn choose(a: &mut Operandi, ctx: &Context, id: NodeId, label: &str) {
    let menu = open_menu(a, ctx, id);
    pick(a, ctx, &menu, label);
}

/// Click `label` in an open menu.
fn pick(a: &mut Operandi, ctx: &Context, menu: &[(String, Pos2)], label: &str) {
    let at = menu
        .iter()
        .find(|(l, _)| l == label)
        .unwrap_or_else(|| panic!("no {label} in {menu:?}"))
        .1;
    draw(a, ctx, SIZE, vec![Event::PointerMoved(at)]);
    draw(a, ctx, SIZE, click(at, PointerButton::Primary));
    draw(a, ctx, SIZE, vec![]);
}

fn labels(menu: &[(String, Pos2)]) -> Vec<&str> {
    menu.iter().map(|(l, _)| l.as_str()).collect()
}

#[test]
fn rename_from_layer_menu_starts_with_the_name_selected() {
    let (mut a, ctx) = shading_app();
    let layer = add(&mut a, NewLayer::Material);
    let original = a.editor.scene.shader_tree.get(layer).unwrap().name.clone();

    choose(&mut a, &ctx, layer, "Rename…");
    draw(&mut a, &ctx, SIZE, vec![Event::Text("Discarded".into())]);
    assert_eq!(
        a.shading
            .tree
            .rename
            .as_ref()
            .map(|(_, name)| name.as_str()),
        Some("Discarded")
    );
    draw(&mut a, &ctx, SIZE, vec![key(Key::Escape)]);
    assert_eq!(
        a.editor.scene.shader_tree.get(layer).unwrap().name,
        original
    );

    choose(&mut a, &ctx, layer, "Rename…");
    draw(&mut a, &ctx, SIZE, vec![Event::Text("Replacement".into())]);
    draw(&mut a, &ctx, SIZE, vec![key(Key::Enter)]);
    assert_eq!(
        a.editor.scene.shader_tree.get(layer).unwrap().name,
        "Replacement"
    );
}

#[test]
fn menu_entries_follow_the_spec_order_and_hide_what_does_not_apply() {
    let (mut a, ctx) = shading_app();
    let material = add(&mut a, NewLayer::Material);
    let menu = open_menu(&mut a, &ctx, material);
    let got = labels(&menu);
    let want = [
        "Rename…",
        "Editor Color",
        "Properties",
        "Duplicate",
        "Copy",
        "Delete",
        "Presets",
    ];
    let seen: Vec<&str> = got.iter().copied().filter(|l| want.contains(l)).collect();
    assert_eq!(seen, want, "{got:?}");
    // Not a group: no Select Polygons; nothing copied: no Paste; not an
    // image: no image entries.
    for hidden in [
        "Select Polygons",
        "Paste",
        "Paste Values",
        "Reload Image",
        "Change Type",
    ] {
        assert!(!got.contains(&hidden), "{hidden} shown: {got:?}");
    }
    // An Image Map gets its entries; a group gets Select Polygons.
    let (mut a, ctx) = shading_app();
    let image = add(&mut a, NewLayer::ImageMap);
    let got = open_menu(&mut a, &ctx, image);
    assert!(labels(&got).contains(&"Replace Image…"), "{got:?}");
    assert!(labels(&got).contains(&"Change Type"), "{got:?}");
    let (mut a, ctx) = shading_app();
    let group = add(&mut a, NewLayer::Group);
    let got = open_menu(&mut a, &ctx, group);
    assert!(labels(&got).contains(&"Select Polygons"), "{got:?}");
}

#[test]
fn duplicate_copy_paste_and_editor_color_from_the_menu() {
    let (mut a, ctx) = shading_app();
    let material = add(&mut a, NewLayer::Material);
    let steps = a.editor.undo.len();
    choose(&mut a, &ctx, material, "Duplicate");
    assert_eq!(a.editor.undo.len(), steps + 1);
    let copy = a.shading.selected.clone();
    assert_eq!(copy.len(), 1);
    assert_ne!(copy[0], material);
    assert_eq!(
        a.editor.scene.shader_tree.get(copy[0]).unwrap().name,
        "Material (2)"
    );
    choose(&mut a, &ctx, material, "Copy");
    assert!(a.shading.clipboard.is_some());
    choose(&mut a, &ctx, material, "Paste");
    assert_eq!(
        a.editor
            .scene
            .shader_tree
            .get(a.shading.selected[0])
            .unwrap()
            .name,
        "Material (3)"
    );
    // Editor Color ▸ Cyan: the submenu opens on hover.
    let menu = open_menu(&mut a, &ctx, material);
    let at = menu.iter().find(|(l, _)| l == "Editor Color").unwrap().1;
    for _ in 0..3 {
        draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(at)]);
    }
    let out = draw(&mut a, &ctx, SIZE, vec![]);
    let nodes = out.platform_output.accesskit_update.unwrap().nodes;
    let teal = nodes
        .iter()
        .map(|(_, n)| n)
        .find(|n| n.label().is_some_and(|l| l.ends_with("Cyan")))
        .and_then(|n| n.bounds())
        .expect("Cyan in the Editor Color submenu");
    let t = pos2(
        ((teal.x0 + teal.x1) / 2.) as f32,
        ((teal.y0 + teal.y1) / 2.) as f32,
    );
    draw(
        &mut a,
        &ctx,
        SIZE,
        vec![Event::PointerMoved(pos2(t.x, at.y))],
    );
    draw(&mut a, &ctx, SIZE, vec![Event::PointerMoved(t)]);
    draw(&mut a, &ctx, SIZE, click(t, PointerButton::Primary));
    draw(&mut a, &ctx, SIZE, vec![]);
    assert_eq!(
        a.editor
            .scene
            .shader_tree
            .get(material)
            .unwrap()
            .layer
            .editor_color,
        Some(operandi_core::EditorColor::Cyan)
    );
}

#[test]
fn instances_from_the_menu_select_each_other_and_deleting_the_source_asks() {
    let (mut a, ctx) = shading_app();
    let source = add(&mut a, NewLayer::Constant);
    choose(&mut a, &ctx, source, "Create Instance");
    let inst = a.shading.selected[0];
    assert_eq!(
        a.editor.scene.shader_tree.instance_source(inst),
        Some(source)
    );
    let menu = open_menu(&mut a, &ctx, inst);
    let got = labels(&menu);
    assert!(got.contains(&"Select Source of Instance"), "{got:?}");
    assert!(!got.contains(&"Change Type"), "{got:?}");
    assert!(!got.contains(&"Select Instances"), "{got:?}");
    pick(&mut a, &ctx, &menu, "Select Source of Instance");
    assert_eq!(a.shading.selected, [source]);
    choose(&mut a, &ctx, source, "Select Instances");
    assert_eq!(a.shading.selected, [inst]);
    choose(&mut a, &ctx, source, "Delete");
    assert_eq!(a.shading.confirm_delete.as_deref(), Some(&[source][..]));
    assert!(a.editor.scene.shader_tree.get(source).is_some());
}

/// Press at `from`, move in steps to `to`, release.
fn drag(a: &mut Operandi, ctx: &Context, from: Pos2, to: Pos2) {
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    draw(a, ctx, SIZE, vec![Event::PointerMoved(from)]);
    draw(a, ctx, SIZE, vec![press(from, true)]);
    for k in 1..=6 {
        let p = from + (to - from) * (k as f32 / 6.);
        draw(a, ctx, SIZE, vec![Event::PointerMoved(p)]);
    }
    draw(a, ctx, SIZE, vec![press(to, false)]);
    draw(a, ctx, SIZE, vec![]);
}

#[test]
fn library_items_lock_and_drag_into_the_tree_as_references() {
    let (mut a, ctx) = shading_app();
    let group = add(&mut a, NewLayer::Group);
    a.run(Command::ShaderTree(ShaderTreeEdit::ToLibrary(vec![group])));
    assert!(
        a.editor
            .scene
            .shader_tree
            .get(group)
            .unwrap()
            .extras
            .library
    );
    a.shading.open_sections.insert("Library");
    draw(&mut a, &ctx, SIZE, vec![]);
    let menu = open_menu(&mut a, &ctx, group);
    assert!(labels(&menu).contains(&"Lock Preset"), "{menu:?}");
    pick(&mut a, &ctx, &menu, "Lock Preset");
    assert!(a.editor.scene.shader_tree.get(group).unwrap().extras.locked);
    let menu = open_menu(&mut a, &ctx, group);
    assert!(labels(&menu).contains(&"Unlock Preset"), "{menu:?}");
    pick(&mut a, &ctx, &menu, "Unlock Preset");
    // Drag the Library item onto the Base Material row's lower half: a
    // reference lands below it.
    draw(&mut a, &ctx, SIZE, vec![]);
    let from = a.shading.row_rects[&group][1].center();
    let base = a.shading.row_rects[&4][1];
    let to = pos2(base.center().x, base.bottom() - 3.);
    let next = a.editor.scene.shader_tree.next_id;
    drag(&mut a, &ctx, from, to);
    let tree = &a.editor.scene.shader_tree;
    assert_eq!(
        tree.reference_target(next),
        Some(group),
        "a reference was dropped"
    );
    assert!(tree.get(group).unwrap().extras.library, "the item stays");
    let menu = open_menu(&mut a, &ctx, next);
    assert!(labels(&menu).contains(&"References"), "{menu:?}");
    assert!(!labels(&menu).contains(&"Create Instance"), "{menu:?}");
}
