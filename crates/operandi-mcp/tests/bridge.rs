//! End-to-end checks through the MCP message layer, the way an agent drives it.
use operandi_mcp::{Bridge, protocol};
use serde_json::{Value, json};

fn rpc(b: &mut Bridge, id: u64, method: &str, params: Value) -> Value {
    protocol::handle(
        b,
        &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
    )
    .expect("a request gets a response")
}

/// Call a tool; return its structured result, or the error text.
fn call(b: &mut Bridge, name: &str, arguments: Value) -> Result<Value, String> {
    let r = rpc(
        b,
        1,
        "tools/call",
        json!({"name": name, "arguments": arguments}),
    );
    let result = &r["result"];
    if result["isError"] == json!(true) {
        Err(result["content"][0]["text"].as_str().unwrap().to_string())
    } else {
        Ok(result["structuredContent"].clone())
    }
}

fn counts(state: &Value) -> (u64, u64, u64) {
    let a = &state["active"];
    (
        a["vertices"].as_u64().unwrap(),
        a["edges"].as_u64().unwrap(),
        a["faces"].as_u64().unwrap(),
    )
}

fn cube(b: &mut Bridge) {
    call(
        b,
        "tool_run",
        json!({"tool": "prim.cube", "params": {"size": [2, 2, 2]}}),
    )
    .unwrap();
}

#[test]
fn handshake_lists_tools_and_negotiates_version() {
    let mut b = Bridge::new();
    let init = rpc(
        &mut b,
        1,
        "initialize",
        json!({"protocolVersion": "2025-03-26", "capabilities": {},
               "clientInfo": {"name": "test", "version": "0"}}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(init["result"]["serverInfo"]["name"], "operandi-mcp");
    let unknown = rpc(
        &mut b,
        2,
        "initialize",
        json!({"protocolVersion": "1999-01-01"}),
    );
    assert_eq!(
        unknown["result"]["protocolVersion"],
        protocol::PROTOCOL_VERSIONS[0]
    );
    // Notifications get no response.
    assert!(
        protocol::handle(
            &mut b,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .is_none()
    );
    let tools = rpc(&mut b, 3, "tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for name in [
        "scene_info",
        "mesh_get",
        "select",
        "tool_run",
        "command_run",
        "undo",
        "redo",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    assert_eq!(rpc(&mut b, 4, "nope", json!({}))["error"]["code"], -32601);
    assert_eq!(
        rpc(&mut b, 5, "tools/call", json!({"name": "nope"}))["error"]["code"],
        -32602
    );
    let parse = protocol::handle_line(&mut b, "{not json").unwrap();
    assert_eq!(parse["error"]["code"], -32700);
}

#[test]
fn cube_extrude_bevel_with_undo_and_redo() {
    let mut b = Bridge::new();
    assert_eq!(
        call(&mut b, "scene_info", json!({})).unwrap()["undo"],
        json!([])
    );
    cube(&mut b);
    let mesh = call(&mut b, "mesh_get", json!({})).unwrap();
    assert_eq!(mesh["vertices"].as_array().unwrap().len(), 8);
    assert_eq!(mesh["faces"].as_array().unwrap().len(), 6);
    assert_eq!(mesh["edges"].as_array().unwrap().len(), 12);
    assert!(mesh["vertices"].as_array().unwrap().iter().all(|v| {
        v.as_array()
            .unwrap()
            .iter()
            .all(|c| c.as_f64().unwrap().abs() == 1.)
    }));

    let selected = call(&mut b, "select", json!({"mode": "polygon", "faces": [5]})).unwrap();
    assert_eq!(selected["selection"]["faces"], json!([5]));
    let before = call(&mut b, "mesh_get", json!({})).unwrap();
    let extruded = call(
        &mut b,
        "tool_run",
        json!({"tool": "poly.extrude", "params": {"extent": [0, 0.5, 0]}}),
    )
    .unwrap();
    assert_eq!(counts(&extruded), (12, 20, 10));
    let after_extrude = call(&mut b, "mesh_get", json!({})).unwrap();
    let beveled = call(
        &mut b,
        "tool_run",
        json!({"tool": "poly.bevel", "params": {"inset": 0.1, "shift": 0.2}}),
    )
    .unwrap();
    assert_eq!(counts(&beveled), (16, 28, 14));
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(
        info["undo"],
        json!(["Cube", "Selection", "Extrude", "Bevel"])
    );
    assert_eq!(info["live_tool"], Value::Null);

    // Undo twice returns the exact pre-extrude mesh; Redo replays Extrude.
    let undone = call(&mut b, "undo", json!({"count": 2})).unwrap();
    assert_eq!(undone["redo"], 2);
    assert_eq!(call(&mut b, "mesh_get", json!({})).unwrap(), before);
    call(&mut b, "redo", json!({})).unwrap();
    assert_eq!(call(&mut b, "mesh_get", json!({})).unwrap(), after_extrude);
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["redo"], json!(["Bevel"]));
}

#[test]
fn failed_calls_change_nothing() {
    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [5]})).unwrap();
    let before = call(&mut b, "scene_info", json!({})).unwrap();
    // The tool activates, then the bad parameter fails: no live tool remains.
    let err = call(
        &mut b,
        "tool_run",
        json!({"tool": "poly.extrude", "params": {"bogus": 1}}),
    )
    .unwrap_err();
    assert!(err.contains("bogus"), "{err}");
    assert!(call(&mut b, "select", json!({"mode": "polygon", "faces": [99]})).is_err());
    assert!(call(&mut b, "undo", json!({"count": 50})).is_err());
    assert!(call(&mut b, "command_run", json!({"command": "edge.addLoop"})).is_err());
    assert_eq!(call(&mut b, "scene_info", json!({})).unwrap(), before);
}

