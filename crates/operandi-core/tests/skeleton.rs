//! BEHAVIOR-RIG-PHASE5 §2: joints and the Skeleton tool's edits
//! (SK-10..19), through the `operandi.skel.*` commands.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

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

fn pos(e: &Editor, id: u64) -> DVec3 {
    e.scene.world_matrix(id).unwrap().w_axis.truncate()
}

fn near(a: DVec3, b: DVec3) {
    assert!(a.distance(b) < 1e-9, "{a} vs {b}");
}

fn last(e: &Editor) -> u64 {
    e.scene.items.last().unwrap().id
}

fn by_name(e: &Editor, name: &str) -> u64 {
    e.scene
        .items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no {name}"))
        .id
}

/// Root at the origin, then (0,1,0), then (0,2,0): a 3-joint chain.
fn chain() -> (Editor, [u64; 3]) {
    let mut e = Editor::default();
    ok(&mut e, "operandi.skel.add position:0,0,0");
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,1,0 parent:{a}"),
    );
    let b = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0,2,0 parent:{b}"),
    );
    let c = last(&e);
    (e, [a, b, c])
}

#[test]
fn sk10_joints_chain_as_children_and_each_add_is_one_step() {
    let (mut e, [a, b, c]) = chain();
    assert!(e.scene.item(a).unwrap().is_joint());
    assert_eq!(e.scene.item(b).unwrap().parent, Some(a));
    assert_eq!(e.scene.item(c).unwrap().parent, Some(b));
    near(pos(&e, c), DVec3::new(0., 2., 0.));
    assert_eq!(e.undo.len(), 3, "SK-19 one step per joint");
    assert_eq!(e.scene.selection.item, Some(c));
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.item(c).is_none());
    e.dispatch(Command::Redo).unwrap();
    near(pos(&e, c), DVec3::new(0., 2., 0.));
    // SK-15 automatic numbers.
    let names: Vec<_> = [a, b, c]
        .iter()
        .map(|i| e.scene.item(*i).unwrap().name.clone())
        .collect();
    assert_eq!(names, ["Joint1", "Joint2", "Joint3"]);
}

#[test]
fn sk16_a_joint_aims_z_down_its_bone_x_across() {
    let (e, [a, b, _]) = chain();
    let w = e.scene.world_matrix(a).unwrap();
    near(w.z_axis.truncate(), DVec3::Y);
    assert!(w.x_axis.truncate().dot(DVec3::Y).abs() < 1e-9);
    // Aiming the parent didn't move the child.
    near(pos(&e, b), DVec3::new(0., 1., 0.));
}

#[test]
fn sk16_preserve_alignment_keeps_orientation_and_align_reaims() {
    let mut e = Editor::default();
    ok(&mut e, "operandi.skel.add position:0,0,0");
    let a = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:1,0,0 parent:{a} preserveAlignment:true"),
    );
    near(e.scene.world_matrix(a).unwrap().z_axis.truncate(), DVec3::Z);
    ok(
        &mut e,
        &format!("operandi.skel.align joint:{a} direction:X axis:Y"),
    );
    let w = e.scene.world_matrix(a).unwrap();
    near(w.x_axis.truncate(), DVec3::X);
    assert!(w.y_axis.truncate().dot(DVec3::X).abs() < 1e-9);
    near(pos(&e, last(&e)), DVec3::X);
    assert!(err(&mut e, "operandi.skel.align direction:Q").contains("X, Y or Z"));
}

#[test]
fn sk13_move_with_and_without_compensation() {
    let (mut e, [_, b, c]) = chain();
    ok(
        &mut e,
        &format!("operandi.skel.move joint:{b} position:1,1,0"),
    );
    near(pos(&e, b), DVec3::new(1., 1., 0.));
    near(pos(&e, c), DVec3::new(0., 2., 0.));
    ok(
        &mut e,
        &format!("operandi.skel.move joint:{b} position:2,1,0 compensate:false"),
    );
    near(pos(&e, c), DVec3::new(1., 2., 0.));
    e.dispatch(Command::Undo).unwrap();
    near(pos(&e, c), DVec3::new(0., 2., 0.));
}

