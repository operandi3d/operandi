//! glTF 2.0, as `.gltf` (JSON with the binary data embedded as a base64 data
//! URI, so the export is one file) and `.glb` (binary container).
//!
//! Import walks the default scene's node tree, bakes each node's world
//! transform into its points and makes one item per mesh node. glTF splits
//! points wherever UVs or normals differ; import welds points with identical
//! positions so the result is connected, editable topology, and keeps the UVs
//! per polygon corner. A triangle is smooth when its stored corner normals
//! differ (a flat triangle stores one normal three times). Points, lines,
//! textures, skins, morph targets and animation are not imported.
//!
//! Export writes one node and mesh per item, one primitive per material, and
//! triangulated polygons with normals and the Texture UVs (V flipped to glTF's
//! top-left origin).
//!
//! Materials: a stored material exports its Color times Diffuse as the base
//! color and its Specular as the IOR (KHR_materials_ior), plus Operandi's own
//! settings in the material's `extras`, so a round trip restores them
//! exactly. Importing a material without those extras makes a material with
//! the file's base color at 100% Diffuse. Polygons whose tag has no stored
//! material export with a neutral placeholder that imports as no material.
use super::{RawFace, RawMesh, WorldMesh, corner_normals};
use crate::Result;
use base64::Engine;
use glam::{DMat4, DVec2, DVec3};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub(super) fn read(
    bytes: &[u8],
    folder: Option<&Path>,
    stem: &str,
    notes: &mut Vec<String>,
) -> Result<Vec<RawMesh>> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| format!("Invalid glTF: {e}"))?;
    let mut buffers = Vec::new();
    for buffer in gltf.document.buffers() {
        let data = match buffer.source() {
            gltf::buffer::Source::Bin => {
                gltf.blob.clone().ok_or("GLB is missing its binary chunk")?
            }
            gltf::buffer::Source::Uri(uri) => load_uri(uri, folder)?,
        };
        if data.len() < buffer.length() {
            return Err(format!(
                "glTF buffer {} is shorter than declared",
                buffer.index()
            ));
        }
        buffers.push(data);
    }
    let mut out = Vec::new();
    let mut skipped = 0;
    let mut visit = vec![];
    match gltf
        .document
        .default_scene()
        .or_else(|| gltf.document.scenes().next())
    {
        Some(scene) => visit.extend(scene.nodes().map(|n| (n, DMat4::IDENTITY))),
        // No scene: every mesh at the origin.
        None => {
            for mesh in gltf.document.meshes() {
                out.push(read_mesh(
                    &mesh,
                    None,
                    DMat4::IDENTITY,
                    &buffers,
                    stem,
                    &mut skipped,
                )?);
            }
        }
    }
    let mut seen = 0usize;
    while let Some((node, parent)) = visit.pop() {
        seen += 1;
        if seen > 100_000 {
            return Err("glTF node tree is too large or cyclic".into());
        }
        let m = node.transform().matrix();
        let local = DMat4::from_cols_array_2d(&m.map(|c| c.map(f64::from)));
        let world = parent * local;
        if let Some(mesh) = node.mesh() {
            out.push(read_mesh(
                &mesh,
                node.name(),
                world,
                &buffers,
                stem,
                &mut skipped,
            )?);
        }
        visit.extend(node.children().map(|c| (c, world)));
    }
    if skipped > 0 {
        notes.push(format!("Skipped {skipped} glTF point/line primitive(s)"));
    }
    Ok(out)
}

