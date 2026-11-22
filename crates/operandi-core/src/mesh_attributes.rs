//! Sparse authored attributes. Missing entries remain absent (never assigned zero).
use crate::{Mesh, PolygonRepresentation, Result};
use glam::{DVec2, DVec3, DVec4};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshCrease {
    pub a: u32,
    pub b: u32,
    pub weight: f64,
}
/// Typed sparse authored records for map classes without legacy fixed-vector storage.
/// Defaults affect evaluation only; they never create authored records.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedMap {
    pub kind: String,
    pub dimension: u8,
    pub zero_default: bool,
    pub points: BTreeMap<u32, Vec<f64>>,
    pub corners: Vec<Vec<Option<Vec<f64>>>>,
}
impl TypedMap {
    pub fn effective(&self, mesh: &Mesh, f: usize, c: usize) -> Option<Vec<f64>> {
        self.corners
            .get(f)
            .and_then(|v| v.get(c).cloned().flatten())
            .or_else(|| {
                mesh.faces
                    .get(f)
                    .and_then(|face| face.vertices.get(c))
                    .and_then(|v| self.points.get(v))
                    .cloned()
            })
            .or_else(|| self.zero_default.then(|| vec![0.; self.dimension as usize]))
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshAttributes {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub typed_maps: BTreeMap<String, TypedMap>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub vertex_sets: BTreeMap<String, BTreeSet<u32>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub polygon_sets: BTreeMap<String, BTreeSet<u32>>,
    /// Named edge selection sets: each edge by its two point indices, lower
    /// first. Native version 13.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub edge_sets: BTreeMap<String, BTreeSet<(u32, u32)>>,
    /// Selection set groups: a named, ordered list of the sets it holds.
    /// Only names are stored, so topology edits never touch them. Native
    /// version 13.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set_groups: BTreeMap<String, Vec<crate::SetRef>>,

    /// EX-008..011: continuous UVs and effective corner overrides are distinct.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub point_uvs: BTreeMap<String, BTreeMap<u32, DVec2>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub corner_weights: BTreeMap<String, Vec<Vec<Option<f64>>>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub corner_colors: BTreeMap<String, Vec<Vec<Option<DVec4>>>>,
    /// Exact exceptional edge incidences produced by admitted Thick facing.
    /// This does not admit arbitrary nonmanifold input to modeling operations.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub extrude_edge_uses: BTreeMap<String, Vec<[u32; 3]>>,
    /// Authored relative morph deltas; absent records evaluate to zero. SA-007
    /// specifies zero new Spike deltas, not a universal attribute-average rule.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub relative_morphs: BTreeMap<String, BTreeMap<u32, DVec3>>,
    /// Optional per-face smoothing group labels. Empty is the legacy bool-only
    /// representation; this records identity without claiming evaluated shading.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub smoothing_labels: Vec<Option<String>>,

    /// Empty denotes legacy all-Face. Otherwise one representation per face.
    pub representations: Vec<PolygonRepresentation>,
    pub creases: Vec<MeshCrease>,
    /// Per-map face/corner assignment identities. None means ownership is unknown
    /// for legacy assigned coordinates, or unassigned when the UV slot is absent.
    /// Coordinates remain in Mesh::uv_maps. Never merge records by equal values.
    pub uv_records: BTreeMap<String, Vec<Vec<Option<u32>>>>,
    /// Explicit creation order for supplied normal maps (BV-R12). Legacy
    /// single-map documents need no metadata; multiple unordered maps refuse
    /// directional ownership rather than assuming lexical or active-map order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub normal_map_order: Vec<String>,
    pub corner_normals: BTreeMap<String, Vec<Vec<Option<DVec3>>>>,
    pub point_weights: BTreeMap<String, BTreeMap<u32, f64>>,
    pub point_colors: BTreeMap<String, BTreeMap<u32, DVec4>>,
    /// Explicit observed degenerate cage faces, not a global validation bypass.
    pub degenerate_faces: BTreeSet<u32>,
    /// Solid Sketch nodes, so the tool can pick the sketch up again
    /// (solid_sketch_store.rs, native version 45).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solid_sketch: Option<Box<crate::solid_sketch_store::StoredSketch>>,
    /// The selected (current) UV map, which every UV tool, the UV view and
    /// Unwrap act on (UVE-1 section 1, UVT-01; uv_maps.rs). `None`, or a
    /// name the mesh no longer has, falls back to `target_uv_map`'s choice.
    /// Native version 47.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_uv_map: Option<String>,
}
impl MeshAttributes {
    pub fn has_typed_data(&self) -> bool {
        !self.typed_maps.is_empty()
            || !self.vertex_sets.is_empty()
            || !self.polygon_sets.is_empty()
            || !self.edge_sets.is_empty()
    }

    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        if self.typed_maps.len() > 64
            || self.vertex_sets.len() > 1024
            || self.polygon_sets.len() > 1024
            || self.edge_sets.len() > 1024
            || self.set_groups.len() > 1024
        {
            return Err("Typed map/set budget exceeded".into());
        }
        for (name, m) in &self.typed_maps {
            let dimension = match m.kind.as_str() {
                "uv2" => 2,
                "weight1" => 1,
                "rgba4" => 4,
                "normal3" | "relative_morph3" | "rgb3" | "vector3" | "absolute_position3"
                | "pose_shape3" => 3,
                "tangent_basis6" | "position_derivative6" => 6,
                _ => return Err(format!("Unsupported typed map {name}: {}", m.kind)),
            };
            let valid = |v: &Vec<f64>| v.len() == dimension && v.iter().all(|x| x.is_finite());
            if name.len() > 1_048_576
                || m.dimension as usize != dimension
                || m.points
                    .iter()
                    .any(|(&v, x)| v as usize >= mesh.vertices.len() || !valid(x))
                || (!m.corners.is_empty()
                    && (m.corners.len() != mesh.faces.len()
                        || m.corners.iter().zip(&mesh.faces).any(|(cs, f)| {
                            cs.len() != f.vertices.len() || cs.iter().flatten().any(|v| !valid(v))
                        })))
                || (m.kind == "relative_morph3" && m.corners.iter().flatten().any(Option::is_some))
            {
                return Err(format!("Invalid typed map {name}"));
            }
        }
        for (name, set) in &self.vertex_sets {
            if name.len() > 1_048_576 || set.iter().any(|&v| v as usize >= mesh.vertices.len()) {
                return Err("Invalid vertex set".into());
            }
        }
        for (name, set) in &self.polygon_sets {
            if name.len() > 1_048_576 || set.iter().any(|&f| f as usize >= mesh.faces.len()) {
                return Err("Invalid polygon set".into());
            }
        }
        // Edge set members must name points that exist; an edge that a later
        // edit removed is dropped when that edit is recorded
        // (`selection_sets::prune_edge_sets`), so it is not an error here.
        for (name, set) in &self.edge_sets {
            if name.len() > 1_048_576
                || set
                    .iter()
                    .any(|&(a, b)| a >= b || b as usize >= mesh.vertices.len())
            {
                return Err("Invalid edge set".into());
            }
        }
        for (name, sets) in &self.set_groups {
            if name.len() > 1_048_576 || sets.len() > 4096 {
                return Err("Invalid selection set group".into());
            }
        }

        if self.point_uvs.len() > 64
            || self.corner_weights.len() > 64
            || self.corner_colors.len() > 64
            || self
                .point_uvs
                .keys()
                .chain(self.corner_weights.keys())
                .chain(self.corner_colors.keys())
                .any(|name| name.len() > 1_048_576)
        {
            return Err("Extrude map storage budget exceeded".into());
        }
        for (name, map) in &self.point_uvs {
            if !mesh.uv_maps.contains_key(name)
                || map
                    .iter()
                    .any(|(&v, p)| v as usize >= mesh.vertices.len() || !p.is_finite())
            {
                return Err("Invalid continuous UV map".into());
            }
        }
        for map in self.corner_weights.values() {
            if map.len() != mesh.faces.len()
                || map.iter().zip(&mesh.faces).any(|(m, f)| {
                    m.len() != f.vertices.len() || m.iter().flatten().any(|v| !v.is_finite())
                })
            {
                return Err("Invalid corner weight map".into());
            }
        }
        for map in self.corner_colors.values() {
            if map.len() != mesh.faces.len()
                || map.iter().zip(&mesh.faces).any(|(m, f)| {
                    m.len() != f.vertices.len() || m.iter().flatten().any(|v| !v.is_finite())
                })
            {
                return Err("Invalid corner color map".into());
            }
        }
        if !self.smoothing_labels.is_empty() && self.smoothing_labels.len() != mesh.faces.len() {
            return Err("Invalid smoothing label face count".into());
        }
        if self.relative_morphs.len() > 64
            || self
                .relative_morphs
                .keys()
                .any(|name| name.len() > 1_048_576)
            || self
                .smoothing_labels
                .iter()
                .flatten()
                .any(|label| label.len() > 1_048_576)
        {
            return Err("Relative morph/smoothing label storage budget exceeded".into());
        }
        for map in self.relative_morphs.values() {
            if map
                .iter()
                .any(|(&v, delta)| v as usize >= mesh.vertices.len() || !delta.is_finite())
            {
                return Err("Invalid relative morph delta".into());
            }
        }

        if !self.representations.is_empty() && self.representations.len() != mesh.faces.len() {
            return Err("Invalid polygon representation count".into());
        }
        let edges: BTreeSet<_> = if self.creases.is_empty() {
            BTreeSet::new()
        } else {
            mesh.edges().into_iter().collect()
        };
        let mut seen = BTreeSet::new();
        for c in &self.creases {
            if !c.weight.is_finite()
                || c.a >= c.b
                || !edges.contains(&(c.a, c.b))
                || !seen.insert((c.a, c.b))
            {
                return Err("Invalid or duplicate crease".into());
            }
        }
        for (name, records) in &self.uv_records {
            let map = mesh
                .uv_maps
                .get(name)
                .ok_or("UV record map has no coordinate map")?;
            if records.len() != mesh.faces.len() || map.len() != mesh.faces.len() {
                return Err("Invalid UV record face count".into());
            }
            let mut assignments = BTreeMap::<u32, (u32, DVec2)>::new();
            for ((ids, uvs), face) in records.iter().zip(map).zip(&mesh.faces) {
                if ids.len() != face.vertices.len() || uvs.len() != face.vertices.len() {
                    return Err("Invalid UV record corner count".into());
                }
                for ((record, uv), &vertex) in ids.iter().zip(uvs).zip(&face.vertices) {
                    if let Some(id) = record {
                        let value = uv.ok_or("UV record ID assigned to an absent coordinate")?;
                        if !value.is_finite() {
                            return Err("Non-finite UV assignment".into());
                        }
                        if let Some(previous) = assignments.insert(*id, (vertex, value))
                            && previous != (vertex, value)
                        {
                            return Err("UV record reused for a different vertex or value".into());
                        }
                    }
                }
            }
        }
        if !self.normal_map_order.is_empty()
            && (self.normal_map_order.len() != self.corner_normals.len()
                || self.normal_map_order.iter().collect::<BTreeSet<_>>().len()
                    != self.normal_map_order.len()
                || self
                    .normal_map_order
                    .iter()
                    .any(|n| !self.corner_normals.contains_key(n)))
        {
            return Err("Invalid normal-map creation order".into());
        }
        for (name, normals) in &self.corner_normals {
            if name.is_empty()
                || normals.len() != mesh.faces.len()
                || normals.iter().zip(&mesh.faces).any(|(n, f)| {
                    n.len() != f.vertices.len() || n.iter().flatten().any(|v| !v.is_finite())
                })
            {
                return Err("Invalid sparse normal map".into());
            }
        }
        for map in self.point_weights.values() {
            for (&v, w) in map {
                if v as usize >= mesh.vertices.len() || !w.is_finite() {
                    return Err("Invalid point weight".into());
                }
            }
        }
        for map in self.point_colors.values() {
            for (&v, c) in map {
                if v as usize >= mesh.vertices.len() || !c.is_finite() {
                    return Err("Invalid point color".into());
                }
            }
        }
        if self
            .degenerate_faces
            .iter()
            .any(|&f| f as usize >= mesh.faces.len())
        {
            return Err("Invalid degenerate face index".into());
        }
        Ok(())
    }
}

/// Atomic append preserving all original vertex IDs, cycles, tags and assignments.
/// New map/normal slots on existing geometry are absent; equal UV coordinates
/// never merge corners. Crease IDs are offset, not regenerated from edge order.
pub fn append_mesh(target: &mut Mesh, generated: Mesh) -> Result<()> {
    target.validate()?;
    generated.validate()?;
    if target
        .vertices
        .len()
        .saturating_add(generated.vertices.len())
        > 5_000_000
        || target.faces.len().saturating_add(generated.faces.len()) > 5_000_000
    {
        return Err("Mesh append budget exceeded".into());
    }
    let mut out = target.clone();
    let start = out.vertices.len() as u32;
    let face_start = out.faces.len();
    for name in generated.uv_maps.keys() {
        out.uv_maps.entry(name.clone()).or_insert_with(|| {
            target
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect()
        });
    }
    for (name, map) in &mut out.uv_maps {
        match generated.uv_maps.get(name) {
            Some(new) => map.extend(new.clone()),
            None => map.extend(generated.faces.iter().map(|f| vec![None; f.vertices.len()])),
        }
    }
    let attr = &mut out.attributes;
    for (name, set) in &generated.attributes.vertex_sets {
        attr.vertex_sets
            .entry(name.clone())
            .or_default()
            .extend(set.iter().map(|v| v + start));
    }
    for (name, set) in &generated.attributes.polygon_sets {
        attr.polygon_sets
            .entry(name.clone())
            .or_default()
            .extend(set.iter().map(|v| v + face_start as u32));
    }
    for (name, map) in &generated.attributes.typed_maps {
        if let Some(old) = attr.typed_maps.get(name)
            && (old.kind != map.kind
                || old.dimension != map.dimension
                || old.zero_default != map.zero_default)
        {
            return Err(format!("Incompatible typed map {name}"));
        }
        attr.typed_maps
            .entry(name.clone())
            .or_insert_with(|| TypedMap {
                kind: map.kind.clone(),
                dimension: map.dimension,
                zero_default: map.zero_default,
                points: BTreeMap::new(),
                corners: target
                    .faces
                    .iter()
                    .map(|f| vec![None; f.vertices.len()])
                    .collect(),
            });
    }
    for (name, map) in &mut attr.typed_maps {
        let new = generated.attributes.typed_maps.get(name);
        let needs_corners = !map.corners.is_empty() || new.is_some_and(|m| !m.corners.is_empty());
        if map.corners.is_empty() && needs_corners {
            map.corners = target
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect();
        }
        if let Some(new) = generated.attributes.typed_maps.get(name) {
            map.points
                .extend(new.points.iter().map(|(&v, x)| (v + start, x.clone())));
            if needs_corners {
                map.corners.extend(if new.corners.is_empty() {
                    generated
                        .faces
                        .iter()
                        .map(|f| vec![None; f.vertices.len()])
                        .collect()
                } else {
                    new.corners.clone()
                });
            }
        } else if needs_corners {
            map.corners
                .extend(generated.faces.iter().map(|f| vec![None; f.vertices.len()]));
        }
    }

    for (name, map) in &generated.attributes.point_uvs {
        attr.point_uvs
            .entry(name.clone())
            .or_default()
            .extend(map.iter().map(|(&v, &p)| (v + start, p)));
    }
    for (name, uses) in &generated.attributes.extrude_edge_uses {
        let (a, b) = name.split_once(':').ok_or("Invalid extrusion edge")?;
        let a: u32 = a.parse().map_err(|_| "Invalid extrusion edge")?;
        let b: u32 = b.parse().map_err(|_| "Invalid extrusion edge")?;
        attr.extrude_edge_uses.insert(
            format!("{}:{}", a + start, b + start),
            uses.iter()
                .map(|[f, a, b]| [f + face_start as u32, a + start, b + start])
                .collect(),
        );
    }
    for name in generated.attributes.corner_weights.keys() {
        attr.corner_weights.entry(name.clone()).or_insert_with(|| {
            target
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect()
        });
    }
    for (name, map) in &mut attr.corner_weights {
        map.extend(
            generated
                .attributes
                .corner_weights
                .get(name)
                .cloned()
                .unwrap_or_else(|| {
                    generated
                        .faces
                        .iter()
                        .map(|f| vec![None; f.vertices.len()])
                        .collect()
                }),
        );
    }
    for name in generated.attributes.corner_colors.keys() {
        attr.corner_colors.entry(name.clone()).or_insert_with(|| {
            target
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect()
        });
    }
    for (name, map) in &mut attr.corner_colors {
        map.extend(
            generated
                .attributes
                .corner_colors
                .get(name)
                .cloned()
                .unwrap_or_else(|| {
                    generated
                        .faces
                        .iter()
                        .map(|f| vec![None; f.vertices.len()])
                        .collect()
                }),
        );
    }
    for (name, map) in &generated.attributes.relative_morphs {
        attr.relative_morphs
            .entry(name.clone())
            .or_default()
            .extend(map.iter().map(|(&i, &delta)| (i + start, delta)));
    }
    if !attr.smoothing_labels.is_empty() || !generated.attributes.smoothing_labels.is_empty() {
        if attr.smoothing_labels.is_empty() {
            attr.smoothing_labels.resize(face_start, None);
        }
        if generated.attributes.smoothing_labels.is_empty() {
            attr.smoothing_labels
                .extend(vec![None; generated.faces.len()]);
        } else {
            attr.smoothing_labels
                .extend(generated.attributes.smoothing_labels.clone());
        }
    }

    for name in generated.attributes.uv_records.keys() {
        attr.uv_records.entry(name.clone()).or_insert_with(|| {
            target
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect()
        });
    }
    for (name, records) in &mut attr.uv_records {
        if let Some(new) = generated.attributes.uv_records.get(name) {
            // Source IDs remain byte-for-byte unchanged; offset only new records.
            // Missing legacy identity stays unknown, rather than inferring a seam.
            let offset = match records.iter().flatten().flatten().max() {
                Some(id) => id
                    .checked_add(1)
                    .ok_or("UV record identity budget exhausted")?,
                None => 0,
            };
            for face in new {
                let ids = face
                    .iter()
                    .map(|id| {
                        id.map(|id| {
                            id.checked_add(offset)
                                .ok_or("UV record identity budget exhausted")
                        })
                        .transpose()
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                records.push(ids);
            }
        } else {
            records.extend(generated.faces.iter().map(|f| vec![None; f.vertices.len()]));
        }
    }
    if !attr.representations.is_empty() || !generated.attributes.representations.is_empty() {
        if attr.representations.is_empty() {
            attr.representations
                .resize(face_start, PolygonRepresentation::Face);
        }
        if generated.attributes.representations.is_empty() {
            attr.representations
                .extend(vec![PolygonRepresentation::Face; generated.faces.len()]);
        } else {
            attr.representations
                .extend(&generated.attributes.representations);
        }
    }
    if !attr.normal_map_order.is_empty() || !generated.attributes.normal_map_order.is_empty() {
        if attr.normal_map_order.is_empty() {
            if attr.corner_normals.len() > 1 {
                return Err("Normal-map creation order is unknown in destination".into());
            }
            attr.normal_map_order = attr.corner_normals.keys().cloned().collect();
        }
        let names: Vec<_> = if generated.attributes.normal_map_order.is_empty() {
            generated
                .attributes
                .corner_normals
                .keys()
                .cloned()
                .collect()
        } else {
            generated.attributes.normal_map_order.clone()
        };
        if generated.attributes.normal_map_order.is_empty() && names.len() > 1 {
            return Err("Normal-map creation order is unknown in appended mesh".into());
        }
        for n in names {
            if !attr.normal_map_order.contains(&n) {
                attr.normal_map_order.push(n);
            }
        }
    }
    for name in generated.attributes.corner_normals.keys() {
        attr.corner_normals.entry(name.clone()).or_insert_with(|| {
            target
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect()
        });
    }
    for (name, map) in &mut attr.corner_normals {
        match generated.attributes.corner_normals.get(name) {
            Some(new) => map.extend(new.clone()),
            None => map.extend(generated.faces.iter().map(|f| vec![None; f.vertices.len()])),
        }
    }
    attr.creases
        .extend(generated.attributes.creases.iter().map(|c| MeshCrease {
            a: c.a + start,
            b: c.b + start,
            weight: c.weight,
        }));
    for (name, map) in &generated.attributes.point_weights {
        attr.point_weights
            .entry(name.clone())
            .or_default()
            .extend(map.iter().map(|(&i, &w)| (i + start, w)));
    }
    for (name, map) in &generated.attributes.point_colors {
        attr.point_colors
            .entry(name.clone())
            .or_default()
            .extend(map.iter().map(|(&i, &c)| (i + start, c)));
    }
    attr.degenerate_faces.extend(
        generated
            .attributes
            .degenerate_faces
            .iter()
            .map(|&i| i + face_start as u32),
    );
    out.vertices.extend(generated.vertices);
    out.faces.extend(generated.faces.into_iter().map(|mut f| {
        for i in &mut f.vertices {
            *i += start;
        }
        f
    }));
    out.validate()?;
    *target = out;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;

    fn triangle() -> Mesh {
        Mesh {
            vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
            faces: vec![Face {
                vertices: vec![0, 1, 2],
                uv: None,
                material: "material".into(),
                smooth: true,
                part: Some("part".into()),
                material_label: None,
            }]
            .into(),
            ..Mesh::default()
        }
    }

    fn attributed() -> Mesh {
        let mut mesh = triangle();
        mesh.uv_maps
            .insert("uv".into(), vec![vec![Some(DVec2::X), None, None]]);
        let a = &mut mesh.attributes;
        a.typed_maps.insert(
            "typed".into(),
            TypedMap {
                kind: "weight1".into(),
                dimension: 1,
                zero_default: false,
                points: [(1, vec![0.5])].into(),
                corners: vec![vec![Some(vec![0.75]), None, None]],
            },
        );
        a.vertex_sets.insert("vertices".into(), [1].into());
        a.polygon_sets.insert("polygons".into(), [0].into());
        a.point_uvs.insert("uv".into(), [(1, DVec2::Y)].into());
        a.corner_weights
            .insert("weight".into(), vec![vec![Some(0.25), None, None]]);
        a.corner_colors
            .insert("color".into(), vec![vec![Some(DVec4::ONE), None, None]]);
        a.relative_morphs
            .insert("morph".into(), [(2, DVec3::Z)].into());
        a.smoothing_labels = vec![Some("smooth".into())];
        a.representations = vec![PolygonRepresentation::Face];
        a.uv_records
            .insert("uv".into(), vec![vec![Some(3), None, None]]);
        a.corner_normals
            .insert("normal".into(), vec![vec![Some(DVec3::Z), None, None]]);
        a.normal_map_order = vec!["normal".into()];
        a.creases.push(MeshCrease {
            a: 0,
            b: 1,
            weight: 0.5,
        });
        a.point_weights.insert("weight".into(), [(1, 0.25)].into());
        a.point_colors
            .insert("color".into(), [(2, DVec4::ONE)].into());
        mesh.validate().unwrap();
        mesh
    }

    #[test]
    fn mesh_attributes_append_plain_and_empty_meshes() {
        let original = triangle();
        let mut mesh = original.clone();
        append_mesh(&mut mesh, Mesh::default()).unwrap();
        assert_eq!(mesh, original);
        let mut empty = Mesh::default();
        append_mesh(&mut empty, original.clone()).unwrap();
        assert_eq!(empty, original);
        append_mesh(&mut mesh, original).unwrap();
        assert_eq!(mesh.faces[1].vertices, [3, 4, 5]);
        assert_eq!(mesh.faces[1].material, "material");
        assert_eq!(&*mesh.attributes, &MeshAttributes::default());
        assert!(mesh.uv_maps.is_empty());
        let mut empty = Mesh::default();
        append_mesh(&mut empty, Mesh::default()).unwrap();
        assert_eq!(empty, Mesh::default());
    }

    #[test]
    fn mesh_attributes_append_shared_maps_and_indices() {
        let mut mesh = attributed();
        append_mesh(&mut mesh, attributed()).unwrap();
        let a = &mesh.attributes;
        assert_eq!(mesh.faces[1].vertices, [3, 4, 5]);
        assert_eq!(a.vertex_sets["vertices"], [1, 4].into());
        assert_eq!(a.polygon_sets["polygons"], [0, 1].into());
        assert_eq!(
            a.typed_maps["typed"]
                .points
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [1, 4]
        );
        assert_eq!(a.typed_maps["typed"].corners[1][0], Some(vec![0.75]));
        assert_eq!(a.point_uvs["uv"].get(&4), Some(&DVec2::Y));
        assert_eq!(mesh.uv_maps["uv"][1][0], Some(DVec2::X));
        assert_eq!(
            a.uv_records["uv"],
            vec![vec![Some(3), None, None], vec![Some(7), None, None]]
        );
        assert_eq!(a.corner_weights["weight"][1][0], Some(0.25));
        assert_eq!(a.corner_colors["color"][1][0], Some(DVec4::ONE));
        assert_eq!(a.corner_normals["normal"][1][0], Some(DVec3::Z));
        assert_eq!(a.normal_map_order, ["normal"]);
        assert_eq!(a.relative_morphs["morph"].get(&5), Some(&DVec3::Z));
        assert_eq!(a.point_weights["weight"].get(&4), Some(&0.25));
        assert_eq!(a.point_colors["color"].get(&5), Some(&DVec4::ONE));
        assert_eq!(
            a.smoothing_labels,
            [Some("smooth".into()), Some("smooth".into())]
        );
        assert_eq!(a.representations, [PolygonRepresentation::Face; 2]);
        assert_eq!(
            a.creases[1],
            MeshCrease {
                a: 3,
                b: 4,
                weight: 0.5
            }
        );
        mesh.validate().unwrap();
    }

    #[test]
    fn mesh_attributes_append_maps_on_only_one_side() {
        for source_has_attributes in [false, true] {
            let (mut target, source) = if source_has_attributes {
                (triangle(), attributed())
            } else {
                (attributed(), triangle())
            };
            append_mesh(&mut target, source).unwrap();
            let present = usize::from(source_has_attributes);
            let absent = 1 - present;
            let a = &target.attributes;
            assert_eq!(target.uv_maps["uv"][absent], vec![None; 3]);
            assert_eq!(target.uv_maps["uv"][present][0], Some(DVec2::X));
            assert_eq!(a.typed_maps["typed"].corners[absent], vec![None; 3]);
            assert_eq!(a.corner_weights["weight"][absent], vec![None; 3]);
            assert_eq!(a.corner_colors["color"][absent], vec![None; 3]);
            assert_eq!(a.corner_normals["normal"][absent], vec![None; 3]);
            assert_eq!(a.uv_records["uv"][absent], vec![None; 3]);
            assert_eq!(a.smoothing_labels[absent], None);
            assert_eq!(a.smoothing_labels[present].as_deref(), Some("smooth"));
            assert_eq!(a.representations.len(), 2);
            let offset = if source_has_attributes { 3 } else { 0 };
            assert!(a.vertex_sets["vertices"].contains(&(1 + offset)));
            assert!(a.point_uvs["uv"].contains_key(&(1 + offset)));
            assert!(a.point_weights["weight"].contains_key(&(1 + offset)));
            assert!(a.point_colors["color"].contains_key(&(2 + offset)));
            assert!(a.relative_morphs["morph"].contains_key(&(2 + offset)));
            target.validate().unwrap();
        }
    }

    #[test]
    fn mesh_attributes_effective_and_validation() {
        let mesh = attributed();
        let mut map = mesh.attributes.typed_maps["typed"].clone();
        assert_eq!(map.effective(&mesh, 0, 0), Some(vec![0.75]));
        assert_eq!(map.effective(&mesh, 0, 1), Some(vec![0.5]));
        assert_eq!(map.effective(&mesh, 0, 2), None);
        map.zero_default = true;
        assert_eq!(map.effective(&mesh, 0, 2), Some(vec![0.0]));
        assert_eq!(map.effective(&mesh, 99, 99), Some(vec![0.0]));
        assert!(mesh.attributes.has_typed_data());
        assert!(!MeshAttributes::default().has_typed_data());
        let mut invalid = (*mesh.attributes).clone();
        invalid
            .typed_maps
            .get_mut("typed")
            .unwrap()
            .points
            .insert(9, vec![1.0]);
        assert!(invalid.validate(&mesh).is_err());
    }

    #[test]
    fn mesh_attributes_append_errors_are_atomic() {
        let mut target = attributed();
        let before = target.clone();
        let mut source = attributed();
        source
            .attributes
            .typed_maps
            .get_mut("typed")
            .unwrap()
            .zero_default = true;
        assert!(append_mesh(&mut target, source).is_err());
        assert_eq!(target, before);
        let mut invalid = triangle();
        invalid
            .attributes
            .vertex_sets
            .insert("bad".into(), [9].into());
        assert!(append_mesh(&mut target, invalid).is_err());
        assert_eq!(target, before);
    }
}
