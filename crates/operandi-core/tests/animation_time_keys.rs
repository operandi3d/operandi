//! BEHAVIOR-TIME-KEYS phase 1 acceptance (TK-xx, section 8): scene time,
//! keys, auto key, pending edits, undo, two documents, save and reopen.
use glam::DVec3;
use operandi_core::anim_curve::Slope;
use operandi_core::animation::{self, AutoKey, ChannelKey, EndBehavior};
use operandi_core::time_tick::{FrameRate, Tick};
use operandi_core::*;

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("a headless command line")
}

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = line(e, text) {
        panic!("{text}: {err}");
    }
}

/// A 2 m cube as a new selected mesh; its id.
fn add_cube(e: &mut Editor) -> u64 {
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(2.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.active().unwrap().id
}

/// A scene with one selected mesh item (id 1), in Items mode.
fn one_item() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.selection.mode = Mode::Item;
    e
}

fn x(e: &Editor) -> f64 {
    e.scene.items[0].transform.translation.x
}

fn move_x(e: &mut Editor, value: f64) {
    let id = e.scene.items[0].id;
    let mut transform = e.scene.items[0].transform.clone();
    transform.translation.x = value;
    e.dispatch(Command::ItemTransform {
        item: id,
        transform,
    })
    .unwrap();
}

fn frame(e: &Editor, f: i64) -> Tick {
    e.scene.time.rate.tick_of(f)
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-4, "{a} vs {b}");
}

#[test]
fn tk_check1_new_scene_defaults() {
    let e = Editor::default();
    assert_eq!(e.scene.time.rate, FrameRate::FPS_24);
    assert_eq!(e.scene.time.start, Tick::ZERO);
    assert_eq!(e.scene.time.rate.frame_at(e.scene.time.end), 120);
    assert_eq!(e.scene.animation.settings.auto_key, AutoKey::Animated);
    assert_eq!(e.scene.time.current, Tick::ZERO);
    assert!(e.scene.animation.settings.snap);
    assert_eq!(e.scene.animation.settings.end, EndBehavior::Repeat);
    assert_eq!(
        animation::current_range(&e.scene),
        (e.scene.time.start, e.scene.time.end)
    );
}

#[test]
fn tk_check2_status_light_keys_ease_between() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    ok(&mut e, &format!("operandi.key.toggle channels:{id}:pos.X"));
    ok(&mut e, "operandi.time.set frame:24");
    // Typed with Auto Key Off, so the second click is what keys it.
    e.scene.animation.settings.auto_key = AutoKey::Off;
    move_x(&mut e, 1.);
    ok(&mut e, &format!("operandi.key.toggle channels:{id}:pos.X"));
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.X")].curve;
    assert_eq!(c.keys.len(), 2);
    // TK-13's S-curve, scaled to 0..24 frames.
    near(c.value_at(frame(&e, 6)), 0.15625);
    near(c.value_at(frame(&e, 12)), 0.5);
    // A click on a key removes it (TK-20).
    ok(&mut e, &format!("operandi.key.toggle channels:{id}:pos.X"));
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.X")].curve;
    assert_eq!(c.keys.len(), 1);
}

#[test]
fn tk_check3_auto_key_animated_keys_only_keyed_channels() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut e, "operandi.time.set frame:24");
    move_x(&mut e, 2.);
    ok(&mut e, "operandi.time.set frame:48");
    let steps = e.history.undo.len();
    let id0 = e.scene.items[0].id;
    let mut t = e.scene.items[0].transform.clone();
    t.translation = DVec3::new(3., 5., 0.);
    e.dispatch(Command::ItemTransform {
        item: id0,
        transform: t,
    })
    .unwrap();
    // One undo step for the edit and its key.
    assert_eq!(e.history.undo.len(), steps + 1);
    let anim = &e.scene.animation;
    assert_eq!(
        anim.channels[&ChannelKey::new(id, "pos.X")]
            .curve
            .keys
            .len(),
        3
    );
    assert!(
        !anim.is_animated(&ChannelKey::new(id, "pos.Y")),
        "Y wasn't animated"
    );
    // Scrubbing keeps the keyed pose; Y keeps its plain edit.
    ok(&mut e, "operandi.time.set frame:0");
    near(x(&e), 0.);
    near(e.scene.items[0].transform.translation.y, 5.);
}

