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

/// SK-01/03 through the bridge: setup mode shows and edits setup values.
#[test]
fn setup_mode_through_the_bridge() {
    let mut b = Bridge::new();
    let id = cube(&mut b, [0., 1., 0.]);
    run(
        &mut b,
        "operandi.key.add",
        json!({"channel": format!("{id}:pos.Y"), "frame": 0, "value": 0}),
    );
    run(
        &mut b,
        "operandi.key.add",
        json!({"channel": format!("{id}:pos.Y"), "frame": 10, "value": 4}),
    );
    run(&mut b, "operandi.time.set", json!({"frame": 10}));
    run(&mut b, "operandi.anim.setup", json!({"on": "true"}));
    let pos_y = |b: &mut Bridge| query(b, id)["world"]["position"][1].as_f64().unwrap();
    assert!((pos_y(&mut b) - 1.).abs() < 1e-9, "the setup value");
    run(
        &mut b,
        "transform.channel",
        json!({"name": "pos.Y", "value": 3, "item": id}),
    );
    run(&mut b, "operandi.anim.setup", json!({"on": "false"}));
    assert!((pos_y(&mut b) - 4.).abs() < 1e-9, "the keys again");
    run(
        &mut b,
        "operandi.anim.restoreSetup",
        json!({"items": id.to_string()}),
    );
    assert!(
        (pos_y(&mut b) - 3.).abs() < 1e-9,
        "back to the edited setup value"
    );
}

fn skel(b: &mut Bridge, args: Value) -> Value {
    let out = call(
        b,
        "command_run",
        json!({"command": "operandi.skel.query", "args": args}),
    )
    .unwrap();
    serde_json::from_str(out["value"].as_str().unwrap()).unwrap()
}

#[test]
fn sk10_sk14_skeleton_through_the_bridge() {
    let mut b = Bridge::new();
    run(&mut b, "operandi.skel.add", json!({"position": [0, 1, 0]}));
    let root = skel(&mut b, json!({}))["joints"][0]["id"].as_u64().unwrap();
    run(
        &mut b,
        "operandi.skel.add",
        json!({"position": "1,1,0", "parent": root, "symmetry": "X"}),
    );
    let all = skel(&mut b, json!({"root": root}));
    let joints = all["joints"].as_array().unwrap();
    assert_eq!(joints.len(), 3);
    let names: Vec<&str> = joints.iter().map(|j| j["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Joint1", "Joint1_L", "Joint1_R"]);
    near(&joints[2]["position"], [-1., 1., 0.]);
    let left = joints[1]["id"].as_u64().unwrap();
    run(
        &mut b,
        "operandi.skel.move",
        json!({"joint": left, "position": [2, 1, 0]}),
    );
    near(
        &skel(&mut b, json!({"root": left}))["joints"][0]["position"],
        [2., 1., 0.],
    );
    run(&mut b, "operandi.skel.delete", json!({"joint": left}));
    assert_eq!(
        skel(&mut b, json!({}))["joints"].as_array().unwrap().len(),
        2
    );
}

fn skin(b: &mut Bridge, args: Value) -> Value {
    let out = call(
        b,
        "command_run",
        json!({"command": "operandi.skin.query", "args": args}),
    )
    .unwrap();
    serde_json::from_str(out["value"].as_str().unwrap()).unwrap()
}

#[test]
fn sk02_sk22_sk30_bind_through_the_bridge() {
    let mut b = Bridge::new();
    let mesh = cube(&mut b, [0., 0., 0.]);
    run(
        &mut b,
        "operandi.skel.add",
        json!({"position": [0, -0.3, 0]}),
    );
    let root = skel(&mut b, json!({}))["joints"][0]["id"].as_u64().unwrap();
    // SK-22: a root with two children.
    for x in [-0.3, 0.3] {
        run(
            &mut b,
            "operandi.skel.add",
            json!({"position": [x, 0.3, 0], "parent": root}),
        );
    }
    let bind = json!({"items": mesh.to_string(), "root": root, "type": "distance"});
    let refused = call(
        &mut b,
        "command_run",
        json!({"command": "operandi.skin.bind", "args": bind}),
    )
    .unwrap_err();
    assert!(refused.contains("Bind needs Setup mode"), "{refused}");
    run(&mut b, "operandi.anim.setup", json!({"on": "true"}));
    run(&mut b, "operandi.skin.bind", bind);
    run(&mut b, "operandi.anim.setup", json!({"on": "false"}));
    let q = skin(&mut b, json!({"item": mesh}));
    assert_eq!(q["joints"].as_array().unwrap().len(), 3);
    // SK-30: every point's weights add up to 100%.
    for p in 0..8 {
        let w = skin(&mut b, json!({"item": mesh, "vertex": p}));
        let total: f64 = w["weights"]
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_f64().unwrap())
            .sum();
        assert!((total - 1.).abs() < 1e-6, "point {p}: {w}");
    }
}

