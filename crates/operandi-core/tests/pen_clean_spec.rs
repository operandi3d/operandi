//! Pen against `specifications/pen-2026-11-05`: points entered in order, then
//! Drop. Positions, counts, polygon types and corner order up to rotation.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "pen-2026-11-05";

/// Not applicable or open.
const SKIP: &[(&str, &str)] = &[
    (
        "PN04-2",
        "Spline Patch: Operandi has no spline patch polygons yet",
    ),
    (
        "PN04-7",
        "an out-of-range type index; Operandi's types are a fixed list",
    ),
];

fn options(s: &Value) -> PenOptions {
    PenOptions {
        output: match s["type"].as_str().unwrap() {
            "vertices" => PenType::Vertices,
            "subdiv" => PenType::Subdivs,
            "psubdiv" => PenType::CatmullClark,
            "polyline" => PenType::Polyline,
            "bspline" => PenType::BSpline,
            _ => PenType::Polygon,
        },
        flip: s["flip"] == 1,
        close: s["close"] == 1,
        select_new: true,
        wall: match s["wall"].as_str().unwrap() {
            "inner" => WallMode::Inner,
            "outer" => WallMode::Outer,
            "both" => WallMode::BothSides,
            _ => WallMode::Off,
        },
        offset: s["offset"].as_f64().unwrap(),
        inset: s["inset"].as_f64().unwrap(),
        segments: s["segs"].as_u64().unwrap() as u32,
        make_uvs: s["uvs"] == 1,
        uv_view: None,
        plane_normal: DVec3::Z,
        // Numeric Make Quads with four points is the plain quad in entry
        // order (PN06), not the click strip's pairs.
        make_quads: false,
    }
}

fn type_name(r: Option<&PolygonRepresentation>) -> &'static str {
    match r {
        None | Some(PolygonRepresentation::Face) => "face",
        Some(PolygonRepresentation::Subdivs) => "subdiv",
        Some(PolygonRepresentation::CatmullClark) => "psubdiv",
        Some(PolygonRepresentation::Patch) => "patch",
        Some(PolygonRepresentation::Polyline { curve, .. }) => match curve {
            operandi_core::curves::CurveKind::BSpline => "BSpline",
            _ => "line",
        },
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::PenStart).map_err(|x| x.to_string())?;
    e.dispatch(Command::PenOptions(options(&c["settings"])))
        .map_err(|x| x.to_string())?;
    for p in c["entered_points"].as_array().unwrap() {
        e.dispatch(Command::PenCapture(point(p)))
            .map_err(|x| x.to_string())?;
    }
    e.dispatch(Command::Drop).map_err(|x| x.to_string())?;
    let got = &e.scene.active().unwrap().mesh;
    let want = &c["expected"];
    same_mesh(got, want, 1e-5)?;
    let want_mesh = mesh(want);
    let matches = polygon_matches(got, &want_mesh, 1e-5)?;
    for (i, t) in want["polygon_types"].as_array().unwrap().iter().enumerate() {
        let (j, _) = matches[i];
        let name = type_name(got.attributes.representations.get(j));
        if name != t.as_str().unwrap() {
            return Err(format!("polygon {i} type {name}, expected {t}"));
        }
    }
    if let Some(rows) = want["uv_per_polygon_corner"].as_array() {
        let map = got.uv_maps.get("Texture").ok_or("no Texture map")?;
        for (i, row) in rows.iter().enumerate() {
            let (j, r) = matches[i];
            let row = row.as_array().unwrap();
            for (k, uv) in row.iter().enumerate() {
                let g = map[j][(k + r) % row.len()].ok_or("missing UV")?;
                let w = DVec2::new(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap());
                if (g - w).length() > 1e-5 {
                    return Err(format!("polygon {i} corner {k} UV {g}, expected {w}"));
                }
            }
        }
    }
    Ok(())
}

#[test]
#[cfg(operandi_specs)]
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

/// Decided (Q-PEN-06): Pen starts with an empty point list.
#[test]
fn a_new_pen_session_starts_empty() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.synchronize_identities().unwrap();
    e.dispatch(Command::PenStart).unwrap();
    e.dispatch(Command::PenCapture(DVec3::X)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::PenStart).unwrap();
    assert!(e.session.pen_draft.as_ref().unwrap().points().is_empty());
    let d = PenOptions::default();
    assert_eq!(
        (d.output, d.flip, d.close, d.wall),
        (PenType::Polygon, false, false, WallMode::Off)
    );
    assert_eq!(
        (d.offset, d.inset, d.segments, d.make_uvs),
        (0., 0., 10, false)
    );
}
