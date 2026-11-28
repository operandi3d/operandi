//! Each Move/Rotate/Scale/Transform drag is its own undo step, and Undo/Redo
//! keep the tool active (Jason, 2026-11-03). Original test data.
use glam::DVec3;
use operandi_core::*;

fn fixture() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0];
    // Start from an empty history (New Mesh is a step of its own).
    e.history.undo.clear();
    e
}
fn point(e: &Editor) -> DVec3 {
    e.scene.active().unwrap().mesh.vertices[0]
}
/// One drag: a preview with this offset, then the release.
fn drag(e: &mut Editor, delta: DVec3) {
    let p = Parameters {
        delta,
        ..e.session.live.as_ref().unwrap().params.clone()
    };
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::CommitTransformStep).unwrap();
}
fn tool(e: &Editor) -> Option<Tool> {
    e.session.live.as_ref().map(|l| l.tool)
}

#[test]
fn each_drag_is_one_step_and_undo_keeps_the_tool() {
    let mut e = fixture();
    let start = point(&e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let center = e
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .unwrap()
        .center;
    drag(&mut e, DVec3::X);
    drag(&mut e, DVec3::Y);
    assert_eq!(point(&e), start + DVec3::new(1., 1., 0.));
    // Each drag is a tweak of the Move session, not a document step yet
    // (Jason, 2026-11-04); Drop records the session as one step.
    assert!(e.history.undo.is_empty());
    assert!(e.session.tweaks.can_undo());
    // The next drag starts from zero, with the handle on the moved point.
    let live = e.session.live.as_ref().unwrap();
    assert_eq!(live.params.delta, DVec3::ZERO);
    assert_eq!(
        live.action_frame.unwrap().center,
        center + DVec3::new(1., 1., 0.)
    );

    assert!(e.history_eligibility(HistoryAction::Undo).is_available());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(point(&e), start + DVec3::X);
    assert_eq!(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        center + DVec3::X
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(point(&e), start);

    e.dispatch(Command::Redo).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(point(&e), start + DVec3::X);
    // Dragging again after Redo continues from the redone scene.
    drag(&mut e, DVec3::Z);
    assert_eq!(point(&e), start + DVec3::new(1., 0., 1.));
    assert!(e.history.redo.is_empty());
}

#[test]
fn empty_history_undo_closes_the_tool_and_keeps_the_scene() {
    let mut e = fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let before = e.scene.clone();
    // A click without movement records nothing.
    e.dispatch(Command::CommitTransformStep).unwrap();
    assert!(e.history.undo.is_empty());
    // Undo undoes the activation (MU-MID-03): Move closes, nothing moves.
    assert!(e.history_eligibility(HistoryAction::Undo).is_available());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(tool(&e), None);
    assert_eq!(e.scene, before);
}

#[test]
fn pending_field_edit_is_undone_and_redone_as_a_step() {
    let mut e = fixture();
    let start = point(&e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    // A typed offset, not yet released by a drag.
    let p = Parameters {
        delta: DVec3::X * 2.,
        ..e.session.live.as_ref().unwrap().params.clone()
    };
    e.dispatch(Command::Preview(p)).unwrap();
    let typed = point(&e);
    let eligibility = e.history_eligibility(HistoryAction::Undo);
    assert!(eligibility.is_available());
    assert_eq!(eligibility.label, Some("Move"));
    assert!(!e.history_eligibility(HistoryAction::Redo).is_available());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(point(&e), start);
    assert_eq!(tool(&e), Some(Tool::Move));
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(point(&e), typed);
    assert_eq!(tool(&e), Some(Tool::Move));
}

#[test]
fn drop_after_drags_adds_no_extra_step() {
    let mut e = fixture();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag(&mut e, DVec3::X);
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none());
    assert_eq!(e.history.undo.len(), 1);
    // Ordinary Undo after Drop still works and leaves no tool behind.
    e.dispatch(Command::Undo).unwrap();
    assert!(e.session.live.is_none());
}

#[test]
fn cancel_discards_the_whole_session() {
    let mut e = fixture();
    let start = point(&e);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag(&mut e, DVec3::X);
    let p = Parameters {
        delta: DVec3::Y,
        ..e.session.live.as_ref().unwrap().params.clone()
    };
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    // Cancel takes back the whole Move session, drags included.
    assert_eq!(point(&e), start);
    assert!(e.history.undo.is_empty());
}

#[test]
fn drop_records_all_drags_as_one_step() {
    let mut e = fixture();
    let start = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag(&mut e, DVec3::X);
    drag(&mut e, DVec3::Y);
    let moved = e.scene.clone();
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo[0].label, "Move");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, start);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, moved);
}

#[test]
fn rotate_and_scale_steps_reset_to_neutral() {
    for (t, delta, neutral) in [
        (Tool::Rotate, DVec3::new(0., 0., 90.), DVec3::ZERO),
        (Tool::Scale, DVec3::splat(2.), DVec3::ONE),
    ] {
        let mut e = fixture();
        e.scene.selection.vertices = vec![0, 1];
        let start = e.scene.clone();
        e.dispatch(Command::Activate(t)).unwrap();
        drag(&mut e, delta);
        let after = e.scene.clone();
        assert_ne!(after, start);
        assert_eq!(
            e.session.live.as_ref().unwrap().params.delta,
            neutral,
            "{t:?}"
        );
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(tool(&e), Some(t));
        assert_eq!(e.scene, start);
        e.dispatch(Command::Redo).unwrap();
        assert_eq!(e.scene, after);
        assert_eq!(tool(&e), Some(t));
    }
}

#[test]
fn item_mode_move_steps_undo_with_the_tool_live() {
    let mut e = fixture();
    e.scene.selection.mode = Mode::Item;
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag(&mut e, DVec3::X);
    let moved = e.scene.active().unwrap().transform.clone();
    assert_ne!(moved, Default::default());
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(tool(&e), Some(Tool::Move));
    assert_eq!(e.scene.active().unwrap().transform, Default::default());
}
