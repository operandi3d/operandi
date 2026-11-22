//! LXO export for Modo 17.1v1, written from `specifications/lxo-export-2026-11-03`
//! (BEHAVIOR-WRITER.md, LXE-00 to LXE-15) on top of the import spec's
//! encodings. LXE-00: Modo crashes rather than refusing a bad file, so this
//! writes exactly the verified layout and nothing experimental. Anything the
//! spec doesn't cover (empty meshes, polylines, degenerate polygons, maps
//! other than UVs) is left out and listed in the notes, never improvised.
use crate::{Mesh, Result, Scene};
use glam::{DQuat, DVec2, EulerRot};
use std::collections::BTreeMap;

const PRODUCER: &str = "Operandi";
/// LXE-04: the fixed tail Modo 17.1v1 writes after the item id. Any other
/// bytes crashed Modo.
pub(super) const LAYR_TAIL: [u8; 22] = [
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x02,
    0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
];
/// LXE-06 corner-count limit for a FACE polygon.
const MAX_CORNERS: usize = 1023;
/// Import-spec budgets; a file we write should also be one we would read.
const MAX_POINTS: usize = 5_000_000;
const MAX_POLYGONS: usize = 5_000_000;
const MAX_ITEMS: usize = crate::MAX_SCENE_ITEMS;

/// Big-endian writer for the IFF-style primitives in format/01-container.md.
#[derive(Default)]
struct Bytes(Vec<u8>);
impl Bytes {
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn f32(&mut self, v: f64) {
        self.0.extend_from_slice(&(v as f32).to_be_bytes());
    }
    fn id(&mut self, id: &[u8; 4]) {
        self.0.extend_from_slice(id);
    }
    /// NUL-terminated, padded to an even length from the string's start.
    fn s0(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(0);
        if (s.len() + 1) % 2 == 1 {
            self.0.push(0);
        }
    }
    /// Two bytes below 0xFF00, else FF plus 24 value bits.
    fn vx(&mut self, v: u32) {
        if v < 0xFF00 {
            self.u16(v as u16);
        } else {
            self.u32(0xFF00_0000 | v);
        }
    }
    fn chunk(&mut self, id: &[u8; 4], payload: &Bytes) -> Result<()> {
        let len = u32::try_from(payload.0.len()).map_err(|_| "LXO chunk exceeds 4 GiB")?;
        self.id(id);
        self.u32(len);
        self.0.extend_from_slice(&payload.0);
        if len % 2 == 1 {
            self.0.push(0);
        }
        Ok(())
    }
    fn sub(&mut self, id: &[u8; 4], payload: &Bytes) -> Result<()> {
        let len = u16::try_from(payload.0.len()).map_err(|_| "LXO subchunk exceeds 64 KiB")?;
        self.id(id);
        self.u16(len);
        self.0.extend_from_slice(&payload.0);
        if len % 2 == 1 {
            self.0.push(0);
        }
        Ok(())
    }
}

/// Strings are S0, so an embedded NUL would end them early.
fn text(s: &str) -> String {
    s.replace('\0', "")
}

/// One item's geometry, filtered to what the spec covers.
struct Layer {
    name: String,
    points: Vec<glam::DVec3>,
    polygons: Vec<Vec<u32>>,
    /// Per kept polygon: (material label, part label).
    labels: Vec<(Option<String>, Option<String>)>,
    /// Map name → per kept polygon, per corner UV.
    uvs: BTreeMap<String, Vec<Vec<Option<DVec2>>>>,
}

