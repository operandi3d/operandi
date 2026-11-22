//! Editing UVs in place: the UV transform (E6) and the island affine
//! engine (E7) of `specifications/uv-editing-2026-11-16`.
//!
//! Both start from a [`UvPlan`]: which corners of the current UV map a tool
//! acts on (UVT-02 scope, UVT-03 UV points, UVT-42 seams), with each
//! corner's island (UVT-04) and edge row. 3D never moves (UVT-06): only the
//! current map's values change.
//!
//! - **E6**: Move, Rotate, Scale, Transform and the other pipe tools run in
//!   U/V when started from a UV view (UVE-1 section 2, UVT-40..42,
//!   UVT-130): the tool's own code runs on the map's UV proxy (uv_pipe.rs)
//!   through the same tool session, action centre, falloff and undo as in
//!   3D. [`UvToolSpace`] holds the session's UV state.
//! - **E7** ([`IslandOp`], [`apply_island_op`]): group the target
//!   (Entire, Island, Polygon, Edge row), work out one 2D affine per group
//!   from a rule, apply it. Fit, Flip, Mirror and Axis Align are rows of
//!   [`RULES`]; their buttons are rows of the tool preset table
//!   (`presets/factory-tool-presets.toml`, operation `uv_island`). Later
//!   stages add rules here, not commands.
//!
//! Provisional readings (docs/SPECIFICATION_QUESTIONS.md, UVE-1 / UVT):
//! - UVE-1 Q6: positive angles turn counter-clockwise in UV space.
//! - UVT-Q05: snapping in a UV view is to the grid only (0.1 steps).
//! - UVT-Q03 (applied to E6): a position falloff is read at (U, V, 0).
//! - Action centres in UV space: uv_pipe.rs (UVT-133).
//! - UVE-1 Q4: Fit's Keep Proportion starts on, the block sits at the
//!   region's lower-left corner, Gaps is a margin on every side; Alt+click
//!   (per Polygon, "the same as a Barycentric layout") stretches to fill.
//! - UVE-1 Q5 / UVT-52: Mirror's Center starts at 0.5.
//! - Edge rows outside Edge mode: runs of selected points joined by mesh
//!   edges in Vertex mode, each island in Polygon mode.
use crate::topology::edge_key;
use crate::{Mesh, Mode, Result, Selection};
use glam::{DMat2, DVec2};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Corners at the same mesh point whose UVs are within this of each other
/// are one UV position (as the UV view's seam test).
const SAME_UV: f64 = 1e-6;

/// One corner of the current map: (polygon, corner index).
pub type Corner = (u32, u32);

/// Which corners a UV tool acts on, worked out once per tool session or
/// command.
#[derive(Clone, Debug, PartialEq)]
pub struct UvPlan {
    pub map: String,
    /// Target corners, by polygon then corner.
    pub corners: Vec<Corner>,
    /// The mesh point of each target corner (falloff reads it).
    pub points: Vec<u32>,
    /// The UV island (UVT-04) of each target corner.
    pub islands: Vec<u32>,
    /// The edge row of each target corner (E7 Edge row grouping).
    pub rows: Vec<u32>,
    /// The corner's UV position key: corners with the same key are one UV
    /// position (averages count a position once).
    pub positions: Vec<u32>,
}

fn uv_key(p: DVec2) -> (i64, i64) {
    let q = |x: f64| (x / SAME_UV).round() as i64;
    (q(p.x), q(p.y))
}

/// The polygons a UV tool may touch: mapped in `map` (every corner has a
/// UV) and not hidden.
fn usable_faces(mesh: &Mesh, map: &str, hidden: &[bool]) -> Vec<bool> {
    let Some(rows) = mesh.uv_maps.get(map) else {
        return vec![false; mesh.faces.len()];
    };
    (0..mesh.faces.len())
        .map(|f| {
            !hidden.get(f).copied().unwrap_or(false)
                && !mesh.is_polyline(f)
                && rows.get(f).is_some_and(|r| {
                    r.len() == mesh.faces[f].vertices.len() && r.iter().all(Option::is_some)
                })
        })
        .collect()
}

