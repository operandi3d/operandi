//! BEHAVIOR-RIG-PHASE6 §2: FABRIK chains in a scene (IK-15..18).
use glam::{DQuat, DVec3};
use operandi_core::*;
use serde_json::Value;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

fn query(e: &Editor) -> Value {
    serde_json::from_str(
        &command_exec::answer(e, "operandi.ik.query", &Default::default())
            .unwrap()
            .unwrap(),
    )
    .unwrap()
}

fn joints(e: &Editor) -> Vec<DVec3> {
    query(e)["solvers"][0]["joints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| {
            let p = &j["position"];
            DVec3::new(
                p[0].as_f64().unwrap(),
                p[1].as_f64().unwrap(),
                p[2].as_f64().unwrap(),
            )
        })
        .collect()
}

/// `n` unit bones along +Y with chain IK; (editor, joints, goal).
fn tail(n: usize) -> (Editor, Vec<u64>, u64) {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for k in 0..=n {
        let mut line = format!("operandi.skel.add position:0,{k},0 name:Tail");
        if let Some(p) = ids.last() {
            line.push_str(&format!(" parent:{p}"));
        }
        ok(&mut e, &line);
        ids.push(last(&e));
    }
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.ik.applyChain joints:{},{}", ids[0], ids[n]),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    let goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name.ends_with("_IKGoal"))
        .unwrap()
        .id;
    (e, ids, goal)
}

fn goal_to(e: &mut Editor, goal: u64, to: DVec3) {
    e.scene.item_mut(goal).unwrap().transform.translation = to;
}

#[test]
fn ik18_the_three_cases_in_a_scene() {
    let (mut e, _, goal) = tail(4);
    goal_to(&mut e, goal, DVec3::new(0., 4., 0.));
    for (i, p) in joints(&e).iter().enumerate() {
        assert!(
            p.distance(DVec3::new(0., i as f64, 0.)) < 1e-9,
            "no change: {p}"
        );
    }
    goal_to(&mut e, goal, DVec3::new(0., 10., 0.));
    for (i, p) in joints(&e).iter().enumerate() {
        assert!(
            p.distance(DVec3::new(0., i as f64, 0.)) < 1e-9,
            "straight: {p}"
        );
    }
    let g = DVec3::new(2., 2., 0.);
    goal_to(&mut e, goal, g);
    let j = joints(&e);
    assert!(j[4].distance(g) <= 0.004 + 1e-9, "{}", j[4]);
    for w in j.windows(2) {
        assert!((w[0].distance(w[1]) - 1.).abs() < 1e-6);
    }
}

#[test]
fn ik15_apply_needs_root_then_end_in_setup() {
    let mut e = Editor::default();
    ok(&mut e, "operandi.skel.add position:0,0,0");
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,1,0 parent:{a}"),
    );
    let b = last(&e);
    let line = format!("operandi.ik.applyChain joints:{b},{a}");
    ok(&mut e, "operandi.anim.setup on:true");
    let err = command_line::run_line(&mut e, &line).unwrap().unwrap_err();
    assert_eq!(err, ik_commands::ROOT_THEN_END);
    let steps = e.undo.len();
    ok(&mut e, &format!("operandi.ik.applyChain joints:{a},{b}"));
    assert_eq!(e.undo.len(), steps + 1);
}

#[test]
fn ik17_pole_blend_and_twist_spread() {
    let (mut e, ids, goal) = tail(4);
    let piece = e.scene.animation.rig.pieces[0].id;
    goal_to(&mut e, goal, DVec3::new(0., 3., 0.));
    // A pole on +Z: the chain bows that way.
    e.scene.new_mesh().unwrap();
    let pole = last(&e);
    e.scene.item_mut(pole).unwrap().transform.translation = DVec3::new(0., 1.5, 5.);
    ok(
        &mut e,
        &format!("operandi.ik.pole piece:{piece} mode:assign pole:{pole}"),
    );
    let j = joints(&e);
    assert!(j[2].z > 0.1, "{j:?}");
    ok(
        &mut e,
        &format!("operandi.ik.pole piece:{piece} mode:remove"),
    );
    // Blend 0: the FK pose.
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:blend value:0"),
    );
    assert!(joints(&e)[4].distance(DVec3::new(0., 4., 0.)) < 1e-9);
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:blend value:1"),
    );
    // Twist the goal 90° about Y: the twist shares out along the chain.
    goal_to(&mut e, goal, DVec3::new(0., 4., 0.));
    let rest: Vec<DVec3> = {
        let s = e.evaluated(evaluate::Purpose::Final);
        ids.iter()
            .map(|id| s.world_matrix(*id).unwrap().x_axis.truncate())
            .collect()
    };
    let t = &mut e.scene.item_mut(goal).unwrap().transform;
    t.rotation =
        t.angles_for(DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2) * t.rotation_quat());
    let s = e.evaluated(evaluate::Purpose::Final);
    let turned = |k: usize| {
        s.world_matrix(ids[k])
            .unwrap()
            .x_axis
            .truncate()
            .angle_between(rest[k])
    };
    assert!(
        (turned(0) - std::f64::consts::FRAC_PI_2 / 4.).abs() < 0.05,
        "the root turns a share: {}",
        turned(0)
    );
    assert!(
        (turned(3) - std::f64::consts::FRAC_PI_2).abs() < 0.05,
        "the last bone all of it: {}",
        turned(3)
    );
}

#[test]
fn chain_ik_saves_and_reopens() {
    let (mut e, _, goal) = tail(6);
    goal_to(&mut e, goal, DVec3::new(2., 3., 1.));
    let before = joints(&e);
    let b = Editor {
        scene: persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap(),
        ..Default::default()
    };
    for (x, y) in joints(&b).iter().zip(&before) {
        assert!(x.distance(*y) < 1e-9);
    }
}
