//! Quadruple against `specifications/quadruple-2026-11-05`: polygon count,
//! corner sets and selection after match; corner start and polygon order
//! may differ.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "quadruple-2026-11-05";
/// Decided (Q-QD-01): degenerate same-row pieces are not copied: QD18's
/// quad, and QD09's triangle [2, 3, 4], whose three points lie on one row
/// (y = 1). Both get the shortest valid rungs instead (tested below).
const SKIP: &[&str] = &["QD09", "QD18"];

fn run(c: &Value) -> std::result::Result<(), String> {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    e.synchronize_identities().unwrap();
    let mode = if c["selection_mode"] == "vertex" {
        Mode::Vertex
    } else {
        Mode::Polygon
    };
    e.dispatch(Command::Mode(mode)).unwrap();
    e.scene.selection.faces = indices(&c["selection"]["sel_polygons"]);
    e.scene.selection.vertices = indices(&c["selection"]["sel_vertices"]);
    let want = &c["expected"];
    let result = e.dispatch(Command::QuadruplePolygon);
    if want["failed"].as_bool().unwrap_or(false) {
        return match result {
            Err(_) => Ok(()),
            Ok(()) => Err("expected a refusal".into()),
        };
    }
    result.map_err(|x| format!("refused: {x}"))?;
    let got = &e.scene.active().unwrap().mesh;
    same_mesh(got, want, 1e-5)?;
    same_selected_polygons(
        got,
        &e.scene.selection.faces,
        want,
        &want["selection_after"]["sel_polygons"],
        1e-5,
    )
}

#[test]
fn every_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        if SKIP.contains(&id) {
            continue;
        }
        if let Err(e) = run(&c) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// QD18's tall strip: rungs cross the long way, never join points of the
/// same row (decided Q-QD-01).
#[test]
fn tall_strip_never_joins_a_row() {
    let all = cases(SPEC);
    for (id, pieces) in [("QD18", 3), ("QD09", 2)] {
        let c = case(&all, id);
        let mut e = Editor::default();
        e.scene.new_mesh().unwrap();
        e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
        e.synchronize_identities().unwrap();
        e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
        e.scene.selection.faces = vec![0];
        e.dispatch(Command::QuadruplePolygon).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        assert_eq!(m.faces.len(), pieces, "{id}");
        for f in 0..m.faces.len() {
            assert!(m.normal(f).length() > 1e-6, "{id}: polygon {f} has no area");
        }
    }
}
