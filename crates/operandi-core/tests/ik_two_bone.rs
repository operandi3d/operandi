//! BEHAVIOR-RIG-PHASE6 §1: two-bone IK in a scene (IK-01..08).
use glam::DVec3;
use operandi_core::*;
use serde_json::{Value, json};

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn err(e: &mut Editor, text: &str) -> String {
    command_line::run_line(e, text)
        .expect("headless")
        .expect_err("refused")
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

fn at(v: &Value) -> DVec3 {
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

fn near(a: DVec3, b: DVec3) {
    assert!(a.distance(b) < 1e-3, "{a} vs {b}");
}

/// IK-04's chain: root at the origin, bones 1.0198 bending toward −Z,
/// with two-bone IK applied. Returns the editor, joints and goal.
fn leg() -> (Editor, [u64; 3], u64) {
    let mut e = Editor::default();
    ok(
        &mut e,
        "operandi.skel.add position:0,0,0 name:Hip preserveAlignment:true",
    );
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,1,-0.2 parent:{a} name:Knee preserveAlignment:true"),
    );
    let b = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,2,0 parent:{b} name:Ankle preserveAlignment:true"),
    );
    let c = last(&e);
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.ik.applyTwoBone joints:{a},{b},{c}"),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    let goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name == "Hip1_IKGoal")
        .unwrap()
        .id;
    (e, [a, b, c], goal)
}

fn move_goal(e: &mut Editor, goal: u64, to: DVec3) {
    e.scene.item_mut(goal).unwrap().transform.translation = to;
}

fn joints(e: &Editor) -> Vec<DVec3> {
    query(e)["solvers"][0]["joints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| at(&j["position"]))
        .collect()
}

#[test]
fn ik02_apply_makes_goal_group_and_solver_in_one_step() {
    let mut e = Editor::default();
    ok(
        &mut e,
        "operandi.skel.add position:0,0,0 name:Hip preserveAlignment:true",
    );
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,1,-0.2 parent:{a} preserveAlignment:true"),
    );
    let b = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,2,0 parent:{b} preserveAlignment:true"),
    );
    let c = last(&e);
    let line = format!("operandi.ik.applyTwoBone joints:{a},{b},{c}");
    assert_eq!(err(&mut e, &line), ik_commands::NEEDS_SETUP);
    ok(&mut e, "operandi.anim.setup on:true");
    assert_eq!(
        err(&mut e, &format!("operandi.ik.applyTwoBone joints:{a},{b}")),
        ik_commands::THREE_JOINTS
    );
    assert_eq!(
        err(
            &mut e,
            &format!("operandi.ik.applyTwoBone joints:{a},{c},{b}")
        ),
        ik_commands::THREE_JOINTS
    );
    let steps = e.undo.len();
    ok(&mut e, &line);
    assert_eq!(e.undo.len(), steps + 1);
    let goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name == "Hip1_IKGoal")
        .unwrap();
    assert_eq!(goal.locator, Some(locator::LocatorKind::IkGoal));
    let group = e
        .scene
        .items
        .iter()
        .find(|i| i.name == "Hip1_IKRoot")
        .unwrap()
        .id;
    assert_eq!(goal.parent, Some(group));
    assert_eq!(e.scene.item(a).unwrap().parent, Some(group));
    near(
        e.scene.world_matrix(goal.id).unwrap().w_axis.truncate(),
        DVec3::new(0., 2., 0.),
    );
    // Nothing moved: the goal sits on the end joint.
    near(joints(&e)[2], DVec3::new(0., 2., 0.));
    near(joints(&e)[1], DVec3::new(0., 1., -0.2));
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.animation.rig.pieces.is_empty());
}

#[test]
fn ik03_a_straight_chain_is_refused() {
    let mut e = Editor::default();
    ok(&mut e, "operandi.skel.add position:0,0,0");
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,1,0 parent:{a}"),
    );
    let b = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,2,0 parent:{b}"),
    );
    let c = last(&e);
    ok(&mut e, "operandi.anim.setup on:true");
    assert_eq!(
        err(
            &mut e,
            &format!("operandi.ik.applyTwoBone joints:{a},{b},{c}")
        ),
        ik_commands::STRAIGHT
    );
}

