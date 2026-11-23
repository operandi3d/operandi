//! Create panel Merge group (BEHAVIOR-MERGE.md): Merge and Unmerge of mesh
//! items.
//!
//! - Merge (MRG-01/06/07): the first-selected mesh keeps its name and
//!   transform and takes the other selected meshes' geometry, kept where it
//!   is in the world (converted to the survivor's own space). Nothing is
//!   welded. The other meshes are deleted. Their children keep their world
//!   position under the survivor's parent (MRG-Q06, Jason 2026-11-05: not
//!   the reference's jump). Fewer than two meshes: nothing happens, no
//!   message (MRG-09). One undo step (MRG-10).
//! - Unmerge (UNM-03): each disconnected piece of the selected mesh after
//!   the first becomes a new mesh "<name> (2)", "(3)"… with the same
//!   transform and parent, points unchanged in local space. Pieces touching
//!   at a single point count as one. The original stays selected. One undo
//!   step; a one-piece mesh is left alone.
use crate::clipboard::{KeptFace, subset};
use crate::{Editor, Mesh, Result};
use std::collections::BTreeSet;

/// Groups of polygons joined through shared points, in order of their first
/// polygon. Loose points join no group.
pub fn pieces(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..mesh.faces.len()).collect();
    fn root(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut owner: Vec<Option<usize>> = vec![None; mesh.vertices.len()];
    for (f, face) in mesh.faces.iter().enumerate() {
        for &v in &face.vertices {
            match owner[v as usize] {
                None => owner[v as usize] = Some(f),
                Some(g) => {
                    let (a, b) = (root(&mut parent, f), root(&mut parent, g));
                    if a != b {
                        parent[a.max(b)] = a.min(b);
                    }
                }
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut index = std::collections::BTreeMap::new();
    for f in 0..mesh.faces.len() {
        let r = root(&mut parent, f);
        let at = *index.entry(r).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[at].push(f);
    }
    groups
}

fn whole(mesh: &Mesh, faces: &[usize]) -> Vec<KeptFace> {
    faces
        .iter()
        .map(|&f| KeptFace {
            source: f,
            corners: (0..mesh.faces[f].vertices.len()).collect(),
        })
        .collect()
}

/// `state` renumbered for a piece cut out by `subset`: `point` maps old
/// points to the piece's, and the piece's polygons are `faces` in order.
/// Locked or hidden elements outside the piece are dropped, so the piece
/// never refers to points or polygons it doesn't have (which made a saved
/// scene fail to reopen; found by the monkey sessions, 2026-11-18).
fn piece_state(
    state: &crate::element_state::ElementState,
    point: &[Option<u32>],
    faces: &[KeptFace],
) -> crate::element_state::ElementState {
    use crate::element_state::{ElementMask, ElementState};
    let id = |v: u32| point.get(v as usize).copied().flatten();
    let face: std::collections::HashMap<u32, u32> = faces
        .iter()
        .enumerate()
        .map(|(i, f)| (f.source as u32, i as u32))
        .collect();
    let mask = |m: &ElementMask| ElementMask {
        vertices: m.vertices.iter().filter_map(|&v| id(v)).collect(),
        edges: m
            .edges
            .iter()
            .filter_map(|&(a, b)| {
                let (a, b) = (id(a)?, id(b)?);
                Some((a.min(b), a.max(b)))
            })
            .collect(),
        faces: m
            .faces
            .iter()
            .filter_map(|f| face.get(f).copied())
            .collect(),
    };
    ElementState {
        locked: mask(&state.locked),
        hidden: mask(&state.hidden),
    }
}

impl Editor {
    pub(crate) fn merge_items(&mut self) -> Result<()> {
        self.require_idle()?;
        // The first picked mesh survives (pick order, not the primary).
        let ids: Vec<u64> = self
            .scene
            .picked_items()
            .into_iter()
            .filter(|id| self.scene.items.iter().any(|i| i.id == *id && i.is_mesh()))
            .collect();
        if ids.len() < 2 {
            return Ok(());
        }
        let survivor = ids[0];
        let inverse = {
            let m = self.scene.world_matrix(survivor)?;
            if m.determinant().abs() < 1e-15 {
                return Err(
                    "The first mesh's transform is flat; Merge can't place geometry in it".into(),
                );
            }
            m.inverse()
        };
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let mut scene = self.scene.clone();
        // A survivor under a mesh that merges away moves up, keeping its place.
        loop {
            let parent = scene.item(survivor).and_then(|i| i.parent);
            match parent {
                Some(p) if ids[1..].contains(&p) => {
                    let up = scene.item(p).and_then(|i| i.parent);
                    scene.set_parent(survivor, up, true)?;
                }
                _ => break,
            }
        }
        let survivor_parent = scene.item(survivor).and_then(|i| i.parent);
        for &id in &ids[1..] {
            let to_survivor = inverse * scene.world_matrix(id)?;
            let mut mesh: Mesh = (*scene.item(id).ok_or("Missing mesh")?.mesh).clone();
            for v in &mut mesh.vertices {
                *v = to_survivor.transform_point3(*v);
            }
            let target = scene.item_mut(survivor).ok_or("Missing mesh")?;
            crate::append_mesh(&mut target.mesh, mesh)?;
            // Children keep their world place (MRG-Q06 decided).
            let children: Vec<u64> = scene
                .items
                .iter()
                .filter(|i| i.parent == Some(id))
                .map(|i| i.id)
                .collect();
            for child in children {
                scene.set_parent(child, survivor_parent, true)?;
            }
            scene.items.retain(|i| i.id != id);
        }
        scene.select_item(Some(survivor));
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
        self.record("Merge Meshes".into(), before);
        Ok(())
    }

    pub(crate) fn unmerge_item(&mut self) -> Result<()> {
        self.require_idle()?;
        let item = self
            .scene
            .active()
            .filter(|i| i.is_mesh())
            .ok_or("Select a mesh to Unmerge")?;
        let (id, name) = (item.id, item.name.clone());
        let mesh: Mesh = (*item.mesh).clone();
        let groups = pieces(&mesh);
        if groups.len() < 2 {
            return Ok(());
        }
        if self.scene.items.len() + groups.len() > crate::MAX_SCENE_ITEMS {
            return Err("Scene item budget exhausted".into());
        }
        let before = self.scene.clone();
        let ids_before = self.identities.current.clone();
        let mut scene = self.scene.clone();
        // The original keeps the first piece and its loose points.
        let loose: BTreeSet<u32> = mesh
            .used_points()
            .iter()
            .enumerate()
            .filter(|(_, u)| !**u)
            .map(|(v, _)| v as u32)
            .collect();
        let first_faces = whole(&mesh, &groups[0]);
        let (first, first_points) = subset(&mesh, &first_faces, &loose)?;
        let template = scene.item(id).ok_or("Missing mesh")?.clone();
        let original = scene.item_mut(id).ok_or("Missing mesh")?;
        original.mesh = first.into();
        original.elements = piece_state(&template.elements, &first_points, &first_faces);
        let mut new_ids = Vec::new();
        let mut n = 2;
        for group in &groups[1..] {
            let piece_faces = whole(&mesh, group);
            let (piece, piece_points) = subset(&mesh, &piece_faces, &BTreeSet::new())?;
            let next = scene.next_id.max(self.next_item_id);
            if next >= u64::MAX - 1 {
                return Err("Scene identity budget exhausted".into());
            }
            let piece_name = loop {
                let candidate = format!("{name} ({n})");
                n += 1;
                if scene.items.iter().all(|i| i.name != candidate) {
                    break candidate;
                }
            };
            let mut item = template.clone();
            item.id = next;
            item.name = piece_name;
            item.mesh = piece.into();
            item.elements = piece_state(&template.elements, &piece_points, &piece_faces);
            item.source_id = None;
            item.source_display_name = None;
            // Placed right after the original in the list.
            let at = scene.item_index(id).unwrap_or(0) + 1 + new_ids.len();
            scene.items.insert(at.min(scene.items.len()), item);
            scene.next_id = next + 1;
            self.next_item_id = scene.next_id;
            new_ids.push(next);
        }
        // The original stays selected; its components were renumbered.
        let mode = scene.selection.mode;
        scene.selection = crate::Selection {
            item: Some(id),
            mode,
            ..Default::default()
        };
        self.scene = scene;
        let result = self
            .scene
            .validate()
            .and_then(|()| self.identities.synchronize(&self.scene));
        if let Err(e) = result {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record("Unmerge Mesh".into(), before);
        Ok(())
    }
}
