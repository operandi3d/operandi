//! MP-C13 SO-U01..04 application checks for Scale Offset, driven through the
//! actual Polygon row, the Enter Inset Amount widgets, the command entry and
//! the input map. Expected scenes are the source scene with literal corner
//! positions written in (core fixtures SO-F03/SO-AFF-F01, the hand-derived
//! rhombus inset and the rich square); none is produced by running the
//! operation. Geometry itself is covered by the core test targets; native
//! input, display scale and GUI save/relaunch remain separate gates.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use egui::accesskit::{self, Role};
use glam::{DVec2, DVec4};

const SIZE: Vec2 = vec2(1440., 2000.);
const ACTION: &str = "@ppi_PolyScale_Offset.py";

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

fn square() -> Vec<DVec3> {
    vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 2., 0.),
        DVec3::new(0., 2., 0.),
    ]
}

/// One mesh with `vertices` as face 0 (all corners in order), selected in
/// Polygon mode with the Polygon toolbox shown.
fn app_with(vertices: Vec<DVec3>) -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let corners = (0..vertices.len() as u32).collect();
    *a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
        vertices,
        faces: vec![face(corners)].into(),
        ..Default::default()
    };
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.editor.scene.validate().unwrap();
    a.editor.synchronize_identities().unwrap();
    a.category = toolbox::Category::Polygon;
    a
}

/// Square face with authored corner/point/face data, a far disjoint
/// triangle, an unused distant point and a second unrelated item.
fn rich_app() -> Operandi {
    let mut a = app_with(square());
    let id = a.editor.scene.selection.item;
    let m = &mut *a.editor.scene.active_mut().unwrap().mesh;
    m.vertices.extend([
        DVec3::new(500., 0., 0.),
        DVec3::new(501., 0., 0.),
        DVec3::new(500., 1., 0.),
        DVec3::new(4e8, -4e8, 2e8),
    ]);
    m.faces[0] = Face {
        vertices: vec![0, 1, 2, 3],
        uv: Some(vec![
            DVec2::new(0., 0.),
            DVec2::new(1., 0.),
            DVec2::new(1., 1.),
            DVec2::new(0., 1.),
        ]),
        material: "Steel".into(),
        smooth: true,
        part: Some("Panel".into()),
        material_label: Some("Brushed".into()),
    };
    m.faces.push(face(vec![4, 5, 6]));
    m.uv_maps.insert(
        "UV".into(),
        vec![
            vec![
                Some(DVec2::new(0.1, 0.2)),
                None,
                Some(DVec2::new(0.5, 0.7)),
                None,
            ],
            vec![None; 3],
        ],
    );
    let at = &mut m.attributes;
    at.uv_records.insert(
        "UV".into(),
        vec![vec![Some(1), None, Some(2), None], vec![None; 3]],
    );
    at.corner_weights.insert(
        "w".into(),
        vec![vec![Some(0.2), None, None, Some(0.8)], vec![None; 3]],
    );
    at.corner_colors.insert(
        "c".into(),
        vec![
            vec![None, Some(DVec4::new(0.2, 0.3, 0.4, 1.)), None, None],
            vec![None; 3],
        ],
    );
    at.corner_normals.insert(
        "n".into(),
        vec![vec![None, None, Some(DVec3::Z), None], vec![None; 3]],
    );
    at.point_weights
        .insert("pw".into(), [(0, 0.25), (2, 0.5), (7, 1.)].into());
    at.point_colors
        .insert("pc".into(), [(3, DVec4::new(0.9, 0.1, 0.1, 1.))].into());
    at.relative_morphs
        .insert("lift".into(), [(1, DVec3::new(0., 0., 0.5))].into());
    at.smoothing_labels = vec![Some("Group A".into()), None];
    at.representations = vec![PolygonRepresentation::Subdivs, PolygonRepresentation::Face];
    at.creases = vec![MeshCrease {
        a: 0,
        b: 1,
        weight: 0.4,
    }];
    at.polygon_sets.insert("Panels".into(), [0].into());
    at.vertex_sets.insert("Marked".into(), [0, 2, 7].into());
    let selection = a.editor.scene.selection.clone();
    a.run(Command::NewMesh);
    let other = a.editor.scene.active_mut().unwrap();
    other.name = "Unrelated".into();
    other.transform.translation = DVec3::new(-3., 1., 2.);
    a.editor.scene.selection = selection;
    a.editor.scene.selection.item = id;
    a.editor.scene.selection.vertices = vec![2, 7];
    a.editor.scene.selection.edges = vec![(0, 1)];
    a.editor.scene.validate().unwrap();
    a.editor.synchronize_identities().unwrap();
    a
}

