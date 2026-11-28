//! MN-05: Reset Tool Attributes for every tool, from the Edit menu, its
//! key (Ctrl+D, Cmd+D on a Mac) and `tool.reset`. One undo step named
//! "Reset <tool>"; the tool stays active.
use super::tests::{PLATFORM_CMD, box_app, draw};
use super::*;

const ACTION: &str = "Reset Tool Attributes";

fn polygon_app() -> Operandi {
    let mut a = box_app();
    a.run(Command::Mode(Mode::Polygon));
    a.editor.scene.selection.faces = vec![0];
    a.editor.history.undo.clear();
    a
}

fn params(a: &Operandi) -> Parameters {
    a.editor
        .session
        .live
        .as_ref()
        .expect("a live tool")
        .params
        .clone()
}

fn tool(a: &Operandi) -> Option<Tool> {
    a.editor.session.live.as_ref().map(|l| l.tool)
}

/// What `t` starts with on a fresh launch, on the same scene.
fn fresh_start(setup: fn() -> Operandi, t: Tool) -> Parameters {
    let mut a = setup();
    a.activate(t);
    params(&a)
}

#[test]
fn edit_menu_entry_is_greyed_without_a_tool() {
    let mut a = polygon_app();
    let state = a.line_state(ACTION).unwrap();
    assert_eq!(
        state.unavailable.as_deref(),
        Some("No tool is active, so there is nothing to reset")
    );
    a.activate(Tool::Move);
    assert_eq!(a.line_state(ACTION).unwrap().unavailable, None);
    let edit = a.forms.shipped("menu.edit").unwrap();
    assert!(
        edit.controls
            .iter()
            .any(|c| c.command.as_deref() == Some(ACTION) && c.label == ACTION),
        "Edit menu has the entry"
    );
}

#[test]
fn move_resets_to_zero_keeps_the_tool_and_is_one_named_undo_step() {
    let mut a = polygon_app();
    a.activate(Tool::Move);
    let base = a.editor.scene.clone();
    let moved = Parameters {
        delta: DVec3::new(1., 0., 0.),
        ..params(&a)
    };
    a.run(Command::Preview(moved));
    assert_ne!(a.editor.scene, base);
    let after_move = a.editor.scene.clone();
    a.execute(ACTION);
    assert_eq!(tool(&a), Some(Tool::Move), "{}", a.status);
    assert_eq!(params(&a).delta, DVec3::ZERO);
    assert_eq!(a.editor.scene, base, "the live result updates at once");
    let undo = a.line_state("app.undo").unwrap().title.unwrap();
    assert_eq!(undo, "Undo Reset Move");
    a.run(Command::Undo);
    assert_eq!(params(&a).delta, DVec3::new(1., 0., 0.));
    assert_eq!(a.editor.scene, after_move);
    assert_eq!(tool(&a), Some(Tool::Move));
    let redo = a.line_state("app.redo").unwrap().title.unwrap();
    assert_eq!(redo, "Redo Reset Move");
    a.run(Command::Redo);
    assert_eq!(params(&a).delta, DVec3::ZERO);
}

#[test]
fn tools_that_refused_before_now_reset_to_their_factory_values() {
    // Vertex Bevel refused "Reset is unsupported for this tool" before.
    let vertex = || {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Vertex));
        a.editor.scene.selection.vertices = vec![0];
        a
    };
    let mut a = vertex();
    a.execute("tool.set vert.bevel on");
    let factory = params(&a);
    let changed = Parameters {
        shift: factory.shift + 0.1,
        ..factory.clone()
    };
    a.run(Command::Preview(changed));
    a.execute("tool.reset");
    assert_eq!(params(&a), factory, "{}", a.status);
    assert_eq!(tool(&a), Some(Tool::VertexBevel));

    // Inset: Polygon mode.
    let mut a = polygon_app();
    a.activate(Tool::Inset);
    let factory = params(&a);
    a.run(Command::Preview(Parameters {
        inset: 0.2,
        ..factory.clone()
    }));
    assert_ne!(params(&a), factory);
    a.execute("tool.reset poly.inset");
    assert_eq!(
        params(&a),
        fresh_start(polygon_app, Tool::Inset),
        "{}",
        a.status
    );
}

