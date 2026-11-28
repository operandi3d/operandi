use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
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
                material: ["Cool Stone", "Warm Clay"][i].into(),
                smooth: i == 0,
                part: Some(part.into()),
                material_label: Some(["Cool Stone", "Warm Clay"][i].into()),
            })
            .collect(),
        ..Default::default()
    };
    mesh.uv_maps
        .insert("Texture".into(), vec![vec![Some(DVec2::ZERO); 4]; 2]);
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
    s.active_mut().unwrap().mesh.faces[0].material = name.into();
    s.active_mut().unwrap().mesh.faces[0].material_label = Some(name.into());
    // The tag also gets its shader tree group (ST-1); factory settings. A
    // tree never edited first adopts the scene's other tags (ST-4).
    s.shader_tree = scene.render_shader_tree().into_owned();
    s.shader_tree
        .ensure_material(name, &operandi_core::Material::default())
        .unwrap();
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

#[test]
fn material_panel_and_command_compare_full_scene_and_history() {
    let source = fixture();
    let expected = retagged(&source.scene, "Warm Clay");
    // The dialog also stores the material's settings (factory ones here);
    // the name-only command leaves the material table alone.
    let mut panel_expected = expected.clone();
    panel_expected
        .materials
        .insert("Warm Clay".into(), operandi_core::Material::default());
    let ctx = context();
    let mut a = app(&source);
    let before = a.editor.clone();
    click_button(&mut a, &ctx, "Set Material...");
    assert!(a.set_material.is_some());
    assert!(a.editor == before);
    type_name(&mut a, &ctx, "Warm Clay");
    assert!(a.editor == before);
    click_button(&mut a, &ctx, "OK");
    assert!(a.set_material.is_none(), "{}", a.status);
    assert_eq!(a.editor.scene, panel_expected);
    let ids = a.editor.identities.current.clone();
    a.execute("undo");
    assert_eq!(a.editor.scene, source.scene);
    a.execute("redo");
    assert_eq!(a.editor.scene, panel_expected);
    assert_eq!(a.editor.identities.current, ids);
    let mut direct = app(&source);
    direct.execute("poly.setMaterial {Warm Clay}");
    assert_eq!(direct.editor.scene, expected);
}
#[test]
fn material_cancel_refusal_stale_and_focus() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    let before = a.editor.clone();
    a.execute("poly.setMaterial");
    assert!(a.set_material.is_some());
    frame(&mut a, &ctx, vec![]);
    type_name(&mut a, &ctx, "M");
    assert!(a.set_material.is_some());
    click_button(&mut a, &ctx, "Cancel");
    assert!(a.editor == before);
    for cmd in [
        "poly.setMaterial ?",
        "poly.setMaterial {}",
        "poly.setMaterial a b",
    ] {
        a.execute(cmd);
        assert!(a.editor == before);
    }
    a.execute("help poly.setMaterial");
    assert!(a.status.contains("creating"));
    a.execute("poly.setMaterial");
    a.editor = fixture();
    let scene = a.editor.scene.clone();
    frame(&mut a, &ctx, vec![]);
    type_name(&mut a, &ctx, "Warm Clay");
    click_button(&mut a, &ctx, "OK");
    assert_eq!(a.editor.scene, scene);
    assert!(a.set_material.is_some());
}
#[test]
fn material_row_order_and_m_remap() {
    let source = fixture();
    let ctx = context();
    let mut a = app(&source);
    frame(&mut a, &ctx, vec![]);
    // Jason 2026-11-05: Set Part and Set Material come after Align;
    // SP-14: Set Part sits directly above Set Material.
    assert!(button_by_line(&a, "Polygon Align").bottom() <= button_by_line(&a, "Set Part").top());
    assert!(button_by_line(&a, "Set Part").bottom() <= button_by_line(&a, "Set Material").top());
    assert_eq!(a.bindings["Set Material"], "M");
    let p = a.viewport_rect.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(p), key(Key::M)]);
    assert!(a.set_material.is_some());
    frame(&mut a, &ctx, vec![key(Key::Escape)]);
    // Escape closed it (UIC-85); J is free once its default owner lets go.
    assert!(a.set_material.is_none());
    a.bindings.retain(|_, k| k != "J");
    a.bindings.insert("Set Material".into(), "J".into());
    frame(&mut a, &ctx, vec![Event::PointerMoved(p), key(Key::J)]);
    assert!(a.set_material.is_some());
}
#[test]
fn material_parked_redo_cancel_noop_and_success() {
    let mut e = fixture();
    e.scene.active_mut().unwrap().mesh.faces[0].material = "Warm Clay".into();
    e.scene.active_mut().unwrap().mesh.faces[0].material_label = Some("Warm Clay".into());
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .polygon_sets
        .clear();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .corner_weights
        .clear();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .corner_colors
        .clear();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(e.i2_can_redo());
    let mut a = app(&e);
    let parked = a.editor.i2.selected.clone();
    let source = a.editor.scene.clone();
    let ctx = context();
    a.execute("poly.setMaterial");
    frame(&mut a, &ctx, vec![]);
    click_button(&mut a, &ctx, "Cancel");
    assert_eq!(a.editor.scene, source);
    assert_eq!(a.editor.i2.selected, parked);
    assert!(a.editor.i2_can_redo());
    a.execute("poly.setMaterial {Warm Clay}");
    assert_eq!(a.editor.scene, source);
    assert_eq!(a.editor.i2.selected, parked);
    assert!(a.editor.i2_can_redo());
    a.execute("poly.setMaterial ?");
    assert_eq!(a.editor.scene, source);
    assert_eq!(a.editor.i2.selected, parked);
    assert!(a.editor.i2_can_redo());
}
#[test]
fn material_escape_discards_draft() {
    let source = fixture();
    let mut a = app(&source);
    let ctx = context();
    a.execute("poly.setMaterial");
    frame(&mut a, &ctx, vec![]);
    type_name(&mut a, &ctx, "Warm Clay");
    frame(&mut a, &ctx, vec![key(Key::Escape)]);
    assert!(a.set_material.is_none());
    assert_eq!(a.editor.scene, source.scene);
}

