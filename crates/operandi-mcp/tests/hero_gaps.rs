//! Gaps found making hero renders through the bridge: importing an OBJ
//! into the open scene, and placing primitives at a world-space position.
//! (Viewport shading and panels are app-only; their tests are in
//! `crates/operandi-app/src/mcp.rs`.)
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

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-mcp-hero-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The world-space bounding box centre of an item's mesh.
fn world_centre(b: &mut Bridge, item: u64) -> [f64; 3] {
    let mesh = call(
        b,
        "mesh_get",
        json!({"item": item, "world": true, "edges": false}),
    )
    .unwrap();
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for v in mesh["vertices"].as_array().unwrap() {
        for a in 0..3 {
            let x = v[a].as_f64().unwrap();
            lo[a] = lo[a].min(x);
            hi[a] = hi[a].max(x);
        }
    }
    assert!(lo[0] <= hi[0], "the mesh has vertices");
    [0, 1, 2].map(|a| (lo[a] + hi[a]) / 2.)
}

/// Gap 1: scene_import adds the OBJ's mesh beside what is there, as one
/// undo step, and refuses bad input without changing anything.
#[test]
fn scene_import_adds_an_obj_as_one_step() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let dir = temp("import");
    let obj = dir.join("card.obj");
    std::fs::write(
        &obj,
        "o Card\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n",
    )
    .unwrap();
    let before = b.editor.scene.items.len();
    let r = call(&mut b, "scene_import", json!({"path": obj})).unwrap();
    assert_eq!(r["imported"].as_array().unwrap().len(), 1, "{r}");
    assert_eq!(b.editor.scene.items.len(), before + 1);
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(
        info["undo"].as_array().unwrap().last().unwrap(),
        "Import card.obj"
    );
    call(&mut b, "undo", json!({})).unwrap();
    assert_eq!(b.editor.scene.items.len(), before);
    let scene = b.editor.scene.clone();
    for path in [
        dir.join("gone.obj"),
        dir.join("scene.lxo"),
        dir.join("a.txt"),
    ] {
        assert!(call(&mut b, "scene_import", json!({"path": path})).is_err());
        assert!(b.editor.scene == scene);
    }
    assert!(call(&mut b, "scene_import", json!({})).is_err());
}

/// Gap 4: primitive_add's position is the primitive's centre in world
/// space, whatever the active mesh's own transform; no position means the
/// world origin (not the work plane or action centre).
#[test]
fn primitive_position_lands_at_world_coordinates() {
    let mut b = Bridge::new();
    let at = [1.0, 2.0, -3.0];
    for shape in [
        "cube",
        "sphere",
        "cylinder",
        "cone",
        "ellipsoid",
        "torus",
        "capsule",
    ] {
        call(
            &mut b,
            "command_run",
            json!({"command": "mesh.new", "args": {}}),
        )
        .unwrap();
        let item = b.editor.scene.selection.item.unwrap();
        // The mesh item is moved, turned and scaled; the primitive still
        // lands at the given world point.
        call(&mut b, "command_run", json!({"command": "item.transform", "args": {
            "translation": [5.0, -1.0, 0.5], "rotation": [0.3, 0.7, 0.0], "scale": [2.0, 1.0, 0.5]
        }}))
        .unwrap();
        call(
            &mut b,
            "primitive_add",
            json!({"shape": shape, "params": {"position": at}}),
        )
        .unwrap();
        let c = world_centre(&mut b, item);
        for a in 0..3 {
            assert!(
                (c[a] - at[a]).abs() < 1e-6,
                "{shape}: centre {c:?}, wanted {at:?}"
            );
        }
        // Without a position: the world origin.
        call(
            &mut b,
            "command_run",
            json!({"command": "mesh.new", "args": {}}),
        )
        .unwrap();
        let item = b.editor.scene.selection.item.unwrap();
        call(
            &mut b,
            "command_run",
            json!({"command": "item.transform", "args": {"translation": [4.0, 4.0, 4.0]}}),
        )
        .unwrap();
        call(&mut b, "primitive_add", json!({"shape": shape})).unwrap();
        let c = world_centre(&mut b, item);
        for a in 0..3 {
            assert!(c[a].abs() < 1e-6, "{shape} default: centre {c:?}");
        }
    }
    let def = Bridge::tool_definitions()
        .into_iter()
        .find(|t| t["name"] == "primitive_add")
        .unwrap();
    assert!(
        def["inputSchema"]["properties"]["params"]["properties"]["position"]["description"]
            .as_str()
            .unwrap()
            .contains("world")
    );
}
