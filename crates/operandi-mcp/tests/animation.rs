//! Animation through the bridge (PLAN rule: the MCP drives every animation
//! feature; BEHAVIOR-TIME-KEYS check 9, BEHAVIOR-TIMELINE ball run check
//! 8): time, ranges, keys, slopes, copy/paste, markers, auto key, pending
//! edits and the evaluated values at any time, all with `command_run`.
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

fn run(b: &mut Bridge, command: &str, args: Value) {
    if let Err(e) = call(b, "command_run", json!({"command": command, "args": args})) {
        panic!("{command} {args}: {e}");
    }
}

fn query(b: &mut Bridge, args: Value) -> Value {
    let out = call(
        b,
        "command_run",
        json!({"command": "operandi.anim.query", "args": args}),
    )
    .unwrap();
    serde_json::from_str(out["value"].as_str().unwrap()).unwrap()
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-4, "{a} vs {b}");
}

fn set(b: &mut Bridge, id: u64, name: &str, value: f64) {
    run(
        b,
        "transform.channel",
        json!({"name": name, "value": value, "item": id}),
    );
}

/// The bouncing-ball run (BEHAVIOR-TIMELINE section 10) through the bridge.
#[test]
fn bouncing_ball_through_the_bridge() {
    let mut b = Bridge::new();
    let added = call(&mut b, "primitive_add", json!({"shape": "sphere"})).unwrap();
    let id = added["active"]["item"].as_u64().expect("the sphere's id");
    call(&mut b, "select", json!({"mode": "item", "item": id})).unwrap();
    // Defaults (check 1).
    let q = query(&mut b, json!({}));
    assert_eq!(q["fps"], 24.);
    assert_eq!(q["end"], 120.);
    assert_eq!(q["autoKey"], "animated");
    assert_eq!(q["frame"], 0.);
    // Keys at 0, then Auto Key at 12 and 24 (a squash at 12).
    set(&mut b, id, "pos.Y", 2.);
    let all = format!("{id}:pos.Y,{id}:scl.X,{id}:scl.Y,{id}:scl.Z");
    run(&mut b, "operandi.key.set", json!({"channels": all}));
    run(&mut b, "operandi.time.set", json!({"frame": 12}));
    set(&mut b, id, "pos.Y", 0.);
    set(&mut b, id, "scl.Y", 0.6);
    set(&mut b, id, "scl.X", 1.2);
    set(&mut b, id, "scl.Z", 1.2);
    run(&mut b, "operandi.time.set", json!({"frame": 24}));
    set(&mut b, id, "pos.Y", 2.);
    set(&mut b, id, "scl.Y", 1.);
    set(&mut b, id, "scl.X", 1.);
    set(&mut b, id, "scl.Z", 1.);
    // Linear contact keys.
    let at = |f: u32| {
        ["pos.Y", "scl.X", "scl.Y", "scl.Z"]
            .map(|c| format!("{id}:{c}@{f}"))
            .join(",")
    };
    run(&mut b, "operandi.key.select", json!({"keys": at(12)}));
    run(&mut b, "operandi.key.slope", json!({"type": "linear"}));
    // Copy f0, paste replace at f24.
    run(&mut b, "operandi.key.select", json!({"keys": at(0)}));
    run(&mut b, "operandi.key.copy", json!({}));
    run(&mut b, "operandi.time.keyTime", json!({"frame": 24}));
    run(&mut b, "operandi.key.paste", json!({"mode": "replace"}));
    // Ranges, playback, markers.
    run(
        &mut b,
        "operandi.time.range",
        json!({"start": 0, "end": 23}),
    );
    run(
        &mut b,
        "operandi.time.settings",
        json!({"endBehavior": "repeat"}),
    );
    run(
        &mut b,
        "operandi.marker.add",
        json!({"frame": 12, "scene": true}),
    );
    run(&mut b, "operandi.marker.edit", json!({"label": "Contact"}));
    run(&mut b, "operandi.time.play", json!({}));
    assert_eq!(query(&mut b, json!({}))["playing"], "forwards");
    run(&mut b, "operandi.time.play", json!({"stop": true}));
    // Read the evaluated values back (TL-84's frames).
    let q = query(
        &mut b,
        json!({"channels": format!("{id}:pos.Y,{id}:scl.Y"), "frames": "0,6,12,24"}),
    );
    let y = &q["channels"][0]["values"];
    near(y["0"].as_f64().unwrap(), 2.);
    near(y["6"].as_f64().unwrap(), 1.25);
    near(y["12"].as_f64().unwrap(), 0.);
    near(y["24"].as_f64().unwrap(), 2.);
    near(q["channels"][1]["values"]["12"].as_f64().unwrap(), 0.6);
    assert_eq!(q["channels"][0]["keys"][1]["in"], "linear");
    assert_eq!(q["rangeEnd"], 23.);
    assert_eq!(q["markers"][0]["label"], "Contact");
    // Scrub and read the posed item back through item_get.
    run(&mut b, "operandi.time.set", json!({"frame": 6}));
    let v = call(
        &mut b,
        "command_run",
        json!({"command": "transform.channel", "args": {"name": "pos.Y", "value": "?", "item": id}}),
    )
    .unwrap();
    assert!(v["value"].as_str().unwrap().contains("1.25"), "{v}");
}

/// Check 9: auto key, pending edits, Apply/Discard, stepping and key jumps
/// through the bridge.
#[test]
fn pending_edits_and_stepping_through_the_bridge() {
    let mut b = Bridge::new();
    let added = call(&mut b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let id = added["active"]["item"].as_u64().unwrap();
    call(&mut b, "select", json!({"mode": "item", "item": id})).unwrap();
    run(
        &mut b,
        "operandi.key.transforms",
        json!({"type": "position"}),
    );
    run(&mut b, "operandi.time.step", json!({"frames": 10}));
    run(&mut b, "operandi.key.autoKey", json!({"mode": "off"}));
    set(&mut b, id, "pos.X", 3.);
    let q = query(&mut b, json!({}));
    assert_eq!(q["pending"], json!([format!("{id}:pos.X")]));
    run(&mut b, "operandi.key.apply", json!({}));
    assert_eq!(query(&mut b, json!({}))["pending"], json!([]));
    set(&mut b, id, "pos.X", 9.);
    run(&mut b, "operandi.key.discard", json!({}));
    run(&mut b, "operandi.time.go", json!({"to": "firstKey"}));
    assert_eq!(query(&mut b, json!({}))["frame"], 0.);
    run(&mut b, "operandi.time.go", json!({"to": "nextKey"}));
    let q = query(&mut b, json!({"item": id}));
    assert_eq!(q["frame"], 10.);
    near(q["channels"][0]["now"].as_f64().unwrap(), 3.);
    run(
        &mut b,
        "operandi.time.settings",
        json!({"fps": 30, "display": "smpte"}),
    );
    let q = query(&mut b, json!({}));
    assert_eq!(q["fps"], 30.);
    assert_eq!(q["display"], "smpte");
    // Undo takes back the settings, never the time.
    call(&mut b, "undo", json!({})).unwrap();
    let q = query(&mut b, json!({}));
    assert_eq!(q["fps"], 24.);
    near(q["seconds"].as_f64().unwrap(), 10. / 24.);
}
