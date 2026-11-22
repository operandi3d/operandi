//! MP-C04 Polygon Triple (TRI-001..004): replace each explicitly selected
//! simple polygon of the active mesh (nonplanar ones are triangulated in their
//! best-fit plane) with `n - 2` triangles over its
//! existing boundary vertices, as one undoable edit.
//!
//! Technique (Operandi's own, from general computational-geometry practice):
//! ear clipping in the polygon's best-fit plane chart. The chart normal is the
//! Newell normal, so the source winding is counter-clockwise in the chart.
//! Simplicity is checked explicitly before clipping, and every result is
//! checked afterwards by area and edge-pairing postconditions that do not use
//! the clipping state. Diagonals are deterministic for identical input
//! (mesh, selection set); no particular tie-break between valid decompositions
//! is claimed, and none reproduces a reference product's triangulation.
//!
//! Numerical eligibility, relative to the polygon's bounding-box diagonal `L`:
//! * planarity is not required: corners are projected onto the Newell plane
//!   and the projection must be simple;
//! * nondegenerate: polygon and every output triangle have area greater than
//!   `TRIPLE_AREA_TOLERANCE * L²`; every edge is longer than
//!   `TRIPLE_LENGTH_TOLERANCE * L`;
//! * simple: nonadjacent edges stay farther apart than that length tolerance
//!   and no adjacent edge pair folds back onto itself.
//!
//! Corners collinear with their neighbours stay triangle corners (no point is
//! dropped or added) and are never used as an ear tip. A diagonal that already
//! exists as a mesh edge (or as another selected polygon's new diagonal) is
//! never used, since it would make a non-manifold edge; if every remaining ear
//! needs such a diagonal the whole edit is refused. Ear search is O(n²) for a
//! convex n-gon and at worst O(n³) within the 4096-corner face budget.
use crate::{Editor, Face, Mesh, MeshAttributes, Mode, Result, TypedMap};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum plane distance of a corner, as a fraction of the polygon extent.
/// Loose enough for single-precision imported coordinates, far below any
/// visible fold.
pub const TRIPLE_PLANARITY_TOLERANCE: f64 = 1e-6;
/// Minimum edge length and edge-to-edge clearance, as a fraction of extent.
pub const TRIPLE_LENGTH_TOLERANCE: f64 = 1e-9;
/// Minimum polygon/triangle area, as a fraction of the squared extent.
pub const TRIPLE_AREA_TOLERANCE: f64 = 1e-12;

/// Source of one output face of [`triple_faces`]. Also the explicit identity
/// correspondence consumed by `MeshIdentity::split_faces`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct FaceSplitOrigin {
    /// Source face index.
    pub face: usize,
    /// `None`: the source face survives unchanged at the same index.
    /// `Some`: source corner ordinals of this piece, in output corner order.
    pub corners: Option<Vec<usize>>,
}

/// A nonneutral Triple result for one mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct TripleOutcome {
    pub mesh: Mesh,
    /// One entry per output face.
    pub origins: Vec<FaceSplitOrigin>,
    /// Output faces replacing each split source face, in triangle order. The
    /// first piece reuses the source index; later pieces are appended.
    pub pieces: BTreeMap<u32, Vec<u32>>,
}

/// Best-fit plane chart of one polygon. Shared with Merge (MER-001), which
/// applies the same eligibility tolerances to its merged boundary.
pub(crate) struct Chart {
    pub(crate) points: Vec<DVec2>,
    pub(crate) normal: DVec3,
    /// Twice the polygon area (`|Newell vector|`).
    pub(crate) area2: f64,
    pub(crate) extent: f64,
}

pub(crate) fn chart(points: &[DVec3]) -> Result<Chart> {
    chart_projected(points, true)
}

