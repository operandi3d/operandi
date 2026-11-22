//! Polygon Reduce against `specifications/poly-reduce-2026-11-05`.
//!
//! Decided (Q-RD-03): Operandi uses its own reduction and matches the rules,
//! not Modo's exact meshes; unused points are removed (Q-RD-02); no Legacy
//! Mode. So each case checks the polygon-count rule, the no-op rules, kept
//! peaks and outline corners, and that no unused points remain.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "poly-reduce-2026-11-05";

fn editor(m: Mesh, selected: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = selected;
    e.undo.clear();
    e
}

fn params(s: &Value) -> ReductionParameters {
    ReductionParameters {
        number: s["number"].as_u64().unwrap() as u32,
        boundary_weight: s["boundaryWeight"].as_f64().unwrap() * 100.,
        material_border_weight: s["materialWeight"].as_f64().unwrap() * 100.,
        legacy: s["method"] != 0,
    }
}

fn reduce(c: &Value) -> std::result::Result<Editor, String> {
    let source = mesh(&c["input"]);
    let mut e = editor(source, indices(&c["selection"]["sel_polygons"]));
    let p = params(&c["settings"]);
    e.dispatch(Command::ArmReduction)
        .map_err(|x| x.to_string())?;
    if p.number != 0 {
        e.dispatch(Command::InitializeReduction)
            .map_err(|x| x.to_string())?;
    }
    e.dispatch(Command::ConfigureReduction(p))
        .map_err(|x| x.to_string())?;
    e.dispatch(Command::Apply).map_err(|x| x.to_string())?;
    Ok(e)
}

fn has(m: &Mesh, q: DVec3) -> bool {
    m.vertices.iter().any(|v| (*v - q).length() < 1e-9)
}

#[test]
fn every_case_follows_the_rules() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap().to_string();
        if c["settings"]["method"] != 0 {
            // Legacy Mode: Operandi has none (decided Q-RD-03).
            continue;
        }
        let source = mesh(&c["input"]);
        let selected = indices(&c["selection"]["sel_polygons"]);
        let e = match reduce(&c) {
            Ok(e) => e,
            Err(x) => {
                failures.push(format!("{id}: refused: {x}"));
                continue;
            }
        };
        let got = &e.scene.active().unwrap().mesh;
        let want = mesh(&c["expected"]);
        // Count rule, including the no-op cases (same count as the input).
        if got.faces.len() != want.faces.len() {
            failures.push(format!(
                "{id}: {} polygons, expected {}",
                got.faces.len(),
                want.faces.len()
            ));
            continue;
        }
        let unchanged = want.faces.len() == source.faces.len();
        if unchanged && **got != source {
            failures.push(format!("{id}: should leave the mesh unchanged"));
        }
        // No unused points (decided Q-RD-02).
        if got.used_points().iter().any(|u| !u) {
            failures.push(format!("{id}: unused points remain"));
        }
        // Shape kept: a raised peak survives; on a flat grid the outline
        // corners do (rule 4).
        let peak = source.vertices.iter().find(|v| v.z > 0.5).copied();
        if let Some(peak) = peak
            && !has(got, peak)
        {
            failures.push(format!("{id}: lost the peak {peak}"));
        }
        if peak.is_none() && !unchanged && c["settings"]["boundaryWeight"] == 1. {
            for corner in [(-2., -2.), (2., -2.), (2., 2.), (-2., 2.)] {
                if !has(got, DVec3::new(corner.0, corner.1, 0.)) {
                    failures.push(format!("{id}: lost the outline corner {corner:?}"));
                }
            }
        }
        // Selection after: unchanged.
        if e.scene.selection.faces != selected {
            failures.push(format!("{id}: selection {:?}", e.scene.selection.faces));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn legacy_mode_is_refused() {
    let p = ReductionParameters {
        legacy: true,
        ..Default::default()
    };
    assert!(p.validate_basic().is_err());
}
