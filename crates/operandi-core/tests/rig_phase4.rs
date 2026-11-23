//! BEHAVIOR-RIG-PHASE4 §1-4 in core: evaluation rules (RG-01..06),
//! parenting and pivots (RG-10..14), dynamic parents (RG-20..25) and
//! constraints (RG-26..46), replaying every measured number (1e-4) as
//! command lines (UIC-160, MCP rule).
use glam::{DMat4, DVec3};
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

fn near(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-4, "{a} vs {b}");
}

/// A mesh item at `at`.
fn item(e: &mut Editor, at: DVec3) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.translation = at;
    id
}

fn query(e: &Editor, item: u64) -> Value {
    let text = command_exec::answer(
        e,
        "operandi.rig.query",
        &serde_json::json!({ "item": item })
            .as_object()
            .unwrap()
            .clone(),
    )
    .expect("a query")
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The evaluated world position (what the viewport and renders show).
fn shown(e: &Editor, item: u64) -> DVec3 {
    let v = query(e, item)["world"]["position"].clone();
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

fn axis(e: &Editor, item: u64, a: &str) -> DVec3 {
    let v = query(e, item)["world"]["axes"][a].clone();
    DVec3::new(
        v[0].as_f64().unwrap(),
        v[1].as_f64().unwrap(),
        v[2].as_f64().unwrap(),
    )
}

fn piece_of(e: &Editor, item: u64, kind: &str) -> u64 {
    e.scene
        .animation
        .rig
        .on(item)
        .find(|p| p.kind == kind)
        .unwrap_or_else(|| panic!("no {kind} on {item}"))
        .id
}

/// Positions in the evaluated scene agree with the query (readers see the
/// rig through `evaluated`).
fn displayed(e: &Editor, item: u64) -> DVec3 {
    let s = e.evaluated(evaluate::Purpose::Final);
    s.world_matrix(item).unwrap().w_axis.truncate()
}

// ---- RG-28 / RG-29 / RG-31: position constraint ----

#[test]
fn rg28_weights_are_normalised_across_targets() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(5., 5., 5.));
    let a = item(&mut e, DVec3::new(2., 0., 0.));
    let b = item(&mut e, DVec3::new(0., 4., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a},{b}"),
    );
    let p = piece_of(&e, c, "position");
    for (wa, wb, want) in [
        (1., 1., DVec3::new(1., 2., 0.)),
        (1., 3., DVec3::new(0.5, 3., 0.)),
        (0., 3., DVec3::new(0., 4., 0.)),
        (0.5, 0., DVec3::new(2., 0., 0.)),
        // 0,0: the item's own transform.
        (0., 0., DVec3::new(5., 5., 5.)),
    ] {
        ok(
            &mut e,
            &format!("operandi.rig.set piece:{p} channel:weight.0 value:{wa}"),
        );
        ok(
            &mut e,
            &format!("operandi.rig.set piece:{p} channel:weight.1 value:{wb}"),
        );
        near(shown(&e, c), want);
        near(displayed(&e, c), want);
    }
}

#[test]
fn rg29_a_constraint_replaces_its_group_and_offset_adds_in_world() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(5., 5., 5.));
    let a = item(&mut e, DVec3::new(2., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a}"),
    );
    near(shown(&e, c), DVec3::new(2., 0., 0.));
    // Own position ignored, and kept: it is still 5,5,5 on the item.
    assert_eq!(
        e.scene.item(c).unwrap().transform.translation,
        DVec3::splat(5.)
    );
    let p = piece_of(&e, c, "position");
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:offset.pos.X value:1"),
    );
    near(shown(&e, c), DVec3::new(3., 0., 0.));
    // RG-03: Enable off = as if the piece weren't there.
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:enable value:0"),
    );
    near(shown(&e, c), DVec3::splat(5.));
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:enable value:1"),
    );
    // Removed: the own transform comes back (RG-29).
    ok(
        &mut e,
        &format!("operandi.rig.unconstrain kind:position items:{c}"),
    );
    near(shown(&e, c), DVec3::splat(5.));
}

#[test]
fn rg31_influence_and_affect_axes_blend_with_the_own_transform() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(0., 0., 0.));
    let a = item(&mut e, DVec3::new(2., 4., 6.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a}"),
    );
    let p = piece_of(&e, c, "position");
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:influence value:0.5"),
    );
    near(shown(&e, c), DVec3::new(1., 2., 3.));
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:influence value:1"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:affect.Y value:0"),
    );
    near(shown(&e, c), DVec3::new(2., 0., 6.));
}

