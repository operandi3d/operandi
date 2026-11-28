//! Subdivision surfaces (`specifications/subdivision-2026-11-03`).
//!
//! Polygons carry their kind in `MeshAttributes::representations`: `Face`,
//! `Subdivs` (Subdivision, Tab) or `CatmullClark` (Shift+Tab).
//! The cage is what the mesh stores and what tools edit; the smooth surface
//! is evaluated from it for display and for Freeze.
//!
//! Catmull-Clark follows SDV-05 to SDV-07 (standard rules, Boundary Rule,
//! semi-sharp creases with sharpness = 10 x edge weight). Subdivision (Tab)
//! follows SDV-09 v2: one Catmull-Clark step, then a biquadratic patch per
//! quad sampled on an L x L grid. Edge weights on Subdivision polygons use
//! Operandi's own rule (EW-05): the first step creases exactly like
//! Catmull-Clark, then edges still sharp act like open edges in the patch
//! stage. A connected smooth region that mixes both kinds is treated as all
//! Catmull-Clark (SDV-14, an Operandi choice), so it stays closed.
use crate::topology::edge_key;
use crate::{Face, Mesh, PolygonRepresentation, Result};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod nurbs;
pub mod patches;

/// SDV-06 Boundary Rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryRule {
    #[default]
    Crease,
    Smooth,
    CreaseEdges,
}

/// SDV-03 per-mesh settings. `Default` is a new mesh's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubdivisionSettings {
    pub subdivision_level: u8,
    pub catmull_clark_level: u8,
    pub subdivision_render_level: u8,
    pub catmull_clark_render_level: u8,
    pub boundary_rule: BoundaryRule,
    pub uv_boundary_rule: BoundaryRule,
    pub linear_uvs: bool,
    /// G7 PC-7: cells along each side of a live patch (Patch Subdivisions).
    #[serde(
        default = "default_patch_cells",
        skip_serializing_if = "is_default_patch_cells"
    )]
    pub patch_subdivisions: u32,
}
fn default_patch_cells() -> u32 {
    crate::patch_live::DEFAULT_CELLS
}
fn is_default_patch_cells(n: &u32) -> bool {
    *n == crate::patch_live::DEFAULT_CELLS
}
impl Default for SubdivisionSettings {
    fn default() -> Self {
        Self {
            subdivision_level: 2,
            catmull_clark_level: 2,
            subdivision_render_level: 2,
            catmull_clark_render_level: 2,
            boundary_rule: BoundaryRule::Crease,
            uv_boundary_rule: BoundaryRule::Crease,
            linear_uvs: false,
            patch_subdivisions: crate::patch_live::DEFAULT_CELLS,
        }
    }
}

/// Highest display level offered. SDV-Q08 leaves the maximum open; this
/// is an Operandi budget (a quad becomes 4^L quads).
pub const MAX_LEVEL: u8 = 6;

/// Which key was pressed (SDV-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    /// Tab: Face → Subdivision; Subdivision or Catmull-Clark → Face.
    Subdivision,
    /// Shift+Tab: Face → Catmull-Clark; Subdivision or Catmull-Clark → Face.
    CatmullClark,
}

fn kind(mesh: &Mesh, face: usize) -> PolygonRepresentation {
    mesh.attributes
        .representations
        .get(face)
        .copied()
        .unwrap_or_default()
}

fn is_smooth(kind: PolygonRepresentation) -> bool {
    matches!(
        kind,
        PolygonRepresentation::Subdivs | PolygonRepresentation::CatmullClark
    )
}

/// SDV-14 (Operandi choice): each polygon's kind as subdivided. Smooth
/// polygons are grouped into regions joined by shared edges; a region that
/// mixes Subdivision and Catmull-Clark polygons is subdivided as all
/// Catmull-Clark, so it stays connected with no gaps. Face polygons and
/// lines keep their kind and don't join regions.
fn effective_kinds(mesh: &Mesh) -> Vec<PolygonRepresentation> {
    let mut kinds: Vec<PolygonRepresentation> =
        (0..mesh.faces.len()).map(|f| kind(mesh, f)).collect();
    let mask: Vec<bool> = (0..kinds.len())
        .map(|f| is_smooth(kinds[f]) && !mesh.is_polyline(f))
        .collect();
    let smooth = |f: usize| mask[f];
    let both = (0..kinds.len()).any(|f| smooth(f) && kinds[f] == PolygonRepresentation::Subdivs)
        && (0..kinds.len()).any(|f| smooth(f) && kinds[f] == PolygonRepresentation::CatmullClark);
    if !both {
        return kinds;
    }
    // Union-find over smooth polygons sharing an edge.
    let mut parent: Vec<usize> = (0..kinds.len()).collect();
    fn root(parent: &mut [usize], mut f: usize) -> usize {
        while parent[f] != f {
            parent[f] = parent[parent[f]];
            f = parent[f];
        }
        f
    }
    let mut first: crate::fxhash::FxHashMap<(u32, u32), usize> = Default::default();
    for (fi, face) in mesh.faces.iter().enumerate() {
        if !smooth(fi) {
            continue;
        }
        let v = &face.vertices;
        for i in 0..v.len() {
            let k = edge_key(v[i], v[(i + 1) % v.len()]);
            match first.get(&k) {
                Some(&other) => {
                    let (a, b) = (root(&mut parent, fi), root(&mut parent, other));
                    parent[a] = b;
                }
                None => {
                    first.insert(k, fi);
                }
            }
        }
    }
    let mut has_cc = vec![false; kinds.len()];
    for (f, &k) in kinds.iter().enumerate() {
        if smooth(f) && k == PolygonRepresentation::CatmullClark {
            let r = root(&mut parent, f);
            has_cc[r] = true;
        }
    }
    for f in 0..kinds.len() {
        if smooth(f) && kinds[f] == PolygonRepresentation::Subdivs && has_cc[root(&mut parent, f)] {
            kinds[f] = PolygonRepresentation::CatmullClark;
        }
    }
    kinds
}

/// True when any polygon of `mesh` is a smooth (subdivision) polygon.
pub fn has_smooth(mesh: &Mesh) -> bool {
    // Polygons past the representation list are the default kind, so only
    // the list is read (empty for most meshes) instead of every polygon.
    let listed = mesh.attributes.representations.len().min(mesh.faces.len());
    (0..listed).any(|f| is_smooth(kind(mesh, f)))
        || (listed < mesh.faces.len() && is_smooth(PolygonRepresentation::default()))
}

/// Triangles the views draw for `mesh` at `settings`' display levels,
/// counted from the polygon list without building the surface: a plain
/// n-gon is n - 2 triangles; a Catmull-Clark polygon at level L becomes
/// n·4^(L-1) quads and a Subdivision polygon n·L² quads (SDV-04), two
/// triangles each; level 0 keeps the polygon. Polylines draw none.
/// The viewport counts its cached GPU geometry instead (gpu_scene.rs); this
/// is for items it doesn't hold.
/// A mesh with no listed kinds (every polygon a plain Face: no SubD, no
/// polylines) is answered from the remembered polygon summary, without a
/// pass over the polygons (the graphics memory watch counts per item
/// while a big scene is navigated).
pub fn drawn_triangles(mesh: &Mesh, settings: &SubdivisionSettings) -> u64 {
    if mesh.attributes.representations.is_empty() && !is_smooth(PolygonRepresentation::default()) {
        return mesh.face_summary().plain_triangles;
    }
    drawn_count(mesh, settings, |n| n.saturating_sub(2), 2)
}

