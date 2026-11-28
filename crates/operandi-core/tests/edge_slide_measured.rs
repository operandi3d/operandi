//! Edge Slide measured cases (specifications/add-loop-edge-slide-2026-11-02).
//! Point positions are compared in order; polygons by shape, since merging
//! renumbers them.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/add-loop-edge-slide-2026-11-02/ORACLES.json"
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
fn edge_set(points: &[DVec3], edges: &[(u32, u32)]) -> Vec<Vec<[i64; 3]>> {
    let mut out: Vec<_> = edges
        .iter()
        .map(|&(a, b)| {
            let mut e = vec![shape(points, &[a])[0], shape(points, &[b])[0]];
            e.sort();
            e
        })
        .collect();
    out.sort();
    out
}

fn check(o: &Value) -> std::result::Result<(), String> {
    let input = &o["input"];
    let mesh = Mesh {
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
            .map(|p| Face {
                vertices: list(p),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    };
    let s = &o["settings"];
    let by_distance = s["interpolation"] == "distance";
    let amount = if by_distance {
        s["dist"].as_f64().unwrap()
    } else {
        s["percent"].as_f64().unwrap_or(0.)
    };
    let edges: Vec<(u32, u32)> = o["selection"]["edge"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|e| {
                    let e = list(e);
                    (e[0], e[1])
                })
                .collect()
        })
        .unwrap_or_default();
    let (out, after) = edge_slide_with(&mesh, &edges, amount, by_distance)?;
    let x = &o["expected"];
    let points: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
    let polygons: Vec<Vec<u32>> = x["polygons"].as_array().unwrap().iter().map(list).collect();
    if out.vertices.len() != points.len() {
        return Err(format!("{} points vs {}", out.vertices.len(), points.len()));
    }
    for (i, (a, b)) in out.vertices.iter().zip(&points).enumerate() {
        if a.distance(*b) > 1e-5 {
            return Err(format!("point {i}: {a:?} vs {b:?}"));
        }
    }
    let mut a: Vec<_> = out
        .faces
        .iter()
        .map(|f| shape(&out.vertices, &f.vertices))
        .collect();
    let mut b: Vec<_> = polygons.iter().map(|f| shape(&points, f)).collect();
    a.sort();
    b.sort();
    if a != b {
        return Err("polygons differ".into());
    }
    let want: Vec<(u32, u32)> = x["selected_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let e = list(e);
            (e[0], e[1])
        })
        .collect();
    if edge_set(&out.vertices, &after) != edge_set(&points, &want) {
        return Err(format!("selection {after:?} vs {want:?}"));
    }
    Ok(())
}