fn layer(name: &str, mesh: &Mesh, notes: &mut Vec<String>) -> Result<Layer> {
    let mut kept = Vec::new();
    let (mut lines, mut degenerate, mut subdivision) = (0, 0, 0);
    for (f, face) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(f) {
            lines += 1;
        } else if mesh.attributes.degenerate_faces.contains(&(f as u32)) {
            degenerate += 1;
        } else {
            if face.vertices.len() > MAX_CORNERS {
                return Err(format!(
                    "{name}: a polygon has {} corners; LXO export allows {MAX_CORNERS}",
                    face.vertices.len()
                ));
            }
            if !matches!(
                mesh.attributes.representations.get(f),
                None | Some(crate::PolygonRepresentation::Face)
            ) {
                subdivision += 1;
            }
            kept.push(f);
        }
    }
    for (count, what) in [
        (lines, "polyline(s) not exported"),
        (degenerate, "zero-area polygon(s) not exported"),
        (
            subdivision,
            "subdivision polygon(s) exported as plain polygons",
        ),
    ] {
        if count > 0 {
            notes.push(format!("{name}: {count} {what}"));
        }
    }
    if mesh.attributes.has_typed_data()
        || !mesh.attributes.point_weights.is_empty()
        || !mesh.attributes.point_colors.is_empty()
        || !mesh.attributes.corner_weights.is_empty()
        || !mesh.attributes.corner_colors.is_empty()
        || !mesh.attributes.corner_normals.is_empty()
        || !mesh.attributes.relative_morphs.is_empty()
    {
        notes.push(format!(
            "{name}: weight, color, normal, morph maps and selection sets not exported"
        ));
    }
    // LXE-Q04 leaves unused points unverified: write only points in use.
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    for &f in &kept {
        for &v in &mesh.faces[f].vertices {
            remap[v as usize] = 0;
        }
    }
    let mut points = Vec::new();
    for (i, slot) in remap.iter_mut().enumerate() {
        if *slot == 0 {
            *slot = points.len() as u32;
            points.push(mesh.vertices[i]);
        }
    }
    let mut uvs = BTreeMap::new();
    for (map, values) in &mesh.uv_maps {
        uvs.insert(
            text(map),
            kept.iter().map(|&f| values[f].clone()).collect::<Vec<_>>(),
        );
    }
    Ok(Layer {
        name: text(name),
        points,
        polygons: kept
            .iter()
            .map(|&f| {
                mesh.faces[f]
                    .vertices
                    .iter()
                    .map(|&v| remap[v as usize])
                    .collect()
            })
            .collect(),
        labels: kept
            .iter()
            .map(|&f| {
                let face = &mesh.faces[f];
                let material = face
                    .material_label
                    .clone()
                    .or_else(|| (!face.material.is_empty()).then(|| face.material.clone()));
                (material.map(|m| text(&m)), face.part.as_deref().map(text))
            })
            .collect(),
        uvs,
    })
}

/// ZXY Euler angles (radians) for an item's rotation. Items already in ZXY
/// keep their values; XYZ items are converted to the same rotation.
fn zxy_angles(t: &crate::Transform) -> glam::DVec3 {
    if t.rotation_order() == crate::scene::RotationOrder::Zxy {
        return t.rotation;
    }
    let q: DQuat = t.rotation_quat();
    // Transform::matrix composes ZXY as Ry * Rx * Rz, which is glam's YXZ.
    let (y, x, z) = q.to_euler(EulerRot::YXZ);
    glam::DVec3::new(x, y, z)
}

