//! Structural route for Mode only. An attemptable route does not certify field
//! confirmation, Drop, geometry, identity validation or publication success.
use crate::{Command, Editor, Mode, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeRouteEligibility {
    /// Nothing is open: the mode changes at once.
    Direct,
    /// A tool is open. Changing mode drops it first, like picking another
    /// tool does (Jason, 2026-11-05): one keypress, two results.
    DropsToolFirst,
}
impl Editor {
    /// Pure observation, freshly repeated by execution. The target mode does
    /// not alter the route, even when it equals the current mode.
    pub fn mode_route_eligibility(&self) -> ModeRouteEligibility {
        if self.session.has_active_task() || self.i2_active() {
            ModeRouteEligibility::DropsToolFirst
        } else {
            ModeRouteEligibility::Direct
        }
    }
    pub(crate) fn dispatch_mode(&mut self, mode: Mode) -> Result<()> {
        match self.mode_route_eligibility() {
            ModeRouteEligibility::DropsToolFirst => {
                // The same rule as picking another tool: Drop keeps the open
                // tool's work as its own undo step, then the mode changes. Both
                // run on a staged copy, so a Drop that fails (a field error, a
                // Pen draft too short) leaves the tool, scene and history as
                // they were and reports why.
                let mut staged = self.clone();
                staged.dispatch(Command::Drop)?;
                if staged.session.has_active_task() || staged.i2_active() {
                    return Err("Finish the current tool before changing mode".into());
                }
                staged.dispatch(Command::Mode(mode))?;
                *self = staged;
                Ok(())
            }
            ModeRouteEligibility::Direct => {
                self.scene.selection.mode = mode;
                // Any mode button leaves a tag selection mode.
                self.scene.selection.tag = None;
                self.revision = self.revision.wrapping_add(1);
                Ok(())
            }
        }
    }
}