#[test]
fn ik04_the_measured_solves() {
    let (mut e, _, goal) = leg();
    move_goal(&mut e, goal, DVec3::new(0., 1.5, 0.));
    let j = joints(&e);
    near(j[1], DVec3::new(0., 0.75, -0.691));
    near(j[2], DVec3::new(0., 1.5, 0.));
    move_goal(&mut e, goal, DVec3::new(0., 1., 1.));
    let j = joints(&e);
    near(j[1], DVec3::new(0., 1.0196, -0.0196));
    near(j[2], DVec3::new(0., 1., 1.));
    move_goal(&mut e, goal, DVec3::new(0., 3., 0.));
    let j = joints(&e);
    near(j[2], DVec3::new(0., 2.0396, 0.));
    near(j[1], DVec3::new(0., 1.0198, 0.));
    assert_eq!(query(&e)["solvers"][0]["met"], false);
}

#[test]
fn ik04_the_bend_never_flips_as_the_goal_circles() {
    let (mut e, _, goal) = leg();
    for k in 0..36 {
        let t = k as f64 / 36. * std::f64::consts::TAU;
        // Round the root in the YZ... plane's side: the knee stays on −Z
        // of the root-to-goal line, i.e. on the rest side.
        let to = DVec3::new(0.5 * t.cos(), 1.5, 0.5 * t.sin());
        move_goal(&mut e, goal, to);
        let j = joints(&e);
        near(j[2], to);
        let axis = to.normalize();
        let off = j[1] - axis * j[1].dot(axis);
        let rest = DVec3::NEG_Z - axis * DVec3::NEG_Z.dot(axis);
        assert!(off.dot(rest) > 0., "flipped at step {k}: knee {}", j[1]);
    }
}

#[test]
fn ik05_ik06_orient_and_pole_point_the_knee() {
    let (mut e, [_, _, _], goal) = leg();
    move_goal(&mut e, goal, DVec3::new(0., 1.5, 0.));
    let piece = e.scene.animation.rig.pieces[0].id;
    ok(
        &mut e,
        &format!(
            "operandi.rig.set piece:{piece} channel:orient value:{}",
            std::f64::consts::PI
        ),
    );
    assert!(joints(&e)[1].z > 0.6, "turned to +Z: {}", joints(&e)[1]);
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:orient value:0"),
    );
    ok(&mut e, "operandi.ik.pole mode:create");
    let pole = e
        .scene
        .items
        .iter()
        .find(|i| i.name == "Hip1_IKPole")
        .unwrap()
        .id;
    // The pole sits on the bend side; moving it to +X points the knee there.
    assert!(e.scene.world_matrix(pole).unwrap().w_axis.z < -0.5);
    e.scene.item_mut(pole).unwrap().transform.translation = DVec3::new(3., 0.75, 0.);
    let k = joints(&e)[1];
    assert!(k.x > 0.6 && k.z.abs() < 1e-6, "{k}");
    ok(&mut e, "operandi.ik.pole mode:remove");
    assert!(joints(&e)[1].z < -0.6);
}

#[test]
fn ik05_blend_zero_is_the_fk_pose() {
    let (mut e, _, goal) = leg();
    move_goal(&mut e, goal, DVec3::new(0., 1.5, 0.));
    let piece = e.scene.animation.rig.pieces[0].id;
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:blend value:0"),
    );
    near(joints(&e)[2], DVec3::new(0., 2., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:blend value:0.5"),
    );
    let half = joints(&e)[2];
    assert!(half.y < 2. && half.y > 1.5, "{half}");
}

#[test]
fn ik07_ik08_soft_and_stretch() {
    let (mut e, _, goal) = leg();
    let piece = e.scene.animation.rig.pieces[0].id;
    move_goal(&mut e, goal, DVec3::new(0., 3., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:stretch value:1"),
    );
    near(joints(&e)[2], DVec3::new(0., 3., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:stretch value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:soft value:0.1"),
    );
    assert!(joints(&e)[2].y < 2.0396);
}

#[test]
fn ik_saves_and_reopens() {
    let (mut e, _, goal) = leg();
    move_goal(&mut e, goal, DVec3::new(0., 1.5, 0.));
    let before = joints(&e);
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    let b = Editor {
        scene: back,
        ..Default::default()
    };
    for (x, y) in joints(&b).iter().zip(&before) {
        near(*x, *y);
    }
    let q = query(&b);
    assert_eq!(q["solvers"][0]["met"], true);
    assert_eq!(q["solvers"][0]["kind"], json!("ik.twoBone"));
}
