//! Edge and Vertex Extrude against `specifications/extrude-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "extrude-2026-11-05";

/// Cases not checked as Modo measured them.
const OPEN: &[(&str, &str)] = &[
    // Q-EX-03 decided: border edges get a proper ridge, not Modo's cut.
    (
        "EE04",
        "border edge: decided to build a ridge (tested below)",
    ),
    (
        "EE07",
        "nothing selected includes border edges (tested below)",
    ),
    // Q-EX-03 stays open for single-polygon vertices; VE04 and VE08
    // follow the measured cut (provisional) and are checked like the rest.
];

fn edges(v: &Value) -> Vec<(u32, u32)> {
    v.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|e| {
            let e = indices(e);
            (e[0], e[1])
        })
        .collect()
}

struct Run {
    mesh: Mesh,
    selected_points: Vec<u32>,
    selected_edges: Vec<(u32, u32)>,
}

fn apply(c: &Value) -> std::result::Result<Run, String> {
    let source = mesh(&c["input"]);
    let s = &c["settings"];
    let (extrude, width) = (s["Extrude"].as_f64().unwrap(), s["Width"].as_f64().unwrap());
    let material = s["Use Material"]
        .as_bool()
        .unwrap()
        .then(|| s["Material Name"].as_str().unwrap().to_string());
    match c["tool"].as_str().unwrap() {
        "Edge Extrude" => {
            let (mesh, raised) = edge_extrude_with(
                &source,
                &edges(&c["selection"]["edges"]),
                extrude,
                width,
                material.as_deref(),
            )
            .map_err(|e| format!("refused: {e}"))?;
            let mut points: Vec<u32> = raised.iter().flat_map(|&(a, b)| [a, b]).collect();
            points.sort_unstable();
            points.dedup();
            Ok(Run {
                mesh,
                selected_points: points,
                selected_edges: raised,
            })
        }
        _ => {
            let (mesh, tips) = vertex_extrude_with(
                &source,
                &indices(&c["selection"]["vertices"]),
                extrude,
                width,
                material.as_deref(),
            )
            .map_err(|e| format!("refused: {e}"))?;
            Ok(Run {
                mesh,
                selected_points: tips,
                selected_edges: Vec::new(),
            })
        }
    }
}

