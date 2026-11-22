//! Shared UV helpers for tools that add points (Edge Bevel/Chamfer/Extrude,
//! Vertex Bevel, Grow Quads).
//!
//! Provisional rule (Jason, 2026-11-01; the edge specs don't cover UVs): a
//! new point's UV is interpolated inside each polygon that uses it, from that
//! polygon's own corner UVs, so UV seams between polygons stay intact.
use crate::{Face, Mesh};
use glam::DVec2;
use std::collections::BTreeMap;

/// Per-polygon corner UVs of one map.
pub(crate) type UvRows = Vec<Vec<Option<DVec2>>>;

/// True when the mesh has per-corner data these tools still can't carry:
/// legacy per-polygon UVs, corner weights/colors/normals or typed corner
/// maps. UV maps and their records are handled.
pub(crate) fn has_uncarried_corner_data(mesh: &Mesh) -> bool {
    let at = &mesh.attributes;
    mesh.faces.iter().any(|f| f.uv.is_some())
        || at
            .corner_weights
            .values()
            .flatten()
            .flatten()
            .any(Option::is_some)
        || at
            .corner_colors
            .values()
            .flatten()
            .flatten()
            .any(Option::is_some)
        || at
            .corner_normals
            .values()
            .flatten()
            .flatten()
            .any(Option::is_some)
        || at
            .typed_maps
            .values()
            .any(|m| m.corners.iter().flatten().any(Option::is_some))
}

/// The UV polygon `face` gives point `v` in `rows`, if it uses `v`.
pub(crate) fn corner_uv(mesh: &Mesh, rows: &UvRows, face: usize, v: u32) -> Option<DVec2> {
    let corner = mesh
        .faces
        .get(face)?
        .vertices
        .iter()
        .position(|&x| x == v)?;
    *rows.get(face)?.get(corner)?
}

/// SQ-UV-01 at seams (new-point-uvs UVN-T-01, T-04, T-06): a point's
/// continuous UV, the value it has where no polygon overrides it. A stored
/// point UV wins; a mesh that keeps only corner UVs has none, so the value in
/// the lowest-numbered polygon of `around` stands in (provisional,
/// SPECIFICATION_QUESTIONS "New-point UVs at seams"). `around` lists
/// `(polygon, the point's UV in it)`.
pub(crate) fn continuous_uv(
    points: Option<&BTreeMap<u32, DVec2>>,
    v: u32,
    around: impl IntoIterator<Item = (usize, Option<DVec2>)>,
) -> Option<DVec2> {
    if let Some(&uv) = points.and_then(|m| m.get(&v)) {
        return Some(uv);
    }
    around
        .into_iter()
        .filter_map(|(f, uv)| Some((f, uv?)))
        .min_by_key(|&(f, _)| f)
        .map(|(_, uv)| uv)
}

/// Whether `faces` lie in one plane. The seam rules above were measured on
/// a flat grid; at a fold (a cube's edge) the tools keep interpolating in
/// each polygon (provisional).
pub(crate) fn flat(mesh: &Mesh, faces: impl IntoIterator<Item = usize>) -> bool {
    let mut first = None;
    faces.into_iter().all(|f| {
        let n = mesh.normal(f).normalize_or_zero();
        let reference = *first.get_or_insert(n);
        n.dot(reference) > 1. - 1e-9
    })
}

/// Whether a polygon's UV at a point is the point's continuous UV (no seam
/// override there).
pub(crate) fn is_continuous(uv: Option<DVec2>, continuous: Option<DVec2>) -> bool {
    match (uv, continuous) {
        (Some(a), Some(b)) => a.distance(b) <= 1e-9,
        (a, b) => a.is_none() && b.is_none(),
    }
}

pub(crate) fn lerp(a: Option<DVec2>, b: Option<DVec2>, t: f64) -> Option<DVec2> {
    Some(a?.lerp(b?, t))
}

/// UV record ids rebuilt from coordinates: corners of one point with the
/// same UV share an id, as the records require.
pub(crate) fn records_from_uvs(faces: &[Face], uvs: &UvRows) -> Vec<Vec<Option<u32>>> {
    let mut ids = BTreeMap::<(u32, u64, u64), u32>::new();
    faces
        .iter()
        .zip(uvs)
        .map(|(f, row)| {
            f.vertices
                .iter()
                .zip(row)
                .map(|(&v, uv)| {
                    let uv = (*uv)?;
                    let next = ids.len() as u32;
                    Some(
                        *ids.entry((v, uv.x.to_bits(), uv.y.to_bits()))
                            .or_insert(next),
                    )
                })
                .collect()
        })
        .collect()
}
