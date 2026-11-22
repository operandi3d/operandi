//! Polygon Bevel's intrinsic contour construction. Widget frames never enter
//! this module. Every candidate is built from an immutable stage source.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BevelEdges {
    #[default]
    Inner,
    Outer,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BevelFalloff {
    Vertex,
    #[default]
    Averaged,
    Polygon,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BevelUvs {
    #[default]
    Connected,
    U,
    V,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BevelOptions {
    pub shift_range: f64,
    pub inset_range: f64,
    pub edges: BevelEdges,
    pub use_material: bool,
    pub material: String,
    pub segments: u32,
    pub square: bool,
    pub edge_rail: bool,
    pub offset_even: bool,
    pub thicken: bool,
    pub max_angle: f64,
    pub use_normal_map: bool,
    pub open_boundary: bool,
    pub auto_weld: bool,
    pub weld_range: f64,
    pub falloff: BevelFalloff,
    pub falloff_shift: bool,
    pub falloff_inset: bool,
    pub uvs: BevelUvs,
    pub reverse_direction: bool,
    pub reverse_inset: bool,
    pub keep_aspect: bool,
}
impl Default for BevelOptions {
    fn default() -> Self {
        Self {
            shift_range: 0.,
            inset_range: 0.,
            edges: BevelEdges::Inner,
            use_material: false,
            material: String::new(),
            segments: 1,
            square: false,
            edge_rail: false,
            offset_even: true,
            thicken: false,
            max_angle: 89.9,
            use_normal_map: false,
            open_boundary: false,
            auto_weld: false,
            weld_range: 0.0001,
            falloff: BevelFalloff::Averaged,
            falloff_shift: true,
            falloff_inset: true,
            uvs: BevelUvs::Connected,
            reverse_direction: false,
            reverse_inset: false,
            keep_aspect: true,
        }
    }
}
#[derive(Clone, Debug)]
pub struct BevelResult {
    pub mesh: Mesh,
    pub selected: Vec<u32>,
}

fn unit(v: DVec3) -> Result<DVec3> {
    if !v.is_finite() || v.length_squared() < 1e-24 {
        Err("Bevel has an indeterminate local direction; source retained".into())
    } else {
        Ok(v.normalize())
    }
}
// Two independent stateless streams. A source face (or the whole grouped
// operation) has a stable identity throughout a preview; redraws consume no RNG.
fn variation(identity: u64, stream: u64) -> f64 {
    let mut x = identity
        .wrapping_add(stream)
        .wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    (x >> 11) as f64 * (2. / ((1u64 << 53) as f64)) - 1.
}
/// BV-R12: merge smooth normal neighborhoods before summing their responses.
/// Hard neighborhoods contribute separately, so an orthogonal crease offsets
/// by the entered distance in both incident planes.
fn normal_response(normals: &[DVec3], angular: &[DVec3], angle: f64) -> Result<DVec3> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let threshold = angle.max(0.).to_radians().cos();
    for index in 0..normals.len() {
        let mut merged = vec![index];
        while let Some(i) = groups.iter().position(|g| {
            g.iter().any(|&m| {
                merged
                    .iter()
                    .any(|&k| angular[m].dot(angular[k]) > threshold)
            })
        }) {
            merged.extend(groups.remove(i));
        }
        groups.push(merged);
    }
    let mut response = DVec3::ZERO;
    for g in groups {
        let sum: DVec3 = g.iter().map(|&i| normals[i]).sum();
        let n = unit(sum)?;
        let dots: Vec<_> = g
            .iter()
            .map(|&i| normals[i].dot(n))
            .filter(|&d| d > 0.)
            .collect();
        let divisor = dots.iter().sum::<f64>() / dots.len() as f64;
        if divisor.abs() < 1e-10 {
            return Err("Bevel normal neighborhood is singular".into());
        }
        response += n / divisor;
    }
    Ok(response)
}

/// At the exact zero-angle boundary, strict cosine classification is sensitive
/// to the admitted measurements' single-precision cage coordinates. Use that
/// finite precision only for the gate, keeping displacement, contour geometry
/// and authored coordinates in f64. Ordinary positive-angle geometry is unchanged.
/// Collapsed/unrepresentable quantized charts fall back to the original normal.
fn zero_angle_normal(source: &Mesh, face: usize, geometric: DVec3) -> DVec3 {
    let quantize = |v: DVec3| v.as_vec3().as_dvec3();
    let vertices = &source.faces[face].vertices;
    let origin = quantize(source.vertices[vertices[0] as usize]);
    let mut sum = DVec3::ZERO;
    for i in 0..vertices.len() {
        let a = quantize(source.vertices[vertices[i] as usize]) - origin;
        let b = quantize(source.vertices[vertices[(i + 1) % vertices.len()] as usize]) - origin;
        sum += a.cross(b);
    }
    if sum.is_finite() && sum.length_squared().is_finite() && sum.length_squared() > 1e-24 {
        sum.normalize()
    } else {
        geometric
    }
}

/// Spatial weights are evaluated at source points, never at a polygon centroid.
/// None bypasses falloff. This API does not admit dedicated polygon providers.
pub fn bevel_geometry(
    source: &Mesh,
    selected: &[u32],
    shift: f64,
    inset: f64,
    grouped: bool,
    p: &BevelOptions,
    weights: Option<&[f64]>,
) -> Result<BevelResult> {
    bevel_geometry_with_profile(source, selected, shift, inset, grouped, p, weights, None)
}

#[allow(clippy::too_many_arguments)]
pub fn bevel_geometry_with_profile(
    source: &Mesh,
    selected: &[u32],
    shift: f64,
    inset: f64,
    grouped: bool,
    p: &BevelOptions,
    weights: Option<&[f64]>,
    profile: Option<crate::BevelProfile>,
) -> Result<BevelResult> {
    bevel_draft(
        source, selected, shift, inset, grouped, p, weights, profile, false, None,
    )?
    .finish()
}

/// Bevel as applied from the panel: at zero Shift and Inset it still builds
/// the walls, flat on the original edges (PB-O-04), where a drag at zero
/// leaves the mesh as it was.
#[allow(clippy::too_many_arguments)]
pub fn bevel_geometry_applied(
    source: &Mesh,
    selected: &[u32],
    shift: f64,
    inset: f64,
    grouped: bool,
    p: &BevelOptions,
    weights: Option<&[f64]>,
    profile: Option<crate::BevelProfile>,
) -> Result<BevelResult> {
    bevel_draft(
        source, selected, shift, inset, grouped, p, weights, profile, true, None,
    )?
    .finish()
}

/// Twist Bevel's ring placement (twist_bevel.rs, spec 3.5 and section 8):
/// Bevel builds the rings, walls and attributes; the placer says where
/// each ring sits. With a placer, `bevel_draft` runs with Shift 0 and
/// Inset 1, so each point's inset move is its unit (mitred) offset.
pub(crate) trait RingPlacer {
    /// Ring positions t for rings 1..=Segments, rising to 1.
    fn rings(&self) -> &[f64];
    /// Region `region` is left as it is (Shift and Inset both 0).
    fn skip(&self, region: usize) -> bool;
    /// Region `region`'s signed inset at ring `level` (1-based).
    fn inset(&self, region: usize, level: u32) -> f64;
    /// Where a point at `point` with unit inset offset `unit_inset` sits on
    /// ring `level` of region `region`.
    fn place(&self, region: usize, level: u32, point: DVec3, unit_inset: DVec3) -> DVec3;
    /// Where the cap's source normal `normal` points after placement.
    fn cap_normal(&self, region: usize, normal: DVec3) -> DVec3;
}

/// Bevel's ring builder with each ring placed by `placer` (Twist Bevel).
pub(crate) fn bevel_placed(
    source: &Mesh,
    selected: &[u32],
    grouped: bool,
    p: &BevelOptions,
    placer: &dyn RingPlacer,
) -> Result<BevelResult> {
    bevel_draft(
        source,
        selected,
        0.,
        1.,
        grouped,
        p,
        None,
        None,
        false,
        Some(placer),
    )?
    .finish()
}

struct BevelDraft {
    mesh: Mesh,
    selected: Vec<u32>,
    validation_error: Option<String>,
}
impl BevelDraft {
    fn finish(self) -> Result<BevelResult> {
        if let Some(error) = self.validation_error {
            return Err(error);
        }
        let mut mesh = self.mesh;
        if mesh.attributes.has_typed_data() {
            // Set members a weld took away drop out of their sets (N29).
            let (points, faces) = (mesh.vertices.len() as u32, mesh.faces.len() as u32);
            let edges = mesh.edges();
            let a = &mut mesh.attributes;
            for set in a.vertex_sets.values_mut() {
                set.retain(|&v| v < points);
            }
            for set in a.polygon_sets.values_mut() {
                set.retain(|&f| f < faces);
            }
            for set in a.edge_sets.values_mut() {
                set.retain(|&(x, y)| edges.contains(&(x.min(y), x.max(y))));
            }
        }
        mesh.validate()?;
        Ok(BevelResult {
            mesh,
            selected: self.selected,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn bevel_draft(
    source: &Mesh,
    selected: &[u32],
    shift: f64,
    inset: f64,
    grouped: bool,
    p: &BevelOptions,
    weights: Option<&[f64]>,
    profile: Option<crate::BevelProfile>,
    build_zero: bool,
    placer: Option<&dyn RingPlacer>,
) -> Result<BevelDraft> {
    source.require_surface_elements()?;
    let mut validation_error = None;
    if [
        shift,
        inset,
        p.shift_range,
        p.inset_range,
        p.max_angle,
        p.weld_range,
    ]
    .iter()
    .any(|x| !x.is_finite())
        || p.weld_range < 0.
        || p.segments == 0
        || p.segments > 4096
        || p.material.len() > 1_048_576
    {
        return Err(
            "Bevel requires finite distances/angle, nonnegative Weld Range and 1..4096 Segments (operation budget)".into(),
        );
    }
    let profile_samples = profile.map(|shape| shape.samples(p)).transpose()?;
    let samples = if let Some(placer) = placer {
        placer.rings().iter().map(|&t| DVec2::splat(t)).collect()
    } else {
        profile_samples.unwrap_or_else(|| {
            (1..=p.segments)
                .map(|k| DVec2::splat(k as f64 / p.segments as f64))
                .collect()
        })
    };
    let divisions = samples.len() as u32;
    if weights.is_some_and(|w| {
        w.len() != source.vertices.len() || w.iter().any(|v| !v.is_finite() || *v < 0. || *v > 1.)
    }) {
        return Err("Bevel spatial weights must be finite source-point weights in 0..1".into());
    }
    let regions = source.polygon_regions(selected, grouped)?;
    if p.auto_weld
        && regions
            .iter()
            .flat_map(|r| r.faces.iter())
            .map(|&f| source.faces[f].vertices.len())
            .sum::<usize>()
            .saturating_mul(divisions as usize)
            > 8_000_000
    {
        return Err("Bevel weld exceeds 8 million cap-validation corner budget".into());
    }
    source.require_disjoint_exception_component(selected)?;
    // Preserve the existing attribute boundary until a class has explicit
    // correspondence below. Never turn sparse absence into a dense zero map.
    // Named selection sets ride along (N29, Jason 2026-11-17): Bevel keeps
    // every source point and polygon at its index (new ones are added), so
    // set members still name the same elements; the new side polygons and
    // points join no set. Typed maps and morphs still refuse.
    if !source.attributes.typed_maps.is_empty() || !source.attributes.relative_morphs.is_empty() {
        // Spec: BV-Q04
        return Err("Bevel can't keep some of this mesh's map data; nothing changed".into());
    }
    let estimate: usize = regions
        .iter()
        .map(|r| r.boundary.len().saturating_mul(divisions as usize + 3))
        .sum();
    let attribute_maps = source.uv_maps.len()
        + source.attributes.point_uvs.len()
        + source.attributes.corner_weights.len()
        + source.attributes.corner_colors.len()
        + source.attributes.corner_normals.len()
        + source.attributes.point_weights.len()
        + source.attributes.point_colors.len()
        + 1;
    if estimate.saturating_mul(4).saturating_mul(attribute_maps) > 8_000_000 {
        return Err("Bevel exceeds 8,000,000 generated attribute-entry budget".into());
    }
    if estimate > 1_000_000 || source.vertices.len().saturating_add(estimate) > 5_000_000 {
        return Err("Bevel exceeds generated geometry budget".into());
    }
    let zero = shift == 0. && inset == 0. && p.shift_range == 0. && p.inset_range == 0.;
    if zero && !build_zero {
        return Ok(BevelDraft {
            mesh: source.clone(),
            selected: selected.to_vec(),
            validation_error: None,
        });
    }
    let incidence = source.edge_counts();
    let mut out = source.clone();
    let mut descendants = selected.to_vec();
    // Each output vertex/corner has an explicit source donor, even for bands.
    let mut levels = vec![0.; source.vertices.len()];
    let mut uv_offsets: BTreeMap<(String, usize, usize), DVec2> = BTreeMap::new();
    let mut origins: Vec<u32> = (0..source.vertices.len() as u32).collect();
    let mut donors: Vec<Vec<(usize, usize)>> = source
        .faces
        .iter()
        .enumerate()
        .map(|(f, x)| (0..x.vertices.len()).map(|c| (f, c)).collect())
        .collect();
    let mut bands = Vec::new();
    let mut band_corners = BTreeMap::new();
    let mut weld_edge_donors: BTreeMap<(u32, u32), BTreeSet<(u32, u32)>> = BTreeMap::new();
    let mut welded_uv_owners: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    let mut owners: Vec<usize> = (0..source.faces.len()).collect();
    let mut skin_vertices = BTreeMap::new();
    let mut collapsed_caps: BTreeSet<usize> = BTreeSet::new();
    if p.thicken {
        // Per point: its polygons, and whether it is on an open border.
        let mut point_faces = vec![Vec::new(); source.vertices.len()];
        for (f, face) in source.faces.iter().enumerate() {
            for &v in &face.vertices {
                point_faces[v as usize].push(f);
            }
        }
        let mut on_border = vec![false; source.vertices.len()];
        for (&(a, b), &count) in incidence.iter() {
            if count == 1 {
                on_border[a as usize] = true;
                on_border[b as usize] = true;
            }
        }
        for v in 0..source.vertices.len() as u32 {
            let fs = &point_faces[v as usize];
            if !fs.is_empty()
                && fs.iter().all(|&f| selected.contains(&(f as u32)))
                && !on_border[v as usize]
            {
                let id = out.vertices.len() as u32;
                out.vertices.push(source.vertices[v as usize]);
                origins.push(v);
                levels.push(0.);
                skin_vertices.insert(v, id);
            }
        }
    }
    let mut square_cache = BTreeMap::new();
    let total_boundary: usize = regions
        .iter()
        .filter(|r| {
            grouped || weights.is_none_or(|w| r.vertices.iter().all(|&v| w[v as usize] > 0.))
        })
        .map(|r| r.boundary.len())
        .sum();
    let mut boundary_offset = 0;
    // Which placed region each cap belongs to (the fold check below).
    let mut cap_regions: BTreeMap<usize, usize> = BTreeMap::new();
    for (region_index, region) in regions.into_iter().enumerate() {
        if let Some(placer) = placer {
            if placer.skip(region_index) {
                continue;
            }
            for &f in &region.faces {
                cap_regions.insert(f, region_index);
            }
        }
        let point_weights: BTreeMap<_, _> = region
            .vertices
            .iter()
            .map(|&v| (v, weights.map_or(1., |w| w[v as usize])))
            .collect();
        if !grouped && point_weights.values().any(|&w| w == 0.) {
            continue;
        }
        let mean_weight = point_weights.values().sum::<f64>() / point_weights.len() as f64;
        let identity = if grouped {
            0
        } else {
            region.faces[0] as u64 + 1
        };
        let amount_shift = shift + p.shift_range * variation(identity, 0x74a92315);
        let amount_inset = profile
            .filter(|_| p.keep_aspect)
            .map_or(inset, |shape| shift * shape.aspect())
            + p.inset_range * variation(identity, 0xa2e9587f);
        let sign = if p.edges == BevelEdges::Outer {
            -1.
        } else {
            1.
        };
        let incoming: BTreeMap<_, _> = region.boundary.iter().map(|e| (e.b, e)).collect();
        let outgoing: BTreeMap<_, _> = region.boundary.iter().map(|e| (e.a, e)).collect();
        let mut moves = BTreeMap::new();
        let mut corners = BTreeMap::new();
        let mut incoming_sides = BTreeMap::new();
        let mut open_vertices = BTreeSet::new();
        for e in &region.boundary {
            if p.open_boundary && incidence[&edge_key(e.a, e.b)] == 1 {
                open_vertices.insert(e.a);
                open_vertices.insert(e.b);
            }
        }
        for &v in &region.vertices {
            let mut incident: Vec<_> = region
                .faces
                .iter()
                .filter(|&&f| source.faces[f].vertices.contains(&v))
                .map(|&f| {
                    let n = source.normal(f);
                    (
                        n,
                        if p.max_angle <= 0. {
                            zero_angle_normal(source, f, n)
                        } else {
                            n
                        },
                    )
                })
                .collect();
            incident.sort_by(|a, b| {
                a.0.x
                    .total_cmp(&b.0.x)
                    .then(a.0.y.total_cmp(&b.0.y))
                    .then(a.0.z.total_cmp(&b.0.z))
            });
            let ns: Vec<_> = incident.iter().map(|x| x.0).collect();
            let angular: Vec<_> = incident.iter().map(|x| x.1).collect();
            let chart = unit(ns.iter().copied().sum())?;
            let mut direction = normal_response(&ns, &angular, p.max_angle)?;
            if p.use_normal_map {
                if source.attributes.corner_normals.len() > 1
                    && source.attributes.normal_map_order.is_empty()
                {
                    return Err(
                        // Spec: BV-Q04
                        "Bevel can't handle this normal map; nothing changed".into(),
                    );
                }
                let owner = source
                    .attributes
                    .normal_map_order
                    .first()
                    .and_then(|n| source.attributes.corner_normals.get(n))
                    .or_else(|| source.attributes.corner_normals.values().next());
                if let Some(map) = owner {
                    let supplied: Vec<_> = region
                        .faces
                        .iter()
                        .flat_map(|&f| {
                            source.faces[f]
                                .vertices
                                .iter()
                                .enumerate()
                                .filter(move |&(_, x)| *x == v)
                                .filter_map(move |(c, _)| {
                                    map[f][c].filter(|n| n.length_squared() > 1e-24)
                                })
                        })
                        .collect();
                    if let Some(&n) = supplied.first() {
                        if supplied.iter().any(|m| m.distance_squared(n) > 1e-16) {
                            // Spec: BV-Q04
                            return Err("Bevel: conflicting normal directions at a corner".into());
                        }
                        direction = unit(n)?;
                    }
                }
            }
            let mut offset = DVec3::ZERO;
            if let Some(e) = outgoing.get(&v) {
                let prev = incoming[&v].a;
                let point = source.vertices[v as usize];
                let before = point - source.vertices[prev as usize];
                let after = source.vertices[e.b as usize] - point;
                // Local contour frame retains the original cap depth. At smooth
                // boundary vertices the two projected edges share that frame.
                let project = |d: DVec3| d - chart * d.dot(chart);
                let a = if p.open_boundary && incidence[&edge_key(prev, v)] == 1 {
                    DVec3::ZERO
                } else {
                    chart.cross(unit(project(before))?)
                };
                let b = if p.open_boundary && incidence[&edge_key(v, e.b)] == 1 {
                    DVec3::ZERO
                } else {
                    chart.cross(unit(project(after))?)
                };
                incoming_sides.insert(v, if a.length_squared() > 1e-20 { a } else { b });
                let denominator = 1. + a.dot(b);
                if denominator < 1e-12 {
                    return Err("Bevel boundary folds back on itself".into());
                }
                offset = (a + b) / denominator;
                if !p.offset_even {
                    offset = unit(offset)?;
                }
                if p.edge_rail && grouped {
                    let mut rails: Vec<_> = region
                        .faces
                        .iter()
                        .flat_map(|&f| {
                            let vs = &source.faces[f].vertices;
                            vs.iter()
                                .enumerate()
                                .filter(move |&(_, x)| *x == v)
                                .flat_map(move |(c, _)| {
                                    [vs[(c + 1) % vs.len()], vs[(c + vs.len() - 1) % vs.len()]]
                                })
                        })
                        .filter(|&x| x != prev && x != e.b)
                        .map(|x| unit(source.vertices[x as usize] - point))
                        .collect::<Result<Vec<_>>>()?;
                    let target = unit(offset)?;
                    let tangent = unit(project(after))?;
                    rails.sort_by(|a, b| {
                        b.dot(target)
                            .total_cmp(&a.dot(target))
                            .then_with(|| b.dot(tangent).total_cmp(&a.dot(tangent)))
                    });
                    if let Some(&rail) = rails.first() {
                        offset = rail;
                    }
                }
                // Square detail belongs only to boundary corners incident to
                // one selected face. Smooth/shared junctions remain connected.
                if p.square && ns.len() == 1 && before.cross(after).dot(chart) > 1e-12 {
                    corners.insert(v, (unit(-before)?, unit(after)?));
                }
            }

            let w = if !grouped && p.falloff == BevelFalloff::Averaged {
                mean_weight
            } else {
                point_weights[&v]
            };
            moves.insert(
                v,
                (
                    direction * (sign * amount_shift * if p.falloff_shift { w } else { 1. }),
                    offset * (sign * amount_inset * if p.falloff_inset { w } else { 1. }),
                ),
            );
        }
        if (amount_inset != 0. || placer.is_some()) && !p.auto_weld {
            let chart = region
                .faces
                .iter()
                .map(|&f| source.normal(f))
                .sum::<DVec3>()
                .normalize_or_zero();
            if chart != DVec3::ZERO
                && region
                    .faces
                    .iter()
                    .all(|&f| source.normal(f).dot(chart) > 1e-6)
            {
                let origin =
                    source.vertices[*region.vertices.first().ok_or("Empty Bevel region")? as usize];
                let span = region
                    .vertices
                    .iter()
                    .map(|&v| (source.vertices[v as usize] - origin).length())
                    .fold(1., f64::max);
                // A profile may return to a safe endpoint after an intermediate
                // contour crosses itself. Validate every generated profile row.
                let rows = if profile.is_some() || placer.is_some() {
                    samples.as_slice()
                } else {
                    &samples[samples.len() - 1..]
                };
                for (row, sample) in rows.iter().enumerate() {
                    // A placed ring is checked flat, before it is moved
                    // along the spine: only its inset can fold it.
                    let placed = placer.map(|placer| {
                        placer.inset(region_index, (samples.len() - rows.len() + row + 1) as u32)
                    });
                    let cap: BTreeMap<_, _> = region
                        .vertices
                        .iter()
                        .map(|&v| {
                            let (s, i) = moves[&v];
                            let at = match placed {
                                Some(amount) => source.vertices[v as usize] + i * amount,
                                None => source.vertices[v as usize] + s * sample.y + i * sample.x,
                            };
                            (v, at)
                        })
                        .collect();
                    // PB-C-01/04: an Inset past the centre isn't clamped or
                    // refused; the cap may pass through itself. Only Twist
                    // Bevel's placed rings still refuse a fold.
                    if let Err(error) =
                        crate::polygon::validate_cap_edges(source, &region, &cap, chart, span)
                        && placer.is_some()
                    {
                        validation_error.get_or_insert(error);
                    }
                }
            }
        }
        // Connected UVs use each source island's axis-aligned extent ratio,
        // independently for every map. Missing records remain missing. Seams
        // retain face-corner ownership rather than being averaged at a point.
        for (name, map) in &source.uv_maps {
            let effective = |f: usize, c: usize| {
                map[f][c].or_else(|| {
                    source
                        .attributes
                        .point_uvs
                        .get(name)?
                        .get(&source.faces[f].vertices[c])
                        .copied()
                })
            };
            let values: Vec<_> = region
                .faces
                .iter()
                .flat_map(|&f| {
                    (0..source.faces[f].vertices.len()).filter_map(move |c| effective(f, c))
                })
                .collect();
            if values.len()
                != region
                    .faces
                    .iter()
                    .map(|&f| source.faces[f].vertices.len())
                    .sum::<usize>()
            {
                continue;
            }
            let lo = values
                .iter()
                .copied()
                .fold(DVec2::splat(f64::INFINITY), DVec2::min);
            let hi = values
                .iter()
                .copied()
                .fold(DVec2::splat(f64::NEG_INFINITY), DVec2::max);
            let xyz_lo = region
                .vertices
                .iter()
                .map(|&v| source.vertices[v as usize])
                .fold(DVec3::splat(f64::INFINITY), DVec3::min);
            let xyz_hi = region
                .vertices
                .iter()
                .map(|&v| source.vertices[v as usize])
                .fold(DVec3::splat(f64::NEG_INFINITY), DVec3::max);
            let ratio = (hi - lo).element_sum() / (xyz_hi - xyz_lo).element_sum();
            if ratio == 0. {
                continue;
            }
            for &f in &region.faces {
                for (c, &v) in source.faces[f].vertices.iter().enumerate() {
                    let value = effective(f, c).ok_or("Missing UV correspondence")?;
                    let mut perpendiculars = Vec::new();
                    for e in &region.boundary {
                        if e.a != v && e.b != v {
                            continue;
                        }
                        let cs = source.faces[e.face].vertices.len();
                        let d = (e.corner + 1) % cs;
                        let a = effective(e.face, e.corner).ok_or("Missing UV boundary")?;
                        let b = effective(e.face, d).ok_or("Missing UV boundary")?;
                        let at = if e.a == v { a } else { b };
                        if at.distance_squared(value) > 1e-16 {
                            continue;
                        }
                        let tangent = b - a;
                        if tangent.length_squared() < 1e-24 {
                            // Spec: BV-Q04. A zero-length UV edge gives no
                            // direction: this corner's UV stays put (N29,
                            // was a refusal).
                            continue;
                        }
                        let area = (0..cs)
                            .map(|k| {
                                effective(e.face, k)
                                    .unwrap()
                                    .perp_dot(effective(e.face, (k + 1) % cs).unwrap())
                            })
                            .sum::<f64>();
                        let n = DVec2::new(-tangent.y, tangent.x).normalize() * area.signum();
                        perpendiculars.push(n);
                    }
                    let offset = match perpendiculars.as_slice() {
                        [] => DVec2::ZERO,
                        [a] => *a,
                        [a, b] => {
                            let d = 1. + a.dot(*b);
                            // Spec: BV-Q04. Where the UV outline folds back
                            // on itself the corner's UV stays put instead of
                            // refusing the whole Bevel (N29, Jason
                            // 2026-11-17: mirrored UV layouts hit this).
                            if d < 1e-12 {
                                DVec2::ZERO
                            } else {
                                (*a + *b) / d
                            }
                        }
                        // Spec: BV-Q04. A branching outline: likewise kept.
                        _ => DVec2::ZERO,
                    };
                    uv_offsets.insert((name.clone(), f, c), offset * (sign * amount_inset * ratio));
                }
            }
        }
        let mut ordinals = BTreeMap::new();
        for start in &region.boundary {
            let mut v = start.a;
            while !ordinals.contains_key(&v) {
                ordinals.insert(v, ordinals.len());
                v = outgoing[&v].b;
            }
        }
        let region_offset = boundary_offset;
        boundary_offset += region.boundary.len();
        let boundary: BTreeSet<_> = outgoing.keys().copied().collect();
        let mut previous: BTreeMap<_, _> = region.vertices.iter().map(|&v| (v, v)).collect();
        // Outer keeps the selected cap fixed and displaces the source-side
        // contour; unselected neighbors keep their shared original identities.
        let mut square_points = BTreeMap::new();
        for (&v, &(a, b)) in &corners {
            let mut ids = Vec::new();
            for d in [a, b] {
                let pos = source.vertices[v as usize] + d * amount_inset.abs();
                let key = (v, pos.to_array().map(f64::to_bits));
                let id = *square_cache.entry(key).or_insert_with(|| {
                    let id = out.vertices.len() as u32;
                    out.vertices.push(pos);
                    origins.push(v);
                    levels.push(0.);
                    id
                });
                ids.push(id);
            }
            square_points.insert(v, (ids[0], ids[1]));
        }
        // Split the unselected neighbor's shared edge with the same source-side
        // corner points; otherwise Square Corner would leave a T junction.
        for e in &region.boundary {
            let a = square_points.get(&e.a).map(|x| x.1);
            let b = square_points.get(&e.b).map(|x| x.0);
            if a.is_none() && b.is_none() {
                continue;
            }
            for (fi, f) in source
                .faces
                .iter()
                .enumerate()
                .filter(|(fi, _)| !selected.contains(&(*fi as u32)))
            {
                if let Some(c) =
                    f.vertices.iter().enumerate().position(|(c, &v)| {
                        v == e.b && f.vertices[(c + 1) % f.vertices.len()] == e.a
                    })
                {
                    let at = out.faces[fi]
                        .vertices
                        .iter()
                        .position(|&v| v == e.b)
                        .ok_or("Bevel neighbor correspondence")?
                        + 1;
                    let mut ids = Vec::new();
                    let mut ds = Vec::new();
                    if let Some(b) = b {
                        ids.push(b);
                        ds.push((fi, c));
                    }
                    if let Some(a) = a {
                        ids.push(a);
                        ds.push((fi, (c + 1) % f.vertices.len()));
                    }
                    out.faces[fi].vertices.splice(at..at, ids);
                    donors[fi].splice(at..at, ds);
                }
            }
        }
        for level in 1..=divisions {
            let sample = samples[level as usize - 1];
            let t = sample.x;
            let final_level = level == divisions;
            let mut current = BTreeMap::new();
            let row_moves: BTreeMap<_, _> = moves
                .iter()
                .map(|(&v, &(s, i))| (v, (s * sample.y, i * sample.x)))
                .collect();
            let weld = if p.auto_weld {
                Some(crate::bevel_weld::plan(
                    source,
                    &region,
                    &row_moves,
                    &incoming_sides,
                    p.weld_range,
                    p.offset_even,
                    p.edge_rail && grouped,
                )?)
            } else {
                None
            };
            let mut row_ids = BTreeMap::new();
            let donor = |v: u32| -> (usize, usize) {
                region
                    .faces
                    .iter()
                    .find_map(|&f| {
                        source.faces[f]
                            .vertices
                            .iter()
                            .position(|&x| x == v)
                            .map(|c| (f, c))
                    })
                    .expect("Region vertices have source corners")
            };
            if let Some(weld) = &weld {
                crate::bevel_weld::validate_survivors(source, &region, &weld.root)?;
                let positions = region
                    .vertices
                    .iter()
                    .map(|&v| {
                        (
                            v,
                            source.vertices[v as usize] + row_moves[&v].0 + weld.inset[&v],
                        )
                    })
                    .collect();
                if let Err(error) =
                    crate::bevel_weld::validate_row(source, &region, &positions, &weld.root)
                {
                    validation_error.get_or_insert(format!("Auto Weld: {error}"));
                }
            }
            for &v in &region.vertices {
                if !final_level && !boundary.contains(&v) {
                    continue;
                }
                let root = weld.as_ref().map_or(v, |w| w.root[&v]);
                if let Some(&id) = row_ids.get(&root) {
                    current.insert(v, id);
                    continue;
                }
                let (s, i) = row_moves[&root];
                let effective_i = weld.as_ref().map_or(i, |w| w.inset[&root]);
                let position = match placer {
                    Some(placer) => placer.place(
                        region_index,
                        level,
                        source.vertices[root as usize],
                        moves[&root].1,
                    ),
                    None => source.vertices[root as usize] + s + effective_i,
                };
                let reuse = (!boundary.contains(&root) && final_level)
                    || (grouped
                        && p.open_boundary
                        && outgoing.get(&root).is_some_and(|e| {
                            incidence[&edge_key(root, e.b)] == 1
                                && incidence[&edge_key(incoming[&root].a, root)] == 1
                        }));
                let id = if reuse {
                    out.vertices[root as usize] = position;
                    root
                } else {
                    let id = out.vertices.len() as u32;
                    out.vertices.push(position);
                    origins.push(root);
                    levels.push(0.);
                    id
                };
                // Connected UVs move by the inset at this ring.
                levels[id as usize] = placer.map_or(t, |placer| placer.inset(region_index, level));
                if let Some(weld) = &weld {
                    let last = weld.uv_owner[&root];
                    if last != root {
                        welded_uv_owners.insert(id, donor(last));
                        if weld.metric_uv.contains(&root) && amount_inset != 0. {
                            levels[id as usize] =
                                weld.inset[&last].length() / amount_inset.abs() * sample.x.signum();
                        }
                    }
                }
                row_ids.insert(root, id);
                current.insert(v, id);
            }
            for e in &region.boundary {
                if p.open_boundary && incidence[&edge_key(e.a, e.b)] == 1 {
                    continue;
                }
                let f = &source.faces[e.face];
                let c = e.corner;
                let d = (c + 1) % f.vertices.len();
                let a = if level == 1 {
                    square_points.get(&e.a).map_or(previous[&e.a], |x| x.1)
                } else {
                    previous[&e.a]
                };
                let b = if level == 1 {
                    square_points.get(&e.b).map_or(previous[&e.b], |x| x.0)
                } else {
                    previous[&e.b]
                };
                let mut wall = f.clone();
                wall.vertices = if p.edges == BevelEdges::Outer {
                    vec![b, a, current[&e.a], current[&e.b]]
                } else {
                    vec![a, b, current[&e.b], current[&e.a]]
                };
                wall.uv = None;
                if p.use_material {
                    wall.material = p.material.clone();
                    wall.material_label = Some(p.material.clone());
                }
                bands.push((
                    out.faces.len(),
                    if grouped {
                        region_offset + ordinals[&e.a]
                    } else {
                        (region.boundary.len() * 2 - 2 - e.corner) % region.boundary.len()
                    },
                    if grouped {
                        total_boundary
                    } else {
                        region.boundary.len()
                    },
                    level,
                    ordinals[&e.b] <= ordinals[&e.a],
                ));
                let original_donors = if p.edges == BevelEdges::Outer {
                    vec![(e.face, d), (e.face, c), (e.face, c), (e.face, d)]
                } else {
                    vec![(e.face, c), (e.face, d), (e.face, d), (e.face, c)]
                };
                let mut wall_donors = original_donors;
                for (corner, wall_donor) in wall_donors.iter_mut().enumerate().take(4).skip(2) {
                    let id = wall.vertices[corner];
                    let original = source.faces[wall_donor.0].vertices[wall_donor.1];
                    if origins[id as usize] != original {
                        *wall_donor = donor(origins[id as usize]);
                    }
                }
                let keep = crate::bevel_weld::surviving_corners(&wall.vertices, false);
                if keep.len() < 3 {
                    bands.pop();
                    continue;
                }
                band_corners.insert(out.faces.len(), keep.clone());
                wall.vertices = keep.iter().map(|&c| wall.vertices[c]).collect();
                donors.push(keep.iter().map(|&c| wall_donors[c]).collect());
                out.faces.push(wall);
                owners.push(e.face);
            }
            if level == 1 {
                for (&v, &(a, b)) in &square_points {
                    let e = outgoing[&v];
                    let mut f = source.faces[e.face].clone();
                    f.vertices = vec![v, b, current[&v], a];
                    f.uv = None;
                    if p.use_material {
                        f.material = p.material.clone();
                        f.material_label = Some(p.material.clone());
                    }
                    out.faces.push(f);
                    donors.push(vec![(e.face, e.corner); 4]);
                    owners.push(e.face);
                }
            }
            if p.auto_weld {
                for &f in &region.faces {
                    let vs = &source.faces[f].vertices;
                    for c in 0..vs.len() {
                        let a = vs[c];
                        let b = vs[(c + 1) % vs.len()];
                        let (na, nb) = (current[&a], current[&b]);
                        if na != nb {
                            weld_edge_donors
                                .entry(edge_key(na, nb))
                                .or_default()
                                .insert(edge_key(a, b));
                        }
                    }
                }
            }
            if final_level && p.edges == BevelEdges::Inner {
                for &f in &region.faces {
                    let mapped: Vec<_> = source.faces[f]
                        .vertices
                        .iter()
                        .map(|v| current[v])
                        .collect();
                    let keep = if p.auto_weld {
                        crate::bevel_weld::surviving_corners(&mapped, true)
                    } else {
                        (0..mapped.len()).collect()
                    };
                    // PB-C-02/03: a cap whose corners all weld together is
                    // removed (with its selection) once the bevel is built.
                    if p.auto_weld && keep.len() < 3 {
                        collapsed_caps.insert(f);
                    }
                    out.faces[f].vertices = keep.iter().map(|&c| mapped[c]).collect();
                    donors[f] = keep
                        .iter()
                        .map(|&c| {
                            let original = source.faces[f].vertices[c];
                            let root = origins[mapped[c] as usize];
                            if root == original {
                                (f, c)
                            } else {
                                donor(root)
                            }
                        })
                        .collect();
                }
            }
            previous = current;
        }
        if p.thicken
            && region.vertices.iter().all(|v| {
                !source
                    .faces
                    .iter()
                    .enumerate()
                    .any(|(f, x)| !selected.contains(&(f as u32)) && x.vertices.contains(v))
            })
        {
            for &f in &region.faces {
                let mut skin = source.faces[f].clone();
                skin.vertices = skin
                    .vertices
                    .iter()
                    .rev()
                    .map(|v| skin_vertices.get(v).copied().unwrap_or(*v))
                    .collect();
                skin.uv = None;
                descendants.push(out.faces.len() as u32);
                out.faces.push(skin);
                donors.push(
                    (0..source.faces[f].vertices.len())
                        .rev()
                        .map(|c| (f, c))
                        .collect(),
                );
                owners.push(f);
            }
        }
    }
    let uv_donors: Vec<Vec<_>> = donors
        .iter()
        .enumerate()
        .map(|(f, ds)| {
            ds.iter()
                .enumerate()
                .map(|(c, &donor)| {
                    welded_uv_owners
                        .get(&out.faces[f].vertices[c])
                        .copied()
                        .unwrap_or(donor)
                })
                .collect()
        })
        .collect();
    let uv_origins: Vec<_> = origins
        .iter()
        .enumerate()
        .map(|(v, &origin)| {
            welded_uv_owners
                .get(&(v as u32))
                .map_or(origin, |&(f, c)| source.faces[f].vertices[c])
        })
        .collect();
    transfer(
        source,
        &mut out,
        &origins,
        &uv_origins,
        &donors,
        &uv_donors,
        &owners,
    )?;
    if p.uvs == BevelUvs::Connected {
        for (name, map) in &mut out.uv_maps {
            for (f, ds) in uv_donors.iter().enumerate() {
                for (c, &(sf, sc)) in ds.iter().enumerate() {
                    let id = out.faces[f].vertices[c];
                    let t = levels[id as usize];
                    if let Some(delta) = uv_offsets.get(&(name.clone(), sf, sc)) {
                        if let Some(value) = map[f][c].as_mut() {
                            *value += *delta * t;
                        } else if let Some(points) = out.attributes.point_uvs.get_mut(name)
                            && let Some(&original) = source
                                .attributes
                                .point_uvs
                                .get(name)
                                .and_then(|m| m.get(&uv_origins[id as usize]))
                        {
                            points.insert(id, original + *delta * t);
                        }
                    }
                }
            }
        }
    }
    if p.uvs != BevelUvs::Connected {
        if source.uv_maps.is_empty() {
            return Err("Select an existing UV map for Bevel U/V".into());
        }
        for (name, map) in &mut out.uv_maps {
            let continuous = source.attributes.point_uvs.get(name);
            let complete = source.faces.iter().enumerate().all(|(f, x)| {
                (0..x.vertices.len()).all(|c| {
                    source.uv_maps[name][f][c].is_some()
                        || continuous.is_some_and(|m| m.contains_key(&x.vertices[c]))
                })
            });
            let empty = source.uv_maps[name].iter().flatten().all(Option::is_none)
                && continuous.is_none_or(|m| m.is_empty());
            if !complete && !empty {
                return Err(format!(
                    // Spec: BV-Q04
                    "Bevel can't handle partly assigned UVs in {name}; nothing changed"
                ));
            }
            let strips = continuous.is_some_and(|m| !m.is_empty()) || empty;
            let coordinate = |fraction: f64, t: f64| {
                if p.uvs == BevelUvs::U {
                    DVec2::new(fraction - 1., t)
                } else {
                    DVec2::new(-t, fraction)
                }
            };
            for &(f, ordinal, count, level, wraps) in &bands {
                let a = ordinal as f64 / count as f64;
                let b = (ordinal + 1) as f64 / count as f64;
                let low = (level - 1) as f64 / divisions as f64;
                let high = level as f64 / divisions as f64;
                let values = [
                    coordinate(a, low),
                    coordinate(b, low),
                    coordinate(b, high),
                    coordinate(a, high),
                ];
                for (c, &original_corner) in band_corners[&f].iter().enumerate() {
                    let value = values[original_corner];
                    // Grouped corner records retain their inherited values except
                    // each contour's closing endpoint. Independent faces visit
                    // their bands in reverse corner order: generated endpoints
                    // receive strip coordinates, while a first source visit
                    // preserves its authored corner and subsequent visits write
                    // the current strip endpoint. Cap corners keep their records.
                    let write_corner = if grouped {
                        wraps && (original_corner == 1 || original_corner == 2)
                    } else {
                        original_corner >= 2
                            || (original_corner == 1 && ordinal > 0)
                            || (original_corner == 0 && ordinal + 1 == count)
                    };
                    if strips || write_corner {
                        map[f][c] = Some(value);
                    }
                }
                if empty {
                    for original_corner in [2, 3] {
                        let Some(c) = band_corners[&f].iter().position(|&c| c == original_corner)
                        else {
                            continue;
                        };
                        let vertex = out.faces[f].vertices[c];
                        for &cap in selected {
                            for (ci, &v) in out.faces[cap as usize].vertices.iter().enumerate() {
                                if v == vertex && map[cap as usize][ci].is_none() {
                                    map[cap as usize][ci] = Some(if original_corner == 2 {
                                        coordinate(b, 1.)
                                    } else {
                                        coordinate(a, 1.)
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for &f in selected {
        if collapsed_caps.contains(&(f as usize)) {
            continue;
        }
        // A placed cap turns with its ring (bend, twist about an axis).
        let expected = match (placer, cap_regions.get(&(f as usize))) {
            (Some(placer), Some(&region)) => placer.cap_normal(region, source.normal(f as usize)),
            (Some(_), None) => continue,
            (None, _) => source.normal(f as usize),
        };
        let checked = (|| -> Result<()> {
            let triangles = out.surface_triangles(f as usize).map_err(|e| {
                format!("Inset creates an invalid Bevel cap: {e}; reduce displacement")
            })?;
            for [a, b, c] in triangles {
                let face = &out.faces[f as usize];
                let [a, b, c] = [a, b, c].map(|c| out.vertices[face.vertices[c] as usize]);
                if (b - a).cross(c - a).dot(expected) <= 0. {
                    return Err("Inset folds/inverts a cap triangle; reduce its magnitude".into());
                }
            }
            if out.normal(f as usize).dot(expected) <= 0. {
                return Err("Inset inverts a cap; reduce its magnitude".into());
            }
            Ok(())
        })();
        match checked {
            Err(error) if placer.is_some() => {
                validation_error.get_or_insert(error);
            }
            // PB-C-01/04: a zero-size, inverted or self-crossing cap is kept
            // as asked; a zero-size one is an admitted degenerate face.
            Err(_) if out.normal(f as usize) == DVec3::ZERO => {
                out.attributes.degenerate_faces.insert(f);
            }
            _ => {}
        }
    }
    // The measured supplied-direction cases contain exactly zero-area bands
    // when the supplied normal lies along a boundary edge. Record those explicit
    // cage faces; other invalid bands still refuse atomically.
    for f in source.faces.len()..out.faces.len() {
        if weights.is_some()
            && (0..out.faces[f].vertices.len()).any(|c| {
                out.vertices[out.faces[f].vertices[c] as usize]
                    == out.vertices
                        [out.faces[f].vertices[(c + 1) % out.faces[f].vertices.len()] as usize]
            })
        {
            // FAL-T-01: a corner of weight 0 keeps its new point on the
            // original corner, and the band polygon there has no area; the
            // reference keeps it (counts don't change under a falloff).
            out.attributes.degenerate_faces.insert(f as u32);
            if out.normal(f) == DVec3::ZERO {
                continue;
            }
        }
        if ((p.use_normal_map && inset == 0.) || profile.is_some() || zero)
            && out.normal(f) == DVec3::ZERO
        {
            out.attributes.degenerate_faces.insert(f as u32);
            continue;
        }
        if out.surface_triangles(f).is_err() && placer.is_none() {
            // PB-C-01/04: the walls of an Inset past the centre cross
            // themselves; kept as asked (zero-size ones admitted).
            if out.normal(f) == DVec3::ZERO {
                out.attributes.degenerate_faces.insert(f as u32);
            }
        } else if let Err(error) = out.surface_triangles(f) {
            validation_error.get_or_insert(if placer.is_some() {
                // Twist Bevel: a wall quad turned too far for its height.
                format!(
                    "a wall twists or folds too far ({error}); add Segments or Shift, or lower Rotation"
                )
            } else {
                format!("Inset creates an invalid Bevel band: {error}; reduce displacement")
            });
        }
    }
    // New vertex ownership cannot reuse a source UV assignment identity.
    for (name, records) in &mut out.attributes.uv_records {
        let mut next = source.attributes.uv_records[name]
            .iter()
            .flatten()
            .flatten()
            .copied()
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("UV record identity overflow")?;
        let mut generated = BTreeMap::new();
        for (f, row) in records.iter_mut().enumerate() {
            for (c, id) in row.iter_mut().enumerate() {
                if let Some(old) = *id {
                    let (sf, sc) = uv_donors[f][c];
                    let v = out.faces[f].vertices[c];
                    let uv = out.uv_maps[name][f][c].ok_or("UV assignment without coordinate")?;
                    if v != source.faces[sf].vertices[sc]
                        || source.uv_maps[name][sf][sc] != Some(uv)
                    {
                        let key = (old, v, uv.to_array().map(f64::to_bits));
                        let new = if let Some(&id) = generated.get(&key) {
                            id
                        } else {
                            let id = next;
                            next = next.checked_add(1).ok_or("UV record identity overflow")?;
                            generated.insert(key, id);
                            id
                        };
                        *id = Some(new);
                    }
                }
            }
        }
    }
    let source_creases: BTreeMap<_, _> = source
        .attributes
        .creases
        .iter()
        .map(|c| ((c.a, c.b), c.weight))
        .collect();
    out.attributes.creases = out
        .edges()
        .into_iter()
        .filter_map(|(a, b)| {
            source_creases
                .get(&edge_key(origins[a as usize], origins[b as usize]))
                .map(|&weight| crate::MeshCrease { a, b, weight })
        })
        .collect();
    if p.auto_weld {
        let mut creases: BTreeMap<_, _> = out
            .attributes
            .creases
            .iter()
            .map(|c| (edge_key(c.a, c.b), c.weight))
            .collect();
        let actual_edges: BTreeSet<_> = out
            .edges()
            .into_iter()
            .map(|(a, b)| edge_key(a, b))
            .collect();
        for (target, donors) in weld_edge_donors {
            if !actual_edges.contains(&target) {
                continue;
            }
            let mut values = donors.iter().map(|e| source_creases.get(e).copied());
            let value = values.next().unwrap_or(None);
            if values.any(|v| v != value) {
                return Err(
                    // Spec: BV-Q04
                    "Bevel: conflicting creases at a merged edge; nothing changed".into(),
                );
            }
            creases.remove(&target);
            if let Some(weight) = value {
                creases.insert(target, weight);
            }
        }
        out.attributes.creases = creases
            .into_iter()
            .map(|((a, b), weight)| crate::MeshCrease { a, b, weight })
            .collect();
    }
    if !collapsed_caps.is_empty() {
        out.remove_faces(&collapsed_caps)?;
        let removed_before = |f: u32| collapsed_caps.range(..f as usize).count() as u32;
        descendants = descendants
            .into_iter()
            .filter(|&f| !collapsed_caps.contains(&(f as usize)))
            .map(|f| f - removed_before(f))
            .collect();
    }
    // Record only the exact incidences produced by this candidate, as the
    // existing format requires for independently thickened adjacent faces.
    let mut uses: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (fi, f) in out.faces.iter().enumerate() {
        for c in 0..f.vertices.len() {
            let a = f.vertices[c];
            let b = f.vertices[(c + 1) % f.vertices.len()];
            uses.entry(edge_key(a, b))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    for ((a, b), u) in uses {
        if u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]) {
            out.attributes
                .extrude_edge_uses
                .insert(format!("{a}:{b}"), u);
        }
    }
    // PB-G-01/03: the reference puts multi-face caps into the selected slots
    // in reverse order. Operandi keeps each cap in its own polygon's slot:
    // live identity and attribute correspondence rely on it (deliberate
    // departure, see SPECIFICATION_QUESTIONS.md, Polygon Bevel).
    Ok(BevelDraft {
        mesh: out,
        selected: descendants,
        validation_error,
    })
}

fn transfer(
    source: &Mesh,
    out: &mut Mesh,
    origins: &[u32],
    uv_origins: &[u32],
    donors: &[Vec<(usize, usize)>],
    uv_donors: &[Vec<(usize, usize)>],
    owners: &[usize],
) -> Result<()> {
    for (name, map) in &source.uv_maps {
        out.uv_maps.insert(
            name.clone(),
            uv_donors
                .iter()
                .map(|ds| ds.iter().map(|&(f, c)| map[f][c]).collect())
                .collect(),
        );
    }
    for (fi, ds) in uv_donors.iter().enumerate() {
        out.faces[fi].uv = if ds.iter().all(|&(f, _)| source.faces[f].uv.is_some()) {
            Some(
                ds.iter()
                    .map(|&(f, c)| source.faces[f].uv.as_ref().unwrap()[c])
                    .collect(),
            )
        } else {
            None
        };
    }
    let a = &source.attributes;
    let b = &mut out.attributes;
    if !a.representations.is_empty() {
        b.representations = owners.iter().map(|&f| a.representations[f]).collect();
    }
    if !a.smoothing_labels.is_empty() {
        b.smoothing_labels = owners
            .iter()
            .map(|&f| a.smoothing_labels[f].clone())
            .collect();
    }
    macro_rules! corners {
        ($field:ident) => {
            for (name, map) in &a.$field {
                b.$field.insert(
                    name.clone(),
                    donors
                        .iter()
                        .map(|ds| ds.iter().map(|&(f, c)| map[f][c]).collect())
                        .collect(),
                );
            }
        };
    }
    for (name, map) in &a.uv_records {
        b.uv_records.insert(
            name.clone(),
            uv_donors
                .iter()
                .map(|ds| ds.iter().map(|&(f, c)| map[f][c]).collect())
                .collect(),
        );
    }
    corners!(corner_normals);
    corners!(corner_weights);
    corners!(corner_colors);
    macro_rules! points {
        ($field:ident) => {
            for (name, map) in &a.$field {
                let mut m = map.clone();
                for (v, &src) in origins.iter().enumerate().skip(source.vertices.len()) {
                    if let Some(&value) = map.get(&src) {
                        m.insert(v as u32, value);
                    }
                }
                b.$field.insert(name.clone(), m);
            }
        };
    }
    for (name, map) in &a.point_uvs {
        let mut values = map.clone();
        for (v, &src) in uv_origins.iter().enumerate().skip(source.vertices.len()) {
            if let Some(&value) = map.get(&src) {
                values.insert(v as u32, value);
            }
        }
        b.point_uvs.insert(name.clone(), values);
    }
    points!(point_weights);
    points!(point_colors);
    Ok(())
}
