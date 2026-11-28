//! MP-T11 Polygon Reduction session (RED-001..004). Arming shows Number 0
//! and captures nothing. Initialization (a stationary viewport click, or Apply
//! of a target configured while armed) captures the immutable source scene and
//! identities. Every Number change replaces the displayed preview from that
//! source, so 32→24→32 restores the source exactly. Apply records one edit and
//! re-arms; Drop records the current preview and exits; Cancel restores the
//! source and identities and exits, leaving history and redo untouched.
//! Under D1, Undo/Redo are refused while the task is live, armed included.
use crate::*;

/// Immutable source captured by initialization.
#[derive(Clone, Debug, PartialEq)]
pub struct ReductionSource {
    pub scene: Scene,
    pub identities: IdentityScene,
    /// Source triangle count (the RED-002 "current" count).
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NumericReduction {
    pub parameters: ReductionParameters,
    /// `None` while armed; Number then reads 0.
    pub source: Option<ReductionSource>,
    /// Triangle count of the displayed result. Differs from the requested
    /// Number only when no further valid reduction exists.
    pub achieved: Option<u32>,
}
impl NumericReduction {
    pub fn initialized(&self) -> bool {
        self.source.is_some()
    }
    pub fn current_count(&self) -> Option<u32> {
        self.source.as_ref().map(|s| s.count)
    }
}

impl Editor {
    /// Read-only RED-001 eligibility of the current scene for initialization:
    /// one visible identity-transform active mesh, Polygon mode, neutral
    /// action context and an explicit selection. Data/shape eligibility is
    /// checked by `reduction_source_count`.
    pub fn reduction_target(&self) -> Result<(u64, Vec<u32>)> {
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Polygon Reduction".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Polygon Reduction requires Polygon selection mode".into());
        }
        let item = self
            .scene
            .active()
            .ok_or("Select a mesh before Polygon Reduction")?;
        if !item.visible {
            return Err("Show the mesh before Polygon Reduction".into());
        }
        // Reduction works in the mesh's own space; item transforms don't matter.
        if self.action_context != ActionContext::default() || self.spatial_falloff.is_some() {
            return Err("Basic Polygon Reduction supports the neutral action context without falloff; other contexts are deferred".into());
        }
        // Nothing selected reduces the whole mesh (poly-reduce-2026-11-05).
        let faces = if self.scene.selection.faces.is_empty() {
            (0..item.mesh.faces.len() as u32).collect()
        } else {
            self.scene.selection.faces.clone()
        };
        Ok((item.id, faces))
    }
    pub(crate) fn arm_reduction(&mut self) -> Result<()> {
        self.numeric_arm_guard("Polygon Reduction")?;
        self.session.reduction = Some(NumericReduction::default());
        Ok(())
    }
    /// RED-002 stationary initialization: captures the source and shows its
    /// current count without changing geometry. A click while already
    /// initialized keeps the current session.
    pub(crate) fn initialize_reduction(&mut self) -> Result<()> {
        let session = self
            .session
            .reduction
            .as_ref()
            .ok_or("Arm Polygon Reduction first")?;
        if session.initialized() {
            return Ok(());
        }
        self.scene.validate()?;
        let (item, faces) = self.reduction_target()?;
        let mesh = &self
            .scene
            .item(item)
            .ok_or("Missing Polygon Reduction target")?
            .mesh;
        let total = reduction_source_count(mesh, &faces)?;
        // With part of the mesh selected, Number counts the selected
        // polygons (provisional, Q-RD-01: 8 of 16 selected at Number 8
        // changed nothing, RD08).
        let count = if faces.len() < mesh.faces.len() {
            faces.len() as u32
        } else {
            total
        };
        self.identities.validate_current(&self.scene)?;
        let source = ReductionSource {
            scene: self.scene.clone(),
            identities: self.identities.current.clone(),
            count,
        };
        let session = self
            .session
            .reduction
            .as_mut()
            .ok_or("Arm Polygon Reduction first")?;
        session.source = Some(source);
        session.parameters.number = count;
        session.achieved = Some(count);
        Ok(())
    }
    pub(crate) fn configure_reduction(&mut self, p: ReductionParameters) -> Result<()> {
        p.validate_basic()?;
        let session = self
            .session
            .reduction
            .as_ref()
            .ok_or("Arm Polygon Reduction first")?;
        let Some(count) = session.current_count() else {
            // Armed: retain the target for Apply; 0 remains the status.
            self.session.reduction.as_mut().unwrap().parameters = p;
            return Ok(());
        };
        // Number 0 (the default) or at least the current count changes
        // nothing (poly-reduce-2026-11-05, RD01/RD09).
        let requested = if p.number == 0 || p.number >= count {
            count
        } else {
            p.number
        };
        self.publish_reduction(requested, &p)?;
        self.session.reduction.as_mut().unwrap().parameters = p;
        Ok(())
    }
    /// Replace the displayed preview with the reduction of the immutable
    /// source. The requested source count restores source scene/identities.
    fn publish_reduction(&mut self, requested: u32, p: &ReductionParameters) -> Result<()> {
        let source = self
            .session
            .reduction
            .as_ref()
            .and_then(|s| s.source.clone())
            .ok_or("Initialize Polygon Reduction first")?;
        let previous = (self.scene.clone(), self.identities.current.clone());
        let achieved = if requested == source.count {
            self.scene = source.scene.clone();
            self.identities.current = source.identities.clone();
            source.count
        } else {
            let item = source
                .scene
                .selection
                .item
                .ok_or("Missing Polygon Reduction target")?;
            let source_item = source
                .scene
                .active()
                .ok_or("Missing Polygon Reduction target")?;
            let faces = if source.scene.selection.faces.is_empty() {
                (0..source_item.mesh.faces.len() as u32).collect()
            } else {
                source.scene.selection.faces.clone()
            };
            // A partial selection's target is its own count; the rest of the
            // mesh keeps its polygons.
            let others = source_item.mesh.faces.len() - faces.len();
            let mut outcome =
                reduce_polygons(&source_item.mesh, &faces, requested + others as u32, p)?;
            outcome.achieved = outcome.achieved.saturating_sub(others as u32);
            let ids = source
                .identities
                .meshes
                .get(&item)
                .ok_or("Missing mesh identity owner")?
                .clone();
            let reduced = self.identities.with_ids(|next| {
                ids.reduced(
                    &source_item.mesh,
                    &outcome.mesh,
                    &outcome.correspondence,
                    next,
                )
            })?;
            let mut scene = source.scene.clone();
            scene.selection = reduced_selection(
                &source.scene.selection,
                &outcome.mesh,
                &outcome.correspondence,
            );
            scene
                .active_mut()
                .ok_or("Missing Polygon Reduction target")?
                .mesh = outcome.mesh.into();
            scene.validate()?;
            let mut identities = source.identities.clone();
            identities.meshes.insert(item, reduced.into());
            self.scene = scene;
            self.identities.current = identities;
            outcome.achieved
        };
        if let Err(e) = self.identities.observe(&self.scene) {
            (self.scene, self.identities.current) = previous;
            return Err(e);
        }
        self.session
            .reduction
            .as_mut()
            .ok_or("Arm Polygon Reduction first")?
            .achieved = Some(achieved);
        Ok(())
    }
    /// Record the displayed preview against the captured source, once.
    fn record_reduction(&mut self, source: ReductionSource) {
        if self.scene != source.scene {
            self.identities.record_before = Some(source.identities);
            self.record("Polygon Reduction".into(), source.scene);
        }
    }
    /// RED-004 Apply: one authored edit, then armed for a next session. An
    /// armed target is initialized from the current scene first.
    pub(crate) fn apply_reduction(&mut self) -> Result<()> {
        let session = self
            .session
            .reduction
            .as_ref()
            .ok_or("Arm Polygon Reduction first")?;
        if !session.initialized() {
            let p = session.parameters;
            // Number 0 changes nothing (RD01).
            if p.number == 0 {
                return Ok(());
            }
            self.initialize_reduction()?;
            self.configure_reduction(p)?;
        }
        let session = self
            .session
            .reduction
            .as_mut()
            .ok_or("Arm Polygon Reduction first")?;
        let source = session
            .source
            .take()
            .ok_or("Initialize Polygon Reduction first")?;
        session.parameters.number = 0;
        session.achieved = None;
        self.record_reduction(source);
        Ok(())
    }
    /// RED-004 Drop: publish a changed preview and exit. Armed Drop records
    /// nothing.
    pub(crate) fn drop_reduction(&mut self) -> Result<()> {
        let session = self
            .session
            .reduction
            .take()
            .ok_or("Arm Polygon Reduction first")?;
        if let Some(source) = session.source {
            self.record_reduction(source);
        }
        Ok(())
    }
    /// RED-004 explicit Cancel: discard only the uncommitted preview. History,
    /// redo and earlier Applied edits are untouched.
    pub(crate) fn cancel_reduction(&mut self) -> Result<()> {
        let session = self
            .session
            .reduction
            .take()
            .ok_or("Arm Polygon Reduction first")?;
        if let Some(source) = session.source {
            self.scene = source.scene;
            self.identities.current = source.identities;
            self.identities.observe(&self.scene)?;
        }
        Ok(())
    }
}

