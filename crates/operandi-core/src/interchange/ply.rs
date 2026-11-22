//! PLY (the Stanford polygon file format): a public format with a text
//! header followed by ASCII or binary element data. Read: `vertex` positions,
//! per-vertex texture coordinates (`s`/`t`, `u`/`v` or `texture_u`/
//! `texture_v`) and `face` index lists, with per-corner `texcoord` lists
//! where a file has them; other elements and properties are skipped and
//! counted. Written: one mesh holding every visible item (PLY has no
//! objects), polygons as n-gons, UVs as a per-face `texcoord` list so seams
//! survive. Normals are recomputed rather than stored.
use super::{RawFace, RawMesh, WorldMesh, num};
use crate::Result;
use glam::{DVec2, DVec3};
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    Ascii,
    Little,
    Big,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}
impl Scalar {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return None,
        })
    }
    fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

#[derive(Clone, Debug)]
struct Property {
    name: String,
    /// `Some(count type)` for a list property.
    list: Option<Scalar>,
    value: Scalar,
}

#[derive(Clone, Debug)]
struct Element {
    name: String,
    count: usize,
    properties: Vec<Property>,
}

/// Reads values one at a time from the body, in either encoding.
struct Body<'a> {
    bytes: &'a [u8],
    at: usize,
    encoding: Encoding,
    /// ASCII: the words still to read.
    words: std::str::SplitAsciiWhitespace<'a>,
}
impl Body<'_> {
    fn value(&mut self, kind: Scalar) -> Result<f64> {
        if self.encoding == Encoding::Ascii {
            let word = self.words.next().ok_or("PLY data ends early")?;
            return word
                .parse::<f64>()
                .map_err(|_| format!("PLY: invalid number {word:?}"));
        }
        let n = kind.size();
        let raw = self
            .bytes
            .get(self.at..self.at + n)
            .ok_or("PLY data ends early")?;
        self.at += n;
        let mut b = [0u8; 8];
        b[..n].copy_from_slice(raw);
        if self.encoding == Encoding::Big {
            b[..n].reverse();
        }
        Ok(match kind {
            Scalar::I8 => b[0] as i8 as f64,
            Scalar::U8 => b[0] as f64,
            Scalar::I16 => i16::from_le_bytes([b[0], b[1]]) as f64,
            Scalar::U16 => u16::from_le_bytes([b[0], b[1]]) as f64,
            Scalar::I32 => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            Scalar::U32 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            Scalar::F32 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
            Scalar::F64 => f64::from_le_bytes(b),
        })
    }
    fn count(&mut self, kind: Scalar) -> Result<usize> {
        let n = self.value(kind)?;
        if !(0.0..=1e9).contains(&n) || n.fract() != 0.0 {
            return Err(format!("PLY: invalid list length {n}"));
        }
        Ok(n as usize)
    }
}

/// Split the header from the body; returns the header text and body offset.
fn header(bytes: &[u8]) -> Result<(&str, usize)> {
    if !bytes.starts_with(b"ply") {
        return Err("Not a PLY file (no \"ply\" magic)".into());
    }
    let marker = b"end_header";
    let at = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .ok_or("PLY header has no end_header")?;
    let mut body = at + marker.len();
    // The header ends with one line break: \n, or \r\n from some writers.
    if bytes.get(body) == Some(&b'\r') {
        body += 1;
    }
    if bytes.get(body) == Some(&b'\n') {
        body += 1;
    }
    let text = std::str::from_utf8(&bytes[..at]).map_err(|_| "PLY header is not text")?;
    Ok((text, body))
}

