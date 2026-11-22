//! The UV tab's everyday one-shot commands (BEHAVIOR-UV-TOOLS §7.5–7.9,
//! §9.1–9.3, §11, §12; build plan stage 3): one [`UvOp`] per button, each
//! one undo step (UVT-07) on the active mesh's selected UV map (UVT-01).
//! 3D never moves (UVT-06); hidden polygons are never touched (UVT-02).
//!
//! - E7 rows that work on whole islands: Quick Align (UVT-54), Island Align
//!   (UVT-55), Orient Pieces (UVT-56), To Origin (UVT-57), Texel Density and
//!   Real World Size (UVT-58/59).
//! - E9 cut and join: Split (UVT-60), Sew (UVT-61), Move and Sew (UVT-62).
//! - E11 queries: Select Border (UVT-80), Select Error UVs (UVT-81), Select
//!   Unmapped (UVT-82).
//! - E12 values: Copy / Cut / Paste (UVT-90/91), Delete (UVT-93), Set Value
//!   (UVT-94), Rotate Polygons (UVT-95), Merge UV Maps (UVT-96).
//!
//! Operandi keeps UV seams as different UV values at one point (no separate
//! seam records), so a split is a move apart: the plain click's
//! [`SPLIT_PLAIN`] is too small to see but makes the sides separate UV
//! positions.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVT-Q06, UVT-Q07, UVT-Q10,
//! UVO-P01..P06): Quick Align turns about the edge's middle; Split's sizes;
//! Rotate Polygons gives each corner the next corner's UV; Sew's "selected
//! side" is the side whose island holds more selected polygons (the larger
//! island when that ties); Paste Connectivity walks both selections from
//! their first polygons; Error UVs' overlap test is between triangles of
//! different polygons.
use crate::topology::edge_key;
use crate::{Mesh, Mode, Result, Scene, Selection};
use glam::{DMat2, DVec2, DVec3};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// UVT-Q07 (build guesses): Split's distances in UV units.
pub const SPLIT_PLAIN: f64 = 1e-5;
pub const SPLIT_MEDIUM: f64 = 0.01;
pub const SPLIT_LARGE: f64 = 0.03;

/// Quick Align and Orient Pieces' direction choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AxisChoice {
    #[default]
    Auto,
    Horizontal,
    Vertical,
}

/// Island Align's choice on one axis (UVT-55).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Side {
    Min,
    Max,
    Center,
    #[default]
    Unchanged,
}

/// Texel Density panel's buttons (UVT §7.9).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TexelAction {
    /// Each selected island to `density` on its own.
    Individually,
    /// All selected islands together to `density`.
    Overall,
    /// All selected islands to their combined average.
    Normalize,
}

/// The Texel Density values (UVT §7.9).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TexelOptions {
    /// Texture resolution for the sums, pixels (1024 start, build guess).
    pub image_size: f64,
    /// Target pixels per metre.
    pub density: f64,
    /// Item scale counts when measuring 3D size.
    pub world_space: bool,
    /// Alt: islands are turned upright first (Orient Pieces Auto).
    pub upright: bool,
}
impl Default for TexelOptions {
    fn default() -> Self {
        Self {
            image_size: 1024.,
            density: 512.,
            world_space: true,
            upright: false,
        }
    }
}

/// Sew and Move and Sew's Connect To (UVT-61/62).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConnectTo {
    #[default]
    Average,
    /// The unselected side comes to the selected one.
    Selected,
    /// The selected side goes to the unselected one.
    Unselected,
}

/// Select Error UVs' tests (UVT-81).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ErrorTests {
    pub cross: bool,
    pub self_intersection: bool,
    pub flipped: bool,
    pub stacked: bool,
    pub udim_border: bool,
    pub zero_area: bool,
}
impl Default for ErrorTests {
    fn default() -> Self {
        Self {
            cross: true,
            self_intersection: true,
            flipped: true,
            stacked: true,
            udim_border: false,
            zero_area: true,
        }
    }
}

/// Paste's matching (UVT-91).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PasteMode {
    /// By topology from the first selected polygon.
    #[default]
    Connectivity,
    /// By the order the polygons were selected.
    Selection,
}

/// One UV button's run.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UvOp {
    QuickAlign(AxisChoice),
    IslandAlign {
        u: Side,
        v: Side,
    },
    Orient(AxisChoice),
    ToOrigin {
        udim: u32,
    },
    Texel(TexelAction, TexelOptions),
    /// Real World Size: one UV unit stands for `metres` (Apply Overall).
    RealWorldSize {
        metres: f64,
        world_space: bool,
    },
    Split {
        distance: f64,
    },
    Sew {
        connect: ConnectTo,
        selected_only: bool,
    },
    MoveSew {
        connect: ConnectTo,
        scale: bool,
    },
    SelectBorder,
    SelectErrors(ErrorTests),
    SelectUnmapped {
        invert: bool,
    },
    Copy,
    Cut,
    Paste(PasteMode),
    Delete,
    SetValue {
        u: Option<f64>,
        v: Option<f64>,
    },
    RotatePolygons,
    /// Into the first map; the others are deleted.
    MergeMaps(Vec<String>),
    /// UDIM Indicator's Select UVs in the Region (UVT-113): the polygons
    /// whose UVs lie wholly inside the tile.
    SelectTile {
        udim: u32,
    },
    /// UDIM Indicator's arrows: the selected islands one tile over.
    NudgeTile {
        du: i32,
        dv: i32,
    },
    /// One stroke of the Split and Sew brush (UVT-63): the mesh points
    /// brushed over are split (`sew` off) or sewn to their partners
    /// brushed too; the selection stays.
    Brush {
        points: Vec<u32>,
        sew: bool,
        distance: f64,
        connect: ConnectTo,
    },
}

impl UvOp {
    pub fn label(&self) -> &'static str {
        match self {
            Self::QuickAlign(_) => "Quick Align",
            Self::IslandAlign { .. } => "Island Align",
            Self::Orient(_) => "Orient Pieces",
            Self::ToOrigin { .. } => "To Origin",
            Self::Texel(TexelAction::Individually, _) => "Apply Texel Density Individually",
            Self::Texel(TexelAction::Overall, _) => "Apply Texel Density Overall",
            Self::Texel(TexelAction::Normalize, _) => "Normalize UVs",
            Self::RealWorldSize { .. } => "Real World Size",
            Self::Split { .. } => "Split UVs",
            Self::Sew { .. } => "Sew UVs",
            Self::MoveSew { .. } => "Move and Sew",
            Self::SelectBorder => "Select UV Border",
            Self::SelectErrors(_) => "Select Error UVs",
            Self::SelectUnmapped { .. } => "Select Unmapped",
            Self::Copy => "Copy UVs",
            Self::Cut => "Cut UVs",
            Self::Paste(_) => "Paste UVs",
            Self::Delete => "Delete UVs",
            Self::SetValue { .. } => "Set UV Value",
            Self::RotatePolygons => "Rotate Polygon UVs",
            Self::MergeMaps(_) => "Merge UV Maps",
            Self::SelectTile { .. } => "Select UVs in Tile",
            Self::NudgeTile { .. } => "Move UVs a Tile",
            Self::Brush { sew: false, .. } => "Split Brush",
            Self::Brush { sew: true, .. } => "Sew Brush",
        }
    }
}

/// The UV clipboard (UVT-90): the copied polygons' corner counts and UVs,
/// in the order they were copied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UvClipboard {
    pub rows: Vec<Vec<DVec2>>,
    /// The copied polygons' neighbours (by row), for Connectivity pastes.
    pub source: Option<(Mesh, Vec<u32>)>,
}

type Rows = Vec<Vec<Option<DVec2>>>;

fn key(p: DVec2) -> (i64, i64) {
    let q = |x: f64| (x / 1e-6).round() as i64;
    (q(p.x), q(p.y))
}

/// A polygon's signed UV area (positive counter-clockwise).
fn signed_area(uv: &[DVec2]) -> f64 {
    (0..uv.len())
        .map(|i| uv[i].perp_dot(uv[(i + 1) % uv.len()]))
        .sum::<f64>()
        * 0.5
}

/// The UV state the commands read: usable polygons, UV positions and
/// islands.
struct Uvs<'a> {
    mesh: &'a Mesh,
    rows: &'a Rows,
    used: Vec<bool>,
    /// UV position id of each used corner.
    pos: Vec<Vec<usize>>,
    island_of_pos: Vec<usize>,
    islands: usize,
}

