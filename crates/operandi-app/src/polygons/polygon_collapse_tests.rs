//! COL-B02/B03 app routes for basic Polygon Collapse. The Polygons panel
//! Commands › Collapse row, `poly.collapse` command entry and the Input Map
//! action "Polygon Collapse" (default literal Ctrl+Backspace) are each driven
//! independently on equal starting scenes with synthetic egui pointer/key
//! events on the real widgets; widget bounds come from egui's AccessKit
//! output. Undo/Redo use the production menu action routing. These are
//! worker-local synthetic-event checks: no native input, focus, display-scale
//! or native GUI claim is made.
//!
//! Fixture: the admitted COL-F01 source (quad face 0 selected) with Default
//! tags, plus an unrelated tagged box. The expected scene is authored
//! literally from the fixture's expected result in Operandi's documented
//! layout (outer points in source order, new point last).
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use egui::accesskit::{self, Role};
use operandi_core::Mesh;
use std::collections::BTreeMap;

// Tall enough to show the whole Polygon tab without scrolling; scrolling
// itself is covered by layout_tests::toolbox_scrolls_to_its_last_row.
const SIZE: Vec2 = vec2(1440., 2000.);

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

fn tagged(vertices: &[u32]) -> Face {
    Face {
        vertices: vertices.to_vec(),
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: Some("Default".into()),
        material_label: None,
    }
}

fn plate(positions: &[(f64, f64)], cycles: &[&[u32]]) -> Mesh {
    Mesh {
        vertices: positions
            .iter()
            .map(|&(x, y)| DVec3::new(x, y, 0.))
            .collect(),
        faces: cycles.iter().map(|c| tagged(c)).collect(),
        ..Default::default()
    }
}

fn source_mesh() -> Mesh {
    plate(
        &[
            (0., 0.),
            (3., 0.),
            (2., 1.),
            (0., 2.),
            (-1., -1.),
            (4., -1.),
            (4., 2.),
            (2., 3.),
            (-1., 3.),
        ],
        &[
            &[0, 1, 2, 3],
            &[4, 5, 1, 0],
            &[5, 6, 2, 1],
            &[6, 7, 8, 3, 2],
            &[8, 4, 0, 3],
        ],
    )
}

