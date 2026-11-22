//! Packed mesh storage (stage 1 of the packed-mesh plan).
//!
//! A `PackedMesh` holds the same contents as a `Mesh` in a few flat arrays
//! instead of one heap object per polygon. Every array sits behind its own
//! `Arc`, so a packed copy that only moves points shares its polygon, tag and
//! UV arrays with the version it came from.
//!
//! Packing is exact: `PackedMesh::from_mesh(m).to_mesh() == m`, with every
//! `f64` coming back bit for bit (signed zeros and NaN payloads included).
//!
//! Layout, for `P` polygons with `C` corners in total:
//! - points: one `DVec3` per point (f64, as in `Mesh`);
//! - polygons: `P + 1` start offsets into one array of `C` point indices;
//! - tags: one name table shared by materials, parts and material labels,
//!   with a `u16` index per polygon (`u32` once a mesh has more than 65,535
//!   distinct names), plus one smooth bit per polygon;
//! - UV maps: each distinct value once plus a `u32` per corner, or one
//!   `DVec2` per corner when that is smaller (see `CornerUvs`); the legacy
//!   `Face::uv` lists keep one `DVec2` per corner and a bit per polygon;
//! - `MeshAttributes` are kept as they are, behind one `Arc`.
//!
//! The `MeshRead` trait reads both forms, so code that only looks at a mesh
//! never has to unpack it. Nothing in the app stores packed meshes yet; the
//! slot that does (`MeshSlot`) is stage 2.
use crate::{Face, Mesh, MeshAttributes, Result};
use glam::{DVec2, DVec3};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Meshes with fewer polygons than this stay in their full form: packing
/// them saves too little to be worth the work. The same limit the scene
/// file packing uses (`mesh_pack::PACK_MIN`).
pub const PACK_MIN_POLYGONS: usize = crate::mesh_pack::PACK_MIN;

/// True when `mesh` is big enough to be stored packed.
pub fn worth_packing(mesh: &Mesh) -> bool {
    mesh.faces.len() >= PACK_MIN_POLYGONS
}

/// A fixed-length bit array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bits {
    words: Box<[u64]>,
    len: usize,
}
impl Bits {
    fn from_iter(len: usize, values: impl Iterator<Item = bool>) -> Self {
        let mut words = vec![0u64; len.div_ceil(64)].into_boxed_slice();
        for (i, v) in values.enumerate() {
            if v {
                words[i / 64] |= 1 << (i % 64);
            }
        }
        Self { words, len }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn get(&self, i: usize) -> bool {
        i < self.len && self.words[i / 64] & (1 << (i % 64)) != 0
    }
    fn all(&self) -> bool {
        (0..self.len).all(|i| self.get(i))
    }
    fn heap_bytes(&self) -> usize {
        self.words.len() * 8
    }
}

/// Per-polygon indices into the name table: narrow while it fits.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Indices {
    U16(Box<[u16]>),
    U32(Box<[u32]>),
}
impl Indices {
    fn new(values: Vec<u32>, table_len: usize) -> Self {
        if table_len <= u16::MAX as usize + 1 {
            Self::U16(values.into_iter().map(|v| v as u16).collect())
        } else {
            Self::U32(values.into_boxed_slice())
        }
    }
    fn get(&self, f: usize) -> usize {
        match self {
            Self::U16(v) => v[f] as usize,
            Self::U32(v) => v[f] as usize,
        }
    }
    fn heap_bytes(&self) -> usize {
        match self {
            Self::U16(v) => v.len() * 2,
            Self::U32(v) => v.len() * 4,
        }
    }
}

/// Material, part, label and smooth flag of every polygon.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PolygonTags {
    /// Every distinct material, part and label name, each once.
    names: Box<[String]>,
    material: Indices,
    /// `None` when no polygon has a part. Otherwise 0 = no part and
    /// `i + 1` = `names[i]`.
    part: Option<Indices>,
    /// As `part`, for `Face::material_label`.
    label: Option<Indices>,
    smooth: Bits,
}
impl PolygonTags {
    fn heap_bytes(&self) -> usize {
        self.names
            .iter()
            .map(|n| n.capacity() + std::mem::size_of::<String>())
            .sum::<usize>()
            + self.material.heap_bytes()
            + self.part.as_ref().map_or(0, Indices::heap_bytes)
            + self.label.as_ref().map_or(0, Indices::heap_bytes)
            + self.smooth.heap_bytes()
    }
    fn optional(&self, indices: &Option<Indices>, f: usize) -> Option<&str> {
        match indices.as_ref()?.get(f) {
            0 => None,
            i => Some(&self.names[i - 1]),
        }
    }
}

