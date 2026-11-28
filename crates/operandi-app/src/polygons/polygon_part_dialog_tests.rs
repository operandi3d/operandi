//! PART-002/003/005/006 app routes for basic Set Part. The Polygon panel row,
//! the dialog's Name field, existing-name list, OK, Cancel, Return and Escape
//! are driven with synthetic egui pointer/key/text events on the real widgets;
//! widget bounds and values come from egui's AccessKit output. Undo/Redo use
//! the production menu action routing. These are worker-local synthetic-event
//! checks: no native input, focus, display-scale or native GUI claim is made.
//!
//! Fixture: the admitted original two quads (initial tags SourceA/Neighbor,
//! face 0 explicitly selected) with independently authored rich attributes,
//! plus an unrelated box whose even faces are tagged "Unrelated". Expected
//! scenes are the source with only face 0's part changed.
use super::tests::{draw, key, pointer};
use super::*;
use egui::accesskit::{self, Role};
use glam::{DVec2, DVec4};
use operandi_core::Mesh;
use std::collections::BTreeSet;

const SIZE: Vec2 = vec2(1440., 2000.);

fn context() -> Context {
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx
}

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let road = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    let quads = [[0u32, 3, 2, 1], [1, 2, 5, 4]];
    let mut mesh = Mesh {
        vertices: [
            (0., 0., 0.),
            (1., 0., 0.),
            (1., 0., 1.),
            (0., 0., 1.),
            (2., 0., 0.),
            (2., 0., 1.),
        ]
        .map(|(x, y, z)| DVec3::new(x, y, z))
        .to_vec(),
        faces: quads
            .iter()
            .zip(["SourceA", "Neighbor"])
            .enumerate()
            .map(|(i, (v, part))| Face {
                vertices: v.to_vec(),
                uv: Some(vec![DVec2::splat(i as f64); 4]),
                material: ["Asphalt", "Verge"][i].into(),
                smooth: i == 0,
                part: Some(part.into()),
                material_label: Some(format!("label-{i}")),
            })
            .collect(),
        ..Default::default()
    };
    let a = &mut mesh.attributes;
    a.polygon_sets.insert("Lane".into(), BTreeSet::from([0, 1]));
    a.corner_weights
        .insert("Wear".into(), vec![vec![Some(0.1); 4], vec![None; 4]]);
    a.corner_colors.insert(
        "Paint".into(),
        vec![vec![Some(DVec4::ONE); 4], vec![Some(DVec4::splat(0.5)); 4]],
    );
    a.smoothing_labels = vec![Some("hard".into()), None];
    a.representations = vec![PolygonRepresentation::Subdivs, PolygonRepresentation::Face];
    a.creases = vec![MeshCrease {
        a: 1,
        b: 2,
        weight: 0.5,
    }];
    let mut unrelated = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1).unwrap();
    for (i, f) in unrelated.faces.iter_mut().enumerate() {
        f.part = (i % 2 == 0).then(|| "Unrelated".into());
    }
    for item in &mut e.scene.items {
        item.mesh = if item.id == road {
            mesh.clone()
        } else {
            unrelated.clone()
        }
        .into();
    }
    e.scene.select_item(Some(road));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    e
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

