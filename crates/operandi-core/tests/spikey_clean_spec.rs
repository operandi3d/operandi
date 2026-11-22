//! Spikey against `specifications/spikey-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "spikey-2026-11-05";

/// Q-SP-01: the normal for non-planar polygons is open.
const OPEN: &[(&str, &str)] = &[("SP06", "Q-SP-01: non-planar normal")];

fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let selected = indices(&c["selection"]["sel_polygons"]);
    let factor = c["settings"]["factor"].as_f64().unwrap();
    let r = spike_geometry(&source, &selected, factor).map_err(|e| format!("refused: {e}"))?;
    let e = &c["expected"];
    same_mesh(&r.mesh, e, 1e-5)?;
    // Selection after, as the editor applies it (rule 4): the new triangles
    // when part of the mesh was selected, nothing otherwise.
    let whole = selected.is_empty() || selected.len() == source.faces.len();
    let after: Vec<u32> = if whole {
        Vec::new()
    } else {
        selected
            .iter()
            .flat_map(|&f| r.descendants[f as usize].iter().copied())
            .collect()
    };
    same_selected_polygons(
        &r.mesh,
        &after,
        e,
        &e["selection_after"]["sel_polygons"],
        1e-5,
    )
}

#[test]
fn every_admitted_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        if OPEN.iter().any(|(o, _)| *o == id) {
            continue;
        }
        if let Err(e) = run(&c) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
