//! Set Position against `specifications/set-position-2026-11-05`: local,
//! world and work-plane coordinates, and which points each mode affects.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "set-position-2026-11-05";

/// Not checked as measured.
const SKIP: &[(&str, &str)] = &[(
    "ST15",
    "a rotated work plane: Operandi's work plane is axis-aligned",
)];

fn vec3(v: &Value) -> DVec3 {
    let a = v.as_array().unwrap();
    DVec3::new(
        a[0].as_f64().unwrap(),
        a[1].as_f64().unwrap(),
        a[2].as_f64().unwrap(),
    )
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let setup = &c["setup"];
    {
        let item = e.scene.active_mut().unwrap();
        item.mesh = mesh(&c["input"]).into();
        if !setup["item_position"].is_null() {
            item.transform.translation = vec3(&setup["item_position"]);
        }
        if !setup["item_rotation_deg"].is_null() {
            item.transform.rotation =
                vec3(&setup["item_rotation_deg"]) * (std::f64::consts::PI / 180.);
        }
        if !setup["item_scale"].is_null() {
            item.transform.scale = vec3(&setup["item_scale"]);
        }
    }
    if !setup["work_plane"].is_null() {
        e.scene.workplane.center = vec3(&setup["work_plane"]["center"]);
    }
    e.synchronize_identities().unwrap();
    let mode = match c["selection_mode"].as_str().unwrap() {
        "vertex" => Mode::Vertex,
        "edge" => Mode::Edge,
        "item" => Mode::Item,
        _ => Mode::Polygon,
    };
    e.dispatch(Command::Mode(mode)).unwrap();
    let sel = &c["selection"];
    e.scene.selection.faces = indices(&sel["sel_polygons"]);
    e.scene.selection.vertices = indices(&sel["sel_vertices"]);
    e.scene.selection.edges = sel["sel_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            let x = indices(x);
            (x[0], x[1])
        })
        .collect();
    let before = e.scene.selection.clone();
    let o = &c["option"];
    let axis = match o["axis"].as_str().unwrap() {
        "x" => PositionAxis::X,
        "y" => PositionAxis::Y,
        _ => PositionAxis::Z,
    };
    let space = PositionSpace::from_switches(
        o["workPlane"].as_bool().unwrap(),
        o["worldSpace"].as_bool().unwrap(),
    )?;
    e.dispatch(Command::SetPosition(PositionAssignment {
        axis,
        value: o["value"].as_f64().unwrap(),
        space,
        expected: None,
    }))
    .map_err(|x| format!("refused: {x}"))?;
    let got = &e.scene.active().unwrap().mesh;
    let want = points(&c["expected"]["points"]);
    if got.vertices.len() != want.len()
        || got
            .vertices
            .iter()
            .zip(&want)
            .any(|(a, b)| (*a - *b).length() > 1e-9)
    {
        return Err(format!("points {:?}", got.vertices));
    }
    let polygons: Vec<Vec<u32>> = got.faces.iter().map(|f| f.vertices.clone()).collect();
    let want_polygons: Vec<Vec<u32>> = c["expected"]["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(indices)
        .collect();
    if polygons != want_polygons {
        return Err("polygons changed".into());
    }
    if e.scene.selection != before {
        return Err("selection changed".into());
    }
    Ok(())
}

#[test]
fn every_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        let id = c["id"].as_str().unwrap();
        if SKIP.iter().any(|(s, _)| *s == id) {
            continue;
        }
        if let Err(e) = run(&c) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Provisional (Q-ST-01): both switches on is refused.
#[test]
fn both_switches_are_refused() {
    assert!(PositionSpace::from_switches(true, true).is_err());
}

/// Cancel-free undo: one Set Position is one undo step.
#[test]
fn undo_restores_points() {
    let all = cases(SPEC);
    let c = case(&all, "ST01");
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![0, 1, 2];
    let source = e.scene.active().unwrap().mesh.vertices.clone();
    e.dispatch(Command::SetPosition(PositionAssignment {
        axis: PositionAxis::Y,
        value: 0.5,
        space: PositionSpace::Local,
        expected: None,
    }))
    .unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.vertices, source);
}
