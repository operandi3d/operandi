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

/// IK-18 through the bridge: four unit bones along +Y.
#[test]
fn ik18_fabrik_through_the_bridge() {
    let mut b = Bridge::new();
    let mut ids = Vec::new();
    for k in 0..5 {
        let id = joint(&mut b, [0., k as f64, 0.], ids.last().copied());
        ids.push(id);
    }
    run(&mut b, "operandi.anim.setup", json!({"on": "true"}));
    run(
        &mut b,
        "operandi.ik.applyChain",
        json!({"joints": format!("{},{}", ids[0], ids[4])}),
    );
    run(&mut b, "operandi.anim.setup", json!({"on": "false"}));
    let goal = read(&mut b, "operandi.ik.query", json!({}))["solvers"][0]["goal"]
        .as_u64()
        .unwrap();
    goal_to(&mut b, goal, [0., 10., 0.]);
    let q = read(&mut b, "operandi.ik.query", json!({}));
    near(&q["solvers"][0]["joints"][4]["position"], [0., 4., 0.]);
    goal_to(&mut b, goal, [2., 2., 0.]);
    let q = read(&mut b, "operandi.ik.query", json!({"tolerance": 0.004}));
    assert_eq!(q["solvers"][0]["met"], true, "{q}");
    let j: Vec<[f64; 3]> = q["solvers"][0]["joints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| [0, 1, 2].map(|i| j["position"][i].as_f64().unwrap()))
        .collect();
    for w in j.windows(2) {
        let d = ((w[0][0] - w[1][0]).powi(2)
            + (w[0][1] - w[1][1]).powi(2)
            + (w[0][2] - w[1][2]).powi(2))
        .sqrt();
        assert!((d - 1.).abs() < 1e-6, "{d}");
    }
}

/// IK-41 through the bridge: the knee's bend stops at its limit.
#[test]
fn ik41_limits_through_the_bridge() {
    let mut b = Bridge::new();
    let (_, goal) = leg(&mut b);
    goal_to(&mut b, goal, [0., 1.5, 0.]);
    let q = read(&mut b, "operandi.ik.query", json!({}));
    let mid = q["solvers"][0]["joints"][1]["joint"].as_u64().unwrap();
    let bend = |q: &Value| {
        let p: Vec<[f64; 3]> = (0..3)
            .map(|k| {
                [0, 1, 2].map(|i| {
                    q["solvers"][0]["joints"][k]["position"][i]
                        .as_f64()
                        .unwrap()
                })
            })
            .collect();
        let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let v = [p[2][0] - p[1][0], p[2][1] - p[1][1], p[2][2] - p[1][2]];
        let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        let len = |w: [f64; 3]| (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
        (dot / (len(u) * len(v))).clamp(-1., 1.).acos().to_degrees()
    };
    let free = bend(&q);
    assert!(free > 60., "{free}");
    run(
        &mut b,
        "operandi.ik.jointSet",
        json!({"joint": mid, "enableLimits": true, "useLimitsX": true, "minX": -30, "maxX": 30}),
    );
    let q = read(&mut b, "operandi.ik.query", json!({}));
    // The rest bend (22.6°) plus the knee's own turn, at most 30°.
    let held = bend(&q);
    let rest = 2. * 0.2f64.atan().to_degrees();
    assert!(held <= rest + 30. + 1e-6 && held < free, "{held}");
    assert_eq!(q["solvers"][0]["met"], false);
}

fn positions(b: &mut Bridge) -> Vec<[f64; 3]> {
    let q = read(b, "operandi.ik.query", json!({}));
    (0..3)
        .map(|k| {
            [0, 1, 2].map(|i| {
                q["solvers"][0]["joints"][k]["position"][i]
                    .as_f64()
                    .unwrap()
            })
        })
        .collect()
}

fn same(a: &[[f64; 3]], b: &[[f64; 3]], what: &str) {
    for (x, y) in a.iter().zip(b) {
        for i in 0..3 {
            assert!((x[i] - y[i]).abs() < 1e-6, "{what}: {x:?} vs {y:?}");
        }
    }
}

/// IK-31 through the bridge: Switch to FK at 24 and back to IK at 48;
/// scrubbing across either switch shows no pop.
#[test]
fn ik31_switch_no_pop_through_the_bridge() {
    let mut b = Bridge::new();
    let (piece, goal) = leg(&mut b);
    let frame = |b: &mut Bridge, f: i64| run(b, "operandi.time.set", json!({"frame": f}));
    frame(&mut b, 0);
    goal_to(&mut b, goal, [0., 1.9, 0.]);
    run(
        &mut b,
        "operandi.key.set",
        json!({"channels": format!("{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z")}),
    );
    frame(&mut b, 24);
    goal_to(&mut b, goal, [0.5, 1.2, 0.]);
    run(
        &mut b,
        "operandi.key.set",
        json!({"channels": format!("{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z")}),
    );
    let mut before = Vec::new();
    for f in [0, 12, 23, 24] {
        frame(&mut b, f);
        before.push(positions(&mut b));
    }
    frame(&mut b, 24);
    run(
        &mut b,
        "operandi.ik.switch",
        json!({"piece": piece, "mode": "fk"}),
    );
    for (k, f) in [0, 12, 23, 24].iter().enumerate() {
        frame(&mut b, *f);
        same(&positions(&mut b), &before[k], &format!("to FK, frame {f}"));
    }
    frame(&mut b, 48);
    let fk48 = positions(&mut b);
    run(
        &mut b,
        "operandi.ik.switch",
        json!({"piece": piece, "mode": "ik"}),
    );
    same(&positions(&mut b), &fk48, "to IK, frame 48");
    frame(&mut b, 24);
    same(&positions(&mut b), &before[3], "to IK, frame 24");
}
