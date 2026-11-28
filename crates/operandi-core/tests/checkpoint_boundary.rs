use glam::DVec3;
use operandi_core::*;

fn fixture() -> Editor {
    let mut e = Editor {
        scene: serde_json::from_str(include_str!("../../../docs/i2/expectations/graphs/G0.json"))
            .unwrap(),
        ..Default::default()
    };
    e.synchronize_identities().unwrap();
    e
}

fn completed_ids(e: &Editor) {
    for c in e.history.undo.iter().chain(&e.history.redo) {
        c.identity_before
            .as_ref()
            .unwrap()
            .validate(&c.before)
            .unwrap();
        c.identity_after
            .as_ref()
            .unwrap()
            .validate(&c.after)
            .unwrap();
    }
}

fn restore(
    e: &mut Editor,
    request: HistoryRequest,
    label: &str,
    scene: &Scene,
    ids: &IdentityScene,
) {
    let report = e.dispatch_history_reported(request).unwrap();
    assert!(
        matches!(report.effect, HistoryEffect::Completed { label: actual, .. } if actual == label)
    );
    assert_eq!(&e.scene, scene);
    assert_eq!(&e.identities.current, ids);
    completed_ids(e);
}

#[test]
fn ordinary_checkpoints_publish_valid_sidecars_restore_exact_ids_and_branch() {
    let mut e = fixture();
    let original = e.scene.clone();
    let original_ids = e.identities.current.clone();
    e.dispatch(Command::Rename("boundary rename".into()))
        .unwrap();
    let renamed = e.scene.clone();
    let renamed_ids = e.identities.current.clone();
    assert_eq!(e.history.undo.last().unwrap().label, "Rename Mesh");
    completed_ids(&e);
    e.dispatch(Command::ClearSelection).unwrap();
    let selected = e.scene.clone();
    let selected_ids = e.identities.current.clone();
    assert_ne!(selected.selection, renamed.selection);
    assert_eq!(e.history.undo.last().unwrap().label, "Clear Selection");
    completed_ids(&e);
    restore(
        &mut e,
        HistoryRequest::Undo,
        "Clear Selection",
        &renamed,
        &renamed_ids,
    );
    restore(
        &mut e,
        HistoryRequest::Undo,
        "Rename Mesh",
        &original,
        &original_ids,
    );
    restore(
        &mut e,
        HistoryRequest::Redo,
        "Rename Mesh",
        &renamed,
        &renamed_ids,
    );
    restore(
        &mut e,
        HistoryRequest::Redo,
        "Clear Selection",
        &selected,
        &selected_ids,
    );
    restore(
        &mut e,
        HistoryRequest::Undo,
        "Clear Selection",
        &renamed,
        &renamed_ids,
    );
    e.dispatch(Command::Rename("new branch".into())).unwrap();
    assert!(e.history.redo.is_empty());
    let branch = e.history.undo.last().unwrap();
    assert_eq!(branch.before, renamed);
    assert_eq!(branch.identity_before.as_ref().unwrap(), &renamed_ids);
    assert_eq!(
        branch.identity_after.as_ref().unwrap(),
        &e.identities.current
    );
    completed_ids(&e);
}

