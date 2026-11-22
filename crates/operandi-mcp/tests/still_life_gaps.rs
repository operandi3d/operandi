//! Gaps found building a still-life scene through the bridge: item choice
//! in component modes, argument checking, documented arguments, primitive
//! schemas, radians, camera and light properties, smoothing.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};
use std::f64::consts::FRAC_PI_2;

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

fn tool_def(name: &str) -> Value {
    Bridge::tool_definitions()
        .into_iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("{name} is listed"))
}

fn faces(b: &mut Bridge, item: u64) -> usize {
    call(b, "mesh_get", json!({"item": item, "edges": false})).unwrap()["faces"]
        .as_array()
        .unwrap()
        .len()
}

/// 1: `item` makes that mesh active in Polygon mode, so the next
/// primitive goes into it.
#[test]
fn select_item_makes_the_mesh_active_in_component_modes() {
    let mut b = Bridge::new();
    let first = call(&mut b, "scene_info", json!({})).unwrap()["items"][0]["id"]
        .as_u64()
        .unwrap();
    command(&mut b, "mesh.new", json!({})).unwrap();
    let second = call(&mut b, "scene_info", json!({})).unwrap()["selection"]["item"]
        .as_u64()
        .unwrap();
    assert_ne!(first, second);
    let state = call(&mut b, "select", json!({"mode": "polygon", "item": first})).unwrap();
    assert_eq!(state["selection"]["item"], first);
    assert_eq!(state["active"]["item"], first);
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    assert_eq!(faces(&mut b, first), 6);
    assert_eq!(faces(&mut b, second), 0);
    // An unknown select key is refused too.
    let e = call(
        &mut b,
        "select",
        json!({"mode": "polygon", "polygons": [0]}),
    )
    .unwrap_err();
    assert!(e.contains("polygons") && e.contains("faces"), "{e}");
}

/// 2: unknown argument keys are errors naming what the command takes, and
/// nothing changes.
#[test]
fn command_run_refuses_unknown_argument_keys() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let before = call(&mut b, "scene_info", json!({})).unwrap();
    for (name, args, bad) in [
        ("item.transform", json!({"position": [1, 2, 3]}), "position"),
        ("item.transform", json!({"delta": [1, 0, 0]}), "delta"),
        ("item.select", json!({"id": 2}), "id"),
        ("poly.flip", json!({"all": true}), "all"),
    ] {
        let e = command(&mut b, name, args).unwrap_err();
        assert!(e.contains(&format!("\"{bad}\"")), "{e}");
        if name == "item.transform" {
            assert!(e.contains("translation, rotation, scale"), "{e}");
        }
    }
    assert_eq!(e_of(&mut b), before);
    // The documented names work.
    command(&mut b, "item.transform", json!({"translation": [1, 2, 3]})).unwrap();
    let t = &call(&mut b, "scene_info", json!({})).unwrap()["items"][0]["transform"];
    assert_eq!(t["translation"], json!([1.0, 2.0, 3.0]));

    fn e_of(b: &mut Bridge) -> Value {
        call(b, "scene_info", json!({})).unwrap()
    }
}

