use glam::DVec3;
use operandi_core::*;
use serde_json::{Value, json};
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn test_directory(label: &str) -> std::path::PathBuf {
    let parent = root().join("development/i2-review-fixes/test-io");
    std::fs::create_dir_all(&parent).unwrap();
    let path = parent.join(format!(
        "{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    path
}
fn scene(name: &str) -> Scene {
    serde_json::from_slice(
        &std::fs::read(root().join(format!("docs/i2/expectations/graphs/{name}.json"))).unwrap(),
    )
    .unwrap()
}
fn make_editor(name: &str) -> Editor {
    let mut e = Editor {
        scene: scene(name),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}
fn arm(e: &mut Editor, z: f64) {
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(p(z))).unwrap();
}
fn p(z: f64) -> Parameters {
    Parameters {
        extent: DVec3::Z * z,
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }
}
fn begin(e: &mut Editor) {
    e.dispatch(Command::BeginExtrude).unwrap();
}
fn mov(e: &mut Editor, z: f64) {
    e.dispatch(Command::Preview(p(z))).unwrap();
}
fn ids(e: &Editor, name: &str) {
    let i = &e.identities.current.meshes[&1];
    let actual = json!({"vertices":i.vertices,"faces":i.faces,"corners":i.corners,"edges":i.edges.iter().map(|((a,b),id)|json!({"vertices":[a,b],"id":id})).collect::<Vec<_>>(),"logical_faces":i.logical_faces});
    let mut v: Value = serde_json::from_slice(
        &std::fs::read(root().join(format!("docs/i2/expectations/identities/{name}.json")))
            .unwrap(),
    )
    .unwrap();
    v.as_object_mut().unwrap().remove("next_id");
    assert_eq!(actual, v);
}
fn check(e: &Editor, g: &str, i: &str, cursor: u8) {
    assert_eq!(e.scene, scene(g));
    ids(e, i);
    assert_eq!(e.i2.selected.as_ref().map(|s| s.cursor), Some(cursor));
    e.scene.validate().unwrap();
    e.identities.current.validate(&e.scene).unwrap();
}
fn park(e: &mut Editor) {
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
}
#[test]
fn exact_local_states_signed_zero_and_resume() {
    for (base, zero, moved, ii, bi) in [
        ("G0", "G1", "G2", "I1", "I0"),
        ("P0", "P1", "P2", "PI1", "PI0"),
        ("SP0", "SP1", "SP2", "PI1", "PI0"),
        ("EQ0", "EQ1", "EQ2", "I1", "I0"),
    ] {
        let mut e = make_editor(base);
        let scope = e.identities.current.clone();
        arm(&mut e, 0.5);
        begin(&mut e);
        check(&e, zero, ii, 1);
        assert_eq!(e.session.live.as_ref().unwrap().params.extent, DVec3::ZERO);
        assert!(e.history.undo.is_empty());
        mov(&mut e, 0.5);
        check(&e, moved, ii, 2);
        let old = e.identity_ticket().unwrap();
        let pub0 = e.identities.publication;
        park(&mut e);
        check(&e, base, bi, 0);
        assert_eq!(e.identities.current, scope);
        assert!(e.dispatch_with_ticket(old, Command::Redo).is_err());
        e.dispatch(Command::Redo).unwrap();
        check(&e, zero, ii, 1);
        e.dispatch(Command::Redo).unwrap();
        check(&e, moved, ii, 2);
        assert!(e.identities.publication > pub0);
        mov(&mut e, 0.);
        check(&e, zero, ii, 1);
        assert!(e.i2.selected.as_ref().unwrap().movement.is_none());
    }
}
#[test]
fn signed_moves_full_fixed_graphs() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    for (z, g) in [
        (-0.25, "Gn25"),
        (0.25, "Gp25"),
        (0.75, "Gp75"),
        (0., "Greturn0"),
    ] {
        mov(&mut e, z);
        assert_eq!(e.scene, scene(g));
        ids(&e, "I1");
        assert_eq!(e.identities.next_id().unwrap(), 83);
    }
}
#[test]
fn flat_prior_cancel_and_irreversible_parking() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    park(&mut e);
    let a = e.i2.selected.clone();
    arm(&mut e, 0.);
    begin(&mut e);
    assert_eq!(e.i2.prior, a);
    assert_eq!(e.i2.selected.as_ref().unwrap().key, 2);
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.i2.selected == a);
    assert!(e.i2.prior.is_none());
    assert_eq!(e.identities.next_id().unwrap(), 115);
    e.dispatch(Command::Drop).unwrap();
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    park(&mut e);
    assert!(e.i2.prior.is_none());
    let k = e.i2.selected.as_ref().unwrap().key;
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.i2.selected.is_none());
    assert_eq!(k, 3);
    assert_eq!(e.scene, scene("G0"));
    assert_eq!(e.identities.next_id().unwrap(), 147);
}
#[test]
fn completed_history_anchor_and_capacity() {
    for n in [3, 98, 99, 100] {
        let mut e = make_editor("G0");
        for index in (1..=n).rev() {
            let before = scene(&format!("Xminus{index}"));
            let after = if index == 1 {
                scene("G0")
            } else {
                scene(&format!("Xminus{}", index - 1))
            };
            e.history.undo.push(std::sync::Arc::new(Checkpoint {
                label: format!("H{}", n - index + 1),
                before,
                after,
                identity_before: Some(e.identities.current.clone()),
                identity_after: Some(e.identities.current.clone()),
                fold: None,
                records: Default::default(),
                mark: 0,
            }));
        }
        arm(&mut e, 0.);
        begin(&mut e);
        mov(&mut e, 0.5);
        assert_eq!(e.history.undo.len(), n);
        assert_eq!(e.history.redo.len(), 0);
        park(&mut e);
        for k in 1..=3 {
            e.dispatch(Command::Undo).unwrap();
            assert_eq!(e.scene, scene(&format!("Xminus{k}")));
        }
        for _ in 0..3 {
            e.dispatch(Command::Redo).unwrap();
        }
        check(&e, "G0", "I0", 0);
        e.dispatch(Command::Redo).unwrap();
        e.dispatch(Command::Redo).unwrap();
        check(&e, "G2", "I1", 2);
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.len(), (n + 1).min(100));
        assert!(e.i2.selected.is_none());
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, scene("G0"));
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, scene("G2"));
        assert!(e.session.live.is_none());
    }
}
#[test]
fn untouched_old_redo_until_first_park_or_finish() {
    let mut e = make_editor("G0");
    e.dispatch(Command::SelectPolygonSet("bottom".into()))
        .unwrap();
    e.dispatch(Command::Undo).unwrap();
    let old = e.history.redo.clone();
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    assert!(e.history.redo == old);
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.history.redo == old);
    begin(&mut e);
    mov(&mut e, 0.5);
    park(&mut e);
    assert!(e.history.redo.is_empty());
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.history.redo.is_empty());
}
#[test]
fn two_completed_stages_and_numeric_apply_unchanged() {
    for (z, b1, b2, ii) in [
        (0.5, "AM_B1", "AM_B2", "AM_B2I"),
        (0., "AZ_B1", "AZ_B2", "AZ_B2I"),
    ] {
        let mut e = make_editor("G0");
        arm(&mut e, 0.);
        begin(&mut e);
        mov(&mut e, z);
        e.dispatch(Command::Continue).unwrap();
        assert_eq!(e.history.undo.len(), 1);
        assert!(!e.session.live.as_ref().unwrap().applied);
        begin(&mut e);
        assert_eq!(e.scene, scene(b1));
        mov(&mut e, 0.5);
        assert_eq!(e.scene, scene(b2));
        ids(&e, ii);
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.len(), 2);
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, scene(if z == 0. { "G1" } else { "G2" }));
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, scene(b2));
        assert!(e.session.live.is_none());
    }
    let mut e = make_editor("G0");
    arm(&mut e, 0.5);
    e.dispatch(Command::Apply).unwrap();
    assert_eq!(e.scene, scene("G2"));
    assert!(e.i2.selected.is_none());
    assert_eq!(e.history.undo.len(), 1);
}
#[test]
fn routing_changed_noop_refusal_cone_metadata_and_other_owner() {
    let rows: Value = serde_json::from_slice(
        &std::fs::read(root().join("docs/i2-routing-clarification/CASES-v3.json")).unwrap(),
    )
    .unwrap();
    // An armed Cone dropped with no preview records nothing now (Cone keeps
    // its preview on Drop like Ellipsoid; no "Drop Cone" step), so the
    // parked stage survives exactly as after Cancel: that row is expected.
    let cancel_row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "cone-cancel")
        .unwrap()
        .clone();
    for row in rows.as_array().unwrap() {
        let mut e = make_editor("G0");
        frozen_traces::with_document(&mut e, "H3c3");
        arm(&mut e, 0.);
        begin(&mut e);
        mov(&mut e, 0.5);
        park(&mut e);
        let before = e.clone();
        let id = row["id"].as_str().unwrap();
        let cmd = match id {
            "mode-changed" => Command::Mode(Mode::Vertex),
            "mode-same" => Command::Mode(Mode::Polygon),
            "workplane-changed" => Command::Workplane(Workplane {
                fixed: true,
                plane: 1,
                center: DVec3::ZERO,
            }),
            "workplane-same" => Command::Workplane(Default::default()),
            "workplane-invalid" => Command::Workplane(Workplane {
                fixed: true,
                plane: 3,
                center: DVec3::ZERO,
            }),
            "item-changed" => Command::SelectItem(None),
            "item-same" => Command::SelectItem(Some(1)),
            "item-missing" => Command::SelectItem(Some(2)),
            "cone-arm" => Command::ArmCone,
            "cone-cancel" => {
                e.dispatch(Command::ArmCone).unwrap();
                Command::Cancel
            }
            "cone-drop" => {
                e.dispatch(Command::ArmCone).unwrap();
                Command::Drop
            }
            "set-same" => Command::SelectPolygonSet("top".into()),
            "set-changed" => Command::SelectPolygonSet("bottom".into()),
            "set-missing" => Command::SelectPolygonSet("missing".into()),
            "arm-move" => Command::Activate(Tool::Move),
            _ => unreachable!(),
        };
        let row = if id == "cone-drop" { &cancel_row } else { row };
        let before_pub = e.identities.publication;
        let before_req = e.identities.request;
        let ok = e.dispatch(cmd).is_ok();
        assert_eq!(ok, row["accepted"].as_bool().unwrap(), "{id}");
        assert_eq!(
            e.scene,
            serde_json::from_value::<Scene>(row["scene"].clone()).unwrap(),
            "{id}"
        );
        assert_eq!(
            e.identities.publication - before_pub,
            row["publication_delta"].as_u64().unwrap(),
            "{id} publication"
        );
        assert_eq!(
            e.identities.request - before_req,
            row["request_delta"].as_u64().unwrap(),
            "{id} request"
        );
        assert_eq!(
            e.history.undo.len(),
            row["document_cursor"].as_u64().unwrap() as usize
        );
        assert!(e.history.redo.is_empty());
        for (c, d) in e
            .history
            .undo
            .iter()
            .zip(row["document"].as_array().unwrap())
        {
            assert_eq!(c.label, d["label"].as_str().unwrap());
            assert_eq!(
                c.before,
                serde_json::from_value::<Scene>(d["before"].clone()).unwrap()
            );
            assert_eq!(
                c.after,
                serde_json::from_value::<Scene>(d["after"].clone()).unwrap()
            );
        }
        let discard = row["selected_stage"].is_null();
        assert_eq!(e.i2.selected.is_none(), discard, "{id}");
        if !discard {
            assert!(e.i2.selected == before.i2.selected);
        }
        assert!(e.identities.current == before.identities.current);
        assert!(e.i2.prior.is_none());
        if !ok {
            assert!(e == before);
        }
        drop(before); // Release the explicit assertion snapshot before inventory.
        let actual_owner = if e.session.cone.is_some() {
            Some("ConeArmed")
        } else {
            e.session.live.as_ref().map(|l| {
                assert_eq!(l.tool, Tool::Move);
                assert!(!l.applied);
                "MoveArmed"
            })
        };
        assert_eq!(actual_owner, row["tool_owner"].as_str());
        for c in &e.history.undo {
            assert_eq!(c.identity_before.as_ref(), Some(&e.identities.current));
            assert_eq!(c.identity_after.as_ref(), Some(&e.identities.current));
        }
        let roots = 2 * e.history.undo.len()
            + 1
            + e.i2
                .selected
                .as_ref()
                .map_or(0, |s| 2 + usize::from(s.movement.is_some()))
            + usize::from(e.session.live.is_some());
        assert_eq!(roots as u64, row["root_pairs"]["total"].as_u64().unwrap());
        assert_eq!(e.identities.next_id().unwrap(), 83);
        assert_eq!(e.i2.serial.next().unwrap(), 2);
        let selected = e
            .scene
            .selection
            .faces
            .iter()
            .map(|&f| e.identities.current.meshes[&1].faces[f as usize].0)
            .collect::<Vec<_>>();
        assert_eq!(json!(selected), row["ordered_face_ids"]);
    }
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    park(&mut e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.i2.selected.is_some());
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Redo).unwrap();
    check(&e, "G1", "I1", 1);
}
#[test]
fn invalid_inputs_budgets_stale_foreign_and_failed_save_are_atomic() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    for params in [
        p(f64::NAN),
        Parameters {
            extent: DVec3::X,
            ..p(0.)
        },
        Parameters {
            extrude_sides: 2,
            ..p(0.5)
        },
    ] {
        let before = e.clone();
        assert!(e.dispatch(Command::ConfigureExtrude(params)).is_err());
        assert!(e == before);
    }
    let mut foreign = make_editor("G0");
    let t = foreign.identity_ticket().unwrap();
    let before = e.clone();
    assert!(e.dispatch_with_ticket(t, Command::Undo).is_err());
    assert!(e == before);
    let directory = test_directory("save-refusal");
    let blocker = directory.join("not-a-directory");
    std::fs::write(&blocker, b"owned save obstruction").unwrap();
    let path = blocker.join("output.opscene");
    assert!(e.save_committed(&path).is_err());
    assert!(e == before);
    assert_eq!(std::fs::read(&blocker).unwrap(), b"owned save obstruction");
    assert!(!path.exists());
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    let before = e.clone();
    assert!(e.save_committed(&path).is_err());
    assert!(e == before);
    assert_eq!(std::fs::read(&blocker).unwrap(), b"owned save obstruction");
    assert!(!path.exists());
    std::fs::remove_file(blocker).unwrap();
    std::fs::remove_dir(directory).unwrap();
    let mut exhausted = make_editor("G0");
    arm(&mut exhausted, 0.);
    exhausted.identities.publication = u64::MAX;
    let before = exhausted.clone();
    assert!(exhausted.dispatch(Command::BeginExtrude).is_err());
    assert!(exhausted == before);
    assert_eq!(exhausted.identities.next_id().unwrap(), 51);
}
#[test]
fn successful_active_and_parked_save_preserve_exact_authored_bytes() {
    let directory = test_directory("save-success");
    let path = directory.join("owned.opscene");
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    e.save_committed(&path).unwrap();
    assert!(e.i2.selected.is_none());
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(persistence::load(&path).unwrap(), scene("G2"));
    e.dispatch(Command::Undo).unwrap();
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    park(&mut e);
    let s = e.i2.selected.clone();
    e.save_committed(&path).unwrap();
    assert!(e.i2.selected == s);
    assert_eq!(persistence::load(&path).unwrap(), scene("G0"));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn literal_numeric_conversions_and_full_refusal_matrix() {
    let units: Value = serde_json::from_slice(
        &std::fs::read(root().join("docs/i2/expectations/UNITS.json")).unwrap(),
    )
    .unwrap();
    for row in units["exact_valid"].as_array().unwrap() {
        assert_eq!(
            numeric::parse(row["raw"].as_str().unwrap()).unwrap(),
            row["value"].as_f64().unwrap()
        );
    }
    for raw in units["invalid_raw"].as_array().unwrap() {
        assert!(numeric::parse(raw.as_str().unwrap()).is_err());
    }
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    for p in [
        p(f64::INFINITY),
        p(f64::NEG_INFINITY),
        p(1e12),
        Parameters {
            extrude_mode: ExtrudeMode::Thick,
            ..p(0.)
        },
        Parameters {
            extrude_mode: ExtrudeMode::Auto,
            ..p(0.)
        },
        Parameters {
            extrude_uvs: ExtrudeUvs::U,
            ..p(0.)
        },
        Parameters {
            extrude_uvs: ExtrudeUvs::V,
            ..p(0.)
        },
        Parameters {
            extrude_uv_map: Some("Empty".into()),
            ..p(0.)
        },
        Parameters {
            extent: DVec3::Y,
            ..p(0.)
        },
    ] {
        let before = e.clone();
        assert!(e.dispatch(Command::ConfigureExtrude(p)).is_err());
        assert!(e == before);
    }
    let before = e.clone();
    assert!(e.dispatch(Command::Apply).is_err());
    assert!(e == before);
    for kind in 0..7 {
        let ids = &e.identities.current.meshes[&1];
        let mut h = FaceHandle {
            scope: ids.scope,
            face: ids.faces[1],
        };
        let faces = match kind {
            0 => vec![],
            1 => vec![h, h],
            2 => {
                h.face = FaceId(999);
                vec![h]
            }
            3 => {
                h.scope.epoch += 1;
                vec![h]
            }
            4 => {
                h.scope.item = 2;
                vec![h]
            }
            5 => {
                h.scope.generation += 1;
                vec![h]
            }
            _ => {
                h.face = ids.faces[0];
                vec![h]
            }
        };
        let t = e.identity_ticket().unwrap();
        let before = e.clone();
        assert!(
            e.dispatch_faces(t, &faces, Command::Preview(p(0.5)))
                .is_err()
        );
        assert!(e == before);
    }
}
#[test]
fn actual_temp_collision_preserves_v0_blocker_and_both_stage_owners() {
    let directory = test_directory("save-collision");
    let path = directory.join("owned.opscene");
    let temp = directory.join(format!(".owned.opscene.{}.tmp", std::process::id()));
    let original = std::fs::read(root().join("docs/i2/expectations/FILE_V0.opscene")).unwrap();
    let sentinel = std::fs::read(root().join("docs/i2/expectations/BLOCKER_SENTINEL.bin")).unwrap();
    for state in 0..3 {
        std::fs::write(&path, &original).unwrap();
        std::fs::write(&temp, &sentinel).unwrap();
        let mut e = make_editor("G0");
        arm(&mut e, 0.);
        begin(&mut e);
        mov(&mut e, 0.5);
        if state > 0 {
            park(&mut e);
        }
        if state == 2 {
            arm(&mut e, 0.);
            begin(&mut e);
        }
        let before = e.clone();
        // The planted temporary no longer collides (unique names), so the
        // failure is injected.
        operandi_core::persistence::refuse_next_save_on_this_thread();
        assert!(e.save_committed(&path).is_err());
        assert!(e == before);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read(&temp).unwrap(), sentinel);
        std::fs::remove_file(&temp).unwrap();
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
#[test]
fn late_results_after_cancel_and_new_request_do_not_reactivate_old_stage() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    let old = e.identity_ticket().unwrap();
    let fresh = e.identity_ticket().unwrap();
    let before = e.clone();
    assert!(e.dispatch_with_ticket(old, Command::BeginExtrude).is_err());
    assert!(e == before);
    e.dispatch_with_ticket(fresh, Command::BeginExtrude)
        .unwrap();
    mov(&mut e, 0.5);
    park(&mut e);
    let a = e.i2.selected.clone();
    arm(&mut e, 0.);
    begin(&mut e);
    let late = e.identity_ticket().unwrap();
    e.dispatch(Command::Cancel).unwrap();
    let before = e.clone();
    assert!(
        e.dispatch_with_ticket(late, Command::Preview(p(0.5)))
            .is_err()
    );
    assert!(e == before);
    assert!(e.i2.selected == a);
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    let old = e.identity_ticket().unwrap();
    let before = e.clone();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    let restored = e.clone();
    assert_eq!(e.scene, before.scene);
    assert!(
        e.dispatch_with_ticket(old, Command::Preview(p(0.25)))
            .is_err()
    );
    assert!(e == restored);
}
#[test]
fn flat_root_inventory_readers_and_repeated_branching_have_no_hidden_stage_chain() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    park(&mut e);
    let reader = e.clone();
    for index in 0..12 {
        arm(&mut e, 0.);
        begin(&mut e);
        mov(&mut e, 0.25);
        assert!(e.i2.prior.is_some());
        assert_eq!(e.i2.selected.as_ref().unwrap().key, 2 + index);
        e.dispatch(Command::Cancel).unwrap();
        assert!(e.i2.prior.is_none());
        assert_eq!(e.i2.selected.as_ref().unwrap().key, 1);
        e.dispatch(Command::Drop).unwrap();
    }
    assert_eq!(reader.scene, scene("G0"));
    assert_eq!(
        reader
            .i2
            .selected
            .as_ref()
            .unwrap()
            .movement
            .as_ref()
            .unwrap()
            .scene,
        scene("G2")
    );
    assert_eq!(reader.i2.serial.next().unwrap(), 14);
    // Full-copy reader is an explicit independent owner. Only the allocator is
    // shared; stage structures contain owned snapshots and no Arc/back links.
    drop(reader);
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.i2.selected.is_none() && e.i2.prior.is_none());
}

