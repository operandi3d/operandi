//! BEHAVIOR-TIMELINE phase 2 (TL-xx) in core: the track row's key edits,
//! the key clipboard, insert/remove frame, ranges, markers, and the
//! bouncing-ball acceptance run (section 10) as command lines.
use glam::DVec3;
use operandi_core::anim_curve::Slope;
use operandi_core::animation::{self, ChannelKey};
use operandi_core::*;

fn line(e: &mut Editor, text: &str) -> Result<Option<String>> {
    command_line::run_line(e, text).expect("a headless command line")
}

fn ok(e: &mut Editor, text: &str) {
    if let Err(err) = line(e, text) {
        panic!("{text}: {err}");
    }
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-4, "{a} vs {b}");
}

/// A sphere-sized mesh item, selected, Items mode; its id.
fn ball() -> (Editor, u64) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        size: DVec3::splat(1.),
        ..e.session.live.as_ref().unwrap().params.clone()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.scene.selection.mode = Mode::Item;
    let id = e.scene.active().unwrap().id;
    (e, id)
}

fn curve(e: &Editor, id: u64, ch: &str) -> anim_curve::Curve {
    e.scene.animation.channels[&ChannelKey::new(id, ch)]
        .curve
        .clone()
}

fn at(e: &Editor, id: u64, ch: &str, f: f64) -> f64 {
    curve(e, id, ch).value_at(animation::at_frames(e.scene.time.rate, f))
}

/// Section 10 steps 1-5 and 7 (render, playblast and USD are app and
/// export tests). Returns the editor and the ball's id.
pub fn bouncing_ball() -> (Editor, u64) {
    let (mut e, id) = ball();
    let undo_start = e.history.undo.len();
    // 2. Height and scale keys at 0, 12, 24, a squash at 12. Frame 0 keys
    // by hand; Auto Key Animated keys the edits at 12 and 24.
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:2 item:{id}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.set channels:{id}:pos.Y,{id}:scl.X,{id}:scl.Y,{id}:scl.Z"),
    );
    ok(&mut e, "operandi.time.set frame:12");
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:0 item:{id}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:scl.Y value:0.6 item:{id}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:scl.X value:1.2 item:{id}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:scl.Z value:1.2 item:{id}"),
    );
    ok(&mut e, "operandi.time.set frame:24");
    ok(
        &mut e,
        &format!("transform.channel name:pos.Y value:2 item:{id}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:scl.Y value:1 item:{id}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:scl.X value:1 item:{id}"),
    );
    ok(
        &mut e,
        &format!("transform.channel name:scl.Z value:1 item:{id}"),
    );
    assert_eq!(curve(&e, id, "pos.Y").keys.len(), 3);
    // 3. The f12 keys go Linear (a sharp contact).
    let f12: Vec<String> = ["pos.Y", "scl.X", "scl.Y", "scl.Z"]
        .iter()
        .map(|c| format!("{id}:{c}@12"))
        .collect();
    ok(
        &mut e,
        &format!("operandi.key.select keys:{}", f12.join(",")),
    );
    ok(&mut e, "operandi.key.slope type:linear");
    // 4. Copy f0, key time at 24, paste replace: the loop matches.
    let f0: Vec<String> = ["pos.Y", "scl.X", "scl.Y", "scl.Z"]
        .iter()
        .map(|c| format!("{id}:{c}@0"))
        .collect();
    ok(
        &mut e,
        &format!("operandi.key.select keys:{}", f0.join(",")),
    );
    ok(&mut e, "operandi.key.copy");
    ok(&mut e, "operandi.time.keyTime frame:24");
    ok(&mut e, "operandi.key.paste mode:replace");
    // 5. Loop playback over 0-23 (frame 24 is frame 0 again).
    ok(&mut e, "operandi.time.range start:0 end:23");
    ok(&mut e, "operandi.time.settings endBehavior:repeat");
    ok(&mut e, "operandi.marker.add frame:12 scene:true");
    ok(&mut e, "operandi.marker.edit label:Contact");
    let _ = undo_start;
    (e, id)
}

