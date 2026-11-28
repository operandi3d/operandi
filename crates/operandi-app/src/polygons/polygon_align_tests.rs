//! MP-C08 Align app routes: panel row, poly.align, remapped key, acknowledgment,
//! focus/modal precedence, history and task-local save/reopen.
use super::tests::{draw, key, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use operandi_core::{Mesh, persistence};

/// ALN-F01 swapped-slot variant (B at slot 0) with a sparse UV map on A.
fn fixture() -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    let face = |vertices: Vec<u32>, material: &str| Face {
        vertices,
        uv: None,
        material: material.into(),
        smooth: false,
        part: None,
        material_label: Some(material.into()),
    };
    let mut mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(2., 0., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(2., 1., 0.),
        ],
        faces: vec![
            face(vec![1, 4, 5, 2], "Blue"),
            face(vec![0, 1, 4, 3], "Red"),
        ]
        .into(),
        ..Default::default()
    };
    mesh.uv_maps.insert(
        "Texture".into(),
        vec![
            vec![None; 4],
            vec![
                Some(glam::DVec2::new(0., 0.)),
                None,
                Some(glam::DVec2::new(1., 1.)),
                Some(glam::DVec2::new(0., 1.)),
            ],
        ],
    );
    mesh.attributes
        .extrude_edge_uses
        .insert("1:4".into(), vec![[0, 1, 4], [1, 1, 4]]);
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![0, 1];
    a.editor.scene.validate().unwrap();
    a.category = toolbox::Category::Polygon;
    a
}
fn expected(scene: &Scene) -> Scene {
    let mut s = scene.clone();
    let m = &mut s.active_mut().unwrap().mesh;
    m.faces[1].vertices = vec![0, 3, 4, 1];
    m.uv_maps.get_mut("Texture").unwrap()[1] = vec![
        Some(glam::DVec2::new(0., 0.)),
        Some(glam::DVec2::new(0., 1.)),
        Some(glam::DVec2::new(1., 1.)),
        None,
    ];
    m.attributes.extrude_edge_uses.clear();
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
fn click(a: &mut Operandi, c: &Context, p: Pos2) {
    frame(a, c, vec![Event::PointerMoved(p)]);
    frame(a, c, vec![pointer(p, true, Modifiers::NONE)]);
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
/// Find the OK button of the visible acknowledgment by its accessible label.
fn acknowledge(a: &mut Operandi, c: &Context) {
    use egui::accesskit::Role;
    c.enable_accesskit();
    let out = draw(a, c, vec2(1440., 2000.), vec![]);
    let nodes: Vec<_> = out
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect();
    assert!(nodes.iter().any(|n| {
        n.label()
            .or(n.value())
            .is_some_and(|s| s == "1 polygon flipped.")
    }));
    let ok = nodes
        .iter()
        .find(|n| n.role() == Role::Button && n.label() == Some("OK"))
        .and_then(|n| n.bounds())
        .unwrap();
    let p = pos2(((ok.x0 + ok.x1) / 2.) as f32, ((ok.y0 + ok.y1) / 2.) as f32);
    click(a, c, p);
    assert!(a.align_notice.is_none());
}

#[test]
fn panel_command_and_remapped_key_match_expected_scene() {
    for route in ["panel", "command", "key"] {
        let mut a = fixture();
        a.bindings.insert("Polygon Align".into(), "C".into());
        let before = a.editor.scene.clone();
        let c = context();
        frame(&mut a, &c, vec![]);
        match route {
            "panel" => {
                let p = button_by_line(&a, "Polygon Align").center();
                click(&mut a, &c, p);
            }
            "command" => a.execute("poly.align"),
            _ => {
                let p = a.viewport_rect.center();
                frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
            }
        }
        assert_eq!(a.editor.scene, expected(&before), "{route}: {}", a.status);
        assert_eq!(a.editor.scene.selection.faces, vec![0, 1]);
        assert_eq!(a.align_notice.as_ref().map(|n| n.flipped), Some(1));
        // The open acknowledgment owns input: another Align is refused.
        a.execute("poly.align");
        assert_eq!(a.editor.scene, expected(&before));
        acknowledge(&mut a, &c);
        a.execute("undo");
        assert_eq!(a.editor.scene, before);
        a.execute("redo");
        assert_eq!(a.editor.scene, expected(&before));
    }
}

#[test]
fn default_unbound_help_remap_focus_and_modal_precedence() {
    let mut a = fixture();
    let c = context();
    frame(&mut a, &c, vec![]);
    assert_eq!(input::default_bindings()["Polygon Align"], "");
    assert_eq!(a.bindings["Polygon Align"], "");
    a.execute("help poly.align");
    assert!(a.status.contains("no shortcut assigned"), "{}", a.status);
    let before = a.editor.scene.clone();
    a.execute("poly.align extra");
    assert_eq!(a.editor.scene, before);
    assert!(a.status.contains("takes no arguments"));
    assert!(a.align_notice.is_none());

    a.bindings.insert("Polygon Align".into(), "C".into());
    a.execute("help poly.align");
    assert!(a.status.contains("shortcut C"), "{}", a.status);
    // A focused text field consumes the typed C.
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
    // Another modal blocks every route.
    a.show_bindings = true;
    a.execute("poly.align");
    assert_eq!(a.editor.scene, before);
    a.show_bindings = false;
    // Focus recovered: the remapped key works in the viewport.
    let p = a.viewport_rect.center();
    frame(&mut a, &c, vec![Event::PointerMoved(p), key(Key::C)]);
    assert_eq!(a.editor.scene, expected(&before));
}

#[test]
fn persisted_remap_is_displayed_on_row_and_tooltip() {
    use egui::accesskit::Role;
    use std::io::Write;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-align-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("align-{}.json", std::process::id()));
    let mut map = input::default_bindings();
    map.insert("Polygon Align".into(), "Alt+J".into());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(&serde_json::to_vec(&map).unwrap()).unwrap();
    drop(file);
    let reloaded = input::load_bindings(&path);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(reloaded, map);
    let mut a = fixture();
    a.bindings = reloaded;
    let ctx = context();
    ctx.enable_accesskit();
    ctx.style_mut(|s| {
        s.interaction.tooltip_delay = 0.;
        s.interaction.show_tooltips_only_when_still = false;
    });
    draw(&mut a, &ctx, vec2(1440., 2000.), vec![]);
    let row = button_by_line(&a, "Polygon Align");
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
                .is_some_and(|s| s.starts_with("Align") && s.contains("[Alt+J]"))
    }));
    assert!(labels.iter().any(|n| {
        n.label()
            .or(n.value())
            .is_some_and(|s| s.contains("Align") && s.contains("[Alt+J]"))
    }));
    // UIC-04: the tooltip names the command in words, not as a line.
    assert!(!labels.iter().any(|n| {
        n.label()
            .or(n.value())
            .is_some_and(|s| s.contains("poly.align"))
    }));
}

