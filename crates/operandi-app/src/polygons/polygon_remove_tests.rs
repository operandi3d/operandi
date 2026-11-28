//! REM-001/003/004 app routes for basic Polygon Remove. The Polygons panel
//! Commands › Remove row, `poly.remove` command entry and the Input Map
//! action "Remove" (default Shift+Backspace) are each driven independently on
//! equal starting scenes with synthetic egui pointer/key events on the real
//! widgets; widget bounds come from egui's AccessKit output. Undo/Redo use the
//! production menu action routing. These are worker-local synthetic-event
//! checks: no native input, focus, display-scale or native GUI claim is made.
//!
//! Fixture: the admitted original positions/faces (face 0 selected) plus one
//! isolated point, sparse UVs and point weights, and an unrelated tagged box.
//! The expected scene is authored literally from the fixture's retained faces.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use egui::accesskit::{self, Role};
use glam::DVec2;
use operandi_core::Mesh;
use std::collections::BTreeMap;

const SIZE: Vec2 = vec2(1440., 2000.);
const CYCLES: [&[u32]; 3] = [&[0, 3, 1], &[2, 1, 3], &[1, 2, 5, 4]];

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

fn positions() -> Vec<DVec3> {
    [
        (0., 0., 0.),
        (1., 0., 0.),
        (1., 0., 1.),
        (0., 0., 1.),
        (2., 0., 0.),
        (2., 0., 1.),
        (3., 0., 3.),
    ]
    .map(|(x, y, z)| DVec3::new(x, y, z))
    .to_vec()
}

/// Rows of the original faces in storage order `order`.
fn road(order: &[usize]) -> Mesh {
    let texture = [
        vec![Some(DVec2::new(0.1, 0.1)), None, Some(DVec2::new(0.3, 0.1))],
        vec![None, Some(DVec2::new(0.6, 0.2)), None],
        vec![
            None,
            None,
            Some(DVec2::new(0.9, 0.9)),
            Some(DVec2::new(0.8, 0.9)),
        ],
    ];
    let mut mesh = Mesh {
        vertices: positions(),
        faces: order
            .iter()
            .map(|&r| Face {
                vertices: CYCLES[r].to_vec(),
                uv: None,
                material: format!("m{r}"),
                smooth: r == 2,
                part: (r != 1).then(|| format!("part{r}")),
                material_label: None,
            })
            .collect(),
        uv_maps: BTreeMap::from([(
            "Texture".into(),
            order.iter().map(|&r| texture[r].clone()).collect(),
        )])
        .into(),
        ..Default::default()
    };
    mesh.attributes
        .point_weights
        .insert("Mask".into(), BTreeMap::from([(0, 0.75), (6, 0.25)]));
    mesh
}

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let road_id = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let mut unrelated = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in unrelated.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    for item in &mut e.scene.items {
        item.mesh = if item.id == road_id {
            road(&[0, 1, 2])
        } else {
            unrelated.clone()
        }
        .into();
    }
    e.scene.select_item(Some(road_id));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e
}

/// The fixture with original faces 1 and 2 only and no selected faces.
fn expected(source: &Scene) -> Scene {
    let mut s = source.clone();
    s.active_mut().unwrap().mesh = road(&[1, 2]).into();
    s.selection.faces.clear();
    s
}

fn app(fixture: &Editor) -> Operandi {
    let mut a = Operandi::empty();
    a.editor = fixture.clone();
    a.saved = a.editor.scene.clone();
    a.category = toolbox::Category::Polygon;
    a.camera.view = View::Perspective;
    a.frame(false);
    a
}

fn frame(a: &mut Operandi, ctx: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    draw(a, ctx, SIZE, events)
        .platform_output
        .accesskit_update
        .expect("AccessKit enabled")
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}

fn rect(n: &accesskit::Node) -> Rect {
    let b = n.bounds().expect("widget bounds");
    Rect::from_min_max(
        pos2(b.x0 as f32, b.y0 as f32),
        pos2(b.x1 as f32, b.y1 as f32),
    )
}

