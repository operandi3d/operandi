//! BEHAVIOR-RIG-PHASE4 §5 in core: channel links (RG-51 direct, RG-52
//! relationship) and user channels (RG-53, RG-54), replaying the measured
//! numbers.
use glam::DVec3;
use operandi_core::*;
use serde_json::Value;

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("a headless command line")
}

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = line(e, text) {
        panic!("{text}: {err}");
    }
}

fn item(e: &mut Editor) -> u64 {
    e.scene.new_mesh().unwrap();
    e.scene.items.last().unwrap().id
}

/// A channel as shown (evaluated).
fn shown(e: &Editor, item: u64, channel: &str) -> f64 {
    let s = e.evaluated(evaluate::Purpose::Final);
    animation::read(&s, &animation::ChannelKey::new(item, channel)).unwrap()
}

fn query(e: &Editor, item: u64) -> Value {
    let a = serde_json::json!({ "item": item })
        .as_object()
        .unwrap()
        .clone();
    serde_json::from_str(
        &command_exec::answer(e, "operandi.rig.query", &a)
            .unwrap()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn rg51_a_direct_link_copies_the_raw_value() {
    let mut e = Editor::default();
    let driven = item(&mut e);
    let driver = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{driven}:scl.Z driver:{driver}:pos.X"),
    );
    for x in [0., 0.5, 2., -1.] {
        e.scene.item_mut(driver).unwrap().transform.translation.x = x;
        assert!((shown(&e, driven, "scl.Z") - x).abs() < 1e-9, "{x}");
    }
    // The driven item's own value is untouched (ACS-API-31) and the
    // channel reports as driven (RG-04).
    assert_eq!(e.scene.item(driven).unwrap().transform.scale.z, 1.);
    assert!(rig::driven_by(&e.scene, driven, "scl.Z").is_some());
    assert!(rig::drives_channel(&e.scene, driver, "pos.X"));
    // The world follows: the shown mesh scales.
    let s = e.evaluated(evaluate::Purpose::Final);
    let w = s.world_matrix(driven).unwrap();
    assert!((w.z_axis.truncate().length() - 1.).abs() < 1e-9);
}

#[test]
fn rg51_vectors_link_by_component_and_text_is_refused() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let b = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{a}:rot driver:{b}:pos"),
    );
    assert_eq!(e.scene.animation.rig.pieces.len(), 3);
    e.scene.item_mut(b).unwrap().transform.translation = DVec3::new(0.1, 0.2, 0.3);
    assert!((shown(&e, a, "rot.Y") - 0.2).abs() < 1e-9);
    ok(
        &mut e,
        &format!("operandi.rig.userChannelAdd item:{b} name:label type:string"),
    );
    let err = line(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{a}:pos.X driver:{b}:label"),
    )
    .unwrap_err();
    assert!(err.contains("Text channels"), "{err}");
}

