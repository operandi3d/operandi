//! The import half of plan_extras.rs: the weight, typed and colour maps,
//! morphs, parts and selection sets Operandi writes (USD-IQ22) come back
//! as the same mesh data. Each is recognised by its customData
//! ("operandi").
//!
//! Other programs' extras come in too (USD-IQ23, provisional):
//! - primvars other than UVs, colours, normals and skinning weights
//!   ("foreign", see [`foreign`]): a float per point becomes a weight
//!   map, per corner (faceVarying, or uniform spread over each polygon's
//!   corners) a corner weight map; ints and vectors become typed maps;
//!   constant ones are left out and noted. This is how Blender's vertex
//!   groups and attributes and Houdini's point and primitive attributes
//!   arrive.
//! - GeomSubsets of any family but materialBind and Operandi's own:
//!   face subsets become polygon selection sets, point subsets point
//!   sets and edge subsets edge sets, named after the subset prim
//!   (Houdini's groups).
//! - every UsdSkelBlendShape bound to the mesh becomes a morph map
//!   (Blender's shape keys), as Operandi's own do; inbetweens are left
//!   out.
use crate::stage::{MeshData, Primvar, StageData};
use glam::{DMat3, DVec3, DVec4};
use operandi_core::{Face, Mesh, Result, TypedMap};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Typed maps a mesh may hold (MeshAttributes::validate).
const TYPED_BUDGET: usize = 64;

/// One Mesh prim being read into the import's mesh.
pub struct Context<'a> {
    pub stage: &'a StageData,
    pub data: &'a MeshData,
    /// The mesh's point index of the prim's first point.
    pub base: u32,
    pub point_count: usize,
    /// The polygons kept, in order: their file polygon and the file corner
    /// each kept corner came from. The first is the mesh's `face_start`.
    pub faces: &'a [(usize, Vec<usize>)],
    pub face_start: usize,
    /// The file-to-scene turn and the metres per file unit.
    pub turn: DMat3,
    pub scale: f64,
}

/// A map's values per corner, for the mesh polygons it covers.
type CornerLists<T> = Vec<(usize, Vec<Option<T>>)>;

/// A typed map read but not stored yet: name, level, (kind, dimension,
/// zero default) and values.
type PendingTyped = (String, String, (String, usize, bool), Values);

/// What the Mesh prims of one item carried, gathered until the mesh is
/// finished.
#[derive(Default)]
pub struct Extras {
    point_weights: BTreeMap<String, BTreeMap<u32, f64>>,
    corner_weights: BTreeMap<String, CornerLists<f64>>,
    /// Kind, dimension and zero default of each typed map.
    typed: BTreeMap<String, (String, u8, bool)>,
    typed_points: BTreeMap<String, BTreeMap<u32, Vec<f64>>>,
    typed_corners: BTreeMap<String, CornerLists<Vec<f64>>>,
    point_colors: BTreeMap<String, BTreeMap<u32, DVec4>>,
    corner_colors: BTreeMap<String, CornerLists<DVec4>>,
    morphs: BTreeMap<String, BTreeMap<u32, DVec3>>,
    parts: Vec<(usize, String)>,
    polygon_sets: BTreeMap<String, BTreeSet<u32>>,
    vertex_sets: BTreeMap<String, BTreeSet<u32>>,
    edge_sets: BTreeMap<String, BTreeSet<(u32, u32)>>,
}

/// A map's values at one level: per prim point, or per kept polygon's
/// corners.
enum Values {
    Points(Vec<Option<Vec<f64>>>),
    Corners(Vec<Vec<Option<Vec<f64>>>>),
}

fn text<'v>(custom: &'v Value, key: &str) -> Option<&'v str> {
    custom.get(key).and_then(Value::as_str)
}

/// The custom name, else the USD one.
fn name_of(custom: Option<&Value>, usd: &str) -> String {
    custom
        .and_then(|c| text(c, "name"))
        .unwrap_or(usd)
        .to_string()
}

/// Typed map kinds holding directions (turned to the scene's up axis) and
/// positions or offsets (also scaled to metres).
fn turned(kind: &str) -> (bool, bool) {
    match kind {
        "normal3" | "vector3" => (true, false),
        "relative_morph3" | "absolute_position3" | "pose_shape3" => (true, true),
        _ => (false, false),
    }
}

