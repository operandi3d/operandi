//! Poly Haul's modes against `specifications/poly-haul-modes-2026-11-06`,
//! with the amounts Modo recorded for each drag.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "poly-haul-modes-2026-11-06";

fn settings(c: &Value) -> &Value {
    &c["expected"]["after"]["tool_settings"]["Poly Haul"]
}

fn after(c: &Value) -> &Value {
    c["expected"]["after"]["meshes"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
}

/// The mode, the element the drag started on and its amounts.
fn setup(c: &Value) -> Option<(PolyHaulMode, HaulTarget, HaulAmount)> {
    let s = settings(c);
    let f = |k: &str| s[k].as_f64().unwrap();
    let amount = HaulAmount {
        offset: f("offset"),
        inset: f("inset"),
        scale: f("scaleX"),
        position: f("position"),
        vector: DVec3::new(f("vectorX"), f("vectorY"), f("vectorZ")),
    };
    let (mode, target) = match c["id"].as_str().unwrap() {
        "C1" => (PolyHaulMode::Offset, HaulTarget::None),
        "C2" => (PolyHaulMode::Extrude, HaulTarget::None),
        // The top face's +X edge.
        "C3" => (PolyHaulMode::Bevel, HaulTarget::Edge(5, 6)),
        "C4" => (PolyHaulMode::Scale, HaulTarget::None),
        // The recorded mesh and Position are the first try's: 100 % along
        // the edge, no change. The second try is checked below.
        "C5" => (PolyHaulMode::Split, HaulTarget::Edge(2, 0)),
        "C7" => (PolyHaulMode::Remove, HaulTarget::Polygon(1)),
        "C8" => (
            PolyHaulMode::Tear,
            HaulTarget::Point {
                point: 6,
                polygon: 3,
            },
        ),
        "C9" => (
            PolyHaulMode::Slide,
            HaulTarget::Point {
                point: 4,
                polygon: 3,
            },
        ),
        // C6 Add Loop is open (Q-PH-05).
        _ => return None,
    };
    Some((mode, target, amount))
}

#[test]
fn every_mode_matches_its_recorded_drag() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        let Some((mode, target, amount)) = setup(&c) else {
            continue;
        };
        let source = mesh(&c["input"]);
        let selected = indices(&c["input"]["selected_polygons"]);
        let result = match poly_haul_mode_geometry(&source, &selected, mode, target, amount) {
            Ok(r) => r,
            Err(e) => {
                failures.push(format!("{id}: refused: {e}"));
                continue;
            }
        };
        let mut want = after(&c).clone();
        if id == "C9" {
            // Decided (Q-PH-06): Slide holds the point to the edge, so it
            // moves only the drag's share along +X: (0.362, 0, 0).
            want["points"][4] = serde_json::json!([0.362, 0.0, 0.0]);
        }
        if let Err(e) = same_mesh(&result.mesh, &want, 1e-5) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// C5's second try: a short drag down the left edge, 49.3 % from its top
/// point, cuts the quad across at y 0.007 (4/1 → 6/2).
#[test]
fn split_cuts_across_at_the_dragged_share() {
    let all = cases(SPEC);
    let c = case(&all, "C5");
    let source = mesh(&c["input"]);
    let amount = HaulAmount {
        position: 0.493,
        ..Default::default()
    };
    let r = poly_haul_mode_geometry(
        &source,
        &[0],
        PolyHaulMode::Split,
        HaulTarget::Edge(2, 0),
        amount,
    )
    .unwrap();
    assert_eq!((r.mesh.vertices.len(), r.mesh.faces.len()), (6, 2));
    for p in &r.mesh.vertices[4..] {
        assert!((p.y - 0.007).abs() < 1e-9, "{p}");
    }
    assert_eq!(r.faces, Some(vec![]), "selection cleared");
}

/// Through the live tool: Remove on the clicked polygon, then Drop is one
/// undo step; Cancel leaves the scene as it was.
#[test]
fn live_tool_remove_drop_undo_and_cancel() {
    let all = cases(SPEC);
    let c = case(&all, "C7");
    let start = |e: &mut Editor| {
        e.scene.new_mesh().unwrap();
        e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
        e.scene.selection.mode = Mode::Polygon;
        e.scene.selection.faces = vec![1];
        e.synchronize_identities().unwrap();
        e.dispatch(Command::Activate(Tool::PolyHaul)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.poly_haul_mode = PolyHaulMode::Remove;
        p.poly_haul_target = HaulTarget::Polygon(1);
        e.dispatch(Command::Preview(p)).unwrap();
        assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 5);
        assert!(e.scene.selection.faces.is_empty(), "selection cleared");
    };
    let mut e = Editor::default();
    start(&mut e);
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 5);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
    let mut e = Editor::default();
    start(&mut e);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 6);
}
