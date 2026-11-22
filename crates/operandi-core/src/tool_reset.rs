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
        self.active_task()
            .map(|task| format!("Reset {}", crate::history_policy::task_label(task)))
    }

    /// MN-05.1: the active tool's values back to the factory.
    pub(crate) fn reset_tool_attributes(&mut self) -> Result<()> {
        let task = self
            .active_task()
            .ok_or("No tool is active, so there is nothing to reset")?;
        let own = match task {
            ActiveTask::Cone => {
                let mut staged = self.clone();
                staged.cone_preset = Default::default();
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
        let live = self.live.as_ref().ok_or("No active tool")?;
        let tool = live.tool;
        let mut probe = self.clone();
        probe.forget_tool_memory();
        // Start it outright (no Click to Start arming) from the scene the
        // tool works on, as a fresh pick would.
        probe.auto_activate_flags = false;
        probe.armed = None;
        probe.scene = live.base.clone();
        probe.live = None;
        probe.dispatch_inner(Command::Activate(tool), &mut None)?;
        let factory = probe
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
            let l = self.live.as_mut().unwrap();
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
        self.smooth_shift_preset = fresh.smooth_shift_preset;
        self.chamfer_preset = fresh.chamfer_preset;
        self.edge_bevel_preset = fresh.edge_bevel_preset;
        self.edge_chamfer_preset = fresh.edge_chamfer_preset;
        self.poly_haul_preset = fresh.poly_haul_preset;
        self.sketch_precision_preset = fresh.sketch_precision_preset;
        self.vertex_merge_preset = fresh.vertex_merge_preset;
        self.bend_preset = fresh.bend_preset;
        self.thicken_preset = fresh.thicken_preset;
        self.bridge_preset = fresh.bridge_preset;
        self.ellipsoid_preset = fresh.ellipsoid_preset;
        self.shape_presets = fresh.shape_presets;
        self.cone_preset = fresh.cone_preset;
        self.extrude_preset = fresh.extrude_preset;
        self.cube_preset = fresh.cube_preset;
        self.slice_preset = fresh.slice_preset;
        self.subdivide_preset = fresh.subdivide_preset;
        self.unsubdivide_preset = fresh.unsubdivide_preset;
        self.freeze_preset = fresh.freeze_preset;
        self.duplicate_preset = fresh.duplicate_preset;
        self.detriangulate_preset = fresh.detriangulate_preset;
        self.relax_mesh_preset = fresh.relax_mesh_preset;
        self.uv_unwrap_preset = fresh.uv_unwrap_preset;
        self.uv_atlas_preset = fresh.uv_atlas_preset;
        self.uv_pack_preset = fresh.uv_pack_preset;
        self.retopo_preset = fresh.retopo_preset;
        self.voxel_remesh_preset = fresh.voxel_remesh_preset;
        self.octree_remesh_preset = fresh.octree_remesh_preset;
        self.boolean_preset = fresh.boolean_preset;
        self.solid_drill_preset = fresh.solid_drill_preset;
        self.axis_drill_preset = fresh.axis_drill_preset;
    }
}
