//! Statistics panel counts (spec: development/specs/statistics-panel/
//! BEHAVIOR-STATISTICS.md, with the answers Q1..Q11 in its header).
//!
//! The panel counts the geometry being worked on, sorted into groups (by
//! polygon size, by how many edges meet at a vertex, by part, material,
//! selection set and so on), and says how many of each group are selected.
//! This module is the counting, with no window: [`snapshot`] takes what a
//! refresh needs from a scene (cheap: shared meshes and the selection) and
//! [`compute`] counts it, on any thread. The app runs `compute` off the UI
//! thread (statistics_ui.rs) so a 5M-polygon mesh never stalls the views.
//!
//! - What is counted (spec §5): the active meshes, as the 3D view decides
//!   what is editable ([`target_meshes`]). Several active meshes: totals
//!   across all. Hidden elements are left out (Q3).
//! - Speed (spec §9): topology-only results (the edge list, how many edges
//!   and polygons meet at each point, polygon tags) are remembered per
//!   polygon list allocation ([`Topology`]), so a drag that only moves
//!   points pays for none of them. Only the groups the panel has open are
//!   counted in full ([`Need`]); the expensive checks only when the Checks
//!   group is open. Big loops run on every core ([`par_ranges`]).
//! - Rows and the elements in them are one definition: counting and the
//!   panel's +/− (statistics_edit.rs) both ask the same `*_slots`
//!   functions which rows an element belongs to.
use crate::element_state::ElementMask;
use crate::multi_active::ComponentSelection;
use crate::{Mesh, Mode, PolygonRepresentation, Scene, Shared};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex};

/// "In a straight line" (spec §4.1): the two edges are within 0.01° of a
/// straight line.
pub const STRAIGHT_DEGREES: f64 = 0.01;
/// "Sharp angle" (spec §4.3): a polygon corner under 1°.
pub const SHARP_DEGREES: f64 = 1.0;
/// "Flat" edges and "Flat with a neighbour" polygons: two polygons sharing
/// an edge whose normals are within this angle lie in one plane (the edge
/// is in both planes). Provisional (STAT-Q02), same as the straight-line
/// angle.
pub const COPLANAR_DEGREES: f64 = 0.01;

/// The four top-level groups (spec §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Vertex,
    Edge,
    Polygon,
    Item,
}
impl Kind {
    /// The selection mode the group's +/− selects in (Q2).
    pub fn mode(self) -> Mode {
        match self {
            Kind::Vertex => Mode::Vertex,
            Kind::Edge => Mode::Edge,
            Kind::Polygon => Mode::Polygon,
            Kind::Item => Mode::Item,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Kind::Vertex => "Vertices",
            Kind::Edge => "Edges",
            Kind::Polygon => "Polygons",
            Kind::Item => "Items",
        }
    }
    fn word(self) -> &'static str {
        match self {
            Kind::Vertex => "vertices",
            Kind::Edge => "edges",
            Kind::Polygon => "polygons",
            Kind::Item => "items",
        }
    }
}

/// Edge > Border rows (spec §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Border {
    Part,
    Material,
    Geometry,
    Uv,
    Flat,
}
impl Border {
    pub const ALL: [Border; 5] = [
        Border::Part,
        Border::Material,
        Border::Geometry,
        Border::Uv,
        Border::Flat,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Border::Part => "Part",
            Border::Material => "Material",
            Border::Geometry => "Geometry",
            Border::Uv => "UV",
            Border::Flat => "Flat",
        }
    }
    fn word(self) -> &'static str {
        match self {
            Border::Part => "part",
            Border::Material => "material",
            Border::Geometry => "geometry",
            Border::Uv => "uv",
            Border::Flat => "flat",
        }
    }
}

/// Polygon > Checks rows (spec §4.3, Q4, Q6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Check {
    SinglePoint,
    NotFlat,
    Overlapping,
    FlatNeighbour,
    SharpAngle,
    Concave,
}
impl Check {
    pub const ALL: [Check; 6] = [
        Check::SinglePoint,
        Check::NotFlat,
        Check::Overlapping,
        Check::FlatNeighbour,
        Check::SharpAngle,
        Check::Concave,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Check::SinglePoint => "Single point",
            Check::NotFlat => "Not flat",
            Check::Overlapping => "Overlapping",
            Check::FlatNeighbour => "Flat with a neighbour",
            Check::SharpAngle => "Sharp angle",
            Check::Concave => "Concave",
        }
    }
    fn word(self) -> &'static str {
        match self {
            Check::SinglePoint => "singlePoint",
            Check::NotFlat => "notFlat",
            Check::Overlapping => "overlapping",
            Check::FlatNeighbour => "flatNeighbour",
            Check::SharpAngle => "sharpAngle",
            Check::Concave => "concave",
        }
    }
    /// Checks that read point positions polygon by polygon (spec §9: only
    /// while the Checks group is open). Single point reads only topology.
    pub fn expensive(self) -> bool {
        self != Check::SinglePoint
    }
}

/// Polygon > By type rows: every polygon type Operandi has (spec §4.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PolyType {
    Face,
    SubD,
    CatmullClark,
    Patch,
    Line,
    Curve,
    BSpline,
    Bezier,
}
impl PolyType {
    pub const ALL: [PolyType; 8] = [
        PolyType::Face,
        PolyType::SubD,
        PolyType::CatmullClark,
        PolyType::Patch,
        PolyType::Line,
        PolyType::Curve,
        PolyType::BSpline,
        PolyType::Bezier,
    ];
    pub fn of(rep: Option<&PolygonRepresentation>) -> PolyType {
        use crate::curves::CurveKind;
        match rep.copied().unwrap_or_default() {
            PolygonRepresentation::Face => PolyType::Face,
            PolygonRepresentation::Subdivs => PolyType::SubD,
            PolygonRepresentation::CatmullClark => PolyType::CatmullClark,
            PolygonRepresentation::Patch => PolyType::Patch,
            PolygonRepresentation::Polyline { curve, .. } => match curve {
                CurveKind::Line => PolyType::Line,
                CurveKind::Curve { .. } => PolyType::Curve,
                CurveKind::BSpline => PolyType::BSpline,
                CurveKind::Bezier => PolyType::Bezier,
            },
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            PolyType::Face => "Face",
            PolyType::SubD => "SubD",
            PolyType::CatmullClark => "Catmull-Clark",
            PolyType::Patch => "Patch",
            PolyType::Line => "Line",
            PolyType::Curve => "Curve",
            PolyType::BSpline => "B-Spline",
            PolyType::Bezier => "Bezier",
        }
    }
    fn word(self) -> &'static str {
        match self {
            PolyType::Face => "face",
            PolyType::SubD => "subd",
            PolyType::CatmullClark => "catmullClark",
            PolyType::Patch => "patch",
            PolyType::Line => "line",
            PolyType::Curve => "curve",
            PolyType::BSpline => "bspline",
            PolyType::Bezier => "bezier",
        }
    }
    fn index(self) -> usize {
        PolyType::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

/// Vertex map kinds (spec §4.1 "By vertex map": one sub-group per kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MapKind {
    Weight,
    Uv,
    Morph,
    Color,
    Normal,
    /// Typed maps of another kind (tangent bases, derivatives ...).
    Other,
}
impl MapKind {
    pub const ALL: [MapKind; 6] = [
        MapKind::Weight,
        MapKind::Uv,
        MapKind::Morph,
        MapKind::Color,
        MapKind::Normal,
        MapKind::Other,
    ];
    pub fn label(self) -> &'static str {
        match self {
            MapKind::Weight => "Weight",
            MapKind::Uv => "UV",
            MapKind::Morph => "Morph",
            MapKind::Color => "Color",
            MapKind::Normal => "Normal",
            MapKind::Other => "Other",
        }
    }
    fn word(self) -> &'static str {
        match self {
            MapKind::Weight => "weight",
            MapKind::Uv => "uv",
            MapKind::Morph => "morph",
            MapKind::Color => "color",
            MapKind::Normal => "normal",
            MapKind::Other => "other",
        }
    }
    /// The kind a typed map's `kind` string belongs to.
    pub fn of_typed(kind: &str) -> MapKind {
        match kind {
            "weight1" => MapKind::Weight,
            "uv2" => MapKind::Uv,
            "relative_morph3" | "absolute_position3" | "pose_shape3" => MapKind::Morph,
            "rgba4" | "rgb3" => MapKind::Color,
            "normal3" => MapKind::Normal,
            _ => MapKind::Other,
        }
    }
}

