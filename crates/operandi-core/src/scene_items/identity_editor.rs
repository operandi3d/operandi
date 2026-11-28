//! Atomic editor boundary for runtime sidecars. Existing semantic dispatch stays
//! local; this owns eligibility, full validation and identity checkpoint capture.
use crate::*;
impl Editor {
    pub fn synchronize_identities(&mut self) -> Result<()> {
        self.identities.synchronize(&self.scene)
    }
    pub fn read_mesh(&mut self, item: u64) -> Result<MeshReadView<'_>> {
        self.synchronize_identities()?;
        Ok(MeshReadView {
            mesh: &self.scene.item(item).ok_or("Missing item")?.mesh,
            identities: self
                .identities
                .current
                .meshes
                .get(&item)
                .ok_or("Missing identity owner")?,
            publication: self.identities.publication,
        })
    }
    pub fn identity_ticket(&mut self) -> Result<IdentityTicket> {
        self.synchronize_identities()?;
        self.identities.ticket()
    }
    pub fn dispatch_with_ticket(&mut self, ticket: IdentityTicket, cmd: Command) -> Result<()> {
        self.synchronize_identities()?;
        self.identities.check(ticket)?;
        self.dispatch(cmd)
    }
    /// Stable targets must still describe the currently displayed ordered
    /// selection. This boundary never silently retargets an armed session.
    pub fn dispatch_faces(
        &mut self,
        ticket: IdentityTicket,
        faces: &[FaceHandle],
        cmd: Command,
    ) -> Result<()> {
        self.synchronize_identities()?;
        self.identities.check(ticket)?;
        let item = self.scene.active().ok_or("Select a mesh")?;
        let ids = &self.identities.current.meshes[&item.id];
        if faces.is_empty() {
            return Err("Select explicit stable faces".into());
        }
        let mut selected = Vec::new();
        for &h in faces {
            let f = ids.face_index(h)?;
            if selected.contains(&f) {
                return Err("Duplicate stable face target".into());
            }
            selected.push(f);
        }
        if selected != self.scene.selection.faces {
            return Err("Stable target selection changed".into());
        }
        if let Some(live) = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::Extrude)
            && (matches!(&cmd, Command::Preview(_))
                || matches!(&cmd, Command::Apply) && !live.applied)
        {
            // Preview evaluates live.base, not the displayed selection. A fresh
            // publication ticket cannot authorize a different source basis.
            // Apply during an existing preview keeps its legacy refusal below.
            let source = live.base.active().ok_or("Missing armed extrusion source")?;
            let basis = live
                .identity_base
                .as_ref()
                .and_then(|ids| ids.meshes.get(&source.id))
                .ok_or("Missing armed extrusion identities")?;
            basis.validate_shared(source.mesh.shared())?;
            let targets = live
                .base
                .selection
                .faces
                .iter()
                .map(|&f| {
                    basis
                        .faces
                        .get(f as usize)
                        .copied()
                        .map(|face| FaceHandle {
                            scope: basis.scope,
                            face,
                        })
                        .ok_or_else(|| String::from("Invalid armed extrusion target"))
                })
                .collect::<Result<Vec<_>>>()?;
            if faces != targets {
                return Err("Stable targets differ from the armed extrusion basis".into());
            }
        }
        self.dispatch(cmd)
    }
    pub fn dispatch(&mut self, cmd: Command) -> Result<()> {
        self.dispatch_transaction(cmd, &mut None)
    }
    pub fn dispatch_history_reported(
        &mut self,
        request: HistoryRequest,
    ) -> std::result::Result<HistoryCompletion, HistoryFailure> {
        let mut report = Some(crate::history_completion::HistoryReport::new(request, self));
        let result = self.dispatch_transaction(request.command(), &mut report);
        let report = report.unwrap(); // this request-local slot is never removed
        match result {
            Ok(()) => Ok(report.finish(self)),
            Err(message) => Err(report.fail(self, message)),
        }
    }
    fn dispatch_transaction(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        // UIC-307: a top-level command an open tool would refuse settles
        // the tool, runs, and starts the tool again (tool_yield.rs).
        if self.session.dispatch_depth == 0
            && let Some(restart) = self.restart_command()
        {
            let retry = cmd.clone();
            return match self.dispatch_transaction_once(cmd, report) {
                Err(e) if crate::tool_yield::idle_refusal(&e) => {
                    self.yield_tool_for(restart, retry)
                }
                result => result,
            };
        }
        self.dispatch_transaction_once(cmd, report)
    }
    fn dispatch_transaction_once(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        // Refuse invalid names before staging can allocate any identities.
        if let Command::NewNamedMesh(name) = &cmd
            && (name.trim().is_empty() || name.len() > 1024)
        {
            return Err("Name must contain 1–1024 bytes".into());
        }
        // UIC-160/161 (PD-Q12): a typed or scripted Item Transform of the
        // live item tool's Planar locator is the same tweak as its viewport
        // drag, so both give the same result and undo step.
        let cmd = crate::planar_texture::route_item_transform(self, cmd);
        // VP-04 (last_mesh.rs): with nothing selected, the last selected
        // mesh is still the one a command edits.
        // CS-01 (click_start.rs): an armed tool takes the commands that
        // would make it live or change the mesh; picking a tool flagged
        // off arms it. Only for top-level commands, so staged one-shots
        // and the tools' own inner commands run as before.
        let top = self.session.dispatch_depth == 0;
        if top && let Some(result) = self.armed_command(&cmd) {
            return result;
        }
        let arms = top && crate::click_start::may_arm(&cmd);
        self.session.dispatch_depth += 1;
        let task_before = self.session.active_task();
        let unadopted = self.adopt_last_mesh_for(&cmd);
        let adopted = unadopted.as_ref().map(|_| self.scene.clone());
        let result = self.dispatch_selected(cmd, report);
        self.session.dispatch_depth = self.session.dispatch_depth.saturating_sub(1);
        if arms && result.is_ok() {
            self.arm_if_flagged();
        }
        if let (Some(unadopted), Some(adopted)) = (unadopted, adopted) {
            self.settle_last_mesh(unadopted, &adopted, task_before, result.is_ok());
        }
        self.scene.note_last_mesh();
        result
    }
    fn dispatch_selected(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        // Undo with a transform tool's unrecorded change: publish that change
        // as its own step first, in a transaction of its own like Drop, so
        // Undo reverts it and Redo brings it back (Jason, 2026-11-03).
        if matches!(&cmd, Command::Undo)
            && self.transform_change_pending()
            && self.tweak_session().is_none()
            && !self.i2_owns_history(crate::HistoryAction::Undo)
            && self
                .history_eligibility(crate::HistoryAction::Undo)
                .permit()
                .is_ok()
        {
            self.dispatch_transaction(Command::CommitTransformStep, &mut None)?;
        }
        // CMD-B3-03: Quadruple with only quads and triangles selected is a
        // quiet no-op; answer it before the topology routes below run.
        // Both no-op shortcuts read the primary only; with several active
        // meshes the per-mesh runs decide (multi_tools.rs).
        let several = self.command_mesh_targets(&cmd).is_some();
        if matches!(&cmd, Command::QuadruplePolygon) && !several && self.quadruple_is_noop() {
            return Ok(());
        }
        if !several && let Command::CleanupFloating(expected) = &cmd {
            let (target, cleaned) = self.cleanup_preflight(expected.as_ref())?;
            if cleaned.survivors.len() == target.mesh_vertex_count() {
                return Ok(());
            }
        }
        let preserve_numeric_draft = matches!(
            &cmd,
            Command::ConfigureEllipsoid(_)
                | Command::ConfigureShape(_)
                | Command::ConfigureCone(_)
                | Command::ConfigureSpike(_)
        );
        // These routes can replace topology without an admitted survivor map.
        // Invalidate even a byte-identical replacement, rather than infer that
        // coincident/recreated components survived. Selection/context-only routes
        // do not enter this category. Changed unported meshes also rebind in sync.
        let topology_tool = self.session.live.as_ref().is_some_and(|l| {
            matches!(
                l.tool,
                Tool::Box
                    | Tool::Sphere
                    | Tool::Cylinder
                    | Tool::Bevel
                    | Tool::TwistBevel
                    | Tool::Chamfer
                    | Tool::EdgeExtend
                    | Tool::EdgeExtrude
                    | Tool::EdgeBevel
                    | Tool::EdgeChamfer
                    | Tool::VertexCreate
                    | Tool::VertexBevel
                    | Tool::VertexExtrude
                    | Tool::VertexMerge
                    | Tool::VertexTear
                    | Tool::PolyHaul
                    | Tool::Inset
                    | Tool::Shift
                    | Tool::SketchExtrude
                    | Tool::SmoothShift
                    | Tool::Thicken
            )
        });
        let unported_topology = match &cmd {
            Command::Apply => {
                topology_tool
                    || self.session.spike.is_some()
                    || self.session.bridge.is_some()
                    || self.session.cone.is_some()
                    || self.session.ellipsoid.is_some()
                    || self.session.shape.is_some()
            }
            // Live Drop publishes session/history only: its preview was
            // already rebound. Numeric constructors append at Drop, which is
            // detected as a changed mesh by synchronize below.
            Command::Drop => false,
            Command::Preview(_) => topology_tool,
            Command::ConfigureBevel(_)
            | Command::ConfigureSmoothShift(_)
            | Command::ConfigureChamfer(_) => {
                topology_tool && self.session.live.as_ref().is_some_and(|l| l.applied)
            }
            _ => false,
        };
        let unrecorded_document_edit = matches!(
            &cmd,
            Command::Mode(_) | Command::Workplane(_) | Command::SelectItem(_)
        );
        let completed_replacement = matches!(&cmd, Command::Apply) && unported_topology;
        let i2_selection_publication = self.i2.selected.is_some()
            && matches!(
                &cmd,
                Command::Mode(_)
                    | Command::Workplane(_)
                    | Command::SelectItem(_)
                    | Command::SelectPolygonSet(_)
                    | Command::UseSelectionSet { .. }
                    | Command::Selection(_)
                    | Command::Select { .. }
                    | Command::ClearSelection
                    | Command::EndSelection { .. }
            );
        let flip_identity = matches!(&cmd, Command::FlipPolygons);
        let history_command = matches!(&cmd, Command::Undo | Command::Redo);
        let stack_command = matches!(
            &cmd,
            Command::MeshOpCreate(_)
                | Command::MeshBlend(_)
                | Command::MeshOpDelete(_)
                | Command::MeshOpMove { .. }
                | Command::MeshOpEnable { .. }
                | Command::MeshOpSet { .. }
                | Command::MeshOpSelect(_)
                | Command::MeshOpIndex { .. }
                | Command::MeshOpSelOp { .. }
                | Command::MeshOpFreeze { .. }
                | Command::MeshOpFreezeAll
                | Command::MeshOpEditBase { .. }
                // Spec freeze-transforms rule 4.2.10: Freeze bakes into the
                // Base Mesh and the stack stays live.
                | Command::FreezeTransform { .. }
        );
        let task_before = self.mid_undo_task();
        let tweak_task_before = self.tweak_session();
        let tweak_command = self.tweak_before(&cmd);
        let is_tweak = self.is_tweak(&cmd);
        let serial_before = self.history.history_serial;
        let initial_scene = self.scene.clone();
        let mut staged = self.clone();
        staged.synchronize_identities()?;
        let initial_scopes = staged.identities.current.clone();
        staged.identities.record_before = Some(staged.identities.current.clone());
        // MAM-01: tools edit only the primary mesh; Cancel only reverts.
        let primary_guard = !history_command && !matches!(&cmd, Command::Cancel);
        let creating = self.creating_shape();
        let result = (|| {
            if let Err(error) = staged.dispatch_inner(cmd, report) {
                return Err(
                    staged.explain_mesh_stack_error(error, stack_command || history_command)
                );
            }
            if !history_command {
                staged.scene.prune_component_selections_keeping(
                    Some(&initial_scene),
                    &staged.multi_fresh.clone(),
                );
            }
            if primary_guard {
                staged
                    .guard_primary_only_edit(&initial_scene, creating || staged.creating_shape())?;
            }
            // Geometry lock and hide (element_state.rs): one rule for
            // every command; Undo/Redo restore recorded scenes as they were.
            if !history_command {
                staged.protect_elements(&initial_scene)?;
            }
            // Mesh Stack (mesh_stack_editor.rs): SAFE-05/06 for every command.
            if !history_command {
                staged.guard_mesh_stack(&initial_scene, stack_command)?;
            } else {
                staged.settle_mesh_stack();
            }
            staged.settle_preset();
            staged.track_tweak(
                is_tweak,
                tweak_command,
                tweak_task_before,
                serial_before,
                history_command,
            );
            staged.track_tool_activation(task_before, history_command);
            if (unrecorded_document_edit && staged.scene != initial_scene) || completed_replacement
            {
                staged.i2_branch();
            }
            if flip_identity {
                let primary = initial_scene.selection.item.ok_or("Flip target missing")?;
                // The primary, and every other mesh Flip ran on
                // (multi_tools.rs).
                let mut flipped_items = vec![primary];
                flipped_items.extend(staged.multi_fresh.iter().filter(|&&id| id != primary));
                for item in flipped_items {
                    let source = initial_scene.item(item).ok_or("Flip source missing")?;
                    let output = staged.scene.item(item).ok_or("Flip output missing")?;
                    let ids = initial_scopes
                        .meshes
                        .get(&item)
                        .ok_or("Flip identity missing")?;
                    // Flip keeps the first corner and reverses the rest (as
                    // Align does); the flipped polygons are those whose
                    // corners changed (all of them when nothing was
                    // selected).
                    let flipped: std::collections::BTreeSet<usize> = (0..source.mesh.faces.len())
                        .filter(|&f| {
                            output.mesh.faces.get(f).map(|x| &x.vertices)
                                != Some(&source.mesh.faces[f].vertices)
                        })
                        .collect();
                    let next = ids.anchored_reversed_faces(&source.mesh, &output.mesh, &flipped)?;
                    staged.identities.current.meshes.insert(item, next.into());
                }
                staged.identities.observe(&staged.scene)?;
            }
            if i2_selection_publication {
                // These explicit owners change selection/context, never mesh
                // payloads. Observe the privately validated identities here so
                // synchronization does not publish a second intermediate token.
                staged.scene.validate()?;
                staged.identities.observe(&staged.scene)?;
            }
            staged.synchronize_identities()?;
            if unported_topology
                && let Some(item) = staged.scene.selection.item
                && let Some(old) = initial_scopes.meshes.get(&item)
                && staged
                    .identities
                    .current
                    .meshes
                    .get(&item)
                    .is_some_and(|now| now.scope == old.scope)
            {
                staged.identities.invalidate_mesh(&staged.scene, item)?;
            }
            // The same for the other meshes a tool ran on (multi_tools.rs).
            if unported_topology {
                for &item in staged.multi_fresh.clone().iter() {
                    if Some(item) != staged.scene.selection.item
                        && let Some(old) = initial_scopes.meshes.get(&item)
                        && staged
                            .identities
                            .current
                            .meshes
                            .get(&item)
                            .is_some_and(|now| now.scope == old.scope)
                    {
                        staged.identities.invalidate_mesh(&staged.scene, item)?;
                    }
                }
            }
            if staged.session.live.is_none() {
                staged.multi_fresh.clear();
            }
            for c in staged
                .history
                .undo
                .iter_mut()
                .chain(&mut staged.history.redo)
            {
                if c.identity_after.is_none() {
                    let c = std::sync::Arc::make_mut(c);
                    if c.identity_before
                        .as_ref()
                        .is_none_or(|ids| ids.validate(&c.before).is_err())
                    {
                        c.identity_before = Some(staged.identities.allocate_snapshot(&c.before)?);
                    }
                    let after = if staged.identities.current.validate(&c.after).is_ok() {
                        staged.identities.current.clone()
                    } else {
                        staged.identities.allocate_snapshot(&c.after)?
                    };
                    c.identity_after = Some(after);
                }
            }
            staged.identities.advance()?;
            staged.identities.record_before = None;
            Ok(())
        })();
        match result {
            Ok(()) => {
                if let Some(report) = report {
                    report.authored_scene_changed = staged.scene != initial_scene;
                }
                *self = staged;
                Ok(())
            }
            Err(e) => {
                // Allocation marks belong to the shared document bank, not
                // staged snapshots. Err cannot rewind their consumed ranges.
                if preserve_numeric_draft {
                    self.session.ellipsoid = staged.session.ellipsoid;
                    self.session.shape = staged.session.shape;
                    self.session.cone = staged.session.cone;
                    self.session.spike = staged.session.spike;
                }
                // Existing invalid-preview diagnostics are UI/session state. They do
                // not publish the failed geometry or replace accepted history.
                if let (Some(l), Some(s)) = (&mut self.session.live, &staged.session.live) {
                    if let Some(report) = report {
                        report.live_diagnostic_changed = l.preview_error != s.preview_error;
                    }
                    l.preview_error = s.preview_error.clone();
                }
                Err(e)
            }
        }
    }
}
