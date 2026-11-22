//! MP-T11 Polygon Reduction (RED-001..004). General meshes use the quadric
//! error path in `reduction_qem`; this module keeps the exact profile that
//! reduces one explicitly and
//! completely selected planar triangulated disk to a requested triangle count
//! without changing its covered region, boundary locus or orientation.
//!
//! Technique (Operandi's own, from general mesh-simplification practice):
//! repeated half-edge collapse of a point onto an existing neighbouring point.
//! No point moves, so the covered region stays the same exactly when every
//! reconnected triangle stays positively oriented in the plane chart and the
//! disk stays manifold. Interior points may collapse onto any neighbour. A
//! boundary point collapses only along a boundary edge, and only when it lies
//! collinear between its two current boundary neighbours, so collinear
//! boundary samples are simplified without changing the boundary locus and
//! corners are never removed. The link condition (the two endpoints share
//! exactly the neighbours of the collapsed edge's faces) keeps a manifold disk
//! without duplicate faces.
//!
//! Each step chooses the collapse whose new triangles have the best minimum
//! shape quality, then the shorter edge, then the lower point indices, so the
//! result is deterministic for identical input. An interior collapse removes
//! two triangles and a boundary collapse one; a step never overshoots the
//! requested count. When no valid collapse remains, the achieved count is
//! reported separately rather than presented as the requested one. No
//! particular triangulation is claimed and none reproduces a reference
//! product's output.
//!
//! Every result is checked afterwards by `validate_planar_disk` plus covered
//! area and boundary-locus postconditions which do not use the collapse state.
//! Budget: at most `REDUCTION_FACE_BUDGET` source triangles. Each step scans
//! every point and neighbour (O(F·d²) for point degree d), so a full reduction
//! is O(F²·d²); faster queues are deferred with general shapes.
use crate::{Face, Mesh, MeshAttributes, PolygonRepresentation, Result, TypedMap};
use glam::{DVec2, DVec3};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Source triangle budget for the basic quadratic collapse search.
pub const REDUCTION_FACE_BUDGET: usize = 4096;
/// Maximum plane distance of a point, as a fraction of the mesh extent.
pub const REDUCTION_PLANARITY_TOLERANCE: f64 = 1e-9;
/// Minimum doubled triangle area, as a fraction of the squared extent.
pub const REDUCTION_AREA_TOLERANCE: f64 = 1e-12;
/// Collinearity and boundary-locus distance, as a fraction of the extent.
pub const REDUCTION_LENGTH_TOLERANCE: f64 = 1e-9;
/// Relative tolerance for equal covered areas.
pub const REDUCTION_AREA_MATCH: f64 = 1e-9;
/// The only released Boundary/Material Border weight (percent).
pub const REDUCTION_BASIC_WEIGHT: f64 = 100.;

/// Polygon Reduce options (poly-reduce-2026-11-05). Boundary and Material
/// Border Weight scale how strongly open borders and material borders are
/// kept. There is no Legacy Mode (decided Q-RD-03); `legacy` is refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReductionParameters {
    /// Requested triangle count. 0 is the armed, uninitialized status only.
    pub number: u32,
    /// Percent.
    pub boundary_weight: f64,
    /// Percent.
    pub material_border_weight: f64,
    pub legacy: bool,
}
impl Default for ReductionParameters {
    fn default() -> Self {
        Self {
            number: 0,
            boundary_weight: REDUCTION_BASIC_WEIGHT,
            material_border_weight: REDUCTION_BASIC_WEIGHT,
            legacy: false,
        }
    }
}
impl ReductionParameters {
    pub fn validate_basic(&self) -> Result<()> {
        for w in [self.boundary_weight, self.material_border_weight] {
            if !w.is_finite() || !(0. ..=10_000.).contains(&w) {
                return Err("Boundary and Material Border Weight must be 0–10000 %".into());
            }
        }
        if self.legacy {
            return Err("Operandi's Polygon Reduce has no Legacy Mode".into());
        }
        Ok(())
    }
    /// RED-002 explicit Number entry. Refuses nonfinite, fractional,
    /// nonpositive and overflowing values; 0 is a status, never a request.
    pub fn number_from_value(value: f64) -> Result<u32> {
        if !value.is_finite() {
            return Err("Number must be a finite positive integer".into());
        }
        if value.fract() != 0. {
            return Err("Number must be a whole number of polygons".into());
        }
        if value <= 0. {
            return Err(
                "Number must be a positive integer; 0 is only the armed status and never deletes the mesh"
                    .into(),
            );
        }
        if value > u32::MAX as f64 {
            return Err("Number exceeds the supported integer range".into());
        }
        Ok(value as u32)
    }
}