/// Item > By type rows (spec §4.4; only types present, Q7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemType {
    Mesh,
    Instance,
    Camera,
    Light,
    Backdrop,
    Texture,
    Locator,
    Environment,
    Replicator,
    Kit,
}
impl ItemType {
    pub const ALL: [ItemType; 10] = [
        ItemType::Mesh,
        ItemType::Instance,
        ItemType::Camera,
        ItemType::Light,
        ItemType::Backdrop,
        ItemType::Texture,
        ItemType::Locator,
        ItemType::Environment,
        ItemType::Replicator,
        ItemType::Kit,
    ];
    pub fn of(item: &crate::Item) -> ItemType {
        if item.camera.is_some() {
            ItemType::Camera
        } else if item.light.is_some() {
            ItemType::Light
        } else if item.backdrop.is_some() {
            ItemType::Backdrop
        } else if item.texture.is_some() {
            ItemType::Texture
        } else if item.locator.is_some() {
            ItemType::Locator
        } else if item.environment.is_some() {
            ItemType::Environment
        } else if item.replicator.is_some() {
            ItemType::Replicator
        } else if item.kit.is_some() {
            ItemType::Kit
        } else if item.instance_of.is_some() {
            ItemType::Instance
        } else {
            ItemType::Mesh
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ItemType::Mesh => "Mesh",
            ItemType::Instance => "Mesh Instance",
            ItemType::Camera => "Camera",
            ItemType::Light => "Light",
            ItemType::Backdrop => "Backdrop",
            ItemType::Texture => "Texture Locator",
            ItemType::Locator => "Locator",
            ItemType::Environment => "Environment",
            ItemType::Replicator => "Instancer",
            ItemType::Kit => "Kit Item",
        }
    }
    fn word(self) -> &'static str {
        match self {
            ItemType::Mesh => "mesh",
            ItemType::Instance => "instance",
            ItemType::Camera => "camera",
            ItemType::Light => "light",
            ItemType::Backdrop => "backdrop",
            ItemType::Texture => "texture",
            ItemType::Locator => "locator",
            ItemType::Environment => "environment",
            ItemType::Replicator => "replicator",
            ItemType::Kit => "kit",
        }
    }
}

/// The "more than 4" bucket of the by-count rows.
pub const MORE: u8 = 5;

/// One countable row of the panel (spec §4). Heading-only rows are the
/// panel's own; every `Row` has a count and +/−.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Row {
    /// The top row of a group: every vertex, edge, polygon or item.
    All(Kind),
    /// Vertices with exactly this many edges (0..=4, [`MORE`] = more).
    VertexEdges(u8),
    /// Vertices used by exactly this many polygons (0..=4, [`MORE`]).
    VertexPolygons(u8),
    /// Border > In a straight line.
    VertexStraight,
    VertexMap(MapKind, String),
    VertexSet(String),
    /// Edges used by exactly this many polygons (1..=4, [`MORE`]).
    EdgePolygons(u8),
    EdgeBorder(Border),
    /// By vertex map: subdivision edge weights (creases).
    EdgeWeight,
    EdgeSet(String),
    PolygonType(PolyType),
    PolygonCheck(Check),
    /// Polygons with this many points (1..=4, [`MORE`] = n-gons).
    PolygonVertices(u8),
    Part(String),
    Material(String),
    PolygonSet(String),
    Smoothing(String),
    ItemType(ItemType),
    /// An item group (Groups panel), by its id.
    ItemGroup(u64),
}

impl Row {
    /// Which group the row's elements belong to.
    pub fn kind(&self) -> Kind {
        match self {
            Row::All(k) => *k,
            Row::VertexEdges(_)
            | Row::VertexPolygons(_)
            | Row::VertexStraight
            | Row::VertexMap(..)
            | Row::VertexSet(_) => Kind::Vertex,
            Row::EdgePolygons(_) | Row::EdgeBorder(_) | Row::EdgeWeight | Row::EdgeSet(_) => {
                Kind::Edge
            }
            Row::PolygonType(_)
            | Row::PolygonCheck(_)
            | Row::PolygonVertices(_)
            | Row::Part(_)
            | Row::Material(_)
            | Row::PolygonSet(_)
            | Row::Smoothing(_) => Kind::Polygon,
            Row::ItemType(_) | Row::ItemGroup(_) => Kind::Item,
        }
    }
    /// A row for a named thing the context menu can rename or delete
    /// (spec §7).
    pub fn is_named(&self) -> bool {
        matches!(
            self,
            Row::VertexMap(..)
                | Row::VertexSet(_)
                | Row::EdgeSet(_)
                | Row::Part(_)
                | Row::Material(_)
                | Row::PolygonSet(_)
                | Row::Smoothing(_)
                | Row::ItemGroup(_)
        )
    }
    /// Selection sets and vertex maps: Copy and Paste (spec §7).
    pub fn copies(&self) -> bool {
        matches!(
            self,
            Row::VertexMap(..) | Row::VertexSet(_) | Row::EdgeSet(_) | Row::PolygonSet(_)
        )
    }
    /// The name a named row shows (and renames).
    pub fn name(&self) -> Option<&str> {
        match self {
            Row::VertexMap(_, n)
            | Row::VertexSet(n)
            | Row::EdgeSet(n)
            | Row::Part(n)
            | Row::Material(n)
            | Row::PolygonSet(n)
            | Row::Smoothing(n) => Some(n),
            _ => None,
        }
    }
    /// Whether counting this row reads point positions polygon by polygon
    /// (spec §9's expensive checks).
    pub fn expensive(&self) -> bool {
        matches!(self, Row::PolygonCheck(c) if c.expensive())
    }
    /// The label a by-count row shows: "0" .. "4", "more than 4".
    pub fn count_label(n: u8) -> String {
        if n >= MORE {
            "more than 4".into()
        } else {
            n.to_string()
        }
    }
    /// A stable key for the panel's tree (and its saved open groups).
    pub fn key(&self) -> u64 {
        stable_hash(&self.to_string())
    }
}

/// FNV-1a: stable across runs and builds (tree keys, saved state).
pub fn stable_hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn count_word(n: u8) -> String {
    if n >= MORE {
        "more".into()
    } else {
        n.to_string()
    }
}
fn parse_count(text: &str, min: u8) -> Result<u8, String> {
    if text.eq_ignore_ascii_case("more") {
        return Ok(MORE);
    }
    match text.parse::<u8>() {
        Ok(n) if (min..MORE).contains(&n) => Ok(n),
        _ => Err(format!("{text}: use {min}..4 or more")),
    }
}

/// The row's script form (`operandi.statistics.select row:`), e.g.
/// `polygons.vertices:3`, `edges.border:flat`, `polygons.part:Wheel`.
impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Row::All(k) => write!(f, "{}", k.word()),
            Row::VertexEdges(n) => write!(f, "vertices.edges:{}", count_word(*n)),
            Row::VertexPolygons(n) => write!(f, "vertices.polygons:{}", count_word(*n)),
            Row::VertexStraight => write!(f, "vertices.straight"),
            Row::VertexMap(k, n) => write!(f, "vertices.map:{}:{n}", k.word()),
            Row::VertexSet(n) => write!(f, "vertices.set:{n}"),
            Row::EdgePolygons(n) => write!(f, "edges.polygons:{}", count_word(*n)),
            Row::EdgeBorder(b) => write!(f, "edges.border:{}", b.word()),
            Row::EdgeWeight => write!(f, "edges.weight"),
            Row::EdgeSet(n) => write!(f, "edges.set:{n}"),
            Row::PolygonType(t) => write!(f, "polygons.type:{}", t.word()),
            Row::PolygonCheck(c) => write!(f, "polygons.check:{}", c.word()),
            Row::PolygonVertices(n) => write!(f, "polygons.vertices:{}", count_word(*n)),
            Row::Part(n) => write!(f, "polygons.part:{n}"),
            Row::Material(n) => write!(f, "polygons.material:{n}"),
            Row::PolygonSet(n) => write!(f, "polygons.set:{n}"),
            Row::Smoothing(n) => write!(f, "polygons.smoothing:{n}"),
            Row::ItemType(t) => write!(f, "items.type:{}", t.word()),
            Row::ItemGroup(id) => write!(f, "items.group:{id}"),
        }
    }
}

impl std::str::FromStr for Row {
    type Err = String;
    fn from_str(text: &str) -> Result<Row, String> {
        let text = text.trim();
        let (head, value) = match text.split_once(':') {
            Some((h, v)) => (h, Some(v)),
            None => (text, None),
        };
        let need = |v: Option<&str>| -> Result<String, String> {
            match v {
                Some(v) if !v.is_empty() => Ok(v.to_owned()),
                _ => Err(format!("{head} needs a value after ':'")),
            }
        };
        let pick = |v: Option<&str>, words: &[&str]| -> Result<usize, String> {
            let v = need(v)?;
            words
                .iter()
                .position(|w| w.eq_ignore_ascii_case(&v))
                .ok_or_else(|| format!("{v}: use one of {}", words.join(", ")))
        };
        Ok(match head.to_ascii_lowercase().as_str() {
            "vertices" => Row::All(Kind::Vertex),
            "edges" => Row::All(Kind::Edge),
            "polygons" => Row::All(Kind::Polygon),
            "items" => Row::All(Kind::Item),
            "vertices.edges" => Row::VertexEdges(parse_count(&need(value)?, 0)?),
            "vertices.polygons" => Row::VertexPolygons(parse_count(&need(value)?, 0)?),
            "vertices.straight" => Row::VertexStraight,
            "vertices.map" => {
                let v = need(value)?;
                let (kind, name) = v
                    .split_once(':')
                    .ok_or("vertices.map:<kind>:<name>, e.g. vertices.map:weight:Mask")?;
                let words: Vec<&str> = MapKind::ALL.iter().map(|k| k.word()).collect();
                let i = pick(Some(kind), &words)?;
                Row::VertexMap(MapKind::ALL[i], need(Some(name))?)
            }
            "vertices.set" => Row::VertexSet(need(value)?),
            "edges.polygons" => Row::EdgePolygons(parse_count(&need(value)?, 1)?),
            "edges.border" => {
                let words: Vec<&str> = Border::ALL.iter().map(|b| b.word()).collect();
                Row::EdgeBorder(Border::ALL[pick(value, &words)?])
            }
            "edges.weight" => Row::EdgeWeight,
            "edges.set" => Row::EdgeSet(need(value)?),
            "polygons.type" => {
                let words: Vec<&str> = PolyType::ALL.iter().map(|t| t.word()).collect();
                Row::PolygonType(PolyType::ALL[pick(value, &words)?])
            }
            "polygons.check" => {
                let words: Vec<&str> = Check::ALL.iter().map(|c| c.word()).collect();
                Row::PolygonCheck(Check::ALL[pick(value, &words)?])
            }
            "polygons.vertices" => Row::PolygonVertices(parse_count(&need(value)?, 1)?),
            "polygons.part" => Row::Part(need(value)?),
            "polygons.material" => Row::Material(need(value)?),
            "polygons.set" => Row::PolygonSet(need(value)?),
            "polygons.smoothing" => Row::Smoothing(need(value)?),
            "items.type" => {
                let words: Vec<&str> = ItemType::ALL.iter().map(|t| t.word()).collect();
                Row::ItemType(ItemType::ALL[pick(value, &words)?])
            }
            "items.group" => Row::ItemGroup(
                need(value)?
                    .parse()
                    .map_err(|_| "items.group:<group id>".to_owned())?,
            ),
            other => return Err(format!("Unknown statistics row {other}")),
        })
    }
}

