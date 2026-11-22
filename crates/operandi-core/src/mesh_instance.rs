//! Mesh Instances: an item whose `instance_of` names a mesh item shows
//! that item's mesh, shares its storage, and keeps its own transform,
//! name, parent and visibility. Edit either and both change.
//!
//! Provisional (built for USD import, docs/BEHAVIOR-USD-IMPORT.md U5;
//! IT-PHY-04 describes making one by Control+Shift drag, not built yet):
//! instead of every reader looking the source up, an instance holds the
//! source's mesh as its own (one shared copy-on-write mesh, so nothing is
//! duplicated in memory), and [`Scene::sync_instances`] puts it back after
//! every recorded edit:
//! - a polygon edit made on an instance (its mesh changed, its source's
//!   didn't) goes to the source, and from there to every other instance;
//! - otherwise every instance takes its source's mesh;
//! - an instance of an instance follows the first source up the chain; an
//!   instance whose source is gone, or isn't a mesh, becomes an ordinary
//!   mesh item keeping the mesh it shows.
use crate::{MeshSlot, Scene};
use std::collections::BTreeMap;

impl Scene {
    /// The mesh item instance `id` finally shows, following instances of
    /// instances; `None` when the chain breaks or loops.
    pub fn instance_source(&self, id: u64) -> Option<u64> {
        let mut at = self.item(id)?.instance_of?;
        for _ in 0..self.items.len() {
            let item = self.item(at)?;
            if !item.is_mesh() {
                return None;
            }
            match item.instance_of {
                Some(next) => at = next,
                None => return (at != id).then_some(at),
            }
        }
        None
    }

    /// Item `id`'s mesh becomes its whole instance group's: its source's
    /// (when `id` is an instance) and every instance's. Nothing happens
    /// for an item outside any group.
    pub fn share_mesh_of(&mut self, id: u64) {
        let Some(item) = self.item(id) else {
            return;
        };
        let source = self.instance_source(id).unwrap_or(id);
        let shared = self.items.iter().any(|i| i.instance_of.is_some())
            && (source != id || self.items.iter().any(|i| i.instance_of == Some(id)));
        if !shared {
            return;
        }
        let mesh = item.mesh.clone();
        if let Some(s) = self.items.iter_mut().find(|i| i.id == source) {
            s.mesh = mesh;
        }
        self.sync_instances(None);
    }