#[test]
fn rg30_compensation_keeps_the_item_in_place_on_apply() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(1., 1., 1.));
    let a = item(&mut e, DVec3::new(3., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a} compensate:true"),
    );
    near(shown(&e, c), DVec3::new(1., 1., 1.));
    // Then it follows the target.
    e.scene.item_mut(a).unwrap().transform.translation.x = 4.;
    near(shown(&e, c), DVec3::new(2., 1., 1.));
}

// ---- RG-44: direction ----

#[test]
fn rg44_direction_hits_the_measured_axes_and_the_degenerate_case() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::ZERO);
    let t = item(&mut e, DVec3::new(1., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:direction items:{c} targets:{t}"),
    );
    near(axis(&e, c, "Z"), DVec3::new(1., 0., 0.));
    near(axis(&e, c, "Y"), DVec3::new(0., 1., 0.));
    near(axis(&e, c, "X"), DVec3::new(0., 0., -1.));
    // Target at +Z: identity.
    e.scene.item_mut(t).unwrap().transform.translation = DVec3::new(0., 0., 1.);
    near(axis(&e, c, "X"), DVec3::X);
    near(axis(&e, c, "Y"), DVec3::Y);
    near(axis(&e, c, "Z"), DVec3::Z);
    // Straight up, parallel to Up: no NaN, no flip.
    e.scene.item_mut(t).unwrap().transform.translation = DVec3::new(0., 1., 0.);
    near(axis(&e, c, "Z"), DVec3::new(0., 1., 0.));
    near(axis(&e, c, "X"), DVec3::new(0., 0., 1.));
    near(axis(&e, c, "Y"), DVec3::new(1., 0., 0.));
}

#[test]
fn rg44_up_items_and_roll() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::ZERO);
    let t = item(&mut e, DVec3::new(0., 0., 5.));
    let up = item(&mut e, DVec3::new(1., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:direction items:{c} targets:{t}"),
    );
    let p = piece_of(&e, c, "direction");
    // Up from another item's position: Y now points at +X.
    ok(
        &mut e,
        &format!("operandi.rig.upVector piece:{p} item:{up}"),
    );
    near(axis(&e, c, "Z"), DVec3::Z);
    near(axis(&e, c, "Y"), DVec3::X);
    // Remove: back to the Up Vector channel (+Y).
    ok(&mut e, &format!("operandi.rig.upVector piece:{p}"));
    near(axis(&e, c, "Y"), DVec3::Y);
    // Roll 90° about the aim axis.
    ok(
        &mut e,
        &format!(
            "operandi.rig.set piece:{p} channel:roll value:{}",
            std::f64::consts::FRAC_PI_2
        ),
    );
    near(axis(&e, c, "Z"), DVec3::Z);
    near(axis(&e, c, "X"), DVec3::Y);
}

// ---- RG-40..43, 45, 46: the other kinds ----

#[test]
fn rotation_scale_and_transform_follow_their_targets() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(1., 0., 0.));
    let a = item(&mut e, DVec3::new(0., 3., 0.));
    {
        let t = &mut e.scene.item_mut(a).unwrap().transform;
        t.rotation = DVec3::new(0., 0., std::f64::consts::FRAC_PI_2);
        t.scale = DVec3::new(2., 2., 2.);
    }
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:rotation items:{c} targets:{a}"),
    );
    near(axis(&e, c, "X"), DVec3::Y);
    near(shown(&e, c), DVec3::new(1., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.unconstrain kind:rotation items:{c}"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:scale items:{c} targets:{a}"),
    );
    let s = query(&e, c)["world"]["scale"].clone();
    assert!((s[0].as_f64().unwrap() - 2.).abs() < 1e-4);
    ok(
        &mut e,
        &format!("operandi.rig.unconstrain kind:scale items:{c}"),
    );
    // Transform with compensation: a weighted parent.
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:transform items:{c} targets:{a} compensate:true"),
    );
    near(shown(&e, c), DVec3::new(1., 0., 0.));
    e.scene.item_mut(a).unwrap().transform.translation.y = 4.;
    near(shown(&e, c), DVec3::new(1., 1., 0.));
}