/// Sum over the drawn polygons: `plain(n)` for an n-sided polygon drawn as
/// it is, `per_quad` for each quad a smooth one refines into.
fn drawn_count(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
    plain: impl Fn(u64) -> u64,
    per_quad: u64,
) -> u64 {
    let corners = |f: usize| mesh.faces[f].vertices.len() as u64;
    let drawn = (0..mesh.faces.len()).filter(|&f| !mesh.is_polyline(f));
    if !has_smooth(mesh) {
        return drawn.map(|f| plain(corners(f))).sum();
    }
    let kinds = effective_kinds(mesh);
    drawn
        .map(|f| {
            let per_corner = match kinds[f] {
                PolygonRepresentation::CatmullClark if settings.catmull_clark_level > 0 => {
                    4u64.pow(u32::from(settings.catmull_clark_level) - 1)
                }
                PolygonRepresentation::Subdivs if settings.subdivision_level > 0 => {
                    u64::from(settings.subdivision_level).pow(2)
                }
                _ => return plain(corners(f)),
            };
            per_quad * corners(f) * per_corner
        })
        .sum()
}

/// Smooth pieces from which the viewport draws Subdivision polygons as
/// patches on the graphics card (docs/SUBD_GPU_DESIGN.md) instead of
/// storing the evaluated surface. Below it the stored surface draws as
/// before (incremental drag updates, every material and lock colour).
pub const PATCH_MIN_PIECES: u64 = 500_000;

/// Memory a stored surface piece (a quad) costs to draw: the CPU surface
/// and the GPU corners, ~1.1 KB per triangle (measured 2026-11-13).
pub const STORED_BYTES_PER_PIECE: u64 = 2_300;

/// Smooth pieces `mesh` shows at `settings`' display levels.
pub fn display_pieces(mesh: &Mesh, settings: &SubdivisionSettings) -> u64 {
    if !has_smooth(mesh) {
        return 0;
    }
    let kinds = effective_kinds(mesh);
    (0..mesh.faces.len())
        .filter(|&f| is_smooth(kinds[f]) && !mesh.is_polyline(f))
        .map(|f| {
            let n = mesh.faces[f].vertices.len() as u64;
            match kinds[f] {
                PolygonRepresentation::CatmullClark if settings.catmull_clark_level > 0 => {
                    n * 4u64.pow(u32::from(settings.catmull_clark_level) - 1)
                }
                PolygonRepresentation::Subdivs if settings.subdivision_level > 0 => {
                    n * u64::from(settings.subdivision_level).pow(2)
                }
                _ => 1,
            }
        })
        .sum()
}

/// Whether the viewport draws `mesh`'s Subdivision polygons as patches:
/// eligible (patches::eligible) and at least `PATCH_MIN_PIECES` pieces.
pub fn draws_as_patches(mesh: &Mesh, settings: &SubdivisionSettings, min_pieces: u64) -> bool {
    patches::eligible(mesh, settings.subdivision_level)
        && display_pieces(mesh, settings) >= min_pieces
}

/// Memory the viewport needs to show `mesh` subdivided at `settings`:
/// patches (`patches::BYTES_PER_PATCH`, a CPU and a GPU copy) or the stored
/// surface (`STORED_BYTES_PER_PIECE`).
pub fn display_bytes(mesh: &Mesh, settings: &SubdivisionSettings) -> u64 {
    if draws_as_patches(mesh, settings, PATCH_MIN_PIECES) {
        patches::patch_count(mesh).saturating_mul(2 * patches::BYTES_PER_PATCH)
    } else {
        display_pieces(mesh, settings).saturating_mul(STORED_BYTES_PER_PIECE)
    }
}

/// SDV-01/02: flip the kind of `faces` (every polygon when empty). Points,
/// polygons and their order are unchanged.
pub fn toggle(mesh: &Mesh, faces: &[u32], toggle: Toggle) -> Result<Mesh> {
    let targets: BTreeSet<usize> = if faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        faces.iter().map(|&f| f as usize).collect()
    };
    if targets.iter().any(|&f| f >= mesh.faces.len()) {
        return Err("A selected polygon no longer exists".into());
    }
    if targets.iter().any(|&f| mesh.is_polyline(f)) {
        return Err("Lines can't become subdivision polygons; nothing changed".into());
    }
    if targets.is_empty() {
        return Err("The mesh has no polygons to convert".into());
    }
    let mut out = mesh.clone();
    let reps = &mut out.attributes.representations;
    reps.resize(mesh.faces.len(), PolygonRepresentation::Face);
    for f in targets {
        reps[f] = match (reps[f], toggle) {
            (PolygonRepresentation::Face, Toggle::Subdivision) => PolygonRepresentation::Subdivs,
            (PolygonRepresentation::Face, Toggle::CatmullClark) => {
                PolygonRepresentation::CatmullClark
            }
            _ => PolygonRepresentation::Face,
        };
    }
    // An all-Face mesh keeps the compact empty representation list.
    if reps.iter().all(|r| *r == PolygonRepresentation::Face) {
        reps.clear();
    }
    out.validate()?;
    Ok(out)
}

/// A polygon mesh being refined: points, polygons, per-edge sharpness.
#[derive(Clone, Debug, Default)]
struct Net {
    points: Vec<DVec3>,
    faces: Vec<Vec<u32>>,
    /// (low, high) point pair → sharpness.
    sharp: BTreeMap<(u32, u32), f64>,
    /// Per polygon, per corner: whether the side from that corner to the
    /// next lies on a cage edge (drawn as the traced wireframe), and which
    /// one, by its two cage point numbers (low, high).
    sides: Vec<Vec<Side>>,
}

/// The cage edge a refined polygon side lies on, if any.
pub type Side = Option<(u32, u32)>;

