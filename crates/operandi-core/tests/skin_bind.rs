//! BEHAVIOR-RIG-PHASE5 §3: Bind, Unbind and Rebind (SK-20..25).
use glam::DVec3;
use operandi_core::*;
use std::f64::consts::FRAC_PI_2;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn err(e: &mut Editor, text: &str) -> String {
    command_line::run_line(e, text)
        .expect("headless")
        .expect_err("should be refused")
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

/// An arm along X from -0.5 to 1 (a 30-segment box) with shoulder,
/// elbow and wrist joints inside it.
struct Arm {
    e: Editor,
    mesh: u64,
    joints: [u64; 3],
}

fn arm() -> Arm {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let mesh = last(&e);
    e.scene.item_mut(mesh).unwrap().mesh = primitive::segmented_cube(
        DVec3::new(0.25, 0., 0.),
        DVec3::new(1.5, 0.2, 0.2),
        [30, 2, 2],
    )
    .unwrap()
    .into();
    ok(&mut e, "operandi.skel.add position:-0.4,0,0 name:Shoulder");
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0.25,0,0 parent:{a} name:Elbow"),
    );
    let b = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0.9,0,0 parent:{b} name:Wrist"),
    );
    let c = last(&e);
    Arm {
        e,
        mesh,
        joints: [a, b, c],
    }
}

/// Turns the elbow about world Z through its own position.
fn turn_z(a: &mut Arm, angle: f64) {
    let parent = a.e.scene.world_matrix(a.joints[0]).unwrap();
    let (_, pq, _) = parent.to_scale_rotation_translation();
    let t = &mut a.e.scene.item_mut(a.joints[1]).unwrap().transform;
    let q = pq.inverse() * glam::DQuat::from_rotation_z(angle) * pq * t.rotation_quat();
    t.rotation = t.angles_for(q);
}

fn bind(a: &mut Arm, extra: &str) {
    ok(&mut a.e, "operandi.anim.setup on:true");
    ok(
        &mut a.e,
        &format!(
            "operandi.skin.bind items:{} root:{} {extra}",
            a.mesh, a.joints[0]
        ),
    );
    ok(&mut a.e, "operandi.anim.setup on:false");
}

fn maps(a: &Arm) -> std::collections::BTreeMap<String, std::collections::BTreeMap<u32, f64>> {
    a.e.scene
        .item(a.mesh)
        .unwrap()
        .mesh
        .open()
        .attributes
        .point_weights
        .clone()
}

fn shown(a: &Arm) -> Vec<DVec3> {
    let s = a.e.evaluated(evaluate::Purpose::Final);
    s.item(a.mesh).unwrap().mesh.open().vertices.clone()
}

fn rest(a: &Arm) -> Vec<DVec3> {
    a.e.scene.item(a.mesh).unwrap().mesh.open().vertices.clone()
}

/// Every weighted point's weights add up to 1, and every point has some.
fn totals_are_one(a: &Arm) {
    let m = maps(a);
    for p in 0..rest(a).len() as u32 {
        let t: f64 = m.values().filter_map(|w| w.get(&p)).sum();
        assert!((t - 1.).abs() < 1e-6, "point {p}: {t}");
    }
}

#[test]
fn sk02_bind_needs_setup_mode() {
    let mut a = arm();
    let text = format!("operandi.skin.bind items:{} root:{}", a.mesh, a.joints[0]);
    assert_eq!(
        err(&mut a.e, &text),
        "Bind needs Setup mode. Turn on Setup and try again."
    );
    assert!(a.e.scene.item(a.mesh).unwrap().mesh_stack.is_none());
}

#[test]
fn sk21_heat_bind_bends_the_arm_with_nothing_left_behind() {
    let mut a = arm();
    let steps = a.e.undo.len();
    bind(&mut a, "");
    assert_eq!(a.e.undo.len(), steps + 1, "SK-23 one step");
    let m = maps(&a);
    for name in ["Shoulder1", "Elbow1", "Wrist1"] {
        assert!(m.contains_key(name), "{:?}", m.keys());
    }
    totals_are_one(&a);
    // Near the shoulder end the shoulder leads; past the wrist, the wrist.
    let r = rest(&a);
    let far = (0..r.len())
        .max_by(|x, y| r[*x].x.total_cmp(&r[*y].x))
        .unwrap() as u32;
    let near = (0..r.len())
        .min_by(|x, y| r[*x].x.total_cmp(&r[*y].x))
        .unwrap() as u32;
    assert!(m["Shoulder1"].get(&near).copied().unwrap_or(0.) > 0.5);
    // Past the wrist the elbow's bone and the wrist share it (they meet
    // at the wrist); the shoulder has almost none.
    assert!(m["Shoulder1"].get(&far).copied().unwrap_or(0.) < 0.05);
    assert!(m["Wrist1"].get(&far).copied().unwrap_or(0.) > 0.2);
    // Bend the elbow 90° about Z: the hand end swings up, no spikes.
    turn_z(&mut a, FRAC_PI_2);
    let bent = shown(&a);
    assert!(bent[far as usize].y > 0.5, "{}", bent[far as usize]);
    assert!(bent[near as usize].distance(r[near as usize]) < 0.05);
    for (p, q) in bent.iter().zip(&r) {
        assert!(
            p.distance(DVec3::new(0.25, 0., 0.)) < 1.1,
            "spike at {p} from {q}"
        );
    }
}

