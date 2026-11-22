//! PF-12 through the bridge: `tool_run edge.extend` at its Reset values
//! adds the zero-width polygon and selects its new edge.
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

#[test]
fn edge_extend_at_reset_adds_the_zero_width_row() {
    let dir = std::env::temp_dir().join(format!("operandi-pf12-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("grid.opscene");
    let face =
        |v: [u32; 4]| json!({"vertices": v, "uv": null, "material": "Default", "smooth": false});
    let points: Vec<Value> = (0..9).map(|k| json!([k % 3, 0, k / 3])).collect();
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "Grid", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {"vertices": points, "faces": [face([0, 3, 4, 1]), face([1, 4, 5, 2]), face([3, 6, 7, 4]), face([4, 7, 8, 5])]}
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Edge", "vertices": [], "edges": [[0, 1]], "faces": []},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    // Other tool runs first, leaving their own values behind.
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "edge.extend", "params": {"delta": [0, 0, -0.5], "segments": [2, 1, 1]}}),
    )
    .unwrap();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "poly.bevel", "params": {"shift": 0.2}}),
    )
    .unwrap();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    call(&mut b, "tool_run", json!({"tool": "edge.extend"})).unwrap();
    let m = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let v = m["vertices"].as_array().unwrap();
    assert_eq!(v.len(), 11, "{m}");
    assert_eq!(v[9], json!([1.0, 0.0, 0.0]));
    assert_eq!(v[10], json!([0.0, 0.0, 0.0]));
    let cyclic = |f: &Value| {
        let f: Vec<u64> = f["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap())
            .collect();
        (0..f.len()).any(|k| [f[k..].to_vec(), f[..k].to_vec()].concat() == [1, 9, 10, 0])
    };
    assert!(m["faces"].as_array().unwrap().iter().any(cyclic), "{m}");
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert!(info.to_string().contains("[9,10]"), "{info}");
    // The new points are the point selection too (Vertex mode).
    assert_eq!(info["selection"]["vertices"], json!([9, 10]), "{info}");
}