/// Union-find over `n` elements.
struct Sets(Vec<u32>);
impl Sets {
    fn new(n: usize) -> Self {
        Self((0..n as u32).collect())
    }
    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            self.0[x as usize] = self.0[self.0[x as usize] as usize];
            x = self.0[x as usize];
        }
        x
    }
    fn join(&mut self, a: u32, b: u32) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a as usize] = b;
        }
    }
}

/// The plan for `selection` on `mesh`'s map `map` (UVT-02/03/42):
/// - Polygon mode: the selected polygons' corners. Without Tear Off a UV
///   position they share with an unselected polygon moves too (it is one
///   UV point); across a seam the other side's position differs and stays.
///   Tear Off (UVT-41) moves only the selected polygons' own corners, which
///   splits them from their neighbours.
/// - Vertex mode: every UV position of the selected points.
/// - Edge mode: the UV positions of the selected edges' ends in the
///   polygons along those edges, with the positions they share.
/// - Nothing selected (or Item mode): every corner of the visible polygons.
pub fn plan(
    mesh: &Mesh,
    map: &str,
    selection: &Selection,
    hidden: &[bool],
    tear_off: bool,
) -> Result<UvPlan> {
    let rows = mesh
        .uv_maps
        .get(map)
        .ok_or_else(|| format!("The mesh has no UV map called {map}"))?;
    let usable = usable_faces(mesh, map, hidden);
    let uv = |f: usize, c: usize| rows[f][c].expect("usable corners are mapped");
    // UV positions: corners at one point with the same UV.
    let mut position_of: HashMap<(u32, (i64, i64)), u32> = HashMap::new();
    let mut corner_position: Vec<Vec<u32>> = vec![Vec::new(); mesh.faces.len()];
    let mut position_corners: Vec<Vec<Corner>> = Vec::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        if !usable[f] {
            continue;
        }
        for (c, &p) in face.vertices.iter().enumerate() {
            let next = position_corners.len() as u32;
            let id = *position_of.entry((p, uv_key(uv(f, c)))).or_insert(next);
            if id == next {
                position_corners.push(Vec::new());
            }
            position_corners[id as usize].push((f as u32, c as u32));
            corner_position[f].push(id);
        }
    }
    let nothing = match selection.mode {
        Mode::Polygon => selection.faces.is_empty(),
        Mode::Vertex => selection.vertices.is_empty(),
        Mode::Edge => selection.edges.is_empty(),
        Mode::Item => true,
    };
    let mut take = vec![Vec::<bool>::new(); mesh.faces.len()];
    for (f, face) in mesh.faces.iter().enumerate() {
        take[f] = vec![nothing && usable[f]; face.vertices.len()];
    }
    if !nothing {
        let mut seeds: Vec<Corner> = Vec::new();
        match selection.mode {
            Mode::Polygon => {
                for &f in &selection.faces {
                    if usable.get(f as usize).copied().unwrap_or(false) {
                        let n = mesh.faces[f as usize].vertices.len();
                        seeds.extend((0..n).map(|c| (f, c as u32)));
                    }
                }
            }
            Mode::Vertex => {
                let points: BTreeSet<u32> = selection.vertices.iter().copied().collect();
                for (f, face) in mesh.faces.iter().enumerate() {
                    if usable[f] {
                        for (c, p) in face.vertices.iter().enumerate() {
                            if points.contains(p) {
                                seeds.push((f as u32, c as u32));
                            }
                        }
                    }
                }
            }
            Mode::Edge => {
                let edges: BTreeSet<(u32, u32)> = selection
                    .edges
                    .iter()
                    .map(|&(a, b)| edge_key(a, b))
                    .collect();
                for (f, face) in mesh.faces.iter().enumerate() {
                    if !usable[f] {
                        continue;
                    }
                    let v = &face.vertices;
                    for i in 0..v.len() {
                        let j = (i + 1) % v.len();
                        if edges.contains(&edge_key(v[i], v[j])) {
                            seeds.push((f as u32, i as u32));
                            seeds.push((f as u32, j as u32));
                        }
                    }
                }
            }
            Mode::Item => {}
        }
        let expand = !(tear_off && selection.mode == Mode::Polygon);
        for (f, c) in seeds {
            if expand {
                let id = corner_position[f as usize][c as usize];
                for &(g, d) in &position_corners[id as usize] {
                    take[g as usize][d as usize] = true;
                }
            } else {
                take[f as usize][c as usize] = true;
            }
        }
    }
    // Islands over the usable polygons: joined through shared UV positions
    // along edges (UVT-04).
    let mut islands = Sets::new(mesh.faces.len());
    {
        let mut edge_use: HashMap<(u32, u32, u32, u32), u32> = HashMap::new();
        for (f, face) in mesh.faces.iter().enumerate() {
            if !usable[f] {
                continue;
            }
            let n = face.vertices.len();
            for i in 0..n {
                let j = (i + 1) % n;
                let (a, b) = (corner_position[f][i], corner_position[f][j]);
                let key = (
                    face.vertices[i].min(face.vertices[j]),
                    face.vertices[i].max(face.vertices[j]),
                    a.min(b),
                    a.max(b),
                );
                match edge_use.get(&key) {
                    Some(&g) => islands.join(g, f as u32),
                    None => {
                        edge_use.insert(key, f as u32);
                    }
                }
            }
        }
    }
    // Edge rows: connected runs of the selection along mesh edges.
    let row_points: Option<Sets> = match selection.mode {
        Mode::Edge | Mode::Vertex if !nothing => {
            let mut sets = Sets::new(mesh.vertices.len());
            if selection.mode == Mode::Edge {
                for &(a, b) in &selection.edges {
                    if (a as usize) < mesh.vertices.len() && (b as usize) < mesh.vertices.len() {
                        sets.join(a, b);
                    }
                }
            } else {
                let points: BTreeSet<u32> = selection.vertices.iter().copied().collect();
                for (a, b) in mesh.edges().iter().copied() {
                    if points.contains(&a) && points.contains(&b) {
                        sets.join(a, b);
                    }
                }
            }
            Some(sets)
        }
        _ => None,
    };
    let mut row_points = row_points;
    let mut out = UvPlan {
        map: map.to_owned(),
        corners: Vec::new(),
        points: Vec::new(),
        islands: Vec::new(),
        rows: Vec::new(),
        positions: Vec::new(),
    };
    for (f, face) in mesh.faces.iter().enumerate() {
        for (c, &p) in face.vertices.iter().enumerate() {
            if !take[f][c] {
                continue;
            }
            let island = islands.find(f as u32);
            out.corners.push((f as u32, c as u32));
            out.points.push(p);
            out.islands.push(island);
            out.rows.push(match row_points.as_mut() {
                Some(sets) => sets.find(p),
                None => island,
            });
            out.positions.push(corner_position[f][c]);
        }
    }
    Ok(out)
}

