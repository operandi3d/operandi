//! Read-only history routing shared by presentation and execution. This describes
//! existing category lifecycles; it does not unify their storage or sessions.
use crate::session_ownership::{HistoryPosition, HistoryRoute};
use crate::{Editor, Tool};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryAction {
    Undo,
    Redo,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryOwner {
    Document,
    ExtrudeStage,
    Cone,
    Ellipsoid,
    /// Torus, Capsule, Arc or Tube.
    Shape,
    Spike,
    Bridge,
    Pen,
    SolidSketch,
    Reduction,
    LiveTool(Tool),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryRefusal {
    NumericEllipsoid,
    NumericShape,
    /// Mid-tool undo (MTU-Q5, provisional): Redo would replace a preview
    /// that isn't a step yet.
    PendingPreview,
    PenRedo,
    ConeOtherOperation(HistoryAction),
    LiveTool(HistoryAction),
    /// RED-004/D1: a live Reduction task (armed or previewing) disables
    /// history until Drop or Cancel.
    Reduction(HistoryAction),
    NoLocalExtrudeRedo,
    ParkedAnchor,
    ParkedBasis,
    LocalMovement,
    ResumeContext,
    ResumeOwner,
    ResumePreview,
    ActiveExtrudeSource(String),
}
impl fmt::Display for HistoryRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PenRedo => "Finish or Cancel Pen before Redo",
            Self::PendingPreview => "Undo the preview, or Apply or Drop it, before Redo",
            Self::NumericEllipsoid => {
                "Drop or Cancel numeric Ellipsoid before changing selection/context or history"
            }
            Self::NumericShape => {
                "Drop or Cancel the primitive tool before changing selection/context or history"
            }
            Self::ConeOtherOperation(HistoryAction::Undo) => {
                "Drop or Cancel numeric Cone before undoing another operation"
            }
            Self::ConeOtherOperation(HistoryAction::Redo) => {
                "Drop or Cancel numeric Cone before redoing another operation"
            }
            Self::LiveTool(HistoryAction::Undo) => "Drop or Cancel the tool before Undo",
            Self::LiveTool(HistoryAction::Redo) => "Drop or Cancel the tool before Redo",
            Self::Reduction(HistoryAction::Undo) => "Drop or Cancel Polygon Reduction before Undo",
            Self::Reduction(HistoryAction::Redo) => "Drop or Cancel Polygon Reduction before Redo",
            Self::NoLocalExtrudeRedo => "No local extrusion Redo",
            Self::ParkedAnchor => "Return to the parked anchor before local Redo",
            Self::ParkedBasis => "Parked stage anchor changed",
            Self::LocalMovement => "No local movement to Redo",
            Self::ResumeContext => "Local Extrude Redo requires None/World and no spatial falloff",
            Self::ResumeOwner => "Another tool owns this task",
            Self::ResumePreview => "Drop or Cancel the current preview before local Redo",
            Self::ActiveExtrudeSource(reason) => reason,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryAvailability {
    Available,
    /// Disabled presentation, successful execution with no history step. Existing
    /// category cleanup/publication effects are preserved by dispatch.
    EmptyHistory,
    Refused(HistoryRefusal),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEligibility<'a> {
    pub owner: HistoryOwner,
    pub availability: HistoryAvailability,
    /// Entry label without Undo/Redo prefix or presentation shortcut.
    pub label: Option<&'a str>,
}
impl HistoryEligibility<'_> {
    pub fn is_available(&self) -> bool {
        self.availability == HistoryAvailability::Available
    }
    /// Execution re-queries current state before calling this. Empty history is
    /// intentionally not a refusal, even though a menu disables it.
    pub(crate) fn permit_reported(
        &self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> crate::Result<()> {
        self.permit().inspect_err(|_| {
            if let (Some(report), HistoryAvailability::Refused(reason)) =
                (report, &self.availability)
            {
                report.refusal = Some(reason.clone());
            }
        })
    }
    pub(crate) fn permit(&self) -> crate::Result<()> {
        match &self.availability {
            HistoryAvailability::Refused(reason) => Err(reason.to_string()),
            _ => Ok(()),
        }
    }
}
impl Editor {
    /// Inspect only: no dispatch, synchronization, reservations or history copies.
    /// The returned decision is descriptive, never a permit for later execution.
    pub fn history_eligibility(&self, action: HistoryAction) -> HistoryEligibility<'_> {
        // Solid Sketch keeps its own edit steps (solid_sketch_editor.rs);
        // with none left Undo closes the tool.
        if let Some(session) = &self.session.solid_sketch {
            let available = match action {
                HistoryAction::Undo => true,
                HistoryAction::Redo => session.can_redo(),
            };
            return HistoryEligibility {
                owner: HistoryOwner::SolidSketch,
                availability: if available {
                    HistoryAvailability::Available
                } else {
                    HistoryAvailability::EmptyHistory
                },
                label: available.then_some("Solid Sketch"),
            };
        }
        if let Some(draft) = &self.session.pen_draft {
            return HistoryEligibility {
                owner: HistoryOwner::Pen,
                availability: match action {
                    HistoryAction::Undo if !draft.points().is_empty() => {
                        HistoryAvailability::Available
                    }
                    HistoryAction::Undo => HistoryAvailability::EmptyHistory,
                    HistoryAction::Redo => HistoryAvailability::Refused(HistoryRefusal::PenRedo),
                },
                label: (action == HistoryAction::Undo && !draft.points().is_empty())
                    .then_some("Pen Point"),
            };
        }
        if self.session.reduction.is_some() {
            return HistoryEligibility {
                owner: HistoryOwner::Reduction,
                availability: HistoryAvailability::Refused(HistoryRefusal::Reduction(action)),
                label: None,
            };
        }
        let stack = match action {
            HistoryAction::Undo => &self.history.undo,
            HistoryAction::Redo => &self.history.redo,
        };
        let label = stack.last().map(|entry| entry.label.as_str());
        // Redo of a tool's activation step, right after Undo closed it (U4).
        if action == HistoryAction::Redo
            && let Some(tool) = self.redo_reopens_tool()
        {
            return HistoryEligibility {
                owner: HistoryOwner::Document,
                availability: HistoryAvailability::Available,
                label: Some(tool),
            };
        }
        // Inside a tool: its own tweaks come first (Jason, 2026-11-04).
        if let Some(task) = self.tweak_session()
            && match action {
                HistoryAction::Undo => self.session.tweaks.can_undo(),
                HistoryAction::Redo => self.session.tweaks.can_redo(),
            }
        {
            return HistoryEligibility {
                owner: match task {
                    crate::ActiveTask::Live(tool) => HistoryOwner::LiveTool(tool),
                    crate::ActiveTask::Cone => HistoryOwner::Cone,
                    crate::ActiveTask::Ellipsoid => HistoryOwner::Ellipsoid,
                    _ => HistoryOwner::Shape,
                },
                availability: HistoryAvailability::Available,
                label: Some(
                    self.session
                        .tweaks
                        .top_name(action == HistoryAction::Redo)
                        .unwrap_or(task_label(task)),
                ),
            };
        }
        // With nothing of its own left, Undo closes the tool (MU-MID-02/03).
        if action == HistoryAction::Undo
            && self.undo_closes_tool()
            && let Some(task) = self.mid_undo_task()
            && !matches!(
                task,
                crate::ActiveTask::Ellipsoid | crate::ActiveTask::Shape(_)
            )
        {
            return HistoryEligibility {
                owner: match task {
                    crate::ActiveTask::Live(tool) => HistoryOwner::LiveTool(tool),
                    crate::ActiveTask::Cone => HistoryOwner::Cone,
                    crate::ActiveTask::Spike => HistoryOwner::Spike,
                    _ => HistoryOwner::Bridge,
                },
                availability: HistoryAvailability::Available,
                label: Some(task_label(task)),
            };
        }
        // Document Redo would replace the tool's unrecorded tweaks.
        if action == HistoryAction::Redo
            && let Some(task) = self.tweak_session()
            && self.session.tweaks.can_undo()
            && !self.history.redo.is_empty()
        {
            return HistoryEligibility {
                owner: match task {
                    crate::ActiveTask::Live(tool) => HistoryOwner::LiveTool(tool),
                    crate::ActiveTask::Cone => HistoryOwner::Cone,
                    crate::ActiveTask::Ellipsoid => HistoryOwner::Ellipsoid,
                    _ => HistoryOwner::Shape,
                },
                availability: HistoryAvailability::Refused(HistoryRefusal::PendingPreview),
                label,
            };
        }
        let ownership = self.session_ownership();
        if let HistoryRoute::LocalExtrude(stage) =
            ownership.history_route(action, HistoryPosition::of(self))
        {
            let result = match action {
                HistoryAction::Undo => Ok(()),
                HistoryAction::Redo => self.i2_redo_eligibility(),
            };
            let label = if result.is_ok() {
                Some(match (action, stage.cursor) {
                    (HistoryAction::Undo, 2) | (HistoryAction::Redo, 1) => "Move Extrude",
                    _ => "Begin Extrude",
                })
            } else {
                label
            };
            return HistoryEligibility {
                owner: HistoryOwner::ExtrudeStage,
                availability: match result {
                    Ok(()) => HistoryAvailability::Available,
                    Err(reason) => HistoryAvailability::Refused(reason),
                },
                label,
            };
        }
        let mut owner = HistoryOwner::Document;
        // Mid-tool undo for the primitive tools (MTU-Q1..Q6, provisional):
        // Undo first discards a preview that isn't a step yet, then steps
        // back with the tool still active; Redo waits until no preview shows.
        let primitive = self.primitive_session();
        if let Some(session) = primitive {
            owner = session.owner;
        }
        let preview = primitive.and_then(|s| s.pending_preview);
        let refusal = if primitive.is_some() {
            (action == HistoryAction::Redo && preview.is_some())
                .then_some(HistoryRefusal::PendingPreview)
        } else if ownership.reduction.is_some() {
            owner = HistoryOwner::Reduction;
            Some(HistoryRefusal::Reduction(action))
        } else if let Some(live) = ownership.live
            && !(action == HistoryAction::Undo
                && (matches!(live.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                    || live.tool.is_smooth_shift())
                && !live.applied)
            // Transform tools keep running through Undo/Redo; a pending
            // (unrecorded) change can be undone but not redone over.
            && !(live.tool.is_transform()
                && live.preview_error.is_none()
                && (action == HistoryAction::Undo || self.scene == live.base))
        {
            owner = HistoryOwner::LiveTool(live.tool);
            Some(HistoryRefusal::LiveTool(action))
        } else {
            owner = if let Some(live) = ownership.live.filter(|l| l.tool.is_transform()) {
                HistoryOwner::LiveTool(live.tool)
            } else if ownership.bridge {
                HistoryOwner::Bridge
            } else if ownership.cone.is_some() {
                HistoryOwner::Cone
            } else if ownership.spike.is_some() {
                HistoryOwner::Spike
            } else {
                owner
            };
            None
        };
        // Undo of a transform tool's pending change: it becomes its own
        // step, so there is something to undo even with an empty stack.
        let pending = ownership.live.filter(|live| {
            action == HistoryAction::Undo && live.tool.is_transform() && self.scene != live.base
        });
        let label = pending.map_or(label, |live| Some(live.tool.label()));
        let pending_preview = action == HistoryAction::Undo && preview.is_some();
        let label = if pending_preview { preview } else { label };
        // With none of the tool's own steps left, Undo closes the tool
        // (MU-MID-02/03): there is something to undo even with no stack.
        let closes_tool = action == HistoryAction::Undo && self.undo_closes_tool();
        let label = match self.mid_undo_task() {
            Some(task) if closes_tool && pending.is_none() && !pending_preview => {
                Some(task_label(task))
            }
            _ => label,
        };
        HistoryEligibility {
            owner,
            availability: if let Some(reason) = refusal {
                HistoryAvailability::Refused(reason)
            } else if stack.is_empty() && pending.is_none() && !pending_preview && !closes_tool {
                HistoryAvailability::EmptyHistory
            } else {
                HistoryAvailability::Available
            },
            label,
        }
    }
}

/// The active primitive tool, for mid-tool undo.
#[derive(Clone, Copy)]
pub(crate) struct PrimitiveSession {
    pub(crate) owner: HistoryOwner,
    /// The tool's name when it shows a preview that isn't a step yet.
    pub(crate) pending_preview: Option<&'static str>,
}
impl Editor {
    /// Cube/Sphere/Cylinder (drawn, `live`) or a numeric primitive session
    /// (Ellipsoid, Cone, Torus/Capsule/Arc/Tube).
    pub(crate) fn primitive_session(&self) -> Option<PrimitiveSession> {
        let pending = |shown: bool, name: &'static str| shown.then_some(name);
        if let Some(s) = &self.session.ellipsoid {
            return Some(PrimitiveSession {
                owner: HistoryOwner::Ellipsoid,
                pending_preview: pending(s.preview.is_some(), "Ellipsoid"),
            });
        }
        if let Some(s) = &self.session.shape {
            return Some(PrimitiveSession {
                owner: HistoryOwner::Shape,
                pending_preview: pending(s.preview.is_some(), s.kind().label()),
            });
        }
        if self.session.cone.is_some() {
            // Cone's value changes are undo steps of their own (MU-MID-01),
            // so its preview is never unrecorded.
            return Some(PrimitiveSession {
                owner: HistoryOwner::Cone,
                pending_preview: None,
            });
        }
        let live = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool.is_primitive())?;
        Some(PrimitiveSession {
            owner: HistoryOwner::LiveTool(live.tool),
            pending_preview: pending(live.base != self.scene, live.tool.label()),
        })
    }
}

pub(crate) fn task_label(task: crate::ActiveTask) -> &'static str {
    match task {
        crate::ActiveTask::Live(tool) => tool.label(),
        crate::ActiveTask::Cone => "Cone",
        crate::ActiveTask::Ellipsoid => "Ellipsoid",
        crate::ActiveTask::Shape(kind) => kind.label(),
        crate::ActiveTask::Spike => "Spike",
        crate::ActiveTask::Bridge => "Bridge",
        crate::ActiveTask::Reduction => "Polygon Reduction",
        crate::ActiveTask::Pen => "Pen",
        crate::ActiveTask::SolidSketch => "Solid Sketch",
    }
}