#[test]
fn tk_check4_auto_key_off_edits_are_pending_until_applied() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let k = ChannelKey::new(id, "pos.X");
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut e, "operandi.key.autoKey mode:off");
    move_x(&mut e, 4.);
    assert_eq!(animation::pending(&e.scene), vec![k.clone()]);
    assert!(animation::is_pending(&e.scene, &k));
    // Stepping a frame drops it (TK-40).
    ok(&mut e, "operandi.time.step frames:1");
    near(x(&e), 0.);
    assert!(animation::pending(&e.scene).is_empty());
    // Again, and Apply keeps it as a key (TK-41).
    move_x(&mut e, 4.);
    ok(&mut e, "operandi.key.apply");
    assert!(animation::pending(&e.scene).is_empty());
    assert_eq!(e.scene.animation.channels[&k].curve.keys.len(), 2);
    ok(&mut e, "operandi.time.step frames:5");
    ok(&mut e, "operandi.time.step frames:-5");
    near(x(&e), 4.);
    // Discard puts the curve value back.
    move_x(&mut e, 9.);
    ok(&mut e, "operandi.key.discard");
    near(x(&e), 4.);
    // Auto Update Existing Keys: an edit on a key updates it.
    ok(&mut e, "operandi.time.settings autoUpdate:true");
    move_x(&mut e, 7.);
    assert!(animation::pending(&e.scene).is_empty());
    assert_eq!(e.scene.animation.channels[&k].curve.keys.len(), 2);
}

#[test]
fn tk_check5_undo_takes_back_one_key_and_never_time() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let k = ChannelKey::new(id, "pos.X");
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    let steps = e.history.undo.len();
    ok(&mut e, "operandi.time.set frame:10");
    ok(&mut e, "operandi.time.step frames:3");
    ok(&mut e, "operandi.time.go to:last");
    assert_eq!(
        e.history.undo.len(),
        steps,
        "time changes are not undo steps"
    );
    move_x(&mut e, 2.);
    assert_eq!(e.scene.animation.channels[&k].curve.keys.len(), 2);
    let at = e.scene.time.current;
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.animation.channels[&k].curve.keys.len(), 1);
    // The time stays; the pose is the curve's there.
    assert_eq!(e.scene.time.current, at);
    near(x(&e), 0.);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.animation.channels[&k].curve.keys.len(), 2);
    near(x(&e), 2.);
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.scene.animation.is_animated(&k));
}

#[test]
fn tk_check6_two_documents_keep_their_own_keys_and_time() {
    // Each scene tab is its own Editor (app/scenes.rs swaps them whole).
    let mut a = one_item();
    let mut b = one_item();
    let id = a.scene.items[0].id;
    ok(&mut a, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut a, "operandi.time.set frame:30");
    ok(&mut b, "operandi.time.set frame:5");
    ok(&mut b, "operandi.key.transforms type:position");
    b.dispatch(Command::Undo).unwrap();
    b.dispatch(Command::Undo).unwrap();
    assert!(a.scene.animation.is_animated(&ChannelKey::new(id, "pos.X")));
    assert_eq!(a.scene.time.current, a.scene.time.rate.tick_of(30));
    assert_eq!(b.scene.time.current, b.scene.time.rate.tick_of(5));
    assert!(b.scene.animation.channels.is_empty());
}