fn button(nodes: &[accesskit::Node], label: &str) -> Option<accesskit::Node> {
    nodes
        .iter()
        .find(|n| n.role() == Role::Button && n.label() == Some(label))
        .cloned()
}

fn click_button(a: &mut Operandi, ctx: &Context, label: &str) {
    let nodes = frame(a, ctx, vec![]);
    let at = rect(&button(&nodes, label).unwrap_or_else(|| panic!("{label} button"))).center();
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

/// Press and release `key` over the viewport. The release matters: egui
/// reports a second press of a key it still considers down as a repeat,
/// which shortcuts deliberately ignore.
fn chord(a: &mut Operandi, ctx: &Context, key: Key, modifiers: Modifiers) {
    frame(a, ctx, vec![]);
    let pos = a.viewport_rect.center();
    frame(a, ctx, vec![Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        frame(
            a,
            ctx,
            vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}

#[test]
fn rem_004_panel_command_and_shortcut_routes_match_on_equal_scenes() {
    let source = fixture();
    let want = expected(&source.scene);
    let ctx = context();

    let mut panel = app(&source);
    click_button(&mut panel, &ctx, "Remove");
    assert_eq!(panel.editor.scene, want, "{}", panel.status);
    assert_eq!(panel.status, "Removed 1 polygon");

    let mut command = app(&source);
    command.execute("poly.remove");
    assert_eq!(command.editor.scene, want, "{}", command.status);
    let mut upper = app(&source);
    upper.execute("POLY.REMOVE");
    assert_eq!(upper.editor.scene, want);

    let mut shortcut = app(&source);
    chord(&mut shortcut, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert_eq!(shortcut.editor.scene, want, "{}", shortcut.status);

    for a in [&panel, &command, &upper, &shortcut] {
        assert_eq!(a.editor.history.undo.len(), 1);
        assert_eq!(a.editor.history.undo[0].label, "Remove");
        assert!(a.editor.history.redo.is_empty());
        assert_eq!(a.editor.scene.items.len(), 2, "not item deletion");
    }

    panel.menu_action(&ctx, app_menu::line_action("app.undo"));
    assert_eq!(panel.editor.scene, source.scene);
    panel.menu_action(&ctx, app_menu::line_action("app.redo"));
    assert_eq!(panel.editor.scene, want);
}

/// Fixed M1 inventory order of the Polygons panel Commands group as drawn:
/// Remove above Set Part... and Set Material..., which follow Align, in the
/// same column (SP-14). Bounds are the rendered widgets' AccessKit bounds.
#[test]
fn rem_001_commands_group_draws_remove_before_set_part() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let nodes = frame(&mut a, &ctx, vec![]);
    let remove = rect(&button(&nodes, "Remove").expect("Remove row"));
    let set_part = rect(&button(&nodes, "Set Part...").expect("Set Part row"));
    assert!(
        remove.bottom() <= set_part.top(),
        "Remove {remove:?} must be above Set Part {set_part:?}"
    );
    assert_eq!(remove.left(), set_part.left());
    assert_eq!(remove.width(), set_part.width());
    let mut rows: Vec<_> = nodes
        .iter()
        .filter(|n| n.role() == Role::Button && n.bounds().is_some())
        .filter(|n| {
            let r = rect(n);
            r.center().y > remove.bottom()
                && r.center().y < set_part.top()
                && r.left() < remove.right()
                && r.right() > remove.left()
        })
        .filter_map(|n| Some((rect(n).top(), n.label()?.to_owned())))
        .collect();
    // AccessKit node order is not drawing order: order rows by position.
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let between: Vec<_> = rows.into_iter().map(|(_, label)| label).collect();
    // COL-B02 inserts the text-only Collapse row directly after Remove;
    // Set Part... comes right after Align (Jason 2026-11-05).
    assert!(
        between.first().map(String::as_str) == Some("Collapse")
            && between.last().is_some_and(|l| l.starts_with("Align")),
        "rows between Remove and Set Part: {between:?}"
    );
    assert_eq!(button_by_line(&a, "Remove"), remove);
    assert_eq!(button_by_line(&a, "Set Part"), set_part);
    let material = rect(&button(&nodes, "Set Material...").expect("Set Material row"));
    assert!(
        set_part.bottom() <= material.top(),
        "SP-14: Set Part {set_part:?} must be above Set Material {material:?}"
    );
}

#[test]
fn rem_001_default_binding_is_contextual_and_distinct_from_item_deletion() {
    let defaults = input::default_bindings();
    assert_eq!(defaults["Remove"], "Shift+Backspace");
    assert_eq!(defaults["Delete Item"], "Backspace");
    let source = fixture();
    let ctx = context();
    // Edge mode: the contextual action is unavailable; nothing else fires.
    let mut edges = source.clone();
    edges.scene.selection.mode = Mode::Edge;
    edges.scene.selection.faces.clear();
    let mut a = app(&edges);
    let before = a.editor.clone();
    chord(&mut a, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert!(a.editor == before);
    // Shift+Backspace runs Remove (points stay); plain Backspace and
    // forward Delete both run `delete`, which also drops the points only
    // those polygons used (Jason, 2026-11-21). Items stay either way.
    let mut b = app(&source);
    let items = b.editor.scene.items.len();
    chord(&mut b, &ctx, Key::Backspace, Modifiers::NONE);
    let mut c = app(&source);
    chord(&mut c, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert_eq!(c.editor.history.undo.last().unwrap().label, "Remove");
    let mut d = app(&source);
    chord(&mut d, &ctx, Key::Delete, Modifiers::NONE);
    assert_eq!(d.editor.history.undo.last().unwrap().label, "Delete");
    assert_eq!(b.editor.scene, d.editor.scene);
    let (deleted, removed) = (
        d.editor.scene.active().unwrap(),
        c.editor.scene.active().unwrap(),
    );
    assert_eq!(deleted.mesh.faces.len(), removed.mesh.faces.len());
    assert_eq!(deleted.mesh.vertices.len() + 1, removed.mesh.vertices.len());
    assert_eq!(b.editor.scene.items.len(), items);
    assert_eq!(d.editor.scene.items.len(), items);
}

#[test]
fn rem_001_user_remap_moves_the_shortcut_and_its_label() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.bindings.insert("Remove".into(), "Alt+Backspace".into());
    assert!(a.shortcut("Remove").contains("Alt+Backspace"));
    let before = a.editor.clone();
    chord(&mut a, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert!(a.editor == before, "the old default no longer fires");
    chord(&mut a, &ctx, Key::Backspace, Modifiers::ALT);
    assert_eq!(a.editor.scene, expected(&source.scene), "{}", a.status);
    // Unbound: no shortcut route at all, the panel row still works.
    let mut unbound = app(&source);
    unbound.bindings.insert("Remove".into(), String::new());
    assert_eq!(unbound.shortcut("Remove"), "");
    let before = unbound.editor.clone();
    chord(&mut unbound, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert!(unbound.editor == before);
    click_button(&mut unbound, &ctx, "Remove");
    assert_eq!(unbound.editor.scene, expected(&source.scene));
}

/// Saved remap: the real Input Map dialog's Save button writes the draft;
/// a fresh app then merges that file over the defaults exactly as startup
/// (`Operandi::new`, which needs an eframe CreationContext) does. A file saved
/// before Remove existed keeps the default Shift+Backspace.
#[test]
fn rem_001_saved_input_map_remap_and_legacy_map_default() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.show_bindings = true;
    a.bindings_draft = a.bindings.clone();
    a.bindings_draft
        .insert("Remove".into(), "Alt+Backspace".into());
    // Let the newly opened window settle its position before locating Save.
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    click_button(&mut a, &ctx, "Save Input Map");
    assert_eq!(a.status, "Input map saved");
    assert!(!a.show_bindings);
    let path = a.environment.paths().input_map();
    let stored: BTreeMap<String, String> =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(stored["Remove"], "Alt+Backspace");

    let reload = |stored: BTreeMap<String, String>| {
        let mut fresh = app(&source);
        fresh.bindings = input::default_bindings();
        fresh.bindings.extend(stored);
        fresh
    };
    let mut fresh = reload(stored.clone());
    assert!(fresh.shortcut("Remove").contains("Alt+Backspace"));
    let before = fresh.editor.clone();
    chord(&mut fresh, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert!(fresh.editor == before, "saved remap replaces the default");
    chord(&mut fresh, &ctx, Key::Backspace, Modifiers::ALT);
    assert_eq!(
        fresh.editor.scene,
        expected(&source.scene),
        "{}",
        fresh.status
    );

    let mut legacy = stored;
    legacy.remove("Remove");
    let mut old = reload(legacy);
    assert_eq!(old.bindings["Remove"], "Shift+Backspace");
    chord(&mut old, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert_eq!(old.editor.scene, expected(&source.scene), "{}", old.status);
}

#[test]
fn rem_001_focused_text_field_owns_shift_backspace() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    // The Set Part dialog's focused Name field stands in for any text field.
    a.execute("set part");
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_some(), "{}", a.status);
    frame(&mut a, &ctx, vec![Event::Text("ab".into())]);
    let before = a.editor.clone();
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        }],
    );
    assert!(a.editor == before, "no polygons removed while typing");
    assert!(a.set_part.is_some());
    frame(&mut a, &ctx, vec![key(Key::Escape)]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    assert_eq!(a.editor.scene, source.scene);
}

#[test]
fn rem_001_help_and_invalid_arguments_refuse_without_mutation() {
    let source = fixture();
    let mut a = app(&source);
    let before = a.editor.clone();
    a.execute("help poly.remove");
    assert!(a.status.contains("no arguments"), "{}", a.status);
    a.execute("help");
    assert!(a.status.contains("poly.remove"), "{}", a.status);
    for bad in [
        "poly.remove 0",
        "poly.remove ?",
        "poly.remove faces:0",
        "poly.remove {a",
        "poly.remove \"x",
    ] {
        a.execute(bad);
        assert!(a.editor == before, "{bad}");
        assert_ne!(a.status, "Ready", "{bad}");
        assert!(!a.status.starts_with("Removed"), "{bad}");
    }
}

#[test]
fn rem_001_active_tool_draft_and_fields_survive_every_route() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.execute("spike");
    assert!(a.editor.session.spike.is_some(), "{}", a.status);
    a.fields
        .insert("spi_strength".into(), "not a number".into());
    let editor = a.editor.clone();
    let fields = a.fields.clone();
    a.execute("poly.remove");
    assert!(a.status.contains("Drop or Cancel"), "{}", a.status);
    a.execute("remove");
    click_button(&mut a, &ctx, "Remove");
    chord(&mut a, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert!(a.editor == editor);
    assert_eq!(a.fields, fields);
}

#[test]
fn rem_003_task_native_save_open_preserves_survivors_and_orphan_point() {
    let source = fixture();
    let mut a = app(&source);
    a.execute("poly.remove");
    let authored = a.editor.scene.clone();
    assert_eq!(authored, expected(&source.scene));
    let path = a.environment.paths().data_root().join("remove.opscene");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(a.save(path.clone()), "{}", a.status);
    let mut reopened = Operandi::empty();
    reopened.open(path);
    assert_eq!(reopened.editor.scene, authored);
    let mesh = &reopened.editor.scene.active().unwrap().mesh;
    assert_eq!(mesh.vertices, positions());
    assert_eq!(mesh.attributes.point_weights["Mask"][&0], 0.75);
}