fn retagged(scene: &Scene, name: &str) -> Scene {
    let mut s = scene.clone();
    s.active_mut().unwrap().mesh.faces[0].part = Some(name.into());
    s
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

fn find(nodes: &[accesskit::Node], role: Role, label: &str) -> Option<accesskit::Node> {
    nodes
        .iter()
        .find(|n| n.role() == role && n.label() == Some(label))
        .cloned()
}

fn click(a: &mut Operandi, ctx: &Context, at: Pos2) {
    frame(a, ctx, vec![Event::PointerMoved(at)]);
    frame(a, ctx, vec![pointer(at, true, Modifiers::NONE)]);
    frame(a, ctx, vec![pointer(at, false, Modifiers::NONE)]);
}

fn click_button(a: &mut Operandi, ctx: &Context, label: &str) {
    let node = |a: &mut Operandi| {
        let nodes = frame(a, ctx, vec![]);
        rect(&find(&nodes, Role::Button, label).unwrap_or_else(|| panic!("{label} button")))
    };
    // Rows at the end of the Commands group (Set Part..., Set Material...)
    // sit below the fold at SIZE: scroll the panel down as a user would, and
    // let the smooth scroll settle before clicking.
    for _ in 0..20 {
        let r = node(a);
        if r.left() >= a.viewport_rect.left() || r.bottom() <= a.viewport_rect.bottom() {
            break;
        }
        let at = pos2(r.center().x, a.viewport_rect.center().y);
        frame(a, ctx, vec![Event::PointerMoved(at)]);
        frame(
            a,
            ctx,
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., -60.),
                modifiers: Modifiers::NONE,
            }],
        );
    }
    let mut r = node(a);
    for _ in 0..20 {
        let next = node(a);
        if next == r {
            break;
        }
        r = next;
    }
    click(a, ctx, r.center());
}

/// The dialog's Name field (the only text input inside the modal).
fn name_field(nodes: &[accesskit::Node]) -> Option<accesskit::Node> {
    let ok = find(nodes, Role::Button, "OK")?;
    nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput)
        .min_by_key(|n| (rect(n).center().distance(rect(&ok).center())) as i64)
        .cloned()
}

fn field_value(a: &mut Operandi, ctx: &Context) -> String {
    let nodes = frame(a, ctx, vec![]);
    name_field(&nodes)
        .and_then(|n| n.value().map(str::to_owned))
        .unwrap_or_default()
}

/// Replace the focused field's text with typed key/text events.
fn type_name(a: &mut Operandi, ctx: &Context, text: &str) {
    let old = field_value(a, ctx).chars().count();
    let mut events = vec![key(Key::End)];
    events.extend((0..old).map(|_| key(Key::Backspace)));
    events.push(Event::Text(text.into()));
    frame(a, ctx, events);
}

fn open_from_panel(a: &mut Operandi, ctx: &Context) {
    click_button(a, ctx, "Set Part...");
    let nodes = frame(a, ctx, vec![]);
    assert!(a.set_part.is_some(), "dialog open: {}", a.status);
    assert!(
        nodes
            .iter()
            .any(|n| n.label().or(n.value()) == Some("Set Part"))
    );
    assert!(find(&nodes, Role::Button, "Cancel").is_some());
}

#[test]
fn part_006_panel_dialog_ok_matches_direct_command_and_menu_history() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let before = a.editor.clone();
    open_from_panel(&mut a, &ctx);
    // Single common selected tag prefills the draft; opening is history-neutral.
    assert_eq!(field_value(&mut a, &ctx), "SourceA");
    assert!(a.editor == before);
    type_name(&mut a, &ctx, "Road Part");
    assert_eq!(field_value(&mut a, &ctx), "Road Part");
    assert!(a.editor == before, "typing is draft-only");
    click_button(&mut a, &ctx, "OK");
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    let expected = retagged(&source.scene, "Road Part");
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.editor.history.undo.len(), 1);
    assert_eq!(a.editor.history.undo[0].label, "Set Part");

    // Direct command on a separate equal starting scene: same full result.
    let mut direct = app(&source);
    direct.execute("poly.setPart {Road Part}");
    assert_eq!(direct.editor.scene, a.editor.scene);
    assert_eq!(direct.editor.history.undo.len(), 1);
    // Identical repeat is neutral.
    direct.execute("poly.setPart \"Road Part\"");
    assert_eq!(direct.editor.scene, expected);
    assert_eq!(direct.editor.history.undo.len(), 1);
    assert!(direct.status.contains("already"), "{}", direct.status);

    a.menu_action(&ctx, app_menu::line_action("app.undo"));
    assert_eq!(a.editor.scene, source.scene);
    a.menu_action(&ctx, app_menu::line_action("app.redo"));
    assert_eq!(a.editor.scene, expected);
}