/// One UV map, read per corner of the polygons' corner array.
///
/// Neighbouring polygons usually share the UV of a shared point, so most
/// maps store each distinct value once plus a `u32` per corner (about 32
/// bytes a quad instead of 64). Maps where most corners differ store one
/// value per corner instead, whichever is smaller. Values are matched by
/// their exact bits, so both layouts give back every value unchanged.
#[derive(Clone, Debug, PartialEq)]
pub enum CornerUvs {
    Shared {
        values: Box<[DVec2]>,
        /// Index into `values` per corner; `UNSET` for a corner with no UV.
        corners: Box<[u32]>,
    },
    PerCorner {
        /// Unset corners hold zero; `set` says which hold a value.
        values: Box<[DVec2]>,
        /// `None` when every corner holds a value.
        set: Option<Bits>,
    },
}
const UNSET: u32 = u32::MAX;
impl CornerUvs {
    fn new(corners: impl Iterator<Item = Option<DVec2>> + Clone, count: usize) -> Self {
        // A shared point's UV repeats at about four corners: size for that.
        let mut index: crate::fxhash::FxHashMap<[u64; 2], u32> =
            crate::fxhash::FxHashMap::with_capacity_and_hasher(count / 4, Default::default());
        let mut values = Vec::new();
        let mut shared = Vec::with_capacity(count);
        for uv in corners.clone() {
            shared.push(match uv {
                None => UNSET,
                Some(v) => *index
                    .entry([v.x.to_bits(), v.y.to_bits()])
                    .or_insert_with(|| {
                        values.push(v);
                        values.len() as u32 - 1
                    }),
            });
        }
        // `UNSET` is reserved, so a table can't reach it.
        let fits = values.len() < UNSET as usize;
        if fits && values.len() * 16 + count * 4 <= count * 16 + count.div_ceil(64) * 8 {
            return Self::Shared {
                values: values.into_boxed_slice(),
                corners: shared.into_boxed_slice(),
            };
        }
        drop((index, values, shared));
        let set = Bits::from_iter(count, corners.clone().map(|v| v.is_some()));
        Self::PerCorner {
            values: corners.map(|v| v.unwrap_or(DVec2::ZERO)).collect(),
            set: (!set.all()).then_some(set),
        }
    }
    fn get(&self, corner: usize) -> Option<DVec2> {
        match self {
            Self::Shared { values, corners } => match *corners.get(corner)? {
                UNSET => None,
                i => Some(values[i as usize]),
            },
            Self::PerCorner { values, set } => {
                let value = *values.get(corner)?;
                match set {
                    Some(set) if !set.get(corner) => None,
                    _ => Some(value),
                }
            }
        }
    }
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Shared { values, corners } => values.len() * 16 + corners.len() * 4,
            Self::PerCorner { values, set } => {
                values.len() * 16 + set.as_ref().map_or(0, Bits::heap_bytes)
            }
        }
    }
}

/// The legacy per-polygon `Face::uv` lists.
#[derive(Clone, Debug, PartialEq)]
struct FaceUvs {
    /// One bit per polygon: does it have a `Face::uv` list?
    faces: Bits,
    /// One value per corner; corners of polygons without a list hold zero.
    values: Box<[DVec2]>,
}

/// A mesh in packed form. Cloning is cheap (one reference count per array).
#[derive(Clone, Debug)]
pub struct PackedMesh {
    points: Arc<[DVec3]>,
    /// `polygon_count + 1` offsets into `corners`.
    starts: Arc<[u32]>,
    corners: Arc<[u32]>,
    tags: Arc<PolygonTags>,
    face_uvs: Option<Arc<FaceUvs>>,
    uv_maps: BTreeMap<String, Arc<CornerUvs>>,
    attributes: Arc<MeshAttributes>,
}

