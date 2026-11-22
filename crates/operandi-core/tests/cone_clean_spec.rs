//! Cone against `specifications/cone-2026-11-05`.
#[path = "support/clean_spec.rs"]
mod clean_spec;

use clean_spec::*;
use glam::{DVec2, DVec3};
use operandi_core::*;
use serde_json::Value;

const SPEC: &str = "cone-2026-11-05";

/// Cases whose rule the spec leaves open; not checked.
const OPEN: &[(&str, &str)] = &[("CN03a", "Q-CN-04: Bulge"), ("CN03b", "Q-CN-04: Bulge")];

/// Cases whose UVs follow an open rule; geometry is still checked.
const OPEN_UVS: &[(&str, &str)] = &[("CN08", "Q-CN-05: Distortion Compensation")];

fn cap(s: &str) -> ConeCap {
    match s {
        "None" => ConeCap::None,
        "Single Polygon" | "Single" => ConeCap::Single,
        "Quad Grid" => ConeCap::QuadGrid,
        "Radial" => ConeCap::Radial,
        other => panic!("cap {other}"),
    }
}

fn params(s: &Value) -> ConeParameters {
    let mut p = ConeParameters::default();
    let f = |k: &str| s[k].as_f64();
    let xyz = |a: &str, b: &str, c: &str, d: DVec3| {
        DVec3::new(
            f(a).unwrap_or(d.x),
            f(b).unwrap_or(d.y),
            f(c).unwrap_or(d.z),
        )
    };
    p.position = xyz("Position X", "Position Y", "Position Z", p.position);
    p.size = xyz("Size X", "Size Y", "Size Z", p.size);
    if let Some(n) = s["Sides"].as_u64() {
        p.sides = n as u32;
    }
    if let Some(m) = s["Segments"].as_u64() {
        p.segments = m as u32;
    }
    if let Some(a) = s["Axis"].as_str() {
        p.axis = "XYZ".find(a).unwrap();
    }
    if let Some(t) = s["Polygon Type"].as_str() {
        p.polygon = match t {
            "Face" => PolygonRepresentation::Face,
            "Subdivs" => PolygonRepresentation::Subdivs,
            _ => PolygonRepresentation::CatmullClark,
        };
    }
    if let Some(c) = s["Bottom Cap"].as_str() {
        p.bottom_cap = cap(c);
    }
    if let Some(c) = s["Top Cap"].as_str() {
        p.top_cap = cap(c);
    }
    if let Some(w) = f("Edge Weight (%)") {
        p.edge_weight = w;
    }
    if let Some(b) = s["Make UVs"].as_bool() {
        p.make_uvs = b;
    }
    if let Some(b) = s["Distortion Compensation"].as_bool() {
        p.compensation = b;
    }
    if let Some(g) = f("Gaps (%)") {
        p.gaps = g;
    }
    p
}

fn uv(v: &Value) -> DVec2 {
    DVec2::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap())
}

fn run(c: &Value) -> std::result::Result<(), String> {
    let id = c["id"].as_str().unwrap();
    let got = cone_geometry(&params(&c["settings"])).map_err(|e| format!("refused: {e}"))?;
    let e = &c["expected"];
    let tol = tolerance(c);
    same_mesh(&got, e, tol)?;
    let want = mesh(e);
    let matches = polygon_matches(&got, &want, tol)?;
    if let Some(types) = e["polygon_types"].as_array() {
        let kind = |r: Option<&PolygonRepresentation>| match r {
            None | Some(PolygonRepresentation::Face) => "face",
            Some(PolygonRepresentation::Subdivs) => "subdiv",
            Some(PolygonRepresentation::CatmullClark) => "psubdiv",
            Some(_) => "other",
        };
        for (i, &(j, _)) in matches.iter().enumerate() {
            let want_type = types.get(i).or(types.first()).unwrap();
            if kind(got.attributes.representations.get(j)) != want_type {
                return Err(format!("polygon {i} type, expected {want_type}"));
            }
        }
    }
    if let Some(checked) = e["edge_weights_checked"].as_array() {
        let nonzero = e["edge_weights_nonzero"].as_object().unwrap();
        for name in checked {
            let ends: Vec<u32> = name
                .as_str()
                .unwrap()
                .split('-')
                .map(|x| x.parse().unwrap())
                .collect();
            let (a, b) = (
                want.vertices[ends[0] as usize],
                want.vertices[ends[1] as usize],
            );
            let find = |q: DVec3| {
                got.vertices
                    .iter()
                    .position(|&g| (g - q).abs().max_element() <= tol)
                    .unwrap() as u32
            };
            let (ga, gb) = (find(a), find(b));
            let weight = got
                .attributes
                .creases
                .iter()
                .find(|k| (k.a, k.b) == (ga.min(gb), ga.max(gb)))
                .map_or(0., |k| k.weight);
            let want_weight = nonzero
                .get(name.as_str().unwrap())
                .and_then(Value::as_f64)
                .unwrap_or(0.);
            if (weight - want_weight).abs() > 1e-6 {
                return Err(format!(
                    "edge {name} weight {weight}, expected {want_weight}"
                ));
            }
        }
    }
    if OPEN_UVS.iter().any(|(o, _)| *o == id) {
        return Ok(());
    }
    if let Some(maps) = e["uv_per_corner"].as_object() {
        for (name, rows) in maps {
            let got_rows = got
                .uv_maps
                .get(name)
                .ok_or_else(|| format!("no UV map {name}"))?;
            for (i, row) in rows.as_array().unwrap().iter().enumerate() {
                let (j, r) = matches[i];
                let row = row.as_array().unwrap();
                for (k, want_uv) in row.iter().enumerate() {
                    let g = got_rows[j][(k + r) % row.len()]
                        .ok_or_else(|| format!("polygon {i} corner {k} has no UV"))?;
                    if (g - uv(want_uv)).abs().max_element() > 1e-5 {
                        return Err(format!(
                            "polygon {i} corner {k} UV {g}, expected {}",
                            uv(want_uv)
                        ));
                    }
                }
            }
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

#[test]
fn reset_defaults() {
    let p = ConeParameters::default();
    assert_eq!(p.position, DVec3::ZERO);
    assert_eq!(p.size, DVec3::new(0.5, 0.75, 0.5));
    assert_eq!((p.sides, p.segments, p.axis), (24, 1, 1));
    assert_eq!(
        (p.top_cap, p.bottom_cap),
        (ConeCap::Single, ConeCap::Single)
    );
    assert_eq!(p.polygon, PolygonRepresentation::Face);
    assert_eq!(
        (p.edge_weight, p.make_uvs, p.compensation, p.gaps),
        (100., true, false, 0.)
    );
}
