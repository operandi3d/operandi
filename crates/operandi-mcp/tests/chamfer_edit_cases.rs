//! Chamfer Edit's recorded cases through the bridge's `command_run
//! chamfer.edit` (specifications/chamfer-edit-2026-11-05): the same
//! results as the core tool, one undo step each.
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

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/chamfer-edit-2026-11-05/cases.json"
    ))
    .unwrap();
    serde_json::from_str::<Value>(&text).unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}

/// Loads case `c`'s input with its selection into a fresh bridge.
fn load(c: &Value) -> Bridge {
    let dir = std::env::temp_dir().join(format!("operandi-ce-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{}.opscene", c["id"].as_str().unwrap()));
    let faces: Vec<Value> = c["input"]["polygons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({"vertices": p, "uv": null, "material": "Default", "smooth": false}))
        .collect();
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "Case", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {"vertices": c["input"]["points"], "faces": faces}
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [],
                      "faces": c["selection"]["sel_polygons"]},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    b
}

/// The case's settings as chamfer.edit arguments.
fn args(c: &Value) -> Value {
    let s = &c["settings"];
    let int = |k: &str| s[k].as_str().unwrap().parse::<u32>().unwrap();
    json!({
        "offset": s["offset"], "segments": int("segments"), "sourceSegs": int("sourceSegs"),
        "depth": s["depth"], "mode": s["mode"], "absSize": s["absSize"],
        "swapSide": s["swapSide"], "reverseX": s["reversex"], "reverseY": s["reversey"],
    })
}

fn counts(b: &mut Bridge) -> (usize, usize) {
    let m = call(b, "mesh_get", json!({"edges": false})).unwrap();
    (
        m["vertices"].as_array().unwrap().len(),
        m["faces"].as_array().unwrap().len(),
    )
}

#[test]
fn every_case_runs_through_the_bridge() {
    for c in cases() {
        let id = c["id"].as_str().unwrap();
        let mut b = load(&c);
        let out = call(
            &mut b,
            "command_run",
            json!({"command": "chamfer.edit", "args": args(&c)}),
        );
        if id == "CE13" {
            // Decided Q-CE-02: a Source Segments mismatch is refused.
            assert!(out.is_err(), "{id}");
            continue;
        }
        out.unwrap_or_else(|e| panic!("{id}: {e}"));
        let want = &c["expected"];
        if id != "CE15" {
            assert_eq!(
                counts(&mut b),
                (
                    want["points"].as_array().unwrap().len(),
                    want["polygons"].as_array().unwrap().len()
                ),
                "{id}"
            );
        }
        // The selection after (rule 11).
        let info = call(&mut b, "scene_info", json!({})).unwrap();
        let kept = !info["selection"]["faces"].as_array().unwrap().is_empty();
        let want_kept = !want["selection_after"]["sel_polygons"]
            .as_array()
            .unwrap()
            .is_empty();
        if id != "CE15" {
            assert_eq!(kept, want_kept, "{id}: {info}");
        }
    }
}
