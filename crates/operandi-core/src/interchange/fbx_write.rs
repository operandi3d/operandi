//! FBX export: binary FBX 7.4 (the version most apps read), written in
//! Rust from the public description of the format: a header, a tree of
//! node records whose properties are typed values and arrays, and a
//! footer. The constants in the header and footer (the file id, creation
//! time and closing magic that Autodesk's readers check) are the ones
//! Blender's FBX exporter writes (GPL-2.0-or-later, `encode_bin.py`;
//! numbers only, no code taken).
//!
//! What's written: one `Model` + `Geometry` per visible mesh item, with
//! the world transform baked into the points like the other flat writers,
//! polygons kept as n-gons, per-corner normals (the viewport's smoothing),
//! the first UV map as `Texture`, per-polygon smoothing flags and one
//! `Material` (Phong, diffuse colour) per polygon tag. Coordinates are
//! metres with `UnitScaleFactor` 100 (FBX counts in centimetres), Y up.
//! Polylines, cameras, lights, hierarchy and animation are not written.
use super::{WorldMesh, corner_normals};
use crate::Result;
use std::collections::BTreeMap;

const VERSION: u32 = 7400;
const HEAD_MAGIC: &[u8] = b"Kaydara FBX Binary  \x00\x1a\x00";
const TIME_ID: &str = "1970-01-01 10:00:00:000";
const FILE_ID: [u8; 16] = [
    0x28, 0xb3, 0x2a, 0xeb, 0xb6, 0x24, 0xcc, 0xc2, 0xbf, 0xc8, 0xb0, 0x2a, 0xa9, 0x2b, 0xfc, 0xf1,
];
const FOOT_ID: [u8; 16] = [
    0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26, 0x7e,
];
const FOOT_MAGIC: [u8; 16] = [
    0xf8, 0x5a, 0x8c, 0x6a, 0xde, 0xf5, 0xd9, 0x7e, 0xec, 0xe9, 0x0c, 0xe3, 0x75, 0x8f, 0x29, 0x0b,
];
/// End of a nested list for versions before 7500: a record of zeros.
const SENTINEL: [u8; 13] = [0; 13];

/// One property of a node record.
#[derive(Clone, Debug)]
enum Prop {
    Bool(bool),
    I32(i32),
    I64(i64),
    F64(f64),
    Str(Vec<u8>),
    Raw(Vec<u8>),
    I32s(Vec<i32>),
    F64s(Vec<f64>),
}

impl Prop {
    fn str(s: &str) -> Prop {
        Prop::Str(s.as_bytes().to_vec())
    }
    /// `Name\0\x01Class`, the binary form of ASCII's `Class::Name`.
    fn object_name(name: &str, class: &str) -> Prop {
        let mut b = name.replace('\0', "").into_bytes();
        b.extend_from_slice(b"\x00\x01");
        b.extend_from_slice(class.as_bytes());
        Prop::Str(b)
    }
    fn write(&self, out: &mut Vec<u8>) {
        fn array<T>(out: &mut Vec<u8>, code: u8, items: &[T], each: impl Fn(&T, &mut Vec<u8>)) {
            out.push(code);
            out.extend((items.len() as u32).to_le_bytes());
            out.extend(0u32.to_le_bytes()); // not compressed
            let size = std::mem::size_of_val(items);
            out.extend((size as u32).to_le_bytes());
            for item in items {
                each(item, out);
            }
        }
        match self {
            Prop::Bool(v) => {
                out.push(b'C');
                out.push(u8::from(*v));
            }
            Prop::I32(v) => {
                out.push(b'I');
                out.extend(v.to_le_bytes());
            }
            Prop::I64(v) => {
                out.push(b'L');
                out.extend(v.to_le_bytes());
            }
            Prop::F64(v) => {
                out.push(b'D');
                out.extend(v.to_le_bytes());
            }
            Prop::Str(b) | Prop::Raw(b) => {
                out.push(if matches!(self, Prop::Str(_)) {
                    b'S'
                } else {
                    b'R'
                });
                out.extend((b.len() as u32).to_le_bytes());
                out.extend(b);
            }
            Prop::I32s(v) => array(out, b'i', v, |x, o| o.extend(x.to_le_bytes())),
            Prop::F64s(v) => array(out, b'd', v, |x, o| o.extend(x.to_le_bytes())),
        }
    }
}

/// A node record: name, properties, children.
#[derive(Clone, Debug, Default)]
struct Node {
    name: &'static str,
    props: Vec<Prop>,
    children: Vec<Node>,
}

