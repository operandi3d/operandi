//! LXO import, phase 1 (spec `lxo-import-phase1-2026-11-05`): the modelling
//! content of a Modo scene file. Mesh items and names, points, polygons and
//! polygon types, material and part tags, UV maps, subdivision edge
//! weights, item transforms, parenting and visibility; locators, group
//! locators and mesh instances with them (LXO-AN-Q01..Q03). Everything else
//! (cameras, lights, shader tree, envelopes, actions, animation) is skipped
//! without failing.
//!
//! Points are stored left-handed: Modo's coordinates negate Z. Polygon
//! corners are stored the other way round: Modo's order keeps the first
//! corner and reverses the rest. Item transform values are stored as Modo
//! shows them.
//!
//! Provisional readings (spec open questions): rotation and scale are used
//! as stored (Q-LX-01); polygon types other than FACE, SUBD and PSUB are
//! skipped and counted (Q-LX-02); several POLS chunks append, with PTAG and
//! VMAD indices relative to the latest (Q-LX-03); points are absolute, so
//! the layer's pivot is kept as a note only (Q-LX-04, answered 2026-11-15);
//! PART tags become polygon parts
//! (Q-LX-05); `visible` values other than `default` and `off` import as
//! visible and are reported (Q-LX-06).
use super::wire::{Chunk, Reader};
use super::{Candidate, Control, Error, Report, Result};
use crate::{
    Face, Mesh, MeshCrease, PolygonRepresentation, Scene,
    scene::{Item, Transform},
};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

fn err(category: &'static str, detail: &str) -> Error {
    Error::new(category, 0, detail)
}

#[derive(Clone, Debug)]
enum Value {
    Number(f64),
    Text(String),
}

#[derive(Default)]
struct RawItem {
    kind: String,
    name: String,
    visible_name: Option<String>,
    channels: BTreeMap<String, Value>,
    /// (graph, target reference, ordinal)
    links: Vec<(String, u32, u32)>,
}

#[derive(Default)]
struct Layer {
    item: Option<u32>,
    name: String,
    pivot: DVec3,
    points: Vec<DVec3>,
    faces: Vec<Face>,
    representations: Vec<PolygonRepresentation>,
    /// Global polygon index of each polygon of the latest POLS chunk
    /// (`None` for a skipped polygon type).
    latest: Vec<Option<usize>>,
    /// Per map name: point values (VMAP) and (point, polygon) values (VMAD).
    point_uvs: BTreeMap<String, BTreeMap<u32, DVec2>>,
    corner_uvs: BTreeMap<String, BTreeMap<(u32, usize), DVec2>>,
    creases: BTreeMap<(u32, u32), f64>,
}

/// A channel value: the type's low 4 bits give the kind (1 integer, 2
/// float, 3 string); the high bits are ignored.
fn value(r: &mut Reader<'_>, kind: u16, c: &mut Control<'_>) -> Result<Option<Value>> {
    Ok(match kind & 0xf {
        1 => Some(Value::Number(r.u32()? as i32 as f64)),
        2 => Some(Value::Number(r.f32()?)),
        3 => Some(Value::Text(r.string(c)?)),
        _ => None,
    })
}