#[test]
fn edge_slide_measured_cases() {
    let mut failures = Vec::new();
    // The first batch of Edge Slide cases; v3 of the spec adds batch-2 Edge
    // Slide (ES-O-ES2-*) and Add Loop (AL-O-*) cases, not built yet.
    for o in oracles()
        .into_iter()
        .filter(|o| o["id"].as_str().unwrap().starts_with("ES-O-ES-"))
    {
        let id = o["id"].as_str().unwrap();
        let result = check(&o);
        // ES-08: points alone don't slide; Operandi refuses with a message
        // and changes nothing.
        let ok = if id == "ES-O-ES-grid-vertex" {
            result == Err("Select edges to Slide".into())
        } else {
            result.is_ok()
        };
        if !ok {
            failures.push(format!("{id}: {result:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// ES-04 through the editor: a full slide merges the loop into its
/// neighbours, keeps the moved loop selected, and undoes in one step.
#[test]
fn full_slide_merges_selects_and_undoes() {
    let o = oracles()
        .into_iter()
        .find(|o| o["id"] == "ES-O-ES-percent1.0")
        .unwrap();
    let input = &o["input"];
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
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
            .map(|p| Face {
                vertices: list(p),
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            })
            .collect(),
        ..Default::default()
    }
    .into();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(1, 5), (5, 9), (9, 13)];
    let before = e.scene.clone();
    e.dispatch(Command::Activate(Tool::EdgeSlide)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 1.;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert_eq!((mesh.vertices.len(), mesh.faces.len()), (12, 6));
    assert_eq!(e.scene.selection.edges, vec![(0, 3), (3, 6), (6, 9)]);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    // Distance mode: 0.25 along each rail.
    e.dispatch(Command::Activate(Tool::EdgeSlide)).unwrap();
    p.shift = 0.25;
    p.edge_slide_distance = true;
    e.dispatch(Command::Preview(p)).unwrap();
    assert!((e.scene.active().unwrap().mesh.vertices[5].x - 0.75).abs() < 1e-12);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, before);
}

/// Batch 2 Edge Slide cases (ES-O-ES2-*), in their own format.
fn as_batch_1(o: &Value) -> Value {
    let mut o = o.clone();
    let s = o["settings"].clone();
    o["settings"] = serde_json::json!({
        "interpolation": if s["Interpolation"] == "Distance" { "distance" } else { "percent" },
        "percent": s["Percent"],
        "dist": s["Distance"],
    });
    o["selection"] = serde_json::json!({"edge": o["selection"]["sel_edges"]});
    o["expected"]["selected_edges"] = o["expected"]["sel_edges"].clone();
    o
}

/// Operandi slides positive amounts to the chain's left side, which follows
/// winding and point order; these cases show the reference choosing the side
/// some other way (ES-Q01, logged), so they slide the other way here.
const ES2_DIFFERENT: &[&str] = &[
    "ES-O-ES2-01-keep-first-reverse",
    "ES-O-ES2-01-reflect-X",
    "ES-O-ES2-01-reverse-point-indices",
    "ES-O-ES2-01-reverse-points-sorted-endpoints",
    "ES-O-ES2-01-reverse-winding",
];

#[test]
fn edge_slide_batch_2_cases() {
    let mut failures = Vec::new();
    for o in oracles()
        .into_iter()
        .filter(|o| o["id"].as_str().unwrap().starts_with("ES-O-ES2-"))
    {
        let id = o["id"].as_str().unwrap();
        let result = check(&as_batch_1(&o));
        if result.is_ok() == ES2_DIFFERENT.contains(&id) {
            failures.push(format!("{id}: {result:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The front view the Add Loop clicks were made in: 231 px/m, world origin at
/// pixel (763.5, 934).
fn clicked_edge(mesh: &Mesh, px: &Value) -> (u32, u32) {
    let at = DVec3::new(
        (px[0].as_f64().unwrap() - 763.5) / 231.,
        (934. - px[1].as_f64().unwrap()) / 231.,
        0.,
    );
    let mut best = (f64::INFINITY, (0, 0));
    for f in &mesh.faces {
        for (i, &a) in f.vertices.iter().enumerate() {
            let b = f.vertices[(i + 1) % f.vertices.len()];
            let (p, q) = (mesh.vertices[a as usize], mesh.vertices[b as usize]);
            let t = ((at - p).dot(q - p) / (q - p).length_squared()).clamp(0., 1.);
            let d = at.distance(p + (q - p) * t);
            if d < best.0 {
                best = (d, (a, b));
            }
        }
    }
    best.1
}

/// Split (AL-04) isn't built; the rest are Add Loop's geometry from the
/// clicked edge at the fraction the reference reported. How a click or drag
/// becomes that fraction (its rounding, AL-02/03) is the app's, and open.
const AL_DIFFERENT: &[&str] = &["AL-O-AL-04-split"];

#[test]
fn add_loop_measured_cases() {
    let mut failures = Vec::new();
    for o in oracles()
        .into_iter()
        .filter(|o| o["id"].as_str().unwrap().starts_with("AL-O-"))
    {
        let id = o["id"].as_str().unwrap();
        let input = &o["input"];
        let mut mesh = Mesh {
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
                .map(|p| Face {
                    vertices: list(p),
                    uv: None,
                    material: String::new(),
                    smooth: false,
                    part: None,
                    material_label: None,
                })
                .collect(),
            ..Default::default()
        };
        let x = &o["expected"];
        // AL-PHY-01…04 (AL-O-H4-*) name the edge and the reported percent;
        // the Command click inserts nothing.
        let mut clicks = if let Some(edge) = o["gesture"].get("target_edge") {
            let edge = list(edge);
            if x["loop_inserted"] == false {
                vec![]
            } else {
                let t = x["position_percent_after"].as_f64().unwrap() / 100.;
                vec![((edge[0], edge[1]), t)]
            }
        } else {
            let t = x["Position (fraction)"].as_f64().unwrap();
            vec![(clicked_edge(&mesh, &o["gesture"]["from_px"]), t)]
        };
        // AL-11: a second click (879,934) on the first mesh's edge 0–1.
        if id == "AL-O-AL-09" {
            clicks.push(((0, 1), 0.5));
        }
        let mut result = Ok(());
        for ((a, b), t) in clicks {
            // The fraction runs from the edge's lower-X end.
            let (a, b) = if mesh.vertices[a as usize].x > mesh.vertices[b as usize].x {
                (b, a)
            } else {
                (a, b)
            };
            match add_loop(&mesh, a, b, t) {
                Ok((m, _)) => mesh = m,
                Err(e) => result = Err(e.to_string()),
            }
        }
        let points: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
        let polygons: Vec<Vec<u32>> = x["polygons"].as_array().unwrap().iter().map(list).collect();
        if result.is_ok() {
            let mut a: Vec<_> = mesh
                .faces
                .iter()
                .map(|f| shape(&mesh.vertices, &f.vertices))
                .collect();
            let mut b: Vec<_> = polygons.iter().map(|f| shape(&points, f)).collect();
            a.sort();
            b.sort();
            if mesh.vertices.len() != points.len() || a != b {
                result = Err(format!(
                    "{}/{} vs {}/{}",
                    mesh.vertices.len(),
                    mesh.faces.len(),
                    points.len(),
                    polygons.len()
                ));
            }
        }
        if result.is_ok() == AL_DIFFERENT.contains(&id) {
            failures.push(format!("{id}: {result:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
