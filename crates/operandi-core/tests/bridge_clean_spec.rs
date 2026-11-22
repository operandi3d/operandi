//! Bridge against `specifications/bridge-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "bridge-2026-11-05";

/// Cases whose rule the spec leaves open.
const OPEN: &[(&str, &str)] = &[("BR12", "Q-BR-03: Reverse Direction pairing")];

fn params(s: &Value) -> BridgeParameters {
    BridgeParameters {
        segments: s["segments"].as_u64().unwrap() as u32,
        twist: s["twist"].as_i64().unwrap() as i32,
        mode: match s["mode"].as_str().unwrap() {
            "linear" => BridgeMode::Linear,
            "smooth" => BridgeMode::Smooth,
            _ => BridgeMode::Curve,
        },
        remove: s["remove"] == 1,
        flip: s["flip"] == 1,
        reverse: s["orient"] == 1,
        tension: s["tension"].as_f64().unwrap(),
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let sel = &c["selection"];
    let e = &c["expected"];
    let tol = 1e-5;
    if c["selection_mode"] == "edge" {
        let edges: Vec<(u32, u32)> = sel["sel_edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                let x = indices(x);
                (x[0], x[1])
            })
            .collect();
        let s = &c["settings"];
        let (out, _) = edge_bridge_geometry(
            &source,
            &edges,
            s["segments"].as_u64().unwrap() as u32,
            s["twist"].as_i64().unwrap() as i32,
        )
        .map_err(|e| format!("refused: {e}"))?;
        return same_mesh(&out, e, tol);
    }
    let selected = indices(&sel["sel_polygons"]);
    let r = bridge_geometry(&source, &selected, params(&c["settings"]))
        .map_err(|e| format!("refused: {e}"))?;
    same_mesh(&r.mesh, e, tol)?;
    same_selected_polygons(
        &r.mesh,
        &r.selected,
        e,
        &e["selection_after"]["sel_polygons"],
        tol,
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

#[test]
fn reset_defaults() {
    let p = BridgeParameters::default();
    assert_eq!((p.segments, p.twist, p.mode), (0, 0, BridgeMode::Curve));
    assert!(p.remove && !p.flip && !p.reverse);
    assert_eq!(p.tension, 1.);
}