/// `scene` with the active item's first `corners.len()` points replaced.
fn literal(scene: &Scene, corners: &[[f64; 3]]) -> Scene {
    let mut expected = scene.clone();
    let v = &mut expected.active_mut().unwrap().mesh.vertices;
    for (i, c) in corners.iter().enumerate() {
        v[i] = DVec3::from(*c);
    }
    expected
}

fn square_inset(d: f64, sx: f64, sy: f64) -> Vec<[f64; 3]> {
    let (x, y) = (d / sx, d / sy);
    vec![
        [x, y, 0.],
        [2. - x, y, 0.],
        [2. - x, 2. - y, 0.],
        [x, 2. - y, 0.],
    ]
}

/// Full scene equality allowing `tolerance` on vertex coordinates only.
fn assert_scene(actual: &Scene, expected: &Scene, tolerance: f64) {
    let mut normalized = actual.clone();
    for (a, b) in normalized.items.iter_mut().zip(&expected.items) {
        assert_eq!(a.mesh.vertices.len(), b.mesh.vertices.len());
        for (p, q) in a.mesh.vertices.iter().zip(&b.mesh.vertices) {
            assert!(p.distance(*q) <= tolerance, "{p} != {q}");
        }
        a.mesh.vertices = b.mesh.vertices.clone();
    }
    assert_eq!(&normalized, expected);
}

fn context() -> Context {
    let c = Context::default();
    theme::install(&c);
    c.enable_accesskit();
    c.style_mut(|s| {
        s.interaction.tooltip_delay = 0.;
        s.interaction.show_tooltips_only_when_still = false;
    });
    c
}
fn frame(a: &mut Operandi, c: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    draw(a, c, SIZE, events)
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}
fn rect(n: &accesskit::Node) -> Rect {
    let b = n.bounds().unwrap();
    Rect::from_min_max(
        pos2(b.x0 as f32, b.y0 as f32),
        pos2(b.x1 as f32, b.y1 as f32),
    )
}
fn labels(nodes: &[accesskit::Node]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|n| n.label().or(n.value()).map(str::to_string))
        .collect()
}
fn find(nodes: &[accesskit::Node], role: Role, name: &str) -> Option<accesskit::Node> {
    nodes
        .iter()
        .find(|n| n.role() == role && n.label().or(n.value()) == Some(name))
        .cloned()
}
fn click(a: &mut Operandi, c: &Context, p: Pos2) {
    frame(a, c, vec![Event::PointerMoved(p)]);
    frame(a, c, vec![pointer(p, true, Modifiers::NONE)]);
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
fn button(a: &mut Operandi, c: &Context, name: &str) {
    let nodes = frame(a, c, vec![]);
    let p = rect(&find(&nodes, Role::Button, name).unwrap_or_else(|| panic!("no {name}"))).center();
    click(a, c, p);
}
fn click_row(a: &mut Operandi, c: &Context) {
    frame(a, c, vec![]);
    let p = button_by_line(a, "Scale Offset").center();
    click(a, c, p);
}
/// The Inset field: the dialog's only text input.
fn inset_field(nodes: &[accesskit::Node]) -> accesskit::Node {
    let ok = rect(&find(nodes, Role::Button, "OK").expect("dialog OK"));
    nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput)
        .min_by_key(|n| rect(n).center().distance(ok.center()) as i32)
        .unwrap()
        .clone()
}
fn type_inset(a: &mut Operandi, c: &Context, text: &str) {
    // The modal re-centres one frame after its content changes (for example
    // when a refusal appears), so let it settle before locating the field.
    frame(a, c, vec![]);
    let nodes = frame(a, c, vec![]);
    let field = inset_field(&nodes);
    let old = field.value().unwrap_or_default().chars().count();
    click(a, c, rect(&field).center());
    let mut events = vec![key(Key::End)];
    events.extend((0..old).map(|_| key(Key::Backspace)));
    events.push(Event::Text(text.into()));
    frame(a, c, events);
}

