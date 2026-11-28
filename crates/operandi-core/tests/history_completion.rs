use glam::DVec3;
use operandi_core::*;

fn fixture() -> Editor {
    let mut editor = Editor {
        scene: serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json"))
            .unwrap(),
        ..Default::default()
    };
    editor.synchronize_identities().unwrap();
    editor
}
fn moved() -> Editor {
    let mut e = fixture();
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
fn applied(e: &mut Editor, request: HistoryRequest) -> HistoryCompletion {
    let result = e.dispatch_history_reported(request).unwrap();
    assert_eq!(result.request, request);
    assert_eq!(result.publication.epoch, e.identities.epoch);
    assert_eq!(result.publication.publication, e.identities.publication);
    assert_eq!(result.publication.request, e.identities.request);
    result
}

#[test]
fn empty_success_is_distinct_from_content_change_and_task_closure() {
    let mut e = fixture();
    for request in [
        HistoryRequest::Undo,
        HistoryRequest::Redo,
        HistoryRequest::Cancel,
    ] {
        let old = e.identities.publication;
        let report = applied(&mut e, request);
        assert_eq!(
            report.effect,
            if request == HistoryRequest::Cancel {
                HistoryEffect::Cancel
            } else {
                HistoryEffect::Empty
            }
        );
        assert!(
            !report.authored_scene_changed && !report.foreground_closed && !report.restored_local()
        );
        assert!(report.publication.publication > old);
    }
    // Undo of an armed tool's activation closes it (MU-MID-02).
    for tool in [Tool::Bevel, Tool::Extrude, Tool::SmoothShift, Tool::Thicken] {
        e.dispatch(Command::Activate(tool)).unwrap();
        let report = applied(&mut e, HistoryRequest::Undo);
        assert_eq!(
            report.effect,
            HistoryEffect::ToolClosed {
                label: tool.label().into()
            }
        );
        assert!(report.foreground_closed && !report.authored_scene_changed);
    }
    e.dispatch(Command::ArmSpike).unwrap();
    let report = applied(&mut e, HistoryRequest::Redo);
    assert_eq!(report.effect, HistoryEffect::Empty);
    assert!(report.foreground_closed && !report.authored_scene_changed);
}

#[test]
fn armed_cone_drop_records_no_step_and_apply_is_a_plain_step() {
    // Drop with no preview is neutral and Apply is a plain "Cone" step,
    // both like Ellipsoid.
    let mut e = fixture();
    let steps = e.history.undo.len();
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps);
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::ConfigureCone(ConeParameters::default()))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    for (request, action) in [
        (HistoryRequest::Undo, HistoryAction::Undo),
        (HistoryRequest::Redo, HistoryAction::Redo),
    ] {
        let report = applied(&mut e, request);
        assert!(
            matches!(report.effect, HistoryEffect::Completed { action: actual, ref label } if actual == action && label == "Cone")
        );
        assert!(report.authored_scene_changed && !report.restored_local());
    }
}

#[test]
fn local_steps_resume_and_cancel_report_the_executed_route() {
    let mut e = moved();
    let key = e.i2.selected.as_ref().unwrap().key;
    for (request, action, from, to) in [
        (HistoryRequest::Undo, HistoryAction::Undo, 2, 1),
        (HistoryRequest::Undo, HistoryAction::Undo, 1, 0),
        (HistoryRequest::Redo, HistoryAction::Redo, 0, 1),
        (HistoryRequest::Redo, HistoryAction::Redo, 1, 2),
    ] {
        let report = applied(&mut e, request);
        assert_eq!(
            report.effect,
            HistoryEffect::Local {
                action,
                stage: key,
                from,
                to
            }
        );
        assert!(report.restored_local());
        assert_eq!(e.i2.selected.as_ref().unwrap().cursor, to);
    }
    let report = applied(&mut e, HistoryRequest::Cancel);
    assert_eq!(
        report.effect,
        HistoryEffect::LocalCancel {
            stage: key,
            from: 2,
            restored_prior: None
        }
    );
    assert!(report.restored_local() && !report.foreground_closed);
    assert!(e.session.live.is_some());
    e.dispatch(Command::Drop).unwrap();
    e = moved();
    applied(&mut e, HistoryRequest::Undo);
    applied(&mut e, HistoryRequest::Undo);
    let report = applied(&mut e, HistoryRequest::Cancel);
    assert_eq!(report.effect, HistoryEffect::ParkedDiscard);
    assert!(
        !report.restored_local() && !report.authored_scene_changed && !report.foreground_closed
    );
}