#[test]
fn remembered_last_use_values_are_not_the_reset_target() {
    let edges = || {
        let mut a = box_app();
        a.run(Command::Mode(Mode::Edge));
        a.editor.scene.selection.edges = vec![(0, 1)];
        a
    };
    let factory = fresh_start(edges, Tool::EdgeBevel);
    let mut a = edges();
    // An earlier use remembered 3 segments.
    a.editor
        .presets
        .get_mut::<operandi_core::EdgeBevelToolPreset>()
        .segments = 3;
    a.activate(Tool::EdgeBevel);
    assert_eq!(params(&a).chamfer.segments, 3);
    a.run(Command::ResetToolAttributes);
    assert_eq!(params(&a).chamfer.segments, factory.chamfer.segments);
    assert_eq!(tool(&a), Some(Tool::EdgeBevel));
}

#[test]
fn ctrl_d_resets_while_a_tool_is_active_and_does_nothing_otherwise() {
    assert_eq!(input::default_bindings()[ACTION], "Cmd+D");
    let ctx = Context::default();
    let size = vec2(1440., 900.);
    let press = Event::Key {
        key: egui::Key::D,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: PLATFORM_CMD,
    };
    let mut a = polygon_app();
    draw(&mut a, &ctx, size, vec![]);
    let scene = a.editor.scene.clone();
    let undo = a.editor.history.undo.len();
    let status = a.status.clone();
    let release = Event::Key {
        key: egui::Key::D,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: PLATFORM_CMD,
    };
    draw(&mut a, &ctx, size, vec![press.clone(), release]);
    assert_eq!(a.editor.scene, scene);
    assert_eq!(a.editor.history.undo.len(), undo);
    assert!(!a.editor.session.has_active_task());
    assert_eq!(a.status, status, "no tool: the key does nothing");

    a.activate(Tool::Move);
    let moved = Parameters {
        delta: DVec3::new(0., 1., 0.),
        ..params(&a)
    };
    a.run(Command::Preview(moved));
    draw(&mut a, &ctx, size, vec![press]);
    assert_eq!(params(&a).delta, DVec3::ZERO, "{}", a.status);
    assert_eq!(tool(&a), Some(Tool::Move));
}

/// Every tool that starts on a box in one of the component modes resets
/// without refusing, and stays active.
#[test]
fn every_tool_resets() {
    let setups: [fn() -> Operandi; 4] = [
        polygon_app,
        || {
            let mut a = box_app();
            a.run(Command::Mode(Mode::Edge));
            a.editor.scene.selection.edges = vec![(0, 1)];
            a
        },
        || {
            let mut a = box_app();
            a.run(Command::Mode(Mode::Vertex));
            a.editor.scene.selection.vertices = vec![0, 1];
            a
        },
        || {
            let mut a = box_app();
            a.run(Command::Mode(Mode::Item));
            a
        },
    ];
    let mut reset = Vec::new();
    for t in Tool::ALL {
        for setup in setups {
            let mut a = Box::new(setup());
            a.activate(t);
            if tool(&a) != Some(t) {
                continue;
            }
            a.status.clear();
            a.run(Command::ResetToolAttributes);
            assert_eq!(tool(&a), Some(t), "{t:?}: {}", a.status);
            assert!(
                !a.status.to_ascii_lowercase().contains("reset"),
                "{t:?} refused: {}",
                a.status
            );
            reset.push(t);
            break;
        }
    }
    assert!(
        reset.len() > 40,
        "only {} tools started: {reset:?}",
        reset.len()
    );
}