impl<'a> Uvs<'a> {
    fn new(mesh: &'a Mesh, rows: &'a Rows, hidden: &[bool]) -> Self {
        let used: Vec<bool> = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(f, face)| {
                !hidden.get(f).copied().unwrap_or(false)
                    && !mesh.is_polyline(f)
                    && rows.get(f).is_some_and(|r| {
                        r.len() == face.vertices.len() && r.iter().all(Option::is_some)
                    })
            })
            .collect();
        let mut ids: HashMap<(u32, (i64, i64)), usize> = HashMap::new();
        let mut pos = vec![Vec::new(); mesh.faces.len()];
        for (f, face) in mesh.faces.iter().enumerate() {
            if !used[f] {
                continue;
            }
            pos[f] = face
                .vertices
                .iter()
                .enumerate()
                .map(|(c, &v)| {
                    let n = ids.len();
                    *ids.entry((v, key(rows[f][c].unwrap()))).or_insert(n)
                })
                .collect();
        }
        let n = ids.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for ids in pos.iter().filter(|p| !p.is_empty()) {
            for w in 1..ids.len() {
                let (a, b) = (find(&mut parent, ids[0]), find(&mut parent, ids[w]));
                if a != b {
                    parent[a.max(b)] = a.min(b);
                }
            }
        }
        let mut roots = HashMap::new();
        let island_of_pos = (0..n)
            .map(|x| {
                let r = find(&mut parent, x);
                let k = roots.len();
                *roots.entry(r).or_insert(k)
            })
            .collect();
        Self {
            mesh,
            rows,
            used,
            pos,
            island_of_pos,
            islands: roots.len(),
        }
    }
    fn uv(&self, f: usize, c: usize) -> DVec2 {
        self.rows[f][c].unwrap()
    }
    fn island_of_face(&self, f: usize) -> Option<usize> {
        self.pos[f].first().map(|&p| self.island_of_pos[p])
    }
    /// Polygons of the selection (UVT-02): selected polygons, polygons
    /// touching selected points or edges; all usable ones when nothing is
    /// selected.
    fn selected_faces(&self, s: &Selection) -> Vec<usize> {
        let on_points: BTreeSet<u32> = s.vertices.iter().copied().collect();
        let on_edges: BTreeSet<(u32, u32)> = s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
        let picked: Vec<usize> = (0..self.mesh.faces.len())
            .filter(|&f| self.used[f])
            .filter(|&f| {
                let v = &self.mesh.faces[f].vertices;
                match s.mode {
                    Mode::Polygon => s.faces.contains(&(f as u32)),
                    Mode::Vertex => v.iter().any(|p| on_points.contains(p)),
                    Mode::Edge => (0..v.len())
                        .any(|i| on_edges.contains(&edge_key(v[i], v[(i + 1) % v.len()]))),
                    Mode::Item => false,
                }
            })
            .collect();
        let had = match s.mode {
            Mode::Polygon => !s.faces.is_empty(),
            Mode::Vertex => !s.vertices.is_empty(),
            Mode::Edge => !s.edges.is_empty(),
            Mode::Item => false,
        };
        if had {
            picked
        } else {
            (0..self.mesh.faces.len())
                .filter(|&f| self.used[f])
                .collect()
        }
    }
    /// The islands holding the selection, as sets of polygons.
    fn selected_islands(&self, s: &Selection) -> BTreeMap<usize, Vec<usize>> {
        let wanted: BTreeSet<usize> = self
            .selected_faces(s)
            .into_iter()
            .filter_map(|f| self.island_of_face(f))
            .collect();
        let mut out: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for f in 0..self.mesh.faces.len() {
            if let Some(i) = self.island_of_face(f).filter(|i| wanted.contains(i)) {
                out.entry(i).or_default().push(f);
            }
        }
        out
    }
    /// Each UV position of `faces` once, with its UV.
    fn positions(&self, faces: &[usize]) -> BTreeMap<usize, DVec2> {
        let mut out = BTreeMap::new();
        for &f in faces {
            for (c, &p) in self.pos[f].iter().enumerate() {
                out.entry(p).or_insert_with(|| self.uv(f, c));
            }
        }
        out
    }
}

fn bounds(pts: impl Iterator<Item = DVec2>) -> (DVec2, DVec2) {
    pts.fold((DVec2::MAX, DVec2::MIN), |(lo, hi), p| {
        (lo.min(p), hi.max(p))
    })
}

/// Writes `f(corner uv)` into every corner of `faces`.
fn transform_faces(out: &mut Rows, faces: &[usize], f: impl Fn(DVec2) -> DVec2) {
    for &face in faces {
        for c in out[face].iter_mut().flatten() {
            *c = f(*c);
        }
    }
}

/// The turn (radians) that lays `pts` in their smallest bounding box.
fn minimum_area_angle(pts: &[DVec2]) -> f64 {
    let hull = convex_hull(pts);
    let mut best = (f64::MAX, 0.);
    for i in 0..hull.len() {
        let d = hull[(i + 1) % hull.len()] - hull[i];
        if d.length() < 1e-12 {
            continue;
        }
        let a = -d.y.atan2(d.x);
        let rot = DMat2::from_angle(a);
        let (lo, hi) = bounds(hull.iter().map(|&p| rot * p));
        let area = (hi - lo).x * (hi - lo).y;
        if area < best.0 - 1e-15 {
            best = (area, a);
        }
    }
    best.1
}

fn convex_hull(pts: &[DVec2]) -> Vec<DVec2> {
    let mut p: Vec<DVec2> = pts.to_vec();
    p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    p.dedup_by(|a, b| (*a - *b).length() < 1e-12);
    if p.len() < 3 {
        return p;
    }
    let cross = |o: DVec2, a: DVec2, b: DVec2| (a - o).perp_dot(b - o);
    let mut lower: Vec<DVec2> = Vec::new();
    for &q in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], q) <= 0. {
            lower.pop();
        }
        lower.push(q);
    }
    let mut upper: Vec<DVec2> = Vec::new();
    for &q in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], q) <= 0. {
            upper.pop();
        }
        upper.push(q);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// 3D area of polygon `f` (world-scaled with `world`).
fn area_3d(mesh: &Mesh, f: usize, world: glam::DMat4) -> f64 {
    let v: Vec<DVec3> = mesh.faces[f]
        .vertices
        .iter()
        .map(|&p| world.transform_point3(mesh.vertices[p as usize]))
        .collect();
    let mut n = DVec3::ZERO;
    for i in 1..v.len().saturating_sub(1) {
        n += (v[i] - v[0]).cross(v[i + 1] - v[0]);
    }
    n.length() * 0.5
}

/// Texel density of `faces` (UVT-59): Image Size × sqrt(UV area ÷ 3D area).
pub fn density(mesh: &Mesh, rows: &Rows, faces: &[usize], image: f64, world: glam::DMat4) -> f64 {
    let (mut uv, mut d3) = (0., 0.);
    for &f in faces {
        let pts: Vec<DVec2> = rows[f].iter().flatten().copied().collect();
        uv += signed_area(&pts).abs();
        d3 += area_3d(mesh, f, world);
    }
    if d3 <= 1e-18 {
        return 0.;
    }
    image * (uv / d3).sqrt()
}