/// Which groups are open, so only they are counted in full (spec §9).
/// Collapsed groups get only their top count.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Need {
    pub vertices: bool,
    pub edges: bool,
    pub polygons: bool,
    /// The polygon Checks group is open (its expensive checks run).
    pub checks: bool,
}
impl Need {
    pub const ALL: Need = Need {
        vertices: true,
        edges: true,
        polygons: true,
        checks: true,
    };
    pub const TOP: Need = Need {
        vertices: false,
        edges: false,
        polygons: false,
        checks: false,
    };
}

/// One mesh a refresh counts: shared, so taking it costs a reference
/// count, and the UI may go on editing while it is counted.
#[derive(Clone, Debug)]
pub struct MeshInput {
    pub id: u64,
    pub mesh: Shared<Mesh>,
    pub hidden: ElementMask,
    pub selected: ComponentSelection,
}

/// One item, for the Items group.
#[derive(Clone, Debug)]
pub struct ItemInput {
    pub id: u64,
    pub kind: ItemType,
    pub selected: bool,
}

/// Everything one refresh reads, taken from the scene on the UI thread.
#[derive(Clone, Debug)]
pub struct Input {
    pub meshes: Vec<MeshInput>,
    pub items: Vec<ItemInput>,
    /// Item groups: id, name, members.
    pub groups: Vec<(u64, String, Vec<u64>)>,
    pub need: Need,
}

/// The meshes the geometry groups count (spec §5): the active meshes (the
/// selected, shown mesh items), or with none of them in a component mode
/// every shown mesh, as the 3D view lets you pick on any mesh then.
pub fn target_meshes(scene: &Scene) -> Vec<u64> {
    let active = scene.active_meshes();
    if !active.is_empty() || scene.selection.mode == Mode::Item {
        return active;
    }
    scene
        .items
        .iter()
        .filter(|i| i.is_mesh() && scene.shows(i) && !scene.is_display_hidden(i.id))
        .map(|i| i.id)
        .collect()
}

/// What a refresh needs from `scene`: cheap (shared meshes; the selection
/// and hidden sets are copied).
pub fn snapshot(scene: &Scene, need: Need) -> Input {
    let meshes = target_meshes(scene)
        .into_iter()
        .filter_map(|id| scene.item(id))
        .map(|item| MeshInput {
            id: item.id,
            mesh: item.mesh.shared().clone(),
            hidden: item.elements.hidden.clone(),
            selected: ComponentSelection {
                vertices: scene.selection.vertices_of(item.id).to_vec(),
                edges: scene.selection.edges_of(item.id).to_vec(),
                faces: scene.selection.faces_of(item.id).to_vec(),
            },
        })
        .collect();
    let items = scene
        .items
        .iter()
        .map(|i| ItemInput {
            id: i.id,
            kind: ItemType::of(i),
            selected: scene.selection.has_item(i.id),
        })
        .collect();
    let groups = scene
        .groups
        .iter()
        .map(|g| (g.id, g.name.clone(), g.members.clone()))
        .collect();
    Input {
        meshes,
        items,
        groups,
        need,
    }
}

/// A row's two numbers: how many elements, how many of them selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub count: u64,
    pub selected: u64,
}

/// The names the named categories list (spec §4), across the counted
/// meshes, sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    pub parts: Vec<String>,
    pub materials: Vec<String>,
    pub vertex_sets: Vec<String>,
    pub edge_sets: Vec<String>,
    pub polygon_sets: Vec<String>,
    pub smoothing: Vec<String>,
    pub vertex_maps: Vec<(MapKind, String)>,
    pub edge_weights: bool,
    pub item_types: Vec<ItemType>,
    pub item_groups: Vec<(u64, String)>,
}

/// One refresh's result.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub rows: HashMap<Row, Tally>,
    pub names: Names,
    /// What was counted in full (rows outside it are not in `rows`).
    pub need: Option<Need>,
    /// Meshes counted (0: the geometry groups are dimmed, spec §10).
    pub meshes: usize,
}
impl Report {
    pub fn get(&self, row: &Row) -> Option<Tally> {
        self.rows.get(row).copied()
    }
}

// ------------------------------------------------------------ topology

/// Topology-only facts about a mesh, remembered per polygon list
/// allocation (and point count and polygon types), so drags that only move
/// points never rebuild them.
#[derive(Debug, Default)]
pub struct Topology {
    nv: usize,
    reps: Vec<PolygonRepresentation>,
    /// Edges owned by their lower point: point `a`'s edges are
    /// `edge_other[edge_start[a]..edge_start[a + 1]]` (the higher points,
    /// ascending). Edge index order is the sorted `(a, b)` order.
    edge_start: Vec<u32>,
    edge_other: Vec<u32>,
    /// The distinct elements (polygons and lines) using each edge, in
    /// polygon order: edge `e`'s are `edge_faces[ef_start[e]..ef_start[e + 1]]`.
    ef_start: Vec<u32>,
    edge_faces: Vec<u32>,
    /// Surface polygons (not lines) using each edge.
    edge_surface: Vec<u32>,
    /// Distinct edges at each point.
    v_edges: Vec<u32>,
    /// Distinct elements using each point.
    v_polys: Vec<u32>,
    /// Points with exactly two edges, with their two neighbours.
    two_edge: Vec<[u32; 3]>,
    /// Each polygon's part and material as an index into the names
    /// (`u32::MAX` = no part).
    part_of: Vec<u32>,
    part_names: Vec<String>,
    material_of: Vec<u32>,
    material_names: Vec<String>,
}

const NONE: u32 = u32::MAX;

impl Topology {
    pub fn edge_count(&self) -> usize {
        self.edge_other.len()
    }
    /// The edge index of `(a, b)` (any order), if the mesh has it.
    pub fn edge_index(&self, a: u32, b: u32) -> Option<usize> {
        let (a, b) = (a.min(b), a.max(b));
        let s = *self.edge_start.get(a as usize)? as usize;
        let e = *self.edge_start.get(a as usize + 1)? as usize;
        self.edge_other[s..e].binary_search(&b).ok().map(|i| s + i)
    }
    /// Edge `e`'s two points, lower first.
    pub fn edge(&self, e: usize) -> (u32, u32) {
        // The owner is the last point whose range starts at or before e.
        let a = self.edge_start.partition_point(|&s| s as usize <= e) - 1;
        (a as u32, self.edge_other[e])
    }
    fn faces_of_edge(&self, e: usize) -> &[u32] {
        &self.edge_faces[self.ef_start[e] as usize..self.ef_start[e + 1] as usize]
    }
    /// Every edge's two points, in edge index order.
    fn edge_pairs(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::with_capacity(self.edge_other.len());
        for a in 0..self.nv {
            for &b in &self.edge_other[self.edge_start[a] as usize..self.edge_start[a + 1] as usize]
            {
                out.push((a as u32, b));
            }
        }
        out
    }
}

/// How many ranges a loop of `n` steps splits into: one per core for big
/// loops, one for small ones (threads cost more than they save there).
fn parts_for(n: usize) -> usize {
    if n < 40_000 {
        return 1;
    }
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
        .clamp(1, 16)
}

/// Runs `f` on consecutive ranges of `0..n`, one per core, and gives the
/// results in range order.
pub fn par_ranges<T: Send>(n: usize, f: impl Fn(std::ops::Range<usize>) -> T + Sync) -> Vec<T> {
    par_ranges_in(n, parts_for(n), f)
}

/// [`par_ranges`] in `parts` ranges.
fn par_ranges_in<T: Send>(
    n: usize,
    parts: usize,
    f: impl Fn(std::ops::Range<usize>) -> T + Sync,
) -> Vec<T> {
    if parts <= 1 {
        return vec![f(0..n)];
    }
    let step = n.div_ceil(parts);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..parts)
            .map(|p| {
                let r = (p * step).min(n)..((p + 1) * step).min(n);
                let f = &f;
                s.spawn(move || f(r))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("statistics worker panicked"))
            .collect()
    })
}

