//! Synthetic SP panel/dialog/command checks; native input is separate.
use super::tests::{draw, key, pointer};
use super::*;
use egui::accesskit::{self, Role};
const SIZE: Vec2 = vec2(1440., 900.);
fn context() -> Context {
    let c = Context::default();
    theme::install(&c);
    c.enable_accesskit();
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
fn find(nodes: &[accesskit::Node], role: Role, name: &str) -> accesskit::Node {
    nodes
        .iter()
        .find(|n| n.role() == role && n.label().or(n.value()) == Some(name))
        .unwrap()
        .clone()
}
fn click(a: &mut Operandi, c: &Context, p: Pos2) {
    frame(a, c, vec![Event::PointerMoved(p)]);
    frame(a, c, vec![pointer(p, true, Modifiers::NONE)]);
    frame(a, c, vec![pointer(p, false, Modifiers::NONE)]);
}
fn choose(a: &mut Operandi, c: &Context, role: Role, name: &str) {
    let nodes = frame(a, c, vec![]);
    let p = rect(&find(&nodes, role, name)).center();
    click(a, c, p);
}
fn type_value(a: &mut Operandi, c: &Context, text: &str) {
    let nodes = frame(a, c, vec![]);
    let ok = rect(&find(&nodes, Role::Button, "OK"));
    let field = nodes
        .iter()
        .filter(|n| n.role() == Role::TextInput)
        .min_by_key(|n| (rect(n).center().distance(ok.center())) as i32)
        .unwrap();
    let old = field.value().unwrap_or_default().chars().count();
    click(a, c, rect(field).center());
    let mut events = vec![key(Key::End)];
    events.extend((0..old).map(|_| key(Key::Backspace)));
    events.push(Event::Text(text.into()));
    frame(a, c, events);
}

fn parked_app() -> Operandi {
    let scene: Scene = serde_json::from_str(include_str!(
        "../../../../docs/i2/expectations/graphs/G0.json"
    ))
    .unwrap();
    let mut a = Operandi::empty();
    a.editor = Editor {
        scene,
        ..Default::default()
    };
    a.category = toolbox::Category::Polygon;
    a.editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    a.editor
        .dispatch(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }))
        .unwrap();
    a.editor.dispatch(Command::BeginExtrude).unwrap();
    let mut p = a.editor.session.live.as_ref().unwrap().params.clone();
    p.extent = DVec3::Z * 0.5;
    a.editor.dispatch(Command::ConfigureExtrude(p)).unwrap();
    a.editor.dispatch(Command::Undo).unwrap();
    a.editor.dispatch(Command::Undo).unwrap();
    assert!(a.editor.i2_can_redo() && a.editor.session.live.is_none());
    a
}
#[test]
fn parked_extrude_cancel_and_modal_commit_or_direct_command() {
    for modal in [false, true] {
        let mut a = parked_app();
        let source = a.editor.scene.clone();
        let parked = a.editor.i2.selected.clone();
        let undo = a.editor.history.undo.clone();
        let redo = a.editor.history.redo.clone();
        let c = context();
        frame(&mut a, &c, vec![]);
        a.execute("vert.set");
        assert!(a.set_position.is_some());
        choose(&mut a, &c, Role::Button, "Cancel");
        assert_eq!(a.editor.scene, source);
        assert_eq!(a.editor.i2.selected, parked);
        assert!(
            a.editor.history.undo == undo
                && a.editor.history.redo == redo
                && a.editor.i2_can_redo()
        );
        a.execute("vert.set z 1m false false");
        assert_eq!(a.editor.scene, source);
        assert_eq!(a.editor.i2.selected, parked);
        assert!(a.editor.i2_can_redo());
        a.execute("vert.set z invalid false false");
        assert_eq!(a.editor.scene, source);
        assert_eq!(a.editor.i2.selected, parked);
        assert!(a.editor.i2_can_redo());
        let mut expected = source.clone();
        for v in [4_usize, 5, 6, 7] {
            expected.active_mut().unwrap().mesh.vertices[v].z = 2.;
        }
        if modal {
            a.execute("vert.set");
            frame(&mut a, &c, vec![]);
            choose(&mut a, &c, Role::RadioButton, "Z");
            type_value(&mut a, &c, "2 m");
            choose(&mut a, &c, Role::Button, "OK");
        } else {
            a.execute("vert.set z 2m false false");
        }
        assert_eq!(a.editor.scene, expected);
        assert!(a.editor.i2.selected.is_none());
        a.execute("undo");
        assert_eq!(a.editor.scene, source);
    }
}

#[test]
fn pen_part_position_composition_and_active_pen_refusal() {
    let mut a = Operandi::empty();
    a.run(Command::NewMesh);
    let unrelated = a.editor.scene.selection.item;
    a.run(Command::NewMesh);
    a.editor.scene.selection.mode = Mode::Polygon;
    a.camera.view = View::Front;
    a.execute("tool.set prim.pen on");
    assert!(a.editor.session.pen_draft.is_some(), "{}", a.status);
    for point in [
        DVec3::new(-1., -1., 0.),
        DVec3::new(1., -1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(-1., 1., 0.),
    ] {
        a.editor.dispatch(Command::PenCapture(point)).unwrap();
    }
    let draft = a.editor.session.pen_draft.clone();
    let scene = a.editor.scene.clone();
    let undo = a.editor.history.undo.clone();
    let redo = a.editor.history.redo.clone();
    a.execute("vert.set z 2m false false");
    assert_eq!(a.editor.session.pen_draft, draft);
    assert_eq!(a.editor.scene, scene);
    assert!(a.editor.history.undo == undo);
    assert!(a.editor.history.redo == redo);
    a.execute("tool.drop");
    a.editor.scene.selection.faces = vec![0];
    a.execute("poly.setPart {Original Part}");
    let before = a.editor.scene.clone();
    assert!(before.items.iter().any(|item| Some(item.id) == unrelated));
    let ids = a.editor.identities.current.clone();
    let mut expected = before.clone();
    for vertex in &mut expected.active_mut().unwrap().mesh.vertices {
        vertex.z = 2.;
    }
    a.execute("vert.set z 2m false false");
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.editor.identities.current, ids);
    a.execute("undo");
    assert_eq!(a.editor.scene, before);
    assert_eq!(a.editor.identities.current, ids);
    a.execute("redo");
    assert_eq!(a.editor.scene, expected);
    assert_eq!(a.editor.identities.current, ids);
}