#[test]
fn commands_selection_ops_and_items() {
    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [5]})).unwrap();
    let grown = call(&mut b, "selection_op", json!({"op": "grow"})).unwrap();
    assert_eq!(grown["selection"]["faces"].as_array().unwrap().len(), 5);
    let all = call(
        &mut b,
        "selection_op",
        json!({"op": "convert", "to": "edge"}),
    )
    .unwrap();
    assert_eq!(all["selection"]["mode"], "edge");

    call(&mut b, "select", json!({"mode": "polygon", "faces": [5]})).unwrap();
    let tripled = call(&mut b, "command_run", json!({"command": "poly.triple"})).unwrap();
    assert_eq!(counts(&tripled), (8, 13, 7));

    call(
        &mut b,
        "command_run",
        json!({"command": "item.rename", "args": {"name": "Base"}}),
    )
    .unwrap();
    call(
        &mut b,
        "command_run",
        json!({"command": "item.transform", "args": {"translation": [0, 3, 0]}}),
    )
    .unwrap();
    let world = call(&mut b, "mesh_get", json!({"world": true, "edges": false})).unwrap();
    assert!(
        world["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v[1].as_f64().unwrap() >= 2.)
    );
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["items"][0]["name"], "Base");
    assert_eq!(
        info["items"][0]["transform"]["translation"],
        json!([0.0, 3.0, 0.0])
    );

    let fresh = call(&mut b, "scene_new", json!({})).unwrap();
    assert_eq!(counts(&fresh), (0, 0, 0));
    assert_eq!(fresh["undo"], 0);
}

