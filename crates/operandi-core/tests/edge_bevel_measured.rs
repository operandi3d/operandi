//! Edge Bevel / Edge Chamfer measured cases
//! (specifications/edge-bevel-chamfer-2026-11-02). Compared by geometry:
//! the same point positions and polygon shapes, in any index order.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("OPERANDI_SPECS_DIR"),
        "/specifications/edge-bevel-chamfer-2026-11-02/ORACLES.json"
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
    let chamfer = s.get("offset").is_some();
    let width = s["offset"].as_f64().or(s["inset"].as_f64()).unwrap();
    // Round Level L gives 2·L segments (EBC-B3-02); Segments is direct.
    let rows = if chamfer {
        s["segments"].as_u64().unwrap_or(1) as u32
    } else {
        (2 * s["level"].as_u64().unwrap_or(0) as u32).max(1)
    };
    let depth = s["depth"].as_f64().unwrap_or(1.) * 100.;
    let options = EdgeBevelOptions {
        mode: if s["mode"] == "width" {
            EdgeBevelMode::Width
        } else {
            EdgeBevelMode::Inset
        },
        shape: match s["shape"].as_str() {
            Some("square") => EdgeBevelShape::Square,
            Some("sharp") => EdgeBevelShape::Sharp,
            _ => EdgeBevelShape::Round,
        },
        edge_end: if s["edgeEnd"] == "triangleFan" {
            EdgeEnd::TriangleFan
        } else {
            EdgeEnd::Profile
        },
        stop_at_edge: s["stopAtEdge"] != 0,
        auto_weld: s["autoWeld"] == 1,
        ..Default::default()
    };
    let edges: Vec<(u32, u32)> = o["selected_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let e = list(e);
            (e[0].min(e[1]), e[0].max(e[1]))
        })
        .collect();
    let settings = EdgeBevelSettings {
        width,
        rows,
        depth,
        chamfer,
        options,
    };
    let out = edge_bevel_with(&mesh, &edges, &settings, None)?.mesh;
    let used: std::collections::BTreeSet<u32> = out
        .faces
        .iter()
        .flat_map(|f| f.vertices.iter().copied())
        .collect();
    let x = &o["expected"];
    let points: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
    let polygons: Vec<Vec<u32>> = x["polygons"].as_array().unwrap().iter().map(list).collect();
    let mut a: Vec<_> = out
        .faces
        .iter()
        .map(|f| shape(&out.vertices, &f.vertices))
        .collect();
    let mut b: Vec<_> = polygons.iter().map(|f| shape(&points, f)).collect();
    a.sort();
    b.sort();
    if used.len() != points.len() || a != b {
        return Err(format!(
            "{} points / {} polygons vs {} / {}",
            used.len(),
            out.faces.len(),
            points.len(),
            polygons.len()
        ));
    }
    Ok(())
}

/// Every first-batch case matches.
const NOT_BUILT: &[&str] = &[];

#[test]
fn edge_bevel_measured_cases() {
    let mut failures = Vec::new();
    for o in oracles()
        .into_iter()
        .filter(|o| !o["id"].as_str().unwrap().starts_with("EBC-O-E3"))
    {
        let id = o["id"].as_str().unwrap();
        match (NOT_BUILT.contains(&id), check(&o)) {
            (false, Err(e)) => failures.push(format!("{id}: {e}")),
            (true, Ok(())) => failures.push(format!("{id}: now matches; move it out of NOT_BUILT")),
            _ => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// EBC-J-01..03 selection after, through the editor: Edge Bevel selects the
/// strips' edges (4 for one edge, 7 for the L, 12 at a corner), Edge
/// Chamfer selects nothing; one undo restores the box.
#[test]
fn selection_after_and_undo() {
    let o = oracles()
        .into_iter()
        .find(|o| o["id"] == "EBC-O-bevel-one")
        .unwrap();
    let input = &o["input"];
    let box_mesh = Mesh {
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
    for (tool, edges, count) in [
        (Tool::EdgeBevel, vec![(6, 7)], 4),
        (Tool::EdgeBevel, vec![(6, 7), (5, 6)], 7),
        (Tool::EdgeBevel, vec![(6, 7), (5, 6), (2, 6)], 12),
        (Tool::EdgeChamfer, vec![(6, 7), (5, 6)], 0),
    ] {
        let mut e = Editor::default();
        e.dispatch(Command::NewMesh).unwrap();
        e.scene.active_mut().unwrap().mesh = box_mesh.clone().into();
        e.dispatch(Command::Mode(Mode::Edge)).unwrap();
        e.scene.selection.edges = edges;
        let before = e.scene.clone();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.chamfer.offset = 0.1;
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.scene.selection.edges.len(), count, "{tool:?}");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, before);
    }
}

/// Batch 3 cases (EBC-B3-*), in their own settings format, converted to
/// the first batch's.
fn check_b3(o: &Value) -> std::result::Result<(), String> {
    let s = &o["settings"];
    let chamfer = s["tool"] == "Edge Chamfer";
    let width = ["Inset m", "Offset m", "Inset or Offset m", "Width m"]
        .iter()
        .find_map(|k| s[*k].as_f64())
        .ok_or("no width")?;
    let mut o = o.clone();
    let mut settings = serde_json::json!({});
    if chamfer {
        settings["offset"] = width.into();
        settings["segments"] = s["Segments"].as_u64().unwrap_or(1).into();
    } else {
        settings["inset"] = width.into();
        settings["level"] = s["Round Level"].as_u64().unwrap_or(0).into();
    }
    if s["Mode"] == "Width" {
        settings["mode"] = "width".into();
    }
    if let Some(shape) = s["Shape"].as_str() {
        settings["shape"] = shape.to_ascii_lowercase().into();
    }
    if s["Edge End"] == "Triangle Fan" {
        settings["edgeEnd"] = "triangleFan".into();
    }
    o["settings"] = settings;
    o["selected_edges"] = o["selection"]["sel_edges"].clone();
    check(&o)
}

/// Batch 3 measured cases that match; the rest are listed in
/// SPECIFICATION_QUESTIONS.md (Batch 3).
const B3_MATCHING: &[&str] = &[
    "EBC-O-E3-01-bevel-sides4",
    "EBC-O-E3-01-bevel-sides6",
    "EBC-O-E3-01-chamfer-sides8",
    "EBC-O-E3-01-sides3",
    "EBC-O-E3-01-sides8",
    "EBC-O-E3-02-level1",
    "EBC-O-E3-02-level3",
    "EBC-O-E3-02-level4",
    "EBC-O-E3-03-sides3",
    "EBC-O-E3-03-sides6",
    "EBC-O-E3-05-chamfer-seg2",
    "EBC-O-E3-06-one-bevel",
    "EBC-O-E3-06-row-bevel",
    "EBC-O-E3-06-row-chamfer",
    "EBC-O-E3-08-sharp-level1",
    "EBC-O-E3-08-sharp-level2",
    "EBC-O-E3-08-sharp-level3",
    "EBC-O-E3-08-square-level1",
    "EBC-O-E3-08-square-level2",
    "EBC-O-E3-08-square-level3",
];

#[test]
fn edge_bevel_batch_3_cases() {
    let mut report = Vec::new();
    for o in oracles()
        .into_iter()
        .filter(|o| o["id"].as_str().unwrap().starts_with("EBC-O-E3"))
    {
        let id = o["id"].as_str().unwrap();
        let result = check_b3(&o);
        if result.is_ok() != B3_MATCHING.contains(&id) {
            report.push(format!("{id}: {result:?}"));
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}
