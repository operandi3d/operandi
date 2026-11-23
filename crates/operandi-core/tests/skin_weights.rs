//! BEHAVIOR-RIG-PHASE5 §5: weight tools (SK-40..45) through the
//! `operandi.weight.*` commands.
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

fn query(e: &Editor, args: Value) -> Value {
    serde_json::from_str(
        &command_exec::answer(e, "operandi.weight.query", args.as_object().unwrap())
            .unwrap()
            .unwrap(),
    )
    .unwrap()
}

/// A symmetric bar across X, bound with Distance to a root at the
/// middle and _L / _R arms.
fn bound() -> (Editor, u64, [u64; 3]) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let mesh = last(&e);
    e.scene.item_mut(mesh).unwrap().mesh =
        primitive::segmented_cube(DVec3::ZERO, DVec3::new(2., 0.2, 0.2), [20, 1, 1])
            .unwrap()
            .into();
    ok(&mut e, "operandi.skel.add position:0,0,0 name:Root");
    let root = last(&e);
    ok(
        &mut e,
        &format!("operandi.skel.add position:0.8,0,0 parent:{root} symmetry:X name:Arm"),
    );
    let l = e
        .scene
        .items
        .iter()
        .find(|i| i.name == "Arm1_L")
        .unwrap()
        .id;
    let r = e
        .scene
        .items
        .iter()
        .find(|i| i.name == "Arm1_R")
        .unwrap()
        .id;
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("operandi.skin.bind items:{mesh} root:{root} type:distance"),
    );
    ok(&mut e, "operandi.anim.setup on:false");
    (e, mesh, [root, l, r])
}

fn weights(
    e: &Editor,
    mesh: u64,
) -> std::collections::BTreeMap<String, std::collections::BTreeMap<u32, f64>> {
    e.scene
        .item(mesh)
        .unwrap()
        .mesh
        .open()
        .attributes
        .point_weights
        .clone()
}

fn total(e: &Editor, mesh: u64, p: u32) -> f64 {
    query(e, json!({"item": mesh, "vertex": p}))["total"]
        .as_f64()
        .unwrap()
}

#[test]
fn set_query_and_adjust_with_normalize() {
    let (mut e, mesh, _) = bound();
    ok(
        &mut e,
        &format!("operandi.weight.set item:{mesh} map:Root1 points:0,1 value:0.9 normalize:true"),
    );
    let q = query(&e, json!({"item": mesh, "map": "Root1", "vertex": 0}));
    assert_eq!(q["value"], 0.9);
    assert!(total(&e, mesh, 0) <= 1. + 1e-9);
    ok(
        &mut e,
        &format!("operandi.weight.set item:{mesh} map:Root1 points:0 erase:true"),
    );
    assert!(query(&e, json!({"item": mesh, "map": "Root1", "vertex": 0}))["value"].is_null());
    ok(
        &mut e,
        &format!("operandi.weight.adjust item:{mesh} map:Root1 points:1 weight:0.5"),
    );
    assert_eq!(
        query(&e, json!({"item": mesh, "map": "Root1", "vertex": 1}))["value"],
        1.0
    );
    let steps = e.undo.len();
    ok(&mut e, &format!("operandi.weight.normalize item:{mesh}"));
    assert_eq!(e.undo.len(), steps + 1);
    for p in 0..e.scene.item(mesh).unwrap().mesh.open().vertices.len() as u32 {
        let t = total(&e, mesh, p);
        assert!(t == 0. || (t - 1.).abs() < 1e-9, "point {p}: {t}");
    }
}

