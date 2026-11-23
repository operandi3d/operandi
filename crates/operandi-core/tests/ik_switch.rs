//! BEHAVIOR-RIG-PHASE6 §4: IK/FK blend and switching (IK-30..33).
use glam::DVec3;
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

/// IK-04's leg; (editor, joints, goal, piece).
fn leg() -> (Editor, [u64; 3], u64, u64) {
    let mut e = Editor::default();
    ok(
        &mut e,
        "operandi.skel.add position:0,0,0 preserveAlignment:true",
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
        .find(|i| i.name.ends_with("_IKGoal"))
        .unwrap()
        .id;
    let piece = e.scene.animation.rig.pieces[0].id;
    (e, [a, b, c], goal, piece)
}

fn at_frame(e: &mut Editor, f: i64) {
    ok(e, &format!("operandi.time.set frame:{f}"));
}

/// World positions of the chain at frame `f`.
fn pose(e: &mut Editor, joints: [u64; 3], f: i64) -> Vec<DVec3> {
    at_frame(e, f);
    let s = e.evaluated(evaluate::Purpose::Final);
    joints
        .iter()
        .map(|j| s.world_matrix(*j).unwrap().w_axis.truncate())
        .collect()
}

fn same(a: &[DVec3], b: &[DVec3], what: &str) {
    for (x, y) in a.iter().zip(b) {
        assert!(x.distance(*y) < 1e-6, "{what}: {x} vs {y}");
    }
}

/// The goal keyed from (0,1.9,0) at 0 to (0.5,1.2,0) at 24.
fn animate_goal(e: &mut Editor, goal: u64) {
    at_frame(e, 0);
    ok(
        e,
        &format!("transform.channel name:pos.Y value:1.9 item:{goal}"),
    );
    ok(
        e,
        &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z"),
    );
    at_frame(e, 24);
    ok(
        e,
        &format!("transform.channel name:pos.X value:0.5 item:{goal}"),
    );
    ok(
        e,
        &format!("transform.channel name:pos.Y value:1.2 item:{goal}"),
    );
    ok(
        e,
        &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z"),
    );
}

#[test]
fn ik31_switch_to_fk_and_back_never_pops() {
    let (mut e, j, goal, _) = leg();
    animate_goal(&mut e, goal);
    let before: Vec<Vec<DVec3>> = (0..=24).step_by(6).map(|f| pose(&mut e, j, f)).collect();
    at_frame(&mut e, 24);
    e.scene.select_item(Some(goal));
    let steps = e.undo.len();
    ok(&mut e, "operandi.ik.switch");
    assert_eq!(e.undo.len(), steps + 1, "one step");
    // At 24 nothing moved; before 24 everything is as it was.
    for (k, f) in (0..=24).step_by(6).enumerate() {
        let now = pose(&mut e, j, f);
        same(&now, &before[k], &format!("frame {f} after Switch to FK"));
    }
    // Animate FK to 48: turn the hip and key it.
    at_frame(&mut e, 48);
    ok(
        &mut e,
        &format!("transform.channel name:rot.Z value:0.5 item:{}", j[0]),
    );
    ok(&mut e, &format!("operandi.key.set channels:{}:rot.Z", j[0]));
    let fk48 = pose(&mut e, j, 48);
    let fk24 = pose(&mut e, j, 24);
    same(&fk24, &before[4], "frame 24 with FK keyed on");
    // Moving the goal now doesn't matter in FK.
    // Switch back to IK at 48: no pop there, nor anywhere before.
    at_frame(&mut e, 48);
    e.scene.select_item(Some(goal));
    ok(&mut e, "operandi.ik.switch");
    same(&pose(&mut e, j, 48), &fk48, "frame 48 after Switch to IK");
    same(&pose(&mut e, j, 24), &fk24, "frame 24 after Switch to IK");
    same(
        &pose(&mut e, j, 0),
        &before[0],
        "frame 0 after Switch to IK",
    );
}

#[test]
fn ik30_blend_is_a_keyable_channel() {
    let (mut e, j, goal, piece) = leg();
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0., 1.5, 0.);
    at_frame(&mut e, 0);
    ok(
        &mut e,
        &format!("operandi.key.add channel:{piece}:blend frame:0 value:1"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{piece}:blend frame:10 value:0"),
    );
    let ik = pose(&mut e, j, 0);
    let fk = pose(&mut e, j, 10);
    assert!(ik[2].distance(DVec3::new(0., 1.5, 0.)) < 1e-6);
    assert!(fk[2].distance(DVec3::new(0., 2., 0.)) < 1e-6);
    let half = pose(&mut e, j, 5);
    assert!(half[2].y > 1.5 && half[2].y < 2.);
}

#[test]
fn ik32_set_and_delete_keys_select_channels() {
    let (mut e, j, goal, piece) = leg();
    e.scene.select_item(Some(goal));
    ok(&mut e, "operandi.ik.setKeys");
    for id in j.iter().chain([&goal]) {
        assert!(
            e.scene
                .animation
                .channels
                .get(&animation::ChannelKey::new(*id, "rot.Z"))
                .is_some_and(|c| !c.curve.is_empty())
        );
    }
    assert!(
        e.scene
            .animation
            .channels
            .contains_key(&animation::ChannelKey::new(piece, "blend"))
    );
    ok(&mut e, "operandi.ik.setKeys delete:true");
    assert!(
        e.scene
            .animation
            .channels
            .get(&animation::ChannelKey::new(goal, "pos.X"))
            .is_none_or(|c| c.curve.is_empty())
    );
    ok(&mut e, "operandi.ik.selectSwitch");
    let sel = e.scene.selected_items();
    for id in j.iter().chain([&goal]) {
        assert!(sel.contains(id));
    }
    ok(&mut e, "operandi.ik.selectSolvers");
    assert_eq!(e.scene.selected_items(), vec![j[0]]);
}

#[test]
fn ik33_disable_update_rest_and_match() {
    let (mut e, j, goal, piece) = leg();
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0., 1.5, 0.);
    let solved = pose(&mut e, j, 0);
    assert!(
        command_line::run_line(&mut e, "operandi.ik.disable on:true")
            .unwrap()
            .is_err(),
        "Setup only"
    );
    ok(&mut e, "operandi.anim.setup on:true");
    ok(&mut e, "operandi.ik.disable on:true");
    let fk = pose(&mut e, j, 0);
    assert!(fk[2].distance(DVec3::new(0., 2., 0.)) < 1e-6, "no solve");
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(back.animation.rig.ik_disabled, "saved");
    ok(&mut e, "operandi.ik.disable on:false");
    same(&pose(&mut e, j, 0), &solved, "solving again");
    // Move the knee to bend the other way in Setup, update the rest pose:
    // the knee now bends toward +Z.
    ok(&mut e, "operandi.ik.disable on:true");
    e.scene.item_mut(j[1]).unwrap().transform.translation.z = 0.2;
    e.scene.item_mut(j[2]).unwrap().transform.translation.z = -0.2;
    ok(&mut e, &format!("operandi.ik.updateRest piece:{piece}"));
    ok(&mut e, &format!("operandi.ik.match piece:{piece}"));
    let g = e.scene.world_matrix(goal).unwrap().w_axis.truncate();
    assert!(
        g.distance(DVec3::new(0., 2., 0.)) < 1e-6,
        "goal on the end: {g}"
    );
    ok(&mut e, "operandi.ik.disable on:false");
    ok(&mut e, "operandi.anim.setup on:false");
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0., 1.5, 0.);
    let p = pose(&mut e, j, 0);
    assert!(p[1].z > 0.6, "bends to +Z now: {}", p[1]);
    let _: Value = serde_json::from_str(
        &command_exec::answer(&e, "operandi.ik.query", &Default::default())
            .unwrap()
            .unwrap(),
    )
    .unwrap();
}