/// Validated basic domain: one planar, consistently oriented, manifold
/// triangulated disk with a single simple boundary loop.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanarDisk {
    /// Unit normal about which every triangle is counter-clockwise.
    pub normal: DVec3,
    /// Covered area.
    pub area: f64,
    /// Counter-clockwise boundary cycle, starting at its lowest point index.
    pub boundary: Vec<u32>,
    /// Bounding-box diagonal; scales every tolerance.
    pub extent: f64,
}

/// Explicit source-to-result correspondence (RED-003), also consumed by
/// `MeshIdentity::reduced`. Output points keep source positions.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ReductionCorrespondence {
    /// Source point of each output point, strictly ascending.
    pub vertex_sources: Vec<u32>,
    /// `Some(f)`: the output face is source face `f` unchanged (same corner
    /// cycle). `None`: a reconnected face with new identity.
    pub face_sources: Vec<Option<u32>>,
    /// Source points with no output point, ascending.
    pub retired_vertices: Vec<u32>,
    /// Source faces with no unchanged output face, ascending.
    pub retired_faces: Vec<u32>,
}

/// A validated Reduction result. `achieved` may differ from `requested` only
/// when no further valid collapse exists; callers must present both.
#[derive(Clone, Debug, PartialEq)]
pub struct ReductionOutcome {
    pub mesh: Mesh,
    pub correspondence: ReductionCorrespondence,
    pub requested: u32,
    pub achieved: u32,
}

struct Chart {
    points: Vec<DVec2>,
    normal: DVec3,
    extent: f64,
}

/// Twice the signed area of triangle `abc` (positive when counter-clockwise).
fn cross2(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b - a).perp_dot(c - a)
}

fn point_segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared()).clamp(0., 1.);
    (a + d * t).distance(p)
}

fn point_segment_distance3(p: DVec3, a: DVec3, b: DVec3) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared()).clamp(0., 1.);
    (a + d * t).distance(p)
}