/// Arranges one refused-open context.
type Setup = fn(&mut Operandi);

/// Editor equality except the shared runtime publication/request ticket
/// counters, which every accepted core dispatch advances (core no-change
/// contract; they are not Scene, component identity or history state).
fn assert_editor_unchanged(a: &Operandi, before: &Editor) {
    let mut normalized = a.editor.clone();
    normalized.identities.publication = before.identities.publication;
    normalized.identities.request = before.identities.request;
    assert!(normalized == *before, "editor changed: {}", a.status);
}

#[test]
fn row_dialog_and_command_entry_apply_literal_square_results() {
    let mut a = app_with(square());
    let c = context();
    let before = a.editor.scene.clone();
    let ids = a.editor.identities.current.clone();
    let undo = a.editor.history.undo.len();
    click_row(&mut a, &c);
    assert!(a.scale_offset.is_some(), "{}", a.status);
    let nodes = frame(&mut a, &c, vec![]);
    let text = labels(&nodes);
    // UIC-81: the title is the command's name.
    assert!(text.iter().any(|s| s == "Scale Offset"), "{text:?}");
    assert!(text.iter().any(|s| s == "Inset"), "{text:?}");
    assert!(find(&nodes, Role::Button, "Cancel").is_some());
    assert_eq!(inset_field(&nodes).value(), Some("0 m"));
    type_inset(&mut a, &c, "0.25 m");
    assert_eq!(a.editor.scene, before, "typing must not edit geometry");
    button(&mut a, &c, "OK");
    assert!(a.scale_offset.is_none(), "{}", a.status);
    assert_eq!(a.status, "Scale Offset applied");
    let expected = literal(&before, &square_inset(0.25, 1., 1.));
    assert_scene(&a.editor.scene, &expected, 0.);
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    assert_eq!(a.editor.identities.current, ids);
    a.execute("undo");
    assert_eq!(a.editor.scene, before);
    a.execute("redo");
    assert_scene(&a.editor.scene, &expected, 0.);
    a.execute("undo");

    // The exact command opens the same dialog; the first Return commits
    // the typed field, the next confirms once (UIC-85).
    a.execute(ACTION);
    assert!(a.scale_offset.is_some(), "{}", a.status);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "-0.25 m");
    frame(&mut a, &c, vec![key(Key::Enter)]);
    assert!(a.scale_offset.is_some(), "{}", a.status);
    frame(&mut a, &c, vec![key(Key::Enter)]);
    assert!(a.scale_offset.is_none(), "{}", a.status);
    assert_scene(
        &a.editor.scene,
        &literal(&before, &square_inset(-0.25, 1., 1.)),
        0.,
    );
    assert_eq!(a.editor.history.undo.len(), undo + 1);
    frame(&mut a, &c, vec![key(Key::Enter)]);
    assert_eq!(
        a.editor.history.undo.len(),
        undo + 1,
        "Return after closing must not repeat"
    );
}

#[test]
fn triangle_fixture_and_lowercase_command_spelling() {
    let mut a = app_with(vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(4., 0., 0.),
        DVec3::new(0., 3., 0.),
    ]);
    let before = a.editor.scene.clone();
    let c = context();
    a.execute("@ppi_polyscale_offset.py");
    assert!(a.scale_offset.is_some(), "{}", a.status);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "250 mm");
    button(&mut a, &c, "OK");
    let expected = literal(
        &before,
        &[[0.25, 0.25, 0.], [3.25, 0.25, 0.], [0.25, 2.5, 0.]],
    );
    assert_scene(&a.editor.scene, &expected, 1e-12);
}

