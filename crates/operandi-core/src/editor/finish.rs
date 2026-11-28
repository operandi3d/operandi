//! Apply, Drop and Cancel of the live tool.
use super::*;

impl Editor {
    /// Apply the live tool or numeric session.
    pub(super) fn handle_apply(&mut self) -> Result<AfterCommand> {
        // PF-12: Apply on an Edge Extend still at its Reset values adds the
        // new zero-width row, as a click start does (`extend_from_click`);
        // a live preview at those values leaves the mesh alone.
        if let Some(l) = self.session.live.as_ref()
            && l.tool == Tool::EdgeExtend
            && !l.applied
            && !self.session.extend_from_click
        {
            self.session.extend_from_click = true;
            let p = l.params.clone();
            if let Err(e) = self.dispatch(Command::Preview(p)) {
                self.session.extend_from_click = false;
                return Err(e);
            }
        }
        if let Some(l) = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool.is_smooth_shift())
            && let Some(e) = &l.preview_error
        {
            return Err(format!("Correct this stage before Apply: {e}"));
        }

        if self.session.reduction.is_some() {
            self.apply_reduction()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.bridge.is_some() {
            self.apply_bridge()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.spike.is_some() {
            self.apply_spike()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.ellipsoid.is_some() {
            self.commit_ellipsoid(false)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.shape.is_some() {
            self.commit_shape(false)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.cone.is_some() {
            self.apply_cone()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.live.as_ref().is_some_and(|l| {
            (matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                || l.tool.is_smooth_shift())
                && !l.applied
        }) {
            let mut staged = self.clone();
            let p = staged.session.live.as_ref().unwrap().params.clone();
            let before = staged.scene.clone();
            staged.preview(p.clone(), true)?;
            if staged.scene != before {
                staged.record(
                    staged.session.live.as_ref().unwrap().tool.label().into(),
                    before,
                );
            }
            if staged.session.live.as_ref().unwrap().tool == Tool::Extrude {
                *staged.presets.get_mut::<crate::ExtrudePreset>() = p;
            }
            let base = staged.scene.clone();
            let l = staged.session.live.as_mut().unwrap();
            l.base = base;
            l.identity_base = Some(staged.identities.current.clone());
            l.identity_preview = None;
            l.applied = false;
            *self = staged;
        } else if self
            .session
            .live
            .as_ref()
            .is_some_and(|l| l.tool.is_bevel() && l.applied && l.preview_error.is_none())
        {
            // Panel edits make Bevel live at once (TA-01), so Apply finds a
            // live stage: it keeps that bevel as it is and the next edit or
            // Apply bevels the result again (bevel-activation rule 6).
            let l = self.session.live.as_ref().unwrap();
            let mut next = l.clone();
            if l.base != self.scene {
                self.identities.record_before = l.identity_base.clone();
                let before = l.base.clone();
                self.record(l.tool.label().into(), before);
            }
            next.base = self.scene.clone();
            next.identity_base = Some(self.identities.current.clone());
            next.identity_preview = None;
            next.applied = false;
            self.session.live = Some(next);
        } else if self.session.live.as_ref().is_some_and(|l| {
            matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                || l.tool.is_smooth_shift()
        }) {
            return Err(
                "Apply is unavailable during a live polygon preview; Drop or Continue this stage"
                    .into(),
            );
        } else if let Some(live) = self.session.live.as_ref() {
            // Legacy internal callers remain idempotent. The GUI disables
            // Apply while interactive, as the approved presentation policy.
            let p = live.params.clone();
            self.preview(p, true)?;
        } else {
            let mut staged = self.clone();
            staged.dispatch(Command::Activate(Tool::Box))?;
            staged.dispatch(Command::Preview(
                self.presets.get::<crate::CubePreset>().clone(),
            ))?;
            staged.dispatch(Command::Drop)?;
            *self = staged;
        }
        Ok(AfterCommand::BumpRevision)
    }

    pub(super) fn unit_resolution(&self, t: Tool) -> Option<[u32; 2]> {
        match t {
            Tool::Sphere => self.sphere_unit_resolution,
            Tool::Cylinder => self.cylinder_unit_resolution,
            _ => None,
        }
    }

    pub(super) fn reset_primitive(&mut self) -> Result<()> {
        let live = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool.is_primitive())
            .ok_or("Reset needs a live Cube, Sphere or Cylinder")?;
        let tool = live.tool;
        let mut params = live.params.clone();
        params.segments = Parameters::default().segments;
        params.make_uvs = true;
        params.cylinder_caps = Parameters::default().cylinder_caps;
        params.radial_axis = Parameters::default().radial_axis;
        params.sphere_level = None;
        if let (Some(radial), Some(initial)) = (params.radial.as_mut(), tool.initial_radial()) {
            radial.sides = initial.sides;
            radial.segments = initial.segments;
        }
        let mut staged = self.clone();
        staged.primitive_uvs_off[primitive_slot(tool)] = false;
        match tool {
            Tool::Box => staged.presets.get_mut::<crate::CubePreset>().segments = params.segments,
            Tool::Sphere => {
                staged.sphere_unit_resolution = None;
                staged.sphere_shape = None;
            }
            _ => {
                staged.cylinder_unit_resolution = None;
                staged.cylinder_shape = None;
            }
        }
        staged.set_primitive_params(params)?;
        *self = staged;
        Ok(())
    }

    /// Replace a live primitive's settings: rebuild the preview when
    /// something is drawn, otherwise just store them without creating geometry.
    pub(super) fn set_primitive_params(&mut self, params: Parameters) -> Result<()> {
        let live = self.session.live.as_ref().ok_or("No active primitive")?;
        if self.scene != live.base {
            let mut staged = self.clone();
            staged.dispatch(Command::Preview(params))?;
            *self = staged;
        } else if let Some(l) = self.session.live.as_mut() {
            l.params = params;
        }
        Ok(())
    }

    /// Drop the live tool or numeric session, keeping its result.
    /// PE-M-03 / PE-O-01/03: after Extrude both caps are selected, the new
    /// one and, where Thick (or Auto on an isolated region) keeps it, the
    /// reversed source polygon. During the drag only the moving caps are
    /// selected; each appended moving cap's source is its kept bottom cap.
    pub(crate) fn select_extrude_caps(&mut self, l: &Live) {
        if let Some(faces) = extrude_caps_on_drop(l, &self.scene.selection.faces) {
            self.scene.selection.faces = faces;
        }
    }

    pub(super) fn handle_drop(&mut self) -> Result<AfterCommand> {
        if self.session.reduction.is_some() {
            self.drop_reduction()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.bridge.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.spike.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.ellipsoid.is_some() {
            self.commit_ellipsoid(true)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.shape.is_some() {
            self.commit_shape(true)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.session.cone.is_some() {
            self.drop_cone()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if let Some(error) = self
            .session
            .live
            .as_ref()
            .and_then(|l| l.preview_error.as_ref())
        {
            return Err(format!(
                "Correct the current stage or Cancel before Drop: {error}"
            ));
        }
        let session_base = self.tweak_session_base();
        if let Some(l) = self.session.live.take() {
            // RPT-08: dropping the tool forgets the amount a middle-click
            // repeats.
            self.session.last_transform_step = None;
            if l.tool.is_smooth_shift() {
                self.retain_surface(l.tool, l.params.smooth_shift);
            }
            if l.tool == Tool::Chamfer {
                *self.presets.get_mut::<crate::ChamferPreset>() = l.params.chamfer;
            }
            if matches!(l.tool, Tool::EdgeBevel | Tool::EdgeChamfer) {
                let preset = crate::EdgeBevelPreset {
                    segments: l.params.chamfer.segments,
                    depth: l.params.chamfer.depth,
                    options: l.params.edge_bevel.clone(),
                };
                if l.tool == Tool::EdgeBevel {
                    *self.presets.get_mut::<crate::EdgeBevelToolPreset>() = preset;
                } else {
                    *self.presets.get_mut::<crate::EdgeChamferToolPreset>() = preset;
                }
            }
            if l.tool == Tool::PolyHaul {
                *self.presets.get_mut::<crate::PolyHaulPreset>() = l.params.poly_haul_offset;
            }
            if l.tool == Tool::SketchExtrude {
                *self.presets.get_mut::<crate::SketchPrecisionPreset>() =
                    Some(l.params.sketch_precision);
            }
            if l.tool == Tool::VertexMerge {
                *self.presets.get_mut::<crate::VertexMergePreset>() = Some(l.params.shift);
            }
            if l.tool == Tool::Bend {
                *self.presets.get_mut::<crate::bend::BendPreset>() = Some(l.params.bend);
            }
            if l.tool.is_slice() {
                *self.presets.get_mut::<crate::slice::SlicePreset>() =
                    l.params.slice.options_only();
            }
            if l.tool.is_duplicate() {
                **self.presets.get_mut::<crate::duplicate::DuplicatePreset>() =
                    l.params.duplicate.options_only();
            }
            if l.tool == Tool::Extrude {
                *self.presets.get_mut::<crate::ExtrudePreset>() = l.params.clone();
                if let Some(multi) = self.multi_extrude.take() {
                    // Several meshes: each selects its own caps.
                    if l.applied {
                        for (id, caps) in multi.dropped {
                            self.scene.selection.set_faces_of(id, caps);
                        }
                    }
                } else {
                    self.select_extrude_caps(&l);
                }
            }
            if let Some(p) = l.params.radial {
                match l.tool {
                    Tool::Sphere => {
                        self.sphere_unit_resolution = Some([p.sides, p.segments]);
                        self.sphere_shape = Some((l.params.radial_axis, l.params.sphere_level));
                    }
                    Tool::Cylinder => {
                        self.cylinder_unit_resolution = Some([p.sides, p.segments]);
                        self.cylinder_shape = Some((l.params.cylinder_caps, l.params.radial_axis));
                    }
                    _ => {}
                }
            }
            if l.tool.is_primitive() {
                self.primitive_uvs_off[primitive_slot(l.tool)] = !l.params.make_uvs;
            }
            if l.tool == Tool::Box {
                *self.presets.get_mut::<crate::CubePreset>() = l.params;
            }
            // The whole session is one step: from before its first tweak.
            let (base, identity_base) = match session_base {
                Some((scene, ids)) => (scene, Some(ids)),
                None => (l.base, l.identity_base),
            };
            if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend) {
                self.record_bevel_session(l.tool, base, identity_base);
            } else if base != self.scene {
                self.identities.record_before = identity_base;
                // A tool preset's session is named after the preset.
                let label = self
                    .session
                    .preset
                    .as_ref()
                    .filter(|p| p.tool == l.tool)
                    .map_or(l.tool.label(), crate::PresetSession::label);
                self.record(label.to_string(), base);
            }
        }
        Ok(AfterCommand::BumpRevision)
    }

    /// Cancel the live tool or numeric session, discarding its preview.
    pub(super) fn handle_cancel(
        &mut self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        if let Some(report) = report {
            report.effect = crate::HistoryEffect::Cancel;
        }
        if self.session.reduction.is_some() {
            self.cancel_reduction()?;
        }
        self.cancel_live_bridge()?;
        self.cancel_live_spike()?;
        self.session.bridge = None;
        self.session.spike = None;
        self.session.ellipsoid = None;
        self.session.shape = None;
        self.session.cone = None;
        let session_base = self.tweak_session_base();
        if let Some(l) = self.session.live.take() {
            self.session.last_transform_step = None;
            if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend) {
                // Cancel restores this stage and keeps the bevels finished
                // before it, recorded as one step from the session start.
                self.scene = l.base;
                if let Some(ids) = l.identity_base {
                    self.identities.current = ids;
                    self.identities.observe(&self.scene)?;
                }
                if let Some((start, ids)) = session_base {
                    self.record_bevel_session(l.tool, start, Some(ids));
                }
                return Ok(());
            }
            // Cancel takes back every tweak of the session.
            let (base, ids) = match session_base {
                Some((scene, ids)) => (scene, Some(ids)),
                None => (l.base, l.identity_base),
            };
            self.scene = base;
            if let Some(ids) = ids {
                self.identities.current = ids;
                self.identities.observe(&self.scene)?;
            }
        }
        Ok(())
    }
}
