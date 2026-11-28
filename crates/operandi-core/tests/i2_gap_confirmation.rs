//! Exact reviewed gap literals; no candidate output is used as an oracle.
use glam::DVec3;
use operandi_core::*;
use serde_json::{Value, json};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(p: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join("docs/i2-gap-plan").join(p)).unwrap())
        .unwrap()
}
fn fixed(name: &str) -> Value {
    read(&format!("states/{name}.json"))
}
fn exact(a: &Value, b: &Value) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => assert_eq!(a.as_f64(), b.as_f64()),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                exact(a, b)
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "keys {a:?} {b:?}");
            for (k, v) in a {
                exact(v, &b[k])
            }
        }
        _ => assert_eq!(a, b),
    }
}
fn parameters(p: &Parameters) -> Value {
    assert!(
        p.radial.is_none(),
        "Frozen non-radial profiles require absent radial parameters"
    );
    json!({"bevel":p.bevel,"smooth_shift":{"offset":p.smooth_shift.offset,"scale":p.smooth_shift.scale,"angle":p.smooth_shift.angle,"thicken":p.smooth_shift.thicken,"sharp":p.smooth_shift.sharp,"angle_weight":p.smooth_shift.angle_weight,"both_sides":p.smooth_shift.both_sides},"position":p.position,"size":p.size,"inset":p.inset,"shift":p.shift,"extent":p.extent,"group_polygons":p.group_polygons,"extrude_sides":p.extrude_sides,"extrude_mode":format!("{:?}",p.extrude_mode),"extrude_uvs":format!("{:?}",p.extrude_uvs),"extrude_uv_map":p.extrude_uv_map,"extrude_uv_selected":p.extrude_uv_selected,"delta":p.delta,"rotation_delta":p.rotation_delta,"scale_delta":p.scale_delta,"box_handle_mode":format!("{:?}",p.box_handle_mode),"show_corners":p.show_corners,"segments":p.segments,"radial":Value::Null})
}
fn identities(s: &Scene, i: &IdentityScene, v: &Value, epoch: u64) {
    assert_eq!(i.meshes.len(), 1);
    let ids = &i.meshes[&1];
    assert_eq!(ids.scope.process, std::process::id());
    assert_eq!(ids.scope.epoch, epoch);
    assert_eq!(ids.scope.item, 1);
    let generation = if v["vertices"][0].as_u64() == Some(83) {
        2
    } else {
        1
    };
    assert_eq!(ids.scope.generation, generation);
    let actual = json!({"vertices":ids.vertices,"faces":ids.faces,"corners":ids.corners,"logical_faces":ids.logical_faces,"edges":ids.edges.iter().map(|((a,b),id)|json!({"vertices":[a,b],"id":id})).collect::<Vec<_>>()});
    let mut expected = v.clone();
    expected.as_object_mut().unwrap().remove("next_id");
    exact(&actual, &expected);
    assert_eq!(
        ids.layout,
        s.items[0]
            .mesh
            .faces
            .iter()
            .map(|f| f.vertices.clone())
            .collect::<Vec<_>>()
    );
    i.validate(s).unwrap();
}
fn image(s: &Scene, i: &IdentityScene, v: &Value, epoch: u64) {
    assert_eq!(
        *s,
        serde_json::from_value::<Scene>(v["scene"].clone()).unwrap()
    );
    identities(s, i, &v["identities"], epoch);
}
fn roles(p: &EditCorrespondence, v: &Value) {
    exact(&json!(p.source_layout), &v["source_layout"]);
    exact(&json!(p.source_vertex_count), &v["source_vertex_count"]);
    exact(&json!(p.vertex_sources), &v["vertex_sources"]);
    exact(&json!(p.face_sources), &v["face_sources"]);
    assert_eq!(p.walls.len(), v["walls"].as_array().unwrap().len());
    for (w, v) in p.walls.iter().zip(v["walls"].as_array().unwrap()) {
        assert_eq!(w.face as u64, v["output_face_dense"].as_u64().unwrap());
        assert_eq!(
            w.selected as u64,
            v["selected_source_dense"].as_u64().unwrap()
        );
        assert_eq!(w.owner as u64, v["owner_dense"].as_u64().unwrap());
        exact(&json!([w.a, w.b]), &v["source_endpoints_dense"]);
        exact(&json!(w.row), &v["row"]);
        exact(&json!(w.vertices), &v["oriented_output_vertices"]);
    }
}
fn frame(a: Option<ActionFrame>, v: &Value) {
    let a = a.unwrap();
    exact(&json!(a.center), &v["center"]);
    exact(&json!(a.basis.to_cols_array_2d()), &v["basis"]);
}
fn checkpoint(c: &Checkpoint, v: &Value, epoch: u64) {
    assert_eq!(c.label, v["label"].as_str().unwrap());
    image(
        &c.before,
        c.identity_before.as_ref().unwrap(),
        &v["before"],
        epoch,
    );
    image(
        &c.after,
        c.identity_after.as_ref().unwrap(),
        &v["after"],
        epoch,
    );
}
fn check(e: &Editor, name: &str) {
    let v = fixed(name);
    let epoch = e.identities.epoch;
    image(&e.scene, &e.identities.current, &v["visible"], epoch);
    e.scene.validate().unwrap();
    assert_eq!(
        e.history.undo.len() as u64,
        v["document"]["cursor"].as_u64().unwrap()
    );
    let entries = v["document"]["entries"].as_array().unwrap();
    assert_eq!(e.history.undo.len() + e.history.redo.len(), entries.len());
    for (a, b) in e
        .history
        .undo
        .iter()
        .chain(e.history.redo.iter().rev())
        .zip(entries)
    {
        checkpoint(a, b, epoch)
    }
    assert!(e.i2.prior.is_none());
    if v["selected_stage"].is_null() {
        assert!(e.i2.selected.is_none());
    } else {
        let a = e.i2.selected.as_ref().unwrap();
        let b = &v["selected_stage"];
        assert_eq!(a.key, b["key"].as_u64().unwrap());
        assert_eq!(a.anchor as u64, b["anchor"].as_u64().unwrap());
        assert_eq!(a.cursor as u64, b["cursor"].as_u64().unwrap());
        assert_eq!(a.ever_parked, b["ever_parked"].as_bool().unwrap());
        image(&a.base.scene, &a.base.identities, &b["base"], epoch);
        image(&a.begun.scene, &a.begun.identities, &b["begun"], epoch);
        if b["movement"].is_null() {
            assert!(a.movement.is_none());
            assert!(a.movement_params.is_none());
        } else {
            let m = a.movement.as_ref().unwrap();
            image(&m.scene, &m.identities, &b["movement"], epoch);
            exact(
                &parameters(a.movement_params.as_ref().unwrap()),
                &b["movement_params"],
            );
        }
        exact(&parameters(&a.armed_params), &b["armed_params"]);
        frame(a.frame, &b["frame"]);
        roles(&a.plan, &b["correspondence"]);
    }
    if v["live"].is_null() {
        assert!(e.session.live.is_none())
    } else {
        let a = e.session.live.as_ref().unwrap();
        let b = &v["live"];
        assert_eq!(a.tool.label(), b["tool"].as_str().unwrap());
        assert_eq!(a.applied, b["applied"].as_bool().unwrap());
        assert!(a.preview_error.is_none());
        image(
            &a.base,
            a.identity_base.as_ref().unwrap(),
            &b["base"],
            epoch,
        );
        exact(&parameters(&a.params), &b["params"]);
        frame(a.action_frame, &b["action_frame"]);
        if b["identity_preview"].is_null() {
            assert!(a.identity_preview.is_none())
        } else {
            let (p, ids) = a.identity_preview.as_ref().unwrap();
            roles(p, &b["identity_preview"]["roles"]);
            let mut side = IdentityScene::default();
            side.meshes.insert(1, ids.clone().into());
            let begun = &e.i2.selected.as_ref().unwrap().begun.scene;
            identities(
                begun,
                &side,
                &b["identity_preview"]["begun_identities"],
                epoch,
            );
        }
    }
    exact(
        &parameters(e.presets.get::<operandi_core::ExtrudePreset>()),
        &v["extrude_preset"],
    );
    assert_eq!(
        format!("{:?}", e.action_context.center),
        v["action_context"]["center"].as_str().unwrap()
    );
    assert_eq!(
        format!("{:?}", e.action_context.axis),
        v["action_context"]["axis"].as_str().unwrap()
    );
    assert_eq!(
        e.identities.next_id().unwrap(),
        v["geometry_bank_next"].as_u64().unwrap()
    );
    assert_eq!(
        e.i2.serial.next().unwrap(),
        v["stage_bank_next"].as_u64().unwrap()
    );
    assert!(
        e.session.cone.is_none()
            && e.session.spike.is_none()
            && e.session.ellipsoid.is_none()
            && e.selection_start.is_none()
    );
    let selected = e
        .scene
        .selection
        .faces
        .iter()
        .map(|&f| e.identities.current.meshes[&1].faces[f as usize].0)
        .collect::<Vec<_>>();
    exact(&json!(selected), &v["ordered_selected_face_ids"]);
    let roots = 1
        + 2 * (e.history.undo.len() + e.history.redo.len())
        + usize::from(e.session.live.is_some())
        + e.i2
            .selected
            .as_ref()
            .map_or(0, |s| 2 + usize::from(s.movement.is_some()));
    assert_eq!(
        roots as u64,
        v["logical_roots"]["total_pairs"].as_u64().unwrap()
    );
    println!(
        "FIXED_STATE_PASS {name} roots={roots} pub={} req={}",
        e.identities.publication, e.identities.request
    );
}
fn make() -> Editor {
    let mut e = Editor {
        scene: serde_json::from_value(read("graphs/G0.json")).unwrap(),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}
fn p(z: f64) -> Parameters {
    Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        extent: DVec3::Z * z,
        ..Default::default()
    }
}
fn dispatch(e: &mut Editor, c: Command, after: &str, dp: u64, dr: u64) {
    let (a, b) = (e.identities.publication, e.identities.request);
    e.dispatch(c).unwrap();
    assert_eq!(
        (e.identities.publication - a, e.identities.request - b),
        (dp, dr)
    );
    check(e, after)
}
fn arm(e: &mut Editor) {
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(p(0.))).unwrap();
}
fn moved() -> Editor {
    let mut e = make();
    arm(&mut e);
    e.dispatch(Command::BeginExtrude).unwrap();
    e.dispatch(Command::ConfigureExtrude(p(0.5))).unwrap();
    check(&e, "moved");
    e
}
fn parked() -> Editor {
    let mut e = moved();
    dispatch(&mut e, Command::Undo, "localS1", 1, 1);
    dispatch(&mut e, Command::Undo, "park", 1, 1);
    e
}
#[test]
fn fixed_clean_polygon_origin_prefix_and_separate_redos() {
    let mut e = make();
    check(&e, "freshG0");
    dispatch(
        &mut e,
        Command::Activate(Tool::Extrude),
        "armedDefault",
        1,
        1,
    );
    dispatch(&mut e, Command::ConfigureExtrude(p(0.)), "armed", 1, 1);
    dispatch(&mut e, Command::Drop, "N1-origin", 1, 1);
    for (set, state) in [
        ("bottom", "N1-bottom"),
        ("front", "N1-front"),
        ("top", "N1-top"),
    ] {
        dispatch(&mut e, Command::SelectPolygonSet(set.into()), state, 2, 2)
    }
    dispatch(&mut e, Command::Activate(Tool::Extrude), "N1-armed", 1, 1);
    dispatch(&mut e, Command::BeginExtrude, "N1-initialS1", 1, 1);
    dispatch(&mut e, Command::ConfigureExtrude(p(0.5)), "N1-moved", 1, 1);
    dispatch(&mut e, Command::Undo, "N1-localS1", 1, 1);
    dispatch(&mut e, Command::Undo, "N1-park-c3", 1, 1);
    for s in ["N1-park-c2", "N1-park-c1", "N1-park-c0"] {
        dispatch(&mut e, Command::Undo, s, 1, 1)
    }
    for s in [
        "N1-park-c1",
        "N1-park-c2",
        "N1-park-c3",
        "N1-resumeS1",
        "N1-resumeS2",
    ] {
        dispatch(&mut e, Command::Redo, s, 1, 1)
    }
}
#[test]
fn fixed_drop_continue_at_begin_remove_movement_redo() {
    let mut e = moved();
    dispatch(&mut e, Command::Undo, "localS1", 1, 1);
    dispatch(&mut e, Command::Drop, "dropS1", 1, 1);
    dispatch(&mut e, Command::Undo, "dropS1-undo", 1, 1);
    dispatch(&mut e, Command::Redo, "dropS1-redo", 1, 1);
    let mut e = moved();
    dispatch(&mut e, Command::Undo, "localS1", 1, 1);
    dispatch(&mut e, Command::Continue, "continueS1", 1, 1);
    dispatch(&mut e, Command::BeginExtrude, "continue-nextBegin", 1, 1);
}
#[test]
fn fixed_successful_active_and_parked_save_bytes() {
    let d = root()
        .join("development/i2-review-fixes/test-io/gap")
        .join(std::process::id().to_string());
    assert!(!d.exists(), "Never overwrite an earlier task-local run");
    std::fs::create_dir_all(&d).unwrap();
    for (label, mut e, v0, want, state) in [
        ("active", moved(), "G0", "G2", "activeSave"),
        ("parked", parked(), "G2", "G0", "parkedSave"),
    ] {
        let path = d.join(format!("{label}.opscene"));
        assert!(!path.exists());
        std::fs::copy(
            root().join(format!("docs/i2-gap-plan/wire/{v0}.expected.opscene")),
            &path,
        )
        .unwrap();
        let (p, q) = (e.identities.publication, e.identities.request);
        e.save_committed(&path).unwrap();
        assert_eq!(
            (e.identities.publication - p, e.identities.request - q),
            (1, 1)
        );
        check(&e, state);
        let expected =
            std::fs::read(root().join(format!("docs/i2-gap-plan/wire/{want}.expected.opscene")))
                .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), expected);
        let decoded = persistence::load(&path).unwrap();
        assert_eq!(decoded, e.scene);
        let mut reopened = Editor {
            scene: decoded,
            ..Default::default()
        };
        reopened.synchronize_identities().unwrap();
        check(
            &reopened,
            if label == "active" {
                "reopenG2"
            } else {
                "reopenG0"
            },
        );
        assert_ne!(reopened.identities.epoch, e.identities.epoch);
        if label == "active" {
            e.save_committed(&path).unwrap();
            check(&e, state);
            assert_eq!(std::fs::read(&path).unwrap(), expected);
        } else {
            dispatch(&mut e, Command::Redo, "parkedSave-redoBegin", 1, 1);
            dispatch(&mut e, Command::Redo, "parkedSave-redoMove", 1, 1);
            assert_eq!(std::fs::read(&path).unwrap(), expected);
        }
    }
}
#[test]
fn fixed_other_tool_applied_move_redo_guard_and_legacy_ids() {
    for finish in ["cancel", "drop", "undo"] {
        let mut e = parked();
        let old = e.identity_ticket().unwrap();
        dispatch(
            &mut e,
            Command::Activate(Tool::Move),
            "otherMoveArmed",
            1,
            1,
        );
        let mut bad = e.session.live.as_ref().unwrap().params.clone();
        bad.delta.x = f64::NAN;
        let before = e.clone();
        assert!(e.dispatch(Command::Preview(bad)).is_err());
        assert!(e == before);
        drop(before);
        check(&e, "otherMoveArmed");
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.delta = DVec3::X * 0.25;
        dispatch(&mut e, Command::Preview(p), "otherMoveApplied", 2, 2);
        // Undo is available with Move live (Jason, 2026-11-03); Redo over
        // its unrecorded change is not.
        let before = e.clone();
        assert!(e.dispatch(Command::Redo).is_err());
        assert!(e == before);
        drop(before);
        check(&e, "otherMoveApplied");
        let before = e.clone();
        assert!(e.dispatch_with_ticket(old, Command::Redo).is_err());
        assert!(e == before);
        drop(before);
        check(&e, "otherMoveApplied");
        match finish {
            "cancel" => {
                dispatch(&mut e, Command::Cancel, "otherMoveCancel", 1, 1);
                dispatch(&mut e, Command::Redo, "otherMoveCancelRedo", 1, 1);
            }
            "drop" => {
                dispatch(&mut e, Command::Drop, "otherMoveDrop", 1, 1);
                dispatch(&mut e, Command::Undo, "otherMoveUndo", 1, 1);
                dispatch(&mut e, Command::Redo, "otherMoveRedo", 1, 1);
            }
            _ => {
                // Undo with Move live takes back its last tweak and keeps
                // Move, leaving the document history alone; Redo puts the
                // tweak back (Jason, 2026-11-04). So the states are the armed
                // and applied ones from above.
                for (cmd, state) in [
                    (Command::Undo, "otherMoveArmed"),
                    (Command::Redo, "otherMoveApplied"),
                ] {
                    let (a, b) = (e.identities.publication, e.identities.request);
                    e.dispatch(cmd).unwrap();
                    assert_eq!(
                        (e.identities.publication - a, e.identities.request - b),
                        (1, 1)
                    );
                    assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(Tool::Move));
                    check(&e, state);
                }
            }
        }
    }
}
