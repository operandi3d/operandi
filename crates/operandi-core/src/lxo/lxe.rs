//! Import of the LXE profile: the minimal layout Operandi's LXO export writes
//! (`specifications/lxo-export-2026-11-03`, BEHAVIOR-IMPORT-PROFILE.md,
//! LXE-IMP-01 to 06). It is a third producer profile beside Modo 16.0v4 and
//! 17.0v8, recognized only when a file matches every LXE-IMP-01 rule. A file
//! that matches only some of them is handed to the Modo profile rules
//! unchanged; the two rule sets are never mixed in one file.
use super::wire::{Chunk, Reader};
use super::{Candidate, Control, Error, Report, Result};
use crate::{Face, Mesh, Scene};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet};

/// LXE-04's 22-byte LAYR tail (shared with the writer).
use super::export::LAYR_TAIL;

const MAX_POINTS: usize = 5_000_000;
const MAX_POLYGONS: usize = 5_000_000;
const MAX_CORNERS: usize = 20_000_000;
const MAX_MAPS: usize = 64;
const MAX_ITEMS: usize = crate::MAX_SCENE_ITEMS;

/// Why decoding stopped: the file isn't in this profile (try the Modo rules),
/// or it is and something in it is wrong (refuse).
enum Fail {
    NotProfile,
    Refuse(Error),
}
impl From<Error> for Fail {
    fn from(e: Error) -> Self {
        Fail::Refuse(e)
    }
}
type Step<T> = std::result::Result<T, Fail>;