fn positions(m: &Mesh, ids: &[u32]) -> Vec<Vec<glam::DVec3>> {
    let mut v: Vec<Vec<glam::DVec3>> = ids.iter().map(|&i| vec![m.vertices[i as usize]]).collect();
    v.sort_by(|a, b| a[0].to_array().partial_cmp(&b[0].to_array()).unwrap());
    v
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let r = apply(c)?;
    let tol = tolerance(c);
    let e = &c["expected"];
    same_mesh(&r.mesh, e, tol)?;
    let want = mesh(e);
    let after = &e["selection_after"];
    let want_points = if after["vertices"].is_null() && c["tool"] == "Vertex Extrude" {
        Vec::new()
    } else {
        indices(&after["vertices"])
    };
    // Selecting edges also reads back their points; an unchanged mesh keeps
    // only the edge selection.
    if !(c["tool"] == "Edge Extrude" && after["vertices"].is_null()) {
        same_polygons(
            &positions(&r.mesh, &r.selected_points),
            &positions(&want, &want_points),
            tol,
        )
        .map_err(|e| format!("selected points: {e}"))?;
    }
    let as_polygons = |m: &Mesh, list: &[(u32, u32)]| {
        list.iter()
            .map(|&(a, b)| vec![m.vertices[a as usize], m.vertices[b as usize]])
            .collect::<Vec<_>>()
    };
    let sym = |list: Vec<Vec<glam::DVec3>>| {
        // An edge matches either way round: a 2-corner cycle covers both.
        list
    };
    same_polygons(
        &sym(as_polygons(&r.mesh, &r.selected_edges)),
        &sym(as_polygons(&want, &edges(&after["edges"]))),
        tol,
    )
    .map_err(|e| format!("selected edges: {e}"))?;
    if let Some(names) = e["polygon_materials"].as_array() {
        let want_tagged: Vec<u32> = (0..names.len() as u32)
            .filter(|&i| names[i as usize] != "Default")
            .collect();
        let got_tagged: Vec<u32> = (0..r.mesh.faces.len() as u32)
            .filter(|&i| r.mesh.faces[i as usize].material != "Default")
            .collect();
        same_polygons(
            &polygon_positions(&r.mesh, &got_tagged),
            &polygon_positions(&want, &want_tagged),
            tol,
        )
        .map_err(|e| format!("tagged polygons: {e}"))?;
        let name = c["settings"]["Material Name"].as_str().unwrap();
        if got_tagged
            .iter()
            .any(|&f| r.mesh.faces[f as usize].material != name)
        {
            return Err("tagged polygons carry the wrong material".into());
        }
    }
    Ok(())
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

/// Q-EX-03 decided (2026-11-05): a border edge gets a proper ridge, with no
/// open sliver and no notch in the neighbour (EE04's input).
#[test]
fn border_edge_builds_a_closed_ridge() {
    let all = cases(SPEC);
    let c = case(&all, "EE04");
    let source = mesh(&c["input"]);
    let (out, raised) = edge_extrude_with(&source, &[(0, 1)], 0.1, 0.05, None).unwrap();
    out.validate().unwrap();
    // One ridge quad on the polygon's side, raised along its normal.
    assert_eq!(raised.len(), 1);
    let (a, b) = raised[0];
    let ridge = [out.vertices[a as usize], out.vertices[b as usize]];
    for q in ridge {
        assert!((q.y + 0.5).abs() < 1e-9 && (q.z - 0.1).abs() < 1e-9, "{q}");
    }
    // The cut polygon shrinks to y = -0.45 and stays in its plane.
    let cut = polygon_positions(&out, &[0]);
    assert!(
        cut[0]
            .iter()
            .all(|q| q.z.abs() < 1e-9 && q.y >= -0.45 - 1e-9)
    );
    // The neighbour keeps its shape: none of its corners moved off its
    // outline (a point added on its edge is not a notch).
    let neighbour = polygon_positions(&out, &[1]);
    assert!(neighbour[0].iter().any(|q| (q.x, q.y) == (0., -0.5)));
    assert!(
        neighbour[0]
            .iter()
            .all(|q| q.x >= -1e-9 && q.z.abs() < 1e-9)
    );
    // No open sliver: the only open edges are the strip's own outline, the
    // ridge edge and the ridge's end edges.
    let edges = out.edge_counts();
    let open = edges.values().filter(|&&n| n == 1).count();
    assert!(open <= 10, "{open} open edges");
}

/// EE07 with border edges built as ridges: the interior cross matches the
/// reference, every edge gets a ridge and the result is a valid mesh.
#[test]
fn nothing_selected_extrudes_every_edge() {
    let all = cases(SPEC);
    let c = case(&all, "EE07");
    let source = mesh(&c["input"]);
    let (out, raised) = edge_extrude_with(&source, &[], 0.1, 0.05, None).unwrap();
    out.validate().unwrap();
    assert_eq!(raised.len(), source.edges().len());
    // The cross's middle ridge point and the inset corners at the centre.
    let has = |x: f64, y: f64, z: f64| {
        out.vertices
            .iter()
            .any(|q| (q.x - x).abs() < 1e-9 && (q.y - y).abs() < 1e-9 && (q.z - z).abs() < 1e-9)
    };
    assert!(has(0., 0., 0.1));
    for (x, y) in [(0.05, 0.05), (-0.05, 0.05), (0.05, -0.05), (-0.05, -0.05)] {
        assert!(has(x, y, 0.), "({x}, {y})");
    }
    // Every original point became a ridge point; none is left in the plane.
    for v in &source.vertices {
        assert!(!has(v.x, v.y, 0.));
    }
}

#[test]
fn width_zero_and_nothing_selected_vertex_extrude() {
    let all = cases(SPEC);
    let grid = mesh(&case(&all, "VE02")["input"]);
    let (same, _) = vertex_extrude_with(&grid, &[4], 0.1, 0., None).unwrap();
    assert_eq!(same, grid);
    let (same, _) = edge_extrude_with(&grid, &[(1, 4)], 0.1, 0., None).unwrap();
    assert_eq!(same, grid);
}

/// EE07 / VE08 through the editor: with nothing selected both tools turn
/// on (they used to be refused), start with a Width, extrude everything on
/// Drop, and undo puts the grid back.
#[test]
fn nothing_selected_tools_start_and_apply() {
    let all = cases(SPEC);
    let grid = mesh(&case(&all, "VE08")["input"]);
    for (mode, tool) in [
        (Mode::Edge, Tool::EdgeExtrude),
        (Mode::Vertex, Tool::VertexExtrude),
    ] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = grid.clone().into();
        e.dispatch(Command::Mode(mode)).unwrap();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        assert!(p.inset > 0., "{tool:?} starts with a Width");
        p.shift = 0.1;
        p.inset = if tool == Tool::EdgeExtrude { 0.05 } else { 0.1 };
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        let out = &e.scene.active().unwrap().mesh;
        out.validate().unwrap();
        // Every interior or border point became a raised tip or ridge point.
        assert!(out.vertices.iter().any(|q| (q.z - 0.1).abs() < 1e-9));
        if tool == Tool::VertexExtrude {
            // The reference's mesh (VE08): corners cut, five tips selected.
            assert_eq!((out.vertices.len(), out.faces.len()), (29, 16));
            assert_eq!(e.scene.selection.vertices.len(), 5);
        }
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(*e.scene.active().unwrap().mesh, grid);
    }
}

/// VE04: a corner with one polygon is cut off; the point goes, nothing is
/// raised and nothing stays selected (Q-EX-03's vertex part is open; this
/// follows the measured result, provisional).
#[test]
fn lone_corner_is_cut_off() {
    let all = cases(SPEC);
    let grid = mesh(&case(&all, "VE04")["input"]);
    let (out, tips) = vertex_extrude_with(&grid, &[0], 0.1, 0.1, None).unwrap();
    out.validate().unwrap();
    assert!(tips.is_empty());
    assert_eq!((out.vertices.len(), out.faces.len()), (10, 4));
    assert!(out.vertices.iter().all(|q| q.z == 0.));
    assert!(!out.vertices.iter().any(|q| (q.x, q.y) == (-1., -1.)));
    // A lone corner next to a raised point: the raised tip keeps the right
    // number after the corner point is dropped.
    let (out, tips) = vertex_extrude_with(&grid, &[0, 4], 0.1, 0.1, None).unwrap();
    assert_eq!(tips.len(), 1);
    let tip = out.vertices[tips[0] as usize];
    assert!(
        (tip - glam::DVec3::new(0., 0., 0.1)).length() < 1e-9,
        "{tip}"
    );
}
