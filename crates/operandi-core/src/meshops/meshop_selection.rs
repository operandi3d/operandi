//! Selection operations (MO-50..54, PG-xx): an op's Selection input,
//! re-answered on every evaluation. A stack of them sits on each op after
//! its Select by Index (MO-20): each picks polygons of the mesh below and
//! combines with what's under it by its Blend Mode (MO-51).
//!
//! Select by Previous Operation (PG-xx) reads named groups an op lower in
//! the stack produced, such as a primitive's `caps` or an Array's `clone2`.
//! A producing op writes each group as a reserved polygon set on its
//! result ([`OpContext::tag`](crate::meshop_types::OpContext::tag)), so the
//! group follows the polygons through every op above that carries
//! selection sets along. The runtime strips reserved sets from what it
//! hands out (viewport, render, Freeze, links).
//!
//! Each op says what its Selection input picks ([`SelElement`], GitHub
//! #19): polygons, edges (numbered as `Mesh::edges`) or points. Index,
//! Range, Pattern and Random count in that kind; groups, sets and the rule
//! ops pick in their own kind and convert (an edge is picked when one of
//! its polygons is, a polygon when one of its edges is, a point when it
//! is on a picked polygon or edge).
//!
//! Provisional (MOQ-21..24, SEL-Q01..Q06): Range and Pattern channel
//! names, Select by Selection Set's type id and the rule ops' ids are
//! Operandi's own; the rules follow Steve Hill's own descriptions of his
//! Selma kit (GitHub #19), measured in the mesh's own space.
use crate::meshop_types::SelElement;
use crate::{Mesh, Result};
use serde::{Deserialize, Serialize};

/// Reserved polygon set names start with this (not typeable).
const GROUP_PREFIX: &str = "\u{1}meshop:";

/// The reserved set that holds op `op`'s group `group` (PG-03 short id).
pub fn group_set(op: u64, group: &str) -> String {
    format!("{GROUP_PREFIX}{op}:{group}")
}

/// `mesh` without reserved group sets; borrowed when it has none.
pub fn strip_groups(mesh: &crate::Shared<Mesh>) -> crate::Shared<Mesh> {
    if !mesh
        .attributes
        .polygon_sets
        .keys()
        .any(|k| k.starts_with(GROUP_PREFIX))
    {
        return mesh.clone();
    }
    let mut m = mesh.to_owned_value();
    m.attributes
        .polygon_sets
        .retain(|k, _| !k.starts_with(GROUP_PREFIX));
    let out = crate::Shared::new(m);
    out.record_checked();
    out
}

/// `mesh` without its group sets, and those sets. Kernels never see the
/// sets (some refuse any selection set); [`carry_groups`] puts them back.
pub fn split_groups(mesh: &Mesh) -> (std::borrow::Cow<'_, Mesh>, Vec<(String, Vec<u32>)>) {
    if !mesh
        .attributes
        .polygon_sets
        .keys()
        .any(|k| k.starts_with(GROUP_PREFIX))
    {
        return (std::borrow::Cow::Borrowed(mesh), Vec::new());
    }
    let mut clean = mesh.clone();
    let mut groups = Vec::new();
    clean.attributes.polygon_sets.retain(|k, v| {
        if k.starts_with(GROUP_PREFIX) {
            groups.push((k.clone(), v.iter().copied().collect()));
            false
        } else {
            true
        }
    });
    (std::borrow::Cow::Owned(clean), groups)
}

/// Puts `groups` of `input` onto `output`: a polygon of the output that
/// has exactly the corners of a grouped input polygon (same point numbers
/// in the same order) is in its groups. Kernels keep untouched polygons
/// that way, so a group follows the polygons an op leaves alone; a
/// polygon an op rebuilt leaves its groups (provisional, MOQ-22).
pub fn carry_groups(groups: &[(String, Vec<u32>)], input: &Mesh, output: &mut Mesh) {
    if groups.is_empty() {
        return;
    }
    let mut at: std::collections::HashMap<&[u32], u32> = std::collections::HashMap::new();
    for (f, face) in input.faces.iter().enumerate() {
        at.entry(face.vertices.as_slice()).or_insert(f as u32);
    }
    let mut map: Vec<Option<u32>> = vec![None; input.faces.len()];
    for (f, face) in output.faces.iter().enumerate() {
        if let Some(&old) = at.get(face.vertices.as_slice())
            && map[old as usize].is_none()
        {
            map[old as usize] = Some(f as u32);
        }
    }
    for (name, faces) in groups {
        if output.attributes.polygon_sets.contains_key(name) {
            continue;
        }
        let set: std::collections::BTreeSet<u32> = faces
            .iter()
            .filter_map(|&f| map.get(f as usize).copied().flatten())
            .collect();
        output.attributes.polygon_sets.insert(name.clone(), set);
    }
}

/// MO-51 Blend Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Blend {
    /// Replaces what's below it.
    #[default]
    Override,
    Add,
    Subtract,
    /// Keeps only what both select.
    Multiply,
}
impl Blend {
    pub const ALL: [Blend; 4] = [Self::Override, Self::Add, Self::Subtract, Self::Multiply];
    pub fn label(self) -> &'static str {
        match self {
            Self::Override => "Override",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Multiply => "Multiply",
        }
    }
}

