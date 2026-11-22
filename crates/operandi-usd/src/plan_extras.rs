//! What a mesh carries beyond its shape, for the USD export: weight maps,
//! typed maps, colour maps, morphs, parts and selection sets. Worked out
//! in plain Rust like the rest of the plan; the bridge writes them with
//! `ou_define_mesh_extras`, and the import (`import_extras.rs`) reads
//! them back.
//!
//! How each lands in USD (USD-IQ22, provisional):
//! - weight maps: float primvars; typed maps: float, float2..4 or (6
//!   numbers) float with elementSize 6 primvars, rgb3 / rgba4 as colours;
//!   colour maps: color4f primvars. The first colour map of a mesh is
//!   `displayColor` (color3f), with `displayOpacity` beside it when any
//!   alpha isn't 1. Point maps are `vertex` primvars, corner maps
//!   `faceVarying`. A map that doesn't cover every point or corner is an
//!   indexed primvar whose `unauthoredValuesIndex` (0) marks the missing
//!   ones, so "no value" stays apart from 0.
//! - morph maps: UsdSkelBlendShape prims bound with UsdSkelBindingAPI.
//! - parts and polygon selection sets: face GeomSubsets; point selection
//!   sets: point GeomSubsets, in families "operandi:part",
//!   "operandi:polygonSet" and "operandi:pointSet".
//! - edge selection sets: the mesh's customData, as point pairs.
//!
//! Each primvar, subset and blend shape carries customData under
//! "operandi": its Operandi name (USD names are made safe) and what kind
//! of map it is, so the import puts it back where it came from.
use operandi_core::Mesh;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Everything `ou_define_mesh_extras` writes on one Mesh prim.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshExtras {
    pub primvars: Vec<PrimvarOut>,
    pub subsets: Vec<GroupOut>,
    pub blend_shapes: Vec<BlendShapeOut>,
    /// The mesh prim's own customData (under "operandi").
    pub custom: Map<String, Value>,
    /// A mesh light on this mesh (mesh_lights.rs): MeshLightAPI's inputs.
    pub light: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimvarOut {
    /// The primvar's USD name (already safe and unique on its mesh).
    pub name: String,
    /// "float", "float2", "float3", "float4", "color3" or "color4".
    pub ty: &'static str,
    /// Numbers per element for a "float" holding more than one.
    pub element_size: usize,
    /// "vertex" or "faceVarying".
    pub interpolation: &'static str,
    pub values: Vec<f32>,
    pub indices: Option<Vec<i32>>,
    pub unauthored: Option<i32>,
    pub custom: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroupOut {
    pub name: String,
    /// "face" or "point".
    pub element: &'static str,
    pub family: &'static str,
    pub family_type: &'static str,
    pub indices: Vec<i32>,
    pub custom: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlendShapeOut {
    pub name: String,
    pub offsets: Vec<[f32; 3]>,
    /// The points the offsets move, when not every point.
    pub point_indices: Option<Vec<i32>>,
    pub custom: Map<String, Value>,
}

pub const FAMILY_PART: &str = "operandi:part";
pub const FAMILY_POLYGON_SET: &str = "operandi:polygonSet";
pub const FAMILY_POINT_SET: &str = "operandi:pointSet";

impl MeshExtras {
    pub fn is_empty(&self) -> bool {
        self.primvars.is_empty()
            && self.subsets.is_empty()
            && self.blend_shapes.is_empty()
            && self.custom.is_empty()
            && self.light.is_none()
    }

    /// The description `ou_define_mesh_extras` reads, and the buffers it
    /// names.
    pub fn json_and_buffers(&self) -> (String, Vec<Vec<u8>>) {
        let mut buffers: Vec<Vec<u8>> = Vec::new();
        let mut push = |bytes: Vec<u8>| {
            buffers.push(bytes);
            buffers.len() - 1
        };
        let floats = |v: &[f32]| v.iter().flat_map(|x| x.to_ne_bytes()).collect::<Vec<u8>>();
        let ints = |v: &[i32]| v.iter().flat_map(|x| x.to_ne_bytes()).collect::<Vec<u8>>();
        let primvars: Vec<Value> = self
            .primvars
            .iter()
            .map(|p| {
                let mut o = json!({
                    "name": p.name,
                    "type": p.ty,
                    "interpolation": p.interpolation,
                    "values": push(floats(&p.values)),
                    "custom": p.custom,
                });
                if p.element_size > 1 {
                    o["elementSize"] = json!(p.element_size);
                }
                if let Some(i) = &p.indices {
                    o["indices"] = json!(push(ints(i)));
                }
                if let Some(u) = p.unauthored {
                    o["unauthored"] = json!(u);
                }
                o
            })
            .collect();
        let subsets: Vec<Value> = self
            .subsets
            .iter()
            .map(|s| {
                json!({
                    "name": s.name,
                    "elementType": s.element,
                    "family": s.family,
                    "familyType": s.family_type,
                    "indices": push(ints(&s.indices)),
                    "custom": s.custom,
                })
            })
            .collect();
        let shapes: Vec<Value> = self
            .blend_shapes
            .iter()
            .map(|b| {
                let flat: Vec<f32> = b.offsets.iter().flatten().copied().collect();
                let mut o = json!({
                    "name": b.name,
                    "offsets": push(floats(&flat)),
                    "custom": b.custom,
                });
                if let Some(i) = &b.point_indices {
                    o["pointIndices"] = json!(push(ints(i)));
                }
                o
            })
            .collect();
        let mut doc = json!({
            "primvars": primvars,
            "subsets": subsets,
            "blendShapes": shapes,
            "custom": self.custom,
        });
        if let Some(l) = &self.light {
            doc["light"] = l.clone();
        }
        (doc.to_string(), buffers)
    }
}

/// USD's prim and property name rule, as the bridge applies it (Blender's
/// make_safe_name): letters, digits and `_`, not starting with a digit.
pub fn safe_name(name: &str) -> String {
    if name.is_empty() {
        return "_".into();
    }
    let mut out = String::with_capacity(name.len() + 1);
    let mut first = true;
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        out.push('_');
        first = false;
    }
    for c in name.chars() {
        let ok = c == '_' || c.is_ascii_alphabetic() || (!first && c.is_ascii_digit());
        out.push(if ok { c } else { '_' });
        first = false;
    }
    out
}

/// Primvar names already used on one mesh prim.
struct Names(BTreeSet<String>);
impl Names {
    /// A safe name for `wanted` not used yet on this prim.
    fn take(&mut self, wanted: &str) -> String {
        let base = safe_name(wanted);
        let mut name = base.clone();
        let mut n = 2;
        while !self.0.insert(name.clone()) {
            name = format!("{base}_{n}");
            n += 1;
        }
        name
    }
}

/// One element per point or corner, or none where the map has no value.
type Elements = Vec<Option<Vec<f32>>>;

/// Primvar values, indices and the unauthored index.
type Packed = (Vec<f32>, Option<Vec<i32>>, Option<i32>);

/// `elements` as primvar values: plain when every element has a value,
/// else indexed with element 0 standing for "no value". `None` when no
/// element has one.
fn pack(elements: &Elements, dims: usize) -> Option<Packed> {
    if elements.iter().all(Option::is_none) {
        return None;
    }
    if elements.iter().all(Option::is_some) {
        return Some((
            elements.iter().flatten().flatten().copied().collect(),
            None,
            None,
        ));
    }
    let mut values = vec![0.0; dims];
    let mut indices = Vec::with_capacity(elements.len());
    for e in elements {
        match e {
            Some(v) => {
                indices.push((values.len() / dims) as i32);
                values.extend(v);
            }
            None => indices.push(0),
        }
    }
    Some((values, Some(indices), Some(0)))
}

/// Which mesh element a map is attached to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    Point,
    Corner,
}
impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Point => "point",
            Level::Corner => "corner",
        }
    }
    fn interpolation(self) -> &'static str {
        match self {
            Level::Point => "vertex",
            Level::Corner => "faceVarying",
        }
    }
}