/// Like [`chart`], but with `strict == false` a nonplanar polygon is projected
/// onto its Newell (best-fit) plane instead of refused. Triple uses this: it
/// splits any polygon with more than three vertices.
pub(crate) fn chart_projected(points: &[DVec3], strict: bool) -> Result<Chart> {
    if points.len() < 3 {
        return Err("a polygon needs at least three corners".into());
    }
    if points.iter().any(|p| !p.is_finite()) {
        return Err("non-finite corner position".into());
    }
    let lo = points.iter().copied().fold(DVec3::INFINITY, DVec3::min);
    let hi = points.iter().copied().fold(DVec3::NEG_INFINITY, DVec3::max);
    let extent = (hi - lo).length();
    if !extent.is_finite() || extent <= 0. {
        return Err("degenerate polygon (all corners coincide)".into());
    }
    let centroid = points.iter().copied().sum::<DVec3>() / points.len() as f64;
    let mut newell = DVec3::ZERO;
    for (i, &p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        newell += (p - centroid).cross(q - centroid);
    }
    let area2 = newell.length();
    if area2.is_nan() || area2 <= 2. * TRIPLE_AREA_TOLERANCE * extent * extent {
        return Err("zero-area polygon".into());
    }
    let normal = newell / area2;
    if strict
        && points
            .iter()
            .any(|&p| (p - centroid).dot(normal).abs() > TRIPLE_PLANARITY_TOLERANCE * extent)
    {
        return Err("materially nonplanar polygon".into());
    }
    // (u, v, normal) is right-handed, so the source winding is CCW in (u, v).
    let seed = if normal.x.abs() < 0.8 {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = normal.cross(seed).normalize();
    let v = normal.cross(u);
    let chart: Vec<DVec2> = points
        .iter()
        .map(|&p| {
            let d = p - centroid;
            DVec2::new(d.dot(u), d.dot(v))
        })
        .collect();
    Ok(Chart {
        points: chart,
        normal,
        area2,
        extent,
    })
}

/// Twice the signed area of triangle `abc` (positive when CCW).
fn cross(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b - a).perp_dot(c - a)
}

fn point_segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared()).clamp(0., 1.);
    (a + d * t).distance(p)
}

