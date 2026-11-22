//! Compact storage for big meshes in scene files (native version 34).
//!
//! A mesh's points, polygons and UV maps are written as JSON arrays while
//! they are small, so small scenes stay readable. Once one of them has
//! `PACK_MIN` or more entries it is written as a packed object instead:
//!
//! ```text
//! {"packed": "points", "count": N, "blobs": {"xyz": "<base64>"}}
//! ```
//!
//! Each blob is base64 of little-endian numbers (u8, u32 or f64), so values
//! come back bit for bit. A pretty-printed polygon took about 600 bytes;
//! packed it takes about 45. Readers accept both forms, so older files keep
//! opening. On reading, every blob's length must match the declared
//! `count` exactly and every string index must be in range; point indices
//! are checked afterwards by `Mesh::validate`, as for the array form.
use crate::Face;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use glam::{DVec2, DVec3};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::marker::PhantomData;

/// Arrays with at least this many entries are packed.
pub const PACK_MIN: usize = 1024;

/// True when an array of `len` entries is written packed.
pub fn packs(len: usize) -> bool {
    len >= PACK_MIN
}

/// True when `mesh` writes any packed array (so its scene needs version 34).
pub fn mesh_packs(mesh: &crate::Mesh) -> bool {
    packs(mesh.vertices.len())
        || packs(mesh.faces.len())
        || mesh.uv_maps.values().any(|m| packs(m.len()))
}

/// A base64 blob, borrowed from the input where the reader allows it.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Blob<'a>(#[serde(borrow)] Cow<'a, str>);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Packed<'a> {
    packed: String,
    count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    strings: Vec<String>,
    #[serde(borrow)]
    blobs: BTreeMap<String, Blob<'a>>,
}

impl<'a> Packed<'a> {
    fn new(kind: &str, count: usize) -> Self {
        Self {
            packed: kind.into(),
            count,
            strings: Vec::new(),
            blobs: BTreeMap::new(),
        }
    }
    fn put(&mut self, name: &str, bytes: Vec<u8>) {
        self.blobs
            .insert(name.into(), Blob(Cow::Owned(STANDARD.encode(bytes))));
    }
    /// The blob `name`, which must hold exactly `len` values of `width` bytes.
    fn bytes(&self, name: &str, len: usize, width: usize) -> Result<Vec<u8>, String> {
        let blob = self
            .blobs
            .get(name)
            .ok_or_else(|| format!("packed {}: missing {name}", self.packed))?;
        let expected = len
            .checked_mul(width)
            .ok_or_else(|| format!("packed {}: {name} too large", self.packed))?;
        // Refuse before decoding when the text can't be that size.
        if blob.0.len() / 4 * 3 < expected || blob.0.len() / 4 * 3 > expected + 2 {
            return Err(format!(
                "packed {}: {name} holds the wrong number of values",
                self.packed
            ));
        }
        let bytes = STANDARD
            .decode(blob.0.as_bytes())
            .map_err(|e| format!("packed {}: {name}: {e}", self.packed))?;
        if bytes.len() != expected {
            return Err(format!(
                "packed {}: {name} holds the wrong number of values",
                self.packed
            ));
        }
        Ok(bytes)
    }
    fn u32s(&self, name: &str, len: usize) -> Result<Vec<u32>, String> {
        Ok(self
            .bytes(name, len, 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| u32::from_le_bytes(c))
            .collect())
    }
    fn f64s(&self, name: &str, len: usize) -> Result<Vec<f64>, String> {
        Ok(self
            .bytes(name, len, 8)?
            .as_chunks::<8>()
            .0
            .iter()
            .map(|&c| f64::from_le_bytes(c))
            .collect())
    }
    fn expect(&self, kind: &str) -> Result<(), String> {
        if self.packed == kind {
            Ok(())
        } else {
            Err(format!("expected packed {kind}, found {}", self.packed))
        }
    }
}

fn le_u32(values: impl Iterator<Item = u32>, capacity: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(capacity * 4);
    values.for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
    out
}
fn le_f64(values: impl Iterator<Item = f64>, capacity: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(capacity * 8);
    values.for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
    out
}
fn count_u32(n: usize) -> Result<u32, String> {
    u32::try_from(n).map_err(|_| "packed mesh array too large".to_string())
}
/// Sum of per-entry corner counts, refusing overflow.
fn total(counts: &[u32]) -> Result<usize, String> {
    counts.iter().try_fold(0usize, |sum, &c| {
        sum.checked_add(c as usize)
            .ok_or_else(|| "packed corner counts overflow".to_string())
    })
}

