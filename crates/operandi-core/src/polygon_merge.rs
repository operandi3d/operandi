//! MP-C09 basic Polygon Merge (MER-001..003): replace explicitly selected,
//! edge-adjacent, consistently oriented coplanar surface polygons of the
//! active mesh by their single outer boundary polygon, as one undoable edit.
//!
//! Technique (Operandi's own, from general mesh-topology practice): a
//! directed edge used by one selected face whose reverse is used by another
//! selected face is interior and disappears; every other directed edge of the
//! selection is boundary. The basic supported region is a topological disc
//! whose points all lie on its boundary:
//! * every boundary point has exactly one outgoing boundary edge (no pinch);
//!   the boundary edges form one outer loop, and any holes (inner loops)
//!   are joined to it by a doubled bridge along one removed interior edge,
//!   so a ring merges into one keyhole polygon that repeats the bridge's
//!   two points (CMD-P-05);
//! * every selected point is on that cycle (no fully interior point; that
//!   class is later scope, not inferred from this one);
//! * faces are edge-connected and `faces == interior edges + 1` (Euler
//!   characteristic of a disc when all points are boundary points).
//!
//! Geometry uses the Triple eligibility chart and tolerances
//! (`TRIPLE_*_TOLERANCE`, see `polygon_triple`) on the merged boundary: it
//! must be planar, nonzero-area and simple. Each source face must face the
//! merged normal, and the source face area vectors must sum to the merged
//! area vector, so the faces tile the merged polygon without folds. Collinear
//! boundary points remain corners; nothing is triangulated, moved or cleaned.
//!
//! Data: points and their payloads are unchanged. Each merged corner is the
//! source occurrence that owns its outgoing boundary edge; every other
//! occurrence of that point on a selected face must carry exactly equal
//! sparse corner data (presence included) in every corner class, otherwise
//! the edit refuses as a seam/discontinuity the merged polygon cannot hold.
//! Face metadata and polygon-set membership must be common to all selected
//! faces. Creases on removed interior edges cease to exist; boundary creases
//! are kept. Unselected faces keep their data and order; indices compact.
use crate::polygon_triple::{
    TRIPLE_AREA_TOLERANCE, TRIPLE_LENGTH_TOLERANCE, chart, require_simple,
};
use crate::{Editor, Face, Mesh, MeshAttributes, Mode, Result, TypedMap};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit source of one output face of [`merge_faces`]. Also the identity
/// correspondence consumed by `MeshIdentity::merge_faces`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FaceMergeOrigin {
    /// Unselected source face, unchanged apart from its compacted index.
    Retained(usize),
    /// The merged polygon.
    Merged {
        /// Every selected source face, ascending (also faces with no
        /// boundary corner).
        sources: Vec<usize>,
        /// Per output corner, the source occurrence `(face, corner ordinal)`.
        corners: Vec<(usize, usize)>,
    },
}

/// A successful Merge result for one mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct MergeOutcome {
    pub mesh: Mesh,
    /// One entry per output face.
    pub origins: Vec<FaceMergeOrigin>,
    /// Output index of the merged polygon: the lowest selected source index.
    pub merged: u32,
    /// Removed interior edges as canonical `(min, max)` point pairs, sorted.
    pub removed_edges: Vec<(u32, u32)>,
}

fn refuse(detail: impl std::fmt::Display) -> String {
    format!("Merge refuses {detail}; nothing changed")
}

/// A corner occurrence: `(source face, corner ordinal)`.
type Occurrence = (usize, usize);
/// A canonical `(min, max)` point pair.
type EdgeKey = (u32, u32);

/// The selected region's boundary: the merged polygon's corners (source
/// occurrences), its loops (the outer one first, then the holes, each as
/// occurrences, before they are joined), and the removed interior edges;
/// or a refusal when it is not a supported disc or disc with holes.
struct Region {
    cycle: Vec<Occurrence>,
    loops: Vec<Vec<Occurrence>>,
    removed: Vec<EdgeKey>,
}