/// One Catmull-Clark step (SDV-05 to SDV-07).
/// `soften`: negative sharpness (EW2-04, the Subdivision kind only) pushes
/// edge points away from the edge instead of being ignored.
fn catmull_clark_step(net: &Net, rule: BoundaryRule, soften: bool) -> Net {
    // Flat arrays and a fast hash: this runs on every frame of a drag on a
    // subdivided mesh, so it avoids sorted maps and per-point lists.
    let n_points = net.points.len();
    let face_points: Vec<DVec3> = net
        .faces
        .iter()
        .map(|f| f.iter().map(|&v| net.points[v as usize]).sum::<DVec3>() / f.len() as f64)
        .collect();
    // Edges in first-seen order; each corner's edge to the next corner.
    let corners: usize = net.faces.iter().map(Vec::len).sum();
    let mut edge_id: crate::fxhash::FxHashMap<(u32, u32), u32> =
        crate::fxhash::FxHashMap::with_capacity_and_hasher(corners, Default::default());
    let mut edges: Vec<(u32, u32)> = Vec::with_capacity(corners / 2 + 1);
    // Up to two adjacent faces and the use count per edge.
    let mut edge_faces: Vec<([usize; 2], u32)> = Vec::with_capacity(corners / 2 + 1);
    let mut corner_edge: Vec<Vec<u32>> = Vec::with_capacity(net.faces.len());
    for (fi, f) in net.faces.iter().enumerate() {
        let n = f.len();
        let mut ids = Vec::with_capacity(n);
        for i in 0..n {
            let k = edge_key(f[i], f[(i + 1) % n]);
            let id = *edge_id.entry(k).or_insert_with(|| {
                edges.push(k);
                edge_faces.push(([0; 2], 0));
                (edges.len() - 1) as u32
            });
            let slot = &mut edge_faces[id as usize];
            if slot.1 < 2 {
                slot.0[slot.1 as usize] = fi;
            }
            slot.1 += 1;
            ids.push(id);
        }
        corner_edge.push(ids);
    }
    let sharpness = |k: &(u32, u32)| {
        if net.sharp.is_empty() {
            0.0
        } else {
            net.sharp.get(k).copied().unwrap_or(0.0).max(0.0)
        }
    };
    let mut points = vec![DVec3::ZERO; n_points + edges.len() + net.faces.len()];
    // Edge points.
    for (e, k) in edges.iter().enumerate() {
        let (a, b) = (net.points[k.0 as usize], net.points[k.1 as usize]);
        let mid = (a + b) / 2.0;
        let s = sharpness(k);
        let (faces, uses) = edge_faces[e];
        // EW2-04: a negative weight (sharpness / 10, down to -1) blends the
        // edge point away from the edge's midpoint.
        let raw = net.sharp.get(k).copied().unwrap_or(0.0);
        let s = if soften && raw < 0.0 { raw / 10.0 } else { s };
        points[n_points + e] = if uses != 2 || s >= 1.0 {
            mid
        } else {
            let smooth = (a + b + face_points[faces[0]] + face_points[faces[1]]) / 4.0;
            smooth + (mid - smooth) * s
        };
    }
    // Per point: adjacent face-point sum and count, edge-midpoint sum and
    // count, and the first two boundary / sharp neighbours with counts.
    #[derive(Clone, Copy, Default)]
    struct Around {
        faces: DVec3,
        face_count: u32,
        mids: DVec3,
        edge_count: u32,
        boundary: [DVec3; 2],
        boundary_count: u32,
        sharp: [DVec3; 2],
        sharp_count: u32,
        sharp_sum: f64,
    }
    let mut around = vec![Around::default(); n_points];
    for (fi, f) in net.faces.iter().enumerate() {
        for &v in f {
            let a = &mut around[v as usize];
            a.faces += face_points[fi];
            a.face_count += 1;
        }
    }
    for (e, k) in edges.iter().enumerate() {
        let open = edge_faces[e].1 != 2;
        let s = sharpness(k);
        for (v, other) in [(k.0, k.1), (k.1, k.0)] {
            let p = net.points[v as usize];
            let q = net.points[other as usize];
            let a = &mut around[v as usize];
            a.mids += (p + q) / 2.0;
            a.edge_count += 1;
            if open {
                if a.boundary_count < 2 {
                    a.boundary[a.boundary_count as usize] = q;
                }
                a.boundary_count += 1;
            }
            if s > 0.0 {
                if a.sharp_count < 2 {
                    a.sharp[a.sharp_count as usize] = q;
                }
                a.sharp_count += 1;
                a.sharp_sum += s;
            }
        }
    }
    for v in 0..n_points {
        let p = net.points[v];
        let a = around[v];
        if a.edge_count == 0 {
            points[v] = p;
            continue;
        }
        if a.boundary_count > 0 {
            // SDV-06: a corner is a boundary point used by one polygon.
            let corner = a.face_count == 1;
            points[v] = if a.boundary_count == 2 && (!corner || rule != BoundaryRule::Crease) {
                (a.boundary[0] + 6.0 * p + a.boundary[1]) / 8.0
            } else {
                p
            };
            continue;
        }
        let n = a.edge_count as f64;
        let f = a.faces / a.face_count as f64;
        let r = a.mids / n;
        let smooth = (f + 2.0 * r + (n - 3.0) * p) / n;
        let average = a.sharp_sum / a.sharp_count.max(1) as f64;
        let rule_point = match a.sharp_count {
            0 | 1 => None,
            2 => Some((a.sharp[0] + 6.0 * p + a.sharp[1]) / 8.0),
            _ => Some(p),
        };
        points[v] = match rule_point {
            None => smooth,
            Some(q) if average >= 1.0 => q,
            Some(q) => smooth + (q - smooth) * average,
        };
    }
    // Face points and the new quads.
    let mut faces = Vec::with_capacity(corners);
    let mut sides = Vec::with_capacity(corners);
    let face_base = n_points + edges.len();
    for (fi, f) in net.faces.iter().enumerate() {
        let center = (face_base + fi) as u32;
        points[center as usize] = face_points[fi];
        let n = f.len();
        let ids = &corner_edge[fi];
        for i in 0..n {
            let prev = n_points as u32 + ids[(i + n - 1) % n];
            let next = n_points as u32 + ids[i];
            faces.push(vec![f[i], next, center, prev]);
            // A child's first side is half of the parent's side i, its last
            // half of side i-1; the two inner sides are new.
            let parent = &net.sides[fi];
            sides.push(vec![parent[i], None, None, parent[(i + n - 1) % n]]);
        }
    }
    // SDV-07: each half of a sharp edge carries sharpness s - 1.
    let mut sharp = BTreeMap::new();
    for (k, &s) in &net.sharp {
        // EW2-04: a negative weight carries on unchanged to the patch stage.
        let child = if s > 1.0 {
            s - 1.0
        } else if soften && s < 0.0 {
            s
        } else {
            continue;
        };
        if let Some(&id) = edge_id.get(k) {
            let mid = n_points as u32 + id;
            sharp.insert(edge_key(k.0, mid), child);
            sharp.insert(edge_key(mid, k.1), child);
        }
    }
    Net {
        points,
        faces,
        sharp,
        sides,
    }
}

/// A polygon net: points, polygons and (low, high) edge sharpness.
pub(crate) type PlainNet = (Vec<DVec3>, Vec<Vec<u32>>, BTreeMap<(u32, u32), f64>);

/// One Catmull-Clark step of a whole polygon net, for multiresolution
/// levels (multires.rs). Old points keep their numbers, then come the edge
/// points and the face points; each polygon becomes its corner quads, in
/// corner order. `sharp` maps (low, high) point pairs to sharpness.
pub(crate) fn catmull_clark_net(
    points: Vec<DVec3>,
    faces: Vec<Vec<u32>>,
    sharp: BTreeMap<(u32, u32), f64>,
    rule: BoundaryRule,
) -> PlainNet {
    let sides = faces.iter().map(|f| vec![None; f.len()]).collect();
    let net = Net {
        points,
        faces,
        sharp,
        sides,
    };
    let out = catmull_clark_step(&net, rule, false);
    (out.points, out.faces, out.sharp)
}