#[test]
fn ball_run_curves_and_loop() {
    let (e, id) = bouncing_ball();
    let y = curve(&e, id, "pos.Y");
    assert_eq!(y.keys[1].slope_in, Slope::Linear);
    assert_eq!(y.keys[1].slope_out, Slope::Linear);
    assert_eq!(y.keys[0].slope_out, Slope::SmoothFlat);
    // Heights: 2 at the top, 0 on contact, the ease in between: flat at
    // the top, Hermite into a straight-line slope of -4 per second.
    near(at(&e, id, "pos.Y", 0.), 2.);
    near(at(&e, id, "pos.Y", 6.), 1.25);
    near(at(&e, id, "pos.Y", 12.), 0.);
    near(at(&e, id, "pos.Y", 18.), 1.25);
    near(at(&e, id, "pos.Y", 24.), 2.);
    near(at(&e, id, "scl.Y", 12.), 0.6);
    near(at(&e, id, "scl.Y", 24.), 1.);
    // The loop: the curve at 24 matches 0 exactly, and playback from 23
    // goes back to 0.
    let r = e.scene.time.rate;
    let (next, stop) = animation::playback_step(&e.scene, r.tick_of(23), r.tick_of(0), 1., false);
    assert_eq!((next, stop), (r.tick_of(0), false));
    for ch in ["pos.Y", "scl.X", "scl.Y", "scl.Z"] {
        near(at(&e, id, ch, 24.), at(&e, id, ch, 0.));
    }
    assert_eq!(e.scene.animation.markers[0].label, "Contact");
}

#[test]
fn ball_run_undo_walks_back_and_scrubs_never_appear() {
    let (mut e, id) = bouncing_ball();
    let labels: Vec<String> = e.history.undo.iter().map(|c| c.label.clone()).collect();
    assert!(
        !labels
            .iter()
            .any(|l| l.contains("Time") && !l.contains("Settings")),
        "{labels:?}"
    );
    // Undo the marker label, the marker, settings, range, paste, slopes...
    let steps = e.history.undo.len();
    for _ in 0..6 {
        e.dispatch(Command::Undo).unwrap();
    }
    assert_eq!(e.history.undo.len(), steps - 6);
    // The f12 keys are Smooth Flat again (the slope step was undone).
    assert_eq!(curve(&e, id, "pos.Y").keys[1].slope_in, Slope::SmoothFlat);
}

#[test]
fn ball_run_through_the_query_reads_values_at_frames() {
    let (mut e, id) = bouncing_ball();
    let text = line(
        &mut e,
        &format!("operandi.anim.query channels:{id}:pos.Y frames:0,6,12,24"),
    )
    .unwrap()
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let ch = &v["channels"][0];
    assert_eq!(ch["channel"], format!("{id}:pos.Y"));
    near(ch["values"]["6"].as_f64().unwrap(), 1.25);
    near(ch["values"]["24"].as_f64().unwrap(), 2.);
    assert_eq!(ch["keys"][1]["in"], "linear");
    assert_eq!(v["rangeEnd"], 23.);
    assert_eq!(v["markers"][0]["label"], "Contact");
}

#[test]
fn tl42_43_move_and_scale_keys_by_whole_frames() {
    let (mut e, id) = ball();
    for f in [0, 10, 20] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    }
    ok(
        &mut e,
        &format!("operandi.key.select keys:{id}:pos.X@10,{id}:pos.X@20"),
    );
    ok(&mut e, "operandi.key.move frames:2");
    let times: Vec<f64> = curve(&e, id, "pos.X")
        .keys
        .iter()
        .map(|k| animation::frames(e.scene.time.rate, k.time))
        .collect();
    assert_eq!(times, [0., 12., 22.]);
    assert!(line(&mut e, &format!("operandi.key.select keys:{id}:pos.X@12")).is_ok());
    assert!(
        line(&mut e, "operandi.key.scale factor:2").is_err(),
        "needs two keys"
    );
    ok(
        &mut e,
        &format!("operandi.key.select keys:{id}:pos.X@12,{id}:pos.X@22"),
    );
    ok(&mut e, "operandi.key.scale factor:1.5 frame:12");
    let times: Vec<f64> = curve(&e, id, "pos.X")
        .keys
        .iter()
        .map(|k| animation::frames(e.scene.time.rate, k.time))
        .collect();
    assert_eq!(times, [0., 12., 27.]);
    // Moving onto another key replaces it.
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.X@27"));
    ok(&mut e, "operandi.key.move frames:-15");
    assert_eq!(curve(&e, id, "pos.X").keys.len(), 2);
    ok(&mut e, "operandi.key.invert");
}

