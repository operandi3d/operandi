//! BEHAVIOR-RIG-PHASE6 §6: the Pose tool's drags, scripted (IK-50..54).
use crate::ik_body::{biped, world};
use glam::DVec3;
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn xyz(p: DVec3) -> String {
    format!("{},{},{}", p.x, p.y, p.z)
}

#[test]
fn ik51_ik52_drag_the_head_down_feet_pinned_knees_bend() {
    let (mut e, ids) = biped();
    let feet = [world(&e, ids["footL"]), world(&e, ids["footR"])];
    let knee_before = world(&e, ids["kneeL"]);
    let head = world(&e, ids["head"]);
    let to = head - DVec3::new(0., 0.3, 0.);
    let steps = e.undo.len();
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} to:{} pins:{},{} rootWeight:1",
            ids["head"],
            xyz(to),
            ids["footL"],
            ids["footR"]
        ),
    );
    assert_eq!(e.undo.len(), steps + 1, "one step");
    assert!(
        world(&e, ids["head"]).distance(to) < 0.02,
        "{}",
        world(&e, ids["head"])
    );
    for (k, f) in ["footL", "footR"].iter().enumerate() {
        assert!(
            world(&e, ids[f]).distance(feet[k]) < 0.02,
            "{f} stays: {}",
            world(&e, ids[f])
        );
    }
    let knee = world(&e, ids["kneeL"]);
    assert!(knee.distance(knee_before) > 0.02, "the knee bends: {knee}");
    // The pose is in the joints' own channels: no solver was left behind.
    assert!(e.scene.animation.rig.pieces.is_empty());
}

#[test]
fn ik52_dragging_up_to_the_nearest_pin_only() {
    let (mut e, ids) = biped();
    let elbow = world(&e, ids["elbowL"]);
    let spine = world(&e, ids["spine"]);
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} to:0.8,1.8,0.2 pins:{}",
            ids["handL"], ids["armL"]
        ),
    );
    assert!(
        world(&e, ids["elbowL"]).distance(elbow) > 1e-3,
        "the elbow follows"
    );
    assert!(
        world(&e, ids["spine"]).distance(spine) < 1e-9,
        "above the pin: untouched"
    );
}

#[test]
fn ik54_auto_key_keys_every_moved_joint() {
    let (mut e, ids) = biped();
    e.scene.animation.settings.auto_key = animation::AutoKey::All;
    ok(&mut e, "operandi.time.set frame:5");
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} to:0.7,1.7,0.2 pins:{}",
            ids["handL"], ids["spine"]
        ),
    );
    let t = e.scene.time.current;
    for j in ["armL", "elbowL"] {
        let keyed = animation::TRANSFORM_CHANNELS.iter().any(|c| {
            e.scene
                .animation
                .channels
                .get(&animation::ChannelKey::new(ids[j], c))
                .is_some_and(|ch| ch.curve.key_at(t).is_some())
        });
        assert!(keyed, "{j} keyed");
    }
    let untouched = animation::TRANSFORM_CHANNELS.iter().any(|c| {
        e.scene
            .animation
            .channels
            .contains_key(&animation::ChannelKey::new(ids["footL"], c))
    });
    assert!(!untouched, "the feet didn't move, no keys");
}

#[test]
fn ik51_rigid_twist_and_fk_modes() {
    let (mut e, ids) = biped();
    let all: Vec<DVec3> = ids.values().map(|i| world(&e, *i)).collect();
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} mode:rigid by:1,0,0",
            ids["head"]
        ),
    );
    for (i, id) in ids.values().enumerate() {
        assert!(world(&e, *id).distance(all[i] + DVec3::X) < 1e-9);
    }
    e.dispatch(Command::Undo).unwrap();
    // Dragging the root moves everything too.
    ok(
        &mut e,
        &format!("operandi.pose.drag joint:{} to:0,1.5,0", ids["hips"]),
    );
    assert!(world(&e, ids["head"]).distance(DVec3::new(0., 2.5, 0.)) < 1e-9);
    e.dispatch(Command::Undo).unwrap();
    // FK: only the elbow turns, its bone aims at the point.
    let shoulder = world(&e, ids["armL"]);
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} mode:fk to:0.6,2,0",
            ids["elbowL"]
        ),
    );
    let hand = world(&e, ids["handL"]);
    let elbow = world(&e, ids["elbowL"]);
    assert!(
        (hand - elbow)
            .normalize()
            .dot(DVec3::new(0., 0.5, 0.05).normalize())
            > 0.999,
        "{hand}"
    );
    assert!(world(&e, ids["armL"]).distance(shoulder) < 1e-9);
    // Twist: the hand stays on the bone line, the joint turns.
    ok(
        &mut e,
        &format!(
            "operandi.pose.drag joint:{} mode:twist angle:45",
            ids["elbowL"]
        ),
    );
    assert!(world(&e, ids["handL"]).distance(hand) < 1e-9);
}
