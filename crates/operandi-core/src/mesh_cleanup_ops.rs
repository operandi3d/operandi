//! Mesh Cleanup's repairs (spec `mesh-cleanup-2026-11-05`), as one geometry
//! function. The twelve options, in the dialog's order, are indexed by
//! [`CleanupOption`].
//!
//! Order of the repairs: one- and two-point polygons, duplicate corners,
//! colinear points, face-normal start corners, unify, fix gaps, then point
//! removal and merging (a floating point on top of a used one is removed as
//! floating, not merged, MC22). Points that weren't merged keep their order;
//! each merged point is appended after them (rule 7).
//!
//! Provisional readings: the merge is exact (Q-MC-02); a face-normal fix
//! starts the corner list two corners later (Q-MC-03, one case); Merge Disco
//! UVs and Remove Disco Weight Values change nothing (not observed,
//! Q-MC-04).
use crate::polygon_triple::{FaceSplitOrigin, remap_mesh};
use crate::{Mesh, Result};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

/// Option indices into the twelve flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupOption {
    FloatingVertices = 0,
    OnePointPolygons = 1,
    TwoPointPolygons = 2,
    DuplicatePoints = 3,
    Colinear = 4,
    FaceNormals = 5,
    MergeVertices = 6,
    MergeDiscoUvs = 7,
    UnifyPolygons = 8,
    ForceUnify = 9,
    RemoveDiscoWeights = 10,
    FixGaps = 11,
}

/// A cleaned mesh and where each source polygon went.
#[derive(Clone, Debug, PartialEq)]
pub struct CleanupResult {
    pub mesh: Mesh,
    /// Output index of each source polygon (`None`: removed).
    pub faces: Vec<Option<u32>>,
    /// Output index of each source point (`None`: removed).
    pub points: Vec<Option<u32>>,
}

fn colinear(a: DVec3, b: DVec3, c: DVec3) -> bool {
    let (ab, ac) = (b - a, c - a);
    let scale = ab.length_squared().max(ac.length_squared()).max(1e-300);
    ab.cross(ac).length_squared() <= 1e-18 * scale * scale
}