fn segment_distance(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> f64 {
    let (o1, o2) = (cross2(a, b, c), cross2(a, b, d));
    let (o3, o4) = (cross2(c, d, a), cross2(c, d, b));
    if o1 * o2 < 0. && o3 * o4 < 0. {
        return 0.;
    }
    point_segment_distance(a, c, d)
        .min(point_segment_distance(b, c, d))
        .min(point_segment_distance(c, a, b))
        .min(point_segment_distance(d, a, b))
}

/// Shape quality in (0, 1]: 1 for an equilateral triangle.
fn quality(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    let lengths = (b - a).length_squared() + (c - b).length_squared() + (a - c).length_squared();
    2. * 3f64.sqrt() * cross2(a, b, c) / lengths
}

/// `u` lies strictly between `p` and `n` on their segment, within `eps`.
fn collinear_between(p: DVec2, u: DVec2, n: DVec2, eps: f64) -> bool {
    let d = n - p;
    let length = d.length();
    length > eps
        && d.perp_dot(u - p).abs() / length <= eps
        && (u - p).dot(d) > 0.
        && (n - u).dot(d) > 0.
}

/// Plane chart of a triangle mesh whose faces were already checked to be
/// triangles over valid point indices.
fn chart(mesh: &Mesh) -> Result<Chart> {
    let points = &mesh.vertices;
    if points.is_empty() || points.iter().any(|p| !p.is_finite()) {
        return Err("non-finite or missing point positions".into());
    }
    let lo = points.iter().copied().fold(DVec3::INFINITY, DVec3::min);
    let hi = points.iter().copied().fold(DVec3::NEG_INFINITY, DVec3::max);
    let extent = (hi - lo).length();
    if !extent.is_finite() || extent <= 0. {
        return Err("all points coincide".into());
    }
    let mut sum = DVec3::ZERO;
    for f in &mesh.faces {
        let [a, b, c] = [0usize, 1, 2].map(|k| points[f.vertices[k] as usize]);
        sum += (b - a).cross(c - a);
    }
    let length = sum.length();
    if length.is_nan() || length <= 0. {
        return Err("zero-area or inconsistently oriented surface".into());
    }
    let normal = sum / length;
    let centroid = points.iter().copied().sum::<DVec3>() / points.len() as f64;
    if points
        .iter()
        .any(|&p| (p - centroid).dot(normal).abs() > REDUCTION_PLANARITY_TOLERANCE * extent)
    {
        return Err("materially nonplanar surface (curved surfaces are deferred)".into());
    }
    // (u, v, normal) is right-handed, so positive faces are CCW in (u, v).
    let seed = if normal.x.abs() < 0.8 {
        DVec3::X
    } else {
        DVec3::Y
    };
    let u = normal.cross(seed).normalize();
    let v = normal.cross(u);
    Ok(Chart {
        points: points
            .iter()
            .map(|&p| {
                let d = p - centroid;
                DVec2::new(d.dot(u), d.dot(v))
            })
            .collect(),
        normal,
        extent,
    })
}

/// Each point's incident faces must form one edge-connected fan.
fn require_single_fans(mesh: &Mesh) -> Result<()> {
    let mut incident: Vec<Vec<usize>> = vec![Vec::new(); mesh.vertices.len()];
    for (fi, f) in mesh.faces.iter().enumerate() {
        for &v in &f.vertices {
            incident[v as usize].push(fi);
        }
    }
    for (v, faces) in incident.iter().enumerate() {
        let mut reached = vec![false; faces.len()];
        let mut pending = vec![0usize];
        reached[0] = true;
        while let Some(i) = pending.pop() {
            for (j, &other) in faces.iter().enumerate() {
                let shares_edge = mesh.faces[faces[i]]
                    .vertices
                    .iter()
                    .any(|&w| w as usize != v && mesh.faces[other].vertices.contains(&w));
                if !reached[j] && shares_edge {
                    reached[j] = true;
                    pending.push(j);
                }
            }
        }
        if reached.iter().any(|r| !r) {
            return Err(format!(
                "point {v} joins separate fans (nonmanifold incidence)"
            ));
        }
    }
    Ok(())
}

/// RED-001/003 domain check, used for both the source and every result.
pub fn validate_planar_disk(mesh: &Mesh) -> Result<PlanarDisk> {
    if mesh.faces.is_empty() {
        return Err("no triangles".into());
    }
    if mesh.faces.len() > REDUCTION_FACE_BUDGET {
        return Err(format!(
            "more than {REDUCTION_FACE_BUDGET} triangles exceeds the basic Reduction budget"
        ));
    }
    for (fi, f) in mesh.faces.iter().enumerate() {
        if f.vertices.len() != 3 {
            return Err(format!(
                "polygon {fi} is not a triangle (basic Reduction needs a triangulated disk)"
            ));
        }
        if f.vertices
            .iter()
            .any(|&v| v as usize >= mesh.vertices.len())
        {
            return Err(format!("polygon {fi} names a missing point"));
        }
        let [a, b, c] = [f.vertices[0], f.vertices[1], f.vertices[2]];
        if a == b || b == c || c == a {
            return Err(format!("triangle {fi} repeats a corner"));
        }
    }
    let mut used = vec![false; mesh.vertices.len()];
    for f in &mesh.faces {
        for &v in &f.vertices {
            used[v as usize] = true;
        }
    }
    if used.iter().any(|u| !u) {
        return Err("isolated point (exceptional incidence)".into());
    }
    let chart = chart(mesh)?;
    let q = &chart.points;
    let min_area2 = 2. * REDUCTION_AREA_TOLERANCE * chart.extent * chart.extent;
    let mut area2 = 0.;
    for (fi, f) in mesh.faces.iter().enumerate() {
        let a2 = cross2(
            q[f.vertices[0] as usize],
            q[f.vertices[1] as usize],
            q[f.vertices[2] as usize],
        );
        if a2.is_nan() || a2 <= min_area2 {
            return Err(format!("triangle {fi} is inverted or has zero area"));
        }
        area2 += a2;
    }
    let mut directed = BTreeSet::new();
    for f in &mesh.faces {
        for k in 0..3 {
            if !directed.insert((f.vertices[k], f.vertices[(k + 1) % 3])) {
                return Err("nonmanifold edge or inconsistent winding".into());
            }
        }
    }
    let mut next = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for &(a, b) in &directed {
        if !directed.contains(&(b, a)) && (next.insert(a, b).is_some() || !incoming.insert(b)) {
            return Err("boundary touches itself at a point (nonmanifold incidence)".into());
        }
    }
    let boundary_edges = next.len();
    let Some(&start) = next.keys().next() else {
        return Err("closed surface; basic Reduction needs one open boundary".into());
    };
    let mut boundary = vec![start];
    let mut v = next[&start];
    while v != start {
        if boundary.len() >= boundary_edges {
            return Err("malformed boundary loop".into());
        }
        boundary.push(v);
        v = *next.get(&v).ok_or("open boundary chain")?;
    }
    if boundary.len() != boundary_edges {
        return Err("more than one boundary loop (holes are unsupported)".into());
    }
    let edges = (directed.len() + boundary_edges) / 2;
    let euler = mesh.vertices.len() as i64 - edges as i64 + mesh.faces.len() as i64;
    if euler != 1 {
        return Err("surface is not one connected disk".into());
    }
    require_single_fans(mesh)?;
    // A single simple boundary whose enclosed area equals the summed
    // positive triangle areas leaves no room for overlaps, folds or
    // T-junctions (each would cover some region twice).
    let eps = REDUCTION_LENGTH_TOLERANCE * chart.extent;
    let n = boundary.len();
    let corner = |i: usize| q[boundary[i % n] as usize];
    for i in 0..n {
        let (a, b) = (corner(i), corner(i + 1));
        for j in i + 1..n {
            let (c, d) = (corner(j), corner(j + 1));
            let touching = if j == i + 1 {
                point_segment_distance(a, c, d) <= eps || point_segment_distance(d, a, b) <= eps
            } else if i == 0 && j == n - 1 {
                point_segment_distance(b, c, d) <= eps || point_segment_distance(c, a, b) <= eps
            } else {
                segment_distance(a, b, c, d) <= eps
            };
            if touching {
                return Err("self-intersecting or folded boundary".into());
            }
        }
    }
    let enclosed2: f64 = (0..n).map(|i| corner(i).perp_dot(corner(i + 1))).sum();
    if enclosed2.is_nan()
        || enclosed2 <= 0.
        || (enclosed2 - area2).abs() > REDUCTION_AREA_MATCH * enclosed2
    {
        return Err("triangles overlap or leave cracks inside the boundary".into());
    }
    Ok(PlanarDisk {
        normal: chart.normal,
        area: area2 / 2.,
        boundary,
        extent: chart.extent,
    })
}

fn refuse<T>(what: &str) -> Result<T> {
    Err(format!(
        "Polygon Reduction cannot preserve {what} losslessly; nothing changed"
    ))
}

fn all_absent<T>(rows: &[Vec<Option<T>>]) -> bool {
    rows.iter().flatten().all(Option::is_none)
}

fn absent_maps<T: Clone>(
    maps: &BTreeMap<String, Vec<Vec<Option<T>>>>,
    count: usize,
) -> BTreeMap<String, Vec<Vec<Option<T>>>> {
    maps.keys()
        .map(|name| (name.clone(), vec![vec![None; 3]; count]))
        .collect()
}

/// The authored face values shared by every source face (RED-003). Returned
/// with empty corners as the template of every output triangle.
fn common_face(mesh: &Mesh) -> Result<Face> {
    let first = mesh
        .faces
        .first()
        .ok_or("Polygon Reduction needs triangles")?;
    for face in &mesh.faces {
        let Face {
            vertices: _,
            uv,
            material,
            smooth,
            part,
            material_label,
        } = face;
        if uv.is_some() {
            return refuse("assigned corner UVs");
        }
        if *material != first.material
            || *smooth != first.smooth
            || *part != first.part
            || *material_label != first.material_label
        {
            return refuse("conflicting face material, part or smoothing values");
        }
    }
    Ok(Face {
        vertices: Vec::new(),
        uv: None,
        material: first.material.clone(),
        smooth: first.smooth,
        part: first.part.clone(),
        material_label: first.material_label.clone(),
    })
}

/// RED-003 data policy. The destructuring is deliberately exhaustive: a new
/// attribute class fails to compile here until its Reduction mapping is
/// decided. Only common, all-or-none and absent data can survive a change
/// of points and connectivity; anything else refuses before mutation.
fn require_lossless_data(mesh: &Mesh) -> Result<()> {
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
        set_groups: _,
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
        normal_map_order: _,
        corner_normals,
        point_weights,
        point_colors,
        degenerate_faces,
        solid_sketch: _,
        current_uv_map: _,
    } = &**attributes;
    common_face(mesh)?;
    if !uv_maps.values().all(|rows| all_absent(rows)) {
        return refuse("assigned UV map values or seams");
    }
    if typed_maps
        .values()
        .any(|m| !m.points.is_empty() || !all_absent(&m.corners))
    {
        return refuse("assigned typed map values");
    }
    if vertex_sets
        .values()
        .any(|s| !s.is_empty() && s.len() != vertices.len())
    {
        return refuse("partial vertex set membership");
    }
    if polygon_sets
        .values()
        .any(|s| !s.is_empty() && s.len() != faces.len())
    {
        return refuse("partial or conflicting polygon set membership");
    }
    // Reduction rebuilds every edge, so a saved edge set cannot follow it.
    if edge_sets.values().any(|s| !s.is_empty()) {
        return refuse("edge set membership");
    }
    if point_uvs.values().any(|m| !m.is_empty()) {
        return refuse("continuous UV values");
    }
    if !corner_weights.values().all(|rows| all_absent(rows))
        || !corner_colors.values().all(|rows| all_absent(rows))
    {
        return refuse("corner weight or color values");
    }
    if !extrude_edge_uses.is_empty() {
        return refuse("exceptional edge incidence");
    }
    if relative_morphs.values().any(|m| !m.is_empty()) {
        return refuse("relative morph deltas");
    }
    if smoothing_labels.windows(2).any(|w| w[0] != w[1]) {
        return refuse("conflicting smoothing labels");
    }
    if representations.windows(2).any(|w| w[0] != w[1])
        || representations
            .iter()
            .any(|r| matches!(r, PolygonRepresentation::Polyline { .. }))
    {
        return refuse("mixed polygon types or Polyline elements");
    }
    if !creases.is_empty() {
        return refuse("edge creases");
    }
    if !uv_records.values().all(|rows| all_absent(rows)) {
        return refuse("UV record assignments");
    }
    if !corner_normals.values().all(|rows| all_absent(rows)) {
        return refuse("authored corner normals");
    }
    if point_weights.values().any(|m| !m.is_empty()) || point_colors.values().any(|m| !m.is_empty())
    {
        return refuse("point weight or color values");
    }
    if !degenerate_faces.is_empty() {
        return refuse("degenerate cage polygons");
    }
    Ok(())
}