#[test]
fn material_close_discards_draft() {
    let source = fixture();
    let mut a = app(&source);
    let ctx = context();
    a.execute("poly.setMaterial");
    frame(&mut a, &ctx, vec![]);
    type_name(&mut a, &ctx, "Warm Clay");
    // UIC-82: no close box; a click outside the dialog closes it.
    click(&mut a, &ctx, pos2(8., 8.));
    assert!(a.set_material.is_none());
    assert_eq!(a.editor.scene, source.scene);
}
#[test]
fn material_parked_redo_modal_edit_branches_and_undoes() {
    let mut e = fixture();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .polygon_sets
        .clear();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .corner_weights
        .clear();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .corner_colors
        .clear();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(e.i2_can_redo());
    let source = e.scene.clone();
    let mut expected = retagged(&source, "Warm Clay");
    expected
        .materials
        .insert("Warm Clay".into(), operandi_core::Material::default());
    let mut a = app(&e);
    let ctx = context();
    a.execute("poly.setMaterial");
    frame(&mut a, &ctx, vec![]);
    type_name(&mut a, &ctx, "Warm Clay");
    click_button(&mut a, &ctx, "OK");
    assert_eq!(a.editor.scene, expected);
    assert!(a.editor.i2.selected.is_none());
    a.execute("undo");
    assert_eq!(a.editor.scene, source);
}
#[test]
fn active_pen_blocks_material_preflight_without_consuming_draft() {
    let mut e = fixture();
    let item = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenCapture(DVec3::new(0., 0., 0.)))
        .unwrap();
    e.scene.select_item(Some(item));
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    let mut a = app(&e);
    let before = a.editor.clone();
    let ctx = context();
    a.execute("poly.setMaterial");
    assert!(a.set_material.is_none());
    assert!(a.editor == before);
    assert!(a.status.contains("Pen"));
    frame(&mut a, &ctx, vec![]);
    click_button(&mut a, &ctx, "Set Material...");
    assert!(a.set_material.is_none());
    assert!(a.editor == before);
    assert!(a.status.contains("Pen"));
}

#[test]
fn competing_position_material_and_tool_preserve_parked_redo() {
    let mut e = fixture();
    let attributes = &mut e.scene.active_mut().unwrap().mesh.attributes;
    attributes.polygon_sets.clear();
    attributes.corner_weights.clear();
    attributes.corner_colors.clear();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(e.i2_can_redo());
    let mut a = app(&e);
    let ctx = context();
    let before = a.editor.clone();
    a.execute("poly.setMaterial");
    assert!(a.set_material.is_some());
    a.execute("vert.set y 2m false false");
    assert!(a.editor == before);
    assert!(a.editor.i2_can_redo());
    a.execute("tool.set prim.cube on");
    assert!(a.editor == before);
    assert!(a.editor.i2_can_redo());
    frame(&mut a, &ctx, vec![]);
    click_button(&mut a, &ctx, "Cancel");
    assert!(a.editor == before);
    a.execute("vert.set");
    assert!(a.set_position.is_some());
    a.execute("poly.setMaterial {Warm Clay}");
    assert!(a.editor == before);
    assert!(a.editor.i2_can_redo());
    frame(&mut a, &ctx, vec![]);
    click_button(&mut a, &ctx, "Cancel");
    assert!(a.editor == before);
    a.run(Command::Redo);
    assert!(a.editor.i2_active());
    let active = a.editor.clone();
    a.execute("poly.setMaterial");
    assert!(a.set_material.is_none());
    assert!(a.editor == active);
    a.execute("poly.setMaterial {Warm Clay}");
    assert!(a.editor == active);
}

