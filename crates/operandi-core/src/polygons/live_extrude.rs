//! Bounded I2 transaction-local Begin/Move history. Full authored snapshots are
//! the correctness control; completed document history remains authoritative.
//! A selected segment and one flat parked alternative cannot form a chain.
use crate::session_ownership::{HistoryPosition, HistoryRoute, LocalResumeObstacle};
use crate::*;
use glam::DVec3;
use std::sync::{Arc, Mutex};

/// Experimental capture eligibility, not a persisted identity or stable API.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ExtrudeStageTicket {
    pub identity: IdentityTicket,
    pub stage: u64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StageImage {
    pub scene: Scene,
    pub identities: IdentityScene,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ExtrudeStage {
    pub key: u64,
    pub anchor: usize,
    pub base: StageImage,
    pub begun: StageImage,
    pub movement: Option<StageImage>,
    pub armed_params: Parameters,
    pub movement_params: Option<Parameters>,
    pub frame: Option<ActionFrame>,
    pub plan: EditCorrespondence,
    pub cursor: u8,
    pub ever_parked: bool,
}
/// Shared only to burn stage serials across failed private candidates. Published
/// StageImages themselves are owned immutable values; no mutable history links.
#[derive(Clone)]
pub struct StageSerial(Arc<Mutex<u64>>);
impl Default for StageSerial {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(1)))
    }
}
impl PartialEq for StageSerial {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl StageSerial {
    pub fn next(&self) -> Result<u64> {
        self.0
            .lock()
            .map(|v| *v)
            .map_err(|_| "Stage allocator unavailable".into())
    }
    fn take(&self) -> Result<u64> {
        let mut n = self.0.lock().map_err(|_| "Stage allocator unavailable")?;
        let key = *n;
        *n = n.checked_add(1).ok_or("Stage budget exhausted")?;
        Ok(key)
    }
}
#[derive(Clone, Default, PartialEq)]
pub struct LiveExtrudeHistory {
    pub selected: Option<ExtrudeStage>,
    pub prior: Option<ExtrudeStage>,
    pub serial: StageSerial,
    #[cfg(test)]
    fail_after_reservations: Option<u64>,
}
impl Editor {
    pub fn extrude_stage_ticket(&mut self) -> Result<ExtrudeStageTicket> {
        let stage = self
            .i2
            .selected
            .as_ref()
            .filter(|s| s.cursor > 0)
            .ok_or("No active extrusion stage")?
            .key;
        Ok(ExtrudeStageTicket {
            identity: self.identity_ticket()?,
            stage,
        })
    }
    pub fn check_extrude_stage_ticket(&self, t: ExtrudeStageTicket) -> Result<()> {
        self.identities.check(t.identity)?;
        if !self
            .i2
            .selected
            .as_ref()
            .is_some_and(|s| s.cursor > 0 && s.key == t.stage)
        {
            return Err("Stale extrusion stage key".into());
        }
        Ok(())
    }
    pub fn dispatch_extrude_stage(&mut self, t: ExtrudeStageTicket, cmd: Command) -> Result<()> {
        self.check_extrude_stage_ticket(t)?;
        if !matches!(&cmd, Command::Preview(_) | Command::ConfigureExtrude(_)) {
            return Err("Stage capture only authorizes movement".into());
        }
        self.dispatch(cmd)
    }

