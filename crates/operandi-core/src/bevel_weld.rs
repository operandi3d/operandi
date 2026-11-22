//! Planar Bevel contour contacts. This operates on one source region at a time;
//! no search includes another island, an unselected face or a source-skin point.
use crate::topology::edge_key;
use crate::{Mesh, Result, polygon::Region};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct WeldPlan {
    pub inset: BTreeMap<u32, DVec3>,
    pub root: BTreeMap<u32, u32>,
    pub uv_owner: BTreeMap<u32, u32>,
    pub metric_uv: BTreeSet<u32>,
}
fn intersection(a: DVec3, da: DVec3, b: DVec3, db: DVec3, n: DVec3) -> Option<(f64, f64)> {
    let det = da.cross(db).dot(n);
    if det.abs() <= 1e-12 * da.length() * db.length() {
        return None;
    }
    let q = b - a;
    let t = q.cross(db).dot(n) / det;
    let u = q.cross(da).dot(n) / det;
    (t.is_finite() && u.is_finite()).then_some((t, u))
}

pub(crate) fn plan(
    source: &Mesh,
    region: &Region,
    moves: &BTreeMap<u32, (DVec3, DVec3)>,
    incoming: &BTreeMap<u32, DVec3>,
    range: f64,
    even: bool,
    rail: bool,
) -> Result<WeldPlan> {
    let n = source.normal(region.faces[0]);
    let origin = source.vertices[*region.vertices.first().ok_or("Empty weld region")? as usize];
    let span = region
        .vertices
        .iter()
        .map(|&v| source.vertices[v as usize].distance(origin))
        .fold(1., f64::max);
    let eps = span * 1e-9;
    let shift = moves[region.vertices.first().unwrap()].0;
    if !range.is_finite() || range < 0. {
        return Err("Weld Range requires a finite nonnegative distance".into());
    }
    if region.vertices.iter().any(|&v| {
        let (s, i) = moves[&v];
        (source.vertices[v as usize] - origin).dot(n).abs() > eps
            || (s - shift).dot(n).abs() > eps
            || i.dot(n).abs() > eps
    }) {
        return Err(
            "Auto Weld requires a planar region and coplanar displaced contour; source retained"
                .into(),
        );
    }
    let boundary: BTreeSet<_> = region.boundary.iter().flat_map(|e| [e.a, e.b]).collect();
    if boundary.len().saturating_mul(boundary.len()) > 16_000_000 {
        return Err("Planar weld exceeds 16 million contact-pair budget".into());
    }
    let edges: BTreeSet<_> = region.boundary.iter().map(|e| edge_key(e.a, e.b)).collect();
    let base = |v: u32| source.vertices[v as usize] + moves[&v].0;
    let carrier = region
        .boundary
        .iter()
        .map(|e| base(e.a).distance(base(e.b)))
        .fold(f64::INFINITY, f64::min);
    if !carrier.is_finite() || carrier <= eps {
        return Err("Auto Weld has a collapsed source contour".into());
    }
    // Mutual nearest source contacts avoid transitive proximity chains. Source
    // corner order breaks equal-distance identity ties; geometry is not averaged.
    let mut rank = BTreeMap::new();
    for &f in &region.faces {
        for &v in &source.faces[f].vertices {
            let next = rank.len();
            rank.entry(v).or_insert(next);
        }
    }
    let mut nearest = BTreeMap::new();
    for &a in &boundary {
        if let Some(b) = boundary
            .iter()
            .copied()
            .filter(|&b| b != a && base(a).distance(base(b)) < range - eps)
            .min_by(|&b, &c| {
                base(a)
                    .distance_squared(base(b))
                    .total_cmp(&base(a).distance_squared(base(c)))
                    .then(rank[&b].cmp(&rank[&c]))
            })
        {
            nearest.insert(a, b);
        }
    }
    let pairs: Vec<_> = nearest
        .iter()
        .filter_map(|(&a, &b)| {
            (rank[&a] < rank[&b] && nearest.get(&b) == Some(&a)).then_some((a, b))
        })
        .collect();
    let active: BTreeSet<_> = pairs.iter().flat_map(|&(a, b)| [a, b]).collect();
    let mut inset: BTreeMap<_, _> = moves.iter().map(|(&v, &(_, i))| (v, i)).collect();
    let clearance = (range - shift.length()).max(0.) / carrier;
    if !rail {
        for &(a, b) in &pairs {
            let adjacent = edges.contains(&edge_key(a, b));
            let d = (base(b) - base(a)).normalize();
            let axis = if adjacent { n.cross(d) } else { d };
            let factor = if adjacent {
                1. + clearance
            } else {
                1. - clearance
            };
            for v in [a, b] {
                let old = inset[&v];
                if old.length_squared() < 1e-24 {
                    continue;
                }
                let raw = old + axis * (old.dot(axis) * (factor - 1.));
                let corrected = if even {
                    let side = incoming[&v];
                    let divisor = raw.dot(side);
                    if divisor.abs() < 1e-12 * raw.length() {
                        return Err(
                            "Weld Range gives a singular contour direction; source retained".into(),
                        );
                    }
                    raw * (old.dot(side) / divisor)
                } else {
                    if raw.length_squared() < 1e-24 {
                        return Err(
                            "Weld Range cancels a contour direction; source retained".into()
                        );
                    }
                    raw.normalize() * old.length()
                };
                inset.insert(v, corrected);
            }
        }
    }
    let mut limits: BTreeMap<_, _> = boundary.iter().map(|&v| (v, 1_f64)).collect();
    let mut stop = 1_f64;
    let mut metric_pairs = Vec::new();
    let mut contacts = BTreeSet::new();
    for &(a, b) in &edges {
        if let Some((t, u)) = intersection(base(a), inset[&a], base(b), inset[&b], n)
            && t > 1e-10
            && u > 1e-10
        {
            contacts.insert(edge_key(a, b));
            stop = stop.min(t.max(u));
            limits.insert(a, limits[&a].min(t));
            limits.insert(b, limits[&b].min(u));
        }
    }
    for &(a, b) in &pairs {
        if edges.contains(&edge_key(a, b)) {
            continue;
        }
        if let Some((t, u)) = intersection(base(a), inset[&a], base(b), inset[&b], n) {
            if t > 1e-10 && u > 1e-10 {
                limits.insert(a, limits[&a].min(t));
                limits.insert(b, limits[&b].min(u));
                metric_pairs.push((a, b));
                contacts.insert(edge_key(a, b));
            } else {
                // An intersection behind a source corner cannot reverse its
                // motion. Limit the forward ray at its partner's support line.
                let d = (base(b) - base(a)).normalize();
                for (v, other) in [(a, b), (b, a)] {
                    let denominator = inset[&v].dot(d);
                    if denominator.abs() > 1e-20 {
                        let t = (base(other) - base(v)).dot(d) / denominator;
                        if t > 1e-10 {
                            limits.insert(v, limits[&v].min(t));
                        }
                    }
                }
            }
        }
    }
    // Range contacts constrain ray travel, not a global merge radius. A contact
    // must lie forward on both rays and within each corner's repair reach.
    let vs: Vec<_> = boundary.iter().copied().collect();
    for (index, &a) in vs.iter().enumerate() {
        for &b in &vs[index + 1..] {
            if edges.contains(&edge_key(a, b)) || !active.contains(&a) || !active.contains(&b) {
                continue;
            }
            if let Some((t, u)) = intersection(base(a), inset[&a], base(b), inset[&b], n)
                && t > 1e-10
                && u > 1e-10
                && (inset[&a] * t).length() <= range + eps
                && (inset[&b] * u).length() <= range + eps
            {
                contacts.insert(edge_key(a, b));
                limits.insert(a, limits[&a].min(t));
                limits.insert(b, limits[&b].min(u));
            }
        }
    }
    for (&v, i) in &mut inset {
        *i *= limits.get(&v).copied().unwrap_or(1.).min(stop);
    }
    let mut root: BTreeMap<_, _> = region.vertices.iter().map(|&v| (v, v)).collect();
    let mut uv_owner = root.clone();
    let mut ordered = vs;
    ordered.sort_by_key(|v| rank[v]);
    for (index, &a) in ordered.iter().enumerate() {
        let r = root[&a];
        for &b in &ordered[index + 1..] {
            if contacts.contains(&edge_key(a, b))
                && (base(a) + inset[&a]).distance(base(b) + inset[&b]) <= span * 1e-7
            {
                let old = root[&b];
                for value in root.values_mut() {
                    if *value == old {
                        *value = r;
                    }
                }
                let last = if rank[&uv_owner[&r]] > rank[&uv_owner[&old]] {
                    uv_owner[&r]
                } else {
                    uv_owner[&old]
                };
                uv_owner.insert(r, last);
            }
        }
    }
    let mut metric_uv = BTreeSet::new();
    for (a, b) in metric_pairs {
        if root[&a] == root[&b] {
            metric_uv.insert(root[&a]);
        }
    }
    let uv_owner = root.iter().map(|(&v, &r)| (v, uv_owner[&r])).collect();
    Ok(WeldPlan {
        inset,
        root,
        uv_owner,
        metric_uv,
    })
}