#[test]
fn part_003_command_opened_dialog_return_and_named_argument() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    frame(&mut a, &ctx, vec![]);
    a.execute("poly.setPart");
    // The opening frame ignores Return; the field already has focus.
    frame(&mut a, &ctx, vec![key(Key::Enter)]);
    assert!(a.set_part.is_some());
    assert_eq!(a.editor.scene, source.scene);
    type_name(&mut a, &ctx, "  Body  panel ");
    // UIC-85: the first Return commits the field, the next is OK.
    frame(&mut a, &ctx, vec![key(Key::Enter)]);
    assert!(a.set_part.is_some());
    frame(&mut a, &ctx, vec![key(Key::Enter)]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none(), "{}", a.status);
    assert_eq!(a.editor.scene, retagged(&source.scene, "  Body  panel "));

    let mut named = app(&source);
    named.execute("poly.setPart name:{  Body  panel }");
    assert_eq!(named.editor.scene, a.editor.scene);
    let mut upper = app(&source);
    upper.execute("POLY.SETPART M1PartB");
    assert_eq!(upper.editor.scene, retagged(&source.scene, "M1PartB"));
    let mut help = app(&source);
    help.execute("help poly.setPart");
    assert!(help.status.contains("poly.setPart {Road Part}"));
    help.execute("help");
    assert!(help.status.contains("poly.setPart"));
    assert!(help.editor == app(&source).editor);
}

#[test]
fn part_002_cancel_escape_and_list_choice_leave_scene_and_history_exact() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.run(Command::Selection(SelectionOperation::Grow));
    a.run(Command::Undo);
    let before = a.editor.clone();
    assert_eq!(before.history.redo.len(), 1);

    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Draft");
    click_button(&mut a, &ctx, "Cancel");
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    assert!(a.editor == before, "Cancel");

    open_from_panel(&mut a, &ctx);
    let nodes = frame(&mut a, &ctx, vec![]);
    // Skip the view menus, the Properties panel's Rotation Order list and
    // the modes bar's Polygon Tag Type popup.
    let combo = nodes
        .iter()
        .find(|n| {
            n.role() == Role::ComboBox
                && !matches!(
                    n.value(),
                    Some(
                        "XYZ"
                            | "ZXY"
                            | "Shaded"
                            | "Wireframe"
                            | "Perspective"
                            | "Top"
                            | "Front"
                            | "Right"
                            | "Material"
                            | crate::item_transform_ui::MENU_TEXT
                    )
                )
        })
        .expect("existing-name list")
        .clone();
    click(&mut a, &ctx, rect(&combo).center());
    let nodes = frame(&mut a, &ctx, vec![]);
    for name in ["Neighbor", "SourceA", "Unrelated"] {
        assert!(nodes.iter().any(|n| n.label() == Some(name)), "{name}");
    }
    let choice = nodes
        .iter()
        .find(|n| n.label() == Some("Unrelated"))
        .unwrap()
        .clone();
    click(&mut a, &ctx, rect(&choice).center());
    assert_eq!(field_value(&mut a, &ctx), "Unrelated");
    assert!(a.editor == before, "list choice is draft-only");
    // Close the list if still open, then Escape abandons the dialog.
    if egui::Popup::is_any_open(&ctx) {
        frame(&mut a, &ctx, vec![key(Key::Escape)]);
    }
    assert!(a.set_part.is_some());
    frame(&mut a, &ctx, vec![key(Key::Escape)]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    assert!(a.editor == before, "Escape");

    // Choosing then OK assigns the chosen existing name.
    open_from_panel(&mut a, &ctx);
    let nodes = frame(&mut a, &ctx, vec![]);
    let combo = nodes
        .iter()
        .find(|n| {
            n.role() == Role::ComboBox
                && !matches!(
                    n.value(),
                    Some(
                        "XYZ"
                            | "ZXY"
                            | "Shaded"
                            | "Wireframe"
                            | "Perspective"
                            | "Top"
                            | "Front"
                            | "Right"
                            | "Material"
                            | crate::item_transform_ui::MENU_TEXT
                    )
                )
        })
        .unwrap()
        .clone();
    click(&mut a, &ctx, rect(&combo).center());
    let nodes = frame(&mut a, &ctx, vec![]);
    let choice = nodes
        .iter()
        .find(|n| n.label() == Some("Neighbor"))
        .unwrap()
        .clone();
    click(&mut a, &ctx, rect(&choice).center());
    click_button(&mut a, &ctx, "OK");
    frame(&mut a, &ctx, vec![]);
    assert_eq!(a.editor.scene, retagged(&before.scene, "Neighbor"));
    assert_eq!(a.editor.history.undo.len(), before.history.undo.len() + 1);
    assert!(a.editor.history.redo.is_empty());
}