#[test]
fn sk42_mirror_swaps_left_and_right() {
    let (mut e, mesh, _) = bound();
    let m = weights(&e, mesh);
    // Wreck the right side, then mirror left to right.
    let minus: Vec<String> = e
        .scene
        .item(mesh)
        .unwrap()
        .mesh
        .open()
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, p)| p.x < -1e-3)
        .map(|(i, _)| i.to_string())
        .collect();
    for map in ["Arm1_R", "Arm1_L", "Root1"] {
        ok(
            &mut e,
            &format!(
                "operandi.weight.set item:{mesh} map:{map} points:{} erase:true",
                minus.join(",")
            ),
        );
    }
    ok(&mut e, &format!("operandi.weight.mirror item:{mesh}"));
    let after = weights(&e, mesh);
    for map in ["Arm1_R", "Arm1_L", "Root1"] {
        assert_eq!(after[map].len(), m[map].len(), "{map}");
        for (p, w) in &m[map] {
            assert!((after[map][p] - w).abs() < 1e-9, "{map} {p}");
        }
    }
    // Totals still 100% (SK §8.2).
    let mesh_n = e.scene.item(mesh).unwrap().mesh.open().vertices.len() as u32;
    for p in 0..mesh_n {
        assert!((total(&e, mesh, p) - 1.).abs() < 1e-6);
    }
}

#[test]
fn sk42_smooth_merge_split_and_transfer() {
    let (mut e, mesh, _) = bound();
    let before = weights(&e, mesh)["Root1"].clone();
    ok(
        &mut e,
        &format!("operandi.weight.smooth item:{mesh} map:Root1 amount:1"),
    );
    assert_ne!(weights(&e, mesh)["Root1"], before);
    ok(
        &mut e,
        &format!("operandi.weight.merge item:{mesh} from:Arm1_L into:Root1"),
    );
    let w = weights(&e, mesh);
    assert!(!w.contains_key("Arm1_L"));
    ok(
        &mut e,
        &format!("operandi.weight.split item:{mesh} map:Root1"),
    );
    let w = weights(&e, mesh);
    assert!(w.contains_key("Root1_L") && w.contains_key("Root1_R") && !w.contains_key("Root1"));
    // Transfer onto a copy at the same place: same weights.
    e.scene.new_mesh().unwrap();
    let copy = last(&e);
    let geo = e.scene.item(mesh).unwrap().mesh.open().clone();
    let mut bare = geo.clone();
    bare.attributes.point_weights.clear();
    e.scene.item_mut(copy).unwrap().mesh = bare.into();
    ok(
        &mut e,
        &format!("operandi.weight.transfer item:{copy} from:{mesh}"),
    );
    assert_eq!(weights(&e, copy), geo.attributes.point_weights);
}

#[test]
fn sk23_a_joints_map_follows_its_rename() {
    let (mut e, mesh, [root, ..]) = bound();
    e.scene.select_item(Some(root));
    e.dispatch(Command::Rename("Hips".into())).unwrap();
    let w = weights(&e, mesh);
    assert!(w.contains_key("Hips") && !w.contains_key("Root1"));
    let q = command_exec::answer(
        &e,
        "operandi.skin.query",
        json!({"item": mesh}).as_object().unwrap(),
    )
    .unwrap()
    .unwrap();
    assert!(q.contains("\"map\":\"Hips\""), "{q}");
    assert_eq!(
        skin_weights::map_for_joint(&e.scene, root),
        Some((mesh, "Hips".into()))
    );
}

#[test]
fn sk43_painting_a_joint_map_normalises_across_the_skin_only() {
    let (mut e, mesh, _) = bound();
    // A non-joint map that painting must leave alone.
    {
        let it = e.scene.item_mut(mesh).unwrap();
        let mut m = it.mesh.open().clone();
        m.attributes.point_weights.insert(
            "Mask".into(),
            (0..m.vertices.len() as u32).map(|p| (p, 1.)).collect(),
        );
        it.mesh = m.into();
    }
    ok(
        &mut e,
        &format!("operandi.weight.set item:{mesh} map:Root1 points:3 value:1 normalize:true"),
    );
    let w = weights(&e, mesh);
    assert_eq!(w["Mask"][&3], 1., "not in the skin");
    let joints: f64 = ["Root1", "Arm1_L", "Arm1_R"]
        .iter()
        .filter_map(|m| w[*m].get(&3))
        .sum();
    assert!((joints - 1.).abs() < 1e-9);
}