impl PackedMesh {
    /// Packs `mesh`. Refuses (and the caller keeps the full form) only when
    /// the mesh has more corners than a `u32` can count, or a UV list whose
    /// shape does not match its polygons, which `Mesh::validate` rejects too.
    pub fn from_mesh(mesh: &Mesh) -> Result<Self> {
        let mut starts = Vec::with_capacity(mesh.faces.len() + 1);
        let mut corner_total = 0usize;
        starts.push(0);
        for face in &mesh.faces {
            corner_total += face.vertices.len();
            starts.push(
                u32::try_from(corner_total)
                    .map_err(|_| "Mesh has too many polygon corners to pack".to_string())?,
            );
        }
        let mut corners = Vec::with_capacity(corner_total);
        for face in &mesh.faces {
            corners.extend_from_slice(&face.vertices);
        }

        // Face::uv: every list must match its polygon's corners.
        let face_uvs = if mesh.faces.iter().any(|f| f.uv.is_some()) {
            let mut values = Vec::with_capacity(corner_total);
            for face in &mesh.faces {
                match &face.uv {
                    Some(uv) if uv.len() == face.vertices.len() => values.extend_from_slice(uv),
                    Some(_) => return Err("A polygon's UV list does not match its corners".into()),
                    None => values.extend(std::iter::repeat_n(DVec2::ZERO, face.vertices.len())),
                }
            }
            Some(Arc::new(FaceUvs {
                faces: Bits::from_iter(mesh.faces.len(), mesh.faces.iter().map(|f| f.uv.is_some())),
                values: values.into_boxed_slice(),
            }))
        } else {
            None
        };

        let mut uv_maps = BTreeMap::new();
        for (name, polygons) in &mesh.uv_maps {
            if polygons.len() != mesh.faces.len()
                || polygons
                    .iter()
                    .zip(&mesh.faces)
                    .any(|(uv, face)| uv.len() != face.vertices.len())
            {
                return Err(format!("UV map {name} does not match the mesh's polygons"));
            }
            let corners = polygons.iter().flatten().copied();
            uv_maps.insert(
                name.clone(),
                Arc::new(CornerUvs::new(corners, corner_total)),
            );
        }

        Ok(Self {
            points: mesh.vertices.as_slice().into(),
            starts: starts.into(),
            corners: corners.into(),
            tags: Arc::new(pack_tags(&mesh.faces)),
            face_uvs,
            uv_maps,
            attributes: Arc::new((*mesh.attributes).clone()),
        })
    }

    /// The full, editable form. Equal to the mesh this was packed from.
    pub fn to_mesh(&self) -> Mesh {
        let faces = (0..self.polygon_count())
            .map(|f| Face {
                vertices: self.polygon(f).to_vec(),
                uv: self.face_uvs.as_ref().and_then(|uvs| {
                    uvs.faces
                        .get(f)
                        .then(|| uvs.values[self.corner_range(f)].to_vec())
                }),
                material: self.material(f).to_owned(),
                smooth: self.smooth(f),
                part: self.part(f).map(str::to_owned),
                material_label: self.material_label(f).map(str::to_owned),
            })
            .collect();
        let uv_maps = self
            .uv_maps
            .iter()
            .map(|(name, uvs)| {
                let polygons = (0..self.polygon_count())
                    .map(|f| self.corner_range(f).map(|c| uvs.get(c)).collect())
                    .collect();
                (name.clone(), polygons)
            })
            .collect();
        Mesh {
            attributes: ((*self.attributes).clone()).into(),
            vertices: self.points.to_vec(),
            faces,
            uv_maps,
        }
    }

    /// Range of `polygon`'s corners in the corner arrays.
    fn corner_range(&self, polygon: usize) -> std::ops::Range<usize> {
        self.starts[polygon] as usize..self.starts[polygon + 1] as usize
    }

    /// Heap bytes this packed mesh holds, counting shared arrays in full.
    /// `MeshAttributes` are counted by `attributes_heap_bytes`.
    pub fn heap_bytes(&self) -> usize {
        self.points.len() * 24
            + self.starts.len() * 4
            + self.corners.len() * 4
            + self.tags.heap_bytes()
            + self
                .face_uvs
                .as_ref()
                .map_or(0, |u| u.faces.heap_bytes() + u.values.len() * 16)
            + self
                .uv_maps
                .iter()
                .map(|(n, u)| n.capacity() + u.heap_bytes())
                .sum::<usize>()
    }

    /// True when `a` and `b` share their polygon arrays (corners, starts and
    /// tags), so a points-only change between them copied none of those.
    pub fn shares_polygons(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.starts, &b.starts)
            && Arc::ptr_eq(&a.corners, &b.corners)
            && Arc::ptr_eq(&a.tags, &b.tags)
    }

    /// True when `a` and `b` share every UV array.
    pub fn shares_uvs(a: &Self, b: &Self) -> bool {
        let face = match (&a.face_uvs, &b.face_uvs) {
            (Some(x), Some(y)) => Arc::ptr_eq(x, y),
            (None, None) => true,
            _ => false,
        };
        face && a.uv_maps.len() == b.uv_maps.len()
            && a.uv_maps
                .iter()
                .zip(&b.uv_maps)
                .all(|((na, x), (nb, y))| na == nb && Arc::ptr_eq(x, y))
    }

    /// A copy with new point positions and everything else shared. Point
    /// count must not change, since polygons index the points.
    pub fn with_points(&self, points: Vec<DVec3>) -> Result<Self> {
        if points.len() != self.points.len() {
            return Err("New points must keep the point count".into());
        }
        Ok(Self {
            points: points.into(),
            ..self.clone()
        })
    }
}