fn is_open_line(rep: Option<&PolygonRepresentation>) -> bool {
    matches!(
        rep,
        Some(PolygonRepresentation::Polyline { closed: false, .. })
    )
}
fn is_line(rep: Option<&PolygonRepresentation>) -> bool {
    matches!(rep, Some(PolygonRepresentation::Polyline { .. }))
}

/// Interns names polygon by polygon; consecutive polygons usually share
/// one, so most polygons cost a string compare.
fn intern<'a>(names: impl Iterator<Item = Option<&'a str>>, n: usize) -> (Vec<u32>, Vec<String>) {
    let mut of = Vec::with_capacity(n);
    let mut index: HashMap<&str, u32> = HashMap::new();
    let mut list: Vec<String> = Vec::new();
    let mut last: Option<(&str, u32)> = None;
    for name in names {
        let Some(name) = name else {
            of.push(NONE);
            continue;
        };
        if let Some((l, i)) = last
            && l == name
        {
            of.push(i);
            continue;
        }
        let i = *index.entry(name).or_insert_with(|| {
            list.push(name.to_owned());
            (list.len() - 1) as u32
        });
        last = Some((name, i));
        of.push(i);
    }
    (of, list)
}

/// Builds the topology of `mesh` (no memo).
pub fn build_topology(mesh: &Mesh) -> Topology {
    let nv = mesh.vertices.len();
    let faces = &mesh.faces;
    let reps = &mesh.attributes.representations;
    // Point -> polygons, packed.
    let mut offsets = vec![0u32; nv + 1];
    for f in faces.iter() {
        for &v in &f.vertices {
            if let Some(o) = offsets.get_mut(v as usize + 1) {
                *o += 1;
            }
        }
    }
    for i in 1..offsets.len() {
        offsets[i] += offsets[i - 1];
    }
    let mut next = offsets.clone();
    let mut point_faces = vec![0u32; offsets[nv] as usize];
    for (fi, f) in faces.iter().enumerate() {
        for &v in &f.vertices {
            if let Some(n) = next.get_mut(v as usize) {
                point_faces[*n as usize] = fi as u32;
                *n += 1;
            }
        }
    }
    drop(next);
    struct Chunk {
        v_edges: Vec<u32>,
        v_polys: Vec<u32>,
        /// Owned edges per point of the range (count), their other ends,
        /// and per edge its elements and surface count.
        owned: Vec<u32>,
        other: Vec<u32>,
        ef_len: Vec<u32>,
        ef: Vec<u32>,
        surface: Vec<u32>,
        two: Vec<[u32; 3]>,
    }
    let chunks = par_ranges(nv, |range| {
        let mut c = Chunk {
            v_edges: Vec::with_capacity(range.len()),
            v_polys: Vec::with_capacity(range.len()),
            owned: Vec::with_capacity(range.len()),
            other: Vec::new(),
            ef_len: Vec::new(),
            ef: Vec::new(),
            surface: Vec::new(),
            two: Vec::new(),
        };
        let mut nb: Vec<(u32, u32)> = Vec::new();
        for v in range {
            nb.clear();
            let users = &point_faces[offsets[v] as usize..offsets[v + 1] as usize];
            let mut polys = 0u32;
            let mut last = NONE;
            for &f in users {
                if f == last {
                    continue;
                }
                last = f;
                polys += 1;
                let vs = &faces[f as usize].vertices;
                let n = vs.len();
                let open = is_open_line(reps.get(f as usize));
                for i in 0..n {
                    if vs[i] as usize != v {
                        continue;
                    }
                    if !open || i + 1 < n {
                        let o = vs[(i + 1) % n];
                        if o as usize != v {
                            nb.push((o, f));
                        }
                    }
                    if !open || i > 0 {
                        let o = vs[(i + n - 1) % n];
                        if o as usize != v {
                            nb.push((o, f));
                        }
                    }
                }
            }
            nb.sort_unstable();
            nb.dedup();
            let mut edges = 0u32;
            let mut owned = 0u32;
            let mut ends = [NONE; 2];
            let mut i = 0;
            while i < nb.len() {
                let o = nb[i].0;
                let mut j = i;
                while j < nb.len() && nb[j].0 == o {
                    j += 1;
                }
                if (edges as usize) < 2 {
                    ends[edges as usize] = o;
                }
                edges += 1;
                if o as usize > v {
                    owned += 1;
                    c.other.push(o);
                    c.ef_len.push((j - i) as u32);
                    let mut surface = 0;
                    for &(_, f) in &nb[i..j] {
                        c.ef.push(f);
                        if !is_line(reps.get(f as usize)) {
                            surface += 1;
                        }
                    }
                    c.surface.push(surface);
                }
                i = j;
            }
            if edges == 2 {
                c.two.push([v as u32, ends[0], ends[1]]);
            }
            c.v_edges.push(edges);
            c.v_polys.push(polys);
            c.owned.push(owned);
        }
        c
    });
    let ne: usize = chunks.iter().map(|c| c.other.len()).sum();
    let nef: usize = chunks.iter().map(|c| c.ef.len()).sum();
    let mut t = Topology {
        nv,
        reps: reps.clone(),
        edge_start: Vec::with_capacity(nv + 1),
        edge_other: Vec::with_capacity(ne),
        ef_start: Vec::with_capacity(ne + 1),
        edge_faces: Vec::with_capacity(nef),
        edge_surface: Vec::with_capacity(ne),
        v_edges: Vec::with_capacity(nv),
        v_polys: Vec::with_capacity(nv),
        two_edge: Vec::new(),
        ..Default::default()
    };
    t.edge_start.push(0);
    t.ef_start.push(0);
    for c in chunks {
        for &o in &c.owned {
            let last = *t.edge_start.last().unwrap_or(&0);
            t.edge_start.push(last + o);
        }
        for &l in &c.ef_len {
            let last = *t.ef_start.last().unwrap_or(&0);
            t.ef_start.push(last + l);
        }
        t.edge_other.extend(c.other);
        t.edge_faces.extend(c.ef);
        t.edge_surface.extend(c.surface);
        t.v_edges.extend(c.v_edges);
        t.v_polys.extend(c.v_polys);
        t.two_edge.extend(c.two);
    }
    if nv == 0 {
        t.edge_start = vec![0];
    }
    let n = faces.len();
    (t.part_of, t.part_names) = intern(faces.iter().map(|f| f.part.as_deref()), n);
    (t.material_of, t.material_names) =
        intern(faces.iter().map(|f| Some(material_name(&f.material))), n);
    t
}

/// The material a polygon shows under: blank is the default material.
pub fn material_name(tag: &str) -> &str {
    if tag.trim().is_empty() {
        "Default"
    } else {
        tag
    }
}

/// The topology of `mesh`, remembered on its polygon list.
pub fn topology(mesh: &Mesh) -> Arc<Topology> {
    let memo = mesh.faces.derived(|_| Mutex::new(None::<Arc<Topology>>));
    let mut slot = memo.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(t) = slot.as_ref()
        && t.nv == mesh.vertices.len()
        && t.reps == mesh.attributes.representations
    {
        return t.clone();
    }
    let t = Arc::new(build_topology(mesh));
    *slot = Some(t.clone());
    t
}

// ------------------------------------------------------------ one mesh

/// Per-element flags: `None` means "every element".
type Flags = Option<Vec<bool>>;

fn flag(f: &Flags, i: usize) -> bool {
    f.as_ref()
        .is_none_or(|v| v.get(i).copied().unwrap_or(false))
}

/// Slot layout of the partition rows (rows an element is in or not by a
/// rule of its own). Named rows of polygons (parts, materials, smoothing
/// groups) follow the fixed slots.
mod slot {
    pub const ALL: usize = 0;
    // Vertices.
    pub const V_EDGES: usize = 1; // .. +6
    pub const V_POLYS: usize = 7; // .. +6
    pub const V_STRAIGHT: usize = 13;
    pub const V_LEN: usize = 14;
    // Edges.
    pub const E_POLYS: usize = 1; // 1..=5 -> 1..=5
    pub const E_BORDER: usize = 6; // .. +5
    pub const E_LEN: usize = 11;
    // Polygons.
    pub const F_TYPE: usize = 1; // .. +8
    pub const F_CHECK: usize = 9; // .. +6
    pub const F_VERTS: usize = 15; // 1..=5 -> 15..=19
    pub const F_PART: usize = 20;
}

fn bucket(n: u32) -> usize {
    (n as usize).min(MORE as usize)
}

/// What one mesh's rows need, worked out once per refresh.
pub(crate) struct MeshEval<'a> {
    pub mesh: &'a Mesh,
    pub topo: Arc<Topology>,
    /// Elements counted (not hidden).
    v_ok: Flags,
    e_ok: Flags,
    f_ok: Flags,
    /// Per-edge border flags (bit per `Border`), when edges are open.
    e_border: Vec<u8>,
    /// Per-polygon check flags (bit per `Check`), when checks are open.
    f_check: Vec<u8>,
    v_straight: Vec<bool>,
    smoothing_of: Vec<u32>,
    smoothing_names: Vec<String>,
    need: Need,
}