#[test]
fn rg45_distance_exact_minimum_and_maximum() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(3., 0., 0.));
    let t = item(&mut e, DVec3::ZERO);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:distance items:{c} targets:{t}"),
    );
    let p = piece_of(&e, c, "distance");
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:distance value:2"),
    );
    near(shown(&e, c), DVec3::new(2., 0., 0.));
    // Minimum 2: at 3 it may stay.
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:clamp value:1"),
    );
    near(shown(&e, c), DVec3::new(3., 0., 0.));
    // Maximum 2: pulled in.
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:clamp value:2"),
    );
    near(shown(&e, c), DVec3::new(2., 0., 0.));
}

/// A straight open curve item from 0,0,0 to 10,0,0 through 2,0,0.
fn curve(e: &mut Editor) -> u64 {
    e.scene.new_mesh().unwrap();
    let id = e.scene.items.last().unwrap().id;
    let mut mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::new(2., 0., 0.), DVec3::new(10., 0., 0.)],
        faces: Shared::new(vec![Face {
            vertices: vec![0, 1, 2],
            uv: None,
            material: "Default".into(),
            smooth: true,
            part: None,
            material_label: None,
        }]),
        ..Default::default()
    };
    mesh.attributes.representations = vec![PolygonRepresentation::Polyline {
        closed: false,
        curve: curves::CurveKind::Line,
    }];
    e.scene.item_mut(id).unwrap().mesh = MeshSlot::from(mesh);
    id
}

#[test]
fn rg46_path_percent_is_arc_length_with_wrap_and_orientation() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::new(0., 5., 0.));
    let k = curve(&mut e);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:path items:{c} targets:{k}"),
    );
    let p = piece_of(&e, c, "path");
    // 50% is half the length (5 m), not the middle point (2 m).
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:percent value:0.5"),
    );
    near(shown(&e, c), DVec3::new(5., 0., 0.));
    // Axis Z along the curve.
    near(axis(&e, c, "Z"), DVec3::X);
    // Without Wrap the end holds; with it 125% is 25%.
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:percent value:1.25"),
    );
    near(shown(&e, c), DVec3::new(10., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:wrap value:1"),
    );
    near(shown(&e, c), DVec3::new(2.5, 0., 0.));
    // Offset adds to Percent.
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:pathOffset value:0.25"),
    );
    near(shown(&e, c), DVec3::new(5., 0., 0.));
    // Path Position keeps the item's own rotation.
    ok(
        &mut e,
        &format!("operandi.rig.unconstrain kind:path items:{c}"),
    );
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:pathPosition items:{c} targets:{k}"),
    );
    near(axis(&e, c, "Z"), DVec3::Z);
}

// ---- RG-20..25: dynamic parent, the measured run ----

#[test]
fn rg22_dynamic_parent_with_compensation() {
    let mut e = Editor::default();
    let child = item(&mut e, DVec3::new(1., 0., 0.));
    let parent = item(&mut e, DVec3::new(0., 0., 5.));
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentAdd item:{child} parents:{parent} compensate:true"),
    );
    near(shown(&e, child), DVec3::new(1., 0., 0.));
    let p = piece_of(&e, child, "dynamicParent");
    let piece = e.scene.animation.rig.piece(p).unwrap();
    near(piece.vec3("offset.pos."), DVec3::new(0., 0., -5.));
    assert_eq!(piece.num("parent"), 1.);
    // The parent moves +2 in Y: so does the child.
    e.scene.item_mut(parent).unwrap().transform.translation.y += 2.;
    near(shown(&e, child), DVec3::new(1., 2., 0.));
    // World (index 0) with compensation: stays, offset (0,2,0).
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{child} parent:0 compensate:true"),
    );
    near(shown(&e, child), DVec3::new(1., 2., 0.));
    near(
        e.scene.animation.rig.piece(p).unwrap().vec3("offset.pos."),
        DVec3::new(0., 2., 0.),
    );
    e.scene.item_mut(parent).unwrap().transform.translation.y += 3.;
    near(shown(&e, child), DVec3::new(1., 2., 0.));
}

