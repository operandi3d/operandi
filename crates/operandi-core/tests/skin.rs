//! BEHAVIOR-RIG-PHASE5 §4: the skin piece (SK-30..36, SK-51, SK-52).
use glam::DVec3;
use operandi_core::mesh_stack::{MeshOp, MeshStack, OpKind};
use operandi_core::skin::{self, SkinValues};
use operandi_core::*;
use std::collections::BTreeMap;
use std::f64::consts::FRAC_PI_2;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

/// An arm: shoulder joint at (-0.3,0,0), elbow (forearm) at the origin,
/// and a mesh whose point 0 sits at (0.2,0,0) weighted `w` to the
/// forearm and the rest to the shoulder.
struct Arm {
    e: Editor,
    mesh: u64,
    elbow: u64,
}

fn arm(w: f64, interpolation: u32) -> Arm {
    let mut e = Editor::default();
    ok(
        &mut e,
        "operandi.skel.add position:-0.3,0,0 preserveAlignment:true",
    );
    let shoulder = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,0,0 parent:{shoulder} preserveAlignment:true"),
    );
    let elbow = last(&e);
    e.scene.new_mesh().unwrap();
    let mesh = last(&e);
    let mut m = primitive::cube_geometry(DVec3::new(1., 1., 1.), DVec3::splat(0.1)).unwrap();
    m.vertices[0] = DVec3::new(0.2, 0., 0.);
    let map = |w: f64| BTreeMap::from([(0u32, w)]);
    m.attributes
        .point_weights
        .insert("Shoulder".into(), map(1. - w));
    m.attributes.point_weights.insert("Elbow".into(), map(w));
    let mut values = SkinValues {
        interpolation,
        ..Default::default()
    };
    values.add(shoulder, "Shoulder");
    values.add(elbow, "Elbow");
    let item = e.scene.item_mut(mesh).unwrap();
    item.mesh = m.into();
    item.mesh_stack = Some(MeshStack {
        ops: vec![MeshOp {
            id: 0,
            enabled: true,
            kind: OpKind::Op {
                type_id: skin::SKIN_OP.into(),
                values: serde_json::to_value(values).unwrap(),
            },
            selection: None,
            selops: Vec::new(),
        }],
        next_id: 1,
    });
    for j in [shoulder, elbow] {
        assert!(skin::ensure_setup_channels(&mut e.scene, j));
    }
    Arm { e, mesh, elbow }
}

fn point(a: &Arm, p: usize) -> DVec3 {
    let s = a.e.evaluated(evaluate::Purpose::Final);
    s.item(a.mesh).unwrap().mesh.open().vertices[p]
}

fn bend(a: &mut Arm) {
    a.e.scene.item_mut(a.elbow).unwrap().transform.rotation.z = FRAC_PI_2;
}

fn near(a: DVec3, b: DVec3) {
    assert!(a.distance(b) < 1e-4, "{a} vs {b}");
}

#[test]
fn sk33_linear_blend_halves_the_move() {
    let mut a = arm(0.5, skin::LINEAR);
    near(point(&a, 0), DVec3::new(0.2, 0., 0.));
    bend(&mut a);
    let p = point(&a, 0);
    near(p, DVec3::new(0.1, 0.1, 0.));
    assert!((p.length() - 0.1414).abs() < 1e-4);
    // Local (each influence Linear) is the same (SK-32).
    let mut b = arm(0.5, skin::LOCAL);
    bend(&mut b);
    near(point(&b, 0), DVec3::new(0.1, 0.1, 0.));
}

#[test]
fn sk33_dual_quaternion_keeps_the_volume() {
    let mut a = arm(0.5, skin::DUAL_QUATERNION);
    bend(&mut a);
    let p = point(&a, 0);
    near(p, DVec3::new(0.1414, 0.1414, 0.));
    assert!((p.length() - 0.2).abs() < 1e-4);
}

#[test]
fn sk33_full_forearm_weight_follows_it_either_way() {
    for method in [skin::LINEAR, skin::DUAL_QUATERNION] {
        let mut a = arm(1., method);
        bend(&mut a);
        near(point(&a, 0), DVec3::new(0., 0.2, 0.));
    }
}

