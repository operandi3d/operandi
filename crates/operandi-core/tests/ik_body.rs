//! BEHAVIOR-RIG-PHASE6 §3: full-body IK (IK-20..27).
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;
use std::collections::HashMap;

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

/// A simple biped (hips, spine, head, legs, arms), joint ids by name.
pub fn biped() -> (Editor, HashMap<&'static str, u64>) {
    let mut e = Editor::default();
    let mut ids: HashMap<&'static str, u64> = HashMap::new();
    let parts: [(&str, [f64; 3], Option<&str>); 13] = [
        ("hips", [0., 1., 0.], None),
        ("spine", [0., 1.5, 0.], Some("hips")),
        ("head", [0., 2., 0.], Some("spine")),
        ("hipL", [0.2, 1., 0.], Some("hips")),
        ("kneeL", [0.2, 0.5, 0.05], Some("hipL")),
        ("footL", [0.2, 0., 0.], Some("kneeL")),
        ("hipR", [-0.2, 1., 0.], Some("hips")),
        ("kneeR", [-0.2, 0.5, 0.05], Some("hipR")),
        ("footR", [-0.2, 0., 0.], Some("kneeR")),
        ("armL", [0.3, 1.5, 0.], Some("spine")),
        ("elbowL", [0.6, 1.5, -0.05], Some("armL")),
        ("handL", [0.9, 1.5, 0.], Some("elbowL")),
        ("armR", [-0.3, 1.5, 0.], Some("spine")),
    ];
    for (name, at, parent) in parts {
        let mut line = format!(
            "operandi.skel.add position:{},{},{} name:{name} preserveAlignment:true",
            at[0], at[1], at[2]
        );
        if let Some(p) = parent {
            line.push_str(&format!(" parent:{}", ids[p]));
        }
        ok(&mut e, &line);
        ids.insert(name, e.scene.items.last().unwrap().id);
    }
    (e, ids)
}

pub fn world(e: &Editor, id: u64) -> DVec3 {
    e.evaluated(evaluate::Purpose::Final)
        .world_matrix(id)
        .unwrap()
        .w_axis
        .truncate()
}

/// Assigned, feet pinned, a goal on the left hand; (editor, ids, goal, piece).
pub fn rigged() -> (Editor, HashMap<&'static str, u64>, u64, u64) {
    let (mut e, ids) = biped();
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.ik.assign joints:{}", ids["hips"]),
    );
    for f in ["footL", "footR"] {
        ok(
            &mut e,
            &format!("operandi.ik.addTask joint:{} mode:position", ids[f]),
        );
    }
    ok(
        &mut e,
        &format!("operandi.ik.addGoal joint:{}", ids["handL"]),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    let goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name.starts_with("handL") && i.name.ends_with("_IKGoal"))
        .unwrap()
        .id;
    let piece = e.scene.animation.rig.pieces[0].id;
    (e, ids, goal, piece)
}

#[test]
fn ik20_ik21_assign_covers_the_hierarchy_and_moves_nothing() {
    let (mut e, ids) = biped();
    let line = format!("operandi.ik.assign joints:{}", ids["hips"]);
    assert_eq!(err(&mut e, &line), ik_commands::NEEDS_SETUP);
    ok(&mut e, "operandi.anim.setup on:true");
    e.scene.item_mut(ids["kneeL"]).unwrap().transform.scale = DVec3::splat(2.);
    assert!(err(&mut e, &line).contains("Freeze scale on kneeL"));
    e.scene.item_mut(ids["kneeL"]).unwrap().transform.scale = DVec3::ONE;
    let before: Vec<DVec3> = ids.values().map(|i| world(&e, *i)).collect();
    ok(&mut e, &line);
    let p = e.scene.animation.rig.pieces[0].clone();
    assert_eq!(p.kind, "ik.body");
    assert_eq!(ik_body::covered(&e.scene, &p).len(), 13);
    let after: Vec<DVec3> = ids.values().map(|i| world(&e, *i)).collect();
    assert_eq!(before, after, "no task: pure FK");
    // Only the branch to the left foot.
    e.dispatch(Command::Undo).unwrap();
    ok(
        &mut e,
        &format!("operandi.ik.assign joints:{},{}", ids["hips"], ids["footL"]),
    );
    let p = e.scene.animation.rig.pieces[0].clone();
    assert_eq!(ik_body::covered(&e.scene, &p).len(), 4);
}

