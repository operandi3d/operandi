//! Duplicate Hierarchy (duplicate-panel-2026-11-20 DX-51): a copy of each
//! selected item together with all its children, at every level, keeping
//! the parent/child links. Copies of children sit under the copy of their
//! parent; each copied branch's root keeps its source's parent. One undo
//! step. Plain Duplicate (`Command::DuplicateItem`) copies the item alone
//! (DXQ-4: that is what it already does).
use crate::{Editor, Item, Result, Scene};
use std::collections::{BTreeMap, BTreeSet};

/// The selected items whose ancestors aren't selected too, in item-list
/// order: each is the root of one copied branch.
fn branch_roots(scene: &Scene) -> Vec<u64> {
    let picked: BTreeSet<u64> = scene.selected_items().into_iter().collect();
    let under_picked = |id: u64| {
        let mut at = scene.item(id).and_then(|i| i.parent);
        for _ in 0..scene.items.len() {
            let Some(p) = at else { return false };
            if picked.contains(&p) {
                return true;
            }
            at = scene.item(p).and_then(|i| i.parent);
        }
        false
    };
    scene
        .items
        .iter()
        .map(|i| i.id)
        .filter(|id| picked.contains(id) && !under_picked(*id))
        .collect()
}

/// `root` and everything under it, in item-list order.
fn branch_ids(scene: &Scene, root: u64) -> Vec<u64> {
    let mut inside: BTreeSet<u64> = BTreeSet::from([root]);
    // Item-list order needn't put parents first, so repeat until settled.
    loop {
        let before = inside.len();
        for item in &scene.items {
            if item.parent.is_some_and(|p| inside.contains(&p)) {
                inside.insert(item.id);
            }
        }
        if inside.len() == before {
            break;
        }
    }
    scene
        .items
        .iter()
        .map(|i| i.id)
        .filter(|id| inside.contains(id))
        .collect()
}

impl Editor {
    /// DX-51 on the scene: refuses with nothing selected and leaves the
    /// scene as it was on any error. The caller records the undo step.
    pub(crate) fn duplicate_hierarchy(&mut self) -> Result<()> {
        let roots = branch_roots(&self.scene);
        if roots.is_empty() {
            return Err("Select an item to duplicate".into());
        }
        let mut scene = self.scene.clone();
        let mut next = scene.next_id.max(self.next_item_id);
        let mut taken: BTreeSet<String> = scene.items.iter().map(|i| i.name.clone()).collect();
        let mut made_roots = Vec::new();
        for root in roots {
            let ids = branch_ids(&self.scene, root);
            if next >= u64::MAX - ids.len() as u64 - 1
                || scene.items.len() + ids.len() > crate::MAX_SCENE_ITEMS
            {
                return Err("Scene item/identity budget exhausted".into());
            }
            let new_id: BTreeMap<u64, u64> = ids
                .iter()
                .enumerate()
                .map(|(k, &id)| (id, next + k as u64))
                .collect();
            next += ids.len() as u64;
            let mut copies: Vec<Item> = Vec::with_capacity(ids.len());
            for id in &ids {
                let source = self.scene.item(*id).ok_or("Missing item")?;
                let mut copy = source.clone();
                copy.id = new_id[id];
                // The branch root keeps its source's parent; the others
                // follow their parent's copy.
                copy.parent = source.parent.map(|p| new_id.get(&p).copied().unwrap_or(p));
                let base = source.name.clone();
                let name = (2..)
                    .map(|n| format!("{base} ({n})"))
                    .find(|n| !taken.contains(n))
                    .unwrap_or(base);
                taken.insert(name.clone());
                copy.name = name;
                copy.source_id = None;
                copy.source_display_name = None;
                // ENV §6, as Duplicate: a copied environment gets its own
                // layer ids.
                if let Some(env) = copy.environment.as_mut() {
                    scene.renumber_environment(env)?;
                }
                copies.push(copy);
            }
            made_roots.push(new_id[&root]);
            scene.items.extend(copies);
        }
        scene.next_id = next;
        // The copies of the branch roots are selected, as Duplicate selects
        // its copy.
        scene.select_item(made_roots.first().copied());
        scene.selection.also = made_roots[1..].to_vec();
        scene.validate()?;
        self.next_item_id = scene.next_id;
        self.scene = scene;
        Ok(())
    }
}