/// Builds the shared name table and the per-polygon indices into it.
fn pack_tags(faces: &[Face]) -> PolygonTags {
    // Each distinct name is stored once; the map's keys borrow from `faces`.
    let mut names: Vec<&str> = Vec::new();
    let mut index: HashMap<&str, u32> = HashMap::new();
    let material: Vec<u32> = faces
        .iter()
        .map(|f| intern_name(&f.material, &mut names, &mut index))
        .collect();
    let part = faces.iter().any(|f| f.part.is_some()).then(|| {
        faces
            .iter()
            .map(|f| {
                f.part
                    .as_deref()
                    .map_or(0, |n| intern_name(n, &mut names, &mut index) + 1)
            })
            .collect::<Vec<u32>>()
    });
    let label = faces.iter().any(|f| f.material_label.is_some()).then(|| {
        faces
            .iter()
            .map(|f| {
                f.material_label
                    .as_deref()
                    .map_or(0, |n| intern_name(n, &mut names, &mut index) + 1)
            })
            .collect::<Vec<u32>>()
    });
    // Optional indices are shifted by one, so they need one more slot.
    let table = names.len() + 1;
    PolygonTags {
        material: Indices::new(material, table),
        part: part.map(|v| Indices::new(v, table)),
        label: label.map(|v| Indices::new(v, table)),
        smooth: Bits::from_iter(faces.len(), faces.iter().map(|f| f.smooth)),
        names: names.into_iter().map(str::to_owned).collect(),
    }
}

fn intern_name<'a>(
    name: &'a str,
    names: &mut Vec<&'a str>,
    index: &mut HashMap<&'a str, u32>,
) -> u32 {
    *index.entry(name).or_insert_with(|| {
        names.push(name);
        names.len() as u32 - 1
    })
}

/// One UV map of either form, read per polygon corner.
#[derive(Clone, Copy)]
pub enum UvMapRef<'a> {
    Full(&'a [Vec<Option<DVec2>>]),
    Packed {
        uvs: &'a CornerUvs,
        starts: &'a [u32],
    },
}
impl UvMapRef<'_> {
    /// The UV of corner `corner` of polygon `polygon`, if it has one.
    pub fn get(&self, polygon: usize, corner: usize) -> Option<DVec2> {
        match self {
            Self::Full(polygons) => polygons.get(polygon)?.get(corner).copied().flatten(),
            Self::Packed { uvs, starts } => {
                let start = *starts.get(polygon)? as usize;
                let end = *starts.get(polygon + 1)? as usize;
                (start + corner < end)
                    .then(|| uvs.get(start + corner))
                    .flatten()
            }
        }
    }
}

/// Read-only access to a mesh in either form. Code that only looks at a
/// mesh goes through this, so a packed mesh never has to be unpacked.
pub trait MeshRead {
    fn points(&self) -> &[DVec3];
    fn polygon_count(&self) -> usize;
    /// The point indices of `polygon`, in winding order.
    fn polygon(&self, polygon: usize) -> &[u32];
    fn material(&self, polygon: usize) -> &str;
    fn part(&self, polygon: usize) -> Option<&str>;
    fn material_label(&self, polygon: usize) -> Option<&str>;
    fn smooth(&self, polygon: usize) -> bool;
    /// True when `polygon` carries a legacy `Face::uv` list.
    fn has_face_uv(&self, polygon: usize) -> bool;
    /// Corner `corner` of `polygon`'s legacy `Face::uv` list.
    fn face_uv(&self, polygon: usize, corner: usize) -> Option<DVec2>;
    fn uv_map_names(&self) -> Vec<&str>;
    fn uv_map(&self, name: &str) -> Option<UvMapRef<'_>>;
    fn attributes(&self) -> &MeshAttributes;

    fn point_count(&self) -> usize {
        self.points().len()
    }
    fn point(&self, index: usize) -> DVec3 {
        self.points()[index]
    }
    fn corner_count(&self) -> usize {
        (0..self.polygon_count())
            .map(|f| self.polygon(f).len())
            .sum()
    }
}

