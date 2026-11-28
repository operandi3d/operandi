//! Multiple active meshes, stage 3 (Jason, 2026-11-08): mesh-editing tools
//! and commands act on every active mesh, each with its own component
//! selection, as one undo step.
//!
//! Every single-mesh operation already works on "the primary item and the
//! components in the primary fields". Rather than give each one an item
//! parameter, the editor runs it once per target mesh with that mesh
//! standing in as the sole selected item (`Selection::sole`), then folds
//! the results back: each mesh's new components into its own entry, and
//! the history steps the runs recorded into one.
//!
//! Which meshes: with something of the current mode selected on any active
//! mesh, the meshes with a selection (a mesh with nothing selected is left
//! alone); with nothing selected anywhere, every active mesh, as a whole
//! (MAM-Q11, MAM-Q12). One mesh, the primary, takes the ordinary path.
use crate::{Command, Editor, Mode, Parameters, Result, Scene, Tool};

impl Scene {
    /// The meshes a mesh edit runs on, primary first, or `None` for just
    /// the primary (the single-mesh path). In Item mode every active mesh
    /// (each as a whole, as Item mode does with one mesh).
    pub fn mesh_edit_targets(&self) -> Option<Vec<u64>> {
        if self.selection.mode != Mode::Item {
            return self.component_transform_meshes();
        }
        let active = self.active_meshes();
        match active[..] {
            [] => None,
            [only] if Some(only) == self.selection.item => None,
            _ => Some(active),
        }
    }
    fn mesh_name(&self, id: u64) -> String {
        self.item(id)
            .map_or_else(|| format!("mesh {id}"), |i| i.name.clone())
    }
}

/// One-shot mesh edits that run on every active mesh (Batch A). Commands
/// that join, bridge or move geometry between items (Merge Items, Paste,
/// Weld with explicit point ids, Sync Vertex Order) stay on the primary.
pub(crate) fn runs_on_each_mesh(cmd: &Command) -> bool {
    use crate::{EdgeCommand, VertexCommand};
    match cmd {
        Command::FlipPolygons
        | Command::UnifyPolygons(_)
        | Command::SplitPolygon
        | Command::QuadruplePolygon
        | Command::SpinQuads
        | Command::AlignPolygons
        | Command::Triple
        | Command::RemovePolygons
        | Command::DeleteComponents
        | Command::MergePolygons
        | Command::CollapsePolygon
        | Command::FlattenNgons
        | Command::ScaleOffsetPolygon(_)
        | Command::CleanupFloating(_)
        | Command::MeshCleanup { .. }
        | Command::MakePolygon
        | Command::MakePolygonAs { .. }
        | Command::SubdivideEdges { .. }
        | Command::SubdividePolygons { .. }
        | Command::UnsubdividePolygons(_)
        | Command::Detriangulate(_)
        | Command::UvUnwrap(_)
        | Command::UvAtlas(_)
        | Command::DuplicateSelectedPolygons
        | Command::Symmetrize(_)
        | Command::AssignSelectionSet(_)
        | Command::SetPart(_)
        | Command::SetMaterial(_)
        | Command::SetPosition(_) => true,
        Command::EdgeCommand(c) => EdgeCommand::ALL.contains(c),
        Command::VertexCommand(c) => matches!(
            c,
            VertexCommand::Split
                | VertexCommand::SplitEdge
                | VertexCommand::Join { .. }
                | VertexCommand::Merge { .. }
                | VertexCommand::Collapse
                | VertexCommand::Remove
                | VertexCommand::RemoveEdge
                | VertexCommand::Align { .. }
        ),
        _ => false,
    }
}

/// Selection commands that run on every active mesh (MAM-Q01 rules: a
/// mesh where one fails keeps its selection; the command fails only when
/// it fails on every mesh).
pub(crate) fn selects_on_each_mesh(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::SelectSymmetrical | Command::UseSelectionSet { .. } | Command::SelectPolygonSet(_)
    )
}

/// The command as each mesh's run sees it. A dialog's expected target is
/// a snapshot of the primary alone; the several-mesh run is checked as a
/// whole instead (the dialog is modal, so nothing changes in between).
fn per_mesh_command(cmd: &Command) -> Command {
    let mut cmd = cmd.clone();
    match &mut cmd {
        Command::SetPart(a) => a.expected = None,
        Command::SetMaterial(a) => a.expected = None,
        Command::SetPosition(a) => a.expected = None,
        Command::CleanupFloating(t) => *t = None,
        Command::MeshCleanup { target, .. } => *target = None,
        _ => {}
    }
    cmd
}

/// Whether `tool` runs on every active mesh (Batches B and C): one handle
/// at the combined centre, the same tool properties on every mesh. Their
/// amounts are world distances (each tool divides by its item's scale).
/// Each tool's card says (`ToolCard::multi_mesh`).
pub(crate) fn tool_runs_on_each_mesh(tool: Tool) -> bool {
    tool.card().multi_mesh
}

