//! BEHAVIOR-GRAPH-EDITOR (GE-xx) in core: the edits the graph editor runs,
//! as command lines (UIC-164, MCP rule), on the bouncing ball.
use glam::DVec3;
use operandi_core::anim_curve::{Behavior, Slope};
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

/// The bouncing ball: Y 2 / 0 / 2 at 0 / 12 / 24, linear contact.
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
    let y = format!("{id}:pos.Y");
    ok(
        &mut e,
        &format!("operandi.key.add channel:{y} frame:0 value:2"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{y} frame:12 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{y} frame:24 value:2"),
    );
    ok(&mut e, &format!("operandi.key.select keys:{y}@12"));
    ok(&mut e, "operandi.key.slope type:linear");
    (e, id)
}

fn y_at(e: &Editor, id: u64, f: f64) -> f64 {
    e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")]
        .curve
        .value_at(animation::at_frames(e.scene.time.rate, f))
}

#[test]
fn ge_check2_moving_the_top_key_up_is_one_step() {
    let (mut e, id) = ball();
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@0"));
    let steps = e.history.undo.len();
    ok(&mut e, "operandi.key.setValue value:1 mode:add");
    assert_eq!(e.history.undo.len(), steps + 1);
    near(y_at(&e, id, 0.), 3.);
    assert!(y_at(&e, id, 4.) > 2.);
    e.dispatch(Command::Undo).unwrap();
    near(y_at(&e, id, 0.), 2.);
    ok(&mut e, "operandi.key.setFrame frame:2");
    near(y_at(&e, id, 2.), 2.);
}

#[test]
fn ge_check3_broken_handles_change_the_bounce() {
    let (mut e, id) = ball();
    let before = y_at(&e, id, 15.);
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@12"));
    ok(&mut e, "operandi.key.break");
    ok(&mut e, "operandi.key.handle side:out slope:0.4 weight:1.6");
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    let k = &c.keys[1];
    assert!(k.broken);
    assert_eq!(k.slope_out, Slope::Manual);
    assert_eq!(k.slope_in, Slope::Linear, "the in side keeps its own");
    near(c.slope(1, false) / 24., 0.4);
    assert!((y_at(&e, id, 15.) - before).abs() > 0.01);
    // Unify lines the out side back up with the in side.
    ok(&mut e, "operandi.key.unify");
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    near(c.slope(1, false), c.slope(1, true));
}

#[test]
fn ge_check4_post_repeat_bounces_forever() {
    let (mut e, id) = ball();
    ok(
        &mut e,
        &format!("operandi.curve.behavior channels:{id}:pos.Y post:repeat pre:oscillate"),
    );
    for f in [6., 12., 18.] {
        near(y_at(&e, id, f + 24.), y_at(&e, id, f));
        near(y_at(&e, id, f + 240.), y_at(&e, id, f));
    }
    near(y_at(&e, id, -6.), y_at(&e, id, 6.));
    ok(
        &mut e,
        &format!("operandi.curve.behavior channels:{id}:pos.Y post:offsetRepeat"),
    );
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@24"));
    ok(&mut e, "operandi.key.setValue value:3");
    near(y_at(&e, id, 36.), y_at(&e, id, 12.) + 1.);
    ok(
        &mut e,
        &format!("operandi.curve.behavior channels:{id}:pos.Y post:linear"),
    );
    ok(
        &mut e,
        &format!("operandi.curve.behavior channels:{id}:pos.Y post:reset"),
    );
    near(y_at(&e, id, 30.), 0.);
    // Saved as version 61, read back the same.
    let bytes = persistence::encode(&e.scene).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("\"version\": 61"));
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(
        back.animation.channels[&ChannelKey::new(id, "pos.Y")]
            .curve
            .post,
        Behavior::Reset
    );
}

#[test]
fn ge_check5_cycle_matches_the_last_key_to_the_first() {
    let (mut e, id) = ball();
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@24"));
    ok(&mut e, "operandi.key.setValue value:2.5");
    near(y_at(&e, id, 24.), 2.5);
    ok(&mut e, &format!("operandi.curve.cycle channels:{id}:pos.Y"));
    near(y_at(&e, id, 24.), 2.);
    // The key keeps its own value (the ghost).
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    near(c.keys[2].value, 2.5);
}