#[test]
fn rich_data_unrelated_item_and_native_save_reopen() {
    let mut a = rich_app();
    let before = a.editor.scene.clone();
    let ids = a.editor.identities.current.clone();
    let c = context();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "0.5");
    button(&mut a, &c, "OK");
    let expected = literal(&before, &square_inset(0.5, 1., 1.));
    assert_scene(&a.editor.scene, &expected, 0.);
    assert_eq!(a.editor.identities.current, ids);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-scale-offset-ui-20261029/scenes");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("scale-offset-{}.opscene", std::process::id()));
    assert!(!path.exists());
    assert!(a.save(path.clone()), "{}", a.status);
    let mut reopened = Operandi::empty();
    reopened.open(path.clone());
    assert_eq!(reopened.status, "Scene opened");
    assert_eq!(reopened.editor.scene, a.editor.scene);
    std::fs::remove_file(path).unwrap();
    a.execute("undo");
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.editor.identities.current, ids);
}

#[test]
fn scaled_reflected_and_sheared_items_use_scene_distance() {
    // SO-AFF-F01 nonuniform 2x3: local x moves d/2, y moves d/3.
    let mut a = app_with(square());
    a.editor.scene.active_mut().unwrap().transform.scale = DVec3::new(-2., 3., 1.);
    a.editor.synchronize_identities().unwrap();
    let before = a.editor.scene.clone();
    let c = context();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "0.25 m");
    button(&mut a, &c, "OK");
    assert_scene(
        &a.editor.scene,
        &literal(&before, &square_inset(0.25, 2., 3.)),
        1e-12,
    );

    // Parent scale (-2, 1, 1) over a child rotated 45 degrees: a rhombus of
    // side sqrt10 and area 8, so the local inset is t = d * sqrt10 / 4.
    let mut a = app_with(square());
    let child = a.editor.scene.selection.item.unwrap();
    a.editor.scene.active_mut().unwrap().transform.rotation =
        DVec3::new(0., 0., std::f64::consts::FRAC_PI_4);
    let selection = a.editor.scene.selection.clone();
    a.run(Command::NewMesh);
    let parent = a.editor.scene.active_mut().unwrap();
    parent.transform.scale = DVec3::new(-2., 1., 1.);
    let parent_id = parent.id;
    a.editor.scene.item_mut(child).unwrap().parent = Some(parent_id);
    a.editor.scene.selection = selection;
    a.editor.scene.validate().unwrap();
    a.editor.synchronize_identities().unwrap();
    let before = a.editor.scene.clone();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "-0.5 m");
    button(&mut a, &c, "OK");
    let t = -0.5 * 10f64.sqrt() / 4.;
    assert_scene(
        &a.editor.scene,
        &literal(&before, &square_inset(t, 1., 1.)),
        1e-12,
    );
}

#[test]
fn zero_cancel_escape_and_dismissal_keep_redo_and_scene() {
    let mut a = app_with(square());
    let c = context();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "0.25 m");
    button(&mut a, &c, "OK");
    a.execute("undo");
    assert_eq!(a.editor.history.redo.len(), 1);
    let before = a.editor.clone();

    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    button(&mut a, &c, "OK");
    assert!(a.scale_offset.is_none());
    assert_eq!(a.status, "Scale Offset: nothing changed");
    assert_editor_unchanged(&a, &before);

    let before = a.editor.clone();
    for close in ["Cancel", "Escape", "outside"] {
        a.execute(ACTION);
        frame(&mut a, &c, vec![]);
        type_inset(&mut a, &c, "0.75 m");
        match close {
            "Cancel" => button(&mut a, &c, "Cancel"),
            "Escape" => {
                frame(&mut a, &c, vec![key(Key::Escape)]);
            }
            _ => click(&mut a, &c, pos2(4., SIZE.y - 4.)),
        }
        assert!(a.scale_offset.is_none(), "{close}: {}", a.status);
        assert!(a.editor == before, "{close} changed the editor");
    }
    a.execute("redo");
    assert_scene(
        &a.editor.scene,
        &literal(&before.scene, &square_inset(0.25, 1., 1.)),
        0.,
    );
}