/// Split (UVT-60): each UV position the cut divides is pulled apart, each
/// side moving `distance / 2` into its own polygon. The cut is the selected
/// edges; a polygon selection's border; or, at selected points, every
/// edge (split in every direction).
fn split(u: &Uvs<'_>, s: &Selection, distance: f64, out: &mut Rows) -> usize {
    let mesh = u.mesh;
    let selected: BTreeSet<usize> = s.faces.iter().map(|&f| f as usize).collect();
    let cut_edges: BTreeSet<(u32, u32)> = s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    let cut_points: BTreeSet<u32> = s.vertices.iter().copied().collect();
    // Corners at each UV position.
    let mut at: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for f in 0..mesh.faces.len() {
        for (c, &p) in u.pos[f].iter().enumerate() {
            at.entry(p).or_default().push((f, c));
        }
    }
    let is_cut = |fa: usize, fb: usize, e: (u32, u32), point: u32| -> bool {
        match s.mode {
            Mode::Edge => cut_edges.contains(&e),
            Mode::Polygon => selected.contains(&fa) != selected.contains(&fb),
            Mode::Vertex => cut_points.contains(&point),
            Mode::Item => false,
        }
    };
    let edge_faces = mesh.edge_faces();
    let mut moved = 0;
    for corners in at.values() {
        if corners.len() < 2 {
            continue;
        }
        // Join corners of polygons that share a UV edge through this
        // position, unless that edge is cut.
        let n = corners.len();
        let mut group: Vec<usize> = (0..n).collect();
        fn find(g: &mut [usize], mut x: usize) -> usize {
            while g[x] != x {
                x = g[x];
            }
            x
        }
        for i in 0..n {
            for j in i + 1..n {
                let ((fa, ca), (fb, cb)) = (corners[i], corners[j]);
                let va = &mesh.faces[fa].vertices;
                let point = va[ca];
                let vb = &mesh.faces[fb].vertices;
                let la = va.len();
                let lb = vb.len();
                for na in [va[(ca + 1) % la], va[(ca + la - 1) % la]] {
                    for (k, &nb) in [vb[(cb + 1) % lb], vb[(cb + lb - 1) % lb]]
                        .iter()
                        .enumerate()
                    {
                        if na != nb {
                            continue;
                        }
                        let ka = va.iter().position(|&x| x == na).unwrap();
                        let kb = if k == 0 {
                            (cb + 1) % lb
                        } else {
                            (cb + lb - 1) % lb
                        };
                        let shared = key(u.uv(fa, ka)) == key(u.uv(fb, kb));
                        if shared && !is_cut(fa, fb, edge_key(point, na), point) {
                            let (x, y) = (find(&mut group, i), find(&mut group, j));
                            group[x.max(y)] = x.min(y);
                        }
                    }
                }
            }
        }
        let roots: BTreeSet<usize> = (0..n).map(|i| find(&mut group, i)).collect();
        if roots.len() < 2 {
            continue;
        }
        for &(f, c) in corners {
            let v = &mesh.faces[f].vertices;
            let n = v.len();
            let centre = (0..n).map(|k| u.uv(f, k)).sum::<DVec2>() / n as f64;
            let here = u.uv(f, c);
            // Away from the cut edges at this corner, square to them (so a
            // straight cut opens by exactly `distance`); towards the
            // polygon's middle where every edge is cut (a point split).
            let mut push = DVec2::ZERO;
            for k in [(c + n - 1) % n, (c + 1) % n] {
                let other = v[k];
                let e = edge_key(v[c], other);
                let neighbour = edge_faces
                    .get(&e)
                    .and_then(|list| list.iter().copied().find(|&g| g != f && u.used[g]));
                let cut =
                    s.mode != Mode::Vertex && neighbour.is_some_and(|g| is_cut(f, g, e, v[c]));
                if cut {
                    let along = (u.uv(f, k) - here).normalize_or_zero();
                    let mut normal = along.perp();
                    if normal.dot(centre - here) < 0. {
                        normal = -normal;
                    }
                    push += normal;
                }
            }
            let d = if push.length() > 1e-12 {
                // Two cut edges at a corner: the push reaches the
                // distance square to each.
                let unit = push.normalize();
                let cos = (push.length() / 2.).clamp(0.5, 1.);
                unit / if push.length() > 1.5 { cos } else { 1. }
            } else {
                (centre - here).normalize_or_zero()
            };
            out[f][c] = Some(here + d * distance * 0.5);
            moved += 1;
        }
    }
    moved
}

/// One side of a seam edge: (polygon, corner at the low point, corner at
/// the high point).
type SideCorners = (usize, usize, usize);

/// One seam edge between two islands: both sides' corners.
#[derive(Clone, Copy)]
struct SeamEdge {
    sides: [SideCorners; 2],
}

/// The seam edges of the selection (UVT-61): selected edges, or edges
/// between two selected points, whose two polygons disagree on UVs.
fn seam_edges(u: &Uvs<'_>, s: &Selection) -> Vec<SeamEdge> {
    let mesh = u.mesh;
    let on_points: BTreeSet<u32> = s.vertices.iter().copied().collect();
    let on_edges: BTreeSet<(u32, u32)> = s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    let wanted = |e: (u32, u32)| match s.mode {
        Mode::Edge => on_edges.contains(&e),
        Mode::Vertex => on_points.contains(&e.0) && on_points.contains(&e.1),
        Mode::Polygon => {
            // A polygon selection's border edges.
            true
        }
        Mode::Item => false,
    };
    let mut sides: BTreeMap<(u32, u32), Vec<SideCorners>> = BTreeMap::new();
    for f in 0..mesh.faces.len() {
        if !u.used[f] {
            continue;
        }
        let v = &mesh.faces[f].vertices;
        for i in 0..v.len() {
            let j = (i + 1) % v.len();
            let e = edge_key(v[i], v[j]);
            if !wanted(e) {
                continue;
            }
            // Corner order (low point, high point).
            let (ca, cb) = if v[i] < v[j] { (i, j) } else { (j, i) };
            sides.entry(e).or_default().push((f, ca, cb));
        }
    }
    let selected: BTreeSet<usize> = s.faces.iter().map(|&f| f as usize).collect();
    sides
        .into_values()
        .filter(|s2| s2.len() == 2)
        .filter(|s2| {
            s.mode != Mode::Polygon || selected.contains(&s2[0].0) != selected.contains(&s2[1].0)
        })
        .filter(|s2| {
            let ((f0, a0, b0), (f1, a1, b1)) = (s2[0], s2[1]);
            key(u.uv(f0, a0)) != key(u.uv(f1, a1)) || key(u.uv(f0, b0)) != key(u.uv(f1, b1))
        })
        .map(|s2| SeamEdge {
            sides: [s2[0], s2[1]],
        })
        .collect()
}

/// Which side of a seam counts as "selected" (UVO-P03): the side whose
/// island holds more selected polygons, else the larger island.
fn selected_side(u: &Uvs<'_>, s: &Selection, e: &SeamEdge, sizes: &[usize]) -> usize {
    let counts: Vec<usize> = e
        .sides
        .iter()
        .map(|&(f, _, _)| {
            let i = u.island_of_face(f);
            s.faces
                .iter()
                .filter(|&&g| u.island_of_face(g as usize) == i && i.is_some())
                .count()
        })
        .collect();
    if counts[0] != counts[1] {
        return if counts[0] > counts[1] { 0 } else { 1 };
    }
    let size = |k: usize| u.island_of_face(e.sides[k].0).map_or(0, |i| sizes[i]);
    if size(0) >= size(1) { 0 } else { 1 }
}

/// Moves every corner at the UV position of (f, c) to `to`.
fn move_position(u: &Uvs<'_>, out: &mut Rows, f: usize, c: usize, to: DVec2) {
    let p = u.pos[f][c];
    for (row, ids) in out.iter_mut().zip(&u.pos) {
        for (k, &q) in ids.iter().enumerate() {
            if q == p {
                row[k] = Some(to);
            }
        }
    }
}

/// Fits `from` onto `to` with one turn, (optionally) uniform scale and
/// move, mirroring when needed: the transform as a closure.
fn similarity(from: &[DVec2], to: &[DVec2], scale: bool) -> impl Fn(DVec2) -> DVec2 {
    let n = from.len().max(1) as f64;
    let (ca, cb) = (from.iter().sum::<DVec2>() / n, to.iter().sum::<DVec2>() / n);
    let (mut re, mut im, mut den) = (0., 0., 0.);
    for (a, b) in from.iter().zip(to) {
        let (a, b) = (*a - ca, *b - cb);
        re += b.x * a.x + b.y * a.y;
        im += b.y * a.x - b.x * a.y;
        den += a.length_squared();
    }
    let mut z = if den < 1e-24 {
        DVec2::new(1., 0.)
    } else {
        DVec2::new(re / den, im / den)
    };
    if !scale {
        z = z.normalize_or(DVec2::new(1., 0.));
    }
    move |p: DVec2| {
        let d = p - ca;
        cb + DVec2::new(z.x * d.x - z.y * d.y, z.x * d.y + z.y * d.x)
    }
}

/// Polygon `f`'s UV triangles (a fan).
fn triangles(rows: &Rows, f: usize) -> Vec<[DVec2; 3]> {
    let pts: Vec<DVec2> = rows[f].iter().flatten().copied().collect();
    (1..pts.len().saturating_sub(1))
        .map(|i| [pts[0], pts[i], pts[i + 1]])
        .collect()
}

