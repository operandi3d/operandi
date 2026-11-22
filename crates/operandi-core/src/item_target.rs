//! Targeting (LP-04, shared with the camera): an item aims its −Z at a
//! target item's centre, with +Y kept as close to world up as the aim
//! allows, then turned by Roll about the aim. Built for any item kind; the
//! panels show it on cameras and on the light kinds the spec lists.
//!
//! The aim is applied in `Scene::world_matrix`, so the viewport, handles
//! and renders all follow when either item moves. The item's own Rotation
//! stays stored but unused while the target is enabled.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md):
//! - LPROP-Q10: the aimed item keeps its own Scale; a parent's scale or
//!   shear doesn't reach it. Aiming straight up or down keeps world −Z as
//!   the picture's "up" reference.
//! - LPROP-Q11: Set Target refuses the item itself and anything under it
//!   (their place depends on the aim). Targets that aim back at each other
//!   stop after a few levels and keep their own rotation there.
use crate::Result;
use crate::scene::{Item, Scene};
use glam::{DMat4, DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// How deep target lookups go before a target is ignored.
const MAX_AIM_DEPTH: u32 = 8;

/// An item's target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ItemTarget {
    /// The target item's id. A deleted target is ignored.
    pub item: u64,
    /// Enable: off keeps the target but uses the item's own rotation.
    pub enable: bool,
    /// Roll about the aim, radians.
    pub roll: f64,
}

impl Default for ItemTarget {
    fn default() -> Self {
        Self {
            item: 0,
            enable: true,
            roll: 0.0,
        }
    }
}

/// The world rotation that points −Z along `forward` with +Y as near world
/// up as possible, turned by `roll` about the aim.
pub fn aim_rotation(forward: DVec3, roll: f64) -> Option<DQuat> {
    let f = forward.try_normalize()?;
    let yaw = (-f.x).atan2(-f.z);
    let pitch = f.y.clamp(-1.0, 1.0).asin();
    Some(DQuat::from_rotation_y(yaw) * DQuat::from_rotation_x(pitch) * DQuat::from_rotation_z(roll))
}

impl Scene {
    /// The world rotation something drives item `item` to, when its
    /// world matrix (before the drive) is `world`: a Physical Sun first,
    /// else an enabled target.
    pub(crate) fn driven_rotation(
        &self,
        item: &Item,
        world: DMat4,
        aim_depth: u32,
    ) -> Option<DQuat> {
        if let Some(sun) = item
            .light
            .as_ref()
            .and_then(crate::physical_sun::driven_sun_rotation)
        {
            return Some(sun);
        }
        let target = item.common.target.as_ref().filter(|t| t.enable)?;
        if target.item == item.id || aim_depth >= MAX_AIM_DEPTH {
            return None;
        }
        let at = self
            .world_matrix_aimed(target.item, aim_depth + 1)
            .ok()?
            .w_axis
            .truncate();
        aim_rotation(at - world.w_axis.truncate(), target.roll)
    }

    /// Whether `candidate` is `item` or below it in the hierarchy.
    pub fn is_same_or_below(&self, candidate: u64, item: u64) -> bool {
        let mut at = Some(candidate);
        for _ in 0..=self.items.len() {
            match at {
                Some(id) if id == item => return true,
                Some(id) => at = self.item(id).and_then(|i| i.parent),
                None => return false,
            }
        }
        false
    }

    /// Items `item` may aim at (Set Target's list).
    pub fn target_candidates(&self, item: u64) -> Vec<u64> {
        self.items
            .iter()
            .filter(|i| !self.is_same_or_below(i.id, item))
            .map(|i| i.id)
            .collect()
    }

    /// Check Set Target for every item in `ids`.
    pub(crate) fn check_target(&self, ids: &[u64], target: u64) -> Result<()> {
        if !self.item(target).is_some() {
            return Err("Choose a target item".into());
        }
        if ids.iter().any(|id| self.is_same_or_below(target, *id)) {
            return Err("An item can't target itself or an item under it".into());
        }
        Ok(())
    }

    /// Whether item `id`'s rotation is driven (rotation fields greyed).
    pub fn rotation_driven(&self, id: u64) -> bool {
        self.item(id).is_some_and(|i| {
            i.light
                .as_ref()
                .is_some_and(|l| crate::physical_sun::driven_sun_rotation(l).is_some())
                || i.common
                    .target
                    .as_ref()
                    .is_some_and(|t| t.enable && self.item(t.item).is_some())
        })
    }
}
