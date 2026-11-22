//! Mesh command measured cases (specifications/mesh-commands-2026-11-02),
//! run through the editor commands the panel uses. Results are compared by
//! geometry (positions and polygon shapes, any order) and point count.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

fn oracles() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/mesh-commands-2026-11-02/ORACLES.json"
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
fn shapes(points: &[DVec3], polygons: &[Vec<u32>]) -> Vec<Vec<[i64; 3]>> {
    let mut out: Vec<_> = polygons
        .iter()
        .map(|p| {
            let q: Vec<[i64; 3]> = p
                .iter()
                .map(|&v| {
                    points[v as usize]
                        .to_array()
                        .map(|x| (x * 1e4).round() as i64)
                })
                .collect();
            let start = (0..q.len()).min_by_key(|&i| q[i]).unwrap_or(0);
            (0..q.len()).map(|k| q[(start + k) % q.len()]).collect()
        })
        .collect();
    out.sort();
    out
}

fn run(o: &Value) -> std::result::Result<Mesh, String> {
    let input = &o["input"];
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh)?;
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
    let sel = &o["selection"];
    let s = &o["settings"];
    if let Some(p) = sel.get("polygon") {
        e.dispatch(Command::Mode(Mode::Polygon))?;
        e.scene.selection.faces = list(p);
    } else if let Some(edges) = sel.get("edge") {
        e.dispatch(Command::Mode(Mode::Edge))?;
        e.scene.selection.edges = edges
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                let x = list(x);
                (x[0].min(x[1]), x[0].max(x[1]))
            })
            .collect();
    } else if let Some(v) = sel.get("vertex") {
        e.dispatch(Command::Mode(Mode::Vertex))?;
        e.scene.selection.vertices = list(v);
    }
    let flag = |k: &str| s[k].as_bool().unwrap_or(false);
    let command = match o["tool"].as_str().unwrap() {
        "Triple" => Command::Triple,
        "Quadruple" => Command::QuadruplePolygon,
        "Spin Quads" => Command::SpinQuads,
        "Polygon Align" | "Align" => Command::AlignPolygons,
        "Polygon Merge" => Command::MergePolygons,
        "Polygon Collapse" => Command::CollapsePolygon,
        "Polygon Remove" => Command::RemovePolygons,
        "Unify Polygons" => Command::UnifyPolygons(UnifyRequest {
            force: flag("Force"),
            expected: None,
        }),
        "Edge Remove" => Command::EdgeCommand(EdgeCommand::Remove),
        "Edge Collapse" => Command::EdgeCommand(EdgeCommand::Collapse),
        "Edge Join" => Command::EdgeCommand(if flag("Average") {
            EdgeCommand::JoinAveraged
        } else {
            EdgeCommand::Join
        }),
        "Edge Split" => Command::EdgeCommand(EdgeCommand::Split),
        "Edge Subdivide" if s.get("Count").is_some() => Command::SubdivideEdges {
            count: s["Count"].as_u64().unwrap() as u32,
            percent: s["Percent"].as_f64(),
        },
        "Edge Subdivide" => Command::EdgeCommand(EdgeCommand::Subdivide),
        "Vertex Merge" => Command::VertexCommand(VertexCommand::Merge {
            distance: s["Distance"].as_f64().unwrap_or(0.),
            options: MergeOptions::default(),
        }),
        "Vertex Join" => Command::VertexCommand(VertexCommand::Join {
            average: flag("Average"),
            options: MergeOptions::default(),
        }),
        "Vertex Collapse" => Command::VertexCommand(VertexCommand::Collapse),
        "Vertex Remove" => Command::VertexCommand(VertexCommand::Remove),
        "Vertex Split" => Command::VertexCommand(VertexCommand::Split),
        "Vertex Align" => Command::VertexCommand(VertexCommand::Align {
            axes: if s["Axis"] == "Y" {
                [false, true, false]
            } else {
                [true; 3]
            },
            uniform: false,
        }),
        other => return Err(format!("no command for {other}")),
    };
    e.dispatch(command)?;
    Ok(e.scene.active().unwrap().mesh.to_owned_value())
}

