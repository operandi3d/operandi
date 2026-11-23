//! The Duplicate tools' item copier (BEHAVIOR-DUPLICATE-PANEL DX-30..38):
//! one new item per selected mesh item and placement, either an ordinary
//! mesh (Item mode, DS-12) or a Mesh Instance of its source (the Instance
//! buttons), with the Item section's Parent and Hierarchy choices. Also the
//! Item menu's Instance command (DX-38).
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, DX-Q*):
//! - Parent Off keeps each copy at its source's level (a top-level source
//!   gives top-level copies), which is what Item mode did before DX-32.
//! - With Hierarchy on, mesh children of an Instance copy are instances
//!   too; other children are plain copies.
//! - Instance Mirror gives the copy a negative scale (DX-34): the viewport
//!   and Cycles both keep a mirrored item's faces facing out (DXQ-2, checked
//!   in code: `gpu_scene.rs` flips winding for a negative determinant and
//!   Cycles takes the item's matrix as the object transform).
//! - Bounding Box (draw as box) is left out until items have a per-item
//!   box display (UIC-117).
use crate::duplicate::{faces_bounds, item_placement, transforms};
use crate::{Item, Result, Scene};
use glam::{DMat3, DMat4, DVec3};
use std::collections::BTreeSet;

/// Where new items go in the item list (DX-31 Parent).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParentMode {
    /// At the source's own level.
    #[default]
    Off,
    /// A chain: each new item is the child of the one before (the first
    /// is the source's child).
    Hierarchy,
    /// All children of the source.
    Root,
    /// All under a new Group Locator made under the source.
    Primary,
}
impl ParentMode {
    pub const ALL: [ParentMode; 4] = [
        ParentMode::Off,
        ParentMode::Hierarchy,
        ParentMode::Root,
        ParentMode::Primary,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ParentMode::Off => "Off",
            ParentMode::Hierarchy => "Hierarchy",
            ParentMode::Root => "Root",
            ParentMode::Primary => "Primary",
        }
    }
}

/// The Item section of a Duplicate tool (DX-31). Starting values: all off.
/// The Instance switch is `DuplicateParameters::copier` (on for the
/// Instance buttons, off for the plain ones; set when the tool starts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemCopies {
    pub parent: ParentMode,
    /// The source's children (all levels) are copied too, under each copy.
    pub hierarchy: bool,
}

/// Whether the open button makes instances (DX-30).
pub fn instancing(p: &crate::DuplicateParameters) -> bool {
    p.copier == Some(crate::duplicate_copier::Copier::Instance)
}

/// Whether the open button makes Replicators (G8 RP-6).
pub fn replicating(p: &crate::DuplicateParameters) -> bool {
    p.copier == Some(crate::duplicate_copier::Copier::Replica)
}

/// The mesh item `id` shows: its own, or its instance chain's source.
fn shown_mesh_source(scene: &Scene, id: u64) -> u64 {
    scene.instance_source(id).unwrap_or(id)
}

/// The items an item copier works on (DX-11): the selected mesh items in
/// Item mode, the active mesh items in a component mode. An instance
/// counts as a mesh here.
pub fn copier_sources(scene: &Scene) -> Vec<u64> {
    let is_mesh = |id: &u64| scene.item(*id).is_some_and(|i| i.is_mesh());
    if scene.selection.mode == crate::Mode::Item {
        return scene.selected_items().into_iter().filter(is_mesh).collect();
    }
    let ids = scene.active_meshes();
    if ids.is_empty() {
        scene
            .active()
            .map(|i| i.id)
            .into_iter()
            .filter(is_mesh)
            .collect()
    } else {
        ids
    }
}

/// Names after `base` that aren't taken, like Duplicate: "Name (2)", ...
fn copy_name(base: &str, taken: &mut BTreeSet<String>) -> String {
    let name = (2..)
        .map(|n| format!("{base} ({n})"))
        .find(|n| !taken.contains(n))
        .unwrap_or_else(|| base.to_string());
    taken.insert(name.clone());
    name
}

/// A transform from a world matrix under `parent_world`, keeping a
/// negative scale (instances can't re-make their mesh).
pub(crate) fn local_transform(parent_world: DMat4, world: DMat4) -> Result<crate::Transform> {
    let local = parent_world.inverse() * world;
    // A mirroring matrix: split off a mirror on X (where the plain Mirror
    // puts it too), decompose the rest, and keep the mirror as -X scale.
    let mirrored = DMat3::from_mat4(local).determinant() < 0.;
    let flip = DMat4::from_scale(DVec3::new(-1., 1., 1.));
    let m = if mirrored { local * flip } else { local };
    let (mut scale, rotation, translation) = m.to_scale_rotation_translation();
    if mirrored {
        scale.x = -scale.x;
    }
    if !(scale.is_finite() && translation.is_finite()) {
        return Err("The copy's transform isn't finite".into());
    }
    let [x, y, z] = crate::scene::RotationOrder::Xyz.angles(rotation).to_array();
    Ok(crate::Transform {
        translation,
        rotation: DVec3::new(x, y, z),
        scale,
        zxy: false,
        order: None,
    })
}

