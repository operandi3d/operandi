//! SA-010 inactive numeric pipe. Configuration never changes geometry. Each
//! Apply evaluates the current scene, validates a candidate, then records once.
use crate::*;

impl Editor {
    fn spike_target(&self, target: u64) -> Result<()> {
        self.scene.validate()?;
        if self.scene.selection.mode != Mode::Polygon || self.scene.selection.item != Some(target) {
            return Err("Spike requires one identified mesh in Polygon mode".into());
        }
        let item = self
            .scene
            .active()
            .filter(|i| i.visible)
            .ok_or("Select a visible mesh for Spike; empty items target nothing")?;
        let transform = &item.transform;
        let scaled = crate::scene::Transform {
            scale: glam::DVec3::new(2., 3., 4.),
            ..Default::default()
        };
        let placed = crate::scene::Transform {
            translation: glam::DVec3::new(5., -2., 3.),
            rotation: glam::DVec3::new(0., 0., std::f64::consts::FRAC_PI_2),
            ..Default::default()
        };
        if *transform != crate::scene::Transform::default()
            && *transform != scaled
            && *transform != placed
        {
            // Spec: SA-009 / SA-Q02
            return Err("Spike only supports a few tested item transforms".into());
        }
        if item.parent.is_some() || item.transform.scale.min_element() <= 0. {
            // Spec: SA-Q02
            return Err("Spike doesn't support parented, mirrored or flattened items yet".into());
        }
        if self.action_context != ActionContext::default() {
            return Err(
                // Spec: SA-Q02
                "Spike supports only the default action center".into(),
            );
        }
        if item.mesh.faces.is_empty() {
            return Err("Spike requires polygons".into());
        }
        Ok(())
    }
    pub(crate) fn arm_spike(&mut self) -> Result<()> {
        self.numeric_arm_guard("Spike")?;
        let target = self
            .scene
            .active()
            .ok_or("Select a visible mesh for Spike")?
            .id;
        self.spike_target(target)?;
        self.session.spike = Some(NumericSpike::armed(target, self.spike_strength));
        Ok(())
    }
    pub(crate) fn configure_spike(&mut self, strength: f64) -> Result<()> {
        if !strength.is_finite() {
            return Err("Spike Strength must be finite".into());
        }
        let session = self.session.spike.as_mut().ok_or("Arm Spike first")?;
        let changed = session.requested != strength;
        session.requested = strength;
        self.spike_strength = strength;
        // UIC-78: while the last Apply's result is live, a new Strength
        // reshapes it, still one undo step.
        if changed && self.spike_result_live() {
            let fold = self.session.spike.as_ref().and_then(|s| s.applied_step);
            let fold = fold.ok_or("No Spike result")?;
            let last = self.history.undo.last().cloned().ok_or("No Spike result")?;
            let candidate = self.spiked_from(&last.before, strength)?;
            let shown = std::mem::replace(&mut self.scene, candidate);
            self.record_folded("Spike Apply", shown, fold);
        }
        Ok(())
    }

    /// UIC-78: the last Apply's result is still live (`live_apply.rs`).
    pub fn spike_result_live(&self) -> bool {
        self.live_step_shown(self.session.spike.as_ref().and_then(|s| s.applied_step))
    }

    /// The cheap half of `spike_result_live`, for the panel's every-frame
    /// look.
    pub fn spike_step_newest(&self) -> bool {
        self.live_step_newest(self.session.spike.as_ref().and_then(|s| s.applied_step))
    }

    /// UIC-78 Cancel: take back the live result with its undo step.
    pub(crate) fn cancel_live_spike(&mut self) -> Result<()> {
        if !self.spike_result_live() {
            return Ok(());
        }
        self.pop_live_step()
    }

    pub(crate) fn apply_spike(&mut self) -> Result<()> {
        // UIC-78: Apply again settles a live result and applies to what is
        // selected now (the result's own selection).
        let session = self.session.spike.as_ref().ok_or("Arm Spike first")?;
        self.spike_target(session.target)?;
        let candidate = self.spiked_from(&self.scene, session.requested)?;
        let before = std::mem::replace(&mut self.scene, candidate);
        let fold = self.live_apply_fold();
        self.record_folded("Spike Apply", before, fold);
        if let Some(s) = self.session.spike.as_mut() {
            s.applied_step = Some(fold);
        }
        Ok(())
    }

    /// `scene` with its selected polygons spiked by `strength`.
    fn spiked_from(&self, scene: &Scene, strength: f64) -> Result<Scene> {
        let target = self.session.spike.as_ref().ok_or("Arm Spike first")?.target;
        let source = &scene.active().ok_or("Missing Spike target")?.mesh;
        let selected = &scene.selection.faces;
        let weights = self.falloff_weights(scene, target)?;
        let result =
            crate::spike::spike_geometry_weighted(source, selected, strength, weights.as_deref())?;
        let whole_target = selected.is_empty() || selected.len() == source.faces.len();
        let mut candidate = scene.clone();
        candidate.selection.faces = if whole_target {
            Vec::new()
        } else {
            selected
                .iter()
                .flat_map(|&f| result.descendants[f as usize].iter().copied())
                .collect()
        };
        candidate.active_mut().ok_or("Missing Spike target")?.mesh = result.mesh.into();
        candidate.validate()?;
        Ok(candidate)
    }
}
