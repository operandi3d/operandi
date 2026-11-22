//! Flip Polygons against `specifications/poly-flip-2026-11-05`: corner
//! order must match exactly (first corner kept).
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "poly-flip-2026-11-05";

fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = source.clone().into();
    let mode = match c["selection_mode"].as_str().unwrap() {
        "vertex" => Mode::Vertex,
        "edge" => Mode::Edge,
        "item" => Mode::Item,
        _ => Mode::Polygon,
    };
    e.dispatch(Command::Mode(mode)).unwrap();
    let sel = &c["selection"];
    e.scene.selection.faces = indices(&sel["sel_polygons"]);
    e.scene.selection.vertices = indices(&sel["sel_vertices"]);
    let before = e.scene.selection.clone();
    let result = e.dispatch(Command::FlipPolygons);
    let want = &c["expected"];
    match (want["message"].as_str(), result) {
        // The reference message is wrapped as "ERR command failed: … (code)".
        (Some(msg), Err(got)) if msg.contains(got.as_str()) => {}
        (Some(msg), r) => return Err(format!("expected refusal {msg:?}, got {r:?}")),
        (None, Err(got)) => return Err(format!("refused: {got}")),
        (None, Ok(())) => {}
    }
    let got = &e.scene.active().unwrap().mesh;
    let polygons: Vec<Vec<u32>> = got.faces.iter().map(|f| f.vertices.clone()).collect();
    let expected: Vec<Vec<u32>> = want["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(indices)
        .collect();
    if polygons != expected {
        return Err(format!("polygons {polygons:?}"));
    }
    if got.vertices != source.vertices {
        return Err("points moved".into());
    }
    if e.scene.selection.faces != before.faces || e.scene.selection.mode != mode {
        return Err("selection changed".into());
    }
    Ok(())
}

#[test]
fn every_case_matches_exactly() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