/// A fresh copy of `item` with id `id`: an instance of `mesh_source` or an
/// ordinary copy.
fn fresh(item: &Item, id: u64, instance_of: Option<u64>) -> Item {
    let mut copy = item.clone();
    copy.id = id;
    copy.source_id = None;
    copy.source_display_name = None;
    if instance_of.is_some() {
        copy.instance_of = instance_of;
    }
    copy
}

/// Hands out item ids.
pub(crate) struct Ids(u64);
impl Ids {
    /// Ids from `next` on (Mesh Paint Instance, mesh_paint.rs).
    pub(crate) fn starting(next: u64) -> Self {
        Self(next)
    }
    /// The next id not yet taken.
    pub(crate) fn next(&self) -> u64 {
        self.0
    }
    pub(crate) fn take(&mut self) -> Result<u64> {
        if self.0 >= u64::MAX - 1 {
            return Err("Scene item/identity budget exhausted".into());
        }
        self.0 += 1;
        Ok(self.0 - 1)
    }
}

/// Copies of `root`'s descendants (all levels, item-list order) under
/// `new_root`, keeping their local transforms. Mesh children become
/// instances when `instance` is set.
fn copy_descendants(
    scene: &Scene,
    root: u64,
    new_root: u64,
    instance: bool,
    ids: &mut Ids,
    taken: &mut BTreeSet<String>,
) -> Result<Vec<Item>> {
    let mut out = Vec::new();
    let mut map = vec![(root, new_root)];
    let mut i = 0;
    while i < map.len() {
        let (old, new) = map[i];
        i += 1;
        for child in scene.items.iter().filter(|c| c.parent == Some(old)) {
            let id = ids.take()?;
            let of = (instance && child.is_mesh()).then(|| shown_mesh_source(scene, child.id));
            let mut copy = fresh(child, id, of);
            copy.parent = Some(new);
            copy.name = copy_name(&child.name, taken);
            out.push(copy);
            map.push((child.id, id));
        }
    }
    Ok(out)
}

/// Item copies (DS-12, DX-30..37): one new item per source and placement,
/// put straight after its source in the item list.
pub(crate) fn item_copies(
    tool: crate::editor::Tool,
    scene: &mut Scene,
    p: &crate::DuplicateParameters,
    next_item_id: u64,
) -> Result<()> {
    let copies = *p.copies(tool).ok_or("Not a Copies tool")?;
    let options = copies.item;
    let sources = copier_sources(scene);
    if replicating(p) && sources.is_empty() {
        return Err(crate::replicator::NOTHING_TO_REPLICATE.into());
    }
    let mut ids = Ids(scene.next_id.max(next_item_id));
    for id in sources {
        let world = scene.world_matrix(id)?;
        let item = scene.item(id).ok_or("Select an item")?;
        let all: Vec<usize> = (0..item.mesh.faces.len()).collect();
        let centre = faces_bounds(scene, id, &all)
            .map_or(world.transform_point3(DVec3::ZERO), |(lo, hi)| {
                (lo + hi) / 2.
            });
        let placed = transforms(tool, p, centre)?;
        if replicating(p) {
            crate::replicator::add(scene, id, &placed, p.replica, &mut ids)?;
            continue;
        }
        place_items(
            scene,
            id,
            &placed,
            &copies,
            options,
            instancing(p),
            &mut ids,
        )?;
    }
    scene.next_id = ids.0;
    Ok(())
}