#[test]
fn ge_check6_shadow_copy_swap_swap_is_back() {
    let (mut e, id) = ball();
    let k = ChannelKey::new(id, "pos.Y");
    let original = e.scene.animation.channels[&k].curve.clone();
    ok(
        &mut e,
        &format!("operandi.curve.shadow channels:{id}:pos.Y action:copy"),
    );
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@0"));
    ok(&mut e, "operandi.key.setValue value:5");
    let edited = e.scene.animation.channels[&k].curve.clone();
    ok(
        &mut e,
        &format!("operandi.curve.shadow channels:{id}:pos.Y action:swap"),
    );
    assert_eq!(e.scene.animation.channels[&k].curve, original);
    ok(
        &mut e,
        &format!("operandi.curve.shadow channels:{id}:pos.Y action:swap"),
    );
    assert_eq!(e.scene.animation.channels[&k].curve, edited);
    ok(
        &mut e,
        &format!("operandi.curve.shadow channels:{id}:pos.Y action:clear"),
    );
    assert!(e.scene.animation.shadows.is_empty());
}

#[test]
fn ge32_key_keeping_shape_leaves_the_curve_alone() {
    let (mut e, id) = ball();
    let before: Vec<f64> = (0..=96).map(|i| y_at(&e, id, i as f64 / 4.)).collect();
    ok(
        &mut e,
        &format!("operandi.key.keepShape channels:{id}:pos.Y frame:5"),
    );
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    assert_eq!(c.keys.len(), 4);
    for (i, b) in before.iter().enumerate() {
        near(y_at(&e, id, i as f64 / 4.), *b);
    }
}

#[test]
fn ge50_weights_bend_the_segment_and_plain_weights_change_nothing() {
    let (mut e, id) = ball();
    let before = y_at(&e, id, 6.);
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@0"));
    ok(&mut e, "operandi.key.handle side:out weight:1");
    near(y_at(&e, id, 6.), before);
    ok(&mut e, "operandi.key.handle side:out weight:2.5");
    assert!(
        y_at(&e, id, 6.) > before,
        "a longer flat handle holds the top longer"
    );
    ok(&mut e, "operandi.key.autoHandles");
    near(y_at(&e, id, 6.), before);
}

#[test]
fn ge60_interpolation_and_gex1_euler_filter() {
    let (mut e, id) = ball();
    ok(
        &mut e,
        &format!("operandi.curve.interpolation channels:{id}:pos.Y type:linear"),
    );
    near(y_at(&e, id, 6.), 1.);
    ok(
        &mut e,
        &format!("operandi.curve.interpolation channels:{id}:pos.Y type:stepped"),
    );
    near(y_at(&e, id, 11.), 2.);
    // A rotation that jumps a whole turn is unwrapped.
    let r = format!("{id}:rot.Z");
    ok(
        &mut e,
        &format!("operandi.key.add channel:{r} frame:0 value:3.0"),
    );
    ok(
        &mut e,
        &format!(
            "operandi.key.add channel:{r} frame:1 value:{}",
            3.1 - std::f64::consts::TAU
        ),
    );
    ok(&mut e, &format!("operandi.curve.eulerFilter channels:{r}"));
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "rot.Z")].curve;
    near(c.keys[1].value, 3.1);
    assert!(
        line(
            &mut e,
            &format!("operandi.curve.eulerFilter channels:{id}:pos.Y")
        )
        .is_err()
    );
}

#[test]
fn gex2_simplify_drops_keys_that_change_nothing() {
    let (mut e, id) = ball();
    ok(
        &mut e,
        &format!("operandi.key.keepShape channels:{id}:pos.Y frame:5"),
    );
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@5"));
    ok(&mut e, "operandi.key.autoHandles slopes:false");
    ok(
        &mut e,
        &format!("operandi.curve.simplify channels:{id}:pos.Y tolerance:0.01"),
    );
    let n = e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")]
        .curve
        .keys
        .len();
    assert!(n <= 4);
    // The query reports handles, behaviours and the shadow flag.
    let text = line(&mut e, &format!("operandi.anim.query channels:{id}:pos.Y"))
        .unwrap()
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let ch = &v["channels"][0];
    assert_eq!(ch["post"], "constant");
    assert!(ch["keys"][0]["outSlope"].is_number());
}

