//! Node editing in the app (NE-T3, NE-T4, NE-T8, UIC-160).
use crate::node_edit_ui::{BoardAction, edit_line};
use crate::tests::box_app;
use operandi_core::Command;
use operandi_core::node_view::{NodeEdit, Target, layer_name, mix_name};
use operandi_core::shader_tree::{Drop, NewLayer, ShaderTreeEdit};

const MATERIAL: u64 = 4;

/// A box whose Base Material sits in a Material Group; the group's id.
fn app_with_group() -> (crate::Operandi, u64) {
    let mut app = box_app();
    let e = &mut app.editor;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
        layer: NewLayer::Group,
        selected: None,
    }))
    .unwrap();
    let group = e.scene.shader_tree.next_id - 1;
    e.dispatch(Command::ShaderTree(ShaderTreeEdit::Move {
        ids: vec![MATERIAL],
        drop: Drop {
            parent: Some(group),
            index: 0,
        },
    }))
    .unwrap();
    app.execute(&format!("operandi.nodeMaterial.showGraph layer:{group}"));
    assert_eq!(app.graph_window.group, Some(group));
    (app, group)
}

fn surface(app: &crate::Operandi, group: u64) -> operandi_core::shader_eval::Principled {
    operandi_core::materialx::tree_standard::group_surface(&app.editor.scene, group).unwrap()
}

fn noise(app: &mut crate::Operandi, group: u64) {
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Add {
            category: "noise3d".into(),
            ty: "color3".into(),
            at: [0., 0.],
        })],
    );
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Connect {
            from: "Noise".into(),
            output: "out".into(),
            to: Target::Layer {
                layer: MATERIAL,
                channel: "diffuse_color".into(),
            },
        })],
    );
    assert!(
        app.editor
            .scene
            .shader_tree
            .channel_driver(MATERIAL, operandi_core::shader_eval::Ch::DiffuseColor)
            .is_some(),
        "{}",
        app.status
    );
}

#[test]
fn ne_t3_mixing_nodes_refuse_deletion_and_edit_their_layer() {
    let (mut app, group) = app_with_group();
    let before = app.editor.scene.clone();
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Delete(vec![mix_name(
            MATERIAL,
        )]))],
    );
    assert_eq!(app.editor.scene, before);
    assert!(app.status.contains("locked"), "{}", app.status);
    // The mixing node's opacity is the layer's (one undo step).
    let steps = app.editor.undo.len();
    let mut node = app.editor.scene.shader_tree.get(MATERIAL).unwrap().clone();
    node.layer.opacity = 0.4;
    app.apply_board_actions(group, vec![BoardAction::Layer(Box::new(node), false)]);
    assert_eq!(
        app.editor
            .scene
            .shader_tree
            .get(MATERIAL)
            .unwrap()
            .layer
            .opacity,
        0.4
    );
    assert_eq!(app.editor.undo.len(), steps + 1);
    let board = app.editor.scene.shader_tree.board(group).unwrap();
    let mix = board.get(&mix_name(MATERIAL)).unwrap();
    assert_eq!(
        mix.input("opacity").unwrap().value,
        operandi_core::node_view_board::Value::Numbers(vec![0.4])
    );
}

#[test]
fn ne_t4_deleting_a_texture_layers_box_is_one_step_both_ways() {
    let (mut app, group) = app_with_group();
    app.editor
        .dispatch(Command::ShaderTree(ShaderTreeEdit::Add {
            layer: NewLayer::Constant,
            selected: Some(MATERIAL),
        }))
        .unwrap();
    let texture = app.editor.scene.shader_tree.next_id - 1;
    assert_eq!(
        app.editor.scene.shader_tree.material_group_of(texture),
        Some(group)
    );
    noise(&mut app, group);
    let before = app.editor.scene.clone();
    assert!(
        app.editor
            .scene
            .shader_tree
            .board(group)
            .unwrap()
            .get(&layer_name(texture))
            .is_some()
    );
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Delete(vec![layer_name(
            texture,
        )]))],
    );
    let tree = &app.editor.scene.shader_tree;
    assert!(
        tree.get(texture).is_none(),
        "the row is gone: {}",
        app.status
    );
    assert!(tree.board(group).unwrap().get(&mix_name(texture)).is_none());
    app.editor.dispatch(Command::Undo).unwrap();
    assert_eq!(
        app.editor.scene, before,
        "row, box and wires back in one step"
    );
}