impl Context<'_> {
    /// `pv`'s elements, `None` where it marks "no value".
    fn values(&self, pv: &Primvar, level: &str) -> Result<Option<Values>> {
        let s = self.stage;
        let values = s.f32s(&pv.values)?;
        let lookup = match &pv.indices {
            Some(i) => Some(s.i32s(i)?),
            None => None,
        };
        let width = pv.dims.max(1);
        let element = |k: usize| -> Option<Vec<f64>> {
            let k = match &lookup {
                Some(l) => {
                    let i = *l.get(k)?;
                    if pv.unauthored == Some(i as i64) {
                        return None;
                    }
                    usize::try_from(i).ok()?
                }
                None => k,
            };
            let v = values.get(k * width..k * width + width)?;
            let out: Vec<f64> = v.iter().map(|&x| x as f64).collect();
            out.iter().all(|x| x.is_finite()).then_some(out)
        };
        Ok(match (level, pv.interpolation.as_str()) {
            ("point", "vertex" | "varying") => {
                Some(Values::Points((0..self.point_count).map(element).collect()))
            }
            ("corner", "faceVarying") => Some(Values::Corners(
                self.faces
                    .iter()
                    .map(|(_, kept)| kept.iter().map(|&c| element(c)).collect())
                    .collect(),
            )),
            // One value per polygon, on each of its corners.
            ("corner", "uniform") => Some(Values::Corners(
                self.faces
                    .iter()
                    .map(|(f, kept)| vec![element(*f); kept.len()])
                    .collect(),
            )),
            _ => None,
        })
    }

    /// The mesh polygon each file polygon became, if it was kept.
    fn kept_faces(&self) -> BTreeMap<usize, usize> {
        self.faces
            .iter()
            .enumerate()
            .map(|(k, (f, _))| (*f, self.face_start + k))
            .collect()
    }
}

