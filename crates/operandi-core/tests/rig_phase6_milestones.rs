//! BEHAVIOR-RIG-PHASE6 §8: the five milestone checks.
use crate::ik_body::{biped, world};
use glam::{DQuat, DVec3};
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

fn frame(e: &mut Editor, f: i64) {
    ok(e, &format!("operandi.time.set frame:{f}"));
}

/// Milestone 1: Leg plant: a two-bone leg with a pole; the hips go down, the foot
/// stays, the knee points along the pole and never flips as the pole
/// turns 0–180° round the leg.
#[test]
fn milestone_1_leg_plant() {
    let mut e = Editor::default();
    ok(
        &mut e,
        "operandi.skel.add position:0,1,0 name:Hip preserveAlignment:true",
    );
    let hip = last(&e);
    ok(
        &mut e,
        &format!(
            "operandi.skel.add position:0,0.5,0.08 parent:{hip} name:Knee preserveAlignment:true"
        ),
    );
    let knee = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,0,0 parent:{knee} name:Foot preserveAlignment:true"),
    );
    let foot = last(&e);
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.ik.applyTwoBone joints:{hip},{knee},{foot}"),
    );
    ok(&mut e, "operandi.ik.pole mode:create");
    ok(&mut e, "operandi.anim.setup on:false");
    let pole = e
        .scene
        .items
        .iter()
        .find(|i| i.name.ends_with("_IKPole"))
        .unwrap()
        .id;
    let planted = world(&e, foot);
    // Hips down 0.2: the foot stays, the knee bends forward (to the pole).
    e.scene.item_mut(hip).unwrap().transform.translation.y -= 0.2;
    assert!(
        world(&e, foot).distance(planted) < 1e-6,
        "foot stays: {}",
        world(&e, foot)
    );
    assert!(
        world(&e, knee).z > 0.1,
        "knee along the pole: {}",
        world(&e, knee)
    );
    // The pole round the leg 0..180°: the knee follows, no flip.
    let (h, f) = (world(&e, hip), world(&e, foot));
    let axis = (f - h).normalize();
    let mid = (h + f) / 2.;
    let mut previous: Option<DVec3> = None;
    for k in 0..=18 {
        let angle = k as f64 / 18. * std::f64::consts::PI;
        let at = mid + DQuat::from_axis_angle(axis, angle) * DVec3::new(0., 0., 1.);
        e.scene.item_mut(pole).unwrap().transform.translation = at;
        let kp = world(&e, knee);
        let off = (kp - h) - axis * (kp - h).dot(axis);
        let want = (at - h) - axis * (at - h).dot(axis);
        assert!(
            off.normalize().dot(want.normalize()) > 0.999,
            "step {k}: knee {kp} vs pole {at}"
        );
        if let Some(p) = previous {
            assert!(off.normalize().dot(p) > 0.9, "no flip at step {k}");
        }
        previous = Some(off.normalize());
        assert!(world(&e, foot).distance(planted) < 1e-6);
    }
}

/// Milestone 2: Switch: IK 0–24, switch to FK at 24, FK to 48; scrubbing across 24
/// never pops.
#[test]
fn milestone_2_switch_without_a_pop() {
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
    frame(&mut e, 0);
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:1.9 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z"),
    );
    frame(&mut e, 24);
    ok(
        &mut e,
        &format!("transform.channel name:pos.X value:0.5 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:1.3 item:{goal}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z"),
    );
    let ik24 = world(&e, c);
    e.scene.select_item(Some(goal));
    ok(&mut e, "operandi.ik.switch mode:fk");
    frame(&mut e, 48);
    ok(
        &mut e,
        &format!("transform.channel name:rot.Z value:0.6 item:{a}"),
    );
    ok(&mut e, &format!("operandi.key.set channels:{a}:rot.Z"));
    // Scrub back and forth across 24.
    let mut previous: Option<DVec3> = None;
    for f in (16..=32).chain((16..=32).rev()) {
        frame(&mut e, f);
        let p = world(&e, c);
        if f == 24 {
            assert!(p.distance(ik24) < 1e-6, "24 holds the IK pose");
        }
        if let Some(q) = previous {
            assert!(p.distance(q) < 0.12, "a pop at frame {f}: {q} → {p}");
        }
        previous = Some(p);
    }
}