fn segments_cross(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    let o = |p: DVec2, q: DVec2, r: DVec2| (q - p).perp_dot(r - p);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    ((d1 > 1e-12 && d2 < -1e-12) || (d1 < -1e-12 && d2 > 1e-12))
        && ((d3 > 1e-12 && d4 < -1e-12) || (d3 < -1e-12 && d4 > 1e-12))
}

fn point_in_triangle(p: DVec2, t: &[DVec2; 3]) -> bool {
    let s = |a: DVec2, b: DVec2| (b - a).perp_dot(p - a);
    let (a, b, c) = (s(t[0], t[1]), s(t[1], t[2]), s(t[2], t[0]));
    (a > 1e-12 && b > 1e-12 && c > 1e-12) || (a < -1e-12 && b < -1e-12 && c < -1e-12)
}

fn triangles_overlap(p: &[DVec2; 3], q: &[DVec2; 3]) -> bool {
    for i in 0..3 {
        for j in 0..3 {
            if segments_cross(p[i], p[(i + 1) % 3], q[j], q[(j + 1) % 3]) {
                return true;
            }
        }
    }
    let inside = |a: &[DVec2; 3], b: &[DVec2; 3]| {
        let c = (b[0] + b[1] + b[2]) / 3.;
        point_in_triangle(c, a)
    };
    inside(p, q) || inside(q, p)
}

/// Select Error UVs (UVT-81): the polygons that fail any ticked test.
fn error_faces(u: &Uvs<'_>, tests: ErrorTests) -> BTreeSet<u32> {
    let mesh = u.mesh;
    let faces: Vec<usize> = (0..mesh.faces.len()).filter(|&f| u.used[f]).collect();
    let mut bad = BTreeSet::new();
    let pts = |f: usize| -> Vec<DVec2> { u.rows[f].iter().flatten().copied().collect() };
    for &f in &faces {
        let p = pts(f);
        let area = signed_area(&p);
        if tests.zero_area && area.abs() < 1e-12 {
            bad.insert(f as u32);
        }
        if tests.flipped && area < -1e-12 {
            bad.insert(f as u32);
        }
        if tests.self_intersection && p.len() >= 4 {
            let n = p.len();
            let crossed = (0..n).any(|i| {
                (i + 2..n).any(|j| {
                    (i + 1) % n != j
                        && (j + 1) % n != i
                        && segments_cross(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n])
                })
            });
            if crossed {
                bad.insert(f as u32);
            }
        }
        if tests.udim_border {
            let (lo, hi) = bounds(p.iter().copied());
            if lo.x.floor() != (hi.x - 1e-9).floor() || lo.y.floor() != (hi.y - 1e-9).floor() {
                bad.insert(f as u32);
            }
        }
    }
    if tests.cross || tests.stacked {
        // Bucket polygons by their UV bounds on a coarse grid.
        let cell = 1. / 64.;
        let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        let mut boxes = HashMap::new();
        for &f in &faces {
            let (lo, hi) = bounds(pts(f).into_iter());
            boxes.insert(f, (lo, hi));
            for x in (lo.x / cell).floor() as i64..=(hi.x / cell).floor() as i64 {
                for y in (lo.y / cell).floor() as i64..=(hi.y / cell).floor() as i64 {
                    grid.entry((x, y)).or_default().push(f);
                }
            }
        }
        let mut seen = BTreeSet::new();
        for list in grid.values() {
            for (i, &a) in list.iter().enumerate() {
                for &b in &list[i + 1..] {
                    if !seen.insert((a.min(b), a.max(b))) {
                        continue;
                    }
                    let (pa, pb) = (pts(a), pts(b));
                    if tests.stacked {
                        let mut ka: Vec<_> = pa.iter().map(|&p| key(p)).collect();
                        let mut kb: Vec<_> = pb.iter().map(|&p| key(p)).collect();
                        ka.sort();
                        kb.sort();
                        if ka == kb {
                            bad.insert(a as u32);
                            bad.insert(b as u32);
                            continue;
                        }
                    }
                    if tests.cross {
                        let (la, ha) = boxes[&a];
                        let (lb, hb) = boxes[&b];
                        if la.x > hb.x || lb.x > ha.x || la.y > hb.y || lb.y > ha.y {
                            continue;
                        }
                        let hit = triangles(u.rows, a).iter().any(|ta| {
                            triangles(u.rows, b)
                                .iter()
                                .any(|tb| triangles_overlap(ta, tb))
                        });
                        if hit {
                            bad.insert(a as u32);
                            bad.insert(b as u32);
                        }
                    }
                }
            }
        }
    }
    bad
}

/// Paste Connectivity (UVT-91): the target polygon and corner offset of
/// each clipboard row, walking both meshes from their first polygons.
fn match_by_topology(
    src: &Mesh,
    src_faces: &[u32],
    dst: &Mesh,
    dst_faces: &[u32],
) -> Option<Vec<(u32, usize)>> {
    let (s0, d0) = (*src_faces.first()?, *dst_faces.first()?);
    if src.faces[s0 as usize].vertices.len() != dst.faces[d0 as usize].vertices.len() {
        return None;
    }
    let in_src: BTreeSet<u32> = src_faces.iter().copied().collect();
    let in_dst: BTreeSet<u32> = dst_faces.iter().copied().collect();
    let edge_faces = |m: &Mesh| {
        let mut map: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for (f, face) in m.faces.iter().enumerate() {
            let v = &face.vertices;
            for i in 0..v.len() {
                map.entry(edge_key(v[i], v[(i + 1) % v.len()]))
                    .or_default()
                    .push(f as u32);
            }
        }
        map
    };
    let (es, ed) = (edge_faces(src), edge_faces(dst));
    let mut matched: HashMap<u32, (u32, usize)> = HashMap::new();
    matched.insert(s0, (d0, 0));
    let mut queue = std::collections::VecDeque::from([s0]);
    while let Some(s) = queue.pop_front() {
        let (d, off) = matched[&s];
        let sv = &src.faces[s as usize].vertices;
        let dv = &dst.faces[d as usize].vertices;
        let n = sv.len();
        for i in 0..n {
            let (sa, sb) = (sv[i], sv[(i + 1) % n]);
            let (da, db) = (dv[(i + off) % n], dv[(i + 1 + off) % n]);
            let Some(sn) = es[&edge_key(sa, sb)]
                .iter()
                .copied()
                .find(|&g| g != s && in_src.contains(&g))
            else {
                continue;
            };
            if matched.contains_key(&sn) {
                continue;
            }
            let dn = ed[&edge_key(da, db)]
                .iter()
                .copied()
                .find(|&g| g != d && in_dst.contains(&g))?;
            let snv = &src.faces[sn as usize].vertices;
            let dnv = &dst.faces[dn as usize].vertices;
            if snv.len() != dnv.len() {
                return None;
            }
            // Line the neighbours up on the shared edge: sn's corner at sa
            // takes dn's corner at da.
            let ks = snv.iter().position(|&x| x == sa)?;
            let kd = dnv.iter().position(|&x| x == da)?;
            let off = (kd + snv.len() - ks) % snv.len();
            matched.insert(sn, (dn, off));
            queue.push_back(sn);
        }
    }
    src_faces.iter().map(|f| matched.get(f).copied()).collect()
}