mod frozen_traces {
    use super::*;
    #[derive(Clone)]
    struct ReaderSnapshot {
        visible: StageImage,
        selected: Option<ExtrudeStage>,
    }
    impl ReaderSnapshot {
        fn capture(e: &Editor) -> Self {
            assert!(e.i2.prior.is_none());
            Self {
                visible: StageImage {
                    scene: e.scene.clone(),
                    identities: e.identities.current.clone(),
                },
                selected: e.i2.selected.clone(),
            }
        }
    }

    fn data(path: &str) -> Value {
        let current = path.replace(
            "docs/i2-simple/expectations/",
            "docs/i2-initial-begin-correction/",
        );
        let corrected = root().join(&current);
        if current != path && corrected.is_file() {
            return serde_json::from_slice(&std::fs::read(corrected).unwrap()).unwrap();
        }
        serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
    }
    fn state(name: &str) -> Value {
        data(&format!("docs/i2-simple/expectations/states/{name}.json"))
    }
    fn traces() -> Vec<Value> {
        ["TRACES.json", "TRACES_EXTRA.json", "TRACES_ZERO.json"]
            .iter()
            .flat_map(|f| {
                data(&format!("docs/i2-simple/expectations/{f}"))
                    .as_array()
                    .unwrap()
                    .clone()
            })
            .collect()
    }
    fn actions() -> Value {
        let mut a = data("docs/i2-simple/expectations/DOCUMENT_ACTIONS.json");
        let b = data("docs/i2-simple/expectations/DOCUMENT_ACTIONS_EXTRA.json");
        a.as_object_mut()
            .unwrap()
            .extend(b.as_object().unwrap().clone());
        a
    }
    fn image_ids(ids: &IdentityScene, name: &str) {
        let mut fake = Editor::default();
        fake.identities.current = ids.clone();
        super::ids(&fake, name);
    }
    fn check_image(i: &StageImage, r: &Value) {
        assert_eq!(i.scene, scene(r["graph"].as_str().unwrap()));
        image_ids(&i.identities, r["identity"].as_str().unwrap());
    }
    fn check_stage(e: &Editor, s: &ExtrudeStage, name: &str, anchor: &Value) {
        let d = data(&format!("docs/i2-simple/expectations/stages/{name}.json"));
        assert_eq!(s.key, d["stage_key"].as_u64().unwrap());
        assert_eq!(s.cursor as u64, d["local_cursor"].as_u64().unwrap());
        assert_eq!(s.ever_parked, d["has_ever_parked"].as_bool().unwrap());
        assert_eq!(s.anchor as u64, anchor["cursor"].as_u64().unwrap());
        check_image(&s.base, &d["roots"]["base"]);
        check_image(&s.begun, &d["roots"]["begun"]);
        if d["roots"]["movement"].is_null() {
            assert!(s.movement.is_none());
        } else {
            check_image(s.movement.as_ref().unwrap(), &d["roots"]["movement"]);
        }
        for image in [&s.base, &s.begun].into_iter().chain(s.movement.iter()) {
            assert_eq!(
                image.identities.meshes[&1].scope,
                e.identities.current.meshes[&1].scope
            );
        }
        let arm = data(&format!(
            "docs/i2/expectations/sessions/{}.json",
            d["roots"]["base"]["session"].as_str().unwrap()
        ));
        let xyz: [f64; 3] =
            serde_json::from_value(arm["params"]["extent_world_m"].clone()).unwrap();
        assert_eq!(s.armed_params.extent, DVec3::from_array(xyz));
        let center: [f64; 3] = serde_json::from_value(arm["frame"]["center"].clone()).unwrap();
        let cols: [[f64; 3]; 3] = serde_json::from_value(arm["frame"]["basis"].clone()).unwrap();
        assert_eq!(
            s.frame,
            Some(ActionFrame {
                center: DVec3::from_array(center),
                basis: glam::DMat3::from_cols(
                    DVec3::from_array(cols[0]),
                    DVec3::from_array(cols[1]),
                    DVec3::from_array(cols[2])
                )
            })
        );
        assert_eq!(s.armed_params.extrude_mode, ExtrudeMode::Shift);
        assert_eq!(s.armed_params.extrude_uvs, ExtrudeUvs::None);
        assert_eq!(s.armed_params.extrude_sides, 1);
        assert_eq!(s.armed_params.extrude_uv_map.as_deref(), Some("Texture"));
        assert!(s.armed_params.extrude_uv_selected && s.armed_params.group_polygons);
        let role = d["roles"]
            .as_str()
            .or_else(|| d["roots"]["begun"]["session_projection"]["roles"].as_str())
            .unwrap_or("A");
        let role = data(&format!("docs/i2/expectations/roles/{role}.json"));
        assert_eq!(json!(s.plan.source_layout), role["source_layout"]);
        assert_eq!(
            json!(s.plan.source_vertex_count),
            role["source_vertex_count"]
        );
        assert_eq!(json!(s.plan.vertex_sources), role["vertex_sources"]);
        assert_eq!(json!(s.plan.face_sources), role["face_sources"]);
        assert_eq!(s.plan.walls.len(), role["walls"].as_array().unwrap().len());
        for (wall, w) in s.plan.walls.iter().zip(role["walls"].as_array().unwrap()) {
            assert_eq!(wall.face as u64, w["output_face_dense"].as_u64().unwrap());
            assert_eq!(
                wall.selected as u64,
                w["selected_source_dense"].as_u64().unwrap()
            );
            assert_eq!(wall.owner as u64, w["owner_dense"].as_u64().unwrap());
            assert_eq!(json!([wall.a, wall.b]), w["source_endpoints_dense"]);
            assert_eq!(json!(wall.vertices), w["oriented_output_vertices"]);
            assert_eq!(wall.row as u64, w["row"].as_u64().unwrap());
        }
    }
    fn verify(e: &Editor, name: &str, readers: &[ReaderSnapshot]) {
        let s = state(name);
        assert_eq!(
            e.scene,
            scene(s["visible"]["graph"].as_str().unwrap()),
            "{name} graph"
        );
        image_ids(
            &e.identities.current,
            s["visible"]["identity"].as_str().unwrap(),
        );
        assert_eq!(
            e.identities.next_id().unwrap(),
            s["geometry_next_id"].as_u64().unwrap(),
            "{name} geometry allocation"
        );
        assert_eq!(
            e.i2.serial.next().unwrap(),
            s["stage_next_id"].as_u64().unwrap(),
            "{name} stage allocation"
        );
        assert_eq!(
            e.presets.get::<operandi_core::ExtrudePreset>().extent.z,
            s["preset_world_z"].as_f64().unwrap(),
            "{name} preset"
        );
        let doc = data(&format!(
            "docs/i2-simple/expectations/documents/{}.json",
            s["document"].as_str().unwrap()
        ));
        let cursor = doc["cursor"].as_u64().unwrap() as usize;
        assert_eq!(e.history.undo.len(), cursor, "{name} cursor");
        assert_eq!(
            e.history.undo.len() + e.history.redo.len(),
            doc["entries"].as_array().unwrap().len(),
            "{name} completed entries"
        );
        let a = actions();
        for (c, n) in e
            .history
            .undo
            .iter()
            .chain(e.history.redo.iter().rev())
            .zip(doc["entries"].as_array().unwrap())
        {
            let expected = &a[n.as_str().unwrap()];
            assert_eq!(c.label, expected["label"].as_str().unwrap(), "{name}");
            assert_eq!(
                c.before,
                scene(expected["before"]["graph"].as_str().unwrap())
            );
            assert_eq!(c.after, scene(expected["after"]["graph"].as_str().unwrap()));
            image_ids(
                c.identity_before.as_ref().unwrap(),
                expected["before"]["identity"].as_str().unwrap(),
            );
            image_ids(
                c.identity_after.as_ref().unwrap(),
                expected["after"]["identity"].as_str().unwrap(),
            );
        }
        let count = &s["expected_counts"];
        assert_eq!(
            usize::from(e.i2.selected.is_some()) as u64,
            count["selected_stage_objects"].as_u64().unwrap()
        );
        assert_eq!(
            usize::from(e.i2.prior.is_some()) as u64,
            count["saved_prior_stage_objects"].as_u64().unwrap()
        );
        if let Some(st) = &e.i2.selected {
            check_stage(
                e,
                st,
                s["selected_stage"].as_str().unwrap(),
                &s["selected_anchor"],
            );
        }
        if let Some(pr) = &e.i2.prior {
            check_stage(
                e,
                pr,
                s["saved_single_prior_parked"].as_str().unwrap(),
                &s["prior_anchor"],
            );
        }
        let owner = e.session.live.as_ref().map(|l| match (l.tool, l.applied) {
            (Tool::Extrude, true) => "ExtrudeBegun",
            (Tool::Extrude, false) => "ExtrudeArmed",
            (Tool::Move, false) => "MoveArmed",
            _ => "unsupported",
        });
        assert_eq!(owner, s["tool_owner"].as_str(), "{name} owner");
        if let Some(l) = &e.session.live {
            assert!(l.preview_error.is_none());
            if let Some(st) = &e.i2.selected
                && st.cursor > 0
            {
                assert_eq!(l.base, st.base.scene);
                assert_eq!(l.identity_base.as_ref(), Some(&st.base.identities));
                assert_eq!(
                    l.params.extent,
                    if st.cursor == 1 {
                        DVec3::ZERO
                    } else {
                        st.movement_params.as_ref().unwrap().extent
                    }
                );
                assert_eq!(l.action_frame, st.frame);
            }
        }
        assert_eq!(
            readers.iter().filter(|r| r.selected.is_some()).count() as u64,
            count["external_reader_stage_objects"].as_u64().unwrap(),
            "{name} declared readers"
        );
        let roots = |s: &Option<ExtrudeStage>| {
            s.as_ref()
                .map_or(0, |s| 2 + usize::from(s.movement.is_some()))
        };
        let entries = |s: &Option<ExtrudeStage>| {
            s.as_ref()
                .map_or(0, |s| 1 + usize::from(s.movement.is_some()))
        };
        let external = readers
            .iter()
            .map(|r| 1 + roots(&r.selected))
            .sum::<usize>();
        let document = 2 * (e.history.undo.len() + e.history.redo.len());
        let selected = roots(&e.i2.selected);
        let prior = roots(&e.i2.prior);
        let live = usize::from(e.session.live.is_some());
        let inventory = json!({"completed_entries":e.history.undo.len()+e.history.redo.len(),"completed_applied":e.history.undo.len(),"completed_redo":e.history.redo.len(),"selected_stage_objects":usize::from(e.i2.selected.is_some()),"saved_prior_stage_objects":usize::from(e.i2.prior.is_some()),"external_reader_stage_objects":readers.iter().filter(|r|r.selected.is_some()).count(),"retained_stage_objects_total":usize::from(e.i2.selected.is_some())+usize::from(e.i2.prior.is_some())+readers.iter().filter(|r|r.selected.is_some()).count(),"selected_local_entries":entries(&e.i2.selected),"selected_local_cursor":e.i2.selected.as_ref().map(|s|s.cursor),"prior_local_entries":entries(&e.i2.prior),"document_snapshot_pairs":document,"current_visible_snapshot_pairs":1,"selected_stage_snapshot_pairs":selected,"prior_stage_snapshot_pairs":prior,"legacy_live_base_pairs":live,"external_reader_snapshot_pairs":external,"full_snapshot_control_pairs_total":document+1+selected+prior+live+external});
        assert_eq!(&inventory, count, "{name} complete logical root inventory");
        let definitions = data("docs/i2-simple/expectations/READERS.json");
        let anchors = data("docs/i2-simple/expectations/READER_ANCHOR_BINDINGS.json");
        for (reader, id) in readers.iter().zip(s["readers"].as_array().unwrap()) {
            let id = id.as_str().unwrap();
            let d = &definitions[id];
            check_image(&reader.visible, &d["visible"]);
            check_stage(
                e,
                reader.selected.as_ref().unwrap(),
                d["stage"].as_str().unwrap(),
                &anchors[id]["captured_anchor"],
            );
        }
        let actual_selected = e
            .scene
            .selection
            .faces
            .iter()
            .map(|&f| e.identities.current.meshes[&1].faces[f as usize].0)
            .collect::<Vec<_>>();
        assert_eq!(json!(actual_selected), s["ordered_selected_face_ids"]);
    }
    pub(super) fn with_document(e: &mut Editor, name: &str) {
        let d = data(&format!(
            "docs/i2-simple/expectations/documents/{name}.json"
        ));
        let a = actions();
        let all = d["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                let v = &a[n.as_str().unwrap()];
                std::sync::Arc::new(Checkpoint {
                    label: v["label"].as_str().unwrap().into(),
                    before: scene(v["before"]["graph"].as_str().unwrap()),
                    after: scene(v["after"]["graph"].as_str().unwrap()),
                    identity_before: Some(e.identities.current.clone()),
                    identity_after: Some(e.identities.current.clone()),
                    fold: None,
                    records: Default::default(),
                    mark: 0,
                })
            })
            .collect::<Vec<_>>();
        let c = d["cursor"].as_u64().unwrap() as usize;
        e.history.undo = all[..c].to_vec();
        e.history.redo = all[c..].iter().rev().cloned().collect();
    }
    fn start(name: &str, depth: usize) -> (Editor, Vec<ReaderSnapshot>) {
        assert!(depth < 40, "fixture path cycle {name}");
        if name == "BaseArmed"
            || name == "ArmedPre5"
            || name.starts_with("Cap") && name.ends_with("Base")
        {
            let mut e = make_editor("G0");
            let doc = if name.starts_with("Cap") {
                name.trim_end_matches("Base")
            } else {
                "H3R"
            };
            with_document(&mut e, doc);
            arm(&mut e, if name == "ArmedPre5" { 0.5 } else { 0. });
            return (e, vec![]);
        }
        if ["pBase", "spBase", "eqBase"].contains(&name) {
            let mut e = make_editor(match name {
                "pBase" => "P0",
                "spBase" => "SP0",
                _ => "EQ0",
            });
            arm(&mut e, 0.);
            return (e, vec![]);
        }
        if ["mBArmed", "zBArmed"].contains(&name) {
            let mut e = make_editor("G0");
            arm(&mut e, 0.);
            begin(&mut e);
            mov(&mut e, if name == "mBArmed" { 0.5 } else { 0. });
            e.dispatch(Command::Continue).unwrap();
            return (e, vec![]);
        }
        if name == "ParkExtrudeArmed" {
            let (mut e, readers) = start("A_park", depth + 1);
            e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
            return (e, readers);
        }
        if name == "ReaderA_ParkA" {
            let (mut e, mut readers) = start("A_move", depth + 1);
            readers.push(ReaderSnapshot::capture(&e));
            park(&mut e);
            return (e, readers);
        }
        for t in traces() {
            let mut prev = t["start"].as_str().unwrap();
            for step in t["steps"].as_array().unwrap() {
                if step["after"].as_str() == Some(name) && prev != name {
                    let (mut e, mut readers) = start(t["start"].as_str().unwrap(), depth + 1);
                    for s in t["steps"].as_array().unwrap() {
                        execute(&mut e, s, &mut readers);
                        if s["after"].as_str() == Some(name) {
                            return (e, readers);
                        }
                    }
                }
                prev = step["after"].as_str().unwrap();
            }
        }
        panic!("No independent setup path for {name}")
    }
    fn execute(e: &mut Editor, step: &Value, readers: &mut Vec<ReaderSnapshot>) {
        let a = step["action"].as_str().unwrap();
        let after = step["after"].as_str().unwrap();
        let before_pub = e.identities.publication;
        let before_req = e.identities.request;
        if a.starts_with("Arm Extrude;Begin") {
            e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
            begin(e);
        } else if a.starts_with("Begin") {
            begin(e);
        } else if a.starts_with("Undo Move then Undo Begin") {
            park(e);
        } else if a.starts_with("Redo Begin then Redo Move") {
            e.dispatch(Command::Redo).unwrap();
            e.dispatch(Command::Redo).unwrap();
        } else if a.starts_with("Undo") || a.starts_with("Park C") {
            e.dispatch(Command::Undo).unwrap();
        } else if a.starts_with("Redo") || a.starts_with("separate Redo") {
            e.dispatch(Command::Redo).unwrap();
        } else if a.starts_with("Cancel") || a.starts_with("Explicit parked Cancel") {
            e.dispatch(Command::Cancel).unwrap();
        } else if a.starts_with("Drop") || a.starts_with("Finish") {
            e.dispatch(Command::Drop).unwrap();
        } else if a.starts_with("Arm Move") {
            e.dispatch(Command::Activate(Tool::Move)).unwrap();
        } else if a.starts_with("Recall") {
            let n = if a.contains("missing") {
                "missing"
            } else if a.contains("bottom") {
                "bottom"
            } else {
                "top"
            };
            let ok = e.dispatch(Command::SelectPolygonSet(n.into())).is_ok();
            assert_eq!(ok, n != "missing");
        } else if a.starts_with("Hold") {
            readers.push(ReaderSnapshot::capture(e));
        } else if a.starts_with("Release RC") {
            readers.pop().unwrap();
        } else if a.starts_with("Release RA") {
            readers.clear();
        } else if a.starts_with("field") {
            e.dispatch(Command::ConfigureExtrude(p(
                numeric::parse("250 mm").unwrap()
            )))
            .unwrap();
        } else if a.starts_with("Move")
            || a.starts_with("first pointer")
            || a.starts_with("identical0")
        {
            let z = if a.starts_with("identical0") || a.starts_with("MoveTo0") {
                0.
            } else if a.contains(".25") {
                0.25
            } else {
                0.5
            };
            mov(e, z);
        } else {
            panic!("Unmapped action {a}");
        }
        assert_eq!(
            e.identities.publication - before_pub,
            step["publication_delta"].as_u64().unwrap_or(0),
            "{a} publication"
        );
        assert_eq!(
            e.identities.request - before_req,
            step["request_delta"].as_u64().unwrap_or(0),
            "{a} request"
        );
        verify(e, after, readers);
    }
    #[test]
    fn all_34_fixed_trace_groups_full_geometry_identity_document_and_local_state() {
        let mut failures = Vec::new();
        for t in traces() {
            let name = t["id"].as_str().unwrap();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                println!("I2 TRACE {name}");
                let (mut e, mut readers) = start(t["start"].as_str().unwrap(), 0);
                verify(&e, t["start"].as_str().unwrap(), &readers);
                for step in t["steps"].as_array().unwrap() {
                    execute(&mut e, step, &mut readers);
                }
            }));
            if result.is_err() {
                failures.push(name.to_owned());
                println!("I2 TRACE FAIL {name}");
            } else {
                println!("I2 TRACE PASS {name}");
            }
        }
        assert!(
            failures.is_empty(),
            "Unchanged frozen trace failures: {failures:?}"
        );
    }
}

