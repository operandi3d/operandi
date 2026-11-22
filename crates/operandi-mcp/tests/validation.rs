//! Argument and dispatch failures through the headless bridge.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};

fn assert_refusals(bridge: &mut Bridge, cases: &[(&str, Value, &str)]) {
    let before = bridge.scene_info();
    let scene = bridge.editor.scene.clone();
    for (tool, args, message) in cases {
        assert_eq!(
            bridge.call(tool, args).unwrap_err(),
            *message,
            "{tool} {args}",
        );
        assert_eq!(bridge.scene_info(), before, "{tool} {args}");
        assert_eq!(bridge.editor.scene, scene, "{tool} {args}");
    }
}

#[test]
fn refuses_unknown_tools_and_bad_dispatch_arguments() {
    let mut bridge = Bridge::new();
    assert_refusals(
        &mut bridge,
        &[
            ("missing", json!({}), "Unknown tool missing"),
            (
                "scene_new",
                json!({"empty_mesh": 0}),
                "Expected a boolean, got 0",
            ),
            ("command_run", json!({"command": 1}), "command is required"),
            (
                "command_run",
                json!({"command": "item.channel", "args": []}),
                "args must be an object",
            ),
            (
                "tool_run",
                json!({"tool": "missing"}),
                "Unknown tool missing",
            ),
            (
                "tool_run",
                json!({"tool": "prim.cube", "params": []}),
                "params must be an object",
            ),
            ("redo", json!({}), "Only 0 step(s) to redo"),
            ("undo", json!({"count": -1}), "count must be an integer"),
        ],
    );
}

#[test]
fn rejects_invalid_picks_and_selection_operations() {
    let mut bridge = Bridge::new();
    bridge
        .call("primitive_add", &json!({"shape": "cube"}))
        .unwrap();
    assert_refusals(
        &mut bridge,
        &[
            ("select", json!({"mode": "face"}), "Unknown mode face"),
            (
                "select",
                json!({"mode": "vertex", "vertices": 0}),
                "vertices must be an array of indices",
            ),
            (
                "select",
                json!({"mode": "vertex", "vertices": [4294967296_u64]}),
                "Expected an index, got 4294967296",
            ),
            (
                "select",
                json!({"mode": "edge", "edges": "all"}),
                "edges must be an array of pairs",
            ),
            (
                "select",
                json!({"mode": "edge", "edges": [[0, 1, 2]]}),
                "Each edge is a [a, b] pair",
            ),
            (
                "select",
                json!({"mode": "edge", "edges": [[0, -1]]}),
                "Expected an index, got -1",
            ),
            (
                "select",
                json!({"mode": "polygon", "faces": [0], "add": "yes"}),
                "Expected a boolean, got \"yes\"",
            ),
            ("selection_op", json!({"op": "convert"}), "to is required"),
            (
                "selection_op",
                json!({"op": "spin"}),
                "Unknown selection op spin",
            ),
        ],
    );
}

#[test]
fn validates_item_ids_and_property_targets() {
    let mut bridge = Bridge::new();
    let id = bridge.summary()["active"]["item"].as_u64().unwrap();
    assert_refusals(
        &mut bridge,
        &[
            (
                "item_get",
                json!({"item": -1}),
                "item must be an integer id",
            ),
            ("item_get", json!({"item": 999}), "No item 999"),
            ("mesh_get", json!({"item": 999}), "No item 999"),
            (
                "camera_set",
                json!({"item": id}),
                &format!("Item {id} is not a camera"),
            ),
            (
                "light_set",
                json!({"item": id}),
                &format!("Item {id} is not a light"),
            ),
            (
                "backdrop_set",
                json!({"item": id}),
                &format!("Item {id} is not a backdrop"),
            ),
            (
                "meshop_set",
                json!({}),
                "op is required, written op:<item>:<op>",
            ),
            (
                "meshop_set",
                json!({"op": "op:1"}),
                "op must be written op:<item>:<op>",
            ),
            (
                "meshop_set",
                json!({"op": "op:1:2", "values": []}),
                "values is an object of option names and values",
            ),
        ],
    );

    bridge
        .call("scene_new", &json!({"empty_mesh": false}))
        .unwrap();
    assert_refusals(
        &mut bridge,
        &[
            ("item_get", json!({}), "No active item; pass item"),
            ("mesh_get", json!({}), "No active item"),
            (
                "select",
                json!({"mode": "vertex"}),
                "No active item; pass item",
            ),
        ],
    );
}

