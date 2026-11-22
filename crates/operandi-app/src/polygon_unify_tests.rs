//! Synthetic options, acknowledgment and command routes. Native checks remain separate.
use super::tests::{draw, pointer};
use super::toolbox_form_tests::button_by_line;
use super::*;
use egui::accesskit::{self, Role};
use operandi_core::Mesh;
fn fixture() -> Operandi {
    let mut a = Operandi::empty();
    a.editor.scene.new_mesh().unwrap();
    let face = |part: &str| Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "same".into(),
        material_label: Some("same".into()),
        smooth: false,
        part: Some(part.into()),
    };
    let mut mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y],
        faces: vec![face("A"), face("B")].into(),
        ..Default::default()
    };
    for (x, y) in [(0u32, 1u32), (1, 2), (2, 3), (3, 0)] {
        mesh.attributes.extrude_edge_uses.insert(
            format!("{}:{}", x.min(y), x.max(y)),
            vec![[0, x, y], [1, x, y]],
        );
    }
    a.editor.scene.active_mut().unwrap().mesh = mesh.into();
    a.editor.scene.selection.mode = Mode::Polygon;
    a.editor.scene.selection.faces = vec![1, 0];
    a.category = toolbox::Category::Polygon;
    a
}
fn context() -> Context {
    let c = Context::default();
    theme::install(&c);
    c.enable_accesskit();
    c
}
fn frame(a: &mut Operandi, c: &Context, events: Vec<Event>) -> Vec<accesskit::Node> {
    draw(a, c, vec2(1440., 2000.), events)
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .collect()
}
fn button(a: &mut Operandi, c: &Context, label: &str) {
    frame(a, c, vec![]);
    frame(a, c, vec![]);
    let nodes = frame(a, c, vec![]);
    let n = nodes
        .iter()
        .find(|n| n.role() == Role::Button && n.label().is_some_and(|s| s.trim() == label))
        .unwrap_or_else(|| panic!("missing {label}"));
    let r = n.bounds().unwrap();
    let p = pos2(((r.x0 + r.x1) * 0.5) as f32, ((r.y0 + r.y1) * 0.5) as f32);
    frame(a, c, vec![Event::PointerMoved(p)]);
    frame(a, c, vec![pointer(p, true, Modifiers::NONE)]);
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
fn checkbox(a: &mut Operandi, c: &Context, label: &str) {
    frame(a, c, vec![]);
    frame(a, c, vec![]);
    let nodes = frame(a, c, vec![]);
    let n = nodes
        .iter()
        .find(|n| n.role() == Role::CheckBox && n.label() == Some(label))
        .unwrap_or_else(|| panic!("missing checkbox {label}"));
    let r = n.bounds().unwrap();
    let p = pos2(((r.x0 + r.x1) * 0.5) as f32, ((r.y0 + r.y1) * 0.5) as f32);
    frame(
        a,
        c,
        vec![Event::PointerMoved(p), pointer(p, true, Modifiers::NONE)],
    );
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
#[test]
fn options_cancel_and_zero_acknowledgment_history() {
    let mut a = fixture();
    let c = context();
    let before = a.editor.scene.clone();
    button(&mut a, &c, "Unify...");
    assert!(a.unify.is_some());
    button(&mut a, &c, "Cancel");
    assert!(a.unify.is_none());
    assert_eq!(a.editor.scene, before);
    assert!(a.editor.undo.is_empty());
    a.execute("poly.unify false");
    let mut expected = before.clone();
    expected.selection.faces.clear();
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.unify_count, Some(0));
    assert_eq!(a.editor.undo.len(), 1);
    button(&mut a, &c, "OK");
    assert!(a.unify_count.is_none());
    assert_eq!(a.editor.undo.len(), 1);
    a.execute("undo");
    assert_eq!(a.editor.scene, before);
    a.execute("redo");
    assert_eq!(a.editor.scene, expected);
}
#[test]
fn force_command_and_modal_ownership() {
    let mut a = fixture();
    let before = a.editor.scene.clone();
    let mut expected = before.clone();
    let m = &mut expected.active_mut().unwrap().mesh;
    m.faces.remove(1);
    m.attributes.extrude_edge_uses.clear();
    expected.selection.faces = vec![0];
    a.execute("poly.unify true");
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.unify_count, Some(1));
    let frozen = a.editor.clone();
    a.execute("undo");
    assert!(a.editor == frozen);
    let c = context();
    button(&mut a, &c, "OK");
    assert!(a.unify_count.is_none(), "ack must close: {}", a.status);
    a.execute("undo");
    assert_eq!(a.editor.scene, before, "undo status: {}", a.status);
    let mut a = fixture();
    a.execute("poly.unify nonsense");
    assert!(a.unify.is_none());
    assert!(a.unify_count.is_none());
    assert!(a.editor.undo.is_empty());
}
#[test]
fn remapped_key_and_text_focus() {
    let mut a = fixture();
    let c = context();
    a.bindings.insert("Unify".into(), "C".into());
    frame(&mut a, &c, vec![]);
    let p = a.viewport_rect.center();
    c.memory_mut(|m| m.request_focus(Id::new("command-entry")));
    frame(&mut a, &c, vec![]);
    let key = |pressed| Event::Key {
        key: Key::C,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(&mut a, &c, vec![key(true)]);
    frame(&mut a, &c, vec![key(false)]);
    assert!(a.unify.is_none());
    c.memory_mut(|m| m.surrender_focus(Id::new("command-entry")));
    frame(&mut a, &c, vec![Event::PointerMoved(p)]);
    frame(&mut a, &c, vec![key(true)]);
    frame(&mut a, &c, vec![key(false)]);
    assert!(a.unify.is_some(), "{}", a.status);
}
#[test]
fn options_cancel_keeps_parked_extrude_redo() {
    let mut a = Operandi::empty();
    a.editor.scene =
        serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json")).unwrap();
    a.editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    a.editor
        .dispatch(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }))
        .unwrap();
    a.editor.dispatch(Command::BeginExtrude).unwrap();
    a.editor.dispatch(Command::Undo).unwrap();
    assert!(a.editor.i2_can_redo());
    let before = a.editor.clone();
    a.execute("poly.unify");
    assert!(a.unify.is_some());
    let c = context();
    button(&mut a, &c, "Cancel");
    assert!(a.editor == before);
    assert!(a.editor.i2_can_redo());
}
#[test]
fn saved_remap_help_and_ack_blocks_second_edit() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-unify-20261029/input-map-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("unify-{}.json", std::process::id()));
    assert!(!path.exists());
    std::fs::write(&path, br#"{"Unify":"C"}"#).unwrap();
    let mut a = fixture();
    a.bindings = input::load_bindings(&path);
    assert_eq!(a.bindings["Unify"], "C");
    assert!(a.shortcut("Unify").contains("C"));
    a.execute("help poly.unify");
    assert!(
        a.status.contains("poly.unify") && a.status.contains("Current shortcut: C"),
        "{}",
        a.status
    );
    let c = context();
    c.style_mut(|s| {
        s.interaction.tooltip_delay = 0.;
        s.interaction.show_tooltips_only_when_still = false;
    });
    let nodes = frame(&mut a, &c, vec![]);
    let row_rect = button_by_line(&a, "Unify");
    let row_node = nodes
        .iter()
        .find(|n| {
            n.role() == Role::Button
                && n.label().is_some_and(|s| {
                    s.starts_with("Unify...") && s.contains("[C]") && !s.contains("poly.unify")
                })
        })
        .expect("remapped Unify button label");
    let bounds = row_node.bounds().expect("Unify button bounds");
    assert_eq!(
        Rect::from_min_max(
            pos2(bounds.x0 as f32, bounds.y0 as f32),
            pos2(bounds.x1 as f32, bounds.y1 as f32)
        ),
        row_rect
    );
    let row = row_rect.center();
    let mut labels = Vec::new();
    for _ in 0..3 {
        labels = frame(&mut a, &c, vec![Event::PointerMoved(row)])
            .iter()
            .filter_map(|n| n.label().or(n.value()).map(str::to_string))
            .collect::<Vec<_>>();
    }
    assert!(
        labels
            .iter()
            .any(|s| s.contains("poly.unify") && s.contains("[C]")),
        "tooltip: {labels:?}"
    );
    frame(&mut a, &c, vec![]);
    let p = a.viewport_rect.center();
    let key = |pressed| Event::Key {
        key: Key::C,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    frame(&mut a, &c, vec![Event::PointerMoved(p), key(true)]);
    frame(&mut a, &c, vec![key(false)]);
    assert!(a.unify.is_some());
    button(&mut a, &c, "OK");
    assert_eq!(a.unify_count, Some(0));
    let before = a.editor.clone();
    frame(&mut a, &c, vec![key(true)]);
    frame(&mut a, &c, vec![key(false)]);
    assert!(a.editor == before);
    button(&mut a, &c, "OK");
    assert!(a.unify_count.is_none());
    frame(&mut a, &c, vec![Event::PointerMoved(p)]);
    frame(&mut a, &c, vec![key(true)]);
    frame(&mut a, &c, vec![key(false)]);
    assert!(
        a.unify.is_some(),
        "remapped key after acknowledgment: {}",
        a.status
    );
    button(&mut a, &c, "Cancel");
    std::fs::remove_file(path).unwrap();
}
#[test]
fn same_winding_valid_native_scene_opens_and_roundtrips_through_app() {
    let source = fixture().editor.scene;
    source.validate().unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../development/m1-unify-20261029/entry-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("unify-entry-{}.opscene", std::process::id()));
    assert!(!path.exists());
    persistence::save(&path, &source).unwrap();
    let mut a = Operandi::empty();
    a.open(path.clone());
    assert_eq!(a.status, "Scene opened");
    assert_eq!(a.editor.scene, source);
    a.editor.synchronize_identities().unwrap();
    let ids = a.editor.identities.current.clone();
    a.execute("poly.unify true");
    assert_eq!(a.unify_count, Some(1));
    let result = a.editor.scene.clone();
    let c = context();
    button(&mut a, &c, "OK");
    assert!(a.unify_count.is_none(), "ack must close: {}", a.status);
    a.execute("undo");
    assert_eq!(a.editor.scene, source, "undo status: {}", a.status);
    assert_eq!(a.editor.identities.current, ids);
    a.execute("redo");
    assert_eq!(a.editor.scene, result);
    assert!(a.save(path.clone()));
    let mut reopened = Operandi::empty();
    reopened.open(path.clone());
    assert_eq!(reopened.status, "Scene opened");
    assert_eq!(reopened.editor.scene, result);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn panel_force_checkbox_apply_and_modified_cancel_with_redo() {
    let c = context();
    let mut cancel = fixture();
    cancel
        .editor
        .dispatch(Command::Rename("Changed".into()))
        .unwrap();
    cancel.editor.dispatch(Command::Undo).unwrap();
    assert!(!cancel.editor.redo.is_empty());
    let before = cancel.editor.clone();
    button(&mut cancel, &c, "Unify...");
    checkbox(&mut cancel, &c, "Force Unify");
    button(&mut cancel, &c, "Cancel");
    assert!(cancel.editor == before);
    assert!(!cancel.editor.redo.is_empty());
    cancel.execute("redo");
    assert_eq!(cancel.editor.scene.active().unwrap().name, "Changed");
    let c = context();
    let mut a = fixture();
    let original = a.editor.scene.clone();
    let mut expected = original.clone();
    let m = &mut expected.active_mut().unwrap().mesh;
    m.faces.remove(1);
    m.attributes.extrude_edge_uses.clear();
    expected.selection.faces = vec![0];
    button(&mut a, &c, "Unify...");
    checkbox(&mut a, &c, "Force Unify");
    button(&mut a, &c, "OK");
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.unify_count, Some(1));
    assert_eq!(a.editor.undo.len(), 1);
    button(&mut a, &c, "OK");
    assert_eq!(a.editor.undo.len(), 1);
    a.execute("undo");
    assert_eq!(a.editor.scene, original);
    a.execute("redo");
    assert_eq!(a.editor.scene, expected);
}
#[test]
fn unassigned_help_is_truthful() {
    let mut a = fixture();
    a.execute("help poly.unify");
    assert!(
        a.status.contains("Current shortcut: unassigned"),
        "{}",
        a.status
    );
}

#[test]
fn unify_options_and_count_block_align_and_history() {
    let mut a = fixture();
    let c = context();
    let initial = a.editor.clone();
    a.execute("poly.unify");
    assert!(a.unify.is_some());
    a.execute("poly.align");
    assert!(a.align_notice.is_none());
    assert!(a.editor == initial);
    button(&mut a, &c, "Cancel");
    a.execute("poly.unify false");
    assert_eq!(a.unify_count, Some(0));
    let after = a.editor.clone();
    a.execute("poly.align");
    assert!(a.align_notice.is_none());
    a.execute("undo");
    assert!(a.editor == after);
    button(&mut a, &c, "OK");
    a.execute("undo");
    assert_eq!(a.editor.scene, initial.scene);
}