/// Build the output mesh: common face values on every triangle, absent rows
/// for every sparse per-corner map, all-or-none sets kept all-or-none, empty
/// definitions retained. Exhaustive for the same reason as the policy above.
fn rebuild(mesh: &Mesh, vertex_sources: &[u32], triangles: &[[u32; 3]]) -> Result<Mesh> {
    require_lossless_data(mesh)?;
    let common = common_face(mesh)?;
    let Mesh {
        attributes,
        vertices,
        faces: _,
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
        extrude_edge_uses: _,
        relative_morphs,
        smoothing_labels,
        representations,
        creases: _,
        uv_records,
        normal_map_order,
        corner_normals,
        point_weights,
        point_colors,
        degenerate_faces: _,
        solid_sketch,
        current_uv_map,
    } = &**attributes;
    let count = triangles.len();
    let point_count = u32::try_from(vertex_sources.len())
        .map_err(|_| String::from("Mesh point budget exceeded"))?;
    let face_count = u32::try_from(count).map_err(|_| String::from("Mesh face budget exceeded"))?;
    Ok(Mesh {
        attributes: (MeshAttributes {
            solid_sketch: solid_sketch.clone(),
            current_uv_map: current_uv_map.clone(),
            typed_maps: typed_maps
                .iter()
                .map(|(name, m)| {
                    (
                        name.clone(),
                        TypedMap {
                            kind: m.kind.clone(),
                            dimension: m.dimension,
                            zero_default: m.zero_default,
                            points: BTreeMap::new(),
                            corners: if m.corners.is_empty() {
                                Vec::new()
                            } else {
                                vec![vec![None; 3]; count]
                            },
                        },
                    )
                })
                .collect(),
            vertex_sets: vertex_sets
                .iter()
                .map(|(name, s)| {
                    let members = if s.is_empty() {
                        BTreeSet::new()
                    } else {
                        (0..point_count).collect()
                    };
                    (name.clone(), members)
                })
                .collect(),
            // Only empty edge sets reach here (see the refusal above).
            edge_sets: edge_sets.clone(),
            set_groups: set_groups.clone(),
            polygon_sets: polygon_sets
                .iter()
                .map(|(name, s)| {
                    let members = if s.is_empty() {
                        BTreeSet::new()
                    } else {
                        (0..face_count).collect()
                    };
                    (name.clone(), members)
                })
                .collect(),
            point_uvs: point_uvs.clone(),
            corner_weights: absent_maps(corner_weights, count),
            corner_colors: absent_maps(corner_colors, count),
            extrude_edge_uses: BTreeMap::new(),
            relative_morphs: relative_morphs.clone(),
            smoothing_labels: smoothing_labels
                .first()
                .map(|label| vec![label.clone(); count])
                .unwrap_or_default(),
            representations: representations
                .first()
                .map(|&r| vec![r; count])
                .unwrap_or_default(),
            creases: Vec::new(),
            uv_records: absent_maps(uv_records, count),
            normal_map_order: normal_map_order.clone(),
            corner_normals: absent_maps(corner_normals, count),
            point_weights: point_weights.clone(),
            point_colors: point_colors.clone(),
            degenerate_faces: BTreeSet::new(),
        })
        .into(),
        vertices: vertex_sources
            .iter()
            .map(|&v| vertices[v as usize])
            .collect(),
        faces: triangles
            .iter()
            .map(|t| Face {
                vertices: t.to_vec(),
                ..common.clone()
            })
            .collect(),
        uv_maps: absent_maps(uv_maps, count).into(),
    })
}