/// `None` when the file is not in the LXE profile.
pub(super) fn read(chunks: &[Chunk<'_>], c: &mut Control<'_>) -> Result<Option<Candidate>> {
    match decode(chunks, c) {
        Ok(candidate) => Ok(Some(candidate)),
        Err(Fail::NotProfile) => Ok(None),
        Err(Fail::Refuse(e)) => Err(e),
    }
}

fn refuse<T>(chunk: &Chunk<'_>, category: &'static str, detail: &str) -> Step<T> {
    Err(Fail::Refuse(Error::new(category, chunk.offset, detail)))
}

/// Structural checks that decide the profile: a parse failure here means the
/// file is not LXE, so the Modo rules get it.
fn shape<T>(r: Result<T>) -> Step<T> {
    r.map_err(|_| Fail::NotProfile)
}

fn require(ok: bool) -> Step<()> {
    if ok { Ok(()) } else { Err(Fail::NotProfile) }
}

#[derive(Default)]
struct Layer {
    item: u32,
    mesh: Mesh,
    points: bool,
    polygons: bool,
    /// UV map name → continuous point values (VMAP).
    vmap: BTreeMap<String, BTreeMap<u32, DVec2>>,
    /// UV map name → (point, polygon, value) seam records (VMAD).
    vmad: BTreeMap<String, Vec<(u32, u32, DVec2)>>,
    tagged: BTreeSet<([u8; 4], u32)>,
}

struct Item {
    kind: String,
    name: String,
    parent: Option<u32>,
    /// (target mesh id, ordinal) of an `xfrmCore` link.
    core: Option<(u32, u32)>,
    /// The COLR item tag (Editor Color, IC-Q1), if any.
    color: Option<crate::EditorColor>,
}

fn decode(chunks: &[Chunk<'_>], c: &mut Control<'_>) -> Step<Candidate> {
    // LXE-IMP-01 header: VRSN (16,0), APPV (17,4001,1,any), ENCO 2, first and once.
    require(chunks.len() >= 3)?;
    let ids: Vec<&[u8; 4]> = chunks.iter().map(|ch| &ch.id).collect();
    require(ids[..3] == [b"VRSN", b"APPV", b"ENCO"])?;
    require(
        chunks[3..]
            .iter()
            .all(|ch| !matches!(&ch.id, b"VRSN" | b"APPV" | b"ENCO")),
    )?;
    let mut r = Reader::new(chunks[0].data, chunks[0].offset);
    require(shape(r.u32())? == 16 && shape(r.u32())? == 0)?;
    let mut r = Reader::new(chunks[1].data, chunks[1].offset);
    let words = [
        shape(r.u32())?,
        shape(r.u32())?,
        shape(r.u32())?,
        shape(r.u32())?,
    ];
    require(words[..3] == [17, 4001, 1])?;
    let producer = shape(r.string(c))?;
    let mut r = Reader::new(chunks[2].data, chunks[2].offset);
    require(shape(r.u32())? == 2 && r.done())?;
    // Only the listed chunks, in LXE-IMP-04 order: tables, geometry, items, setup.
    let rank = |id: &[u8; 4]| match id {
        b"CHNM" | b"TAGS" => Some(0),
        b"LAYR" | b"PNTS" | b"VMAP" | b"POLS" | b"VMAD" | b"PTAG" => Some(1),
        b"ITEM" => Some(2),
        b"ACTN" => Some(3),
        _ => None,
    };
    let mut last = 0;
    for ch in &chunks[3..] {
        let rank = rank(&ch.id).ok_or(Fail::NotProfile)?;
        require(rank >= last)?;
        last = rank;
    }
    let count = |id: &[u8; 4]| chunks.iter().filter(|ch| &ch.id == id).count();
    require(count(b"CHNM") <= 1 && count(b"TAGS") <= 1 && count(b"ACTN") <= 1)?;
    let mut report = Report {
        producer: format!("{producer} (Operandi LXO export profile)"),
        ..Default::default()
    };

    // Geometry groups (LXE-04 to LXE-08).
    let mut names: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    let mut layers: Vec<Layer> = Vec::new();
    let (mut total_points, mut total_polygons, mut total_corners) = (0usize, 0usize, 0usize);
    for ch in &chunks[3..] {
        c.check("LXE geometry")?;
        let mut r = Reader::new(ch.data, ch.offset);
        match &ch.id {
            b"CHNM" => {
                let n = r.u32()? as usize;
                if n > ch.data.len() / 2 {
                    return refuse(ch, "resource_limit", "CHNM count exceeds payload");
                }
                for _ in 0..n {
                    names.push(r.string(c)?);
                }
                r.end()?;
            }
            b"TAGS" => {
                while !r.done() {
                    tags.push(r.string(c)?);
                }
            }
            b"LAYR" => {
                // LXE-04, byte for byte apart from ordinal, name and item id.
                shape(r.u16())?;
                require(shape(r.u16())? == 5)?;
                for _ in 0..3 {
                    require(shape(r.u32())? == 0)?;
                }
                shape(r.string(c))?;
                require(shape(r.take(2))? == [0xff, 0xff])?;
                require(shape(r.take(8))? == [0x40, 0, 0, 0, 0x3d, 0xb2, 0xb8, 0xc2])?;
                for v in [1.0f32, 0., 0., 0., 1., 0., 0., 0., 1.] {
                    require(shape(r.take(4))? == v.to_be_bytes())?;
                }
                let item = shape(r.u32())?;
                require(shape(r.take(22))? == LAYR_TAIL && r.done())?;
                if layers.len() >= MAX_ITEMS {
                    return refuse(ch, "resource_limit", "Too many meshes");
                }
                layers.push(Layer {
                    item,
                    ..Default::default()
                });
            }
            _ if rank(&ch.id) == Some(1) => {
                let layer = match layers.last_mut() {
                    Some(l) => l,
                    None => return refuse(ch, "missing_geometry_scope", "Geometry before LAYR"),
                };
                geometry(
                    ch,
                    &mut r,
                    layer,
                    &tags,
                    c,
                    &mut total_points,
                    &mut total_polygons,
                    &mut total_corners,
                )?;
            }
            _ => {}
        }
    }
    if count(b"CHNM") == 1 && count(b"ACTN") == 0 {
        return refuse(
            &chunks[3],
            "unreferenced_table",
            "CHNM without a setup action",
        );
    }
    if count(b"TAGS") == 1 && count(b"PTAG") == 0 {
        return refuse(
            &chunks[3],
            "unreferenced_table",
            "TAGS without polygon tags",
        );
    }

    // Items (LXE-09 to LXE-11).
    let mut items: BTreeMap<u32, Item> = BTreeMap::new();
    let mut renders = 0;
    for ch in chunks.iter().filter(|ch| &ch.id == b"ITEM") {
        c.check("LXE items")?;
        let mut r = Reader::new(ch.data, ch.offset);
        let kind = shape(r.string(c))?;
        require(matches!(
            kind.as_str(),
            "mesh" | "translation" | "rotation" | "scale" | "polyRender"
        ))?;
        let name = r.string(c)?;
        let id = r.u32()?;
        let subs = shape(r.chunks(true, c))?;
        let mut item = Item {
            kind,
            name,
            parent: None,
            core: None,
            color: None,
        };
        let mut colr = 0;
        let mut layr = 0;
        for sub in &subs {
            let mut s = Reader::new(sub.data, sub.offset);
            match (&sub.id, item.kind.as_str()) {
                (b"LAYR", "mesh") => {
                    require(sub.data.len() == 12)?;
                    layr += 1;
                }
                (b"LINK", kind) => {
                    let graph = shape(s.string(c))?;
                    let target = shape(s.u32())?;
                    let ordinal = shape(s.u32())?;
                    require(s.done())?;
                    match (graph.as_str(), kind) {
                        ("parent", "mesh") if item.parent.is_none() && ordinal == 0 => {
                            item.parent = Some(target)
                        }
                        ("xfrmCore", "scale" | "rotation" | "translation")
                            if item.core.is_none() =>
                        {
                            let expected = ["scale", "rotation", "translation"]
                                .iter()
                                .position(|k| *k == kind)
                                .unwrap() as u32;
                            require(ordinal == expected)?;
                            item.core = Some((target, ordinal));
                        }
                        _ => return Err(Fail::NotProfile),
                    }
                }
                // Editor Color: one COLR item tag; unknown or "none" names
                // read as none.
                (b"ITAG", "mesh") => {
                    let id = shape(s.id())?;
                    let value = shape(s.string(c))?;
                    require(s.done() && &id == b"COLR")?;
                    colr += 1;
                    require(colr == 1)?;
                    item.color = crate::EditorColor::from_name(&value);
                }
                _ => return Err(Fail::NotProfile),
            }
        }
        require(match item.kind.as_str() {
            "mesh" => layr == 1,
            // LXE-16: at most one render item, bare (no subchunks), ignored.
            "polyRender" => {
                renders += 1;
                subs.is_empty() && renders == 1
            }
            _ => item.core.is_some(),
        })?;
        if items.insert(id, item).is_some() {
            return refuse(ch, "duplicate_item_id", "Repeated ITEM id");
        }
    }

    // Setup channels (LXE-12, LXE-IMP-02): the only source of values.
    let mut values: BTreeMap<u32, BTreeMap<String, Value>> = BTreeMap::new();
    if let Some(ch) = chunks.iter().find(|ch| &ch.id == b"ACTN") {
        let mut r = Reader::new(ch.data, ch.offset);
        require(shape(r.string(c))? == "setup" && shape(r.string(c))? == "setup")?;
        shape(r.u32())?;
        let mut owner = None;
        for sub in shape(r.chunks(true, c))? {
            c.check("LXE channels")?;
            let mut s = Reader::new(sub.data, sub.offset);
            match &sub.id {
                b"ITEM" => {
                    let id = s.u32()?;
                    s.end()?;
                    if !items.contains_key(&id) {
                        return refuse(
                            ch,
                            "invalid_item_reference",
                            "Setup channel owner is not an item",
                        );
                    }
                    owner = Some(id);
                }
                b"CHAN" => {
                    let Some(id) = owner else {
                        return refuse(ch, "missing_channel_owner", "CHAN before any ITEM");
                    };
                    let name = names.get(s.vx()? as usize).cloned().ok_or_else(|| {
                        Error::new(
                            "invalid_channel_reference",
                            sub.offset,
                            "CHNM index out of range",
                        )
                    })?;
                    let kind = s.u16()?;
                    if s.vx()? != 0 {
                        return refuse(
                            ch,
                            "animated_modeling_dependency",
                            "Envelope on a setup channel",
                        );
                    }
                    let value = match kind {
                        2 => Value::Float(s.f32()?),
                        3 => Value::Text(s.string(c)?),
                        _ => {
                            return refuse(
                                ch,
                                "unsupported_channel_type",
                                "Setup channels are F32 or S0",
                            );
                        }
                    };
                    s.end()?;
                    let allowed = match items[&id].kind.as_str() {
                        "mesh" => &["visible"][..],
                        "translation" => &["type", "pos.X", "pos.Y", "pos.Z"],
                        "rotation" => &["type", "order", "rot.X", "rot.Y", "rot.Z"],
                        "scale" => &["type", "scl.X", "scl.Y", "scl.Z"],
                        _ => &[],
                    };
                    if !allowed.contains(&name.as_str()) {
                        return refuse(
                            ch,
                            "unknown_modeling_channel",
                            &format!("Unsupported channel {name}"),
                        );
                    }
                    if values
                        .entry(id)
                        .or_default()
                        .insert(name.clone(), value)
                        .is_some()
                    {
                        return refuse(
                            ch,
                            "duplicate_action_channel",
                            &format!("Repeated channel {name}"),
                        );
                    }
                }
                _ => return Err(Fail::NotProfile),
            }
        }
    }

    // Build the scene in layer order (LXE-O-12 shows names in layer order).
    let mut scene = Scene::default();
    let mut scene_id: BTreeMap<u32, u64> = BTreeMap::new();
    let owners: BTreeSet<u32> = layers.iter().map(|l| l.item).collect();
    if owners.len() != layers.len()
        || items
            .iter()
            .any(|(id, i)| i.kind == "mesh" && !owners.contains(id))
    {
        return Err(Error::new(
            "invalid_layer_item_reference",
            0,
            "LAYR and mesh ITEMs must match one to one",
        )
        .into());
    }
    for mut layer in layers {
        c.check("LXE maps")?;
        finish_maps(&mut layer)?;
        let item = items
            .get(&layer.item)
            .filter(|i| i.kind == "mesh")
            .ok_or_else(|| {
                Error::new(
                    "invalid_layer_item_reference",
                    0,
                    "LAYR must resolve to a mesh ITEM",
                )
            })?;
        let mut mesh = layer.mesh;
        mesh.attributes.extrude_edge_uses = crate::interchange::edge_exceptions(&mesh);
        mesh.validate()
            .map_err(|e| Error::new("invalid_mesh", 0, &format!("{}: {e}", item.name)))?;
        scene
            .new_mesh()
            .map_err(|e| Error::new("resource_limit", 0, &e))?;
        let new = scene.items.last_mut().unwrap();
        new.name = item.name.clone();
        new.common.editor_color = item.color;
        new.transform.zxy = true;
        new.visible = match values.get(&layer.item).and_then(|v| v.get("visible")) {
            None => true,
            Some(Value::Text(t)) if t == "off" => false,
            Some(Value::Text(t)) if t == "default" => true,
            _ => {
                return Err(
                    Error::new("unsupported_visibility", 0, "visible is off or default").into(),
                );
            }
        };
        report.meshes += 1;
        report.points += mesh.vertices.len();
        report.polygons += mesh.faces.len();
        report.corners += mesh.faces.iter().map(|f| f.vertices.len()).sum::<usize>();
        report.hidden += usize::from(!new.visible);
        for name in mesh.uv_maps.keys() {
            if !report.uv_maps.contains(name) {
                report.uv_maps.push(name.clone());
            }
        }
        new.mesh = mesh.into();
        scene_id.insert(layer.item, new.id);
    }
    let mut stacks: BTreeSet<(u32, u32)> = BTreeSet::new();
    for (id, item) in &items {
        if let Some(parent) = item.parent {
            let parent = *scene_id.get(&parent).ok_or_else(|| {
                Error::new(
                    "invalid_parent_reference",
                    0,
                    "Parent is not an imported mesh",
                )
            })?;
            let child = scene_id[id];
            scene.item_mut(child).unwrap().parent = Some(parent);
        }
        let Some((target, ordinal)) = item.core else {
            continue;
        };
        let mesh = *scene_id.get(&target).ok_or_else(|| {
            Error::new(
                "invalid_transform_reference",
                0,
                "Transform target is not a mesh",
            )
        })?;
        if !stacks.insert((target, ordinal)) {
            return Err(Error::new("duplicate_transform", 0, "Repeated transform ordinal").into());
        }
        let v = values.get(id).cloned().unwrap_or_default();
        if v.get("type")
            .is_some_and(|t| *t != Value::Text("core".into()))
        {
            return Err(Error::new(
                "unsupported_transform_type",
                0,
                "Transform type must be core",
            )
            .into());
        }
        // LXE-IMP-03: absent components are 0 (pos, rot) or 1 (scl).
        let axis = |base: &str, absent: f64| -> Step<DVec3> {
            let mut out = [absent; 3];
            for (o, a) in out.iter_mut().zip(["X", "Y", "Z"]) {
                match v.get(&format!("{base}.{a}")) {
                    None => {}
                    Some(Value::Float(f)) => *o = *f,
                    Some(_) => {
                        return Err(Error::new(
                            "unsupported_channel_type",
                            0,
                            "Transform values are F32",
                        )
                        .into());
                    }
                }
            }
            Ok(DVec3::from_array(out))
        };
        let target = scene.item_mut(mesh).unwrap();
        match ordinal {
            0 => target.transform.scale = axis("scl", 1.0)?,
            1 => {
                if v.get("order") != Some(&Value::Text("zxy".into())) {
                    return Err(Error::new(
                        "unsupported_rotation_order",
                        0,
                        "Rotation order must be zxy",
                    )
                    .into());
                }
                target.transform.rotation = axis("rot", 0.0)?;
            }
            _ => target.transform.translation = axis("pos", 0.0)?,
        }
    }
    // Labels for the result dialog.
    let mut materials = BTreeSet::new();
    let mut parts = BTreeSet::new();
    for item in &scene.items {
        for f in &item.mesh.faces {
            materials.extend(f.material_label.clone());
            parts.extend(f.part.clone());
        }
    }
    report.materials = materials.into_iter().collect();
    report.parts = parts.into_iter().collect();
    let first = scene.items.first().map(|i| i.id);
    scene.select_item(first);
    scene
        .validate()
        .map_err(|e| Error::new("invalid_scene", 0, &e))?;
    Ok(Candidate { scene, report })
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Float(f64),
    Text(String),
}

/// PNTS, POLS, VMAP, VMAD and PTAG for the current layer.
#[allow(clippy::too_many_arguments)]
fn geometry(
    ch: &Chunk<'_>,
    r: &mut Reader<'_>,
    layer: &mut Layer,
    tags: &[String],
    c: &mut Control<'_>,
    total_points: &mut usize,
    total_polygons: &mut usize,
    total_corners: &mut usize,
) -> Step<()> {
    match &ch.id {
        b"PNTS" => {
            if layer.points || layer.polygons {
                return refuse(
                    ch,
                    "unsupported_repeated_geometry_block",
                    "PNTS must come once, before POLS",
                );
            }
            layer.points = true;
            if !ch.data.len().is_multiple_of(12) {
                return refuse(
                    ch,
                    "invalid_point_payload_size",
                    "PNTS must contain F32 triples",
                );
            }
            let n = ch.data.len() / 12;
            *total_points += n;
            if *total_points > MAX_POINTS {
                return refuse(ch, "resource_limit", "Point budget");
            }
            c.charge(n * 24)?;
            let mesh = &mut layer.mesh;
            mesh.vertices.reserve_exact(n);
            for _ in 0..n {
                // LXE-IMP-05: file space negates Z.
                let (x, y, z) = (r.f32()?, r.f32()?, r.f32()?);
                mesh.vertices.push(DVec3::new(x, y, -z));
            }
        }
        b"POLS" => {
            if layer.polygons {
                return refuse(ch, "unsupported_repeated_geometry_block", "Repeated POLS");
            }
            layer.polygons = true;
            if r.id()? != *b"FACE" {
                return Err(Fail::NotProfile);
            }
            let mesh = &mut layer.mesh;
            while !r.done() {
                c.check("LXE polygons")?;
                let n = r.u16()? as usize;
                if !(3..=1023).contains(&n) {
                    return refuse(
                        ch,
                        "unsupported_polygon",
                        "FACE polygons have 3 to 1023 corners",
                    );
                }
                *total_polygons += 1;
                *total_corners += n;
                if *total_polygons > MAX_POLYGONS || *total_corners > MAX_CORNERS {
                    return refuse(ch, "resource_limit", "Polygon budget");
                }
                c.charge(64 + n * 8)?;
                let mut vertices = Vec::with_capacity(n);
                for _ in 0..n {
                    let v = r.vx()?;
                    if v as usize >= mesh.vertices.len() {
                        return refuse(ch, "invalid_point_reference", "Polygon point out of range");
                    }
                    vertices.push(v);
                }
                // LXE-IMP-05: reverse keeping the first corner.
                vertices[1..].reverse();
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
        b"VMAP" | b"VMAD" => {
            if r.id()? != *b"TXUV" || r.u16()? != 2 {
                return refuse(
                    ch,
                    "unsupported_map",
                    "Only 2D TXUV maps are in this profile",
                );
            }
            let name = r.string(c)?;
            let vmad = ch.id == *b"VMAD";
            if vmad && !layer.polygons {
                return refuse(ch, "map_before_polygons", "VMAD must follow POLS");
            }
            let repeat = if vmad {
                layer.vmad.contains_key(&name)
            } else {
                layer.vmap.contains_key(&name)
            };
            if repeat {
                return refuse(ch, "duplicate_map", "Repeated map");
            }
            let mut known: BTreeSet<&String> = layer.vmap.keys().collect();
            known.extend(layer.vmad.keys());
            if !known.contains(&name) && known.len() >= MAX_MAPS {
                return refuse(ch, "resource_limit", "64 maps per mesh");
            }
            let mut points = BTreeMap::new();
            let mut seams = Vec::new();
            while !r.done() {
                c.check("LXE maps")?;
                let point = r.vx()?;
                let poly = if vmad { Some(r.vx()?) } else { None };
                let uv = DVec2::new(r.f32()?, r.f32()?);
                if point as usize >= layer.mesh.vertices.len() {
                    return refuse(ch, "invalid_map_reference", "Map point out of range");
                }
                c.charge(48)?;
                match poly {
                    Some(f) => seams.push((point, f, uv)),
                    None => {
                        if points.insert(point, uv).is_some() {
                            return refuse(ch, "duplicate_map_record", "Repeated VMAP point");
                        }
                    }
                }
            }
            if vmad {
                layer.vmad.insert(name, seams);
            } else {
                layer.vmap.insert(name, points);
            }
        }
        b"PTAG" => {
            let kind = r.id()?;
            if !layer.polygons || !matches!(&kind, b"MATR" | b"PART") {
                return refuse(
                    ch,
                    "unsupported_polygon_tag",
                    "PTAG is MATR or PART after POLS",
                );
            }
            while !r.done() {
                let f = r.vx()?;
                let label = tags
                    .get(r.u16()? as usize)
                    .ok_or_else(|| {
                        Error::new(
                            "invalid_tag_reference",
                            ch.offset,
                            "TAGS index out of range",
                        )
                    })?
                    .clone();
                if !layer.tagged.insert((kind, f)) {
                    return refuse(ch, "duplicate_polygon_membership", "Repeated polygon tag");
                }
                let face = layer.mesh.faces.get_mut(f as usize).ok_or_else(|| {
                    Error::new(
                        "invalid_tag_reference",
                        ch.offset,
                        "PTAG polygon out of range",
                    )
                })?;
                if kind == *b"MATR" {
                    face.material = label.clone();
                    face.material_label = Some(label);
                } else {
                    face.part = Some(label);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Build a layer's UV maps once all its chunks are read: VMAP values go to
/// every corner on that point, then VMAD records override single corners
/// (LXE-IMP-03: without VMAD, every corner takes its point's value).
fn finish_maps(layer: &mut Layer) -> Result<()> {
    let mesh = &mut layer.mesh;
    let mut names: BTreeSet<String> = layer.vmap.keys().cloned().collect();
    names.extend(layer.vmad.keys().cloned());
    for name in names {
        let mut map: Vec<Vec<Option<DVec2>>> = mesh
            .faces
            .iter()
            .map(|f| vec![None; f.vertices.len()])
            .collect();
        if let Some(points) = layer.vmap.get(&name) {
            for (f, face) in mesh.faces.iter().enumerate() {
                for (corner, v) in face.vertices.iter().enumerate() {
                    map[f][corner] = points.get(v).copied();
                }
            }
        }
        let mut seen = BTreeSet::new();
        for &(point, poly, uv) in layer.vmad.get(&name).into_iter().flatten() {
            let face = mesh.faces.get(poly as usize).ok_or_else(|| {
                Error::new("invalid_map_reference", 0, "VMAD polygon out of range")
            })?;
            let corner = face
                .vertices
                .iter()
                .position(|&v| v == point)
                .ok_or_else(|| {
                    Error::new(
                        "point_not_on_polygon",
                        0,
                        "VMAD point is not on its polygon",
                    )
                })?;
            if !seen.insert((poly, point)) {
                return Err(Error::new(
                    "duplicate_map_record",
                    0,
                    "Repeated VMAD corner",
                ));
            }
            map[poly as usize][corner] = Some(uv);
        }
        mesh.uv_maps.insert(name, map);
    }
    Ok(())
}