#[test]
fn parked_extrude_redo_survives_zero_cancel_and_refusal() {
    let mut a = app_with(square());
    a.editor.scene.active_mut().unwrap().mesh.uv_maps.insert(
        "Texture".into(),
        vec![vec![
            Some(DVec2::new(0., 0.)),
            Some(DVec2::new(1., 0.)),
            Some(DVec2::new(1., 1.)),
            Some(DVec2::new(0., 1.)),
        ]],
    );
    a.editor.synchronize_identities().unwrap();
    a.editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    a.editor
        .dispatch(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            extrude_uv_map: Some("Texture".into()),
            extrude_uv_selected: true,
            ..Default::default()
        }))
        .unwrap();
    a.editor.dispatch(Command::BeginExtrude).unwrap();
    let c = context();
    let active = a.editor.clone();
    a.execute(ACTION);
    assert!(a.scale_offset.is_none(), "active Extrude stage owns input");
    assert!(a.editor == active);
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.extent = DVec3::Z * 0.5;
    a.editor.dispatch(Command::ConfigureExtrude(p)).unwrap();
    a.editor.dispatch(Command::Undo).unwrap();
    a.editor.dispatch(Command::Undo).unwrap();
    assert!(a.editor.i2_can_redo() && a.editor.session.live.is_none());
    let parked = a.editor.clone();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    button(&mut a, &c, "OK");
    assert_editor_unchanged(&a, &parked);
    assert!(a.editor.i2_can_redo());
    let parked = a.editor.clone();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "1.5 m");
    button(&mut a, &c, "OK");
    assert!(a.scale_offset.is_some(), "core refusal keeps the dialog");
    assert!(a.editor == parked);
    button(&mut a, &c, "Cancel");
    assert!(a.editor == parked && a.editor.i2_can_redo());
}

#[test]
fn invalid_text_and_core_refusal_keep_an_editable_dialog() {
    let mut a = app_with(square());
    let c = context();
    let before = a.editor.clone();
    a.execute(ACTION);
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "abc");
    button(&mut a, &c, "OK");
    assert!(a.scale_offset.is_some());
    assert!(a.scale_offset.as_ref().unwrap().has_error());
    assert!(a.editor == before);
    type_inset(&mut a, &c, "1 m");
    button(&mut a, &c, "OK");
    let error = a
        .scale_offset
        .as_ref()
        .map(|d| d.error_text())
        .unwrap_or_default();
    assert!(error.contains("collapses or inverts"), "{error}");
    assert!(a.editor == before);
    type_inset(&mut a, &c, "0.5 m");
    let probe = frame(&mut a, &c, vec![]);
    assert_eq!(inset_field(&probe).value(), Some("0.5 m"));
    button(&mut a, &c, "OK");
    assert!(a.scale_offset.is_none(), "{}", a.status);
    assert_scene(
        &a.editor.scene,
        &literal(&before.scene, &square_inset(0.5, 1., 1.)),
        0.,
    );

    // SO-U01 (peer amendment 1): statically ineligible geometry is refused
    // before the dialog opens; see
    // `static_ineligible_geometry_is_refused_by_every_opening_route`.
}