#[test]
fn refuses_bad_paths_and_preset_arguments_before_file_access() {
    let mut bridge = Bridge::new();
    assert_refusals(
        &mut bridge,
        &[
            ("scene_import", json!({}), "path is required"),
            (
                "scene_import",
                json!({"path": "scene.OPSCENE"}),
                "scene_import adds interchange files; open .opscene files with scene_load",
            ),
            (
                "scene_import",
                json!({"path": "scene.LXO"}),
                "scene_import can't read .lxo files (their import asks which groups to bring in); use File > Open in the app",
            ),
            (
                "scene_import",
                json!({"path": "scene.txt"}),
                "scene_import reads obj, stl, gltf, glb, geo, ply, fbx, abc, 3mf, usd, usda, usdc, usdz files, not scene.txt",
            ),
            (
                "preset_save",
                json!({"path": "unused", "kind": "mesh"}),
                "kind must be item, assembly, environment, renderSettings or material",
            ),
            (
                "preset_save",
                json!({"path": "unused", "kind": "material"}),
                "node is required for a material preset",
            ),
            (
                "preset_save",
                json!({"path": "unused", "kind": "environment", "meshes": "yes"}),
                "Expected a boolean, got \"yes\"",
            ),
        ],
    );
}

#[test]
fn rejects_invalid_scene_save_arguments_before_touching_files() {
    let mut bridge = Bridge::new();
    let dir = std::env::temp_dir().join(format!("operandi-mcp-bad-save-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let new_path = dir.join("new.opscene");
    let existing_path = dir.join("existing.opscene");
    let existing_bytes = b"existing file";
    std::fs::write(&existing_path, existing_bytes).unwrap();

    for path in [&new_path, &existing_path] {
        let args = json!({"path": path, "copy": "yes"});
        assert_refusals(
            &mut bridge,
            &[(
                "scene_save",
                args.clone(),
                "Expected a boolean, got \"yes\"",
            )],
        );
        let mut request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                 "params": {"name": "scene_save", "arguments": args}});
        assert_eq!(protocol::called_tool(&request), None);
        assert_eq!(
            protocol::handle(&mut bridge, &request),
            Some(protocol::refusal(
                &request,
                "Expected a boolean, got \"yes\""
            )),
        );
        request["params"]["arguments"]["copy"] = json!(false);
        assert_eq!(protocol::called_tool(&request), Some("scene_save"));
    }

    let new_file = new_path.try_exists().unwrap();
    let existing_file = std::fs::read(&existing_path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!new_file, "an invalid save created a file");
    assert_eq!(
        existing_file, existing_bytes,
        "an invalid save changed a file"
    );
}