/// L6: the bridge lists and runs exactly the command registry's headless
/// commands, and its tool schema offers the same names.
#[test]
fn bridge_commands_are_the_registry() {
    use std::collections::BTreeSet;
    let registry: BTreeSet<&str> = operandi_core::command_registry::COMMANDS
        .iter()
        .filter(|e| e.flags.headless)
        .map(|e| e.name)
        .collect();
    let bridge: BTreeSet<&str> = operandi_mcp::bridge::commands().into_iter().collect();
    let specs: BTreeSet<&str> = operandi_mcp::commands::specs().map(|s| s.name).collect();
    assert_eq!(bridge, registry);
    assert_eq!(specs, registry);
    let listed: BTreeSet<String> = tool_def("command_run")["inputSchema"]["properties"]["command"]
        ["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(listed, registry.iter().map(|s| s.to_string()).collect());
    let mut b = Bridge::new();
    let all = call(&mut b, "command_help", json!({})).unwrap();
    let helped: BTreeSet<String> = all["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["command"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(helped, registry.iter().map(|s| s.to_string()).collect());
    // A typed-only command isn't runnable here.
    let refused = call(
        &mut b,
        "command_run",
        json!({"command": "poly.subdivide", "args": {}}),
    );
    assert!(refused.is_err());
}

/// 8: every command's arguments are documented and discoverable.
#[test]
fn command_help_documents_every_command() {
    let commands = operandi_mcp::bridge::commands();
    for name in &commands {
        assert!(
            operandi_mcp::commands::spec(name).is_some(),
            "{name} has no argument spec"
        );
    }
    for spec in operandi_mcp::commands::specs() {
        assert!(
            commands.contains(&spec.name),
            "{} is documented but not runnable",
            spec.name
        );
    }
    let mut b = Bridge::new();
    let one = call(&mut b, "command_help", json!({"command": "item.transform"})).unwrap();
    let args: Vec<&str> = one["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert_eq!(args, ["item", "translation", "rotation", "scale"]);
    assert!(
        one["args"][2]["about"]
            .as_str()
            .unwrap()
            .contains("radians")
    );
    let types = call(&mut b, "command_help", json!({"command": "item.add"})).unwrap();
    assert!(types["types"].as_array().unwrap().len() > 3, "{types}");
    let all = call(&mut b, "command_help", json!({})).unwrap();
    assert_eq!(all["commands"].as_array().unwrap().len(), commands.len());
    // The command_run description carries the argument names too.
    let d = tool_def("command_run")["description"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(d.contains("item.name {name, layer?}"), "{d}");
}

/// 3: primitive_add's schema requires and lists shape, takes any case, and
/// cone takes radius as the same half extents as size.
#[test]
fn primitive_add_schema_and_spellings() {
    let def = tool_def("primitive_add");
    assert_eq!(def["inputSchema"]["required"], json!(["shape"]));
    let shapes = def["inputSchema"]["properties"]["shape"]["enum"]
        .as_array()
        .unwrap();
    for s in ["cube", "Sphere", "cylinder", "Cone", "tube"] {
        assert!(shapes.contains(&json!(s)), "{s}");
    }
    let mut b = Bridge::new();
    call(
        &mut b,
        "primitive_add",
        json!({"shape": "Cone", "params": {"radius": [1, 2, 1]}}),
    )
    .unwrap();
    let m = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let ys: Vec<f64> = m["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v[1].as_f64().unwrap())
        .collect();
    let height =
        ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min);
    assert!(
        (height - 4.).abs() < 1e-9,
        "radius y 2 = half height: {height}"
    );
    let e = call(
        &mut b,
        "primitive_add",
        json!({"shape": "cone", "params": {"diameter": 1}}),
    )
    .unwrap_err();
    assert!(e.contains("diameter") && e.contains("size"), "{e}");
    let e = call(&mut b, "primitive_add", json!({})).unwrap_err();
    assert!(e.contains("cube") && e.contains("tube"), "{e}");
}

/// 10: cube, sphere and cylinder through primitive_add, from Reset values,
/// with the documented size meanings.
#[test]
fn primitive_add_cube_sphere_cylinder() {
    let mut b = Bridge::new();
    let extent = |b: &mut Bridge| -> [f64; 3] {
        let m = call(b, "mesh_get", json!({"edges": false})).unwrap();
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for v in m["vertices"].as_array().unwrap() {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k].as_f64().unwrap());
                hi[k] = hi[k].max(v[k].as_f64().unwrap());
            }
        }
        [0, 1, 2].map(|k| hi[k] - lo[k])
    };
    let s = call(
        &mut b,
        "primitive_add",
        json!({"shape": "cube", "params": {"size": [2, 1, 1], "segments": 2}}),
    )
    .unwrap();
    // FULL lengths; 2 segments per axis = 24 faces.
    assert_eq!(s["active"]["faces"], 24);
    assert_eq!(extent(&mut b), [2., 1., 1.]);
    call(&mut b, "undo", json!({})).unwrap();
    call(
        &mut b,
        "primitive_add",
        json!({"shape": "sphere", "params": {"radius": 1, "sides": 8, "segments": 4}}),
    )
    .unwrap();
    // HALF extents: radius 1 spans 2.
    let e = extent(&mut b);
    assert!((e[1] - 2.).abs() < 1e-9, "{e:?}");
    // 8 sides x 4 bands: 16 triangles at the poles + 16 quads.
    assert_eq!(faces(&mut b, 1), 32);
    call(&mut b, "undo", json!({})).unwrap();
    call(
        &mut b,
        "primitive_add",
        json!({"shape": "cylinder", "params": {"radius": [0.5, 1, 0.5], "sides": 6}}),
    )
    .unwrap();
    let e = extent(&mut b);
    assert!((e[1] - 2.).abs() < 1e-9, "{e:?}");
    assert_eq!(faces(&mut b, 1), 8, "6 sides, 1 row, 2 caps");
    // The cube's remembered segments don't leak into the next call.
    call(&mut b, "undo", json!({})).unwrap();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    assert_eq!(faces(&mut b, 1), 6);
}

/// 9: the bridge takes radians everywhere.
#[test]
fn rotations_are_radians() {
    let mut b = Bridge::new();
    call(
        &mut b,
        "primitive_add",
        json!({"shape": "cube", "params": {"size": [2, 1, 1]}}),
    )
    .unwrap();
    call(&mut b, "select", json!({"mode": "polygon"})).unwrap();
    call(&mut b, "selection_op", json!({"op": "all"})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "TransformRotate", "params": {"delta": [0, 0, FRAC_PI_2]}}),
    )
    .unwrap();
    let m = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let max = |k: usize| {
        m["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v[k].as_f64().unwrap())
            .fold(f64::MIN, f64::max)
    };
    // The 2 m length turned from X onto Y.
    assert!((max(1) - 1.).abs() < 1e-6 && (max(0) - 0.5).abs() < 1e-6);
    // Every angle field in the schemas says radians.
    let text = json!(Bridge::tool_definitions()).to_string();
    assert!(!text.contains("degrees"), "a schema still says degrees");
    assert!(tool_def("tool_run").to_string().contains("rotation_delta"));
}