impl crate::Editor {
    /// `Command::UvOp`: one UV button, one undo step.
    pub(crate) fn uv_op(&mut self, op: UvOp) -> Result<()> {
        self.require_idle()?;
        let label = op.label();
        let hidden = self.uv_hidden();
        let before = self.scene.clone();
        // A brush stroke works on the points it passed over, as a point
        // selection, and leaves the real selection alone.
        let (selection, op) = match op {
            UvOp::Brush {
                points,
                sew,
                distance,
                connect,
            } => (
                Selection {
                    mode: Mode::Vertex,
                    item: self.scene.selection.item,
                    vertices: points,
                    ..Default::default()
                },
                if sew {
                    // The brush sews point by point: every UV position of a
                    // brushed point meets its partners (UVT-63, both copies
                    // were under the brush).
                    UvOp::Brush {
                        points: Vec::new(),
                        sew: true,
                        distance: 0.,
                        connect,
                    }
                } else {
                    UvOp::Split { distance }
                },
            ),
            op => (self.scene.selection.clone(), op),
        };
        let id = self
            .scene
            .active()
            .ok_or_else(|| format!("Select a mesh for {label}"))?
            .id;
        let world = self.scene.world_matrix(id)?;
        let item = self.scene.active().unwrap();
        if item.camera.is_some() || item.light.is_some() {
            return Err(format!("Select a mesh for {label}"));
        }
        let mut mesh: Mesh = (*item.mesh).clone();
        if let UvOp::MergeMaps(names) = &op {
            merge_maps(&mut mesh, names)?;
            return self.finish_uv_op(id, mesh, before, label);
        }
        let map = crate::uv_maps::current_map(&mesh)
            .ok_or_else(|| format!("{label}: the mesh has no UV map yet"))?;
        let rows = mesh.uv_maps[&map].clone();
        let u = Uvs::new(&mesh, &rows, &hidden);
        let mut out = rows.clone();
        let mut new_selection: Option<Selection> = None;
        match &op {
            UvOp::QuickAlign(choice) => {
                for (_, faces) in u.selected_islands(&selection) {
                    let Some((a, b)) = reference_segment(&u, &selection, &faces) else {
                        continue;
                    };
                    let d = b - a;
                    if d.length() < 1e-12 {
                        continue;
                    }
                    let angle = d.y.atan2(d.x);
                    let horizontal = match choice {
                        AxisChoice::Auto => d.x.abs() >= d.y.abs(),
                        AxisChoice::Horizontal => true,
                        AxisChoice::Vertical => false,
                    };
                    let target = if horizontal {
                        (angle / std::f64::consts::PI).round() * std::f64::consts::PI
                    } else {
                        ((angle - std::f64::consts::FRAC_PI_2) / std::f64::consts::PI).round()
                            * std::f64::consts::PI
                            + std::f64::consts::FRAC_PI_2
                    };
                    let rot = DMat2::from_angle(target - angle);
                    let mid = (a + b) * 0.5;
                    transform_faces(&mut out, &faces, |p| mid + rot * (p - mid));
                }
            }
            UvOp::IslandAlign { u: su, v: sv } => {
                let islands = u.selected_islands(&selection);
                let all: Vec<usize> = islands.values().flatten().copied().collect();
                let (lo, hi) = bounds(u.positions(&all).into_values());
                for faces in islands.values() {
                    let (l, h) = bounds(u.positions(faces).into_values());
                    let shift = |side: Side, axis: usize| match side {
                        Side::Min => lo[axis] - l[axis],
                        Side::Max => hi[axis] - h[axis],
                        Side::Center => (lo[axis] + hi[axis] - l[axis] - h[axis]) * 0.5,
                        Side::Unchanged => 0.,
                    };
                    let t = DVec2::new(shift(*su, 0), shift(*sv, 1));
                    transform_faces(&mut out, faces, |p| p + t);
                }
            }
            UvOp::Orient(choice) => {
                for faces in u.selected_islands(&selection).values() {
                    orient(&u, &mut out, faces, *choice);
                }
            }
            UvOp::ToOrigin { udim } => {
                let corner = crate::uv_edit::udim_corner(*udim)?;
                for faces in u.selected_islands(&selection).values() {
                    let (l, _) = bounds(u.positions(faces).into_values());
                    let t = corner - l;
                    transform_faces(&mut out, faces, |p| p + t);
                }
            }
            UvOp::Texel(action, o) => {
                if !(o.image_size > 0. && o.image_size.is_finite()) {
                    return Err("Image Size must be more than zero".into());
                }
                if *action != TexelAction::Normalize && !(o.density > 0. && o.density.is_finite()) {
                    return Err("Texel Density must be more than zero".into());
                }
                let w = if o.world_space {
                    world
                } else {
                    glam::DMat4::IDENTITY
                };
                let islands = u.selected_islands(&selection);
                if o.upright {
                    for faces in islands.values() {
                        orient(&u, &mut out, faces, AxisChoice::Auto);
                    }
                }
                let all: Vec<usize> = islands.values().flatten().copied().collect();
                let overall = density(&mesh, &out, &all, o.image_size, w);
                for faces in islands.values() {
                    let now = density(&mesh, &out, faces, o.image_size, w);
                    let want = match action {
                        TexelAction::Individually => o.density,
                        TexelAction::Overall => now * o.density / overall.max(1e-18),
                        TexelAction::Normalize => overall,
                    };
                    if now <= 1e-18 {
                        continue;
                    }
                    let k = want / now;
                    let (l, h) =
                        bounds(faces.iter().flat_map(|&f| out[f].iter().flatten().copied()));
                    let c = (l + h) * 0.5;
                    transform_faces(&mut out, faces, |p| c + (p - c) * k);
                }
            }
            UvOp::RealWorldSize {
                metres,
                world_space,
            } => {
                if !(*metres > 0. && metres.is_finite()) {
                    return Err("1 UV must stand for more than zero".into());
                }
                // One UV unit per `metres`: Apply Overall at an image of one
                // pixel per UV unit.
                let w = if *world_space {
                    world
                } else {
                    glam::DMat4::IDENTITY
                };
                let islands = u.selected_islands(&selection);
                let all: Vec<usize> = islands.values().flatten().copied().collect();
                let overall = density(&mesh, &out, &all, 1., w);
                if overall > 1e-18 {
                    let k = (1. / metres) / overall;
                    for faces in islands.values() {
                        let (l, h) =
                            bounds(faces.iter().flat_map(|&f| out[f].iter().flatten().copied()));
                        let c = (l + h) * 0.5;
                        transform_faces(&mut out, faces, |p| c + (p - c) * k);
                    }
                }
            }
            UvOp::Split { distance } => {
                if !(distance.is_finite() && *distance >= 0.) {
                    return Err("Split distance must be zero or more".into());
                }
                if matches!(selection.mode, Mode::Item)
                    || (selection.faces.is_empty()
                        && selection.edges.is_empty()
                        && selection.vertices.is_empty())
                {
                    return Err("Split UVs: select the edges, points or polygons to split".into());
                }
                if split(&u, &selection, distance.max(SPLIT_PLAIN), &mut out) == 0 {
                    return Err("Split UVs: nothing selected is joined in UV".into());
                }
            }
            UvOp::Sew {
                connect,
                selected_only,
            } => {
                let seams = seam_edges(&u, &selection);
                if seams.is_empty() {
                    return Err("Sew UVs: select edges or points on a UV seam".into());
                }
                let sizes = island_sizes(&u);
                for e in &seams {
                    let sel = selected_side(&u, &selection, e, &sizes);
                    for end in 0..2 {
                        let pick = |k: usize| {
                            let (f, a, b) = e.sides[k];
                            (f, if end == 0 { a } else { b })
                        };
                        let (fs, cs) = pick(sel);
                        let (fo, co) = pick(1 - sel);
                        let (ps, po) = (u.uv(fs, cs), u.uv(fo, co));
                        let to = match (connect, selected_only) {
                            (_, true) | (ConnectTo::Unselected, _) => po,
                            (ConnectTo::Selected, _) => ps,
                            (ConnectTo::Average, _) => (ps + po) * 0.5,
                        };
                        move_position(&u, &mut out, fs, cs, to);
                        move_position(&u, &mut out, fo, co, to);
                    }
                }
            }
            UvOp::MoveSew { connect, scale } => {
                let seams = seam_edges(&u, &selection);
                if seams.is_empty() {
                    return Err("Move and Sew: select edges or points on a UV seam".into());
                }
                let sizes = island_sizes(&u);
                // Per pair of islands: the seam points on each side.
                let mut pairs: BTreeMap<(usize, usize), (Vec<DVec2>, Vec<DVec2>)> = BTreeMap::new();
                for e in &seams {
                    let sel = selected_side(&u, &selection, e, &sizes);
                    let (fs, as_, bs) = e.sides[sel];
                    let (fo, ao, bo) = e.sides[1 - sel];
                    let (Some(is), Some(io)) = (u.island_of_face(fs), u.island_of_face(fo)) else {
                        continue;
                    };
                    if is == io {
                        continue;
                    }
                    let entry = pairs.entry((is, io)).or_default();
                    entry.0.extend([u.uv(fs, as_), u.uv(fs, bs)]);
                    entry.1.extend([u.uv(fo, ao), u.uv(fo, bo)]);
                }
                let islands_faces = |i: usize| -> Vec<usize> {
                    (0..mesh.faces.len())
                        .filter(|&f| u.island_of_face(f) == Some(i))
                        .collect()
                };
                for ((is, io), (ps, po)) in pairs {
                    let mid: Vec<DVec2> =
                        ps.iter().zip(&po).map(|(a, b)| (*a + *b) * 0.5).collect();
                    match connect {
                        ConnectTo::Selected => {
                            let t = similarity(&po, &ps, *scale);
                            transform_faces(&mut out, &islands_faces(io), t);
                        }
                        ConnectTo::Unselected => {
                            let t = similarity(&ps, &po, *scale);
                            transform_faces(&mut out, &islands_faces(is), t);
                        }
                        ConnectTo::Average => {
                            let t = similarity(&po, &mid, *scale);
                            transform_faces(&mut out, &islands_faces(io), t);
                            let t = similarity(&ps, &mid, *scale);
                            transform_faces(&mut out, &islands_faces(is), t);
                        }
                    }
                }
                // Then weld the seam: both sides to their average.
                let moved = Uvs::new(&mesh, &out, &hidden);
                let mut welded = out.clone();
                for e in &seams {
                    for end in 0..2 {
                        let at = |k: usize| {
                            let (f, a, b) = e.sides[k];
                            (f, if end == 0 { a } else { b })
                        };
                        let ((f0, c0), (f1, c1)) = (at(0), at(1));
                        let to = (moved.uv(f0, c0) + moved.uv(f1, c1)) * 0.5;
                        move_position(&moved, &mut welded, f0, c0, to);
                        move_position(&moved, &mut welded, f1, c1, to);
                    }
                }
                out = welded;
            }
            UvOp::SelectBorder => {
                let mut edges = BTreeSet::new();
                let mut count: HashMap<(usize, usize), u32> = HashMap::new();
                for f in 0..mesh.faces.len() {
                    let p = &u.pos[f];
                    for i in 0..p.len() {
                        let (a, b) = (p[i], p[(i + 1) % p.len()]);
                        *count.entry((a.min(b), a.max(b))).or_default() += 1;
                    }
                }
                for f in 0..mesh.faces.len() {
                    let (p, v) = (&u.pos[f], &mesh.faces[f].vertices);
                    for i in 0..p.len() {
                        let j = (i + 1) % p.len();
                        if count[&(p[i].min(p[j]), p[i].max(p[j]))] == 1 {
                            edges.insert(edge_key(v[i], v[j]));
                        }
                    }
                }
                new_selection = Some(Selection {
                    mode: Mode::Edge,
                    item: selection.item,
                    edges: edges.into_iter().collect(),
                    ..Default::default()
                });
            }
            UvOp::SelectErrors(tests) => {
                new_selection = Some(Selection {
                    mode: Mode::Polygon,
                    item: selection.item,
                    faces: error_faces(&u, *tests).into_iter().collect(),
                    ..Default::default()
                });
            }
            UvOp::SelectUnmapped { invert } => {
                let faces = (0..mesh.faces.len())
                    .filter(|&f| !mesh.is_polyline(f) && !hidden.get(f).copied().unwrap_or(false))
                    .filter(|&f| {
                        let mapped = rows.get(f).is_some_and(|r| r.iter().all(Option::is_some));
                        mapped == *invert
                    })
                    .map(|f| f as u32)
                    .collect();
                new_selection = Some(Selection {
                    mode: Mode::Polygon,
                    item: selection.item,
                    faces,
                    ..Default::default()
                });
            }
            UvOp::Copy | UvOp::Cut => {
                let faces = polygon_order(&u, &selection);
                if faces.is_empty() {
                    return Err(format!("{label}: select polygons with UVs"));
                }
                self.uv_clipboard = Some(UvClipboard {
                    rows: faces
                        .iter()
                        .map(|&f| rows[f as usize].iter().flatten().copied().collect())
                        .collect(),
                    source: Some((mesh.clone(), faces.clone())),
                });
                if op == UvOp::Copy {
                    return Ok(());
                }
                for &f in &faces {
                    out[f as usize] = vec![None; out[f as usize].len()];
                }
            }
            UvOp::Paste(mode) => {
                let clip = self
                    .uv_clipboard
                    .clone()
                    .ok_or("Paste UVs: copy some UVs first")?;
                let targets = polygon_order_any(&mesh, &selection, &hidden);
                if targets.is_empty() {
                    return Err("Paste UVs: select the polygons to paste onto".into());
                }
                let pairs: Vec<(u32, usize)> = match mode {
                    PasteMode::Selection => targets.iter().map(|&f| (f, 0)).collect(),
                    PasteMode::Connectivity => {
                        let (src, src_faces) =
                            clip.source.as_ref().ok_or("Paste UVs: nothing to match")?;
                        match_by_topology(src, src_faces, &mesh, &targets)
                            .ok_or("Paste UVs: the selections don't have the same topology")?
                    }
                };
                if pairs.len() != clip.rows.len() && *mode == PasteMode::Selection {
                    return Err(format!(
                        "Paste UVs: {} polygons were copied, {} are selected",
                        clip.rows.len(),
                        pairs.len()
                    ));
                }
                for ((f, off), row) in pairs.iter().zip(&clip.rows) {
                    let n = mesh.faces[*f as usize].vertices.len();
                    if row.len() != n {
                        return Err("Paste UVs: a polygon has a different number of corners".into());
                    }
                    out[*f as usize] = (0..n).map(|c| Some(row[(c + n - off) % n])).collect();
                }
            }
            UvOp::Delete => {
                let faces = polygon_order_any(&mesh, &selection, &hidden);
                for &f in &faces {
                    out[f as usize] = vec![None; out[f as usize].len()];
                }
            }
            UvOp::SetValue { u: nu, v: nv } => {
                if nu.is_none() && nv.is_none() {
                    return Err("Set UV Value: give U, V or both".into());
                }
                if nu.is_some_and(|x| !x.is_finite()) || nv.is_some_and(|x| !x.is_finite()) {
                    return Err("UV values must be numbers".into());
                }
                let plan = crate::uv_edit::plan(&mesh, &map, &selection, &hidden, false)?;
                for &(f, c) in &plan.corners {
                    if let Some(p) = out[f as usize][c as usize].as_mut() {
                        if let Some(x) = nu {
                            p.x = *x;
                        }
                        if let Some(y) = nv {
                            p.y = *y;
                        }
                    }
                }
            }
            UvOp::RotatePolygons => {
                for f in polygon_order_any(&mesh, &selection, &hidden) {
                    let row = &out[f as usize];
                    if row.iter().all(Option::is_some) && !row.is_empty() {
                        let n = row.len();
                        // UVT-Q10 (build guess): each corner takes the next
                        // corner's UV.
                        out[f as usize] = (0..n).map(|c| row[(c + 1) % n]).collect();
                    }
                }
            }
            UvOp::MergeMaps(_) => unreachable!("handled above"),
            UvOp::Brush { connect, .. } => {
                let mut welded = 0;
                for &v in &selection.vertices {
                    let mut at: Vec<(usize, usize)> = Vec::new();
                    for f in 0..mesh.faces.len() {
                        if !u.used[f] {
                            continue;
                        }
                        for (c, &p) in mesh.faces[f].vertices.iter().enumerate() {
                            if p == v {
                                at.push((f, c));
                            }
                        }
                    }
                    let positions: std::collections::BTreeSet<usize> =
                        at.iter().map(|&(f, c)| u.pos[f][c]).collect();
                    if positions.len() < 2 {
                        continue;
                    }
                    let to = match connect {
                        ConnectTo::Average => {
                            positions
                                .iter()
                                .map(|&q| {
                                    let (f, c) = at
                                        .iter()
                                        .find(|&&(f, c)| u.pos[f][c] == q)
                                        .copied()
                                        .unwrap();
                                    u.uv(f, c)
                                })
                                .sum::<DVec2>()
                                / positions.len() as f64
                        }
                        // Source / Destination: the first or last copy.
                        ConnectTo::Selected => u.uv(at[0].0, at[0].1),
                        ConnectTo::Unselected => u.uv(at[at.len() - 1].0, at[at.len() - 1].1),
                    };
                    for &(f, c) in &at {
                        out[f][c] = Some(to);
                    }
                    welded += 1;
                }
                if welded == 0 {
                    return Err("Sew Brush: brush over both copies of a seam point".into());
                }
            }
            UvOp::SelectTile { udim } => {
                let lo = crate::uv_edit::udim_corner(*udim)?;
                let hi = lo + DVec2::ONE;
                let faces = (0..mesh.faces.len())
                    .filter(|&f| u.used[f])
                    .filter(|&f| {
                        rows[f].iter().flatten().all(|p| {
                            p.x >= lo.x - 1e-9
                                && p.y >= lo.y - 1e-9
                                && p.x <= hi.x + 1e-9
                                && p.y <= hi.y + 1e-9
                        })
                    })
                    .map(|f| f as u32)
                    .collect();
                new_selection = Some(Selection {
                    mode: Mode::Polygon,
                    item: selection.item,
                    faces,
                    ..Default::default()
                });
            }
            UvOp::NudgeTile { du, dv } => {
                let t = DVec2::new(*du as f64, *dv as f64);
                for faces in u.selected_islands(&selection).values() {
                    transform_faces(&mut out, faces, |p| p + t);
                }
            }
        }
        if let Some(s) = new_selection {
            self.scene.selection = s;
            if self.scene.selection == before.selection {
                return Ok(());
            }
            self.record(label.into(), before);
            return Ok(());
        }
        if out == rows {
            return Err(format!("{label}: nothing to change"));
        }
        mesh.uv_maps.insert(map.clone(), out);
        mesh.attributes.point_uvs.remove(&map);
        if mesh.attributes.uv_records.contains_key(&map) {
            let records = crate::corner_uv::records_from_uvs(&mesh.faces, &mesh.uv_maps[&map]);
            mesh.attributes.uv_records.insert(map, records);
        }
        self.finish_uv_op(id, mesh, before, label)
    }

