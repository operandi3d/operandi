//! Combined H3 save confirmation using unchanged reviewed literals.
//! Comparison helpers retained locally from the reviewed gap adapter; runtime is unchanged.
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
#[test]
fn fixed_h3_active_save_and_repeat() {
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
        dispatch(&mut e, Command::SelectPolygonSet(set.into()), state, 2, 2);
    }
    dispatch(&mut e, Command::Activate(Tool::Extrude), "N1-armed", 1, 1);
    dispatch(&mut e, Command::BeginExtrude, "N1-initialS1", 1, 1);
    dispatch(&mut e, Command::ConfigureExtrude(p(0.5)), "N1-moved", 1, 1);
    dispatch(&mut e, Command::Undo, "N1-localS1", 1, 1);
    dispatch(&mut e, Command::Undo, "N1-park-c3", 1, 1);
    for state in ["N1-park-c2", "N1-park-c1", "N1-park-c0"] {
        dispatch(&mut e, Command::Undo, state, 1, 1);
    }
    for state in [
        "N1-park-c1",
        "N1-park-c2",
        "N1-park-c3",
        "N1-resumeS1",
        "N1-resumeS2",
    ] {
        dispatch(&mut e, Command::Redo, state, 1, 1);
    }
    let d = root()
        .join("development/i2-review-fixes/test-io/origin")
        .join(std::process::id().to_string());
    assert!(!d.exists(), "Never overwrite an earlier semantic run");
    std::fs::create_dir_all(&d).unwrap();
    let source = d.join("source-G0.opscene");
    let destination = d.join("active-save.opscene");
    let g0 = std::fs::read(root().join("docs/i2-gap-plan/wire/G0.expected.opscene")).unwrap();
    let g2 = std::fs::read(root().join("docs/i2-gap-plan/wire/G2.expected.opscene")).unwrap();
    std::fs::write(&source, &g0).unwrap();
    std::fs::write(&destination, &g0).unwrap();
    let old_epoch = e.identities.epoch;
    for iteration in 1..=2 {
        let marks = (e.identities.publication, e.identities.request);
        e.save_committed(&destination).unwrap();
        assert_eq!(
            (
                e.identities.publication - marks.0,
                e.identities.request - marks.1
            ),
            (1, 1)
        );
        check(&e, "activeSaveH3");
        assert_eq!(e.history.undo.len(), 4);
        assert!(e.history.redo.is_empty());
        let bytes = std::fs::read(&destination).unwrap();
        assert_eq!(bytes, g2, "Strict W1 bytes at Save {iteration}");
        assert_eq!(std::fs::read(&source).unwrap(), g0);
        let decoded = persistence::load(&destination).unwrap();
        let expected: Scene = serde_json::from_value(read("graphs/G2.json")).unwrap();
        assert_eq!(decoded, expected);
        assert_eq!(decoded, e.scene);
        println!(
            "ACTUAL_SAVE_PASS iteration={iteration} path={} bytes={} actions={}",
            destination.display(),
            bytes.len(),
            e.history.undo.len()
        );
    }
    let mut reopened = Editor {
        scene: persistence::load(&destination).unwrap(),
        ..Default::default()
    };
    reopened.synchronize_identities().unwrap();
    assert_ne!(reopened.identities.epoch, old_epoch);
    check(&reopened, "reopenG2");
    assert_eq!(std::fs::read(&destination).unwrap(), g2);
    println!(
        "SEMANTIC_REOPEN_PASS same_process_only path={}",
        destination.display()
    );
}
