//! BEHAVIOR-RIG-PHASE6 §7: Bake turns solved IK into plain keys.
use glam::DVec3;
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

fn pose(e: &mut Editor, joints: &[u64], f: i64) -> Vec<DVec3> {
    ok(e, &format!("operandi.time.set frame:{f}"));
    let s = e.evaluated(evaluate::Purpose::Final);
    joints
        .iter()
        .map(|j| s.world_matrix(*j).unwrap().w_axis.truncate())
        .collect()
}

#[test]
fn bake_turns_solved_ik_into_keys_and_blends_to_fk() {
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
    ok(&mut e, "operandi.time.set frame:0");
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:1.9 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.set channels:{goal}:pos.Y,{goal}:pos.Z"),
    );
    ok(&mut e, "operandi.time.set frame:24");
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:1.2 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:pos.Z value:0.4 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.set channels:{goal}:pos.Y,{goal}:pos.Z"),
    );
    let joints = [a, b, c];
    let before: Vec<Vec<DVec3>> = (0..=24)
        .step_by(4)
        .map(|f| pose(&mut e, &joints, f))
        .collect();
    // Baking the knee bakes the whole chain.
    ok(
        &mut e,
        &format!("operandi.rig.bake items:{b} start:0 end:24"),
    );
    let piece = &e.scene.animation.rig.pieces[0];
    assert!(piece.flag("enable"), "the solver stays, switched to FK");
    let blend = animation::read(&e.scene, &animation::ChannelKey::new(piece.id, "blend")).unwrap();
    assert_eq!(blend, 0.);
    for (k, f) in (0..=24).step_by(4).enumerate() {
        let now = pose(&mut e, &joints, f);
        for (x, y) in now.iter().zip(&before[k]) {
            assert!(x.distance(*y) < 1e-6, "frame {f}: {x} vs {y}");
        }
    }
    for j in [a, b] {
        assert!(
            e.scene
                .animation
                .channels
                .get(&animation::ChannelKey::new(j, "rot.X"))
                .is_some_and(|c| c.curve.keys.len() >= 20)
        );
    }
}