fn read_mesh(
    mesh: &gltf::Mesh,
    node_name: Option<&str>,
    world: DMat4,
    buffers: &[Vec<u8>],
    stem: &str,
    skipped: &mut usize,
) -> Result<RawMesh> {
    let mirrored = world.determinant() < 0.0;
    let mut raw = RawMesh {
        name: node_name
            .or(mesh.name())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{stem} {}", mesh.index() + 1)),
        ..Default::default()
    };
    let mut weld: HashMap<[u64; 3], u32> = HashMap::new();
    for primitive in mesh.primitives() {
        use gltf::mesh::Mode;
        if !matches!(
            primitive.mode(),
            Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan
        ) {
            *skipped += 1;
            continue;
        }
        let reader = primitive.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
        let Some(positions) = reader.read_positions() else {
            continue;
        };
        let points: Vec<u32> = positions
            .map(|p| {
                let w = world.transform_point3(DVec3::from_array(p.map(f64::from)));
                let next = raw.points.len() as u32;
                *weld
                    .entry(w.to_array().map(f64::to_bits))
                    .or_insert_with(|| {
                        raw.points.push(w);
                        next
                    })
            })
            .collect();
        let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(Iterator::collect);
        let uvs: Option<Vec<[f32; 2]>> = reader.read_tex_coords(0).map(|t| t.into_f32().collect());
        let indices: Vec<u32> = match reader.read_indices() {
            Some(i) => i.into_u32().collect(),
            None => (0..points.len() as u32).collect(),
        };
        let tris: Vec<[u32; 3]> = match primitive.mode() {
            Mode::Triangles => indices.as_chunks::<3>().0.to_vec(),
            Mode::TriangleStrip => (0..indices.len().saturating_sub(2))
                .map(|i| {
                    if i % 2 == 0 {
                        [indices[i], indices[i + 1], indices[i + 2]]
                    } else {
                        [indices[i + 1], indices[i], indices[i + 2]]
                    }
                })
                .collect(),
            _ => (1..indices.len().saturating_sub(1))
                .map(|i| [indices[0], indices[i], indices[i + 1]])
                .collect(),
        };
        let material = match (primitive.material().index(), primitive.material().name()) {
            (_, Some(name)) => name.to_owned(),
            (Some(i), None) => format!("Material {}", i + 1),
            (None, None) => String::new(),
        };
        if primitive.material().index().is_some()
            && !raw.materials.contains_key(material_key(&material))
            && let Some(settings) = read_material(&primitive.material())
        {
            raw.materials
                .insert(material_key(&material).to_owned(), settings);
        }
        for mut t in tris {
            if t.iter().any(|&i| i as usize >= points.len()) {
                return Err(format!(
                    "glTF mesh '{}' has an index out of range",
                    raw.name
                ));
            }
            if mirrored {
                t.swap(1, 2);
            }
            let smooth = normals.as_ref().is_some_and(|n| {
                let get = |i: u32| n.get(i as usize).copied().unwrap_or_default();
                get(t[0]) != get(t[1]) || get(t[0]) != get(t[2])
            });
            raw.faces.push(RawFace {
                vertices: t.iter().map(|&i| points[i as usize]).collect(),
                uv: uvs.as_ref().and_then(|uv| {
                    t.iter()
                        .map(|&i| {
                            uv.get(i as usize)
                                .map(|u| DVec2::new(u[0] as f64, 1.0 - u[1] as f64))
                        })
                        .collect()
                }),
                material: material.clone(),
                smooth,
            });
        }
    }
    Ok(raw)
}

/// The material table key for a tag: empty means `Default`.
fn material_key(tag: &str) -> &str {
    if crate::material::is_default_name(tag) {
        crate::material::DEFAULT_MATERIAL
    } else {
        tag
    }
}

/// Settings for an imported glTF material; `None` for Operandi's own
/// placeholder (a tag without stored settings) and anything invalid.
fn read_material(m: &gltf::Material) -> Option<crate::Material> {
    let extras: Option<Value> = m
        .extras()
        .as_ref()
        .and_then(|raw| serde_json::from_str(raw.get()).ok());
    if let Some(extras) = &extras {
        if extras.get("operandiPlaceholder") == Some(&Value::Bool(true)) {
            return None;
        }
        if let Some(own) = extras.get("operandi") {
            // Exports before shader tree Tier 1 stored Specular as the
            // reflectance; they carry no "operandiSpecular" marker.
            let amount = extras.get("operandiSpecular") == Some(&Value::from("amount"));
            return serde_json::from_value::<crate::Material>(own.clone())
                .ok()
                .map(|mut m| {
                    if !amount {
                        m.specular = crate::material::specular_amount_for_reflectance(m.specular);
                    }
                    m
                })
                .filter(|m| m.validate().is_ok());
        }
    }
    let [r, g, b, _] = m.pbr_metallic_roughness().base_color_factor();
    let material = crate::Material {
        color: [r, g, b].map(|c| f64::from(c).clamp(0.0, 1.0)),
        diffuse: 1.0,
        ..Default::default()
    };
    material.validate().is_ok().then_some(material)
}

