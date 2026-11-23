//! BEHAVIOR-RIG-PHASE4 §9: the four milestone rigs, built with command
//! lines only and checked frame by frame.
use glam::DVec3;
use operandi_core::*;
use std::f64::consts::{PI, TAU};

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("headless")
}

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = line(e, text) {
        panic!("{text}: {err}");
    }
}

pub(crate) fn item(e: &mut Editor, name: &str, at: DVec3) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    let i = e.scene.item_mut(id).unwrap();
    i.name = name.into();
    i.transform.translation = at;
    id
}

fn world(e: &Editor, id: u64) -> glam::DMat4 {
    e.evaluated(evaluate::Purpose::Final)
        .world_matrix(id)
        .unwrap()
}

fn frame(e: &mut Editor, f: f64) {
    ok(e, &format!("operandi.time.set frame:{f}"));
}

/// Linear slopes on every key of `channel`.
fn linear(e: &mut Editor, channel: &str, frames: &[u32]) {
    let keys: Vec<String> = frames.iter().map(|f| format!("{channel}@{f}")).collect();
    ok(e, &format!("operandi.key.select keys:{}", keys.join(",")));
    ok(e, "operandi.key.slope type:linear");
}

#[test]
fn milestone1_piston_rod_aims_at_the_crank_pin() {
    let mut e = Editor::default();
    let crank = item(&mut e, "Crank", DVec3::ZERO);
    let pin = item(&mut e, "Pin", DVec3::new(1., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.parent items:{pin} parent:{crank} compensate:true"),
    );
    let rod = item(&mut e, "Rod", DVec3::new(-3., 0., 0.));
    // One keyed user channel turns the crank through a direct link.
    ok(
        &mut e,
        &format!("operandi.rig.userChannelAdd item:{crank} name:turn type:angle"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{crank}:turn frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{crank}:turn frame:48 value:{TAU}"),
    );
    linear(&mut e, &format!("{crank}:turn"), &[0, 48]);
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{crank}:rot.Z driver:{crank}:turn"),
    );
    // The rod's X axis aims at the pin.
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:direction items:{rod} targets:{pin}"),
    );
    let p = e.scene.animation.rig.pieces.last().unwrap().id;
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:axis value:0"),
    );
    for f in [0., 6., 12., 24., 36., 47.] {
        frame(&mut e, f);
        let angle = TAU * f / 48.;
        let pin_at = world(&e, pin).w_axis.truncate();
        assert!(
            (pin_at - DVec3::new(angle.cos(), angle.sin(), 0.)).length() < 1e-6,
            "pin at {f}"
        );
        let r = world(&e, rod);
        let aim = (pin_at - r.w_axis.truncate()).normalize();
        assert!(
            (r.x_axis.truncate().normalize() - aim).length() < 1e-6,
            "rod at {f}"
        );
    }
}

#[test]
fn milestone2_wheels_spin_from_the_cars_distance() {
    let mut e = Editor::default();
    let car = item(&mut e, "Car", DVec3::ZERO);
    let wheel = item(&mut e, "Wheel", DVec3::new(0., 0.5, 0.));
    ok(
        &mut e,
        &format!("operandi.rig.parent items:{wheel} parent:{car} compensate:true"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{car}:pos.Z frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{car}:pos.Z frame:24 value:10"),
    );
    // dist / (2*pi*r) * 360 degrees, r = 0.5; the channel is radians.
    ok(
        &mut e,
        &format!(
            "operandi.rig.modifierAdd kind:expression output:{wheel}:rot.X inputs:dist={car}:pos.Z expr:{{dist / (2*pi*0.5) * 360 * pi / 180}}"
        ),
    );
    for f in [0., 5., 12., 24.] {
        frame(&mut e, f);
        let dist = e.scene.item(car).unwrap().transform.translation.z;
        let want = dist / (2. * PI * 0.5) * TAU;
        let s = e.evaluated(evaluate::Purpose::Final);
        let got = s.item(wheel).unwrap().transform.rotation.x;
        assert!((got - want).abs() < 1e-9, "frame {f}: {got} vs {want}");
    }
}

/// A bent open curve: (0,0,0) → (4,0,0) → (4,0,6), 10 m long.
fn path_curve(e: &mut Editor) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    let mut mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::new(4., 0., 0.), DVec3::new(4., 0., 6.)],
        faces: Shared::new(vec![Face {
            vertices: vec![0, 1, 2],
            uv: None,
            material: "Default".into(),
            smooth: true,
            part: None,
            material_label: None,
        }]),
        ..Default::default()
    };
    mesh.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: false,
        curve: curves::CurveKind::Line,
    }];
    e.scene.item_mut(id).unwrap().mesh = MeshSlot::from(mesh);
    id
}