#[test]
fn part_002_shortcuts_do_not_fire_while_typing_and_blank_is_refused() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let before = a.editor.clone();
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "");
    // W (Move), B (Bevel) and Q (Drop) are bound by default.
    for (k, t) in [(Key::W, "w"), (Key::B, "b"), (Key::Q, "q")] {
        frame(&mut a, &ctx, vec![key(k), Event::Text(t.into())]);
    }
    assert_eq!(field_value(&mut a, &ctx), "wbq");
    assert!(a.editor == before, "no tool activated while typing");
    type_name(&mut a, &ctx, "   ");
    let nodes = frame(&mut a, &ctx, vec![]);
    let ok = find(&nodes, Role::Button, "OK").unwrap();
    assert!(ok.is_disabled(), "blank name disables OK");
    assert!(
        nodes
            .iter()
            .any(|n| n.label().or(n.value()).is_some_and(|t| t.contains("blank")))
    );
    frame(&mut a, &ctx, vec![key(Key::Enter)]);
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_some(), "Return on blank keeps the dialog");
    assert!(a.editor == before);
    a.execute("poly.setPart \"  \"");
    assert!(a.status.contains("blank"));
    assert!(a.editor == before);
}

#[test]
fn part_003_malformed_commands_and_unsupported_contexts_refuse_atomically() {
    let source = fixture();
    let mut a = app(&source);
    let before = a.editor.clone();
    for bad in [
        "poly.setPart a b",
        "poly.setPart {Road Part",
        "poly.setPart \"Road",
        "poly.setPart name:a name:b",
        "poly.setPart name:",
        "poly.setPart ?",
    ] {
        a.execute(bad);
        assert!(a.editor == before, "{bad}");
        assert!(a.set_part.is_none(), "{bad}");
        assert_ne!(a.status, "Ready", "{bad}");
    }
}

/// SP-05: with nothing selected the dialog opens and OK names every polygon
/// of the active mesh in one undo step. The two polygons have different
/// parts, so the Name field starts from the Default Part preference.
#[test]
fn sp_05_empty_selection_names_the_whole_mesh() {
    let mut empty = fixture();
    empty.scene.selection.faces.clear();
    let ctx = context();
    let mut e = app(&empty);
    e.defaults_preferences.default_part = "Panel".into();
    e.execute("poly.setPart");
    let dialog = e
        .set_part
        .as_ref()
        .expect("dialog opens with nothing selected");
    assert_eq!(dialog.name(), "Panel");
    frame(&mut e, &ctx, vec![]);
    // The Name field has focus: Return commits it, the next Return is OK.
    frame(&mut e, &ctx, vec![key(Key::Enter)]);
    frame(&mut e, &ctx, vec![key(Key::Enter)]);
    assert!(e.set_part.is_none(), "{}", e.status);
    let mut expected = empty.scene.clone();
    for f in &mut expected.active_mut().unwrap().mesh.faces {
        f.part = Some("Panel".into());
    }
    assert_eq!(e.editor.scene, expected);
    assert_eq!(e.editor.history.undo.len(), 1);
    // Cancel (SP-C04) changes nothing and adds no undo step.
    let mut c = app(&empty);
    c.execute("poly.setPart");
    frame(&mut c, &ctx, vec![]);
    frame(&mut c, &ctx, vec![key(Key::Escape)]);
    assert!(c.set_part.is_none());
    assert_eq!(c.editor.scene, empty.scene);
    assert!(c.editor.history.undo.is_empty());
}

