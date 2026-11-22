//! Scene-changing commands merged on 2026-11-11 that `command_run` and
//! `tool_run` now reach: Freeze, Absolute Scaling, weight maps, Node
//! Material import and defaults, interchange export, and the Vertex Map
//! and Invert falloffs.
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

fn faces(b: &mut Bridge) -> usize {
    call(b, "mesh_get", json!({"edges": false})).unwrap()["faces"]
        .as_array()
        .unwrap()
        .len()
}

fn undo_depth(b: &Bridge) -> usize {
    b.editor.undo.len()
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("operandi-mcp-gaps-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// A unit cube whose six polygons are Catmull-Clark (level 2 by default).
fn subd_cube() -> Bridge {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    call(
        &mut b,
        "subdivision_set",
        json!({"toggle": "catmull_clark", "catmull_clark_level": 1}),
    )
    .unwrap();
    b
}

#[test]
fn freeze_turns_a_subd_cube_into_plain_polygons() {
    let mut b = subd_cube();
    let depth = undo_depth(&b);
    command(&mut b, "poly.freeze", json!({})).unwrap();
    assert_eq!(undo_depth(&b), depth + 1);
    // Level 1 splits each quad into four.
    assert_eq!(faces(&mut b), 24);
    let s = call(&mut b, "item_get", json!({})).unwrap()["subdivision"].clone();
    assert_eq!(s["catmull_clark_polygons"], 0);
    assert_eq!(s["face_polygons"], 24);
    // tess false leaves subdivision surfaces alone: nothing to freeze.
    call(&mut b, "undo", json!({})).unwrap();
    assert!(command(&mut b, "poly.freeze", json!({"tess": false})).is_err());
    assert_eq!(faces(&mut b), 6);
    // A successful Freeze's options are remembered for the next bare call.
    command(&mut b, "poly.freeze", json!({"face": 1, "cangle": 0.1})).unwrap();
    assert_eq!(faces(&mut b), 24);
    let preset = &b.editor.freeze_preset;
    assert_eq!(
        preset.curves_as,
        operandi_core::freeze::FreezeCurves::Polylines
    );
    assert!((preset.refinement_degrees - 0.1f64.to_degrees()).abs() < 1e-9);
    // Bad values change nothing; the inert arguments are accepted.
    call(&mut b, "undo", json!({})).unwrap();
    assert!(command(&mut b, "poly.freeze", json!({"face": 7})).is_err());
    assert!(command(&mut b, "poly.freeze", json!({"cangle": 0})).is_err());
    command(
        &mut b,
        "poly.freeze",
        json!({"drill": true, "morphName": "m"}),
    )
    .unwrap();
    assert_eq!(faces(&mut b), 24);
}

#[test]
fn absolute_scaling_scales_to_typed_sizes_and_grab_reads() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let grabbed = command(&mut b, "scale.absolute", json!({"mode": "grab"})).unwrap();
    assert_eq!(grabbed["value"], "Grab Size: 1 × 1 × 1 m");
    let depth = undo_depth(&b);
    command(
        &mut b,
        "scale.absolute",
        json!({"mode": "explicit", "absolute_size_x": 4, "absolute_size_center_x": "low"}),
    )
    .unwrap();
    assert_eq!(undo_depth(&b), depth + 1);
    let mesh = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let xs: Vec<f64> = mesh["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v[0].as_f64().unwrap())
        .collect();
    let (lo, hi) = xs
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), &x| (l.min(x), h.max(x)));
    // Low stays put; Y and Z (0) are left alone.
    assert!(
        (lo + 0.5).abs() < 1e-9 && (hi - 3.5).abs() < 1e-9,
        "{lo} {hi}"
    );
    let grabbed = command(&mut b, "scale.absolute", json!({"mode": "grab"})).unwrap();
    assert_eq!(grabbed["value"], "Grab Size: 4 × 1 × 1 m");
    // The origin choice keeps the geometry's place around the origin.
    command(
        &mut b,
        "scale.absolute",
        json!({"mode": "explicit", "absolute_size_x": 8, "absolute_size_center_x": "origin"}),
    )
    .unwrap();
    let grabbed = command(&mut b, "scale.absolute", json!({"mode": "grab"})).unwrap();
    assert_eq!(grabbed["value"], "Grab Size: 8 × 1 × 1 m");
    // Errors leave the scene alone.
    let depth = undo_depth(&b);
    assert!(command(&mut b, "scale.absolute", json!({})).is_err());
    assert!(command(&mut b, "scale.absolute", json!({"mode": "stretch"})).is_err());
    assert!(
        command(
            &mut b,
            "scale.absolute",
            json!({"mode": "explicit", "absolute_size_center_y": "middle"})
        )
        .is_err()
    );
    assert_eq!(undo_depth(&b), depth);
}