/// A 2D affine map `p -> m * p + t`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine2 {
    pub m: DMat2,
    pub t: DVec2,
}
impl Affine2 {
    pub const IDENTITY: Self = Self {
        m: DMat2::IDENTITY,
        t: DVec2::ZERO,
    };
    pub fn apply(&self, p: DVec2) -> DVec2 {
        self.m * p + self.t
    }
    /// Scale by `s` (per axis) about `c`, then add nothing else.
    pub fn scale_about(c: DVec2, s: DVec2) -> Self {
        let m = DMat2::from_diagonal(s);
        Self { m, t: c - m * c }
    }
    /// The E6 transform: scale, then turn by `degrees` counter-clockwise
    /// (UVE-1 Q6), both about `pivot`, then move by `offset`.
    pub fn about(pivot: DVec2, offset: DVec2, degrees: f64, scale: DVec2) -> Self {
        let m = DMat2::from_angle(degrees.to_radians()) * DMat2::from_diagonal(scale);
        Self {
            m,
            t: pivot - m * pivot + offset,
        }
    }
}

/// The UV-space state of a tool session started from a UV view
/// (`Parameters::uv`): the pipe runs on the map's UV proxy (uv_pipe.rs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UvToolSpace {
    /// The map edited: the mesh's selected map when the tool started.
    pub map: String,
    /// UVT-41 Tear Off: the selected polygons come away from their
    /// unselected neighbours. Ctrl/Cmd+Shift turns it on for one drag.
    pub tear_off: bool,
    /// UVT-Q05 (provisional, grid only): Move and Transform offsets land
    /// the action centre on multiples of this step.
    pub grid_snap: Option<f64>,
    /// The UV view's zoom in screen pixels per UV unit, for a Range in
    /// pixels (`uv_pipe::RANGE_UNITS`); 0 when unknown.
    pub pixels_per_uv: f64,
    /// The proxy, made on the first frame of a drag from the tool's base
    /// scene and kept for the rest of it (AGENTS.md "Fast drags").
    pub proxy: Option<std::sync::Arc<crate::uv_pipe::UvProxy>>,
    /// Chosen by name (Slide's UV Map field, UVT-137), not by the view the
    /// tool runs in: the app then leaves the space alone when the pointer
    /// moves between UV and 3D views.
    pub pinned: bool,
}