/// SDV-09 (v2) Subdivision (Tab): one Catmull-Clark step with the curve rule
/// on every boundary point, then a biquadratic Bezier patch on each quad,
/// sampled on an L x L grid. Samples on shared edges and corners are shared.
fn patch_surface(net: &Net, level: u8) -> Net {
    // EW-05 (Operandi choice): the step creases exactly like Catmull-Clark
    // (EW-03); its children carry sharpness s - 1, which the patch stage
    // below reads as "how much like an open edge" (capped at 1).
    let step = catmull_clark_step(net, BoundaryRule::Smooth, true);
    let grids = patch_grids(&step);
    let l = level as usize;
    let bern = |t: f64| [(1.0 - t) * (1.0 - t), 2.0 * t * (1.0 - t), t * t];
    let mut out = Net::default();
    // Shared samples: corners by CC point, edge interiors by (edge, k from
    // the lower-numbered end).
    let mut corner_index: crate::fxhash::FxHashMap<u32, u32> = Default::default();
    let mut edge_index: crate::fxhash::FxHashMap<((u32, u32), usize), u32> = Default::default();
    for (fi, f) in step.faces.iter().enumerate() {
        let c = &grids[fi];
        let eval = |u: f64, v: f64| {
            let (bu, bv) = (bern(u), bern(v));
            let mut p = DVec3::ZERO;
            for a in 0..3 {
                for b in 0..3 {
                    p += c[a][b] * bu[a] * bv[b];
                }
            }
            p
        };
        // Sample (i, j) on this quad, reusing shared ones.
        let mut grid = vec![vec![0u32; l + 1]; l + 1];
        for (i, row) in grid.iter_mut().enumerate() {
            for (j, slot) in row.iter_mut().enumerate() {
                let shared = match (i, j) {
                    (0, 0) => Some(Err(f[0])),
                    (_, 0) if i == l => Some(Err(f[1])),
                    (_, _) if i == l && j == l => Some(Err(f[2])),
                    (0, _) if j == l => Some(Err(f[3])),
                    (_, 0) => Some(Ok((f[0], f[1], i))),
                    (_, _) if i == l => Some(Ok((f[1], f[2], j))),
                    (_, _) if j == l => Some(Ok((f[3], f[2], i))),
                    (0, _) => Some(Ok((f[0], f[3], j))),
                    _ => None,
                };
                let next = out.points.len() as u32;
                let point = || eval(i as f64 / l as f64, j as f64 / l as f64);
                *slot = match shared {
                    Some(Err(corner)) => *corner_index.entry(corner).or_insert_with(|| {
                        out.points.push(point());
                        next
                    }),
                    Some(Ok((a, b, k))) => {
                        // Position k of l from a toward b; key from the lower end.
                        let along = if a < b { k } else { l - k };
                        *edge_index
                            .entry((edge_key(a, b), along))
                            .or_insert_with(|| {
                                out.points.push(point());
                                next
                            })
                    }
                    None => {
                        out.points.push(point());
                        next
                    }
                };
            }
        }
        for i in 0..l {
            for j in 0..l {
                out.faces.push(vec![
                    grid[i][j],
                    grid[i + 1][j],
                    grid[i + 1][j + 1],
                    grid[i][j + 1],
                ]);
                let parent = &step.sides[fi];
                out.sides.push(vec![
                    parent[0].filter(|_| j == 0),
                    parent[1].filter(|_| i + 1 == l),
                    parent[2].filter(|_| j + 1 == l),
                    parent[3].filter(|_| i == 0),
                ]);
            }
        }
    }
    out
}

/// The biquadratic Bézier controls of each quad of `step` (the Tab
/// surface's one Catmull-Clark step), `c[a][b]` with `a` along f0 → f1 and
/// `b` along f0 → f3. Shared by sampling (`patch_surface`) and exact CAD
/// export (`nurbs.rs`).
fn patch_grids(step: &Net) -> Vec<[[DVec3; 3]; 3]> {
    let hard = |a: u32, b: u32| -> f64 {
        if step.sharp.is_empty() {
            0.0
        } else {
            // EW2-04 (Operandi choice): a negative weight w gives the
            // fraction w (sharpness / 10), extrapolating away from the
            // open-edge controls.
            let s = step.sharp.get(&edge_key(a, b)).copied().unwrap_or(0.0);
            if s < 0.0 { s / 10.0 } else { s.min(1.0) }
        }
    };
    let centroid = |f: &Vec<u32>| f.iter().map(|&v| step.points[v as usize]).sum::<DVec3>() / 4.0;
    let centers: Vec<DVec3> = step.faces.iter().map(centroid).collect();
    let mut edge_faces: crate::fxhash::FxHashMap<(u32, u32), Vec<usize>> = Default::default();
    let mut point_faces: Vec<Vec<usize>> = vec![Vec::new(); step.points.len()];
    for (fi, f) in step.faces.iter().enumerate() {
        for i in 0..4 {
            edge_faces
                .entry(edge_key(f[i], f[(i + 1) % 4]))
                .or_default()
                .push(fi);
            point_faces[f[i] as usize].push(fi);
        }
    }
    let edge_control = |a: u32, b: u32| {
        let faces = &edge_faces[&edge_key(a, b)];
        let mid = (step.points[a as usize] + step.points[b as usize]) / 2.0;
        if faces.len() == 2 {
            let smooth = (centers[faces[0]] + centers[faces[1]]) / 2.0;
            smooth + (mid - smooth) * hard(a, b)
        } else {
            mid
        }
    };
    let corner_control = |v: u32| {
        // Open edges at v, found from the quads around it.
        let mut open = Vec::new();
        for &fi in &point_faces[v as usize] {
            let f = &step.faces[fi];
            let i = f.iter().position(|&p| p == v).unwrap();
            for other in [f[(i + 1) % 4], f[(i + 3) % 4]] {
                let k = edge_key(v, other);
                if edge_faces[&k].len() != 2 && !open.contains(&k) {
                    open.push(k);
                }
            }
        }
        if open.is_empty() {
            let smooth = point_faces[v as usize]
                .iter()
                .map(|&fi| centers[fi])
                .sum::<DVec3>()
                / point_faces[v as usize].len() as f64;
            // EW-05: still-sharp edges at v, each once.
            let mut sharp: Vec<(u32, f64)> = Vec::new();
            for &fi in &point_faces[v as usize] {
                let f = &step.faces[fi];
                let i = f.iter().position(|&p| p == v).unwrap();
                for other in [f[(i + 1) % 4], f[(i + 3) % 4]] {
                    let h = hard(v, other);
                    if h != 0.0 && !sharp.iter().any(|&(o, _)| o == other) {
                        sharp.push((other, h));
                    }
                }
            }
            let p = step.points[v as usize];
            let (target, amount) = match sharp.len() {
                // One sharp edge: the normal control, like the crease rule's
                // smooth point.
                0 | 1 => return smooth,
                // A crease through v: the average of the two edges'
                // midpoints, as on an open boundary.
                2 => (
                    (p * 2.0 + step.points[sharp[0].0 as usize] + step.points[sharp[1].0 as usize])
                        / 4.0,
                    (sharp[0].1 + sharp[1].1) / 2.0,
                ),
                // Three or more: a corner that stays put.
                n => (p, sharp.iter().map(|s| s.1).sum::<f64>() / n as f64),
            };
            smooth + (target - smooth) * amount
        } else {
            open.iter()
                .map(|k| (step.points[k.0 as usize] + step.points[k.1 as usize]) / 2.0)
                .sum::<DVec3>()
                / open.len() as f64
        }
    };
    // Each point's corner control once, not once per quad around it.
    let corner_controls: Vec<DVec3> = (0..step.points.len() as u32)
        .map(|v| {
            if point_faces[v as usize].is_empty() {
                step.points[v as usize]
            } else {
                corner_control(v)
            }
        })
        .collect();
    let corner_control = |v: u32| corner_controls[v as usize];
    step.faces
        .iter()
        .enumerate()
        .map(|(fi, f)| {
            let mut c = [[DVec3::ZERO; 3]; 3];
            c[0][0] = corner_control(f[0]);
            c[2][0] = corner_control(f[1]);
            c[2][2] = corner_control(f[2]);
            c[0][2] = corner_control(f[3]);
            c[1][0] = edge_control(f[0], f[1]);
            c[2][1] = edge_control(f[1], f[2]);
            c[1][2] = edge_control(f[2], f[3]);
            c[0][1] = edge_control(f[3], f[0]);
            c[1][1] = centers[fi];
            c
        })
        .collect()
}