/// An array type that has a packed form.
trait Packable: Sized {
    type Item: for<'de> Deserialize<'de>;
    fn from_items(items: Vec<Self::Item>) -> Self;
    fn pack(&self) -> Result<Packed<'static>, String>;
    fn unpack(packed: Packed<'_>) -> Result<Self, String>;
}

impl Packable for Vec<DVec3> {
    type Item = DVec3;
    fn from_items(items: Vec<DVec3>) -> Self {
        items
    }
    fn pack(&self) -> Result<Packed<'static>, String> {
        let mut p = Packed::new("points", self.len());
        p.put(
            "xyz",
            le_f64(self.iter().flat_map(|v| v.to_array()), self.len() * 3),
        );
        Ok(p)
    }
    fn unpack(p: Packed<'_>) -> Result<Self, String> {
        p.expect("points")?;
        let xyz = p.f64s("xyz", p.count.checked_mul(3).ok_or("too many points")?)?;
        Ok(xyz
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&c| DVec3::from_array(c))
            .collect())
    }
}

const SMOOTH: u8 = 1;
const HAS_UV: u8 = 2;
const HAS_PART: u8 = 4;
const HAS_LABEL: u8 = 8;

impl Packable for Vec<Face> {
    type Item = Face;
    fn from_items(items: Vec<Face>) -> Self {
        items
    }
    fn pack(&self) -> Result<Packed<'static>, String> {
        let mut p = Packed::new("polygons", self.len());
        // Materials, parts and labels share one string table.
        let mut table: BTreeMap<String, u32> = BTreeMap::new();
        let mut strings = Vec::new();
        let mut intern = |s: &str| -> Result<u32, String> {
            if let Some(&i) = table.get(s) {
                return Ok(i);
            }
            let i = count_u32(strings.len())?;
            strings.push(s.to_string());
            table.insert(s.to_string(), i);
            Ok(i)
        };
        let mut corners = Vec::with_capacity(self.len());
        let mut flags = Vec::with_capacity(self.len());
        let mut material = Vec::with_capacity(self.len());
        let mut part = Vec::new();
        let mut label = Vec::new();
        let mut uv = Vec::new();
        let mut indices = 0;
        for f in self {
            corners.push(count_u32(f.vertices.len())?);
            indices += f.vertices.len();
            let mut flag = 0;
            if f.smooth {
                flag |= SMOOTH;
            }
            if let Some(u) = &f.uv {
                // Per-face UVs must match the corner count to pack; the
                // validator enforces the same.
                if u.len() != f.vertices.len() {
                    return Err("polygon UV count differs from its corners".into());
                }
                flag |= HAS_UV;
                uv.extend(u.iter().flat_map(|v| v.to_array()));
            }
            if let Some(s) = &f.part {
                flag |= HAS_PART;
                part.push(intern(s)?);
            }
            if let Some(s) = &f.material_label {
                flag |= HAS_LABEL;
                label.push(intern(s)?);
            }
            material.push(intern(&f.material)?);
            flags.push(flag);
        }
        p.put("corners", le_u32(corners.iter().copied(), corners.len()));
        p.put(
            "indices",
            le_u32(
                self.iter().flat_map(|f| f.vertices.iter().copied()),
                indices,
            ),
        );
        p.put("flags", flags);
        p.put("material", le_u32(material.iter().copied(), material.len()));
        p.put("part", le_u32(part.iter().copied(), part.len()));
        p.put("label", le_u32(label.iter().copied(), label.len()));
        p.put("uv", le_f64(uv.iter().copied(), uv.len()));
        p.strings = strings;
        Ok(p)
    }
    fn unpack(p: Packed<'_>) -> Result<Self, String> {
        p.expect("polygons")?;
        let n = p.count;
        let corners = p.u32s("corners", n)?;
        let flags = p.bytes("flags", n, 1)?;
        let material = p.u32s("material", n)?;
        let count = |bit: u8| flags.iter().filter(|&&f| f & bit != 0).count();
        let part = p.u32s("part", count(HAS_PART))?;
        let label = p.u32s("label", count(HAS_LABEL))?;
        let indices = p.u32s("indices", total(&corners)?)?;
        let uv_corners = total(
            &corners
                .iter()
                .zip(&flags)
                .map(|(&c, &f)| if f & HAS_UV != 0 { c } else { 0 })
                .collect::<Vec<_>>(),
        )?;
        let uv = p.f64s("uv", uv_corners.checked_mul(2).ok_or("too many UVs")?)?;
        let string = |i: u32| {
            p.strings
                .get(i as usize)
                .cloned()
                .ok_or_else(|| "packed polygons: string index out of range".to_string())
        };
        let (mut at, mut uv_at, mut part_at, mut label_at) = (0, 0, 0, 0);
        let mut faces = Vec::with_capacity(n);
        for i in 0..n {
            let c = corners[i] as usize;
            let flag = flags[i];
            if flag & !(SMOOTH | HAS_UV | HAS_PART | HAS_LABEL) != 0 {
                return Err("packed polygons: unknown flag".into());
            }
            let face_uv = (flag & HAS_UV != 0).then(|| {
                let u = uv[uv_at..uv_at + 2 * c]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&v| DVec2::from_array(v))
                    .collect();
                uv_at += 2 * c;
                u
            });
            let face_part = if flag & HAS_PART != 0 {
                part_at += 1;
                Some(string(part[part_at - 1])?)
            } else {
                None
            };
            let face_label = if flag & HAS_LABEL != 0 {
                label_at += 1;
                Some(string(label[label_at - 1])?)
            } else {
                None
            };
            faces.push(Face {
                vertices: indices[at..at + c].to_vec(),
                uv: face_uv,
                material: string(material[i])?,
                smooth: flag & SMOOTH != 0,
                part: face_part,
                material_label: face_label,
            });
            at += c;
        }
        Ok(faces)
    }
}