fn segment_distance(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> f64 {
    let (o1, o2) = (cross(a, b, c), cross(a, b, d));
    let (o3, o4) = (cross(c, d, a), cross(c, d, b));
    if o1 * o2 < 0. && o3 * o4 < 0. {
        return 0.;
    }
    point_segment_distance(a, c, d)
        .min(point_segment_distance(b, c, d))
        .min(point_segment_distance(c, a, b))
        .min(point_segment_distance(d, a, b))
}

/// Refuse zero-length edges, touching/crossing nonadjacent edges and adjacent
/// edges that fold back. O(n²) edge pairs.
pub(crate) fn require_simple(q: &[DVec2], eps: f64) -> Result<()> {
    let n = q.len();
    let edge = |i: usize| (q[i], q[(i + 1) % n]);
    for i in 0..n {
        let (a, b) = edge(i);
        if a.distance(b) <= eps {
            return Err("zero-length polygon edge".into());
        }
    }
    for i in 0..n {
        let (a, b) = edge(i);
        for j in i + 1..n {
            let (c, d) = edge(j);
            let touching = if j == i + 1 {
                // Shared corner b == c.
                point_segment_distance(a, c, d) <= eps || point_segment_distance(d, a, b) <= eps
            } else if i == 0 && j == n - 1 {
                // Shared corner a == d.
                point_segment_distance(b, c, d) <= eps || point_segment_distance(c, a, b) <= eps
            } else {
                segment_distance(a, b, c, d) <= eps
            };
            if touching {
                return Err("self-intersecting, touching or folded polygon boundary".into());
            }
        }
    }
    Ok(())
}

/// Triangulate one simple planar polygon given its corner positions in
/// boundary order. Returns `n - 2` corner-ordinal triangles in source winding.
/// `forbidden(a, b)` names corner pairs that must not become a diagonal.
pub fn triangulate_polygon(
    points: &[DVec3],
    forbidden: impl Fn(usize, usize) -> bool,
) -> Result<Vec<[usize; 3]>> {
    let n = points.len();
    if n > 4096 {
        return Err("polygon exceeds the 4096-corner budget".into());
    }
    let plane = chart_projected(points, false)?;
    let q = &plane.points;
    let eps = TRIPLE_LENGTH_TOLERANCE * plane.extent;
    let min_area2 = 2. * TRIPLE_AREA_TOLERANCE * plane.extent * plane.extent;
    require_simple(q, eps)?;
    let signed2: f64 = (0..n).map(|i| q[i].perp_dot(q[(i + 1) % n])).sum();
    if signed2.is_nan() || signed2 <= min_area2 {
        return Err("polygon winding/area is inconsistent with its plane".into());
    }
    if n == 3 {
        return Ok(vec![[0, 1, 2]]);
    }
    let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
    let mut remaining = n;
    let mut triangles = Vec::with_capacity(n - 2);
    let mut tip = 0;
    let mut misses = 0;
    let mut blocked_by_edge = false;
    while remaining > 3 {
        let (a, c) = (prev[tip], next[tip]);
        let ear = cross(q[a], q[tip], q[c]) > min_area2 && {
            // No other remaining corner inside or within eps of the triangle.
            let mut p = next[c];
            let mut clear = true;
            while p != a {
                let inside = cross(q[a], q[tip], q[p]) >= 0.
                    && cross(q[tip], q[c], q[p]) >= 0.
                    && cross(q[c], q[a], q[p]) >= 0.;
                if inside
                    || point_segment_distance(q[p], q[a], q[tip]) <= eps
                    || point_segment_distance(q[p], q[tip], q[c]) <= eps
                    || point_segment_distance(q[p], q[c], q[a]) <= eps
                {
                    clear = false;
                    break;
                }
                p = next[p];
            }
            clear
        };
        if ear && forbidden(a, c) {
            blocked_by_edge = true;
        } else if ear {
            triangles.push([a, tip, c]);
            next[a] = c;
            prev[c] = a;
            remaining -= 1;
            misses = 0;
            tip = c;
            continue;
        }
        tip = next[tip];
        misses += 1;
        if misses > remaining {
            return Err(if blocked_by_edge {
                "every remaining diagonal would duplicate an existing mesh edge".into()
            } else {
                "polygon cannot be triangulated without repair".into()
            });
        }
    }
    let last = [prev[tip], tip, next[tip]];
    if cross(q[last[0]], q[last[1]], q[last[2]]) <= min_area2 {
        return Err("polygon cannot be triangulated into nondegenerate triangles".into());
    }
    triangles.push(last);
    verify_triangulation(points, &plane, &triangles)?;
    Ok(triangles)
}

/// Postconditions independent of the clipping state: `n - 2` positively
/// oriented, nondegenerate triangles whose area vectors sum to the polygon's;
/// each source boundary edge used once in source direction and every other
/// directed edge paired once with its reverse (no T-junctions or overlaps).
fn verify_triangulation(points: &[DVec3], chart: &Chart, triangles: &[[usize; 3]]) -> Result<()> {
    let n = points.len();
    let fail = || Err::<(), String>("internal triangulation check failed; nothing changed".into());
    if triangles.len() != n - 2 {
        return fail();
    }
    let min_area2 = 2. * TRIPLE_AREA_TOLERANCE * chart.extent * chart.extent;
    let mut total = DVec3::ZERO;
    let mut uses = BTreeMap::<(usize, usize), u32>::new();
    for t in triangles {
        let [a, b, c] = [points[t[0]], points[t[1]], points[t[2]]];
        let area = (b - a).cross(c - a);
        if area.dot(chart.normal) <= min_area2 {
            return fail();
        }
        total += area;
        for k in 0..3 {
            *uses.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    if (total.dot(chart.normal) - chart.area2).abs() > 1e-9 * chart.area2 {
        return fail();
    }
    for i in 0..n {
        if uses.get(&(i, (i + 1) % n)) != Some(&1) || uses.contains_key(&((i + 1) % n, i)) {
            return fail();
        }
    }
    for (&(a, b), &count) in &uses {
        let boundary = b == (a + 1) % n;
        if count != 1 || (!boundary && uses.get(&(b, a)) != Some(&1)) {
            return fail();
        }
    }
    Ok(())
}

fn corner_rows<T: Clone>(rows: &[Vec<T>], origins: &[FaceSplitOrigin]) -> Vec<Vec<T>> {
    origins
        .iter()
        .map(|o| match &o.corners {
            None => rows[o.face].clone(),
            Some(cs) => cs.iter().map(|&c| rows[o.face][c].clone()).collect(),
        })
        .collect()
}

fn per_corner_maps<T: Clone>(
    maps: &BTreeMap<String, Vec<Vec<T>>>,
    origins: &[FaceSplitOrigin],
) -> BTreeMap<String, Vec<Vec<T>>> {
    maps.iter()
        .map(|(name, rows)| (name.clone(), corner_rows(rows, origins)))
        .collect()
}

fn face_rows<T: Clone>(rows: &[T], origins: &[FaceSplitOrigin]) -> Vec<T> {
    origins.iter().map(|o| rows[o.face].clone()).collect()
}

/// Rebuild every face/corner-indexed authored datum through `origins`. The
/// destructuring is deliberately exhaustive: a new attribute class fails to
/// compile here until its Triple mapping is decided.
pub(crate) fn remap_mesh(
    mesh: &Mesh,
    origins: &[FaceSplitOrigin],
    pieces: &BTreeMap<u32, Vec<u32>>,
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
        .map(|o| {
            let Face {
                vertices,
                uv,
                material,
                smooth,
                part,
                material_label,
            } = &faces[o.face];
            match &o.corners {
                None => faces[o.face].clone(),
                Some(cs) => Face {
                    vertices: cs.iter().map(|&c| vertices[c]).collect(),
                    uv: uv.as_ref().map(|uv| cs.iter().map(|&c| uv[c]).collect()),
                    material: material.clone(),
                    smooth: *smooth,
                    part: part.clone(),
                    material_label: material_label.clone(),
                },
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
                .filter(|(_, o)| set.contains(&(o.face as u32)))
                .map(|(i, _)| i as u32)
                .collect();
            (name.clone(), members)
        })
        .collect();
    // Exceptional edge uses follow the piece that owns the same directed edge.
    // Mesh validation lists an edge's uses in ascending face order, and an
    // early face's boundary can move to an appended piece, so re-sort.
    let mut edge_uses = BTreeMap::new();
    for (key, uses) in extrude_edge_uses {
        let mut mapped = uses
            .iter()
            .map(|&[f, a, b]| match pieces.get(&f) {
                None => Ok([f, a, b]),
                Some(ps) => ps
                    .iter()
                    .copied()
                    .find(|&p| {
                        let vs = &out_faces[p as usize].vertices;
                        (0..vs.len()).any(|i| vs[i] == a && vs[(i + 1) % vs.len()] == b)
                    })
                    .map(|p| [p, a, b])
                    .ok_or_else(|| {
                        String::from("Triple cannot map an exceptional edge use; nothing changed")
                    }),
            })
            .collect::<Result<Vec<_>>>()?;
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
            creases: creases.clone(),
            uv_records: per_corner_maps(uv_records, origins),
            normal_map_order: normal_map_order.clone(),
            corner_normals: per_corner_maps(corner_normals, origins),
            point_weights: point_weights.clone(),
            point_colors: point_colors.clone(),
            // Selected degenerate faces are refused, so every listed face is
            // retained at its own index.
            degenerate_faces: degenerate_faces.clone(),
        })
        .into(),
        vertices: vertices.clone(),
        faces: out_faces.into(),
        uv_maps: per_corner_maps(uv_maps, origins).into(),
    })
}

/// Triple the selected faces of `mesh`. `Ok(None)` means every selected face
/// is already a triangle (exact no-op). Errors leave nothing changed.
pub fn triple_faces(mesh: &Mesh, selected: &[u32]) -> Result<Option<TripleOutcome>> {
    mesh.validate()?;
    if selected.is_empty() {
        return Err("Select polygons before Triple".into());
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
            return Err("Triple supports surface polygons only; nothing changed".into());
        }
        if mesh.attributes.degenerate_faces.contains(&f) {
            return Err(format!(
                "Triple refuses degenerate cage polygon {f}; nothing changed"
            ));
        }
    }
    // Ascending source order makes the result independent of selection order.
    let mut edges: BTreeSet<(u32, u32)> = mesh.edges().into_iter().collect();
    let mut splits = BTreeMap::new();
    for &f in &targets {
        let ids = &mesh.faces[f as usize].vertices;
        if ids.len() == 3 {
            continue;
        }
        let points: Vec<DVec3> = ids.iter().map(|&v| mesh.vertices[v as usize]).collect();
        let key = |a: usize, b: usize| (ids[a].min(ids[b]), ids[a].max(ids[b]));
        let triangles = triangulate_polygon(&points, |a, b| edges.contains(&key(a, b)))
            .map_err(|e| format!("Triple refuses polygon {f}: {e}; nothing changed"))?;
        for t in &triangles {
            for k in 0..3 {
                edges.insert(key(t[k], t[(k + 1) % 3]));
            }
        }
        splits.insert(f, triangles);
    }
    if splits.is_empty() {
        return Ok(None);
    }
    let mut origins: Vec<FaceSplitOrigin> = (0..mesh.faces.len())
        .map(|face| FaceSplitOrigin {
            face,
            corners: None,
        })
        .collect();
    let mut pieces = BTreeMap::new();
    for (&f, triangles) in &splits {
        origins[f as usize].corners = Some(triangles[0].to_vec());
        let mut out = vec![f];
        for t in &triangles[1..] {
            out.push(u32::try_from(origins.len()).map_err(|_| "Mesh face budget exceeded")?);
            origins.push(FaceSplitOrigin {
                face: f as usize,
                corners: Some(t.to_vec()),
            });
        }
        pieces.insert(f, out);
    }
    if origins.len() > 5_000_000 {
        return Err("Triple would exceed the mesh face budget; nothing changed".into());
    }
    let out = remap_mesh(mesh, &origins, &pieces)?;
    out.validate()
        .map_err(|e| format!("Triple result failed validation ({e}); nothing changed"))?;
    Ok(Some(TripleOutcome {
        mesh: out,
        origins,
        pieces,
    }))
}