/// Resolve a buffer URI: an embedded base64 data URI, or a file beside the
/// glTF. Remote URLs and paths outside the glTF's folder are refused.
fn load_uri(uri: &str, folder: Option<&Path>) -> Result<Vec<u8>> {
    if let Some(data) = uri.strip_prefix("data:") {
        let (_, payload) = data
            .split_once(";base64,")
            .ok_or("glTF data URI is not base64")?;
        return base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| format!("glTF data URI: {e}"));
    }
    let relative = percent_decode(uri);
    let path = Path::new(&relative);
    if uri.contains("://")
        || path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!("glTF buffer '{uri}' is outside the file's folder"));
    }
    let full = folder.unwrap_or(Path::new(".")).join(path);
    operandi_files::read_whole(&full, super::MAX_IMPORT_BYTES)
        .map_err(|e| format!("glTF buffer '{uri}': {e}"))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = bytes.get(i + 1..i + 3)
            && let Ok(hex) = std::str::from_utf8(hex)
            && let Ok(v) = u8::from_str_radix(hex, 16)
        {
            out.push(v);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Binary data and the JSON views onto it.
#[derive(Default)]
struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}
impl Builder {
    /// Append 4-byte values as one buffer view and accessor; returns its index.
    fn accessor(
        &mut self,
        data: &[u8],
        count: usize,
        kind: &str,
        component: u32,
        target: u32,
        extra: Value,
    ) -> usize {
        let offset = self.bin.len();
        self.bin.extend_from_slice(data);
        // Keep every view on a 4-byte boundary (2-byte indices may end
        // short of one).
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        self.views.push(json!({
            "buffer": 0, "byteOffset": offset, "byteLength": data.len(), "target": target
        }));
        let mut accessor = json!({
            "bufferView": self.views.len() - 1, "componentType": component,
            "count": count, "type": kind
        });
        if let (Value::Object(a), Value::Object(e)) = (&mut accessor, extra) {
            a.extend(e);
        }
        self.accessors.push(accessor);
        self.accessors.len() - 1
    }
}

/// An output vertex: point, normal bits, UV bits.
type CornerKey = (u32, [u64; 3], Option<[u64; 2]>);

const FLOAT: u32 = 5126;
const UNSIGNED_INT: u32 = 5125;
const UNSIGNED_SHORT: u32 = 5123;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

fn floats(values: impl IntoIterator<Item = f64>) -> Vec<u8> {
    values
        .into_iter()
        .flat_map(|v| (v as f32).to_le_bytes())
        .collect()
}