#[test]
fn sk30_weights_are_normalised_and_unweighted_points_stay() {
    // 0.25 + 0.25 is a whole move shared half and half.
    let mut a = arm(0.5, skin::LINEAR);
    {
        let m = a.e.scene.item_mut(a.mesh).unwrap();
        let mut mesh = m.mesh.open().clone();
        for w in mesh.attributes.point_weights.values_mut() {
            w.insert(0, 0.25);
        }
        m.mesh = mesh.into();
    }
    let rest1 = point(&a, 1);
    bend(&mut a);
    near(point(&a, 0), DVec3::new(0.1, 0.1, 0.));
    near(point(&a, 1), rest1);
}

#[test]
fn sk51_setup_mode_shows_the_rest_shape_and_moves_the_bind_pose() {
    let mut a = arm(1., skin::LINEAR);
    bend(&mut a);
    near(point(&a, 0), DVec3::new(0., 0.2, 0.));
    ok(&mut a.e, "operandi.anim.setup on:true");
    // The elbow's setup rotation is 0: in Setup the mesh is at rest.
    a.e.scene.item_mut(a.elbow).unwrap().transform.rotation.z = 0.;
    near(point(&a, 0), DVec3::new(0.2, 0., 0.));
    // Moving the elbow in Setup moves the bind pose: the mesh stays.
    ok(
        &mut a.e,
        &format!(
            "operandi.skel.move joint:{} position:0.05,0,0 preserveAlignment:true",
            a.elbow
        ),
    );
    near(point(&a, 0), DVec3::new(0.2, 0., 0.));
    ok(&mut a.e, "operandi.anim.setup on:false");
    near(point(&a, 0), DVec3::new(0.2, 0., 0.));
    // Bending now turns about the moved elbow.
    bend(&mut a);
    near(point(&a, 0), DVec3::new(0.05, 0.15, 0.));
}

#[test]
fn the_skin_saves_and_reopens() {
    let mut a = arm(0.5, skin::DUAL_QUATERNION);
    bend(&mut a);
    let before = point(&a, 0);
    let back = persistence::decode(&persistence::encode(&a.e.scene).unwrap()).unwrap();
    let e = Editor {
        scene: back,
        ..Default::default()
    };
    let b = Arm {
        e,
        mesh: a.mesh,
        elbow: a.elbow,
    };
    near(point(&b, 0), before);
}

/// SK-52: a 50k-point mesh on 60 joints, four weights a point, plays at
/// 24 fps. Debug builds only check it runs; release checks the speed
/// (`cargo test --release -p operandi-core --test all skin::` ).
#[test]
fn sk52_skinning_fifty_thousand_points_on_sixty_joints() {
    let mut e = Editor::default();
    let mut joints = Vec::new();
    let mut parent: Option<u64> = None;
    for k in 0..60 {
        let mut line = format!(
            "operandi.skel.add position:0,{},0 preserveAlignment:true",
            k as f64 * 0.05
        );
        if let Some(p) = parent {
            line.push_str(&format!(" parent:{p}"));
        }
        ok(&mut e, &line);
        parent = Some(last(&e));
        joints.push(last(&e));
    }
    e.scene.new_mesh().unwrap();
    let mesh = last(&e);
    let mut m = primitive::segmented_cube(
        DVec3::new(0., 1.5, 0.),
        DVec3::new(0.5, 3., 0.5),
        [40, 300, 40],
    )
    .unwrap();
    let n = m.vertices.len();
    assert!(n >= 50_000, "{n}");
    let mut values = SkinValues::default();
    for (j, id) in joints.iter().enumerate() {
        let name = format!("J{j}");
        let mut w = BTreeMap::new();
        for p in 0..n {
            if (p + j) % 15 < 1 || p % 60 == j {
                w.insert(p as u32, 0.25);
            }
        }
        m.attributes.point_weights.insert(name.clone(), w);
        values.add(*id, &name);
    }
    let item = e.scene.item_mut(mesh).unwrap();
    item.mesh = m.into();
    item.mesh_stack = Some(MeshStack {
        ops: vec![MeshOp {
            id: 0,
            enabled: true,
            kind: OpKind::Op {
                type_id: skin::SKIN_OP.into(),
                values: serde_json::to_value(values).unwrap(),
            },
            selection: None,
            selops: Vec::new(),
        }],
        next_id: 1,
    });
    for j in &joints {
        skin::ensure_setup_channels(&mut e.scene, *j);
    }
    let frames = if cfg!(debug_assertions) { 2 } else { 24 };
    let start = std::time::Instant::now();
    for f in 0..frames {
        e.scene.item_mut(joints[10]).unwrap().transform.rotation.z = 0.01 * f as f64 + 0.01;
        let s = e.evaluated(evaluate::Purpose::Final);
        assert_ne!(
            s.item(mesh).unwrap().mesh.open().vertices,
            e.scene.item(mesh).unwrap().mesh.open().vertices
        );
    }
    let per_frame = start.elapsed().as_secs_f64() / frames as f64;
    if !cfg!(debug_assertions) {
        eprintln!("SK-52 skin: {:.1} ms a frame", per_frame * 1000.);
        assert!(per_frame < 1. / 24., "{per_frame} s a frame");
    }
}