/// Refine one kind's polygons. `patch_kind` selects Subdivision (Tab).
fn refine(net: Net, level: u8, rule: BoundaryRule, patch_kind: bool) -> Net {
    let mut net = net;
    if level == 0 {
        return net;
    }
    if patch_kind {
        net = patch_surface(&net, level);
    } else {
        for _ in 0..level {
            net = catmull_clark_step(&net, rule, false);
        }
    }
    net
}

/// The display surface: each smooth polygon replaced by quads at its kind's
/// display level (SDV-04, SDV-12); Face polygons are kept as they are.
/// Face polygons cut off the smooth region (SDV-13); a connected region
/// mixing both smooth kinds is refined as Catmull-Clark (SDV-14).
pub fn subdivide(mesh: &Mesh, settings: &SubdivisionSettings) -> Result<Mesh> {
    Ok(display_surface(mesh, settings)?.0)
}

/// `subdivide`, plus the source polygon of every output polygon (for
/// drawing selection and rollover on the smooth surface).
pub fn display_surface(mesh: &Mesh, settings: &SubdivisionSettings) -> Result<(Mesh, Vec<usize>)> {
    surface(mesh, settings, false).map(|(m, s, _)| (m, s))
}

/// `display_surface`, plus per output polygon and corner whether the side
/// from that corner to the next lies on a cage edge: the wireframe
/// traced over the surface. Tracked through the refinement, so no edge
/// lookups are needed.
pub fn display_surface_traced(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
) -> Result<(Mesh, Vec<usize>, Vec<Vec<bool>>)> {
    surface(mesh, settings, false).map(|(m, s, sides)| {
        let traced = sides
            .into_iter()
            .map(|f| f.into_iter().map(|e| e.is_some()).collect())
            .collect();
        (m, s, traced)
    })
}

/// `display_surface_traced` naming the cage edge each traced side lies on,
/// so a cage edge (selected, weighted) can be drawn on the surface.
/// The surface carries the cage's UV maps, so image textures and other
/// UV-read materials map onto it; a drag moves points only
/// (`update_surface`), which leaves those rows valid.
pub fn display_surface_edges(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
) -> Result<(Mesh, Vec<usize>, Vec<Vec<Side>>)> {
    surface_with(mesh, settings, false, !mesh.uv_maps.is_empty())
}

/// For each cage polygon, the range of display-surface polygons it became
/// (`sources` from `display_surface`; each polygon's pieces are adjacent).
pub fn source_blocks(sources: &[usize], polygons: usize) -> Vec<std::ops::Range<usize>> {
    let mut blocks = vec![0..0; polygons];
    let mut start = 0;
    for i in 1..=sources.len() {
        if i == sources.len() || sources[i] != sources[start] {
            blocks[sources[start]] = start..i;
            start = i;
        }
    }
    blocks
}

/// Whether two cages differ only in point positions (same polygons, kinds
/// and creases), so a surface can be updated in place.
pub fn same_topology(a: &Mesh, b: &Mesh) -> bool {
    a.vertices.len() == b.vertices.len()
        && a.faces.len() == b.faces.len()
        && a.attributes.representations == b.attributes.representations
        && a.attributes.creases == b.attributes.creases
        && a.faces
            .iter()
            .zip(&b.faces)
            .all(|(x, y)| x.vertices == y.vertices)
}

/// The cage points whose surroundings changed from `old` to `new` when the
/// polygons are the same: points that moved, and both ends of every edge
/// whose crease was added, removed or reweighted (a crease changes only the
/// rules at its edge and its two ends, which is less than moving those ends).
/// Sorted. `None` when the polygons differ.
fn changed_points(old: &Mesh, new: &Mesh) -> Option<Vec<usize>> {
    if !(old.vertices.len() == new.vertices.len()
        && old.faces.len() == new.faces.len()
        && old.attributes.representations == new.attributes.representations
        && (crate::Shared::same(&old.faces, &new.faces)
            || old
                .faces
                .iter()
                .zip(&new.faces)
                .all(|(x, y)| x.vertices == y.vertices)))
    {
        return None;
    }
    let mut changed: Vec<usize> = crate::mesh::moved_points(&old.vertices, &new.vertices)
        .into_iter()
        .map(|v| v as usize)
        .collect();
    if old.attributes.creases != new.attributes.creases {
        let weights = |m: &Mesh| -> BTreeMap<(u32, u32), u64> {
            m.attributes
                .creases
                .iter()
                .map(|c| ((c.a.min(c.b), c.a.max(c.b)), c.weight.to_bits()))
                .collect()
        };
        let (was, now) = (weights(old), weights(new));
        for &(a, b) in was.keys().chain(now.keys()) {
            if was.get(&(a, b)) != now.get(&(a, b)) {
                changed.extend([a as usize, b as usize]);
            }
        }
        changed.sort_unstable();
        changed.dedup();
        changed.retain(|&v| v < new.vertices.len());
    }
    Some(changed)
}

/// The cage polygons whose surface pieces change from `old` to `new`
/// (points moved or creases changed): those touching a changed point and
/// their neighbours (pieces share points across an edge). Sorted. `None`
/// when the polygons differ.
pub fn dirty_faces(old: &Mesh, new: &Mesh) -> Option<Vec<usize>> {
    let changed = changed_points(old, new)?;
    if changed.is_empty() {
        return Some(Vec::new());
    }
    let point_faces = new.point_faces();
    let mut rings = Rings::new(new, &point_faces, &changed);
    rings.grow();
    Some(rings.sorted())
}

/// Cage polygons around a set of points, grown a ring at a time through
/// the cached point → polygon lists, with flags instead of sets so a drag
/// frame costs what the neighbourhood costs, not the whole mesh.
struct Rings<'a> {
    mesh: &'a Mesh,
    point_faces: &'a crate::topology_cache::PointFaces,
    inside: Vec<bool>,
    /// The polygons in the set, in the order they joined.
    faces: Vec<usize>,
    /// Where the last ring starts in `faces`.
    frontier: usize,
}
impl<'a> Rings<'a> {
    /// The polygons using any of `points`.
    fn new(
        mesh: &'a Mesh,
        point_faces: &'a crate::topology_cache::PointFaces,
        points: &[usize],
    ) -> Self {
        let mut rings = Rings {
            mesh,
            point_faces,
            inside: vec![false; mesh.faces.len()],
            faces: Vec::new(),
            frontier: 0,
        };
        for &v in points {
            rings.add_around(v as u32);
        }
        rings
    }
    fn add_around(&mut self, v: u32) {
        for &g in self.point_faces.of(v) {
            let g = g as usize;
            if !self.inside[g] {
                self.inside[g] = true;
                self.faces.push(g);
            }
        }
    }
    /// Add one ring: the polygons sharing a point with the last ring.
    fn grow(&mut self) {
        let (start, end) = (self.frontier, self.faces.len());
        for i in start..end {
            let f = self.faces[i];
            for &v in &self.mesh.faces[f].vertices {
                self.add_around(v);
            }
        }
        self.frontier = end;
    }
    fn sorted(&self) -> Vec<usize> {
        let mut out = self.faces.clone();
        out.sort_unstable();
        out
    }
}