#[test]
fn rejects_invalid_preset_and_export_arguments_before_writing() {
    let mut bridge = Bridge::new();
    let dir = std::env::temp_dir().join(format!("operandi-mcp-bad-export-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let preset = dir.join("preset.oppreset");
    let export = dir.join("export.obj");
    // Select a mesh so both operations can write when their arguments are valid.
    bridge
        .call("primitive_add", &json!({"shape": "cube"}))
        .unwrap();
    assert_refusals(
        &mut bridge,
        &[
            (
                "preset_save",
                json!({"path": preset, "kind": "item", "meshes": "yes"}),
                "Expected a boolean, got \"yes\"",
            ),
            (
                "command_run",
                json!({"command": "scene.saveAs", "args": {"filename": export, "export": "yes"}}),
                "Expected a boolean, got \"yes\"",
            ),
        ],
    );
    let output_count = std::fs::read_dir(&dir).unwrap().count();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        output_count, 0,
        "an invalid preset save or export created files"
    );
}

#[test]
fn validates_bridge_options_before_applying() {
    let mut bridge = Bridge::new();
    assert_refusals(
        &mut bridge,
        &[
            (
                "tool_run",
                json!({"tool": "edge.bridge", "falloffs": [{"type": "invert"}]}),
                "Bridge has no falloff",
            ),
            (
                "tool_run",
                json!({"tool": "bridge", "params": []}),
                "params must be an object",
            ),
            (
                "tool_run",
                json!({"tool": "edge.bridge", "params": {"bridge": false}}),
                "params.bridge must be an object",
            ),
            (
                "tool_run",
                json!({"tool": "edge.bridge", "params": {"bridge": {"twist": 2147483648_u64}}}),
                "twist is a whole number",
            ),
            (
                "tool_run",
                json!({"tool": "edge.bridge", "params": {"bridge": {"tension": "tight"}}}),
                "Expected a number, got \"tight\"",
            ),
        ],
    );
}

#[test]
fn validates_action_center_element_axes() {
    let mut bridge = Bridge::new();
    for (center, error) in [
        (
            json!([]),
            "action_center is an object {mode, center?, axis?, normal?}",
        ),
        (json!({"axis": "world"}), "action_center needs a mode"),
        (
            json!({"mode": "origin", "axis": "element"}),
            "axis element needs a normal [x, y, z]",
        ),
        (
            json!({"mode": "origin", "axis": "element", "normal": [0, 0, 0]}),
            "axis element needs a non-zero normal",
        ),
        (
            json!({"mode": "origin", "axis": "xy", "normal": [0, 0, 1]}),
            "action_center normal goes with axis element",
        ),
    ] {
        assert_refusals(
            &mut bridge,
            &[(
                "tool_run",
                json!({"tool": "TransformMove", "action_center": center}),
                error,
            )],
        );
    }
}

#[test]
fn validates_falloff_stack_entries() {
    let mut bridge = Bridge::new();
    for (falloffs, error) in [
        (
            json!({}),
            "falloffs is an array of {type, mode?, index?, mix?}",
        ),
        (json!([false]), "each falloff is an object"),
        (
            json!([{}]),
            "each falloff needs a type: vertexMap, invert or selection",
        ),
        (
            json!([{"type": "invert", "mix": true}]),
            "falloff mix is a name",
        ),
        (
            json!([{"type": "invert", "mix": "average"}]),
            "falloff mix is multiply, add, subtract, max or min",
        ),
        (
            json!([{"type": "vertexMap", "mode": "mean"}]),
            "falloff mode is magnitude or component",
        ),
        (
            json!([{"type": "selection", "shape": "custom", "p0": []}]),
            "falloff p0 is a number",
        ),
        (
            json!([{"type": "linear", "symmetric": "middle"}]),
            "falloff symmetric is none, start or end",
        ),
        (
            json!([{"type": "cylinder", "axis": "diagonal"}]),
            "falloff axis is x, y or z",
        ),
    ] {
        assert_refusals(
            &mut bridge,
            &[(
                "tool_run",
                json!({"tool": "TransformMove", "falloffs": falloffs}),
                error,
            )],
        );
    }
}

#[test]
fn rejects_malformed_slice_parameters_after_activation() {
    let mut bridge = Bridge::new();
    bridge
        .call("primitive_add", &json!({"shape": "cube"}))
        .unwrap();
    assert_refusals(
        &mut bridge,
        &[
            (
                "tool_run",
                json!({"tool": "poly.julienne", "params": {"slice": []}}),
                "slice is an object",
            ),
            (
                "tool_run",
                json!({"tool": "poly.julienne", "params": {"slice": {"number": [1, 2]}}}),
                "slice number is [x, y, z]",
            ),
            (
                "tool_run",
                json!({"tool": "Curve Slice", "params": {"slice": {"points": {}}}}),
                "slice points is a list of [x, y, z]",
            ),
            (
                "tool_run",
                json!({"tool": "Curve Slice", "params": {"slice": {"axis": "diagonal"}}}),
                "slice axis is x, y, z, custom or action",
            ),
            (
                "tool_run",
                json!({"tool": "Primitive Slice", "params": {"slice": {"half_size": [1]}}}),
                "slice half_size is [x, y]",
            ),
        ],
    );
}