impl MeshRead for Mesh {
    fn points(&self) -> &[DVec3] {
        &self.vertices
    }
    fn polygon_count(&self) -> usize {
        self.faces.len()
    }
    fn polygon(&self, polygon: usize) -> &[u32] {
        &self.faces[polygon].vertices
    }
    fn material(&self, polygon: usize) -> &str {
        &self.faces[polygon].material
    }
    fn part(&self, polygon: usize) -> Option<&str> {
        self.faces[polygon].part.as_deref()
    }
    fn material_label(&self, polygon: usize) -> Option<&str> {
        self.faces[polygon].material_label.as_deref()
    }
    fn smooth(&self, polygon: usize) -> bool {
        self.faces[polygon].smooth
    }
    fn has_face_uv(&self, polygon: usize) -> bool {
        self.faces[polygon].uv.is_some()
    }
    fn face_uv(&self, polygon: usize, corner: usize) -> Option<DVec2> {
        self.faces[polygon].uv.as_ref()?.get(corner).copied()
    }
    fn uv_map_names(&self) -> Vec<&str> {
        self.uv_maps.keys().map(String::as_str).collect()
    }
    fn uv_map(&self, name: &str) -> Option<UvMapRef<'_>> {
        self.uv_maps.get(name).map(|m| UvMapRef::Full(m))
    }
    fn attributes(&self) -> &MeshAttributes {
        &self.attributes
    }
    fn corner_count(&self) -> usize {
        self.faces.iter().map(|f| f.vertices.len()).sum()
    }
}

impl MeshRead for PackedMesh {
    fn points(&self) -> &[DVec3] {
        &self.points
    }
    fn polygon_count(&self) -> usize {
        self.starts.len() - 1
    }
    fn polygon(&self, polygon: usize) -> &[u32] {
        &self.corners[self.corner_range(polygon)]
    }
    fn material(&self, polygon: usize) -> &str {
        &self.tags.names[self.tags.material.get(polygon)]
    }
    fn part(&self, polygon: usize) -> Option<&str> {
        self.tags.optional(&self.tags.part, polygon)
    }
    fn material_label(&self, polygon: usize) -> Option<&str> {
        self.tags.optional(&self.tags.label, polygon)
    }
    fn smooth(&self, polygon: usize) -> bool {
        self.tags.smooth.get(polygon)
    }
    fn has_face_uv(&self, polygon: usize) -> bool {
        self.face_uvs.as_ref().is_some_and(|u| u.faces.get(polygon))
    }
    fn face_uv(&self, polygon: usize, corner: usize) -> Option<DVec2> {
        let uvs = self.face_uvs.as_ref()?;
        let range = self.corner_range(polygon);
        (uvs.faces.get(polygon) && corner < range.len()).then(|| uvs.values[range.start + corner])
    }
    fn uv_map_names(&self) -> Vec<&str> {
        self.uv_maps.keys().map(String::as_str).collect()
    }
    fn uv_map(&self, name: &str) -> Option<UvMapRef<'_>> {
        self.uv_maps.get(name).map(|uvs| UvMapRef::Packed {
            uvs,
            starts: &self.starts,
        })
    }
    fn attributes(&self) -> &MeshAttributes {
        &self.attributes
    }
    fn corner_count(&self) -> usize {
        self.corners.len()
    }
}

/// Heap bytes a full `Mesh` holds for its points, polygons and UV maps (not
/// its attributes), counting allocated capacity. For the memory probe and
/// for comparing with `PackedMesh::heap_bytes`.
pub fn full_heap_bytes(mesh: &Mesh) -> usize {
    use std::mem::size_of;
    let faces: usize = mesh
        .faces
        .iter()
        .map(|f| {
            f.vertices.capacity() * 4
                + f.uv.as_ref().map_or(0, |u| u.capacity() * 16)
                + f.material.capacity()
                + f.part.as_ref().map_or(0, String::capacity)
                + f.material_label.as_ref().map_or(0, String::capacity)
        })
        .sum();
    let uvs: usize = mesh
        .uv_maps
        .iter()
        .map(|(name, polygons)| {
            name.capacity()
                + polygons.capacity() * size_of::<Vec<Option<DVec2>>>()
                + polygons
                    .iter()
                    .map(|c| c.capacity() * size_of::<Option<DVec2>>())
                    .sum::<usize>()
        })
        .sum();
    mesh.vertices.capacity() * 24 + mesh.faces.capacity() * size_of::<Face>() + faces + uvs
}