impl Node {
    fn new(name: &'static str) -> Node {
        Node {
            name,
            ..Node::default()
        }
    }
    fn with(name: &'static str, props: Vec<Prop>) -> Node {
        Node {
            name,
            props,
            children: Vec::new(),
        }
    }
    fn child(mut self, node: Node) -> Node {
        self.children.push(node);
        self
    }
    fn add(&mut self, node: Node) -> &mut Node {
        self.children.push(node);
        self.children.last_mut().expect("just pushed")
    }
    /// Writes this record at `out.len()`, which is its absolute offset in
    /// the file (the caller writes from the start of the file).
    fn write(&self, out: &mut Vec<u8>, is_last: bool) -> Result<()> {
        let start = out.len();
        out.extend([0u8; 12]); // end offset, property count, property bytes
        out.push(self.name.len() as u8);
        out.extend(self.name.as_bytes());
        let props_start = out.len();
        for p in &self.props {
            p.write(out);
        }
        let props_len = out.len() - props_start;
        if !self.children.is_empty() {
            write_list(&self.children, out)?;
            out.extend(SENTINEL);
        } else if self.props.is_empty() && !is_last {
            out.extend(SENTINEL);
        }
        let end = u32::try_from(out.len())
            .map_err(|_| "FBX export is over 4 GB; FBX 7.4 can't hold it")?;
        out[start..start + 4].copy_from_slice(&end.to_le_bytes());
        out[start + 4..start + 8].copy_from_slice(&(self.props.len() as u32).to_le_bytes());
        out[start + 8..start + 12].copy_from_slice(&(props_len as u32).to_le_bytes());
        Ok(())
    }
}

fn write_list(nodes: &[Node], out: &mut Vec<u8>) -> Result<()> {
    for (i, n) in nodes.iter().enumerate() {
        n.write(out, i + 1 == nodes.len())?;
    }
    Ok(())
}

/// `P: name, type, label, flags, values...` inside `Properties70`.
fn p(name: &str, kind: &str, label: &str, flags: &str, values: Vec<Prop>) -> Node {
    let mut props = vec![
        Prop::str(name),
        Prop::str(kind),
        Prop::str(label),
        Prop::str(flags),
    ];
    props.extend(values);
    Node::with("P", props)
}
fn p_int(name: &str, v: i32) -> Node {
    p(name, "int", "Integer", "", vec![Prop::I32(v)])
}
fn p_double(name: &str, v: f64) -> Node {
    p(name, "double", "Number", "", vec![Prop::F64(v)])
}
fn p_color(name: &str, c: [f64; 3]) -> Node {
    p(
        name,
        "Color",
        "",
        "A",
        c.into_iter().map(Prop::F64).collect(),
    )
}
fn single(name: &'static str, v: Prop) -> Node {
    Node::with(name, vec![v])
}