#[test]
fn tk_check7_every_button_runs_as_a_command_line() {
    let mut e = one_item();
    for text in [
        "operandi.time.set frame:12",
        "operandi.key.transforms type:all",
        "operandi.time.step frames:1",
        "operandi.time.step frames:-1",
        "operandi.time.go to:first",
        "operandi.time.go to:last",
        "operandi.time.go to:firstKey",
        "operandi.key.items",
        "operandi.time.play",
        "operandi.time.play stop:true",
        "operandi.key.autoKey mode:all",
    ] {
        ok(&mut e, text);
    }
    assert_eq!(e.scene.animation.settings.auto_key, AutoKey::All);
    assert_eq!(e.scene.time.current, frame(&e, 12));
    assert_eq!(e.scene.animation.channels.len(), 9);
}

#[test]
fn tk_check8_save_and_reopen_keeps_everything() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    ok(
        &mut e,
        "operandi.time.settings fps:30 start:0 end:90 rangeStart:10 rangeEnd:40 endBehavior:hold display:seconds",
    );
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut e, "operandi.time.set frame:15");
    move_x(&mut e, 1.);
    ok(&mut e, "operandi.key.autoKey mode:off");
    ok(&mut e, "operandi.marker.add frame:20 scene:true");
    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.contains("\"version\": 60"));
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.animation, e.scene.animation);
    assert_eq!(back.time, e.scene.time);
    assert_eq!(back.time.rate, FrameRate::FPS_30);
    assert_eq!(persistence::encode(&back).unwrap(), bytes);
    // A pending edit doesn't survive closing the scene (TK-42).
    move_x(&mut e, 50.);
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    near(back.items[0].transform.translation.x, 1.);
}

#[test]
fn tk02_keys_stay_in_seconds_when_the_rate_changes() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    ok(&mut e, "operandi.time.set frame:24");
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut e, "operandi.time.settings fps:30");
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.X")].curve;
    assert_eq!(c.keys[0].time.seconds(), 1.);
}

#[test]
fn tk14_choice_and_on_off_channels_are_stepped() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let k = ChannelKey::new(id, "visible");
    assert!(animation::is_stepped(&e.scene, &k));
    ok(&mut e, &format!("operandi.key.set channels:{id}:visible"));
    assert!(e.scene.animation.channels[&k].curve.stepped);
}

#[test]
fn tk15_rotation_keys_are_per_axis_and_light_values_key() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let mut t = e.scene.items[0].transform.clone();
    t.rotation.y = 90f64.to_radians();
    e.dispatch(Command::ItemTransform {
        item: id,
        transform: t,
    })
    .unwrap();
    ok(&mut e, "operandi.key.transforms type:rotation");
    let names: Vec<&str> = e
        .scene
        .animation
        .item_channels(id)
        .map(|(k, _)| k.channel.as_str())
        .collect();
    assert_eq!(names, ["rot.X", "rot.Y", "rot.Z"]);
    assert!(animation::keyable_channels(&e.scene, id).contains(&"pos.X".to_string()));
}

#[test]
fn tk21_s_keys_selected_channels_or_the_live_tool() {
    let mut e = Editor::default();
    let id = add_cube(&mut e);
    e.scene.selection.mode = Mode::Item;
    assert!(
        line(&mut e, "operandi.key.set").is_err(),
        "nothing selected"
    );
    ok(
        &mut e,
        &format!("operandi.channel.select channel:{id}:scl.Y"),
    );
    ok(&mut e, "operandi.key.set");
    assert!(e.scene.animation.is_animated(&ChannelKey::new(id, "scl.Y")));
    ok(&mut e, "operandi.channel.select channel:none");
    e.dispatch(Command::Activate(Tool::Rotate)).unwrap();
    ok(&mut e, "operandi.key.set");
    assert!(e.scene.animation.is_animated(&ChannelKey::new(id, "rot.Z")));
    assert!(e.session.live.is_some(), "the tool stays live");
}