#[test]
fn save_and_load_round_trip() {
    let dir = std::env::temp_dir().join(format!("operandi-mcp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cube.opscene");
    let path = path.to_str().unwrap();
    let mut b = Bridge::new();
    cube(&mut b);
    let mesh = call(&mut b, "mesh_get", json!({})).unwrap();
    call(&mut b, "scene_save", json!({"path": path})).unwrap();
    call(&mut b, "scene_new", json!({})).unwrap();
    let loaded = call(&mut b, "scene_load", json!({"path": path})).unwrap();
    assert_eq!(loaded["undo"], 0);
    assert_eq!(call(&mut b, "mesh_get", json!({})).unwrap(), mesh);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn tweak_moves_one_element_and_keeps_the_selection() {
    let mut b = Bridge::new();
    cube(&mut b);
    call(&mut b, "select", json!({"mode": "polygon", "faces": [2]})).unwrap();
    let before = call(&mut b, "mesh_get", json!({})).unwrap();
    let point = |m: &Value, v: usize| -> Vec<f64> {
        m["vertices"][v]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_f64().unwrap())
            .collect()
    };
    let state = call(
        &mut b,
        "tool_run",
        json!({"tool": "tweak", "params": {"tweak": {"vertex": 0}, "delta": [0.5, 0, 0]}}),
    )
    .unwrap();
    assert_eq!(state["selection"]["faces"], json!([2]));
    let after = call(&mut b, "mesh_get", json!({})).unwrap();
    let mut expected = point(&before, 0);
    expected[0] += 0.5;
    assert_eq!(point(&after, 0), expected);
    for v in 1..8 {
        assert_eq!(point(&after, v), point(&before, v));
    }
    assert_eq!(after["faces"], before["faces"]);
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(
        info["undo"].as_array().unwrap().last(),
        Some(&json!("Element Move"))
    );
    // A bad element is refused and changes nothing.
    assert!(
        call(
            &mut b,
            "tool_run",
            json!({"tool": "tweak", "params": {"tweak": {"polygon": 99}, "delta": [1, 0, 0]}}),
        )
        .is_err()
    );
    assert_eq!(call(&mut b, "mesh_get", json!({})).unwrap(), after);
}

#[test]
fn tweak_options_rigid_moves_the_whole_piece() {
    let mut b = Bridge::new();
    cube(&mut b);
    let before = call(&mut b, "mesh_get", json!({})).unwrap();
    call(
        &mut b,
        "tool_run",
        json!({"tool": "tweak", "params": {
            "tweak": {"vertex": 0},
            "tweak_options": {"connect": "Rigid Connections"},
            "delta": [0.5, 0, 0]
        }}),
    )
    .unwrap();
    let after = call(&mut b, "mesh_get", json!({})).unwrap();
    for v in 0..8 {
        let x = |m: &Value| m["vertices"][v][0].as_f64().unwrap();
        assert!((x(&after) - x(&before) - 0.5).abs() < 1e-12, "point {v}");
    }
    assert!(
        call(
            &mut b,
            "tool_run",
            json!({"tool": "tweak", "params": {"tweak": {"vertex": 0}, "tweak_options": {"connect": "Glue"}}}),
        )
        .is_err()
    );
}

#[test]
fn primitive_add_makes_every_numeric_primitive() {
    let mut b = Bridge::new();
    let cases = [
        (
            json!({"shape": "torus", "params": {"sides": 8, "segments": 4}}),
            (32, 32),
        ),
        (
            json!({"shape": "capsule", "params": {"sides": 8, "segments": 2, "end_segments": 2, "ends": "quad_grid"}}),
            (42, 40),
        ),
        (
            json!({"shape": "arc", "params": {"segments": 4, "end": [0, 0, -1]}}),
            (6, 1),
        ),
        (
            json!({"shape": "tube", "params": {"points": [[0, 0, 0], [0, 1, 0]], "sides": 4, "segments": 1, "radius": 0.25}}),
            (8, 6),
        ),
        (
            json!({"shape": "ellipsoid", "params": {"sides": 8, "segments": 4}}),
            (26, 32),
        ),
        (
            json!({"shape": "cone", "params": {"sides": 8, "axis": "z"}}),
            (9, 9),
        ),
    ];
    let (mut points, mut faces) = (0, 0);
    for (i, (args, (p, f))) in cases.into_iter().enumerate() {
        let state =
            call(&mut b, "primitive_add", args.clone()).unwrap_or_else(|e| panic!("{args}: {e}"));
        points += p;
        faces += f;
        let (v, _, n) = counts(&state);
        assert_eq!((v, n), (points, faces), "{args}");
        // One undo step each (Cone's Drop after Apply records nothing).
        assert_eq!(state["undo"], i as u64 + 1, "{args}");
    }
    // The torus followed the spec's Reset values plus the given counts.
    let mesh = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
    let first: Vec<f64> = mesh["vertices"][0]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert!((first[0]).abs() < 1e-12 && first[1] == 0. && (first[2] + 0.25).abs() < 1e-12);
    let undo = call(&mut b, "scene_info", json!({})).unwrap()["undo"].clone();
    assert_eq!(
        undo,
        json!(["Torus", "Capsule", "Arc", "Tube", "Ellipsoid", "Cone"])
    );
}

#[test]
fn primitive_add_refuses_bad_settings_without_changing_anything() {
    let mut b = Bridge::new();
    for args in [
        json!({"shape": "torus", "params": {"colour": 1}}),
        json!({"shape": "tube", "params": {"points": [[0, 0, 0]]}}),
        json!({"shape": "capsule", "params": {"radius": [0, 1, 1]}}),
        json!({"shape": "teapot"}),
    ] {
        assert!(
            call(&mut b, "primitive_add", args.clone()).is_err(),
            "{args}"
        );
    }
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["undo"], json!([]));
    assert_eq!(info["items"][0]["vertices"], 0);
}