/// The values a transform tool drives, read for UV space: Move's U/V
/// offset is `delta.x/y`, Rotate's angle `delta.z`, Scale's U/V factors
/// `delta.x/y`; Transform has all three (`delta`, `rotation_delta.z`,
/// `scale_delta`).
pub fn tool_values(tool: crate::Tool, p: &crate::Parameters) -> (DVec2, f64, DVec2) {
    use crate::Tool;
    match tool {
        Tool::Move => (p.delta.truncate(), 0., DVec2::ONE),
        Tool::Rotate => (DVec2::ZERO, p.delta.z, DVec2::ONE),
        Tool::Scale => (DVec2::ZERO, 0., p.delta.truncate()),
        Tool::Transform => (
            p.delta.truncate(),
            p.rotation_delta.z,
            p.scale_delta.truncate(),
        ),
        _ => (DVec2::ZERO, 0., DVec2::ONE),
    }
}

// ---------------------------------------------------------------------------
// E7 island affine

/// How E7 groups the target (UVT section 7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grouping {
    /// All target UVs as one block.
    #[default]
    Entire,
    /// Each island (UVT-04) on its own.
    Island,
    /// Each polygon on its own.
    Polygon,
    /// Each run of connected selected edges on its own (the Alt+click
    /// forms).
    EdgeRow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UvAxis {
    #[default]
    U,
    V,
}
impl UvAxis {
    fn index(self) -> usize {
        match self {
            Self::U => 0,
            Self::V => 1,
        }
    }
}

/// Align UVs' Align To (UVE-1 section 6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignTo {
    /// Box minimum.
    Low,
    Average,
    /// Box centre.
    #[default]
    Center,
    /// Box maximum.
    High,
}

/// Fit UVs' options (UVE-1 section 3, UVT-50).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FitOptions {
    /// Scale uniformly (Q4: starts on, provisional).
    pub keep_proportion: bool,
    /// Margin inside the tile on every side, percent of the tile (Q4:
    /// starts 0, provisional).
    pub gaps: f64,
    /// The tile to fit into (UVT-112 numbering; 1001 start).
    pub udim: u32,
}
impl Default for FitOptions {
    fn default() -> Self {
        Self {
            keep_proportion: true,
            gaps: 0.,
            udim: 1001,
        }
    }
}

/// UVT-05: Gaps and Gaps by Pixel are one value; the image size comes from
/// the Texel Density panel (not built yet: 1024, build guess UVT section
/// 7.9).
pub const TEXEL_IMAGE_SIZE: f64 = 1024.;
pub fn gaps_to_pixels(gaps_percent: f64) -> f64 {
    gaps_percent / 100. * TEXEL_IMAGE_SIZE
}
pub fn pixels_to_gaps(pixels: f64) -> f64 {
    pixels / TEXEL_IMAGE_SIZE * 100.
}

/// The lower-left corner of UDIM tile `udim` (UVT-112: 1001 + U tile +
/// 10 × V tile, U tiles 0 to 9).
pub fn udim_corner(udim: u32) -> Result<DVec2> {
    if !(1001..=1999).contains(&udim) {
        return Err("UDIM tiles run from 1001 to 1999".into());
    }
    let i = udim - 1001;
    Ok(DVec2::new((i % 10) as f64, (i / 10) as f64))
}

/// One rule of the engine (a row of [`RULES`]).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum IslandRule {
    /// Fit UVs: scale and move each group into the tile.
    Fit(FitOptions),
    /// Flip: mirror each group about its own bounding-box centre (UVT-51).
    Flip { axis: UvAxis },
    /// Mirror: about the line `axis = center` (UVT-52).
    Mirror { axis: UvAxis, center: f64 },
    /// Axis Align: flatten each group's `axis` to one value (UVT-53).
    Align { axis: UvAxis, to: AlignTo },
}