#[test]
fn arguments_selection_and_owners_refuse_without_mutation() {
    let mut a = app_with(square());
    let before = a.editor.clone();
    for cmd in [
        "@ppi_PolyScale_Offset.py 0.25",
        "@ppi_PolyScale_Offset.py inset:0.25",
        "@ppi_PolyScale_Offset.py {0.25",
    ] {
        a.execute(cmd);
        assert!(a.scale_offset.is_none(), "{cmd}");
        assert!(a.editor == before, "{cmd}");
        assert!(!a.status.is_empty() && a.status != "Ready", "{cmd}");
    }
    a.execute("@ppi_PolyScale_Offset.py 0.25");
    assert!(a.status.contains("takes no arguments"), "{}", a.status);
    // No invented alias.
    a.execute("poly.scaleOffset");
    assert!(a.scale_offset.is_none());

    let cases: [(&str, Setup); 5] = [
        ("Select one polygon", |a| {
            a.editor.scene.selection.faces.clear()
        }),
        ("Polygon selection mode", |a| {
            a.editor.scene.selection.mode = Mode::Vertex
        }),
        ("visible mesh", |a| a.editor.scene.selection.item = None),
        ("active tool", |a| {
            a.editor.dispatch(Command::Activate(Tool::Bevel)).unwrap()
        }),
        ("Pen", |a| {
            a.editor.dispatch(Command::NewMesh).unwrap();
            a.editor.dispatch(Command::PenStart).unwrap();
        }),
    ];
    let c = context();
    for (reason, setup) in cases {
        let mut a = app_with(square());
        setup(&mut a);
        let before = a.editor.clone();
        if reason == "Select one polygon" {
            click_row(&mut a, &c);
        } else {
            a.execute(ACTION);
        }
        assert!(a.scale_offset.is_none(), "{reason}");
        assert!(a.status.contains(reason), "{reason}: {}", a.status);
        assert!(a.editor == before, "{reason}");
    }
    let mut a = app_with(square());
    a.execute("vert.set");
    assert!(a.set_position.is_some());
    a.execute(ACTION);
    assert!(a.scale_offset.is_none());
}

#[test]
fn open_dialog_owns_input_and_stale_targets_are_refused() {
    let mut a = app_with(square());
    let c = context();
    a.execute(ACTION);
    let before = a.editor.clone();
    a.execute("poly.flip");
    assert!(a.status.contains("Close Scale Offset"), "{}", a.status);
    assert!(a.editor == before);
    a.execute("vert.set");
    assert!(a.set_position.is_none());

    // Changed selection after opening.
    a.editor.scene.selection.faces.clear();
    let changed = a.editor.clone();
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "0.25 m");
    button(&mut a, &c, "OK");
    let error = a
        .scale_offset
        .as_ref()
        .map(|d| d.error_text())
        .unwrap_or_default();
    assert!(error.contains("target changed"), "{error}");
    assert!(a.editor == changed);
    button(&mut a, &c, "Cancel");

    // Replacement document with an identical scene.
    let mut a = app_with(square());
    a.execute(ACTION);
    let source = a.editor.scene.clone();
    a.editor = Editor {
        scene: source.clone(),
        ..Default::default()
    };
    frame(&mut a, &c, vec![]);
    type_inset(&mut a, &c, "0.25 m");
    button(&mut a, &c, "OK");
    assert!(a.scale_offset.is_some());
    assert_eq!(a.editor.scene, source);
    assert!(a.editor.history.undo.is_empty());
}

#[test]
fn row_follows_unify_with_shared_glyph_and_tooltip() {
    let mut a = app_with(square());
    let c = context();
    let nodes = frame(&mut a, &c, vec![]);
    let unify = button_by_line(&a, "Unify");
    let row = button_by_line(&a, "Scale Offset");
    // Scale Offset (Detach)... sits between Scale Offset and Flatten Ngons.
    let detach = button_by_line(&a, "@ppi_PolyScale_Offset_Detach.py");
    assert!(row.bottom() <= detach.top() && detach.top() - row.bottom() < 8.);
    let flatten = button_by_line(&a, "Flatten Ngons");
    let split = button_by_line(&a, "Split");
    assert!(
        unify.bottom() <= row.top()
            && row.bottom() <= flatten.top()
            && flatten.bottom() <= split.top()
    );
    assert!(row.top() - unify.bottom() < 8. && flatten.top() - detach.bottom() < 8.);
    assert_eq!(
        rect(&find(&nodes, Role::Button, "Scale Offset...").expect("row")),
        row
    );
    let mut text = Vec::new();
    for _ in 0..3 {
        text = labels(&frame(&mut a, &c, vec![Event::PointerMoved(row.center())]));
    }
    assert!(
        text.iter()
            .any(|s| s.starts_with("Scale Offset...") && s.contains(ACTION)),
        "tooltip: {text:?}"
    );
    // The row's artwork is its own generated scale-offset icon (batch 9),
    // checked with every other icon by icons::app_icons_match_the_generator.
}