/// Milestone 3: Full body: goals on the hands and feet, Translate Root on; a hand
/// goal pulled far: the body leans, the feet stay put.
#[test]
fn milestone_3_full_body_leans_feet_stay() {
    let (mut e, ids) = biped();
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.ik.assign joints:{}", ids["hips"]),
    );
    for j in ["handL", "footL", "footR"] {
        ok(
            &mut e,
            &format!("operandi.ik.addGoal joint:{} mode:position", ids[j]),
        );
    }
    // Feet are planted: their goals pull hardest (IK-25 Weight).
    for j in ["footL", "footR"] {
        ok(
            &mut e,
            &format!("operandi.ik.taskSet joint:{} weight:10", ids[j]),
        );
    }
    ok(&mut e, "operandi.anim.setup on:false");
    let piece = e.scene.animation.rig.pieces[0].id;
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:translateRoot value:1"),
    );
    let feet = [world(&e, ids["footL"]), world(&e, ids["footR"])];
    let hips = world(&e, ids["hips"]);
    let hand_goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name.starts_with("handL") && i.name.ends_with("_IKGoal"))
        .unwrap()
        .id;
    e.scene.item_mut(hand_goal).unwrap().transform.translation = DVec3::new(2., 1.5, 0.);
    assert!(
        world(&e, ids["hips"]).x > hips.x + 0.05,
        "the body leans: {}",
        world(&e, ids["hips"])
    );
    assert!(
        world(&e, ids["handL"]).x > 1.0,
        "the hand reaches: {}",
        world(&e, ids["handL"])
    );
    for (k, f) in ["footL", "footR"].iter().enumerate() {
        let p = world(&e, ids[f]);
        assert!(
            p.distance(feet[k]) < 0.05,
            "{f} stays: {p} (was {})",
            feet[k]
        );
    }
}

/// Milestone 4: Pose tool: both feet pinned, the head dragged down: the knees
/// bend, the feet stay, and with Auto Key every moved joint is keyed.
#[test]
fn milestone_4_pose_tool_crouch() {
    let (mut e, ids) = biped();
    e.scene.animation.settings.auto_key = animation::AutoKey::All;
    frame(&mut e, 10);
    let feet = [world(&e, ids["footL"]), world(&e, ids["footR"])];
    let knees = [world(&e, ids["kneeL"]), world(&e, ids["kneeR"])];
    let head = world(&e, ids["head"]);
    let to = head - DVec3::new(0., 0.3, 0.);
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} to:{},{},{} pins:{},{} rootWeight:1",
            ids["head"], to.x, to.y, to.z, ids["footL"], ids["footR"]
        ),
    );
    for (k, f) in ["footL", "footR"].iter().enumerate() {
        assert!(world(&e, ids[f]).distance(feet[k]) < 0.02, "{f} stays");
    }
    for (k, n) in ["kneeL", "kneeR"].iter().enumerate() {
        assert!(world(&e, ids[n]).distance(knees[k]) > 0.02, "{n} bends");
    }
    let t = e.scene.time.current;
    let before = biped().0;
    for (name, id) in &ids {
        let moved = animation::TRANSFORM_CHANNELS.iter().any(|c| {
            let k = animation::ChannelKey::new(*id, c);
            let (a, b) = (
                animation::read(&before.scene, &k).unwrap(),
                animation::read(&e.scene, &k).unwrap(),
            );
            (a - b).abs() > 1e-6
        });
        let keyed = animation::TRANSFORM_CHANNELS.iter().any(|c| {
            e.scene
                .animation
                .channels
                .get(&animation::ChannelKey::new(*id, c))
                .is_some_and(|ch| ch.curve.key_at(t).is_some())
        });
        assert_eq!(moved, keyed, "{name}: moved {moved}, keyed {keyed}");
    }
}

/// Milestone 5: Tail: a 12-joint FABRIK tail follows a goal along a path with no
/// bone length change.
#[test]
fn milestone_5_tail_follows_a_path() {
    let mut e = Editor::default();
    let mut ids = Vec::new();
    for k in 0..12 {
        let mut line = format!(
            "operandi.skel.add position:0,{},0 name:Tail preserveAlignment:true",
            k as f64 * 0.25
        );
        if let Some(p) = ids.last() {
            line.push_str(&format!(" parent:{p}"));
        }
        ok(&mut e, &line);
        ids.push(last(&e));
    }
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.ik.applyChain joints:{},{}", ids[0], ids[11]),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    let goal = e
        .scene
        .items
        .iter()
        .find(|i| i.name.ends_with("_IKGoal"))
        .unwrap()
        .id;
    let lengths = |e: &Editor| -> Vec<f64> {
        ids.windows(2)
            .map(|w| world(e, w[0]).distance(world(e, w[1])))
            .collect()
    };
    let rest = lengths(&e);
    // The path: an arc in front of the tail, keyed every 6 frames.
    for k in 0..=8 {
        let a = k as f64 / 8. * std::f64::consts::PI;
        frame(&mut e, k * 6);
        let at = DVec3::new(1.5 * a.cos(), 1.8, 1.5 * a.sin());
        for (axis, v) in ["X", "Y", "Z"].iter().zip([at.x, at.y, at.z]) {
            ok(
                &mut e,
                &format!("transform.channel name:pos.{axis} value:{v} item:{goal}"),
            );
        }
        ok(
            &mut e,
            &format!("operandi.key.set channels:{goal}:pos.X,{goal}:pos.Y,{goal}:pos.Z"),
        );
    }
    for f in (0..=48).step_by(3) {
        frame(&mut e, f);
        let g = world(&e, goal);
        let end = world(&e, ids[11]);
        assert!(end.distance(g) < 0.03, "frame {f}: end {end} goal {g}");
        for (k, (now, was)) in lengths(&e).iter().zip(&rest).enumerate() {
            assert!(
                (now - was).abs() < 1e-6,
                "frame {f} bone {k}: {now} vs {was}"
            );
        }
    }
}
