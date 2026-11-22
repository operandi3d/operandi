//! Display of subdivision surfaces (specifications/subdivision-2026-11-03).
//! Smooth polygons draw their evaluated surface at the mesh's display level,
//! and their wireframe is the cage's own edges traced over that surface (the
//! refined edges between pieces of different cage polygons), as Modo draws
//! it, instead of the flat cage floating off the surface. Evaluation is
//! cached per item and redone only when that item's mesh or settings change.
use operandi_core::subdivision::{SubdivisionSettings, display_surface_edges};
use operandi_core::{Item, Mesh, Shared};
use std::collections::BTreeMap;
use std::sync::Arc;

/// An evaluated surface.
#[derive(Clone)]
pub(super) struct Evaluated {
    pub mesh: Mesh,
    /// The cage polygon each refined polygon comes from.
    pub sources: Vec<usize>,
    /// Per refined polygon, per corner: whether the edge from that corner to
    /// the next lies on a cage edge (borders another cage polygon's pieces,
    /// or the surface's open edge).
    pub traced: Vec<Vec<bool>>,
    /// The same sides naming their cage edge (low, high point numbers).
    pub cage_edges: Vec<Vec<operandi_core::subdivision::Side>>,
    /// Per cage polygon, the range of its pieces in `mesh.faces`.
    pub blocks: Vec<std::ops::Range<usize>>,
    /// The pieces using each refined point, for smooth shading normals.
    /// Built with the surface; a drag only moves points, so it stays valid.
    pub point_pieces: operandi_core::topology_cache::PointFaces,
    /// Per cage point, the surface point it shows at (`NO_POINT` when it
    /// stays on the cage). Topology only, so a drag that moves points
    /// keeps it; built on first use.
    pub cage_points: std::sync::OnceLock<Vec<u32>>,
}
pub(super) type Surface = Arc<Evaluated>;

type Entry = (Shared<Mesh>, SubdivisionSettings, Surface);