#[test]
fn cone_apply_and_drop_capture_complete_steps() {
    let mut e = fixture();
    let original = e.scene.clone();
    let original_ids = e.identities.current.clone();
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::ConfigureCone(ConeParameters::default()))
        .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let applied = e.scene.clone();
    let applied_ids = e.identities.current.clone();
    assert_ne!(applied, original);
    assert_eq!(e.history.undo.last().unwrap().label, "Cone");
    completed_ids(&e);
    // Drop after Apply shows no preview: it records nothing.
    let steps = e.history.undo.len();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps);
    assert!(e.session.cone.is_none());
    restore(
        &mut e,
        HistoryRequest::Undo,
        "Cone",
        &original,
        &original_ids,
    );
    restore(&mut e, HistoryRequest::Redo, "Cone", &applied, &applied_ids);
    // Drop with a preview shown keeps it as one "Cone" step.
    // Undo/Redo of an Apply never reopens Cone.
    assert!(e.session.cone.is_none());
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::ConfigureCone(ConeParameters {
        position: glam::DVec3::new(3., 0., 0.),
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let dropped = e.history.undo.last().unwrap();
    assert_eq!(dropped.label, "Cone");
    assert_eq!(dropped.before, applied);
    assert_ne!(dropped.after, applied);
    completed_ids(&e);
}

#[test]
fn i2_local_restore_then_finish_publishes_one_complete_base_to_result_checkpoint() {
    let mut e = fixture();
    let base = e.scene.clone();
    let base_ids = e.identities.current.clone();
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
    let result = e.scene.clone();
    let result_ids = e.identities.current.clone();
    let local = e.dispatch_history_reported(HistoryRequest::Undo).unwrap();
    assert!(matches!(
        local.effect,
        HistoryEffect::Local { from: 2, to: 1, .. }
    ));
    assert!(e.history.undo.is_empty() && e.history.redo.is_empty());
    assert!(
        e.dispatch_history_reported(HistoryRequest::Redo)
            .unwrap()
            .restored_local()
    );
    assert_eq!(e.scene, result);
    assert_eq!(e.identities.current, result_ids);
    e.dispatch(Command::Drop).unwrap();
    assert!(e.i2.selected.is_none());
    assert_eq!(e.history.undo.len(), 1);
    let c = &e.history.undo[0];
    assert_eq!(c.label, "Extrude");
    assert_eq!(c.before, base);
    assert_eq!(c.after, result);
    assert_eq!(c.identity_before.as_ref().unwrap(), &base_ids);
    assert_eq!(c.identity_after.as_ref().unwrap(), &result_ids);
    completed_ids(&e);
    restore(&mut e, HistoryRequest::Undo, "Extrude", &base, &base_ids);
    restore(
        &mut e,
        HistoryRequest::Redo,
        "Extrude",
        &result,
        &result_ids,
    );
}

#[test]
fn substantive_surface_mode_outer_publication_failure_preserves_accepted_state() {
    for tool in [Tool::SmoothShift, Tool::Thicken] {
        let mut e = fixture();
        e.dispatch(Command::Activate(tool)).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.smooth_shift.offset = 0.1;
        e.dispatch(Command::Preview(p)).unwrap();
        assert_ne!(e.scene, e.session.live.as_ref().unwrap().base);
        assert_eq!(
            e.mode_route_eligibility(),
            ModeRouteEligibility::DropsToolFirst
        );
        // Measure the actual nested route, including synchronization advances;
        // leave exactly that much publication capacity for the real command.
        let mut measured = e.clone();
        measured.dispatch(Command::Drop).unwrap();
        measured.dispatch(Command::Mode(Mode::Edge)).unwrap();
        let nested_cost = measured.identities.publication - e.identities.publication;
        assert!(nested_cost > 0);
        e.identities.publication = u64::MAX - nested_cost;
        let before = e.clone();
        let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
        let mut nested_control = e.clone();
        nested_control.dispatch(Command::Drop).unwrap();
        assert_eq!(
            nested_control.history.undo.last().unwrap().label,
            tool.label()
        );
        nested_control.dispatch(Command::Mode(Mode::Edge)).unwrap();
        assert_eq!(nested_control.identities.publication, u64::MAX);
        assert!(nested_control.session.live.is_none());
        assert_eq!(nested_control.scene.selection.mode, Mode::Edge);
        completed_ids(&nested_control);
        assert_eq!(
            e.dispatch(Command::Mode(Mode::Edge)).unwrap_err(),
            "Publication budget exhausted"
        );
        assert_eq!(e.scene, before.scene);
        assert_eq!(e.scene.selection, before.scene.selection);
        assert_eq!(e.identities.current, before.identities.current);
        assert!(e.history.undo == before.history.undo && e.history.redo == before.history.redo);
        assert!(e.session.live == before.session.live && e.i2 == before.i2);
        assert_eq!(e.revision, before.revision);
        assert_eq!(e.identities.publication, before.identities.publication);
        assert_eq!(e.identities.request, before.identities.request);
        // These already-previewed paths need no new IDs/stage reservation. The
        // explicit marks avoid relying on shared-bank pointer equality.
        assert_eq!(
            marks,
            (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
        );
    }
}
