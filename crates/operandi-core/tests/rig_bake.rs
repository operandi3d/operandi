//! BEHAVIOR-RIG-PHASE4 §8 in core: Bake Selected, the scene with the rig
//! written as plain transforms (what USD export samples), and missing
//! pieces in files.
use glam::DVec3;
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn item(e: &mut Editor, at: DVec3) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.translation = at;
    id
}

fn world(e: &Editor, id: u64) -> DVec3 {
    let s = e.evaluated(evaluate::Purpose::Final);
    s.world_matrix(id).unwrap().w_axis.truncate()
}

fn near(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-6, "{a} vs {b}");
}

/// A part following a target that moves 0 → 10 in X over 24 frames.
fn following() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    let part = item(&mut e, DVec3::new(0., 1., 0.));
    let target = item(&mut e, DVec3::ZERO);
    ok(
        &mut e,
        &format!("operandi.key.add channel:{target}:pos.X frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{target}:pos.X frame:24 value:10"),
    );
    ok(
        &mut e,
        &format!(
            "operandi.rig.constrain kind:position items:{part} targets:{target} compensate:true"
        ),
    );
    (e, part, target)
}

#[test]
fn bake_turns_the_evaluated_motion_into_keys_and_disables_the_pieces() {
    let (mut e, part, _) = following();
    let before: Vec<DVec3> = (0..=24)
        .map(|f| {
            ok(&mut e, &format!("operandi.time.set frame:{f}"));
            world(&e, part)
        })
        .collect();
    let steps = e.undo.len();
    ok(
        &mut e,
        &format!("operandi.rig.bake items:{part} start:0 end:24"),
    );
    assert_eq!(e.undo.len(), steps + 1);
    // Disabled, not deleted.
    let p = &e.scene.animation.rig.pieces[0];
    assert_eq!(p.num("enable"), 0.);
    // Same motion from plain keys.
    for (f, want) in before.iter().enumerate() {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        near(world(&e, part), *want);
    }
    assert!(
        e.scene
            .animation
            .channels
            .contains_key(&animation::ChannelKey::new(part, "pos.X"))
    );
    // Undo brings the rig back.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.animation.rig.pieces[0].num("enable"), 1.);
}

#[test]
fn the_baked_scene_keeps_the_hierarchy_with_evaluated_locals() {
    let (mut e, part, target) = following();
    let parent = item(&mut e, DVec3::new(0., 0., 5.));
    ok(
        &mut e,
        &format!("operandi.rig.parent items:{part} parent:{parent} compensate:true"),
    );
    ok(&mut e, "operandi.time.set frame:12");
    let want = world(&e, part);
    let baked = rig_eval::baked_scene(&e.scene);
    assert!(baked.animation.rig.pieces.is_empty());
    assert_eq!(baked.item(part).unwrap().parent, Some(parent));
    near(baked.world_matrix(part).unwrap().w_axis.truncate(), want);
    assert!(rig_eval::rigged_items(&e.scene).contains(&part));
    assert!(!rig_eval::rigged_items(&e.scene).contains(&target));
}