#[test]
fn rg23_keyed_switches_restore_their_offsets_when_scrubbing_back() {
    let mut e = Editor::default();
    let part = item(&mut e, DVec3::new(1., 0., 0.));
    let gripper = item(&mut e, DVec3::new(0., 0., 5.));
    let conveyor = item(&mut e, DVec3::new(0., 3., 0.));
    ok(
        &mut e,
        &format!(
            "operandi.rig.dynamicParentAdd item:{part} parents:{gripper},{conveyor} compensate:true"
        ),
    );
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{part} parent:0 compensate:true key:true"),
    );
    ok(&mut e, "operandi.time.set frame:30");
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{part} parent:1 compensate:true key:true"),
    );
    // The gripper carries it away after 30.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{gripper}:pos.X frame:30 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{gripper}:pos.X frame:60 value:4"),
    );
    ok(&mut e, "operandi.time.set frame:60");
    let at60 = shown(&e, part);
    near(at60, DVec3::new(5., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentSet item:{part} parent:2 compensate:true key:true"),
    );
    near(shown(&e, part), at60);
    // Scrub back: no pops at either switch.
    for (f, want) in [
        (29., DVec3::new(1., 0., 0.)),
        (30., DVec3::new(1., 0., 0.)),
        (60., at60),
    ] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        near(shown(&e, part), want);
    }
    // RG-05: scrubbing to 45 equals playing to it.
    ok(&mut e, "operandi.time.set frame:45");
    let scrubbed = shown(&e, part);
    for f in 0..=45 {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
    }
    near(shown(&e, part), scrubbed);
}

#[test]
fn rg25_remove_dynamic_parent_keeps_the_item_with_compensation() {
    let mut e = Editor::default();
    let child = item(&mut e, DVec3::new(1., 0., 0.));
    let parent = item(&mut e, DVec3::new(0., 0., 5.));
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentAdd item:{child} parents:{parent} compensate:true"),
    );
    e.scene.item_mut(parent).unwrap().transform.translation.y = 2.;
    ok(
        &mut e,
        &format!("operandi.rig.dynamicParentRemove item:{child} compensate:true"),
    );
    near(shown(&e, child), DVec3::new(1., 2., 0.));
    near(
        e.scene.item(child).unwrap().transform.translation,
        DVec3::new(1., 2., 0.),
    );
}

// ---- RG-02 / RG-04 / RG-06: loops, driven channels, playback ----

#[test]
fn rg02_loops_are_refused_and_loaded_loops_break() {
    let mut e = Editor::default();
    let a = item(&mut e, DVec3::ZERO);
    let b = item(&mut e, DVec3::X);
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{a} targets:{b}"),
    );
    let before = e.scene.clone();
    let err = line(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{b} targets:{a}"),
    )
    .unwrap_err();
    assert!(err.contains("depend on itself"), "{err}");
    assert_eq!(e.scene, before, "nothing changed");
    // Through a parent too.
    let c = item(&mut e, DVec3::Y);
    ok(&mut e, &format!("operandi.rig.parent items:{c} parent:{a}"));
    assert!(
        line(
            &mut e,
            &format!("operandi.rig.constrain kind:position items:{b} targets:{c}")
        )
        .is_err()
    );
    // A file with a loop loads and evaluates the loop's pieces as disabled.
    let mut looped = e.scene.clone();
    looped
        .animation
        .rig
        .pieces
        .push(rig::Piece::new(999, "position", b, vec![a]));
    let bytes = persistence::encode(&looped).unwrap();
    let back = persistence::decode(&bytes).unwrap();
    let r = rig_eval::evaluate(&back);
    assert_eq!(r.broken.len(), 2, "{:?}", r.broken);
    assert!(r.world.values().all(|m| m.is_finite()));
}

#[test]
fn rg04_driven_channels_are_reported_and_hand_edits_dont_move_the_result() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::ZERO);
    let a = item(&mut e, DVec3::new(2., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a}"),
    );
    let q = query(&e, c);
    let driven: Vec<&str> = q["driven"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["channel"].as_str().unwrap())
        .collect();
    assert_eq!(driven, ["pos.X", "pos.Y", "pos.Z"]);
    assert_eq!(query(&e, a)["driver"], true);
    ok(
        &mut e,
        &format!("transform.channel name:pos.X value:9 item:{c}"),
    );
    near(shown(&e, c), DVec3::new(2., 0., 0.));
}