/// 11: mesh.new takes a name, one undo step.
#[test]
fn mesh_new_takes_a_name() {
    let mut b = Bridge::new();
    command(&mut b, "mesh.new", json!({"name": "Vase"})).unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["items"][1]["name"], "Vase");
    assert_eq!(info["undo"], json!(["New Mesh"]));
}

/// 12: smoothing is readable and settable.
#[test]
fn smoothing_set_and_read() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "sphere"})).unwrap();
    let m = call(&mut b, "mesh_get", json!({})).unwrap();
    assert!((m["smoothing"]["amount"].as_f64().unwrap() - 1.).abs() < 1e-12);
    assert!(
        m["faces"][0].get("smooth").is_none(),
        "no misleading per-face flag"
    );
    command(
        &mut b,
        "smoothing.set",
        json!({"angle": std::f64::consts::FRAC_PI_3, "amount": 0.5, "groups": "off"}),
    )
    .unwrap();
    let m = call(&mut b, "mesh_get", json!({})).unwrap();
    let s = &m["smoothing"];
    assert!((s["angle"].as_f64().unwrap() - std::f64::consts::FRAC_PI_3).abs() < 1e-12);
    assert!((s["amount"].as_f64().unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(s["groups"], "off");
    assert!(command(&mut b, "smoothing.set", json!({"angle": 4.0})).is_err());
}

/// 5: the echoed selection gives the mode and each mode's own list. Edge
/// Bevel leaves its new points in the point list (EBC-B3-03), and the edge
/// list stays remembered while Polygon mode is current.
#[test]
fn selection_echo_lists_every_modes_own_list() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let s = call(&mut b, "select", json!({"mode": "edge", "edges": [[0, 1]]})).unwrap();
    assert_eq!(s["selection"]["mode"], "edge");
    assert_eq!(s["selection"]["edges"].as_array().unwrap().len(), 1);
    assert_eq!(s["selection"]["vertices"], json!([]));
    let s = call(
        &mut b,
        "tool_run",
        json!({"tool": "edge.bevel", "params": {"chamfer": {"offset": 0.1, "segments": 1}}}),
    )
    .unwrap();
    assert!(
        !s["selection"]["vertices"].as_array().unwrap().is_empty(),
        "{s}"
    );
    let edges = s["selection"]["edges"].clone();
    assert!(!edges.as_array().unwrap().is_empty());
    let s = call(&mut b, "select", json!({"mode": "polygon"})).unwrap();
    assert_eq!(s["selection"]["mode"], "polygon");
    assert_eq!(s["selection"]["faces"], json!([]));
    assert_eq!(s["selection"]["edges"], edges);
}