#[test]
fn unassigned_default_help_remap_persistence_and_text_focus() {
    let mut a = app_with(square());
    assert_eq!(input::default_bindings()["Scale Offset"], "");
    assert_eq!(a.bindings["Scale Offset"], "");
    a.execute("help");
    assert!(a.status.contains(ACTION), "{}", a.status);
    a.execute(&format!("help {ACTION}"));
    assert!(
        a.status.contains("Enter Inset Amount") && a.status.contains("no shortcut assigned"),
        "{}",
        a.status
    );

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-scale-offset-ui-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    // An older stored map without this action keeps its own remaps and
    // leaves Scale Offset unassigned; no key is stolen.
    let legacy = dir.join(format!("legacy-{}.json", std::process::id()));
    assert!(!legacy.exists());
    std::fs::write(&legacy, br#"{"Unify":"C"}"#).unwrap();
    let loaded = input::load_bindings(&legacy);
    assert_eq!(loaded["Unify"], "C");
    assert_eq!(loaded["Scale Offset"], "");
    std::fs::remove_file(&legacy).unwrap();
    // A task remap persists and changes only this action.
    let path = dir.join(format!("remap-{}.json", std::process::id()));
    assert!(!path.exists());
    std::fs::write(&path, br#"{"Scale Offset":"C"}"#).unwrap();
    let loaded = input::load_bindings(&path);
    std::fs::remove_file(&path).unwrap();
    let mut expected = input::default_bindings();
    expected.insert("Scale Offset".into(), "C".into());
    assert_eq!(loaded, expected);
    a.bindings = loaded;
    a.execute(&format!("help {ACTION}"));
    assert!(a.status.contains("shortcut C"), "{}", a.status);
    let c = context();
    frame(&mut a, &c, vec![]);
    let row = button_by_line(&a, "Scale Offset");
    let mut text = Vec::new();
    for _ in 0..3 {
        text = labels(&frame(&mut a, &c, vec![Event::PointerMoved(row.center())]));
    }
    assert!(
        text.iter().any(|s| s.contains(ACTION) && s.contains("[C]")),
        "{text:?}"
    );

    // Suppressed while the command entry owns text input.
    c.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &c, vec![key(Key::C), Event::Text("c".into())]);
    assert!(a.scale_offset.is_none());
    c.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    let release = Event::Key {
        key: Key::C,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(&mut a, &c, vec![release.clone()]);
    let p = a.viewport_rect.center();
    frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
    frame(&mut a, &c, vec![release.clone()]);
    assert!(a.scale_offset.is_some(), "{}", a.status);
    // While the dialog owns input the key is ordinary text, not a re-open.
    let before = a.editor.clone();
    frame(&mut a, &c, vec![key(Key::C), release]);
    assert!(a.editor == before);
    button(&mut a, &c, "Cancel");
    assert!(a.scale_offset.is_none());
}

/// Opening routes that must share one offer check.
#[derive(Clone, Copy, Debug)]
enum Route {
    Row,
    Command,
    RemappedKey,
}

fn open_by(a: &mut Operandi, c: &Context, route: Route) {
    match route {
        Route::Row => click_row(a, c),
        Route::Command => a.execute(ACTION),
        Route::RemappedKey => {
            frame(a, c, vec![]);
            let p = a.viewport_rect.center();
            frame(a, c, vec![Event::PointerMoved(p), key(Key::C)]);
            frame(
                a,
                c,
                vec![Event::Key {
                    key: Key::C,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
    }
}

/// `app_with`, plus a second face when `extra` is given, and an ordinary
/// redo entry so nonmutation also covers parked redo.
fn ineligible_app(points: Vec<DVec3>, extra: Option<(Vec<DVec3>, Vec<u32>)>) -> Operandi {
    let mut a = app_with(points);
    if let Some((more, corners)) = extra {
        let m = &mut *a.editor.scene.active_mut().unwrap().mesh;
        m.vertices.extend(more);
        m.faces.push(face(corners));
        a.editor.scene.validate().unwrap();
        a.editor.synchronize_identities().unwrap();
    }
    let selection = a.editor.scene.selection.clone();
    a.run(Command::NewMesh);
    a.run(Command::Undo);
    assert_eq!(a.editor.history.redo.len(), 1, "{}", a.status);
    assert_eq!(a.editor.scene.selection, selection);
    a.bindings.insert("Scale Offset".into(), "C".into());
    a
}

/// Builds one statically ineligible scene.
type MakeApp = fn() -> Operandi;

#[test]
fn static_ineligible_geometry_is_refused_by_every_opening_route() {
    let concave = || {
        ineligible_app(
            vec![
                DVec3::new(0., 0., 0.),
                DVec3::new(4., 0., 0.),
                DVec3::new(4., 4., 0.),
                DVec3::new(2., 1., 0.),
                DVec3::new(0., 4., 0.),
            ],
            None,
        )
    };
    let nonplanar = || {
        let mut p = square();
        p[2].z = 0.5;
        ineligible_app(p, None)
    };
    // Face 1 reuses points 1 and 2 of the selected square.
    // A polygon sharing points with a neighbour is no longer ineligible: it
    // takes the framed-inset path (see scale_offset_frames_a_connected_polygon).
    let cases: [(&str, MakeApp); 2] = [("not strictly convex", concave), ("nonplanar", nonplanar)];
    let c = context();
    for (reason, make) in cases {
        for route in [Route::Row, Route::Command, Route::RemappedKey] {
            let mut a = make();
            frame(&mut a, &c, vec![]);
            let before = a.editor.clone();
            open_by(&mut a, &c, route);
            assert!(a.scale_offset.is_none(), "{reason} via {route:?} opened");
            assert!(
                a.status.contains("Scale Offset cannot apply") && a.status.contains(reason),
                "{reason} via {route:?}: {}",
                a.status
            );
            assert!(
                a.editor == before,
                "{reason} via {route:?} changed the editor"
            );
        }
    }
}

#[test]
fn eligible_targets_still_open_by_every_route_and_distance_refusal_retries() {
    let c = context();
    for route in [Route::Row, Route::Command, Route::RemappedKey] {
        let mut a = app_with(square());
        a.bindings.insert("Scale Offset".into(), "C".into());
        a.editor.scene.active_mut().unwrap().transform.scale = DVec3::new(-2., 3., 1.);
        a.editor.synchronize_identities().unwrap();
        let before = a.editor.clone();
        open_by(&mut a, &c, route);
        assert!(a.scale_offset.is_some(), "{route:?}: {}", a.status);
        assert!(a.editor == before, "opening must not edit");
        // Distance-dependent refusal stays in the editable dialog.
        type_inset(&mut a, &c, "2 m");
        button(&mut a, &c, "OK");
        let error = a
            .scale_offset
            .as_ref()
            .map(|d| d.error_text())
            .unwrap_or_default();
        assert!(error.contains("collapses or inverts"), "{route:?}: {error}");
        assert!(a.editor == before);
        type_inset(&mut a, &c, "0.25 m");
        button(&mut a, &c, "OK");
        assert!(a.scale_offset.is_none(), "{route:?}: {}", a.status);
        assert_scene(
            &a.editor.scene,
            &literal(&before.scene, &square_inset(0.25, 2., 3.)),
            1e-12,
        );
    }
}

#[test]
fn scale_offset_shrinks_connected_polygons_in_place() {
    // Faces of a cube shrink by moving their points: nothing is added and
    // the neighbours follow (spec polyscale-offset B1–B2).
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    a.editor.scene.active_mut().unwrap().mesh =
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
            .unwrap()
            .into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![1, 0];
    a.run(Command::ScaleOffsetPolygon(0.25));
    let mesh = &a.editor.scene.active().unwrap().mesh;
    mesh.validate().unwrap();
    assert_eq!(
        (mesh.vertices.len(), mesh.faces.len()),
        (8, 6),
        "{}",
        a.status
    );
    assert_ne!(
        **mesh,
        operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1).unwrap()
    );
    assert_eq!(a.editor.history.undo.last().unwrap().label, "Scale Offset");
    a.run(Command::Undo);
    assert_eq!(a.editor.scene.active().unwrap().mesh.faces.len(), 6);
}