#[test]
fn rg06_only_items_downstream_of_a_change_are_evaluated() {
    let mut e = Editor::default();
    let mut targets = Vec::new();
    for i in 0..200 {
        let t = item(&mut e, DVec3::new(i as f64, 0., 0.));
        let c = item(&mut e, DVec3::ZERO);
        ok(
            &mut e,
            &format!("operandi.rig.constrain kind:position items:{c} targets:{t}"),
        );
        targets.push(t);
    }
    let _ = e.evaluated(evaluate::Purpose::Final);
    let first = e.eval.rig_computed();
    let _ = e.evaluated(evaluate::Purpose::Final);
    assert_eq!(
        e.eval.rig_computed(),
        first,
        "nothing changed, nothing evaluated"
    );
    e.scene
        .item_mut(targets[7])
        .unwrap()
        .transform
        .translation
        .y = 1.;
    let _ = e.evaluated(evaluate::Purpose::Final);
    assert_eq!(
        e.eval.rig_computed(),
        first + 1,
        "one target moved: one item"
    );
    // The playback speed test: 200 constrained items, a key each, 48 frames.
    for (n, t) in targets.iter().enumerate() {
        ok(
            &mut e,
            &format!("operandi.key.add channel:{t}:pos.Y frame:0 value:0"),
        );
        ok(
            &mut e,
            &format!("operandi.key.add channel:{t}:pos.Y frame:48 value:{n}"),
        );
    }
    let start = std::time::Instant::now();
    for f in 0..48 {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        let _ = e.evaluated(evaluate::Purpose::Display { base_edit: None });
    }
    let per_frame = start.elapsed().as_secs_f64() / 48.;
    // Debug build; a release frame is far quicker. 24 fps = 41.7 ms.
    assert!(per_frame < 1. / 24., "{:.2} ms per frame", per_frame * 1e3);
}

#[test]
fn rg05_and_the_memo_never_go_stale() {
    // The memoised evaluation always equals a fresh one (the plan's
    // stale-view check), through edits, undo and time changes.
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::ZERO);
    let a = item(&mut e, DVec3::new(2., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:direction items:{c} targets:{a}"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.Y frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pos.Y frame:10 value:3"),
    );
    let check = |e: &Editor| {
        assert_eq!(
            e.eval.rig(&e.scene).world,
            rig_eval::evaluate(&e.scene).world
        );
    };
    for f in [0., 5., 10., 3.] {
        ok(&mut e, &format!("operandi.time.set frame:{f}"));
        check(&e);
    }
    e.dispatch(Command::Undo).unwrap();
    check(&e);
    e.dispatch(Command::Redo).unwrap();
    check(&e);
}

// ---- RG-10..14: parenting and pivots ----

#[test]
fn rg11_12_parent_unparent_insert_extract_with_and_without_compensation() {
    let mut e = Editor::default();
    let a = item(&mut e, DVec3::new(1., 0., 0.));
    let p = item(&mut e, DVec3::new(0., 5., 0.));
    // Without compensation the local values stay and the item jumps.
    ok(
        &mut e,
        &format!("operandi.rig.parent items:{a} parent:{p} compensate:false"),
    );
    near(shown(&e, a), DVec3::new(1., 5., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.unparent items:{a} compensate:false"),
    );
    near(shown(&e, a), DVec3::new(1., 0., 0.));
    // With it the item stays.
    ok(
        &mut e,
        &format!("operandi.rig.parent items:{a} parent:{p} compensate:true"),
    );
    near(shown(&e, a), DVec3::new(1., 0., 0.));
    assert_eq!(e.scene.item(a).unwrap().parent, Some(p));
    // Insert m between p and a: m takes a's place, a goes under m.
    let m = item(&mut e, DVec3::new(0., 0., 2.));
    ok(
        &mut e,
        &format!("operandi.rig.insert item:{m} child:{a} compensate:true"),
    );
    assert_eq!(e.scene.item(m).unwrap().parent, Some(p));
    assert_eq!(e.scene.item(a).unwrap().parent, Some(m));
    near(shown(&e, a), DVec3::new(1., 0., 0.));
    // Extract m: a moves up to p.
    ok(
        &mut e,
        &format!("operandi.rig.extract item:{m} compensate:true"),
    );
    assert_eq!(e.scene.item(a).unwrap().parent, Some(p));
    assert_eq!(e.scene.item(m).unwrap().parent, None);
    near(shown(&e, a), DVec3::new(1., 0., 0.));
    // Each was one undo step.
    let steps = e.undo.len();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.undo.len(), steps - 1);
    assert_eq!(e.scene.item(m).unwrap().parent, Some(p));
}