fn weight(b: &mut Bridge, args: Value) -> Value {
    let out = call(
        b,
        "command_run",
        json!({"command": "operandi.weight.query", "args": args}),
    )
    .unwrap();
    serde_json::from_str(out["value"].as_str().unwrap()).unwrap()
}

/// SK-33 through the bridge: 50/50 shoulder/forearm, forearm turned 90°
/// about Z at the elbow (the origin): Linear halves the move, Dual
/// Quaternion turns the point 45° keeping its distance. SK-43: weights
/// written with normalize keep the skin's totals at 100%.
#[test]
fn sk33_sk43_skin_maths_and_weights_through_the_bridge() {
    let mut b = Bridge::new();
    let mesh = cube(&mut b, [0., 0., 0.]);
    run(
        &mut b,
        "operandi.skel.add",
        json!({"position": [-0.3, 0, 0], "preserveAlignment": true, "name": "Shoulder"}),
    );
    let shoulder = skel(&mut b, json!({}))["joints"][0]["id"].as_u64().unwrap();
    run(
        &mut b,
        "operandi.skel.add",
        json!({"position": [0, 0, 0], "parent": shoulder, "preserveAlignment": true, "name": "Elbow"}),
    );
    let elbow = skel(&mut b, json!({}))["joints"][1]["id"].as_u64().unwrap();
    run(&mut b, "operandi.anim.setup", json!({"on": "true"}));
    run(
        &mut b,
        "operandi.skin.bind",
        json!({"items": mesh.to_string(), "root": shoulder, "type": "rigid"}),
    );
    run(&mut b, "operandi.anim.setup", json!({"on": "false"}));
    let all: Vec<u32> = (0..8).collect();
    let points = all.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    for map in ["Shoulder1", "Elbow1"] {
        run(
            &mut b,
            "operandi.weight.set",
            json!({"item": mesh, "map": map, "points": points, "value": 0.5}),
        );
    }
    run(
        &mut b,
        "transform.channel",
        json!({"name": "rot.Z", "value": std::f64::consts::FRAC_PI_2, "item": elbow}),
    );
    let at = |b: &mut Bridge| {
        let q = skin(b, json!({"item": mesh, "vertex": 0}));
        let get = |k: &str| -> [f64; 3] {
            let v = &q[k];
            [0, 1, 2].map(|i| v[i].as_f64().unwrap())
        };
        (get("rest"), get("position"))
    };
    let (r, p) = at(&mut b);
    let turned = [-r[1], r[0], r[2]];
    near(
        &json!(p),
        [(r[0] + turned[0]) / 2., (r[1] + turned[1]) / 2., r[2]],
    );
    run(
        &mut b,
        "operandi.skin.set",
        json!({"item": mesh, "interpolation": "dualQuaternion"}),
    );
    let (r, p) = at(&mut b);
    let c = std::f64::consts::FRAC_1_SQRT_2;
    near(&json!(p), [c * r[0] - c * r[1], c * r[0] + c * r[1], r[2]]);
    // SK-43.
    run(
        &mut b,
        "operandi.weight.set",
        json!({"item": mesh, "map": "Elbow1", "points": "3", "value": 0.8, "normalize": true}),
    );
    let w = weight(&mut b, json!({"item": mesh, "vertex": 3}));
    assert!((w["total"].as_f64().unwrap() - 1.).abs() < 1e-9, "{w}");
}

/// Mirror and transfer round trip: mirrored weights match, and a copy
/// takes them back by transfer.
#[test]
fn mirror_and_transfer_through_the_bridge() {
    let mut b = Bridge::new();
    let mesh = cube(&mut b, [0., 0., 0.]);
    run(&mut b, "operandi.skel.add", json!({"position": [0, 0, 0]}));
    let root = skel(&mut b, json!({}))["joints"][0]["id"].as_u64().unwrap();
    run(
        &mut b,
        "operandi.skel.add",
        json!({"position": [0.3, 0, 0], "parent": root, "symmetry": "X"}),
    );
    run(&mut b, "operandi.anim.setup", json!({"on": "true"}));
    run(
        &mut b,
        "operandi.skin.bind",
        json!({"items": mesh.to_string(), "root": root, "type": "distance"}),
    );
    let before = weight(&mut b, json!({"item": mesh, "map": "Joint1_R"}))["values"].clone();
    run(&mut b, "operandi.weight.mirror", json!({"item": mesh}));
    assert_eq!(
        weight(&mut b, json!({"item": mesh, "map": "Joint1_R"}))["values"],
        before
    );
    let copy = cube(&mut b, [0., 0., 0.]);
    run(
        &mut b,
        "operandi.weight.transfer",
        json!({"item": copy, "from": mesh}),
    );
    assert_eq!(
        weight(&mut b, json!({"item": copy, "map": "Joint1_R"}))["values"],
        before
    );
}
