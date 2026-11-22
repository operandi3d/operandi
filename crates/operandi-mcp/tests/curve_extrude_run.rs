//! `tool_run CurveExtrude` (bridge request 2026-11-20): the CX-Q8 check
//! case through the bridge.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};

fn call(b: &mut Bridge, name: &str, arguments: Value) -> Result<Value, String> {
    let r = protocol::handle(
        b,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {"name": name, "arguments": arguments}}),
    )
    .expect("a request gets a response");
    let result = &r["result"];
    if result["isError"] == json!(true) {
        Err(result["content"][0]["text"].as_str().unwrap().to_string())
    } else {
        Ok(result["structuredContent"].clone())
    }
}

fn v(p: &Value) -> [f64; 3] {
    let a = p.as_array().unwrap();
    [
        a[0].as_f64().unwrap(),
        a[1].as_f64().unwrap(),
        a[2].as_f64().unwrap(),
    ]
}

#[test]
fn the_end_cap_faces_the_carried_normal() {
    let dir = std::env::temp_dir().join(format!("operandi-cx-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("quad.opscene");
    // A 0.2 m quad at the origin facing +Y.
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "Quad", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {"vertices": [[-0.1, 0, -0.1], [-0.1, 0, 0.1], [0.1, 0, 0.1], [0.1, 0, -0.1]],
                 "faces": [{"vertices": [0, 1, 2, 3], "uv": null, "material": "Default", "smooth": false}]}
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": [0]},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    let undo = call(&mut b, "scene_info", json!({})).unwrap()["undo"]
        .as_array()
        .unwrap()
        .len();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "CurveExtrude", "params": {"curve_extrude": {
            "points": [[0, 0, 0], [0, 1, 0], [1, 2, 0]], "steps": 4}}}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(
        info["undo"].as_array().unwrap().len(),
        undo + 1,
        "one undo step"
    );
    // The end cap (selected afterwards, CX-15) faces (0.9502, 0.3115, 0).
    let cap = info["selection"]["faces"][0].as_u64().unwrap() as usize;
    let m = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let face: Vec<[f64; 3]> = m["faces"][cap]["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| v(&m["vertices"][i.as_u64().unwrap() as usize]))
        .collect();
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let (e1, e2) = (sub(face[1], face[0]), sub(face[2], face[0]));
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    let n = [n[0] / len, n[1] / len, n[2] / len];
    assert!(
        (n[0] - 0.9502).abs() < 1e-4 && (n[1] - 0.3115).abs() < 1e-4 && n[2].abs() < 1e-4,
        "{n:?}"
    );
    // Unknown options are refused, nothing changes.
    let before = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    assert!(call(&mut b, "tool_run", json!({"tool": "CurveExtrude", "params": {"curve_extrude": {"points": [[0, 0, 0], [0, 1, 0]], "wobble": 1}}})).is_err());
    assert_eq!(
        call(&mut b, "mesh_get", json!({"edges": false})).unwrap(),
        before
    );
}

/// tools/list's tool enum names every tool tool_run takes, and tool_run
/// knows every name in it (a strict client checks the enum).
#[test]
fn the_tool_enum_lists_every_tool_tool_run_takes() {
    let tools = operandi_mcp::Bridge::tool_definitions();
    let run = tools.iter().find(|t| t["name"] == "tool_run").unwrap();
    let listed: Vec<String> = run["inputSchema"]["properties"]["tool"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let takes = operandi_mcp::bridge::TOOLS
        .iter()
        .map(|(id, _)| *id)
        .chain(operandi_mcp::bridge::SESSION_TOOLS);
    for id in takes {
        assert!(listed.iter().any(|l| l == id), "{id} missing from the enum");
    }
    for id in &listed {
        let mut b = Bridge::new();
        if let Err(e) = call(&mut b, "tool_run", json!({"tool": id})) {
            assert!(!e.starts_with("Unknown tool"), "{id}: {e}");
        }
    }
}

/// CE3 through the bridge: Cap Start on, Cap End off selects the start cap.
#[test]
fn cap_start_only_selects_the_start_cap() {
    let dir = std::env::temp_dir().join(format!("operandi-cx-ce3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("quad.opscene");
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "Quad", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {"vertices": [[-0.1, 0, -0.1], [-0.1, 0, 0.1], [0.1, 0, 0.1], [0.1, 0, -0.1]],
                 "faces": [{"vertices": [0, 1, 2, 3], "uv": null, "material": "Default", "smooth": false}]}
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": [0]},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "CurveExtrude", "params": {"curve_extrude": {
            "points": [[0, 0, 0], [0, 1, 0], [1, 2, 0]], "steps": 4,
            "cap_start": true, "cap_end": false}}}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    let faces = info["selection"]["faces"].as_array().unwrap();
    assert_eq!(faces.len(), 1, "{info}");
    let m = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let cap = &m["faces"][faces[0].as_u64().unwrap() as usize]["vertices"];
    // The source polygon [0, 1, 2, 3], reversed.
    let got: Vec<u64> = cap
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let back = [3u64, 2, 1, 0];
    assert!(
        (0..4).any(|k| [got[k..].to_vec(), got[..k].to_vec()].concat() == back),
        "{got:?}"
    );
}

/// Both caps on: both are selected afterwards, Align to Path on or off.
#[test]
fn both_caps_are_selected() {
    for align in [true, false] {
        let dir = std::env::temp_dir().join(format!("operandi-cx-both-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("quad.opscene");
        let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
            "id": 1, "name": "Quad", "visible": true,
            "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
            "mesh": {"vertices": [[-0.1, 0, -0.1], [-0.1, 0, 0.1], [0.1, 0, 0.1], [0.1, 0, -0.1]],
                     "faces": [{"vertices": [0, 1, 2, 3], "uv": null, "material": "Default", "smooth": false}]}
        }], "next_id": 2,
            "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": [0]},
            "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
        std::fs::write(&path, scene.to_string()).unwrap();
        let mut b = Bridge::new();
        call(&mut b, "scene_load", json!({"path": path})).unwrap();
        call(
            &mut b,
            "tool_run",
            json!({"tool": "CurveExtrude", "params": {"curve_extrude": {
                "points": [[0, 0, 0], [0, 1, 0], [1, 2, 0]], "steps": 4,
                "cap_start": true, "cap_end": true, "align_to_path": align}}}),
        )
        .unwrap();
        let info = call(&mut b, "scene_info", json!({})).unwrap();
        assert_eq!(info["selection"]["mode"], "polygon", "{info}");
        assert_eq!(
            info["selection"]["faces"].as_array().unwrap().len(),
            2,
            "align {align}: {info}"
        );
    }
}
