//! Inset against `specifications/inset-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "inset-2026-11-05";

fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let selected = indices(&c["selection"]["sel_polygons"]);
    let inset = c["settings"]["inset"].as_f64().unwrap();
    let r = inset_geometry(&source, &selected, inset).map_err(|e| format!("refused: {e}"))?;
    let e = &c["expected"];
    // Polygon order is free (decided Q-IN-01): compared as a set.
    same_mesh(&r.mesh, e, 1e-5)?;
    same_selected_polygons(
        &r.mesh,
        &r.selected,
        e,
        &e["selection_after"]["sel_polygons"],
        1e-5,
    )
}

#[test]
fn every_admitted_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Provisional UV rule (Q-IN-04): new corners take the UV their position has
/// in the polygon's own UV layout.
#[test]
fn inner_corners_follow_the_polygon_uvs() {
    let all = cases(SPEC);
    let mut quad = mesh(&case(&all, "IN02")["input"]);
    let uv = |x: f64, y: f64| Some(glam::DVec2::new(x, y));
    // Corners [0, 1, 3, 2] at (-0.5,-0.5), (0.5,-0.5), (0.5,0.5), (-0.5,0.5).
    quad.uv_maps.insert(
        "Texture".into(),
        vec![vec![uv(0., 0.), uv(1., 0.), uv(1., 1.), uv(0., 1.)]],
    );
    let r = inset_geometry(&quad, &[0], 0.1).unwrap();
    let rows = &r.mesh.uv_maps["Texture"];
    let inner = &rows[0];
    for (corner, want) in inner
        .iter()
        .zip([uv(0.1, 0.1), uv(0.9, 0.1), uv(0.9, 0.9), uv(0.1, 0.9)])
    {
        assert!(
            (corner.unwrap() - want.unwrap()).length() < 1e-9,
            "{corner:?}"
        );
    }
    assert!(rows.iter().flatten().all(Option::is_some));
}
