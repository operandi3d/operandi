//! BEHAVIOR-RIG-PHASE5 §8 milestone checks 1 and 2 (check 3, the Blender
//! round trip, is operandi-usd's `blender_round_trip`).
use glam::{DQuat, DVec3};
use operandi_core::*;
use std::f64::consts::FRAC_PI_2;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn by_name(e: &Editor, name: &str) -> u64 {
    e.scene
        .items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no {name}"))
        .id
}

/// A bar from x -1 to 1 with a root at the middle and symmetric arms
/// (elbows at ±0.5, wrists at ±0.9) drawn with symmetry, Heat bound.
fn arms(dq: bool) -> (Editor, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let mesh = e.scene.items.last().unwrap().id;
    e.scene.item_mut(mesh).unwrap().mesh =
        primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 0.2, 0.2), [40, 2, 2])
            .unwrap()
            .into();
    ok(
        &mut e,
        "operandi.skel.add position:0,0,0 name:Root preserveAlignment:true",
    );
    let root = by_name(&e, "Root1");
    ok(
        &mut e,
        &format!(
            "operandi.skel.add position:0.5,0,0 parent:{root} symmetry:X name:Elbow preserveAlignment:true"
        ),
    );
    let l = by_name(&e, "Elbow1_L");
    ok(
        &mut e,
        &format!(
            "operandi.skel.add position:0.9,0,0 parent:{l} symmetry:X name:Wrist preserveAlignment:true"
        ),
    );
    assert!(by_name(&e, "Wrist1_R") > 0, "mirrored");
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.skin.bind items:{mesh} root:{root} type:heat dualQuaternion:{dq}"),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    (e, mesh)
}

fn shown(e: &Editor, mesh: u64) -> Vec<DVec3> {
    e.evaluated(evaluate::Purpose::Final)
        .item(mesh)
        .unwrap()
        .mesh
        .open()
        .vertices
        .clone()
}

fn bend_left_elbow(e: &mut Editor) {
    let id = by_name(e, "Elbow1_L");
    let t = &mut e.scene.item_mut(id).unwrap().transform;
    t.rotation = t.angles_for(DQuat::from_rotation_z(FRAC_PI_2) * t.rotation_quat());
}

#[test]
fn check_1_bend_an_arm_nothing_left_behind_no_spikes() {
    let (mut e, mesh) = arms(false);
    let rest = shown(&e, mesh);
    bend_left_elbow(&mut e);
    let bent = shown(&e, mesh);
    let elbow = DVec3::new(0.5, 0., 0.);
    for (p, (a, b)) in rest.iter().zip(&bent).enumerate() {
        // Past the left elbow everything swings up with the forearm.
        if a.x > 0.75 {
            assert!(b.y > 0.2, "point {p} left behind at {b} (rest {a})");
        }
        // The other arm stays (heat leaves at most a trace past the root).
        if a.x < -0.25 {
            assert!(
                a.distance(*b) < 5e-3,
                "point {p} on the right moved: {a} → {b}"
            );
        }
        // No spikes: nothing ends farther from the elbow than it started
        // (plus the bar's half thickness).
        assert!(
            b.distance(elbow) <= a.distance(elbow) + 0.15,
            "spike at point {p}: {a} → {b}"
        );
    }
}

#[test]
fn check_1_dual_quaternion_keeps_the_elbow_volume() {
    let elbow = DVec3::new(0.5, 0., 0.);
    let mut thickness = Vec::new();
    for dq in [false, true] {
        let (mut e, mesh) = arms(dq);
        let rest = shown(&e, mesh);
        bend_left_elbow(&mut e);
        let bent = shown(&e, mesh);
        // How far the points round the elbow end up from it, against rest.
        let near: Vec<usize> = (0..rest.len())
            .filter(|&p| (rest[p].x - 0.5).abs() < 0.06)
            .collect();
        let kept: f64 = near
            .iter()
            .map(|&p| bent[p].distance(elbow) / rest[p].distance(elbow))
            .sum::<f64>()
            / near.len() as f64;
        thickness.push(kept);
    }
    assert!(
        thickness[1] > thickness[0],
        "DQ {} vs linear {}",
        thickness[1],
        thickness[0]
    );
    assert!(thickness[1] > 0.95, "DQ keeps the volume: {}", thickness[1]);
}

#[test]
fn check_2_fix_weights_paint_smooth_mirror_totals_stay_100() {
    let (mut e, mesh) = arms(false);
    let n = e.scene.item(mesh).unwrap().mesh.open().vertices.len();
    let left: Vec<String> = (0..n)
        .filter(|&p| e.scene.item(mesh).unwrap().mesh.open().vertices[p].x > 0.6)
        .map(|p| p.to_string())
        .collect();
    ok(
        &mut e,
        &format!(
            "operandi.weight.set item:{mesh} map:Wrist1_L points:{} value:0.8 normalize:true",
            left.join(",")
        ),
    );
    ok(
        &mut e,
        &format!("operandi.weight.smooth item:{mesh} map:Wrist1_L"),
    );
    ok(&mut e, &format!("operandi.weight.normalize item:{mesh}"));
    ok(&mut e, &format!("operandi.weight.mirror item:{mesh}"));
    let w = &e
        .scene
        .item(mesh)
        .unwrap()
        .mesh
        .open()
        .attributes
        .point_weights;
    let skin = skin_weights::skin_maps(&e.scene, mesh).unwrap();
    for p in 0..n as u32 {
        let t: f64 = skin.iter().filter_map(|m| w[m].get(&p)).sum();
        assert!((t - 1.).abs() < 1e-6, "point {p}: {t}");
    }
    // Mirrored: a right point has the left twin's wrist weight.
    let verts = &e.scene.item(mesh).unwrap().mesh.open().vertices;
    let (p, _) = verts.iter().enumerate().find(|(_, v)| v.x > 0.8).unwrap();
    let twin = verts
        .iter()
        .position(|v| (v.x + verts[p].x).abs() < 1e-9 && v.y == verts[p].y && v.z == verts[p].z)
        .unwrap();
    assert!((w["Wrist1_L"][&(p as u32)] - w["Wrist1_R"][&(twin as u32)]).abs() < 1e-9);
}
