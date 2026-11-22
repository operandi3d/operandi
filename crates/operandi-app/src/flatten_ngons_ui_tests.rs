//! Synthetic app routing checks; native interaction and display remain pending.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;

const ACTION: &str = "@ppi_flatten_ngons.py";
fn app() -> Operandi {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let points = [
        [-2., 0., 0.15],
        [-1., -1., -0.1],
        [1., -1., -0.1],
        [2., 0., 0.15],
        [0., 2., -0.1],
    ];
    let mut vertices: Vec<DVec3> = points.iter().map(|p| DVec3::from(*p)).collect();
    vertices.extend(points.iter().map(|p| DVec3::from(*p) + DVec3::X * 10.));
    let face = |vertices| Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    };
    *a.editor.scene.active_mut().unwrap().mesh = operandi_core::Mesh {
        vertices,
        faces: vec![face(vec![0, 1, 2, 3, 4]), face(vec![5, 6, 7, 8, 9])].into(),
        ..Default::default()
    };
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.editor.scene.selection.vertices = vec![2];
    a.editor.synchronize_identities().unwrap();
    a.category = toolbox::Category::Polygon;
    a
}
fn frame(a: &mut Operandi, c: &Context, events: Vec<Event>) {
    let _ = draw(a, c, vec2(1440., 2000.), events);
}
fn click_row(a: &mut Operandi, c: &Context) {
    frame(a, c, vec![]);
    let p = button_by_line(a, "Flatten Ngons").center();
    frame(a, c, vec![Event::PointerMoved(p)]);
    frame(a, c, vec![pointer(p, true, Modifiers::NONE)]);
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
#[test]
fn row_command_and_remap_share_literal_geometry_and_history() {
    for route in 0..3 {
        let mut a = app();
        let c = Context::default();
        theme::install(&c);
        let before = a.editor.scene.clone();
        // The same edit through the core command is the expected result.
        let mut probe = a.editor.clone();
        probe.dispatch(Command::FlattenNgons).unwrap();
        let expected = probe.scene.clone();
        assert!(expected.selection.faces.is_empty() && expected.selection.vertices.is_empty());
        let undo = a.editor.undo.len();
        match route {
            0 => click_row(&mut a, &c),
            1 => a.execute(ACTION),
            _ => {
                a.bindings.insert("Flatten Ngons".into(), "C".into());
                frame(&mut a, &c, vec![]);
                let p = a.viewport_rect.center();
                frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
            }
        }
        assert_eq!(a.editor.scene, expected, "route {route}: {}", a.status);
        assert_eq!(a.editor.undo.len(), undo + 1);
        let after = a.editor.scene.clone();
        a.run(Command::Undo);
        assert_eq!(a.editor.scene, before);
        a.run(Command::Redo);
        assert_eq!(a.editor.scene, after);
    }
}
#[test]
fn help_unassigned_remap_focus_and_malformed() {
    let mut a = app();
    assert_eq!(input::default_bindings()["Flatten Ngons"], "");
    a.execute("help @ppi_flatten_ngons.py");
    assert!(a.status.contains("no shortcut assigned") && a.status.contains("active visible mesh"));
    a.bindings.insert("Flatten Ngons".into(), "C".into());
    a.execute("help @ppi_flatten_ngons.py");
    assert!(a.status.contains("shortcut C"));
    let before = a.editor.clone();
    a.execute("@ppi_flatten_ngons.py 1");
    assert!(a.status.contains("no arguments"));
    assert!(a.editor == before);
    let c = Context::default();
    theme::install(&c);
    frame(&mut a, &c, vec![]);
    c.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &c, vec![key(Key::C), Event::Text("c".into())]);
    assert_eq!(a.editor.scene, before.scene);
}
#[test]
fn refusal_no_change_and_modal_leave_editor_untouched() {
    // Planar n-gons and nothing selected: no edit at all.
    let mut a = app();
    for p in &mut a.editor.scene.active_mut().unwrap().mesh.vertices {
        p.z = 0.;
    }
    a.editor.scene.selection.faces.clear();
    a.editor.scene.selection.vertices.clear();
    let before = a.editor.clone();
    a.execute(ACTION);
    assert!(a.status.contains("already planar"));
    assert!(a.editor == before);
    let mut a = app();
    a.show_bindings = true;
    let before = a.editor.clone();
    a.execute(ACTION);
    assert!(a.status.contains("Close"));
    assert!(a.editor == before);
}

#[test]
fn active_tool_blocks_command_and_remap_without_confirmation() {
    for route in 1..3 {
        let mut a = app();
        a.editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        let before = a.editor.clone();
        let c = Context::default();
        theme::install(&c);
        match route {
            0 => click_row(&mut a, &c),
            1 => a.execute(ACTION),
            _ => {
                a.bindings.insert("Flatten Ngons".into(), "C".into());
                frame(&mut a, &c, vec![]);
                let p = a.viewport_rect.center();
                frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
            }
        }
        assert!(a.editor == before, "route {route}: {}", a.status);
        assert!(a.status.contains("tool"), "route {route}: {}", a.status);
    }
}