/// Reads one Modo scene file (any `VRSN` but the Modo 8–10 formats, which
/// `decode`/`convert` read).
/// `preset`: the file is a mesh preset (`LXPM`), whose layers may have no
/// mesh item; each such layer then imports as a mesh of its own.
pub(super) fn read(chunks: &[Chunk<'_>], c: &mut Control<'_>, preset: bool) -> Result<Candidate> {
    let mut report = Report::default();
    let mut names: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    let mut layers: Vec<Layer> = Vec::new();
    let mut items: BTreeMap<u32, RawItem> = BTreeMap::new();
    let mut skipped_chunks: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped_types = 0usize;
    // Records inside chunks that are read but not used (item records,
    // polygon tag and map types), by name.
    let mut skipped_records: BTreeMap<String, usize> = BTreeMap::new();
    for ch in chunks {
        c.check("reading chunks")?;
        let mut r = Reader::new(ch.data, ch.offset);
        match &ch.id {
            b"VRSN" => {
                let (a, b) = (r.u32()?, r.u32()?);
                let text = r.string(c).unwrap_or_default();
                if report.producer.is_empty() {
                    report.producer = format!("{text} (LXO format {a}.{b})");
                }
            }
            b"APPV" => {
                // Information only: the saving application's version.
                for _ in 0..4 {
                    r.u32()?;
                }
                if let Ok(text) = r.string(c) {
                    report.producer = text;
                }
            }
            b"ENCO" => {
                if r.u32()? != 2 {
                    return Err(r.error("unsupported_encoding", "Only UTF-8 (ENCO 2) is read"));
                }
            }
            b"TAGS" => {
                while !r.done() {
                    c.record(true)?;
                    tags.push(r.string(c)?);
                }
            }
            b"CHNM" => {
                let n = r.u32()? as usize;
                if n > r.data.len() / 2 {
                    return Err(r.error("resource_limit", "CHNM count exceeds its payload"));
                }
                for _ in 0..n {
                    names.push(r.string(c)?);
                }
            }
            b"LAYR" => {
                if layers.len() >= crate::MAX_SCENE_ITEMS {
                    return Err(r.error("resource_limit", "Mesh layer count"));
                }
                let _index = r.u16()?;
                let _flags = r.u16()?;
                let pivot = DVec3::new(r.f32()?, r.f32()?, r.f32()?);
                let name = r.string(c)?;
                let _legacy_parent = r.u16()?;
                let _subdivision_level = r.f32()?;
                let _curve_angle = r.f32()?;
                // Scale pivot and six unused values.
                r.take(9 * 4)?;
                let item = r.u32()?;
                layers.push(Layer {
                    item: Some(item),
                    name,
                    pivot,
                    ..Default::default()
                });
            }
            b"PNTS" | b"POLS" | b"PTAG" | b"VMAP" | b"VMAD" | b"VMED" => {
                let layer = layers
                    .last_mut()
                    .ok_or_else(|| r.error("missing_geometry_scope", "Geometry before any LAYR"))?;
                geometry(
                    layer,
                    ch,
                    &tags,
                    &mut skipped_types,
                    &mut skipped_records,
                    c,
                )?;
            }
            b"ITEM" => {
                if items.len() >= 4 * crate::MAX_SCENE_ITEMS {
                    return Err(r.error("resource_limit", "Item count"));
                }
                let kind = r.string(c)?;
                let name = r.string(c)?;
                let id = r.u32()?;
                let mut item = RawItem {
                    kind,
                    name,
                    ..Default::default()
                };
                for sub in r.chunks(true, c)? {
                    let mut s = Reader::new(sub.data, sub.offset);
                    match &sub.id {
                        b"VNAM" => item.visible_name = Some(s.string(c)?),
                        b"LINK" => {
                            let graph = s.string(c)?;
                            let target = s.u32()?;
                            let ordinal = s.u32()?;
                            item.links.push((graph, target, ordinal));
                        }
                        b"CHAN" => {
                            let index = s.vx()? as usize;
                            let kind = s.u16()?;
                            if let (Some(name), Some(v)) =
                                (names.get(index), value(&mut s, kind, c)?)
                            {
                                item.channels.insert(name.clone(), v);
                            }
                        }
                        b"CHNV" => {
                            let base = s.string(c)?;
                            let kind = s.u16()?;
                            let n = s.u16()?;
                            for _ in 0..n {
                                let component = s.string(c)?;
                                let Some(v) = value(&mut s, kind, c)? else {
                                    break;
                                };
                                item.channels.insert(format!("{base}.{component}"), v);
                            }
                        }
                        // Display, rig and package records don't change
                        // geometry or placement.
                        other => {
                            *skipped_records
                                .entry(format!("ITEM.{}", String::from_utf8_lossy(other)))
                                .or_default() += 1;
                        }
                    }
                }
                if items.insert(id, item).is_some() {
                    return Err(
                        r.error("duplicate_item_reference", "Two ITEMs share a reference ID")
                    );
                }
            }
            other => {
                *skipped_chunks
                    .entry(String::from_utf8_lossy(other).into_owned())
                    .or_default() += 1;
            }
        }
    }
    if report.producer.is_empty() {
        report.producer = "Unknown producer".into();
    }
    for (id, n) in &skipped_chunks {
        report
            .omissions
            .insert(format!("{id} chunks (not modelling)"), *n);
    }
    for (id, n) in &skipped_records {
        report
            .omissions
            .insert(format!("{id} records not imported"), *n);
    }
    if skipped_types > 0 {
        report.omissions.insert(
            "polygons of types other than Face, Subdivs and Catmull-Clark".into(),
            skipped_types,
        );
    }
    scene(layers, items, report, c, preset)
}

fn geometry(
    layer: &mut Layer,
    ch: &Chunk<'_>,
    tags: &[String],
    skipped_types: &mut usize,
    skipped_records: &mut BTreeMap<String, usize>,
    c: &mut Control<'_>,
) -> Result<()> {
    let mut skip = |record: String| *skipped_records.entry(record).or_default() += 1;
    let mut r = Reader::new(ch.data, ch.offset);
    let point = |r: &Reader<'_>, layer: &Layer, v: u32| {
        if (v as usize) < layer.points.len() {
            Ok(v)
        } else {
            Err(r.error("invalid_point_index", "A point index is out of range"))
        }
    };
    match &ch.id {
        b"PNTS" => {
            if !ch.data.len().is_multiple_of(12) {
                return Err(r.error("invalid_points", "PNTS size is not a multiple of 12"));
            }
            c.charge(ch.data.len() / 12 * 48)?;
            while !r.done() {
                // Q-LX-04: stored points are absolute; the pivot doesn't
                // move them.
                let p = DVec3::new(r.f32()?, r.f32()?, r.f32()?);
                layer.points.push(DVec3::new(p.x, p.y, -p.z));
            }
        }
        b"POLS" => {
            let kind = r.id()?;
            let representation = match &kind {
                b"FACE" => Some(PolygonRepresentation::Face),
                b"SUBD" => Some(PolygonRepresentation::Subdivs),
                b"PSUB" => Some(PolygonRepresentation::CatmullClark),
                _ => None,
            };
            layer.latest.clear();
            if representation.is_none() {
                // Curves (BCRV, CURV, BEZR), text, bones and other types
                // don't share the face record layout (a B-spline curve
                // stores more after its count), so their records aren't
                // parsed: the whole chunk is skipped and named in the
                // report, and its PTAGs find no polygons.
                skip(format!("POLS.{}", String::from_utf8_lossy(&kind)));
                return Ok(());
            }
            while !r.done() {
                c.check("polygons")?;
                let n = (r.u16()? & 0x3ff) as usize;
                let mut corners = Vec::with_capacity(n);
                for _ in 0..n {
                    let v = r.vx()?;
                    corners.push(point(&r, layer, v)?);
                }
                c.charge(256 + n * 64)?;
                let Some(representation) = representation else {
                    *skipped_types += 1;
                    layer.latest.push(None);
                    continue;
                };
                // Modo's corner order: the first corner, then the rest reversed.
                if corners.len() > 1 {
                    corners[1..].reverse();
                }
                layer.latest.push(Some(layer.faces.len()));
                layer.faces.push(Face {
                    vertices: corners,
                    uv: None,
                    material: "Default".into(),
                    smooth: false,
                    part: None,
                    material_label: None,
                });
                layer.representations.push(representation);
            }
        }
        b"PTAG" => {
            let kind = r.id()?;
            if !matches!(&kind, b"MATR" | b"PART") {
                // Text and smoothing tags etc. aren't imported yet.
                skip(format!("PTAG.{}", String::from_utf8_lossy(&kind)));
                return Ok(());
            }
            while !r.done() {
                let polygon = r.vx()? as usize;
                let tag = r.u16()? as usize;
                let name = tags
                    .get(tag)
                    .ok_or_else(|| r.error("invalid_tag_index", "A tag index is out of range"))?;
                let Some(Some(f)) = layer.latest.get(polygon).copied() else {
                    continue;
                };
                match &kind {
                    b"MATR" => {
                        layer.faces[f].material = name.clone();
                        layer.faces[f].material_label = Some(name.clone());
                    }
                    b"PART" => layer.faces[f].part = Some(name.clone()),
                    _ => {}
                }
            }
        }
        b"VMAP" | b"VMAD" => {
            let kind = r.id()?;
            let dimension = r.u16()? as usize;
            let name = r.string(c)?;
            let uv = &kind == b"TXUV" && dimension == 2;
            if !uv {
                skip(format!(
                    "{}.{}",
                    String::from_utf8_lossy(&ch.id),
                    String::from_utf8_lossy(&kind)
                ));
            }
            if uv {
                layer.point_uvs.entry(name.clone()).or_default();
            }
            while !r.done() {
                let v = r.vx()?;
                let v = point(&r, layer, v)?;
                let polygon = if &ch.id == b"VMAD" {
                    Some(r.vx()? as usize)
                } else {
                    None
                };
                let mut values = Vec::with_capacity(dimension);
                for _ in 0..dimension {
                    values.push(r.f32()?);
                }
                if !uv {
                    continue;
                }
                let value = DVec2::new(values[0], values[1]);
                match polygon {
                    None => {
                        layer.point_uvs.get_mut(&name).unwrap().insert(v, value);
                    }
                    Some(p) => {
                        if let Some(Some(f)) = layer.latest.get(p).copied() {
                            layer
                                .corner_uvs
                                .entry(name.clone())
                                .or_default()
                                .insert((v, f), value);
                        }
                    }
                }
            }
        }
        b"VMED" => {
            let kind = r.id()?;
            let dimension = r.u16()? as usize;
            let _name = r.string(c)?;
            while !r.done() {
                let a = r.vx()?;
                let a = point(&r, layer, a)?;
                let b = r.vx()?;
                let b = point(&r, layer, b)?;
                let mut values = Vec::with_capacity(dimension);
                for _ in 0..dimension {
                    values.push(r.f32()?);
                }
                if &kind == b"SUBV"
                    && let Some(&w) = values.first()
                {
                    layer.creases.insert((a.min(b), a.max(b)), w);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn number(item: &RawItem, key: &str) -> Option<f64> {
    match item.channels.get(key) {
        Some(Value::Number(n)) => Some(*n),
        _ => None,
    }
}

fn text<'a>(item: &'a RawItem, key: &str) -> Option<&'a str> {
    match item.channels.get(key) {
        Some(Value::Text(t)) => Some(t),
        _ => None,
    }
}

/// Q-LX-04: the note for a nonzero layer pivot (`convert::layer_pivot_note`).
fn pivot_note(mesh: &str, pivot: DVec3) -> Option<String> {
    super::convert::layer_pivot_note(mesh, pivot.to_array())
}

/// Item kinds imported as scene items. Modo's locators and group
/// locators become Group Locators, Operandi's one transform-only item
/// (provisional, LXO-AN-Q01); a mesh instance becomes a Mesh Instance.
fn scene_kind(kind: &str) -> bool {
    matches!(kind, "mesh" | "locator" | "groupLocator" | "meshInst")
}

/// An item's transform from its stack of transform items. Each of
/// position, rotation and scale is read from the stack's main ("core")
/// item of that kind; other transforms in the stack (zeroed transforms,
/// shear) are left out, with a note when they aren't identity
/// (provisional, LXO-AN-Q02).
fn item_transform(stack: &[(u32, &RawItem)], report: &mut Report) -> Transform {
    let mut transform = Transform {
        zxy: true,
        order: None,
        ..Default::default()
    };
    for kind in ["translation", "rotation", "scale"] {
        let all: Vec<&RawItem> = stack
            .iter()
            .map(|(_, t)| *t)
            .filter(|t| t.kind == kind)
            .collect();
        let Some(main) = all
            .iter()
            .find(|t| text(t, "type") == Some("core"))
            .or(all.first())
            .copied()
        else {
            continue;
        };
        let (base, default) = match kind {
            "translation" => ("pos", 0.),
            "rotation" => ("rot", 0.),
            _ => ("scl", 1.),
        };
        let read = |t: &RawItem| {
            DVec3::new(
                number(t, &format!("{base}.X")).unwrap_or(default),
                number(t, &format!("{base}.Y")).unwrap_or(default),
                number(t, &format!("{base}.Z")).unwrap_or(default),
            )
        };
        for other in all.iter().filter(|t| !std::ptr::eq(**t, main)) {
            if (read(other) - DVec3::splat(default)).abs().max_element() > 1e-9 {
                report.omit(format!("extra {kind} transforms not combined"));
            }
        }
        let v = read(main);
        match base {
            "pos" => transform.translation = v,
            "rot" => {
                transform.rotation = v;
                match text(main, "order").and_then(rotation_order) {
                    Some(order) => transform.set_rotation_order(order),
                    None if text(main, "order").is_some() => {
                        report.omit("unknown rotation orders (read as ZXY)")
                    }
                    None => {}
                }
            }
            _ => transform.scale = v,
        }
    }
    let sheared = stack.iter().any(|(_, t)| {
        t.kind == "shear"
            && ["shearXY", "shearXZ", "shearYZ"]
                .iter()
                .any(|k| number(t, k).is_some_and(|v| v.abs() > 1e-6))
    });
    if sheared {
        report.omit("shear transforms not imported");
    }
    transform
}

/// A Modo rotation order word (`zxy`).
fn rotation_order(word: &str) -> Option<crate::scene::RotationOrder> {
    use crate::scene::RotationOrder as R;
    Some(match word {
        "xyz" => R::Xyz,
        "xzy" => R::Xzy,
        "yxz" => R::Yxz,
        "yzx" => R::Yzx,
        "zxy" => R::Zxy,
        "zyx" => R::Zyx,
        _ => return None,
    })
}

fn scene(
    layers: Vec<Layer>,
    items: BTreeMap<u32, RawItem>,
    mut report: Report,
    c: &mut Control<'_>,
    preset: bool,
) -> Result<Candidate> {
    let mut loose = Vec::new();
    // Layers by the mesh ITEM that owns them (resolved after the whole file).
    let mut by_item: BTreeMap<u32, Layer> = BTreeMap::new();
    for layer in layers {
        match layer.item {
            Some(id) if items.get(&id).is_some_and(|i| i.kind == "mesh") => {
                if by_item.insert(id, layer).is_some() {
                    return Err(err(
                        "ambiguous_layer_owner",
                        "Two layers name the same mesh ITEM",
                    ));
                }
            }
            _ if preset => loose.push(layer),
            _ => report.omit("mesh layers without a mesh item"),
        }
    }
    // Transform items by the item they move (LINK graph xfrmCore), in
    // stack order.
    let mut stacks: BTreeMap<u32, Vec<(u32, &RawItem)>> = BTreeMap::new();
    for item in items.values() {
        if !matches!(
            item.kind.as_str(),
            "translation" | "rotation" | "scale" | "shear"
        ) {
            continue;
        }
        for (graph, target, ordinal) in &item.links {
            if graph == "xfrmCore" {
                stacks.entry(*target).or_default().push((*ordinal, item));
            }
        }
    }
    for stack in stacks.values_mut() {
        stack.sort_by_key(|(ordinal, _)| *ordinal);
    }
    let skipped: BTreeSet<&str> = items
        .values()
        .map(|i| i.kind.as_str())
        .filter(|k| !scene_kind(k) && !matches!(*k, "translation" | "rotation" | "scale" | "shear"))
        .collect();
    for kind in skipped {
        report.omit(format!("{kind} items (not modelling)"));
    }
    let mut scene = Scene::default();
    let imported: Vec<u32> = items
        .iter()
        .filter(|(_, i)| scene_kind(&i.kind))
        .map(|(&id, _)| id)
        .collect();
    let mut locators = 0usize;
    for &id in &imported {
        c.check("scene conversion")?;
        let item = &items[&id];
        let transform = item_transform(stacks.get(&id).map_or(&[][..], |s| s), &mut report);
        // `allOff` hides the item and everything under it (GV-01).
        let (visible, hides_children) = match text(item, "visible") {
            None | Some("default") => (true, false),
            Some("off") => (false, false),
            Some("allOff") => (false, true),
            Some(_) => {
                report.omit("visibility values other than default and off (imported visible)");
                (true, false)
            }
        };
        let parent = item
            .links
            .iter()
            .find(|(graph, target, _)| {
                graph == "parent" && items.get(target).is_some_and(|i| scene_kind(&i.kind))
            })
            .map(|(_, target, _)| u64::from(*target) + 1);
        // A Mesh Instance shows the mesh its `source` link names.
        let instance_of = (item.kind == "meshInst")
            .then(|| {
                item.links
                    .iter()
                    .find(|(graph, target, _)| {
                        graph == "source" && items.get(target).is_some_and(|i| i.kind == "mesh")
                    })
                    .map(|(_, target, _)| u64::from(*target) + 1)
            })
            .flatten();
        let locator = matches!(item.kind.as_str(), "locator" | "groupLocator")
            .then_some(crate::locator::LocatorKind::Group);
        locators += usize::from(locator.is_some());
        let layer = by_item.remove(&id).unwrap_or_default();
        let name = if item.name.is_empty() {
            item.visible_name
                .clone()
                .unwrap_or_else(|| layer.name.clone())
        } else {
            item.name.clone()
        };
        let mesh = if item.kind == "mesh" {
            report.notes.extend(pivot_note(&name, layer.pivot));
            let (mesh, notes) = build_mesh(layer, &name)?;
            report.notes.extend(notes);
            mesh
        } else {
            if item.kind == "meshInst" && instance_of.is_none() {
                report.omit("mesh instances without a source mesh (imported empty)");
            }
            Mesh::default()
        };
        scene.items.push(Item {
            id: u64::from(id) + 1,
            name,
            source_id: Some(id),
            source_display_name: item.visible_name.clone(),
            subdivision: None,
            backdrop: None,
            camera: None,
            light: None,
            texture: None,
            locator,
            environment: None,
            common: Default::default(),
            elements: Default::default(),
            hides_children,
            mesh_stack: None,
            kit: None,
            instance_of,
            replicator: None,
            visible,
            parent,
            transform,
            mesh: mesh.into(),
        });
    }
    // A mesh preset's layers without an item: meshes in place, named after
    // the layer.
    let first = items.keys().max().map_or(1, |m| u64::from(*m) + 2);
    for (next, layer) in (first..).zip(loose) {
        c.check("scene conversion")?;
        let name = if layer.name.is_empty() {
            "Mesh".to_string()
        } else {
            layer.name.clone()
        };
        report.notes.extend(pivot_note(&name, layer.pivot));
        let (mesh, note) = build_mesh(layer, &name)?;
        report.notes.extend(note);
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
                order: None,
                ..Default::default()
            },
            mesh: mesh.into(),
        });
    }
    if locators > 0 {
        report.notes.push(format!(
            "{locators} locators and group locators imported as Group Locators"
        ));
    }
    scene.sync_instances(None);
    // A parent link to an item that isn't imported (or a cycle) drops the link.
    let ids: BTreeSet<u64> = scene.items.iter().map(|i| i.id).collect();
    for i in 0..scene.items.len() {
        let mut seen = BTreeSet::new();
        let mut at = scene.items[i].parent;
        while let Some(p) = at {
            if !ids.contains(&p) || !seen.insert(p) || p == scene.items[i].id {
                report.omit("parent links that loop or point outside the import");
                scene.items[i].parent = None;
                break;
            }
            at = scene.item(p).and_then(|x| x.parent);
        }
    }
    scene.next_id = scene.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    let first = scene
        .items
        .iter()
        .find(|i| i.visible)
        .or(scene.items.first())
        .map(|i| i.id);
    scene.select_item(first);
    c.check("candidate validation")?;
    scene.validate_for_import().map_err(|e| match e {
        crate::scene::SceneValidationError::FiniteTransformRange(detail) => {
            err("numeric_range_or_composition_error", &detail)
        }
        crate::scene::SceneValidationError::Other(detail) => err("invalid_candidate", &detail),
    })?;
    let mut uv = Vec::new();
    let mut materials = BTreeSet::new();
    let mut parts = BTreeSet::new();
    for item in scene
        .items
        .iter()
        .filter(|i| i.is_mesh() && i.instance_of.is_none())
    {
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
    c.check("native persistence preflight")?;
    crate::persistence::encode(&scene).map_err(|e| err("native_persistence_limit", &e))?;
    c.check("ready to commit")?;
    Ok(Candidate { scene, report })
}

/// Builds one layer's mesh, with notes on what was dropped or marked.
fn build_mesh(layer: Layer, name: &str) -> Result<(Mesh, Vec<String>)> {
    let mut mesh = Mesh {
        vertices: layer.points,
        faces: layer.faces.into(),
        ..Default::default()
    };
    // Every mesh carries a "Texture" UV map, empty when the file has none.
    let mut maps: BTreeSet<String> = layer.point_uvs.keys().cloned().collect();
    maps.extend(layer.corner_uvs.keys().cloned());
    maps.insert("Texture".into());
    for name in maps {
        let points = layer.point_uvs.get(&name);
        let corners = layer.corner_uvs.get(&name);
        let rows = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(f, face)| {
                face.vertices
                    .iter()
                    .map(|&v| {
                        corners
                            .and_then(|m| m.get(&(v, f)))
                            .or_else(|| points.and_then(|m| m.get(&v)))
                            .copied()
                    })
                    .collect()
            })
            .collect();
        mesh.uv_maps.insert(name, rows);
    }
    if mesh
        .faces
        .iter()
        .zip(&layer.representations)
        .any(|(_, r)| *r != PolygonRepresentation::Face)
    {
        mesh.attributes.representations = layer.representations;
    }
    // Edge weights are kept only on polygon edges. Modo also stores them
    // on edges no polygon has (curve spans, edges of deleted polygons,
    // a point paired with itself); those have nothing to crease.
    let edges: BTreeSet<(u32, u32)> = if layer.creases.is_empty() {
        BTreeSet::new()
    } else {
        mesh.edges().into_iter().collect()
    };
    let mut stray = 0usize;
    mesh.attributes.creases = layer
        .creases
        .into_iter()
        .filter(|&(_, w)| w != 0.)
        .filter(|&(edge, w)| {
            let keep = w.is_finite() && edges.contains(&edge);
            stray += usize::from(!keep);
            keep
        })
        .map(|((a, b), weight)| MeshCrease { a, b, weight })
        .collect();
    // Modo keeps zero-area polygons and non-manifold edges; so does
    // Operandi, marked the way every other importer marks them.
    let mut notes = Vec::new();
    if stray > 0 {
        notes.push(format!(
            "{name}: {stray} edge weights on edges no polygon has were not imported"
        ));
    }
    crate::interchange::finish_mesh(&mut mesh, name, &mut notes).map_err(|e| {
        err(
            "invalid_mesh",
            &format!("A mesh could not be imported: {e}"),
        )
    })?;
    Ok((mesh, notes))
}