impl ReductionCorrespondence {
    /// Exact description of `source` → `output`; refuses guessed maps.
    pub fn validate(&self, source: &Mesh, output: &Mesh) -> Result<()> {
        let bad = || Err("Reduction correspondence does not describe this edit".to_string());
        if self.vertex_sources.len() != output.vertices.len()
            || self.face_sources.len() != output.faces.len()
            || self.vertex_sources.windows(2).any(|w| w[0] >= w[1])
            || self
                .vertex_sources
                .iter()
                .any(|&v| v as usize >= source.vertices.len())
            || self
                .vertex_sources
                .iter()
                .zip(&output.vertices)
                .any(|(&v, p)| source.vertices[v as usize] != *p)
        {
            return bad();
        }
        let kept: BTreeSet<u32> = self.vertex_sources.iter().copied().collect();
        let retired: Vec<u32> = (0..source.vertices.len() as u32)
            .filter(|v| !kept.contains(v))
            .collect();
        if retired != self.retired_vertices {
            return bad();
        }
        let mut retained = BTreeSet::new();
        for (face, origin) in output.faces.iter().zip(&self.face_sources) {
            let Some(f) = origin else {
                continue;
            };
            let Some(src) = source.faces.get(*f as usize) else {
                return bad();
            };
            let mapped: Option<Vec<u32>> = face
                .vertices
                .iter()
                .map(|&v| self.vertex_sources.get(v as usize).copied())
                .collect();
            if mapped.as_ref() != Some(&src.vertices) || !retained.insert(*f) {
                return bad();
            }
        }
        let retired_faces: Vec<u32> = (0..source.faces.len() as u32)
            .filter(|f| !retained.contains(f))
            .collect();
        if retired_faces != self.retired_faces {
            return bad();
        }
        Ok(())
    }
}