/// One UV map: per polygon, per corner, a coordinate or none.
impl Packable for Vec<Vec<Option<DVec2>>> {
    type Item = Vec<Option<DVec2>>;
    fn from_items(items: Vec<Self::Item>) -> Self {
        items
    }
    fn pack(&self) -> Result<Packed<'static>, String> {
        let mut p = Packed::new("uv map", self.len());
        let mut corners = Vec::with_capacity(self.len());
        let mut present = Vec::new();
        let mut values = Vec::new();
        for face in self {
            corners.push(count_u32(face.len())?);
            for v in face {
                present.push(u8::from(v.is_some()));
                if let Some(v) = v {
                    values.extend([v.x, v.y]);
                }
            }
        }
        p.put("corners", le_u32(corners.iter().copied(), corners.len()));
        p.put("present", present);
        p.put("uv", le_f64(values.iter().copied(), values.len()));
        Ok(p)
    }
    fn unpack(p: Packed<'_>) -> Result<Self, String> {
        p.expect("uv map")?;
        let corners = p.u32s("corners", p.count)?;
        let present = p.bytes("present", total(&corners)?, 1)?;
        if present.iter().any(|&b| b > 1) {
            return Err("packed uv map: bad presence flag".into());
        }
        let set = present.iter().filter(|&&b| b == 1).count();
        let uv = p.f64s("uv", set.checked_mul(2).ok_or("too many UVs")?)?;
        let (mut at, mut uv_at) = (0, 0);
        let mut map = Vec::with_capacity(p.count);
        for &c in &corners {
            let face = present[at..at + c as usize]
                .iter()
                .map(|&b| {
                    (b == 1).then(|| {
                        uv_at += 2;
                        DVec2::new(uv[uv_at - 2], uv[uv_at - 1])
                    })
                })
                .collect();
            map.push(face);
            at += c as usize;
        }
        Ok(map)
    }
}

