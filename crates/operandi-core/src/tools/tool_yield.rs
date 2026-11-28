//! UIC-307 (Jason 2026-11-27, GitHub #36): tools aren't modal. A command
//! from a panel, menu or shortcut that refuses to run beside an open tool
//! ("Drop or Cancel the active tool first") runs anyway: the tool's work
//! settles as its own undo step (nothing, if it changed nothing), the
//! command runs as the next step, and the same tool starts again on what
//! is selected now. A command that made a new mesh leaves the tool on it.
//! Everything runs on a staged copy, so a failure anywhere leaves tool,
//! scene and history as they were and reports the command's own refusal.
use crate::{ActiveTask, Command, Editor, Result};

/// The refusals `require_idle` gives. Only these turn into a settle-and-run;
/// a command's other refusals (no selection, wrong mode) stand.
pub(crate) fn idle_refusal(message: &str) -> bool {
    matches!(
        message,
        "Drop or Cancel the active tool first"
            | "Drop or Cancel Bridge first"
            | "Drop or Cancel Polygon Reduction first"
    )
}

impl Editor {
    /// What starts the open task again after an outside command, or `None`
    /// when it can't be set aside now: nothing open, a drag or selection
    /// gesture held, a preset row (its Drop restores remembered context),
    /// or a Pen or Solid Sketch draft (drafts finish or cancel themselves).
    pub(crate) fn restart_command(&self) -> Option<Command> {
        if self.session.tweak_held
            || self.selection_start.is_some()
            || self.session.preset.is_some()
        {
            return None;
        }
        Some(match self.session.active_task()? {
            ActiveTask::Live(tool) => Command::Activate(tool),
            ActiveTask::Cone => Command::ArmCone,
            ActiveTask::Ellipsoid => Command::ArmEllipsoid,
            ActiveTask::Shape(kind) => Command::ArmShape(kind),
            ActiveTask::Spike => Command::ArmSpike,
            ActiveTask::Bridge => Command::ArmBridge,
            ActiveTask::Reduction => Command::ArmReduction,
            ActiveTask::Pen | ActiveTask::SolidSketch => return None,
        })
    }

    /// Settle the open task, run `cmd`, start the task again with `restart`.
    /// If the task can't start again (nothing selected now, the new item
    /// isn't a mesh) the command still stands and the tool stays dropped,
    /// as after picking it with nothing to work on.
    pub(crate) fn yield_tool_for(&mut self, restart: Command, cmd: Command) -> Result<()> {
        let mut staged = Box::new(self.clone());
        staged.dispatch(Command::Drop)?;
        if staged.session.has_active_task() || staged.i2_active() {
            return Err("Finish the current tool first".into());
        }
        staged.dispatch(cmd)?;
        let _ = staged.dispatch(restart);
        *self = *staged;
        Ok(())
    }
}
