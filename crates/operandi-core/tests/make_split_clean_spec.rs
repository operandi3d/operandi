//! Make Polygon and Split Polygon against
//! `specifications/make-split-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "make-split-2026-11-05";

/// Not checked as measured.
const SKIP: &[(&str, &str)] = &[
    (
        "MP11b",
        "Spline Patch: Operandi has no spline patch polygons",
    ),
    ("MP06", "one-point polygons: Operandi doesn't have them yet"),
    (
        "MP18",
        "Q-MS-02: two separate edges (Operandi keeps its quad)",
    ),
    (
        "MP09",
        "Close Curve: Operandi closes the curve with a flag, not repeated points",
    ),
    // Decided Q-MS-03: 3+ points on one polygon cut every consecutive pair.
    ("SP04", "decided Q-MS-03 (tested below)"),
    ("SP10", "decided Q-MS-03"),
    ("SP13", "decided Q-MS-03"),
];

fn kind(option: &Value) -> (MakeType, bool) {
    let o = option.as_array().unwrap();
    let t = match o[0].as_str().unwrap() {
        "face" => MakeType::Face,
        "subdiv" | "patch" => MakeType::Subdivs,
        "psubdiv" => MakeType::CatmullClark,
        "curve" => MakeType::Curve,
        "line" => MakeType::Polyline,
        _ => MakeType::Auto,
    };
    (t, o.get(1).is_some_and(|v| v == "true"))
}

fn type_name(m: &Mesh, f: usize) -> &'static str {
    if m.faces[f].vertices.len() == 1 {
        return "onePoint";
    }
    match m.attributes.representations.get(f) {
        None | Some(PolygonRepresentation::Face) => "face",
        Some(PolygonRepresentation::Subdivs) => "subdiv",
        Some(PolygonRepresentation::CatmullClark) => "psubdiv",
        Some(PolygonRepresentation::Patch) => "patch",
        Some(PolygonRepresentation::Polyline { curve, .. }) => match curve {
            operandi_core::curves::CurveKind::Line => "line",
            _ => "curve",
        },
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let id = c["id"].as_str().unwrap();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let mut source = mesh(&c["input"]);
    if let Some(types) = c["input"]["polygon_types"].as_array() {
        source.attributes.representations = types
            .iter()
            .map(|t| match t.as_str().unwrap() {
                "subdiv" => PolygonRepresentation::Subdivs,
                "psubdiv" => PolygonRepresentation::CatmullClark,
                _ => PolygonRepresentation::Face,
            })
            .collect();
    }
    if id == "MP19" {
        // "points on a subdiv-tagged open mesh": the input polygons are Subdivs.
        source.attributes.representations =
            vec![PolygonRepresentation::Subdivs; source.faces.len()];
    }
    e.scene.active_mut().unwrap().mesh = source.clone().into();
    e.synchronize_identities().unwrap();
    let mode = if c["selection_mode"] == "edge" {
        Mode::Edge
    } else {
        Mode::Vertex
    };
    e.dispatch(Command::Mode(mode)).unwrap();
    let sel = &c["selection"];
    e.scene.selection.vertices = indices(&sel["sel_vertices"]);
    e.scene.selection.faces = indices(&sel["sel_polygons"]);
    e.scene.selection.edges = sel["sel_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            let x = indices(x);
            (x[0], x[1])
        })
        .collect();
    let want = &c["expected"];
    let result = if id.starts_with("SP") {
        e.dispatch(Command::SplitPolygon)
    } else {
        let (t, close) = kind(&c["option"]);
        e.dispatch(Command::MakePolygonAs { kind: t, close })
    };
    let failed = want["failed"].as_bool().unwrap_or(false);
    match (failed, result) {
        (true, Err(_)) => return Ok(()),
        (true, Ok(())) => return Err("expected a refusal".into()),
        (false, Err(x)) => return Err(format!("refused: {x}")),
        (false, Ok(())) => {}
    }
    let got = &e.scene.active().unwrap().mesh;
    same_mesh(got, want, 1e-5)?;
    let want_mesh = mesh(want);
    let matches = polygon_matches(got, &want_mesh, 1e-5)?;
    if let Some(types) = want["polygon_types"].as_array() {
        for (i, t) in types.iter().enumerate() {
            let name = type_name(got, matches[i].0);
            if name != t.as_str().unwrap() {
                return Err(format!("polygon {i} type {name}, expected {t}"));
            }
        }
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

/// Decided (Q-MS-03): three or more points on one polygon cut between each
/// consecutive pair in selection order (SP04's hexagon, points 0, 2, 4).
#[test]
fn split_cuts_each_consecutive_pair() {
    let all = cases(SPEC);
    let c = case(&all, "SP04");
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh(&c["input"]).into();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Mode(Mode::Vertex)).unwrap();
    e.scene.selection.vertices = vec![0, 2, 4];
    e.dispatch(Command::SplitPolygon).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    // Two cuts: 0–2 and 2–4 leave triangles [0,1,2], [2,3,4] and [0,2,4,5].
    assert_eq!(m.faces.len(), 3);
    let edges = m.edges();
    assert!(edges.contains(&(0, 2)) && edges.contains(&(2, 4)));
}