fn status(o: &Value) -> String {
    let x = &o["expected"];
    let points: Vec<DVec3> = x["points"].as_array().unwrap().iter().map(vec3).collect();
    let polygons: Vec<Vec<u32>> = x["polygons"].as_array().unwrap().iter().map(list).collect();
    let input = &o["input"];
    let in_points: Vec<DVec3> = input["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(vec3)
        .collect();
    let in_polygons: Vec<Vec<u32>> = input["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(list)
        .collect();
    let unchanged = in_points.len() == points.len()
        && in_points
            .iter()
            .zip(&points)
            .all(|(a, b)| a.distance(*b) < 1e-9)
        && in_polygons == polygons;
    match run(o) {
        Ok(m) => {
            let faces: Vec<Vec<u32>> = m.faces.iter().map(|f| f.vertices.clone()).collect();
            if m.vertices.len() == points.len()
                && shapes(&m.vertices, &faces) == shapes(&points, &polygons)
            {
                "match".into()
            } else {
                format!(
                    "{}/{} vs {}/{}",
                    m.vertices.len(),
                    m.faces.len(),
                    points.len(),
                    polygons.len()
                )
            }
        }
        // The reference changing nothing and Operandi refusing (with a
        // message, scene unchanged) count as the same outcome.
        Err(_) if unchanged => "match".into(),
        Err(e) => format!("error: {e}"),
    }
}

/// Not matching yet, each logged in SPECIFICATION_QUESTIONS.md (Mesh
/// commands): duplicate or mixed-winding polygons can't be loaded (Operandi
/// refuses non-manifold meshes), Vertex
/// Collapse and Subdivide Percent follow other readings (CMD-Q04, CMD-Q07).
/// This test loads meshes directly; tests/oracle_replay.rs in operandi-mcp
/// replays the mixed-winding cases the way an opened scene records them.
const DIFFERENT: &[&str] = &[
    "CMD-O-align-mixed-winding",
    "CMD-O-unify-dups",
    "CMD-O-unify-dups-force",
    "CMD-O-edge-subdivide-percent25",
];

#[test]
fn mesh_command_measured_cases() {
    let mut report = Vec::new();
    for o in oracles()
        .into_iter()
        .filter(|o| !o["id"].as_str().unwrap().starts_with("CMD-O-M3"))
    {
        let id = o["id"].as_str().unwrap();
        let status = status(&o);
        if (status == "match") == DIFFERENT.contains(&id) {
            report.push(format!("{id}: {status}"));
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}

/// Batch 3 cases (CMD-B3-*), in their own format.
fn as_batch_1(o: &Value) -> Value {
    let mut o = o.clone();
    o["tool"] = o["settings"]["operation"].clone();
    let sel = o["selection"].clone();
    o["selection"] = if !sel["sel_polygons"].as_array().unwrap().is_empty() {
        serde_json::json!({"polygon": sel["sel_polygons"]})
    } else if !sel["sel_edges"].as_array().unwrap().is_empty() {
        serde_json::json!({"edge": sel["sel_edges"]})
    } else {
        serde_json::json!({"vertex": sel["sel_vertices"]})
    };
    if o["tool"] == "Edge Split" {
        o["settings"]["Caps"] = o["settings"]["Caps"].clone();
    }
    o
}

/// Batch 3 cases that match; the rest (Align on flipped strips, which
/// Operandi can't load, and the closed-loop Edge Split) are logged.
const B3_MATCHING: &[&str] = &[
    "CMD-O-M3-01-count1",
    "CMD-O-M3-01-count2",
    "CMD-O-M3-01-count3",
    "CMD-O-M3-01-count4",
    "CMD-O-M3-01-percent10",
    "CMD-O-M3-01-percent25",
    "CMD-O-M3-01-percent50",
    "CMD-O-M3-01-percent75",
    "CMD-O-M3-02-asymmetric3",
    "CMD-O-M3-02-border",
    "CMD-O-M3-02-valence3",
    "CMD-O-M3-02-valence4",
    "CMD-O-M3-02-valence5",
    "CMD-O-M3-03-grid-mixed",
    "CMD-O-M3-03-grid-same",
    "CMD-O-M3-03-pair-mixed",
    "CMD-O-M3-03-pair-same",
    "CMD-O-M3-05-caps0-gap0",
];

#[test]
fn mesh_command_batch_3_cases() {
    let mut report = Vec::new();
    for o in oracles()
        .into_iter()
        .filter(|o| o["id"].as_str().unwrap().starts_with("CMD-O-M3"))
    {
        let id = o["id"].as_str().unwrap();
        let status = status(&as_batch_1(&o));
        if (status == "match") != B3_MATCHING.contains(&id) {
            report.push(format!("{id}: {status}"));
        }
    }
    assert!(report.is_empty(), "{}", report.join("\n"));
}
