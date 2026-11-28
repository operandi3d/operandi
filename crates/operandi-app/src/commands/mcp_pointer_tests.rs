//! The bridge's `app_command` and `pointer_stroke` (mcp_pointer.rs) in the
//! app: paint commands run, a stroke paints through the real input path
//! and answers after its release, pen pressure reaches the brush, and bad
//! arguments are refused before anything happens.
use super::*;
use crate::tests::draw;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

const SIZE: egui::Vec2 = egui::vec2(1400., 900.);

fn line(name: &str, arguments: Value) -> String {
    json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}})
    .to_string()
}

/// A call answered now: its structured result, or the error text.
fn call(a: &mut Operandi, name: &str, arguments: Value) -> std::result::Result<Value, String> {
    let r = a.mcp_answer(&line(name, arguments)).expect("an answer");
    let result = &r["result"];
    if result["isError"] == json!(true) {
        Err(result["content"][0]["text"].as_str().unwrap().to_string())
    } else {
        Ok(result["structuredContent"].clone())
    }
}

/// A call that may be answered on a later frame: queue it, draw frames
/// until its answer comes (at most 200).
fn call_later(
    a: &mut Operandi,
    ctx: &Context,
    name: &str,
    arguments: Value,
) -> std::result::Result<Value, String> {
    let answer: Arc<Mutex<Option<Value>>> = Arc::default();
    let slot = answer.clone();
    a.mcp_queue(
        &line(name, arguments),
        Box::new(move |v| *slot.lock().unwrap() = v),
    );
    for _ in 0..200 {
        if answer.lock().unwrap().is_some() {
            break;
        }
        let _ = draw(a, ctx, SIZE, vec![]);
        a.serve_mcp(ctx);
    }
    let r = answer
        .lock()
        .unwrap()
        .take()
        .expect("answered within 200 frames");
    let result = &r["result"];
    if result["isError"] == json!(true) {
        Err(result["content"][0]["text"].as_str().unwrap().to_string())
    } else {
        Ok(result["structuredContent"].clone())
    }
}

/// A sphere seen from the front, laid out, ready for bridge calls.
fn sphere() -> (Operandi, Context) {
    let ctx = Context::default();
    theme::install(&ctx);
    let mut a = Operandi::empty();
    a.initialize_new_document();
    a.run(Command::InsertUnit {
        tool: Tool::Sphere,
        destination: operandi_core::UnitDestination::ExistingMesh,
    });
    a.camera.view = crate::camera::View::Front;
    a.frame(false);
    for _ in 0..3 {
        let _ = draw(&mut a, &ctx, SIZE, vec![]);
    }
    (a, ctx)
}

/// A horizontal line of view points through the view's centre.
fn across(a: &Operandi, n: usize) -> Value {
    let c = a.viewport_rect.size() / 2.;
    json!(
        (0..n)
            .map(|k| [c.x - 60. + k as f32 * 6., c.y])
            .collect::<Vec<_>>()
    )
}

#[cfg(feature = "painting")]
fn brightest(a: &Operandi) -> f32 {
    let image = a.current_image().unwrap();
    let p = &a.editor.scene.paint.buffers[&image];
    (0..p.pixel_count()).map(|i| p.get(i)[0]).fold(0., f32::max)
}

#[cfg(feature = "painting")]
#[test]
fn app_command_runs_paint_commands_and_refuses_a_dialog() {
    let (mut a, _ctx) = sphere();
    let steps = a.editor.history.undo.len();
    let out = call(
        &mut a,
        "app_command",
        json!({"line": "operandi.paint.addTexture kind:color resolution:128 color:0,0,0"}),
    )
    .unwrap();
    assert!(a.paint_target().is_some(), "{}", a.status);
    assert_eq!(out["undo"], json!(steps + 1), "one undo step: {out}");
    call(
        &mut a,
        "app_command",
        json!({"line": "operandi.paint.airbrush"}),
    )
    .unwrap();
    assert!(a.painting.brush.tool.is_some(), "the paint tool is on");
    // With the tool on, other app commands still run (acts as the person).
    call(
        &mut a,
        "app_command",
        json!({"line": "operandi.paint.set size:25"}),
    )
    .unwrap();
    // A bare New Image opens the panel: an error naming it.
    let e = call(&mut a, "app_command", json!({"line": "operandi.image.new"})).unwrap_err();
    assert!(e.contains("painting dialog"), "{e}");
    assert!(a.painting_dialog_open(), "it waits for the person");
    // And while it is open, the bridge's edits are refused.
    let e = call(
        &mut a,
        "app_command",
        json!({"line": "operandi.paint.fill"}),
    )
    .unwrap_err();
    assert!(e.contains("dialog open"), "{e}");
}