/// Extrude on several meshes (`Editor::multi_extrude`): per mesh, the
/// moving caps a continuation extrudes again, and the caps Drop selects.
#[derive(Clone, Default, PartialEq)]
pub struct MultiExtrude {
    pub moved: std::collections::BTreeMap<u64, Vec<u32>>,
    pub dropped: std::collections::BTreeMap<u64, Vec<u32>>,
    /// Each mesh's `Live::identity_preview` cache: two meshes can have the
    /// same edit plan but never the same identities.
    pub previews:
        std::collections::BTreeMap<u64, Option<(crate::EditCorrespondence, crate::MeshIdentity)>>,
}

impl crate::Selection {
    /// Give mesh `id` these selected polygons, keeping its points and edges.
    pub(crate) fn set_faces_of(&mut self, id: u64, faces: Vec<u32>) {
        let mut c = self.take_components(id);
        c.faces = faces;
        self.put_components(id, c);
    }
}

impl Editor {
    /// The meshes a one-shot command runs on, or `None` for the ordinary
    /// single-mesh path.
    pub(crate) fn command_mesh_targets(&self, cmd: &Command) -> Option<Vec<u64>> {
        if self.session.has_active_task() || self.selection_start.is_some() {
            return None;
        }
        let component_mode = self.scene.selection.mode != Mode::Item;
        if selects_on_each_mesh(cmd) {
            // Selection commands: every active mesh, like Select All.
            let active = self.scene.active_meshes();
            return match active[..] {
                [] => None,
                [only] if Some(only) == self.scene.selection.item => None,
                _ => Some(active),
            };
        }
        let edits = runs_on_each_mesh(cmd)
            // Hide/Lock of selected components; Clear and Item mode already
            // cover every selected item (element_state.rs).
            || (component_mode
                && matches!(cmd, Command::EditElements { op, .. } if *op != crate::ElementOp::Clear));
        if !edits {
            return None;
        }
        self.scene.mesh_edit_targets()
    }
    /// Run `f` once per target mesh, each standing in as the sole selected
    /// item with its own components, and keep what each run left: its
    /// mesh, its new components. All or nothing: a run that fails fails
    /// the whole command (named after its mesh), and the caller's staged
    /// copy is dropped. The steps the runs recorded become one step from
    /// the scene before the first run to the scene after the last.
    pub(crate) fn run_on_meshes(
        &mut self,
        targets: &[u64],
        lenient: bool,
        mut f: impl FnMut(&mut Editor) -> Result<()>,
    ) -> Result<()> {
        let initial = self.scene.clone();
        let mark = self.history_mark();
        let tool_steps = self.history.tool_steps;
        let max_undo = self.history.settings.max_undo_level;
        let fold = self.history.undo_fold.take();
        // No trimming or folding until the runs' steps are joined below
        // (Astra P9-001).
        self.history.settings.max_undo_level = usize::MAX;
        let paused = std::mem::replace(&mut self.history.undo_trim_paused, true);
        let result = (|| {
            let mut first_error = None;
            let mut done = 0;
            for &id in targets {
                let scene_before = self.scene.clone();
                let mark = self.history_mark();
                let saved = std::mem::take(&mut self.scene.selection);
                self.scene.selection = saved.sole(id);
                let run = f(self);
                let after = std::mem::replace(&mut self.scene.selection, saved);
                if let Err(e) = run {
                    let e = format!("{}: {e}", self.scene.mesh_name(id));
                    if !lenient {
                        return Err(e);
                    }
                    // Lenient: this mesh stays as it was.
                    self.scene = scene_before;
                    self.history
                        .undo
                        .truncate(self.history.undo.len() - self.steps_since(mark));
                    first_error.get_or_insert(e);
                    continue;
                }
                self.scene.selection.take_sole_result(id, after);
                self.multi_fresh.insert(id);
                done += 1;
            }
            match first_error {
                Some(e) if done == 0 => Err(e),
                _ => Ok(()),
            }
        })();
        self.history.settings.max_undo_level = max_undo;
        self.history.undo_fold = fold;
        self.history.undo_trim_paused = paused;
        result?;
        let from = self.history.undo.len() - self.steps_since(mark);
        let mut steps = self.history.undo.drain(from..);
        let first = steps.next().map(std::sync::Arc::unwrap_or_clone);
        let mut records = first
            .as_ref()
            .map(|f| f.records.clone())
            .unwrap_or_default();
        let mut merged = 0;
        for step in steps {
            records
                .0
                .extend(std::sync::Arc::unwrap_or_clone(step).records.0);
            merged += 1;
        }
        if let Some(first) = first {
            self.history
                .undo
                .push(std::sync::Arc::new(crate::Checkpoint {
                    identity_before: first.identity_before,
                    identity_after: None,
                    label: first.label,
                    before: initial,
                    after: self.scene.clone(),
                    fold: self.history.undo_fold,
                    records,
                    mark: first.mark,
                }));
            let excess = self.history.undo.len().saturating_sub(max_undo.max(1));
            self.history.undo.drain(..excess);
            self.trim_undo_memory();
            if let (Some(n), Some(start)) = (self.history.tool_steps.as_mut(), tool_steps) {
                *n = (*n).saturating_sub(merged).max(start);
            }
        }
        Ok(())
    }
    /// A one-shot command on every target mesh.
    pub(crate) fn command_on_meshes(&mut self, targets: &[u64], cmd: &Command) -> Result<()> {
        let each = per_mesh_command(cmd);
        self.run_on_meshes(targets, selects_on_each_mesh(cmd), |editor| {
            editor.dispatch_inner(each.clone(), &mut None)
        })?;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// The meshes a live tool works on, from its base scene, or `None`
    /// for the single-mesh path.
    pub(crate) fn tool_mesh_targets(tool: Tool, base: &Scene) -> Option<Vec<u64>> {
        if !tool_runs_on_each_mesh(tool) {
            return None;
        }
        base.mesh_edit_targets()
    }
    /// Activate `tool` across several meshes: each target must be able to
    /// start it on its own (the same checks and messages as one mesh), and
    /// the session then works on the whole scene with one handle at the
    /// combined centre. Starting values that come from a mesh (Edge
    /// Weight's current weight, Extrude's starting Width, Vertex Slide's
    /// rail) come from the first target (MAM-Q13).
    pub(crate) fn activate_on_meshes(&mut self, tool: Tool, targets: &[u64]) -> Result<()> {
        let mut first = None;
        for &id in targets {
            let mut sole = self.clone();
            sole.scene.selection = self.scene.selection.sole(id);
            sole.handle_activate(tool)
                .map_err(|e| format!("{}: {e}", self.scene.mesh_name(id)))?;
            if first.is_none() {
                first = sole.session.live.take();
            }
        }
        let mut live = first.ok_or("Select a mesh")?;
        live.base = self.scene.clone();
        live.identity_base = Some(self.identities.current.clone());
        live.identity_preview = None;
        live.applied = false;
        if live.action_frame.is_some() {
            live.action_frame = Some(Self::extrude_frame(self.action_context, tool, &self.scene)?);
        }
        self.scene = live.base.clone();
        self.session.live = Some(live);
        if tool.shows_result_at_once() {
            let p = self
                .session
                .live
                .as_ref()
                .ok_or("No active tool")?
                .params
                .clone();
            self.preview(p, false)?;
        }
        Ok(())
    }
    /// One preview of a live tool on every target: each mesh's result is
    /// worked out from the session's base as that mesh alone sees it, and
    /// the scene then has every mesh's result and new components.
    pub(crate) fn preview_on_meshes(
        &mut self,
        targets: &[u64],
        p: Parameters,
        apply: bool,
    ) -> Result<()> {
        let base = self
            .session
            .live
            .as_ref()
            .ok_or("No active tool")?
            .base
            .clone();
        let mut selection = base.selection.clone();
        let mut scene = base.clone();
        let extrude = self
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::Extrude);
        let mut caps = MultiExtrude {
            previews: self
                .multi_extrude
                .take()
                .map(|m| m.previews)
                .unwrap_or_default(),
            ..Default::default()
        };
        let result: Result<()> = (|| {
            for &id in targets {
                // Built on the meshes already done, so each run's scene
                // (and the identities it observes) matches the last one's.
                let mut sole = scene.clone();
                sole.selection = base.selection.sole(id);
                let live = self.session.live.as_mut().ok_or("No active tool")?;
                live.base = sole;
                if extrude {
                    live.identity_preview = caps.previews.remove(&id).flatten();
                }
                self.preview_one(p.clone(), apply)
                    .map_err(|e| format!("{}: {e}", base.mesh_name(id)))?;
                if extrude {
                    let faces = &self.scene.selection.faces;
                    let live = self.session.live.as_ref().ok_or("No active tool")?;
                    caps.moved.insert(id, self.extrude_moved_caps.clone());
                    let dropped = crate::editor::extrude_caps_on_drop(live, faces)
                        .unwrap_or_else(|| faces.clone());
                    caps.dropped.insert(id, dropped);
                    caps.previews.insert(id, live.identity_preview.clone());
                }
                selection.take_sole_result(id, self.scene.selection.clone());
                scene = self.scene.clone();
            }
            Ok(())
        })();
        self.session.live.as_mut().ok_or("No active tool")?.base = base;
        result?;
        scene.selection = selection;
        scene.validate()?;
        self.scene = scene;
        self.multi_fresh.extend(targets.iter().copied());
        if extrude {
            self.multi_extrude = Some(Box::new(caps));
        }
        Ok(())
    }
}
