//! Ellipsoid against `specifications/ellipsoid-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "ellipsoid-2026-11-05";

/// UVs whose rule is open; geometry still checked.
const OPEN_UVS: &[(&str, &str)] = &[("EL13", "Q-EL-02: Distortion Compensation range")];

fn params(s: &Value) -> EllipsoidParameters {
    let f = |k: &str| s[k].as_f64().unwrap();
    EllipsoidParameters {
        position: DVec3::new(f("cenX"), f("cenY"), f("cenZ")),
        radius: DVec3::new(f("sizeX"), f("sizeY"), f("sizeZ")),
        sides: s["sides"].as_u64().unwrap() as u32,
        segments: s["segments"].as_u64().unwrap() as u32,
        polygon: match s["polType"].as_str().unwrap() {
            "subdiv" => PolygonRepresentation::Subdivs,
            "psubdiv" => PolygonRepresentation::CatmullClark,
            _ => PolygonRepresentation::Face,
        },
        bulge_top: f("bulgeTop"),
        bulge_side: f("bulgeSide"),
        axis: s["axis"].as_u64().unwrap() as usize,
        make_uvs: s["uvs"] == 1,
        compensation: s["uvCompensation"] == 1,
        gaps: f("uvGaps") * 100.,
        ..Default::default()
    }
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let id = c["id"].as_str().unwrap();
    let got = ellipsoid_geometry(&params(&c["settings"])).map_err(|e| format!("refused: {e}"))?;
    let e = &c["expected"];
    same_mesh(&got, e, 1e-5)?;
    let want = mesh(e);
    let matches = polygon_matches(&got, &want, 1e-5)?;
    for (i, t) in e["polygon_types"].as_array().unwrap().iter().enumerate() {
        let name = match got.attributes.representations.get(matches[i].0) {
            None | Some(PolygonRepresentation::Face) => "face",
            Some(PolygonRepresentation::Subdivs) => "subdiv",
            Some(PolygonRepresentation::CatmullClark) => "psubdiv",
            Some(_) => "other",
        };
        if name != t.as_str().unwrap() {
            return Err(format!("polygon {i} type {name}"));
        }
    }
    if OPEN_UVS.iter().any(|(o, _)| *o == id) {
        return Ok(());
    }
    let Some(rows) = e["uv_per_polygon_corner"].as_array() else {
        return Ok(());
    };
    let map = got.uv_maps.get("Texture").ok_or("no Texture map")?;
    let top = (want.vertices.len() - 1) as u32;
    for (i, row) in rows.iter().enumerate() {
        let (j, r) = matches[i];
        let mut row: Vec<DVec2> = row
            .as_array()
            .unwrap()
            .iter()
            .map(|uv| DVec2::new(uv[0].as_f64().unwrap(), uv[1].as_f64().unwrap()))
            .collect();
        // Decided (Q-EL-01): the top fan uses the plain layout, not the
        // reference's swapped u on its two ring corners (rule 11).
        if want.faces[i].vertices.first() == Some(&top) {
            let (a, b) = (row[1].x, row[2].x);
            row[1].x = b;
            row[2].x = a;
        }
        for (k, w) in row.iter().enumerate() {
            let g = map[j][(k + r) % row.len()].ok_or("missing UV")?;
            if (g - *w).length() > 1e-5 {
                return Err(format!("polygon {i} corner {k} UV {g}, expected {w}"));
            }
        }
    }
    Ok(())
}

#[test]
fn every_case_matches() {
    let mut failures = Vec::new();
    for c in cases(SPEC) {
        if let Err(e) = run(&c) {
            failures.push(format!("{}: {e}", c["id"].as_str().unwrap()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn reset_defaults() {
    let p = EllipsoidParameters::default();
    assert_eq!(
        (p.radius, p.sides, p.segments, p.axis),
        (DVec3::splat(0.5), 24, 12, 1)
    );
    assert_eq!(
        (
            p.bulge_top,
            p.bulge_side,
            p.make_uvs,
            p.compensation,
            p.gaps
        ),
        (0., 0., true, false, 0.)
    );
}