/// What a selection op picks (MO-53).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SelKind {
    /// Stored polygon numbers, `0-9,12` form.
    #[serde(rename = "index.selop.item")]
    Index { index: String },
    /// Every `step`th polygon from `start` to `end` (inclusive; 0 = the
    /// last polygon).
    #[serde(rename = "selbyrange.selop.item")]
    Range { start: u32, end: u32, step: u32 },
    /// A repeating on/off list: `1,0` = every other polygon. Accepts
    /// 1/0, True/False, Yes/No.
    #[serde(rename = "pattern.selop.item")]
    Pattern { pattern: String },
    /// A group an op lower in the stack produced (PG-01..04): `source` is
    /// that op's id, `name` its short group id (case-sensitive).
    #[serde(rename = "previous.selop")]
    Previous { source: u64, name: String },
    /// The polygons of a selection set on the mesh below.
    #[serde(rename = "operandi.selset.selop")]
    SelectionSet { name: String },
    /// Random selection
    #[serde(rename = "random.selop")]
    Random { seed: u32, probability: f32 },
    // ---- Rule ops (#19; Steve Hill's Selma descriptions, 2026-11-25).
    // Worked out again on every evaluation, so they follow topology
    // changes below. Where a Maximum is below its Minimum, only the
    // Minimum applies. Angles in degrees, distances in metres, in the
    // mesh's own space.
    /// Edges whose two polygons meet at between `min` and `max` degrees
    /// (0 = flat); `convex` and `concave` say which bends count.
    #[serde(rename = "operandi.edgeAngle.selop")]
    EdgeAngle {
        min: f64,
        max: f64,
        #[serde(default = "yes")]
        convex: bool,
        #[serde(default = "yes")]
        concave: bool,
    },
    /// Edges on an open border (one polygon).
    #[serde(rename = "operandi.border.selop")]
    Border {},
    /// Polygons with `min` to `max` corners (Triangles 3-3, Quads 4-4,
    /// N-gons 5 with no maximum).
    #[serde(rename = "operandi.vertexCount.selop")]
    VertexCount { min: u32, max: u32 },
    /// Polygons whose area is between `min`² and `max`² (the sides of
    /// squares of that area, so the values have sensible units).
    #[serde(rename = "operandi.area.selop")]
    Area { min: f64, max: f64 },
    /// Edges `min` to `max` metres long.
    #[serde(rename = "operandi.edgeLength.selop")]
    EdgeLength { min: f64, max: f64 },
    /// Elements whose normal is `min` to `max` degrees from the direction
    /// `x y z` (an edge's normal is its points' average, a point's its
    /// polygons' average).
    #[serde(rename = "operandi.normal.selop")]
    Normal {
        x: f64,
        y: f64,
        z: f64,
        min: f64,
        max: f64,
    },
    /// Elements on one side of the plane through `x y z` facing `nx ny nz`
    /// (`negative` = the other side), measured at each element's centre.
    #[serde(rename = "operandi.plane.selop")]
    Plane {
        x: f64,
        y: f64,
        z: f64,
        nx: f64,
        ny: f64,
        nz: f64,
        negative: bool,
        #[serde(rename = "includeZero")]
        include_zero: bool,
    },
    /// Elements whose centre is `min` to `max` metres from `x y z`.
    #[serde(rename = "operandi.distance.selop")]
    Distance {
        x: f64,
        y: f64,
        z: f64,
        min: f64,
        max: f64,
    },
    /// `number` elements at random, or `percent` of them when `number` is 0.
    #[serde(rename = "operandi.randomCount.selop")]
    RandomCount {
        seed: u32,
        number: u32,
        percent: f64,
    },
    /// Every element of randomly chosen parts (connected polygon islands),
    /// each part with `chance` percent.
    #[serde(rename = "operandi.randomParts.selop")]
    RandomParts { seed: u32, chance: f64 },
    /// Points with `min` to `max` edges.
    #[serde(rename = "operandi.vertexEdges.selop")]
    VertexEdges { min: u32, max: u32 },
    /// The mirror image of the selection below it, across the checked
    /// axes together (X and Y: (1,1,1) picks (-1,-1,1)), matched within
    /// `tolerance` metres. Override by default; Add keeps both.
    #[serde(rename = "operandi.symmetric.selop")]
    Symmetric {
        x: bool,
        y: bool,
        z: bool,
        tolerance: f64,
    },
    /// Edges on a seam of the UV map `map`: two polygons share the edge
    /// but not its UVs.
    #[serde(rename = "operandi.uvSeam.selop")]
    UvSeam { map: String },
    // ---- Batch 2 (meshops-gaps-2026-11-25; ids Operandi's own, MOQ-39).
    /// Every element of the parts (polygons joined through shared points)
    /// the selection below touches.
    #[serde(rename = "operandi.connected.selop")]
    Connected {},
    /// Polygons whose tag `tag` (0 Material, 1 Part) is `name`.
    #[serde(rename = "operandi.polygonTag.selop")]
    PolygonTag { tag: u32, name: String },
    /// Polygons reached from the selection below across edges while
    /// facing within `angle` degrees of the polygon they grew from.
    #[serde(rename = "operandi.coplanar.selop")]
    Coplanar { angle: f64 },
    /// Edges marked hard (the vertex normal tools' Hard Edges).
    #[serde(rename = "operandi.hardEdges.selop")]
    HardEdges {},
    /// Points whose value in weight map `map` is `min` to `max` (a point
    /// with no value counts as 0).
    #[serde(rename = "operandi.vmapValue.selop")]
    VertexMapValue { map: String, min: f64, max: f64 },
    // ---- Tidy batch (MESHOPS-TIDY-SPEC 2026-11-27, MOQ-69).
    /// Elements whose weight in the op's own falloff (a copy taken when
    /// added; Use Active Falloff / Clear Falloff change it) is at least
    /// `threshold` (a fraction), measured at each element's centre in the
    /// mesh's own space (TB-01..05). No falloff picks nothing.
    #[serde(rename = "pmodel.falloffSelOp")]
    Falloff {
        threshold: f64,
        #[serde(default)]
        falloff: Option<crate::ShapeWeights>,
    },
    /// The edge loops grown from the selection so far (TB-10..20).
    #[serde(rename = "selop.loopConditional.item")]
    ConditionalLoop(crate::meshop_conditional_loop::LoopRules),
}
fn yes() -> bool {
    true
}
impl SelKind {
    /// The kinds Add Selection offers, with their labels.
    pub fn menu() -> [(&'static str, SelKind); 29] {
        [
            (
                "Select by Index",
                SelKind::Index {
                    index: String::new(),
                },
            ),
            (
                "Select by Range",
                SelKind::Range {
                    start: 0,
                    end: 0,
                    step: 1,
                },
            ),
            (
                "Select by Pattern",
                SelKind::Pattern {
                    pattern: "1,0".into(),
                },
            ),
            (
                "Select by Previous Operation",
                SelKind::Previous {
                    source: 0,
                    name: String::new(),
                },
            ),
            (
                "Select by Selection Set",
                SelKind::SelectionSet {
                    name: String::new(),
                },
            ),
            (
                "Select Randomly",
                SelKind::Random {
                    seed: 314159265,
                    probability: 50.0,
                },
            ),
            (
                "Select N Randomly",
                SelKind::RandomCount {
                    seed: 314159265,
                    number: 10,
                    percent: 50.,
                },
            ),
            (
                "Select Random Parts",
                SelKind::RandomParts {
                    seed: 314159265,
                    chance: 50.,
                },
            ),
            ("Select Triangles", SelKind::VertexCount { min: 3, max: 3 }),
            ("Select Quads", SelKind::VertexCount { min: 4, max: 4 }),
            ("Select N-gons", SelKind::VertexCount { min: 5, max: 0 }),
            (
                "Select Polygons by Vertex Count",
                SelKind::VertexCount { min: 3, max: 4 },
            ),
            (
                "Select Polygons by Area",
                SelKind::Area { min: 0., max: 1. },
            ),
            (
                "Select Edges by Angle",
                SelKind::EdgeAngle {
                    min: 30.,
                    max: 180.,
                    convex: true,
                    concave: true,
                },
            ),
            (
                "Select Edges by Length",
                SelKind::EdgeLength { min: 0., max: 1. },
            ),
            ("Select Border", SelKind::Border {}),
            (
                "Select Edges by UV Seam",
                SelKind::UvSeam {
                    map: "Texture".into(),
                },
            ),
            (
                "Select Vertices by Edge Count",
                SelKind::VertexEdges { min: 5, max: 0 },
            ),
            (
                "Select by Normal",
                SelKind::Normal {
                    x: 0.,
                    y: 1.,
                    z: 0.,
                    min: 0.,
                    max: 45.,
                },
            ),
            (
                "Select by Plane",
                SelKind::Plane {
                    x: 0.,
                    y: 0.,
                    z: 0.,
                    nx: 0.,
                    ny: 1.,
                    nz: 0.,
                    negative: false,
                    include_zero: true,
                },
            ),
            (
                "Select by Distance",
                SelKind::Distance {
                    x: 0.,
                    y: 0.,
                    z: 0.,
                    min: 0.,
                    max: 1.,
                },
            ),
            (
                "Select Symmetric",
                SelKind::Symmetric {
                    x: true,
                    y: false,
                    z: false,
                    tolerance: 0.0001,
                },
            ),
            ("Select Connected", SelKind::Connected {}),
            (
                "Select by Polygon Tag",
                SelKind::PolygonTag {
                    tag: 0,
                    name: "Default".into(),
                },
            ),
            ("Select Coplanar Polygons", SelKind::Coplanar { angle: 1. }),
            ("Select by Hard Edges", SelKind::HardEdges {}),
            (
                "Select Vertex Map Value",
                SelKind::VertexMapValue {
                    map: "Weight".into(),
                    min: 0.5,
                    max: 1.,
                },
            ),
            (
                "Select by Falloff",
                SelKind::Falloff {
                    threshold: 0.5,
                    falloff: None,
                },
            ),
            (
                "Select Conditional Loop",
                SelKind::ConditionalLoop(Default::default()),
            ),
        ]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Index { .. } => "Select by Index",
            Self::Range { .. } => "Select by Range",
            Self::Pattern { .. } => "Select by Pattern",
            Self::Previous { .. } => "Select by Previous Operation",
            Self::SelectionSet { .. } => "Select by Selection Set",
            Self::Random { .. } => "Select Randomly",
            Self::EdgeAngle { .. } => "Select Edges by Angle",
            Self::Border {} => "Select Border",
            Self::VertexCount { min: 3, max: 3 } => "Select Triangles",
            Self::VertexCount { min: 4, max: 4 } => "Select Quads",
            Self::VertexCount { min: 5, max: 0 } => "Select N-gons",
            Self::VertexCount { .. } => "Select Polygons by Vertex Count",
            Self::Area { .. } => "Select Polygons by Area",
            Self::EdgeLength { .. } => "Select Edges by Length",
            Self::Normal { .. } => "Select by Normal",
            Self::Plane { .. } => "Select by Plane",
            Self::Distance { .. } => "Select by Distance",
            Self::RandomCount { .. } => "Select N Randomly",
            Self::RandomParts { .. } => "Select Random Parts",
            Self::VertexEdges { .. } => "Select Vertices by Edge Count",
            Self::Symmetric { .. } => "Select Symmetric",
            Self::UvSeam { .. } => "Select Edges by UV Seam",
            Self::Connected {} => "Select Connected",
            Self::PolygonTag { .. } => "Select by Polygon Tag",
            Self::Coplanar { .. } => "Select Coplanar Polygons",
            Self::HardEdges {} => "Select by Hard Edges",
            Self::VertexMapValue { .. } => "Select Vertex Map Value",
            Self::Falloff { .. } => "Select by Falloff",
            Self::ConditionalLoop(_) => "Select Conditional Loop",
        }
    }
}