fn query(e: &Editor, args: serde_json::Value) -> serde_json::Value {
    let a = args.as_object().unwrap().clone();
    serde_json::from_str(
        &command_exec::answer(e, "operandi.skin.query", &a)
            .unwrap()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn skin_set_query_and_bake_weights() {
    let mut a = arm(0.5, skin::LOCAL);
    bend(&mut a);
    let m = a.mesh;
    let q = query(&a.e, serde_json::json!({"item": m, "vertex": 0}));
    assert_eq!(q["joints"].as_array().unwrap().len(), 2);
    assert_eq!(q["weights"]["Elbow"], 0.5);
    let p = &q["position"];
    assert!((p[0].as_f64().unwrap() - 0.1).abs() < 1e-4, "{p}");
    ok(
        &mut a.e,
        &format!("operandi.skin.set item:{m} interpolation:dualQuaternion"),
    );
    near(point(&a, 0), DVec3::new(0.1414, 0.1414, 0.));
    // Sharpness binds harder to the strongest: make the elbow stronger.
    {
        let it = a.e.scene.item_mut(m).unwrap();
        let mut mesh = it.mesh.open().clone();
        mesh.attributes
            .point_weights
            .get_mut("Elbow")
            .unwrap()
            .insert(0, 0.75);
        mesh.attributes
            .point_weights
            .get_mut("Shoulder")
            .unwrap()
            .insert(0, 0.25);
        it.mesh = mesh.into();
    }
    ok(
        &mut a.e,
        &format!("operandi.skin.set item:{m} sharpness:2 threshold:0.01"),
    );
    let w = query(&a.e, serde_json::json!({"item": m, "vertex": 0}))["weights"]["Elbow"]
        .as_f64()
        .unwrap();
    assert!(w > 0.95, "{w}");
    ok(&mut a.e, &format!("operandi.skin.bakeWeights item:{m}"));
    let it = a.e.scene.item(m).unwrap();
    let baked = it.mesh.open().attributes.point_weights["Elbow"][&0];
    assert!((baked - w).abs() < 1e-9);
    let q = query(&a.e, serde_json::json!({"item": m}));
    assert_eq!(q["sharpness"], 0.0);
    assert_eq!(q["threshold"], 0.0);
    // Turning an influence off: its points follow the others only.
    let elbow = a.elbow;
    ok(
        &mut a.e,
        &format!("operandi.skin.set item:{m} joint:{elbow} enable:false"),
    );
    near(point(&a, 0), DVec3::new(0.2, 0., 0.));
    assert!(
        command_line::run_line(&mut a.e, "operandi.skin.set item:1 maxSources:9")
            .unwrap()
            .is_err()
    );
}