/// Encode the scene as an LXO file for Modo 17.1v1.
pub fn write(scene: &Scene, notes: &mut Vec<String>) -> Result<Vec<u8>> {
    scene.validate()?;
    // A child hidden by its parent's eye (GV-01) is written hidden, the
    // same as an item hidden on its own.
    let resolved = scene.resolved_eyes();
    let scene = &resolved;
    // Empty meshes are LXE-Q04: unverified, so they are left out. One that
    // parents other items can't be left out without moving its children.
    let has_children = |id| scene.items.iter().any(|i| i.parent == Some(id));
    let mut items = Vec::new();
    let mut layers = Vec::new();
    for item in &scene.items {
        let l = layer(&item.name, &item.mesh, notes)?;
        if l.polygons.is_empty() {
            if has_children(item.id) {
                return Err(format!(
                    "'{}' has no polygons but parents other items; LXO export can't write empty meshes yet",
                    item.name
                ));
            }
            notes.push(format!(
                "'{}' has no polygons and was not exported",
                item.name
            ));
            continue;
        }
        items.push(item);
        layers.push(l);
    }
    if items.is_empty() {
        return Err("Nothing to export: no item has polygons".into());
    }
    if items.len() > MAX_ITEMS
        || layers.iter().map(|l| l.points.len()).sum::<usize>() > MAX_POINTS
        || layers.iter().map(|l| l.polygons.len()).sum::<usize>() > MAX_POLYGONS
    {
        return Err("Scene exceeds the LXO export budget".into());
    }
    // File-local item ids: meshes 1..=n, then three transform items each.
    let mesh_id: BTreeMap<u64, u32> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.id, i as u32 + 1))
        .collect();
    let n = items.len() as u32;

    // Channel names, in first-use order (LXE-12).
    let hidden = items.iter().any(|i| !i.visible);
    let mut channels: Vec<&str> = vec![
        "type", "order", "pos.X", "pos.Y", "pos.Z", "rot.X", "rot.Y", "rot.Z", "scl.X", "scl.Y",
        "scl.Z",
    ];
    if hidden {
        channels.push("visible");
    }
    let channel = |name: &str| channels.iter().position(|c| *c == name).unwrap() as u32;

    // Labels, shared by every mesh (LXE-08).
    let mut tags: Vec<String> = Vec::new();
    let mut tag_index: BTreeMap<String, u16> = BTreeMap::new();
    for l in &layers {
        for (m, p) in &l.labels {
            for label in [m, p].into_iter().flatten() {
                if !tag_index.contains_key(label) {
                    let i = u16::try_from(tags.len()).map_err(|_| "Too many distinct labels")?;
                    tag_index.insert(label.clone(), i);
                    tags.push(label.clone());
                }
            }
        }
    }

    let mut body = Bytes::default();
    // LXE-02: VRSN, APPV, ENCO, in that order.
    let mut p = Bytes::default();
    p.u32(16);
    p.u32(0);
    p.s0(PRODUCER);
    body.chunk(b"VRSN", &p)?;
    let mut p = Bytes::default();
    for word in [17, 4001, 1, 0] {
        p.u32(word);
    }
    p.s0(PRODUCER);
    body.chunk(b"APPV", &p)?;
    let mut p = Bytes::default();
    p.u32(2);
    body.chunk(b"ENCO", &p)?;
    // LXE-03: CHNM, TAGS, geometry groups, items, setup action.
    // CHNM is a U32 count then the names (import spec format/03).
    let mut p = Bytes::default();
    p.u32(channels.len() as u32);
    for c in &channels {
        p.s0(c);
    }
    body.chunk(b"CHNM", &p)?;
    if !tags.is_empty() {
        let mut p = Bytes::default();
        for t in &tags {
            p.s0(t);
        }
        body.chunk(b"TAGS", &p)?;
    }
    for (ordinal, (item, l)) in items.iter().zip(&layers).enumerate() {
        write_layer(
            &mut body,
            ordinal as u16,
            mesh_id[&item.id],
            l,
            &tag_index,
            notes,
        )?;
    }
    for (i, item) in items.iter().enumerate() {
        let mut p = Bytes::default();
        p.s0("mesh");
        p.s0(&text(&item.name));
        p.u32(mesh_id[&item.id]);
        let mut s = Bytes::default();
        s.u32(i as u32);
        s.u32(0);
        s.0.extend_from_slice(&[0xc8, 0xc8, 0xc8, 0xff]);
        p.sub(b"LAYR", &s)?;
        if let Some(parent) = item.parent {
            let parent = mesh_id
                .get(&parent)
                .ok_or("An item's parent was not exported")?;
            let mut s = Bytes::default();
            s.s0("parent");
            s.i32(*parent as i32);
            s.i32(0);
            p.sub(b"LINK", &s)?;
        }
        // Editor Color (IC-Q1): the COLR item tag, only when set.
        if let Some(color) = item.common.editor_color {
            let mut s = Bytes::default();
            s.id(b"COLR");
            s.s0(color.key());
            p.sub(b"ITAG", &s)?;
        }
        body.chunk(b"ITEM", &p)?;
    }
    // LXE-11: scale, rotation, translation items for every mesh. The spec
    // allows omitting them for identity transforms; writing them always is
    // the verified form and what Operandi's importer expects.
    let transform_id = |i: u32, ordinal: u32| n + 1 + i * 3 + ordinal;
    for i in 0..n {
        for (ordinal, kind) in ["scale", "rotation", "translation"].into_iter().enumerate() {
            let mut p = Bytes::default();
            p.s0(kind);
            p.s0("");
            p.u32(transform_id(i, ordinal as u32));
            let mut s = Bytes::default();
            s.s0("xfrmCore");
            s.i32((i + 1) as i32);
            s.i32(ordinal as i32);
            p.sub(b"LINK", &s)?;
            body.chunk(b"ITEM", &p)?;
        }
    }
    // LXE-16 (v3): one bare render item; Modo fills in its settings.
    let mut p = Bytes::default();
    p.s0("polyRender");
    p.s0("");
    p.u32(transform_id(n, 0));
    body.chunk(b"ITEM", &p)?;
    // LXE-12: one setup action holding every channel value.
    let mut p = Bytes::default();
    p.s0("setup");
    p.s0("setup");
    p.u32(1);
    let float = |p: &mut Bytes, name: &str, v: f64| -> Result<()> {
        let mut s = Bytes::default();
        s.vx(channel(name));
        s.u16(2);
        s.vx(0);
        s.f32(v);
        p.sub(b"CHAN", &s)
    };
    let string = |p: &mut Bytes, name: &str, v: &str| -> Result<()> {
        let mut s = Bytes::default();
        s.vx(channel(name));
        s.u16(3);
        s.vx(0);
        s.s0(v);
        p.sub(b"CHAN", &s)
    };
    let owner = |p: &mut Bytes, id: u32| -> Result<()> {
        let mut s = Bytes::default();
        s.u32(id);
        p.sub(b"ITEM", &s)
    };
    for (i, item) in items.iter().enumerate() {
        let i = i as u32;
        let t = &item.transform;
        let rot = zxy_angles(t);
        owner(&mut p, transform_id(i, 0))?;
        string(&mut p, "type", "core")?;
        float(&mut p, "scl.X", t.scale.x)?;
        float(&mut p, "scl.Y", t.scale.y)?;
        float(&mut p, "scl.Z", t.scale.z)?;
        owner(&mut p, transform_id(i, 1))?;
        string(&mut p, "type", "core")?;
        string(&mut p, "order", "zxy")?;
        float(&mut p, "rot.X", rot.x)?;
        float(&mut p, "rot.Y", rot.y)?;
        float(&mut p, "rot.Z", rot.z)?;
        owner(&mut p, transform_id(i, 2))?;
        string(&mut p, "type", "core")?;
        float(&mut p, "pos.X", t.translation.x)?;
        float(&mut p, "pos.Y", t.translation.y)?;
        float(&mut p, "pos.Z", t.translation.z)?;
        if !item.visible {
            owner(&mut p, i + 1)?;
            string(&mut p, "visible", "off")?;
        }
    }
    body.chunk(b"ACTN", &p)?;

    // LXE-01: one FORM of type LXOB, length = file size - 8.
    let len = u32::try_from(body.0.len() + 4).map_err(|_| "LXO file exceeds 4 GiB")?;
    let mut out = Vec::with_capacity(body.0.len() + 12);
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(b"LXOB");
    out.extend_from_slice(&body.0);
    Ok(out)
}

