//! What the undo history really holds in memory (hidden-limits sweep A1).
//!
//! The history used to count only `size_of::<Item>()` per item per step, so
//! a mesh's points and polygons were never counted: 100 edits of a 5M
//! polygon scan kept about 100 versions of about 1 GB each, and the app ran
//! out of memory instead of trimming. Here each step is charged the bytes
//! of the mesh data only it keeps alive. Copies share data (a point move
//! keeps the old polygons, a checkpoint shares unchanged meshes), so every
//! allocation is counted once: walking the history from the newest step
//! back, a step pays for allocations not already held by the scene, the
//! redo list or a newer step. Trimming the oldest step then frees about
//! what it was charged.
use crate::{Face, Item, MeshSlot, MeshView, Scene, Shared};
use glam::DVec2;
use std::collections::HashSet;
use std::mem::size_of;

/// The bytes of one polygon list, remembered per allocation.
struct FaceBytes(usize);
/// The bytes of one mesh's UV maps, remembered per allocation.
struct UvBytes(usize);

fn faces_bytes(faces: &Shared<Vec<Face>>) -> usize {
    faces
        .derived(|faces| {
            FaceBytes(
                faces.capacity() * size_of::<Face>()
                    + faces
                        .iter()
                        .map(|f| {
                            f.vertices.capacity() * 4
                                + f.uv
                                    .as_ref()
                                    .map_or(0, |u| u.capacity() * size_of::<DVec2>())
                                + f.material.capacity()
                                + f.part.as_ref().map_or(0, String::capacity)
                                + f.material_label.as_ref().map_or(0, String::capacity)
                        })
                        .sum::<usize>(),
            )
        })
        .0
}

type UvMaps = std::collections::BTreeMap<String, Vec<Vec<Option<DVec2>>>>;

fn uv_bytes(maps: &Shared<UvMaps>) -> usize {
    maps.derived(|maps| {
        UvBytes(
            maps.values()
                .map(|map| {
                    map.capacity() * size_of::<Vec<Option<DVec2>>>()
                        + map
                            .iter()
                            .map(|f| f.capacity() * size_of::<Option<DVec2>>())
                            .sum::<usize>()
                })
                .sum(),
        )
    })
    .0
}

/// The allocations already counted.
#[derive(Default)]
pub(crate) struct Tally {
    stamps: HashSet<u64>,
    shared: HashSet<usize>,
}

impl Tally {
    /// Counts `scene`'s allocations not seen yet and returns their bytes.
    /// `previous` is the scene walked just before: items whose mesh is the
    /// same slot at the same place are skipped without hashing, which keeps
    /// a history of big multi-item scenes cheap to walk.
    pub(crate) fn add_scene(&mut self, scene: &Scene, previous: Option<&Scene>) -> usize {
        let mut bytes = scene.items.capacity() * size_of::<Item>();
        for (i, item) in scene.items.iter().enumerate() {
            if previous
                .and_then(|p| p.items.get(i))
                .is_some_and(|p| MeshSlot::same(&p.mesh, &item.mesh))
            {
                continue;
            }
            bytes += self.add_mesh(&item.mesh);
        }
        bytes
    }

    fn add_mesh(&mut self, slot: &MeshSlot) -> usize {
        let first = self.stamps.insert(slot.stamp());
        match slot.read() {
            MeshView::Packed(packed) => {
                if first {
                    packed.heap_bytes()
                } else {
                    0
                }
            }
            MeshView::Open(_) => {
                // Open already: `shared()` doesn't open anything here.
                let open = slot.shared();
                let mut bytes = 0;
                if self.shared.insert(open.addr()) {
                    bytes += open.vertices.capacity() * size_of::<glam::DVec3>()
                        + size_of::<crate::Mesh>();
                }
                if self.shared.insert(open.faces.addr()) {
                    bytes += faces_bytes(&open.faces);
                }
                if self.shared.insert(open.uv_maps.addr()) {
                    bytes += uv_bytes(&open.uv_maps);
                }
                if self.shared.insert(open.attributes.addr()) {
                    bytes += size_of::<crate::MeshAttributes>();
                }
                bytes
            }
        }
    }
}

/// Each undo step's charge, oldest first (see the module comment), and the
/// bytes the current scene and the redo list hold, which aren't charged.
pub(crate) fn step_costs(
    scene: &Scene,
    undo: &[std::sync::Arc<crate::Checkpoint>],
    redo: &[std::sync::Arc<crate::Checkpoint>],
) -> Vec<usize> {
    let mut tally = Tally::default();
    let mut previous = scene;
    tally.add_scene(scene, None);
    for step in redo {
        tally.add_scene(&step.before, Some(previous));
        tally.add_scene(&step.after, Some(&step.before));
        previous = &step.after;
    }
    let mut previous = scene;
    let mut costs: Vec<usize> = undo
        .iter()
        .rev()
        .map(|step| {
            let cost = tally.add_scene(&step.after, Some(previous))
                + tally.add_scene(&step.before, Some(&step.after));
            previous = &step.before;
            cost
        })
        .collect();
    costs.reverse();
    costs
}
