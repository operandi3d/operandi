//! BEHAVIOR-RIG-PHASE6 §5: joint limits (IK-40..42).
use glam::DVec3;
use operandi_core::*;
use serde_json::{Value, json};

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

/// IK-04's leg with two-bone IK and its goal at (0, 1.5, 0).
fn leg() -> (Editor, [u64; 3]) {
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
    e.scene.item_mut(goal).unwrap().transform.translation = DVec3::new(0., 1.5, 0.);
    (e, [a, b, c])
}

/// The knee's solved local turn about X, in degrees.
fn knee_x(e: &Editor, [a, b, _]: [u64; 3]) -> f64 {
    let s = e.evaluated(evaluate::Purpose::Final);
    let local = s.world_matrix(a).unwrap().inverse() * s.world_matrix(b).unwrap();
    let (_, q, _) = local.to_scale_rotation_translation();
    s.item(b).unwrap().transform.angles_for(q).x.to_degrees()
}

#[test]
fn ik41_soft_wall_then_hard_stop() {
    use ik_joint::soft_clamp;
    assert_eq!(soft_clamp(0.5, 0., 1., 1.), 0.5, "inside: untouched");
    assert!(soft_clamp(0.95, 0., 1., 1.) < 0.95, "near the wall: slowed");
    assert!(soft_clamp(0.95, 0., 1., 1.) > 0.9);
    assert_eq!(soft_clamp(5., 0., 1., 1.), 1., "stops at the value");
    assert_eq!(soft_clamp(-5., 0., 1., 1.), 0.);
    assert_eq!(soft_clamp(2., 0., 1., 0.), 1., "strength 0: a hard clamp");
}

#[test]
fn ik41_two_bone_knee_stops_at_its_limit() {
    let (mut e, j) = leg();
    let free = knee_x(&e, j);
    assert!(free.abs() > 30., "{free}");
    ok(
        &mut e,
        &format!(
            "operandi.ik.jointSet joint:{} enableLimits:true useLimitsX:true minX:-15 maxX:15",
            j[1]
        ),
    );
    let held = knee_x(&e, j);
    assert!(held.abs() <= 15. + 1e-6, "{held}");
    assert!(held.abs() > 10., "pressed against the wall: {held}");
    let q: Value = serde_json::from_str(
        &command_exec::answer(&e, "operandi.ik.query", &Default::default())
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        q["solvers"][0]["met"], false,
        "the limit wins over the goal"
    );
    // Enable Limits off: free again.
    ok(
        &mut e,
        &format!("operandi.ik.jointSet joint:{} enableLimits:false", j[1]),
    );
    assert!((knee_x(&e, j) - free).abs() < 1e-6);
}

#[test]
fn ik42_limit_fk_clamps_edits() {
    let (mut e, j) = leg();
    ok(
        &mut e,
        &format!(
            "operandi.ik.jointSet joint:{} enableLimits:true useLimitsZ:true minZ:-45 maxZ:45 limitFk:true",
            j[2]
        ),
    );
    ok(
        &mut e,
        &format!(
            "transform.channel name:rot.Z value:{} item:{}",
            120f64.to_radians(),
            j[2]
        ),
    );
    let z = e
        .scene
        .item(j[2])
        .unwrap()
        .transform
        .rotation
        .z
        .to_degrees();
    assert!((z - 45.).abs() < 1e-9, "{z}");
    // Limit FK off: edits go through.
    ok(
        &mut e,
        &format!("operandi.ik.jointSet joint:{} limitFk:false", j[2]),
    );
    ok(
        &mut e,
        &format!(
            "transform.channel name:rot.Z value:{} item:{}",
            120f64.to_radians(),
            j[2]
        ),
    );
    let z = e
        .scene
        .item(j[2])
        .unwrap()
        .transform
        .rotation
        .z
        .to_degrees();
    assert!((z - 120.).abs() < 1e-9, "{z}");
}

#[test]
fn joint_settings_query_undo_and_save() {
    let (mut e, j) = leg();
    let steps = e.undo.len();
    ok(
        &mut e,
        &format!(
            "operandi.ik.jointSet joint:{} enableLimits:true useLimitsX:true minX:-10 maxX:90 rotateY:false",
            j[1]
        ),
    );
    assert_eq!(e.undo.len(), steps + 1);
    let q: Value = serde_json::from_str(
        &command_exec::answer(
            &e,
            "operandi.ik.jointQuery",
            json!({"joint": j[1]}).as_object().unwrap(),
        )
        .unwrap()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(q["axes"]["X"]["max"], 90.0);
    assert_eq!(q["axes"]["Y"]["rotate"], false);
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back.animation.rig.joint_ik, e.scene.animation.rig.joint_ik);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.animation.rig.joint_ik.is_empty());
}