pub(super) fn read(bytes: &[u8], stem: &str, notes: &mut Vec<String>) -> Result<Vec<RawMesh>> {
    let (text, start) = header(bytes)?;
    let mut encoding = None;
    let mut elements: Vec<Element> = Vec::new();
    for line in text.lines().skip(1) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            [] | ["comment", ..] | ["obj_info", ..] => {}
            ["format", kind, _version] => {
                encoding = Some(match *kind {
                    "ascii" => Encoding::Ascii,
                    "binary_little_endian" => Encoding::Little,
                    "binary_big_endian" => Encoding::Big,
                    other => return Err(format!("Unknown PLY format {other:?}")),
                });
            }
            ["element", name, count] => elements.push(Element {
                name: (*name).into(),
                count: count
                    .parse()
                    .map_err(|_| format!("PLY: invalid element count {count:?}"))?,
                properties: Vec::new(),
            }),
            ["property", "list", count, value, name] => {
                let element = elements
                    .last_mut()
                    .ok_or("PLY property before any element")?;
                element.properties.push(Property {
                    name: (*name).into(),
                    list: Some(Scalar::parse(count).ok_or("PLY: unknown list count type")?),
                    value: Scalar::parse(value).ok_or("PLY: unknown property type")?,
                });
            }
            ["property", value, name] => {
                let element = elements
                    .last_mut()
                    .ok_or("PLY property before any element")?;
                element.properties.push(Property {
                    name: (*name).into(),
                    list: None,
                    value: Scalar::parse(value).ok_or("PLY: unknown property type")?,
                });
            }
            _ => return Err(format!("PLY header line not understood: {line:?}")),
        }
    }
    let encoding = encoding.ok_or("PLY header has no format line")?;
    let ascii_body = if encoding == Encoding::Ascii {
        std::str::from_utf8(&bytes[start..]).map_err(|_| "PLY ASCII data is not text")?
    } else {
        ""
    };
    let mut body = Body {
        bytes,
        at: start,
        encoding,
        words: ascii_body.split_ascii_whitespace(),
    };
    let mut points: Vec<DVec3> = Vec::new();
    let mut point_uv: Vec<Option<DVec2>> = Vec::new();
    let mut faces: Vec<RawFace> = Vec::new();
    let mut skipped_elements: Vec<String> = Vec::new();
    for element in &elements {
        let role = |names: &[&str]| {
            element
                .properties
                .iter()
                .position(|p| p.list.is_none() && names.contains(&p.name.as_str()))
        };
        match element.name.as_str() {
            "vertex" => {
                let (Some(x), Some(y), Some(z)) = (role(&["x"]), role(&["y"]), role(&["z"])) else {
                    return Err("PLY vertex element has no x, y and z".into());
                };
                let u = role(&["s", "u", "texture_u"]);
                let v = role(&["t", "v", "texture_v"]);
                let mut values = vec![0.0; element.properties.len()];
                points.reserve(element.count.min(1 << 24));
                for _ in 0..element.count {
                    for (i, p) in element.properties.iter().enumerate() {
                        values[i] = match p.list {
                            None => body.value(p.value)?,
                            Some(count) => {
                                for _ in 0..body.count(count)? {
                                    body.value(p.value)?;
                                }
                                0.0
                            }
                        };
                    }
                    points.push(DVec3::new(values[x], values[y], values[z]));
                    point_uv.push(u.zip(v).map(|(u, v)| DVec2::new(values[u], values[v])));
                }
            }
            "face" => {
                let indices = element
                    .properties
                    .iter()
                    .position(|p| {
                        p.list.is_some()
                            && matches!(p.name.as_str(), "vertex_indices" | "vertex_index")
                    })
                    .ok_or("PLY face element has no vertex_indices list")?;
                let texcoord = element
                    .properties
                    .iter()
                    .position(|p| p.list.is_some() && p.name == "texcoord");
                faces.reserve(element.count.min(1 << 24));
                for _ in 0..element.count {
                    let mut face = RawFace::default();
                    let mut corner_uv = Vec::new();
                    for (i, p) in element.properties.iter().enumerate() {
                        let Some(count) = p.list else {
                            body.value(p.value)?;
                            continue;
                        };
                        let n = body.count(count)?;
                        for _ in 0..n {
                            let value = body.value(p.value)?;
                            if i == indices {
                                if value < 0.0 || value.fract() != 0.0 || value > u32::MAX as f64 {
                                    return Err(format!("PLY: invalid vertex index {value}"));
                                }
                                face.vertices.push(value as u32);
                            } else if Some(i) == texcoord {
                                corner_uv.push(value);
                            }
                        }
                    }
                    if corner_uv.len() == face.vertices.len() * 2 && !corner_uv.is_empty() {
                        face.uv = Some(
                            corner_uv
                                .as_chunks::<2>()
                                .0
                                .iter()
                                .map(|&c| DVec2::from_array(c))
                                .collect(),
                        );
                    }
                    faces.push(face);
                }
            }
            _ => {
                // Edges, materials and anything else: read past them.
                for _ in 0..element.count {
                    for p in &element.properties {
                        match p.list {
                            None => {
                                body.value(p.value)?;
                            }
                            Some(count) => {
                                for _ in 0..body.count(count)? {
                                    body.value(p.value)?;
                                }
                            }
                        }
                    }
                }
                if element.count > 0 {
                    skipped_elements.push(format!("{} {}", element.count, element.name));
                }
            }
        }
    }
    if !skipped_elements.is_empty() {
        notes.push(format!(
            "PLY element(s) not imported: {}",
            skipped_elements.join(", ")
        ));
    }
    // Per-vertex UVs become per-corner UVs where the face has none of its own.
    if point_uv.iter().any(Option::is_some) {
        for face in faces.iter_mut().filter(|f| f.uv.is_none()) {
            let uv: Option<Vec<DVec2>> = face
                .vertices
                .iter()
                .map(|&v| point_uv.get(v as usize).copied().flatten())
                .collect();
            face.uv = uv;
        }
    }
    Ok(vec![RawMesh {
        name: stem.into(),
        points,
        faces,
        ..Default::default()
    }])
}