fn result_mesh() -> Mesh {
    plate(
        &[
            (-1., -1.),
            (4., -1.),
            (4., 2.),
            (2., 3.),
            (-1., 3.),
            (1.25, 0.75),
        ],
        &[&[0, 1, 5], &[1, 2, 5], &[2, 3, 4, 5], &[4, 0, 5]],
    )
}

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let mut unrelated = Mesh::box_mesh(DVec3::new(9., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in unrelated.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    for item in &mut e.scene.items {
        item.mesh = if item.id == target {
            source_mesh()
        } else {
            unrelated.clone()
        }
        .into();
    }
    e.scene.select_item(Some(target));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e
}

fn expected(source: &Scene) -> Scene {
    let mut s = source.clone();
    s.active_mut().unwrap().mesh = result_mesh().into();
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

/// Press and release `key` over the viewport (see polygon_remove_tests).
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
fn col_b02_panel_command_and_shortcut_routes_match_on_equal_scenes() {
    let source = fixture();
    let want = expected(&source.scene);
    let ctx = context();

    let mut panel = app(&source);
    click_button(&mut panel, &ctx, "Collapse");
    assert_eq!(panel.editor.scene, want, "{}", panel.status);
    assert_eq!(panel.status, "Collapse: 1 polygon collapsed");

    let mut command = app(&source);
    command.execute("poly.collapse");
    assert_eq!(command.editor.scene, want, "{}", command.status);
    let mut upper = app(&source);
    upper.execute("POLY.COLLAPSE");
    assert_eq!(upper.editor.scene, want);
    let mut action = app(&source);
    action.execute("polygon collapse");
    assert_eq!(action.editor.scene, want);

    let mut shortcut = app(&source);
    chord(&mut shortcut, &ctx, Key::Backspace, Modifiers::CTRL);
    assert_eq!(shortcut.editor.scene, want, "{}", shortcut.status);

    for a in [&panel, &command, &upper, &action, &shortcut] {
        assert_eq!(a.editor.history.undo.len(), 1);
        assert_eq!(a.editor.history.undo[0].label, "Collapse");
        assert!(a.editor.history.redo.is_empty());
        assert_eq!(a.editor.scene.items.len(), 2, "not item deletion");
    }

    panel.menu_action(&ctx, app_menu::line_action("app.undo"));
    assert_eq!(panel.editor.scene, source.scene);
    assert_eq!(panel.editor.scene.selection.faces, [0]);
    panel.menu_action(&ctx, app_menu::line_action("app.redo"));
    assert_eq!(panel.editor.scene, want);
    // A repeat with nothing selected refuses without a duplicate edit.
    panel.execute("poly.collapse");
    assert_eq!(panel.editor.scene, want);
    assert_eq!(panel.editor.history.undo.len(), 1);
    assert!(panel.status.contains("Select polygons"), "{}", panel.status);
}

/// Commands group order: Merge, Remove, Collapse, ... Align, Set Part...,
/// Set Material....
#[test]
fn col_b02_collapse_row_follows_remove_text_only() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let nodes = frame(&mut a, &ctx, vec![]);
    let remove = rect(&button(&nodes, "Remove").expect("Remove row"));
    let collapse = rect(&button(&nodes, "Collapse").expect("Collapse row"));
    let material = rect(&button(&nodes, "Set Material...").expect("Set Material row"));
    assert!(remove.bottom() <= collapse.top());
    assert!(collapse.bottom() <= material.top());
    assert_eq!(collapse.left(), remove.left());
    assert_eq!(collapse.width(), remove.width());
    assert_eq!(collapse.height(), remove.height());
    assert_eq!(button_by_line(&a, "Polygon Collapse"), collapse);
}

#[test]
fn col_b02_default_binding_is_literal_ctrl_backspace_in_polygon_context() {
    let defaults = input::default_bindings();
    assert_eq!(defaults["Polygon Collapse"], "Ctrl+Backspace");
    assert_eq!(defaults["Remove"], "Shift+Backspace");
    assert_eq!(defaults["Delete Item"], "Backspace");
    let source = fixture();
    let ctx = context();
    // Command+Backspace is not converted to Control.
    let mut cmd = app(&source);
    let before = cmd.editor.clone();
    chord(
        &mut cmd,
        &ctx,
        Key::Backspace,
        Modifiers::MAC_CMD | Modifiers::COMMAND,
    );
    assert!(cmd.editor == before);
    // Edge/Vertex/Item contexts: no Collapse workflow is invented.
    for mode in [Mode::Edge, Mode::Vertex, Mode::Item] {
        let mut other = source.clone();
        other.scene.selection.mode = mode;
        other.scene.selection.faces.clear();
        let mut a = app(&other);
        let before = a.editor.clone();
        chord(&mut a, &ctx, Key::Backspace, Modifiers::CTRL);
        assert!(a.editor == before, "{mode:?}");
    }
    // Shift+Backspace stays Remove; plain Backspace over the viewport is
    // not Collapse: it is Delete, like the forward Delete key (Jason,
    // 2026-11-21: both delete keys run `delete`).
    let mut remove = app(&source);
    chord(&mut remove, &ctx, Key::Backspace, Modifiers::SHIFT);
    assert_eq!(remove.editor.history.undo[0].label, "Remove");
    let mut plain = app(&source);
    chord(&mut plain, &ctx, Key::Backspace, Modifiers::NONE);
    assert_eq!(plain.editor.history.undo[0].label, "Delete");
    let mut forward = app(&source);
    chord(&mut forward, &ctx, Key::Delete, Modifiers::NONE);
    assert_eq!(plain.editor.scene, forward.editor.scene);
}

#[test]
fn col_b02_row_tooltip_and_help_follow_user_remap() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.execute("help poly.collapse");
    assert!(a.status.contains("shortcut Ctrl+Backspace"), "{}", a.status);
    a.bindings
        .insert("Polygon Collapse".into(), "Alt+Backspace".into());
    assert!(a.shortcut("Polygon Collapse").contains("Alt+Backspace"));
    a.execute("help poly.collapse");
    assert!(a.status.contains("shortcut Alt+Backspace"), "{}", a.status);
    let before = a.editor.clone();
    chord(&mut a, &ctx, Key::Backspace, Modifiers::CTRL);
    assert!(a.editor == before, "the old default no longer fires");
    chord(&mut a, &ctx, Key::Backspace, Modifiers::ALT);
    assert_eq!(a.editor.scene, expected(&source.scene), "{}", a.status);
    // Unbound: help says so, no shortcut route, the row still works.
    let mut unbound = app(&source);
    unbound
        .bindings
        .insert("Polygon Collapse".into(), String::new());
    assert_eq!(unbound.shortcut("Polygon Collapse"), "");
    unbound.execute("help poly.collapse");
    assert!(unbound.status.contains("no shortcut assigned"));
    let before = unbound.editor.clone();
    chord(&mut unbound, &ctx, Key::Backspace, Modifiers::CTRL);
    assert!(unbound.editor == before);
    click_button(&mut unbound, &ctx, "Collapse");
    assert_eq!(unbound.editor.scene, expected(&source.scene));
}