/// Explicit complete selection (RED-001): every polygon of the mesh, no
/// duplicates. Empty is never all; partial regions are deferred.
fn require_complete_selection(mesh: &Mesh, selected: &[u32]) -> Result<()> {
    if selected.is_empty() {
        return Err("Select polygons for Polygon Reduction; an empty selection is not all".into());
    }
    let set: BTreeSet<u32> = selected.iter().copied().collect();
    if set.len() != selected.len() {
        return Err("Duplicate polygon selection".into());
    }
    if set.len() != mesh.faces.len() || set.iter().any(|&f| f as usize >= mesh.faces.len()) {
        return Err("Basic Polygon Reduction needs every polygon of the mesh explicitly selected; partial regions are deferred".into());
    }
    Ok(())
}

/// Whether the exact planar-disk profile below covers this input; anything
/// else uses the general quadric-error path in `reduction_qem`.
fn planar_disk_eligible(mesh: &Mesh, selected: &[u32]) -> bool {
    require_complete_selection(mesh, selected).is_ok()
        && require_lossless_data(mesh).is_ok()
        && validate_planar_disk(mesh).is_ok()
}

/// Initialization check: the current count of an eligible source.
pub fn reduction_source_count(mesh: &Mesh, selected: &[u32]) -> Result<u32> {
    if !planar_disk_eligible(mesh, selected) {
        return crate::reduction_qem::general_reduction_count(mesh, selected);
    }
    planar_disk_source_count(mesh, selected)
}

fn planar_disk_source_count(mesh: &Mesh, selected: &[u32]) -> Result<u32> {
    require_complete_selection(mesh, selected)?;
    require_lossless_data(mesh)?;
    validate_planar_disk(mesh).map_err(|e| {
        format!("Polygon Reduction supports one planar triangulated disk: {e}; nothing changed")
    })?;
    u32::try_from(mesh.faces.len()).map_err(|_| String::from("Mesh face budget exceeded"))
}

struct Collapse {
    quality: f64,
    length2: f64,
    from: u32,
    to: u32,
    removed: Vec<usize>,
}
impl Collapse {
    fn better_than(&self, other: &Self) -> bool {
        other
            .quality
            .total_cmp(&self.quality)
            .then(self.length2.total_cmp(&other.length2))
            .then((self.from, self.to).cmp(&(other.from, other.to)))
            == Ordering::Less
    }
}

fn internal(what: &str) -> String {
    format!("Polygon Reduction internal check failed ({what}); nothing changed")
}