    pub fn i2_active(&self) -> bool {
        self.i2.selected.as_ref().is_some_and(|s| s.cursor > 0)
    }
    /// Shared routing precedence for queries and local execution. Completed
    /// document Redo retains ownership until the exact parked anchor is reached.
    pub(crate) fn i2_owns_history(&self, action: HistoryAction) -> bool {
        matches!(
            self.session_ownership()
                .history_route(action, HistoryPosition::of(self)),
            HistoryRoute::LocalExtrude(_)
        )
    }
    pub fn i2_can_undo(&self) -> bool {
        self.i2_active()
    }
    pub fn i2_can_redo(&self) -> bool {
        self.i2_redo_eligibility().is_ok()
    }
    /// This is also the execution guard. Ordinary document Redo below the
    /// anchor stays with its existing owner and is not gated by this predicate.
    pub(crate) fn i2_redo_eligibility(&self) -> std::result::Result<(), HistoryRefusal> {
        let s = self
            .i2
            .selected
            .as_ref()
            .ok_or(HistoryRefusal::NoLocalExtrudeRedo)?;
        match s.cursor {
            0 => {
                if s.anchor != self.history.undo.len() || !self.history.redo.is_empty() {
                    return Err(HistoryRefusal::ParkedAnchor);
                }
                self.i2_resume_owner()?;
                if self.scene != s.base.scene || self.identities.current != s.base.identities {
                    return Err(HistoryRefusal::ParkedBasis);
                }
            }
            1 if s.movement.is_some() => self
                .i2_check_active_source(s)
                .map_err(HistoryRefusal::ActiveExtrudeSource)?,
            _ => return Err(HistoryRefusal::LocalMovement),
        }
        Ok(())
    }
    pub(crate) fn i2_branch(&mut self) {
        self.i2.selected = None;
        self.i2.prior = None;
    }
    fn i2_resume_owner(&self) -> std::result::Result<(), HistoryRefusal> {
        // I2-PR01: hidden parked history cannot restore a stored World frame
        // while the current displayed context/falloff is outside this profile.
        if self.action_context != ActionContext::default() || self.spatial_falloff.is_some() {
            return Err(HistoryRefusal::ResumeContext);
        }
        match self.session_ownership().local_resume_obstacle() {
            Some(LocalResumeObstacle::IncomingTaskOrSelection) => Err(HistoryRefusal::ResumeOwner),
            Some(LocalResumeObstacle::LivePreview) => Err(HistoryRefusal::ResumePreview),
            None => Ok(()),
        }
    }