/// One E7 run: a rule and how the target is grouped.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IslandOp {
    pub rule: IslandRule,
    pub grouping: Grouping,
}

/// The rule kinds the preset table can name, with what Shift and Alt do on
/// their buttons. A later stage adds rows here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    Fit,
    Flip,
    Mirror,
    Align,
}

/// A modifier on an E7 button (UVT-08, UVT-50..53). Ctrl/Cmd+click opens
/// the rule's dialog in the app instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Click {
    #[default]
    Plain,
    Shift,
    Alt,
}

/// One row of the rule table: the grouping of a plain, Shift and Alt
/// click (`None`: the modifier does what a plain click does).
pub struct RuleRow {
    pub kind: RuleKind,
    pub label: &'static str,
    pub plain: Grouping,
    pub shift: Option<Grouping>,
    pub alt: Option<Grouping>,
}

/// The E7 rule table (stage 1 rows).
pub const RULES: &[RuleRow] = &[
    // UVT-50: plain = Entire, Shift = per Island, Alt = per Polygon.
    RuleRow {
        kind: RuleKind::Fit,
        label: "Fit UVs",
        plain: Grouping::Entire,
        shift: Some(Grouping::Island),
        alt: Some(Grouping::Polygon),
    },
    // UVT-51: Alt = Per Polygon.
    RuleRow {
        kind: RuleKind::Flip,
        label: "Flip",
        plain: Grouping::Entire,
        shift: None,
        alt: Some(Grouping::Polygon),
    },
    // UVT-52: one line for everything.
    RuleRow {
        kind: RuleKind::Mirror,
        label: "Mirror",
        plain: Grouping::Entire,
        shift: None,
        alt: None,
    },
    // UVT-53: Alt = each edge row.
    RuleRow {
        kind: RuleKind::Align,
        label: "Align",
        plain: Grouping::Entire,
        shift: None,
        alt: Some(Grouping::EdgeRow),
    },
];

pub fn rule_row(kind: RuleKind) -> &'static RuleRow {
    RULES
        .iter()
        .find(|r| r.kind == kind)
        .expect("every rule kind has a row")
}

/// A preset row's E7 values (`actor.uv` in the preset table).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IslandPreset {
    pub rule: RuleKind,
    #[serde(default)]
    pub axis: Option<UvAxis>,
    #[serde(default)]
    pub to: Option<AlignTo>,
}

/// The remembered dialog values plain clicks use (UVT-08).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct IslandMemory {
    pub fit: FitOptions,
    /// The Fit dialog's grouping.
    pub fit_grouping: Grouping,
    /// Flip's With Options Per Polygon.
    pub flip_per_polygon: bool,
    /// Mirror's Center (UVT-52, 0.5 start, build guess).
    pub mirror_center: f64,
    /// The Align UVs dialog's last Direction and Align To.
    pub align_axis: UvAxis,
    pub align_to: AlignTo,
    pub align_per_row: bool,
}
impl Default for IslandMemory {
    fn default() -> Self {
        Self {
            fit: FitOptions::default(),
            fit_grouping: Grouping::Entire,
            flip_per_polygon: false,
            mirror_center: 0.5,
            align_axis: UvAxis::V,
            align_to: AlignTo::Low,
            align_per_row: false,
        }
    }
}

impl IslandPreset {
    /// The run a click on this button makes, from the remembered values.
    pub fn op(&self, click: Click, memory: &IslandMemory) -> Result<IslandOp> {
        let row = rule_row(self.rule);
        let grouping = match click {
            Click::Plain => row.plain,
            Click::Shift => row.shift.unwrap_or(row.plain),
            Click::Alt => row.alt.unwrap_or(row.plain),
        };
        let axis = self.axis.unwrap_or_default();
        let rule = match self.rule {
            RuleKind::Fit => IslandRule::Fit(FitOptions {
                // Alt: per Polygon as a Barycentric layout fills the tile.
                keep_proportion: memory.fit.keep_proportion && click != Click::Alt,
                ..memory.fit
            }),
            RuleKind::Flip => IslandRule::Flip { axis },
            RuleKind::Mirror => IslandRule::Mirror {
                axis,
                center: memory.mirror_center,
            },
            RuleKind::Align => IslandRule::Align {
                axis,
                to: self.to.ok_or("An Align button names Low or High")?,
            },
        };
        Ok(IslandOp { rule, grouping })
    }
}