#[test]
fn weight_maps_are_created_and_selected() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    command(&mut b, "vertMap.new", json!({"name": "Mask", "value": 0.5})).unwrap();
    let maps = b.editor.scene.vertex_maps();
    assert!(maps.iter().any(|m| m.name == "Mask"), "{maps:?}");
    assert_eq!(b.editor.selected_weight_map.as_deref(), Some("Mask"));
    command(
        &mut b,
        "vertMap.new",
        json!({"name": "Empty", "init": false}),
    )
    .unwrap();
    assert_eq!(b.editor.selected_weight_map.as_deref(), Some("Empty"));
    command(&mut b, "select.vertexMap", json!({"name": "Mask"})).unwrap();
    assert_eq!(b.editor.selected_weight_map.as_deref(), Some("Mask"));
    command(
        &mut b,
        "select.vertexMap",
        json!({"name": "Mask", "mode": "remove"}),
    )
    .unwrap();
    assert_eq!(b.editor.selected_weight_map, None);
    // UV editing stage 1: type uv makes a UV map on the active mesh.
    command(&mut b, "vertMap.new", json!({"name": "UV", "type": "uv"})).unwrap();
    let mesh = &b.editor.scene.active().unwrap().mesh;
    assert_eq!(
        operandi_core::uv_maps::current_map(mesh).as_deref(),
        Some("UV")
    );
    assert!(command(&mut b, "vertMap.new", json!({"name": "M", "type": "morph"})).is_err());
    assert!(command(&mut b, "vertMap.new", json!({})).is_err());
    assert!(command(&mut b, "select.vertexMap", json!({"mode": "toggle"})).is_err());
}

#[test]
fn node_materials_import_and_defaults_set_and_read() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/node-graphs/noise_ramp.json"
    );
    let mut b = Bridge::new();
    let depth = undo_depth(&b);
    command(
        &mut b,
        "operandi.nodeMaterial.import",
        json!({"filename": fixture}),
    )
    .unwrap();
    assert_eq!(undo_depth(&b), depth + 1);
    let graph = b.editor.scene.shader_tree.node_graphs.graphs[0].id;
    let read = |b: &mut Bridge| {
        command(
            b,
            "operandi.nodeMaterial.setDefault",
            json!({"graph": graph, "input": "Roughness", "value": "?"}),
        )
        .unwrap()["value"]
            .clone()
    };
    assert_eq!(read(&mut b), "0.45");
    command(
        &mut b,
        "operandi.nodeMaterial.setDefault",
        json!({"graph": graph, "input": "Roughness", "value": 0.25}),
    )
    .unwrap();
    assert_eq!(read(&mut b), "0.25");
    command(
        &mut b,
        "operandi.nodeMaterial.setDefault",
        json!({"graph": graph, "input": "Tint", "value": [1, 0.5, 0.5]}),
    )
    .unwrap();
    assert!(
        command(
            &mut b,
            "operandi.nodeMaterial.setDefault",
            json!({"graph": graph, "input": "Nope", "value": 1})
        )
        .is_err()
    );
    let missing = temp("missing.json");
    assert!(
        command(
            &mut b,
            "operandi.nodeMaterial.import",
            json!({"filename": missing})
        )
        .is_err()
    );
}