#[test]
fn rg12_parent_from_the_selection_makes_the_last_picked_the_parent() {
    let mut e = Editor::default();
    let a = item(&mut e, DVec3::ZERO);
    let b = item(&mut e, DVec3::ZERO);
    let p = item(&mut e, DVec3::ZERO);
    e.scene.selection.mode = Mode::Item;
    e.scene.selection.also = vec![a, b];
    e.scene.selection.item = Some(p);
    ok(&mut e, "operandi.rig.parent");
    assert_eq!(e.scene.item(a).unwrap().parent, Some(p));
    assert_eq!(e.scene.item(b).unwrap().parent, Some(p));
}

#[test]
fn rg14_pivots_turn_about_themselves_and_moving_one_keeps_the_geometry() {
    let mut e = Editor::default();
    let a = item(&mut e, DVec3::ZERO);
    ok(
        &mut e,
        &format!("operandi.rig.pivot item:{a} position:1,0,0"),
    );
    // The geometry didn't move.
    near(shown(&e, a), DVec3::ZERO);
    let world = e.scene.world_matrix(a).unwrap();
    near(
        world.transform_point3(DVec3::new(1., 0., 0.)),
        DVec3::new(1., 0., 0.),
    );
    // A quarter turn about Z happens about the pivot (1,0,0): the pivot
    // point stays, the origin swings to (1,-1,0).
    e.scene.item_mut(a).unwrap().transform.rotation.z = std::f64::consts::FRAC_PI_2;
    let w = e.scene.world_matrix(a).unwrap();
    near(
        w.transform_point3(DVec3::new(1., 0., 0.)),
        DVec3::new(1., 0., 0.),
    );
    near(w.transform_point3(DVec3::ZERO), DVec3::new(1., -1., 0.));
    // Pivot channels key like any other.
    ok(
        &mut e,
        &format!("operandi.key.add channel:{a}:pivot.pos.Y frame:0 value:0"),
    );
    assert!(
        e.scene
            .animation
            .channels
            .keys()
            .any(|k| k.channel == "pivot.pos.Y")
    );
}

// ---- Files and undo ----

#[test]
fn rig_pieces_save_reopen_and_unknown_kinds_round_trip() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::ZERO);
    let a = item(&mut e, DVec3::new(2., 0., 0.));
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a}"),
    );
    let p = piece_of(&e, c, "position");
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:influence value:0.25"),
    );
    let mut future = rig::Piece::new(777, "geometry", c, vec![a]);
    future
        .extra
        .insert("surface".into(), serde_json::json!({"uv": [0.5, 0.5]}));
    e.scene.animation.rig.pieces.push(future.clone());
    let back = persistence::decode(&persistence::encode(&e.scene).unwrap()).unwrap();
    assert_eq!(back.animation.rig, e.scene.animation.rig);
    let missing = back.animation.rig.piece(777).unwrap();
    assert!(missing.is_missing());
    assert_eq!(missing, &future);
    // A missing piece evaluates as disabled.
    near(shown(&e, c), DVec3::new(0.5, 0., 0.));
}

#[test]
fn every_rig_edit_is_one_undo_step_and_piece_channels_key() {
    let mut e = Editor::default();
    let c = item(&mut e, DVec3::ZERO);
    let a = item(&mut e, DVec3::new(2., 0., 0.));
    let start = e.undo.len();
    ok(
        &mut e,
        &format!("operandi.rig.constrain kind:position items:{c} targets:{a}"),
    );
    let p = piece_of(&e, c, "position");
    ok(
        &mut e,
        &format!("operandi.rig.set piece:{p} channel:influence value:0.5"),
    );
    assert_eq!(e.undo.len(), start + 2);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.animation.rig.piece(p).unwrap().num("influence"), 1.);
    e.dispatch(Command::Undo).unwrap();
    assert!(e.scene.animation.rig.pieces.is_empty());
    e.dispatch(Command::Redo).unwrap();
    // Influence keys at 0 and 10 animate it (RG §8 Keys).
    ok(
        &mut e,
        &format!("operandi.key.add channel:{p}:influence frame:0 value:0"),
    );
    ok(
        &mut e,
        &format!("operandi.key.add channel:{p}:influence frame:10 value:1"),
    );
    ok(&mut e, "operandi.time.set frame:5");
    let x = shown(&e, c).x;
    assert!(x > 0.2 && x < 1.8, "{x}");
    // Removing the piece removes its keys.
    ok(&mut e, &format!("operandi.rig.remove piece:{p}"));
    assert!(!e.scene.animation.channels.keys().any(|k| k.item == p));
    let _ = DMat4::IDENTITY;
}