impl IslandOp {
    pub fn label(&self) -> String {
        let axis = |a: UvAxis| match a {
            UvAxis::U => "U",
            UvAxis::V => "V",
        };
        match self.rule {
            IslandRule::Fit(_) => "Fit UVs".into(),
            IslandRule::Flip { axis: a } => format!("Flip {}", axis(a)),
            IslandRule::Mirror { axis: a, .. } => format!("Mirror {}", axis(a)),
            IslandRule::Align { axis: a, to } => match (a, to) {
                (UvAxis::V, AlignTo::High) => "Align Up".into(),
                (UvAxis::V, AlignTo::Low) => "Align Down".into(),
                (UvAxis::U, AlignTo::Low) => "Align Left".into(),
                (UvAxis::U, AlignTo::High) => "Align Right".into(),
                _ => "Align UVs".into(),
            },
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self.rule {
            IslandRule::Fit(f) => {
                if !f.gaps.is_finite() || !(0. ..50.).contains(&f.gaps) {
                    return Err("Gaps runs from 0 % to under 50 %".into());
                }
                udim_corner(f.udim)?;
            }
            IslandRule::Mirror { center, .. } if !center.is_finite() => {
                return Err("Mirror's Center must be a number".into());
            }
            _ => {}
        }
        Ok(())
    }
}

/// The affine of one group whose UV positions are `pts` (each position
/// once).
fn group_affine(rule: IslandRule, pts: &[DVec2]) -> Result<Affine2> {
    let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for p in pts {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    if pts.is_empty() {
        return Ok(Affine2::IDENTITY);
    }
    let size = hi - lo;
    let tiny = 1e-12;
    Ok(match rule {
        IslandRule::Fit(f) => {
            let margin = f.gaps / 100.;
            let region_lo = udim_corner(f.udim)? + DVec2::splat(margin);
            let room = 1. - 2. * margin;
            let s = if f.keep_proportion {
                let k = match (size.x > tiny, size.y > tiny) {
                    (true, true) => (room / size.x).min(room / size.y),
                    (true, false) => room / size.x,
                    (false, true) => room / size.y,
                    (false, false) => 1.,
                };
                DVec2::splat(k)
            } else {
                DVec2::new(
                    if size.x > tiny { room / size.x } else { 1. },
                    if size.y > tiny { room / size.y } else { 1. },
                )
            };
            let m = DMat2::from_diagonal(s);
            Affine2 {
                m,
                t: region_lo - m * lo,
            }
        }
        IslandRule::Flip { axis } => {
            let c = (lo + hi) * 0.5;
            let mut s = DVec2::ONE;
            s[axis.index()] = -1.;
            Affine2::scale_about(c, s)
        }
        IslandRule::Mirror { axis, center } => {
            let mut s = DVec2::ONE;
            s[axis.index()] = -1.;
            Affine2::scale_about(DVec2::splat(center), s)
        }
        IslandRule::Align { axis, to } => {
            let i = axis.index();
            let value = match to {
                AlignTo::Low => lo[i],
                AlignTo::High => hi[i],
                AlignTo::Center => (lo[i] + hi[i]) * 0.5,
                AlignTo::Average => pts.iter().map(|p| p[i]).sum::<f64>() / pts.len() as f64,
            };
            let mut s = DVec2::ONE;
            s[i] = 0.;
            Affine2::scale_about(DVec2::splat(value), s)
        }
    })
}

/// E7: apply `op` to the plan's corners of `mesh`'s map, in place.
pub fn apply_island_op(mesh: &mut Mesh, plan: &UvPlan, op: &IslandOp) -> Result<()> {
    op.validate()?;
    let rows = mesh
        .uv_maps
        .get(&plan.map)
        .ok_or_else(|| format!("The mesh has no UV map called {}", plan.map))?;
    let start: Vec<DVec2> = plan
        .corners
        .iter()
        .map(|&(f, c)| rows[f as usize][c as usize].unwrap_or_default())
        .collect();
    let group_of = |k: usize| -> u64 {
        match op.grouping {
            Grouping::Entire => 0,
            Grouping::Island => plan.islands[k] as u64,
            Grouping::Polygon => plan.corners[k].0 as u64,
            Grouping::EdgeRow => plan.rows[k] as u64,
        }
    };
    let mut groups: BTreeMap<u64, (Vec<usize>, BTreeMap<u32, DVec2>)> = BTreeMap::new();
    for (k, &at) in start.iter().enumerate() {
        let g = groups.entry(group_of(k)).or_default();
        g.0.push(k);
        // Per Polygon: each corner is its own position (the polygon comes
        // away from its neighbours); otherwise a position counts once.
        let key = if op.grouping == Grouping::Polygon {
            k as u32
        } else {
            plan.positions[k]
        };
        g.1.entry(key).or_insert(at);
    }
    let mut new_rows = rows.clone();
    for (members, positions) in groups.values() {
        let pts: Vec<DVec2> = positions.values().copied().collect();
        let a = group_affine(op.rule, &pts)?;
        for &k in members {
            let (f, c) = plan.corners[k];
            new_rows[f as usize][c as usize] = Some(a.apply(start[k]));
        }
    }
    mesh.uv_maps.insert(plan.map.clone(), new_rows);
    if mesh.attributes.point_uvs.contains_key(&plan.map) {
        mesh.attributes.point_uvs.remove(&plan.map);
    }
    Ok(())
}

impl crate::Editor {
    /// Per-polygon hidden flags of the active item (UVT-02: hidden
    /// polygons are never touched).
    pub(crate) fn uv_hidden(&self) -> Vec<bool> {
        let Some(item) = self.scene.active() else {
            return Vec::new();
        };
        crate::Protection::of(&item.mesh, &item.elements)
            .map(|p| p.face_hidden)
            .unwrap_or_else(|| vec![false; item.mesh.faces.len()])
    }

    /// `Command::UvIsland`: one E7 run on the active mesh's selected map,
    /// one undo step. A mesh with no UV map gets an empty "Texture"
    /// (UVT-01), which has nothing to move yet.
    pub(crate) fn uv_island(&mut self, op: IslandOp) -> Result<()> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err(format!("Finish selection before {}", op.label()));
        }
        op.validate()?;
        let hidden = self.uv_hidden();
        let before = self.scene.clone();
        let selection = self.scene.selection.clone();
        let item = self
            .scene
            .active_mut()
            .filter(|i| i.camera.is_none() && i.light.is_none())
            .ok_or_else(|| format!("Select a mesh for {}", op.label()))?;
        let mut mesh: Mesh = (*item.mesh).clone();
        let map = crate::uv_maps::ensure_map(&mut mesh);
        let plan = plan(&mesh, &map, &selection, &hidden, false)?;
        if plan.corners.is_empty() {
            return Err(format!("{}: no UVs to change in {map}", op.label()));
        }
        apply_island_op(&mut mesh, &plan, &op)?;
        // Remember the dialog values plain clicks use next (UVT-08).
        match op.rule {
            IslandRule::Fit(f) => self.uv_island_memory.fit = f,
            IslandRule::Mirror { center, .. } => self.uv_island_memory.mirror_center = center,
            _ => {}
        }
        item.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(op.label(), before);
        Ok(())
    }

