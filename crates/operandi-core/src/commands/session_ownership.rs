//! Borrowed observations of existing task storage, not another writable owner.
//! Raw claims deliberately remain visible when inconsistent. An active I2 stage
//! and its Live mirror share one history resolver; parked/prior stages do not
//! become extra active tasks. No generic exclusivity gate is installed here.
use crate::{
    Editor, ExtrudeStage, HistoryAction, Live, LiveExtrudeHistory, NumericCone, NumericEllipsoid,
    NumericReduction, NumericShape, NumericSpike, Scene, Tool,
};

#[derive(Clone, Copy)]
pub(crate) struct SessionOwnership<'a> {
    pub(crate) bridge: bool,
    pub(crate) live: Option<&'a Live>,
    pub(crate) cone: Option<&'a NumericCone>,
    pub(crate) ellipsoid: Option<&'a NumericEllipsoid>,
    pub(crate) shape: Option<&'a NumericShape>,
    pub(crate) spike: Option<&'a NumericSpike>,
    pub(crate) reduction: Option<&'a NumericReduction>,
    /// Borrow the full pair of slots: prior is retained even for active or
    /// malformed selected states, rather than erased by a normalized enum.
    pub(crate) extrusion: &'a LiveExtrudeHistory,
    pub(crate) selection: Option<&'a Scene>,
}
#[derive(Clone, Copy)]
pub(crate) struct HistoryPosition {
    undo_len: usize,
    has_completed_redo: bool,
}
impl HistoryPosition {
    pub(crate) fn of(editor: &Editor) -> Self {
        Self {
            undo_len: editor.history.undo.len(),
            has_completed_redo: !editor.history.redo.is_empty(),
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum HistoryRoute<'a> {
    LocalExtrude(&'a ExtrudeStage),
    Ordinary,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LocalResumeObstacle {
    IncomingTaskOrSelection,
    LivePreview,
}
impl Editor {
    /// Inspect only; no synchronization, snapshots, validation or reservations.
    pub(crate) fn session_ownership(&self) -> SessionOwnership<'_> {
        SessionOwnership {
            bridge: self.session.bridge.is_some(),
            live: self.session.live.as_ref(),
            cone: self.session.cone.as_ref(),
            ellipsoid: self.session.ellipsoid.as_ref(),
            shape: self.session.shape.as_ref(),
            spike: self.session.spike.as_ref(),
            reduction: self.session.reduction.as_ref(),
            extrusion: &self.i2,
            selection: self.selection_start.as_deref(),
        }
    }
}
impl<'a> SessionOwnership<'a> {
    /// Resolver selection only. The existing source/context guard still decides
    /// whether local Redo can execute. Missing mirrors do not block local Undo.
    pub(crate) fn history_route(
        &self,
        action: HistoryAction,
        position: HistoryPosition,
    ) -> HistoryRoute<'a> {
        if let Some(stage) = self.extrusion.selected.as_ref()
            && (stage.cursor > 0
                || (action == HistoryAction::Redo
                    && stage.anchor == position.undo_len
                    && !position.has_completed_redo))
        {
            HistoryRoute::LocalExtrude(stage)
        } else {
            HistoryRoute::Ordinary
        }
    }
    /// Keep current incoming-owner precedence, without changing any session.
    /// Context/falloff and the parked basis are checked by the surrounding guard.
    pub(crate) fn local_resume_obstacle(&self) -> Option<LocalResumeObstacle> {
        if self.bridge
            || self.cone.is_some()
            || self.spike.is_some()
            || self.ellipsoid.is_some()
            || self.shape.is_some()
            || self.shape.is_some()
            || self.reduction.is_some()
            || self.selection.is_some()
        {
            Some(LocalResumeObstacle::IncomingTaskOrSelection)
        } else if self.live.is_some_and(|live| {
            live.applied
                || live.preview_error.is_some()
                || !matches!(live.tool, Tool::Move | Tool::Extrude)
        }) {
            Some(LocalResumeObstacle::LivePreview)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn editor() -> Editor {
        let mut editor = Editor {
            scene: serde_json::from_str(include_str!(
                "../../../../docs/i2/expectations/graphs/G0.json"
            ))
            .unwrap(),
            ..Default::default()
        };
        editor.synchronize_identities().unwrap();
        editor
    }
    fn begin(editor: &mut Editor) {
        editor.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        editor
            .dispatch(Command::ConfigureExtrude(Parameters {
                extrude_mode: ExtrudeMode::Shift,
                extrude_uvs: ExtrudeUvs::None,
                ..Default::default()
            }))
            .unwrap();
        editor.dispatch(Command::BeginExtrude).unwrap();
    }
    fn parked() -> Editor {
        let mut editor = editor();
        begin(&mut editor);
        editor.dispatch(Command::Undo).unwrap();
        editor
    }
    #[test]
    fn observations_borrow_raw_claims_and_alias_the_active_i2_mirror() {
        let mut e = editor();
        let view = e.session_ownership();
        assert!(
            view.live.is_none()
                && view.cone.is_none()
                && view.ellipsoid.is_none()
                && view.spike.is_none()
                && view.selection.is_none()
        );
        assert!(matches!(
            view.history_route(HistoryAction::Undo, HistoryPosition::of(&e)),
            HistoryRoute::Ordinary
        ));
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        assert!(std::ptr::eq(
            e.session_ownership().live.unwrap(),
            e.session.live.as_ref().unwrap()
        ));
        e.dispatch(Command::Cancel).unwrap();
        begin(&mut e);
        let view = e.session_ownership();
        assert!(std::ptr::eq(view.extrusion, &e.i2));
        assert!(std::ptr::eq(
            view.live.unwrap(),
            e.session.live.as_ref().unwrap()
        ));
        let HistoryRoute::LocalExtrude(stage) =
            view.history_route(HistoryAction::Undo, HistoryPosition::of(&e))
        else {
            panic!("active I2 must own local Undo")
        };
        assert!(std::ptr::eq(stage, e.i2.selected.as_ref().unwrap()));
        // One resolver is selected; the mirrored Live remains a raw observation.
        assert_eq!(stage.cursor, 1);
        assert_eq!(view.live.unwrap().tool, Tool::Extrude);
        e.session.live = None;
        assert!(e.session_ownership().live.is_none());
        assert!(matches!(
            e.session_ownership()
                .history_route(HistoryAction::Undo, HistoryPosition::of(&e)),
            HistoryRoute::LocalExtrude(_)
        ));
    }
    #[test]
    fn prior_and_orphan_slots_are_retained_without_normalization() {
        let mut e = parked();
        let prior_key = e.i2.selected.as_ref().unwrap().key;
        begin(&mut e);
        assert_eq!(e.i2.prior.as_ref().unwrap().key, prior_key);
        let view = e.session_ownership();
        assert!(std::ptr::eq(
            view.extrusion.prior.as_ref().unwrap(),
            e.i2.prior.as_ref().unwrap()
        ));
        assert!(matches!(
            view.history_route(HistoryAction::Undo, HistoryPosition::of(&e)),
            HistoryRoute::LocalExtrude(_)
        ));
        // Diagnostic state: preserve even a noncanonical parked+prior or orphan.
        e.i2.selected.as_mut().unwrap().cursor = 0;
        assert_eq!(
            e.session_ownership().extrusion.prior.as_ref().unwrap().key,
            prior_key
        );
        e.i2.selected = None;
        let before = e.clone();
        let view = e.session_ownership();
        assert_eq!(view.extrusion.prior.as_ref().unwrap().key, prior_key);
        assert!(matches!(
            view.history_route(HistoryAction::Redo, HistoryPosition::of(&e)),
            HistoryRoute::Ordinary
        ));
        assert!(e == before);
    }
    #[test]
    fn parked_incoming_claims_and_overlaps_stay_observable() {
        let mut e = parked();
        e.dispatch(Command::ArmCone).unwrap();
        let view = e.session_ownership();
        assert!(view.cone.is_some() && view.extrusion.selected.as_ref().unwrap().cursor == 0);
        assert_eq!(
            view.local_resume_obstacle(),
            Some(LocalResumeObstacle::IncomingTaskOrSelection)
        );
        e.dispatch(Command::Cancel).unwrap();
        e.dispatch(Command::BeginSelection).unwrap();
        let view = e.session_ownership();
        assert!(std::ptr::eq(
            view.selection.unwrap(),
            e.selection_start.as_deref().unwrap()
        ));
        assert_eq!(
            view.local_resume_obstacle(),
            Some(LocalResumeObstacle::IncomingTaskOrSelection)
        );
        e.dispatch(Command::EndSelection { cancel: true }).unwrap();
        let target = e.scene.selection.item.unwrap();
        e.session.cone = Some(NumericCone::armed(target, Default::default()));
        e.session.ellipsoid = Some(NumericEllipsoid::armed(target, Default::default()));
        e.session.spike = Some(NumericSpike::armed(target, 0.5));
        let before = e.clone();
        let view = e.session_ownership();
        assert!(view.cone.is_some() && view.ellipsoid.is_some() && view.spike.is_some());
        assert_eq!(
            view.local_resume_obstacle(),
            Some(LocalResumeObstacle::IncomingTaskOrSelection)
        );
        assert!(e == before);
    }
    #[test]
    fn route_uses_cursor_anchor_and_completed_redo_without_validating_mirrors() {
        let mut e = parked();
        for undo_len in [0, 1, 2] {
            for has_completed_redo in [false, true] {
                for action in [HistoryAction::Undo, HistoryAction::Redo] {
                    let position = HistoryPosition {
                        undo_len,
                        has_completed_redo,
                    };
                    let local = matches!(
                        e.session_ownership().history_route(action, position),
                        HistoryRoute::LocalExtrude(_)
                    );
                    assert_eq!(
                        local,
                        action == HistoryAction::Redo && undo_len == 0 && !has_completed_redo
                    );
                }
            }
        }
        for cursor in [1, 2] {
            e.i2.selected.as_mut().unwrap().cursor = cursor;
            e.session.live = None;
            for action in [HistoryAction::Undo, HistoryAction::Redo] {
                assert!(matches!(
                    e.session_ownership().history_route(
                        action,
                        HistoryPosition {
                            undo_len: 99,
                            has_completed_redo: true
                        }
                    ),
                    HistoryRoute::LocalExtrude(_)
                ));
            }
        }
    }
    #[test]
    fn incoming_owner_obstacles_keep_order_and_clean_armed_move_eligibility() {
        for tool in [Tool::Move, Tool::Extrude, Tool::Bevel, Tool::Rotate] {
            let mut e = parked();
            e.dispatch(Command::Activate(tool)).unwrap();
            let expected = if matches!(tool, Tool::Move | Tool::Extrude) {
                None
            } else {
                Some(LocalResumeObstacle::LivePreview)
            };
            assert_eq!(e.session_ownership().local_resume_obstacle(), expected);
            e.session.live.as_mut().unwrap().preview_error = Some("invalid".into());
            assert_eq!(
                e.session_ownership().local_resume_obstacle(),
                Some(LocalResumeObstacle::LivePreview)
            );
            e.selection_start = Some(Box::new(e.scene.clone()));
            assert_eq!(
                e.session_ownership().local_resume_obstacle(),
                Some(LocalResumeObstacle::IncomingTaskOrSelection)
            );
            e.selection_start = None;
            e.session.live.as_mut().unwrap().preview_error = None;
            e.session.live.as_mut().unwrap().applied = true;
            assert_eq!(
                e.session_ownership().local_resume_obstacle(),
                Some(LocalResumeObstacle::LivePreview)
            );
        }
    }
}