#[test]
fn refusal_keeps_scene_history_redo_and_shows_no_acknowledgment() {
    let mut a = fixture();
    let c = context();
    frame(&mut a, &c, vec![]);
    a.execute("poly.align");
    acknowledge(&mut a, &c);
    a.execute("undo");
    assert!(!a.editor.history.redo.is_empty());
    a.editor.scene.selection.faces = vec![];
    let scene = a.editor.scene.clone();
    let (undo, redo) = (a.editor.history.undo.len(), a.editor.history.redo.len());
    let p = button_by_line(&a, "Polygon Align").center();
    click(&mut a, &c, p);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(
        (a.editor.history.undo.len(), a.editor.history.redo.len()),
        (undo, redo)
    );
    assert!(a.align_notice.is_none());
    assert!(a.status.contains("Select polygons"), "{}", a.status);
    // UIC-307: an armed Reduction settles, Align runs, and Reduction arms
    // again.
    a.editor.scene.selection.faces = vec![0, 1];
    a.run(Command::ArmReduction);
    assert!(a.editor.session.reduction.is_some(), "{}", a.status);
    a.execute("poly.align");
    assert!(a.editor.session.reduction.is_some(), "{}", a.status);
    assert!(a.align_notice.is_none());
}

#[test]
fn saved_result_reopens_exactly() {
    let mut a = fixture();
    let c = context();
    frame(&mut a, &c, vec![]);
    let before = a.editor.scene.clone();
    a.execute("poly.align");
    let after = a.editor.scene.clone();
    assert_eq!(after, expected(&before));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-align-20261029/scene-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("align-{}.opscene", std::process::id()));
    persistence::save(&path, &after).unwrap();
    let reopened = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(reopened, after);
}