#[test]
fn ik22_a_hand_goal_reaches_and_feet_stay_pinned() {
    let (mut e, ids, goal, _) = rigged();
    let feet = [world(&e, ids["footL"]), world(&e, ids["footR"])];
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0.8, 1.8, 0.2);
    let hand = world(&e, ids["handL"]);
    assert!(hand.distance(DVec3::new(0.8, 1.8, 0.2)) < 0.01, "{hand}");
    assert!(world(&e, ids["footL"]).distance(feet[0]) < 0.01);
    assert!(world(&e, ids["footR"]).distance(feet[1]) < 0.01);
    let q: Value = serde_json::from_str(
        &command_exec::answer(
            &e,
            "operandi.ik.query",
            serde_json::json!({"tolerance": 0.01}).as_object().unwrap(),
        )
        .unwrap()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(q["solvers"][0]["met"], true, "{q}");
    assert_eq!(q["solvers"][0]["tasks"].as_array().unwrap().len(), 4);
}

#[test]
fn ik23_translate_root_lets_the_body_lean() {
    let (mut e, ids, goal, piece) = rigged();
    let far = DVec3::new(2.5, 1.5, 0.);
    e.scene.item_mut(goal).unwrap().transform.translation = far;
    let fixed_hips = world(&e, ids["hips"]);
    let fixed_gap = world(&e, ids["handL"]).distance(far);
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:translateRoot value:1"),
    );
    let hips = world(&e, ids["hips"]);
    assert!(hips.x > fixed_hips.x + 0.05, "the body leans: {hips}");
    assert!(world(&e, ids["handL"]).distance(far) < fixed_gap);
    for f in ["footL", "footR"] {
        let p = world(&e, ids[f]);
        assert!(p.y.abs() < 0.05, "{f} stays down: {p}");
    }
}

#[test]
fn ik27_same_frame_same_pose() {
    let (mut e, ids, goal, _) = rigged();
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0.8, 1.7, 0.3);
    let a: Vec<DVec3> = ids.values().map(|i| world(&e, *i)).collect();
    ok(&mut e, "operandi.time.set frame:10");
    ok(&mut e, "operandi.time.set frame:0");
    let b: Vec<DVec3> = ids.values().map(|i| world(&e, *i)).collect();
    assert_eq!(a, b);
}

#[test]
fn ik25_ik26_task_settings_chain_edits_and_remove() {
    let (mut e, ids, goal, piece) = rigged();
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0.8, 1.8, 0.2);
    // Task off: the hand stays.
    let rest = DVec3::new(0.9, 1.5, 0.);
    ok(
        &mut e,
        &format!("operandi.ik.taskSet joint:{} enable:false", ids["handL"]),
    );
    assert!(world(&e, ids["handL"]).distance(rest) < 1e-6);
    ok(
        &mut e,
        &format!(
            "operandi.ik.taskSet joint:{} enable:true joints:1",
            ids["handL"]
        ),
    );
    // Only the elbow moves now: the shoulder stays where it was.
    assert!(world(&e, ids["elbowL"]).distance(DVec3::new(0.6, 1.5, -0.05)) < 1e-6);
    ok(
        &mut e,
        &format!("operandi.ik.taskSet joint:{} joints:-1", ids["handL"]),
    );
    ok(
        &mut e,
        &format!("operandi.ik.removeFromChain joints:{}", ids["armL"]),
    );
    assert!(
        world(&e, ids["handL"]).distance(rest) < 1e-6,
        "the arm is out of the chain"
    );
    ok(
        &mut e,
        &format!("operandi.ik.addToChain joints:{}", ids["armL"]),
    );
    assert!(world(&e, ids["handL"]).distance(rest) > 0.1);
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back.animation.rig.ik_tasks, e.scene.animation.rig.ik_tasks);
    ok(&mut e, &format!("operandi.ik.remove piece:{piece}"));
    assert!(e.scene.animation.rig.pieces.is_empty() && e.scene.animation.rig.ik_tasks.is_empty());
    assert!(world(&e, ids["handL"]).distance(rest) < 1e-6);
}
