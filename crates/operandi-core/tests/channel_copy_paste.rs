//! #54: several selected channels copied and pasted onto another item's
//! channels, or onto other selected channels, as one undo step.
use glam::DVec3;
use operandi_core::animation::{self, ChannelKey};
use operandi_core::camera_light::LightKind;
use operandi_core::*;

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("a headless command line")
}
fn ok(e: &mut Editor, text: &str) {
    line(e, text).unwrap_or_else(|err| panic!("{text}: {err}"));
}

/// Two meshes: the first moved to (1, 2, 3) and turned 30° about Y.
fn two_items() -> (Editor, u64, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let a = e.scene.active().unwrap().id;
    e.scene.item_mut(a).unwrap().transform.translation = DVec3::new(1., 2., 3.);
    e.scene.item_mut(a).unwrap().transform.rotation = DVec3::new(0., 30., 0.);
    e.dispatch(Command::NewMesh).unwrap();
    let b = e.scene.active().unwrap().id;
    (e, a, b)
}
fn read(e: &Editor, item: u64, name: &str) -> f64 {
    animation::read(&e.scene, &ChannelKey::new(item, name)).unwrap()
}

#[test]
fn copied_channels_paste_onto_another_item_by_name() {
    let (mut e, a, b) = two_items();
    for name in ["pos.X", "pos.Y", "rot.Y"] {
        ok(
            &mut e,
            &format!("operandi.channel.select channel:{a}:{name} add:true"),
        );
    }
    ok(&mut e, "operandi.channel.copy");
    let undo = e.history.undo.len();
    // Onto item b's channels of the same names.
    ok(
        &mut e,
        &format!("operandi.channel.paste channels:{b}:pos.X,{b}:pos.Y,{b}:rot.Y"),
    );
    assert_eq!(read(&e, b, "pos.X"), 1.);
    assert_eq!(read(&e, b, "pos.Y"), 2.);
    assert_eq!(read(&e, b, "rot.Y"), 30.);
    assert_eq!(read(&e, b, "pos.Z"), 0., "not copied, not touched");
    assert_eq!(e.history.undo.len(), undo + 1, "one undo step");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(read(&e, b, "pos.X"), 0.);
}

#[test]
fn pasting_onto_other_names_goes_in_order_and_nothing_copied_says_so() {
    let (mut e, a, b) = two_items();
    let err = line(
        &mut e,
        &format!("operandi.channel.paste channels:{b}:pos.X"),
    )
    .unwrap_err();
    assert!(err.contains("Copy"), "{err}");
    ok(&mut e, &format!("operandi.channel.copy channels:{a}:pos.Z"));
    // One value onto a differently named channel: taken in order.
    ok(
        &mut e,
        &format!("operandi.channel.paste channels:{b}:pos.Y"),
    );
    assert_eq!(read(&e, b, "pos.Y"), 3.);
    // No channels given and none selected: the selected item's channels of
    // the copied names.
    e.anim.selected_channels.clear();
    ok(&mut e, "operandi.channel.paste");
    assert_eq!(read(&e, b, "pos.Z"), 3.);
}

/// Pasting onto selected items skips a copied channel an item doesn't
/// have (a light's Intensity onto a mesh) instead of failing the whole
/// paste.
#[test]
fn pasting_onto_items_skips_channels_they_lack() {
    let mut e = Editor::default();
    e.dispatch(Command::NewLight(LightKind::Point)).unwrap();
    let light = e.scene.active().unwrap().id;
    e.scene.item_mut(light).unwrap().transform.translation = DVec3::new(4., 0., 0.);
    ok(
        &mut e,
        &format!("operandi.channel.copy channels:{light}:pos.X,{light}:intensity"),
    );
    e.dispatch(Command::NewMesh).unwrap();
    let mesh = e.scene.active().unwrap().id;
    assert!(
        animation::read(&e.scene, &ChannelKey::new(mesh, "intensity")).is_none(),
        "a mesh has no Intensity"
    );
    e.anim.selected_channels.clear();
    ok(&mut e, "operandi.channel.paste");
    assert_eq!(read(&e, mesh, "pos.X"), 4.);
    // Nothing that fits: says so.
    ok(
        &mut e,
        &format!("operandi.channel.copy channels:{light}:intensity"),
    );
    let err = line(&mut e, "operandi.channel.paste").unwrap_err();
    assert!(err.contains("None of the copied"), "{err}");
}