#[test]
fn ne_t8_hidden_nodes_leave_a_stub_and_never_change_the_render() {
    let (mut app, group) = app_with_group();
    noise(&mut app, group);
    let rendered = surface(&app, group);
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Hide(vec!["Noise".into()]))],
    );
    let board = app.editor.scene.shader_tree.board(group).unwrap();
    assert!(board.get("Noise").unwrap().hidden);
    // The wire is still there: the canvas draws it as a stub on the
    // material's side.
    assert!(
        board
            .links
            .iter()
            .any(|l| l.from == "Noise" && l.to == layer_name(MATERIAL))
    );
    assert_eq!(surface(&app, group), rendered);
    // Double-clicking the stub shows it again.
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Show(vec!["Noise".into()]))],
    );
    assert!(
        !app.editor
            .scene
            .shader_tree
            .board(group)
            .unwrap()
            .get("Noise")
            .unwrap()
            .hidden
    );
    assert_eq!(surface(&app, group), rendered);
}

#[test]
fn every_board_gesture_line_builds_the_same_edit() {
    let (mut app, group) = app_with_group();
    let edits = [
        NodeEdit::Add {
            category: "noise3d".into(),
            ty: "color3".into(),
            at: [10., -20.],
        },
        NodeEdit::Connect {
            from: "Noise".into(),
            output: "out".into(),
            to: Target::Layer {
                layer: MATERIAL,
                channel: "diffuse_color".into(),
            },
        },
        NodeEdit::Disconnect(Target::Free {
            node: "Mix".into(),
            input: "fg".into(),
        }),
        NodeEdit::Delete(vec!["Noise".into(), layer_name(MATERIAL)]),
        NodeEdit::Hide(vec!["Noise".into()]),
        NodeEdit::Show(vec!["Noise".into()]),
        NodeEdit::ShowAll,
        NodeEdit::Move {
            places: vec![("Noise".into(), [3., 4.])],
            continuing: false,
        },
        NodeEdit::SetValue {
            node: "Noise".into(),
            input: "amplitude".into(),
            value: "0.5,1,1".into(),
            continuing: false,
        },
    ];
    for edit in edits {
        let line = edit_line(group, &edit).unwrap();
        let built = operandi_core::command_line::command_for_line(&mut app.editor, &line)
            .unwrap()
            .unwrap()
            .unwrap();
        let Command::ShaderTree(ShaderTreeEdit::Nodes {
            group: g,
            edit: built,
        }) = built
        else {
            panic!("{line}: not a node edit")
        };
        assert_eq!((g, built), (group, edit), "{line}");
    }
}

fn nodes_row(app: &crate::Operandi, group: u64) -> Option<(usize, String)> {
    app.shading_tree_rows()
        .iter()
        .enumerate()
        .find(|(_, r)| r.key == crate::shading_panel::ShaderKey::Nodes(group))
        .map(|(i, r)| (i, r.name.clone()))
}

#[test]
fn ne_t5_ne_11_a_group_with_free_nodes_shows_one_nodes_row() {
    let (mut app, group) = app_with_group();
    assert_eq!(nodes_row(&app, group), None);
    for (cat, ty) in [
        ("noise3d", "color3"),
        ("multiply", "color3"),
        ("constant", "float"),
    ] {
        app.apply_board_actions(
            group,
            vec![BoardAction::Edit(NodeEdit::Add {
                category: cat.into(),
                ty: ty.into(),
                at: [0., 0.],
            })],
        );
    }
    let (at, name) = nodes_row(&app, group).expect("a Nodes row");
    assert_eq!(name, "Nodes (3)");
    // It sits after the group's layers (NE-11), never as a layer (NE-12).
    let rows = app.shading_tree_rows();
    let material = rows
        .iter()
        .position(|r| r.key == crate::shading_panel::ShaderKey::Layer(MATERIAL))
        .unwrap();
    assert!(at > material);
    assert!(rows[at].fixed, "not draggable into the stack");
    // No layer rows for free nodes (NE-12).
    assert!(rows.iter().all(|r| r.name != "Noise"));
    // Gone when the last free node goes.
    app.apply_board_actions(
        group,
        vec![BoardAction::Edit(NodeEdit::Delete(vec![
            "Noise".into(),
            "Multiply".into(),
            "Constant".into(),
        ]))],
    );
    assert_eq!(nodes_row(&app, group), None, "{}", app.status);
}

