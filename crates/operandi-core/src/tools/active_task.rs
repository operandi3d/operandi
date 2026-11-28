//! Which modeling task is open. The tool session keeps each kind of task in
//! its own slot (`live`, `cone`, `ellipsoid`, `spike`, `bridge`, `reduction`,
//! `pen_draft`); at most one is open at a time. Ask here instead of checking
//! the slots one by one, so every caller agrees on what "a tool is active"
//! means.
use crate::{Command, Editor, Tool};

/// The open task. `Live` covers every [`Tool`] that runs through the shared
/// live-tool path; the others are the numeric sessions and the Pen draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveTask {
    Live(Tool),
    Cone,
    Ellipsoid,
    /// Torus, Capsule, Arc or Tube.
    Shape(crate::ShapeKind),
    Spike,
    Bridge,
    Reduction,
    Pen,
    SolidSketch,
}

impl crate::ToolSession {
    /// The open task, if any.
    pub fn active_task(&self) -> Option<ActiveTask> {
        if let Some(live) = &self.live {
            Some(ActiveTask::Live(live.tool))
        } else if self.cone.is_some() {
            Some(ActiveTask::Cone)
        } else if self.ellipsoid.is_some() {
            Some(ActiveTask::Ellipsoid)
        } else if let Some(s) = &self.shape {
            Some(ActiveTask::Shape(s.kind()))
        } else if self.spike.is_some() {
            Some(ActiveTask::Spike)
        } else if self.bridge.is_some() {
            Some(ActiveTask::Bridge)
        } else if self.reduction.is_some() {
            Some(ActiveTask::Reduction)
        } else if self.pen_draft.is_some() {
            Some(ActiveTask::Pen)
        } else if self.solid_sketch.is_some() {
            Some(ActiveTask::SolidSketch)
        } else {
            None
        }
    }

    /// True while any modeling task is open.
    pub fn has_active_task(&self) -> bool {
        self.active_task().is_some()
    }

    /// How many task slots are filled. More than one breaks the rule that
    /// tasks are exclusive; tests check it stays at most one.
    pub fn open_task_slots(&self) -> usize {
        [
            self.live.is_some(),
            self.cone.is_some(),
            self.ellipsoid.is_some(),
            self.shape.is_some(),
            self.spike.is_some(),
            self.bridge.is_some(),
            self.reduction.is_some(),
            self.pen_draft.is_some(),
            self.solid_sketch.is_some(),
        ]
        .into_iter()
        .filter(|&open| open)
        .count()
    }
}

/// What a tool-starting command does while another tool is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolSwitch {
    /// The open tool's own button: drop it.
    Toggle,
    /// Another tool: drop the open one, then start the new one.
    Replace,
}

impl Editor {
    /// How `cmd` affects the open tool, or `None` when it starts no tool or
    /// nothing is open. Starting commands are Activate, every numeric Arm
    /// and Pen Start; "open" covers every task slot and an I2 Extrude stage.
    pub(crate) fn tool_switch(&self, cmd: &Command) -> Option<ToolSwitch> {
        let starts = matches!(
            cmd,
            Command::Activate(_)
                | Command::ActivateDeform(_)
                | Command::ActivatePreset(_)
                | Command::ArmCone
                | Command::ArmEllipsoid
                | Command::ArmShape(_)
                | Command::ArmSpike
                | Command::ArmBridge
                | Command::ArmReduction
                | Command::PenStart
                | Command::SolidSketchStart { .. }
        );
        if !starts {
            return None;
        }
        let open = self
            .session
            .active_task()
            .or_else(|| self.i2_active().then_some(ActiveTask::Live(Tool::Extrude)))?;
        let same = match (cmd, open) {
            (Command::Activate(t), ActiveTask::Live(open)) => {
                *t == open && self.session.preset.is_none()
            }
            (Command::ActivateDeform(p), ActiveTask::Live(_)) => {
                self.session.preset.as_ref().is_some_and(|s| s.id == p.id())
            }
            (Command::ActivatePreset(name), ActiveTask::Live(_)) => self
                .session
                .preset
                .as_ref()
                .is_some_and(|s| crate::tool_preset::row(name).is_some_and(|r| r.id == s.id)),
            (Command::ArmCone, ActiveTask::Cone)
            | (Command::ArmEllipsoid, ActiveTask::Ellipsoid)
            | (Command::ArmSpike, ActiveTask::Spike)
            | (Command::ArmBridge, ActiveTask::Bridge)
            | (Command::ArmReduction, ActiveTask::Reduction) => true,
            (Command::ArmShape(k), ActiveTask::Shape(open)) => *k == open,
            // Pen Start with a draft open keeps drawing (no-op in the Pen
            // route), as before.
            (Command::PenStart, ActiveTask::Pen) => return None,
            (Command::SolidSketchStart { .. }, ActiveTask::SolidSketch) => true,
            _ => false,
        };
        Some(if same {
            ToolSwitch::Toggle
        } else {
            ToolSwitch::Replace
        })
    }
}
