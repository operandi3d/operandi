//! Polygon Axis Slice oracle cases (`docs/specs/edit-wave3/cases-wave3.json`,
//! 17 cases: a flat 4 x 2 m quad, an off-centre quad and a 2 m cube).
//! Points compare by position to 1e-4, order-free; polygons as point-position
//! cycles; the selection afterwards as the set of selected polygons.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

const TOLERANCE: f64 = 1e-4;

fn vec3(v: &Value) -> DVec3 {
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

fn points(v: &Value) -> Vec<DVec3> {
    v.as_array().unwrap().iter().map(vec3).collect()
}

fn indices(v: &Value) -> Vec<usize> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap() as usize)
        .collect()
}

fn polygons(v: &Value) -> Vec<Vec<usize>> {
    v.as_array().unwrap().iter().map(indices).collect()
}

/// The case's settings, from its id (the file's settings_key: numX n =
/// Number X; size X s = Size mode, other sizes off; off = Offset X).
fn settings(id: &str) -> AxisSlice {
    let number = |n: [u32; 3]| AxisSlice {
        number: n,
        ..Default::default()
    };
    let size = |s: f64, off: f64| AxisSlice {
        mode: AxisSliceMode::Size,
        size: DVec3::new(s, 100., 100.),
        offset: DVec3::new(off, 0., 0.),
        ..Default::default()
    };
    let shifted = |n: [u32; 3], off: f64| AxisSlice {
        offset: DVec3::new(off, 0., 0.),
        ..number(n)
    };
    match id {
        "cube_all_numXYZ1" => number([1, 1, 1]),
        "cube_top_numX1" | "quad2_numX1" | "quad_nosel_numX1" | "quad_numX1" => number([1, 0, 0]),
        "quad2_size1" | "quad_size1.0" => size(1., 0.),
        "quad_numX0" => AxisSlice::default(),
        "quad_numX1_off25" => shifted([1, 0, 0], 2.5),
        "quad_numX2" => number([2, 0, 0]),
        "quad_numX2_numZ1" => number([2, 0, 1]),
        "quad_numX2_off03" => shifted([2, 0, 0], 0.3),
        "quad_numX3" => number([3, 0, 0]),
        "quad_size0.7" => size(0.7, 0.),
        "quad_size1.5" => size(1.5, 0.),
        "quad_size1_off03" => size(1., 0.3),
        "quad_size_default" => AxisSlice {
            mode: AxisSliceMode::Size,
            ..Default::default()
        },
        other => panic!("no settings for case {other}"),
    }
}

fn same(a: DVec3, b: DVec3) -> bool {
    a.distance(b) < TOLERANCE
}

fn same_cycle(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len()
        && (0..b.len()).any(|shift| (0..a.len()).all(|i| same(a[i], b[(i + shift) % b.len()])))
}

/// Every cycle of `want` matched once in `got`.
fn same_cycles(got: &[Vec<DVec3>], want: &[Vec<DVec3>]) -> bool {
    let mut left: Vec<&Vec<DVec3>> = got.iter().collect();
    for w in want {
        match left.iter().position(|g| same_cycle(g, w)) {
            Some(i) => {
                left.remove(i);
            }
            None => return false,
        }
    }
    left.is_empty()
}

fn check(case: &Value) -> std::result::Result<(), String> {
    let id = case["id"].as_str().unwrap();
    let mut mesh = Mesh {
        vertices: points(&case["input"]["points"]),
        ..Default::default()
    };
    for p in polygons(&case["input"]["polygons"]) {
        mesh.faces.push(Face {
            vertices: p.into_iter().map(|i| i as u32).collect(),
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    e.scene.selection.faces = indices(&case["input"]["selected_polygons"])
        .into_iter()
        .map(|i| i as u32)
        .collect();
    e.dispatch(Command::Activate(Tool::AxisSlice))
        .map_err(|err| format!("{id}: {err}"))?;
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    *p.slice.axis_slice = settings(id);
    e.dispatch(Command::Preview(p))
        .map_err(|err| format!("{id}: {err}"))?;
    let out = &e.scene.active().unwrap().mesh;
    let got_points = &out.vertices;
    let mut want_points = points(&case["expected"]["points"]);
    // W3-AX-Q1: Operandi wraps each cut round the box's length (Jason's
    // hands-on check), so Offset 2.5 on this 4 m quad puts the cut at
    // -1.5, where the recording has 0.5. Same cut, other place.
    if id == "quad_numX1_off25" {
        for p in &mut want_points {
            if (p.x - 0.5).abs() < 1e-9 {
                p.x = -1.5;
            }
        }
    }
    if got_points.len() != want_points.len()
        || !want_points
            .iter()
            .all(|&p| got_points.iter().any(|&q| same(p, q)))
    {
        return Err(format!(
            "{id}: {} points, want {}",
            got_points.len(),
            want_points.len()
        ));
    }
    let cycle =
        |vs: &[usize], pts: &[DVec3]| -> Vec<DVec3> { vs.iter().map(|&v| pts[v]).collect() };
    let got: Vec<Vec<DVec3>> = out
        .faces
        .iter()
        .map(|f| f.vertices.iter().map(|&v| got_points[v as usize]).collect())
        .collect();
    let want_polygons = polygons(&case["expected"]["polygons"]);
    let want: Vec<Vec<DVec3>> = want_polygons
        .iter()
        .map(|p| cycle(p, &want_points))
        .collect();
    if !same_cycles(&got, &want) {
        return Err(format!(
            "{id}: polygons differ ({} vs {})",
            got.len(),
            want.len()
        ));
    }
    // The selection afterwards (AX-10).
    let got_selected: Vec<Vec<DVec3>> = e
        .scene
        .selection
        .faces
        .iter()
        .map(|&f| got[f as usize].clone())
        .collect();
    let want_selected: Vec<Vec<DVec3>> = indices(&case["expected"]["selected_polygons"])
        .into_iter()
        .map(|i| want[i].clone())
        .collect();
    if !same_cycles(&got_selected, &want_selected) {
        return Err(format!(
            "{id}: {} selected, want {}",
            got_selected.len(),
            want_selected.len()
        ));
    }
    Ok(())
}

#[test]
fn wave3_oracle_cases() {
    let path = std::path::Path::new(env!("OPERANDI_SPECS_DIR"))
        .join("docs/specs/edit-wave3/cases-wave3.json");
    let data: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 17);
    let failures: Vec<String> = cases.iter().filter_map(|c| check(c).err()).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
