//! BRI-R16: transient configuration, current explicit selection, one atomic Apply.
use crate::*;
impl Editor {
    pub fn bridge_eligibility(&self) -> Result<()> {
        self.scene.validate()?;
        let s = self.session.bridge.as_ref().ok_or("Arm Bridge first")?;
        self.scene
            .active()
            .filter(|i| i.visible && i.id == s.target)
            .ok_or(
                "Bridge needs its visible foreground mesh; empty item selection targets nothing",
            )?;
        let edges = self.scene.selection.mode == Mode::Edge;
        if edges && self.scene.selection.edges.len() < 2 {
            return Err("Bridge needs two selected edge runs".into());
        }
        if !edges
            && (self.scene.selection.mode != Mode::Polygon || self.scene.selection.faces.len() < 2)
        {
            return Err("Bridge needs two separate selected polygons or clusters".into());
        }
        // Bridge builds its polygons from the two selections alone, in the
        // mesh's own space: it has no handle to place and no drag to weight,
        // so the work plane, action center, falloff and bevel profile don't
        // enter and needn't be at their defaults (BRI-Q05, reworked
        // 2026-11-15).
        Ok(())
    }
    pub(crate) fn arm_bridge(&mut self) -> Result<()> {
        self.numeric_arm_guard("Bridge")?;
        let target = self
            .scene
            .active()
            .filter(|i| i.visible)
            .ok_or("Select a visible mesh for Bridge")?
            .id;
        self.session.bridge = Some(NumericBridge::armed(
            target,
            *self.presets.get::<crate::BridgePreset>(),
        ));
        Ok(())
    }
    pub(crate) fn configure_bridge(&mut self, p: BridgeParameters) -> Result<()> {
        if !p.tension.is_finite() || p.twist == i32::MIN {
            return Err("Bridge requires finite Tension and normalized Twist".into());
        }
        let session = self.session.bridge.as_mut().ok_or("Arm Bridge first")?;
        let changed = session.requested != p;
        session.requested = p;
        *self.presets.get_mut::<crate::BridgePreset>() = p;
        // UIC-78: while the last Apply's result is live, a new value
        // reshapes that bridge, still one undo step.
        if changed && self.bridge_result_live() {
            self.remake_bridge(p)?;
        }
        Ok(())
    }

    /// UIC-78: the last Apply's result is still live (`live_apply.rs`).
    pub fn bridge_result_live(&self) -> bool {
        self.live_step_shown(self.session.bridge.as_ref().and_then(|s| s.applied_step))
    }

    /// The cheap half of `bridge_result_live`, for the panel's every-frame
    /// look.
    pub fn bridge_step_newest(&self) -> bool {
        self.live_step_newest(self.session.bridge.as_ref().and_then(|s| s.applied_step))
    }

    /// Bridge the live result's starting selection again with `p`, folded
    /// into the same undo step.
    fn remake_bridge(&mut self, p: BridgeParameters) -> Result<()> {
        let fold = self.session.bridge.as_ref().and_then(|s| s.applied_step);
        let fold = fold.ok_or("No Bridge result")?;
        let last = self
            .history
            .undo
            .last()
            .cloned()
            .ok_or("No Bridge result")?;
        let candidate = self.bridged_from(&last.before, p)?;
        candidate.validate()?;
        let edges = last.before.selection.mode == Mode::Edge;
        let id = candidate.active().ok_or("Missing Bridge target")?.id;
        if edges {
            let ids = self.identities.current.clone();
            if let Some(before) = &last.identity_before {
                self.identities.current = before.clone();
            }
            if let Err(e) = self.identities.invalidate_mesh(&candidate, id) {
                self.identities.current = ids;
                return Err(e);
            }
        }
        let shown = std::mem::replace(&mut self.scene, candidate);
        self.record_folded("Bridge Apply", shown, fold);
        Ok(())
    }

    /// UIC-78 Cancel: take back the live result (its undo step goes too).
    /// The tool's values stay as the preset.
    pub(crate) fn cancel_live_bridge(&mut self) -> Result<()> {
        if !self.bridge_result_live() {
            return Ok(());
        }
        self.pop_live_step()
    }

