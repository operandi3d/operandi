//! Phase 4 rigs through the bridge (BEHAVIOR-RIG-PHASE4 §8): the measured
//! numbers of RG-22, RG-28, RG-29 and RG-44 replayed with `command_run`,
//! read back with `operandi.rig.query` (the evaluated world).
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

fn query(b: &mut Bridge, item: u64) -> Value {
    let out = call(
        b,
        "command_run",
        json!({"command": "operandi.rig.query", "args": {"item": item}}),
    )
    .unwrap();
    serde_json::from_str(out["value"].as_str().unwrap()).unwrap()
}

fn near(v: &Value, want: [f64; 3]) {
    for (i, w) in want.iter().enumerate() {
        let got = v[i].as_f64().unwrap();
        assert!((got - w).abs() < 1e-4, "{v} vs {want:?}");
    }
}

/// A cube at `at`; its id.
fn cube(b: &mut Bridge, at: [f64; 3]) -> u64 {
    run(b, "mesh.new", json!({}));
    let added = call(b, "primitive_add", json!({"shape": "cube"})).unwrap();
    let id = added["active"]["item"].as_u64().unwrap();
    for (axis, v) in ["X", "Y", "Z"].iter().zip(at) {
        run(
            b,
            "transform.channel",
            json!({"name": format!("pos.{axis}"), "value": v, "item": id}),
        );
    }
    id
}

#[test]
fn rg28_rg29_weights_and_offset_through_the_bridge() {
    let mut b = Bridge::new();
    let c = cube(&mut b, [5., 5., 5.]);
    let a = cube(&mut b, [2., 0., 0.]);
    let t = cube(&mut b, [0., 4., 0.]);
    run(
        &mut b,
        "operandi.rig.constrain",
        json!({"kind": "position", "items": c.to_string(), "targets": format!("{a},{t}")}),
    );
    let piece = query(&mut b, c)["pieces"][0]["id"].as_u64().unwrap();
    for (wa, wb, want) in [
        (1., 1., [1., 2., 0.]),
        (1., 3., [0.5, 3., 0.]),
        (0., 3., [0., 4., 0.]),
        (0.5, 0., [2., 0., 0.]),
        (0., 0., [5., 5., 5.]),
    ] {
        run(
            &mut b,
            "operandi.rig.set",
            json!({"piece": piece, "channel": "weight.0", "value": wa}),
        );
        run(
            &mut b,
            "operandi.rig.set",
            json!({"piece": piece, "channel": "weight.1", "value": wb}),
        );
        near(&query(&mut b, c)["world"]["position"], want);
    }
    // RG-29: offset X = 1 in world space, (2,0,0) → (3,0,0).
    run(
        &mut b,
        "operandi.rig.set",
        json!({"piece": piece, "channel": "weight.0", "value": 1}),
    );
    run(
        &mut b,
        "operandi.rig.set",
        json!({"piece": piece, "channel": "weight.1", "value": 0}),
    );
    run(
        &mut b,
        "operandi.rig.set",
        json!({"piece": piece, "channel": "offset.pos.X", "value": 1}),
    );
    near(&query(&mut b, c)["world"]["position"], [3., 0., 0.]);
}

#[test]
fn rg44_direction_through_the_bridge() {
    let mut b = Bridge::new();
    let c = cube(&mut b, [0., 0., 0.]);
    let t = cube(&mut b, [1., 0., 0.]);
    run(
        &mut b,
        "operandi.rig.constrain",
        json!({"kind": "direction", "items": c.to_string(), "targets": t.to_string()}),
    );
    let axes = |b: &mut Bridge| query(b, c)["world"]["axes"].clone();
    let a = axes(&mut b);
    near(&a["Z"], [1., 0., 0.]);
    near(&a["Y"], [0., 1., 0.]);
    near(&a["X"], [0., 0., -1.]);
    run(
        &mut b,
        "transform.channel",
        json!({"name": "pos.X", "value": 0, "item": t}),
    );
    run(
        &mut b,
        "transform.channel",
        json!({"name": "pos.Y", "value": 1, "item": t}),
    );
    let a = axes(&mut b);
    near(&a["Z"], [0., 1., 0.]);
    near(&a["X"], [0., 0., 1.]);
    near(&a["Y"], [1., 0., 0.]);
}

