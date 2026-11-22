//! BRI-R16: transient configuration, current explicit selection, one atomic Apply.
use crate::*;
impl Editor {
    pub fn bridge_eligibility(&self) -> Result<()> {
        self.scene.validate()?;
        let s = self.bridge.as_ref().ok_or("Arm Bridge first")?;
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
        self.bridge = Some(NumericBridge::armed(target, self.bridge_preset));
        Ok(())
    }
    pub(crate) fn configure_bridge(&mut self, p: BridgeParameters) -> Result<()> {
        if !p.tension.is_finite() || p.twist == i32::MIN {
            return Err("Bridge requires finite Tension and normalized Twist".into());
        }
        self.bridge.as_mut().ok_or("Arm Bridge first")?.requested = p;
        self.bridge_preset = p;
        Ok(())
    }
    pub(crate) fn apply_bridge(&mut self) -> Result<()> {
        // Nothing or a single polygon selected: nothing happens, no message
        // (bridge-2026-11-05, BR18/BR19). The panel still explains why.
        if self.bridge.is_some()
            && self.scene.selection.mode == Mode::Polygon
            && self.scene.selection.faces.len() < 2
        {
            return Ok(());
        }
        self.bridge_eligibility()?;
        let p = self.bridge.as_ref().ok_or("Arm Bridge first")?.requested;
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
        self.record("Bridge Apply".into(), before);
        Ok(())
    }

    /// The scene after bridging the current selection with `p`, without
    /// recording anything: what Apply would make, and the panel's check.
    pub fn bridged_scene(&self, p: BridgeParameters) -> Result<Scene> {
        // N28 (Jason, 2026-11-17): with Symmetry, symmetric picking selects
        // each pair and its mirror; each side's pair is bridged by itself,
        // the far side with its Twist reversed (its mirror image). Pairs on
        // or across the plane are bridged once.
        let symmetry = self.active_symmetry();
        let edges = self.scene.selection.mode == Mode::Edge;
        let mut candidate = self.scene.clone();
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