    fn finish_uv_op(&mut self, id: u64, mesh: Mesh, before: Scene, label: &str) -> Result<()> {
        self.scene.item_mut(id).ok_or("Missing mesh")?.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }
}

/// Polygons in each island.
fn island_sizes(u: &Uvs<'_>) -> Vec<usize> {
    let mut sizes = vec![0; u.islands];
    for f in 0..u.mesh.faces.len() {
        if let Some(i) = u.island_of_face(f) {
            sizes[i] += 1;
        }
    }
    sizes
}

/// Orient Pieces on one island (UVT-56): turned about its box centre.
fn orient(u: &Uvs<'_>, out: &mut Rows, faces: &[usize], choice: AxisChoice) {
    let pts: Vec<DVec2> = faces
        .iter()
        .flat_map(|&f| out[f].iter().flatten().copied())
        .collect();
    if pts.len() < 3 {
        return;
    }
    let _ = u;
    let mut angle = minimum_area_angle(&pts);
    let rot = DMat2::from_angle(angle);
    let (l, h) = bounds(pts.iter().map(|&p| rot * p));
    let size = h - l;
    match choice {
        AxisChoice::Horizontal if size.y > size.x => angle += std::f64::consts::FRAC_PI_2,
        AxisChoice::Vertical if size.x > size.y => angle += std::f64::consts::FRAC_PI_2,
        _ => {}
    }
    let (l, h) = bounds(pts.iter().copied());
    let c = (l + h) * 0.5;
    let rot = DMat2::from_angle(angle);
    transform_faces(out, faces, |p| c + rot * (p - c));
}