#[test]
fn rg22_dynamic_parent_through_the_bridge() {
    let mut b = Bridge::new();
    let child = cube(&mut b, [1., 0., 0.]);
    let parent = cube(&mut b, [0., 0., 5.]);
    run(
        &mut b,
        "operandi.rig.dynamicParentAdd",
        json!({"item": child, "parents": parent.to_string(), "compensate": true}),
    );
    let q = query(&mut b, child);
    near(&q["world"]["position"], [1., 0., 0.]);
    let ch = &q["pieces"][0]["channels"];
    near(
        &json!([ch["offset.pos.X"], ch["offset.pos.Y"], ch["offset.pos.Z"]]),
        [0., 0., -5.],
    );
    run(
        &mut b,
        "transform.channel",
        json!({"name": "pos.Y", "value": 2, "item": parent}),
    );
    near(&query(&mut b, child)["world"]["position"], [1., 2., 0.]);
    run(
        &mut b,
        "operandi.rig.dynamicParentSet",
        json!({"item": child, "parent": 0, "compensate": true}),
    );
    let q = query(&mut b, child);
    near(&q["world"]["position"], [1., 2., 0.]);
    let ch = &q["pieces"][0]["channels"];
    near(
        &json!([ch["offset.pos.X"], ch["offset.pos.Y"], ch["offset.pos.Z"]]),
        [0., 2., 0.],
    );
    run(
        &mut b,
        "transform.channel",
        json!({"name": "pos.Y", "value": 7, "item": parent}),
    );
    near(&query(&mut b, child)["world"]["position"], [1., 2., 0.]);
}

#[test]
fn a_loop_is_refused_through_the_bridge() {
    let mut b = Bridge::new();
    let x = cube(&mut b, [0., 0., 0.]);
    let y = cube(&mut b, [1., 0., 0.]);
    run(
        &mut b,
        "operandi.rig.constrain",
        json!({"kind": "position", "items": x.to_string(), "targets": y.to_string()}),
    );
    let e = call(
        &mut b,
        "command_run",
        json!({"command": "operandi.rig.constrain", "args": {"kind": "position", "items": y.to_string(), "targets": x.to_string()}}),
    )
    .unwrap_err();
    assert!(e.contains("depend on itself"), "{e}");
}

#[test]
fn rg51_rg52_links_through_the_bridge() {
    let mut b = Bridge::new();
    let driven = cube(&mut b, [0., 0., 0.]);
    let driver = cube(&mut b, [0., 0., 0.]);
    run(
        &mut b,
        "operandi.rig.linkAdd",
        json!({"driven": format!("{driven}:scl.Z"), "driver": format!("{driver}:pos.X")}),
    );
    let scl_z = |b: &mut Bridge| -> f64 {
        let w = query(b, driven)["world"]["scale"].clone();
        w[2].as_f64().unwrap()
    };
    for x in [0.5, 2.] {
        run(
            &mut b,
            "transform.channel",
            json!({"name": "pos.X", "value": x, "item": driver}),
        );
        assert!((scl_z(&mut b) - x).abs() < 1e-4, "{x}");
    }
    // RG-52: (0 m → 0%), (10 m → 100%); 5 m gives exactly 50%.
    run(
        &mut b,
        "operandi.rig.linkRemove",
        json!({"driven": format!("{driven}:scl.Z")}),
    );
    run(
        &mut b,
        "operandi.rig.linkAdd",
        json!({"driven": format!("{driven}:scl.Z"), "driver": format!("{driver}:pos.X"), "mode": "relationship"}),
    );
    let piece = query(&mut b, driven)["pieces"][0]["id"].as_u64().unwrap();
    run(
        &mut b,
        "operandi.rig.relationshipKey",
        json!({"piece": piece, "driver": 0, "driven": 0}),
    );
    run(
        &mut b,
        "operandi.rig.relationshipKey",
        json!({"piece": piece, "driver": 10, "driven": 1}),
    );
    run(
        &mut b,
        "transform.channel",
        json!({"name": "pos.X", "value": 5, "item": driver}),
    );
    assert!((scl_z(&mut b) - 0.5).abs() < 1e-4);
}