/// SYM-SEL-07: a script pick selects only what it names with Symmetry on.
#[test]
fn script_picks_skip_the_mirror() {
    let mut b = Bridge::new();
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    command(&mut b, "symmetry.set", json!({"axis": "x"})).unwrap();
    let s = call(&mut b, "select", json!({"mode": "vertex", "vertices": [0]})).unwrap();
    assert_eq!(s["selection"]["vertices"], json!([0]), "{s}");
    let s = command(&mut b, "select.symmetrical", json!({})).unwrap();
    assert_eq!(
        s["selection"]["vertices"].as_array().unwrap().len(),
        2,
        "{s}"
    );
}

fn add(b: &mut Bridge, kind: &str) -> u64 {
    command(b, "item.add", json!({"type": kind})).unwrap()["selection"]["item"]
        .as_u64()
        .unwrap()
}

/// c: camera properties through the MCP, in metres and radians, one undo
/// step per call.
#[test]
fn camera_properties_get_and_set() {
    let mut b = Bridge::new();
    let cam = add(&mut b, "camera");
    let got = call(&mut b, "item_get", json!({"item": cam})).unwrap();
    assert_eq!(got["kind"], "camera");
    assert!((got["camera"]["focal_length"].as_f64().unwrap() - 0.05).abs() < 1e-12);
    assert!(
        got["camera_units"]["focal_length"]
            .as_str()
            .unwrap()
            .contains("m")
    );
    let undo = call(&mut b, "scene_info", json!({})).unwrap()["undo"]
        .as_array()
        .unwrap()
        .len();
    call(
        &mut b,
        "camera_set",
        json!({"item": cam, "focal_length": 0.085, "film_width": 0.024, "film_height": 0.036,
               "resolution": [800, 1000], "projection": "perspective", "render_camera": true}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["undo"].as_array().unwrap().len(), undo + 1);
    assert_eq!(
        info["undo"].as_array().unwrap().last().unwrap(),
        "Camera Properties"
    );
    let c = &call(&mut b, "item_get", json!({"item": cam})).unwrap()["camera"];
    assert!((c["focal_length"].as_f64().unwrap() - 0.085).abs() < 1e-12);
    assert_eq!(c["resolution"], json!([800, 1000]));
    assert_eq!(c["render_size"], json!([800, 1000]));
    assert_eq!(c["render_camera"], true);
    // Panel ranges and unknown fields are refused; nothing changes.
    assert!(call(&mut b, "camera_set", json!({"item": cam, "f_stop": 0.1})).is_err());
    let e = call(&mut b, "camera_set", json!({"item": cam, "fov": 1})).unwrap_err();
    assert!(e.contains("fov") && e.contains("angle_of_view"), "{e}");
    call(&mut b, "undo", json!({})).unwrap();
    let c = &call(&mut b, "item_get", json!({"item": cam})).unwrap()["camera"];
    assert!((c["focal_length"].as_f64().unwrap() - 0.05).abs() < 1e-12);
}

/// c: light properties, including changing the type.
#[test]
fn light_properties_get_and_set() {
    let mut b = Bridge::new();
    let types =
        call(&mut b, "command_help", json!({"command": "item.add"})).unwrap()["types"].clone();
    let spot = types
        .as_array()
        .unwrap()
        .iter()
        .find_map(|t| t.as_str().filter(|t| t.contains("spot")))
        .expect("a spot light type")
        .to_string();
    let light = add(&mut b, &spot);
    let got = call(&mut b, "item_get", json!({})).unwrap();
    assert_eq!(got["light"]["type"], "Spot");
    assert!(
        got["light_units"]["cone_angle"]
            .as_str()
            .unwrap()
            .contains("rad")
    );
    call(
        &mut b,
        "light_set",
        json!({"item": light, "intensity": 5.5, "color": [1, 0.8, 0.6],
               "cone_angle": 0.5, "shadow": "ray traced"}),
    )
    .unwrap();
    let l = &call(&mut b, "item_get", json!({"item": light})).unwrap()["light"];
    assert_eq!(l["intensity"], 5.5);
    assert_eq!(l["color"], json!([1.0, 0.8, 0.6]));
    assert_eq!(l["cone_angle"], 0.5);
    call(&mut b, "light_set", json!({"type": "area", "width": 2})).unwrap();
    let l = &call(&mut b, "item_get", json!({"item": light})).unwrap()["light"];
    assert_eq!(l["type"], "Area");
    assert_eq!(l["width"], 2.0);
    let e = call(&mut b, "light_set", json!({"type": "laser"})).unwrap_err();
    assert!(e.contains("Directional"), "{e}");
    let mesh = call(&mut b, "scene_info", json!({})).unwrap()["items"][0]["id"].clone();
    assert!(call(&mut b, "light_set", json!({"item": mesh, "intensity": 1})).is_err());
}

/// a/b: the app-only tools are listed, and the headless sidecar says what
/// they need instead of failing silently.
#[test]
fn app_only_tools_are_listed_and_refused_headless() {
    let mut b = Bridge::new();
    for name in operandi_mcp::bridge::APP_TOOLS {
        tool_def(name);
        let e = call(&mut b, name, json!({})).unwrap_err();
        assert!(e.contains("running app"), "{name}: {e}");
    }
    let capture = tool_def("viewport_capture");
    assert!(capture.to_string().contains("camera"));
    let render = tool_def("render");
    for key in ["width", "height", "samples", "path", "timeout"] {
        assert!(
            render["inputSchema"]["properties"].get(key).is_some(),
            "{key}"
        );
    }
}

#[test]
fn base64_matches_the_rfc_examples() {
    use operandi_mcp::protocol::base64;
    for (plain, coded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(plain.as_bytes()), coded);
    }
}