    /// `Command::UvIslandPreset`: a click on an E7 button of the preset
    /// table.
    pub(crate) fn uv_island_preset(&mut self, name: &str, click: Click) -> Result<()> {
        let row = crate::tool_preset::row(name)
            .filter(|r| r.actor.operation.as_deref() == Some("uv_island"))
            .ok_or_else(|| format!("{name} is not a UV button"))?;
        let preset = row.actor.uv.ok_or("A UV button names its rule")?;
        let op = preset.op(click, &self.uv_island_memory)?;
        self.uv_island(op)
    }
}

// ---------------------------------------------------------------------------
// Command-line arguments (`operandi.uv.*`, placeholders until the registry
// thread names them: docs/NAMES-NEEDED.md).

type Args = serde_json::Map<String, serde_json::Value>;

fn word<'a>(a: &'a Args, k: &str) -> Result<Option<&'a str>> {
    crate::command_exec::opt_str(a, k)
}

fn axis_arg(a: &Args, default: UvAxis) -> Result<UvAxis> {
    Ok(match word(a, "axis")? {
        None => default,
        Some(w) if w.eq_ignore_ascii_case("u") || w == "0" => UvAxis::U,
        Some(w) if w.eq_ignore_ascii_case("v") || w == "1" => UvAxis::V,
        Some(w) => return Err(format!("axis is u or v, not {w}")),
    })
}

