//! Deleting items that have children (Jason, 2026-11-08, GL-Q4).
//!
//! The app asks first: "Do you want to delete the child items too?" Yes
//! deletes each item with everything below it. No deletes only the item;
//! its direct children move to the root of the scene in place (same world
//! placement), each keeping its own hierarchy below it. Scripts and MCP
//! choose without the dialog; their default is to move children to the
//! root (provisional, GL-Q6).
use crate::Result;
use crate::scene::Scene;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// What happens to a deleted item's children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildrenOnDelete {
    /// Delete every descendant too.
    Delete,
    /// Move the direct children to the scene root in place.
    #[default]
    MoveToRoot,
}

impl ChildrenOnDelete {
    /// Script key: "delete" or "root".
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "delete" => Some(Self::Delete),
            "root" => Some(Self::MoveToRoot),
            _ => None,
        }
    }
}

/// The question before the scene's only mesh item is deleted (Jason,
/// 2026-11-21).
pub const LAST_MESH_QUESTION: &str =
    "There is only one mesh item in the scene. Do you really want to delete the last mesh item?";

/// The question before every mesh item of a scene with several is deleted
/// at once (Jason, 2026-11-21).
pub const ALL_MESHES_QUESTION: &str = "The selection holds every mesh item in the scene. Do you really want to delete all the mesh items?";

/// The dialog's question for items about to be deleted.
pub fn children_question(scene: &Scene, ids: &[u64]) -> Option<String> {
    let with_children: Vec<&str> = ids
        .iter()
        .filter(|id| scene.items.iter().any(|i| i.parent == Some(**id)))
        .filter_map(|id| scene.item(*id))
        .map(|i| i.name.as_str())
        .collect();
    match with_children.as_slice() {
        [] => None,
        [name] if ids.len() == 1 => Some(format!(
            "The item {name} has child items. Do you want to delete the child items too?"
        )),
        _ => Some(
            "Some of the selected items have child items. Do you want to delete the child items too?"
                .into(),
        ),
    }
}

impl Scene {
    /// The items deleting `ids` with `children` takes: `ids` (those that
    /// exist), plus every descendant when children are deleted too.
    pub fn deleted_with(&self, ids: &[u64], children: ChildrenOnDelete) -> BTreeSet<u64> {
        let mut doomed: BTreeSet<u64> = ids
            .iter()
            .copied()
            .filter(|id| self.item(*id).is_some())
            .collect();
        if children == ChildrenOnDelete::Delete {
            loop {
                let more: Vec<u64> = self
                    .items
                    .iter()
                    .filter(|i| !doomed.contains(&i.id))
                    .filter(|i| i.parent.is_some_and(|p| doomed.contains(&p)))
                    .map(|i| i.id)
                    .collect();
                if more.is_empty() {
                    break;
                }
                doomed.extend(more);
            }
        }
        doomed
    }

    /// Would deleting `ids` with `children` leave the scene without a mesh
    /// item, taking its only mesh or every one of several? (Jason,
    /// 2026-11-21: the app asks first, with `last_mesh_question`.)
    /// Backdrops, cameras, lights and locators are not mesh items; a scene
    /// with none has nothing to warn about.
    pub fn removes_last_mesh(&self, ids: &[u64], children: ChildrenOnDelete) -> bool {
        let doomed = self.deleted_with(ids, children);
        let mut meshes = self.items.iter().filter(|i| i.is_mesh()).peekable();
        meshes.peek().is_some() && meshes.all(|m| doomed.contains(&m.id))
    }

    /// The question `removes_last_mesh` asks: `LAST_MESH_QUESTION` when
    /// the scene has one mesh item, `ALL_MESHES_QUESTION` when it has
    /// several.
    pub fn last_mesh_question(&self) -> &'static str {
        if self.items.iter().filter(|i| i.is_mesh()).count() > 1 {
            ALL_MESHES_QUESTION
        } else {
            LAST_MESH_QUESTION
        }
    }

    /// Does any of `ids` have children?
    pub fn any_has_children(&self, ids: &[u64]) -> bool {
        self.items
            .iter()
            .any(|i| i.parent.is_some_and(|p| ids.contains(&p)))
    }

    /// Delete `ids`, doing `children` with their children. Returns the
    /// names of children placed by their local transform because keeping
    /// their world placement would need shear. The selection is cleared.
    /// An error leaves the scene unchanged.
    pub fn delete_items(&mut self, ids: &[u64], children: ChildrenOnDelete) -> Result<Vec<String>> {
        let mut scene = self.clone();
        let mut doomed: BTreeSet<u64> = ids
            .iter()
            .copied()
            .filter(|id| scene.item(*id).is_some())
            .collect();
        if doomed.is_empty() {
            return Err("Select an item to delete".into());
        }
        let mut sheared = Vec::new();
        match children {
            ChildrenOnDelete::Delete => loop {
                let more: Vec<u64> = scene
                    .items
                    .iter()
                    .filter(|i| !doomed.contains(&i.id))
                    .filter(|i| i.parent.is_some_and(|p| doomed.contains(&p)))
                    .map(|i| i.id)
                    .collect();
                if more.is_empty() {
                    break;
                }
                doomed.extend(more);
            },
            ChildrenOnDelete::MoveToRoot => {
                // Every surviving item whose parent goes: to the root in
                // place, before anything is removed (world matrices still
                // need the old parents).
                let orphans: Vec<u64> = scene
                    .items
                    .iter()
                    .filter(|i| !doomed.contains(&i.id))
                    .filter(|i| i.parent.is_some_and(|p| doomed.contains(&p)))
                    .map(|i| i.id)
                    .collect();
                for id in orphans {
                    if scene.set_parent(id, None, true).is_err() {
                        scene.set_parent(id, None, false)?;
                        if let Some(item) = scene.item(id) {
                            sheared.push(item.name.clone());
                        }
                    }
                }
            }
        }
        scene.items.retain(|i| !doomed.contains(&i.id));
        scene.select_item(None);
        *self = scene;
        Ok(sheared)
    }
}

/// The status line note for children placed without keeping their place.
pub fn sheared_note(names: &[String]) -> Option<String> {
    (!names.is_empty()).then(|| {
        format!(
            "{} kept its own transform: keeping its place at the root would need shear",
            names.join(", ")
        )
    })
}
