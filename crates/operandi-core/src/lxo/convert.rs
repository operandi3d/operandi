use super::{
    decode::{Raw, Value},
    *,
};
use crate::{Face, Item, Mesh, RotationOrder, Transform};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

fn err(category: &'static str, detail: &str) -> Error {
    Error::new(category, 0, detail)
}
fn candidate_validation_error(e: crate::scene::SceneValidationError) -> Error {
    match e {
        crate::scene::SceneValidationError::FiniteTransformRange(detail) => {
            err("numeric_range_or_composition_error", &detail)
        }
        crate::scene::SceneValidationError::Other(detail) => err("invalid_candidate", &detail),
    }
}
#[cfg(test)]
mod validation_mapping_tests {
    use super::*;
    #[test]
    fn nonnumeric_candidate_error_is_not_remapped() {
        let error = candidate_validation_error(crate::scene::SceneValidationError::Other(
            "Invalid workplane".into(),
        ));
        assert_eq!(error.category, "invalid_candidate");
        assert_eq!(error.detail, "Invalid workplane");
    }
}
/// Detail for an out-of-range POLS index; `corner` is in file order.
fn point_reference(layer: &str, polygon: usize, corner: usize, point: u32, mesh: &Mesh) -> String {
    format!(
        "{layer}: polygon {polygon} corner {corner} references point {point}; layer has {} points",
        mesh.vertices.len()
    )
}
fn close(a: &Value, b: &Value) -> bool {
    match (a.number(), b.number()) {
        (Some(a), Some(b)) => (a - b).abs() <= 1e-6 + 1e-6 * a.abs().max(b.abs()),
        _ => a == b,
    }
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
struct Layer {
    id: u32,
    name: String,
    mesh: Mesh,
    bad: Vec<String>,
    /// Format 2.x–4.x: a nonzero LAYR pivot, kept as information only.
    pivot: Option<[f64; 3]>,
}

/// LXO layer pivots (answered by the spec side 2026-11-15): stored points
/// are absolute, so a nonzero LAYR pivot doesn't move them. The per-mesh
/// note for one, else `None`.
pub(super) fn layer_pivot_note(mesh: &str, pivot: [f64; 3]) -> Option<String> {
    // The file stores f32; show it as such.
    let [x, y, z] = pivot.map(|v| v as f32);
    (pivot != [0.0; 3]).then(|| {
        format!("Mesh '{mesh}': layer pivot ({x}, {y}, {z}) kept as information only, not applied")
    })
}
#[derive(Default)]
struct Counts {
    points: usize,
    polys: usize,
    corners: usize,
    maps: usize,
    /// SUBD/PSUB polygons imported as their control cage (LEG-06).
    subdivision: usize,
}
struct Map {
    kind: [u8; 4],
    name: String,
    domain: [u8; 4],
    dimension: u16,
    entries: Vec<(u32, Option<u32>, Vec<f64>)>,
}
/// LEG-04: the observed format 4.x LAYR "rest" bytes after the item id.
fn legacy_rest_allowed(rest: &[u8]) -> bool {
    let h: String = rest.iter().map(|b| format!("{b:02x}")).collect();
    match rest.len() {
        18 => [
            "001000000000000000010002000200020001",
            "001000000000000000010002000200050001",
            "001000000000000000010003000300030001",
        ]
        .contains(&h.as_str()),
        20 => [
            "0010000000000000000100020002000200010001",
            "0010000000000000000100040002000200010001",
            "0010000000000000000100020004000200010001",
            "0010000000000000000100020002000400010001",
        ]
        .contains(&h.as_str()),
        24 => [
            "001000000000000000010002000000000002000200020001",
            "001000000000000000000002000000000002000200020001",
        ]
        .contains(&h.as_str()),
        // Followed by a layer label string and settings, not interpreted.
        70 => h.starts_with("00100000000000000000000200000000000200020002000100000000"),
        _ => false,
    }
}
fn layer(
    legacy: bool,
    old: bool,
    chunks: &[Chunk<'_>],
    tags: &[String],
    counts: &mut Counts,
    report: &mut Report,
    c: &mut Control<'_>,
) -> Result<Layer> {
    let ch = chunks[0];
    let mut r = Reader::new(ch.data, ch.offset);
    r.u16()?;
    r.u16()?;
    let pivot = [r.f32()?, r.f32()?, r.f32()?];
    let name = r.name(c)?;
    let mut bad = Vec::new();
    let tail = if legacy {
        // LEG-04: same fields up to the item id; the rest varies. A far
        // longer record carries sculpt/multiresolution data.
        let tail = r.take(ch.data.len() - r.pos)?;
        if tail.len() < 50 {
            return Err(r.error("truncated_payload", "LAYR shorter than its fixed fields"));
        }
        if tail.len() > 50 + 70 {
            bad.push(format!("{name}: sculpt or multiresolution layer data"));
        } else if old {
            // LEG-OLD-02 (provisional): formats 2.x and 3.x write other
            // subdivision, curve and display settings after the item id.
            // They don't change the polygon cage Operandi imports, so they
            // aren't checked; the transform matrix still is (below).
            report.omit("layer subdivision/display settings not read (older format)");
        } else if !legacy_rest_allowed(&tail[50..])
            || !matches!(&tail[2..6], [0x40, 0, 0, 0] | [0x40, 0xa0, 0, 0])
        {
            bad.push(format!("{name}: unsupported layer settings profile"));
        }
        tail
    } else {
        let tail = r.take(72)?;
        r.end()?;
        tail
    };
    let id = u32::from_be_bytes(tail[46..50].try_into().unwrap());
    if pivot != [0.; 3] && !legacy {
        let [x, y, z] = pivot;
        bad.push(format!(
            "{name}: unsupported offset pivot (layer '{name}', pivot {x}, {y}, {z})"
        ));
    }
    if old {
        if tail[10..46]
            != hex("3f8000000000000000000000000000003f8000000000000000000000000000003f800000")
        {
            bad.push(format!("{name}: unsupported layer matrix"));
        }
    } else if legacy {
        if tail[6..10] != hex("3db2b8c2")
            || tail[10..46]
                != hex("3f8000000000000000000000000000003f8000000000000000000000000000003f800000")
        {
            bad.push(format!("{name}: unsupported layer settings profile"));
        }
    } else if tail[2..10] != hex("400000003db2b8c2")
        || tail[10..46]
            != hex("3f8000000000000000000000000000003f8000000000000000000000000000003f800000")
        || tail[50..] != hex("00100000000000000001000200020002000100010000")
    {
        bad.push(format!("{name}: unsupported layer settings profile"));
    }
    let mut mesh = Mesh::default();
    let mut points = false;
    let mut polys = false;
    let mut maps = Vec::new();
    let mut identities = BTreeSet::new();
    let mut ptag = Vec::new();
    // Format 4.x layers can hold several POLS blocks (one per polygon type);
    // PTAG and VMAD polygon indices count within the latest block. `None`
    // while the latest block was skipped (curves), so its records are too.
    let mut poly_base: Option<u32> = Some(0);
    // Legacy: stored polygon index (as PTAG/VMAD count them) to the face it
    // became; `None` for a polygon left out (fewer than 3 corners).
    let mut stored: Vec<Option<u32>> = Vec::new();
    let mut short = 0usize;
    let mut reps: Vec<crate::PolygonRepresentation> = Vec::new();
    for ch in &chunks[1..] {
        c.check("geometry decoding")?;
        let mut r = Reader::new(ch.data, ch.offset);
        match &ch.id {
            b"BBOX" => {
                r.take(24)?;
                r.end()?;
                report.omit("cached layer bounds");
            }
            b"VMPA" => {
                if ch.data != [0, 0, 0, 1, 0, 0, 0, 6] {
                    bad.push(format!("{name}: unsupported VMPA qualifier"));
                }
            }
            b"PNTS" => {
                if points {
                    return Err(r.error("unsupported_repeated_geometry_block", "Repeated PNTS"));
                }
                points = true;
                if ch.data.len() % 12 != 0 {
                    return Err(r.error(
                        "invalid_point_payload_size",
                        "PNTS must contain F32 triples",
                    ));
                }
                let n = ch.data.len() / 12;
                counts.points += n;
                if counts.points > 5_000_000 {
                    return Err(r.error("point_count_limit", "Global point budget"));
                }
                c.charge(n * 96)?;
                mesh.vertices
                    .try_reserve_exact(n)
                    .map_err(|_| r.error("allocation_failure", "Point allocation"))?;
                while !r.done() {
                    c.check("point decoding")?;
                    let mut coordinate = || {
                        r.f32().map_err(|mut e| {
                            if e.category == "nonfinite_modeling_value" {
                                e.category = "nonfinite_geometry";
                            }
                            e
                        })
                    };
                    mesh.vertices
                        .push(DVec3::new(coordinate()?, coordinate()?, -coordinate()?));
                }
            }
            b"POLS" => {
                if polys && !legacy {
                    return Err(r.error("unsupported_repeated_geometry_block", "Repeated POLS"));
                }
                polys = true;
                let kind = r.id()?;
                if legacy && !matches!(&kind, b"FACE" | b"SUBD" | b"PSUB") {
                    // Curves and other polygon types aren't imported yet.
                    report.omit(format!(
                        "{} polygon block(s) not imported (curves)",
                        String::from_utf8_lossy(&kind)
                    ));
                    poly_base = None;
                    continue;
                }
                poly_base = Some(if legacy {
                    stored.len() as u32
                } else {
                    mesh.faces.len() as u32
                });
                if !matches!(&kind, b"FACE" | b"SUBD" | b"PSUB") {
                    return Err(r.error(
                        "unsupported_face_type",
                        &format!(
                            "{} unparsed, declared payload {} bytes",
                            String::from_utf8_lossy(&kind),
                            ch.data.len()
                        ),
                    ));
                }
                // LEG-06 (Jason, 2026-11-03): SUBD and PSUB share FACE's
                // record layout and import as their control cage; the type is
                // kept per polygon for a later subdivision feature.
                let representation = match &kind {
                    b"SUBD" => Some(crate::PolygonRepresentation::Subdivs),
                    b"PSUB" => Some(crate::PolygonRepresentation::CatmullClark),
                    _ => None,
                };
                // LEG-05: format 4.x counts use 11 bits; the rest must be zero.
                let mask: u16 = if legacy { 2047 } else { 1023 };
                while !r.done() {
                    c.check("polygon decoding")?;
                    let word = r.u16()?;
                    let n = (word & mask) as usize;
                    if word & !mask != 0 {
                        return Err(r.error("unsupported_face_flags", "Nonzero polygon flags"));
                    }
                    if n < 3 && legacy {
                        // LEG-OLD-06 (provisional): one- and two-point
                        // polygons (points and lines) aren't faces; left
                        // out with a count, their tags and UVs with them.
                        for _ in 0..n {
                            r.vx()?;
                        }
                        stored.push(None);
                        short += 1;
                        continue;
                    }
                    if n < 3 {
                        return Err(r.error(
                            "unsupported_or_invalid_face",
                            "A polygon needs at least 3 corners",
                        ));
                    }
                    if representation.is_some() {
                        counts.subdivision += 1;
                    }
                    counts.polys += 1;
                    counts.corners += n;
                    if counts.polys > 5_000_000 || counts.corners > 20_000_000 {
                        return Err(r.error("resource_limit", "Polygon/corner budget"));
                    }
                    c.charge(256 + n * 96)?;
                    let mut vertices = Vec::new();
                    vertices
                        .try_reserve_exact(n)
                        .map_err(|_| r.error("allocation_failure", "Polygon allocation"))?;
                    for corner in 0..n {
                        let at = r.base + r.pos;
                        let point = r.vx()?;
                        // With PNTS already decoded, refuse at the index itself.
                        if points && point as usize >= mesh.vertices.len() {
                            return Err(Error::new(
                                "invalid_point_reference",
                                at,
                                &point_reference(&name, mesh.faces.len(), corner, point, &mesh),
                            ));
                        }
                        vertices.push(point);
                    }
                    vertices[1..].reverse();
                    if legacy {
                        stored.push(Some(mesh.faces.len() as u32));
                    }
                    reps.push(representation.unwrap_or_default());
                    mesh.faces.push(Face {
                        vertices,
                        uv: None,
                        material: String::new(),
                        smooth: false,
                        part: None,
                        material_label: None,
                    });
                }
            }
            b"VMAP" | b"VMAD" | b"VMED" => {
                let kind = r.id()?;
                let dimension = r.u16()?;
                let map_name = r.name(c)?;
                // Format 4.x: only UV maps are imported; any other map, an
                // odd UV map or a repeat is left out with a note, never the
                // mesh (weights, morphs, colors, selection sets, ...).
                if legacy
                    && (kind != *b"TXUV"
                        || dimension != 2
                        || ch.id == *b"VMED"
                        || (ch.id == *b"VMAD" && poly_base.is_none())
                        || !identities.insert((ch.id, kind, map_name.clone())))
                {
                    report.omit(format!(
                        "{} map not imported",
                        String::from_utf8_lossy(&kind).trim()
                    ));
                    continue;
                }
                if dimension > 4 {
                    return Err(r.error("unsupported_map_dimension", "Map dimension exceeds four"));
                }
                let expected = match &kind {
                    b"TXUV" => 2,
                    b"BSPL" | b"WGHT" | b"SUBV" => 1,
                    b"NORM" | b"RGB " | b"MORF" | b"SPOT" => 3,
                    b"RGBA" => 4,
                    b"PICK" | b"EPCK" => 0,
                    _ => {
                        return Err(r.error(
                            "unsupported_modeling_map",
                            &format!(
                                "{map_name}: unknown {} unparsed, declared payload {} bytes",
                                String::from_utf8_lossy(&kind),
                                ch.data.len()
                            ),
                        ));
                    }
                };
                if dimension != expected {
                    return Err(r.error("unsupported_map_dimension", &map_name));
                }
                if !legacy && !identities.insert((ch.id, kind, map_name.clone())) {
                    return Err(r.error("duplicate_map_ownership", "Repeated map block"));
                }
                let mut entries = Vec::new();
                let mut keys = BTreeSet::new();
                let mut repeated = false;
                while !r.done() {
                    c.check("map decoding")?;
                    c.record(true)?;
                    counts.maps += 1;
                    if counts.maps > 20_000_000 {
                        return Err(r.error("resource_limit", "Map entry budget"));
                    }
                    c.charge(512)?;
                    let a = r.vx()?;
                    let b = if ch.id == *b"VMAP" {
                        None
                    } else {
                        Some(r.vx()? + poly_base.unwrap_or(0))
                    };
                    let mut values = Vec::new();
                    for _ in 0..dimension {
                        values.push(r.f32()?);
                    }
                    if !keys.insert((a, b)) {
                        // Legacy: a UV map giving one corner two values is
                        // left out whole, not the mesh.
                        if legacy {
                            repeated = true;
                            continue;
                        }
                        return Err(r.error("duplicate_map_ownership", &map_name));
                    }
                    entries.push((a, b, values));
                }
                if repeated {
                    report.omit(format!(
                        "{name}: UV map '{map_name}' with repeated entries not imported"
                    ));
                    continue;
                }
                maps.push(Map {
                    kind,
                    name: map_name,
                    domain: ch.id,
                    dimension,
                    entries,
                });
            }
            b"PTAG" => {
                let kind = r.id()?;
                if legacy && (!matches!(&kind, b"MATR" | b"PART") || poly_base.is_none()) {
                    report.omit(format!(
                        "{} polygon tags not imported",
                        String::from_utf8_lossy(&kind)
                    ));
                    continue;
                }
                if !matches!(&kind, b"MATR" | b"PART") {
                    return Err(r.error(
                        "unsupported_polygon_membership",
                        "Unknown PTAG classification",
                    ));
                }
                while !r.done() {
                    c.check("tag decoding")?;
                    c.record(true)?;
                    c.charge(128)?;
                    ptag.push((kind, r.vx()? + poly_base.unwrap_or(0), r.u16()?));
                }
            }
            _ if legacy => report.omit(format!(
                "{} layer chunk not imported",
                String::from_utf8_lossy(&ch.id)
            )),
            _ => return Err(r.error("unknown_dependencies", "Unknown geometry chunk")),
        }
    }
    if legacy {
        // Stored polygon indices to face indices. An index past the last
        // stored polygon stays out of range, so it is still caught below.
        let face = |i: u32| match stored.get(i as usize) {
            Some(Some(f)) => Some(*f),
            Some(None) => None,
            None => Some(u32::MAX),
        };
        ptag = ptag
            .into_iter()
            .filter_map(|(kind, i, tag)| face(i).map(|f| (kind, f, tag)))
            .collect();
        for map in &mut maps {
            if map.domain == *b"VMAD" {
                map.entries = std::mem::take(&mut map.entries)
                    .into_iter()
                    .filter_map(|(a, b, v)| match b {
                        Some(i) => face(i).map(|f| (a, Some(f), v)),
                        None => Some((a, b, v)),
                    })
                    .collect();
            }
        }
        if short > 0 {
            report.omit(format!(
                "{name}: {short} one- or two-point polygon(s) not imported"
            ));
        }
    }
    if reps
        .iter()
        .any(|r| *r != crate::PolygonRepresentation::Face)
    {
        mesh.attributes.representations = reps;
    }
    // Format 4.x: points without polygons are kept as they are.
    if points != polys && !(legacy && !polys) {
        return Err(err("missing_required_mesh_data", "PNTS/POLS pair missing"));
    }
    // POLS may precede PNTS in a layer; check those indices once points are
    // known. No byte location is kept for this path, so the offset is 0.
    for (polygon, face) in mesh.faces.iter().enumerate() {
        let n = face.vertices.len();
        for (stored, &point) in face.vertices.iter().enumerate() {
            if point as usize >= mesh.vertices.len() {
                // Stored loops keep corner 0 and reverse the rest.
                let corner = if stored == 0 { 0 } else { n - stored };
                return Err(err(
                    "invalid_point_reference",
                    &point_reference(&name, polygon, corner, point, &mesh),
                ));
            }
        }
    }
    // LEG-14: in format 4.x, bad geometry refuses this mesh only, with its
    // counts; nothing is repaired.
    if legacy && let Some(reason) = legacy_geometry_problems(&mesh) {
        bad.push(format!("Mesh '{name}' skipped: {reason}"));
        return Ok(Layer {
            id,
            name,
            mesh,
            bad,
            pivot: legacy.then_some(pivot),
        });
    }
    // A face with distinct corners but no area is degenerate geometry, not a
    // topology error. This is the mesh's own exact zero-normal test, no
    // tolerance; repeated corners stay with topology validation below.
    for (f, face) in mesh.faces.iter().enumerate() {
        let distinct = face.vertices.iter().collect::<BTreeSet<_>>().len();
        if distinct == face.vertices.len() && mesh.normal(f) == DVec3::ZERO {
            return Err(err(
                "degenerate_geometry",
                &format!("{name}: polygon {f} has zero area"),
            ));
        }
    }
    // Geometry validation precedes all corner indexing and publication.
    mesh.validate()
        .map_err(|e| err("invalid_or_unsupported_topology", &format!("{name}: {e}")))?;
    // Nonplanar polygons are ordinary in Modo meshes; check the same surface
    // triangulation drawing uses, which accepts them. In format 4.x Modo
    // also keeps polygons whose outline crosses itself or is very thin, so
    // Operandi keeps them too (drawn as a simple fan) and counts them; other
    // files still refuse them.
    let mut crossing = 0;
    for f in 0..mesh.faces.len() {
        if let Err(e) = mesh.surface_triangles(f) {
            if legacy {
                crossing += 1;
                continue;
            }
            return Err(err(
                "invalid_or_unsupported_topology",
                &format!("{name}: {e}"),
            ));
        }
    }
    if crossing > 0 {
        report.omit(format!(
            "{name}: {crossing} polygon(s) with a self-crossing or very thin outline kept as they are (drawn approximately)"
        ));
    }
    let map_names = maps
        .iter()
        .map(|m| (m.kind, m.name.as_str()))
        .collect::<BTreeSet<_>>();
    if map_names.len() > 64 {
        return Err(err("resource_limit", "64 maps per mesh"));
    }
    if legacy {
        // A map that refers to missing points or polygons is left out.
        let before = maps.len();
        maps.retain(|map| {
            map.entries.iter().all(|(a, b, _)| {
                (*a as usize) < mesh.vertices.len()
                    && b.is_none_or(|b| {
                        mesh.faces
                            .get(b as usize)
                            .is_some_and(|f| f.vertices.contains(a))
                    })
            })
        });
        for _ in maps.len()..before {
            report.omit(format!(
                "{name}: UV map with broken references not imported"
            ));
        }
    }
    for map in &maps {
        c.check("map reference resolution")?;
        for (a, b, _) in &map.entries {
            if *a as usize >= mesh.vertices.len() {
                return Err(err("invalid_map_reference", &map.name));
            }
            if let Some(b) = b {
                if map.domain == *b"VMAD" {
                    let face = mesh
                        .faces
                        .get(*b as usize)
                        .ok_or_else(|| err("invalid_map_reference", &map.name))?;
                    if !face.vertices.contains(a) {
                        return Err(err("point_not_on_polygon", &map.name));
                    }
                } else if *b as usize >= mesh.vertices.len() {
                    return Err(err("invalid_map_reference", &map.name));
                }
            }
        }
        if map.kind != *b"TXUV" {
            if map.kind == *b"BSPL" && map.name == "B-Spline" && map.entries.is_empty() {
                report.omit("unused empty BSPL/B-Spline declaration");
            } else if map.kind == *b"SUBV" {
                // LEG-06: subdivision weights and creases aren't stored yet.
                report.omit("subdivision weight/crease map not applied");
            } else {
                bad.push(format!(
                    "{name}: unsupported {} map '{}' dimension {} with {} entries",
                    String::from_utf8_lossy(&map.kind),
                    map.name,
                    map.dimension,
                    map.entries.len()
                ));
            }
        } else if map.domain == *b"VMED" {
            bad.push(format!("{name}: unsupported TXUV edge domain"));
        }
    }
    for map in &maps {
        if map.kind == *b"TXUV" && !mesh.uv_maps.contains_key(&map.name) {
            c.charge(mesh.faces.iter().map(|f| 64 + f.vertices.len() * 48).sum())?;
            mesh.uv_maps.insert(
                map.name.clone(),
                mesh.faces
                    .iter()
                    .map(|f| vec![None; f.vertices.len()])
                    .collect(),
            );
        }
    }
    // Continuous point values first, corner overrides second, independent of file order.
    for domain in [*b"VMAP", *b"VMAD"] {
        for map in &maps {
            if map.kind == *b"TXUV" && map.domain == domain {
                let values = mesh.uv_maps.get_mut(&map.name).unwrap();
                let keyed = map
                    .entries
                    .iter()
                    .map(|(a, b, v)| ((*a, *b), DVec2::new(v[0], v[1])))
                    .collect::<BTreeMap<_, _>>();
                for (fi, f) in mesh.faces.iter().enumerate() {
                    c.check("corner conversion")?;
                    for (ci, a) in f.vertices.iter().enumerate() {
                        let key = (
                            *a,
                            if domain == *b"VMAP" {
                                None
                            } else {
                                Some(fi as u32)
                            },
                        );
                        if let Some(uv) = keyed.get(&key) {
                            values[fi][ci] = Some(*uv);
                        }
                    }
                }
            }
        }
    }
    let mut assigned = BTreeSet::new();
    let mut broken_tags = 0;
    for (kind, face, tag) in ptag {
        // Format 4.x: a tag pointing nowhere, or a second tag for the same
        // polygon, is left out rather than refusing the mesh.
        if legacy
            && (tags.get(tag as usize).is_none()
                || face as usize >= mesh.faces.len()
                || assigned.contains(&(kind, face)))
        {
            broken_tags += 1;
            continue;
        }
        let label = tags.get(tag as usize).ok_or_else(|| {
            err(
                "invalid_tag_reference",
                "PTAG references nonexistent TAGS entry",
            )
        })?;
        let face_mut = mesh
            .faces
            .get_mut(face as usize)
            .ok_or_else(|| err("invalid_tag_reference", "PTAG polygon out of range"))?;
        if !assigned.insert((kind, face)) {
            return Err(err(
                "duplicate_polygon_membership",
                "Repeated polygon tag assignment",
            ));
        }
        if kind == *b"MATR" {
            face_mut.material = label.clone();
            face_mut.material_label = Some(label.clone());
        } else {
            face_mut.part = Some(label.clone());
        }
    }
    if broken_tags > 0 {
        report.omit(format!(
            "{name}: {broken_tags} broken or repeated polygon tag(s) not imported"
        ));
    }
    Ok(Layer {
        id,
        name,
        mesh,
        bad,
        pivot: legacy.then_some(pivot),
    })
}

/// `preset`: the file is a mesh preset (`LXPM`): layers with no mesh item
/// import as meshes of their own, and mesh items with no layer are left out.
pub fn convert(mut raw: Raw<'_>, c: &mut Control<'_>, preset: bool) -> Result<Candidate> {
    c.check("reference resolution")?;
    // Formats 2.x, 3.x and 4.x share the lenient reader.
    let legacy = raw.profile.starts_with("Legacy");
    let old = matches!(raw.profile.as_str(), "Legacy 2" | "Legacy 3");
    if legacy {
        apply_action_precedence(&mut raw);
    }
    let mut report = raw.report.clone();
    report.envelope_records = raw.envelopes.len();
    report.active_envelope_references = raw.envelope_references.len();
    report.static_envelope_fields = raw.static_envelope_fields;
    let mut counts = Counts::default();
    let mut layers = BTreeMap::new();
    let mut loose = Vec::new();
    for chunks in &raw.layers {
        let l = layer(legacy, old, chunks, &raw.tags, &mut counts, &mut report, c)?;
        // MC-11 / LEG-OLD-07 (provisional): a preset's layer whose item id
        // names no mesh item is a mesh of its own, as in the phase-1 reader.
        if preset && legacy && raw.items.get(&l.id).is_none_or(|i| i.kind != "mesh") {
            loose.push(l);
            continue;
        }
        if layers.insert(l.id, l).is_some() {
            return Err(err(
                "ambiguous_layer_owner",
                "Two layers reference the same mesh ITEM",
            ));
        }
    }
    if counts.subdivision > 0 {
        report.omit(format!(
            "{} subdivision polygons were imported as their control cage; the smooth surface is not shown yet",
            counts.subdivision
        ));
    }
    let nonmodel = [
        "scene",
        "environment",
        "shaderFolder",
        "polyRender",
        "envMaterial",
        "camera",
        "light",
        "material",
        "advancedMaterial",
        "mask",
        "imageMap",
    ];
    let mut bad: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut neighbors: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut transforms: BTreeMap<u32, BTreeMap<i32, u32>> = BTreeMap::new();
    let mut parents = BTreeMap::new();
    // Track this diagnostic separately from the user-facing omission text.
    let mut static_conflicts = BTreeSet::new();
    // Format 4.x procedural modelling (deformers, mesh ops, channel
    // modifiers) linked to a modeled item, by that item: its group is left
    // out with a plain reason, and a file with nothing else to import is
    // refused with a plain message naming them (Jason, 2026-11-12).
    let mut procedural: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for item in raw.items.values() {
        c.check("dependency inventory")?;
        let modeled = matches!(
            item.kind.as_str(),
            "mesh" | "translation" | "rotation" | "scale" | "meshInst" | "transform"
        );
        // LEG-12: in format 4.x, rigs, schematic nodes, procedural and
        // channel-modifier items are omitted; a mesh that depends on one is
        // refused through its links below.
        if !modeled && legacy && !nonmodel.contains(&item.kind.as_str()) {
            report.omit(format!("{} items (not modelling)", item.kind));
        } else if !modeled && !nonmodel.contains(&item.kind.as_str()) {
            return Err(err(
                "unknown_dependencies",
                &format!(
                    "Unsupported {} '{}' has unclassified modeling dependencies",
                    item.kind, item.name
                ),
            ));
        }
        if nonmodel.contains(&item.kind.as_str()) {
            report.omit(format!("{} items (scene/shader/display state)", item.kind));
        }
        if modeled {
            for reason in &item.unsupported {
                bad.entry(item.id)
                    .or_default()
                    .push(format!("{}: {reason}", item.name));
            }
        }
        if item.kind == "meshInst" || item.kind == "transform" && !legacy {
            bad.entry(item.id).or_default().push(format!(
                "{}: unsupported shared geometry/pivot stack",
                item.name
            ));
        }
        for (graph, target, ordinal) in &item.links {
            let target_item = &raw.items[target];
            if legacy {
                let target_modeled = matches!(
                    target_item.kind.as_str(),
                    "mesh" | "translation" | "rotation" | "scale" | "meshInst" | "transform"
                );
                // Item-group membership never changes geometry or placement.
                if graph == "itemGroups" {
                    report.omit("item group memberships (groups not imported)");
                    continue;
                }
                if matches!(graph.as_str(), "deformers" | "chanMods") && modeled != target_modeled {
                    let (op, model) = if modeled {
                        (target_item, item)
                    } else {
                        (item, target_item)
                    };
                    procedural
                        .entry(model.id)
                        .or_default()
                        .insert(op.kind.clone());
                    continue;
                }
                if !modeled {
                    // LEG-12: a shader-tree item limited to a mesh (a mask's
                    // shadeLoc) shades it but doesn't move or shape it.
                    if target_modeled && graph == "shadeLoc" {
                        report.omit("shader items limited to a mesh (shading not imported)");
                        continue;
                    }
                    // A non-modelling item that drives a modeled one (a
                    // constraint, deformer or channel modifier) refuses it.
                    if target_modeled && graph != "parent" {
                        bad.entry(*target).or_default().push(format!(
                            "depends on {} '{}' ({graph})",
                            item.kind, item.name
                        ));
                    }
                    continue;
                }
                if !matches!(
                    graph.as_str(),
                    "parent" | "xfrmCore" | "xfrmLocal" | "source"
                ) {
                    bad.entry(item.id)
                        .or_default()
                        .push(format!("unsupported {graph} relationship"));
                    continue;
                }
            }
            let known_nonmodel = nonmodel.contains(&item.kind.as_str())
                && nonmodel.contains(&target_item.kind.as_str())
                && matches!(graph.as_str(), "parent" | "localShader");
            if known_nonmodel {
                continue;
            }
            if !matches!(
                graph.as_str(),
                "parent" | "xfrmCore" | "xfrmLocal" | "source"
            ) {
                return Err(err(
                    "unknown_dependencies",
                    &format!("Unknown graph {graph} on '{}'", item.name),
                ));
            }
            neighbors.entry(item.id).or_default().push(*target);
            neighbors.entry(*target).or_default().push(item.id);
            match graph.as_str() {
                "parent" => {
                    if parents.insert(item.id, *target).is_some() {
                        return Err(err("ambiguous_parent", "Multiple parent links"));
                    }
                    if item.kind != "mesh" || target_item.kind != "mesh" {
                        bad.entry(item.id)
                            .or_default()
                            .push("unsupported non-mesh parent".into());
                    }
                }
                "xfrmCore" => {
                    if transforms
                        .entry(*target)
                        .or_default()
                        .insert(*ordinal, item.id)
                        .is_some()
                    {
                        return Err(err(
                            "ambiguous_transform_stack",
                            "Duplicate xfrmCore ordinal",
                        ));
                    }
                }
                _ => bad
                    .entry(item.id)
                    .or_default()
                    .push(format!("unsupported {graph} relationship")),
            }
        }
        if item.kind == "mesh" {
            if preset && legacy && !layers.contains_key(&item.id) {
                // No geometry to import for it; its layer, if any, came in
                // as a loose layer above.
                report.omit("mesh items with no layer, left out");
                continue;
            }
            if !item.has_layer || !layers.contains_key(&item.id) {
                return Err(err(
                    "invalid_layer_item_reference",
                    "Mesh ITEM lacks exactly one LAYR",
                ));
            }
            if legacy {
                // No verified format 4.x mesh channel profile: only the
                // visibility channel changes what Operandi imports.
                if item
                    .channels
                    .get("visible")
                    .is_some_and(|v| !matches!(v.text(), Some("default" | "off")))
                {
                    bad.entry(item.id)
                        .or_default()
                        .push("unsupported visibility".into());
                }
                continue;
            }
        }
    }
    for (id, kinds) in &procedural {
        bad.entry(*id)
            .or_default()
            .push(format!("uses {}", procedural_phrase(kinds)));
    }
    for l in layers.values() {
        if raw.items.get(&l.id).is_none_or(|i| i.kind != "mesh") {
            return Err(err(
                "invalid_layer_item_reference",
                "LAYR must resolve to mesh ITEM",
            ));
        }
        for b in &l.bad {
            bad.entry(l.id).or_default().push(b.clone());
        }
    }
    // Validate hierarchy even for groups that would later be refused.
    for id in parents.keys() {
        let mut visited = BTreeSet::new();
        let mut next = Some(*id);
        while let Some(id) = next {
            c.check("graph validation")?;
            if !visited.insert(id) || visited.len() > 256 {
                return Err(err("hierarchy_cycle", "Cycle or hierarchy depth limit"));
            }
            next = parents.get(&id).copied();
        }
    }
    for a in &raw.actions {
        let mut keys = BTreeSet::new();
        for (owner, key, kind, _env, v) in &a.values {
            c.check("action agreement")?;
            if !keys.insert((*owner, key)) {
                return Err(err("duplicate_action_channel", "Repeated action channel"));
            }
            let item = &raw.items[owner];
            if !matches!(
                item.kind.as_str(),
                "mesh" | "translation" | "rotation" | "scale"
            ) {
                continue;
            }
            if !matches!(a.name.as_str(), "edit" | "scene" | "setup")
                || !matches!(a.kind.as_str(), "edit" | "scene" | "setup")
            {
                bad.entry(*owner)
                    .or_default()
                    .push(format!("unknown action layer {}/{}", a.name, a.kind));
            }
            if !matches!(*kind, 1..=3) {
                bad.entry(*owner)
                    .or_default()
                    .push(format!("animated modeling dependency {key}"));
                continue;
            }
            // LEG-09: format 4.x takes edit, then setup, then ITEM values
            // (applied above) instead of requiring agreement.
            if !legacy && item.channels.get(key).is_none_or(|base| !close(base, v)) {
                static_conflicts.insert(*owner);
                bad.entry(*owner)
                    .or_default()
                    .push(format!("static action disagreement {key}"));
            }
        }
    }
    // Every ordinary mesh has one exact core stack. All transform channels are whitelisted.
    let mut local = BTreeMap::new();
    // Format 4.x stack remainders that aren't a plain transform, baked into
    // the mesh points (LXO-LEG-Q02).
    let mut baked: BTreeMap<u32, glam::DMat4> = BTreeMap::new();
    let no_stack = BTreeMap::new();
    for item in raw.items.values().filter(|i| i.kind == "mesh") {
        // LEG-09/10: in format 4.x any of the three core items may be
        // absent (identity for that part), but at most one of each.
        let stack = match transforms.get(&item.id) {
            Some(stack) => stack,
            None if legacy => &no_stack,
            None => {
                bad.entry(item.id)
                    .or_default()
                    .push("missing core transform stack".into());
                continue;
            }
        };
        let complete = stack.keys().copied().collect::<Vec<_>>() == [0, 1, 2];
        // Which item places each part: (ordinal, kind, channel base, item).
        let mut parts: Vec<(i32, &str, &str, u32)> = Vec::new();
        // LXO-LEG-Q02: format 4.x composes the whole stack, pivots and Zero
        // layers included.
        let mut legacy_tr = None;
        if legacy {
            match legacy_local(&raw, stack) {
                Ok(placed) => {
                    for note in placed.notes {
                        report.notes.push(format!("Mesh '{}': {note}", item.name));
                    }
                    for omission in placed.omissions {
                        report.omit(omission);
                    }
                    if let Some(rest) = placed.baked {
                        baked.insert(item.id, rest);
                    }
                    legacy_tr = Some(placed.transform);
                }
                Err(reason) => {
                    bad.entry(item.id).or_default().push(reason);
                    continue;
                }
            }
        } else if stack.len() != 3 || !complete {
            bad.entry(item.id)
                .or_default()
                .push("unsupported core stack shape".into());
            continue;
        } else {
            for (ordinal, expected, base) in [
                (0, "scale", "scl"),
                (1, "rotation", "rot"),
                (2, "translation", "pos"),
            ] {
                parts.push((ordinal, expected, base, stack[&ordinal]));
            }
        }
        let mut tr = Transform {
            zxy: true,
            order: None,
            ..Default::default()
        };
        for (ordinal, expected, base, id) in parts {
            let transform = &raw.items[&id];
            let kind = transform.channels.get("type").and_then(Value::text);
            if transform.kind != expected || kind != Some("core") {
                bad.entry(item.id)
                    .or_default()
                    .push("unsupported transform type/order".into());
                continue;
            }
            let order = transform.channels.get("order").and_then(Value::text);
            if expected == "rotation" && order != Some("zxy") {
                bad.entry(item.id)
                    .or_default()
                    .push("unsupported rotation order".into());
            }
            for name in transform.channels.keys() {
                if ![
                    format!("{base}.X"),
                    format!("{base}.Y"),
                    format!("{base}.Z"),
                    "type".into(),
                    "order".into(),
                ]
                .contains(name)
                {
                    bad.entry(item.id)
                        .or_default()
                        .push(format!("unknown transform channel {name}"));
                }
            }
            let absent = if ordinal == 0 { 1.0 } else { 0.0 };
            let mut xyz = [absent; 3];
            for (axis, component) in ["X", "Y", "Z"].iter().enumerate() {
                let value = transform
                    .channels
                    .get(&format!("{base}.{component}"))
                    .and_then(Value::number);
                xyz[axis] = match value {
                    Some(v) => v,
                    None => return Err(err("missing_transform_component", expected)),
                };
            }
            match ordinal {
                0 => tr.scale = DVec3::from_array(xyz),
                1 => tr.rotation = DVec3::from_array(xyz),
                _ => tr.translation = DVec3::from_array(xyz),
            }
        }
        if let Some(parent) = parents.get(&item.id)
            && (item.channels.get("visible").and_then(Value::text) == Some("off")
                || raw.items[parent]
                    .channels
                    .get("visible")
                    .and_then(Value::text)
                    == Some("off"))
        {
            bad.entry(item.id)
                .or_default()
                .push("unverified inherited visibility".into());
        }
        local.insert(item.id, legacy_tr.unwrap_or(tr));
    }
    if legacy {
        legacy_envelopes(&raw, &mut bad);
        for (owner, reason) in &raw.unreadable {
            let modeled = raw.items.get(owner).is_some_and(|i| {
                matches!(
                    i.kind.as_str(),
                    "mesh" | "translation" | "rotation" | "scale" | "meshInst" | "transform"
                )
            });
            if modeled {
                bad.entry(*owner).or_default().push(reason.clone());
            } else {
                report.omit("unreadable action values on items not imported");
            }
        }
    } else {
        super::metadata::validate_sizes(&raw, &mut bad, c)?;
        report.unreferenced_envelopes = super::q001::validate(&raw, &mut bad, c)?;
    }
    // Connected groups are atomic, including a mesh's ancestors and transforms.
    let mut rejected = BTreeSet::new();
    for id in bad.keys() {
        let mut todo = vec![*id];
        while let Some(id) = todo.pop() {
            c.check("group classification")?;
            if rejected.insert(id) {
                todo.extend(neighbors.get(&id).into_iter().flatten().copied());
            }
        }
    }
    let mut rejected_mesh_has_static_conflict = false;
    for item in raw
        .items
        .values()
        .filter(|i| i.kind == "mesh" && rejected.contains(&i.id))
    {
        let mut reasons = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut todo = vec![item.id];
        while let Some(id) = todo.pop() {
            if seen.insert(id) {
                rejected_mesh_has_static_conflict |= static_conflicts.contains(&id);
                if let Some(b) = bad.get(&id) {
                    reasons.extend(b.iter().cloned());
                }
                todo.extend(neighbors.get(&id).into_iter().flatten().copied());
            }
        }
        report.rejected_groups.push(format!(
            "Omit mesh '{}' and its dependency group: {}",
            item.name,
            reasons.into_iter().collect::<Vec<_>>().join("; ")
        ));
    }
    if !legacy {
        super::q001::account(&raw, &rejected, &mut report, c)?;
    }
    super::metadata::account(&raw, &rejected, &mut report);
    let mut scene = Scene::default();
    for (id, l) in layers {
        c.check("scene conversion")?;
        if rejected.contains(&id) {
            continue;
        }
        let source = &raw.items[&id];
        let tr = local
            .remove(&id)
            .ok_or_else(|| err("unsupported_transform_stack", "No complete local transform"))?;
        let mut l = l;
        if let Some(note) = l.pivot.and_then(|p| layer_pivot_note(&source.name, p)) {
            report.notes.push(note);
        }
        if let Some(rest) = baked.get(&id) {
            for p in &mut l.mesh.vertices {
                *p = rest.transform_point3(*p);
            }
        }
        c.charge(
            l.mesh.vertices.len() * 48
                + l.mesh
                    .faces
                    .iter()
                    .map(|f| 256 + f.vertices.len() * 64)
                    .sum::<usize>(),
        )?;
        scene.items.push(Item {
            id: u64::from(id) + 1,
            name: source.name.clone(),
            source_id: Some(id),
            source_display_name: source.visible_name.clone(),
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator: None,
            environment: None,
            // Editor Color: the COLR item tag (IC-Q1); missing, "none" or
            // an unknown name is none.
            common: crate::item_common::ItemCommon {
                editor_color: source
                    .tags
                    .iter()
                    .find(|t| &t.id == b"COLR")
                    .and_then(|t| crate::EditorColor::from_name(&t.value)),
                ..Default::default()
            },
            elements: Default::default(),
            hides_children: false,
            mesh_stack: None,
            kit: None,
            instance_of: None,
            replicator: None,
            visible: source.channels.get("visible").and_then(Value::text) != Some("off"),
            parent: parents.get(&id).map(|p| u64::from(*p) + 1),
            transform: tr,
            mesh: l.mesh.into(),
        });
    }
    // Loose preset layers: meshes in place at the root, named after the
    // layer. One with problems is left out with its reasons.
    let mut next = raw.items.keys().max().map_or(1, |m| u64::from(*m) + 2);
    for l in loose {
        c.check("scene conversion")?;
        let name = if l.name.is_empty() {
            "Mesh".to_string()
        } else {
            l.name.clone()
        };
        if !l.bad.is_empty() {
            report
                .rejected_groups
                .push(format!("Omit mesh '{name}': {}", l.bad.join("; ")));
            continue;
        }
        if let Some(note) = l.pivot.and_then(|p| layer_pivot_note(&name, p)) {
            report.notes.push(note);
        }
        scene.items.push(Item {
            id: next,
            name,
            source_id: None,
            source_display_name: None,
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator: None,
            environment: None,
            common: Default::default(),
            elements: Default::default(),
            hides_children: false,
            mesh_stack: None,
            kit: None,
            instance_of: None,
            replicator: None,
            visible: true,
            parent: None,
            transform: Transform {
                zxy: true,
                ..Default::default()
            },
            mesh: l.mesh.into(),
        });
        next += 1;
    }
    if scene.items.is_empty() && !procedural.is_empty() {
        let kinds = procedural.values().flatten().cloned().collect();
        return Err(err(
            "unsupported_procedural_modelling",
            &format!("This file uses {}.", procedural_phrase(&kinds)),
        ));
    }
    if scene.items.is_empty()
        && (raw.items.values().any(|i| i.kind == "mesh") || !report.rejected_groups.is_empty())
    {
        let category = if rejected_mesh_has_static_conflict {
            "unsupported_action_state_conflict"
        } else {
            "unsupported_modeling_group"
        };
        return Err(err(category, &sample(&report.rejected_groups)));
    }
    scene.next_id = scene.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    // Like the other importers, the first visible mesh starts out as the
    // active item, so it draws in the view's active display style.
    let first = scene
        .items
        .iter()
        .find(|i| i.visible)
        .or(scene.items.first())
        .map(|i| i.id);
    scene.select_item(first);
    c.check("candidate validation")?;
    scene
        .validate_for_import()
        .map_err(candidate_validation_error)?;
    let mut uv = Vec::new();
    let mut materials = BTreeSet::new();
    let mut parts = BTreeSet::new();
    for item in &scene.items {
        report.meshes += 1;
        report.points += item.mesh.vertices.len();
        report.polygons += item.mesh.faces.len();
        if !item.visible {
            report.hidden += 1;
        }
        for name in item.mesh.uv_maps.keys() {
            uv.push(format!("{}: {name}", item.name));
        }
        for face in &item.mesh.faces {
            report.corners += face.vertices.len();
            if let Some(m) = &face.material_label {
                materials.insert(m.clone());
            }
            if let Some(p) = &face.part {
                parts.insert(p.clone());
            }
        }
    }
    report.uv_maps = uv;
    report.materials = materials.into_iter().collect();
    report.parts = parts.into_iter().collect();
    for name in std::mem::take(&mut c.ansi_decoded) {
        report
            .notes
            .push(format!("Name '{name}' decoded as ANSI (Windows-1252)"));
    }
    c.check("native persistence preflight")?;
    crate::persistence::encode(&scene).map_err(|e| err("native_persistence_limit", &e))?;
    c.check("ready to commit")?;
    Ok(Candidate { scene, report })
}

/// LEG-09: format 4.x transform ITEM values are stale. Each modeled item's
/// static channels become its `setup` action value, overridden by its `edit`
/// action value; channels in neither keep the ITEM value.
fn apply_action_precedence(raw: &mut Raw<'_>) {
    for layer in ["setup", "edit"] {
        for a in raw
            .actions
            .iter()
            .filter(|a| a.name == layer && a.kind == layer)
        {
            for (owner, key, kind, _env, v) in &a.values {
                if matches!(*kind, 1..=3)
                    && let Some(item) = raw.items.get_mut(owner)
                {
                    item.channels.insert(key.clone(), v.clone());
                }
            }
        }
    }
}

/// A modeled item's channel driven by an envelope refuses its group
/// (LEG-08); mesh curve gradients are skipped (LEG-13); envelopes on items
/// that aren't imported don't matter.
fn legacy_envelopes(raw: &Raw<'_>, bad: &mut BTreeMap<u32, Vec<String>>) {
    for reference in &raw.envelope_references {
        let Some(item) = raw.items.get(&reference.owner) else {
            continue;
        };
        match item.kind.as_str() {
            // LEG-13: curve gradients shape generated curves only; skipped
            // without a note.
            "mesh" if matches!(reference.channel.as_str(), "radGrad" | "rotGrad") => {}
            "mesh" | "translation" | "rotation" | "scale" => bad
                .entry(item.id)
                .or_default()
                .push(format!("animated channel {}", reference.channel)),
            _ => {}
        }
    }
}

/// What a format 4.x mesh's xfrmCore stack places it by (LXO-LEG-Q02).
struct LegacyPlacement {
    transform: Transform,
    /// The part of the stack no Operandi transform holds (shear), baked
    /// into the mesh points before `transform` applies.
    baked: Option<glam::DMat4>,
    /// Per-mesh notes, without the "Mesh 'name': " prefix.
    notes: Vec<String>,
    omissions: Vec<String>,
}

/// One xfrmCore stack item, read.
struct StackPart {
    kind: &'static str,
    name: String,
    core: bool,
    scale: DVec3,
    /// The angles as stored, and the order's axes when it isn't ZXY.
    angles: DVec3,
    order: Option<[usize; 3]>,
    translation: DVec3,
}

impl StackPart {
    fn quat(&self) -> glam::DQuat {
        RotationOrder::Zxy.quat(self.zxy_angles())
    }
    fn zxy_angles(&self) -> DVec3 {
        match self.order {
            Some(axes) => zxy_from_order(axes, self.angles),
            None => self.angles,
        }
    }
    fn matrix(&self) -> glam::DMat4 {
        glam::DMat4::from_scale_rotation_translation(self.scale, self.quat(), self.translation)
    }
    fn turns_or_scales(&self) -> bool {
        self.scale != DVec3::ONE || self.angles != DVec3::ZERO
    }
    fn is_identity(&self) -> bool {
        !self.turns_or_scales() && self.translation == DVec3::ZERO
    }
}

/// Read one xfrmCore item: a scale, rotation or translation item reads its
/// own channels; a `transform` item reads any of the three (provisional).
/// Absent channels take their defaults (LEG-09). Anything else can't be read.
fn legacy_stack_part(item: &super::decode::Item) -> std::result::Result<StackPart, String> {
    let (kind, bases): (&'static str, &[&str]) = match item.kind.as_str() {
        "scale" => ("scale", &["scl"]),
        "rotation" => ("rotation", &["rot"]),
        "translation" => ("translation", &["pos"]),
        "transform" => ("transform", &["scl", "rot", "pos"]),
        other => {
            return Err(format!(
                "unsupported {other} item '{}' in the transform stack",
                item.name
            ));
        }
    };
    for name in item.channels.keys() {
        let known = name == "type"
            || name == "order"
            || bases
                .iter()
                .any(|b| ["X", "Y", "Z"].iter().any(|c| *name == format!("{b}.{c}")));
        if !known {
            return Err(format!("unknown transform channel {name}"));
        }
    }
    let read = |base: &str, absent: f64| -> std::result::Result<DVec3, String> {
        let mut xyz = [absent; 3];
        if bases.contains(&base) {
            for (axis, component) in ["X", "Y", "Z"].iter().enumerate() {
                let key = format!("{base}.{component}");
                if let Some(v) = item.channels.get(&key) {
                    xyz[axis] = v
                        .number()
                        .filter(|n| n.is_finite())
                        .ok_or_else(|| format!("unreadable transform channel {key}"))?;
                }
            }
        }
        Ok(DVec3::from_array(xyz))
    };
    let order = match item.channels.get("order").and_then(Value::text) {
        None | Some("zxy") => None,
        Some(o) => Some(rotation_order(o).ok_or("unsupported rotation order")?),
    };
    Ok(StackPart {
        kind,
        name: item.name.clone(),
        core: item
            .channels
            .get("type")
            .and_then(Value::text)
            .is_none_or(|t| t == "core"),
        scale: read("scl", 1.0)?,
        angles: read("rot", 0.0)?,
        order: if bases.contains(&"rot") { order } else { None },
        translation: read("pos", 0.0)?,
    })
}

/// LXO-LEG-Q02: compose a format 4.x mesh's whole xfrmCore stack in stack
/// order (lowest ordinal applies first, as core scale 0, rotation 1,
/// translation 2), pivots, their inverses and Zero layers included.
fn legacy_local(
    raw: &Raw<'_>,
    stack: &BTreeMap<i32, u32>,
) -> std::result::Result<LegacyPlacement, String> {
    let mut parts = Vec::new();
    for id in stack.values() {
        parts.push(legacy_stack_part(&raw.items[id])?);
    }
    let mut notes = Vec::new();
    let mut omissions = Vec::new();
    // Safe shortcut: a pivot pair ("N" and "N Inverse") cancels when the
    // pivot is zero or nothing between them rotates or scales.
    let mut dropped = vec![false; parts.len()];
    for i in 0..parts.len() {
        if parts[i].core || dropped[i] {
            continue;
        }
        let inverse = format!("{} Inverse", parts[i].name);
        let Some(j) = (0..parts.len()).find(|&j| !dropped[j] && parts[j].name == inverse) else {
            continue;
        };
        let (lo, hi) = (i.min(j), i.max(j));
        let zero = parts[i].is_identity() && parts[j].is_identity();
        if zero || !parts[lo + 1..hi].iter().any(StackPart::turns_or_scales) {
            dropped[i] = true;
            dropped[j] = true;
            omissions.push("pivot pairs with no effect (cancel)".to_string());
        }
    }
    let kept: Vec<&StackPart> = parts
        .iter()
        .zip(&dropped)
        .filter(|(p, d)| !**d && !p.is_identity())
        .map(|(p, _)| p)
        .collect();
    for p in &kept {
        if let (Some(axes), true) = (p.order, p.angles != DVec3::ZERO) {
            let name: String = axes.iter().map(|a| ['X', 'Y', 'Z'][*a]).collect();
            notes.push(format!(
                "rotation order {name} converted to ZXY (same orientation)"
            ));
            omissions.push(format!(
                "rotations in {name} order converted to ZXY (same orientation)"
            ));
        }
    }
    let fresh = Transform {
        zxy: true,
        ..Default::default()
    };
    let mut transform = fresh.clone();
    // A plain stack (at most one scale, rotation and translation, in that
    // order, adjacent translations added) is read part by part, exactly as
    // before pivots were composed.
    let mut rank = 0;
    let mut plain = true;
    for p in &kept {
        let r = match p.kind {
            "scale" => 1,
            "rotation" => 2,
            "translation" => 3,
            _ => 4,
        };
        if r == 4 || r < rank || r == rank && r != 3 {
            plain = false;
            break;
        }
        rank = r;
        match r {
            1 => transform.scale = p.scale,
            2 => transform.rotation = p.zxy_angles(),
            _ => transform.translation += p.translation,
        }
    }
    if plain {
        return Ok(LegacyPlacement {
            transform,
            baked: None,
            notes,
            omissions,
        });
    }
    let m = kept
        .iter()
        .fold(glam::DMat4::IDENTITY, |m, p| p.matrix() * m);
    let tolerance = 1e-9
        * m.to_cols_array()
            .iter()
            .fold(1.0_f64, |a, v| a.max(v.abs()));
    let (s, q, t) = m.to_scale_rotation_translation();
    let rebuilt = glam::DMat4::from_scale_rotation_translation(s, q, t);
    if s.is_finite() && q.is_finite() && rebuilt.abs_diff_eq(m, tolerance) {
        let mut transform = fresh;
        transform.scale = s;
        transform.rotation = RotationOrder::Zxy.angles(q);
        transform.translation = t;
        notes.push("placed by its whole transform stack (pivot or Zero items composed)".into());
        omissions.push("meshes placed by pivot or Zero transform items (composed)".into());
        return Ok(LegacyPlacement {
            transform,
            baked: None,
            notes,
            omissions,
        });
    }
    // Shear: keep the position as the transform, bake the rest.
    let mut transform = fresh;
    transform.translation = m.w_axis.truncate();
    let mut rest = m;
    rest.w_axis = glam::DVec4::W;
    notes.push(
        "transform stack has shear; its rotation and scale were baked into the mesh points".into(),
    );
    omissions.push("meshes with sheared transform stacks (baked into points)".into());
    Ok(LegacyPlacement {
        transform,
        baked: Some(rest),
        notes,
        omissions,
    })
}

/// The axes (0 = X) of a rotation order such as "xzy", in the order they
/// apply: "zxy" rotates about Z, then X, then Y (`Transform::matrix`).
fn rotation_order(order: &str) -> Option<[usize; 3]> {
    let axes: Vec<usize> = order
        .chars()
        .map(|c| match c {
            'x' => Some(0),
            'y' => Some(1),
            'z' => Some(2),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let set: BTreeSet<_> = axes.iter().collect();
    (axes.len() == 3 && set.len() == 3).then(|| [axes[0], axes[1], axes[2]])
}

/// ZXY angles giving the same orientation as `angles` applied in `axes`
/// order. Operandi's transform holds only XYZ or ZXY, so other orders are
/// converted once on import.
fn zxy_from_order(axes: [usize; 3], angles: DVec3) -> DVec3 {
    let mut q = glam::DQuat::IDENTITY;
    for axis in axes {
        let angle = angles[axis];
        let r = match axis {
            0 => glam::DQuat::from_rotation_x(angle),
            1 => glam::DQuat::from_rotation_y(angle),
            _ => glam::DQuat::from_rotation_z(angle),
        };
        q = r * q;
    }
    // Transform::matrix composes ZXY as Ry * Rx * Rz, which is glam's YXZ.
    let (y, x, z) = q.to_euler(glam::EulerRot::YXZ);
    DVec3::new(x, y, z)
}

/// A readable name for a format 4.x procedural item type, else the type.
fn procedural_name(kind: &str) -> String {
    match kind {
        "curvefill" => "Curve Fill",
        "deform.push" => "Push",
        "pmodel.meshmerge" => "Mesh Merge",
        "pmodel.edgeToCurve.item" => "Edge to Curve",
        "genInfluence" => "General Influence",
        "assignselectionset.meshop.item" => "Assign Selection Set",
        "cmMatrixCompose" => "Matrix Compose",
        other => other,
    }
    .to_string()
}

/// "procedural modelling (Curve Fill, Push), which Operandi can't import yet"
fn procedural_phrase(kinds: &BTreeSet<String>) -> String {
    let names: BTreeSet<String> = kinds.iter().map(|k| procedural_name(k)).collect();
    let mut list: Vec<String> = names.iter().take(6).cloned().collect();
    if names.len() > 6 {
        list.push("…".into());
    }
    format!(
        "procedural modelling ({}), which Operandi can't import yet",
        list.join(", ")
    )
}

/// LEG-14: what makes a format 4.x mesh unusable, with counts, or `None`.
fn legacy_geometry_problems(mesh: &Mesh) -> Option<String> {
    let mut uses: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    let (mut repeated, mut zero_area) = (0, 0);
    for (f, face) in mesh.faces.iter().enumerate() {
        if face.vertices.iter().collect::<BTreeSet<_>>().len() != face.vertices.len() {
            repeated += 1;
        } else if mesh.normal(f) == DVec3::ZERO {
            zero_area += 1;
        }
        for i in 0..face.vertices.len() {
            let (a, b) = (
                face.vertices[i],
                face.vertices[(i + 1) % face.vertices.len()],
            );
            uses.entry((a.min(b), a.max(b))).or_default().push((a, b));
        }
    }
    let shared = uses.values().filter(|u| u.len() > 2).count();
    let flipped = uses
        .values()
        .filter(|u| u.len() == 2 && u[0] == u[1])
        .count();
    let parts: Vec<String> = [
        (shared, "edges shared by more than two polygons"),
        (flipped, "edges between polygons facing opposite ways"),
        (zero_area, "zero-area polygons"),
        (repeated, "polygons that repeat a point"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| format!("{n} {what}"))
    .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Format 4.x leniencies, on the Operandi-made legacy fixtures with items
/// and links added in code.
#[cfg(test)]
mod legacy_relax_tests {
    use super::super::{Reader, decode::Item as RawItem};
    use super::*;

    const LG01: &[u8] = include_bytes!(
        "../../../../specifications/lxo-legacy-import-2026-11-03/fixtures/LG-01-v40-appv9-tail70.lxo"
    );
    const LG03: &[u8] = include_bytes!(
        "../../../../specifications/lxo-legacy-import-2026-11-03/fixtures/LG-03-v41-appv10-tail74.lxo"
    );

    /// Decode `bytes`, let `edit` change the raw scene, then convert.
    fn convert_with(bytes: &[u8], edit: impl FnOnce(&mut Raw<'_>)) -> Result<Candidate> {
        let mut go = |_: &str| true;
        let mut c = Control::new(&mut go);
        let mut r = Reader::new(bytes, 0);
        r.id().unwrap();
        r.u32().unwrap();
        r.id().unwrap();
        let chunks = r.chunks(false, &mut c).unwrap();
        let mut raw = super::super::decode::decode(&chunks, &mut c).unwrap();
        edit(&mut raw);
        convert(raw, &mut c, false)
    }

    fn item(id: u32, kind: &str, name: &str, channels: &[(&str, Value)]) -> RawItem {
        RawItem {
            id,
            kind: kind.into(),
            name: name.into(),
            visible_name: None,
            channels: channels
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            links: vec![],
            gradients: vec![],
            unsupported: vec![],
            has_layer: false,
            tags: vec![],
        }
    }

    fn mesh_id(raw: &Raw<'_>, name: &str) -> u32 {
        raw.items
            .values()
            .find(|i| i.kind == "mesh" && i.name == name)
            .unwrap()
            .id
    }

    /// Replace the mesh's rotation item (if any) with `rotation`.
    fn set_rotation(raw: &mut Raw<'_>, mesh: u32, mut rotation: RawItem) {
        for i in raw.items.values_mut().filter(|i| i.kind == "rotation") {
            i.links.retain(|(g, t, _)| !(g == "xfrmCore" && *t == mesh));
        }
        rotation.links.push(("xfrmCore".into(), mesh, 1));
        raw.items.insert(rotation.id, rotation);
    }

    #[test]
    fn every_rotation_order_imports_as_the_same_orientation() {
        let angles = DVec3::new(0.3, -0.7, 1.1);
        let rot = |axis: usize, a: f64| match axis {
            0 => glam::DQuat::from_rotation_x(a),
            1 => glam::DQuat::from_rotation_y(a),
            _ => glam::DQuat::from_rotation_z(a),
        };
        for order in ["xyz", "xzy", "yxz", "yzx", "zxy", "zyx"] {
            let c = convert_with(LG03, |raw| {
                let mesh = mesh_id(raw, "Tail74");
                let r = item(
                    9000,
                    "rotation",
                    "Rotation",
                    &[
                        ("type", Value::Text("core".into())),
                        ("order", Value::Text(order.into())),
                        ("rot.X", Value::Float(angles.x)),
                        ("rot.Y", Value::Float(angles.y)),
                        ("rot.Z", Value::Float(angles.z)),
                    ],
                );
                set_rotation(raw, mesh, r);
            })
            .and_then(Candidate::strict)
            .unwrap_or_else(|e| panic!("{order}: {e}"));
            // The order's first letter applies first.
            let axes = rotation_order(order).unwrap();
            let want = axes
                .iter()
                .fold(glam::DQuat::IDENTITY, |q, a| rot(*a, angles[*a]) * q);
            let item = &c.scene.items[0];
            let got = glam::DQuat::from_mat4(&item.transform.matrix());
            assert!(got.dot(want).abs() > 1.0 - 1e-9, "{order}");
            let noted = c
                .report
                .omissions
                .keys()
                .any(|k| k.contains("converted to ZXY"));
            assert_eq!(noted, order != "zxy", "{order}");
            let per_mesh = format!(
                "Mesh 'Tail74': rotation order {} converted to ZXY (same orientation)",
                order.to_uppercase()
            );
            assert_eq!(
                c.report.notes.contains(&per_mesh),
                order != "zxy",
                "{order}: {:?}",
                c.report.notes
            );
        }
    }

    #[test]
    fn unknown_rotation_order_still_refuses_the_mesh() {
        let e = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            let r = item(
                9000,
                "rotation",
                "Rotation",
                &[("order", Value::Text("xxy".into()))],
            );
            set_rotation(raw, mesh, r);
        })
        .unwrap_err();
        assert!(e.detail.contains("unsupported rotation order"), "{e}");
    }

    /// Replace the mesh's whole xfrmCore stack with `parts`, ordinal = index.
    fn set_stack(raw: &mut Raw<'_>, mesh: u32, parts: Vec<RawItem>) {
        for i in raw.items.values_mut() {
            i.links.retain(|(g, t, _)| !(g == "xfrmCore" && *t == mesh));
        }
        for (n, mut part) in parts.into_iter().enumerate() {
            part.links.push(("xfrmCore".into(), mesh, n as i32));
            raw.items.insert(part.id, part);
        }
    }

    fn xyz(base: &str, v: DVec3) -> Vec<(String, Value)> {
        ["X", "Y", "Z"]
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("{base}.{c}"), Value::Float(v[i])))
            .collect()
    }

    /// A stack item of `kind` named `name` with `type` and the given vector.
    fn part(id: u32, kind: &str, name: &str, kind_type: &str, v: DVec3) -> RawItem {
        let base = match kind {
            "scale" => "scl",
            "rotation" => "rot",
            _ => "pos",
        };
        let mut it = item(id, kind, name, &[("type", Value::Text(kind_type.into()))]);
        it.channels.extend(xyz(base, v));
        it
    }

    fn zxy(angles: DVec3) -> glam::DMat4 {
        glam::DMat4::from_quat(RotationOrder::Zxy.quat(angles))
    }

    const S: DVec3 = DVec3::new(2.0, 0.5, 1.5);
    const R: DVec3 = DVec3::new(0.3, -0.7, 1.1);
    const T: DVec3 = DVec3::new(1.0, -2.0, 3.0);
    const P: DVec3 = DVec3::new(0.25, 4.0, -1.0);

    fn core_stack() -> Vec<RawItem> {
        vec![
            part(9100, "scale", "Scale", "core", S),
            part(9101, "rotation", "Rotation", "core", R),
            part(9102, "translation", "Position", "core", T),
        ]
    }

    #[test]
    fn a_core_only_stack_reads_exactly_as_before() {
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            set_stack(raw, mesh, core_stack());
        })
        .and_then(Candidate::strict)
        .unwrap();
        let tr = &c.scene.items[0].transform;
        assert_eq!((tr.scale, tr.rotation, tr.translation), (S, R, T));
        assert!(tr.zxy && tr.order.is_none());
        assert!(c.report.notes.is_empty(), "{:?}", c.report.notes);
    }

    #[test]
    fn a_pivot_with_rotation_and_scale_moves_the_mesh() {
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            let mut stack = core_stack();
            stack.insert(
                0,
                part(9103, "translation", "Pivot Position Inverse", "piInv", -P),
            );
            stack.insert(3, part(9104, "translation", "Pivot Position", "pivot", P));
            set_stack(raw, mesh, stack);
        })
        .and_then(Candidate::strict)
        .unwrap();
        let want = glam::DMat4::from_translation(T)
            * glam::DMat4::from_translation(P)
            * zxy(R)
            * glam::DMat4::from_scale(S)
            * glam::DMat4::from_translation(-P);
        let got = c.scene.items[0].transform.matrix();
        assert!(got.abs_diff_eq(want, 1e-9), "{got:?}\n{want:?}");
        assert!(
            !got.abs_diff_eq(
                glam::DMat4::from_scale_rotation_translation(S, RotationOrder::Zxy.quat(R), T),
                1e-6
            ),
            "the pivot changes the world position"
        );
        assert!(
            c.report
                .notes
                .iter()
                .any(|n| n == "Mesh 'Tail74': placed by its whole transform stack (pivot or Zero items composed)"),
            "{:?}",
            c.report.notes
        );
    }

    #[test]
    fn zero_layers_hold_the_real_offset() {
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            set_stack(
                raw,
                mesh,
                vec![
                    part(9100, "scale", "Scale", "core", DVec3::ONE),
                    part(9105, "rotation", "Rotation Zero", "zero", R),
                    part(9101, "rotation", "Rotation", "core", DVec3::ZERO),
                    part(9106, "translation", "Position Zero", "zero", T),
                    part(9102, "translation", "Position", "core", DVec3::ZERO),
                ],
            );
        })
        .and_then(Candidate::strict)
        .unwrap();
        let tr = &c.scene.items[0].transform;
        assert_eq!((tr.rotation, tr.translation), (R, T));
        // Both turn: composed.
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            set_stack(
                raw,
                mesh,
                vec![
                    part(9105, "rotation", "Rotation Zero", "zero", R),
                    part(9101, "rotation", "Rotation", "core", -R),
                    part(9106, "translation", "Position Zero", "zero", T),
                ],
            );
        })
        .and_then(Candidate::strict)
        .unwrap();
        let want = glam::DMat4::from_translation(T) * zxy(-R) * zxy(R);
        assert!(c.scene.items[0].transform.matrix().abs_diff_eq(want, 1e-9));
    }

    #[test]
    fn a_pivot_pair_cancels_without_rotation_or_scale() {
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            set_stack(
                raw,
                mesh,
                vec![
                    // Stored either way, a pair with nothing turning or
                    // scaling between it has no effect.
                    part(9103, "translation", "Pivot Position Inverse", "piInv", P),
                    part(9100, "scale", "Scale", "core", DVec3::ONE),
                    part(9104, "translation", "Pivot Position", "pivot", P),
                    part(9102, "translation", "Position", "core", T),
                ],
            );
        })
        .and_then(Candidate::strict)
        .unwrap();
        let tr = &c.scene.items[0].transform;
        assert_eq!(
            (tr.scale, tr.rotation, tr.translation),
            (DVec3::ONE, DVec3::ZERO, T)
        );
        assert!(c.report.notes.is_empty(), "{:?}", c.report.notes);
    }

    #[test]
    fn an_unreadable_stack_item_refuses_the_mesh() {
        for (what, edit) in [
            (
                "unknown transform channel matrix",
                Box::new(|p: &mut RawItem| {
                    p.channels.insert("matrix".into(), Value::Float(1.0));
                }) as Box<dyn Fn(&mut RawItem)>,
            ),
            (
                "unreadable transform channel pos.X",
                Box::new(|p: &mut RawItem| {
                    p.channels.insert("pos.X".into(), Value::Text("?".into()));
                }),
            ),
            (
                "animated channel pos.X",
                Box::new(|p: &mut RawItem| p.unsupported.push("animated channel pos.X".into())),
            ),
        ] {
            let e = convert_with(LG03, |raw| {
                let mesh = mesh_id(raw, "Tail74");
                let mut stack = core_stack();
                let mut pivot = part(9104, "translation", "Pivot Position", "pivot", P);
                edit(&mut pivot);
                stack.push(pivot);
                set_stack(raw, mesh, stack);
            })
            .unwrap_err();
            assert!(e.detail.contains(what), "{what}: {e}");
        }
        let e = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            let mut stack = core_stack();
            stack.push(item(9107, "locator", "Odd", &[]));
            set_stack(raw, mesh, stack);
        })
        .unwrap_err();
        // Refused by the dependency check before the stack is read.
        assert!(e.detail.contains("locator 'Odd'"), "{e}");
    }

    #[test]
    fn a_sheared_stack_bakes_the_rest_into_the_points() {
        let shear = DVec3::new(3.0, 1.0, 1.0);
        let base = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            set_stack(raw, mesh, vec![]);
        })
        .unwrap();
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            let mut stack = core_stack();
            stack.insert(2, part(9108, "scale", "Scale Zero", "zero", shear));
            set_stack(raw, mesh, stack);
        })
        .and_then(Candidate::strict)
        .unwrap();
        let m = glam::DMat4::from_translation(T)
            * glam::DMat4::from_scale(shear)
            * zxy(R)
            * glam::DMat4::from_scale(S);
        let item = &c.scene.items[0];
        let before = &base.scene.items[0].mesh.vertices;
        for (a, b) in before.iter().zip(&item.mesh.vertices) {
            let want = m.transform_point3(*a);
            let got = item.transform.point(*b);
            assert!(got.abs_diff_eq(want, 1e-9), "{got} {want}");
        }
        assert!(c.report.notes.iter().any(|n| n.contains("shear")));
    }

    #[test]
    fn stacked_rotations_compose() {
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            set_rotation(raw, mesh, item(9000, "rotation", "R1", &xyz_ref("rot", R)));
            let mut r2 = item(9001, "rotation", "R2", &xyz_ref("rot", R));
            r2.links.push(("xfrmCore".into(), mesh, 4));
            raw.items.insert(9001, r2);
        })
        .and_then(Candidate::strict)
        .unwrap();
        let tr = &c.scene.items[0].transform;
        assert!(glam::DMat4::from_quat(tr.rotation_quat()).abs_diff_eq(zxy(R) * zxy(R), 1e-9));
    }

    fn xyz_ref(base: &str, v: DVec3) -> Vec<(&'static str, Value)> {
        let names: [&'static str; 3] = match base {
            "rot" => ["rot.X", "rot.Y", "rot.Z"],
            "scl" => ["scl.X", "scl.Y", "scl.Z"],
            _ => ["pos.X", "pos.Y", "pos.Z"],
        };
        (0..3).map(|i| (names[i], Value::Float(v[i]))).collect()
    }

    /// LG01's chunks with the mesh name "LegacyParent" made "L\xE9gacyParent"
    /// (not UTF-8) and ENCO replaced by `enco` (none if `None`).
    fn ansi_name(enco: Option<u32>) -> Result<Candidate> {
        let mut go = |_: &str| true;
        let mut c = Control::new(&mut go);
        let mut r = Reader::new(LG01, 0);
        r.id().unwrap();
        r.u32().unwrap();
        r.id().unwrap();
        let chunks = r.chunks(false, &mut c).unwrap();
        let mut patched_name = false;
        let datas: Vec<Vec<u8>> = chunks
            .iter()
            .map(|ch| {
                let mut d = ch.data.to_vec();
                if &ch.id == b"ITEM"
                    && let Some(i) = d.windows(13).position(|w| w == b"LegacyParent\0")
                {
                    d[i + 1] = 0xE9;
                    patched_name = true;
                }
                d
            })
            .collect();
        assert!(patched_name);
        let enco_data = enco.map(u32::to_be_bytes);
        let mut patched: Vec<Chunk<'_>> = chunks
            .iter()
            .zip(&datas)
            .filter(|(ch, _)| &ch.id != b"ENCO")
            .map(|(ch, d)| Chunk {
                id: ch.id,
                data: d,
                offset: ch.offset,
            })
            .collect();
        if let Some(d) = &enco_data {
            patched.insert(
                1,
                Chunk {
                    id: *b"ENCO",
                    data: d,
                    offset: 0,
                },
            );
        }
        let raw = super::super::decode::decode(&patched, &mut c)?;
        convert(raw, &mut c, false)
    }

    #[test]
    fn a_name_that_is_not_utf8_is_read_as_ansi_without_utf8_enco() {
        for enco in [None, Some(0)] {
            let c = ansi_name(enco).and_then(Candidate::strict).unwrap();
            assert!(c.scene.items.iter().any(|i| i.name == "L\u{e9}gacyParent"));
            assert!(
                c.report
                    .notes
                    .contains(&"Name 'L\u{e9}gacyParent' decoded as ANSI (Windows-1252)".into()),
                "{:?}",
                c.report.notes
            );
        }
        let e = ansi_name(Some(2)).unwrap_err();
        assert_eq!(e.category, "invalid_utf8");
    }

    #[test]
    fn a_layer_pivot_does_not_move_the_points() {
        let mut bytes = LG03.to_vec();
        let at = bytes.windows(4).position(|w| w == b"LAYR").unwrap() + 8 + 4;
        for (i, v) in [1.0_f32, -2.5, 3.0].iter().enumerate() {
            bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&v.to_be_bytes());
        }
        let base = convert_with(LG03, |_| {})
            .and_then(Candidate::strict)
            .unwrap();
        let c = convert_with(&bytes, |_| {})
            .and_then(Candidate::strict)
            .unwrap();
        assert_eq!(
            c.scene.items[0].mesh.vertices,
            base.scene.items[0].mesh.vertices
        );
        assert!(
            c.report.notes.contains(
                &"Mesh 'Tail74': layer pivot (1, -2.5, 3) kept as information only, not applied"
                    .to_string()
            ),
            "{:?}",
            c.report.notes
        );
    }

    #[test]
    fn item_group_membership_does_not_block_a_mesh() {
        let c = convert_with(LG03, |raw| {
            let mesh = mesh_id(raw, "Tail74");
            let mut group = item(9003, "group", "Workspace", &[]);
            group.links.push(("itemGroups".into(), mesh, 0));
            raw.items.insert(9003, group);
        })
        .and_then(Candidate::strict)
        .unwrap();
        assert_eq!(c.scene.items.len(), 1);
        assert!(
            c.report
                .omissions
                .contains_key("item group memberships (groups not imported)")
        );
    }

    #[test]
    fn procedural_modelling_leaves_out_only_its_meshes() {
        let c = convert_with(LG01, |raw| {
            // Unparent the child so the two meshes are separate groups.
            let child = mesh_id(raw, "LegacyChild");
            raw.items
                .get_mut(&child)
                .unwrap()
                .links
                .retain(|(g, _, _)| g != "parent");
            let mut op = item(9010, "deform.push", "", &[]);
            op.links.push(("deformers".into(), child, 0));
            raw.items.insert(9010, op);
        })
        .unwrap();
        assert_eq!(
            c.report.rejected_groups,
            [
                "Omit mesh 'LegacyChild' and its dependency group: uses procedural modelling (Push), which Operandi can't import yet"
            ]
        );
        assert!(c.scene.items.iter().any(|i| i.name == "LegacyParent"));
    }

    #[test]
    fn procedural_modelling_refuses_the_file_plainly() {
        let e = convert_with(LG01, |raw| {
            let mesh = mesh_id(raw, "LegacyParent");
            for (id, kind, graph) in [
                (9004, "curvefill", "deformers"),
                (9005, "deform.push", "deformers"),
                (9006, "pmodel.someNewOp", "deformers"),
                (9007, "cmMatrixCompose", "chanMods"),
            ] {
                let mut op = item(id, kind, kind, &[]);
                op.links.push((graph.into(), mesh, 0));
                raw.items.insert(id, op);
            }
        })
        .unwrap_err();
        assert_eq!(e.category, "unsupported_procedural_modelling");
        assert_eq!(
            e.to_string(),
            "This file uses procedural modelling (Curve Fill, Matrix Compose, Push, pmodel.someNewOp), which Operandi can't import yet."
        );
    }
}