#[test]
fn scene_save_as_exports_interchange_files_and_leaves_the_document_alone() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let doc = temp("doc.opscene");
    call(&mut b, "scene_save", json!({"path": doc})).unwrap();
    let depth = undo_depth(&b);
    // The extension names the format.
    let obj = temp("cube.obj");
    let _ = std::fs::remove_file(&obj);
    command(&mut b, "scene.saveAs", json!({"filename": obj})).unwrap();
    let text = std::fs::read_to_string(&obj).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("f ")).count(), 6);
    // format: names it (ASCII PLY), with or without export:true.
    let ply = temp("cube.ply");
    let _ = std::fs::remove_file(&ply);
    command(
        &mut b,
        "scene.saveAs",
        json!({"filename": ply, "format": "plyAscii", "export": true}),
    )
    .unwrap();
    let text = std::fs::read_to_string(&ply).unwrap();
    assert!(text.starts_with("ply\nformat ascii"), "{text}");
    assert!(text.contains("element face 6"), "{text}");
    // Nothing joined the history, and the document is still doc.opscene,
    // saved.
    assert_eq!(undo_depth(&b), depth);
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["document"]["path"], json!(doc));
    assert_eq!(info["document"]["modified"], false);
    // Refusals: no filename (no panel here), a plain scene save, a
    // disagreeing format, an unknown one, and USD without the app's
    // exporter.
    assert!(
        command(&mut b, "scene.saveAs", json!({}))
            .unwrap_err()
            .contains("filename")
    );
    let plain = temp("plain.opscene");
    assert!(
        command(&mut b, "scene.saveAs", json!({"filename": plain}))
            .unwrap_err()
            .contains("scene_save")
    );
    assert!(!plain.exists());
    assert!(
        command(
            &mut b,
            "scene.saveAs",
            json!({"filename": obj, "format": "glb"})
        )
        .is_err()
    );
    assert!(
        command(
            &mut b,
            "scene.saveAs",
            json!({"filename": temp("x.bin"), "format": "bin"})
        )
        .is_err()
    );
    let usd = temp("cube.usda");
    assert!(
        command(&mut b, "scene.saveAs", json!({"filename": usd}))
            .unwrap_err()
            .contains("--attach")
    );
    assert!(!usd.exists());
}

/// The highest Y of the active mesh's points.
fn top(b: &mut Bridge) -> f64 {
    call(b, "mesh_get", json!({"edges": false})).unwrap()["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v[1].as_f64().unwrap())
        .fold(f64::MIN, f64::max)
}

#[test]
fn tool_run_applies_vertex_map_and_invert_falloffs_for_that_call() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    command(
        &mut b,
        "vertMap.new",
        json!({"name": "Quarter", "value": 0.25}),
    )
    .unwrap();
    call(&mut b, "select", json!({"mode": "vertex"})).unwrap();
    call(&mut b, "selection_op", json!({"op": "all"})).unwrap();
    let move_up = |b: &mut Bridge, falloffs: Value| {
        call(
            b,
            "tool_run",
            json!({"tool": "TransformMove", "params": {"delta": [0, 1, 0]}, "falloffs": falloffs}),
        )
    };
    // Every point weighs 0.25: the cube moves a quarter of the way.
    move_up(&mut b, json!([{"type": "vertexMap"}])).unwrap();
    assert!((top(&mut b) - 0.75).abs() < 1e-9, "{}", top(&mut b));
    // Inverted on top: 1 - 0.25.
    move_up(
        &mut b,
        json!([{"type": "vertexMap", "mode": "magnitude", "index": 0}, {"type": "invert"}]),
    )
    .unwrap();
    assert!((top(&mut b) - 1.5).abs() < 1e-9, "{}", top(&mut b));
    // The falloff was only for those calls: the next move is full.
    assert!(b.editor.spatial_falloff.is_none() && b.editor.added_falloffs.is_empty());
    move_up(&mut b, Value::Null).unwrap();
    assert!((top(&mut b) - 2.5).abs() < 1e-9, "{}", top(&mut b));
    // Bad stacks change nothing.
    let depth = undo_depth(&b);
    assert!(move_up(&mut b, json!([{"type": "spiral"}])).is_err());
    assert!(move_up(&mut b, json!([{"type": "radial", "start": [0, 0, 0]}])).is_err());
    assert!(move_up(&mut b, json!([{"type": "vertexMap", "mix": "blend"}])).is_err());
    assert!(move_up(&mut b, json!([{"type": "invert", "index": 2}])).is_err());
    assert!(move_up(&mut b, json!({"type": "invert"})).is_err());
    assert_eq!(undo_depth(&b), depth);
    assert!((top(&mut b) - 2.5).abs() < 1e-9);
}