/// Interior mutability so picking (which borrows the app immutably) shares
/// the evaluations the viewport draws instead of evaluating again.
#[derive(Default)]
pub(super) struct SubdivisionCache {
    entries: std::cell::RefCell<BTreeMap<u64, Entry>>,
    /// Whole-surface evaluations so far (speed guards, Frame Time).
    full_builds: std::cell::Cell<u64>,
    /// Partial re-evaluations near moved points (speed guards, Frame Time).
    updates: std::cell::Cell<u64>,
    /// Subdivision polygons as patches for the graphics card, per item
    /// (subdivision::patches), with the mesh and level they came from.
    patches: std::cell::RefCell<BTreeMap<u64, PatchEntry>>,
    /// Items whose surface couldn't be evaluated (too big for the CPU
    /// path), with the mesh and settings that failed, so it isn't tried
    /// again every frame.
    failed: std::cell::RefCell<BTreeMap<u64, (Shared<Mesh>, SubdivisionSettings)>>,
}
type PatchEntry = (
    Shared<Mesh>,
    u8,
    Option<(
        operandi_core::subdivision::patches::PatchState,
        Arc<operandi_core::subdivision::patches::PatchNet>,
    )>,
);
impl SubdivisionCache {
    pub fn full_builds(&self) -> u64 {
        self.full_builds.get()
    }
    pub fn updates(&self) -> u64 {
        self.updates.get()
    }
    /// The item's smooth surface, or `None` when it can't be evaluated (the
    /// cage still draws).
    pub fn get(&self, item: &Item) -> Option<Surface> {
        let settings = item.subdivision.clone().unwrap_or_default();
        let mut entries = self.entries.borrow_mut();
        if let Some((mesh, cached, surface)) = entries.get(&item.id)
            && *mesh == item.mesh
            && *cached == settings
        {
            return Some(surface.clone());
        }
        // Only points moved (a drag): recompute the surface near them. The
        // surface carries the cage's UV maps, so changed UVs rebuild it.
        if let Some((mesh, cached, surface)) = entries.get_mut(&item.id)
            && *cached == settings
            && (operandi_core::Shared::same(&mesh.uv_maps, &item.mesh.uv_maps)
                || mesh.uv_maps == item.mesh.uv_maps)
        {
            let evaluated = Arc::make_mut(surface);
            if operandi_core::subdivision::update_surface(
                mesh,
                &item.mesh,
                &settings,
                &mut evaluated.mesh,
                &evaluated.blocks,
            )
            .is_some()
            {
                self.updates.set(self.updates.get() + 1);
                *mesh = item.mesh.clone().into();
                return Some(surface.clone());
            }
        }
        // Still too big: the same mesh, or one whose polygons and kinds are
        // the very same (a drag only moves points).
        if let Some((mesh, cached)) = self.failed.borrow().get(&item.id)
            && *cached == settings
            && (Shared::same(mesh, item.mesh.shared())
                || (operandi_core::Shared::same(&mesh.faces, &item.mesh.faces)
                    && (operandi_core::Shared::same(&mesh.attributes, &item.mesh.attributes)
                        || mesh.attributes.representations
                            == item.mesh.attributes.representations)))
        {
            return None;
        }
        self.full_builds.set(self.full_builds.get() + 1);
        // The stored surface only when it fits in half the free memory
        // (in place of the old fixed 4M-polygon cap); else the cage picks
        // and, unless patches draw it, shows.
        let bytes = operandi_core::subdivision::display_pieces(&item.mesh, &settings)
            .saturating_mul(operandi_core::subdivision::STORED_BYTES_PER_PIECE);
        let fits = bytes <= crate::free_memory::free_bytes() / 2;
        let Some((mesh, sources, cage_edges)) = fits
            .then(|| display_surface_edges(&item.mesh, &settings).ok())
            .flatten()
        else {
            self.failed
                .borrow_mut()
                .insert(item.id, (item.mesh.shared().clone(), settings));
            return None;
        };
        let traced = cage_edges
            .iter()
            .map(|f| f.iter().map(Option::is_some).collect())
            .collect();
        let blocks = operandi_core::subdivision::source_blocks(&sources, item.mesh.faces.len());
        let point_pieces = operandi_core::topology_cache::PointFaces::new(&mesh);
        let surface = Arc::new(Evaluated {
            cage_points: Default::default(),
            point_pieces,
            mesh,
            sources,
            traced,
            cage_edges,
            blocks,
        });
        entries.insert(
            item.id,
            (item.mesh.clone().into(), settings, surface.clone()),
        );
        Some(surface)
    }
    /// The item's Subdivision polygons as patches, when it draws that way
    /// (subdivision::patches::eligible): built once per mesh and level.
    pub fn patches(
        &self,
        item: &Item,
    ) -> Option<Arc<operandi_core::subdivision::patches::PatchNet>> {
        use operandi_core::subdivision::patches;
        let settings = item.subdivision.clone().unwrap_or_default();
        let level = settings.subdivision_level;
        let mut entries = self.patches.borrow_mut();
        if let Some((mesh, l, made)) = entries.get_mut(&item.id)
            && *l == level
        {
            if Shared::same(mesh, item.mesh.shared()) {
                return made.as_ref().map(|(_, net)| net.clone());
            }
            // Only points moved (a drag): update the patches near them.
            if let Some(moved) = patches::moved(mesh, &item.mesh) {
                *mesh = item.mesh.shared().clone();
                let Some((state, net)) = made else {
                    return None;
                };
                if !moved.is_empty() {
                    *net = Arc::new(state.update(net, &item.mesh, &moved));
                    self.updates.set(self.updates.get() + 1);
                }
                return Some(net.clone());
            }
        }
        // Smaller surfaces keep the stored path (subdivision::PATCH_MIN_PIECES).
        let made = operandi_core::subdivision::draws_as_patches(
            &item.mesh,
            &settings,
            crate::subd_gpu::min_pieces(),
        )
        .then(|| patches::PatchState::new(&item.mesh, level))
        .flatten()
        .map(|(state, net)| (state, Arc::new(net)));
        let net = made.as_ref().map(|(_, net)| net.clone());
        entries.insert(item.id, (item.mesh.shared().clone(), level, made));
        net
    }
    /// The item's smooth surface when it has smooth polygons.
    pub fn smooth(&self, item: &Item) -> Option<Surface> {
        operandi_core::subdivision::has_smooth(&item.mesh)
            .then(|| self.get(item))
            .flatten()
    }
}