#[test]
fn completed_redo_below_parked_anchor_is_not_local_restore() {
    let mut e = fixture();
    e.dispatch(Command::Rename("renamed".into())).unwrap();
    e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
    e.dispatch(Command::ConfigureExtrude(Parameters {
        extrude_mode: ExtrudeMode::Shift,
        extrude_uvs: ExtrudeUvs::None,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::BeginExtrude).unwrap();
    applied(&mut e, HistoryRequest::Undo);
    let report = applied(&mut e, HistoryRequest::Undo);
    assert!(matches!(
        report.effect,
        HistoryEffect::Completed {
            action: HistoryAction::Undo,
            ..
        }
    ));
    let report = applied(&mut e, HistoryRequest::Redo);
    assert!(matches!(
        report.effect,
        HistoryEffect::Completed {
            action: HistoryAction::Redo,
            ..
        }
    ));
    assert!(!report.restored_local());
    assert_eq!(e.i2.selected.as_ref().unwrap().cursor, 0);
    assert!(applied(&mut e, HistoryRequest::Redo).restored_local());
}

#[test]
fn refusal_is_typed_at_the_guard_and_final_publication_failure_is_not_success() {
    let mut e = moved();
    applied(&mut e, HistoryRequest::Undo);
    applied(&mut e, HistoryRequest::Undo);
    e.action_context.center = CenterMode::Origin;
    let before = e.clone();
    let failure = e
        .dispatch_history_reported(HistoryRequest::Redo)
        .unwrap_err();
    assert_eq!(
        failure.reason,
        HistoryFailureReason::History(HistoryRefusal::ResumeContext)
    );
    assert!(!failure.live_diagnostic_changed);
    assert!(e == before);

    let mut e = moved();
    e.session.live.as_mut().unwrap().preview_error = Some("retained invalid draft".into());
    e.identities.publication = u64::MAX;
    let before = e.clone();
    let failure = e
        .dispatch_history_reported(HistoryRequest::Cancel)
        .unwrap_err();
    assert_eq!(
        failure.reason,
        HistoryFailureReason::Legacy("Publication budget exhausted".into())
    );
    assert_eq!(failure.publication.publication, u64::MAX);
    assert!(failure.live_diagnostic_changed);
    // Existing error feedback can change while the attempted local Cancel's
    // Scene, stage and history do not publish. Allocation banks are not rewound.
    assert_eq!(e.scene, before.scene);
    assert!(e.i2 == before.i2);
    assert!(e.history.undo == before.history.undo);
    assert!(e.history.redo == before.history.redo);
    assert_eq!(e.revision, before.revision);
    assert!(e.session.live.as_ref().unwrap().preview_error.is_none());
}

#[test]
fn reported_and_legacy_requests_share_state_messages_and_publication() {
    let idle = fixture();
    let active = moved();
    let mut parked = active.clone();
    parked.dispatch(Command::Undo).unwrap();
    parked.dispatch(Command::Undo).unwrap();
    let mut refused = parked.clone();
    refused.action_context.center = CenterMode::Origin;
    let mut ordinary = fixture();
    ordinary
        .dispatch(Command::Rename("renamed".into()))
        .unwrap();
    for (start, request) in [
        (idle.clone(), HistoryRequest::Undo),
        (idle, HistoryRequest::Cancel),
        (ordinary, HistoryRequest::Undo),
        (active.clone(), HistoryRequest::Undo),
        (active, HistoryRequest::Cancel),
        (parked, HistoryRequest::Redo),
        (refused, HistoryRequest::Redo),
    ] {
        // Same document epoch/accepted snapshot; no cloned bank is used as an
        // allocation oracle. These routes do not require new topology.
        let mut legacy = start.clone();
        let mut reported = start;
        let command = match request {
            HistoryRequest::Undo => Command::Undo,
            HistoryRequest::Redo => Command::Redo,
            HistoryRequest::Cancel => Command::Cancel,
        };
        let old = legacy.dispatch(command);
        let new = reported
            .dispatch_history_reported(request)
            .map(|_| ())
            .map_err(|e| e.to_string());
        assert_eq!(old, new);
        assert!(legacy == reported);
        assert_eq!(legacy.identities.current, reported.identities.current);
        assert_eq!(
            legacy.identities.publication,
            reported.identities.publication
        );
        assert_eq!(legacy.identities.request, reported.identities.request);
    }
}