/// Postconditions independent of the collapse state.
fn verify_result(
    source: &Mesh,
    disk: &PlanarDisk,
    out: &Mesh,
    plan: &ReductionCorrespondence,
) -> Result<()> {
    out.validate().map_err(|e| internal(&e))?;
    plan.validate(source, out).map_err(|e| internal(&e))?;
    let result = validate_planar_disk(out).map_err(|e| internal(&e))?;
    if result.normal.dot(disk.normal) <= 0. {
        return Err(internal("orientation"));
    }
    if (result.area - disk.area).abs() > REDUCTION_AREA_MATCH * disk.area {
        return Err(internal("covered area"));
    }
    let source_boundary: BTreeSet<u32> = disk.boundary.iter().copied().collect();
    let out_boundary: Vec<u32> = result
        .boundary
        .iter()
        .map(|&v| plan.vertex_sources[v as usize])
        .collect();
    if out_boundary.iter().any(|v| !source_boundary.contains(v)) {
        return Err(internal("boundary point added"));
    }
    let eps = REDUCTION_LENGTH_TOLERANCE * disk.extent;
    let position = |v: u32| source.vertices[v as usize];
    let n = out_boundary.len();
    for &s in &disk.boundary {
        let on_locus = (0..n).any(|i| {
            point_segment_distance3(
                position(s),
                position(out_boundary[i]),
                position(out_boundary[(i + 1) % n]),
            ) <= eps
        });
        if !on_locus {
            return Err(internal("boundary locus changed"));
        }
    }
    Ok(())
}

/// Reduce the explicitly, completely selected planar disk `mesh` toward
/// `requested` triangles. Errors leave nothing changed.
/// Polygon Reduce with its weights (poly-reduce-2026-11-05): the exact
/// planar-disk profile when it applies at the default weights, otherwise the
/// general quadric reduction.
pub fn reduce_polygons(
    mesh: &Mesh,
    selected: &[u32],
    requested: u32,
    p: &ReductionParameters,
) -> Result<ReductionOutcome> {
    p.validate_basic()?;
    let default_weights = p.boundary_weight == REDUCTION_BASIC_WEIGHT
        && p.material_border_weight == REDUCTION_BASIC_WEIGHT;
    if default_weights {
        return reduce_planar_disk(mesh, selected, requested);
    }
    let source_count = reduction_source_count(mesh, selected)?;
    if requested == 0 || requested > source_count {
        return Err(format!(
            "Number must be 1–{source_count} to reduce; other values change nothing"
        ));
    }
    crate::reduction_qem::reduce_general(
        mesh,
        selected,
        requested,
        p.boundary_weight / 100.,
        p.material_border_weight / 100.,
    )
}