/// The whole file for `meshes`.
pub(super) fn write(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Vec<u8>> {
    let mut next_id: i64 = 1_000_000;
    let mut id = || {
        next_id += 1;
        next_id
    };
    let mut objects = Node::new("Objects");
    let mut connections = Node::new("Connections");
    let connect = |c: &mut Node, child: i64, parent: i64| {
        c.add(Node::with(
            "C",
            vec![Prop::str("OO"), Prop::I64(child), Prop::I64(parent)],
        ));
    };
    // Materials are shared by name across meshes, written once.
    let mut material_ids: BTreeMap<String, i64> = BTreeMap::new();
    let (mut models, mut geometries) = (0, 0);
    let mut lines = 0;
    for m in meshes {
        lines += m.polyline.iter().filter(|&&p| p).count();
        let faces: Vec<usize> = m.surface_faces().collect();
        if faces.is_empty() {
            continue;
        }
        let normals = corner_normals(m);
        let (geometry_id, model_id) = (id(), id());
        let mut vertices = Vec::with_capacity(m.mesh.vertices.len() * 3);
        for v in &m.mesh.vertices {
            vertices.extend([v.x, v.y, v.z]);
        }
        let mut index = Vec::new();
        let mut corner_normal = Vec::new();
        let mut uv_values = Vec::new();
        let mut uv_index = Vec::new();
        let mut smoothing = Vec::new();
        let mut slots: Vec<String> = Vec::new();
        let mut face_slot = Vec::new();
        for &f in &faces {
            let face = &m.mesh.faces[f];
            let n = face.vertices.len();
            for (c, &v) in face.vertices.iter().enumerate() {
                let v = i32::try_from(v).map_err(|_| "Too many points for FBX")?;
                index.push(if c + 1 == n { -v - 1 } else { v });
                let nn = normals
                    .get(f)
                    .and_then(|c2| c2.get(c))
                    .copied()
                    .unwrap_or_default();
                corner_normal.extend([nn.x, nn.y, nn.z]);
                if let Some(uvs) = &m.uvs {
                    let uv = uvs
                        .get(f)
                        .and_then(|u| u.get(c).copied().flatten())
                        .unwrap_or_default();
                    uv_index.push((uv_values.len() / 2) as i32);
                    uv_values.extend([uv.x, uv.y]);
                }
            }
            smoothing.push(i32::from(face.smooth));
            let slot = match slots.iter().position(|s| *s == face.material) {
                Some(i) => i,
                None => {
                    slots.push(face.material.clone());
                    slots.len() - 1
                }
            };
            face_slot.push(slot as i32);
        }
        let mut geometry = Node::with(
            "Geometry",
            vec![
                Prop::I64(geometry_id),
                Prop::object_name(&m.name, "Geometry"),
                Prop::str("Mesh"),
            ],
        );
        geometry.add(Node::new("Properties70"));
        geometry.add(single("GeometryVersion", Prop::I32(124)));
        geometry.add(single("Vertices", Prop::F64s(vertices)));
        geometry.add(single("PolygonVertexIndex", Prop::I32s(index)));
        geometry.add(
            Node::with("LayerElementNormal", vec![Prop::I32(0)])
                .child(single("Version", Prop::I32(102)))
                .child(single("Name", Prop::str("")))
                .child(single(
                    "MappingInformationType",
                    Prop::str("ByPolygonVertex"),
                ))
                .child(single("ReferenceInformationType", Prop::str("Direct")))
                .child(single("Normals", Prop::F64s(corner_normal))),
        );
        geometry.add(
            Node::with("LayerElementSmoothing", vec![Prop::I32(0)])
                .child(single("Version", Prop::I32(102)))
                .child(single("Name", Prop::str("")))
                .child(single("MappingInformationType", Prop::str("ByPolygon")))
                .child(single("ReferenceInformationType", Prop::str("Direct")))
                .child(single("Smoothing", Prop::I32s(smoothing))),
        );
        let has_uv = m.uvs.is_some();
        if has_uv {
            geometry.add(
                Node::with("LayerElementUV", vec![Prop::I32(0)])
                    .child(single("Version", Prop::I32(101)))
                    .child(single("Name", Prop::str(super::UV_MAP)))
                    .child(single(
                        "MappingInformationType",
                        Prop::str("ByPolygonVertex"),
                    ))
                    .child(single(
                        "ReferenceInformationType",
                        Prop::str("IndexToDirect"),
                    ))
                    .child(single("UV", Prop::F64s(uv_values)))
                    .child(single("UVIndex", Prop::I32s(uv_index))),
            );
        }
        geometry.add(
            Node::with("LayerElementMaterial", vec![Prop::I32(0)])
                .child(single("Version", Prop::I32(101)))
                .child(single("Name", Prop::str("")))
                .child(single("MappingInformationType", Prop::str("ByPolygon")))
                .child(single(
                    "ReferenceInformationType",
                    Prop::str("IndexToDirect"),
                ))
                .child(single("Materials", Prop::I32s(face_slot))),
        );
        let element = |kind: &str| {
            Node::new("LayerElement")
                .child(single("Type", Prop::str(kind)))
                .child(single("TypedIndex", Prop::I32(0)))
        };
        let mut layer = Node::with("Layer", vec![Prop::I32(0)])
            .child(single("Version", Prop::I32(100)))
            .child(element("LayerElementNormal"))
            .child(element("LayerElementSmoothing"));
        if has_uv {
            layer = layer.child(element("LayerElementUV"));
        }
        layer = layer.child(element("LayerElementMaterial"));
        geometry.add(layer);
        objects.add(geometry);

        let model = Node::with(
            "Model",
            vec![
                Prop::I64(model_id),
                Prop::object_name(&m.name, "Model"),
                Prop::str("Mesh"),
            ],
        )
        .child(single("Version", Prop::I32(232)))
        .child(Node::new("Properties70").child(p_int("DefaultAttributeIndex", 0)))
        .child(single("Shading", Prop::Bool(true)))
        .child(single("Culling", Prop::str("CullingOff")));
        objects.add(model);
        connect(&mut connections, model_id, 0);
        connect(&mut connections, geometry_id, model_id);
        // A model's materials connect in slot order: slot i is the i-th.
        for name in &slots {
            let material_id = match material_ids.get(name) {
                Some(&mid) => mid,
                None => {
                    let mid = id();
                    material_ids.insert(name.clone(), mid);
                    let settings = m.materials.get(name).cloned().unwrap_or_default();
                    let label = if name.is_empty() {
                        "Default"
                    } else {
                        name.as_str()
                    };
                    objects.add(
                        Node::with(
                            "Material",
                            vec![
                                Prop::I64(mid),
                                Prop::object_name(label, "Material"),
                                Prop::str(""),
                            ],
                        )
                        .child(single("Version", Prop::I32(102)))
                        .child(single("ShadingModel", Prop::str("phong")))
                        .child(single("MultiLayer", Prop::I32(0)))
                        .child(
                            Node::new("Properties70")
                                .child(p_color("DiffuseColor", settings.color))
                                .child(p_double("DiffuseFactor", settings.diffuse))
                                .child(p_color("SpecularColor", [1.0; 3]))
                                .child(p_double("SpecularFactor", settings.specular)),
                        ),
                    );
                    mid
                }
            };
            connect(&mut connections, material_id, model_id);
        }
        models += 1;
        geometries += 1;
    }
    if models == 0 {
        return Err("Nothing to export: no visible item has polygons".into());
    }
    if lines > 0 {
        notes.push(format!(
            "{lines} polyline(s) skipped: FBX export writes polygons only"
        ));
    }

    let count = |kind: &str, n: usize| {
        Node::with("ObjectType", vec![Prop::str(kind)]).child(single("Count", Prop::I32(n as i32)))
    };
    let total = 1 + models + geometries + material_ids.len();
    let definitions = Node::new("Definitions")
        .child(single("Version", Prop::I32(100)))
        .child(single("Count", Prop::I32(total as i32)))
        .child(count("GlobalSettings", 1))
        .child(count("Model", models))
        .child(count("Geometry", geometries))
        .child(count("Material", material_ids.len()));

    let creator = format!("Operandi {}", env!("CARGO_PKG_VERSION"));
    let header = Node::new("FBXHeaderExtension")
        .child(single("FBXHeaderVersion", Prop::I32(1003)))
        .child(single("FBXVersion", Prop::I32(VERSION as i32)))
        .child(single("EncryptionType", Prop::I32(0)))
        .child(single("Creator", Prop::str(&creator)));
    // Y up, Z front, X right (right-handed), metres as 100 cm.
    let settings = Node::new("GlobalSettings")
        .child(single("Version", Prop::I32(1000)))
        .child(
            Node::new("Properties70")
                .child(p_int("UpAxis", 1))
                .child(p_int("UpAxisSign", 1))
                .child(p_int("FrontAxis", 2))
                .child(p_int("FrontAxisSign", 1))
                .child(p_int("CoordAxis", 0))
                .child(p_int("CoordAxisSign", 1))
                .child(p_int("OriginalUpAxis", 1))
                .child(p_int("OriginalUpAxisSign", 1))
                .child(p_double("UnitScaleFactor", 100.0))
                .child(p_double("OriginalUnitScaleFactor", 100.0)),
        );
    let document_id = id();
    let documents = Node::new("Documents")
        .child(single("Count", Prop::I32(1)))
        .child(
            Node::with(
                "Document",
                vec![
                    Prop::I64(document_id),
                    Prop::str("Scene"),
                    Prop::str("Scene"),
                ],
            )
            .child(Node::new("Properties70"))
            .child(single("RootNode", Prop::I64(0))),
        );
    let root = [
        header,
        single("FileId", Prop::Raw(FILE_ID.to_vec())),
        single("CreationTime", Prop::str(TIME_ID)),
        single("Creator", Prop::str(&creator)),
        settings,
        documents,
        Node::new("References"),
        definitions,
        objects,
        connections,
    ];

    let mut out = HEAD_MAGIC.to_vec();
    out.extend(VERSION.to_le_bytes());
    write_list(&root, &mut out)?;
    out.extend(SENTINEL);
    out.extend(FOOT_ID);
    out.extend([0u8; 4]);
    let pad = match (16 - out.len() % 16) % 16 {
        0 => 16,
        n => n,
    };
    out.extend(std::iter::repeat_n(0u8, pad));
    out.extend(VERSION.to_le_bytes());
    out.extend([0u8; 120]);
    out.extend(FOOT_MAGIC);
    Ok(out)
}