/// Items for source `id` at world placements `placed` (each applied on top
/// of the source's world matrix).
pub(crate) fn place_items(
    scene: &mut Scene,
    id: u64,
    placed: &[DMat4],
    copies: &crate::Copies,
    options: ItemCopies,
    instance: bool,
    ids: &mut Ids,
) -> Result<()> {
    let index = scene.item_index(id).ok_or("Select an item")?;
    let world = scene.world_matrix(id)?;
    let source_parent = scene.items[index].parent;
    let parent_world = match source_parent {
        Some(parent) => scene.world_matrix(parent)?,
        None => DMat4::IDENTITY,
    };
    let mut new_count = placed.len();
    if options.hierarchy {
        let below = scene.items.iter().filter(|i| i.parent.is_some()).count();
        new_count = new_count.saturating_mul(below + 1);
    }
    if new_count + scene.items.len() + 1 > crate::MAX_SCENE_ITEMS {
        return Err(format!("Too many copies ({} items)", placed.len()));
    }
    let instance_of = instance.then(|| shown_mesh_source(scene, id));
    let mut taken: BTreeSet<String> = scene.items.iter().map(|i| i.name.clone()).collect();
    let base = scene.items[index].name.clone();
    let mut made: Vec<Item> = Vec::new();
    // Primary: one Group Locator under the source holds the copies.
    let group = if options.parent == ParentMode::Primary && !placed.is_empty() {
        let mut g = fresh(&scene.items[index], ids.take()?, None);
        g.name = copy_name(crate::locator::GROUP_LOCATOR_NAME, &mut taken);
        g.mesh = crate::Mesh::default().into();
        g.locator = Some(crate::locator::LocatorKind::Group);
        g.instance_of = None;
        g.subdivision = None;
        g.mesh_stack = None;
        g.elements = Default::default();
        g.transform = Default::default();
        g.parent = Some(id);
        made.push(g);
        Some(made[0].id)
    } else {
        None
    };
    let mut previous: Option<(u64, DMat4)> = None;
    for (slot, t) in placed.iter().enumerate() {
        let source = &scene.items[index];
        // The copy's world matrix: an ordinary copy re-makes a mirrored
        // mesh (MR-47); an instance keeps the mirror in its transform.
        let (copy_world, mesh) = if instance_of.is_some() {
            (*t * world, None)
        } else {
            let (local, mesh) = item_placement(source, parent_world, world, *t)?;
            (parent_world * local.matrix(), Some(mesh))
        };
        if slot == 0 && copies.replace_source {
            // Replace Source: the original takes slot 0's place.
            let transform = local_transform(parent_world, copy_world)?;
            let source = &mut scene.items[index];
            source.transform = transform;
            if let Some(mesh) = mesh {
                source.mesh = mesh.into();
            }
            continue;
        }
        let new_id = ids.take()?;
        let mut copy = fresh(&scene.items[index], new_id, instance_of);
        if let Some(mesh) = mesh {
            copy.mesh = mesh.into();
        }
        let (parent, under) = match options.parent {
            ParentMode::Off => (source_parent, parent_world),
            ParentMode::Root => (Some(id), scene.world_matrix(id)?),
            ParentMode::Primary => (group, scene.world_matrix(id)?),
            ParentMode::Hierarchy => match previous {
                Some((prev, prev_world)) => (Some(prev), prev_world),
                None => (Some(id), scene.world_matrix(id)?),
            },
        };
        copy.parent = parent;
        copy.transform = local_transform(under, copy_world)?;
        copy.name = copy_name(&base, &mut taken);
        previous = Some((new_id, copy_world));
        made.push(copy);
        if options.hierarchy {
            made.extend(copy_descendants(
                scene, id, new_id, instance, ids, &mut taken,
            )?);
        }
    }
    // Straight after the source and its own subtree's first row.
    let at = index + 1;
    scene.items.splice(at..at, made);
    if instance_of.is_some() {
        scene.sync_instances(None);
    }
    Ok(())
}

/// Curve / Bezier / B-Spline / Pen Instance (DX-30): instances of each
/// source item at the path's placements, as Curve Clone places copies of
/// the whole mesh. The Curve Polygon option doesn't apply.
pub(crate) fn path_items(
    scene: &mut Scene,
    p: &crate::DuplicateParameters,
    next_item_id: u64,
) -> Result<()> {
    let tool = &p.curve_clone;
    if !tool.has_path() {
        return Ok(());
    }
    let copies = tool.clone.copies;
    let mut ids = Ids(scene.next_id.max(next_item_id));
    for id in copier_sources(scene) {
        let world = scene.world_matrix(id)?;
        if world.determinant() == 0. {
            return Err("Path tools need an invertible item transform".into());
        }
        let item = scene.item(id).ok_or("Select an item")?;
        if item.mesh.faces.is_empty() {
            continue;
        }
        let all: Vec<usize> = (0..item.mesh.faces.len()).collect();
        let local = crate::path_tools::local_tool(tool, world.inverse(), None);
        // Mesh-space placements, seen in the world.
        let placed: Vec<DMat4> = crate::path_tools::clone_transforms(&item.mesh, &all, &local)?
            .into_iter()
            .map(|t| world * t * world.inverse())
            .collect();
        if replicating(p) {
            crate::replicator::add(scene, id, &placed, p.replica, &mut ids)?;
            continue;
        }
        place_items(scene, id, &placed, &copies, copies.item, true, &mut ids)?;
    }
    scene.next_id = ids.0;
    Ok(())
}

/// DX-38, the Item menu's Instance: one instance of each selected mesh
/// item, in the same place and at the same level, named like Duplicate's
/// copies. The new instances are selected. Returns how many were made.
pub fn instance_selected(scene: &mut Scene, next_item_id: u64) -> Result<usize> {
    let sources: Vec<u64> = scene
        .selected_items()
        .into_iter()
        .filter(|id| scene.item(*id).is_some_and(|i| i.is_mesh()))
        .collect();
    if sources.is_empty() {
        return Err("Select a mesh item to instance".into());
    }
    let mut ids = Ids(scene.next_id.max(next_item_id));
    let first = ids.0;
    for &id in &sources {
        place_items(
            scene,
            id,
            &[DMat4::IDENTITY],
            &crate::Copies::default(),
            ItemCopies::default(),
            true,
            &mut ids,
        )?;
    }
    scene.next_id = ids.0;
    let made: Vec<u64> = (first..ids.0).collect();
    scene.select_item(made.first().copied());
    scene.selection.also = made.iter().skip(1).copied().collect();
    Ok(made.len())
}