    /// Re-shares every instance's mesh with its source's (see the module
    /// notes). `before` is the scene before the edit, to tell an edit made
    /// on an instance from one made on its source.
    pub fn sync_instances(&mut self, before: Option<&Scene>) {
        if self.items.iter().all(|i| i.instance_of.is_none()) {
            return;
        }
        let sources: BTreeMap<u64, Option<u64>> = self
            .items
            .iter()
            .filter(|i| i.instance_of.is_some())
            .map(|i| (i.id, self.instance_source(i.id)))
            .collect();
        // Edits made on an instance go to its source first.
        let mut pushed: BTreeMap<u64, MeshSlot> = BTreeMap::new();
        if let Some(before) = before {
            for (&id, &source) in &sources {
                let Some(source) = source else { continue };
                if pushed.contains_key(&source) {
                    continue;
                }
                let (Some(now), Some(was), Some(src_now), Some(src_was)) = (
                    self.item(id),
                    before.item(id),
                    self.item(source),
                    before.item(source),
                ) else {
                    continue;
                };
                if !MeshSlot::same(&now.mesh, &was.mesh)
                    && MeshSlot::same(&src_now.mesh, &src_was.mesh)
                    && !MeshSlot::same(&now.mesh, &src_now.mesh)
                {
                    pushed.insert(source, now.mesh.clone());
                }
            }
        }
        for (source, mesh) in pushed {
            if let Some(item) = self.items.iter_mut().find(|i| i.id == source) {
                item.mesh = mesh;
            }
        }
        let meshes: BTreeMap<u64, MeshSlot> = sources
            .values()
            .flatten()
            .filter_map(|&s| self.item(s).map(|i| (s, i.mesh.clone())))
            .collect();
        for item in &mut self.items {
            let Some(&source) = sources.get(&item.id) else {
                continue;
            };
            match source.and_then(|s| meshes.get(&s)) {
                Some(mesh) => {
                    if !MeshSlot::same(&item.mesh, mesh) {
                        item.mesh = mesh.clone();
                    }
                }
                // The source is gone: an ordinary mesh from now on.
                None => item.instance_of = None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Mesh, MeshSlot, Scene};
    use glam::DVec3;

    /// A box and two instances of it.
    fn scene() -> (Scene, u64, u64, u64) {
        let mut s = Scene::default();
        for _ in 0..3 {
            s.new_mesh().unwrap();
        }
        let ids: Vec<u64> = s.items.iter().map(|i| i.id).collect();
        s.items[0].mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
        s.items[1].instance_of = Some(ids[0]);
        s.items[2].instance_of = Some(ids[1]);
        s.sync_instances(None);
        (s, ids[0], ids[1], ids[2])
    }

    #[test]
    fn instances_show_and_share_their_source() {
        let (s, a, b, c) = scene();
        let mesh = |id| &s.item(id).unwrap().mesh;
        assert!(MeshSlot::same(mesh(a), mesh(b)) && MeshSlot::same(mesh(a), mesh(c)));
        assert_eq!(s.instance_source(c), Some(a), "follows the chain");
    }

    #[test]
    fn an_edit_on_either_side_reaches_all() {
        let (mut s, a, b, c) = scene();
        // Edit the source.
        let before = s.clone();
        s.items[0].mesh.shared_mut().vertices[0] = DVec3::splat(9.0);
        s.sync_instances(Some(&before));
        assert_eq!(s.item(c).unwrap().mesh.vertices[0], DVec3::splat(9.0));
        // Edit an instance.
        let before = s.clone();
        s.items[1].mesh.shared_mut().vertices[0] = DVec3::splat(7.0);
        s.sync_instances(Some(&before));
        assert_eq!(s.item(a).unwrap().mesh.vertices[0], DVec3::splat(7.0));
        assert_eq!(s.item(c).unwrap().mesh.vertices[0], DVec3::splat(7.0));
        let _ = b;
    }

    #[test]
    fn deleting_the_source_leaves_ordinary_meshes() {
        let (mut s, a, b, c) = scene();
        s.items.retain(|i| i.id != a);
        s.sync_instances(None);
        assert_eq!(s.item(b).unwrap().instance_of, None);
        assert_eq!(s.item(c).unwrap().instance_of, None);
        assert_eq!(s.item(b).unwrap().mesh.faces.len(), 6);
    }

    #[test]
    fn instances_save_reopen_and_follow_editor_steps() {
        let (s, a, b, c) = scene();
        // Saved as native version 51 and shared again on reopening.
        let bytes = crate::persistence::encode(&s).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("\"version\": 51"), "version");
        let back = crate::persistence::decode(&bytes).unwrap();
        assert_eq!(back.item(b).unwrap().instance_of, Some(a));
        assert!(MeshSlot::same(
            &back.item(a).unwrap().mesh,
            &back.item(c).unwrap().mesh
        ));
        // A recorded edit on an instance reaches the source, and undo
        // takes it back everywhere.
        let mut e = crate::Editor {
            scene: s,
            ..Default::default()
        };
        let before = e.scene.clone();
        let i = e.scene.items.iter().position(|i| i.id == c).unwrap();
        e.scene.items[i].mesh.shared_mut().vertices[1] = DVec3::splat(5.0);
        e.record("Edit".into(), before);
        assert_eq!(e.scene.item(a).unwrap().mesh.vertices[1], DVec3::splat(5.0));
        e.dispatch(crate::Command::Undo).unwrap();
        assert_ne!(e.scene.item(a).unwrap().mesh.vertices[1], DVec3::splat(5.0));
        assert_ne!(e.scene.item(c).unwrap().mesh.vertices[1], DVec3::splat(5.0));
    }
}