/// The part of one mesh a Mesh prim holds.
pub struct PrimSlice<'a> {
    pub mesh: &'a Mesh,
    /// The mesh's polygons in this prim, in prim order.
    pub faces: &'a [usize],
    /// The mesh's points in this prim, in prim order.
    pub points: &'a [u32],
    /// Each mesh point's index in this prim.
    pub point_map: &'a [Option<u32>],
}

impl PrimSlice<'_> {
    /// One element per prim point, from a sparse per-point map.
    fn point_elements<T>(
        &self,
        map: &BTreeMap<u32, T>,
        value: impl Fn(&T) -> Vec<f32>,
    ) -> Elements {
        self.points.iter().map(|p| map.get(p).map(&value)).collect()
    }
    /// One element per prim corner, from a per-polygon corner list.
    fn corner_elements<T>(
        &self,
        map: &[Vec<Option<T>>],
        value: impl Fn(&T) -> Vec<f32>,
    ) -> Elements {
        self.faces
            .iter()
            .flat_map(|&f| {
                let n = self.mesh.faces[f].vertices.len();
                (0..n).map(move |c| (f, c))
            })
            .map(|(f, c)| {
                map.get(f)
                    .and_then(|cs| cs.get(c))
                    .and_then(Option::as_ref)
                    .map(&value)
            })
            .collect()
    }
}