/// Which of the hidden and selection flags are taken into account.
pub(crate) struct EvalOptions<'a> {
    pub hidden: &'a ElementMask,
    pub need: Need,
    /// Count hidden elements too (Unhide's members).
    pub include_hidden: bool,
}

impl<'a> MeshEval<'a> {
    pub(crate) fn new(mesh: &'a Mesh, o: &EvalOptions<'_>) -> Self {
        let topo = topology(mesh);
        let mut eval = MeshEval {
            mesh,
            topo,
            v_ok: None,
            e_ok: None,
            f_ok: None,
            e_border: Vec::new(),
            f_check: Vec::new(),
            v_straight: Vec::new(),
            smoothing_of: Vec::new(),
            smoothing_names: Vec::new(),
            need: o.need,
        };
        if !o.include_hidden && !o.hidden.is_empty() {
            eval.apply_hidden(o.hidden);
        }
        let labels = &mesh.attributes.smoothing_labels;
        if o.need.polygons && labels.iter().any(Option::is_some) {
            (eval.smoothing_of, eval.smoothing_names) =
                intern(labels.iter().map(Option::as_deref), labels.len());
        }
        // Unit polygon normals, once per refresh, for borders and checks.
        let normals = if o.need.edges || o.need.checks {
            face_normals(mesh)
        } else {
            Vec::new()
        };
        if o.need.edges {
            eval.e_border = edge_borders(mesh, &eval.topo, &normals);
        }
        if o.need.vertices {
            eval.v_straight = straight_points(mesh, &eval.topo);
        }
        if o.need.checks {
            eval.f_check = polygon_checks(mesh, &eval.topo, &eval.e_border, &normals);
        } else if o.need.polygons {
            // Single point reads only topology.
            eval.f_check = mesh
                .faces
                .iter()
                .map(|f| u8::from(f.vertices.len() == 1))
                .collect();
        }
        eval
    }

    fn nv(&self) -> usize {
        self.mesh.vertices.len()
    }
    fn nf(&self) -> usize {
        self.mesh.faces.len()
    }
    fn ne(&self) -> usize {
        self.topo.edge_count()
    }

    /// Q3: hidden elements are left out. Hiding reaches up as the views
    /// draw it (element_state.rs): a hidden point hides its edges and
    /// polygons, a hidden edge its polygons; a point or edge whose every
    /// polygon is hidden is hidden too.
    fn apply_hidden(&mut self, hidden: &ElementMask) {
        let t = &self.topo;
        let (nv, nf, ne) = (self.nv(), self.nf(), self.ne());
        let mut vh = vec![false; nv];
        for &v in &hidden.vertices {
            if let Some(h) = vh.get_mut(v as usize) {
                *h = true;
            }
        }
        let mut eh = vec![false; ne];
        for &(a, b) in &hidden.edges {
            if let Some(e) = t.edge_index(a, b) {
                eh[e] = true;
            }
        }
        let mut fh = vec![false; nf];
        for &f in &hidden.faces {
            if let Some(h) = fh.get_mut(f as usize) {
                *h = true;
            }
        }
        for (f, face) in self.mesh.faces.iter().enumerate() {
            if face.vertices.iter().any(|&v| vh[v as usize]) {
                fh[f] = true;
            }
        }
        if !hidden.edges.is_empty() {
            for (e, hidden) in eh.iter().enumerate() {
                if *hidden {
                    for &f in t.faces_of_edge(e) {
                        fh[f as usize] = true;
                    }
                }
            }
        }
        // Points and edges of hidden polygons only.
        let mut v_shown = vec![false; nv];
        for (f, face) in self.mesh.faces.iter().enumerate() {
            if !fh[f] {
                for &v in &face.vertices {
                    v_shown[v as usize] = true;
                }
            }
        }
        for v in 0..nv {
            if t.v_polys[v] > 0 && !v_shown[v] {
                vh[v] = true;
            }
        }
        for (e, (a, b)) in t.edge_pairs().into_iter().enumerate() {
            let users = t.faces_of_edge(e);
            if vh[a as usize]
                || vh[b as usize]
                || (!users.is_empty() && users.iter().all(|&f| fh[f as usize]))
            {
                eh[e] = true;
            }
        }
        self.v_ok = Some(vh.into_iter().map(|h| !h).collect());
        self.e_ok = Some(eh.into_iter().map(|h| !h).collect());
        self.f_ok = Some(fh.into_iter().map(|h| !h).collect());
    }

    pub(crate) fn vertex_ok(&self, v: usize) -> bool {
        flag(&self.v_ok, v)
    }
    pub(crate) fn edge_ok(&self, e: usize) -> bool {
        flag(&self.e_ok, e)
    }
    pub(crate) fn face_ok(&self, f: usize) -> bool {
        flag(&self.f_ok, f)
    }

    /// The partition slots point `v` is in.
    pub(crate) fn vertex_slots(&self, v: usize, push: &mut impl FnMut(usize)) {
        push(slot::ALL);
        if !self.need.vertices {
            return;
        }
        let t = &self.topo;
        push(slot::V_EDGES + bucket(t.v_edges[v]));
        push(slot::V_POLYS + bucket(t.v_polys[v]));
        if self.v_straight.get(v).copied().unwrap_or(false) {
            push(slot::V_STRAIGHT);
        }
    }
    pub(crate) fn edge_slots(&self, e: usize, push: &mut impl FnMut(usize)) {
        push(slot::ALL);
        if !self.need.edges {
            return;
        }
        let s = self.topo.edge_surface[e];
        if s >= 1 {
            push(slot::E_POLYS + bucket(s) - 1);
        }
        let bits = self.e_border.get(e).copied().unwrap_or(0);
        for (i, _) in Border::ALL.iter().enumerate() {
            if bits & (1 << i) != 0 {
                push(slot::E_BORDER + i);
            }
        }
    }
    fn face_slot_count(&self) -> usize {
        slot::F_PART
            + self.topo.part_names.len()
            + self.topo.material_names.len()
            + self.smoothing_names.len()
    }
    pub(crate) fn face_slots(&self, f: usize, push: &mut impl FnMut(usize)) {
        push(slot::ALL);
        if !self.need.polygons {
            return;
        }
        let t = &self.topo;
        push(slot::F_TYPE + PolyType::of(self.mesh.attributes.representations.get(f)).index());
        let bits = self.f_check.get(f).copied().unwrap_or(0);
        for (i, _) in Check::ALL.iter().enumerate() {
            if bits & (1 << i) != 0 {
                push(slot::F_CHECK + i);
            }
        }
        let n = self.mesh.faces[f].vertices.len() as u32;
        if n >= 1 {
            push(slot::F_VERTS + bucket(n) - 1);
        }
        if t.part_of[f] != NONE {
            push(slot::F_PART + t.part_of[f] as usize);
        }
        let mat = slot::F_PART + t.part_names.len();
        if t.material_of[f] != NONE {
            push(mat + t.material_of[f] as usize);
        }
        if let Some(&s) = self.smoothing_of.get(f)
            && s != NONE
        {
            push(mat + t.material_names.len() + s as usize);
        }
    }

    /// The partition slot of `row`, if it is one (named rows of polygons
    /// included); `None` for member-list rows (sets, maps).
    pub(crate) fn slot_of(&self, row: &Row) -> Option<usize> {
        let t = &self.topo;
        let mat = slot::F_PART + t.part_names.len();
        let smooth = mat + t.material_names.len();
        Some(match row {
            Row::All(_) => slot::ALL,
            Row::VertexEdges(n) => slot::V_EDGES + usize::from(*n),
            Row::VertexPolygons(n) => slot::V_POLYS + usize::from(*n),
            Row::VertexStraight => slot::V_STRAIGHT,
            Row::EdgePolygons(n) => slot::E_POLYS + usize::from(*n) - 1,
            Row::EdgeBorder(b) => slot::E_BORDER + Border::ALL.iter().position(|x| x == b)?,
            Row::PolygonType(ty) => slot::F_TYPE + ty.index(),
            Row::PolygonCheck(c) => slot::F_CHECK + Check::ALL.iter().position(|x| x == c)?,
            Row::PolygonVertices(n) => slot::F_VERTS + usize::from(*n) - 1,
            Row::Part(n) => slot::F_PART + t.part_names.iter().position(|x| x == n)?,
            Row::Material(n) => mat + t.material_names.iter().position(|x| x == n)?,
            Row::Smoothing(n) => smooth + self.smoothing_names.iter().position(|x| x == n)?,
            _ => return None,
        })
    }

    /// The members of a member-list row (selection sets, vertex maps, edge
    /// weights), unfiltered, as point, edge or polygon indices.
    pub(crate) fn listed(&self, row: &Row) -> Option<Vec<usize>> {
        let a = &self.mesh.attributes;
        Some(match row {
            Row::VertexSet(n) => a
                .vertex_sets
                .get(n)
                .map(|s| s.iter().map(|&v| v as usize).collect())
                .unwrap_or_default(),
            Row::PolygonSet(n) => a
                .polygon_sets
                .get(n)
                .map(|s| s.iter().map(|&v| v as usize).collect())
                .unwrap_or_default(),
            Row::EdgeSet(n) => a
                .edge_sets
                .get(n)
                .map(|s| {
                    s.iter()
                        .filter_map(|&(x, y)| self.topo.edge_index(x, y))
                        .collect()
                })
                .unwrap_or_default(),
            Row::EdgeWeight => {
                let mut v: Vec<usize> = a
                    .creases
                    .iter()
                    .filter(|c| c.weight != 0.)
                    .filter_map(|c| self.topo.edge_index(c.a, c.b))
                    .collect();
                v.sort_unstable();
                v.dedup();
                v
            }
            Row::VertexMap(kind, name) => map_points(self.mesh, *kind, name)
                .into_iter()
                .map(|v| v as usize)
                .collect(),
            _ => return None,
        })
    }