/// Reads what Operandi and other programs wrote on one Mesh prim into
/// `out`; what's left out goes in `notes`, one line each.
pub fn read(cx: &Context, out: &mut Extras, notes: &mut BTreeSet<String>) -> Result<()> {
    let base = cx.base;
    // Alpha written beside a colour map (displayOpacity), by name and level.
    let mut alphas: BTreeMap<(String, String), Values> = BTreeMap::new();
    let mut colours: Vec<(String, String, usize, Values)> = Vec::new();
    let mut typed: Vec<PendingTyped> = Vec::new();
    for pv in cx.data.primvars.iter().filter(|p| p.kind == "operandi") {
        let Some(custom) = pv.custom.as_ref() else {
            continue;
        };
        let name = name_of(Some(custom), &pv.name);
        let level = text(custom, "level").unwrap_or("point").to_string();
        let Some(values) = cx.values(pv, &level)? else {
            continue;
        };
        match text(custom, "map") {
            Some("weight") => match values {
                Values::Points(v) => {
                    let map = out.point_weights.entry(name).or_default();
                    for (p, w) in v.into_iter().enumerate() {
                        if let Some(w) = w {
                            map.insert(base + p as u32, w[0]);
                        }
                    }
                }
                Values::Corners(v) => {
                    let lists = out.corner_weights.entry(name).or_default();
                    for (k, corners) in v.into_iter().enumerate() {
                        lists.push((
                            cx.face_start + k,
                            corners.into_iter().map(|c| c.map(|w| w[0])).collect(),
                        ));
                    }
                }
            },
            Some("color") | Some("typed") if text(custom, "channel") == Some("alpha") => {
                alphas.insert((name, level), values);
            }
            Some("color") => colours.push((name, level, pv.dims, values)),
            Some("typed") => {
                let kind = text(custom, "kind").unwrap_or_default().to_string();
                let zero = custom.get("zeroDefault").and_then(Value::as_bool);
                // displayColor holds an rgba4 map's RGB; its alpha is
                // beside it.
                let dims = match kind.as_str() {
                    "rgba4" => 4,
                    _ => pv.dims,
                };
                typed.push((name, level, (kind, dims, zero.unwrap_or(false)), values));
            }
            _ => {}
        }
    }
    for pv in cx.data.primvars.iter().filter(|p| p.kind == "foreign") {
        foreign(cx, out, pv, notes)?;
    }
    // Colours last: their alpha (displayOpacity) may come after them.
    for (name, level, meta, values) in typed {
        typed_map(out, cx, &mut alphas, name, level, meta, values);
    }
    for (name, level, dims, values) in colours {
        let alpha = alphas.remove(&(name.clone(), level.clone()));
        let rgba = |v: &[f64], a: Option<f64>| {
            DVec4::new(
                v[0],
                v[1],
                v[2],
                if dims >= 4 { v[3] } else { a.unwrap_or(1.0) },
            )
        };
        match values {
            Values::Points(v) => {
                let a = match &alpha {
                    Some(Values::Points(a)) => a.clone(),
                    _ => vec![None; v.len()],
                };
                let map = out.point_colors.entry(name).or_default();
                for (p, c) in v.into_iter().enumerate() {
                    if let Some(c) = c {
                        let a = a.get(p).cloned().flatten().map(|x| x[0]);
                        map.insert(base + p as u32, rgba(&c, a));
                    }
                }
            }
            Values::Corners(v) => {
                let a = match &alpha {
                    Some(Values::Corners(a)) => a.clone(),
                    _ => Vec::new(),
                };
                let lists = out.corner_colors.entry(name).or_default();
                for (k, corners) in v.into_iter().enumerate() {
                    let list = corners
                        .into_iter()
                        .enumerate()
                        .map(|(c, value)| {
                            let a = a
                                .get(k)
                                .and_then(|f| f.get(c))
                                .cloned()
                                .flatten()
                                .map(|x| x[0]);
                            value.map(|v| rgba(&v, a))
                        })
                        .collect();
                    lists.push((cx.face_start + k, list));
                }
            }
        }
    }

    // Parts and selection sets.
    let kept = cx.kept_faces();
    let s = cx.stage;
    for g in &cx.data.groups {
        let name = name_of(g.custom.as_ref(), &g.name);
        let members = s.i32s(&g.indices)?;
        match g.family.as_str() {
            crate::plan_extras::FAMILY_PART => {
                for f in members {
                    if let Some(&m) = usize::try_from(f).ok().and_then(|f| kept.get(&f)) {
                        out.parts.push((m, name.clone()));
                    }
                }
            }
            crate::plan_extras::FAMILY_POLYGON_SET => {
                let set = out.polygon_sets.entry(name).or_default();
                for f in members {
                    if let Some(&m) = usize::try_from(f).ok().and_then(|f| kept.get(&f)) {
                        set.insert(m as u32);
                    }
                }
            }
            crate::plan_extras::FAMILY_POINT_SET => {
                let set = out.vertex_sets.entry(name).or_default();
                for p in members {
                    if let Ok(p) = usize::try_from(p)
                        && p < cx.point_count
                    {
                        set.insert(base + p as u32);
                    }
                }
            }
            // A family of Operandi's this build doesn't know.
            f if f.starts_with("operandi:") => {}
            // Another program's groups (USD-IQ23).
            _ => match g.element_type.as_str() {
                "face" => {
                    let set = out.polygon_sets.entry(name).or_default();
                    for f in members {
                        if let Some(&m) = usize::try_from(f).ok().and_then(|f| kept.get(&f)) {
                            set.insert(m as u32);
                        }
                    }
                }
                "point" => {
                    let set = out.vertex_sets.entry(name).or_default();
                    for p in members {
                        if let Ok(p) = usize::try_from(p)
                            && p < cx.point_count
                        {
                            set.insert(base + p as u32);
                        }
                    }
                }
                "edge" => {
                    let set = out.edge_sets.entry(name).or_default();
                    for pair in members.as_chunks::<2>().0 {
                        let (Ok(a), Ok(b)) = (usize::try_from(pair[0]), usize::try_from(pair[1]))
                        else {
                            continue;
                        };
                        if a < cx.point_count && b < cx.point_count && a != b {
                            let (a, b) = (base + a as u32, base + b as u32);
                            set.insert((a.min(b), a.max(b)));
                        }
                    }
                }
                other => {
                    notes.insert(format!(
                        "GeomSubset {name}: {other} subsets left out (Operandi has face, point and edge sets)"
                    ));
                }
            },
        }
    }
    if let Some(sets) = cx
        .data
        .custom
        .as_ref()
        .and_then(|c| c.get("edgeSets"))
        .and_then(Value::as_object)
    {
        for (name, pairs) in sets {
            let set = out.edge_sets.entry(name.clone()).or_default();
            let numbers: Vec<i64> = pairs
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            for pair in numbers.as_chunks::<2>().0 {
                let (Ok(a), Ok(b)) = (usize::try_from(pair[0]), usize::try_from(pair[1])) else {
                    continue;
                };
                if a < cx.point_count && b < cx.point_count && a != b {
                    let (a, b) = (base + a as u32, base + b as u32);
                    set.insert((a.min(b), a.max(b)));
                }
            }
        }
    }

    // Blend shapes become morph maps.
    for shape in &cx.data.blend_shapes {
        let name = name_of(shape.custom.as_ref(), &shape.name);
        let offsets = s.f32s(&shape.offsets)?;
        let points = match &shape.point_indices {
            Some(i) => Some(s.i32s(i)?),
            None => None,
        };
        let map = out.morphs.entry(name).or_default();
        for (i, d) in offsets.as_chunks::<3>().0.iter().enumerate() {
            let p = match &points {
                Some(p) => match p.get(i).map(|&p| usize::try_from(p)) {
                    Some(Ok(p)) => p,
                    _ => continue,
                },
                None => i,
            };
            let d = DVec3::new(d[0] as f64, d[1] as f64, d[2] as f64);
            if p < cx.point_count && d.is_finite() {
                map.insert(base + p as u32, cx.turn * d * cx.scale);
            }
        }
    }
    Ok(())
}