/// A cage point with no place of its own on the surface.
pub(super) const NO_POINT: u32 = u32::MAX;

impl SubdivisionCache {
    /// The item's points where they show, in the mesh's own coordinates:
    /// a point whose every polygon is smooth sits on the smooth surface
    /// (where its traced edges meet), any other on the cage. `None` when
    /// the item has no smooth surface (its points are the cage's).
    pub fn shown_points(&self, item: &Item) -> Option<Vec<glam::DVec3>> {
        let surface = self.smooth(item)?;
        let map = surface.cage_points(&item.mesh);
        Some(
            item.mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(v, &p)| match map.get(v) {
                    Some(&s) if s != NO_POINT => surface.mesh.vertices[s as usize],
                    _ => p,
                })
                .collect(),
        )
    }
}

impl SubdivisionCache {
    /// One point of `shown_points` (without building the whole list).
    pub fn shown_point(&self, item: &Item, v: u32) -> Option<glam::DVec3> {
        let cage = *item.mesh.vertices.get(v as usize)?;
        let Some(surface) = self.smooth(item) else {
            return Some(cage);
        };
        Some(match surface.cage_points(&item.mesh).get(v as usize) {
            Some(&s) if s != NO_POINT => surface.mesh.vertices[s as usize],
            _ => cage,
        })
    }
}