/// Saved maps: a remap saved through the real Input Map dialog survives; a
/// map saved before Collapse existed keeps the default unless it already
/// assigned Ctrl+Backspace to another action, which keeps that action.
#[test]
fn col_b02_saved_input_map_remap_and_legacy_maps() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.show_bindings = true;
    a.bindings_draft = a.bindings.clone();
    a.bindings_draft
        .insert("Polygon Collapse".into(), "Alt+Backspace".into());
    frame(&mut a, &ctx, vec![]);
    frame(&mut a, &ctx, vec![]);
    click_button(&mut a, &ctx, "Save Input Map");
    assert_eq!(a.status, "Input map saved");
    let path = a.environment.paths().input_map();
    let loaded = input::load_bindings(&path);
    assert_eq!(loaded["Polygon Collapse"], "Alt+Backspace");

    let dir = a.environment.paths().data_root().join("collapse-maps");
    std::fs::create_dir_all(&dir).unwrap();
    let legacy = dir.join("legacy.json");
    let mut old: BTreeMap<String, String> = input::default_bindings();
    old.remove("Polygon Collapse");
    std::fs::write(&legacy, serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(
        input::load_bindings(&legacy)["Polygon Collapse"],
        "Ctrl+Backspace"
    );
    let taken = dir.join("taken.json");
    old.insert("Frame All".into(), "Ctrl+Backspace".into());
    std::fs::write(&taken, serde_json::to_vec(&old).unwrap()).unwrap();
    let kept = input::load_bindings(&taken);
    assert_eq!(kept["Frame All"], "Ctrl+Backspace");
    assert_eq!(kept["Polygon Collapse"], "");
    let mut fresh = app(&source);
    fresh.bindings = kept;
    let before = fresh.editor.scene.clone();
    chord(&mut fresh, &ctx, Key::Backspace, Modifiers::CTRL);
    assert_eq!(
        fresh.editor.scene, before,
        "user's own action fired instead"
    );
    assert!(fresh.editor.history.undo.is_empty());
}

#[test]
fn col_b02_focused_text_field_and_open_dialog_own_ctrl_backspace() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
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
            modifiers: Modifiers::CTRL,
        }],
    );
    assert!(a.editor == before, "no collapse while typing");
    assert!(a.set_part.is_some());
    frame(&mut a, &ctx, vec![key(Key::Escape)]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    assert_eq!(a.editor.scene, source.scene);

    // A37 Unify options modal: every Collapse route is refused while open.
    let mut m = app(&source);
    m.execute("poly.unify");
    assert!(m.unify.is_some(), "{}", m.status);
    let before = m.editor.clone();
    m.execute("poly.collapse");
    assert!(m.status.contains("Close Unify"), "{}", m.status);
    m.execute("polygon collapse");
    chord(&mut m, &ctx, Key::Backspace, Modifiers::CTRL);
    assert!(m.editor == before);
}

#[test]
fn col_b02_help_and_invalid_arguments_refuse_without_mutation() {
    let source = fixture();
    let mut a = app(&source);
    let before = a.editor.clone();
    a.execute("help poly.collapse");
    assert!(a.status.contains("no arguments"), "{}", a.status);
    a.execute("help");
    assert!(a.status.contains("poly.collapse"), "{}", a.status);
    for bad in [
        "poly.collapse 0",
        "poly.collapse ?",
        "poly.collapse mode:center",
        "poly.collapse {a",
        "poly.collapse \"x",
    ] {
        a.execute(bad);
        assert!(a.editor == before, "{bad}");
        assert_ne!(a.status, "Ready", "{bad}");
        assert!(!a.status.starts_with("Collapse:"), "{bad}");
    }
}