/// SP-Q03 (Jason's call): the Name field shows the part the targets share,
/// and the Default Part preference only when they don't share one.
#[test]
fn sp_q03_name_field_prefers_the_shared_part() {
    let source = fixture();
    let mut shared = app(&source);
    shared.defaults_preferences.default_part = "Panel".into();
    shared.execute("poly.setPart");
    assert_eq!(shared.set_part.as_ref().unwrap().name(), "SourceA");
    let mut untagged = source.clone();
    untagged.scene.active_mut().unwrap().mesh.faces[0].part = None;
    let mut fresh = app(&untagged);
    fresh.defaults_preferences.default_part = "Panel".into();
    fresh.execute("poly.setPart");
    assert_eq!(fresh.set_part.as_ref().unwrap().name(), "Panel");
}

/// SP-15: Geometry > Polygon > Set Part... opens the same dialog.
#[test]
fn sp_15_geometry_menu_opens_set_part() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let menus = a.menu_model(&ctx);
    let geometry = menus.iter().find(|m| m.title == "Geometry").unwrap();
    let polygon = geometry
        .entries
        .iter()
        .find_map(|e| match e {
            app_menu::Entry::Submenu(m) if m.title == "Polygon" => Some(m),
            _ => None,
        })
        .unwrap();
    assert!(polygon.entries.iter().any(|e| matches!(
        e,
        app_menu::Entry::Item(i) if i.action == app_menu::line_action("Set Part") && i.title == "Set Part..."
    )));
    a.menu_action(&ctx, app_menu::line_action("Set Part"));
    assert!(a.set_part.is_some(), "{}", a.status);
}

#[test]
fn part_006_active_tool_draft_is_preserved_by_refusal() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.execute("spike");
    assert!(a.editor.session.spike.is_some(), "{}", a.status);
    a.fields
        .insert("spi_strength".into(), "not a number".into());
    let editor = a.editor.clone();
    let fields = a.fields.clone();
    a.execute("poly.setPart X");
    assert!(a.status.contains("Drop or Cancel"), "{}", a.status);
    a.execute("set part");
    assert!(a.set_part.is_none());
    assert!(a.editor == editor);
    assert_eq!(a.fields, fields);
    let _ = ctx;
}

#[test]
fn part_002_stale_target_on_commit_is_refused_without_applying_elsewhere() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Road Part");
    // Fixture-level change behind the open modal (no GUI route reaches it).
    a.editor.scene.selection.faces = vec![1];
    let changed = a.editor.clone();
    click_button(&mut a, &ctx, "OK");
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    assert!(a.status.contains("changed"), "{}", a.status);
    assert!(a.editor == changed);
}

#[test]
fn part_005_input_map_action_has_no_default_and_user_remap_opens_dialog() {
    assert_eq!(input::default_bindings()["Set Part"], "");
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    a.bindings.insert("Set Part".into(), "Shift+P".into());
    frame(&mut a, &ctx, vec![]);
    let pos = a.viewport_rect.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(pos)]);
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::P,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        }],
    );
    assert!(a.set_part.is_some(), "{}", a.status);
    assert_eq!(a.editor.scene, source.scene);
}

#[test]
fn part_006_task_native_save_open_preserves_full_scene() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Road Part");
    click_button(&mut a, &ctx, "OK");
    let authored = a.editor.scene.clone();
    let path = a.environment.paths().data_root().join("set-part.opscene");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    assert!(a.save(path.clone()), "{}", a.status);
    let mut reopened = Operandi::empty();
    reopened.open(path);
    assert_eq!(reopened.editor.scene, authored);
    assert_eq!(
        reopened.editor.scene.active().unwrap().mesh.faces[0]
            .part
            .as_deref(),
        Some("Road Part")
    );
    let mut direct = app(&source);
    direct.execute("poly.setPart {Road Part}");
    assert_eq!(direct.editor.scene, reopened.editor.scene);
}