/// Keep an oriented corner subsequence. Cap repair also removes a zero-width
/// excursion A-B-A; wall corners only collapse adjacent equal endpoints.
pub(crate) fn surviving_corners(vertices: &[u32], prune_spurs: bool) -> Vec<usize> {
    let mut keep: Vec<_> = (0..vertices.len()).collect();
    loop {
        let count = keep.len();
        if count < 2 {
            break;
        }
        if let Some(i) = (0..count).find(|&i| vertices[keep[i]] == vertices[keep[(i + 1) % count]])
        {
            let remove = if i + 1 == count { i } else { i + 1 };
            keep.remove(remove);
            continue;
        }
        if prune_spurs
            && count >= 3
            && let Some(i) =
                (0..count).find(|&i| vertices[keep[i]] == vertices[keep[(i + 2) % count]])
        {
            let a = (i + 1) % count;
            let b = (i + 2) % count;
            keep.remove(a.max(b));
            keep.remove(a.min(b));
            continue;
        }
        break;
    }
    keep
}

pub(crate) fn validate_row(
    source: &Mesh,
    region: &Region,
    positions: &BTreeMap<u32, DVec3>,
    roots: &BTreeMap<u32, u32>,
) -> Result<()> {
    let mut indices = BTreeMap::new();
    let mut vertices = Vec::new();
    for &v in &region.vertices {
        let root = roots[&v];
        indices.entry(root).or_insert_with(|| {
            let id = vertices.len() as u32;
            vertices.push(positions[&root]);
            id
        });
    }
    let mut faces = Vec::new();
    let mut kept = Vec::new();
    for &f in &region.faces {
        let mut face = source.faces[f].clone();
        let mapped: Vec<_> = face.vertices.iter().map(|v| indices[&roots[v]]).collect();
        face.vertices = surviving_corners(&mapped, true)
            .into_iter()
            .map(|c| mapped[c])
            .collect();
        // PB-C-02/03: a cap welded down to fewer than three corners is
        // removed by the bevel, so it has nothing left to validate.
        if face.vertices.len() < 3 {
            continue;
        }
        face.uv = None;
        faces.push(face);
        kept.push(f);
    }
    let mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    mesh.validate()?;
    for (f, &sf) in kept.iter().enumerate() {
        for [a, b, c] in mesh.surface_triangles(f)? {
            let vs = &mesh.faces[f].vertices;
            let [a, b, c] = [a, b, c].map(|c| mesh.vertices[vs[c] as usize]);
            if (b - a).cross(c - a).dot(source.normal(sf)) <= 0. {
                return Err("Auto Weld inverts a cap triangle; source retained".into());
            }
        }
    }
    let selected = (0..mesh.faces.len() as u32).collect::<Vec<_>>();
    if selected.is_empty() {
        return Ok(());
    }
    for region in mesh.polygon_regions(&selected, true)? {
        let normal = mesh.normal(region.faces[0]);
        let cap = region
            .vertices
            .iter()
            .map(|&v| (v, mesh.vertices[v as usize]))
            .collect();
        let origin = mesh.vertices[*region.vertices.first().unwrap() as usize];
        let span = region
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize].distance(origin))
            .fold(1., f64::max);
        crate::polygon::validate_cap_edges(&mesh, &region, &cap, normal, span)?;
    }
    Ok(())
}