/// Reads either form: a JSON array or a packed object.
struct Either<T>(PhantomData<T>);
impl<'de, T: Packable> Visitor<'de> for Either<T> {
    type Value = T;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an array or a packed array object")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<T, A::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(1 << 16));
        while let Some(item) = seq.next_element::<T::Item>()? {
            items.push(item);
        }
        Ok(T::from_items(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        let packed = Packed::deserialize(de::value::MapAccessDeserializer::new(map))?;
        T::unpack(packed).map_err(de::Error::custom)
    }
}
fn read<'de, D: Deserializer<'de>, T: Packable>(d: D) -> Result<T, D::Error> {
    d.deserialize_any(Either::<T>(PhantomData))
}
fn write<S: Serializer, T: Packable + Serialize>(
    value: &T,
    len: usize,
    s: S,
) -> Result<S::Ok, S::Error> {
    if packs(len) {
        value
            .pack()
            .map_err(serde::ser::Error::custom)?
            .serialize(s)
    } else {
        value.serialize(s)
    }
}

/// `Mesh::vertices`.
pub mod points {
    use super::*;
    pub fn serialize<S: Serializer>(v: &Vec<DVec3>, s: S) -> Result<S::Ok, S::Error> {
        write(v, v.len(), s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<DVec3>, D::Error> {
        read(d)
    }
}
/// `Mesh::faces`.
pub mod polygons {
    use super::*;
    pub fn serialize<S: Serializer>(v: &crate::Shared<Vec<Face>>, s: S) -> Result<S::Ok, S::Error> {
        write(&**v, v.len(), s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<crate::Shared<Vec<Face>>, D::Error> {
        read::<_, Vec<Face>>(d).map(Into::into)
    }
}
/// `Mesh::uv_maps`: each map packed on its own.
pub mod uv_maps {
    use super::*;
    type Map = Vec<Vec<Option<DVec2>>>;
    struct Out<'a>(&'a Map);
    impl Serialize for Out<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            write(self.0, self.0.len(), s)
        }
    }
    struct In(Map);
    impl<'de> Deserialize<'de> for In {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            read(d).map(In)
        }
    }
    pub fn serialize<S: Serializer>(
        m: &crate::Shared<BTreeMap<String, Map>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_map(m.iter().map(|(k, v)| (k, Out(v))))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<crate::Shared<BTreeMap<String, Map>>, D::Error> {
        let m = BTreeMap::<String, In>::deserialize(d)?;
        Ok(m.into_iter().map(|(k, v)| (k, v.0)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mesh;

    fn mesh(n: usize) -> Mesh {
        let vertices = (0..n + 2)
            .map(|i| DVec3::new(i as f64 / 7., (i % 3) as f64, -0.1))
            .collect();
        let faces = (0..n)
            .map(|i| Face {
                vertices: vec![i as u32, i as u32 + 1, i as u32 + 2],
                uv: (i % 2 == 0).then(|| vec![DVec2::new(0.1, 0.2); 3]),
                material: format!("M{}", i % 3),
                smooth: i % 5 == 0,
                part: (i % 4 == 0).then(|| "Part".into()),
                material_label: (i % 6 == 0).then(|| "Label \"quoted\"".into()),
            })
            .collect();
        let mut mesh = Mesh {
            vertices,
            faces,
            ..Default::default()
        };
        let map = (0..n)
            .map(|i| vec![Some(DVec2::new(i as f64, 0.5)), None, Some(DVec2::ZERO)])
            .collect();
        mesh.uv_maps.insert("Texture".into(), map);
        mesh.uv_maps.insert("Empty".into(), Vec::new());
        mesh
    }

    #[test]
    fn packed_and_array_forms_read_back_the_same_mesh() {
        for n in [3, PACK_MIN - 3, PACK_MIN - 2, PACK_MIN, PACK_MIN + 5] {
            let m = mesh(n);
            let text = serde_json::to_string(&m).unwrap();
            assert_eq!(text.contains("\"packed\""), packs(n + 2), "{n}");
            assert_eq!(serde_json::from_str::<Mesh>(&text).unwrap(), m, "{n}");
            // And through a JSON value (owned strings, no borrowing).
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(serde_json::from_value::<Mesh>(value).unwrap(), m, "{n}");
        }
    }

    #[test]
    fn bad_packed_data_is_refused() {
        let text = serde_json::to_string(&mesh(PACK_MIN)).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let faces = &mut value["faces"];
        // A string index past the table.
        let mut bad = faces.clone();
        bad["strings"] = serde_json::json!([]);
        assert!(serde_json::from_value::<Vec<Face>>(bad.clone()).is_err());
        let mut m = value.clone();
        m["faces"] = bad;
        assert!(serde_json::from_value::<Mesh>(m).is_err());
        // Wrong kind, unknown key, missing blob, bad base64.
        for (key, v) in [
            ("packed", serde_json::json!("points")),
            ("extra", serde_json::json!(1)),
        ] {
            let mut m = value.clone();
            m["faces"][key] = v;
            assert!(serde_json::from_value::<Mesh>(m).is_err(), "{key}");
        }
        let mut m = value.clone();
        m["faces"]["blobs"]
            .as_object_mut()
            .unwrap()
            .remove("indices");
        assert!(serde_json::from_value::<Mesh>(m).is_err());
        let mut m = value.clone();
        m["vertices"]["blobs"]["xyz"] = serde_json::json!("!!!!");
        assert!(serde_json::from_value::<Mesh>(m).is_err());
    }
}
