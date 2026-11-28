//! Polygon Bevel measured cases (specifications/polygon-bevel-2026-11-02).
//! Each oracle holds its input mesh, settings and the full expected result.
#![cfg_attr(not(operandi_specs), allow(dead_code, unused_imports))]
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

#[cfg(operandi_specs)]
fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/polygon-bevel-2026-11-02/ORACLES.json"
    ))
    .unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    doc["oracles"].as_array().unwrap().clone()
}
fn vec3(v: &Value) -> DVec3 {
    let a: Vec<f64> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    DVec3::new(a[0], a[1], a[2])
}
fn list(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect()
}
fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// Run one oracle; `Err` describes the first difference.
fn check(o: &Value) -> std::result::Result<(), String> {
    check_with(o, &mut |_| {})
}

fn check_with(o: &Value, inspect: &mut dyn FnMut(&Mesh)) -> std::result::Result<(), String> {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh)?;
    let input = &o["input"];
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: input["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect(),
        faces: input["polygons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| face(list(p)))
            .collect(),
        ..Default::default()
    }
    .into();
    e.dispatch(Command::Mode(Mode::Polygon))?;
    e.scene.selection.faces = list(&o["selected_polygons"]);
    e.dispatch(Command::Activate(Tool::Bevel))?;
    let s = &o["settings"];
    let flag = |k: &str, default: bool| s.get(k).map_or(default, |v| v.as_f64() == Some(1.));
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = s["shift"].as_f64().unwrap_or(0.);
    p.inset = s["inset"].as_f64().unwrap_or(0.);
    p.group_polygons = flag("group", true);
    p.bevel.offset_even = flag("offsetEven", true);
    p.bevel.auto_weld = flag("autoWeld", false);
    p.bevel.edge_rail = flag("edgeRail", false);
    if let Some(a) = s["maxAngle"].as_f64() {
        p.bevel.max_angle = a.to_degrees();
    }
    e.dispatch(Command::Preview(p))?;
    e.dispatch(Command::Drop)?;
    let mesh = &e.scene.active().unwrap().mesh;
    inspect(mesh);
    let x = &o["expected"];
    let points: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
    let polygons: Vec<Vec<u32>> = x["polygons"].as_array().unwrap().iter().map(list).collect();
    let tolerance = o["tolerance"].as_f64().unwrap_or(1e-5).max(1e-6);
    if mesh.vertices.len() != points.len() || mesh.faces.len() != polygons.len() {
        return Err(format!(
            "counts {}/{} vs {}/{}",
            mesh.vertices.len(),
            mesh.faces.len(),
            points.len(),
            polygons.len()
        ));
    }
    for (i, (a, b)) in mesh.vertices.iter().zip(&points).enumerate() {
        if (*a - *b).length() > tolerance {
            return Err(format!("point {i}: {a:?} vs {b:?}"));
        }
    }
    for (i, (f, want)) in mesh.faces.iter().zip(&polygons).enumerate() {
        if &f.vertices != want {
            return Err(format!(
                "polygon {i}: {:?} vs {want:?}\n  got  {:?}\n  want {polygons:?}",
                f.vertices,
                mesh.faces
                    .iter()
                    .map(|f| f.vertices.clone())
                    .collect::<Vec<_>>()
            ));
        }
    }
    let selected = list(&x["selected_polygons"]);
    if e.scene.selection.faces != selected {
        return Err(format!(
            "selection {:?} vs {selected:?}",
            e.scene.selection.faces
        ));
    }
    Ok(())
}

/// A polygon as its corner positions, rotated to start at the smallest one,
/// so results can be compared without depending on index order.
fn shape(points: &[DVec3], polygon: &[u32]) -> Vec<[i64; 3]> {
    let q: Vec<[i64; 3]> = polygon
        .iter()
        .map(|&v| {
            points[v as usize]
                .to_array()
                .map(|x| (x * 1e4).round() as i64)
        })
        .collect();
    let start = (0..q.len()).min_by_key(|&i| q[i]).unwrap_or(0);
    (0..q.len()).map(|k| q[(start + k) % q.len()]).collect()
}