/// The seven NPR Tube oracles end to end through `primitive_add` and
/// `mesh_get`, as an agent would make them.
#[test]
fn primitive_add_tube_matches_the_spec_oracles() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specifications/new-primitives-2026-11-03/ORACLES.json"
    );
    let oracles: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let tubes: Vec<_> = oracles.iter().filter(|o| o["tool"] == "Tube").collect();
    assert_eq!(tubes.len(), 7);
    for o in tubes {
        let id = o["id"].as_str().unwrap();
        let mut params = serde_json::Map::new();
        for (k, v) in o["settings"].as_object().unwrap() {
            let flag = || json!(v.as_f64().unwrap() != 0.);
            let (name, value) = match k.as_str() {
                "uvs" => ("make_uvs", flag()),
                "caps" => ("caps", flag()),
                "closed" => ("closed", flag()),
                "length" => ("by_length", flag()),
                // The bridge takes radians.
                "twist_degrees" => ("twist", json!(v.as_f64().unwrap().to_radians())),
                "control_points" => ("points", v.clone()),
                other => (other, v.clone()),
            };
            params.insert(name.into(), value);
        }
        let mut b = Bridge::new();
        call(
            &mut b,
            "primitive_add",
            json!({"shape": "tube", "params": params}),
        )
        .unwrap();
        let mesh = call(&mut b, "mesh_get", json!({"edges": false})).unwrap();
        let e = &o["expected"];
        let got: Vec<Vec<f64>> = serde_json::from_value(mesh["vertices"].clone()).unwrap();
        let want: Vec<Vec<f64>> = serde_json::from_value(e["points"].clone()).unwrap();
        // By Length spacing is provisional (NPR-Q08): within 6 mm.
        let tol = if id.ends_with("bylength") {
            0.006
        } else {
            1e-5
        };
        assert_eq!(got.len(), want.len(), "{id}");
        for (g, w) in got.iter().zip(&want) {
            assert!(
                g.iter().zip(w).all(|(a, b)| (a - b).abs() <= tol),
                "{id}: {g:?} vs {w:?}"
            );
        }
        let faces: Vec<Value> = mesh["faces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["vertices"].clone())
            .collect();
        assert_eq!(Value::from(faces), e["polygons"], "{id}");
    }
}