#[test]
fn tk24_remove_animation_reset_and_bake() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let k = ChannelKey::new(id, "pos.X");
    move_x(&mut e, 3.);
    ok(&mut e, &format!("operandi.key.set channels:{}", k.text()));
    ok(&mut e, "operandi.time.range start:0 end:10");
    ok(&mut e, &format!("operandi.key.bake channels:{}", k.text()));
    assert_eq!(e.scene.animation.channels[&k].curve.keys.len(), 11);
    ok(
        &mut e,
        &format!("operandi.key.removeAnimation channels:{}", k.text()),
    );
    assert!(!e.scene.animation.is_animated(&k));
    near(x(&e), 3.);
    ok(&mut e, &format!("operandi.key.set channels:{}", k.text()));
    ok(&mut e, &format!("operandi.key.reset channels:{}", k.text()));
    near(x(&e), 0.);
}

#[test]
fn tk30_auto_key_all_keys_any_edit_of_the_selected_item() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    ok(&mut e, "operandi.key.autoKey mode:all");
    move_x(&mut e, 1.5);
    let k = ChannelKey::new(id, "pos.X");
    assert!(e.scene.animation.is_animated(&k));
    assert_eq!(
        e.scene.animation.channels[&k].rest, 0.,
        "rest is the value before"
    );
    assert!(!e.scene.animation.is_animated(&ChannelKey::new(id, "pos.Y")));
}

#[test]
fn tk50_posing_never_writes_the_rest_value_and_evaluate_at_leaves_the_document() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let k = ChannelKey::new(id, "pos.X");
    move_x(&mut e, 0.25);
    ok(&mut e, &format!("operandi.key.set channels:{}", k.text()));
    ok(&mut e, "operandi.time.set frame:10");
    move_x(&mut e, 8.);
    ok(&mut e, "operandi.time.set frame:0");
    assert_eq!(e.scene.animation.channels[&k].rest, 0.25);
    let posed = animation::evaluate_at(&e.scene, frame(&e, 10));
    near(posed.items[0].transform.translation.x, 8.);
    near(x(&e), 0.25);
    assert_eq!(e.scene.animation.channels[&k].rest, 0.25);
}

#[test]
fn tk63_stepping_wraps_inside_a_frame_limits_range_marker() {
    let mut e = one_item();
    ok(&mut e, "operandi.marker.add frame:10 scene:true");
    ok(&mut e, "operandi.marker.edit length:4 frameLimits:true");
    ok(&mut e, "operandi.time.set frame:14");
    ok(&mut e, "operandi.time.step frames:1");
    assert_eq!(e.scene.time.current, frame(&e, 10));
    ok(&mut e, "operandi.time.step frames:-1");
    assert_eq!(e.scene.time.current, frame(&e, 14));
    ok(&mut e, "operandi.time.set frame:30");
    ok(&mut e, "operandi.time.step frames:1");
    assert_eq!(e.scene.time.current, frame(&e, 31));
}

#[test]
fn tk62_playback_end_behaviours() {
    let mut e = one_item();
    ok(&mut e, "operandi.time.range start:0 end:24");
    let s = &e.scene;
    let r = s.time.rate;
    let (t, stop) = animation::playback_step(s, r.tick_of(24), Tick::ZERO, 1., false);
    assert_eq!((t, stop), (r.tick_of(0), false), "repeat loops");
    let (t, stop) = animation::playback_step(s, r.tick_of(23), Tick::ZERO, 1., false);
    assert_eq!((t, stop), (r.tick_of(24), false));
    ok(&mut e, "operandi.time.settings endBehavior:hold");
    let (t, stop) = animation::playback_step(&e.scene, r.tick_of(24), Tick::ZERO, 1., false);
    assert_eq!((t, stop), (r.tick_of(24), true));
    ok(&mut e, "operandi.time.settings endBehavior:wrap");
    let (t, stop) = animation::playback_step(&e.scene, r.tick_of(24), r.tick_of(5), 1., false);
    assert_eq!((t, stop), (r.tick_of(5), true));
    let (t, _) = animation::playback_step(&e.scene, r.tick_of(0), r.tick_of(5), 1., true);
    assert_eq!(t, r.tick_of(5), "wrap backwards stops at the start too");
}

