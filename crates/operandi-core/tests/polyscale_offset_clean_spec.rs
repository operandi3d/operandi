//! PolyScale Offset and Detach against
//! `specifications/polyscale-offset-2026-11-06`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "polyscale-offset-2026-11-06";

fn run(c: &Value) -> std::result::Result<(), String> {
    let id = c["id"].as_str().unwrap();
    let input = &c["input"];
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(input).into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = indices(&input["selected_polygons"]);
    e.synchronize_identities().unwrap();
    let value = if id == "B3" { -0.1 } else { 0.1 };
    let command = if id == "B4" {
        Command::ScaleOffsetDetach(value)
    } else {
        Command::ScaleOffsetPolygon(value)
    };
    e.dispatch(command).map_err(|x| format!("refused: {x}"))?;
    let want = &c["expected"]["after"]["meshes"]["Cube"];
    let got = &e.scene.active().unwrap().mesh;
    same_mesh(got, want, 1e-6)?;
    let selected = indices(&want["selected_polygons"]);
    if selected.is_empty() != e.scene.selection.faces.is_empty() {
        return Err(format!("selection {:?}", e.scene.selection.faces));
    }
    // Undo restores the cube.
    e.dispatch(Command::Undo).unwrap();
    if *e.scene.active().unwrap().mesh != mesh(input) {
        return Err("undo didn't restore the input".into());
    }
    Ok(())
}

#[test]
fn every_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn zero_changes_nothing() {
    let all = cases(SPEC);
    let c = case(&all, "B1");
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![1];
    e.synchronize_identities().unwrap();
    e.dispatch(Command::ScaleOffsetPolygon(0.)).unwrap();
    assert!(e.undo.is_empty());
}