/// Drop Action through the bridge: Match Position, Parent, a refused loop
/// and Place on a cube, each one undo step.
#[test]
fn item_drop_runs_drop_actions() {
    let mut b = Bridge::new();
    cube(&mut b); // item 1, a 2×2×2 cube at the origin
    call(&mut b, "command_run", json!({"command": "mesh.new"})).unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    let ids: Vec<u64> = info["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_u64().unwrap())
        .collect();
    let (dest, src) = (ids[0], ids[1]);
    let run = |b: &mut Bridge, args: Value| {
        call(
            b,
            "command_run",
            json!({"command": "item.drop", "args": args}),
        )
    };
    call(
        &mut b,
        "command_run",
        json!({"command": "item.transform", "args": {"item": dest, "translation": [3, 0, 0]}}),
    )
    .unwrap();
    run(
        &mut b,
        json!({"action": "match_position", "sources": [src], "destination": dest}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(
        info["items"][1]["transform"]["translation"],
        json!([3.0, 0.0, 0.0])
    );
    run(
        &mut b,
        json!({"action": "parent", "sources": [src], "destination": dest}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["items"][1]["parent"], json!(dest));
    // Parenting the parent to its child is refused.
    assert!(
        run(
            &mut b,
            json!({"action": "parent", "sources": [dest], "destination": src})
        )
        .is_err()
    );
    assert!(run(&mut b, json!({"action": "bogus", "destination": dest})).is_err());
    // Place: straight down onto the cube's top at (3.5, 1, 0).
    run(
        &mut b,
        json!({"action": "place", "sources": [src], "destination": dest,
               "ray_origin": [3.5, 10, 0], "ray_direction": [0, -1, 0]}),
    )
    .unwrap();
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    // Under the cube at (3,0,0) without rotation: local (0.5, 1, 0).
    let t = &info["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == json!(src))
        .unwrap()["transform"]["translation"];
    for (k, want) in [0.5, 1.0, 0.0].into_iter().enumerate() {
        assert!((t[k].as_f64().unwrap() - want).abs() < 1e-9, "{t}");
    }
}

/// Scripts give percents as 0–1, as in the name map: `percent: 0.25` puts
/// the Subdivide point a quarter of the way along the edge.
#[test]
fn edge_subdivide_percent_is_a_fraction() {
    let mut b = Bridge::new();
    cube(&mut b);
    let mesh = call(&mut b, "mesh_get", json!({})).unwrap();
    let v = |m: &Value, i: usize| -> [f64; 3] {
        let p = m["vertices"][i].as_array().unwrap();
        [0, 1, 2].map(|k| p[k].as_f64().unwrap())
    };
    let (p0, p1) = (v(&mesh, 0), v(&mesh, 1));
    call(&mut b, "select", json!({"mode": "edge", "edges": [[0, 1]]})).unwrap();
    call(
        &mut b,
        "command_run",
        json!({"command": "edge.subdivide", "args": {"count": 1, "percent": 0.25}}),
    )
    .unwrap();
    let after = call(&mut b, "mesh_get", json!({})).unwrap();
    let new = v(&after, 8);
    for k in 0..3 {
        let expected = p0[k] + (p1[k] - p0[k]) * 0.25;
        let reversed = p1[k] + (p0[k] - p1[k]) * 0.25;
        assert!(
            (new[k] - expected).abs() < 1e-9 || (new[k] - reversed).abs() < 1e-9,
            "{new:?} vs {p0:?}..{p1:?}"
        );
    }
}

#[test]
fn presets_save_and_load_as_one_step() {
    let dir = std::env::temp_dir().join(format!("operandi-mcp-preset-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cube.oppreset");
    let path = path.to_str().unwrap();
    let mut b = Bridge::new();
    cube(&mut b);
    let before = call(&mut b, "scene_info", json!({})).unwrap();
    let saved = call(&mut b, "preset_save", json!({"path": path, "kind": "item"})).unwrap();
    assert_eq!(saved["kind"], "item");
    // Saving is not an edit.
    assert_eq!(call(&mut b, "scene_info", json!({})).unwrap(), before);
    assert!(
        call(
            &mut b,
            "preset_save",
            json!({"path": path, "kind": "material"})
        )
        .is_err()
    );

    call(&mut b, "scene_new", json!({"empty_mesh": false})).unwrap();
    let loaded = call(&mut b, "preset_load", json!({"path": path})).unwrap();
    assert_eq!(loaded["undo"], 1);
    let info = call(&mut b, "scene_info", json!({})).unwrap();
    assert_eq!(info["items"].as_array().unwrap().len(), 1);
    // A scene file is not a preset.
    let scene = dir.join("plain.opscene");
    call(
        &mut b,
        "scene_save",
        json!({"path": scene.to_str().unwrap()}),
    )
    .unwrap();
    let error = call(
        &mut b,
        "preset_load",
        json!({"path": scene.to_str().unwrap()}),
    )
    .unwrap_err();
    assert!(error.contains("not a preset"), "{error}");
}