/// The typed map kind another program's primvar becomes, by its width,
/// role and type (USD-IQ23, provisional). Integers keep their values
/// unturned; vectors with no role are read as directions.
fn foreign_kind(pv: &Primvar) -> Option<&'static str> {
    let int = pv.ty.starts_with("int");
    Some(match (pv.dims, pv.role.as_str()) {
        (1, _) => "weight1",
        (2, _) => "uv2",
        (3, _) if int => "rgb3",
        (3, "Normal") => "normal3",
        (3, "Point") => "absolute_position3",
        (3, _) => "vector3",
        (4, _) => "rgba4",
        _ => return None,
    })
}

/// Another program's primvar (USD-IQ23): a float per point or corner
/// becomes a weight map, anything else a typed map of the nearest kind.
fn foreign(
    cx: &Context,
    out: &mut Extras,
    pv: &Primvar,
    notes: &mut BTreeSet<String>,
) -> Result<()> {
    let level = match pv.interpolation.as_str() {
        "vertex" | "varying" => "point",
        "faceVarying" | "uniform" => "corner",
        other => {
            notes.insert(format!(
                "Primvar {} left out: {other} values (one for the whole mesh)",
                pv.name
            ));
            return Ok(());
        }
    };
    let Some(values) = cx.values(pv, level)? else {
        return Ok(());
    };
    let float = !pv.ty.starts_with("int");
    if pv.dims == 1 && float {
        match values {
            Values::Points(v) => {
                let map = out.point_weights.entry(pv.name.clone()).or_default();
                for (p, w) in v.into_iter().enumerate() {
                    if let Some(w) = w {
                        map.insert(cx.base + p as u32, w[0]);
                    }
                }
            }
            Values::Corners(v) => {
                let lists = out.corner_weights.entry(pv.name.clone()).or_default();
                for (k, corners) in v.into_iter().enumerate() {
                    lists.push((
                        cx.face_start + k,
                        corners.into_iter().map(|c| c.map(|w| w[0])).collect(),
                    ));
                }
            }
        }
        return Ok(());
    }
    let Some(kind) = foreign_kind(pv) else {
        notes.insert(format!(
            "Primvar {} left out: {} values per element",
            pv.name, pv.dims
        ));
        return Ok(());
    };
    if !out.typed.contains_key(&pv.name) && out.typed.len() >= TYPED_BUDGET {
        notes.insert(format!(
            "Primvar {} left out: a mesh holds at most {TYPED_BUDGET} typed maps",
            pv.name
        ));
        return Ok(());
    }
    let mut none = BTreeMap::new();
    typed_map(
        out,
        cx,
        &mut none,
        pv.name.clone(),
        level.to_string(),
        (kind.to_string(), pv.dims, false),
        values,
    );
    Ok(())
}