#[test]
fn tl45_delete_related_and_paste_insert() {
    let (mut e, id) = ball();
    ok(&mut e, "operandi.key.transforms type:position");
    ok(&mut e, "operandi.time.set frame:10");
    ok(&mut e, "operandi.key.transforms type:position");
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@10"));
    ok(&mut e, "operandi.key.delete related:true");
    for ch in ["pos.X", "pos.Y", "pos.Z"] {
        assert_eq!(curve(&e, id, ch).keys.len(), 1, "{ch}");
    }
    ok(&mut e, "operandi.time.set frame:5");
    ok(&mut e, "operandi.key.transforms type:position");
    ok(&mut e, "operandi.time.set frame:20");
    ok(&mut e, "operandi.key.transforms type:position");
    // Copy the f5 key, paste-insert at 5: later keys move by one frame.
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.X@5"));
    ok(&mut e, "operandi.key.copy");
    ok(&mut e, "operandi.time.set frame:5");
    ok(&mut e, "operandi.key.paste mode:insert");
    let times: Vec<i64> = curve(&e, id, "pos.X")
        .keys
        .iter()
        .map(|k| e.scene.time.rate.frame_at(k.time))
        .collect();
    assert_eq!(times, [0, 5, 6, 21]);
}

#[test]
fn tl47_keys_paste_into_another_document() {
    let (mut a, id) = ball();
    ok(&mut a, "operandi.key.transforms type:position");
    ok(
        &mut a,
        &format!("operandi.key.select keys:{id}:pos.X@0,{id}:pos.Y@0"),
    );
    ok(&mut a, "operandi.key.copy");
    let (mut b, other) = ball();
    let steps = b.history.undo.len();
    ok(&mut b, "operandi.time.set frame:7");
    ok(&mut b, "operandi.key.paste");
    assert_eq!(
        b.history.undo.len(),
        steps + 1,
        "one step, in this document only"
    );
    assert!(
        b.scene
            .animation
            .is_animated(&ChannelKey::new(other, "pos.Y"))
    );
    assert_eq!(
        b.scene
            .time
            .rate
            .frame_at(curve(&b, other, "pos.Y").keys[0].time),
        7
    );
}

#[test]
fn tl21_insert_and_remove_frame_never_delete_keys() {
    let (mut e, id) = ball();
    for f in [0, 1, 5] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    }
    ok(&mut e, "operandi.time.set frame:0");
    ok(&mut e, "operandi.time.insertFrame");
    let frames = |e: &Editor| -> Vec<i64> {
        curve(e, id, "pos.X")
            .keys
            .iter()
            .map(|k| e.scene.time.rate.frame_at(k.time))
            .collect()
    };
    assert_eq!(frames(&e), [0, 2, 6]);
    ok(&mut e, "operandi.time.removeFrame");
    ok(&mut e, "operandi.time.removeFrame");
    assert_eq!(frames(&e), [0, 1, 5], "stops before landing on frame 0");
}

#[test]
fn tl13_14_ranges_and_fit() {
    let (mut e, id) = ball();
    ok(&mut e, "operandi.time.set frame:30");
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut e, "operandi.time.set frame:150");
    ok(&mut e, &format!("operandi.key.set channels:{id}:pos.X"));
    ok(&mut e, "operandi.time.fit");
    let r = e.scene.time.rate;
    let (a, b) = animation::current_range(&e.scene);
    assert_eq!((r.frame_at(a), r.frame_at(b)), (30, 150));
    assert_eq!(r.frame_at(e.scene.time.end), 150, "the scene range grows");
    ok(&mut e, "operandi.time.range reset:true");
    assert_eq!(
        animation::current_range(&e.scene),
        (e.scene.time.start, e.scene.time.end)
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(animation::current_range(&e.scene).0, r.tick_of(30));
}

#[test]
fn tl50_53_markers_belong_to_items_or_the_scene() {
    let (mut e, id) = ball();
    ok(&mut e, "operandi.marker.add frame:4");
    assert_eq!(e.scene.animation.markers[0].owner, Some(id));
    ok(&mut e, "operandi.marker.add frame:8 scene:true");
    assert_eq!(e.scene.animation.markers[1].owner, None);
    ok(
        &mut e,
        "operandi.marker.edit length:6 shape:diamond color:1,0,0 showLabel:true",
    );
    let m = &e.scene.animation.markers[1];
    assert!(m.is_range());
    assert_eq!(m.shape, animation::MarkerShape::Diamond);
    e.dispatch(Command::ClearSelection).unwrap();
    assert_eq!(
        e.visible_markers().len(),
        1,
        "item markers show only while selected"
    );
    ok(&mut e, "operandi.marker.select ids:1,2");
    ok(&mut e, "operandi.marker.delete");
    assert!(e.scene.animation.markers.is_empty());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.animation.markers.len(), 2);
}