#[test]
fn sk21_rigid_distance_and_visible() {
    let mut r = arm();
    bind(&mut r, "type:rigid");
    assert!(maps(&r).values().flat_map(|m| m.values()).all(|w| *w == 1.));
    totals_are_one(&r);
    for t in [
        "distance falloff:high",
        "visible falloff:low limitWeights:true weightsPerVertex:2",
    ] {
        let mut a = arm();
        bind(&mut a, &format!("type:{t}"));
        totals_are_one(&a);
        let m = maps(&a);
        if t.contains("limit") {
            for p in 0..rest(&a).len() as u32 {
                assert!(m.values().filter(|w| w.contains_key(&p)).count() <= 2);
            }
        }
    }
}

#[test]
fn sk21_heat_refuses_bad_geometry_and_selects_it() {
    let mut a = arm();
    {
        let it = a.e.scene.item_mut(a.mesh).unwrap();
        let mut mesh = it.mesh.open().clone();
        mesh.vertices.push(DVec3::new(3., 3., 3.));
        it.mesh = mesh.into();
    }
    ok(&mut a.e, "operandi.anim.setup on:true");
    let steps = a.e.undo.len();
    let text = format!("operandi.skin.bind items:{} root:{}", a.mesh, a.joints[0]);
    assert!(err(&mut a.e, &text).contains("loose points"));
    assert_eq!(a.e.undo.len(), steps, "nothing changes");
    assert!(a.e.scene.item(a.mesh).unwrap().mesh_stack.is_none());
    let n = rest(&a).len() as u32;
    assert_eq!(a.e.scene.selection.vertices, vec![n - 1]);
    // The [OP] fallback: Smooth (Distance) binds it.
    ok(&mut a.e, &format!("{text} type:distance"));
}

#[test]
fn sk21_overlapping_shells_are_refused() {
    let mut a = arm();
    {
        let it = a.e.scene.item_mut(a.mesh).unwrap();
        let mut mesh = it.mesh.open().clone();
        let eye = primitive::cube_geometry(DVec3::new(0.25, 0., 0.), DVec3::splat(0.05)).unwrap();
        primitive::append_geometry(&mut mesh, eye).unwrap();
        it.mesh = mesh.into();
    }
    ok(&mut a.e, "operandi.anim.setup on:true");
    let text = format!("operandi.skin.bind items:{} root:{}", a.mesh, a.joints[0]);
    assert_eq!(err(&mut a.e, &text), skin_bind::OVERLAPPING_SHELLS);
    assert_eq!(a.e.scene.selection.vertices.len(), 8, "the smaller shell");
}

#[test]
fn sk22_a_joint_with_two_children_is_one_bone_towards_both() {
    let a = arm();
    let mut e = a.e;
    ok(
        &mut e,
        &format!(
            "operandi.skel.add position:0.25,0.5,0 parent:{}",
            a.joints[1]
        ),
    );
    let bones = skin_bind::bones(&e.scene, a.mesh, a.joints[0], true).unwrap();
    let elbow = bones.iter().find(|b| b.joint == a.joints[1]).unwrap();
    assert_eq!(elbow.ends.len(), 2);
    let p = DVec3::new(0.25, 0.4, 0.);
    assert!(elbow.nearest(p).distance(p) < 1e-9, "on the second segment");
    let leaf = bones.iter().find(|b| b.joint == a.joints[2]).unwrap();
    assert!(leaf.ends.is_empty(), "a leaf is a radius");
}

#[test]
fn sk24_sk25_unbind_and_rebind() {
    let mut a = arm();
    bind(&mut a, "type:distance");
    ok(&mut a.e, "operandi.anim.setup on:true");
    ok(
        &mut a.e,
        &format!(
            "operandi.skel.add position:1.2,0,0 parent:{} name:Tip",
            a.joints[2]
        ),
    );
    let tip = last(&a.e);
    ok(
        &mut a.e,
        &format!("operandi.skin.rebind items:{} joints:{tip}", a.mesh),
    );
    let before = maps(&a);
    assert!(before["Tip1"].is_empty(), "an empty map");
    let q = command_exec::answer(
        &a.e,
        "operandi.skin.query",
        serde_json::json!({"item": a.mesh}).as_object().unwrap(),
    )
    .unwrap()
    .unwrap();
    assert!(q.contains("Tip1"));
    ok(&mut a.e, &format!("operandi.skin.unbind items:{}", a.mesh));
    assert!(a.e.scene.item(a.mesh).unwrap().mesh_stack.is_none());
    assert!(maps(&a).contains_key("Elbow1"), "maps kept");
    a.e.dispatch(Command::Undo).unwrap();
    ok(
        &mut a.e,
        &format!("operandi.skin.unbind items:{} keepMaps:false", a.mesh),
    );
    assert!(!maps(&a).contains_key("Elbow1"));
    assert!(err(&mut a.e, &format!("operandi.skin.unbind items:{}", a.mesh)).contains("no skin"));
}

#[test]
fn bind_saves_and_reopens() {
    let mut a = arm();
    bind(&mut a, "type:heat dualQuaternion:true");
    a.e.scene
        .item_mut(a.joints[1])
        .unwrap()
        .transform
        .rotation
        .z += 1.;
    let before = shown(&a);
    let back = persistence::decode(&persistence::encode(&a.e.scene).unwrap()).unwrap();
    let b = Arm {
        e: Editor {
            scene: back,
            ..Default::default()
        },
        mesh: a.mesh,
        joints: a.joints,
    };
    for (p, q) in shown(&b).iter().zip(&before) {
        assert!(p.distance(*q) < 1e-9);
    }
}