#[test]
fn rg52_a_relationship_goes_through_its_curve() {
    let mut e = Editor::default();
    let driven = item(&mut e);
    let driver = item(&mut e);
    ok(
        &mut e,
        &format!(
            "operandi.rig.linkAdd driven:{driven}:scl.Y driver:{driver}:pos.X mode:relationship"
        ),
    );
    let p = e.scene.animation.rig.pieces[0].id;
    ok(
        &mut e,
        &format!("operandi.rig.relationshipKey piece:{p} driver:0 driven:0"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.relationshipKey piece:{p} driver:10 driven:1"),
    );
    for (x, want) in [(5., 0.5), (0., 0.), (10., 1.), (-3., 0.), (15., 1.)] {
        e.scene.item_mut(driver).unwrap().transform.translation.x = x;
        assert!((shown(&e, driven, "scl.Y") - want).abs() < 1e-9, "{x}");
    }
    // The current values as a key: driver now 4, driven's own value 1.
    e.scene.item_mut(driver).unwrap().transform.translation.x = 4.;
    ok(&mut e, &format!("operandi.rig.relationshipKey piece:{p}"));
    assert_eq!(
        query(&e, driven)["pieces"][0]["curve"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    ok(
        &mut e,
        &format!("operandi.rig.relationshipKeyRemove piece:{p} driver:4"),
    );
    assert_eq!(
        query(&e, driven)["pieces"][0]["curve"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn rg53_user_channels_are_keyable_linkable_saved_and_deleted_with_their_links() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let b = item(&mut e);
    ok(
        &mut e,
        &format!(
            "operandi.rig.userChannelAdd item:{a} name:spin type:angle default:0 min:-10 max:10"
        ),
    );
    ok(
        &mut e,
        &format!("operandi.rig.userChannelAdd item:{a} name:offset type:distance mode:xyz"),
    );
    let q = query(&e, a);
    assert_eq!(q["user"].as_array().unwrap().len(), 2);
    assert!(animation::keyable_channels(&e.scene, a).contains(&"offset.Y".to_string()));
    // Keyed like any channel; clamped to its range.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:spin frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:spin frame:10 value:99"),
    );
    ok(&mut e, "operandi.time.set frame:10");
    assert_eq!(
        animation::read(&e.scene, &animation::ChannelKey::new(a, "spin")),
        Some(10.)
    );
    // Linked: b's rot.Z follows spin.
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{b}:rot.Z driver:{a}:spin"),
    );
    assert!((shown(&e, b, "rot.Z") - 10.).abs() < 1e-9);
    // Rename keeps keys and links.
    ok(
        &mut e,
        &format!("operandi.rig.userChannelEdit item:{a} name:spin rename:crank"),
    );
    assert!(
        e.scene
            .animation
            .channels
            .contains_key(&animation::ChannelKey::new(a, "crank"))
    );
    assert!((shown(&e, b, "rot.Z") - 10.).abs() < 1e-9);
    // Saved and reopened.
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back.animation.rig, e.scene.animation.rig);
    // Delete removes its links too, in one step.
    let steps = e.undo.len();
    ok(
        &mut e,
        &format!("operandi.rig.userChannelRemove item:{a} name:crank"),
    );
    assert_eq!(e.undo.len(), steps + 1);
    assert!(e.scene.animation.rig.pieces.is_empty());
    assert!(e.anim.note.as_deref().is_some_and(|n| n.contains("1 link")));
}

#[test]
fn rg54_bind_into_user_channel() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let ctrl = item(&mut e);
    e.scene.item_mut(a).unwrap().transform.translation.y = 2.;
    ok(
        &mut e,
        &format!("operandi.rig.bindUserChannel channel:{a}:pos.Y target:{ctrl}"),
    );
    let user = &e.scene.animation.rig.user[0];
    assert_eq!(
        (user.item, user.name.as_str(), user.default),
        (ctrl, "pos_Y", 2.)
    );
    assert!(rig::driven_by(&e.scene, a, "pos.Y").is_some());
    let piece = e.scene.animation.rig.pieces[0].id;
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{piece} channel:enable value:1"),
    );
    let k = animation::ChannelKey::new(ctrl, "pos_Y");
    animation::write(&mut e.scene, &k, 5.).unwrap();
    assert!((shown(&e, a, "pos.Y") - 5.).abs() < 1e-9);
}

#[test]
fn link_loops_are_refused() {
    let mut e = Editor::default();
    let a = item(&mut e);
    let b = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{a}:pos.X driver:{b}:pos.Y"),
    );
    let err = line(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{b}:pos.Y driver:{a}:pos.X"),
    )
    .unwrap_err();
    assert!(err.contains("depend on itself"), "{err}");
    // A chain is fine and evaluates in order.
    let c = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{c}:pos.Z driver:{a}:pos.X"),
    );
    e.scene.item_mut(b).unwrap().transform.translation.y = 7.;
    assert!((shown(&e, c, "pos.Z") - 7.).abs() < 1e-9);
}