impl Editor {
    /// Read-only TRI-001 eligibility: idle editor, visible active mesh,
    /// Polygon mode and a nonempty explicit polygon selection (never implicit
    /// all). Returns the active item and selection order.
    pub fn triple_target(&self) -> Result<(u64, Vec<u32>)> {
        if self.i2_active() {
            return Err("Finish or Cancel the Extrude stage before Triple".into());
        }
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before Triple".into());
        }
        if self.scene.selection.mode != Mode::Polygon {
            return Err("Triple requires Polygon selection mode".into());
        }
        let item = self.scene.active().ok_or("Select a mesh before Triple")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before Triple".into());
        }
        if self.scene.selection.faces.is_empty() {
            return Err("Select polygons before Triple".into());
        }
        Ok((item.id, self.scene.selection.faces.clone()))
    }
    /// One atomic edit. Untouched faces keep their indices and identities;
    /// the selection becomes the resulting triangles in source selection order.
    pub(crate) fn triple(&mut self) -> Result<()> {
        let (item_id, selected) = self.triple_target()?;
        let source: Mesh = (*self.scene.active().ok_or("Select a mesh")?.mesh).clone();
        let Some(outcome) = triple_faces(&source, &selected)? else {
            return Ok(());
        };
        let ids = self
            .identities
            .current
            .meshes
            .get(&item_id)
            .ok_or("Missing mesh identity owner")?
            .clone();
        let split = self
            .identities
            .with_ids(|next| ids.split_faces(&source, &outcome.mesh, &outcome.origins, next))?;
        let faces = selected
            .iter()
            .flat_map(|f| outcome.pieces.get(f).cloned().unwrap_or_else(|| vec![*f]))
            .collect();
        let before = self.scene.clone();
        let identities_before = self.identities.current.clone();
        self.scene
            .active_mut()
            .ok_or("Select a mesh before Triple")?
            .mesh = outcome.mesh.into();
        // Vertex/edge selections stay valid: points and boundary edges persist.
        self.scene.selection.faces = faces;
        self.identities.current.meshes.insert(item_id, split.into());
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.observe(&self.scene))
        {
            self.scene = before;
            self.identities.current = identities_before;
            return Err(e);
        }
        self.record("Triple".into(), before);
        Ok(())
    }
}
