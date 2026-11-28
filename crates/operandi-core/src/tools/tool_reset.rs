//! Reset Tool Attributes for every tool (MN-05.1, MN-05.4).
//!
//! `Command::ResetToolAttributes` puts the active tool's Tool Properties
//! values back to the factory ones; the tool stays active and its live
//! result updates at once. Operandi has no per-tool saved default yet, so
//! "factory" is what the tool starts with on a fresh launch (MN-05.1's
//! "user's saved default" has nothing to read today).
//!
//! Tools that already had a reset of their own (Bevel, Extrude, Smooth
//! Shift, the primitives, Bridge, Spike ...) keep it, so their panels and
//! sessions reset exactly as before. Every other live tool is reset by
//! starting it again on a throwaway copy of the editor with its remembered
//! last-use values forgotten, and taking that copy's starting values.
//!
//! The change is one tweak of the tool's own undo history, named
//! "Reset <tool>" (`mid_tool_undo.rs`).
use crate::{ActiveTask, Command, Editor, Result, Tool};

impl Editor {
    /// The undo-step name a reset of the active tool gets (MN-05.4).
    pub(crate) fn reset_step_label(&self) -> Option<String> {
        self.session
            .active_task()
            .map(|task| format!("Reset {}", crate::history_policy::task_label(task)))
    }

    /// MN-05.1: the active tool's values back to the factory.
    pub(crate) fn reset_tool_attributes(&mut self) -> Result<()> {
        let task = self
            .session
            .active_task()
            .ok_or("No tool is active, so there is nothing to reset")?;
        let own = match task {
            ActiveTask::Cone => {
                let mut staged = self.clone();
                staged.presets.forget::<crate::ConePreset>();
                staged.dispatch(Command::ConfigureCone(Default::default()))?;
                *self = staged;
                return Ok(());
            }
            ActiveTask::Ellipsoid => Command::ResetEllipsoid,
            ActiveTask::Shape(_) => Command::ResetShape,
            ActiveTask::Spike => Command::ResetSpike,
            ActiveTask::Bridge => Command::ResetBridge,
            ActiveTask::Reduction => Command::ConfigureReduction(Default::default()),
            ActiveTask::Pen | ActiveTask::SolidSketch => {
                return Err(format!(
                    "{} has no tool values to reset",
                    crate::history_policy::task_label(task)
                ));
            }
            ActiveTask::Live(tool) => match tool {
                Tool::Bevel => Command::ResetBevel,
                Tool::Extrude => Command::ResetExtrude,
                Tool::SmoothShift | Tool::Thicken => Command::ResetSmoothShift,
                Tool::PolyHaul => Command::ResetPolyHaul,
                Tool::SketchExtrude => Command::ResetSketchExtrude,
                Tool::VertexMerge => Command::ResetVertexMerge,
                Tool::Bend => Command::ResetBend,
                Tool::Box | Tool::Sphere | Tool::Cylinder => Command::ResetPrimitive,
                _ => return self.reset_live_to_factory(),
            },
        };
        self.dispatch(own)
    }

    /// Every other live tool: the values it starts with when nothing is
    /// remembered from an earlier use.
    fn reset_live_to_factory(&mut self) -> Result<()> {
        let live = self.session.live.as_ref().ok_or("No active tool")?;
        let tool = live.tool;
        let mut probe = self.clone();
        probe.forget_tool_memory();
        // Start it outright (no Click to Start arming) from the scene the
        // tool works on, as a fresh pick would.
        probe.auto_activate_flags = false;
        probe.session.armed = None;
        probe.scene = live.base.clone();
        probe.session.live = None;
        probe.dispatch_inner(Command::Activate(tool), &mut None)?;
        let factory = probe
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == tool)
            .ok_or_else(|| format!("{} could not be reset", tool.label()))?
            .params
            .clone();
        if factory == live.params {
            return Ok(());
        }
        if live.applied {
            self.dispatch(Command::Preview(factory))
        } else {
            let l = self.session.live.as_mut().unwrap();
            l.params = factory;
            l.preview_error = None;
            Ok(())
        }
    }

    /// Forget every tool's remembered last-use values (a fresh launch's).
    /// Falloff and Constrain to Background memories are tool context, not
    /// tool values, and stay.
    fn forget_tool_memory(&mut self) {
        let fresh = Editor::default();
        self.presets = fresh.presets;
    }
}