    /// Every element of `row`'s kind in it (counted elements only).
    pub(crate) fn members(&self, row: &Row) -> Vec<usize> {
        let ok = |i: usize| match row.kind() {
            Kind::Vertex => self.vertex_ok(i),
            Kind::Edge => self.edge_ok(i),
            _ => self.face_ok(i),
        };
        if let Some(list) = self.listed(row) {
            return list.into_iter().filter(|&i| ok(i)).collect();
        }
        let Some(want) = self.slot_of(row) else {
            return Vec::new();
        };
        let n = match row.kind() {
            Kind::Vertex => self.nv(),
            Kind::Edge => self.ne(),
            _ => self.nf(),
        };
        let chunks = par_ranges(n, |range| {
            let mut out = Vec::new();
            for i in range {
                if !ok(i) {
                    continue;
                }
                let mut hit = false;
                let mut push = |s: usize| hit |= s == want;
                match row.kind() {
                    Kind::Vertex => self.vertex_slots(i, &mut push),
                    Kind::Edge => self.edge_slots(i, &mut push),
                    _ => self.face_slots(i, &mut push),
                }
                if hit {
                    out.push(i);
                }
            }
            out
        });
        chunks.concat()
    }

    /// Edge index -> its two points.
    pub(crate) fn edge_points(&self, e: usize) -> (u32, u32) {
        self.topo.edge(e)
    }
}

/// The points a vertex map has a value at.
pub fn map_points(mesh: &Mesh, kind: MapKind, name: &str) -> BTreeSet<u32> {
    let a = &mesh.attributes;
    let mut out = BTreeSet::new();
    let corners = |out: &mut BTreeSet<u32>, has: &dyn Fn(usize, usize) -> bool| {
        for (f, face) in mesh.faces.iter().enumerate() {
            for (c, &v) in face.vertices.iter().enumerate() {
                if has(f, c) {
                    out.insert(v);
                }
            }
        }
    };
    match kind {
        MapKind::Weight => {
            if let Some(m) = a.point_weights.get(name) {
                out.extend(m.keys());
            }
            if let Some(m) = a.corner_weights.get(name) {
                corners(&mut out, &|f, c| {
                    m.get(f).and_then(|x| x.get(c)).is_some_and(Option::is_some)
                });
            }
        }
        MapKind::Color => {
            if let Some(m) = a.point_colors.get(name) {
                out.extend(m.keys());
            }
            if let Some(m) = a.corner_colors.get(name) {
                corners(&mut out, &|f, c| {
                    m.get(f).and_then(|x| x.get(c)).is_some_and(Option::is_some)
                });
            }
        }
        MapKind::Uv => {
            if let Some(m) = mesh.uv_maps.get(name) {
                corners(&mut out, &|f, c| {
                    m.get(f).and_then(|x| x.get(c)).is_some_and(Option::is_some)
                });
            }
            if let Some(m) = a.point_uvs.get(name) {
                out.extend(m.keys());
            }
        }
        MapKind::Morph => {
            if let Some(m) = a.relative_morphs.get(name) {
                out.extend(m.keys());
            }
        }
        MapKind::Normal => {
            if let Some(m) = a.corner_normals.get(name) {
                corners(&mut out, &|f, c| {
                    m.get(f).and_then(|x| x.get(c)).is_some_and(Option::is_some)
                });
            }
        }
        MapKind::Other => {}
    }
    if let Some(m) = a.typed_maps.get(name)
        && MapKind::of_typed(&m.kind) == kind
    {
        out.extend(m.points.keys());
        corners(&mut out, &|f, c| {
            m.corners
                .get(f)
                .and_then(|x| x.get(c))
                .is_some_and(Option::is_some)
        });
    }
    out.retain(|&v| (v as usize) < mesh.vertices.len());
    out
}

/// Every vertex map of `mesh`, by kind and name.
pub fn map_names(mesh: &Mesh) -> BTreeSet<(MapKind, String)> {
    let a = &mesh.attributes;
    let mut out = BTreeSet::new();
    let mut add = |k: MapKind, names: &mut dyn Iterator<Item = &String>| {
        for n in names {
            out.insert((k, n.clone()));
        }
    };
    add(
        MapKind::Weight,
        &mut a.point_weights.keys().chain(a.corner_weights.keys()),
    );
    add(
        MapKind::Color,
        &mut a.point_colors.keys().chain(a.corner_colors.keys()),
    );
    add(MapKind::Uv, &mut mesh.uv_maps.keys());
    add(MapKind::Morph, &mut a.relative_morphs.keys());
    add(MapKind::Normal, &mut a.corner_normals.keys());
    for (n, m) in &a.typed_maps {
        out.insert((MapKind::of_typed(&m.kind), n.clone()));
    }
    out
}

/// Newell normal of a polygon (zero for a degenerate one).
fn newell(mesh: &Mesh, vs: &[u32]) -> DVec3 {
    let n = vs.len();
    let mut sum = DVec3::ZERO;
    for i in 0..n {
        let p = mesh.vertices[vs[i] as usize];
        let q = mesh.vertices[vs[(i + 1) % n] as usize];
        sum += p.cross(q);
    }
    sum
}

/// Every polygon's unit Newell normal (zero for lines and degenerate ones).
fn face_normals(mesh: &Mesh) -> Vec<DVec3> {
    let reps = &mesh.attributes.representations;
    par_ranges(mesh.faces.len(), |range| {
        range
            .map(|f| {
                if is_line(reps.get(f)) {
                    DVec3::ZERO
                } else {
                    newell(mesh, &mesh.faces[f].vertices).normalize_or_zero()
                }
            })
            .collect::<Vec<_>>()
    })
    .concat()
}

/// Border bits per edge (bit i = `Border::ALL[i]`).
fn edge_borders(mesh: &Mesh, t: &Topology, normals: &[DVec3]) -> Vec<u8> {
    let ne = t.edge_count();
    let uv_name = (!mesh.uv_maps.is_empty())
        .then(|| crate::uv_tools::target_uv_map(mesh, None))
        .filter(|n| mesh.uv_maps.contains_key(n));
    let uv = uv_name.as_ref().and_then(|n| mesh.uv_maps.get(n));
    let reps = &mesh.attributes.representations;
    let cos = COPLANAR_DEGREES.to_radians().cos();
    let chunks = par_ranges(ne, |range| {
        let mut out = Vec::with_capacity(range.len());
        let mut surface: Vec<u32> = Vec::new();
        for e in range {
            surface.clear();
            surface.extend(
                t.faces_of_edge(e)
                    .iter()
                    .copied()
                    .filter(|&f| !is_line(reps.get(f as usize))),
            );
            let mut bits = 0u8;
            if surface.len() == 1 {
                bits |= 1 << 2; // Geometry: an open border.
            }
            if surface.len() >= 2 {
                let first = surface[0] as usize;
                if surface
                    .iter()
                    .any(|&f| t.part_of[f as usize] != t.part_of[first])
                {
                    bits |= 1 << 0;
                }
                if surface
                    .iter()
                    .any(|&f| t.material_of[f as usize] != t.material_of[first])
                {
                    bits |= 1 << 1;
                }
            }
            if surface.len() == 2 {
                let (a, b) = t.edge(e);
                let (f, g) = (surface[0] as usize, surface[1] as usize);
                if let Some(uv) = uv {
                    let at = |face: usize, p: u32| -> Option<glam::DVec2> {
                        let c = mesh.faces[face].vertices.iter().position(|&v| v == p)?;
                        uv.get(face)?.get(c).copied().flatten()
                    };
                    if at(f, a) != at(g, a) || at(f, b) != at(g, b) {
                        bits |= 1 << 3;
                    }
                }
                let (nf, ng) = (normals[f], normals[g]);
                if nf != DVec3::ZERO && ng != DVec3::ZERO && nf.dot(ng) >= cos {
                    bits |= 1 << 4;
                }
            }
            out.push(bits);
        }
        out
    });
    chunks.concat()
}

/// Points with exactly two edges in a straight line (spec §4.1).
fn straight_points(mesh: &Mesh, t: &Topology) -> Vec<bool> {
    let mut out = vec![false; mesh.vertices.len()];
    let cos = (180f64 - STRAIGHT_DEGREES).to_radians().cos();
    for &[v, a, b] in &t.two_edge {
        let p = mesh.vertices[v as usize];
        let da = (mesh.vertices[a as usize] - p).normalize_or_zero();
        let db = (mesh.vertices[b as usize] - p).normalize_or_zero();
        if da != DVec3::ZERO && db != DVec3::ZERO && da.dot(db) <= cos {
            out[v as usize] = true;
        }
    }
    out
}

