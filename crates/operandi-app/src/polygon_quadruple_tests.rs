use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use operandi_core::Mesh;
fn fixture() -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    a.editor.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3, 4, 5],
            uv: None,
            material: "Stone".into(),
            smooth: false,
            part: None,
            material_label: Some("Stone".into()),
        }]
        .into(),
        ..Default::default()
    }
    .into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0];
    a.category = toolbox::Category::Polygon;
    a
}
fn expected(scene: &Scene) -> Scene {
    let mut s = scene.clone();
    let m = &mut s.active_mut().unwrap().mesh;
    let f = m.faces[0].clone();
    let mut a = f.clone();
    a.vertices = vec![0, 1, 4, 5];
    let mut b = f;
    b.vertices = vec![1, 2, 3, 4];
    m.faces = vec![a, b].into();
    s.selection.faces = vec![0, 1];
    s
}
fn frame(a: &mut Operandi, c: &Context, events: Vec<Event>) {
    // Tall enough to show the whole Polygon tab without scrolling.
    draw(a, c, vec2(1440., 2000.), events);
}
#[test]
fn panel_and_command_match_expected_scene() {
    for panel in [true, false] {
        let mut a = fixture();
        let before = a.editor.scene.clone();
        let expected = expected(&before);
        let c = Context::default();
        theme::install(&c);
        frame(&mut a, &c, vec![]);
        if panel {
            let p = button_by_line(&a, "Quadruple").center();
            frame(
                &mut a,
                &c,
                vec![Event::PointerMoved(p), pointer(p, true, Modifiers::NONE)],
            );
            frame(&mut a, &c, vec![pointer(p, false, Modifiers::NONE)]);
        } else {
            a.execute("poly.quadruple");
        }
        assert_eq!(a.editor.scene, expected, "{}", a.status);
        a.execute("undo");
        assert_eq!(a.editor.scene, before);
        a.execute("redo");
        assert_eq!(a.editor.scene, expected);
    }
}
#[test]
fn remap_focus_modal_and_bad_command() {
    let mut a = fixture();
    let c = Context::default();
    theme::install(&c);
    frame(&mut a, &c, vec![]);
    assert_eq!(a.bindings["Quadruple"], "");
    let before = a.editor.scene.clone();
    a.execute("poly.quadruple extra");
    assert_eq!(a.editor.scene, before);
    a.execute("help poly.quadruple");
    assert!(a.status.contains("two opposing rows"));
    a.bindings.insert("Quadruple".into(), "C".into());
    c.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &c, vec![key(Key::C), Event::Text("c".into())]);
    assert_eq!(a.editor.scene, before);
    c.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    frame(
        &mut a,
        &c,
        vec![Event::Key {
            key: Key::C,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    a.show_bindings = true;
    a.execute("poly.quadruple");
    assert_eq!(a.editor.scene, before);
    a.show_bindings = false;
    let p = a.viewport_rect.center();
    frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
    assert_eq!(a.editor.scene, expected(&before));
}

#[test]
fn persisted_quadruple_remap_is_displayed_and_dispatches() {
    use egui::accesskit::Role;
    use std::io::Write;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-quadruple-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("quadruple-{}.json", std::process::id()));
    let mut map = input::default_bindings();
    assert_eq!(map["Quadruple"], "");
    map.insert("Quadruple".into(), "C".into());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(&serde_json::to_vec(&map).unwrap()).unwrap();
    drop(file);
    let reloaded = input::load_bindings(&path);
    assert_eq!(reloaded, map);
    let mut a = fixture();
    a.bindings = reloaded;
    let before = a.editor.scene.clone();
    let ctx = Context::default();
    theme::install(&ctx);
    ctx.enable_accesskit();
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.;
        s.interaction.show_tooltips_only_when_still = false;
    });
    draw(&mut a, &ctx, vec2(1440., 2000.), vec![]);
    let row = button_by_line(&a, "Quadruple");
    let mut labels = Vec::new();
    for _ in 0..3 {
        let out = draw(
            &mut a,
            &ctx,
            vec2(1440., 2000.),
            vec![Event::PointerMoved(row.center())],
        );
        labels = out
            .platform_output
            .accesskit_update
            .unwrap()
            .nodes
            .into_iter()
            .map(|(_, n)| n)
            .collect::<Vec<_>>();
    }
    assert!(labels.iter().any(|n| {
        n.role() == Role::Button
            && n.label()
                .is_some_and(|s| s.contains("Quadruple") && s.contains("[C]"))
    }));
    assert!(labels.iter().any(|n| {
        n.label()
            .or(n.value())
            .is_some_and(|s| s.contains("poly.quadruple") && s.contains("[C]"))
    }));
    assert_eq!(a.editor.scene, before);
    ctx.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &ctx, vec![key(Key::C), Event::Text("c".into())]);
    assert_eq!(a.editor.scene, before);
    ctx.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    frame(
        &mut a,
        &ctx,
        vec![Event::Key {
            key: Key::C,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let p = a.viewport_rect.center();
    frame(&mut a, &ctx, vec![Event::PointerMoved(p), key(Key::C)]);
    assert_eq!(a.editor.scene, expected(&before));
    std::fs::remove_file(path).unwrap();
}