/// Every output polygon stays explicitly selected. Remembered point/edge
/// selections follow surviving points and edges; retired ones drop out.
fn reduced_selection(
    selection: &Selection,
    output: &Mesh,
    plan: &ReductionCorrespondence,
) -> Selection {
    let point = |source: u32| plan.vertex_sources.binary_search(&source).ok();
    let edges: std::collections::BTreeSet<(u32, u32)> = output.edges().into_iter().collect();
    Selection {
        item: selection.item,
        mode: selection.mode,
        also: selection.also.clone(),
        // Reduction edits only the primary; other meshes keep theirs.
        others: selection.others.clone(),
        tag: selection.tag.clone(),
        vertices: selection
            .vertices
            .iter()
            .filter_map(|&v| point(v).map(|i| i as u32))
            .collect(),
        edges: selection
            .edges
            .iter()
            .filter_map(|&(a, b)| {
                let (a, b) = (point(a)? as u32, point(b)? as u32);
                edges.contains(&(a.min(b), a.max(b))).then_some((a, b))
            })
            .collect(),
        // The selected indices stay selected (poly-reduce-2026-11-05 rule 7).
        faces: selection
            .faces
            .iter()
            .copied()
            .filter(|&f| (f as usize) < output.faces.len())
            .collect(),
    }
}