#[test]
fn ne_t2_a_driven_channel_shows_its_marker_and_opens_the_node_view() {
    use crate::tests::{draw, pointer};
    use eframe::egui::{self, Event, Modifiers, vec2};
    let (mut app, group) = app_with_group();
    noise(&mut app, group);
    app.graph_window.group = None;
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    app.upper_frame.show_page(1);
    let size = vec2(1440., 900.);
    app.shading.selected = vec![MATERIAL];
    let mut out = draw(&mut app, &ctx, size, vec![]);
    for _ in 0..2 {
        out = draw(&mut app, &ctx, size, vec![]);
    }
    let marker = find_text(&out, "driven by nodes (Noise)").expect("the driven marker");
    draw(&mut app, &ctx, size, vec![Event::PointerMoved(marker)]);
    draw(
        &mut app,
        &ctx,
        size,
        vec![
            pointer(marker, true, Modifiers::NONE),
            pointer(marker, false, Modifiers::NONE),
        ],
    );
    assert_eq!(app.graph_window.group, Some(group));
    assert_eq!(app.graph_window.board.selected.as_deref(), Some("Noise"));
}

fn find_text(out: &eframe::egui::FullOutput, want: &str) -> Option<eframe::egui::Pos2> {
    fn walk(shape: &eframe::egui::Shape, want: &str, found: &mut Option<eframe::egui::Pos2>) {
        match shape {
            eframe::egui::Shape::Text(t) if t.galley.text() == want => {
                *found = Some(t.pos + t.galley.rect.center().to_vec2())
            }
            eframe::egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, want, found)),
            _ => {}
        }
    }
    let mut found = None;
    for c in &out.shapes {
        walk(&c.shape, want, &mut found);
    }
    found
}

/// Every text drawn this frame, with where it sits.
fn texts(out: &eframe::egui::FullOutput) -> Vec<(String, eframe::egui::Pos2)> {
    fn walk(shape: &eframe::egui::Shape, out: &mut Vec<(String, eframe::egui::Pos2)>) {
        match shape {
            eframe::egui::Shape::Text(t) => out.push((
                t.galley.text().to_string(),
                t.pos + t.galley.rect.center().to_vec2(),
            )),
            eframe::egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for c in &out.shapes {
        walk(&c.shape, &mut found);
    }
    found
}

#[test]
fn ne_01_dragging_a_value_on_a_layer_box_is_its_properties_edit_in_one_step() {
    use crate::tests::{draw, pointer};
    use eframe::egui::{self, Event, Modifiers, vec2};
    let (mut app, _group) = app_with_group();
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    let size = vec2(1440., 900.);
    let mut out = draw(&mut app, &ctx, size, vec![]);
    for _ in 0..3 {
        out = draw(&mut app, &ctx, size, vec![]);
    }
    let all = texts(&out);
    let (_, label) = all
        .iter()
        .find(|(t, _)| t == "Roughness")
        .expect("the material box's Roughness row")
        .clone();
    // The row's value field sits to its right on the same line.
    let (_, field) = all
        .iter()
        .filter(|(t, p)| {
            (p.y - label.y).abs() < 4. && p.x > label.x + 30. && t.parse::<f64>().is_ok()
        })
        .min_by(|a, b| a.1.x.total_cmp(&b.1.x))
        .expect("an editable value on the box")
        .clone();
    let before = app.editor.undo.len();
    let rough = |a: &crate::Operandi| match &a.editor.scene.shader_tree.get(MATERIAL).unwrap().kind
    {
        operandi_core::shader_tree::NodeKind::Material(m) => m.roughness,
        _ => unreachable!(),
    };
    let start = rough(&app);
    draw(&mut app, &ctx, size, vec![Event::PointerMoved(field)]);
    draw(
        &mut app,
        &ctx,
        size,
        vec![pointer(field, true, Modifiers::NONE)],
    );
    for k in 1..=6 {
        let p = field + vec2(6. * k as f32, 0.);
        draw(&mut app, &ctx, size, vec![Event::PointerMoved(p)]);
    }
    let end = field + vec2(36., 0.);
    draw(
        &mut app,
        &ctx,
        size,
        vec![pointer(end, false, Modifiers::NONE)],
    );
    draw(&mut app, &ctx, size, vec![]);
    assert!(rough(&app) > start, "dragged: {} -> {}", start, rough(&app));
    // The headless test window is embedded over the viewport, which also
    // sees the release as a click (a Selection step); in the app the node
    // view is its own window. The drag itself is one step.
    let edits: Vec<String> = app.editor.undo[before..]
        .iter()
        .map(|u| u.label.clone())
        .filter(|l| l != "Selection")
        .collect();
    assert_eq!(edits, ["Edit Base Material"], "one step for the drag");
    while app.editor.undo.len() > before + 1 {
        app.editor.dispatch(operandi_core::Command::Undo).unwrap();
    }
    app.editor.dispatch(operandi_core::Command::Undo).unwrap();
    assert_eq!(rough(&app), start);
}
