//! Smooth Shift and Thicken against `specifications/smooth-shift-thicken-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "smooth-shift-thicken-2026-11-05";

/// Cases where Operandi follows its own rule instead of the probed result
/// (the spec calls both results unexplained). They are not checked against
/// the reference; `sharp_split_moves_offset` covers Operandi's rule.
const OPEN: &[(&str, &str)] = &[
    (
        "SS11b",
        "Q-SS-03: Sharp with split smoothing groups (moved 0.3, not copied)",
    ),
    (
        "SS11d",
        "Q-SS-05: Both Sides on a closed corner (not copied)",
    ),
];

fn params(s: &Value) -> SmoothShiftParameters {
    SmoothShiftParameters {
        offset: s["Offset"].as_f64().unwrap(),
        scale: s["Scale (%)"].as_f64().unwrap() / 100.,
        angle: s["Max Smoothing Angle (deg)"].as_f64().unwrap(),
        thicken: s["Thicken"].as_bool().unwrap(),
        sharp: s["Sharp"].as_bool().unwrap(),
        angle_weight: s["Polygon Angle Weighting"].as_bool().unwrap(),
        both_sides: s["Both Sides"].as_bool().unwrap(),
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let source = mesh(&c["input"]);
    let selected = indices(&c["selection"]["polygons"]);
    let r = smooth_shift_geometry(&source, &selected, params(&c["settings"]), None)
        .map_err(|e| format!("refused: {e}"))?;
    let tol = tolerance(c);
    same_mesh(&r.mesh, &c["expected"], tol)?;
    same_selected_polygons(
        &r.mesh,
        &r.selected,
        &c["expected"],
        &c["expected"]["selection_after"]["polygons"],
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
fn thicken_preset_and_reset_defaults() {
    let ss = SmoothShiftParameters::default();
    assert_eq!(
        (ss.offset, ss.scale, ss.angle, ss.thicken),
        (0., 1., 89.5, false)
    );
    assert!(!ss.sharp && !ss.angle_weight && !ss.both_sides);
    let th = SmoothShiftParameters::thicken_initial();
    assert_eq!(
        (th.offset, th.scale, th.angle, th.thicken),
        (0., 1., 90.1, true)
    );
    assert!(!th.sharp && !th.angle_weight && !th.both_sides);
}

/// Q-SS-03, Operandi's rule: with Sharp on and the faces split by the
/// smoothing angle, each face is its own group, so its points move exactly
/// Offset (SS11b's input: cube top + front, 89.5 degrees, Offset 0.1).
#[test]
fn sharp_split_moves_offset() {
    let c = cases(SPEC)
        .into_iter()
        .find(|c| c["id"] == "SS11b")
        .unwrap();
    let source = mesh(&c["input"]);
    let selected = indices(&c["selection"]["polygons"]);
    let r = smooth_shift_geometry(&source, &selected, params(&c["settings"]), None).unwrap();
    for &f in &r.selected {
        let face = &r.mesh.faces[f as usize];
        let pts: Vec<_> = face
            .vertices
            .iter()
            .map(|&v| r.mesh.vertices[v as usize])
            .collect();
        let top = pts.iter().all(|q| (q.y - 0.6).abs() < 1e-9);
        let front = pts.iter().all(|q| (q.z - 0.6).abs() < 1e-9);
        let bridge = pts.iter().all(|q| q.y.max(q.z) > 0.5 - 1e-9);
        assert!(top || front || bridge, "unexpected selected face {pts:?}");
    }
    let lifted = r
        .mesh
        .vertices
        .iter()
        .filter(|q| (q.y - 0.6).abs() < 1e-9)
        .count();
    assert_eq!(lifted, 4, "top face points should sit at y 0.6");
}
