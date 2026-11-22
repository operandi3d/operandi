//! Bridge request 2026-11-20 (Research's behaviour cases): new headless
//! `command_run` entries and `tool_run` tools. Each is one undo step on the
//! active mesh's selection; options not given take the Reset values; a
//! refusal changes nothing.
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

fn command(b: &mut Bridge, name: &str, args: Value) -> Result<Value, String> {
    call(b, "command_run", json!({"command": name, "args": args}))
}

fn mesh(b: &mut Bridge) -> Value {
    call(b, "mesh_get", json!({"edges": false})).unwrap()
}

fn faces(b: &mut Bridge) -> usize {
    mesh(b)["faces"].as_array().unwrap().len()
}

fn points(b: &mut Bridge) -> Vec<[f64; 3]> {
    mesh(b)["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let p = p.as_array().unwrap();
            [
                p[0].as_f64().unwrap(),
                p[1].as_f64().unwrap(),
                p[2].as_f64().unwrap(),
            ]
        })
        .collect()
}

fn undo_len(b: &mut Bridge) -> usize {
    call(b, "scene_info", json!({})).unwrap()["undo"]
        .as_array()
        .unwrap()
        .len()
}

fn cube(b: &mut Bridge) {
    call(
        b,
        "primitive_add",
        json!({"shape": "cube", "params": {"position": [0, 0, 0]}}),
    )
    .unwrap();
    call(b, "select", json!({"mode": "polygon"})).unwrap();
}

#[test]
fn vert_set_gives_the_selected_points_one_coordinate() {
    let mut b = Bridge::new();
    cube(&mut b);
    call(
        &mut b,
        "select",
        json!({"mode": "vertex", "vertices": [0, 1]}),
    )
    .unwrap();
    let steps = undo_len(&mut b);
    command(&mut b, "vert.set", json!({"axis": "y", "value": 2})).unwrap();
    let p = points(&mut b);
    assert_eq!((p[0][1], p[1][1]), (2., 2.));
    assert!(p[2..].iter().all(|q| q[1] != 2.));
    assert_eq!(undo_len(&mut b), steps + 1);
    // A refusal changes nothing.
    let before = points(&mut b);
    assert!(command(&mut b, "vert.set", json!({"axis": "w", "value": 1})).is_err());
    assert_eq!(points(&mut b), before);
}

#[test]
fn chamfer_and_haul_run_headless() {
    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    // Chamfer Edit reshapes existing bevel strips; a plain cube face has
    // none, so it refuses with the user's message and changes nothing.
    let before = mesh(&mut b);
    let refused = command(&mut b, "chamfer.edit", json!({"offset": 0.1})).unwrap_err();
    assert!(refused.contains("Chamfer Edit"), "{refused}");
    assert_eq!(mesh(&mut b), before);
    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    let before = points(&mut b);
    command(&mut b, "poly.haul", json!({"offset": 0.2})).unwrap();
    assert_ne!(points(&mut b), before);
}

#[test]
fn unify_removes_a_duplicate_polygon() {
    let dir = std::env::temp_dir().join(format!("operandi-pf-bridge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dup.opscene");
    let face =
        json!({"vertices": [0, 1, 2, 3], "uv": null, "material": "Default", "smooth": false});
    let scene = json!({"format": "Operandi", "version": 1, "scene": {"items": [{
        "id": 1, "name": "Dup", "visible": true,
        "transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1]},
        "mesh": {"vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]], "faces": [face.clone(), face]}
    }], "next_id": 2,
        "selection": {"item": 1, "mode": "Polygon", "vertices": [], "edges": [], "faces": []},
        "workplane": {"fixed": true, "plane": 0, "center": [0, 0, 0]}}});
    std::fs::write(&path, scene.to_string()).unwrap();
    let mut b = Bridge::new();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    assert_eq!(faces(&mut b), 2);
    command(&mut b, "poly.unify", json!({})).unwrap();
    assert_eq!(faces(&mut b), 1);
}

#[test]
fn inset_and_quantize_through_tool_run() {
    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [0]})).unwrap();
    let before = faces(&mut b);
    call(
        &mut b,
        "tool_run",
        json!({"tool": "poly.inset", "params": {"inset": 0.1}}),
    )
    .unwrap();
    assert_eq!(faces(&mut b), before + 4, "{}", mesh(&mut b));

    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "vertex"})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "xfrm.quantize", "params": {"step": [0.3, 0, 0]}}),
    )
    .unwrap();
    for p in points(&mut b) {
        let k = p[0] / 0.3;
        assert!((k - k.round()).abs() < 1e-9, "{p:?}");
    }
}

#[test]
fn command_help_describes_the_new_options() {
    let mut b = Bridge::new();
    for (cmd, arg) in [
        ("vert.set", "worldSpace"),
        ("poly.unify", "force"),
        ("chamfer.edit", "sourceSegs"),
        ("poly.haul", "offset"),
    ] {
        let help = call(&mut b, "command_help", json!({"command": cmd})).unwrap();
        assert!(help.to_string().contains(arg), "{cmd}: {help}");
    }
}
