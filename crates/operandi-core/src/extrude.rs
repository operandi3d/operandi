//! Reviewed EX-001..014 numerical extrusion. Pure candidate construction;
//! scene/history and live-stage policies belong to Editor. No renderer ownership.
use crate::topology::edge_key;
use crate::*;
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtrudeMode {
    #[default]
    Auto,
    Thick,
    Shift,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtrudeUvs {
    None,
    #[default]
    U,
    V,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ExtrudeResult {
    pub mesh: Mesh,
    pub caps: Vec<u32>,
    pub moved_caps: Vec<u32>,
    pub correspondence: crate::EditCorrespondence,
}
fn first_normal(m: &Mesh, fi: usize) -> DVec3 {
    let v = &m.faces[fi].vertices;
    (m.vertices[v[1] as usize] - m.vertices[v[0] as usize])
        .cross(m.vertices[*v.last().unwrap() as usize] - m.vertices[v[0] as usize])
        .normalize_or_zero()
}
/// Positive finite scalar truncation/minimum, bounded before integer conversion.
/// The maximum is an Operandi allocation guard, never a reference maximum.
pub fn extrude_side_count(v: f64) -> Result<u32> {
    if !v.is_finite() || v > 1_000_000. {
        return Err("Sides exceeds finite operation budget".into());
    }
    Ok(v.trunc().max(1.) as u32)
}
/// Explicit corners override continuous point values. Missing is never zero.
pub fn effective_uv(m: &Mesh, name: &str, fi: usize, ci: usize) -> Option<DVec2> {
    m.uv_maps.get(name).and_then(|v| v[fi][ci]).or_else(|| {
        m.attributes
            .point_uvs
            .get(name)
            .and_then(|v| v.get(&m.faces[fi].vertices[ci]).copied())
    })
}
impl Mesh {
    pub fn validate_extrude_selection(&self, selected: &[u32]) -> Result<()> {
        self.validate()?;
        self.require_surface_elements()?;
        let set: BTreeSet<_> = selected.iter().copied().collect();
        if selected.is_empty() || set.len() != selected.len() {
            return Err("Select distinct polygons explicitly".into());
        }
        let mut count = 0;
        for &fi in selected {
            let f = self
                .faces
                .get(fi as usize)
                .ok_or("Invalid selected polygon")?;
            count += f.vertices.len();
            if count > 100_000 {
                return Err("Extrude exceeds 100,000 selected corners".into());
            }
            self.surface_triangles(fi as usize)?;
        }
        Ok(())
    }
}
/// Positions and `extent` share the input coordinate space. Editor supplies
/// world geometry for facing and returns the validated result to local storage.
pub fn extrude_geometry(
    source: &Mesh,
    selected: &[u32],
    extent: DVec3,
    sides: u32,
    mode: ExtrudeMode,
    uvs: ExtrudeUvs,
    map: Option<&str>,
) -> Result<ExtrudeResult> {
    source.validate_extrude_selection(selected)?;
    let set_profile = !source.attributes.polygon_sets.is_empty();
    if source.attributes.has_typed_data()
        && (!set_profile || !set_profile_supported(source, selected, extent, sides, mode, uvs)?)
    {
        return Err("This tool cannot yet remap typed maps/named sets; source retained".into());
    }
    if source
        .attributes
        .corner_weights
        .keys()
        .any(|n| !source.attributes.point_weights.contains_key(n))
        || source
            .attributes
            .corner_colors
            .keys()
            .any(|n| !source.attributes.point_colors.contains_key(n))
    {
        // Spec: EX-Q03
        return Err(
            "Extrude can't handle weight or color maps stored only on corners yet; nothing changed"
                .into(),
        );
    }
    if !extent.is_finite() || sides == 0 {
        return Err("Finite Extent and positive Sides required".into());
    }
    if map.is_some_and(|n| !source.uv_maps.contains_key(n)) {
        return Err("UV map definition is missing".into());
    }
    let set: BTreeSet<_> = selected.iter().map(|&f| f as usize).collect();
    let mut points = Vec::new();
    let mut rank = BTreeMap::new();
    for &fi in selected {
        for &v in &source.faces[fi as usize].vertices {
            if let std::collections::btree_map::Entry::Vacant(entry) = rank.entry(v) {
                entry.insert(points.len());
                points.push(v);
            }
        }
    }
    let mut uses: BTreeMap<_, Vec<(usize, usize)>> = BTreeMap::new();
    for (fi, f) in source.faces.iter().enumerate() {
        for (ci, &a) in f.vertices.iter().enumerate() {
            uses.entry(edge_key(a, f.vertices[(ci + 1) % f.vertices.len()]))
                .or_default()
                .push((fi, ci));
        }
    }
    let mut boundary = Vec::new();
    for &fi in selected {
        let fi = fi as usize;
        let f = &source.faces[fi];
        for (ci, &a) in f.vertices.iter().enumerate() {
            let b = f.vertices[(ci + 1) % f.vertices.len()];
            let incident = &uses[&edge_key(a, b)];
            // PE-N-01: an edge shared with two or more unselected polygons
            // still gets a wall (the edge then has one more use).
            if incident.iter().filter(|(f, _)| set.contains(f)).count() == 1 {
                let owner = incident
                    .iter()
                    .find(|(f, _)| !set.contains(f))
                    .map_or(fi, |v| v.0);
                boundary.push((fi, ci, a, b, owner));
            }
        }
    }
    if boundary.is_empty() {
        return Ok(ExtrudeResult {
            mesh: source.clone(),
            caps: selected.to_vec(),
            moved_caps: selected.to_vec(),
            correspondence: crate::EditCorrespondence::identity(source),
        });
    }
    let thick = mode == ExtrudeMode::Thick
        || (mode == ExtrudeMode::Auto
            && !source.faces.iter().enumerate().any(|(fi, f)| {
                !set.contains(&fi) && f.vertices.iter().any(|v| rank.contains_key(v))
            }));
    let zero = thick && extent == DVec3::ZERO;
    let boundary_points: BTreeSet<_> = boundary.iter().flat_map(|v| [v.2, v.3]).collect();
    let generated = if zero {
        0
    } else if thick {
        points.len().saturating_mul(sides as usize)
    } else {
        boundary_points.len().saturating_mul(sides as usize)
    };
    let walls = if zero {
        0
    } else {
        boundary.len().saturating_mul(sides as usize)
    };
    let maps = source.uv_maps.len()
        + source.attributes.corner_normals.len()
        + source.attributes.point_weights.len()
        + source.attributes.point_colors.len()
        + source.attributes.relative_morphs.len()
        + source.attributes.corner_weights.len()
        + source.attributes.corner_colors.len()
        + source.attributes.uv_records.len()
        + 1;
    if generated > 1_000_000
        || walls > 1_000_000
        || source.vertices.len() + generated > 5_000_000
        || source.faces.len() + walls + selected.len() > 5_000_000
        || (walls * 4 + points.len() * 2 + generated).saturating_mul(maps) > 8_000_000
    {
        return Err("Extrude allocation budget exceeded; reduce Sides/selection/maps".into());
    }
    let mut correspondence = crate::EditCorrespondence::identity(source);
    let mut out = source.clone();
    let mut rows = vec![points.iter().map(|&v| (v, v)).collect::<BTreeMap<_, _>>()];
    for k in 1..=if zero { 0 } else { sides } {
        let mut row = BTreeMap::new();
        for &v in &points {
            let id = if !thick && !boundary_points.contains(&v) {
                v
            } else {
                let id = out.vertices.len() as u32;
                out.vertices
                    .push(source.vertices[v as usize] + extent * (k as f64 / sides as f64));
                correspondence.vertex_sources.push(v);
                id
            };
            if id == v {
                out.vertices[v as usize] = source.vertices[v as usize] + extent;
            }
            row.insert(v, id);
            for (name, m) in &source.attributes.point_weights {
                if let Some(&x) = m.get(&v) {
                    out.attributes
                        .point_weights
                        .get_mut(name)
                        .unwrap()
                        .insert(id, x);
                }
            }
            for (name, m) in &source.attributes.point_colors {
                if let Some(&x) = m.get(&v) {
                    out.attributes
                        .point_colors
                        .get_mut(name)
                        .unwrap()
                        .insert(id, x);
                }
            }
            for (name, m) in &source.attributes.relative_morphs {
                if let Some(&x) = m.get(&v) {
                    out.attributes
                        .relative_morphs
                        .get_mut(name)
                        .unwrap()
                        .insert(id, x);
                }
            }
            for (name, m) in &source.attributes.point_uvs {
                if let Some(&x) = m.get(&v) {
                    out.attributes
                        .point_uvs
                        .get_mut(name)
                        .unwrap()
                        .insert(id, x);
                }
            }
        }
        rows.push(row);
    }
    let tip = rows.last().unwrap().clone();
    let mut promoted: BTreeMap<String, BTreeMap<u32, DVec2>> = BTreeMap::new();
    for (name, values) in &source.uv_maps {
        for &fi in selected {
            let fi = fi as usize;
            for (ci, &v) in source.faces[fi].vertices.iter().enumerate() {
                if let Some(x) = values[fi][ci] {
                    promoted
                        .entry(name.clone())
                        .or_default()
                        .entry(tip[&v])
                        .or_insert(x);
                }
            }
        }
    }
    let generated_uv = |v: u32, k: u32| {
        let other = map
            .and_then(|n| source.attributes.point_uvs.get(n))
            .and_then(|m| m.get(&v))
            .map(|p| if uvs == ExtrudeUvs::U { p.y } else { p.x })
            .unwrap_or(rank[&v] as f64 / points.len() as f64);
        if uvs == ExtrudeUvs::U {
            DVec2::new(k as f64 / sides as f64, other)
        } else {
            DVec2::new(other, k as f64 / sides as f64)
        }
    };
    if let Some(name) = map.filter(|_| uvs != ExtrudeUvs::None) {
        for (k, row) in rows.iter().enumerate() {
            for (&v, &id) in row {
                out.attributes
                    .point_uvs
                    .entry(name.into())
                    .or_default()
                    .insert(id, generated_uv(v, if zero { sides } else { k as u32 }));
            }
        }
    }
    let mut caps = Vec::new();
    let mut moved = Vec::new();
    for &fi in selected {
        let fi = fi as usize;
        let positive = first_normal(source, fi).dot(extent) > 0.;
        if thick {
            cap_payload(source, &mut out, fi, fi, &rows[0], positive)?;
            caps.push(fi as u32);
            let id = out.faces.len();
            append_slots(source, &mut out, fi);
            correspondence.face_sources.push(fi);
            cap_payload(source, &mut out, fi, id, &tip, !positive)?;
            caps.push(id as u32);
            moved.push(id as u32);
        } else {
            cap_payload(source, &mut out, fi, fi, &tip, false)?;
            caps.push(fi as u32);
            moved.push(fi as u32);
        }
    }
    for k in 1..rows.len() {
        for &(fi, ci, a, b, owner) in &boundary {
            let mut corners = vec![(a, k), (a, k - 1), (b, k - 1), (b, k)];
            if thick && first_normal(source, owner).dot(extent) <= 0. {
                corners[1..].reverse();
            }
            let id = out.faces.len();
            append_slots(source, &mut out, owner);
            out.faces[id].vertices = corners.iter().map(|&(v, k)| rows[k][&v]).collect();
            correspondence.face_sources.push(owner);
            correspondence.walls.push(crate::WallOrigin {
                face: id,
                selected: fi,
                owner,
                a,
                b,
                row: k,
                vertices: out.faces[id].vertices.clone(),
            });
            if set_profile {
                for (name, members) in &source.attributes.polygon_sets {
                    if members.contains(&(owner as u32)) {
                        out.attributes
                            .polygon_sets
                            .get_mut(name)
                            .unwrap()
                            .insert(id as u32);
                    }
                }
                for (name, records) in &source.attributes.uv_records {
                    out.attributes.uv_records.get_mut(name).unwrap()[id] = corners
                        .iter()
                        .map(|&(v, row)| {
                            if row == 0 {
                                records[owner][source.faces[owner]
                                    .vertices
                                    .iter()
                                    .position(|&x| x == v)
                                    .unwrap()]
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
            out.faces[id].uv = source.faces[owner].uv.as_ref().map(|uv| {
                corners
                    .iter()
                    .map(|&(v, _)| {
                        uv[source.faces[owner]
                            .vertices
                            .iter()
                            .position(|&x| x == v)
                            .unwrap()]
                    })
                    .collect()
            });
            for (name, values) in &source.uv_maps {
                let mut result = Vec::new();
                for &(v, row) in &corners {
                    let owner_ci = source.faces[owner]
                        .vertices
                        .iter()
                        .position(|&x| x == v)
                        .unwrap();
                    let selected_ci = if v == a {
                        ci
                    } else {
                        (ci + 1) % source.faces[fi].vertices.len()
                    };
                    let explicit = if row == 0 {
                        values[owner][owner_ci]
                    } else if row == sides as usize {
                        values[fi][selected_ci]
                    } else {
                        None
                    };
                    let value = if Some(name.as_str()) == map && uvs != ExtrudeUvs::None {
                        let mut p = generated_uv(v, row as u32);
                        if let Some(old) = explicit {
                            if uvs == ExtrudeUvs::U {
                                p.y = old.y;
                            } else {
                                p.x = old.x;
                            }
                        }
                        Some(p)
                    } else if row == 0 {
                        effective_uv(source, name, owner, owner_ci)
                    } else {
                        source
                            .attributes
                            .point_uvs
                            .get(name)
                            .and_then(|m| m.get(&v).copied())
                    };
                    result.push(value);
                }
                out.uv_maps.get_mut(name).unwrap()[id] = result;
            }
            for (name, values) in &source.attributes.corner_normals {
                out.attributes.corner_normals.get_mut(name).unwrap()[id] = corners
                    .iter()
                    .map(|&(v, row)| {
                        if row == 0 {
                            values[owner][source.faces[owner]
                                .vertices
                                .iter()
                                .position(|&x| x == v)
                                .unwrap()]
                        } else {
                            Some(DVec3::ZERO)
                        }
                    })
                    .collect();
            }
            for name in source.attributes.point_weights.keys() {
                out.attributes
                    .corner_weights
                    .entry(name.clone())
                    .or_insert_with(|| {
                        source
                            .faces
                            .iter()
                            .map(|f| vec![None; f.vertices.len()])
                            .collect()
                    });
            }
            // New walls use point correspondence, unlike ordinal cap copying.
            for (name, values) in &mut out.attributes.corner_weights {
                if values.len() <= id {
                    values.resize_with(id + 1, Vec::new);
                }
                values[id] = corners
                    .iter()
                    .map(|&(v, _)| {
                        source
                            .attributes
                            .point_weights
                            .get(name)
                            .and_then(|m| m.get(&v).copied())
                    })
                    .collect();
            }
            for (name, values) in &mut out.attributes.corner_colors {
                values[id] = corners
                    .iter()
                    .map(|&(v, _)| {
                        source
                            .attributes
                            .point_colors
                            .get(name)
                            .and_then(|m| m.get(&v).copied())
                    })
                    .collect();
            }
            if out.normal(id) == DVec3::ZERO {
                out.attributes.degenerate_faces.insert(id as u32);
            }
        }
    }
    for &fi in &moved {
        for (name, values) in &mut out.uv_maps {
            for (ci, &v) in out.faces[fi as usize].vertices.iter().enumerate() {
                if values[fi as usize][ci].is_none()
                    && (Some(name.as_str()) != map || uvs == ExtrudeUvs::None)
                {
                    values[fi as usize][ci] = promoted.get(name).and_then(|m| m.get(&v).copied());
                }
            }
        }
    }
    // Bake effective generated point values into absent corner slots for the
    // current viewport adapter. Keep point storage for later editing semantics.
    for (name, values) in &mut out.uv_maps {
        for (fi, f) in out.faces.iter().enumerate() {
            for (ci, &v) in f.vertices.iter().enumerate() {
                if values[fi][ci].is_none() {
                    values[fi][ci] = out
                        .attributes
                        .point_uvs
                        .get(name)
                        .and_then(|m| m.get(&v).copied());
                }
            }
        }
    }
    // Record identities afresh only for modified/new corners. Unknown legacy
    // assignment identity remains unknown; equal values never imply ownership.
    for (name, ids) in &mut out.attributes.uv_records {
        if set_profile {
            continue;
        }
        let mut next = source.attributes.uv_records[name]
            .iter()
            .flatten()
            .flatten()
            .max()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("UV identity budget")?;
        let required: usize = out
            .faces
            .iter()
            .enumerate()
            .filter(|(fi, _)| *fi >= source.faces.len() || set.contains(fi))
            .map(|(fi, _)| out.uv_maps[name][fi].iter().filter(|v| v.is_some()).count())
            .sum();
        next.checked_add(u32::try_from(required).map_err(|_| "UV identity budget")?)
            .ok_or("UV identity budget")?;
        for (fi, f) in out.faces.iter().enumerate() {
            if fi >= source.faces.len() || set.contains(&fi) {
                ids[fi] = f
                    .vertices
                    .iter()
                    .enumerate()
                    .map(|(ci, _)| {
                        if out.uv_maps[name][fi][ci].is_some() {
                            let id = next;
                            next += 1;
                            Some(id)
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }
    }
    let actual: BTreeSet<_> = out.edges().into_iter().collect();
    let mut creases: BTreeMap<_, _> = source
        .attributes
        .creases
        .iter()
        .filter(|c| actual.contains(&edge_key(c.a, c.b)))
        .map(|c| (edge_key(c.a, c.b), c.weight))
        .collect();
    for c in &source.attributes.creases {
        for row in &rows {
            if let (Some(&a), Some(&b)) = (row.get(&c.a), row.get(&c.b))
                && actual.contains(&edge_key(a, b))
            {
                creases.insert(edge_key(a, b), c.weight);
            }
        }
        if thick {
            for &fi in selected {
                let f = &source.faces[fi as usize];
                for i in 0..f.vertices.len() {
                    if edge_key(f.vertices[i], f.vertices[(i + 1) % f.vertices.len()])
                        == edge_key(c.a, c.b)
                        && first_normal(source, fi as usize).dot(extent) > 0.
                    {
                        let mut reversed = f.vertices.clone();
                        reversed[1..].reverse();
                        creases.insert(
                            edge_key(reversed[i], reversed[(i + 1) % reversed.len()]),
                            c.weight,
                        );
                    }
                }
            }
        }
    }
    out.attributes.creases = creases
        .into_iter()
        .map(|((a, b), weight)| MeshCrease { a, b, weight })
        .collect();
    out.attributes.extrude_edge_uses.clear();
    let mut incidents: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (fi, f) in out.faces.iter().enumerate() {
        for i in 0..f.vertices.len() {
            let a = f.vertices[i];
            let b = f.vertices[(i + 1) % f.vertices.len()];
            incidents
                .entry(edge_key(a, b))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    for ((a, b), v) in incidents {
        if v.len() > 2 || (v.len() == 2 && v[0][1..] == v[1][1..]) {
            out.attributes
                .extrude_edge_uses
                .insert(format!("{a}:{b}"), v);
        }
    }
    out.validate()?;
    correspondence.validate(source, &out)?;
    Ok(ExtrudeResult {
        mesh: out,
        caps,
        moved_caps: moved,
        correspondence,
    })
}
fn append_slots(source: &Mesh, out: &mut Mesh, owner: usize) {
    out.faces.push(source.faces[owner].clone());
    for (n, m) in &mut out.uv_maps {
        m.push(source.uv_maps[n][owner].clone());
    }
    for (n, m) in &mut out.attributes.uv_records {
        m.push(source.attributes.uv_records[n][owner].clone());
    }
    for (n, m) in &mut out.attributes.corner_normals {
        m.push(source.attributes.corner_normals[n][owner].clone());
    }
    for m in out.attributes.corner_weights.values_mut() {
        m.push(vec![None; source.faces[owner].vertices.len()]);
    }
    for m in out.attributes.corner_colors.values_mut() {
        m.push(vec![None; source.faces[owner].vertices.len()]);
    }
    if !out.attributes.representations.is_empty() {
        out.attributes
            .representations
            .push(source.attributes.representations[owner]);
    }
    if !out.attributes.smoothing_labels.is_empty() {
        out.attributes
            .smoothing_labels
            .push(source.attributes.smoothing_labels[owner].clone());
    }
}
fn cap_payload(
    source: &Mesh,
    out: &mut Mesh,
    fi: usize,
    id: usize,
    row: &BTreeMap<u32, u32>,
    reverse: bool,
) -> Result<()> {
    let f = &source.faces[fi];
    let mut cycle = f.vertices.clone();
    if reverse {
        cycle[1..].reverse();
    }
    out.faces[id].vertices = cycle.iter().map(|v| row[v]).collect();
    for (name, map) in &source.attributes.point_weights {
        let values: Vec<_> = f
            .vertices
            .iter()
            .enumerate()
            .map(|(ci, v)| {
                source
                    .attributes
                    .corner_weights
                    .get(name)
                    .and_then(|m| m[fi][ci])
                    .or_else(|| map.get(v).copied())
            })
            .collect();
        let target = out
            .attributes
            .corner_weights
            .entry(name.clone())
            .or_insert_with(|| {
                out.faces
                    .iter()
                    .map(|f| vec![None; f.vertices.len()])
                    .collect()
            });
        target.resize_with(out.faces.len(), Vec::new);
        target[id] = values;
    }
    for (name, map) in &source.attributes.point_colors {
        let values: Vec<_> = f
            .vertices
            .iter()
            .enumerate()
            .map(|(ci, v)| {
                source
                    .attributes
                    .corner_colors
                    .get(name)
                    .and_then(|m| m[fi][ci])
                    .or_else(|| map.get(v).copied())
            })
            .collect();
        let target = out
            .attributes
            .corner_colors
            .entry(name.clone())
            .or_insert_with(|| {
                out.faces
                    .iter()
                    .map(|f| vec![None; f.vertices.len()])
                    .collect()
            });
        target.resize_with(out.faces.len(), Vec::new);
        target[id] = values;
    }
    // Existing corner payloads follow ordinal positions even when geometry flips.
    for name in source.uv_maps.keys() {
        out.uv_maps.get_mut(name).unwrap()[id] = f
            .vertices
            .iter()
            .enumerate()
            .map(|(ci, _)| effective_uv(source, name, fi, ci))
            .collect();
    }
    for (name, values) in &source.attributes.corner_normals {
        out.attributes.corner_normals.get_mut(name).unwrap()[id] = values[fi].clone();
    }
    Ok(())
}

/// Extent-dependent part of the named-set profile, shared with I2 Move.
/// Callers must separately establish the profile's unchanged source conditions.
pub(crate) fn set_profile_extent_supported(source: &Mesh, fi: usize, extent: DVec3) -> bool {
    let normal = first_normal(source, fi);
    !(normal == DVec3::ZERO
        || !extent.is_finite()
        || !extent.length().is_finite()
        || normal.cross(extent).length() > 1e-10 * (1. + extent.length()))
}

/// New I1-S profile only. Existing no-set paths retain their admitted algorithms.
/// This checks all payload, rather than deleting sets to enter an older remapper.
fn set_profile_supported(
    source: &Mesh,
    selected: &[u32],
    extent: DVec3,
    sides: u32,
    mode: ExtrudeMode,
    uvs: ExtrudeUvs,
) -> Result<bool> {
    let a = &source.attributes;
    let allowed = crate::MeshAttributes {
        polygon_sets: a.polygon_sets.clone(),
        uv_records: a.uv_records.clone(),
        degenerate_faces: a.degenerate_faces.clone(),
        ..Default::default()
    };
    if a != &allowed
        || selected.len() != 1
        || sides != 1
        || mode != ExtrudeMode::Shift
        || uvs != ExtrudeUvs::None
        || source.faces.iter().any(|f| f.uv.is_some())
    {
        return Ok(false);
    }
    let fi = selected[0] as usize;
    if source.faces[fi].vertices.len() != 4 {
        return Ok(false);
    }
    source.triangles(fi)?;
    if !set_profile_extent_supported(source, fi, extent) {
        return Ok(false);
    }
    let normal = first_normal(source, fi);
    let q = &source.faces[fi].vertices;
    for c in 0..4 {
        let a = source.vertices[q[c] as usize];
        let b = source.vertices[q[(c + 1) % 4] as usize];
        let d = source.vertices[q[(c + 2) % 4] as usize];
        if (b - a).cross(d - b).dot(normal) <= 0. {
            return Ok(false);
        }
    }
    let mut uses: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for (f, face) in source.faces.iter().enumerate() {
        for c in 0..face.vertices.len() {
            uses.entry(edge_key(
                face.vertices[c],
                face.vertices[(c + 1) % face.vertices.len()],
            ))
            .or_default()
            .push(f);
        }
        if !a.degenerate_faces.contains(&(f as u32)) {
            source.triangles(f)?;
        }
    }
    let face = &source.faces[fi];
    for c in 0..4 {
        let va = face.vertices[c];
        let vb = face.vertices[(c + 1) % 4];
        let incident = &uses[&edge_key(va, vb)];
        if incident.len() > 2 {
            return Ok(false);
        }
        if incident.len() == 1 {
            for records in a.uv_records.values() {
                if records[fi][c].is_some() || records[fi][(c + 1) % 4].is_some() {
                    return Err("I1-D01: known UV source/tip assignment splitting needs a reviewed policy; source retained".into());
                }
            }
        }
    }
    Ok(true)
}