#[test]
fn col_b03_active_tool_and_fields_survive_every_route() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.execute("spike");
    assert!(a.editor.session.spike.is_some(), "{}", a.status);
    a.fields
        .insert("spi_strength".into(), "not a number".into());
    let editor = a.editor.clone();
    let fields = a.fields.clone();
    a.execute("poly.collapse");
    assert!(a.status.contains("Drop or Cancel"), "{}", a.status);
    a.execute("polygon collapse");
    click_button(&mut a, &ctx, "Collapse");
    chord(&mut a, &ctx, Key::Backspace, Modifiers::CTRL);
    assert!(a.editor == editor);
    assert_eq!(a.fields, fields);
}

#[test]
fn col_b04_boundary_and_connected_selections_heal() {
    // A boundary polygon collapses; the mesh outline changes but stays valid.
    let mut boundary = fixture();
    boundary.scene.selection.faces = vec![1];
    let mut a = app(&boundary);
    a.execute("poly.collapse");
    assert_eq!(a.status, "Collapse: 1 polygon collapsed");
    let mesh = &a.editor.scene.active().unwrap().mesh;
    mesh.validate().unwrap();
    assert_eq!(mesh.faces.len(), 4);
    assert_eq!(a.editor.history.undo.len(), 1);

    // Two adjacent polygons form one piece and collapse to one point. The
    // neighbour sharing two of its edges with the piece closes up entirely.
    let mut two = fixture();
    two.scene.selection.faces = vec![0, 2];
    let mut b = app(&two);
    b.execute("poly.collapse");
    assert_eq!(b.status, "Collapse: 2 polygons collapsed");
    let mesh = &b.editor.scene.active().unwrap().mesh;
    mesh.validate().unwrap();
    assert_eq!(
        mesh.vertices.len(),
        4,
        "survivors 4, 7, 8 plus one new point"
    );
    let corners: Vec<usize> = mesh.faces.iter().map(|f| f.vertices.len()).collect();
    assert_eq!(corners, [3, 3]);
    let point = mesh.vertices[3];
    let piece =
        [0u32, 1, 2, 3, 5, 6].map(|v| two.scene.active().unwrap().mesh.vertices[v as usize]);
    assert_eq!(point, piece.iter().copied().sum::<DVec3>() / 6.);
    b.menu_action(&context(), app_menu::line_action("app.undo"));
    assert_eq!(b.editor.scene, two.scene);
}

#[test]
fn col_b04_cube_face_becomes_a_pyramid_and_empty_selection_refuses() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![1];
    e.scene.validate().unwrap();
    let mut a = app(&e);
    a.execute("poly.collapse");
    assert_eq!(a.status, "Collapse: 1 polygon collapsed");
    let mesh = &a.editor.scene.active().unwrap().mesh;
    mesh.validate().unwrap();
    assert_eq!(mesh.vertices.len(), 5);
    assert_eq!(mesh.vertices[4], DVec3::new(0., 0., 1.));
    let mut sizes: Vec<usize> = mesh.faces.iter().map(|f| f.vertices.len()).collect();
    sizes.sort();
    assert_eq!(sizes, [3, 3, 3, 3, 4]);

    let before = a.editor.clone();
    a.execute("poly.collapse");
    assert!(a.status.contains("Select polygons"), "{}", a.status);
    assert!(a.editor == before);
}

#[test]
fn col_b03_task_native_save_open_preserves_result() {
    let source = fixture();
    let mut a = app(&source);
    a.execute("poly.collapse");
    let authored = a.editor.scene.clone();
    assert_eq!(authored, expected(&source.scene));
    let path = a.environment.paths().data_root().join("collapse.opscene");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(a.save(path.clone()), "{}", a.status);
    let mut reopened = Operandi::empty();
    reopened.open(path);
    assert_eq!(reopened.editor.scene, authored);
    assert!(reopened.editor.scene.selection.faces.is_empty());
}