/// 8: docs/MCP_BRIDGE.md carries the same argument reference as
/// command_help.
#[test]
fn the_bridge_doc_lists_every_command_argument() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/MCP_BRIDGE.md"
    ))
    .unwrap();
    let start = "<!-- command-args -->\n";
    let end = "<!-- /command-args -->";
    let from = doc.find(start).expect("start marker") + start.len();
    let to = doc.find(end).expect("end marker");
    let expected = operandi_mcp::commands::markdown();
    assert!(
        doc[from..to] == expected,
        "docs/MCP_BRIDGE.md is out of date; put this between the markers:\n{expected}"
    );
}

/// Follow-up 1: scene_info reports the document's file and whether it has
/// unsaved changes.
#[test]
fn scene_info_reports_the_document() {
    let mut b = Bridge::new();
    let doc = |b: &mut Bridge| call(b, "scene_info", json!({})).unwrap()["document"].clone();
    assert_eq!(
        doc(&mut b),
        json!({"path": null, "name": null, "modified": false})
    );
    call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    assert_eq!(doc(&mut b)["modified"], true);
    let dir = std::env::temp_dir().join(format!("operandi-doc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bowl.opscene");
    call(&mut b, "scene_save", json!({"path": path, "copy": true})).unwrap();
    assert_eq!(doc(&mut b)["modified"], true, "a copy isn't the document");
    call(&mut b, "scene_save", json!({"path": path})).unwrap();
    let d = doc(&mut b);
    assert_eq!(d["modified"], false);
    assert_eq!(d["name"], "bowl.opscene");
    call(&mut b, "scene_new", json!({})).unwrap();
    call(&mut b, "scene_load", json!({"path": path})).unwrap();
    assert_eq!(doc(&mut b)["path"], json!(path));
    assert_eq!(doc(&mut b)["modified"], false);
}

/// Follow-up 2: mesh_get says where smoothing went.
#[test]
fn mesh_get_documents_the_missing_smooth_flag() {
    let d = tool_def("mesh_get")["description"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        d.contains("smoothing.set") && d.contains("smooth flag"),
        "{d}"
    );
}

/// Follow-up 3: render settings, read and set, one undo step, saved.
#[test]
fn render_settings_get_and_set() {
    let mut b = Bridge::new();
    let got = call(&mut b, "render_settings_get", json!({})).unwrap();
    assert_eq!(got["samples"], 16);
    assert_eq!(got["exposure"], "White Level");
    assert_eq!(got["white_level"], 1.0);
    assert!(
        got["units"]["white_level"]
            .as_str()
            .unwrap()
            .contains("white")
    );
    let cam = add(&mut b, "camera");
    let undo = call(&mut b, "scene_info", json!({})).unwrap()["undo"]
        .as_array()
        .unwrap()
        .len();
    call(
        &mut b,
        "render_settings_set",
        json!({"samples": 128, "white_level": 0.5, "gamma": 2.2, "denoise": false,
               "resolution": [640, 480]}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["undo"].as_array().unwrap().len(), undo + 1);
    assert_eq!(
        info["undo"].as_array().unwrap().last().unwrap(),
        "Render Settings"
    );
    let got = call(&mut b, "render_settings_get", json!({})).unwrap();
    assert_eq!(got["samples"], 128);
    assert_eq!(got["white_level"], 0.5);
    assert_eq!(got["gamma"], 2.2);
    assert_eq!(got["denoise"], false);
    assert_eq!(got["resolution"], json!([640, 480]));
    assert_eq!(got["render_camera"], cam);
    // Refusals change nothing.
    for bad in [
        json!({"samples": 0}),
        json!({"exposure": "bright"}),
        json!({"stops": 2}),
    ] {
        assert!(call(&mut b, "render_settings_set", bad).is_err());
    }
    assert_eq!(
        call(&mut b, "render_settings_get", json!({})).unwrap()["samples"],
        128
    );
    // Saved and read back.
    let dir = std::env::temp_dir().join(format!("operandi-render-settings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("r.opscene");
    call(&mut b, "scene_save", json!({"path": path})).unwrap();
    let mut c = Bridge::new();
    call(&mut c, "scene_load", json!({"path": path})).unwrap();
    assert_eq!(
        call(&mut c, "render_settings_get", json!({})).unwrap()["white_level"],
        0.5
    );
    call(&mut b, "undo", json!({})).unwrap();
    assert_eq!(
        call(&mut b, "render_settings_get", json!({})).unwrap()["samples"],
        16
    );
}

/// Jason's MCP-Q02 answer: switching type keeps shared settings and ones
/// both types use, and resets the rest to the new type's defaults.
#[test]
fn switching_light_type_keeps_what_still_applies() {
    let mut b = Bridge::new();
    let light = add(&mut b, "light.spot");
    call(
        &mut b,
        "light_set",
        json!({"item": light, "intensity": 7, "color": [1, 0.5, 0.25], "radius": 0.3,
               "cone_angle": 0.4, "shadow": "none"}),
    )
    .unwrap();
    // Spot → Point: radius applies to both, the cone doesn't.
    call(&mut b, "light_set", json!({"item": light, "type": "point"})).unwrap();
    let l = call(&mut b, "item_get", json!({"item": light})).unwrap()["light"].clone();
    assert_eq!(l["type"], "Point");
    assert_eq!(l["intensity"], 7.0);
    assert_eq!(l["color"], json!([1.0, 0.5, 0.25]));
    assert_eq!(l["shadow"], "None");
    assert_eq!(l["radius"], 0.3);
    assert_eq!(l["intensity_measures"], "Radiant Intensity");
    // Back to Spot: the cone is the Spot default again, not 0.4.
    call(&mut b, "light_set", json!({"item": light, "type": "spot"})).unwrap();
    let l = call(&mut b, "item_get", json!({"item": light})).unwrap()["light"].clone();
    assert_ne!(l["cone_angle"], 0.4);
    // To Area: radius is reset, the area size is the default; one undo step.
    let name = call(&mut b, "item_get", json!({"item": light})).unwrap()["name"].clone();
    call(&mut b, "light_set", json!({"item": light, "type": "area"})).unwrap();
    let got = call(&mut b, "item_get", json!({"item": light})).unwrap();
    assert_eq!(got["name"], name, "the name stays");
    // Converted: radiance over the default outline gives the same light 1 m out.
    let l = &got["light"];
    let area = l["width"].as_f64().unwrap() * l["height"].as_f64().unwrap();
    assert!(
        (l["intensity"].as_f64().unwrap() * area - 7.0).abs() < 1e-9,
        "{l}"
    );
    assert_eq!(got["light"]["intensity_measures"], "Radiance");
    assert_eq!(
        call(&mut b, "scene_info", json!({})).unwrap()["undo"]
            .as_array()
            .unwrap()
            .last()
            .unwrap(),
        "Light Properties"
    );
}