/// A typed map's values; an rgba4 map written as displayColor picks its
/// alpha up from `alphas`.
fn typed_map(
    out: &mut Extras,
    cx: &Context,
    alphas: &mut BTreeMap<(String, String), Values>,
    name: String,
    level: String,
    (kind, dims, zero): (String, usize, bool),
    values: Values,
) {
    let (turn, scale) = turned(&kind);
    let alpha = alphas.remove(&(name.clone(), level.clone()));
    let fix = |mut v: Vec<f64>, a: Option<f64>| -> Vec<f64> {
        if v.len() < dims {
            v.resize(dims, a.unwrap_or(1.0));
        }
        if turn && v.len() == 3 {
            let mut d = cx.turn * DVec3::new(v[0], v[1], v[2]);
            if scale {
                d *= cx.scale;
            }
            v = d.to_array().to_vec();
        }
        v
    };
    out.typed
        .insert(name.clone(), (kind.clone(), dims as u8, zero));
    match values {
        Values::Points(v) => {
            let a = match &alpha {
                Some(Values::Points(a)) => a.clone(),
                _ => Vec::new(),
            };
            let map = out.typed_points.entry(name).or_default();
            for (p, value) in v.into_iter().enumerate() {
                if let Some(value) = value {
                    let a = a.get(p).cloned().flatten().map(|x| x[0]);
                    map.insert(cx.base + p as u32, fix(value, a));
                }
            }
        }
        Values::Corners(v) => {
            let a = match &alpha {
                Some(Values::Corners(a)) => a.clone(),
                _ => Vec::new(),
            };
            let lists = out.typed_corners.entry(name).or_default();
            for (k, corners) in v.into_iter().enumerate() {
                let list = corners
                    .into_iter()
                    .enumerate()
                    .map(|(c, value)| {
                        let a = a
                            .get(k)
                            .and_then(|f| f.get(c))
                            .cloned()
                            .flatten()
                            .map(|x| x[0]);
                        value.map(|v| fix(v, a))
                    })
                    .collect();
                lists.push((cx.face_start + k, list));
            }
        }
    }
}

/// Every polygon's corner list, `None` where `lists` has nothing.
fn full<T: Clone>(lists: CornerLists<T>, sizes: &[usize]) -> Vec<Vec<Option<T>>> {
    let mut out: Vec<Vec<Option<T>>> = sizes.iter().map(|&n| vec![None; n]).collect();
    for (f, corners) in lists {
        if let Some(slot) = out.get_mut(f)
            && slot.len() == corners.len()
        {
            *slot = corners;
        }
    }
    out
}

impl Extras {
    /// Adds a per-point typed map (curve widths, curves.rs).
    pub fn add_point_typed(&mut self, name: &str, kind: &str, values: BTreeMap<u32, Vec<f64>>) {
        let dims = values.values().next().map_or(1, Vec::len) as u8;
        self.typed
            .insert(name.to_string(), (kind.to_string(), dims, false));
        self.typed_points
            .entry(name.to_string())
            .or_default()
            .extend(values);
    }

    /// Sets each polygon's part.
    pub fn apply_parts(&mut self, faces: &mut [Face]) {
        for (f, name) in self.parts.drain(..) {
            if let Some(face) = faces.get_mut(f) {
                face.part = Some(name);
            }
        }
    }

    /// Puts the maps, morphs and sets in `mesh`'s attributes.
    pub fn finish(self, mesh: &mut Mesh) {
        let sizes: Vec<usize> = mesh.faces.iter().map(|f| f.vertices.len()).collect();
        let a = &mut *mesh.attributes;
        a.point_weights.extend(self.point_weights);
        for (name, lists) in self.corner_weights {
            a.corner_weights.insert(name, full(lists, &sizes));
        }
        let mut corners = self.typed_corners;
        let mut points = self.typed_points;
        for (name, (kind, dimension, zero_default)) in self.typed {
            let corners = corners
                .remove(&name)
                .map(|l| full(l, &sizes))
                .unwrap_or_default();
            a.typed_maps.insert(
                name.clone(),
                TypedMap {
                    kind,
                    dimension,
                    zero_default,
                    points: points.remove(&name).unwrap_or_default(),
                    corners,
                },
            );
        }
        a.point_colors.extend(self.point_colors);
        for (name, lists) in self.corner_colors {
            a.corner_colors.insert(name, full(lists, &sizes));
        }
        a.relative_morphs.extend(self.morphs);
        a.polygon_sets.extend(self.polygon_sets);
        a.vertex_sets.extend(self.vertex_sets);
        a.edge_sets.extend(self.edge_sets);
    }
}