#[test]
fn saved_pre_material_m_binding_and_explicit_material_choices() {
    use std::io::Write;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-integration-a34-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("material-binding-{}.json", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let legacy = BTreeMap::from([("Triple".to_owned(), "M".to_owned())]);
    file.write_all(&serde_json::to_vec(&legacy).unwrap())
        .unwrap();
    drop(file);
    let bindings = input::load_bindings(&path);
    assert_eq!(bindings["Triple"], "M");
    assert_eq!(bindings["Set Material"], "");
    let mut source = fixture();
    source
        .scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .representations = vec![PolygonRepresentation::Face; 2];
    let mut a = app(&source);
    a.bindings = bindings;
    let ctx = context();
    frame(&mut a, &ctx, vec![]);
    let p = a.viewport_rect.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(p), key(Key::M)]);
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces.len(),
        3,
        "{}",
        a.status
    );
    assert!(a.set_material.is_none());
    for (value, expected) in [("J", "J"), ("", ""), ("M", "M")] {
        let mut stored = legacy.clone();
        stored.insert("Set Material".into(), value.into());
        std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
        let merged = input::load_bindings(&path);
        assert_eq!(merged["Triple"], "M");
        assert_eq!(merged["Set Material"], expected);
        let mut other = app(&source);
        other.bindings = merged;
        let c = context();
        frame(&mut other, &c, vec![]);
        let p = other.viewport_rect.center();
        if value == "J" {
            frame(&mut other, &c, vec![Event::PointerMoved(p), key(Key::J)]);
            assert!(other.set_material.is_some());
        } else if value == "M" {
            let before = other.editor.scene.clone();
            frame(&mut other, &c, vec![Event::PointerMoved(p), key(Key::M)]);
            assert_eq!(other.editor.scene, before);
            assert!(other.set_material.is_none());
            assert!(other.status.contains("conflict"));
        } else {
            frame(&mut other, &c, vec![Event::PointerMoved(p), key(Key::M)]);
            assert_eq!(other.editor.scene.active().unwrap().mesh.faces.len(), 3);
        }
    }
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn factory_material_m_respects_field_and_modal_focus() {
    let source = fixture();
    let mut a = app(&source);
    let ctx = context();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-integration-a34-20261029/input-map-tests/nonexistent-factory-map.json");
    assert!(!path.exists());
    a.bindings = input::load_bindings(&path);
    assert_eq!(a.bindings["Set Material"], "M");
    frame(&mut a, &ctx, vec![]);
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &ctx, vec![key(Key::M), Event::Text("m".into())]);
    assert!(a.set_material.is_none());
    ctx.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::M,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let p = a.viewport_rect.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(p), key(Key::M)]);
    assert!(a.set_material.is_some());
    let before = a.editor.scene.clone();
    frame(&mut a, &ctx, vec![Event::Text("m".into()), key(Key::M)]);
    assert!(a.set_material.is_some());
    assert_eq!(a.editor.scene, before);
}

#[test]
fn material_and_position_refuse_armed_reduction_without_changing_state() {
    let source = fixture();
    let mut a = app(&source);
    a.run(Command::ArmReduction);
    assert!(a.editor.session.reduction.is_some(), "{}", a.status);
    let before = a.editor.clone();
    a.execute("poly.setMaterial");
    assert!(a.set_material.is_none());
    assert!(a.editor == before);
    a.execute("vert.set");
    assert!(a.set_position.is_none());
    assert!(a.editor == before);
    // UIC-307: with its value given the command runs; Reduction settles
    // and arms again.
    a.execute("poly.setMaterial {Warm Clay}");
    assert!(a.editor.session.reduction.is_some(), "{}", a.status);
}