/// Whether a polygon is flat by the Flatten tools' own rule (Q5):
/// flatten_ngons.rs PF-08 v2, the plane through the centre of its bounding
/// box with the Newell normal, every point within 1e-12 of the extent.
fn is_flat(points: &[DVec3]) -> bool {
    if points.len() < 4 {
        return true;
    }
    let Some(normal) = crate::flatten_ngons::flatten_normal(points) else {
        return true;
    };
    let (lo, hi) = points.iter().fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |(lo, hi), &p| (lo.min(p), hi.max(p)),
    );
    let centre = (lo + hi) * 0.5;
    let extent = (hi - lo).max_element();
    points.iter().all(|&p| {
        ((p - centre).dot(normal)).abs()
            <= crate::flatten_ngons::FLAT_TOLERANCE * extent.max(1e-300)
    })
}

/// Check bits per polygon (bit i = `Check::ALL[i]`).
fn polygon_checks(mesh: &Mesh, t: &Topology, borders: &[u8], normals: &[DVec3]) -> Vec<u8> {
    let nf = mesh.faces.len();
    let reps = &mesh.attributes.representations;
    let sharp = SHARP_DEGREES.to_radians().cos();
    let chunks = par_ranges(nf, |range| {
        let mut out = Vec::with_capacity(range.len());
        let mut points: Vec<DVec3> = Vec::new();
        for f in range {
            let vs = &mesh.faces[f].vertices;
            let mut bits = 0u8;
            if vs.len() == 1 {
                bits |= 1 << 0;
            }
            if is_line(reps.get(f)) || vs.len() < 3 {
                out.push(bits);
                continue;
            }
            points.clear();
            points.extend(vs.iter().map(|&v| mesh.vertices[v as usize]));
            if !is_flat(&points) {
                bits |= 1 << 1;
            }
            let normal = normals[f];
            let n = points.len();
            for i in 0..n {
                let p = points[i];
                let a = points[(i + n - 1) % n] - p;
                let b = points[(i + 1) % n] - p;
                let (la, lb) = (a.length(), b.length());
                if la > 0. && lb > 0. {
                    // A corner under 1°: the two sides point (nearly) the
                    // same way.
                    if a.dot(b) / (la * lb) >= sharp {
                        bits |= 1 << 4;
                    }
                    // An inward corner: its turn goes against the polygon's
                    // facing (Q6: "Concave").
                    let turn = (p - points[(i + n - 1) % n]).cross(points[(i + 1) % n] - p);
                    if normal != DVec3::ZERO && turn.dot(normal) < -1e-12 * la * lb {
                        bits |= 1 << 5;
                    }
                }
            }
            out.push(bits);
        }
        out
    });
    let mut bits = chunks.concat();
    // Flat with a neighbour: a Flat edge's polygons.
    for (e, &b) in borders.iter().enumerate() {
        if b & (1 << 4) != 0 {
            for &f in t.faces_of_edge(e) {
                if !is_line(reps.get(f as usize)) {
                    bits[f as usize] |= 1 << 3;
                }
            }
        }
    }
    if borders.is_empty() {
        // Edges are closed: work the coplanar pairs out here.
        for (e, b) in edge_borders(mesh, t, normals).into_iter().enumerate() {
            if b & (1 << 4) != 0 {
                for &f in t.faces_of_edge(e) {
                    if !is_line(reps.get(f as usize)) {
                        bits[f as usize] |= 1 << 3;
                    }
                }
            }
        }
    }
    // Overlapping: the same point positions as another polygon. Polygons
    // are keyed by a hash of their sorted positions and split into buckets
    // by it, so each core sorts and compares its own buckets; only equal
    // keys are compared point by point.
    const BUCKET_BITS: u32 = 6;
    let buckets = 1usize << BUCKET_BITS;
    let sorted_points = |f: usize, out: &mut Vec<[u64; 3]>| {
        out.clear();
        out.extend(mesh.faces[f].vertices.iter().map(|&v| {
            let p = mesh.vertices[v as usize];
            [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
        }));
        out.sort_unstable();
    };
    let keyed = par_ranges(nf, |range| {
        let mut out: Vec<Vec<(u64, u32)>> = vec![Vec::new(); buckets];
        let mut pos: Vec<[u64; 3]> = Vec::new();
        for f in range {
            if is_line(reps.get(f)) || mesh.faces[f].vertices.is_empty() {
                continue;
            }
            sorted_points(f, &mut pos);
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for p in &pos {
                for x in p {
                    h = (h ^ x).wrapping_mul(0x0100_0000_01b3);
                    h ^= h >> 29;
                }
            }
            out[(h >> (64 - BUCKET_BITS)) as usize].push((h, f as u32));
        }
        out
    });
    let overlapping = par_ranges_in(buckets, parts_for(nf), |range| {
        let mut hits = Vec::new();
        let (mut p, mut q) = (Vec::new(), Vec::new());
        for b in range {
            let mut keys: Vec<(u64, u32)> =
                keyed.iter().flat_map(|k| k[b].iter().copied()).collect();
            keys.sort_unstable();
            let mut i = 0;
            while i < keys.len() {
                let mut j = i + 1;
                while j < keys.len() && keys[j].0 == keys[i].0 {
                    j += 1;
                }
                if j - i > 1 {
                    for x in i..j {
                        sorted_points(keys[x].1 as usize, &mut p);
                        let twin = (i..j).any(|y| {
                            y != x && {
                                sorted_points(keys[y].1 as usize, &mut q);
                                p == q
                            }
                        });
                        if twin {
                            hits.push(keys[x].1);
                        }
                    }
                }
                i = j;
            }
        }
        hits
    });
    for f in overlapping.into_iter().flatten() {
        bits[f as usize] |= 1 << 2;
    }
    bits
}

// ------------------------------------------------------------ counting

/// Counts `n` elements into `slots` partition slots: for each element
/// that is counted, every slot it is in gets one, and one more selected
/// when it is selected.
fn tally(
    n: usize,
    slots: usize,
    ok: impl Fn(usize) -> bool + Sync,
    selected: &[bool],
    classify: impl Fn(usize, &mut dyn FnMut(usize)) + Sync,
) -> Vec<Tally> {
    let parts = par_ranges(n, |range| {
        let mut out = vec![Tally::default(); slots];
        for i in range {
            if !ok(i) {
                continue;
            }
            let sel = selected.get(i).copied().unwrap_or(false);
            classify(i, &mut |s| {
                if let Some(t) = out.get_mut(s) {
                    t.count += 1;
                    t.selected += u64::from(sel);
                }
            });
        }
        out
    });
    let mut total = vec![Tally::default(); slots];
    for p in parts {
        for (t, x) in total.iter_mut().zip(p) {
            t.count += x.count;
            t.selected += x.selected;
        }
    }
    total
}

fn add(rows: &mut HashMap<Row, Tally>, row: Row, t: Tally) {
    let e = rows.entry(row).or_default();
    e.count += t.count;
    e.selected += t.selected;
}

/// Counts one mesh into `report`.
fn count_mesh(input: &MeshInput, need: Need, report: &mut Report) {
    let mesh: &Mesh = &input.mesh;
    let eval = MeshEval::new(
        mesh,
        &EvalOptions {
            hidden: &input.hidden,
            need,
            include_hidden: false,
        },
    );
    let t = eval.topo.clone();
    let (nv, ne, nf) = (eval.nv(), eval.ne(), eval.nf());
    let mut v_sel = vec![false; nv];
    for &v in &input.selected.vertices {
        if let Some(s) = v_sel.get_mut(v as usize) {
            *s = true;
        }
    }
    let mut e_sel = vec![false; ne];
    for &(a, b) in &input.selected.edges {
        if let Some(e) = t.edge_index(a, b) {
            e_sel[e] = true;
        }
    }
    let mut f_sel = vec![false; nf];
    for &f in &input.selected.faces {
        if let Some(s) = f_sel.get_mut(f as usize) {
            *s = true;
        }
    }
    let rows = &mut report.rows;
    // Vertices.
    let v = tally(
        nv,
        slot::V_LEN,
        |i| eval.vertex_ok(i),
        &v_sel,
        |i, push| eval.vertex_slots(i, &mut |s| push(s)),
    );
    add(rows, Row::All(Kind::Vertex), v[slot::ALL]);
    if need.vertices {
        for n in 0..=MORE {
            add(rows, Row::VertexEdges(n), v[slot::V_EDGES + usize::from(n)]);
            add(
                rows,
                Row::VertexPolygons(n),
                v[slot::V_POLYS + usize::from(n)],
            );
        }
        add(rows, Row::VertexStraight, v[slot::V_STRAIGHT]);
    }
    // Edges.
    let e = tally(
        ne,
        slot::E_LEN,
        |i| eval.edge_ok(i),
        &e_sel,
        |i, push| eval.edge_slots(i, &mut |s| push(s)),
    );
    add(rows, Row::All(Kind::Edge), e[slot::ALL]);
    if need.edges {
        for n in 1..=MORE {
            add(
                rows,
                Row::EdgePolygons(n),
                e[slot::E_POLYS + usize::from(n) - 1],
            );
        }
        for (i, b) in Border::ALL.iter().enumerate() {
            add(rows, Row::EdgeBorder(*b), e[slot::E_BORDER + i]);
        }
    }
    // Polygons.
    let f = tally(
        nf,
        eval.face_slot_count(),
        |i| eval.face_ok(i),
        &f_sel,
        |i, push| eval.face_slots(i, &mut |s| push(s)),
    );
    add(rows, Row::All(Kind::Polygon), f[slot::ALL]);
    if need.polygons {
        for ty in PolyType::ALL {
            add(rows, Row::PolygonType(ty), f[slot::F_TYPE + ty.index()]);
        }
        for (i, c) in Check::ALL.iter().enumerate() {
            if need.checks || !c.expensive() {
                add(rows, Row::PolygonCheck(*c), f[slot::F_CHECK + i]);
            }
        }
        for n in 1..=MORE {
            add(
                rows,
                Row::PolygonVertices(n),
                f[slot::F_VERTS + usize::from(n) - 1],
            );
        }
        for (i, n) in t.part_names.iter().enumerate() {
            add(rows, Row::Part(n.clone()), f[slot::F_PART + i]);
        }
        let mat = slot::F_PART + t.part_names.len();
        for (i, n) in t.material_names.iter().enumerate() {
            add(rows, Row::Material(n.clone()), f[mat + i]);
        }
        let smooth = mat + t.material_names.len();
        for (i, n) in eval.smoothing_names.iter().enumerate() {
            add(rows, Row::Smoothing(n.clone()), f[smooth + i]);
        }
    }
    // Member-list rows: sets, maps, edge weights.
    let mut listed = Vec::new();
    let a = &mesh.attributes;
    if need.vertices {
        listed.extend(
            crate::selection_sets::set_names(mesh, crate::selection_sets::SetKind::Vertex)
                .into_iter()
                .map(Row::VertexSet),
        );
        listed.extend(
            map_names(mesh)
                .into_iter()
                .map(|(k, n)| Row::VertexMap(k, n)),
        );
    }
    if need.edges {
        listed.extend(
            crate::selection_sets::set_names(mesh, crate::selection_sets::SetKind::Edge)
                .into_iter()
                .map(Row::EdgeSet),
        );
        if a.creases.iter().any(|c| c.weight != 0.) {
            listed.push(Row::EdgeWeight);
        }
    }
    if need.polygons {
        listed.extend(
            crate::selection_sets::set_names(mesh, crate::selection_sets::SetKind::Polygon)
                .into_iter()
                .map(Row::PolygonSet),
        );
    }
    for row in listed {
        let members = eval.listed(&row).unwrap_or_default();
        let (ok, sel): (&dyn Fn(usize) -> bool, &[bool]) = match row.kind() {
            Kind::Vertex => (&|i| eval.vertex_ok(i), &v_sel),
            Kind::Edge => (&|i| eval.edge_ok(i), &e_sel),
            _ => (&|i| eval.face_ok(i), &f_sel),
        };
        let mut t = Tally::default();
        for i in members.into_iter().filter(|&i| ok(i)) {
            t.count += 1;
            t.selected += u64::from(sel.get(i).copied().unwrap_or(false));
        }
        add(rows, row, t);
    }
    // Names (even of rows not counted, so a collapsed group still knows
    // whether it has anything).
    let names = &mut report.names;
    names.parts.extend(t.part_names.iter().cloned());
    names.materials.extend(t.material_names.iter().cloned());
    names.smoothing.extend(eval.smoothing_names.iter().cloned());
    names.vertex_sets.extend(crate::selection_sets::set_names(
        mesh,
        crate::selection_sets::SetKind::Vertex,
    ));
    names.edge_sets.extend(crate::selection_sets::set_names(
        mesh,
        crate::selection_sets::SetKind::Edge,
    ));
    names.polygon_sets.extend(crate::selection_sets::set_names(
        mesh,
        crate::selection_sets::SetKind::Polygon,
    ));
    names.vertex_maps.extend(map_names(mesh));
    names.edge_weights |= a.creases.iter().any(|c| c.weight != 0.);
}

fn sort_dedup<T: Ord>(v: &mut Vec<T>) {
    v.sort();
    v.dedup();
}

/// One refresh: every open group's rows, for every counted mesh.
pub fn compute(input: &Input) -> Report {
    let mut report = Report {
        need: Some(input.need),
        meshes: input.meshes.len(),
        ..Default::default()
    };
    for m in &input.meshes {
        count_mesh(m, input.need, &mut report);
    }
    // Every top row exists, even with no mesh (spec §10: 0, dimmed).
    for k in [Kind::Vertex, Kind::Edge, Kind::Polygon] {
        report.rows.entry(Row::All(k)).or_default();
    }
    // Items (spec §4.4).
    let mut by_type: BTreeMap<ItemType, Tally> = BTreeMap::new();
    let mut all = Tally::default();
    for i in &input.items {
        let t = by_type.entry(i.kind).or_default();
        t.count += 1;
        t.selected += u64::from(i.selected);
        all.count += 1;
        all.selected += u64::from(i.selected);
    }
    report.rows.insert(Row::All(Kind::Item), all);
    let selected: BTreeSet<u64> = input
        .items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.id)
        .collect();
    let exists: BTreeSet<u64> = input.items.iter().map(|i| i.id).collect();
    for (id, name, members) in &input.groups {
        let members: BTreeSet<u64> = members
            .iter()
            .copied()
            .filter(|m| exists.contains(m))
            .collect();
        report.rows.insert(
            Row::ItemGroup(*id),
            Tally {
                count: members.len() as u64,
                selected: members.intersection(&selected).count() as u64,
            },
        );
        report.names.item_groups.push((*id, name.clone()));
    }
    report.names.item_types = by_type.keys().copied().collect();
    for (ty, t) in by_type {
        report.rows.insert(Row::ItemType(ty), t);
    }
    let n = &mut report.names;
    sort_dedup(&mut n.parts);
    sort_dedup(&mut n.materials);
    sort_dedup(&mut n.smoothing);
    sort_dedup(&mut n.vertex_sets);
    sort_dedup(&mut n.edge_sets);
    sort_dedup(&mut n.polygon_sets);
    sort_dedup(&mut n.vertex_maps);
    // A named row exists on every counted mesh's list, even where one mesh
    // lacks it: give the open groups' missing ones a zero.
    let need = input.need;
    let mut zero = |row: Row| {
        report.rows.entry(row).or_default();
    };
    if need.polygons {
        for p in &n.parts {
            zero(Row::Part(p.clone()));
        }
        for m in &n.materials {
            zero(Row::Material(m.clone()));
        }
        for s in &n.smoothing {
            zero(Row::Smoothing(s.clone()));
        }
        for s in &n.polygon_sets {
            zero(Row::PolygonSet(s.clone()));
        }
    }
    if need.vertices {
        for s in &n.vertex_sets {
            zero(Row::VertexSet(s.clone()));
        }
        for (k, m) in &n.vertex_maps {
            zero(Row::VertexMap(*k, m.clone()));
        }
    }
    if need.edges {
        for s in &n.edge_sets {
            zero(Row::EdgeSet(s.clone()));
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(vertices: Vec<u32>) -> crate::Face {
        crate::Face {
            vertices,
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        }
    }

    #[test]
    fn row_names_round_trip() {
        let rows = [
            Row::All(Kind::Polygon),
            Row::VertexEdges(0),
            Row::VertexEdges(MORE),
            Row::VertexPolygons(3),
            Row::VertexStraight,
            Row::VertexMap(MapKind::Weight, "Mask: a".into()),
            Row::VertexSet("Front".into()),
            Row::EdgePolygons(1),
            Row::EdgeBorder(Border::Flat),
            Row::EdgeWeight,
            Row::EdgeSet("Loop".into()),
            Row::PolygonType(PolyType::CatmullClark),
            Row::PolygonCheck(Check::NotFlat),
            Row::PolygonVertices(MORE),
            Row::Part("Wheel".into()),
            Row::Material("Glass".into()),
            Row::PolygonSet("Top".into()),
            Row::Smoothing("Hard".into()),
            Row::ItemType(ItemType::Camera),
            Row::ItemGroup(42),
        ];
        for row in rows {
            let text = row.to_string();
            assert_eq!(text.parse::<Row>().unwrap(), row, "{text}");
        }
        assert!("polygons.vertices:0".parse::<Row>().is_err());
        assert!("edges.polygons:0".parse::<Row>().is_err());
        assert!("nonsense".parse::<Row>().is_err());
    }

    #[test]
    fn topology_lists_every_edge_once_with_its_polygons() {
        let m = Mesh {
            vertices: (0..6).map(|i| DVec3::new(i as f64, 0., 0.)).collect(),
            faces: vec![face(vec![0, 1, 2, 3]), face(vec![1, 4, 2]), face(vec![5])].into(),
            ..Default::default()
        };
        let t = build_topology(&m);
        let mut edges: Vec<_> = (0..t.edge_count()).map(|e| t.edge(e)).collect();
        edges.sort();
        let mut want = m.edges();
        want.retain(|(a, b)| a != b);
        assert_eq!(edges, want);
        assert_eq!(t.faces_of_edge(t.edge_index(1, 2).unwrap()), &[0, 1]);
        assert_eq!(t.v_edges[5], 0, "a one-point polygon has no edge");
        assert_eq!(t.v_polys[5], 1);
        assert_eq!(t.v_edges[1], 3);
    }
}
