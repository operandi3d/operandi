//! BEHAVIOR-RIG-PHASE4 §6 in core: Expression (RG-60), Linear Blend
//! (RG-61), Math, Clamp, Switch (RG-62) and the matrix outputs (RG-63).
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

fn shown(e: &Editor, item: u64, channel: &str) -> f64 {
    let s = e.evaluated(evaluate::Purpose::Final);
    animation::read(&s, &animation::ChannelKey::new(item, channel)).unwrap()
}

fn set(e: &mut Editor, item: u64, axis: usize, v: f64) {
    e.scene.item_mut(item).unwrap().transform.translation[axis] = v;
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
fn rg60_the_wheel_expression_turns_distance_into_spin() {
    // Milestone 2: dist / (2*pi*r) * 360, r = 0.5, in degrees; the
    // rotation channel takes radians, so the expression converts.
    let mut e = Editor::default();
    let car = item(&mut e);
    let wheel = item(&mut e);
    ok(
        &mut e,
        &format!(
            "operandi.rig.modifierAdd kind:expression output:{wheel}:rot.X inputs:dist={car}:pos.Z expr:{{dist / (2*pi*0.5) * 360 * pi / 180}}"
        ),
    );
    for (d, turns) in [
        (0., 0.),
        (std::f64::consts::PI, 1.),
        (std::f64::consts::PI / 2., 0.5),
    ] {
        set(&mut e, car, 2, d);
        let want = turns * std::f64::consts::TAU;
        assert!((shown(&e, wheel, "rot.X") - want).abs() < 1e-9, "{d}");
    }
}

#[test]
fn rg60_time_frame_errors_and_inputs() {
    let mut e = Editor::default();
    let a = item(&mut e);
    ok(
        &mut e,
        &format!(
            "operandi.rig.modifierAdd kind:expression output:{a}:pos.Y expr:{{frame * 2 + (time > 0 ? 100 : 0)}}"
        ),
    );
    ok(&mut e, "operandi.time.set frame:12");
    assert!((shown(&e, a, "pos.Y") - 124.).abs() < 1e-9);
    // A syntax error is refused when typed, with its position.
    let p = e.scene.animation.rig.pieces[0].id;
    let err = line(
        &mut e,
        &format!("operandi.rig.modifierSet piece:{p} expr:{{1 +}}"),
    )
    .unwrap_err();
    assert!(err.contains("Expression error at"), "{err}");
    // An error in a loaded file shows in the query and the output holds 0.
    e.scene
        .animation
        .rig
        .piece_mut(p)
        .unwrap()
        .names
        .insert("expr".into(), "sin(".into());
    let q = query(&e, a);
    assert!(q["pieces"][0]["error"]["message"].is_string(), "{q}");
    assert_eq!(shown(&e, a, "pos.Y"), 0.);
}

#[test]
fn rg61_rg62_blend_math_clamp_switch() {
    let mut e = Editor::default();
    let x = item(&mut e);
    let y = item(&mut e);
    let out = item(&mut e);
    set(&mut e, x, 0, 2.);
    set(&mut e, y, 0, 6.);
    ok(
        &mut e,
        &format!(
            "operandi.rig.modifierAdd kind:blend output:{out}:pos.X inputs:A={x}:pos.X,B={y}:pos.X"
        ),
    );
    let blend = e.scene.animation.rig.pieces[0].id;
    assert_eq!(shown(&e, out, "pos.X"), 2.);
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{blend} channel:blend value:0.25"),
    );
    assert_eq!(shown(&e, out, "pos.X"), 3.);
    for (op, want) in [
        ("subtract", -4.),
        ("multiply", 12.),
        ("divide", 2. / 6.),
        ("min", 2.),
        ("max", 6.),
        ("power", 64.),
        ("add", 8.),
    ] {
        if e.scene
            .animation
            .rig
            .pieces
            .iter()
            .any(|p| p.kind == "mod.math")
        {
            let m = e
                .scene
                .animation
                .rig
                .pieces
                .iter()
                .find(|p| p.kind == "mod.math")
                .unwrap()
                .id;
            ok(
                &mut e,
                &format!("operandi.rig.modifierSet piece:{m} op:{op}"),
            );
        } else {
            ok(
                &mut e,
                &format!(
                    "operandi.rig.modifierAdd kind:math output:{out}:pos.Y inputs:A={x}:pos.X,B={y}:pos.X op:{op}"
                ),
            );
        }
        assert!((shown(&e, out, "pos.Y") - want).abs() < 1e-9, "{op}");
    }
    ok(
        &mut e,
        &format!("operandi.rig.modifierAdd kind:clamp output:{out}:pos.Z inputs:A={y}:pos.X"),
    );
    let clamp = e.scene.animation.rig.pieces.last().unwrap().id;
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{clamp} channel:max value:4"),
    );
    assert_eq!(shown(&e, out, "pos.Z"), 4.);
    ok(
        &mut e,
        &format!(
            "operandi.rig.modifierAdd kind:switch output:{out}:scl.X inputs:A={x}:pos.X,B={y}:pos.X"
        ),
    );
    let sw = e.scene.animation.rig.pieces.last().unwrap().id;
    assert_eq!(shown(&e, out, "scl.X"), 2.);
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{sw} channel:index value:1"),
    );
    assert_eq!(shown(&e, out, "scl.X"), 6.);
}

#[test]
fn rg63_world_and_local_outputs_follow_an_item() {
    let mut e = Editor::default();
    let parent = item(&mut e);
    let child = item(&mut e);
    let follower = item(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.parent items:{child} parent:{parent} compensate:false"),
    );
    set(&mut e, parent, 1, 3.);
    set(&mut e, child, 1, 1.);
    let k = |n: &str| animation::ChannelKey::new(child, n);
    assert_eq!(animation::read(&e.scene, &k("world.pos.Y")), Some(4.));
    assert_eq!(animation::read(&e.scene, &k("local.pos.Y")), Some(1.));
    assert!(!animation::keyable(&e.scene, child, "world.pos.Y"));
    assert!(animation::write(&mut e.scene, &k("world.pos.Y"), 1.).is_err());
    ok(
        &mut e,
        &format!("operandi.rig.linkAdd driven:{follower}:pos.Y driver:{child}:world.pos.Y"),
    );
    assert_eq!(shown(&e, follower, "pos.Y"), 4.);
}

#[test]
fn modifiers_added_without_wires_only_exist_and_loops_are_refused() {
    let mut e = Editor::default();
    let a = item(&mut e);
    e.scene.selection.item = Some(a);
    ok(&mut e, "operandi.rig.modifierAdd kind:math");
    let m = e.scene.animation.rig.pieces[0].id;
    assert!(query(&e, a)["pieces"][0]["value"].is_null());
    // Wire it so pos.X = pos.X + 1 on the same channel: a loop.
    let err = line(
        &mut e,
        &format!("operandi.rig.modifierSet piece:{m} output:{a}:pos.X inputs:A={a}:pos.X"),
    )
    .unwrap_err();
    assert!(err.contains("depend on itself"), "{err}");
}