#[test]
fn ge42_linked_moves_take_the_related_keys_along_in_time() {
    let (mut e, id) = ball();
    ok(
        &mut e,
        &format!("operandi.key.add channel:{id}:pos.X frame:12 value:1"),
    );
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@12"));
    ok(&mut e, "operandi.key.offset frames:2 value:0.5 link:true");
    let rate = e.scene.time.rate;
    let x = &e.scene.animation.channels[&ChannelKey::new(id, "pos.X")].curve;
    assert_eq!(rate.frame_at(x.keys[0].time), 14, "X moved in time");
    near(x.keys[0].value, 1.);
    let y = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    near(y.keys[1].value, 0.5);
}

#[test]
fn ge44_transform_scales_time_and_value_in_one_step() {
    let (mut e, id) = ball();
    ok(
        &mut e,
        &format!("operandi.key.select keys:{id}:pos.Y@0,{id}:pos.Y@12,{id}:pos.Y@24"),
    );
    let steps = e.history.undo.len();
    ok(
        &mut e,
        "operandi.key.transform timeFactor:2 timePivot:0 valueFactor:0.5 valuePivot:0",
    );
    assert_eq!(e.history.undo.len(), steps + 1);
    let rate = e.scene.time.rate;
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    let frames: Vec<i64> = c.keys.iter().map(|k| rate.frame_at(k.time)).collect();
    assert_eq!(frames, [0, 24, 48]);
    near(c.keys[0].value, 1.);
}

#[test]
fn ge31_drop_copies_leaves_keys_along_the_drag() {
    let (mut e, id) = ball();
    ok(&mut e, &format!("operandi.key.select keys:{id}:pos.Y@0"));
    ok(&mut e, "operandi.key.dropCopies frames:3 value:0.3");
    let rate = e.scene.time.rate;
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    let frames: Vec<i64> = c.keys.iter().map(|k| rate.frame_at(k.time)).collect();
    assert_eq!(frames, [0, 1, 2, 3, 12, 24]);
    near(c.keys[3].value, 2.3);
    near(c.keys[1].value, 2.1);
}

#[test]
fn gex5_retime_moves_a_frame_and_stretches_around_it() {
    let (mut e, id) = ball();
    ok(
        &mut e,
        &format!("operandi.curve.retime channels:{id}:pos.Y low:0 high:24 from:12 to:8"),
    );
    let rate = e.scene.time.rate;
    let c = &e.scene.animation.channels[&ChannelKey::new(id, "pos.Y")].curve;
    let frames: Vec<i64> = c.keys.iter().map(|k| rate.frame_at(k.time)).collect();
    assert_eq!(frames, [0, 8, 24]);
    assert!(line(&mut e, "operandi.curve.retime low:0 high:24 from:30 to:8").is_err());
}

#[test]
fn ge83_envelopes_save_and_apply() {
    let (mut e, id) = ball();
    let dir = std::env::temp_dir().join(format!("operandi-envelope-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bounce.openvelope");
    let p = path.display();
    ok(
        &mut e,
        &format!("operandi.curve.saveEnvelope channels:{id}:pos.Y path:{{{p}}}"),
    );
    ok(
        &mut e,
        &format!("operandi.curve.applyEnvelope channels:{id}:pos.X path:{{{p}}} frame:30"),
    );
    let x = &e.scene.animation.channels[&ChannelKey::new(id, "pos.X")].curve;
    assert_eq!(x.keys.len(), 3);
    assert_eq!(e.scene.time.rate.frame_at(x.keys[0].time), 30);
    near(x.keys[1].value, 0.);
    assert!(
        line(
            &mut e,
            &format!(
                "operandi.curve.applyEnvelope channels:{id}:pos.X path:{{{}}}",
                dir.join("none").display()
            )
        )
        .is_err()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