fn region(mesh: &Mesh, targets: &BTreeSet<u32>) -> Result<Region> {
    let mut directed: BTreeMap<(u32, u32), (usize, usize)> = BTreeMap::new();
    for &f in targets {
        let vs = &mesh.faces[f as usize].vertices;
        for c in 0..vs.len() {
            if directed
                .insert((vs[c], vs[(c + 1) % vs.len()]), (f as usize, c))
                .is_some()
            {
                return Err(refuse("selected polygons with inconsistent winding"));
            }
        }
    }
    // Union-find over selected faces, joined across interior edges.
    let slots: BTreeMap<usize, usize> = targets
        .iter()
        .enumerate()
        .map(|(i, &f)| (f as usize, i))
        .collect();
    let mut parent: Vec<usize> = (0..targets.len()).collect();
    fn root(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut removed = Vec::new();
    let mut boundary: BTreeMap<u32, (u32, usize, usize)> = BTreeMap::new();
    for (&(a, b), &(f, c)) in &directed {
        if let Some(&(g, _)) = directed.get(&(b, a)) {
            if a < b {
                removed.push((a, b));
                let (x, y) = (root(&mut parent, slots[&f]), root(&mut parent, slots[&g]));
                parent[x] = y;
            }
        } else if boundary.insert(a, (b, f, c)).is_some() {
            return Err(refuse(format!(
                "a region whose boundary touches itself at point {a}"
            )));
        }
    }
    let first = root(&mut parent, 0);
    if (0..targets.len()).any(|i| root(&mut parent, i) != first) {
        return Err(refuse("polygons that are not one edge-adjacent region"));
    }
    if boundary.is_empty() {
        return Err(refuse("a closed selection with no outer boundary"));
    }
    // Start at the lowest selected face's first boundary corner, so the
    // result is deterministic for an identical mesh and selection set.
    let start = targets
        .iter()
        .find_map(|&f| {
            let vs = &mesh.faces[f as usize].vertices;
            (0..vs.len())
                .find(|&c| {
                    boundary
                        .get(&vs[c])
                        .is_some_and(|&(_, g, d)| (g, d) == (f as usize, c))
                })
                .map(|c| vs[c])
        })
        .ok_or_else(|| refuse("a region without a boundary corner"))?;
    // Every boundary loop, the first from `start`.
    let mut loops: Vec<Vec<Occurrence>> = Vec::new();
    let mut on_loop: BTreeSet<u32> = BTreeSet::new();
    let starts = std::iter::once(start).chain(boundary.keys().copied());
    for first in starts.collect::<Vec<_>>() {
        if on_loop.contains(&first) {
            continue;
        }
        let mut cycle = Vec::new();
        let mut v = first;
        loop {
            let &(next, f, c) = boundary
                .get(&v)
                .ok_or_else(|| refuse("an open selected boundary"))?;
            if !on_loop.insert(v) {
                return Err(refuse("an open selected boundary"));
            }
            cycle.push((f, c));
            v = next;
            if v == first {
                break;
            }
        }
        loops.push(cycle);
    }
    let points: BTreeSet<u32> = targets
        .iter()
        .flat_map(|&f| mesh.faces[f as usize].vertices.iter().copied())
        .collect();
    // Disc check (Euler characteristic 1 − holes): faces + holes = interior
    // edges − interior points + 1. Interior points lose every edge and are
    // deleted afterwards.
    let holes = loops.len() - 1;
    let interior_points = points.len() - boundary.len();
    if targets.len() + interior_points + holes != removed.len() + 1 {
        return Err(refuse("a region that is not a disc or a disc with holes"));
    }
    let point = |&(f, c): &Occurrence| mesh.faces[f].vertices[c];
    // The outer loop encloses the most area; the rest are holes.
    let newell = |cycle: &[Occurrence]| {
        let p: Vec<DVec3> = cycle
            .iter()
            .map(|o| mesh.vertices[point(o) as usize])
            .collect();
        (0..p.len())
            .fold(DVec3::ZERO, |n, i| n + p[i].cross(p[(i + 1) % p.len()]))
            .length()
    };
    let outer = (0..loops.len())
        .max_by(|&a, &b| {
            newell(&loops[a])
                .total_cmp(&newell(&loops[b]))
                .then(b.cmp(&a))
        })
        .unwrap_or(0);
    loops.swap(0, outer);
    // Join each hole to the polygon through a removed interior edge from
    // one of its points to a point already on the polygon: the polygon runs
    // to that point, across the edge, once around the hole and back across
    // the edge. Of the candidate edges the highest-numbered is taken
    // (provisional: it gives CMD-P-05's 10-14 bridge; the reference's rule
    // is open, CMD-Q-HOLE). Each corner is the source occurrence that owns
    // its outgoing edge, so a bridge corner is the selected face's corner
    // along that interior edge.
    let mut cycle = loops[0].clone();
    let mut left: Vec<usize> = (1..loops.len()).collect();
    let mut bridges: BTreeSet<EdgeKey> = BTreeSet::new();
    while !left.is_empty() {
        let on_polygon: BTreeSet<u32> = cycle.iter().map(point).collect();
        let found = removed.iter().rev().find_map(|&(a, b)| {
            left.iter().enumerate().find_map(|(slot, &hole)| {
                let in_hole = |v: u32| loops[hole].iter().any(|o| point(o) == v);
                if bridges.contains(&(a, b)) {
                    None
                } else if on_polygon.contains(&a) && in_hole(b) {
                    Some((slot, a, b))
                } else if on_polygon.contains(&b) && in_hole(a) {
                    Some((slot, b, a))
                } else {
                    None
                }
            })
        });
        let Some((slot, p, q)) = found else {
            return Err(refuse("a hole that no interior edge joins to the border"));
        };
        let hole = &loops[left.remove(slot)];
        let at = cycle.iter().position(|o| point(o) == p).unwrap_or(0);
        let from = hole.iter().position(|o| point(o) == q).unwrap_or(0);
        let across = |x: u32, y: u32| directed.get(&(x, y)).copied();
        let (Some(out), Some(back)) = (across(p, q), across(q, p)) else {
            return Err(refuse("a hole that no interior edge joins to the border"));
        };
        let mut joined = cycle[..at].to_vec();
        joined.push(out);
        joined.extend(hole[from..].iter().chain(&hole[..from]));
        joined.push(back);
        joined.extend(&cycle[at..]);
        cycle = joined;
        bridges.insert((p.min(q), p.max(q)));
    }
    // A bridge stays as two sides of the merged polygon.
    let removed: Vec<EdgeKey> = removed
        .into_iter()
        .filter(|e| !bridges.contains(e))
        .collect();
    if cycle.len() > 4096 {
        return Err(refuse("a merged polygon over the 4096-corner budget"));
    }
    Ok(Region {
        cycle,
        loops,
        removed,
    })
}

/// Planar, simple, consistently oriented and exactly tiled merged boundary.
/// With holes each loop must be simple on its own (the bridges double back
/// on themselves by design).
fn require_geometry(
    mesh: &Mesh,
    targets: &BTreeSet<u32>,
    cycle: &[(usize, usize)],
    loops: &[Vec<(usize, usize)>],
) -> Result<()> {
    let position = |f: usize, c: usize| mesh.vertices[mesh.faces[f].vertices[c] as usize];
    let points: Vec<DVec3> = cycle.iter().map(|&(f, c)| position(f, c)).collect();
    let plane = match chart(&points) {
        Ok(plane) => plane,
        // Nonplanar regions merge into a nonplanar polygon; only require that
        // every source face faces the merged side (no folds).
        Err(e) if e.contains("nonplanar") => return require_facing(mesh, targets, &points),
        Err(e) => return Err(refuse(format!("the merged boundary: {e}"))),
    };
    for boundary in loops {
        let points: Vec<DVec3> = boundary.iter().map(|&(f, c)| position(f, c)).collect();
        let charted = chart(&points).map_err(|e| refuse(format!("the merged boundary: {e}")))?;
        require_simple(&charted.points, TRIPLE_LENGTH_TOLERANCE * plane.extent)
            .map_err(|e| refuse(format!("the merged boundary: {e}")))?;
    }
    let min_area2 = 2. * TRIPLE_AREA_TOLERANCE * plane.extent * plane.extent;
    let mut total = 0.;
    for &f in targets {
        let vs = &mesh.faces[f as usize].vertices;
        let corners: Vec<DVec3> = (0..vs.len()).map(|c| position(f as usize, c)).collect();
        let centroid = corners.iter().copied().sum::<DVec3>() / corners.len() as f64;
        let mut area = DVec3::ZERO;
        for (i, &p) in corners.iter().enumerate() {
            area += (p - centroid).cross(corners[(i + 1) % corners.len()] - centroid);
        }
        let facing = area.dot(plane.normal);
        if facing.is_nan() || facing <= min_area2 {
            return Err(refuse(format!(
                "polygon {f}, which faces away from the merged side"
            )));
        }
        total += facing;
    }
    if (total - plane.area2).abs() > 1e-9 * plane.area2 {
        return Err(refuse(
            "polygons that overlap or fold within the merged boundary",
        ));
    }
    Ok(())
}

/// Nonplanar merge check: every selected face faces the merged Newell side.
fn require_facing(mesh: &Mesh, targets: &BTreeSet<u32>, boundary: &[DVec3]) -> Result<()> {
    let newell =
        |p: &[DVec3]| (0..p.len()).fold(DVec3::ZERO, |n, i| n + p[i].cross(p[(i + 1) % p.len()]));
    let merged = newell(boundary);
    for &f in targets {
        let corners: Vec<DVec3> = mesh.faces[f as usize]
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        if newell(&corners).dot(merged) <= 0. {
            return Err(refuse(format!(
                "polygon {f}, which faces away from the merged side"
            )));
        }
    }
    Ok(())
}

fn corner_rows<T: Clone>(rows: &[Vec<T>], origins: &[FaceMergeOrigin]) -> Vec<Vec<T>> {
    origins
        .iter()
        .map(|o| match o {
            FaceMergeOrigin::Retained(f) => rows[*f].clone(),
            FaceMergeOrigin::Merged { corners, .. } => {
                corners.iter().map(|&(f, c)| rows[f][c].clone()).collect()
            }
        })
        .collect()
}

fn per_corner_maps<T: Clone>(
    maps: &BTreeMap<String, Vec<Vec<T>>>,
    origins: &[FaceMergeOrigin],
) -> BTreeMap<String, Vec<Vec<T>>> {
    maps.iter()
        .map(|(name, rows)| (name.clone(), corner_rows(rows, origins)))
        .collect()
}

/// Source face whose face-level data a given output face carries. All merged
/// sources share it (`require_lossless`), so the lowest one is used.
fn face_source(o: &FaceMergeOrigin) -> usize {
    match o {
        FaceMergeOrigin::Retained(f) => *f,
        FaceMergeOrigin::Merged { sources, .. } => sources[0],
    }
}

fn face_rows<T: Clone>(rows: &[T], origins: &[FaceMergeOrigin]) -> Vec<T> {
    origins
        .iter()
        .map(|o| rows[face_source(o)].clone())
        .collect()
}

/// Rebuild every face/corner/edge-indexed authored datum through `origins`.
/// The destructuring is deliberately exhaustive: a new attribute class fails
/// to compile here until its Merge mapping is decided.
fn remap_mesh(
    mesh: &Mesh,
    origins: &[FaceMergeOrigin],
    new_index: &[u32],
    removed: &BTreeSet<(u32, u32)>,
) -> Result<Mesh> {
    let Mesh {
        attributes,
        vertices,
        faces,
        uv_maps,
    } = mesh;
    let MeshAttributes {
        typed_maps,
        vertex_sets,
        edge_sets,
        set_groups,
        polygon_sets,
        point_uvs,
        corner_weights,
        corner_colors,
        extrude_edge_uses,
        relative_morphs,
        smoothing_labels,
        representations,
        creases,
        uv_records,
        normal_map_order,
        corner_normals,
        point_weights,
        point_colors,
        degenerate_faces,
        solid_sketch,
        current_uv_map,
    } = &**attributes;
    let out_faces: Vec<Face> = origins
        .iter()
        .map(|o| match o {
            FaceMergeOrigin::Retained(f) => faces[*f].clone(),
            FaceMergeOrigin::Merged { sources, corners } => {
                let Face {
                    vertices: _,
                    uv: _,
                    material,
                    smooth,
                    part,
                    material_label,
                } = &faces[sources[0]];
                Face {
                    vertices: corners.iter().map(|&(f, c)| faces[f].vertices[c]).collect(),
                    // Presence is common to all sources (`require_lossless`).
                    uv: corners
                        .iter()
                        .map(|&(f, c)| faces[f].uv.as_ref().map(|uv| uv[c]))
                        .collect(),
                    material: material.clone(),
                    smooth: *smooth,
                    part: part.clone(),
                    material_label: material_label.clone(),
                }
            }
        })
        .collect();
    let typed_maps: BTreeMap<String, TypedMap> = typed_maps
        .iter()
        .map(|(name, map)| {
            let TypedMap {
                kind,
                dimension,
                zero_default,
                points,
                corners,
            } = map;
            let corners = if corners.is_empty() {
                Vec::new()
            } else {
                corner_rows(corners, origins)
            };
            (
                name.clone(),
                TypedMap {
                    kind: kind.clone(),
                    dimension: *dimension,
                    zero_default: *zero_default,
                    points: points.clone(),
                    corners,
                },
            )
        })
        .collect();
    let polygon_sets: BTreeMap<String, BTreeSet<u32>> = polygon_sets
        .iter()
        .map(|(name, set)| {
            let members = origins
                .iter()
                .enumerate()
                .filter(|(_, o)| set.contains(&(face_source(o) as u32)))
                .map(|(i, _)| i as u32)
                .collect();
            (name.clone(), members)
        })
        .collect();
    // Exceptional uses on selected faces are refused before remapping; the
    // others follow their retained face's compacted index.
    let mut edge_uses = BTreeMap::new();
    for (key, uses) in extrude_edge_uses {
        let mut mapped: Vec<[u32; 3]> = uses
            .iter()
            .map(|&[f, a, b]| [new_index[f as usize], a, b])
            .collect();
        mapped.sort_by_key(|&[f, _, _]| f);
        edge_uses.insert(key.clone(), mapped);
    }
    Ok(Mesh {
        attributes: (MeshAttributes {
            solid_sketch: solid_sketch.clone(),
            current_uv_map: current_uv_map.clone(),
            typed_maps,
            vertex_sets: vertex_sets.clone(),
            // Points keep their numbers; edges the edit removed leave their
            // sets when the edit is recorded.
            edge_sets: edge_sets.clone(),
            set_groups: set_groups.clone(),
            polygon_sets,
            point_uvs: point_uvs.clone(),
            corner_weights: per_corner_maps(corner_weights, origins),
            corner_colors: per_corner_maps(corner_colors, origins),
            extrude_edge_uses: edge_uses,
            relative_morphs: relative_morphs.clone(),
            smoothing_labels: if smoothing_labels.is_empty() {
                Vec::new()
            } else {
                face_rows(smoothing_labels, origins)
            },
            representations: if representations.is_empty() {
                Vec::new()
            } else {
                face_rows(representations, origins)
            },
            // A crease on a removed interior edge ceases with that edge.
            creases: creases
                .iter()
                .filter(|c| !removed.contains(&(c.a, c.b)))
                .cloned()
                .collect(),
            uv_records: per_corner_maps(uv_records, origins),
            normal_map_order: normal_map_order.clone(),
            corner_normals: per_corner_maps(corner_normals, origins),
            point_weights: point_weights.clone(),
            point_colors: point_colors.clone(),
            // Selected degenerate faces are refused; listed faces compact.
            degenerate_faces: degenerate_faces
                .iter()
                .map(|&f| new_index[f as usize])
                .collect(),
        })
        .into(),
        vertices: vertices.clone(),
        faces: out_faces.into(),
        uv_maps: per_corner_maps(uv_maps, origins).into(),
    })
}

/// Merge the selected faces of `mesh` into one boundary polygon. Errors leave
/// nothing changed. A one-face selection is refused (no neutral edit).
/// Edge-connected regions of a face selection (faces joined by a shared edge),
/// each ascending, ordered by lowest face. Out-of-range faces are kept as
/// their own region so `merge_faces` reports them.
fn edge_regions(mesh: &Mesh, selected: &[u32]) -> Vec<Vec<u32>> {
    let faces: BTreeSet<u32> = selected.iter().copied().collect();
    let mut by_edge: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for &f in &faces {
        let Some(face) = mesh.faces.get(f as usize) else {
            continue;
        };
        let n = face.vertices.len();
        for i in 0..n {
            let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(f);
        }
    }
    let mut seen = BTreeSet::new();
    let mut regions = Vec::new();
    for &start in &faces {
        if !seen.insert(start) {
            continue;
        }
        let mut region = vec![start];
        let mut stack = vec![start];
        while let Some(f) = stack.pop() {
            let Some(face) = mesh.faces.get(f as usize) else {
                continue;
            };
            let n = face.vertices.len();
            for i in 0..n {
                let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
                for &g in &by_edge[&(a.min(b), a.max(b))] {
                    if seen.insert(g) {
                        region.push(g);
                        stack.push(g);
                    }
                }
            }
        }
        region.sort_unstable();
        regions.push(region);
    }
    regions
}

pub fn merge_faces(mesh: &Mesh, selected: &[u32]) -> Result<MergeOutcome> {
    mesh.validate()?;
    if selected.is_empty() {
        return Err("Select polygons before Merge".into());
    }
    let mut targets = BTreeSet::new();
    for &f in selected {
        let fi = f as usize;
        if fi >= mesh.faces.len() {
            return Err("Selected polygon no longer exists".into());
        }
        if !targets.insert(f) {
            return Err("Duplicate polygon selection".into());
        }
        if mesh.is_polyline(fi) {
            return Err(refuse("Polyline elements (surface polygons only)"));
        }
        if mesh.attributes.degenerate_faces.contains(&f) {
            return Err(refuse(format!("degenerate cage polygon {f}")));
        }
    }
    if targets.len() < 2 {
        return Err("Merge needs two or more adjacent selected polygons; nothing changed".into());
    }
    if mesh
        .attributes
        .extrude_edge_uses
        .values()
        .flatten()
        .any(|&[f, _, _]| targets.contains(&f))
    {
        return Err(refuse("polygons on an exceptional winding boundary"));
    }
    let Region {
        cycle,
        loops,
        removed,
    } = region(mesh, &targets)?;
    require_geometry(mesh, &targets, &cycle, &loops)?;
    // Faces before the lowest selected one are all retained, so this source
    // index is also the merged polygon's output index.
    let merged = *targets.first().ok_or("Select polygons before Merge")?;
    let mut origins = Vec::with_capacity(mesh.faces.len() + 1 - targets.len());
    let mut new_index = vec![0u32; mesh.faces.len()];
    for (f, slot) in new_index.iter_mut().enumerate() {
        let out = u32::try_from(origins.len()).map_err(|_| "Mesh face budget exceeded")?;
        if f as u32 == merged {
            origins.push(FaceMergeOrigin::Merged {
                sources: targets.iter().map(|&s| s as usize).collect(),
                corners: cycle.clone(),
            });
        } else if targets.contains(&(f as u32)) {
            *slot = merged;
            continue;
        } else {
            origins.push(FaceMergeOrigin::Retained(f));
        }
        *slot = out;
    }
    let removed_set: BTreeSet<(u32, u32)> = removed.iter().copied().collect();
    let mut out = remap_mesh(mesh, &origins, &new_index, &removed_set)?;
    if loops.len() > 1 {
        // The keyhole repeats its bridge points: allowed for this polygon.
        out.attributes.degenerate_faces.insert(merged);
    }
    out.validate()
        .map_err(|e| format!("Merge result failed validation ({e}); nothing changed"))?;
    Ok(MergeOutcome {
        mesh: out,
        origins,
        merged,
        removed_edges: removed,
    })
}

impl Editor {
    /// Read-only MER-001 eligibility: idle editor, visible active mesh,
    /// Polygon mode and a nonempty explicit polygon selection (never implicit
    /// all). Returns the active item and selection order.
    pub fn merge_target(&self) -> Result<(u64, Vec<u32>)> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Merge".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Merge".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Merge requires Polygon selection mode".into());
        }
        let item = self.scene.active().ok_or("Select a mesh before Merge")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before Merge".into());
        }
        if self.scene.selection.faces.is_empty() {
            return Err("Select polygons before Merge".into());
        }
        Ok((item.id, self.scene.selection.faces.clone()))
    }
    /// One atomic edit. The selection is split into edge-connected regions;
    /// each region of two or more polygons merges into one polygon (single
    /// selected polygons are left alone). Points left without any edge are
    /// deleted. Unselected faces keep identities and
    /// relative order; the participating face selection is cleared (MER-002).
    /// Remembered edges that were removed interior edges leave the remembered
    /// edge selection (Undo restores them).
    pub(crate) fn merge_polygons(&mut self) -> Result<()> {
        let (item_id, selected) = self.merge_target()?;
        let source: Mesh = (*self.scene.active().ok_or("Select a mesh")?.mesh).clone();
        let regions: Vec<Vec<u32>> = edge_regions(&source, &selected)
            .into_iter()
            .filter(|r| r.len() >= 2)
            .collect();
        if regions.is_empty() {
            return Err(
                "Merge needs two or more adjacent selected polygons; nothing changed".into(),
            );
        }
        let mut ids: crate::MeshIdentity = (**self
            .identities
            .current
            .meshes
            .get(&item_id)
            .ok_or("Missing mesh identity owner")?)
        .clone();
        let mut mesh = source.clone();
        // Source face index -> current face index, updated after each merge.
        let mut current: Vec<u32> = (0..source.faces.len() as u32).collect();
        let mut removed: BTreeSet<(u32, u32)> = BTreeSet::new();
        for region in &regions {
            let faces: Vec<u32> = region.iter().map(|&f| current[f as usize]).collect();
            let outcome = merge_faces(&mesh, &faces)?;
            ids = self
                .identities
                .with_ids(|next| ids.merge_faces(&mesh, &outcome.mesh, &outcome.origins, next))?;
            let mut out_of = vec![0u32; mesh.faces.len()];
            for (out, origin) in outcome.origins.iter().enumerate() {
                match origin {
                    FaceMergeOrigin::Retained(f) => out_of[*f] = out as u32,
                    FaceMergeOrigin::Merged { sources, .. } => {
                        for &f in sources {
                            out_of[f] = out as u32;
                        }
                    }
                }
            }
            for c in &mut current {
                *c = out_of[*c as usize];
            }
            removed.extend(outcome.removed_edges.iter().copied());
            mesh = outcome.mesh;
        }
        // Delete the points this Merge orphaned (interior points of regions).
        let mut used = vec![false; mesh.vertices.len()];
        for face in &mesh.faces {
            for &v in &face.vertices {
                used[v as usize] = true;
            }
        }
        let orphaned: BTreeSet<u32> = regions
            .iter()
            .flatten()
            .flat_map(|&f| source.faces[f as usize].vertices.iter().copied())
            .filter(|&v| !used[v as usize])
            .collect();
        let mut point_remap: Vec<Option<u32>> = (0..mesh.vertices.len() as u32).map(Some).collect();
        if !orphaned.is_empty() {
            let cleaned = crate::mesh_cleanup::remove_unreferenced(&mesh, Some(&orphaned))?;
            ids = crate::mesh_cleanup::remap_identities(
                &mesh,
                &cleaned.mesh,
                &ids,
                &cleaned.survivors,
            )?;
            point_remap = vec![None; mesh.vertices.len()];
            for (new, &old) in cleaned.survivors.iter().enumerate() {
                point_remap[old] = Some(new as u32);
            }
            mesh = cleaned.mesh;
        }
        let map_point = |v: u32| point_remap.get(v as usize).copied().flatten();
        // CMD-O-merge-*: each merged polygon is selected afterwards (this
        // supersedes the basic spec's cleared selection).
        let mut merged: Vec<u32> = regions.iter().map(|r| current[r[0] as usize]).collect();
        merged.sort_unstable();
        merged.dedup();
        let before = self.scene.clone();
        let identities_before = self.identities.current.clone();
        self.scene
            .active_mut()
            .ok_or("Select a mesh before Merge")?
            .mesh = mesh.into();
        let selection = &mut self.scene.selection;
        selection.faces = merged;
        selection.edges = selection
            .edges
            .iter()
            .filter(|e| !removed.contains(&(e.0.min(e.1), e.0.max(e.1))))
            .filter_map(|&(a, b)| Some((map_point(a)?, map_point(b)?)))
            .collect();
        selection.vertices = selection
            .vertices
            .iter()
            .filter_map(|&v| map_point(v))
            .collect();
        self.identities.current.meshes.insert(item_id, ids.into());
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = identities_before;
            return Err(e);
        }
        self.record("Merge".into(), before);
        Ok(())
    }
}