/// Amendment 1 (review P2): the dialog's target belongs to the document that
/// was open when it was captured. Replacing the document through the real
/// `open` route with an equal-valued saved copy (same item IDs, selection and
/// mesh values) must not accept the obsolete target. Ordinary UI cannot reach
/// `open` while the modal is up (`request_open_path`, menus and shortcuts are
/// gated by `has_modal`); this drives the route directly to prove the commit
/// boundary.
#[test]
fn amendment_1_equal_valued_replacement_document_refuses_open_dialog_commit() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let path = a
        .environment
        .paths()
        .data_root()
        .join("equal-valued.opscene");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // Fixture file: the same scene written by the native format.
    let _ = std::fs::remove_file(&path);
    persistence::save(&path, &a.editor.scene).unwrap();
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Road Part");
    a.open(path);
    assert_eq!(a.editor.scene, source.scene, "equal-valued replacement");
    assert!(a.set_part.is_some());
    let replaced = a.editor.clone();
    click_button(&mut a, &ctx, "OK");
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none());
    assert!(a.status.contains("another document"), "{}", a.status);
    assert_eq!(a.editor.scene, replaced.scene);
    assert_eq!(a.editor.history.undo.len(), replaced.history.undo.len());
    // A dialog opened on the replacement document commits normally.
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Road Part");
    click_button(&mut a, &ctx, "OK");
    frame(&mut a, &ctx, vec![]);
    assert_eq!(a.editor.scene, retagged(&source.scene, "Road Part"));
}

/// Amendment 1 (review note): existing close routes. Operandi's modals have no
/// title-bar close control; the dialog closes by Cancel, Escape (tested
/// above), a click outside it on the modal backdrop, or the application's
/// window-close/quit request, whose handler in `update` calls
/// `request_file(FileAction::Close)` (called here directly; `update` itself
/// needs an eframe frame).
#[test]
fn amendment_1_backdrop_and_window_close_routes_abandon_only_the_draft() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let before = a.editor.clone();
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Draft");
    click(&mut a, &ctx, pos2(8., SIZE.y - 8.));
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none(), "backdrop click closes");
    assert!(a.editor == before, "backdrop close");

    // Clean document: the close request proceeds to close; nothing applied.
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Draft");
    a.request_file(FileAction::Close);
    assert!(a.allow_close);
    assert!(a.editor == before, "clean close");

    // Dirty document: the unsaved-work prompt replaces the dialog, which is
    // abandoned without assignment; cancelling the prompt keeps it closed.
    let mut a = app(&source);
    a.execute("poly.setPart Prior");
    let dirty = a.editor.clone();
    assert!(a.dirty());
    open_from_panel(&mut a, &ctx);
    type_name(&mut a, &ctx, "Draft");
    a.request_file(FileAction::Close);
    assert!(a.pending.is_some());
    frame(&mut a, &ctx, vec![]);
    assert!(a.set_part.is_none(), "{}", a.status);
    assert!(a.editor == dirty, "dirty close");
    click_button(&mut a, &ctx, "Cancel");
    frame(&mut a, &ctx, vec![]);
    assert!(a.pending.is_none() && a.set_part.is_none());
    assert!(a.editor == dirty);
}

/// Original-compatible A6's memory: every Set Part that applies (dialog or
/// command) records its name for the next dialog; a refused one doesn't.
#[test]
fn a6_set_part_remembers_the_last_name_applied() {
    let source = fixture();
    let mut a = app(&source);
    assert_eq!(a.last_set_part, None);
    a.execute("poly.setPart {Road Part}");
    assert_eq!(a.last_set_part.as_deref(), Some("Road Part"));
    a.execute("poly.setPart Hub");
    assert_eq!(a.last_set_part.as_deref(), Some("Hub"));
    a.execute("poly.setPart a b");
    assert_eq!(a.last_set_part.as_deref(), Some("Hub"), "{}", a.status);
}
