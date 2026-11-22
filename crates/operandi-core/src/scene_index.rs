//! A remembered item id → position map for `Scene::item_index`, so the
//! views' per-item lookups (selection, parents, world matrices) cost the
//! same for a million items as for ten instead of a search each.
//!
//! The map is a cache, not part of the scene: copies start empty, it never
//! takes part in equality, and every answer is checked against the item
//! list before it is used, so edits to `Scene::items` anywhere stay safe.
use crate::fxhash::FxHashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct ItemIndex(Mutex<Remembered>);

#[derive(Default)]
struct Remembered {
    at: FxHashMap<u64, u32>,
    /// How many of the list's first items `at` was filled from.
    filled: usize,
}

impl ItemIndex {
    /// Where item `id` is in `items`, if it is there.
    pub(crate) fn find(&self, items: &[crate::Item], id: u64) -> Option<usize> {
        let mut memo = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let check = |memo: &Remembered| {
            memo.at
                .get(&id)
                .copied()
                .filter(|&i| items.get(i as usize).is_some_and(|item| item.id == id))
        };
        if let Some(i) = check(&memo) {
            return Some(i as usize);
        }
        // Items added at the end since: remember just those, so building a
        // big list one item at a time doesn't refill the map each time.
        if memo.filled < items.len() {
            let from = memo.filled;
            memo.at.extend(
                items[from..]
                    .iter()
                    .enumerate()
                    .map(|(k, item)| (item.id, (from + k) as u32)),
            );
            memo.filled = items.len();
            if let Some(i) = check(&memo) {
                return Some(i as usize);
            }
        }
        // Not remembered, or remembered somewhere else: the list changed.
        let found = items.iter().position(|item| item.id == id)?;
        memo.at.clear();
        memo.at.extend(
            items
                .iter()
                .enumerate()
                .map(|(i, item)| (item.id, i as u32)),
        );
        memo.filled = items.len();
        Some(found)
    }
}
impl Clone for ItemIndex {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl PartialEq for ItemIndex {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl std::fmt::Debug for ItemIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ItemIndex")
    }
}

#[cfg(test)]
mod tests {
    use crate::Scene;

    #[test]
    fn lookups_follow_every_edit_of_the_item_list() {
        let mut scene = Scene::default();
        for _ in 0..5 {
            scene.new_mesh().unwrap();
        }
        let ids: Vec<u64> = scene.items.iter().map(|i| i.id).collect();
        for (k, &id) in ids.iter().enumerate() {
            assert_eq!(scene.item_index(id), Some(k));
        }
        // Removed, reordered and copied lists answer from the list itself.
        scene.items.remove(1);
        assert_eq!(scene.item_index(ids[1]), None);
        assert_eq!(scene.item_index(ids[2]), Some(1));
        scene.items.reverse();
        assert_eq!(scene.item_index(ids[0]), Some(3));
        let copy = scene.clone();
        assert_eq!(copy.item_index(ids[4]), Some(0));
        assert_eq!(scene.item(ids[3]).map(|i| i.id), Some(ids[3]));
        assert_eq!(scene.item_index(12345), None);
    }
}
