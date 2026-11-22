//! Vertex Map falloff (spec vertex-map-tools-2026-11-11, VMF-01..13): the
//! measured cases in cases.json, through a 1 m Move along Y.
use operandi_core::*;
use serde_json::Value;
use std::collections::BTreeMap;

fn cases() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/vertex-map-tools-2026-11-11/cases.json"
    );
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    match v {
        Value::Array(a) => a,
        Value::Object(o) => o["cases"].as_array().unwrap().clone(),
        _ => panic!("cases.json"),
    }
}

fn vec3(v: &Value) -> glam::DVec3 {
    glam::DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

/// The falloff a case's stack describes.
fn layer(entry: &Value) -> (Falloff, MixMode) {
    let mix = match entry["Mix Mode"].as_str().unwrap_or("Multiply") {
        "Add" => MixMode::Add,
        "Subtract" => MixMode::Subtract,
        "Max" => MixMode::Max,
        "Min" => MixMode::Min,
        _ => MixMode::Multiply,
    };
    let f = match entry["falloff"].as_str().unwrap() {
        "Vertex Map" => Falloff::VertexMap(VertexMapFalloff {
            mode: if entry["Mode"].as_str() == Some("Component") {
                VertexMapMode::Component
            } else {
                VertexMapMode::Magnitude
            },
            index: entry["Index"].as_u64().unwrap_or(0) as u32,
        }),
        "Invert" => Falloff::Invert,
        "Linear" => {
            let start = entry.get("Start").map(vec3).unwrap_or(glam::DVec3::ZERO);
            let end = entry
                .get("End")
                .map(vec3)
                .unwrap_or(glam::DVec3::new(8., 0., 0.));
            Falloff::Linear(LinearFalloff {
                start,
                end,
                ..LinearFalloff::default()
            })
        }
        other => panic!("unknown falloff {other}"),
    };
    (f, mix)
}

#[test]
fn measured_cases_match() {
    let mut ran = 0;
    for case in cases() {
        let id = case["id"].as_str().unwrap().to_owned();
        let Some(expected) = case["expected"].get("weight_per_point") else {
            continue; // the shading-colour case
        };
        let input = &case["input"];
        let points: Vec<glam::DVec3> = input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect();
        let mut mesh = Mesh {
            vertices: points,
            ..Default::default()
        };
        for p in input["polygons"].as_array().unwrap() {
            mesh.faces.push(Face {
                vertices: p
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect(),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
        for (name, values) in input["weight_maps"].as_object().unwrap() {
            let map: BTreeMap<u32, f64> = values
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter_map(|(i, v)| v.as_f64().map(|w| (i as u32, w)))
                .collect();
            mesh.attributes.point_weights.insert(name.clone(), map);
        }
        let settings = &case["settings"];
        let selected = settings["selected_map"].as_str().map(str::to_owned);
        let stack: Vec<(Falloff, MixMode)> = settings["falloff_stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(layer)
            .collect();
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = mesh.into();
        let id_item = e.scene.active().unwrap().id;
        e.dispatch(Command::SpatialFalloff(Some(stack[0].0)))
            .unwrap();
        e.dispatch(Command::AddedFalloffs(stack[1..].to_vec()))
            .unwrap();
        e.dispatch(Command::SelectWeightMap(selected)).unwrap();
        let w = e
            .active_falloff()
            .unwrap()
            .weights(&e.scene, id_item)
            .unwrap();
        let want: Vec<f64> = expected
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let tol = case["tolerance"].as_f64().unwrap_or(1e-5);
        assert_eq!(w.len(), want.len(), "{id}");
        for (i, (a, b)) in w.iter().zip(&want).enumerate() {
            assert!((a - b).abs() <= tol, "{id} point {i}: {a} vs {b}");
        }
        ran += 1;
    }
    assert!(ran >= 14, "ran {ran} cases");
}

#[test]
fn picking_another_type_clears_the_stack_and_editing_keeps_it() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let vm = Falloff::VertexMap(VertexMapFalloff::default());
    e.dispatch(Command::SpatialFalloff(Some(vm))).unwrap();
    e.dispatch(Command::AddedFalloffs(vec![(
        Falloff::Invert,
        MixMode::Multiply,
    )]))
    .unwrap();
    // Editing the Vertex Map falloff keeps the Invert on top.
    let edited = Falloff::VertexMap(VertexMapFalloff {
        mode: VertexMapMode::Component,
        index: 1,
    });
    e.dispatch(Command::SpatialFalloff(Some(edited))).unwrap();
    assert_eq!(e.added_falloffs.len(), 1);
    // Picking Linear replaces the whole stack (FAL-M-01).
    e.dispatch(Command::SpatialFalloff(Some(
        LinearFalloff::default().into(),
    )))
    .unwrap();
    assert!(e.added_falloffs.is_empty());
    // Index in Magnitude mode is refused (VMF-01).
    let bad = Falloff::VertexMap(VertexMapFalloff {
        mode: VertexMapMode::Magnitude,
        index: 1,
    });
    assert!(e.dispatch(Command::SpatialFalloff(Some(bad))).is_err());
    // Adding with no falloff is refused.
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    assert!(
        e.dispatch(Command::AddedFalloffs(vec![(
            Falloff::Invert,
            MixMode::Add
        )]))
        .is_err()
    );
}

/// VM-40: a painted map under Vertex Map then Invert protects the painted
/// points from a sculpt stroke; unpainted points sculpt as usual.
#[test]
fn a_painted_map_masks_sculpting() {
    use operandi_core::sculpt::{Dab, Stroke, StrokeSetup};
    let n = 20u32;
    let mut mesh = Mesh::default();
    for j in 0..=n {
        for i in 0..=n {
            mesh.vertices.push(glam::DVec3::new(
                i as f64 / n as f64 - 0.5,
                j as f64 / n as f64 - 0.5,
                0.,
            ));
        }
    }
    for j in 0..n {
        for i in 0..n {
            let a = j * (n + 1) + i;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + n + 2, a + n + 1],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    // Paint the left half fully (x < 0).
    let mask: BTreeMap<u32, f64> = mesh
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, p)| p.x < -1e-9)
        .map(|(i, _)| (i as u32, 1.))
        .collect();
    mesh.attributes.point_weights.insert("Mask".into(), mask);
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::SpatialFalloff(Some(Falloff::VertexMap(
        VertexMapFalloff::default(),
    ))))
    .unwrap();
    e.dispatch(Command::AddedFalloffs(vec![(
        Falloff::Invert,
        MixMode::Multiply,
    )]))
    .unwrap();
    e.dispatch(Command::SelectWeightMap(Some("Mask".into())))
        .unwrap();
    let brush = SculptBrush::Push;
    let setup = StrokeSetup {
        brush,
        settings: brush.defaults(),
        ctrl: false,
        shift: false,
        symmetry: Symmetry::OFF,
        falloff: e.active_falloff(),
        lock_borders: false,
        lock_corners: true,
        radius: 0.2,
        weight_map: None,
    };
    let mut scene = e.scene.clone();
    let mut s = Stroke::begin(&scene, &[id], setup).unwrap().unwrap();
    for _ in 0..20 {
        s.dab(Dab {
            center: glam::DVec3::ZERO,
            radius: 0.2,
            view: glam::DVec3::Z,
            normal: glam::DVec3::Z,
            pressure: 1.,
        });
    }
    s.flush(&mut scene);
    let z = |v: u32| scene.active().unwrap().mesh.vertices[v as usize].z;
    let row = 10 * (n + 1);
    assert_eq!(z(row + 8), 0., "painted (left) points are protected");
    assert!(z(row + 12) > 1e-3, "unpainted (right) points sculpt");
}