pub(super) fn write(
    meshes: &[WorldMesh],
    binary: bool,
    notes: &mut Vec<String>,
) -> Result<Vec<u8>> {
    let options = super::io_options();
    let mut b = Builder::default();
    let mut materials: BTreeMap<String, usize> = BTreeMap::new();
    let mut json_meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut lines = 0;
    for m in meshes {
        lines += m.polyline.iter().filter(|&&p| p).count();
        let normals = corner_normals(m);
        // Output vertices are unique (point, normal, uv) combinations.
        let mut key_of: HashMap<CornerKey, u32> = HashMap::new();
        let (mut pos, mut nrm, mut tex) = (
            Vec::<DVec3>::new(),
            Vec::<DVec3>::new(),
            Vec::<DVec2>::new(),
        );
        let mut by_material: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        for f in m.surface_faces() {
            let face = &m.mesh.faces[f];
            let uv = m.uvs.as_ref().and_then(|u| u.get(f));
            let mut corner = |c: usize| {
                let point = face.vertices[c];
                let n = normals[f][c];
                let t = m
                    .uvs
                    .as_ref()
                    .map(|_| uv.and_then(|u| u[c]).unwrap_or_default());
                let key = (
                    point,
                    n.to_array().map(f64::to_bits),
                    t.map(|t| t.to_array().map(f64::to_bits)),
                );
                *key_of.entry(key).or_insert_with(|| {
                    pos.push(m.mesh.vertices[point as usize]);
                    nrm.push(n);
                    if let Some(t) = t {
                        tex.push(DVec2::new(t.x, 1.0 - t.y));
                    }
                    pos.len() as u32 - 1
                })
            };
            let tris = m.triangles(f);
            let indices = by_material.entry(face.material.as_str()).or_default();
            for [x, y, z] in tris {
                indices.extend([corner(x), corner(y), corner(z)]);
            }
        }
        if pos.is_empty() {
            continue;
        }
        // Save Geometry off: the node is kept, without its mesh.
        if !options.gltf_geometry {
            nodes.push(json!({"name": m.name}));
            continue;
        }
        let (lo, hi) = pos.iter().fold(
            (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        );
        let bounds = |v: DVec3| v.to_array().map(|c| c as f32).to_vec();
        let position = b.accessor(
            &floats(pos.iter().flat_map(|p| p.to_array())),
            pos.len(),
            "VEC3",
            FLOAT,
            ARRAY_BUFFER,
            json!({"min": bounds(lo), "max": bounds(hi)}),
        );
        let normal = b.accessor(
            &floats(nrm.iter().flat_map(|n| n.to_array())),
            nrm.len(),
            "VEC3",
            FLOAT,
            ARRAY_BUFFER,
            json!({}),
        );
        let texcoord = (!tex.is_empty()).then(|| {
            b.accessor(
                &floats(tex.iter().flat_map(|t| t.to_array())),
                tex.len(),
                "VEC2",
                FLOAT,
                ARRAY_BUFFER,
                json!({}),
            )
        });
        // Save Tangent Basis: per-vertex tangents from the UVs (an average
        // of the triangles' tangents, made perpendicular to the normal;
        // provisional, not MikkTSpace).
        let tangent = (options.gltf_tangents && !tex.is_empty()).then(|| {
            let tangents = vertex_tangents(&pos, &nrm, &tex, by_material.values().flatten());
            b.accessor(
                &floats(tangents.iter().flat_map(|t| t.to_array())),
                tangents.len(),
                "VEC4",
                FLOAT,
                ARRAY_BUFFER,
                json!({}),
            )
        });
        let short = options.gltf_short_indices && pos.len() < 65_536;
        let mut primitives = Vec::new();
        for (material, indices) in by_material {
            let next = materials.len();
            let material = *materials.entry(material.to_owned()).or_insert(next);
            let data: Vec<u8> = if short {
                indices
                    .iter()
                    .flat_map(|&i| (i as u16).to_le_bytes())
                    .collect()
            } else {
                indices.iter().flat_map(|i| i.to_le_bytes()).collect()
            };
            let accessor = b.accessor(
                &data,
                indices.len(),
                "SCALAR",
                if short { UNSIGNED_SHORT } else { UNSIGNED_INT },
                ELEMENT_ARRAY_BUFFER,
                json!({}),
            );
            let mut attributes = json!({"POSITION": position, "NORMAL": normal});
            if let Some(t) = texcoord {
                attributes["TEXCOORD_0"] = json!(t);
            }
            if let Some(t) = tangent {
                attributes["TANGENT"] = json!(t);
            }
            let mut primitive = json!({"attributes": attributes, "indices": accessor, "mode": 4});
            // Save Materials off: primitives use glTF's default material.
            if options.gltf_materials {
                primitive["material"] = json!(material);
            }
            primitives.push(primitive);
        }
        json_meshes.push(json!({"name": m.name, "primitives": primitives}));
        nodes.push(json!({"name": m.name, "mesh": json_meshes.len() - 1}));
    }
    if lines > 0 {
        notes.push(format!("{lines} polyline(s) skipped in glTF"));
    }
    if nodes.is_empty() {
        return Err("Nothing to export: no visible item has polygons".into());
    }
    let stored = meshes.first().map(|m| &m.materials);
    let mut uses_ior = false;
    if !options.gltf_materials {
        materials.clear();
    }
    let mut material_list = vec![Value::Null; materials.len()];
    for (name, i) in materials {
        let settings = stored.and_then(|s| s.get(material_key(&name)));
        let entry = match settings {
            Some(m) => {
                let [r, g, b] = m.diffuse_color();
                // Specular Amount to the reflectance straight on (F0), then
                // to an index of refraction.
                let f0 = crate::material::specular_reflectance(m.specular);
                let root = f0.clamp(0.0, 0.9).sqrt();
                uses_ior = true;
                json!({
                    "name": if name.is_empty() { "Default".to_owned() } else { name },
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [r, g, b, 1.0],
                        "metallicFactor": 0.0, "roughnessFactor": 0.5
                    },
                    "extensions": {"KHR_materials_ior": {"ior": (1.0 + root) / (1.0 - root)}},
                    "extras": {"operandi": m, "operandiSpecular": "amount"},
                })
            }
            None => json!({
                "name": if name.is_empty() { "Default".to_owned() } else { name },
                "pbrMetallicRoughness": {
                    "baseColorFactor": [0.8, 0.8, 0.8, 1.0],
                    "metallicFactor": 0.0, "roughnessFactor": 0.5
                },
                "extras": {"operandiPlaceholder": true},
            }),
        };
        material_list[i] = entry;
    }
    let mut buffer = json!({"byteLength": b.bin.len()});
    if !binary {
        buffer["uri"] = json!(format!(
            "data:application/octet-stream;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&b.bin)
        ));
    }
    let mut asset = json!({"version": "2.0", "generator": "Operandi"});
    let info = super::file_info();
    if !info.copyright.is_empty() {
        asset["copyright"] = json!(info.copyright);
    }
    if !info.author.is_empty() {
        // glTF has no author field; extras carry it.
        asset["extras"] = json!({"author": info.author});
    }
    let mut root = json!({
        "asset": asset,
        "scene": 0,
        "scenes": [{"nodes": (0..nodes.len()).collect::<Vec<_>>()}],
        "nodes": nodes,
    });
    // glTF forbids empty arrays: leave out what Save Geometry or Save
    // Materials left empty.
    if !json_meshes.is_empty() {
        root["meshes"] = json!(json_meshes);
        root["accessors"] = json!(b.accessors);
        root["bufferViews"] = json!(b.views);
        root["buffers"] = json!([buffer]);
    }
    if !material_list.is_empty() {
        root["materials"] = json!(material_list);
    }
    if uses_ior {
        root["extensionsUsed"] = json!(["KHR_materials_ior"]);
    }
    if !binary {
        return serde_json::to_vec_pretty(&root).map_err(|e| e.to_string());
    }

    Ok(glb(
        serde_json::to_vec(&root).map_err(|e| e.to_string())?,
        b.bin,
    ))
}

