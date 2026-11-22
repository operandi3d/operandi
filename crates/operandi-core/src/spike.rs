//! SA-002..008: independent source-face fans. Source corner order is semantic.
//! Results own a complete candidate mesh and source-to-result correspondence;
//! neither scene state nor a viewport is required to evaluate the operation.
use crate::*;
use glam::{DVec2, DVec3};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
pub struct SpikeResult {
    pub mesh: Mesh,
    /// Result face IDs per source face, including unchanged faces.
    pub descendants: Vec<Vec<u32>>,
}

/// Empty `selected` means all polygons of this mesh, never all scene items.
/// Limits are Operandi allocation guards, not reference Strength clamps.
pub fn spike_geometry(source: &Mesh, selected: &[u32], strength: f64) -> Result<SpikeResult> {
    spike_geometry_weighted(source, selected, strength, None)
}

/// Spike with a falloff: each face's tip height scales by the mean of its
/// corners' weights, and a face whose weights are all 0 stays unspiked
/// (FAL-B3-03).
pub fn spike_geometry_weighted(
    source: &Mesh,
    selected: &[u32],
    strength: f64,
    weights: Option<&[f64]>,
) -> Result<SpikeResult> {
    source.validate()?;
    source.require_surface_elements()?;
    if source.attributes.has_typed_data() {
        return Err("This tool cannot yet remap typed maps/named sets; source retained".into());
    }
    if !strength.is_finite() {
        return Err("Spike Strength must be finite".into());
    }
    if source.faces.is_empty() {
        return Err("Spike needs one or more polygons".into());
    }
    let selected: BTreeSet<_> = if selected.is_empty() {
        (0..source.faces.len() as u32).collect()
    } else {
        let unique: BTreeSet<_> = selected.iter().copied().collect();
        if unique.len() != selected.len()
            || unique.iter().any(|&f| f as usize >= source.faces.len())
        {
            return Err("Invalid or duplicate Spike polygon selection".into());
        }
        unique
    };
    let face_weight = |f: usize| -> f64 {
        weights.map_or(1., |w| {
            let vs = &source.faces[f].vertices;
            vs.iter().map(|&v| w[v as usize]).sum::<f64>() / vs.len() as f64
        })
    };
    let selected: BTreeSet<u32> = selected
        .into_iter()
        .filter(|&f| face_weight(f as usize) > 0.)
        .collect();
    // Refuse any selected face sharing a vertex with an exceptional boundary.
    // Spike reads selected-face corners only, but this guard also covers adjacent
    // topology rather than relying on selected-face IDs alone.
    let exceptional_vertices: BTreeSet<u32> = source
        .attributes
        .extrude_edge_uses
        .values()
        .flatten()
        .flat_map(|r| [r[1], r[2]])
        .collect();
    if selected.iter().any(|&f| {
        source.faces[f as usize]
            .vertices
            .iter()
            .any(|v| exceptional_vertices.contains(v))
    }) {
        return Err("Spike on an exceptional winding boundary is unsupported".into());
    }
    let corners: usize = selected
        .iter()
        .map(|&f| source.faces[f as usize].vertices.len())
        .sum();
    if corners > 100_000
        || source.vertices.len() + selected.len() > 5_000_000
        || source.faces.len() - selected.len() + corners > 5_000_000
    {
        return Err("Spike exceeds the 100,000 selected-corner or mesh allocation budget".into());
    }
    let maps = 1
        + source.uv_maps.len()
        + source.attributes.uv_records.len()
        + source.attributes.corner_normals.len();
    if corners.saturating_mul(3).saturating_mul(maps) > 8_000_000 {
        return Err("Spike exceeds the 8,000,000 generated corner-attribute budget".into());
    }
    // Sparse non-UV maps have no admitted interpolation law. Refuse before
    // building a candidate instead of silently filling or discarding samples.
    for &fi in &selected {
        let f = &source.faces[fi as usize];
        for map in source.attributes.relative_morphs.values() {
            if !f.vertices.iter().all(|v| map.contains_key(v)) {
                // Spec: SA-Q03
                return Err("Spike can't handle partly assigned morph maps yet".into());
            }
        }
        for map in source.attributes.point_weights.values() {
            if !f.vertices.iter().all(|v| map.contains_key(v)) {
                // Spec: SA-Q03
                return Err("Spike can't handle partly assigned weight maps yet".into());
            }
        }
        for map in source.attributes.point_colors.values() {
            if !f.vertices.iter().all(|v| map.contains_key(v)) {
                // Spec: SA-Q03
                return Err("Spike can't handle partly assigned color maps yet".into());
            }
        }
        for map in source.attributes.corner_normals.values() {
            if map[fi as usize].iter().any(Option::is_none) {
                // Spec: SA-Q03
                return Err("Spike can't handle partly assigned normal maps yet".into());
            }
        }
    }
    let mut out = source.clone();
    out.faces.clear();
    for map in out.uv_maps.values_mut() {
        map.clear();
    }
    out.attributes.representations.clear();
    out.attributes.smoothing_labels.clear();
    out.attributes.degenerate_faces.clear();
    for map in out.attributes.uv_records.values_mut() {
        map.clear();
    }
    for map in out.attributes.corner_normals.values_mut() {
        map.clear();
    }
    let mut next_records = std::collections::BTreeMap::new();
    for (name, records) in &source.attributes.uv_records {
        let next = records
            .iter()
            .flatten()
            .flatten()
            .max()
            .copied()
            .map(|id| {
                id.checked_add(1)
                    .ok_or("Spike UV identity budget exhausted")
            })
            .transpose()?
            .unwrap_or(0);
        next_records.insert(name.clone(), next);
    }
    let mut descendants = vec![Vec::new(); source.faces.len()];
    for (fi, face) in source.faces.iter().enumerate() {
        let n = face.vertices.len();
        let apex = if selected.contains(&(fi as u32)) {
            let p0 = source.vertices[face.vertices[0] as usize];
            let p1 = source.vertices[face.vertices[1] as usize];
            let last = source.vertices[face.vertices[n - 1] as usize];
            let cross = (p1 - p0).cross(last - p0);
            let length = cross.length();
            let normal = if length == 0. {
                DVec3::ZERO
            } else {
                cross / length
            };
            let center = source.center(fi);
            let mean_edge = (0..n)
                .map(|i| {
                    source.vertices[face.vertices[i] as usize]
                        .distance(source.vertices[face.vertices[(i + 1) % n] as usize])
                        / n as f64
                })
                .sum::<f64>();
            let point = if length == 0. {
                center
            } else {
                center + normal * (strength * face_weight(fi) * mean_edge)
            };
            if !point.is_finite() || point.abs().max_element() > 1e9 {
                return Err("Spike apex exceeds finite coordinate limits".into());
            }
            let id = out.vertices.len() as u32;
            out.vertices.push(point);
            // SA-007: new relative morph points evaluate to zero, never the
            // arithmetic mean used by weights/colors. Store zero explicitly.
            for map in out.attributes.relative_morphs.values_mut() {
                map.insert(id, DVec3::ZERO);
            }

            for (name, map) in &source.attributes.point_weights {
                let value = face.vertices.iter().map(|v| map[v] / n as f64).sum();
                out.attributes
                    .point_weights
                    .get_mut(name)
                    .unwrap()
                    .insert(id, value);
            }
            for (name, map) in &source.attributes.point_colors {
                let value = face.vertices.iter().map(|v| map[v] / n as f64).sum();
                out.attributes
                    .point_colors
                    .get_mut(name)
                    .unwrap()
                    .insert(id, value);
            }
            Some(id)
        } else {
            None
        };
        let apex_uv_ids: std::collections::BTreeMap<_, _> = source
            .attributes
            .uv_records
            .keys()
            .map(|name| {
                let assigned = source.uv_maps[name][fi].iter().all(Option::is_some);
                let id = if apex.is_some() && assigned {
                    let next = next_records.get_mut(name).unwrap();
                    let id = *next;
                    *next = next
                        .checked_add(1)
                        .ok_or("Spike UV identity budget exhausted")?;
                    Some(id)
                } else {
                    None
                };
                Ok((name.clone(), id))
            })
            .collect::<Result<_>>()?;
        // Compute each apex assignment once per source face, not per triangle.
        let uv_means: std::collections::BTreeMap<_, _> = source
            .uv_maps
            .iter()
            .map(|(name, map)| {
                (
                    name,
                    map[fi]
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(|v| v.into_iter().sum::<DVec2>() / n as f64),
                )
            })
            .collect();
        let normal_means: std::collections::BTreeMap<_, _> = source
            .attributes
            .corner_normals
            .iter()
            .map(|(name, map)| {
                (
                    name,
                    map[fi]
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(|v| v.into_iter().sum::<DVec3>() / n as f64),
                )
            })
            .collect();
        let legacy_mean = face
            .uv
            .as_ref()
            .map(|uv| uv.iter().copied().sum::<DVec2>() / n as f64);
        for i in 0..if apex.is_some() { n } else { 1 } {
            let j = (i + 1) % n;
            let mut result = face.clone();
            if let Some(a) = apex {
                result.vertices = vec![face.vertices[i], face.vertices[j], a];
                result.uv = face
                    .uv
                    .as_ref()
                    .map(|uv| vec![uv[i], uv[j], legacy_mean.unwrap()]);
            }
            let id = out.faces.len() as u32;
            descendants[fi].push(id);
            out.faces.push(result);
            if !source.attributes.smoothing_labels.is_empty() {
                out.attributes
                    .smoothing_labels
                    .push(source.attributes.smoothing_labels[fi].clone());
            }

            if !source.attributes.representations.is_empty() {
                out.attributes
                    .representations
                    .push(source.attributes.representations[fi]);
            }
            for (name, map) in &source.uv_maps {
                let values = if apex.is_some() {
                    vec![map[fi][i], map[fi][j], uv_means[name]]
                } else {
                    map[fi].clone()
                };
                out.uv_maps.get_mut(name).unwrap().push(values);
            }
            for (name, map) in &source.attributes.uv_records {
                out.attributes
                    .uv_records
                    .get_mut(name)
                    .unwrap()
                    .push(if apex.is_some() {
                        vec![map[fi][i], map[fi][j], apex_uv_ids[name]]
                    } else {
                        map[fi].clone()
                    });
            }
            for (name, map) in &source.attributes.corner_normals {
                let values = if apex.is_some() {
                    vec![map[fi][i], map[fi][j], normal_means[name]]
                } else {
                    map[fi].clone()
                };
                out.attributes
                    .corner_normals
                    .get_mut(name)
                    .unwrap()
                    .push(values);
            }
            if (apex.is_none() && source.attributes.degenerate_faces.contains(&(fi as u32)))
                || out.normal(id as usize) == DVec3::ZERO
            {
                out.attributes.degenerate_faces.insert(id);
            }
        }
    }
    // Unselected exceptional faces have a single explicit descendant; remap
    // their ledger face IDs instead of treating vector offsets as stable IDs.
    for records in out.attributes.extrude_edge_uses.values_mut() {
        for record in records {
            let mapped = &descendants[record[0] as usize];
            if mapped.len() != 1 {
                return Err("Cannot preserve exceptional incidence through Spike".into());
            }
            record[0] = mapped[0];
        }
    }
    out.validate()?;
    Ok(SpikeResult {
        mesh: out,
        descendants,
    })
}