/// What the mesh's maps, morphs and sets become on one prim. `taken` are
/// the primvar names the prim already has (its UV maps, normals).
pub fn extras(slice: &PrimSlice, taken: &[String]) -> MeshExtras {
    let mesh = slice.mesh;
    let a = &mesh.attributes;
    let mut out = MeshExtras::default();
    let mut names = Names(taken.iter().cloned().collect());
    names.0.insert("normals".into());
    // displayColor and displayOpacity belong to the first colour map.
    names.0.insert("displayColor".into());
    names.0.insert("displayOpacity".into());
    let mut display_taken = false;
    let mut add = |out: &mut MeshExtras,
                   name: &str,
                   ty: &'static str,
                   dims: usize,
                   level: Level,
                   elements: Elements,
                   mut custom: Map<String, Value>| {
        let Some((values, indices, unauthored)) = pack(&elements, dims) else {
            return;
        };
        custom.insert("name".into(), json!(name));
        custom.insert("level".into(), json!(level.label()));
        let colour = matches!(ty, "color3" | "color4");
        if colour && !display_taken {
            display_taken = true;
            // RGB as displayColor, alpha (when it isn't all 1) as
            // displayOpacity, both naming the map they came from.
            let rgb: Vec<f32> = values
                .chunks(dims)
                .flat_map(|c| c[..3].iter().copied())
                .collect();
            if dims == 4 {
                let alpha: Vec<f32> = values.chunks(4).map(|c| c[3]).collect();
                // The "no value" element is 0 everywhere, alpha included.
                let real = |i: usize| unauthored != Some(i as i32);
                if alpha.iter().enumerate().any(|(i, &x)| real(i) && x != 1.0) {
                    let mut alpha_custom = custom.clone();
                    alpha_custom.insert("channel".into(), json!("alpha"));
                    out.primvars.push(PrimvarOut {
                        name: "displayOpacity".into(),
                        ty: "float",
                        element_size: 1,
                        interpolation: level.interpolation(),
                        values: alpha,
                        indices: indices.clone(),
                        unauthored,
                        custom: alpha_custom,
                    });
                }
            }
            out.primvars.push(PrimvarOut {
                name: "displayColor".into(),
                ty: "color3",
                element_size: 1,
                interpolation: level.interpolation(),
                values: rgb,
                indices,
                unauthored,
                custom,
            });
            return;
        }
        let (ty, element_size) = if dims > 4 { ("float", dims) } else { (ty, 1) };
        out.primvars.push(PrimvarOut {
            name: names.take(name),
            ty,
            element_size,
            interpolation: level.interpolation(),
            values,
            indices,
            unauthored,
            custom,
        });
    };
    let kind = |k: &str| {
        let mut m = Map::new();
        m.insert("map".into(), json!(k));
        m
    };
    let f32s = |v: &Vec<f64>| v.iter().map(|&x| x as f32).collect::<Vec<f32>>();

    // Typed maps first: rgb3 / rgba4 are the "vertex colour" maps, so one
    // of them is displayColor when a mesh has one.
    for (name, map) in &a.typed_maps {
        let dims = map.dimension as usize;
        let ty = match (map.kind.as_str(), dims) {
            ("rgb3", 3) => "color3",
            ("rgba4", 4) => "color4",
            (_, 1) => "float",
            (_, 2) => "float2",
            (_, 3) => "float3",
            (_, 4) => "float4",
            _ => "float",
        };
        let mut custom = kind("typed");
        custom.insert("kind".into(), json!(map.kind));
        custom.insert("zeroDefault".into(), json!(map.zero_default));
        let points = slice.point_elements(&map.points, f32s);
        add(
            &mut out,
            name,
            ty,
            dims,
            Level::Point,
            points,
            custom.clone(),
        );
        let corners = slice.corner_elements(&map.corners, f32s);
        add(&mut out, name, ty, dims, Level::Corner, corners, custom);
    }
    for (name, map) in &a.point_colors {
        let elements = slice.point_elements(map, |c| c.as_vec4().to_array().to_vec());
        add(
            &mut out,
            name,
            "color4",
            4,
            Level::Point,
            elements,
            kind("color"),
        );
    }
    for (name, map) in &a.corner_colors {
        let elements = slice.corner_elements(map, |c| c.as_vec4().to_array().to_vec());
        add(
            &mut out,
            name,
            "color4",
            4,
            Level::Corner,
            elements,
            kind("color"),
        );
    }
    for (name, map) in &a.point_weights {
        let elements = slice.point_elements(map, |&w| vec![w as f32]);
        add(
            &mut out,
            name,
            "float",
            1,
            Level::Point,
            elements,
            kind("weight"),
        );
    }
    for (name, map) in &a.corner_weights {
        let elements = slice.corner_elements(map, |&w| vec![w as f32]);
        add(
            &mut out,
            name,
            "float",
            1,
            Level::Corner,
            elements,
            kind("weight"),
        );
    }

    // Morph maps: one blend shape each, over the points it moves here.
    let mut shape_names = Names(BTreeSet::new());
    for (name, offsets) in &a.relative_morphs {
        let mut moved: Vec<(i32, [f32; 3])> = offsets
            .iter()
            .filter_map(|(&p, d)| {
                let at = (*slice.point_map.get(p as usize)?)?;
                Some((at as i32, d.as_vec3().to_array()))
            })
            .collect();
        moved.sort_by_key(|(p, _)| *p);
        let every = moved.len() == slice.points.len();
        let mut custom = Map::new();
        custom.insert("name".into(), json!(name));
        out.blend_shapes.push(BlendShapeOut {
            name: shape_names.take(name),
            offsets: moved.iter().map(|(_, d)| *d).collect(),
            point_indices: (!every).then(|| moved.iter().map(|(p, _)| *p).collect()),
            custom,
        });
    }

    // Parts and selection sets. Every set goes on every prim of the mesh
    // (empty where none of its elements are), so a set with no members
    // survives too.
    let mut local_face = vec![-1i32; mesh.faces.len()];
    for (i, &f) in slice.faces.iter().enumerate() {
        local_face[f] = i as i32;
    }
    let mut parts: BTreeMap<&str, Vec<i32>> = BTreeMap::new();
    for (i, &f) in slice.faces.iter().enumerate() {
        if let Some(part) = mesh.faces[f].part.as_deref() {
            parts.entry(part).or_default().push(i as i32);
        }
    }
    let named = |n: &str| {
        let mut m = Map::new();
        m.insert("name".into(), json!(n));
        m
    };
    for (part, faces) in parts {
        out.subsets.push(GroupOut {
            name: format!("part_{part}"),
            element: "face",
            family: FAMILY_PART,
            family_type: "nonOverlapping",
            indices: faces,
            custom: named(part),
        });
    }
    for (name, set) in &a.polygon_sets {
        let mut faces: Vec<i32> = set
            .iter()
            .filter_map(|&f| local_face.get(f as usize).copied().filter(|&l| l >= 0))
            .collect();
        faces.sort_unstable();
        out.subsets.push(GroupOut {
            name: format!("polygonSet_{name}"),
            element: "face",
            family: FAMILY_POLYGON_SET,
            family_type: "unrestricted",
            indices: faces,
            custom: named(name),
        });
    }
    for (name, set) in &a.vertex_sets {
        let mut points: Vec<i32> = set
            .iter()
            .filter_map(|&p| (*slice.point_map.get(p as usize)?).map(|l| l as i32))
            .collect();
        points.sort_unstable();
        out.subsets.push(GroupOut {
            name: format!("pointSet_{name}"),
            element: "point",
            family: FAMILY_POINT_SET,
            family_type: "unrestricted",
            indices: points,
            custom: named(name),
        });
    }
    if !a.edge_sets.is_empty() {
        let mut sets = Map::new();
        for (name, edges) in &a.edge_sets {
            let pairs: Vec<i32> = edges
                .iter()
                .filter_map(|&(p, q)| {
                    let a = (*slice.point_map.get(p as usize)?)?;
                    let b = (*slice.point_map.get(q as usize)?)?;
                    Some([a as i32, b as i32])
                })
                .flatten()
                .collect();
            sets.insert(name.clone(), json!(pairs));
        }
        out.custom.insert("edgeSets".into(), Value::Object(sets));
    }
    out
}