impl Evaluated {
    /// Per cage point of `cage` (the mesh this surface was evaluated
    /// from), the surface point it shows at, or `NO_POINT`. The surface
    /// point is where the traced pieces of two of the point's own cage
    /// edges meet; only points whose every polygon is smooth move there.
    pub fn cage_points(&self, cage: &Mesh) -> &[u32] {
        self.cage_points.get_or_init(|| {
            let count = cage.vertices.len();
            let mut out = vec![NO_POINT; count];
            // Points with a polygon that isn't smooth stay on the cage.
            let mut smooth = vec![false; count];
            let mut flat = vec![false; count];
            for (f, face) in cage.faces.iter().enumerate() {
                let on =
                    operandi_core::subdivision::is_smooth_polygon(cage, f) && !cage.is_polyline(f);
                for &v in &face.vertices {
                    if let Some(slot) = if on {
                        smooth.get_mut(v as usize)
                    } else {
                        flat.get_mut(v as usize)
                    } {
                        *slot = true;
                    }
                }
            }
            // Per surface point, the first cage edge a traced side through
            // it lies on; a second, different one makes it a cage corner.
            let mut first: operandi_core::fxhash::FxHashMap<u32, (u32, u32)> = Default::default();
            for (sf, sides) in self.cage_edges.iter().enumerate() {
                let v = &self.mesh.faces[sf].vertices;
                for (i, side) in sides.iter().enumerate() {
                    let Some(edge) = *side else { continue };
                    for end in [v[i], v[(i + 1) % v.len()]] {
                        match first.get(&end) {
                            None => {
                                first.insert(end, edge);
                            }
                            Some(&other) if other != edge => {
                                // Two cage edges share at most one point.
                                let corner = if edge.0 == other.0 || edge.0 == other.1 {
                                    edge.0
                                } else if edge.1 == other.0 || edge.1 == other.1 {
                                    edge.1
                                } else {
                                    continue;
                                };
                                let c = corner as usize;
                                if c < count && smooth[c] && !flat[c] {
                                    out[c] = end;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            out
        })
    }
    /// Each of `wanted` cage edges as segments on the surface, in the
    /// mesh's own coordinates (each segment once). Edges with no smooth
    /// polygon next to them aren't listed.
    pub fn cage_edge_paths(
        &self,
        cage: &Mesh,
        wanted: &std::collections::BTreeSet<(u32, u32)>,
    ) -> BTreeMap<(u32, u32), Segments> {
        let mut out: BTreeMap<(u32, u32), Segments> = BTreeMap::new();
        if wanted.is_empty() {
            return out;
        }
        // Only the pieces of cage polygons using a wanted edge's ends can
        // lie on it (a drag draws its new edges every frame; scanning every
        // piece of a dense surface cost more than the rest of the overlay).
        let point_faces = cage.point_faces();
        let mut blocks: Vec<std::ops::Range<usize>> = wanted
            .iter()
            .flat_map(|&(a, _)| point_faces.of(a).iter())
            .filter_map(|&f| self.blocks.get(f as usize).cloned())
            .collect();
        // In surface order, as a scan of every piece would visit them.
        blocks.sort_unstable_by_key(|r| r.start);
        blocks.dedup();
        let mut seen = std::collections::BTreeSet::new();
        for sf in blocks.into_iter().flatten() {
            let sides = &self.cage_edges[sf];
            let v = &self.mesh.faces[sf].vertices;
            for (i, side) in sides.iter().enumerate() {
                let Some(edge) = side.filter(|e| wanted.contains(e)) else {
                    continue;
                };
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                if seen.insert((a.min(b), a.max(b))) {
                    out.entry(edge).or_default().push((
                        self.mesh.vertices[a as usize],
                        self.mesh.vertices[b as usize],
                    ));
                }
            }
        }
        out
    }
    /// Every traced side as (cage edge, segment), in the mesh's own
    /// coordinates (each segment once), for picking edges on the surface.
    pub fn all_edge_paths(&self) -> Vec<((u32, u32), (glam::DVec3, glam::DVec3))> {
        let mut out = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for (sf, sides) in self.cage_edges.iter().enumerate() {
            let v = &self.mesh.faces[sf].vertices;
            for (i, side) in sides.iter().enumerate() {
                let Some(edge) = *side else { continue };
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                if seen.insert((a.min(b), a.max(b))) {
                    out.push((
                        edge,
                        (
                            self.mesh.vertices[a as usize],
                            self.mesh.vertices[b as usize],
                        ),
                    ));
                }
            }
        }
        out
    }
}

/// Line segments, end to end.
pub(super) type Segments = Vec<(glam::DVec3, glam::DVec3)>;

/// Cage edges `wanted` of `item` as world segments (through `matrix`):
/// on the smooth surface where the edge borders a smooth polygon, else the
/// cage line itself. Edges the mesh doesn't have are skipped.
pub(super) fn edge_segments(
    cache: &SubdivisionCache,
    item: &Item,
    matrix: glam::DMat4,
    wanted: &std::collections::BTreeSet<(u32, u32)>,
) -> Vec<((u32, u32), Segments)> {
    let count = item.mesh.vertices.len() as u32;
    let mut on_surface = if wanted.is_empty() || !operandi_core::subdivision::has_smooth(&item.mesh)
    {
        BTreeMap::new()
    } else {
        cache
            .get(item)
            .map(|surface| surface.cage_edge_paths(&item.mesh, wanted))
            .unwrap_or_default()
    };
    wanted
        .iter()
        .filter(|&&(a, b)| a < count && b < count)
        .map(|&edge| {
            let local = on_surface.remove(&edge).unwrap_or_else(|| {
                vec![(
                    item.mesh.vertices[edge.0 as usize],
                    item.mesh.vertices[edge.1 as usize],
                )]
            });
            let world = local
                .into_iter()
                .map(|(a, b)| (matrix.transform_point3(a), matrix.transform_point3(b)))
                .collect();
            (edge, world)
        })
        .collect()
}

/// Cage edge `edge` (low, high) of `item` as a world polyline from its low
/// point to its high one, where it shows: traced on the smooth surface
/// when it borders a smooth polygon, else the cage line.
pub(super) fn edge_path(
    cache: &SubdivisionCache,
    item: &Item,
    matrix: glam::DMat4,
    edge: (u32, u32),
) -> Vec<glam::DVec3> {
    let mut segments = edge_segments(cache, item, matrix, &[edge].into())
        .pop()
        .map(|(_, s)| s)
        .unwrap_or_default();
    let Some(start) = cache
        .shown_point(item, edge.0)
        .map(|p| matrix.transform_point3(p))
    else {
        return Vec::new();
    };
    // Chain the pieces end to end from the low point (each step takes the
    // piece with an end nearest the chain's end).
    let mut path = vec![start];
    while !segments.is_empty() {
        let end = *path.last().unwrap();
        let (i, flip) = segments
            .iter()
            .enumerate()
            .flat_map(|(i, &(a, b))| [(i, false, a), (i, true, b)])
            .min_by(|x, y| {
                x.2.distance_squared(end)
                    .total_cmp(&y.2.distance_squared(end))
            })
            .map(|(i, flip, _)| (i, flip))
            .unwrap();
        let (a, b) = segments.swap_remove(i);
        let (near, far) = if flip { (b, a) } else { (a, b) };
        if path.len() == 1 {
            path[0] = near;
        }
        path.push(far);
    }
    path
}

/// Where `t` (0..1 from `piece[0]`) on `piece`, one piece of `path`, lies
/// along the whole cage edge, 0..1 from the path's start. The pieces of a
/// traced edge each stand for an equal share of it (refinement splits a
/// cage edge evenly). A piece not on the path gives `t` itself.
pub(super) fn path_parameter(path: &[glam::DVec3], piece: [glam::DVec3; 2], t: f64) -> f64 {
    let pieces = path.len().saturating_sub(1);
    let close = |x: glam::DVec3, y: glam::DVec3| {
        x.distance_squared(y) <= 1e-18_f64.max(x.length_squared() * 1e-20)
    };
    for k in 0..pieces {
        let (a, b) = (path[k], path[k + 1]);
        if close(a, piece[0]) && close(b, piece[1]) {
            return (k as f64 + t) / pieces as f64;
        }
        if close(a, piece[1]) && close(b, piece[0]) {
            return (k as f64 + 1. - t) / pieces as f64;
        }
    }
    t
}

/// A world point `t` (0..1) along `path`, each piece an equal share.
pub(super) fn path_point(path: &[glam::DVec3], t: f64) -> Option<glam::DVec3> {
    let pieces = path.len().checked_sub(1).filter(|&n| n > 0)?;
    let at = t.clamp(0., 1.) * pieces as f64;
    let k = (at.floor() as usize).min(pieces - 1);
    Some(path[k].lerp(path[k + 1], at - k as f64))
}

/// The traced edges found by adjacency, for checking the refinement's own
/// tracking.
#[cfg(test)]
fn traced_edges(mesh: &Mesh, sources: &[usize]) -> Vec<Vec<bool>> {
    // Up to two polygons and the use count per edge (fast hash: this runs on
    // every frame of a drag on a subdivided mesh).
    let corners: usize = mesh.faces.iter().map(|f| f.vertices.len()).sum();
    let mut uses: operandi_core::fxhash::FxHashMap<(u32, u32), ([usize; 2], u32)> =
        operandi_core::fxhash::FxHashMap::with_capacity_and_hasher(
            corners / 2 + 1,
            Default::default(),
        );
    for (f, face) in mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        for i in 0..n {
            let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
            let slot = uses.entry((a.min(b), a.max(b))).or_insert(([0; 2], 0));
            if slot.1 < 2 {
                slot.0[slot.1 as usize] = f;
            }
            slot.1 += 1;
        }
    }
    mesh.faces
        .iter()
        .enumerate()
        .map(|(f, face)| {
            let n = face.vertices.len();
            (0..n)
                .map(|i| {
                    let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
                    let (faces, n) = uses[&(a.min(b), a.max(b))];
                    n != 2 || faces.iter().any(|&g| sources[g] != sources[f])
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use operandi_core::subdivision::display_surface_traced;

    #[test]
    fn traced_edges_are_the_cage_edges_on_the_surface() {
        let cube = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        let smooth = operandi_core::subdivision::toggle(
            &cube,
            &[],
            operandi_core::subdivision::Toggle::CatmullClark,
        )
        .unwrap();
        let (mesh, sources, tracked) =
            display_surface_traced(&smooth, &SubdivisionSettings::default()).unwrap();
        // Level 2: each cage edge becomes 4 refined edges; each is traced on
        // both sides, 12 edges x 4 x 2 = 96 traced polygon sides.
        let count: usize = tracked.iter().flatten().filter(|&&t| t).count();
        assert_eq!(count, 96);
        assert_eq!(tracked, traced_edges(&mesh, &sources));
        // The same for Modo Subdivision at level 3 on a mixed mesh.
        let mixed = operandi_core::subdivision::toggle(
            &cube,
            &[0, 1, 2, 3, 4],
            operandi_core::subdivision::Toggle::Subdivision,
        )
        .unwrap();
        let settings = SubdivisionSettings {
            subdivision_level: 3,
            ..Default::default()
        };
        let (mesh, sources, tracked) = display_surface_traced(&mixed, &settings).unwrap();
        assert_eq!(tracked, traced_edges(&mesh, &sources));
    }

    /// The stored surface carries the cage's UVs: moving points keeps
    /// them, a UV edit rebuilds them.
    #[test]
    fn a_uv_edit_rebuilds_the_stored_surface_uvs() {
        let mut e = operandi_core::Editor::default();
        e.dispatch(operandi_core::Command::NewMesh).unwrap();
        // Enough polygons (a cube refined twice) that a one-point drag
        // stays local.
        let mut cube = operandi_core::Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
        for _ in 0..2 {
            let all = operandi_core::subdivision::toggle(
                &cube,
                &[],
                operandi_core::subdivision::Toggle::CatmullClark,
            )
            .unwrap();
            cube = operandi_core::subdivision::freeze(
                &all,
                &SubdivisionSettings {
                    catmull_clark_level: 1,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let mut smooth = operandi_core::subdivision::toggle(
            &cube,
            &[],
            operandi_core::subdivision::Toggle::Subdivision,
        )
        .unwrap();
        let rows = |u: f64| -> Vec<Vec<Option<glam::DVec2>>> { smooth_rows(&cube, u) };
        smooth.uv_maps = BTreeMap::from([("Texture".to_string(), rows(0.25))]).into();
        e.scene.active_mut().unwrap().mesh = smooth.clone().into();
        let cache = SubdivisionCache::default();
        let first = cache.get(e.scene.active().unwrap()).unwrap();
        let uv_of = |s: &Surface| s.mesh.uv_maps["Texture"][0][0].unwrap().x;
        assert!((uv_of(&first) - 0.25).abs() < 1e-9);
        // A point drag: updated in place, UVs kept.
        let mut moved = smooth.clone();
        moved.vertices[0].x += 0.1;
        e.scene.active_mut().unwrap().mesh = moved.clone().into();
        let dragged = cache.get(e.scene.active().unwrap()).unwrap();
        assert_eq!(cache.updates(), 1);
        assert!((uv_of(&dragged) - 0.25).abs() < 1e-9);
        // A UV edit: rebuilt with the new UVs.
        moved.uv_maps = BTreeMap::from([("Texture".to_string(), rows(0.75))]).into();
        e.scene.active_mut().unwrap().mesh = moved.into();
        let edited = cache.get(e.scene.active().unwrap()).unwrap();
        assert!((uv_of(&edited) - 0.75).abs() < 1e-9);
    }

    /// Every corner of every polygon at (u, u).
    fn smooth_rows(mesh: &operandi_core::Mesh, u: f64) -> Vec<Vec<Option<glam::DVec2>>> {
        mesh.faces
            .iter()
            .map(|f| vec![Some(glam::DVec2::splat(u)); f.vertices.len()])
            .collect()
    }
}