/// Quick Align's reference (UVT-54) in an island: the first selected edge
/// in it, or the line between its first two selected points.
fn reference_segment(u: &Uvs<'_>, s: &Selection, faces: &[usize]) -> Option<(DVec2, DVec2)> {
    let mesh = u.mesh;
    let corner_of = |p: u32| {
        faces.iter().find_map(|&f| {
            let c = mesh.faces[f].vertices.iter().position(|&x| x == p)?;
            Some((f, c))
        })
    };
    match s.mode {
        Mode::Edge => s.edges.iter().find_map(|&(a, b)| {
            faces.iter().find_map(|&f| {
                let v = &mesh.faces[f].vertices;
                let ca = v.iter().position(|&x| x == a)?;
                let cb = v.iter().position(|&x| x == b)?;
                let adjacent = (ca + 1) % v.len() == cb || (cb + 1) % v.len() == ca;
                adjacent.then(|| (u.uv(f, ca), u.uv(f, cb)))
            })
        }),
        Mode::Vertex => {
            let mut found = s.vertices.iter().filter_map(|&p| corner_of(p));
            let (fa, ca) = found.next()?;
            let (fb, cb) = found.next()?;
            Some((u.uv(fa, ca), u.uv(fb, cb)))
        }
        _ => None,
    }
}

/// The selected polygons with UVs, in selection order (Polygon mode only).
fn polygon_order(u: &Uvs<'_>, s: &Selection) -> Vec<u32> {
    if s.mode != Mode::Polygon {
        return Vec::new();
    }
    s.faces
        .iter()
        .copied()
        .filter(|&f| u.used[f as usize])
        .collect()
}

/// The selected polygons (any UVs or none), in selection order; all
/// visible polygons when none are selected.
fn polygon_order_any(mesh: &Mesh, s: &Selection, hidden: &[bool]) -> Vec<u32> {
    let ok =
        |f: u32| !hidden.get(f as usize).copied().unwrap_or(false) && !mesh.is_polyline(f as usize);
    if s.mode == Mode::Polygon && !s.faces.is_empty() {
        return s.faces.iter().copied().filter(|&f| ok(f)).collect();
    }
    (0..mesh.faces.len() as u32).filter(|&f| ok(f)).collect()
}