    pub(crate) fn apply_bridge(&mut self) -> Result<()> {
        // UIC-78: Apply again settles a live result and applies to what is
        // selected now (the result's own selection).
        // Nothing or a single polygon selected: nothing happens, no message
        // (bridge-2026-11-05, BR18/BR19). The panel still explains why.
        if self.session.bridge.is_some()
            && self.scene.selection.mode == Mode::Polygon
            && self.scene.selection.faces.len() < 2
        {
            return Ok(());
        }
        self.bridge_eligibility()?;
        let p = self
            .session
            .bridge
            .as_ref()
            .ok_or("Arm Bridge first")?
            .requested;
        let edges = self.scene.selection.mode == Mode::Edge;
        let id = self.scene.active().ok_or("Missing Bridge target")?.id;
        let candidate = self.bridged_scene(p)?;
        candidate.validate()?;
        let ids = self.identities.current.clone();
        let before = std::mem::replace(&mut self.scene, candidate);
        if edges && let Err(e) = self.identities.invalidate_mesh(&self.scene, id) {
            self.scene = before;
            self.identities.current = ids;
            return Err(e);
        }
        // Its own gesture, so edits while it's live fold into this step.
        let fold = self.live_apply_fold();
        self.record_folded("Bridge Apply", before, fold);
        if let Some(s) = self.session.bridge.as_mut() {
            s.applied_step = Some(fold);
        }
        Ok(())
    }

    /// The scene after bridging the current selection with `p`, without
    /// recording anything: what Apply would make, and the panel's check.
    pub fn bridged_scene(&self, p: BridgeParameters) -> Result<Scene> {
        self.bridged_from(&self.scene, p)
    }

    /// `bridged_scene` from `scene` (with its selection) instead.
    fn bridged_from(&self, scene: &Scene, p: BridgeParameters) -> Result<Scene> {
        // N28 (Jason, 2026-11-17): with Symmetry, symmetric picking selects
        // each pair and its mirror; each side's pair is bridged by itself,
        // the far side with its Twist reversed (its mirror image). Pairs on
        // or across the plane are bridged once.
        let symmetry = self.active_symmetry();
        let edges = scene.selection.mode == Mode::Edge;
        let mut candidate = scene.clone();
        // A pair picked on one side only (picked before Symmetry was on, or
        // a lasso) brings its mirror pair along, as the other Symmetry
        // tools act on both sides (Jason, 2026-11-17). Elements with no
        // mirror partner stay one-sided.
        if symmetry.axis.is_some()
            && let Some(item) = candidate.active().map(|i| i.id)
        {
            let mirror = |c: crate::Component| {
                candidate
                    .symmetric_counterpart(item, c, symmetry)
                    .filter(|&m| m != c)
            };
            if edges {
                let extra: Vec<(u32, u32)> = candidate
                    .selection
                    .edges
                    .iter()
                    .filter_map(|&(a, b)| match mirror(crate::Component::Edge(a, b)) {
                        Some(crate::Component::Edge(x, y)) => Some((x, y)),
                        _ => None,
                    })
                    .collect();
                for e in extra {
                    let key = (e.0.min(e.1), e.0.max(e.1));
                    if !candidate
                        .selection
                        .edges
                        .iter()
                        .any(|&(a, b)| (a.min(b), a.max(b)) == key)
                    {
                        candidate.selection.edges.push(e);
                    }
                }
            } else {
                let extra: Vec<u32> = candidate
                    .selection
                    .faces
                    .iter()
                    .filter_map(|&f| match mirror(crate::Component::Polygon(f)) {
                        Some(crate::Component::Polygon(g)) => Some(g),
                        _ => None,
                    })
                    .collect();
                for g in extra {
                    if !candidate.selection.faces.contains(&g) {
                        candidate.selection.faces.push(g);
                    }
                }
            }
        }
        crate::symmetry_sides::two_pass(&mut candidate, symmetry, |scene, mirrored| {
            let mut p = p;
            if mirrored {
                p.twist = -p.twist;
            }
            let mesh = &scene.active().ok_or("Missing Bridge target")?.mesh;
            if edges {
                // ME-T04: two edge runs or loops of the active mesh.
                let (mesh, edges) = edge_bridge(mesh, &scene.selection.edges, p)?;
                scene.active_mut().ok_or("Missing Bridge target")?.mesh = mesh.into();
                scene.selection.edges = edges;
            } else {
                let result = bridge_geometry(mesh, &scene.selection.faces, p)?;
                scene.selection.faces = result.selected;
                scene.active_mut().ok_or("Missing Bridge target")?.mesh = result.mesh.into();
            }
            Ok(())
        })?;
        Ok(candidate)
    }
}