#[test]
fn sk12_insert_splits_a_bone_and_delete_reparents_in_place() {
    let (mut e, [a, b, c]) = chain();
    ok(&mut e, &format!("operandi.skel.insert joint:{b}"));
    let mid = e.scene.selection.item.unwrap();
    near(pos(&e, mid), DVec3::new(0., 1.5, 0.));
    assert_eq!(e.scene.item(c).unwrap().parent, Some(mid));
    assert_eq!(skeleton::first_child(&e.scene, b), Some(mid));
    near(pos(&e, c), DVec3::new(0., 2., 0.));
    ok(&mut e, &format!("operandi.skel.delete joint:{b}"));
    assert!(e.scene.item(b).is_none());
    assert_eq!(e.scene.item(mid).unwrap().parent, Some(a));
    near(pos(&e, mid), DVec3::new(0., 1.5, 0.));
    assert!(err(&mut e, &format!("operandi.skel.insert joint:{c}")).contains("leaf"));
    e.scene.new_mesh().unwrap();
    let m = last(&e);
    assert!(err(&mut e, &format!("operandi.skel.delete joint:{m}")).contains("isn't a joint"));
}

#[test]
fn sk14_sk15_symmetry_mirrors_and_names_by_side() {
    let mut e = Editor::default();
    ok(&mut e, "operandi.skel.add position:0,1,0 symmetry:X");
    let root = last(&e);
    assert_eq!(
        e.scene.item(root).unwrap().name,
        "Joint1",
        "on the plane: one joint, no suffix"
    );
    ok(
        &mut e,
        &format!("operandi.skel.add position:1,1,0 parent:{root} symmetry:X prefix:arm_"),
    );
    let l = by_name(&e, "arm_Joint1_L");
    let r = by_name(&e, "arm_Joint1_R");
    near(pos(&e, l), DVec3::new(1., 1., 0.));
    near(pos(&e, r), DVec3::new(-1., 1., 0.));
    assert_eq!(e.scene.item(r).unwrap().parent, Some(root));
    ok(
        &mut e,
        &format!("operandi.skel.add position:2,1,0 parent:{l} symmetry:X prefix:arm_"),
    );
    let r2 = by_name(&e, "arm_Joint2_R");
    assert_eq!(
        e.scene.item(r2).unwrap().parent,
        Some(r),
        "under the mirrored parent"
    );
    near(pos(&e, r2), DVec3::new(-2., 1., 0.));
    assert_eq!(e.undo.len(), 3, "a mirrored pair is one step");
}

#[test]
fn sk17_first_child_in_item_order_decides_the_bone() {
    let (mut e, [a, b, _]) = chain();
    ok(
        &mut e,
        &format!("operandi.skel.add position:1,0,0 parent:{a}"),
    );
    let side = last(&e);
    assert_eq!(skeleton::children(&e.scene, a), vec![b, side]);
    assert_eq!(skeleton::first_child(&e.scene, a), Some(b));
    assert!((skeleton::bone_length(&e.scene, a) - 1.).abs() < 1e-9);
    // A second child doesn't re-aim the parent.
    near(e.scene.world_matrix(a).unwrap().z_axis.truncate(), DVec3::Y);
}

#[test]
fn sk11_placement_lands_halfway_through_the_mesh() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let m = last(&e);
    e.scene.item_mut(m).unwrap().mesh = primitive::cube_geometry(DVec3::ZERO, DVec3::ONE)
        .unwrap()
        .into();
    let o = DVec3::new(0.2, 0.1, 5.);
    let mid = skeleton::placement(&e.scene, o, -DVec3::Z, true).unwrap();
    near(mid, DVec3::new(0.2, 0.1, 0.));
    let front = skeleton::placement(&e.scene, o, -DVec3::Z, false).unwrap();
    near(front, DVec3::new(0.2, 0.1, 0.5));
    assert!(skeleton::placement(&e.scene, DVec3::new(3., 0., 5.), -DVec3::Z, true).is_none());
}

#[test]
fn joints_save_reopen_and_query() {
    let (e, [a, _, c]) = chain();
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(back.item(c).unwrap().is_joint());
    assert_eq!(
        back.item(c).unwrap().parent,
        e.scene.item(c).unwrap().parent
    );
    let args = serde_json::json!({ "root": a })
        .as_object()
        .unwrap()
        .clone();
    let text = command_exec::answer(&e, "operandi.skel.query", &args)
        .unwrap()
        .unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    let joints = v["joints"].as_array().unwrap();
    assert_eq!(joints.len(), 3);
    assert_eq!(joints[0]["firstChild"], joints[1]["id"]);
    assert_eq!(joints[0]["boneLength"], 1.0);
    assert_eq!(joints[2]["position"], serde_json::json!([0.0, 2.0, 0.0]));
}
