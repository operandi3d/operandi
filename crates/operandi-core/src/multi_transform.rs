//! Multiple active meshes, stage 2 (Jason, 2026-11-08): Move, Rotate, Scale
//! and Transform act on every active mesh's selected components together,
//! and in Item mode on every selected item, with one handle at the centre
//! of the combined selection (docs/MULTI_ACTIVE_STAGE2_PLAN.md).
//!
//! With one target everything goes through the single-mesh / single-item
//! code exactly as before; these helpers only add the several-target case.
use crate::{Mesh, Mode, Result, Scene};
use glam::DVec3;
use std::collections::BTreeSet;

impl Scene {
    /// The meshes a component-mode transform moves, primary first, or
    /// `None` when that is just the primary (the single-mesh path). With
    /// something of the current mode selected on any active mesh, the
    /// meshes with a selection; with nothing selected anywhere, every
    /// active mesh (all its points, as with one mesh).
    pub fn component_transform_meshes(&self) -> Option<Vec<u64>> {
        let mode = self.selection.mode;
        if mode == Mode::Item {
            return None;
        }
        let active = self.active_meshes();
        let count = |id: u64| match mode {
            Mode::Vertex => self.selection.vertices_of(id).len(),
            Mode::Edge => self.selection.edges_of(id).len(),
            Mode::Polygon => self.selection.faces_of(id).len(),
            Mode::Item => 0,
        };
        let any = active.iter().any(|&id| count(id) > 0);
        let targets: Vec<u64> = active
            .into_iter()
            .filter(|&id| !any || count(id) > 0)
            .collect();
        match targets[..] {
            [] => None,
            [only] if Some(only) == self.selection.item => None,
            _ => Some(targets),
        }
    }
    /// Mesh `id`'s points a component transform moves: its selected points
    /// (edges' and polygons' corners), all of them when it has nothing of
    /// the mode selected (`transform_vertices` for any active mesh).
    pub fn mesh_transform_vertices(&self, id: u64) -> Result<BTreeSet<u32>> {
        if self.selection.item == Some(id) {
            return crate::transform_vertices(self);
        }
        let mut probe = self.clone();
        probe.as_sole_mesh(id, |s| crate::transform_vertices(s))
    }
    /// A probe scene for the action centre of a several-mesh transform:
    /// the target meshes baked into one world-space mesh standing in as the
    /// primary, with their selections renumbered onto it. Centre and axis
    /// rules (Selection, Automatic, Border, Selection axis) then read every
    /// target at once. Never published.
    pub(crate) fn combined_component_probe(&self, targets: &[u64]) -> Result<Scene> {
        let primary = self.selection.item.ok_or("Select a mesh")?;
        let mut mesh = Mesh::default();
        let mut vertices = Vec::new();
        let mut edges = Vec::new();
        let mut faces = Vec::new();
        for &id in targets {
            let item = self.item(id).ok_or("Missing mesh")?;
            let world = self.world_matrix(id)?;
            let offset = mesh.vertices.len() as u32;
            let face_offset = mesh.faces.len() as u32;
            mesh.vertices.extend(
                item.mesh
                    .vertices
                    .iter()
                    .map(|&p| world.transform_point3(p)),
            );
            for face in &item.mesh.faces {
                let mut face = face.clone();
                face.uv = None;
                for v in &mut face.vertices {
                    *v += offset;
                }
                mesh.faces.push(face);
            }
            let s = &self.selection;
            vertices.extend(s.vertices_of(id).iter().map(|v| v + offset));
            edges.extend(
                s.edges_of(id)
                    .iter()
                    .map(|&(a, b)| (a + offset, b + offset)),
            );
            faces.extend(s.faces_of(id).iter().map(|f| f + face_offset));
        }
        let mut probe = self.clone();
        probe.selection.also.clear();
        probe.selection.others.clear();
        probe.selection.tag = None;
        probe.selection.vertices = vertices;
        probe.selection.edges = edges;
        probe.selection.faces = faces;
        let item = probe.item_mut(primary).ok_or("Missing mesh")?;
        item.mesh = mesh.into();
        item.transform = Default::default();
        item.parent = None;
        Ok(probe)
    }

