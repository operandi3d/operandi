//! Gaps found making hero renders through the bridge: material channels,
//! shader tree layers, subdivision, reopening a file, light Render names.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

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
    let dir = std::env::temp_dir().join(format!("operandi-mcp-shading-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// A cube whose polygons carry material "Red".
fn red_cube() -> Bridge {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    call(
        &mut b,
        "select",
        json!({"mode": "polygon", "faces": [0, 1, 2, 3, 4, 5]}),
    )
    .unwrap();
    call(
        &mut b,
        "command_run",
        json!({"command": "poly.setMaterial", "args": {"name": "Red"}}),
    )
    .unwrap();
    b
}

fn undo_depth(b: &mut Bridge) -> usize {
    call(b, "scene_info", json!({})).unwrap()["undo"]
        .as_array()
        .unwrap()
        .len()
}

#[test]
fn material_set_edits_channels_as_one_undo_step() {
    let mut b = red_cube();
    let before = call(&mut b, "material_get", json!({"material": "Red"})).unwrap();
    assert_eq!(before["material"], "Red");
    assert_eq!(before["units"]["roughness"], "fraction 0-1");
    let depth = undo_depth(&mut b);
    call(
        &mut b,
        "material_set",
        json!({"material": "Red", "diffuse_color": [0.8, 0.05, 0.05], "roughness": 0.25,
               "channels": {"metallic": 1, "clearcoat_amount": 0.5},
               "smoothing_angle": FRAC_PI_4, "shading_model": "Physically Based"}),
    )
    .unwrap();
    assert_eq!(undo_depth(&mut b), depth + 1);
    let m = call(&mut b, "material_get", json!({"material": "Red"})).unwrap();
    assert_eq!(m["channels"]["diffuse_color"], json!([0.8, 0.05, 0.05]));
    assert_eq!(m["channels"]["roughness"], 0.25);
    assert_eq!(m["channels"]["metallic"], 1.0);
    assert_eq!(m["channels"]["clearcoat_amount"], 0.5);
    assert_eq!(m["shading_model"], "Physically Based");
    // Radians in and out; the panel keeps degrees.
    assert!((m["channels"]["smoothing_angle"].as_f64().unwrap() - FRAC_PI_4).abs() < 1e-12);
    let layer = m["layer"].as_u64().unwrap();
    let NodeAngle(stored) = stored_angle(&b, layer);
    assert!((stored - 45.).abs() < 1e-9);
    // The default material is reachable by name too.
    call(
        &mut b,
        "material_set",
        json!({"diffuse_color": [0.1, 0.2, 0.3]}),
    )
    .unwrap();
    assert_eq!(
        call(&mut b, "material_get", json!({})).unwrap()["channels"]["diffuse_color"],
        json!([0.1, 0.2, 0.3])
    );
    call(&mut b, "undo", json!({"count": 2})).unwrap();
    let back = call(&mut b, "material_get", json!({"material": "Red"})).unwrap();
    assert_eq!(back["channels"], before["channels"]);
}

struct NodeAngle(f64);
fn stored_angle(b: &Bridge, layer: u64) -> NodeAngle {
    match &b.editor.scene.shader_tree.get(layer).unwrap().kind {
        operandi_core::shader_tree::NodeKind::Material(m) => NodeAngle(m.smoothing_angle),
        _ => panic!("a material"),
    }
}

#[test]
fn bad_material_edits_are_refused_and_change_nothing() {
    let mut b = red_cube();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    let e = call(
        &mut b,
        "material_set",
        json!({"material": "Red", "colour": [1, 0, 0]}),
    )
    .unwrap_err();
    assert!(
        e.contains("no channel colour") && e.contains("diffuse_color"),
        "{e}"
    );
    let e = call(
        &mut b,
        "material_set",
        json!({"material": "Red", "roughness": 2}),
    )
    .unwrap_err();
    assert!(e.contains("roughness must be 0 to 1"), "{e}");
    let e = call(
        &mut b,
        "material_set",
        json!({"material": "Blue", "roughness": 0.1}),
    )
    .unwrap_err();
    assert!(e.contains("No material \"Blue\""), "{e}");
    let e = call(&mut b, "material_set", json!({"material": "Red"})).unwrap_err();
    assert!(e.contains("at least one channel"), "{e}");
    assert_eq!(call(&mut b, "scene_info", json!({})).unwrap(), info);
}

#[test]
fn gradient_layer_goes_into_the_material_group_with_its_keys() {
    let mut b = red_cube();
    let depth = undo_depth(&mut b);
    let out = call(
        &mut b,
        "shader_layer_add",
        json!({"type": "gradient", "material": "Red", "input": "Slope", "effect": "Diffuse Color",
               "color_keys": [{"at": FRAC_PI_2, "color": [1, 1, 0]}, {"at": 0, "color": [0, 0, 1], "shape": "linear"}],
               "opacity": 0.5}),
    )
    .unwrap();
    assert_eq!(undo_depth(&mut b), depth + 1);
    let id = out["layer"].as_u64().unwrap();
    let layer = call(&mut b, "shader_tree_get", json!({"layer": id})).unwrap();
    assert_eq!(layer["type"], "Gradient");
    assert_eq!(layer["input"], "Slope");
    assert_eq!(layer["input_unit"], "rad");
    assert_eq!(layer["opacity"], 0.5);
    // Keys come back sorted and in radians.
    let keys = layer["color_keys"].as_array().unwrap();
    assert_eq!(keys[0]["color"], json!([0.0, 0.0, 1.0]));
    assert_eq!(keys[0]["shape"], "linear");
    assert!((keys[1]["at"].as_f64().unwrap() - FRAC_PI_2).abs() < 1e-12);
    // It sits in Red's group, above the Material.
    let red = call(&mut b, "material_get", json!({"material": "Red"})).unwrap();
    assert_eq!(layer["parent"], red["parent"]);
    let tree = call(&mut b, "shader_tree_get", json!({})).unwrap();
    let rows = tree["layers"].as_array().unwrap();
    let at = |id: &Value| rows.iter().position(|r| &r["layer"] == id).unwrap();
    assert!(at(&json!(id)) < at(&red["layer"]));
    // Undo removes it in one step.
    call(&mut b, "undo", json!({})).unwrap();
    assert!(call(&mut b, "shader_tree_get", json!({"layer": id})).is_err());
}

#[test]
fn constant_vertex_map_and_image_layers_take_their_settings() {
    let mut b = red_cube();
    let c = call(
        &mut b,
        "shader_layer_add",
        json!({"type": "constant", "material": "Red", "channels": {"color": [0.2, 0.4, 0.6]}, "effect": "Roughness"}),
    )
    .unwrap()["layer"]
        .clone();
    let c = call(&mut b, "shader_tree_get", json!({"layer": c})).unwrap();
    assert_eq!(c["channels"]["color"], json!([0.2, 0.4, 0.6]));
    assert_eq!(c["effect"], "Roughness");

    let v = call(
        &mut b,
        "shader_layer_add",
        json!({"type": "vertex_map", "map": {"name": "Weight", "kind": "weight"},
               "channels": {"color_2": [1, 0, 0]}}),
    )
    .unwrap()["layer"]
        .clone();
    let v = call(&mut b, "shader_tree_get", json!({"layer": v})).unwrap();
    assert_eq!(v["map"], json!({"name": "Weight", "kind": "weight"}));
    assert_eq!(v["channels"]["color_2"], json!([1.0, 0.0, 0.0]));

    // An Image Map takes a PNG by path; its size comes from the header.
    let png = temp("checker.png");
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend(64u32.to_be_bytes());
    bytes.extend(32u32.to_be_bytes());
    bytes.extend([8, 2, 0, 0, 0]);
    std::fs::write(&png, bytes).unwrap();
    let i = call(
        &mut b,
        "shader_layer_add",
        json!({"type": "image_map", "material": "Red", "image": png.to_string_lossy()}),
    )
    .unwrap()["layer"]
        .clone();
    let i = call(&mut b, "shader_tree_get", json!({"layer": i})).unwrap();
    let image = b
        .editor
        .scene
        .images
        .iter()
        .find(|im| json!(im.id) == i["image"])
        .unwrap();
    assert_eq!((image.width, image.height), (64, 32));

    // A setting the layer doesn't have is refused.
    let e = call(
        &mut b,
        "shader_layer_set",
        json!({"layer": c["layer"], "input": "Slope"}),
    )
    .unwrap_err();
    assert!(e.contains("no input"), "{e}");
    let e = call(&mut b, "shader_layer_add", json!({"type": "sparkles"})).unwrap_err();
    assert!(e.contains("Unknown layer type"), "{e}");
}

#[test]
fn material_edits_survive_save_and_reopen() {
    let mut b = red_cube();
    call(
        &mut b,
        "material_set",
        json!({"material": "Red", "diffuse_color": [0.9, 0.1, 0.1], "metallic": 0.75}),
    )
    .unwrap();
    call(
        &mut b,
        "shader_layer_add",
        json!({"type": "gradient", "material": "Red"}),
    )
    .unwrap();
    let path = temp("materials.opscene");
    call(
        &mut b,
        "scene_save",
        json!({"path": path.to_string_lossy()}),
    )
    .unwrap();
    let mut reopened = Bridge::new();
    call(
        &mut reopened,
        "scene_load",
        json!({"path": path.to_string_lossy()}),
    )
    .unwrap();
    assert_eq!(
        call(&mut reopened, "material_get", json!({"material": "Red"})).unwrap(),
        call(&mut b, "material_get", json!({"material": "Red"})).unwrap()
    );
    assert_eq!(
        call(&mut reopened, "shader_tree_get", json!({})).unwrap(),
        call(&mut b, "shader_tree_get", json!({})).unwrap()
    );
}

#[test]
fn subdivision_set_toggles_polygons_and_sets_levels() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let item = call(&mut b, "scene_info", json!({})).unwrap()["selection"]["item"].clone();
    let depth = undo_depth(&mut b);
    call(
        &mut b,
        "subdivision_set",
        json!({"toggle": "catmull_clark", "catmull_clark_level": 3, "boundary_rule": "smooth"}),
    )
    .unwrap();
    assert_eq!(undo_depth(&mut b), depth + 1);
    let s = call(&mut b, "item_get", json!({"item": item})).unwrap()["subdivision"].clone();
    assert_eq!(s["catmull_clark_polygons"], 6);
    assert_eq!(s["face_polygons"], 0);
    assert_eq!(s["catmull_clark_level"], 3);
    assert_eq!(s["boundary_rule"], "smooth");
    // Tab on Catmull-Clark polygons turns them back into faces.
    call(&mut b, "subdivision_set", json!({"toggle": "subdivision"})).unwrap();
    let s = call(&mut b, "item_get", json!({})).unwrap()["subdivision"].clone();
    assert_eq!(s["face_polygons"], 6);
    // Bad values change nothing.
    assert!(call(&mut b, "subdivision_set", json!({"subdivision_level": 9})).is_err());
    assert!(call(&mut b, "subdivision_set", json!({})).is_err());
    assert!(call(&mut b, "subdivision_set", json!({"toggle": "loop"})).is_err());
    call(&mut b, "undo", json!({"count": 2})).unwrap();
    let s = call(&mut b, "item_get", json!({})).unwrap()["subdivision"].clone();
    assert_eq!(s["face_polygons"], 6);
    assert_eq!(s["catmull_clark_level"], 2);
}

/// Reopening the file the session already has reads the file again: an
/// edit made to it on disk shows up, rather than the old copy staying.
#[test]
fn scene_load_of_the_open_file_reads_it_again() {
    let path = temp("reopen.opscene");
    let p = path.to_string_lossy().to_string();
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    call(&mut b, "scene_save", json!({"path": p})).unwrap();
    // Someone else changes the file.
    let mut other = Bridge::new();
    call(&mut other, "scene_load", json!({"path": p})).unwrap();
    call(&mut other, "primitive_add", json!({"shape": "sphere"})).unwrap();
    call(&mut other, "scene_save", json!({"path": p})).unwrap();
    let before = call(&mut b, "scene_info", json!({})).unwrap()["items"][0]["faces"].clone();
    call(&mut b, "scene_load", json!({"path": p})).unwrap();
    let after = call(&mut b, "scene_info", json!({})).unwrap()["items"][0]["faces"].clone();
    assert_ne!(before, after);
    assert_eq!(
        after,
        call(&mut other, "scene_info", json!({})).unwrap()["items"][0]["faces"]
    );
}

#[test]
fn light_render_takes_the_panel_names_and_on_off() {
    let mut b = Bridge::new();
    call(
        &mut b,
        "command_run",
        json!({"command": "item.add", "args": {"type": "light.point"}}),
    )
    .unwrap();
    let text = Bridge::tool_definitions()
        .into_iter()
        .find(|t| t["name"] == "light_set")
        .unwrap()["inputSchema"]["properties"]["render"]["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(text.starts_with("Default, Yes, No"), "{text}");
    for (given, shown) in [
        ("Yes", "Yes"),
        ("off", "No"),
        ("On", "Yes"),
        ("No", "No"),
        ("Default", "Default"),
    ] {
        call(&mut b, "light_set", json!({"render": given})).unwrap();
        assert_eq!(
            call(&mut b, "item_get", json!({})).unwrap()["light"]["render"],
            shown
        );
    }
    assert!(call(&mut b, "light_set", json!({"render": "Maybe"})).is_err());
}