/// GLB container: 12-byte header, a JSON chunk padded with spaces and a BIN
/// chunk padded with zeros, each to a 4-byte boundary.
fn glb(mut json: Vec<u8>, mut bin: Vec<u8>) -> Vec<u8> {
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    // No geometry (Save Geometry off): no BIN chunk.
    let bin_chunk = if bin.is_empty() { 0 } else { 8 + bin.len() };
    let total = 12 + 8 + json.len() + bin_chunk;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    if !bin.is_empty() {
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
    }
    out
}

/// Per-vertex tangents (xyz, w = handedness) from triangle UV slopes.
fn vertex_tangents<'a>(
    pos: &[DVec3],
    nrm: &[DVec3],
    tex: &[DVec2],
    indices: impl Iterator<Item = &'a u32>,
) -> Vec<glam::DVec4> {
    let mut tan = vec![DVec3::ZERO; pos.len()];
    let mut bit = vec![DVec3::ZERO; pos.len()];
    let indices: Vec<usize> = indices.map(|&i| i as usize).collect();
    for t in indices.as_chunks::<3>().0 {
        let [a, b, c] = *t;
        let (e1, e2) = (pos[b] - pos[a], pos[c] - pos[a]);
        let (d1, d2) = (tex[b] - tex[a], tex[c] - tex[a]);
        let det = d1.x * d2.y - d2.x * d1.y;
        if det.abs() < 1e-20 {
            continue;
        }
        let r = 1. / det;
        let s = (e1 * d2.y - e2 * d1.y) * r;
        let u = (e2 * d1.x - e1 * d2.x) * r;
        for &i in t {
            tan[i] += s;
            bit[i] += u;
        }
    }
    (0..pos.len())
        .map(|i| {
            let n = nrm[i];
            let mut t = (tan[i] - n * n.dot(tan[i])).normalize_or_zero();
            if t == DVec3::ZERO {
                t = n.any_orthonormal_vector();
            }
            let w = if n.cross(t).dot(bit[i]) < 0. { -1. } else { 1. };
            t.extend(w)
        })
        .collect()
}
