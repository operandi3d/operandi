//! Baseline characterization written/run before the ownership-view migration.
//! Constructed raw overlaps are diagnostics, not admitted native workflows.
use glam::DVec3;
use operandi_core::*;
fn editor() -> Editor {
    let mut e = Editor {
        scene: serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json"))
            .unwrap(),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}
fn moved() -> Editor {
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.extent = DVec3::Z * 0.5;
    e.dispatch(Command::Preview(p)).unwrap();
    e
}
fn parked() -> Editor {
    let mut e = moved();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
    e
}
#[test]
fn missing_or_wrong_i2_mirror_does_not_block_restorative_undo() {
    for wrong in [false, true] {
        let mut e = moved();
        if wrong {
            e.session.live.as_mut().unwrap().tool = Tool::Move;
        } else {
            e.session.live = None;
        }
        assert_eq!(
            e.history_eligibility(HistoryAction::Undo).owner,
            HistoryOwner::ExtrudeStage
        );
        assert!(e.history_eligibility(HistoryAction::Undo).is_available());
        let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 1);
        assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Extrude);
        let expected: Scene =
            serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G1.json"))
                .unwrap();
        assert_eq!(e.scene, expected);
        e.identities.current.validate(&e.scene).unwrap();
        assert_eq!(
            marks,
            (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
        );
    }
}
#[test]
fn raw_overlaps_keep_existing_category_refusal_precedence() {
    let mut e = editor();
    let target = e.scene.selection.item.unwrap();
    e.session.cone = Some(NumericCone {
        target,
        requested: Default::default(),
        preview: None,
        error: None,
        applied_step: None,
    });
    e.session.ellipsoid = Some(NumericEllipsoid {
        target,
        requested: Default::default(),
        preview: None,
        error: None,
        applied_step: None,
    });
    e.session.spike = Some(NumericSpike::armed(target, 0.5));
    for action in [HistoryAction::Undo, HistoryAction::Redo] {
        let before = e.clone();
        let next = e.identities.next_id().unwrap();
        assert_eq!(e.history_eligibility(action).owner, HistoryOwner::Ellipsoid);
        // Nothing to undo or redo yet: an empty success that changes nothing.
        assert_eq!(
            e.history_eligibility(action).availability,
            HistoryAvailability::EmptyHistory
        );
        assert!(e == before);
        assert_eq!(e.identities.next_id().unwrap(), next);
    }
    e.session.ellipsoid = None;
    assert_eq!(
        e.history_eligibility(HistoryAction::Undo).owner,
        HistoryOwner::Cone
    );
}
#[test]
fn parked_context_coexists_with_numeric_tasks_and_cancel_restores_resume() {
    for command in [Command::ArmCone, Command::ArmEllipsoid, Command::ArmSpike] {
        let mut e = parked();
        let stage = e.i2.selected.clone();
        let scene = e.scene.clone();
        e.dispatch(command).unwrap();
        assert_eq!(e.i2.selected, stage);
        assert_eq!(
            e.history_eligibility(HistoryAction::Redo).availability,
            HistoryAvailability::Refused(HistoryRefusal::ResumeOwner)
        );
        let before = e.clone();
        let next = e.identities.next_id().unwrap();
        assert_eq!(
            e.dispatch(Command::Redo).unwrap_err(),
            HistoryRefusal::ResumeOwner.to_string()
        );
        assert!(e == before);
        assert_eq!(e.identities.next_id().unwrap(), next);
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, scene);
        assert_eq!(e.i2.selected, stage);
        assert!(e.history_eligibility(HistoryAction::Redo).is_available());
    }
    // Picking Cone during an active stage drops the stage, then arms.
    let mut e = moved();
    e.dispatch(Command::ArmCone).unwrap();
    assert!(!e.i2_active());
    assert!(e.session.cone.is_some());
}
#[test]
fn selection_gesture_is_separate_and_completion_branches_parked_context() {
    for cancel in [false, true] {
        let mut e = parked();
        let scene = e.scene.clone();
        let stage = e.i2.selected.clone();
        e.dispatch(Command::BeginSelection).unwrap();
        assert!(e.i2.selected.is_some());
        assert_eq!(
            e.history_eligibility(HistoryAction::Redo).availability,
            HistoryAvailability::Refused(HistoryRefusal::ResumeOwner)
        );
        e.dispatch(Command::ClearSelection).unwrap();
        e.dispatch(Command::EndSelection { cancel }).unwrap();
        assert!(e.selection_start.is_none());
        if cancel {
            assert_eq!(e.scene, scene);
            assert_eq!(e.i2.selected, stage);
            assert!(e.history.undo.is_empty());
            assert!(e.history_eligibility(HistoryAction::Redo).is_available());
        } else {
            assert!(e.i2.selected.is_none() && e.i2.prior.is_none());
            assert_eq!(e.history.undo.len(), 1);
            assert_eq!(e.history.undo.last().unwrap().label, "Selection");
            assert!(e.scene.selection.faces.is_empty());
        }
    }
}
#[test]
fn source_level_selection_activate_and_generic_cancel_do_not_normalize_gesture() {
    let mut e = editor();
    let scene = e.scene.clone();
    e.dispatch(Command::BeginSelection).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(e.selection_start.is_some() && e.session.live.is_some());
    e.dispatch(Command::Cancel).unwrap();
    assert!(e.session.live.is_none() && e.selection_start.is_some());
    e.dispatch(Command::EndSelection { cancel: true }).unwrap();
    assert_eq!(e.scene, scene);
    assert!(e.history.undo.is_empty());
}
#[test]
fn successful_empty_history_keeps_cleanup_and_publication_effects() {
    for mode in 0..3 {
        let mut e = editor();
        if mode == 1 {
            e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
        }
        if mode == 2 {
            e.dispatch(Command::ArmSpike).unwrap();
        }
        let action = if mode == 2 {
            HistoryAction::Redo
        } else {
            HistoryAction::Undo
        };
        // An armed Bevel has its activation to undo: Undo closes it
        // (MU-MID-02, Jason 2026-11-04).
        assert_eq!(
            e.history_eligibility(action).availability,
            if mode == 1 {
                HistoryAvailability::Available
            } else {
                HistoryAvailability::EmptyHistory
            }
        );
        let scene = e.scene.clone();
        let publication = e.identities.publication;
        let request = e.identities.request;
        let next = e.identities.next_id().unwrap();
        e.dispatch(if action == HistoryAction::Undo {
            Command::Undo
        } else {
            Command::Redo
        })
        .unwrap();
        assert_eq!(e.scene, scene);
        assert!(e.history.undo.is_empty() && e.history.redo.is_empty());
        assert!(e.session.live.is_none() && e.session.spike.is_none());
        assert!(e.identities.publication > publication && e.identities.request > request);
        assert_eq!(e.identities.next_id().unwrap(), next);
    }
}

#[test]
fn local_resume_reason_order_remains_context_then_incoming_owner_then_preview() {
    let mut e = parked();
    e.dispatch(Command::ArmCone).unwrap();
    // Deliberately overlapping refusal causes: classify, do not normalize them.
    e.action_context.center = CenterMode::Origin;
    for reason in [HistoryRefusal::ResumeContext, HistoryRefusal::ResumeOwner] {
        let before = e.clone();
        let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
        assert_eq!(
            e.history_eligibility(HistoryAction::Redo).availability,
            HistoryAvailability::Refused(reason.clone())
        );
        assert_eq!(e.dispatch(Command::Redo).unwrap_err(), reason.to_string());
        assert!(e == before);
        assert_eq!(
            marks,
            (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
        );
        e.action_context = Default::default();
    }
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.session.live.as_mut().unwrap().preview_error = Some("invalid preview".into());
    let before = e.clone();
    assert_eq!(
        e.dispatch(Command::Redo).unwrap_err(),
        HistoryRefusal::ResumePreview.to_string()
    );
    assert!(e == before);
}