/// Clean `source`. `scope`: only these polygons and the points they use
/// (Polygon mode with a selection); `None` cleans the whole mesh.
pub fn mesh_cleanup_geometry(
    source: &Mesh,
    scope: Option<&BTreeSet<usize>>,
    flags: [bool; 12],
) -> Result<CleanupResult> {
    source.validate()?;
    let on = |o: CleanupOption| flags[o as usize];
    let in_scope = |f: usize| scope.is_none_or(|s| s.contains(&f));
    let p = |v: u32| source.vertices[v as usize];
    // Use counts per point over the whole mesh.
    let mut uses = vec![0usize; source.vertices.len()];
    for face in &source.faces {
        for &v in &face.vertices {
            uses[v as usize] += 1;
        }
    }
    // Working faces: source face and its kept corner ordinals.
    let mut work: Vec<(usize, Vec<usize>)> = Vec::with_capacity(source.faces.len());
    let mut doomed: BTreeSet<u32> = BTreeSet::new();
    for (f, face) in source.faces.iter().enumerate() {
        let n = face.vertices.len();
        if in_scope(f)
            && ((n == 1 && on(CleanupOption::OnePointPolygons))
                || (n == 2 && on(CleanupOption::TwoPointPolygons)))
        {
            doomed.extend(&face.vertices);
            continue;
        }
        let mut corners: Vec<usize> = (0..n).collect();
        if in_scope(f) && n >= 2 {
            if on(CleanupOption::DuplicatePoints) {
                let v = |c: usize| face.vertices[c];
                let mut kept: Vec<usize> = Vec::with_capacity(n);
                for c in 0..n {
                    if kept.last().is_none_or(|&k| v(k) != v(c)) {
                        kept.push(c);
                    }
                }
                while kept.len() > 1 && v(kept[0]) == v(*kept.last().unwrap()) {
                    kept.pop();
                }
                corners = kept;
            }
            if on(CleanupOption::Colinear) && corners.len() > 3 && !source.is_polyline(f) {
                let mut i = 0;
                while i < corners.len() && corners.len() > 3 {
                    let m = corners.len();
                    let (a, b, c) = (
                        face.vertices[corners[(i + m - 1) % m]],
                        face.vertices[corners[i]],
                        face.vertices[corners[(i + 1) % m]],
                    );
                    let between = (p(b) - p(a)).dot(p(c) - p(b)) > 0.;
                    if uses[b as usize] == 1 && between && colinear(p(a), p(b), p(c)) {
                        doomed.insert(b);
                        corners.remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
            if on(CleanupOption::FaceNormals) && corners.len() >= 3 && !source.is_polyline(f) {
                let m = corners.len();
                let v = |k: usize| p(face.vertices[corners[k % m]]);
                if colinear(v(m - 1), v(0), v(1)) {
                    corners.rotate_left(2 % m);
                }
            }
        }
        work.push((f, corners));
    }
    // Unify: a polygon on the same points as an earlier one (rotated or
    // reversed) goes; a different material keeps it unless Force Unify.
    if on(CleanupOption::UnifyPolygons) {
        let mut seen: Vec<(Vec<u32>, String)> = Vec::new();
        work.retain(|(f, corners)| {
            if !in_scope(*f) || source.is_polyline(*f) {
                return true;
            }
            let face = &source.faces[*f];
            let cycle: Vec<u32> = corners.iter().map(|&c| face.vertices[c]).collect();
            let same = |other: &Vec<u32>| {
                let n = cycle.len();
                if other.len() != n {
                    return false;
                }
                let reversed: Vec<u32> = cycle.iter().rev().copied().collect();
                (0..n).any(|r| {
                    (0..n).all(|i| other[i] == cycle[(i + r) % n])
                        || (0..n).all(|i| other[i] == reversed[(i + r) % n])
                })
            };
            let duplicate = seen.iter().any(|(other, material)| {
                same(other) && (on(CleanupOption::ForceUnify) || *material == face.material)
            });
            if !duplicate {
                seen.push((cycle, face.material.clone()));
            }
            !duplicate
        });
    }
    let origins: Vec<FaceSplitOrigin> = work
        .iter()
        .map(|(f, corners)| FaceSplitOrigin {
            face: *f,
            corners: Some(corners.clone()),
        })
        .collect();
    let mut faces_of = vec![None; source.faces.len()];
    for (i, (f, _)) in work.iter().enumerate() {
        faces_of[*f] = Some(i as u32);
    }
    let mut mesh = remap_mesh(source, &origins, &BTreeMap::new())?;
    // Fix Gaps: a point on a polygon's edge, used by its neighbours but not by
    // that polygon, is inserted into its corner list (rule 10).
    if on(CleanupOption::FixGaps) {
        let used: Vec<u32> = {
            let mut u = vec![false; mesh.vertices.len()];
            for face in &mesh.faces {
                for &v in &face.vertices {
                    u[v as usize] = true;
                }
            }
            (0..mesh.vertices.len() as u32)
                .filter(|&v| u[v as usize])
                .collect()
        };
        let in_scope_out: Vec<bool> = work.iter().map(|(f, _)| in_scope(*f)).collect();
        for fi in 0..mesh.faces.len() {
            if !in_scope_out[fi] || mesh.is_polyline(fi) {
                continue;
            }
            let ring = mesh.faces[fi].vertices.clone();
            let n = ring.len();
            let mut grown: Vec<(u32, bool)> = Vec::with_capacity(n);
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                grown.push((a, false));
                let (pa, pb) = (mesh.vertices[a as usize], mesh.vertices[b as usize]);
                let d = pb - pa;
                let length2 = d.length_squared();
                let mut on_edge: Vec<(f64, u32)> = used
                    .iter()
                    .copied()
                    .filter(|v| !ring.contains(v))
                    .filter_map(|v| {
                        let q = mesh.vertices[v as usize];
                        let t = (q - pa).dot(d) / length2;
                        let off = (pa + d * t - q).length_squared();
                        (t > 1e-9 && t < 1. - 1e-9 && off <= 1e-18 * length2).then_some((t, v))
                    })
                    .collect();
                on_edge.sort_by(|x, y| x.0.total_cmp(&y.0));
                grown.extend(on_edge.into_iter().map(|(_, v)| (v, true)));
            }
            if grown.len() == n {
                continue;
            }
            if crate::corner_uv::has_uncarried_corner_data(&mesh) {
                return Err(
                    "Fix Gaps can't carry corner weights, colors or normals yet; nothing changed"
                        .into(),
                );
            }
            mesh.faces[fi].vertices = grown.iter().map(|&(v, _)| v).collect();
            for map in mesh.uv_maps.values_mut() {
                let row = &map[fi];
                let mut k = 0;
                map[fi] = grown
                    .iter()
                    .map(|&(_, new)| {
                        if new {
                            None
                        } else {
                            k += 1;
                            row[k - 1]
                        }
                    })
                    .collect();
            }
        }
        for (name, records) in mesh.attributes.uv_records.iter_mut() {
            *records = crate::corner_uv::records_from_uvs(&mesh.faces, &mesh.uv_maps[name]);
        }
    }
    // Removed or edited polygons change which edges are exceptional: rebuild
    // the exact incidence ledger from the result, as Bridge does.
    mesh.attributes.extrude_edge_uses.clear();
    let mut incidences: BTreeMap<(u32, u32), Vec<[u32; 3]>> = BTreeMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        for i in 0..n {
            let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
            incidences
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    for ((a, b), u) in incidences {
        if u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]) {
            mesh.attributes
                .extrude_edge_uses
                .insert(format!("{a}:{b}"), u);
        }
    }
    // Points: drop doomed and (whole-mesh) floating ones, then merge.
    let mut used = vec![false; mesh.vertices.len()];
    for face in &mesh.faces {
        for &v in &face.vertices {
            used[v as usize] = true;
        }
    }
    let scoped_points: Option<BTreeSet<u32>> = scope.map(|s| {
        s.iter()
            .flat_map(|&f| source.faces[f].vertices.iter().copied())
            .collect()
    });
    let keep: Vec<bool> = (0..mesh.vertices.len() as u32)
        .map(|v| {
            used[v as usize]
                || !(doomed.contains(&v)
                    || (on(CleanupOption::FloatingVertices) && scope.is_none()))
        })
        .collect();
    let mut representative: Vec<u32> = (0..mesh.vertices.len() as u32).collect();
    let mut merged: BTreeSet<u32> = BTreeSet::new();
    if on(CleanupOption::MergeVertices) {
        let mut groups: BTreeMap<[u64; 3], Vec<u32>> = BTreeMap::new();
        for v in 0..mesh.vertices.len() as u32 {
            if !keep[v as usize] || scoped_points.as_ref().is_some_and(|s| !s.contains(&v)) {
                continue;
            }
            let q = mesh.vertices[v as usize]
                .to_array()
                .map(|x| (x + 0.).to_bits());
            groups.entry(q).or_default().push(v);
        }
        for members in groups.values().filter(|m| m.len() > 1) {
            for &m in members {
                representative[m as usize] = members[0];
            }
            merged.insert(members[0]);
        }
        for face in &mut mesh.faces {
            for v in &mut face.vertices {
                *v = representative[*v as usize];
            }
        }
    }
    // Degenerate marks follow their polygons through the removals, and go
    // once a polygon is fixed (an unmarked one stays invalid, as before).
    let degenerate: BTreeSet<u32> = (0..mesh.faces.len())
        .filter(|&fi| {
            let from = work[fi].0 as u32;
            if !source.attributes.degenerate_faces.contains(&from) || mesh.is_polyline(fi) {
                return false;
            }
            let v = &mesh.faces[fi].vertices;
            let mut sorted = v.clone();
            sorted.sort_unstable();
            sorted.windows(2).any(|w| w[0] == w[1]) || mesh.normal(fi) == glam::DVec3::ZERO
        })
        .map(|fi| fi as u32)
        .collect();
    mesh.attributes.degenerate_faces = degenerate;
    let mut survivors: Vec<usize> = (0..mesh.vertices.len())
        .filter(|&v| keep[v] && representative[v] == v as u32 && !merged.contains(&(v as u32)))
        .collect();
    survivors.extend(merged.iter().map(|&v| v as usize));
    let mesh = crate::mesh_cleanup::remap_points(&mesh, &survivors)
        .map_err(|e| format!("Mesh Cleanup produced an invalid mesh ({e}); nothing changed"))?;
    let mut points = vec![None; source.vertices.len()];
    for (new, &old) in survivors.iter().enumerate() {
        points[old] = Some(new as u32);
    }
    for v in 0..source.vertices.len() {
        if points[v].is_none() && keep.get(v).copied().unwrap_or(false) {
            points[v] = points[representative[v] as usize];
        }
    }
    Ok(CleanupResult {
        mesh,
        faces: faces_of,
        points,
    })
}