    fn i2_restore(&mut self, stage: &ExtrudeStage, cursor: u8) -> Result<()> {
        let image = match cursor {
            0 => &stage.base,
            1 => &stage.begun,
            _ => stage.movement.as_ref().ok_or("No movement to restore")?,
        };
        self.scene = image.scene.clone();
        self.identities.current = image.identities.clone();
        self.identities.observe(&self.scene)?;
        self.session.live = if cursor == 0 {
            None
        } else {
            let mut p = if cursor == 2 {
                stage
                    .movement_params
                    .clone()
                    .ok_or("Missing movement parameters")?
            } else {
                stage.armed_params.clone()
            };
            if cursor == 1 {
                p.extent = DVec3::ZERO;
            }
            let item = stage
                .base
                .scene
                .selection
                .item
                .ok_or("Missing stage owner")?;
            Some(Live {
                tool: Tool::Extrude,
                base: stage.base.scene.clone(),
                identity_base: Some(stage.base.identities.clone()),
                identity_preview: Some((
                    stage.plan.clone(),
                    stage.begun.identities.meshes[&item].to_owned_value(),
                )),
                params: p,
                applied: true,
                action_frame: stage.frame,
                placed_centre: None,
                preview_error: None,
                pivots: Default::default(),
            })
        };
        self.extrude_moved_caps = if cursor == 0 {
            vec![]
        } else {
            self.scene.selection.faces.clone()
        };
        Ok(())
    }
    /// Complete bounded source/profile eligibility for the native pre-Begin
    /// route. This does not attempt Begin or test resource budgets. Capture may
    /// retain legacy routing only when this source/profile is unsupported;
    /// runtime/Begin failures must still propagate once this query succeeds.
    pub fn i2_begin_source_supported(&self) -> bool {
        self.i2_validate_begin_source().is_ok()
    }
    fn i2_validate_begin_source(&self) -> Result<()> {
        let l = self.session.live.as_ref().ok_or("Arm Extrude first")?;
        if l.tool != Tool::Extrude || l.applied || self.i2_active() {
            return Err("Source requires an Armed Extrude".into());
        }
        if self.action_context != ActionContext::default()
            || self.spatial_falloff.is_some()
            || l.params.extrude_mode != ExtrudeMode::Shift
            || l.params.extrude_uvs != ExtrudeUvs::None
            || l.params.extrude_sides != 1
            || l.params.extrude_uv_map.as_deref() != Some("Texture")
            || !l.params.extrude_uv_selected
            || !l.params.extent.is_finite()
            || l.params.extent.x != 0.
            || l.params.extent.y != 0.
        {
            return Err("I2 Begin supports None/World, Shift, None UV, Sides1, selected Texture and normal Z only".into());
        }
        self.scene.validate()?;
        self.identities.current.validate(&self.scene)?;
        let item = l.base.active().ok_or("Missing extrusion owner")?;
        if self.scene != l.base
            || l.identity_base.as_ref() != Some(&self.identities.current)
            || l.base.selection.mode != Mode::Polygon
            || l.base.selection.faces.len() != 1
            || l.base.world_matrix(item.id)? != glam::DMat4::IDENTITY
        {
            return Err("Changed or unsupported I2 source basis".into());
        }
        let face = item
            .mesh
            .faces
            .get(l.base.selection.faces[0] as usize)
            .ok_or("Missing polygon")?;
        if face.vertices.len() != 4 {
            return Err("I2 requires one planar convex quad".into());
        }
        let points = face
            .vertices
            .iter()
            .map(|&v| {
                item.mesh
                    .vertices
                    .get(v as usize)
                    .copied()
                    .ok_or_else(|| "Missing quad vertex".to_owned())
            })
            .collect::<Result<Vec<_>>>()?;
        let n = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize_or_zero();
        if n.length_squared() < 0.5
            || (points[3] - points[0]).dot(n).abs() > 1e-10
            || (0..4).any(|i| {
                (points[(i + 1) % 4] - points[i])
                    .cross(points[(i + 2) % 4] - points[(i + 1) % 4])
                    .dot(n)
                    <= 0.
            })
        {
            return Err("I2 requires one planar convex quad".into());
        }
        // Full kernel eligibility is read-only; no identity or serial reservation.
        extrude_geometry(
            &item.mesh,
            &l.base.selection.faces,
            DVec3::ZERO,
            1,
            ExtrudeMode::Shift,
            ExtrudeUvs::None,
            Some("Texture"),
        )?;
        Ok(())
    }
    fn i2_begin(&mut self) -> Result<()> {
        if self.history.undo.len() + self.history.redo.len() > 100 {
            return Err("Malformed completed history capacity".into());
        }
        let l = self
            .session
            .live
            .as_ref()
            .ok_or("Arm Extrude first")?
            .clone();
        if l.tool != Tool::Extrude || l.applied || l.preview_error.is_some() || self.i2_active() {
            return Err("Begin requires an Armed Extrude".into());
        }
        self.i2_validate_begin_source()?;
        self.identities
            .publication
            .checked_add(1)
            .ok_or("Publication budget exhausted")?;
        self.identities
            .request
            .checked_add(1)
            .ok_or("Request budget exhausted")?;
        self.identities
            .next_id()?
            .checked_add(32)
            .ok_or("Identity budget exhausted")?;
        self.i2
            .serial
            .next()?
            .checked_add(1)
            .ok_or("Stage budget exhausted")?;
        let key = self.i2.serial.take()?;
        #[cfg(test)]
        if self.i2.fail_after_reservations == Some(4) {
            self.identities.reserve_through(
                self.identities
                    .next_id()?
                    .checked_add(4)
                    .ok_or("Identity budget exhausted")?,
            )?;
            return Err("Injected failure after four private reservations".into());
        }
        let mut p = l.params.clone();
        p.extent = DVec3::ZERO;
        self.preview(p, true)?;
        #[cfg(test)]
        if self.i2.fail_after_reservations == Some(32) {
            return Err("Injected failure after complete private candidate".into());
        }
        let now = self.session.live.as_ref().ok_or("Missing begun session")?;
        let plan = now
            .identity_preview
            .as_ref()
            .ok_or("Missing Begin correspondence")?
            .0
            .clone();
        let stage = ExtrudeStage {
            key,
            anchor: self.history.undo.len(),
            base: StageImage {
                scene: l.base.clone(),
                identities: l.identity_base.clone().ok_or("Missing source identities")?,
            },
            begun: StageImage {
                scene: self.scene.clone(),
                identities: self.identities.current.clone(),
            },
            movement: None,
            armed_params: l.params,
            frame: l.action_frame,
            plan,
            movement_params: None,
            cursor: 1,
            ever_parked: false,
        };
        self.i2.prior = self.i2.selected.take();
        self.i2.selected = Some(stage);
        Ok(())
    }
    fn i2_check_active_source(&self, stage: &ExtrudeStage) -> Result<()> {
        let expected_image = if stage.cursor == 1 {
            &stage.begun
        } else {
            stage.movement.as_ref().ok_or("Missing current movement")?
        };
        let live = self.session.live.as_ref().ok_or("Missing active session")?;
        let item = stage
            .base
            .scene
            .selection
            .item
            .ok_or("Missing stage target")?;
        let begun_ids = stage
            .begun
            .identities
            .meshes
            .get(&item)
            .ok_or("Missing begun identity owner")?;
        if self.scene != expected_image.scene
            || self.identities.current != expected_image.identities
            || live.base != stage.base.scene
            || live.identity_base.as_ref() != Some(&stage.base.identities)
            || !live
                .identity_preview
                .as_ref()
                .is_some_and(|(plan, ids)| *plan == stage.plan && ids == &**begun_ids)
            || live.action_frame != stage.frame
        {
            return Err("Changed extrusion source, layout, scope or displayed basis".into());
        }
        Ok(())
    }
    fn i2_move(&mut self, p: Parameters, configure: bool) -> Result<()> {
        let mut stage = self.i2.selected.clone().ok_or("No active stage")?;
        self.i2_check_active_source(&stage)?;
        let mut allowed = self
            .session
            .live
            .as_ref()
            .ok_or("Missing active session")?
            .params
            .clone();
        allowed.extent = p.extent;
        // Any finite displacement: axis handles and free (view-plane) drags
        // all move the begun caps (user correction 2026-10-30). The fast path
        // is direction-agnostic and falls back to the full preview otherwise.
        if p != allowed || !p.extent.is_finite() {
            return Err("I2 active stage supports finite Extrude movement only".into());
        }
        // Reuse the exact I1 correspondence/identity basis. apply=true retains the
        // real begun-zero ring instead of invoking legacy zero-preview collapse.
        self.i2_preview_move(&stage, p.clone())?;
        if p.extent == DVec3::ZERO {
            if stage.cursor == 2 {
                stage.movement = None;
                stage.movement_params = None;
            }
            stage.cursor = 1;
        } else {
            stage.movement = Some(StageImage {
                scene: self.scene.clone(),
                identities: self.identities.current.clone(),
            });
            stage.movement_params = Some(p.clone());
            stage.cursor = 2;
        }
        if configure {
            *self.presets.get_mut::<crate::ExtrudePreset>() = p;
        }
        self.i2.selected = Some(stage);
        Ok(())
    }
    /// One Move sample. Uses the fast path when it applies, else the full
    /// Extrude preview. Debug builds also run the full preview on a private
    /// copy and require identical results, so every debug test that drags an
    /// I2 stage checks the fast path against the kernel.
    fn i2_preview_move(&mut self, stage: &ExtrudeStage, p: Parameters) -> Result<()> {
        let reference = cfg!(debug_assertions).then(|| self.clone());
        if !self.i2_fast_move(stage, &p)? {
            return self.preview(p, true);
        }
        if let Some(mut full) = reference {
            full.preview(p, true)
                .expect("full Extrude preview refused a sample the fast path accepted");
            assert!(
                full == *self,
                "I2 fast Move differs from the full Extrude preview"
            );
        }
        Ok(())
    }
    /// I2 Move fast path. Begin admitted only a single planar convex quad,
    /// Shift mode, one side, no UV edits, selected Texture map, None/World
    /// context and an identity world matrix (`i2_validate_begin_source`). For
    /// that profile the kernel's topology, correspondence, attributes and cap
    /// selection do not depend on the extent, but named-set eligibility does.
    /// Recheck that predicate for each sample before trusting the template.
    /// An eligible sample changes only:
    /// - the generated row's positions, and
    /// - which walls are flagged in `degenerate_faces` (zero-area at extent 0).
    ///
    /// Positions use the kernel and preview expressions verbatim (world
    /// transform, `extent * (k / sides)`, inverse transform), so signed zeros
    /// and rounding match. Validation is incremental: the template (the
    /// previous sample's output, fully validated) differs only in new
    /// vertices and the faces that use them (the cap and the walls), which are
    /// rechecked here. Any check that fails returns `Ok(false)`, and the full
    /// preview then produces the authoritative result or error.
    fn i2_fast_move(&mut self, stage: &ExtrudeStage, p: &Parameters) -> Result<bool> {
        let Some(live) = self.session.live.as_ref() else {
            return Ok(false);
        };
        let Some(item_id) = stage.base.scene.selection.item else {
            return Ok(false);
        };
        let world = stage.base.scene.world_matrix(item_id)?;
        if stage.base.scene.selection.faces.len() != 1
            || p.extrude_sides != 1
            || p.extrude_mode != ExtrudeMode::Shift
            || p.extrude_uvs != ExtrudeUvs::None
            || world != glam::DMat4::IDENTITY
            || !p.extent.is_finite()
        {
            return Ok(false);
        }
        let (Some(base), Some(current)) =
            (stage.base.scene.item(item_id), self.scene.item(item_id))
        else {
            return Ok(false);
        };
        if base.mesh.has_polylines() {
            return Ok(false);
        }
        // Begin proves the source profile only at zero extent. The guarded
        // source is unchanged, but typed-data eligibility must hold at this
        // sample's extent too. Decline before any memo/state publication so
        // the full preview supplies its authoritative refusal.
        if base.mesh.attributes.has_typed_data()
            && !crate::extrude::set_profile_extent_supported(
                &base.mesh,
                stage.base.scene.selection.faces[0] as usize,
                p.extent,
            )
        {
            return Ok(false);
        }
        let Some(ids) = self.identities.current.meshes.get(&item_id).cloned() else {
            return Ok(false);
        };
        let plan = &stage.plan;
        let base_len = base.mesh.vertices.len();
        if plan.source_vertex_count != base_len
            || plan.vertex_sources.len() != current.mesh.vertices.len()
            || plan.face_sources.len() != current.mesh.faces.len()
        {
            return Ok(false);
        }
        let mut out = current.mesh.to_owned_value();
        let inverse = world.inverse();
        let step = p.extent * (1_f64 / p.extrude_sides as f64);
        for i in base_len..out.vertices.len() {
            let source =
                world.transform_point3(base.mesh.vertices[plan.vertex_sources[i] as usize]);
            let moved = inverse.transform_point3(source + step);
            if !moved.is_finite() || moved.abs().max_element() > 1e9 {
                return Ok(false);
            }
            out.vertices[i] = moved;
        }
        for wall in &plan.walls {
            if out.normal(wall.face) == DVec3::ZERO {
                out.attributes.degenerate_faces.insert(wall.face as u32);
            } else {
                out.attributes.degenerate_faces.remove(&(wall.face as u32));
            }
        }
        if self.scene.selection.faces.iter().any(|&cap| {
            out.normal(cap as usize) == DVec3::ZERO
                && !out.attributes.degenerate_faces.contains(&cap)
        }) {
            return Ok(false);
        }
        let out: crate::Shared<Mesh> = out.into();
        // Equivalent to full validation relative to the validated template:
        // only new vertices and the faces using them changed (checked above);
        // face layout, edge uses and other attributes are copied unchanged, so
        // the identity layout that matched the template also matches.
        out.record_checked();
        ids.record_checked_against(&out);
        let mut scene = live.base.clone();
        scene.selection = self.scene.selection.clone();
        scene.item_mut(item_id).ok_or("Missing stage target")?.mesh = out.into();
        scene.validate()?;
        self.identities.observe(&scene)?;
        self.scene = scene;
        let l = self.session.live.as_mut().ok_or("No tool")?;
        l.params = p.clone();
        l.preview_error = None;
        l.applied = true;
        Ok(true)
    }
    fn i2_finish(&mut self, continuing: bool) -> Result<()> {
        let stage = self.i2.selected.take().ok_or("Missing active stage")?;
        let live = self.session.live.take().ok_or("Missing active session")?;
        if let Some(error) = live.preview_error {
            return Err(format!("Correct stage before Finish: {error}"));
        }
        self.scene.validate()?;
        self.identities.current.validate(&self.scene)?;
        self.i2.prior = None;
        self.identities.record_before = Some(stage.base.identities);
        *self.presets.get_mut::<crate::ExtrudePreset>() = live.params.clone();
        if !continuing {
            self.select_extrude_caps(&live);
        }
        self.record("Extrude".into(), stage.base.scene);
        if continuing {
            let mut p = live.params;
            p.extent = DVec3::ZERO;
            self.session.live = Some(Live {
                tool: Tool::Extrude,
                base: self.scene.clone(),
                identity_base: Some(self.identities.current.clone()),
                identity_preview: None,
                params: p,
                applied: false,
                preview_error: None,
                pivots: Default::default(),
                action_frame: Some(self.pivot_request(&self.scene, Tool::Extrude).frame()?),
                placed_centre: None,
            });
        }
        Ok(())
    }
    /// Returns false only when the existing command owner must handle the route.
    pub(crate) fn i2_dispatch(
        &mut self,
        cmd: &Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<bool> {
        if matches!(cmd, Command::BeginExtrude) {
            self.i2_begin()?;
            return Ok(true);
        }
        if self.i2_active() {
            match cmd {
                Command::Preview(p) => self.i2_move(p.clone(), false)?,
                Command::ConfigureExtrude(p) => self.i2_move(p.clone(), true)?,
                Command::Undo => {
                    let mut s = self.i2.selected.clone().unwrap();
                    s.cursor -= 1;
                    self.i2_restore(&s, s.cursor)?;
                    if let Some(report) = report.as_mut() {
                        report.effect = crate::HistoryEffect::Local {
                            action: HistoryAction::Undo,
                            stage: s.key,
                            from: s.cursor + 1,
                            to: s.cursor,
                        };
                    }
                    if s.cursor == 0 {
                        s.ever_parked = true;
                        self.history.redo.clear();
                        self.i2.prior = None;
                    }
                    self.i2.selected = Some(s);
                }
                Command::Redo => {
                    let mut s = self.i2.selected.clone().unwrap();
                    self.i2_redo_eligibility().map_err(|reason| {
                        if let Some(report) = report.as_mut() {
                            report.refusal = Some(reason.clone());
                        }
                        reason.to_string()
                    })?;
                    self.i2_restore(&s, 2)?;
                    if let Some(report) = report.as_mut() {
                        report.effect = crate::HistoryEffect::Local {
                            action: HistoryAction::Redo,
                            stage: s.key,
                            from: s.cursor,
                            to: 2,
                        };
                    }
                    s.cursor = 2;
                    self.i2.selected = Some(s);
                }
                Command::Cancel => {
                    let s = self.i2.selected.take().unwrap();
                    self.i2_restore(&s, 0)?;
                    self.session.live = Some(Live {
                        tool: Tool::Extrude,
                        base: s.base.scene,
                        identity_base: Some(s.base.identities),
                        identity_preview: None,
                        params: s.armed_params,
                        applied: false,
                        preview_error: None,
                        pivots: Default::default(),
                        action_frame: s.frame,
                        placed_centre: None,
                    });
                    self.i2.selected = if s.ever_parked {
                        None
                    } else {
                        self.i2.prior.take()
                    };
                    self.i2.prior = None;
                    if let Some(report) = report.as_mut() {
                        report.effect = crate::HistoryEffect::LocalCancel {
                            stage: s.key,
                            from: s.cursor,
                            restored_prior: self.i2.selected.as_ref().map(|prior| prior.key),
                        };
                    }
                }
                Command::Drop => self.i2_finish(false)?,
                Command::Continue => self.i2_finish(true)?,
                Command::ArmSpike
                | Command::ArmEllipsoid
                | Command::ArmShape(_)
                | Command::ArmBridge
                | Command::ArmReduction => {
                    // These categories already stage Drop plus activation in
                    // the legacy dispatcher. Preserve their atomic switch.
                    self.i2_finish(false)?;
                    return Ok(false);
                }
                Command::Activate(tool) => {
                    self.i2_finish(false)?;
                    if *tool != Tool::Extrude {
                        return Ok(false);
                    }
                }
                _ => {
                    return Err("Finish or Cancel the I2 stage before changing this context".into());
                }
            }
            return Ok(true);
        }
        if let Some(s) = self.i2.selected.clone() {
            match cmd {
                // An active tool's own tweak Redo comes first.
                Command::Redo
                    if !(self.tweak_session().is_some() && self.session.tweaks.can_redo())
                        && self.i2_owns_history(HistoryAction::Redo) =>
                {
                    self.i2_redo_eligibility().map_err(|reason| {
                        if let Some(report) = report.as_mut() {
                            report.refusal = Some(reason.clone());
                        }
                        reason.to_string()
                    })?;
                    self.i2_restore(&s, 1)?;
                    self.i2.selected.as_mut().unwrap().cursor = 1;
                    if let Some(report) = report.as_mut() {
                        report.effect = crate::HistoryEffect::Local {
                            action: HistoryAction::Redo,
                            stage: s.key,
                            from: s.cursor,
                            to: 1,
                        };
                    }
                    return Ok(true);
                }
                Command::Cancel
                    if self.session.live.as_ref().is_none_or(|l| {
                        l.tool == Tool::Extrude && !l.applied && l.preview_error.is_none()
                    }) && self.session.cone.is_none()
                        && self.session.ellipsoid.is_none()
                        && self.session.shape.is_none()
                        && self.session.spike.is_none()
                        && self.session.bridge.is_none()
                        && self.session.reduction.is_none() =>
                {
                    self.session.live = None;
                    self.i2_branch();
                    if let Some(report) = report.as_mut() {
                        report.effect = crate::HistoryEffect::ParkedDiscard;
                    }
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }
}

/// Extrude's last-used values (preset_store.rs).
pub enum ExtrudePreset {}
impl crate::PresetKey for ExtrudePreset {
    type Value = crate::Parameters;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn armed() -> Editor {
        let scene = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../docs/i2/expectations/graphs/G0.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut e = Editor {
            scene,
            ..Default::default()
        };
        e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        e.dispatch(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }))
        .unwrap();
        e
    }
    fn armed_fixture(name: &str) -> Editor {
        let scene = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("../../docs/i2/expectations/graphs/{name}.json")),
            )
            .unwrap(),
        )
        .unwrap();
        let mut e = Editor {
            scene,
            ..Default::default()
        };
        e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        e.dispatch(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }))
        .unwrap();
        e
    }
    /// The fast Move path must equal the full Extrude preview exactly, for
    /// every sample it accepts, including signed zero, subnormal and huge
    /// extents and returns to zero (degenerate walls). Out-of-budget samples
    /// must decline so the full preview reports the authoritative error.
    #[test]
    fn i2_fast_move_matches_full_preview_across_extents() {
        let extents = [
            0.5, -0.5, 0.0, -0.0, 1e-300, -1e-300, 5e-324, 0.25, 3.7, -2.1e3, 1e8, -1e8, 0.0, 1e9,
            2e9, -5e9, 0.125,
        ];
        for fixture in ["G0", "P0"] {
            let mut e = armed_fixture(fixture);
            e.dispatch(Command::BeginExtrude).unwrap();
            for z in extents {
                let stage = e.i2.selected.clone().unwrap();
                let mut p = e.session.live.as_ref().unwrap().params.clone();
                p.extent = DVec3::new(0., 0., z);
                let mut fast = e.clone();
                let mut full = e.clone();
                let took = fast.i2_fast_move(&stage, &p);
                let full_result = full.preview(p.clone(), true);
                match took {
                    Ok(true) => {
                        assert_eq!(full_result, Ok(()), "{fixture} {z}");
                        assert!(fast == full, "{fixture} {z}");
                        let mesh = &fast.scene.items[0].mesh;
                        assert_eq!(mesh.validate(), Ok(()));
                        assert_eq!(Mesh::validate(mesh), Ok(()), "full check agrees");
                        for item in &fast.scene.items {
                            let ids = &fast.identities.current.meshes[&item.id];
                            assert_eq!(ids.validate(&item.mesh), Ok(()), "unmemoized layout");
                        }
                    }
                    // Near the 1e9 coordinate budget either path may refuse.
                    Ok(false) => assert!(z.abs() >= 1e9, "{fixture} {z} declined"),
                    Err(error) => assert_eq!(full_result, Err(error), "{fixture} {z}"),
                }
                let _ = e.dispatch(Command::Preview(p));
            }
        }
    }
    #[test]
    fn i2_fast_move_rechecks_named_set_extent_profile() {
        let mut scene = armed_fixture("P0").scene;
        for v in &mut scene.items[0].mesh.vertices {
            *v = DVec3::new(v.z, v.x, v.y);
        }
        let mut e = Editor {
            scene,
            ..Default::default()
        };
        e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        e.dispatch(Command::ConfigureExtrude(Parameters {
            extrude_mode: ExtrudeMode::Shift,
            extrude_uvs: ExtrudeUvs::None,
            ..Default::default()
        }))
        .unwrap();
        e.dispatch(Command::BeginExtrude).unwrap();
        for z in [0., 5e-11, 1e-10, -5e-11, 1.1e-10, 0.5, -0.5] {
            let stage = e.i2.selected.as_ref().unwrap();
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            p.extent = DVec3::new(0., 0., z);
            let mut fast = e.clone();
            let mut full = e.clone();
            let took = fast.i2_fast_move(stage, &p).unwrap();
            let result = full.preview(p, true);
            if z.abs() <= 1e-10 {
                assert!(took, "supported oblique tolerance sample {z} declined");
                assert_eq!(result, Ok(()));
                assert!(fast == full, "oblique sample {z} differs");
                Mesh::validate(&fast.scene.items[0].mesh).unwrap();
                fast.identities.current.validate(&fast.scene).unwrap();
            } else {
                assert!(!took);
                assert!(fast == e, "decline must precede any mutation");
                assert_eq!(
                    result,
                    Err("This tool cannot yet remap typed maps/named sets; source retained".into())
                );
            }
        }
    }
    #[test]
    fn i2_fault_private_reservations_burn_without_publishing_stage_or_alternative() {
        for count in [4, 32] {
            let mut e = armed();
            e.i2.fail_after_reservations = Some(count);
            let before = e.clone();
            assert!(e.dispatch(Command::BeginExtrude).is_err());
            assert!(e == before);
            assert_eq!(e.identities.next_id().unwrap(), 51 + count);
            assert_eq!(e.i2.serial.next().unwrap(), 2);
        }
        let mut e = armed();
        e.dispatch(Command::BeginExtrude).unwrap();
        let mut p = e.session.live.as_ref().unwrap().params.clone();
        p.extent = DVec3::Z * 0.5;
        e.dispatch(Command::Preview(p)).unwrap();
        e.dispatch(Command::Undo).unwrap();
        e.dispatch(Command::Undo).unwrap();
        e.dispatch(Command::Activate(Tool::Extrude)).unwrap();
        e.i2.fail_after_reservations = Some(32);
        let before = e.clone();
        assert!(e.dispatch(Command::BeginExtrude).is_err());
        assert!(e == before);
        assert_eq!(e.identities.next_id().unwrap(), 115);
        assert_eq!(e.i2.serial.next().unwrap(), 3);
        assert_eq!(e.i2.selected.as_ref().unwrap().key, 1);
        assert!(e.i2.prior.is_none());
    }
    #[test]
    fn i2_budget_stage_geometry_request_refusals_precede_reservations() {
        for kind in 0..3 {
            let mut e = armed();
            match kind {
                0 => *e.i2.serial.0.lock().unwrap() = u64::MAX,
                1 => e.identities.reserve_through(u64::MAX - 20).unwrap(),
                _ => e.identities.request = u64::MAX,
            };
            let before = e.clone();
            let marks = (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap());
            assert!(e.dispatch(Command::BeginExtrude).is_err());
            assert!(e == before);
            assert_eq!(
                (e.identities.next_id().unwrap(), e.i2.serial.next().unwrap()),
                marks
            );
        }
    }
}