#[test]
fn wrong_stage_and_changed_source_layout_refuse_fresh_capture() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    let mut t = e.extrude_stage_ticket().unwrap();
    t.stage += 1;
    let before = e.clone();
    assert!(
        e.dispatch_extrude_stage(t, Command::Preview(p(0.5)))
            .is_err()
    );
    assert!(e == before);
    let t = e.extrude_stage_ticket().unwrap();
    e.session.live.as_mut().unwrap().base.items[0].mesh.faces[1]
        .vertices
        .rotate_left(1);
    let before = e.clone();
    assert!(
        e.dispatch_extrude_stage(t, Command::Preview(p(0.5)))
            .is_err()
    );
    assert!(e == before);
    let retired = FaceHandle {
        scope: e.identities.current.meshes[&1].scope,
        face: e.identities.current.meshes[&1].faces[6],
    };
    e.dispatch(Command::Cancel).unwrap();
    let ticket = e.identity_ticket().unwrap();
    let before = e.clone();
    assert!(
        e.dispatch_faces(ticket, &[retired], Command::Preview(p(0.5)))
            .is_err()
    );
    assert!(e == before);
    begin(&mut e);
    assert_ne!(e.identities.current.meshes[&1].faces[6], retired.face);
    let ticket = e.identity_ticket().unwrap();
    let before = e.clone();
    assert!(
        e.dispatch_faces(ticket, &[retired], Command::Preview(p(0.5)))
            .is_err()
    );
    assert!(e == before);
}