#[test]
fn app_command_reports_an_unknown_line() {
    let (mut a, _ctx) = sphere();
    assert!(call(&mut a, "app_command", json!({"line": "no.such.command"})).is_err());
    assert!(call(&mut a, "app_command", json!({})).is_err());
    assert!(call(&mut a, "app_command", json!({"line": "x", "extra": 1})).is_err());
}

#[cfg(feature = "painting")]
#[test]
fn pointer_stroke_paints_one_undo_step_and_answers_after_the_release() {
    let (mut a, ctx) = sphere();
    call(
        &mut a,
        "app_command",
        json!({"line": "operandi.paint.addTexture kind:color resolution:256 color:0,0,0"}),
    )
    .unwrap();
    call(
        &mut a,
        "app_command",
        json!({"line": "operandi.paint.airbrush"}),
    )
    .unwrap();
    let steps = a.editor.history.undo.len();
    let points = across(&a, 20);
    let out = call_later(&mut a, &ctx, "pointer_stroke", json!({"points": points})).unwrap();
    assert_eq!(a.editor.history.undo.len(), steps + 1, "{out}");
    assert_eq!(
        a.editor.history.undo.last().unwrap().label,
        "Airbrush Stroke"
    );
    assert!(brightest(&a) > 0.9, "white paint landed");
    assert!(!a.mcp_stroke_running());
    // Undo and redo through the bridge.
    call(&mut a, "undo", json!({})).unwrap();
    assert!(brightest(&a) < 0.01, "undone");
    call(&mut a, "redo", json!({})).unwrap();
    assert!(brightest(&a) > 0.9, "redone");
}

#[cfg(feature = "painting")]
#[test]
fn pen_pressure_lightens_a_light_stroke() {
    let mut peak = Vec::new();
    for pressure in [0.25, 1.] {
        let (mut a, ctx) = sphere();
        call(
            &mut a,
            "app_command",
            json!({"line": "operandi.paint.addTexture kind:color resolution:256 color:0,0,0"}),
        )
        .unwrap();
        call(
            &mut a,
            "app_command",
            json!({"line": "operandi.paint.paintbrush"}),
        )
        .unwrap();
        let points = across(&a, 12);
        call_later(
            &mut a,
            &ctx,
            "pointer_stroke",
            json!({"points": points, "pressure": pressure}),
        )
        .unwrap();
        peak.push(brightest(&a));
    }
    assert!(
        peak[0] < peak[1] - 0.2,
        "a light press paints fainter: {peak:?}"
    );
    // Later mouse strokes are full pressure again (recorded as a mouse).
    assert_eq!(crate::tablet::pressure_at([1., 1.]), 1.);
}

#[test]
fn pointer_stroke_refuses_bad_arguments_before_anything_happens() {
    let (mut a, ctx) = sphere();
    let steps = a.editor.history.undo.len();
    for (args, says) in [
        (json!({}), "points"),
        (
            json!({"points": [[10, 10]], "world": [[0, 0, 0]]}),
            "not both",
        ),
        (json!({"points": []}), "1 to"),
        (json!({"points": [[-5, 10]]}), "outside the view"),
        (
            json!({"points": [[10, 10], [20, 10]], "pressure": [1]}),
            "one per point",
        ),
        (json!({"points": [[10, 10]], "pressure": 2}), "0 to 1"),
        (
            json!({"points": [[10, 10]], "button": "fourth"}),
            "left, right",
        ),
        (
            json!({"points": [[10, 10]], "modifiers": ["hyper"]}),
            "shift, ctrl",
        ),
        (json!({"points": [[10, 10]], "viewport": 9}), "No viewport"),
    ] {
        let e = call_later(&mut a, &ctx, "pointer_stroke", args.clone()).unwrap_err();
        assert!(e.contains(says), "{args}: {e}");
    }
    assert!(!a.mcp_stroke_running());
    assert_eq!(a.editor.history.undo.len(), steps);
}

#[test]
fn a_click_selects_like_the_persons_click() {
    let (mut a, ctx) = sphere();
    a.run(Command::Mode(Mode::Polygon));
    let c = a.viewport_rect.size() / 2.;
    let out = call_later(
        &mut a,
        &ctx,
        "pointer_stroke",
        json!({"points": [[c.x, c.y]]}),
    )
    .unwrap();
    assert_eq!(
        a.editor.scene.selection.faces.len(),
        1,
        "one polygon picked: {out}"
    );
    // Shift adds a second one.
    call_later(
        &mut a,
        &ctx,
        "pointer_stroke",
        json!({"points": [[c.x + 60., c.y + 40.]], "modifiers": ["shift"]}),
    )
    .unwrap();
    assert_eq!(a.editor.scene.selection.faces.len(), 2);
}
