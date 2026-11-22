//! Item groups: named, ordered sets of items of any kind (mesh, camera,
//! light, Backdrop...). An item may be in several groups. Deleting an item
//! takes it out of every group; deleting a group never deletes items. Group
//! names are unique (a taken name gets " (2)" and so on). Light Linking
//! (LP-30..32) points at a group.
//!
//! No behaviour spec exists yet; every choice here is provisional and logged
//! as GRP-Q* in docs/SPECIFICATION_QUESTIONS.md.
use crate::Result;
use crate::scene::Scene;
use serde::{Deserialize, Serialize};

/// One item group.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemGroup {
    /// Stable id (from the scene's item id counter), so renames keep links.
    pub id: u64,
    pub name: String,
    /// Member item ids, in the order they were added.
    pub members: Vec<u64>,
}

/// Longest group name, bytes.
pub const MAX_GROUP_NAME: usize = 256;
/// Most groups a scene holds.
pub const MAX_GROUPS: usize = 4096;
/// Default name of a new group.
pub const GROUP_NAME: &str = "Group";

impl Scene {
    /// The group called `id`.
    pub fn group(&self, id: u64) -> Option<&crate::item_groups::ItemGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// The group's members that still exist, in order.
    pub fn group_members(&self, id: u64) -> Vec<u64> {
        self.group(id).map_or_else(Vec::new, |g| {
            g.members
                .iter()
                .copied()
                .filter(|m| self.item(*m).is_some())
                .collect()
        })
    }

    /// The groups item `item` is in.
    pub fn groups_of(&self, item: u64) -> Vec<u64> {
        self.groups
            .iter()
            .filter(|g| g.members.contains(&item))
            .map(|g| g.id)
            .collect()
    }

    /// `base`, or `base (2)`... so no other group has the name.
    fn free_group_name(&self, base: &str, except: Option<u64>) -> String {
        let taken = |n: &str| {
            self.groups
                .iter()
                .any(|g| g.name == n && Some(g.id) != except)
        };
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|k| format!("{base} ({k})"))
            .find(|n| !taken(n))
            .unwrap_or_else(|| base.to_string())
    }

    fn clean_name(name: &str) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.len() > MAX_GROUP_NAME {
            return Err("Group names need 1–256 bytes".into());
        }
        Ok(name.to_string())
    }

    /// New Group: a group named `name` (or "Group") holding `members` in
    /// that order (unknown ids are left out). Returns its id.
    pub fn new_group(&mut self, name: Option<&str>, members: &[u64]) -> Result<u64> {
        if self.groups.len() >= MAX_GROUPS || self.next_id >= u64::MAX - 1 {
            return Err("Scene group budget exhausted".into());
        }
        let base = Self::clean_name(name.unwrap_or(GROUP_NAME))?;
        let name = self.free_group_name(&base, None);
        let mut list = Vec::new();
        for m in members {
            if self.item(*m).is_some() && !list.contains(m) {
                list.push(*m);
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.groups.push(ItemGroup {
            id,
            name,
            members: list,
        });
        Ok(id)
    }

    pub fn rename_group(&mut self, id: u64, name: &str) -> Result<()> {
        let base = Self::clean_name(name)?;
        let name = self.free_group_name(&base, Some(id));
        self.groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or("Missing group")?
            .name = name;
        Ok(())
    }

    /// Delete Group: the group goes; its items stay.
    pub fn delete_group(&mut self, id: u64) -> Result<()> {
        let before = self.groups.len();
        self.groups.retain(|g| g.id != id);
        if self.groups.len() == before {
            return Err("Missing group".into());
        }
        Ok(())
    }

    /// Add `items` to the group (already-members and unknown ids skipped).
    pub fn add_to_group(&mut self, id: u64, items: &[u64]) -> Result<()> {
        let known: Vec<u64> = items
            .iter()
            .copied()
            .filter(|m| self.item(*m).is_some())
            .collect();
        let g = self
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or("Missing group")?;
        let mut added = false;
        for m in known {
            if !g.members.contains(&m) {
                g.members.push(m);
                added = true;
            }
        }
        if !added {
            return Err("Those items are already in the group".into());
        }
        Ok(())
    }

    pub fn remove_from_group(&mut self, id: u64, items: &[u64]) -> Result<()> {
        let g = self
            .groups
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or("Missing group")?;
        let before = g.members.len();
        g.members.retain(|m| !items.contains(m));
        if g.members.len() == before {
            return Err("None of those items are in the group".into());
        }
        Ok(())
    }

    /// Take deleted items out of every group.
    pub fn prune_groups(&mut self) {
        if self.groups.is_empty() {
            return;
        }
        let ids: std::collections::BTreeSet<u64> = self.items.iter().map(|i| i.id).collect();
        for g in &mut self.groups {
            g.members.retain(|m| ids.contains(m));
        }
    }

    pub(crate) fn validate_groups(&self) -> Result<()> {
        let mut names = std::collections::BTreeSet::new();
        let mut ids = std::collections::BTreeSet::new();
        if self.groups.len() > MAX_GROUPS {
            return Err("Too many groups".into());
        }
        for g in &self.groups {
            if g.name.trim().is_empty()
                || g.name.len() > MAX_GROUP_NAME
                || !names.insert(g.name.as_str())
                || !ids.insert(g.id)
                || self.item(g.id).is_some()
            {
                return Err("Invalid item groups".into());
            }
            let unique: std::collections::BTreeSet<_> = g.members.iter().collect();
            if unique.len() != g.members.len() {
                return Err("A group lists an item twice".into());
            }
        }
        Ok(())
    }
}
