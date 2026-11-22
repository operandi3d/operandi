//! Flatten Ngons against `specifications/offset-flatten-2026-11-05`.
//! Counts, topology, which polygons may change and the selection after
//! match exactly. Since PF-08 v2 (2026-11-20, Newell normal through the
//! bounding-box centre) the recorded positions are reference only: they
//! came from a different normal, so points are checked against the rule
//! (planar n-gons, points of triangles and quads unmoved) instead.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "offset-flatten-2026-11-05";
fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = source.clone().into();
    e.synchronize_identities().unwrap();
    let mode = if c["selection_mode"] == "vertex" {
        Mode::Vertex
    } else {
        Mode::Polygon
    };
    e.dispatch(Command::Mode(mode)).unwrap();
    e.scene.selection.faces = indices(&c["selection"]["sel_polygons"]);
    e.scene.selection.vertices = indices(&c["selection"]["sel_vertices"]);
    e.dispatch(Command::FlattenNgons)
        .map_err(|x| format!("refused: {x}"))?;
    let got = &e.scene.active().unwrap().mesh;
    let want = mesh(&c["expected"]);
    if got.faces.len() != want.faces.len() || got.vertices.len() != want.vertices.len() {
        return Err("counts differ".into());
    }
    if got
        .faces
        .iter()
        .zip(&want.faces)
        .any(|(a, b)| a.vertices != b.vertices)
    {
        return Err("topology changed".into());
    }
    // Only points of n-gons may move.
    let ngon_point: Vec<bool> = (0..source.vertices.len() as u32)
        .map(|v| {
            source
                .faces
                .iter()
                .any(|f| f.vertices.len() >= 5 && f.vertices.contains(&v))
        })
        .collect();
    for (v, g) in got.vertices.iter().enumerate() {
        if !ngon_point[v] && (*g - source.vertices[v]).length() > 1e-12 {
            return Err(format!("point {v} of a triangle or quad moved"));
        }
    }
    // Every n-gon ends planar.
    for (f, face) in got.faces.iter().enumerate() {
        if face.vertices.len() >= 5 {
            let n = got.normal(f).normalize();
            let p0 = got.vertices[face.vertices[0] as usize];
            for &v in &face.vertices {
                if (got.vertices[v as usize] - p0).dot(n).abs() > 1e-9 {
                    return Err(format!("polygon {f} is not planar"));
                }
            }
        }
    }
    if !e.scene.selection.faces.is_empty() || !e.scene.selection.vertices.is_empty() {
        return Err("selection should be cleared".into());
    }
    if e.scene.selection.mode != mode {
        return Err("mode changed".into());
    }
    Ok(())
}

#[test]
fn every_case_follows_the_rules() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