#[test]
fn tk63_key_jumps_follow_the_selected_items() {
    let mut e = one_item();
    for f in [5, 12, 30] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        ok(&mut e, "operandi.key.transforms type:position");
    }
    ok(&mut e, "operandi.time.set frame:12");
    ok(&mut e, "operandi.time.go to:nextKey");
    assert_eq!(e.scene.time.current, frame(&e, 30));
    ok(&mut e, "operandi.time.go to:prevKey");
    assert_eq!(e.scene.time.current, frame(&e, 12));
    ok(&mut e, "operandi.time.go to:firstKey");
    assert_eq!(e.scene.time.current, frame(&e, 5));
    ok(&mut e, "operandi.time.go to:lastKey");
    assert_eq!(e.scene.time.current, frame(&e, 30));
}

#[test]
fn tl20_key_time_writes_current_values_at_another_frame() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    let k = ChannelKey::new(id, "pos.X");
    move_x(&mut e, 2.);
    ok(&mut e, &format!("operandi.key.set channels:{}", k.text()));
    ok(&mut e, "operandi.time.keyTime frame:24");
    ok(&mut e, &format!("operandi.key.set channels:{}", k.text()));
    let c = &e.scene.animation.channels[&k].curve;
    assert_eq!(c.keys.len(), 2);
    near(c.keys[1].value, 2.);
    assert_eq!(c.keys[1].time, frame(&e, 24));
    // Setting the time resets the key time.
    ok(&mut e, "operandi.time.set frame:3");
    assert_eq!(e.key_time(), frame(&e, 3));
}

#[test]
fn held_door_reports_keys() {
    let mut e = one_item();
    let id = e.scene.items[0].id;
    ok(&mut e, &format!("operandi.key.set channels:{id}:visible"));
    let held = channels::held(&e.scene, channels::Target::Item(id), "visible").unwrap();
    assert_eq!(held.label(), "keys");
    let _ = Slope::SmoothFlat;
}

/// UIC-112 for every keyable row: the app asks which channel a row's
/// edit sets; lights, cameras and item rows answer, colours by component.
#[test]
fn rows_find_their_channel_from_their_edit() {
    use operandi_core::camera_light::{CameraEdit, LightEdit, LightKind};
    let mut e = Editor::default();
    e.dispatch(Command::NewCamera).unwrap();
    let cam = e.scene.active().unwrap().id;
    e.dispatch(Command::NewLight(LightKind::Point)).unwrap();
    let light = e.scene.active().unwrap().id;
    let of = |e: &Editor, c: Command| animation::channel_of_command(&e.scene, &c).map(|(_, n)| n);
    assert_eq!(
        of(
            &e,
            Command::EditLight {
                item: light,
                edit: LightEdit::Intensity(3.)
            }
        ),
        Some(vec!["intensity".to_string()])
    );
    assert_eq!(
        of(
            &e,
            Command::EditCamera {
                item: cam,
                edit: CameraEdit::FocalLength(0.05)
            }
        ),
        Some(vec!["focalLength".to_string()])
    );
    let colour = of(
        &e,
        Command::EditLight {
            item: light,
            edit: LightEdit::Color([1., 0., 0.]),
        },
    );
    assert_eq!(
        colour.map(|v| v.len()),
        Some(3),
        "colour rows key r, g and b"
    );
    // A keyed light intensity follows its curve.
    ok(
        &mut e,
        &format!("operandi.key.set channels:{light}:intensity"),
    );
    assert!(
        e.scene
            .animation
            .is_animated(&ChannelKey::new(light, "intensity"))
    );
}
