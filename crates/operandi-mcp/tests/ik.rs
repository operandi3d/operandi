//! BEHAVIOR-RIG-PHASE6 through the bridge (§7: IK-04, IK-18, IK-31,
//! IK-41).
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

fn read(b: &mut Bridge, command: &str, args: Value) -> Value {
    let out = call(b, "command_run", json!({"command": command, "args": args})).unwrap();
    serde_json::from_str(out["value"].as_str().unwrap()).unwrap()
}

fn near(v: &Value, want: [f64; 3]) {
    for (i, w) in want.iter().enumerate() {
        let got = v[i].as_f64().unwrap();
        assert!((got - w).abs() < 1e-3, "{v} vs {want:?}");
    }
}

/// Adds a joint; its id.
fn joint(b: &mut Bridge, at: [f64; 3], parent: Option<u64>) -> u64 {
    let mut args = json!({"position": at, "preserveAlignment": true});
    if let Some(p) = parent {
        args["parent"] = json!(p);
    }
    run(b, "operandi.skel.add", args);
    let joints = read(b, "operandi.skel.query", json!({}));
    joints["joints"].as_array().unwrap().last().unwrap()["id"]
        .as_u64()
        .unwrap()
}

/// IK-04's chain with two-bone IK; (solver piece, goal).
pub(crate) fn leg(b: &mut Bridge) -> (u64, u64) {
    let a = joint(b, [0., 0., 0.], None);
    let m = joint(b, [0., 1., -0.2], Some(a));
    let e = joint(b, [0., 2., 0.], Some(m));
    run(b, "operandi.anim.setup", json!({"on": "true"}));
    run(
        b,
        "operandi.ik.applyTwoBone",
        json!({"joints": format!("{a},{m},{e}")}),
    );
    run(b, "operandi.anim.setup", json!({"on": "false"}));
    let q = read(b, "operandi.ik.query", json!({}));
    let s = &q["solvers"][0];
    (s["piece"].as_u64().unwrap(), s["goal"].as_u64().unwrap())
}

fn goal_to(b: &mut Bridge, goal: u64, at: [f64; 3]) {
    for (axis, v) in ["X", "Y", "Z"].iter().zip(at) {
        run(
            b,
            "transform.channel",
            json!({"name": format!("pos.{axis}"), "value": v, "item": goal}),
        );
    }
}

#[test]
fn ik04_two_bone_numbers_through_the_bridge() {
    let mut b = Bridge::new();
    let (_, goal) = leg(&mut b);
    for (to, mid, end) in [
        ([0., 1.5, 0.], [0., 0.75, -0.691], [0., 1.5, 0.]),
        ([0., 1., 1.], [0., 1.0196, -0.0196], [0., 1., 1.]),
        ([0., 3., 0.], [0., 1.0198, 0.], [0., 2.0396, 0.]),
    ] {
        goal_to(&mut b, goal, to);
        let q = read(&mut b, "operandi.ik.query", json!({}));
        let j = &q["solvers"][0]["joints"];
        near(&j[1]["position"], mid);
        near(&j[2]["position"], end);
    }
}