/// Genuine app entry for a mixed-winding pair: Flip one face of a consistent
/// pair, save and reopen through the app's file routes, then Align. The
/// exceptional-incidence record comes only from Flip and file validation.
#[test]
fn flip_saved_mixed_pair_reopens_and_aligns_with_roundtrip() {
    let uv = |x: f64, y: f64| Some(glam::DVec2::new(x, y));
    let mut a = fixture();
    {
        // Consistent ALN-F01 B_first result as the starting document.
        let m = &mut a.editor.scene.active_mut().unwrap().mesh;
        m.faces[1].vertices = vec![0, 3, 4, 1];
        m.uv_maps.get_mut("Texture").unwrap()[1] = vec![uv(0., 0.), None, uv(1., 1.), uv(0., 1.)];
        m.attributes.extrude_edge_uses.clear();
    }
    a.editor.scene.validate().unwrap();
    a.editor.scene.selection.faces = vec![1];
    let c = context();
    frame(&mut a, &c, vec![]);
    a.execute("poly.flip");
    let mixed = a.editor.scene.clone();
    let m = &mixed.active().unwrap().mesh;
    // Flip keeps the first corner (poly-flip-2026-11-05).
    assert_eq!(m.faces[1].vertices, vec![0, 1, 4, 3], "{}", a.status);
    assert_eq!(
        m.attributes.extrude_edge_uses.get("1:4"),
        Some(&vec![[0, 1, 4], [1, 1, 4]])
    );
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-align-20261029/scene-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("align-mixed-{}.opscene", std::process::id()));
    assert!(a.save(path.clone()), "{}", a.status);

    let mut b = Operandi::empty();
    b.category = toolbox::Category::Polygon;
    b.open(path.clone());
    assert_eq!(b.status, "Scene opened");
    let mut reopened = b.editor.scene.clone();
    reopened.selection = mixed.selection.clone();
    assert_eq!(reopened, mixed);
    b.editor.scene.selection.mode = Mode::Polygon;
    b.editor.scene.selection.faces = vec![0, 1];
    let before = b.editor.scene.clone();
    let mut want = before.clone();
    {
        let m = &mut want.active_mut().unwrap().mesh;
        // Align's anchored reversal undoes the Flip exactly.
        m.faces[1].vertices = vec![0, 3, 4, 1];
        m.uv_maps.get_mut("Texture").unwrap()[1] = vec![uv(0., 0.), None, uv(1., 1.), uv(0., 1.)];
        m.attributes.extrude_edge_uses.clear();
    }
    let c = context();
    frame(&mut b, &c, vec![]);
    b.execute("poly.align");
    assert_eq!(b.editor.scene, want, "{}", b.status);
    assert_eq!(b.align_notice.as_ref().map(|n| n.flipped), Some(1));
    acknowledge(&mut b, &c);
    b.execute("undo");
    assert_eq!(b.editor.scene, before);
    b.execute("redo");
    assert_eq!(b.editor.scene, want);
    assert!(b.save(path.clone()), "{}", b.status);
    let mut d = Operandi::empty();
    d.open(path.clone());
    std::fs::remove_file(&path).unwrap();
    assert_eq!(d.editor.scene.items, want.items);
}

#[test]
fn align_notice_blocks_unify_and_history_until_acknowledged() {
    let mut a = fixture();
    let before = a.editor.scene.clone();
    a.execute("poly.align");
    assert!(a.align_notice.is_some());
    let after = a.editor.clone();
    a.execute("poly.unify");
    assert!(a.unify.is_none());
    assert!(a.editor == after);
    a.execute("undo");
    assert!(a.editor == after);
    let c = context();
    acknowledge(&mut a, &c);
    assert!(a.align_notice.is_none());
    a.execute("undo");
    assert_eq!(a.editor.scene, before);
}
