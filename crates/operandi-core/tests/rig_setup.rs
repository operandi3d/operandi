//! BEHAVIOR-RIG-PHASE5 §1: Setup mode and the setup (rest) pose (SK-01..03,
//! ACS-API-31: setup values stay apart from edit values).
use operandi_core::*;

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = command_line::run_line(e, text).expect("headless") {
        panic!("{text}: {err}");
    }
}

fn y(e: &Editor, id: u64) -> f64 {
    e.scene.item(id).unwrap().transform.translation.y
}

/// An item keyed pos.Y 0 → 4 over 0..10, set up at 1 (its value when
/// first keyed).
fn keyed() -> (Editor, u64) {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.translation.y = 1.;
    ok(
        &mut e,
        &format!("operandi.key.add channel:{id}:pos.Y frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{id}:pos.Y frame:10 value:4"),
    );
    ok(&mut e, "operandi.time.set frame:10");
    (e, id)
}

#[test]
fn sk01_setup_shows_the_setup_pose_and_edits_change_it() {
    let (mut e, id) = keyed();
    assert_eq!(y(&e, id), 4.);
    let steps = e.undo.len();
    ok(&mut e, "operandi.anim.setup on:true");
    assert_eq!(e.undo.len(), steps, "not an undo step");
    assert_eq!(y(&e, id), 1., "the setup value, keys ignored");
    // Scrubbing in setup mode keeps the setup pose.
    ok(&mut e, "operandi.time.set frame:5");
    assert_eq!(y(&e, id), 1.);
    // An edit changes the setup value, not the keys.
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:2 item:{id}"),
    );
    let c = &e.scene.animation.channels[&animation::ChannelKey::new(id, "pos.Y")];
    assert_eq!(c.rest, 2.);
    assert_eq!(c.curve.keys.len(), 2);
    assert_eq!(c.curve.keys[1].value, 4.);
    // Off: the animated pose again.
    ok(&mut e, "operandi.anim.setup on:false");
    ok(&mut e, "operandi.time.set frame:10");
    assert_eq!(y(&e, id), 4.);
}

#[test]
fn sk01_undo_keeps_setup_mode_as_it_is() {
    let (mut e, id) = keyed();
    ok(&mut e, "operandi.anim.setup on:true");
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:2 item:{id}"),
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.animation.setup, "still in setup mode");
    assert_eq!(y(&e, id), 1., "the old setup value, shown as setup");
    assert_eq!(
        e.scene.animation.channels[&animation::ChannelKey::new(id, "pos.Y")].rest,
        1.
    );
    // Not saved.
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(!back.animation.setup);
}

#[test]
fn sk03_item_to_setup_and_restore_setup() {
    let (mut e, id) = keyed();
    ok(&mut e, "operandi.time.set frame:5");
    let now = y(&e, id);
    let steps = e.undo.len();
    ok(&mut e, &format!("operandi.anim.itemToSetup items:{id}"));
    assert_eq!(e.undo.len(), steps + 1);
    let k = animation::ChannelKey::new(id, "pos.Y");
    assert_eq!(e.scene.animation.channels[&k].rest, now);
    ok(
        &mut e,
        &format!("operandi.anim.restoreSetup items:{id} channels:pos.Y"),
    );
    assert!(
        !e.scene.animation.channels.contains_key(&k),
        "edit values gone"
    );
    assert_eq!(y(&e, id), now, "back to setup");
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.animation.channels.contains_key(&k));
}