/// One selection op (MO-51 common settings + its kind).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelOp {
    pub enable: bool,
    pub invert: bool,
    #[serde(rename = "blendmode")]
    pub blend: Blend,
    #[serde(flatten)]
    pub kind: SelKind,
}
impl SelOp {
    pub fn new(kind: SelKind) -> Self {
        Self {
            enable: true,
            invert: false,
            blend: Blend::Override,
            kind,
        }
    }
}

fn truthy(word: &str) -> Option<bool> {
    match word.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

/// A mesh's elements as one kind sees them: the edge list and which edge
/// numbers each polygon uses, worked out once per resolve.
struct Elements<'a> {
    mesh: &'a Mesh,
    element: SelElement,
    edges: Vec<(u32, u32)>,
    edge_at: std::collections::HashMap<(u32, u32), u32>,
}
impl<'a> Elements<'a> {
    fn new(mesh: &'a Mesh, element: SelElement) -> Self {
        let edges = mesh.edges();
        let edge_at = edges
            .iter()
            .enumerate()
            .map(|(i, &e)| (e, i as u32))
            .collect();
        Self {
            mesh,
            element,
            edges,
            edge_at,
        }
    }
    fn count(&self) -> usize {
        match self.element {
            SelElement::Polygon => self.mesh.faces.len(),
            SelElement::Edge => self.edges.len(),
            SelElement::Point => self.mesh.vertices.len(),
        }
    }
    fn face_edges(&self, f: usize) -> impl Iterator<Item = u32> + '_ {
        self.mesh
            .element_spans(f)
            .filter_map(|(a, b)| self.edge_at.get(&(a.min(b), a.max(b))).copied())
    }
    /// Picked polygons as this kind.
    fn of_polygons(&self, faces: &[bool]) -> Vec<bool> {
        let mut out = vec![false; self.count()];
        for (f, _) in faces.iter().enumerate().filter(|(_, p)| **p) {
            match self.element {
                SelElement::Polygon => out[f] = true,
                SelElement::Edge => self.face_edges(f).for_each(|e| out[e as usize] = true),
                SelElement::Point => {
                    for &v in &self.mesh.faces[f].vertices {
                        out[v as usize] = true;
                    }
                }
            }
        }
        out
    }
    /// Picked edges (numbered as `edges`) as this kind.
    fn of_edges(&self, picked: &[bool]) -> Vec<bool> {
        match self.element {
            SelElement::Edge => picked.to_vec(),
            SelElement::Point => {
                let mut out = vec![false; self.count()];
                for (i, _) in picked.iter().enumerate().filter(|(_, p)| **p) {
                    let (a, b) = self.edges[i];
                    out[a as usize] = true;
                    out[b as usize] = true;
                }
                out
            }
            SelElement::Polygon => (0..self.mesh.faces.len())
                .map(|f| self.face_edges(f).any(|e| picked[e as usize]))
                .collect(),
        }
    }
    fn polygon_set(&self, set: &std::collections::BTreeSet<u32>) -> Vec<bool> {
        let mut faces = vec![false; self.mesh.faces.len()];
        for &f in set {
            if let Some(o) = faces.get_mut(f as usize) {
                *o = true;
            }
        }
        self.of_polygons(&faces)
    }
    /// Picked points as this kind: an edge or polygon when all its points
    /// are picked.
    fn of_points(&self, picked: &[bool]) -> Vec<bool> {
        match self.element {
            SelElement::Point => picked.to_vec(),
            SelElement::Edge => self
                .edges
                .iter()
                .map(|&(a, b)| picked[a as usize] && picked[b as usize])
                .collect(),
            SelElement::Polygon => self
                .mesh
                .faces
                .iter()
                .map(|f| !f.vertices.is_empty() && f.vertices.iter().all(|&v| picked[v as usize]))
                .collect(),
        }
    }
    /// Each element's centre: a polygon's average point, an edge's middle,
    /// a point itself.
    fn centres(&self) -> Vec<glam::DVec3> {
        let p = &self.mesh.vertices;
        match self.element {
            SelElement::Point => p.to_vec(),
            SelElement::Edge => self
                .edges
                .iter()
                .map(|&(a, b)| (p[a as usize] + p[b as usize]) * 0.5)
                .collect(),
            SelElement::Polygon => self
                .mesh
                .faces
                .iter()
                .map(|f| {
                    f.vertices
                        .iter()
                        .map(|&v| p[v as usize])
                        .sum::<glam::DVec3>()
                        / f.vertices.len().max(1) as f64
                })
                .collect(),
        }
    }
    /// Each element's normal (unit, or zero where it has none).
    fn normals(&self) -> Vec<glam::DVec3> {
        let mesh = self.mesh;
        let faces: Vec<glam::DVec3> = (0..mesh.faces.len()).map(|f| mesh.normal(f)).collect();
        if self.element == SelElement::Polygon {
            return faces;
        }
        let mut points = vec![glam::DVec3::ZERO; mesh.vertices.len()];
        for (f, face) in mesh.faces.iter().enumerate() {
            for &v in &face.vertices {
                points[v as usize] += faces[f];
            }
        }
        let points: Vec<glam::DVec3> = points.iter().map(|n| n.normalize_or_zero()).collect();
        match self.element {
            SelElement::Point => points,
            _ => self
                .edges
                .iter()
                .map(|&(a, b)| (points[a as usize] + points[b as usize]).normalize_or_zero())
                .collect(),
        }
    }
    /// Each polygon's part: polygons joined through shared points.
    fn parts(&self) -> (Vec<u32>, u32) {
        let mesh = self.mesh;
        let mut parent: Vec<u32> = (0..mesh.vertices.len() as u32).collect();
        fn find(parent: &mut [u32], mut i: u32) -> u32 {
            while parent[i as usize] != i {
                parent[i as usize] = parent[parent[i as usize] as usize];
                i = parent[i as usize];
            }
            i
        }
        for f in mesh.faces.iter() {
            for w in f.vertices.windows(2) {
                let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
                parent[a.max(b) as usize] = a.min(b);
            }
        }
        let mut ids = std::collections::HashMap::new();
        let part: Vec<u32> = mesh
            .faces
            .iter()
            .map(|f| {
                let root = f
                    .vertices
                    .first()
                    .map_or(u32::MAX, |&v| find(&mut parent, v));
                let next = ids.len() as u32;
                *ids.entry(root).or_insert(next)
            })
            .collect();
        (part, ids.len() as u32)
    }
    /// The polygons `picked` (of this kind) touches: the polygons
    /// themselves, or those using a picked edge or point.
    fn touched_polygons(&self, picked: &[bool]) -> Vec<bool> {
        let mesh = self.mesh;
        match self.element {
            SelElement::Polygon => picked.to_vec(),
            SelElement::Edge => (0..mesh.faces.len())
                .map(|f| self.face_edges(f).any(|e| picked[e as usize]))
                .collect(),
            SelElement::Point => mesh
                .faces
                .iter()
                .map(|f| f.vertices.iter().any(|&v| picked[v as usize]))
                .collect(),
        }
    }
    /// Each edge's polygons (one for a border edge).
    fn edge_faces(&self) -> Vec<Vec<u32>> {
        let mut out = vec![Vec::new(); self.edges.len()];
        for f in 0..self.mesh.faces.len() {
            for e in self.face_edges(f) {
                out[e as usize].push(f as u32);
            }
        }
        out
    }
}

