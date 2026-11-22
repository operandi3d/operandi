//! Locator items (GL-01..GL-08): transform-only items with no geometry.
//! Today there is one kind, the Group Locator, an empty item used as a
//! parent to move, rotate and scale other items together. Parenting and
//! `Scene::world_matrix` already make children follow; this module adds
//! the item kind, Add Item support and "Parent to Group Locator".
//!
//! Provisional (GL-Q1..GL-Q3 in docs/SPECIFICATION_QUESTIONS.md): where
//! a new locator goes, its script key, and its shortcut.
use crate::Result;
use crate::scene::{Item, Scene, Transform};
use serde::{Deserialize, Serialize};

/// Which kind of locator an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocatorKind {
    /// An empty parent for grouping items (GL-01).
    Group,
}

/// The new item's name, and the Add Item entry (GL-02).
pub const GROUP_LOCATOR_NAME: &str = "Group Locator";
/// The undo step and menu entry for GL-04.
pub const PARENT_TO_GROUP_LOCATOR: &str = "Parent to Group Locator";

impl Item {
    /// A Group Locator item (no mesh, transform only).
    pub fn is_group_locator(&self) -> bool {
        self.locator == Some(LocatorKind::Group)
    }
}

impl Scene {
    /// Add a Group Locator at the end of the item list, at the origin with
    /// an identity transform. The selection is left alone.
    pub fn add_group_locator(&mut self) -> Result<u64> {
        self.add_group_locator_named(None)
    }
    /// `add_group_locator`, named `name` as it is (see `new_mesh_named`).
    pub fn add_group_locator_named(&mut self, name: Option<String>) -> Result<u64> {
        if self.next_id >= u64::MAX - 1 || self.items.len() >= crate::MAX_SCENE_ITEMS {
            return Err("Scene item/identity budget exhausted".into());
        }
        let name = name.unwrap_or_else(|| self.free_name(GROUP_LOCATOR_NAME));
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Item {
            id,
            name,
            visible: true,
            transform: Transform::default(),
            mesh: crate::Mesh::default().into(),
            parent: None,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator: Some(LocatorKind::Group),
            environment: None,
            common: Default::default(),
            elements: Default::default(),
            hides_children: false,
            mesh_ops: None,
            kit: None,
            instance_of: None,
            replicator: None,
        });
        Ok(id)
    }

    /// GL-04 Parent to Group Locator. The selected items (texture items
    /// skipped) go under one new Group Locator, which ends up the only
    /// selected item. Returns the locator's id.
    ///
    /// - All share one parent: the locator goes under that parent with an
    ///   identity transform, and every child keeps its local transform.
    /// - Mixed parents: the locator is top level at the world origin and
    ///   every child is re-parented in place (same world placement). A
    ///   child that would need shear refuses the whole command.
    ///
    /// The locator takes the list place of the first selected item.
    /// Errors leave the scene unchanged (the caller works on a copy).
    pub fn parent_to_group_locator(&mut self) -> Result<u64> {
        let mut scene = self.clone();
        let locator = scene.parent_to_group_locator_in_place()?;
        *self = scene;
        Ok(locator)
    }

    fn parent_to_group_locator_in_place(&mut self) -> Result<u64> {
        let selected: Vec<u64> = self
            .selected_items()
            .into_iter()
            .filter(|id| self.item(*id).is_some_and(|i| !i.is_texture_item()))
            .collect();
        let Some(&first) = selected.first() else {
            return Err("Select one or more items to parent to a Group Locator".into());
        };
        // Each selected item's parent before anything moves.
        let parents: Vec<Option<u64>> = selected
            .iter()
            .map(|&id| self.item(id).and_then(|i| i.parent))
            .collect();
        let shared_parent = parents[0];
        let shared = parents.iter().all(|p| *p == shared_parent);
        let anchor = self.item_index(first).ok_or("Missing item")?;
        let locator = self.add_group_locator()?;
        let mut item = self.items.pop().ok_or("Missing new Group Locator")?;
        item.parent = if shared { shared_parent } else { None };
        self.items.insert(anchor, item);
        for (&id, parent) in selected.iter().zip(&parents) {
            // Shared parent: the locator sits where the parent's origin is
            // with an identity transform, so local transforms stay exactly.
            // Mixed parents: the locator is the world origin, so a top-level
            // item keeps its transform as it is and a parented one is
            // re-parented in place (SEL-IT-03 keep position; shear refuses).
            let keep_position = !shared && parent.is_some();
            self.set_parent(id, Some(locator), keep_position)
                .map_err(|e| {
                    let name = self.item(id).map(|i| i.name.clone()).unwrap_or_default();
                    format!("{name}: {e}")
                })?;
        }
        self.select_item(Some(locator));
        self.selection.also.clear();
        Ok(locator)
    }
}