pub fn reduce_planar_disk(
    mesh: &Mesh,
    selected: &[u32],
    requested: u32,
) -> Result<ReductionOutcome> {
    let source_count = reduction_source_count(mesh, selected)?;
    if requested == 0 {
        return Err(
            "Number must be a positive integer; 0 is only the armed status and never deletes the mesh"
                .into(),
        );
    }
    if requested > source_count {
        return Err(format!(
            "Number {requested} exceeds the current {source_count} polygons; Reduction cannot add detail"
        ));
    }
    if !planar_disk_eligible(mesh, selected) {
        return crate::reduction_qem::reduce_general(mesh, selected, requested, 1., 1.);
    }
    if requested == 0 {
        return Err(
            "Number must be a positive integer; 0 is only the armed status and never deletes the mesh"
                .into(),
        );
    }
    if requested > source_count {
        return Err(format!(
            "Number {requested} exceeds the current {source_count} polygons; Reduction cannot add detail"
        ));
    }
    let disk = validate_planar_disk(mesh)?;
    let chart = chart(mesh)?;
    let q = &chart.points;
    let n = mesh.vertices.len();
    let min_area2 = 2. * REDUCTION_AREA_TOLERANCE * chart.extent * chart.extent;
    let eps = REDUCTION_LENGTH_TOLERANCE * chart.extent;
    let mut faces: Vec<Option<[u32; 3]>> = mesh
        .faces
        .iter()
        .map(|f| Some([f.vertices[0], f.vertices[1], f.vertices[2]]))
        .collect();
    let mut touched = vec![false; faces.len()];
    let mut alive = vec![true; n];
    let mut on_boundary = vec![false; n];
    let mut previous = vec![u32::MAX; n];
    let mut following = vec![u32::MAX; n];
    let nb = disk.boundary.len();
    for (i, &v) in disk.boundary.iter().enumerate() {
        on_boundary[v as usize] = true;
        following[v as usize] = disk.boundary[(i + 1) % nb];
        previous[v as usize] = disk.boundary[(i + nb - 1) % nb];
    }
    let target = requested as usize;
    let mut count = faces.len();
    while count > target {
        let deficit = count - target;
        let mut incident: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (fi, f) in faces.iter().enumerate() {
            if let Some(t) = f {
                for &v in t {
                    incident[v as usize].push(fi);
                }
            }
        }
        let neighbours: Vec<BTreeSet<u32>> = (0..n)
            .map(|v| {
                incident[v]
                    .iter()
                    .filter_map(|&fi| faces[fi])
                    .flatten()
                    .filter(|&w| w as usize != v)
                    .collect()
            })
            .collect();
        let mut best: Option<Collapse> = None;
        for u in 0..n {
            if !alive[u] {
                continue;
            }
            if on_boundary[u]
                && !collinear_between(q[previous[u] as usize], q[u], q[following[u] as usize], eps)
            {
                continue;
            }
            for &v in &neighbours[u] {
                let vi = v as usize;
                if on_boundary[u] && v != previous[u] && v != following[u] {
                    continue;
                }
                let removed: Vec<usize> = incident[u]
                    .iter()
                    .copied()
                    .filter(|&fi| faces[fi].is_some_and(|t| t.contains(&v)))
                    .collect();
                // A boundary point moves along its one-face boundary edge;
                // every edge of an interior point has two faces.
                if removed.len() != if on_boundary[u] { 1 } else { 2 } || removed.len() > deficit {
                    continue;
                }
                if neighbours[u].intersection(&neighbours[vi]).count() != removed.len() {
                    continue;
                }
                let mut worst = f64::INFINITY;
                let mut valid = true;
                for &fi in &incident[u] {
                    if removed.contains(&fi) {
                        continue;
                    }
                    let Some(t) = faces[fi] else {
                        continue;
                    };
                    let t = t.map(|w| if w as usize == u { v } else { w });
                    let [a, b, c] = t.map(|w| q[w as usize]);
                    if cross2(a, b, c) <= min_area2 {
                        valid = false;
                        break;
                    }
                    worst = worst.min(quality(a, b, c));
                }
                if !valid || !worst.is_finite() {
                    continue;
                }
                let candidate = Collapse {
                    quality: worst,
                    length2: q[u].distance_squared(q[vi]),
                    from: u as u32,
                    to: v,
                    removed,
                };
                if best.as_ref().is_none_or(|b| candidate.better_than(b)) {
                    best = Some(candidate);
                }
            }
        }
        let Some(collapse) = best else {
            break;
        };
        let (u, v) = (collapse.from as usize, collapse.to);
        for &fi in &collapse.removed {
            faces[fi] = None;
            count -= 1;
        }
        for &fi in &incident[u] {
            if let Some(t) = faces[fi].as_mut() {
                for w in t.iter_mut() {
                    if *w as usize == u {
                        *w = v;
                    }
                }
                touched[fi] = true;
            }
        }
        alive[u] = false;
        if on_boundary[u] {
            let (p, f) = (previous[u], following[u]);
            following[p as usize] = f;
            previous[f as usize] = p;
            on_boundary[u] = false;
        }
    }
    let vertex_sources: Vec<u32> = (0..n).filter(|&v| alive[v]).map(|v| v as u32).collect();
    let mut remap = vec![u32::MAX; n];
    for (i, &v) in vertex_sources.iter().enumerate() {
        remap[v as usize] = i as u32;
    }
    let mut triangles = Vec::with_capacity(count);
    let mut face_sources = Vec::with_capacity(count);
    for (fi, f) in faces.iter().enumerate() {
        if let Some(t) = f {
            triangles.push(t.map(|w| remap[w as usize]));
            face_sources.push((!touched[fi]).then_some(fi as u32));
        }
    }
    let retained: BTreeSet<u32> = face_sources.iter().flatten().copied().collect();
    let correspondence = ReductionCorrespondence {
        retired_vertices: (0..n).filter(|&v| !alive[v]).map(|v| v as u32).collect(),
        retired_faces: (0..mesh.faces.len() as u32)
            .filter(|f| !retained.contains(f))
            .collect(),
        vertex_sources,
        face_sources,
    };
    let out = rebuild(mesh, &correspondence.vertex_sources, &triangles)?;
    verify_result(mesh, &disk, &out, &correspondence)?;
    let achieved =
        u32::try_from(out.faces.len()).map_err(|_| String::from("Mesh face budget exceeded"))?;
    Ok(ReductionOutcome {
        mesh: out,
        correspondence,
        requested,
        achieved,
    })
}