    /// The items an Item-mode transform moves, primary first, or `None`
    /// for just one item (the single-item path). Locked items are left out
    /// (CP-A02), and so is an item below another selected item: it follows
    /// its parent and mustn't move twice.
    pub fn item_transform_targets(&self) -> Option<Vec<u64>> {
        let selected = self.selected_items();
        if selected.len() < 2 {
            return None;
        }
        let chosen: crate::fxhash::FxHashSet<u64> = selected.iter().copied().collect();
        let below_selected = |id: u64| {
            let mut at = self.item(id).and_then(|i| i.parent);
            let mut depth = 0;
            while let Some(p) = at {
                if chosen.contains(&p) {
                    return true;
                }
                depth += 1;
                if depth > 256 {
                    return false;
                }
                at = self.item(p).and_then(|i| i.parent);
            }
            false
        };
        Some(
            selected
                .iter()
                .copied()
                .filter(|&id| {
                    !below_selected(id)
                        && self.item(id).is_some_and(|i| {
                            // ENVIRONMENT-PLACEMENT §2: environments have
                            // no transform.
                            !i.is_locked() && i.environment.is_none()
                        })
                })
                .collect(),
        )
    }
    /// Where each of `targets` rotates and scales about in a several-item
    /// transform, in world space. Provisional (MAM-Q08, Jason's card
    /// pending): one shared centre, the mean of the items' world positions,
    /// which is also where the handle sits. Return each item's own world
    /// position here instead to turn them about their own origins.
    pub fn item_pivots(&self, targets: &[u64]) -> Result<Vec<DVec3>> {
        let positions = targets
            .iter()
            .map(|&id| Ok(self.world_matrix(id)?.w_axis.truncate()))
            .collect::<Result<Vec<DVec3>>>()?;
        let shared = positions.iter().sum::<DVec3>() / positions.len().max(1) as f64;
        Ok(vec![shared; targets.len()])
    }
}

/// An Item-mode Move, Rotate, Scale or Transform of several items: each
/// target's position moves through the shared pivot (`Scene::item_pivots`)
/// in world space, and its rotation and scale channels change as one
/// item's would. `base` is the scene the drag started from.
pub(crate) fn transform_items(
    scene: &mut Scene,
    base: &Scene,
    targets: &[u64],
    tool: crate::Tool,
    p: &crate::Parameters,
) -> Result<()> {
    use crate::Tool;
    if targets.is_empty() {
        return Err("Every selected item is locked (Assembly > Lock)".into());
    }
    let pivots = base.item_pivots(targets)?;
    let (translate, angles, factors) = match tool {
        Tool::Transform => (p.delta, p.rotation_delta, p.scale_delta),
        Tool::Move => (p.delta, DVec3::ZERO, DVec3::ONE),
        Tool::Rotate => (DVec3::ZERO, p.delta, DVec3::ONE),
        Tool::Scale => (DVec3::ZERO, DVec3::ZERO, p.delta),
        _ => return Ok(()),
    };
    let rotation = glam::DQuat::from_euler(
        glam::EulerRot::XYZ,
        angles.x.to_radians(),
        angles.y.to_radians(),
        angles.z.to_radians(),
    );
    for (&id, &pivot) in targets.iter().zip(&pivots) {
        let start = base.world_matrix(id)?.w_axis.truncate();
        let moved = pivot + rotation * ((start - pivot) * factors) + translate;
        // Back into the parent's space (the base parent: it doesn't move,
        // or it's selected and this item isn't a target).
        let parent = base.item(id).and_then(|i| i.parent);
        let local = match parent {
            Some(parent) => base.world_to_local(parent)?.transform_point3(moved),
            None => moved,
        };
        let item = scene.item_mut(id).ok_or("Missing item")?;
        item.transform.translation = local;
        item.transform.rotation += angles.map(f64::to_radians);
        item.transform.scale *= factors;
    }
    Ok(())
}