#[test]
fn milestone3_camera_on_a_path_at_constant_speed_aiming_at_a_target() {
    let mut e = Editor::default();
    let cam = item(&mut e, "Camera", DVec3::ZERO);
    let curve = path_curve(&mut e);
    let target = item(&mut e, "Target", DVec3::new(2., 3., 3.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:pathPosition items:{cam} targets:{curve}"),
    );
    let path = e.scene.animation.rig.pieces[0].id;
    ok(
        &mut e,
        &format!("operandi.key.add channel:{path}:percent frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{path}:percent frame:120 value:1"),
    );
    linear(&mut e, &format!("{path}:percent"), &[0, 120]);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:direction items:{cam} targets:{target}"),
    );
    let mut previous: Option<DVec3> = None;
    for f in 0..=120 {
        frame(&mut e, f as f64);
        let w = world(&e, cam);
        let p = w.w_axis.truncate();
        // Constant speed: 10 m over 120 frames, along the bend too.
        if let Some(q) = previous {
            assert!(
                ((p - q).length() - 10. / 120.).abs() < 1e-6,
                "frame {f}: {}",
                (p - q).length()
            );
        }
        previous = Some(p);
        let aim = (DVec3::new(2., 3., 3.) - p).normalize();
        assert!(
            (w.z_axis.truncate().normalize() - aim).length() < 1e-6,
            "aim at {f}"
        );
    }
    let _ = target;
}

/// The robot arm hand-off rig (milestone 4): a part on World until frame
/// 30, then on the gripper (which carries it to x = 4 by 60), then on the
/// conveyor (which moves it on in Z).
pub(crate) fn hand_off() -> (Editor, u64) {
    let mut e = Editor::default();
    let part = item(&mut e, "Part", DVec3::new(1., 0., 0.));
    let gripper = item(&mut e, "Gripper", DVec3::new(1., 2., 0.));
    let conveyor = item(&mut e, "Conveyor", DVec3::new(0., -1., 0.));
    ok(
        &mut e,
        &format!("operandi.key.add channel:{gripper}:pos.X frame:30 value:1"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{gripper}:pos.X frame:60 value:5"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{conveyor}:pos.Z frame:60 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{conveyor}:pos.Z frame:90 value:6"),
    );
    ok(
        &mut e,
        &format!(
            "operandi.rig.dynamicParentAdd item:{part} parents:{gripper},{conveyor} compensate:true"
        ),
    );
    ok(&mut e, "operandi.time.set frame:0");
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{part} parent:0 compensate:true key:true"),
    );
    ok(&mut e, "operandi.time.set frame:30");
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{part} parent:1 compensate:true key:true"),
    );
    ok(&mut e, "operandi.time.set frame:60");
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{part} parent:2 compensate:true key:true"),
    );
    ok(&mut e, "operandi.time.settings end:90");
    ok(&mut e, "operandi.time.set frame:0");
    (e, part)
}

fn path_of(e: &mut Editor, part: u64, frames: impl Iterator<Item = i64>) -> Vec<(i64, DVec3)> {
    frames
        .map(|f| {
            frame(e, f as f64);
            (f, world(e, part).w_axis.truncate())
        })
        .collect()
}

#[test]
fn milestone4_the_hand_off_has_no_pops_scrubbing_either_way_and_bakes() {
    let (mut e, part) = hand_off();
    let forward = path_of(&mut e, part, 0..=90);
    let backward = path_of(&mut e, part, (0..=90).rev());
    // The same pose whichever way it was reached (RG-05).
    for (f, p) in &forward {
        let q = backward.iter().find(|(g, _)| g == f).unwrap().1;
        assert!((*p - q).length() < 1e-9, "frame {f}");
    }
    // No pops: a frame's step stays under the fastest carrier's (0.3 m, the conveyor's smooth-flat peak).
    for w in forward.windows(2) {
        let step = (w[1].1 - w[0].1).length();
        assert!(step < 0.4, "pop between {} and {}: {step}", w[0].0, w[1].0);
    }
    // Where it is: still to 30, carried to x=5 by 60, then down Z to 6.
    let at = |f: i64| forward.iter().find(|(g, _)| *g == f).unwrap().1;
    assert!((at(30) - DVec3::new(1., 0., 0.)).length() < 1e-9);
    assert!((at(60) - DVec3::new(5., 0., 0.)).length() < 1e-9);
    assert!((at(90) - DVec3::new(5., 0., 6.)).length() < 1e-9);
    // Baked: the same motion from plain keys.
    ok(
        &mut e,
        &format!("operandi.rig.bake items:{part} start:0 end:90"),
    );
    let baked = path_of(&mut e, part, 0..=90);
    for ((f, a), (_, b)) in forward.iter().zip(&baked) {
        assert!((*a - *b).length() < 1e-6, "baked frame {f}: {a} vs {b}");
    }
}