#[test]
fn dialog_opens_with_defaults_and_ok_stores_the_settings_in_one_step() {
    let mut source = fixture();
    // An untagged selection opens on Default with factory settings.
    source.scene.active_mut().unwrap().mesh.faces[0].material = String::new();
    source.scene.active_mut().unwrap().mesh.faces[0].material_label = None;
    let ctx = context();
    let mut a = app(&source);
    a.execute("poly.setMaterial");
    let dialog = a.set_material.as_ref().unwrap();
    assert_eq!(dialog.name(), "Default");
    assert_eq!(dialog.material(), operandi_core::Material::default());
    frame(&mut a, &ctx, vec![]);
    let nodes = frame(&mut a, &ctx, vec![]);
    for label in ["Smoothing", "Make Default", "Add to Library"] {
        assert!(find(&nodes, Role::CheckBox, label).is_some(), "{label}");
    }
    type_name(&mut a, &ctx, "Lacquer");
    let lacquer = operandi_core::Material {
        color: [0.7, 0.1, 0.1],
        diffuse: 0.5,
        specular: 0.3,
        smoothing: false,
        add_to_library: true,
        ..Default::default()
    };
    a.set_material.as_mut().unwrap().show_material(&lacquer);
    let depth = a.editor.history.undo.len();
    click_button(&mut a, &ctx, "OK");
    assert!(a.set_material.is_none(), "{}", a.status);
    assert_eq!(a.editor.history.undo.len(), depth + 1);
    assert_eq!(a.editor.scene.material("Lacquer"), Some(&lacquer));
    assert_eq!(
        a.editor.scene.active().unwrap().mesh.faces[0].material,
        "Lacquer"
    );
    // Reopening on the same polygon shows the stored settings.
    a.execute("poly.setMaterial");
    let dialog = a.set_material.as_ref().unwrap();
    assert_eq!(dialog.name(), "Lacquer");
    assert_eq!(dialog.material(), lacquer);
    click_button(&mut a, &ctx, "Cancel");
    a.execute("undo");
    assert_eq!(a.editor.scene, source.scene);
}

#[test]
fn typing_an_existing_name_loads_its_settings() {
    let mut source = fixture();
    let clay = operandi_core::Material {
        color: [0.6, 0.3, 0.2],
        ..Default::default()
    };
    source
        .scene
        .materials
        .insert("Warm Clay".into(), clay.clone());
    let ctx = context();
    let mut a = app(&source);
    a.execute("poly.setMaterial");
    // Face 0 is Cool Stone, which has no stored settings.
    assert_eq!(a.set_material.as_ref().unwrap().name(), "Cool Stone");
    assert_eq!(
        a.set_material.as_ref().unwrap().material(),
        operandi_core::Material::default()
    );
    frame(&mut a, &ctx, vec![]);
    type_name(&mut a, &ctx, "Warm Clay");
    frame(&mut a, &ctx, vec![]);
    assert_eq!(a.set_material.as_ref().unwrap().material(), clay);
    // Escape cancels without a change.
    frame(&mut a, &ctx, vec![key(Key::Escape)]);
    assert!(a.set_material.is_none());
    assert_eq!(a.editor.scene, source.scene);
}

/// Like Set Part (SP-05): with nothing selected the dialog opens, and OK
/// tags every polygon of the active mesh in one undo step. The polygons have
/// different materials, so the Name field starts at Default.
#[test]
fn material_empty_selection_tags_the_whole_mesh() {
    let mut empty = fixture();
    empty.scene.selection.faces.clear();
    let ctx = context();
    let mut a = app(&empty);
    click_button(&mut a, &ctx, "Set Material...");
    let dialog = a
        .set_material
        .as_ref()
        .expect("dialog opens with nothing selected");
    assert_eq!(dialog.name(), operandi_core::material::DEFAULT_MATERIAL);
    type_name(&mut a, &ctx, "Warm Clay");
    click_button(&mut a, &ctx, "OK");
    assert!(a.set_material.is_none(), "{}", a.status);
    let mut expected = empty.scene.clone();
    for f in &mut expected.active_mut().unwrap().mesh.faces {
        f.material = "Warm Clay".into();
        f.material_label = Some("Warm Clay".into());
    }
    expected
        .materials
        .insert("Warm Clay".into(), operandi_core::Material::default());
    // The tag also gets its shader tree group; a tree never edited first
    // adopts the scene's other tags (ST-1, ST-4).
    expected.shader_tree = empty.scene.render_shader_tree().into_owned();
    expected
        .shader_tree
        .ensure_material("Warm Clay", &operandi_core::Material::default())
        .unwrap();
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.editor.history.undo.len(), 1);
    // The name-only command does the same without the dialog.
    let mut direct = app(&empty);
    direct.execute("poly.setMaterial {Warm Clay}");
    expected.materials.clear();
    assert_eq!(direct.editor.scene, expected);
}