/// Existing complete-map support does not license merging a sparse record or
/// a conflicting seam into an arbitrary winner. Original records stay untouched.
pub(crate) fn validate_survivors(
    source: &Mesh,
    region: &Region,
    roots: &BTreeMap<u32, u32>,
) -> Result<()> {
    let merged: BTreeSet<_> = roots
        .iter()
        .filter_map(|(&v, &r)| (v != r).then_some(r))
        .collect();
    for root in merged {
        let members: Vec<_> = roots
            .iter()
            .filter_map(|(&v, &r)| (r == root).then_some(v))
            .collect();
        let corners = |v: u32| {
            region.faces.iter().flat_map(move |&f| {
                source.faces[f]
                    .vertices
                    .iter()
                    .enumerate()
                    .filter_map(move |(c, &x)| (x == v).then_some((f, c)))
            })
        };
        for (name, map) in &source.uv_maps {
            let point = source.attributes.point_uvs.get(name);
            let mut presence = Vec::new();
            for &v in &members {
                let values: Vec<_> = corners(v)
                    .map(|(f, c)| map[f][c].or_else(|| point.and_then(|m| m.get(&v).copied())))
                    .collect();
                if values.windows(2).any(|p| p[0] != p[1]) {
                    return Err(format!(
                        // Spec: BV-Q04
                        "Bevel can't merge conflicting UV seams in {name}; nothing changed"
                    ));
                }
                presence.extend(values.into_iter().map(|v| v.is_some()));
            }
            if presence.iter().any(|&p| p) && presence.iter().any(|&p| !p) {
                return Err(format!(
                    // Spec: BV-Q04
                    "Bevel can't merge partly assigned UVs in {name}; nothing changed"
                ));
            }
        }
        macro_rules! corner_maps {
            ($field:ident) => {
                for (name, map) in &source.attributes.$field {
                    let mut presence = Vec::new();
                    for &v in &members {
                        let values: Vec<_> = corners(v).map(|(f, c)| map[f][c]).collect();
                        if values.windows(2).any(|p| p[0] != p[1]) {
                            return Err(format!(
                                // Spec: BV-Q04
                                "Bevel can't merge conflicting {} seams in {name}; nothing changed",
                                stringify!($field)
                            ));
                        }
                        presence.extend(values.into_iter().map(|v| v.is_some()));
                    }
                    if presence.iter().any(|&p| p) && presence.iter().any(|&p| !p) {
                        return Err(format!(
                            "Bevel can't merge partly assigned {} in {name}; nothing changed" /* Spec: BV-Q04 */,
                            stringify!($field)
                        ));
                    }
                }
            };
        }
        corner_maps!(corner_weights);
        corner_maps!(corner_colors);
        corner_maps!(corner_normals);
        corner_maps!(uv_records);
        macro_rules! point_maps {
            ($field:ident) => {
                for (name, map) in &source.attributes.$field {
                    let present = members.iter().filter(|v| map.contains_key(v)).count();
                    if present != 0 && present != members.len() {
                        return Err(format!(
                            "Bevel can't merge partly assigned {} in {name}; nothing changed" /* Spec: BV-Q04 */,
                            stringify!($field)
                        ));
                    }
                }
            };
        }
        point_maps!(point_weights);
        point_maps!(point_colors);
        point_maps!(point_uvs);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;
    use glam::DVec2;

    fn quad() -> (Mesh, Region) {
        let mesh = Mesh {
            vertices: vec![DVec3::ZERO, DVec3::X, DVec3::new(1., 1., 0.), DVec3::Y],
            faces: vec![Face {
                vertices: vec![0, 1, 2, 3],
                uv: None,
                material: String::new(),
                smooth: false,
                part: None,
                material_label: None,
            }]
            .into(),
            ..Default::default()
        };
        let region = mesh.polygon_regions(&[0], true).unwrap().remove(0);
        (mesh, region)
    }

    fn contact_plan(delta: f64) -> WeldPlan {
        let (mesh, region) = quad();
        let moves = [
            (0, (DVec3::ZERO, DVec3::new(0.5, 0.5, 0.))),
            (1, (DVec3::ZERO, DVec3::new(-0.5, 0.5 + delta, 0.))),
            (2, (DVec3::ZERO, DVec3::ZERO)),
            (3, (DVec3::ZERO, DVec3::ZERO)),
        ]
        .into_iter()
        .collect();
        plan(&mesh, &region, &moves, &BTreeMap::new(), 0., false, true).unwrap()
    }

    #[test]
    fn exact_contact_welds_and_selects_last_uv_owner() {
        let result = contact_plan(0.);
        assert_eq!(result.root[&0], result.root[&1]);
        assert_eq!(result.uv_owner[&0], 1);
        assert_eq!(result.uv_owner[&1], 1);
        assert!(result.metric_uv.is_empty());
        assert_eq!(result.inset[&0], DVec3::new(0.5, 0.5, 0.));
    }

    #[test]
    fn contact_tolerance_distinguishes_near_and_separate_endpoints() {
        let near = contact_plan(1e-8);
        assert_eq!(near.root[&0], near.root[&1]);
        let outside = contact_plan(1e-6);
        assert_ne!(outside.root[&0], outside.root[&1]);
    }

    #[test]
    fn collapsed_cap_is_removed_and_valid_remaining_cap_is_checked() {
        let (mesh, region) = quad();
        let positions = mesh
            .vertices
            .iter()
            .enumerate()
            .map(|(i, &p)| (i as u32, p))
            .collect();
        let triangle = [(0, 0), (1, 0), (2, 2), (3, 3)].into_iter().collect();
        assert!(validate_row(&mesh, &region, &positions, &triangle).is_ok());
        let collapsed = [(0, 0), (1, 0), (2, 0), (3, 3)].into_iter().collect();
        assert!(validate_row(&mesh, &region, &positions, &collapsed).is_ok());
        let inverted_positions = [
            (0, DVec3::ZERO),
            (1, DVec3::X),
            (2, DVec3::new(1., -1., 0.)),
            (3, -DVec3::Y),
        ]
        .into_iter()
        .collect();
        let unchanged_roots = (0..4).map(|v| (v, v)).collect();
        assert!(validate_row(&mesh, &region, &inverted_positions, &unchanged_roots).is_err());
    }

    #[test]
    fn corner_repair_preserves_original_corner_indices() {
        assert_eq!(surviving_corners(&[0, 0, 1, 2], true), vec![0, 2, 3]);
        assert_eq!(surviving_corners(&[0, 1, 0, 2, 3], true), vec![0, 3, 4]);
        assert_eq!(
            surviving_corners(&[0, 1, 0, 2, 3], false),
            vec![0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn survivor_validation_keeps_complete_uvs_and_selection_sets_unchanged() {
        let (mut mesh, region) = quad();
        let uvs = vec![
            Some(DVec2::ZERO),
            Some(DVec2::X),
            Some(DVec2::ONE),
            Some(DVec2::Y),
        ];
        mesh.uv_maps.insert("UV".into(), vec![uvs]);
        mesh.attributes
            .vertex_sets
            .insert("Points".into(), [0, 1].into_iter().collect());
        mesh.attributes
            .polygon_sets
            .insert("Faces".into(), [0].into_iter().collect());
        let before = mesh.clone();
        let roots = [(0, 0), (1, 0), (2, 2), (3, 3)].into_iter().collect();
        assert!(validate_survivors(&mesh, &region, &roots).is_ok());
        assert_eq!(mesh, before);
    }

    #[test]
    fn survivor_validation_rejects_partly_assigned_uvs_without_changes() {
        let (mut mesh, region) = quad();
        mesh.uv_maps
            .insert("UV".into(), vec![vec![Some(DVec2::ZERO), None, None, None]]);
        let before = mesh.clone();
        let roots = [(0, 0), (1, 0), (2, 2), (3, 3)].into_iter().collect();
        assert!(
            validate_survivors(&mesh, &region, &roots)
                .unwrap_err()
                .contains("partly assigned UVs")
        );
        assert_eq!(mesh, before);
    }
}