/// `v` in `min..=max`; a Maximum below the Minimum is ignored.
fn within(v: f64, min: f64, max: f64) -> bool {
    v >= min - 1e-9 && (max < min || v <= max + 1e-9)
}
fn finite(values: &[f64], what: &str) -> Result<()> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(format!("{what} needs finite values"))
    }
}

impl SelOp {
    /// The elements of `mesh` this op picks, before Invert and Blend;
    /// `below` is the selection so far (Symmetric mirrors it).
    fn picks(&self, el: &Elements, below: &[bool]) -> Result<Vec<bool>> {
        let mesh = el.mesh;
        let n = el.count();
        let what = el.element.plural();
        let mut out = vec![false; n];
        match &self.kind {
            SelKind::Index { index } => {
                let s = crate::mesh_stack::SelectByIndex::parse(index)?;
                if let Some(&bad) = s.polygons.iter().find(|&&f| f as usize >= n) {
                    return Err(format!(
                        "Select by Index names number {bad}, but the mesh below has {n} {what}"
                    ));
                }
                for f in s.polygons {
                    out[f as usize] = true;
                }
            }
            SelKind::Range { start, end, step } => {
                if *step == 0 {
                    return Err("Select by Range needs a Step of at least 1".into());
                }
                if n > 0 {
                    let end = if *end == 0 {
                        n - 1
                    } else {
                        (*end as usize).min(n - 1)
                    };
                    let mut i = *start as usize;
                    while i <= end {
                        out[i] = true;
                        i += *step as usize;
                    }
                }
            }
            SelKind::Pattern { pattern } => {
                let bits: Vec<bool> = pattern
                    .split(',')
                    .filter(|w| !w.trim().is_empty())
                    .map(|w| {
                        truthy(w).ok_or_else(|| {
                            format!(
                                "Select by Pattern: “{}” isn't 1/0, True/False or Yes/No",
                                w.trim()
                            )
                        })
                    })
                    .collect::<Result<_>>()?;
                if !bits.is_empty() {
                    for (i, o) in out.iter_mut().enumerate() {
                        *o = bits[i % bits.len()];
                    }
                }
            }
            SelKind::Previous { source, name } => {
                // PG-04: an unknown or empty name selects nothing.
                if let Some(set) = mesh.attributes.polygon_sets.get(&group_set(*source, name)) {
                    out = el.polygon_set(set);
                }
            }
            SelKind::SelectionSet { name } => {
                // The set of this kind, else a polygon set of that name.
                match el.element {
                    SelElement::Edge if mesh.attributes.edge_sets.contains_key(name) => {
                        for e in &mesh.attributes.edge_sets[name] {
                            if let Some(&i) = el.edge_at.get(&(e.0.min(e.1), e.0.max(e.1))) {
                                out[i as usize] = true;
                            }
                        }
                    }
                    SelElement::Point if mesh.attributes.vertex_sets.contains_key(name) => {
                        for &v in &mesh.attributes.vertex_sets[name] {
                            if let Some(o) = out.get_mut(v as usize) {
                                *o = true;
                            }
                        }
                    }
                    _ => {
                        if let Some(set) = mesh.attributes.polygon_sets.get(name) {
                            out = el.polygon_set(set);
                        }
                    }
                }
            }
            SelKind::Random { seed, probability } => {
                // Each element's draw comes from the seed and its own index
                // (SplitMix64), not from a library generator, so a saved
                // scene picks the same elements on every version and a
                // pick doesn't depend on the ones before it.
                let p = f64::from(*probability) / 100.0;
                for (i, o) in out.iter_mut().enumerate() {
                    let key = (u64::from(*seed) << 32) | i as u64;
                    *o = crate::vertex_map_texture::random01(key) < p;
                }
            }
            SelKind::EdgeAngle {
                min,
                max,
                convex,
                concave,
            } => {
                finite(&[*min, *max], "Select Edges by Angle")?;
                let normals: Vec<glam::DVec3> =
                    (0..mesh.faces.len()).map(|f| mesh.normal(f)).collect();
                let picked: Vec<bool> = el
                    .edge_faces()
                    .iter()
                    .enumerate()
                    .map(|(e, faces)| {
                        // The angle between the two polygons' facings:
                        // 0 flat, 90 a right-angle corner.
                        let [a, b] = faces[..] else {
                            return false;
                        };
                        let (na, nb) = (normals[a as usize], normals[b as usize]);
                        if na == glam::DVec3::ZERO || nb == glam::DVec3::ZERO {
                            return false;
                        }
                        let angle = na.dot(nb).clamp(-1., 1.).acos().to_degrees();
                        if !within(angle, *min, *max) {
                            return false;
                        }
                        // Convex ("outy"): the other polygon's far points
                        // lie behind the first one's plane.
                        let (p, q) = el.edges[e];
                        let on_a = mesh.vertices[p as usize];
                        let far = mesh.faces[b as usize]
                            .vertices
                            .iter()
                            .find(|&&v| v != p && v != q)
                            .map(|&v| mesh.vertices[v as usize]);
                        let outy = far.is_none_or(|f| (f - on_a).dot(na) <= 0.);
                        if outy { *convex } else { *concave }
                    })
                    .collect();
                out = el.of_edges(&picked);
            }
            SelKind::Border {} => {
                let picked: Vec<bool> = el.edge_faces().iter().map(|f| f.len() == 1).collect();
                out = el.of_edges(&picked);
            }
            SelKind::VertexCount { min, max } => {
                let faces: Vec<bool> = (0..mesh.faces.len())
                    .map(|f| {
                        let k = mesh.faces[f].vertices.len() as u32;
                        !mesh.is_polyline(f) && k >= *min && (*max < *min || k <= *max)
                    })
                    .collect();
                out = el.of_polygons(&faces);
            }
            SelKind::Area { min, max } => {
                finite(&[*min, *max], "Select Polygons by Area")?;
                let faces: Vec<bool> = mesh
                    .faces
                    .iter()
                    .map(|f| {
                        let p = |i: usize| mesh.vertices[f.vertices[i] as usize];
                        let n = f.vertices.len();
                        let twice: glam::DVec3 = (0..n).map(|i| p(i).cross(p((i + 1) % n))).sum();
                        // The side of a square of the same area.
                        within((twice.length() * 0.5).sqrt(), *min, *max)
                    })
                    .collect();
                out = el.of_polygons(&faces);
            }
            SelKind::EdgeLength { min, max } => {
                finite(&[*min, *max], "Select Edges by Length")?;
                let picked: Vec<bool> = el
                    .edges
                    .iter()
                    .map(|&(a, b)| {
                        let d = mesh.vertices[a as usize].distance(mesh.vertices[b as usize]);
                        within(d, *min, *max)
                    })
                    .collect();
                out = el.of_edges(&picked);
            }
            SelKind::Normal { x, y, z, min, max } => {
                let dir = glam::DVec3::new(*x, *y, *z);
                finite(&[*x, *y, *z, *min, *max], "Select by Normal")?;
                if dir == glam::DVec3::ZERO {
                    return Err("Select by Normal needs a direction".into());
                }
                let dir = dir.normalize();
                out = el
                    .normals()
                    .iter()
                    .map(|n| {
                        *n != glam::DVec3::ZERO
                            && within(n.dot(dir).clamp(-1., 1.).acos().to_degrees(), *min, *max)
                    })
                    .collect();
            }
            SelKind::Plane {
                x,
                y,
                z,
                nx,
                ny,
                nz,
                negative,
                include_zero,
            } => {
                finite(&[*x, *y, *z, *nx, *ny, *nz], "Select by Plane")?;
                let (at, n) = (
                    glam::DVec3::new(*x, *y, *z),
                    glam::DVec3::new(*nx, *ny, *nz),
                );
                if n == glam::DVec3::ZERO {
                    return Err("Select by Plane needs a normal".into());
                }
                let n = n.normalize();
                out = el
                    .centres()
                    .iter()
                    .map(|c| {
                        let d = (*c - at).dot(n);
                        if d.abs() <= 1e-9 {
                            *include_zero
                        } else {
                            (d < 0.) == *negative
                        }
                    })
                    .collect();
            }
            SelKind::Distance { x, y, z, min, max } => {
                finite(&[*x, *y, *z, *min, *max], "Select by Distance")?;
                let from = glam::DVec3::new(*x, *y, *z);
                out = el
                    .centres()
                    .iter()
                    .map(|c| within(c.distance(from), *min, *max))
                    .collect();
            }
            SelKind::RandomCount {
                seed,
                number,
                percent,
            } => {
                finite(&[*percent], "Select N Randomly")?;
                let k = if *number > 0 {
                    (*number as usize).min(n)
                } else {
                    ((percent.clamp(0., 100.) / 100.) * n as f64).round() as usize
                };
                // Each element draws a key from the seed and its index;
                // the k lowest are picked (same on every version).
                let mut order: Vec<(f64, usize)> = (0..n)
                    .map(|i| {
                        let key = (u64::from(*seed) << 32) | i as u64;
                        (crate::vertex_map_texture::random01(key), i)
                    })
                    .collect();
                order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                for &(_, i) in order.iter().take(k) {
                    out[i] = true;
                }
            }
            SelKind::RandomParts { seed, chance } => {
                finite(&[*chance], "Select Random Parts")?;
                let (part, count) = el.parts();
                let p = chance.clamp(0., 100.) / 100.;
                let chosen: Vec<bool> = (0..count)
                    .map(|i| {
                        let key = (u64::from(*seed) << 32) | u64::from(i);
                        crate::vertex_map_texture::random01(key) < p
                    })
                    .collect();
                let faces: Vec<bool> = part
                    .iter()
                    .map(|&i| chosen.get(i as usize).copied().unwrap_or(false))
                    .collect();
                out = el.of_polygons(&faces);
            }
            SelKind::VertexEdges { min, max } => {
                let mut degree = vec![0u32; mesh.vertices.len()];
                for &(a, b) in &el.edges {
                    degree[a as usize] += 1;
                    degree[b as usize] += 1;
                }
                let points: Vec<bool> = degree
                    .iter()
                    .map(|&d| d > 0 && d >= *min && (*max < *min || d <= *max))
                    .collect();
                out = el.of_points(&points);
            }
            SelKind::Symmetric { x, y, z, tolerance } => {
                finite(&[*tolerance], "Select Symmetric")?;
                let flip = glam::DVec3::new(
                    if *x { -1. } else { 1. },
                    if *y { -1. } else { 1. },
                    if *z { -1. } else { 1. },
                );
                let tol = tolerance.max(1e-12);
                let centres = el.centres();
                // Elements by their centre, on a grid of tolerance-sized
                // cells; a mirrored centre looks in its cell and the 26
                // around it.
                let cell = |c: glam::DVec3| (c / tol).floor().as_i64vec3();
                let mut grid: std::collections::HashMap<glam::I64Vec3, Vec<usize>> =
                    std::collections::HashMap::new();
                for (i, c) in centres.iter().enumerate() {
                    grid.entry(cell(*c)).or_default().push(i);
                }
                for (i, _) in below.iter().enumerate().filter(|(_, b)| **b) {
                    let m = centres[i] * flip;
                    let home = cell(m);
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            for dz in -1..=1 {
                                let key = home + glam::I64Vec3::new(dx, dy, dz);
                                for &j in grid.get(&key).into_iter().flatten() {
                                    if centres[j].distance(m) <= tol {
                                        out[j] = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            SelKind::UvSeam { map } => {
                let Some(uvs) = mesh.uv_maps.get(map.as_str()) else {
                    return Err(format!(
                        "Select Edges by UV Seam: the mesh has no UV map “{map}”"
                    ));
                };
                let uv_of = |f: u32, v: u32| -> Option<glam::DVec2> {
                    let face = &mesh.faces[f as usize];
                    let corner = face.vertices.iter().position(|&w| w == v)?;
                    uvs.get(f as usize)?.get(corner).copied().flatten()
                };
                let picked: Vec<bool> = el
                    .edge_faces()
                    .iter()
                    .enumerate()
                    .map(|(e, faces)| {
                        let [a, b] = faces[..] else {
                            return false;
                        };
                        let (p, q) = el.edges[e];
                        let differs = |v: u32| match (uv_of(a, v), uv_of(b, v)) {
                            (Some(x), Some(y)) => x.distance(y) > 1e-9,
                            _ => false,
                        };
                        differs(p) || differs(q)
                    })
                    .collect();
                out = el.of_edges(&picked);
            }
            SelKind::Connected {} => {
                let (part, parts) = el.parts();
                let mut hit = vec![false; parts as usize];
                for (f, &t) in el.touched_polygons(below).iter().enumerate() {
                    if t {
                        hit[part[f] as usize] = true;
                    }
                }
                let faces: Vec<bool> = part.iter().map(|&p| hit[p as usize]).collect();
                out = el.of_polygons(&faces);
            }
            SelKind::PolygonTag { tag, name } => {
                let faces: Vec<bool> = match tag {
                    0 => mesh.faces.iter().map(|f| f.material == *name).collect(),
                    1 => mesh
                        .faces
                        .iter()
                        .map(|f| f.part.as_deref() == Some(name.as_str()))
                        .collect(),
                    _ => {
                        return Err(
                            "Select by Polygon Tag: Tag must be 0 (Material) or 1 (Part)".into(),
                        );
                    }
                };
                out = el.of_polygons(&faces);
            }
            SelKind::Coplanar { angle } => {
                finite(&[*angle], "Select Coplanar Polygons")?;
                let limit = angle.max(0.).to_radians() + 1e-9;
                let normals: Vec<glam::DVec3> =
                    (0..mesh.faces.len()).map(|f| mesh.normal(f)).collect();
                // Polygons across each edge.
                let mut next: Vec<Vec<u32>> = vec![Vec::new(); mesh.faces.len()];
                for fs in el.edge_faces() {
                    for &a in &fs {
                        for &b in &fs {
                            if a != b {
                                next[a as usize].push(b);
                            }
                        }
                    }
                }
                let mut faces = vec![false; mesh.faces.len()];
                for (seed, &t) in el.touched_polygons(below).iter().enumerate() {
                    if !t || faces[seed] {
                        continue;
                    }
                    let n = normals[seed];
                    faces[seed] = true;
                    let mut stack = vec![seed];
                    while let Some(f) = stack.pop() {
                        for &g in &next[f] {
                            let g = g as usize;
                            let m = normals[g];
                            if !faces[g]
                                && n != glam::DVec3::ZERO
                                && m != glam::DVec3::ZERO
                                && n.angle_between(m) <= limit
                            {
                                faces[g] = true;
                                stack.push(g);
                            }
                        }
                    }
                }
                out = el.of_polygons(&faces);
            }
            SelKind::HardEdges {} => {
                let hard = crate::vertex_normals::hard_edges(mesh);
                let picked: Vec<bool> = el.edges.iter().map(|e| hard.contains(e)).collect();
                out = el.of_edges(&picked);
            }
            SelKind::VertexMapValue { map, min, max } => {
                finite(&[*min, *max], "Select Vertex Map Value")?;
                if let Some(values) = mesh.attributes.point_weights.get(map) {
                    let used = mesh.used_points();
                    let points: Vec<bool> = (0..mesh.vertices.len())
                        .map(|v| {
                            used[v]
                                && within(
                                    values.get(&(v as u32)).copied().unwrap_or(0.),
                                    *min,
                                    *max,
                                )
                        })
                        .collect();
                    out = el.of_points(&points);
                }
            }
            SelKind::Falloff { threshold, falloff } => {
                finite(&[*threshold], "Select by Falloff")?;
                // TB-03: no falloff picks nothing.
                if let Some(w) = falloff {
                    w.validate()?;
                    let weight = w.weigher(glam::DMat4::IDENTITY);
                    let used = (el.element == SelElement::Point).then(|| mesh.used_points());
                    out = el
                        .centres()
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            used.as_ref().is_none_or(|u| u[i]) && weight(*c) >= *threshold - 1e-9
                        })
                        .collect();
                }
            }
            SelKind::ConditionalLoop(rules) => {
                // TB-10: the seeds are the selection so far, as edges.
                let seeds: Vec<bool> = match el.element {
                    SelElement::Edge => below.to_vec(),
                    SelElement::Polygon => {
                        let mut s = vec![false; el.edges.len()];
                        for (f, _) in below.iter().enumerate().filter(|(_, b)| **b) {
                            el.face_edges(f).for_each(|e| s[e as usize] = true);
                        }
                        s
                    }
                    SelElement::Point => el
                        .edges
                        .iter()
                        .map(|&(a, b)| below[a as usize] && below[b as usize])
                        .collect(),
                };
                let loops = crate::meshop_conditional_loop::grow(rules, mesh, &el.edges, &seeds);
                out = el.of_edges(&loops);
            }
        }
        Ok(out)
    }
}

/// An op's Selection input on `mesh`, as `element`s: `None` when it has
/// none (the op works on the whole mesh, PG-05), otherwise the picked
/// elements, which may be none at all (PG-04).
pub fn resolve(
    index: Option<&crate::mesh_stack::SelectByIndex>,
    selops: &[SelOp],
    mesh: &Mesh,
    element: SelElement,
) -> Result<Option<Vec<u32>>> {
    let index = index.filter(|s| !s.polygons.is_empty());
    // Turned-off selection ops act as if they weren't there.
    if index.is_none() && !selops.iter().any(|s| s.enable) {
        return Ok(None);
    }
    let el = Elements::new(mesh, element);
    let n = el.count();
    let mut sel = vec![false; n];
    if let Some(s) = index {
        if let Some(&bad) = s.polygons.iter().find(|&&f| f as usize >= n) {
            return Err(format!(
                "Select by Index names number {bad}, but the mesh below has {n} {}",
                element.plural()
            ));
        }
        for &f in &s.polygons {
            sel[f as usize] = true;
        }
    }
    for s in selops.iter().filter(|s| s.enable) {
        let mut mine = s.picks(&el, &sel)?;
        if s.invert {
            mine.iter_mut().for_each(|b| *b = !*b);
        }
        for (a, b) in sel.iter_mut().zip(mine) {
            *a = match s.blend {
                Blend::Override => b,
                Blend::Add => *a || b,
                Blend::Subtract => *a && !b,
                Blend::Multiply => *a && b,
            };
        }
    }
    Ok(Some(
        sel.iter()
            .enumerate()
            .filter(|(_, b)| **b)
            .map(|(i, _)| i as u32)
            .collect(),
    ))
}
