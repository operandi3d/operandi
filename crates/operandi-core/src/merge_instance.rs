//! Merge Instance (Topology tab). Spec: missing-tools-g3 G3-TOPOLOGY-TAB
//! v1, MT12-01..05, case MI-1.
//!
//! The selected source mesh takes its one instance's geometry, placed by
//! the instance's transform relative to the source; the instance item is
//! deleted. Seam points are not welded (MT12-03). Afterwards the source
//! stays selected in Polygon mode with the added polygons selected. One
//! undo step.
//!
//! Q-MI-1 (Jason, 2026-11-21: flip): when that transform mirrors
//! (negative determinant) the added polygons are turned round, so both
//! halves face the same way. Q-MI-2 (provisional): a mesh with two or
//! more instances is refused.
use crate::{Editor, Mesh, Mode, Result, Scene};

/// The refusal when the selection isn't a source mesh with an instance.
pub const NEEDS_INSTANCE: &str = "Select a mesh that has an instance.";
/// The refusal for a mesh with more than one instance (Q-MI-2).
pub const ONE_INSTANCE: &str = "Merge works on a mesh with one instance";

/// Turn the added polygons round when the placement mirrors (Q-MI-1,
/// Jason 2026-11-21). The one spot that decides it.
const FLIP_MIRRORED: bool = true;

impl Editor {
    /// Merge Instance (MT12).
    pub(crate) fn merge_instance(&mut self) -> Result<()> {
        self.require_idle()?;
        let source = self
            .scene
            .active()
            .filter(|i| i.is_mesh() && i.instance_of.is_none())
            .ok_or(NEEDS_INSTANCE)?
            .id;
        let instances: Vec<u64> = self
            .scene
            .items
            .iter()
            .filter(|i| i.instance_of == Some(source))
            .map(|i| i.id)
            .collect();
        let instance = match instances[..] {
            [] => return Err(NEEDS_INSTANCE.into()),
            [one] => one,
            _ => return Err(ONE_INSTANCE.into()),
        };
        let source_world = self.scene.world_matrix(source)?;
        if source_world.determinant().abs() < 1e-15 {
            return Err(
                "The mesh's transform is flat; Merge Instance can't place geometry in it".into(),
            );
        }
        let placement = source_world.inverse() * self.scene.world_matrix(instance)?;
        let mut copy: Mesh = (*self.scene.item(instance).ok_or(NEEDS_INSTANCE)?.mesh).clone();
        for v in &mut copy.vertices {
            *v = placement.transform_point3(*v);
        }
        if FLIP_MIRRORED && placement.determinant() < 0. {
            copy = turned_round(copy)?;
        }
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let mut scene = self.scene.clone();
        let target = scene.item_mut(source).ok_or(NEEDS_INSTANCE)?;
        let first_new = target.mesh.faces.len() as u32;
        crate::append_mesh(&mut target.mesh, copy)?;
        let added: Vec<u32> = (first_new..target.mesh.faces.len() as u32).collect();
        // The instance's children keep their place under its parent.
        let parent = scene.item(instance).and_then(|i| i.parent);
        let children: Vec<u64> = scene
            .items
            .iter()
            .filter(|i| i.parent == Some(instance))
            .map(|i| i.id)
            .collect();
        for child in children {
            scene.set_parent(child, parent, true)?;
        }
        scene.items.retain(|i| i.id != instance);
        scene.select_item(Some(source));
        let selection = &mut scene.selection;
        selection.mode = Mode::Polygon;
        selection.tag = None;
        selection.vertices.clear();
        selection.edges.clear();
        selection.faces = added;
        self.scene = scene;
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.synchronize(&self.scene))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record("Merge Instance".into(), before);
        Ok(())
    }
}

/// Every polygon of `mesh` turned to face the other way (Flip's rule:
/// the first corner stays first, corner data follows).
fn turned_round(mesh: Mesh) -> Result<Mesh> {
    let mut scene = Scene::default();
    scene.new_mesh()?;
    scene.active_mut().ok_or("Missing mesh")?.mesh = mesh.into();
    scene.selection.mode = Mode::Item;
    let flipped = crate::flip::flip_winding(&scene)?;
    Ok((*flipped.active().ok_or("Missing mesh")?.mesh).clone())
}