/// Every visible item's surface polygons in one point list, with per-corner
/// UVs when any item has them.
struct Flat {
    points: Vec<DVec3>,
    faces: Vec<Vec<u32>>,
    uvs: Option<Vec<Vec<DVec2>>>,
}

fn flatten(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Flat> {
    let mut flat = Flat {
        points: Vec::new(),
        faces: Vec::new(),
        uvs: meshes.iter().any(|m| m.uvs.is_some()).then(Vec::new),
    };
    let mut lines = 0;
    for m in meshes {
        lines += m.polyline.iter().filter(|&&p| p).count();
        let base = u32::try_from(flat.points.len()).map_err(|_| "Too many points for PLY")?;
        flat.points.extend(&m.mesh.vertices);
        for f in m.surface_faces() {
            let face = &m.mesh.faces[f];
            flat.faces
                .push(face.vertices.iter().map(|&v| v + base).collect());
            if let Some(uvs) = &mut flat.uvs {
                let corner = |c: usize| {
                    m.uvs
                        .as_ref()
                        .and_then(|u| u.get(f))
                        .and_then(|u| u.get(c).copied().flatten())
                        .unwrap_or(DVec2::ZERO)
                };
                uvs.push((0..face.vertices.len()).map(corner).collect());
            }
        }
    }
    if lines > 0 {
        notes.push(format!(
            "{lines} polyline(s) skipped: PLY export writes polygons only"
        ));
    }
    let items = meshes.iter().filter(|m| !m.mesh.faces.is_empty()).count();
    if items > 1 {
        notes.push(format!(
            "{items} items written as one PLY mesh (PLY has no objects)"
        ));
    }
    if flat.faces.iter().any(|f| f.len() > 255) {
        return Err("A polygon has more than 255 corners; PLY lists hold at most 255".into());
    }
    Ok(flat)
}

fn header_text(flat: &Flat, format: &str) -> String {
    let mut h = String::new();
    let _ = writeln!(h, "ply\nformat {format} 1.0\ncomment Written by Operandi");
    let _ = writeln!(
        h,
        "element vertex {}\nproperty float x\nproperty float y\nproperty float z",
        flat.points.len()
    );
    let _ = writeln!(
        h,
        "element face {}\nproperty list uchar int vertex_indices",
        flat.faces.len()
    );
    if flat.uvs.is_some() {
        h.push_str("property list uchar float texcoord\n");
    }
    h.push_str("end_header\n");
    h
}

pub(super) fn write_binary(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Vec<u8>> {
    let flat = flatten(meshes, notes)?;
    let mut out = header_text(&flat, "binary_little_endian").into_bytes();
    for p in &flat.points {
        for c in [p.x, p.y, p.z] {
            out.extend((c as f32).to_le_bytes());
        }
    }
    for (i, face) in flat.faces.iter().enumerate() {
        out.push(face.len() as u8);
        for &v in face {
            out.extend((v as i32).to_le_bytes());
        }
        if let Some(uvs) = &flat.uvs {
            out.push((uvs[i].len() * 2) as u8);
            for uv in &uvs[i] {
                out.extend((uv.x as f32).to_le_bytes());
                out.extend((uv.y as f32).to_le_bytes());
            }
        }
    }
    Ok(out)
}

pub(super) fn write_ascii(meshes: &[WorldMesh], notes: &mut Vec<String>) -> Result<Vec<u8>> {
    let flat = flatten(meshes, notes)?;
    let mut out = header_text(&flat, "ascii");
    for p in &flat.points {
        let _ = writeln!(out, "{} {} {}", num(p.x), num(p.y), num(p.z));
    }
    for (i, face) in flat.faces.iter().enumerate() {
        let _ = write!(out, "{}", face.len());
        for v in face {
            let _ = write!(out, " {v}");
        }
        if let Some(uvs) = &flat.uvs {
            let _ = write!(out, " {}", uvs[i].len() * 2);
            for uv in &uvs[i] {
                let _ = write!(out, " {} {}", num(uv.x), num(uv.y));
            }
        }
        out.push('\n');
    }
    Ok(out.into_bytes())
}
