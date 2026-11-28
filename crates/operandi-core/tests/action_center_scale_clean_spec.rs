//! The Scale tool's centre per action-center mode against
//! `specifications/action-center-scale-2026-11-06`: uniform 50 %, item at
//! its recorded position, points compared in the mesh's own coordinates.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "action-center-scale-2026-11-06";

fn center(id: &str) -> CenterMode {
    match id {
        "F1" => CenterMode::Selection,
        "F2" => CenterMode::SelectionBorder,
        "F3" => CenterMode::SelectionAutoAxis,
        // The clicked top corner (0.5, 0.5, 0.5) local, world + (2, 1, 0).
        "F4" => CenterMode::Element(PickedPoint::new(DVec3::new(2.5, 1.5, 0.5))),
        "F5" => CenterMode::Origin,
        // The click's world point (Q-AC-10: how a click maps to depth).
        "F6" => CenterMode::Screen(PickedPoint::new(DVec3::new(1.642, 0.407, 0.645))),
        _ => CenterMode::Local,
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let id = c["id"].as_str().unwrap();
    let input = &c["input"];
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    {
        let item = e.scene.active_mut().unwrap();
        item.mesh = mesh(input).into();
        if !input["item_position"].is_null() {
            item.transform.translation = point(&input["item_position"]);
        }
    }
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = indices(&input["selected_polygons"]);
    e.synchronize_identities().unwrap();
    let center = center(id);
    e.dispatch(Command::ActionContext(ActionContext {
        center,
        // Local by name is Local's centre with Local's axes.
        axis: if center == CenterMode::Local {
            AxisOrientation::Local
        } else {
            AxisOrientation::World
        },
    }))
    .map_err(|x| format!("context refused: {x}"))?;
    e.dispatch(Command::Activate(Tool::Scale))
        .map_err(|x| format!("activate: {x}"))?;
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::splat(0.5);
    e.dispatch(Command::Preview(p))
        .map_err(|x| format!("preview: {x}"))?;
    e.dispatch(Command::Drop).unwrap();
    let want = c["expected"]["after"]["meshes"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    let got = &e.scene.active().unwrap().mesh;
    let want_points = points(&want["points"]);
    // F6's click point is recorded to 3 decimals.
    let tol = if id == "F6" { 1e-3 } else { 1e-6 };
    if got
        .vertices
        .iter()
        .zip(&want_points)
        .any(|(a, b)| (*a - *b).length() > tol)
    {
        return Err(format!("points {:?}", got.vertices));
    }
    Ok(())
}

#[test]
fn every_mode_scales_about_its_centre() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
