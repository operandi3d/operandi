//! Bounded completion facts for Undo/Redo/Cancel. These describe an executed
//! request, never authorize later input, and are not saved document state.
use crate::{Command, Editor, HistoryAction, HistoryRefusal};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryRequest {
    Undo,
    Redo,
    Cancel,
}
impl HistoryRequest {
    pub(crate) fn command(self) -> Command {
        match self {
            Self::Undo => Command::Undo,
            Self::Redo => Command::Redo,
            Self::Cancel => Command::Cancel,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryEffect {
    Empty,
    PenPointUndo,
    /// Mid-tool undo: a primitive's unrecorded preview was thrown away and
    /// the tool stays active (MTU-Q1, provisional).
    PreviewDiscarded {
        label: String,
    },
    /// Mid-tool undo: Undo of the tool's activation step closed the tool
    /// (MU-MID-02/03).
    ToolClosed {
        label: String,
    },
    /// Redo of that step opened the tool again (U4).
    ToolReopened {
        label: String,
    },
    /// Undo took back the active tool's last tweak; the tool stays.
    TweakUndone {
        label: String,
    },
    /// Redo put that tweak back.
    TweakRedone {
        label: String,
    },
    Completed {
        action: HistoryAction,
        label: String,
    },
    Local {
        action: HistoryAction,
        stage: u64,
        from: u8,
        to: u8,
    },
    LocalCancel {
        stage: u64,
        from: u8,
        restored_prior: Option<u64>,
    },
    ParkedDiscard,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistoryPublication {
    pub epoch: u64,
    pub publication: u64,
    pub request: u64,
}
impl HistoryPublication {
    fn of(editor: &Editor) -> Self {
        Self {
            epoch: editor.identities.epoch,
            publication: editor.identities.publication,
            request: editor.identities.request,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryCompletion {
    pub request: HistoryRequest,
    pub effect: HistoryEffect,
    pub authored_scene_changed: bool,
    pub foreground_closed: bool,
    pub publication: HistoryPublication,
}
impl HistoryCompletion {
    pub fn restored_local(&self) -> bool {
        matches!(
            self.effect,
            HistoryEffect::Local { .. } | HistoryEffect::LocalCancel { .. }
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryFailureReason {
    History(HistoryRefusal),
    Legacy(String),
}
impl fmt::Display for HistoryFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::History(reason) => reason.fmt(f),
            Self::Legacy(message) => f.write_str(message),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryFailure {
    pub reason: HistoryFailureReason,
    /// Only the diagnostic change actually retained by the existing error path.
    /// Numeric draft retention belongs to other command families.
    pub live_diagnostic_changed: bool,
    pub publication: HistoryPublication,
}
impl fmt::Display for HistoryFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.reason.fmt(f)
    }
}
impl std::error::Error for HistoryFailure {}

/// Request-local scalar/label facts only. Compatibility dispatch passes None,
/// so it neither captures owner state nor compares an additional Scene.
pub(crate) struct HistoryReport {
    request: HistoryRequest,
    pub(crate) effect: HistoryEffect,
    foreground_before: bool,
    pub(crate) authored_scene_changed: bool,
    pub(crate) refusal: Option<HistoryRefusal>,
    pub(crate) live_diagnostic_changed: bool,
}
impl HistoryReport {
    pub(crate) fn new(request: HistoryRequest, editor: &Editor) -> Self {
        Self {
            request,
            // Ordinary empty Undo/Redo are successful even without a step.
            // All other successful producers explicitly replace this effect.
            effect: HistoryEffect::Empty,
            foreground_before: foreground(editor),
            authored_scene_changed: false,
            refusal: None,
            live_diagnostic_changed: false,
        }
    }
    pub(crate) fn finish(self, editor: &Editor) -> HistoryCompletion {
        HistoryCompletion {
            request: self.request,
            effect: self.effect,
            authored_scene_changed: self.authored_scene_changed,
            foreground_closed: self.foreground_before && !foreground(editor),
            publication: HistoryPublication::of(editor),
        }
    }
    pub(crate) fn fail(self, editor: &Editor, message: String) -> HistoryFailure {
        HistoryFailure {
            reason: self.refusal.map_or(
                HistoryFailureReason::Legacy(message),
                HistoryFailureReason::History,
            ),
            live_diagnostic_changed: self.live_diagnostic_changed,
            publication: HistoryPublication::of(editor),
        }
    }
}
fn foreground(editor: &Editor) -> bool {
    let owner = editor.session_ownership();
    editor.session.pen_draft.is_some()
        || editor.session.solid_sketch.is_some()
        || owner.bridge
        || owner.live.is_some()
        || owner.cone.is_some()
        || owner.ellipsoid.is_some()
        || owner.shape.is_some()
        || owner.spike.is_some()
        || owner.reduction.is_some()
}