/// Same polygons over the same positions, in any order.
fn check_unordered(o: &Value) -> std::result::Result<(), String> {
    match check(o) {
        Ok(()) => Ok(()),
        Err(_) => {
            let (got_points, got_faces) = run(o)?;
            let x = &o["expected"];
            let want_points: Vec<DVec3> =
                x["points"].as_array().unwrap().iter().map(vec3).collect();
            let mut want: Vec<_> = x["polygons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| shape(&want_points, &list(p)))
                .collect();
            let mut got: Vec<_> = got_faces.iter().map(|f| shape(&got_points, f)).collect();
            want.sort();
            got.sort();
            if got == want {
                Ok(())
            } else {
                Err(format!("polygons differ: {got:?} vs {want:?}"))
            }
        }
    }
}

/// Bevel one oracle and return the resulting points and polygons.
fn run(o: &Value) -> std::result::Result<(Vec<DVec3>, Vec<Vec<u32>>), String> {
    let mut o = o.clone();
    o["expected"] = serde_json::json!({"points": [], "polygons": [], "selected_polygons": []});
    let mut out = Err(String::new());
    // `check` builds and bevels; capture its mesh through a counts error.
    let _ = check_with(&o, &mut |mesh| {
        out = Ok((
            mesh.vertices.clone(),
            mesh.faces.iter().map(|f| f.vertices.clone()).collect(),
        ))
    });
    out
}

/// Cases where Operandi gives the same geometry in a different index order:
/// it creates a lone polygon's new points by point index (the reference uses
/// the polygon's corner order), keeps each cap in its own polygon's slot
/// (the reference reverses multi-face caps) and orders walls per polygon
/// (the reference walks the border). Deliberate; see
/// SPECIFICATION_QUESTIONS.md, Polygon Bevel.
const ORDER_ONLY: &[&str] = &[
    "PB-O-B1-box-XY-group",
    "PB-O-B1-box-XY-nogroup",
    "PB-O-B2-box-XY-group-edgeRail",
    "PB-O-B2-box-XY-group-insetonly",
    "PB-O-B2-box-XY-group-maxAngle30",
    "PB-O-B2-box-XY-group-offsetEven0",
    "PB-O-B2-box-XY-group-shiftonly",
    "PB-O-B3-cyl8-two-sides",
];
#[test]
#[cfg(operandi_specs)]
fn polygon_bevel_measured_cases() {
    let mut failures = Vec::new();
    // Batch 3 cases (PB-O-PB3-*) use another format and aren't built yet.
    for o in oracles()
        .into_iter()
        .filter(|o| !o["id"].as_str().unwrap().starts_with("PB-O-PB3"))
    {
        let id = o["id"].as_str().unwrap();
        let result = if ORDER_ONLY.contains(&id) {
            check_unordered(&o)
        } else if id == "PB-O-B6-zero" {
            // PB-O-04: the reference builds zero-size walls at Shift 0 and
            // Inset 0; Operandi leaves the mesh unchanged (no-op preview).
            match check(&o) {
                Err(e) if e.starts_with("counts 4/1") => Ok(()),
                other => Err(format!("expected the unchanged mesh, got {other:?}")),
            }
        } else {
            check(&o)
        };
        if let Err(e) = result {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// PB-O-01 in the editor: Bevel with nothing selected works on every
/// polygon, leaves nothing selected, and undoes in one step.
#[test]
fn empty_selection_bevel_undo() {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.inset = 0.1;
    // A whole closed box grouped has no border to bevel, so ungroup it.
    p.group_polygons = false;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (8 + 24, 6 + 24));
    assert!(e.scene.selection.faces.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

/// PB-C-01: an Inset past the centre isn't refused or clamped. At 0.5 the
/// cap has no size (an admitted degenerate face), at 0.6 it is inverted;
/// both meshes are valid, save and reopen, and draw without trouble.
#[test]
fn inset_past_the_centre_builds_the_cap_as_asked() {
    let mut square = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.),
        ],
        ..Default::default()
    };
    square.faces = vec![Face {
        vertices: vec![0, 1, 2, 3],
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }]
    .into();
    // Each corner past the centre is a half turn, so the 0.6 cap still
    // faces +Z, its corners swapped across the middle.
    for (inset, normal_z) in [(0.5, 0.), (0.6, 1.)] {
        let out = bevel_geometry(
            &square,
            &[0],
            0.,
            inset,
            false,
            &BevelOptions::default(),
            None,
        )
        .unwrap();
        out.mesh.validate().unwrap();
        assert_eq!((out.mesh.vertices.len(), out.mesh.faces.len()), (8, 5));
        let cap = out.selected[0] as usize;
        assert!(
            (out.mesh.normal(cap).z - normal_z).abs() < 1e-9,
            "inset {inset}"
        );
        // The corner from the origin goes `inset` along both sides, past the
        // centre at 0.6.
        let first = out.mesh.faces[cap].vertices[0] as usize;
        assert!((out.mesh.vertices[first] - DVec3::new(inset, inset, 0.)).length() < 1e-9);
        for f in 0..out.mesh.faces.len() {
            let _ = out.mesh.surface_triangles(f);
        }
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = out.mesh.clone().into();
        let dir = std::env::temp_dir().join(format!("inset-past-{}-{inset}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cap.opscene");
        persistence::save(&path, &scene).unwrap();
        assert_eq!(persistence::load(&path).unwrap().items[0].mesh, out.mesh);
        std::fs::remove_dir_all(&dir).ok();
    }
}