/// Merge UV Maps (UVT-96): `names[0]` takes each polygon's UVs from the
/// first map in `names` that has them; the others are deleted.
pub fn merge_maps(mesh: &mut Mesh, names: &[String]) -> Result<()> {
    if names.len() < 2 {
        return Err("Merge UV Maps: select two or more UV maps".into());
    }
    for n in names {
        if !mesh.uv_maps.contains_key(n) {
            return Err(format!("The mesh has no UV map called {n}"));
        }
    }
    let mut merged = mesh.uv_maps[&names[0]].clone();
    for name in &names[1..] {
        let other = &mesh.uv_maps[name];
        for (f, row) in merged.iter_mut().enumerate() {
            if row.iter().any(Option::is_none)
                && let Some(o) = other.get(f).filter(|o| o.iter().all(Option::is_some))
            {
                *row = o.clone();
            }
        }
    }
    mesh.uv_maps.insert(names[0].clone(), merged);
    for name in &names[1..] {
        mesh.uv_maps.remove(name);
        mesh.attributes.point_uvs.remove(name);
        mesh.attributes.uv_records.remove(name);
    }
    mesh.attributes.point_uvs.remove(&names[0]);
    mesh.attributes.uv_records.remove(&names[0]);
    if mesh
        .attributes
        .current_uv_map
        .as_ref()
        .is_some_and(|c| !mesh.uv_maps.contains_key(c))
    {
        mesh.attributes.current_uv_map = Some(names[0].clone());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Script names (`operandi.uv.*` placeholders until the name map gives them:
// docs/NAMES-NEEDED.md).

type Args = serde_json::Map<String, serde_json::Value>;

fn word(a: &Args, k: &str) -> Result<Option<String>> {
    Ok(crate::command_exec::opt_str(a, k)?.map(str::to_ascii_lowercase))
}

fn number(a: &Args, k: &str) -> Result<Option<f64>> {
    a.get(k)
        .filter(|v| !v.is_null())
        .map(crate::command_exec::num)
        .transpose()
}

fn axis_choice(a: &Args) -> Result<AxisChoice> {
    Ok(match word(a, "mode")?.as_deref() {
        None | Some("auto") | Some("0") => AxisChoice::Auto,
        Some("horizontal") | Some("1") => AxisChoice::Horizontal,
        Some("vertical") | Some("2") => AxisChoice::Vertical,
        Some(w) => return Err(format!("mode is auto, horizontal or vertical, not {w}")),
    })
}

fn side(a: &Args, k: &str) -> Result<Side> {
    Ok(match word(a, k)?.as_deref() {
        None | Some("unchanged") => Side::Unchanged,
        Some("min") => Side::Min,
        Some("max") => Side::Max,
        Some("center") | Some("centre") => Side::Center,
        Some(w) => return Err(format!("{k} is min, max, center or unchanged, not {w}")),
    })
}

fn connect(a: &Args) -> Result<ConnectTo> {
    Ok(match word(a, "connect")?.as_deref() {
        None | Some("average") => ConnectTo::Average,
        Some("selected") => ConnectTo::Selected,
        Some("unselected") => ConnectTo::Unselected,
        Some(w) => {
            return Err(format!(
                "connect is average, selected or unselected, not {w}"
            ));
        }
    })
}

/// The command a `operandi.uv.*` script name runs, or `None` when the name
/// isn't one of these.
pub fn script_command(e: &crate::Editor, name: &str, a: &Args) -> Result<Option<crate::Command>> {
    use crate::command_exec::opt_bool;
    let op = match name {
        "operandi.uv.quickAlign" => UvOp::QuickAlign(axis_choice(a)?),
        "operandi.uv.islandAlign" => UvOp::IslandAlign {
            u: side(a, "u")?,
            v: side(a, "v")?,
        },
        "operandi.uv.orient" => UvOp::Orient(axis_choice(a)?),
        "operandi.uv.toOrigin" => UvOp::ToOrigin {
            udim: a
                .get("udim")
                .map(crate::command_exec::as_u32)
                .transpose()?
                .unwrap_or(1001),
        },
        "operandi.uv.texel" => {
            let mut o = e.uv_texel;
            if let Some(v) = number(a, "imageSize")? {
                o.image_size = v;
            }
            if let Some(v) = number(a, "density")? {
                o.density = v;
            }
            if let Some(v) = opt_bool(a, "worldSpace")? {
                o.world_space = v;
            }
            o.upright = opt_bool(a, "upright")?.unwrap_or(false);
            let action = match word(a, "action")?.as_deref() {
                None | Some("individually") => TexelAction::Individually,
                Some("overall") => TexelAction::Overall,
                Some("normalize") => TexelAction::Normalize,
                Some(w) => {
                    return Err(format!(
                        "action is individually, overall or normalize, not {w}"
                    ));
                }
            };
            UvOp::Texel(action, o)
        }
        "operandi.uv.realWorldSize" => UvOp::RealWorldSize {
            metres: number(a, "size")?.unwrap_or(1.),
            world_space: opt_bool(a, "worldSpace")?.unwrap_or(true),
        },
        "operandi.uv.split" => UvOp::Split {
            distance: match (number(a, "distance")?, word(a, "size")?.as_deref()) {
                (Some(d), _) => d,
                (None, None) | (None, Some("plain")) => 0.,
                (None, Some("medium")) => SPLIT_MEDIUM,
                (None, Some("large")) => SPLIT_LARGE,
                (None, Some(w)) => return Err(format!("size is plain, medium or large, not {w}")),
            },
        },
        "operandi.uv.sew" => UvOp::Sew {
            connect: connect(a)?,
            selected_only: opt_bool(a, "selectedOnly")?.unwrap_or(false),
        },
        "operandi.uv.moveSew" => UvOp::MoveSew {
            connect: connect(a)?,
            scale: opt_bool(a, "scale")?.unwrap_or(true),
        },
        "operandi.uv.selectBorder" => UvOp::SelectBorder,
        "operandi.uv.selectErrors" => {
            let d = ErrorTests::default();
            let b = |k: &str, v: bool| Ok::<bool, String>(opt_bool(a, k)?.unwrap_or(v));
            UvOp::SelectErrors(ErrorTests {
                cross: b("cross", d.cross)?,
                self_intersection: b("self", d.self_intersection)?,
                flipped: b("flipped", d.flipped)?,
                stacked: b("stacked", d.stacked)?,
                udim_border: b("udim", d.udim_border)?,
                zero_area: b("zero", d.zero_area)?,
            })
        }
        "operandi.uv.selectUnmapped" => UvOp::SelectUnmapped {
            invert: opt_bool(a, "invert")?.unwrap_or(false),
        },
        "operandi.uv.copy" => UvOp::Copy,
        "operandi.uv.cut" => UvOp::Cut,
        "operandi.uv.paste" => UvOp::Paste(match word(a, "mode")?.as_deref() {
            None | Some("connectivity") => PasteMode::Connectivity,
            Some("selection") => PasteMode::Selection,
            Some(w) => return Err(format!("mode is connectivity or selection, not {w}")),
        }),
        "operandi.uv.delete" => UvOp::Delete,
        "operandi.uv.setValue" => UvOp::SetValue {
            u: number(a, "u")?,
            v: number(a, "v")?,
        },
        "operandi.uv.rotatePolygons" => UvOp::RotatePolygons,
        "operandi.uv.seam" => {
            use crate::uv_seams::SeamEdit;
            let name = crate::command_exec::opt_str(a, "name")?
                .map(str::to_owned)
                .unwrap_or_else(|| e.uv_seam_name());
            let edit = match word(a, "action")?.as_deref() {
                None | Some("add") => SeamEdit::Add(name),
                Some("clear") => SeamEdit::Clear(name),
                Some("select") => SeamEdit::Select(name),
                Some("delete") => SeamEdit::Delete(name),
                Some("marker") => SeamEdit::Marker(name),
                Some("use") => SeamEdit::Use(name),
                Some(w) => {
                    return Err(format!(
                        "action is add, clear, select, delete or marker, not {w}"
                    ));
                }
            };
            return Ok(Some(crate::Command::UvSeam(edit)));
        }
        "operandi.uv.symmetry" => {
            let axis = match word(a, "axis")?.as_deref() {
                None | Some("off") => None,
                Some("u") => Some(crate::uv_edit::UvAxis::U),
                Some("v") => Some(crate::uv_edit::UvAxis::V),
                Some(w) => return Err(format!("axis is off, u or v, not {w}")),
            };
            return Ok(Some(crate::Command::UvSymmetry(
                crate::uv_symmetry::UvSymmetry {
                    axis,
                    center: number(a, "center")?.unwrap_or(e.uv_symmetry.center),
                },
            )));
        }
        "operandi.uv.symmetryCenter" => return Ok(Some(crate::Command::UvSymmetryCenter)),
        "operandi.uv.symmetrize" => {
            let axis = match word(a, "axis")?.as_deref() {
                None | Some("x") | Some("0") => 0,
                Some("y") | Some("1") => 1,
                Some("z") | Some("2") => 2,
                Some(w) => return Err(format!("axis is x, y or z, not {w}")),
            };
            let mirror = match word(a, "mirror")?.as_deref() {
                None | Some("u") => crate::uv_edit::UvAxis::U,
                Some("v") => crate::uv_edit::UvAxis::V,
                Some(w) => return Err(format!("mirror is u or v, not {w}")),
            };
            return Ok(Some(crate::Command::UvSymmetrize(
                crate::uv_symmetry::SymmetrizeOptions {
                    axis,
                    positive_to_negative: opt_bool(a, "positiveToNegative")?.unwrap_or(true),
                    mirror_axis: mirror,
                },
            )));
        }
        "operandi.uv.selectTile" => UvOp::SelectTile {
            udim: a
                .get("udim")
                .map(crate::command_exec::as_u32)
                .transpose()?
                .unwrap_or(1001),
        },
        "operandi.uv.nudgeTile" => {
            let n = |k: &str| -> Result<i32> {
                Ok(number(a, k)?.unwrap_or(0.).round().clamp(-99., 99.) as i32)
            };
            UvOp::NudgeTile {
                du: n("u")?,
                dv: n("v")?,
            }
        }
        "operandi.uv.toMesh" => return Ok(Some(crate::Command::UvToMesh)),
        "operandi.uv.transfer" => return Ok(Some(crate::Command::UvTransfer)),
        "operandi.uv.export" => {
            let format = match word(a, "format")?.as_deref() {
                None | Some("svg") => crate::uv_export::LayoutFormat::Svg,
                Some("eps") => crate::uv_export::LayoutFormat::Eps,
                Some(w) => return Err(format!("format is svg or eps, not {w}")),
            };
            let path = crate::command_exec::req_str(a, "filename")?;
            let size = number(a, "size")?.unwrap_or(1024.);
            let item = e.scene.active().ok_or("Select a mesh")?;
            let map =
                crate::uv_maps::current_map(&item.mesh).ok_or("The mesh has no UV map yet")?;
            let text = crate::uv_export::uv_layout(&item.mesh, &map, format, size)?;
            std::fs::write(path, text).map_err(|err| format!("Couldn't write {path}: {err}"))?;
            return Ok(None);
        }
        "operandi.uv.mergeMaps" => UvOp::MergeMaps(
            crate::command_exec::req_str(a, "names")?
                .split(',')
                .map(|n| n.trim().to_owned())
                .filter(|n| !n.is_empty())
                .collect(),
        ),
        _ => return Ok(None),
    };
    Ok(Some(crate::Command::UvOp(op)))
}