fn number_arg(a: &Args, k: &str) -> Result<Option<f64>> {
    match a.get(k).filter(|v| !v.is_null()) {
        None => Ok(None),
        Some(v) => crate::command_exec::num(v).map(Some),
    }
}

/// `operandi.uv.fit fit:entire|island|polygon keepProportion gaps udim`
/// (gaps in percent; `gapsByPixel` in pixels of the 1024 texture instead).
pub fn fit_args(a: &Args, memory: &IslandMemory) -> Result<IslandOp> {
    let mut f = memory.fit;
    let grouping = match word(a, "fit")? {
        None => memory.fit_grouping,
        Some(w) => match w.to_ascii_lowercase().as_str() {
            "entire" | "0" => Grouping::Entire,
            "island" | "1" => Grouping::Island,
            "polygon" | "2" => Grouping::Polygon,
            _ => return Err(format!("fit is entire, island or polygon, not {w}")),
        },
    };
    if let Some(b) = crate::command_exec::opt_bool(a, "keepProportion")? {
        f.keep_proportion = b;
    }
    if let Some(g) = number_arg(a, "gaps")? {
        f.gaps = g;
    }
    if let Some(px) = number_arg(a, "gapsByPixel")? {
        f.gaps = pixels_to_gaps(px);
    }
    if let Some(v) = a.get("udim").filter(|v| !v.is_null()) {
        f.udim = crate::command_exec::as_u32(v)?;
    }
    let op = IslandOp {
        rule: IslandRule::Fit(f),
        grouping,
    };
    op.validate()?;
    Ok(op)
}

/// `operandi.uv.flip axis:u|v perPolygon`
pub fn flip_args(a: &Args, memory: &IslandMemory) -> Result<IslandOp> {
    let per = crate::command_exec::opt_bool(a, "perPolygon")?.unwrap_or(memory.flip_per_polygon);
    Ok(IslandOp {
        rule: IslandRule::Flip {
            axis: axis_arg(a, UvAxis::U)?,
        },
        grouping: if per {
            Grouping::Polygon
        } else {
            Grouping::Entire
        },
    })
}

/// `operandi.uv.mirror axis:u|v center`
pub fn mirror_args(a: &Args, memory: &IslandMemory) -> Result<IslandOp> {
    let op = IslandOp {
        rule: IslandRule::Mirror {
            axis: axis_arg(a, UvAxis::U)?,
            center: number_arg(a, "center")?.unwrap_or(memory.mirror_center),
        },
        grouping: Grouping::Entire,
    };
    op.validate()?;
    Ok(op)
}

/// `operandi.uv.align direction:up|down|left|right alignTo:low|average|
/// center|high perRow`. A bare direction aligns to its far side (Up =
/// High V, Down = Low V, Left = Low U, Right = High U; UVE-1 section 6).
pub fn align_args(a: &Args, memory: &IslandMemory) -> Result<IslandOp> {
    let (axis, far) = match word(a, "direction")? {
        None => (memory.align_axis, None),
        Some(w) => match w.to_ascii_lowercase().as_str() {
            "up" => (UvAxis::V, Some(AlignTo::High)),
            "down" => (UvAxis::V, Some(AlignTo::Low)),
            "left" => (UvAxis::U, Some(AlignTo::Low)),
            "right" => (UvAxis::U, Some(AlignTo::High)),
            _ => return Err(format!("direction is up, down, left or right, not {w}")),
        },
    };
    let to = match word(a, "alignTo")? {
        None => far.unwrap_or(memory.align_to),
        Some(w) => match w.to_ascii_lowercase().as_str() {
            "low" => AlignTo::Low,
            "average" => AlignTo::Average,
            "center" | "centre" => AlignTo::Center,
            "high" => AlignTo::High,
            _ => return Err(format!("alignTo is low, average, center or high, not {w}")),
        },
    };
    let per = crate::command_exec::opt_bool(a, "perRow")?.unwrap_or(memory.align_per_row);
    Ok(IslandOp {
        rule: IslandRule::Align { axis, to },
        grouping: if per {
            Grouping::EdgeRow
        } else {
            Grouping::Entire
        },
    })
}