/// Rings of context `update_surface` re-subdivides around the polygons
/// whose pieces change. A Catmull-Clark polygon's pieces, at any level,
/// depend only on the polygons sharing a point with it, so one ring makes
/// the changed pieces exact; Subdivision (Tab) is held to the same check
/// (`partial_updates_match_full_rebuilds` compares against full rebuilds).
const CONTEXT_RINGS: usize = 1;

thread_local! {
    static RESUBDIVIDED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many cage polygons `update_surface` has re-subdivided on this
/// thread (dirty polygons plus their context). Speed guards check that a
/// drag's per-frame region stays near what moved.
pub fn resubdivided_polygons() -> u64 {
    RESUBDIVIDED.get()
}

/// Update a display surface after points moved or creases changed,
/// recomputing only near them.
///
/// Subdivision is local: a cage polygon's pieces depend only on the
/// polygons sharing a point with it. So the polygons touching a moved point
/// (and their neighbours, whose pieces share points with them) are
/// re-subdivided together with `CONTEXT_RINGS` more rings of context, and
/// their pieces' points are copied into `surface`. Returns the cage polygons whose
/// pieces changed, or `None` when the topology changed or so much moved that
/// a full recompute is the better choice.
pub fn update_surface(
    old: &Mesh,
    new: &Mesh,
    settings: &SubdivisionSettings,
    surface: &mut Mesh,
    blocks: &[std::ops::Range<usize>],
) -> Option<Vec<usize>> {
    if blocks.len() != new.faces.len() {
        return None;
    }
    // Only point positions are rewritten: the surface's polygons and UV
    // rows stay as built. A UV edit changes no points, so the caller must
    // rebuild the surface when the cage's UVs change (SubdivisionCache).
    let moved = changed_points(old, new)?;
    if moved.is_empty() {
        return Some(Vec::new());
    }
    // Point → polygon lists come from the per-topology memo: a drag's
    // frames all share the same polygons.
    let point_faces = new.point_faces();
    let mut rings = Rings::new(new, &point_faces, &moved);
    rings.grow();
    let dirty = rings.sorted();
    let is_dirty = rings.inside.clone();
    for _ in 0..CONTEXT_RINGS {
        rings.grow();
    }
    if rings.faces.len() * 2 > new.faces.len() {
        return None;
    }
    // The region as its own small cage, in polygon order, its points
    // numbered in first use (the refinement numbers its nets the same way
    // whatever the cage's numbering, so the pieces come out the same).
    let order = rings.sorted();
    let mut local = vec![u32::MAX; new.vertices.len()];
    let mut sub = Mesh::default();
    sub.faces = order
        .iter()
        .map(|&f| {
            let face = &new.faces[f];
            let vertices = face
                .vertices
                .iter()
                .map(|&v| {
                    let slot = &mut local[v as usize];
                    if *slot == u32::MAX {
                        *slot = sub.vertices.len() as u32;
                        sub.vertices.push(new.vertices[v as usize]);
                    }
                    *slot
                })
                .collect();
            Face {
                vertices,
                uv: None,
                material: String::new(),
                smooth: face.smooth,
                part: None,
                material_label: None,
            }
        })
        .collect();
    if !new.attributes.representations.is_empty() {
        // The whole mesh's kinds as subdivided (SDV-14), so a region cut out
        // of a mixed area isn't judged on its own.
        let kinds = effective_kinds(new);
        sub.attributes.representations = order.iter().map(|&f| kinds[f]).collect();
    }
    let in_region = |v: u32| local.get(v as usize).copied().filter(|&l| l != u32::MAX);
    sub.attributes.creases = new
        .attributes
        .creases
        .iter()
        .filter_map(|c| {
            Some(crate::MeshCrease {
                a: in_region(c.a)?,
                b: in_region(c.b)?,
                weight: c.weight,
            })
        })
        .collect();
    RESUBDIVIDED.set(RESUBDIVIDED.get() + order.len() as u64);
    let (sub_out, sub_sources, _) = self::surface(&sub, settings, false).ok()?;
    let sub_blocks = source_blocks(&sub_sources, order.len());
    for (local, &f) in order.iter().enumerate() {
        if !is_dirty[f] {
            continue;
        }
        let (global, piece) = (blocks[f].clone(), sub_blocks[local].clone());
        if global.len() != piece.len() {
            return None;
        }
        for (g, p) in global.zip(piece) {
            for (&gv, &pv) in surface.faces[g]
                .vertices
                .iter()
                .zip(&sub_out.faces[p].vertices)
            {
                surface.vertices[gv as usize] = sub_out.vertices[pv as usize];
            }
        }
    }
    Some(dirty)
}

/// The surface; `freeze` also carries the UV maps and Freeze's crease
/// rules (SDV-15).
fn surface(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
    freeze: bool,
) -> Result<(Mesh, Vec<usize>, Vec<Vec<Side>>)> {
    surface_with(mesh, settings, freeze, freeze)
}

/// A local net of `mesh`'s polygons `source`: their points renumbered in
/// first-use order, their cage sides, and the creases among them
/// (sharpness = 10 × weight).
fn local_net(mesh: &Mesh, source: &[usize]) -> Net {
    let cage_sides = |f: &Face| -> Vec<Side> {
        let v = &f.vertices;
        (0..v.len())
            .map(|i| Some(edge_key(v[i], v[(i + 1) % v.len()])))
            .collect()
    };
    let mut local: crate::fxhash::FxHashMap<u32, u32> = Default::default();
    let mut net = Net::default();
    for &f in source {
        let face = mesh.faces[f]
            .vertices
            .iter()
            .map(|&v| {
                let next = net.points.len() as u32;
                *local.entry(v).or_insert_with(|| {
                    net.points.push(mesh.vertices[v as usize]);
                    next
                })
            })
            .collect();
        net.sides.push(cage_sides(&mesh.faces[f]));
        net.faces.push(face);
    }
    for c in &mesh.attributes.creases {
        if let (Some(&a), Some(&b)) = (local.get(&c.a), local.get(&c.b)) {
            net.sharp
                .insert(edge_key(a, b), 10.0 * c.weight.clamp(-1.0, 1.0));
        }
    }
    net
}

/// `surface` with the UV maps carried on their own (`with_uvs`), for
/// display.
fn surface_with(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
    freeze: bool,
    with_uvs: bool,
) -> Result<(Mesh, Vec<usize>, Vec<Vec<Side>>)> {
    let (subdivision_level, catmull_clark_level, rule) = (
        settings.subdivision_level,
        settings.catmull_clark_level,
        settings.boundary_rule,
    );
    if subdivision_level > MAX_LEVEL || catmull_clark_level > MAX_LEVEL {
        return Err(format!("Subdivision levels go up to {MAX_LEVEL}"));
    }
    let mut out = Mesh {
        vertices: mesh.vertices.clone(),
        ..Default::default()
    };
    let plain = |f: &Face| Face {
        vertices: f.vertices.clone(),
        uv: None,
        material: f.material.clone(),
        smooth: f.smooth,
        part: f.part.clone(),
        material_label: f.material_label.clone(),
    };
    let mut sources = Vec::new();
    let mut traced: Vec<Vec<Side>> = Vec::new();
    let cage_sides = |f: &Face| -> Vec<Side> {
        let v = &f.vertices;
        (0..v.len())
            .map(|i| Some(edge_key(v[i], v[(i + 1) % v.len()])))
            .collect()
    };
    // Per UV map, per output polygon, per corner.
    let mut uvs: BTreeMap<String, Vec<Vec<Option<DVec2>>>> = if with_uvs {
        mesh.uv_maps
            .keys()
            .map(|k| (k.clone(), Vec::new()))
            .collect()
    } else {
        BTreeMap::new()
    };
    let kinds = effective_kinds(mesh);
    for (fi, f) in mesh.faces.iter().enumerate() {
        if !is_smooth(kinds[fi]) && !mesh.is_polyline(fi) {
            out.faces.push(plain(f));
            sources.push(fi);
            traced.push(cage_sides(f));
            for (name, map) in &mut uvs {
                map.push(mesh.uv_maps[name][fi].clone());
            }
        }
    }
    for (rep, level, patch_kind) in [
        (
            PolygonRepresentation::CatmullClark,
            catmull_clark_level,
            false,
        ),
        (PolygonRepresentation::Subdivs, subdivision_level, true),
    ] {
        let source: Vec<usize> = (0..mesh.faces.len()).filter(|&f| kinds[f] == rep).collect();
        if source.is_empty() {
            continue;
        }
        // SDV-04: each polygon corner becomes 4^(L-1) Catmull-Clark quads,
        // or L^2 Subdivision quads.
        let per_corner = match (level, patch_kind) {
            (0, _) => 0,
            (l, true) => (l as usize).pow(2),
            (l, false) => 4usize.pow(l as u32 - 1),
        };
        let net = local_net(mesh, &source);
        let refined = refine(net, level, rule, patch_kind);
        // Each refined polygon descends from one source polygon, n·4^(L-1)
        // children each (level 0 keeps the polygon itself).
        let mut owner = Vec::with_capacity(refined.faces.len());
        for &f in &source {
            let n = if level == 0 {
                1
            } else {
                mesh.faces[f].vertices.len() * per_corner
            };
            owner.extend(std::iter::repeat_n(f, n));
        }
        // SDV-15: UVs follow a straight linear subdivision of each polygon,
        // in the same polygon order the refinement produced.
        for (name, map) in &mut uvs {
            for &f in &source {
                linear_uvs(&mesh.uv_maps[name][f], level, patch_kind, map);
            }
        }
        let base = out.vertices.len() as u32;
        out.vertices.extend(&refined.points);
        traced.extend(refined.sides);
        for (i, face) in refined.faces.into_iter().enumerate() {
            let source = &mesh.faces[owner[i]];
            out.faces.push(Face {
                vertices: face.into_iter().map(|v| v + base).collect(),
                ..plain(source)
            });
            sources.push(owner[i]);
        }
    }
    if freeze {
        // EW-10 (Operandi choice): Freeze drops the weights of frozen
        // edges; edges of Face polygons, which keep their point numbers
        // until `drop_unused`, keep theirs.
        let n = mesh.vertices.len() as u32;
        let kept: BTreeSet<(u32, u32)> = out
            .faces
            .iter()
            .flat_map(|f| {
                let v = &f.vertices;
                (0..v.len()).map(move |i| edge_key(v[i], v[(i + 1) % v.len()]))
            })
            .filter(|&(a, b)| b < n && a < n)
            .collect();
        out.attributes.creases = mesh
            .attributes
            .creases
            .iter()
            .filter(|c| kept.contains(&(c.a, c.b)))
            .cloned()
            .collect();
    }
    drop_unused(&mut out);
    out.uv_maps = uvs.into();
    Ok((out, sources, traced))
}

/// Append the UVs of one source polygon's refined polygons, in refinement
/// order: Catmull-Clark corner quads split into four children per level
/// (lexicographic), Subdivision corner quads split into an L x L grid. A
/// polygon with any unmapped corner stays unmapped.
pub(crate) fn linear_uvs(
    corners: &[Option<DVec2>],
    level: u8,
    patch_kind: bool,
    out: &mut Vec<Vec<Option<DVec2>>>,
) {
    let n = corners.len();
    let push_blank = |out: &mut Vec<Vec<Option<DVec2>>>, count: usize| {
        out.extend(std::iter::repeat_n(vec![None; 4], count));
    };
    if level == 0 {
        out.push(corners.to_vec());
        return;
    }
    let per_corner = if patch_kind {
        (level as usize).pow(2)
    } else {
        4usize.pow(level as u32 - 1)
    };
    let Some(uv): Option<Vec<DVec2>> = corners.iter().copied().collect() else {
        push_blank(out, n * per_corner);
        return;
    };
    let center = uv.iter().copied().sum::<DVec2>() / n as f64;
    fn bilinear(q: &[DVec2; 4], s: f64, t: f64) -> DVec2 {
        q[0] * (1.0 - s) * (1.0 - t) + q[1] * s * (1.0 - t) + q[2] * s * t + q[3] * (1.0 - s) * t
    }
    fn split(q: [DVec2; 4], depth: u8, out: &mut Vec<Vec<Option<DVec2>>>) {
        if depth == 0 {
            out.push(q.iter().map(|&p| Some(p)).collect());
            return;
        }
        let c = (q[0] + q[1] + q[2] + q[3]) / 4.0;
        for i in 0..4 {
            let next = (q[i] + q[(i + 1) % 4]) / 2.0;
            let prev = (q[(i + 3) % 4] + q[i]) / 2.0;
            split([q[i], next, c, prev], depth - 1, out);
        }
    }
    for i in 0..n {
        let quad = [
            uv[i],
            (uv[i] + uv[(i + 1) % n]) / 2.0,
            center,
            (uv[(i + n - 1) % n] + uv[i]) / 2.0,
        ];
        if patch_kind {
            let l = level as f64;
            for a in 0..level {
                for b in 0..level {
                    let (s0, s1, t0, t1) = (
                        a as f64 / l,
                        (a + 1) as f64 / l,
                        b as f64 / l,
                        (b + 1) as f64 / l,
                    );
                    out.push(vec![
                        Some(bilinear(&quad, s0, t0)),
                        Some(bilinear(&quad, s1, t0)),
                        Some(bilinear(&quad, s1, t1)),
                        Some(bilinear(&quad, s0, t1)),
                    ]);
                }
            }
        } else {
            split(quad, level - 1, out);
        }
    }
}

/// Whether polygon `face` of `mesh` is drawn as part of a smooth surface.
pub fn is_smooth_polygon(mesh: &Mesh, face: usize) -> bool {
    is_smooth(kind(mesh, face))
}

/// Remove points no polygon uses, keeping order.
fn drop_unused(mesh: &mut Mesh) {
    let mut used = vec![false; mesh.vertices.len()];
    for f in &mesh.faces {
        for &v in &f.vertices {
            used[v as usize] = true;
        }
    }
    let mut remap = vec![0u32; mesh.vertices.len()];
    let mut points = Vec::new();
    for (i, p) in mesh.vertices.iter().enumerate() {
        if used[i] {
            remap[i] = points.len() as u32;
            points.push(*p);
        }
    }
    mesh.vertices = points;
    for f in &mut mesh.faces {
        for v in &mut f.vertices {
            *v = remap[*v as usize];
        }
    }
    for c in &mut mesh.attributes.creases {
        let (a, b) = (remap[c.a as usize], remap[c.b as usize]);
        (c.a, c.b) = (a.min(b), a.max(b));
    }
    mesh.attributes.creases.sort_by_key(|c| (c.a, c.b));
}

/// SDV-12 Freeze: smooth polygons become plain Face quads at the display
/// level. Points of Face polygons keep their indices first. UV maps carry
/// through as a linear subdivision of each polygon's UVs (SDV-15; exact for
/// one-quad islands, provisional for larger ones, SDV-Q06). Weights on
/// frozen edges and other corner attributes are dropped (EW-10); edges of
/// Face polygons keep their weights.
pub fn freeze(mesh: &Mesh, settings: &SubdivisionSettings) -> Result<Mesh> {
    freeze_only(mesh, settings, None)
}

/// SDV-12 Freeze of only the smooth polygons flagged in `only` (FRZ-01,
/// Polygon mode); `None` freezes them all. The others keep their cage and
/// kind. The flagged polygons freeze as a patch of their own, so along the
/// border with unflagged smooth polygons the result follows the patch's
/// boundary rule (provisional, FRZ-B10).
pub fn freeze_only(
    mesh: &Mesh,
    settings: &SubdivisionSettings,
    only: Option<&[bool]>,
) -> Result<Mesh> {
    let held: Vec<bool> = (0..mesh.faces.len())
        .map(|f| {
            is_smooth(kind(mesh, f))
                && !mesh.is_polyline(f)
                && only.is_some_and(|o| !o.get(f).copied().unwrap_or(false))
        })
        .collect();
    if !(0..mesh.faces.len()).any(|f| is_smooth(kind(mesh, f)) && !held[f]) {
        return Err("The mesh has no subdivision polygons to freeze".into());
    }
    let mut input = std::borrow::Cow::Borrowed(mesh);
    if held.contains(&true) {
        let m = input.to_mut();
        for (f, &h) in held.iter().enumerate() {
            if h {
                m.attributes.representations[f] = PolygonRepresentation::Face;
            }
        }
    }
    let (mut out, sources, _) = surface(&input, settings, true)?;
    if held.contains(&true) {
        out.attributes.representations = sources
            .iter()
            .map(|&s| {
                if held[s] {
                    kind(mesh, s)
                } else {
                    PolygonRepresentation::Face
                }
            })
            .collect();
    }
    out.attributes.extrude_edge_uses = crate::interchange::edge_exceptions(&out);
    if mesh.has_polylines() {
        // FRZ-07: lines (straight polylines, frozen curves) come through
        // unchanged after the surface. A point a line shares with the cage
        // becomes the line's own point (provisional).
        crate::mesh_attributes::append_mesh(&mut out, lines_of(mesh))?;
    }
    out.validate()?;
    Ok(out)
}

/// The mesh's line elements on their own points, in order.
fn lines_of(mesh: &Mesh) -> Mesh {
    let mut out = Mesh::default();
    let mut local = vec![u32::MAX; mesh.vertices.len()];
    for (f, face) in mesh.faces.iter().enumerate() {
        if !mesh.is_polyline(f) {
            continue;
        }
        let vertices = face
            .vertices
            .iter()
            .map(|&v| {
                if local[v as usize] == u32::MAX {
                    local[v as usize] = out.vertices.len() as u32;
                    out.vertices.push(mesh.vertices[v as usize]);
                }
                local[v as usize]
            })
            .collect();
        out.faces.push(Face {
            vertices,
            uv: None,
            ..face.clone()
        });
        out.attributes
            .representations
            .push(mesh.attributes.representations[f]);
    }
    out
}

impl crate::Editor {
    pub(crate) fn subdivision_ready(&self, what: &str) -> Result<()> {
        if self.session.live.is_some()
            || self.session.bridge.is_some()
            || self.session.spike.is_some()
            || self.session.cone.is_some()
            || self.session.ellipsoid.is_some()
            || self.session.shape.is_some()
            || self.session.reduction.is_some()
            || self.selection_start.is_some()
        {
            return Err(format!("Drop or Cancel the active task before {what}"));
        }
        self.scene.validate()
    }

    /// Refuse to show a subdivided surface that needs more memory than is
    /// free (Jason, 2026-11-13: Tab had no check). Without a free-memory
    /// source (`Editor::free_memory`) nothing is checked.
    fn check_display_memory(
        &self,
        mesh: &Mesh,
        settings: &SubdivisionSettings,
        what: &str,
    ) -> Result<()> {
        let crate::FreeMemory(Some(free_memory)) = self.free_memory else {
            return Ok(());
        };
        let bytes = display_bytes(mesh, settings);
        let free = free_memory();
        if bytes > free {
            return Err(format!(
                "{what}: showing {} subdivided polygons needs about {} of memory, but only {} is free; nothing changed",
                crate::polygon_subdivide::group_thousands(display_pieces(mesh, settings)),
                crate::polygon_subdivide::gigabytes(bytes),
                crate::polygon_subdivide::gigabytes(free)
            ));
        }
        Ok(())
    }

    /// SDV-01/02. In Polygon mode the selected polygons change; with none
    /// selected, or in another mode, every polygon of the active mesh does.
    pub(crate) fn toggle_subdivision(&mut self, which: Toggle) -> Result<()> {
        let label = match which {
            Toggle::Subdivision => "Toggle Subdivision",
            Toggle::CatmullClark => "Toggle Catmull-Clark",
        };
        self.subdivision_ready(label)?;
        let selected = if self.scene.selection.mode == crate::Mode::Polygon {
            self.scene.selection.faces.clone()
        } else {
            Vec::new()
        };
        let item = self
            .scene
            .active()
            .filter(|i| self.scene.shows(i))
            .ok_or("Select a visible mesh to subdivide")?;
        let mesh = toggle(&item.mesh, &selected, which)?;
        let settings = item.subdivision.clone().unwrap_or_default();
        self.check_display_memory(&mesh, &settings, label)?;
        let mut candidate = self.scene.clone();
        candidate.active_mut().ok_or("Missing mesh")?.mesh = mesh.into();
        candidate.validate()?;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record(label.into(), before);
        Ok(())
    }

    /// SDV-03: levels and rules for the active mesh, one undo step.
    pub(crate) fn set_subdivision(&mut self, settings: SubdivisionSettings) -> Result<()> {
        self.subdivision_ready("changing subdivision")?;
        if settings.subdivision_level > MAX_LEVEL
            || settings.catmull_clark_level > MAX_LEVEL
            || settings.subdivision_render_level > MAX_LEVEL
            || settings.catmull_clark_render_level > MAX_LEVEL
        {
            return Err(format!("Subdivision levels go up to {MAX_LEVEL}"));
        }
        let item = self.scene.active().ok_or("Select a mesh")?;
        if item.subdivision.clone().unwrap_or_default() == settings {
            return Ok(());
        }
        self.check_display_memory(&item.mesh, &settings, "Subdivision Settings")?;
        let mut candidate = self.scene.clone();
        candidate.active_mut().ok_or("Missing mesh")?.subdivision =
            (settings != SubdivisionSettings::default()).then_some(settings);
        let before = std::mem::replace(&mut self.scene, candidate);
        self.record("Subdivision Settings".into(), before);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