/// LXE-04 to LXE-08 for one mesh: LAYR, PNTS, POLS, then UV maps (VMAD only
/// after POLS) and polygon tags.
fn write_layer(
    body: &mut Bytes,
    ordinal: u16,
    item_id: u32,
    l: &Layer,
    tags: &BTreeMap<String, u16>,
    notes: &mut Vec<String>,
) -> Result<()> {
    let mut p = Bytes::default();
    p.u16(ordinal);
    p.u16(5);
    for _ in 0..3 {
        p.f32(0.0);
    }
    p.s0(&l.name);
    p.i16(-1);
    p.0.extend_from_slice(&2.0f32.to_be_bytes());
    p.0.extend_from_slice(&[0x3d, 0xb2, 0xb8, 0xc2]);
    for v in [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        p.f32(v);
    }
    p.u32(item_id);
    p.0.extend_from_slice(&LAYR_TAIL);
    body.chunk(b"LAYR", &p)?;
    // LXE-05: file space negates Z.
    let mut p = Bytes::default();
    for v in &l.points {
        p.f32(v.x);
        p.f32(v.y);
        p.f32(-v.z);
    }
    body.chunk(b"PNTS", &p)?;
    // LXE-06: corners reversed, keeping the first, for the mirrored space.
    let mut p = Bytes::default();
    p.id(b"FACE");
    for poly in &l.polygons {
        p.u16(poly.len() as u16);
        p.vx(poly[0]);
        for &v in poly[1..].iter().rev() {
            p.vx(v);
        }
    }
    body.chunk(b"POLS", &p)?;
    // LXE-07: each point's continuous value is its first mapped corner's;
    // corners that differ are seams written as VMAD after POLS.
    let f32_pair = |uv: DVec2| [uv.x as f32, uv.y as f32];
    let mut unmapped = 0;
    for (name, values) in &l.uvs {
        let mut point_uv: BTreeMap<u32, DVec2> = BTreeMap::new();
        for (poly, corners) in l.polygons.iter().zip(values) {
            for (&v, uv) in poly.iter().zip(corners) {
                if let Some(uv) = uv {
                    point_uv.entry(v).or_insert(*uv);
                }
            }
        }
        let mut seams = Vec::new();
        for (f, (poly, corners)) in l.polygons.iter().zip(values).enumerate() {
            for (&v, uv) in poly.iter().zip(corners) {
                match uv {
                    Some(uv) if f32_pair(*uv) != f32_pair(point_uv[&v]) => {
                        seams.push((v, f as u32, *uv))
                    }
                    None if point_uv.contains_key(&v) => unmapped += 1,
                    _ => {}
                }
            }
        }
        if point_uv.is_empty() {
            continue;
        }
        let mut p = Bytes::default();
        p.id(b"TXUV");
        p.u16(2);
        p.s0(name);
        for (&v, uv) in &point_uv {
            p.vx(v);
            p.f32(uv.x);
            p.f32(uv.y);
        }
        body.chunk(b"VMAP", &p)?;
        if !seams.is_empty() {
            let mut p = Bytes::default();
            p.id(b"TXUV");
            p.u16(2);
            p.s0(name);
            for (v, f, uv) in seams {
                p.vx(v);
                p.vx(f);
                p.f32(uv.x);
                p.f32(uv.y);
            }
            body.chunk(b"VMAD", &p)?;
        }
    }
    if unmapped > 0 {
        notes.push(format!(
            "{}: {unmapped} UV-less corner(s) take their point's UV in LXO",
            l.name
        ));
    }
    // LXE-08: material then part tags, per polygon.
    for (kind, pick) in [(b"MATR", 0), (b"PART", 1)] {
        let entries: Vec<(u32, u16)> = l
            .labels
            .iter()
            .enumerate()
            .filter_map(|(f, labels)| {
                let label = if pick == 0 { &labels.0 } else { &labels.1 };
                label.as_ref().map(|t| (f as u32, tags[t]))
            })
            .collect();
        if entries.is_empty() {
            continue;
        }
        let mut p = Bytes::default();
        p.id(kind);
        for (f, t) in entries {
            p.vx(f);
            p.u16(t);
        }
        body.chunk(b"PTAG", &p)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