#[test]
fn frozen_malformed_and_unsupported_sources_refuse_before_begin_without_reservations() {
    let rows: Value = serde_json::from_slice(
        &std::fs::read(root().join("docs/i2-simple/expectations/NEGATIVE_CASES.json")).unwrap(),
    )
    .unwrap();
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["id"].as_str().unwrap().starts_with("source-"))
    {
        let name = row["input_graph"].as_str().unwrap();
        let input = scene(name);
        let mut e = if row["classification"] == "Malformed" {
            let mut e = make_editor("G0");
            arm(&mut e, 0.);
            e.scene = input.clone();
            e.session.live.as_mut().unwrap().base = input;
            e
        } else {
            let mut e = make_editor(name);
            arm(&mut e, 0.);
            e
        };
        let before = e.clone();
        let bank = e.identities.next_id().unwrap();
        let serial = e.i2.serial.next().unwrap();
        assert!(e.dispatch(Command::BeginExtrude).is_err(), "{name}");
        assert!(e == before, "{name}");
        assert_eq!(e.identities.next_id().unwrap(), bank, "{name}");
        assert_eq!(e.i2.serial.next().unwrap(), serial, "{name}");
    }
}
#[test]
fn malformed_101_completed_entries_refuse_without_eviction_or_partial_begin() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    let c = Checkpoint {
        label: "Malformed completed input".into(),
        before: e.scene.clone(),
        after: e.scene.clone(),
        identity_before: Some(e.identities.current.clone()),
        identity_after: Some(e.identities.current.clone()),
        fold: None,
        records: Default::default(),
        mark: 0,
    };
    e.history.undo = (0..101).map(|_| std::sync::Arc::new(c.clone())).collect();
    let before = e.clone();
    assert!(e.dispatch(Command::BeginExtrude).is_err());
    assert!(e == before);
    assert_eq!(e.identities.next_id().unwrap(), 51);
    assert_eq!(e.i2.serial.next().unwrap(), 1);
}

#[test]
fn continue_at_local_undo_s1_discards_movement_redo_and_arms_without_new_topology() {
    let mut e = make_editor("G0");
    arm(&mut e, 0.);
    begin(&mut e);
    mov(&mut e, 0.5);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, scene("G1"));
    assert!(e.i2.selected.as_ref().unwrap().movement.is_some());
    e.dispatch(Command::Continue).unwrap();
    assert_eq!(e.scene, scene("G1"));
    assert_eq!(e.history.undo.len(), 1);
    assert!(e.history.redo.is_empty());
    assert!(e.i2.selected.is_none());
    assert!(!e.session.live.as_ref().unwrap().applied);
    assert_eq!(e.session.live.as_ref().unwrap().params.extent, DVec3::ZERO);
    assert_eq!(e.identities.next_id().unwrap(), 83);
    assert_eq!(e.i2.serial.next().unwrap(), 2);
    begin(&mut e);
    assert_eq!(e.scene, scene("AZ_B1"));
    mov(&mut e, 0.5);
    assert_eq!(e.scene, scene("AZ_B2"));
    ids(&e, "AZ_B2I");
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, scene("G1"));
    assert_eq!(e.history.undo.len(), 1);
}
