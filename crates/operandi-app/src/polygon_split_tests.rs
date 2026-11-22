//! Synthetic panel, command and keyboard routes; native interaction is separate.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use glam::DVec2;
use operandi_core::Mesh;
fn fixture() -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    let mesh = Mesh {
        vertices: vec![
            DVec3::new(-1., -1., 0.),
            DVec3::new(1., -1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(-1., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
            material: "Warm Clay".into(),
            material_label: Some("Warm Clay".into()),
            part: Some("Panel".into()),
            smooth: true,
        }]
        .into(),
        ..Default::default()
    };
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.editor.scene.selection.mode = Mode::Vertex;
    a.editor.scene.selection.vertices = vec![0, 2];
    a.category = toolbox::Category::Polygon;
    a
}
fn expected(scene: &Scene) -> Scene {
    let mut s = scene.clone();
    let mesh = &mut s.active_mut().unwrap().mesh;
    let face = mesh.faces[0].clone();
    let mut a = face.clone();
    a.vertices = vec![0, 1, 2];
    a.uv = Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE]);
    let mut b = face;
    b.vertices = vec![0, 2, 3];
    b.uv = Some(vec![DVec2::ZERO, DVec2::ONE, DVec2::Y]);
    mesh.faces = vec![a, b].into();
    s
}
fn frame(a: &mut Operandi, c: &Context, events: Vec<Event>) {
    // Tall enough to show the whole Polygon tab without scrolling.
    draw(a, c, vec2(1440., 2000.), events);
}
fn context() -> Context {
    let c = Context::default();
    theme::install(&c);
    c
}
fn ctrl_l() -> Event {
    Event::Key {
        key: Key::L,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::CTRL,
    }
}
#[test]
fn panel_command_and_ctrl_l_match_independent_expected_scene() {
    for route in 0..3 {
        let mut a = fixture();
        let wanted = expected(&a.editor.scene);
        let c = context();
        frame(&mut a, &c, vec![]);
        let p = button_by_line(&a, "Split").center();
        match route {
            0 => {
                frame(
                    &mut a,
                    &c,
                    vec![Event::PointerMoved(p), pointer(p, true, Modifiers::NONE)],
                );
                frame(&mut a, &c, vec![pointer(p, false, Modifiers::NONE)]);
            }
            1 => a.execute("poly.split"),
            _ => {
                let p = a.viewport_rect.center();
                frame(&mut a, &c, vec![Event::PointerMoved(p), ctrl_l()]);
            }
        }
        assert_eq!(a.editor.scene, wanted, "{}", a.status);
        assert_eq!(a.editor.scene.selection.vertices, vec![0, 2]);
        a.execute("undo");
        assert_eq!(a.editor.scene, fixture().editor.scene);
        a.execute("redo");
        assert_eq!(a.editor.scene, wanted);
    }
}
#[test]
fn split_focus_modal_remap_and_invalid_command_are_atomic() {
    let mut a = fixture();
    let c = context();
    frame(&mut a, &c, vec![]);
    let before = a.editor.scene.clone();
    c.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &c, vec![ctrl_l()]);
    assert_eq!(a.editor.scene, before);
    c.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    a.execute("poly.split extra");
    assert_eq!(a.editor.scene, before);
    a.show_bindings = true;
    a.execute("poly.split");
    assert_eq!(a.editor.scene, before);
    a.show_bindings = false;
    a.bindings.insert("Split".into(), "C".into());
    let p = a.viewport_rect.center();
    frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
    assert_eq!(a.editor.scene, expected(&before));
}
#[test]
fn saved_ctrl_l_remap_is_not_displaced() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-polygon-split-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("split-map-{}.json", std::process::id()));
    assert!(!path.exists());
    let mut map = BTreeMap::from([("Triple".to_string(), "Ctrl+L".to_string())]);
    std::fs::write(&path, serde_json::to_vec(&map).unwrap()).unwrap();
    let b = input::load_bindings(&path);
    assert_eq!(b["Triple"], "Ctrl+L");
    assert_eq!(b["Split"], "");
    map.insert("Split".into(), "Ctrl+L".into());
    std::fs::write(&path, serde_json::to_vec(&map).unwrap()).unwrap();
    assert_eq!(input::load_bindings(&path)["Split"], "Ctrl+L");
    map.insert("Split".into(), "".into());
    std::fs::write(&path, serde_json::to_vec(&map).unwrap()).unwrap();
    assert_eq!(input::load_bindings(&path)["Split"], "");
    std::fs::remove_file(path).unwrap();
}

fn plate(points: &[(f64, f64)], faces: &[&[u32]]) -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    a.editor.scene.active_mut().unwrap().mesh = Mesh {
        vertices: points.iter().map(|&(x, y)| DVec3::new(x, y, 0.)).collect(),
        faces: faces
            .iter()
            .map(|f| Face {
                vertices: f.to_vec(),
                uv: None,
                material: String::new(),
                material_label: None,
                part: None,
                smooth: false,
            })
            .collect(),
        ..Default::default()
    }
    .into();
    a.editor.scene.selection.mode = Mode::Vertex;
    a
}

#[test]
fn split_ngon_and_successive_cuts_across_polygons() {
    // Hexagon: cut corners 0 and 3 into two quads.
    let hex = [
        (2., 0.),
        (1., 1.7),
        (-1., 1.7),
        (-2., 0.),
        (-1., -1.7),
        (1., -1.7),
    ];
    let mut a = plate(&hex, &[&[0, 1, 2, 3, 4, 5]]);
    a.editor.scene.selection.vertices = vec![0, 3];
    a.execute("poly.split");
    assert_eq!(a.status, "Ready");
    let mesh = &a.editor.scene.active().unwrap().mesh;
    let faces: Vec<_> = mesh.faces.iter().map(|f| f.vertices.clone()).collect();
    assert_eq!(faces, [vec![0, 1, 2, 3], vec![0, 3, 4, 5]]);

    // Two quads side by side (0-1-4-3, 1-2-5-4): vertices 0, 4, 2 cut both.
    let grid = [(0., 0.), (1., 0.), (2., 0.), (0., 1.), (1., 1.), (2., 1.)];
    let mut b = plate(&grid, &[&[0, 1, 4, 3], &[1, 2, 5, 4]]);
    b.editor.scene.selection.vertices = vec![3, 1, 5];
    b.execute("poly.split");
    assert_eq!(b.status, "Ready");
    let mesh = &b.editor.scene.active().unwrap().mesh;
    mesh.validate().unwrap();
    assert_eq!(mesh.faces.len(), 4);
    assert!(mesh.faces.iter().all(|f| f.vertices.len() == 3));
    assert_eq!(b.editor.undo.len(), 1, "all cuts are one edit");
    let before = b.editor.scene.clone();
    b.menu_action(&context(), app_menu::line_action("app.undo"));
    assert_ne!(b.editor.scene, before);
    assert_eq!(b.editor.scene.active().unwrap().mesh.faces.len(), 2);
}

#[test]
fn split_refuses_a_cut_outside_a_concave_polygon() {
    // Arrow shape: the chord between corners 1 and 3 passes outside at 2.
    let arrow = [(0., 0.), (2., 0.), (1., 0.5), (2., 1.), (0., 1.)];
    let mut a = plate(&arrow, &[&[0, 1, 2, 3, 4]]);
    a.editor.scene.selection.vertices = vec![1, 3];
    let before = a.editor.scene.clone();
    a.execute("poly.split");
    assert!(a.status.ends_with("nothing changed"), "{}", a.status);
    assert_eq!(a.editor.scene, before);
    assert!(a.editor.undo.is_empty());
}
