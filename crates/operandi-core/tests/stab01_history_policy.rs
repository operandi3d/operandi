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
fn query(
    e: &Editor,
    action: HistoryAction,
    owner: HistoryOwner,
    availability: HistoryAvailability,
    label: Option<&str>,
) {
    let before = e.clone();
    let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
    for _ in 0..16 {
        let q = e.history_eligibility(action);
        assert_eq!(q.owner, owner);
        assert_eq!(q.availability, availability);
        assert_eq!(q.label, label);
    }
    assert!(*e == before);
    assert_eq!(
        marks,
        (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
    );
}
fn moved(e: &mut Editor) {
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
}
fn park(e: &mut Editor) {
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Undo).unwrap();
}
fn refused(e: &mut Editor, action: HistoryAction, reason: HistoryRefusal) {
    let before = e.clone();
    let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
    assert_eq!(
        e.history_eligibility(action).availability,
        HistoryAvailability::Refused(reason.clone())
    );
    let command = match action {
        HistoryAction::Undo => Command::Undo,
        HistoryAction::Redo => Command::Redo,
    };
    assert_eq!(e.dispatch(command).unwrap_err(), reason.to_string());
    assert!(*e == before);
    assert_eq!(
        marks,
        (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap())
    );
}
#[test]
fn ordinary_empty_completed_and_state_change_revalidation() {
    let mut e = editor();
    for action in [HistoryAction::Undo, HistoryAction::Redo] {
        query(
            &e,
            action,
            HistoryOwner::Document,
            HistoryAvailability::EmptyHistory,
            None,
        );
    }
    let scene = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, scene);
    assert!(e.history.undo.is_empty() && e.history.redo.is_empty());
    e.dispatch(Command::Rename("renamed".into())).unwrap();
    query(
        &e,
        HistoryAction::Undo,
        HistoryOwner::Document,
        HistoryAvailability::Available,
        Some("Rename Mesh"),
    );
    e.dispatch(Command::Undo).unwrap();
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::Document,
        HistoryAvailability::Available,
        Some("Rename Mesh"),
    );
    // A prior Available result never authorizes a later live-tool history
    // route. (Move and the primitives keep running through history now;
    // Thicken still refuses.)
    e.dispatch(Command::Activate(Tool::Thicken)).unwrap();
    refused(
        &mut e,
        HistoryAction::Redo,
        HistoryRefusal::LiveTool(HistoryAction::Redo),
    );
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene.active().unwrap().name, "renamed");
}
#[test]
fn armed_category_matrix_preserves_empty_cleanup_and_redo_refusal() {
    for tool in [Tool::Bevel, Tool::Extrude, Tool::SmoothShift, Tool::Thicken] {
        let mut e = editor();
        e.dispatch(Command::Rename("renamed".into())).unwrap();
        e.dispatch(Command::Undo).unwrap();
        e.dispatch(Command::Activate(tool)).unwrap();
        // Undo would undo the activation: the tool closes (MU-MID-02).
        query(
            &e,
            HistoryAction::Undo,
            HistoryOwner::LiveTool(tool),
            HistoryAvailability::Available,
            Some(tool.label()),
        );
        refused(
            &mut e,
            HistoryAction::Redo,
            HistoryRefusal::LiveTool(HistoryAction::Redo),
        );
        e.dispatch(Command::Undo).unwrap();
        assert!(e.session.live.is_none());
        assert_eq!(e.scene, editor().scene);
        // Redo opens the tool again; leaving it keeps the older Redo.
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(tool));
        e.dispatch(Command::Cancel).unwrap();
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene.active().unwrap().name, "renamed");
    }
    // Transform tools and the drawn primitives keep running through
    // Undo/Redo; their activation is a step of its own (MU-MID-02/03):
    // Undo closes the tool, Redo opens it again.
    for tool in [
        Tool::Move,
        Tool::Rotate,
        Tool::Scale,
        Tool::Transform,
        Tool::Box,
        Tool::Sphere,
        Tool::Cylinder,
    ] {
        let mut e = editor();
        e.dispatch(Command::Activate(tool)).unwrap();
        let scene = e.scene.clone();
        query(
            &e,
            HistoryAction::Undo,
            HistoryOwner::LiveTool(tool),
            HistoryAvailability::Available,
            Some(tool.label()),
        );
        query(
            &e,
            HistoryAction::Redo,
            HistoryOwner::LiveTool(tool),
            HistoryAvailability::EmptyHistory,
            None,
        );
        e.dispatch(Command::Undo).unwrap();
        assert!(e.session.live.is_none());
        assert_eq!(e.scene, scene);
        query(
            &e,
            HistoryAction::Redo,
            HistoryOwner::Document,
            HistoryAvailability::Available,
            Some(tool.label()),
        );
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.session.live.as_ref().map(|l| l.tool), Some(tool));
        assert_eq!(e.scene, scene);
    }
}
#[test]
fn numeric_categories_keep_distinct_history_owners() {
    let mut e = editor();
    e.dispatch(Command::Rename("renamed".into())).unwrap();
    e.dispatch(Command::ArmEllipsoid).unwrap();
    for action in [HistoryAction::Undo, HistoryAction::Redo] {
        // Undo would close Ellipsoid (MU-MID-02).
        let label = if action == HistoryAction::Undo {
            Some("Ellipsoid")
        } else {
            None
        };
        let availability = if action == HistoryAction::Undo {
            HistoryAvailability::Available
        } else {
            HistoryAvailability::EmptyHistory
        };
        query(&e, action, HistoryOwner::Ellipsoid, availability, label);
    }
    // Undo closes Ellipsoid; Redo opens it again.
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.ellipsoid.is_none());
    assert_eq!(e.scene.active().unwrap().name, "renamed");
    e.dispatch(Command::Redo).unwrap();
    assert!(e.session.ellipsoid.is_some());
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::ArmSpike).unwrap();
    // Undo closes Spike (its activation); Redo opens it again.
    query(
        &e,
        HistoryAction::Undo,
        HistoryOwner::Spike,
        HistoryAvailability::Available,
        Some("Spike"),
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.spike.is_none());
    e.dispatch(Command::Redo).unwrap();
    assert!(e.session.spike.is_some());
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::ArmSpike).unwrap();
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::Spike,
        HistoryAvailability::EmptyHistory,
        None,
    );
    e.dispatch(Command::Redo).unwrap();
    assert!(e.session.spike.is_none());
    e.dispatch(Command::ArmCone).unwrap();
    // Undo closes Cone (its activation step); Redo opens it again.
    query(
        &e,
        HistoryAction::Undo,
        HistoryOwner::Cone,
        HistoryAvailability::Available,
        Some("Cone"),
    );
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.cone.is_none());
    e.dispatch(Command::Redo).unwrap();
    assert!(e.session.cone.is_some());
    e.dispatch(Command::ConfigureCone(ConeParameters {
        sides: 8,
        segments: 3,
        ..Default::default()
    }))
    .unwrap();
    e.dispatch(Command::Apply).unwrap();
    let scene = e.scene.clone();
    assert_eq!(
        e.history_eligibility(HistoryAction::Undo).owner,
        HistoryOwner::Cone
    );
    assert!(e.history_eligibility(HistoryAction::Undo).is_available());
    e.dispatch(Command::Undo).unwrap();
    assert!(e.history_eligibility(HistoryAction::Redo).is_available());
    assert_eq!(
        e.history_eligibility(HistoryAction::Redo).owner,
        HistoryOwner::Cone
    );
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, scene);
}
#[test]
fn i2_local_states_and_parked_completed_precedence() {
    let mut e = editor();
    e.dispatch(Command::Rename("prior".into())).unwrap();
    moved(&mut e);
    query(
        &e,
        HistoryAction::Undo,
        HistoryOwner::ExtrudeStage,
        HistoryAvailability::Available,
        Some("Move Extrude"),
    );
    refused(&mut e, HistoryAction::Redo, HistoryRefusal::LocalMovement);
    e.dispatch(Command::Undo).unwrap();
    query(
        &e,
        HistoryAction::Undo,
        HistoryOwner::ExtrudeStage,
        HistoryAvailability::Available,
        Some("Begin Extrude"),
    );
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::ExtrudeStage,
        HistoryAvailability::Available,
        Some("Move Extrude"),
    );
    e.dispatch(Command::Redo).unwrap();
    park(&mut e);
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::ExtrudeStage,
        HistoryAvailability::Available,
        Some("Begin Extrude"),
    );
    query(
        &e,
        HistoryAction::Undo,
        HistoryOwner::Document,
        HistoryAvailability::Available,
        Some("Rename Mesh"),
    );
    e.dispatch(Command::Undo).unwrap();
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        ..Default::default()
    }))
    .unwrap();
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::Document,
        HistoryAvailability::Available,
        Some("Rename Mesh"),
    );
    e.dispatch(Command::Redo).unwrap();
    refused(&mut e, HistoryAction::Redo, HistoryRefusal::ResumeContext);
    e.dispatch(Command::ActionContext(Default::default()))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::ExtrudeStage,
        HistoryAvailability::Available,
        Some("Begin Extrude"),
    );
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.session.live.as_ref().unwrap().tool, Tool::Extrude);
    // Only Begin was resumed: one Undo parks it without undoing the document.
    e.dispatch(Command::Undo).unwrap();
    // A cursor above a retained anchor uses ordinary history, not hidden local Redo.
    e.i2.selected.as_mut().unwrap().anchor = 0;
    query(
        &e,
        HistoryAction::Redo,
        HistoryOwner::Document,
        HistoryAvailability::EmptyHistory,
        None,
    );
    let scene = e.scene.clone();
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, scene);
}
#[test]
fn local_redo_context_basis_and_owner_changes_revalidate_atomically() {
    let mut e = editor();
    moved(&mut e);
    park(&mut e);
    assert!(e.history_eligibility(HistoryAction::Redo).is_available());
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Linear(
        LinearFalloff {
            start: DVec3::ZERO,
            end: DVec3::X,
            ..Default::default()
        },
    ))))
    .unwrap();
    refused(&mut e, HistoryAction::Redo, HistoryRefusal::ResumeContext);
    e.dispatch(Command::SpatialFalloff(None)).unwrap();
    e.scene.active_mut().unwrap().name = "external basis change".into();
    refused(&mut e, HistoryAction::Redo, HistoryRefusal::ParkedBasis);
    let mut e = editor();
    moved(&mut e);
    park(&mut e);
    e.selection_start = Some(Box::new(e.scene.clone()));
    refused(&mut e, HistoryAction::Redo, HistoryRefusal::ResumeOwner);
    e.selection_start = None;
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(e.history_eligibility(HistoryAction::Redo).is_available());
    e.session.live.as_mut().unwrap().preview_error = Some("invalid incoming preview".into());
    refused(&mut e, HistoryAction::Redo, HistoryRefusal::ResumePreview);
    let mut e = editor();
    moved(&mut e);
    e.dispatch(Command::Undo).unwrap();
    e.session.live.as_mut().unwrap().action_frame = None;
    let reason = match e.history_eligibility(HistoryAction::Redo).availability {
        HistoryAvailability::Refused(reason @ HistoryRefusal::ActiveExtrudeSource(_)) => reason,
        other => panic!("unexpected {other:?}"),
    };
    refused(&mut e, HistoryAction::Redo, reason);
}
#[test]
fn checked_tickets_remain_authoritative_after_query() {
    let mut e = editor();
    let ticket = e.identity_ticket().unwrap();
    let _ = e.history_eligibility(HistoryAction::Undo);
    e.dispatch(Command::Rename("new publication".into()))
        .unwrap();
    let before = e.clone();
    let next = e.identities.next_id().unwrap();
    assert!(e.dispatch_with_ticket(ticket, Command::Undo).is_err());
    assert!(e == before);
    assert_eq!(e.identities.next_id().unwrap(), next);
}

#[test]
fn prepared_native_menu_fixture_decodes_to_the_exact_original_g0() {
    let decoded = persistence::decode(include_bytes!(
        "../../../docs/post-i2-stab01a-20261026/native-source.opscene"
    ))
    .unwrap();
    assert_eq!(decoded, editor().scene);
}
