//! Vertex Normal Tools (G9 Part A, VN-1..VN-10): hard edges kept as an
//! edge set, a vertex normal map rebuilt from them, and the tools around
//! that map.
//!
//! - VN-1: hardened edges live in the mesh's edge selection set
//!   [`HARD_EDGES`]. [`recalculate`] rebuilds the normal map from it: a
//!   corner's normal averages the polygons around its point reachable
//!   without crossing a hard edge (or, with Harden Smoothing Groups, a
//!   smoothing-group border). One hard edge alone doesn't split a corner
//!   that can be reached the other way round.
//! - The map drawn and rendered is the mesh's first normal map
//!   ([`shown_map`]); `shading` reads it in place of the smoothing angle.
//!
//! PROVISIONAL (VN-Q1), modelled on Blender's Weighted Normal modifier:
//! each polygon counts by its area times its corner angle at the point
//! (either factor off with Area / Angle Weighting); with Sum Co-Planar
//! Areas, polygons within Coplanar Tolerance of each other count as one
//! with their weights added. With Full Weighting off, a corner whose
//! largest such group outweighs every other group [`DOMINANCE`] times over
//! takes that group's direction alone (Blender's Face Influence); with it
//! on, the plain weighted average. High Quality Normals changes nothing.
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// VN-1: the edge set holding the hard edges.
pub const HARD_EDGES: &str = "HardEdges";
/// The normal map's name when none exists yet (VN-1).
pub const DEFAULT_MAP: &str = "Vertex Normal";
/// Full Weighting off: how many times over the largest coplanar group must
/// outweigh the next for a corner to take it alone (VN-Q1, provisional).
pub const DOMINANCE: f64 = 3.;

/// VN-6 Options (Preferences). `Default` is the reference's defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NormalOptions {
    pub use_custom_name: bool,
    /// The custom normal map name.
    pub map_name: String,
    /// Degrees (0–180): Harden By Angle's limit.
    pub angle: f64,
    pub area_weighting: bool,
    pub sum_coplanar_areas: bool,
    pub angle_weighting: bool,
    /// Radians (0–5°).
    pub coplanar_tolerance: f64,
    pub full_weighting: bool,
    /// Recalculate respects the HardEdges set.
    pub harden_edges: bool,
    /// Recalculate also splits at smoothing-group borders.
    pub harden_smoothing_groups: bool,
    pub high_quality_normals: bool,
}

impl Default for NormalOptions {
    fn default() -> Self {
        Self {
            use_custom_name: true,
            map_name: DEFAULT_MAP.into(),
            angle: 40.,
            area_weighting: true,
            sum_coplanar_areas: true,
            angle_weighting: true,
            coplanar_tolerance: 0.005,
            full_weighting: false,
            harden_edges: true,
            harden_smoothing_groups: true,
            high_quality_normals: true,
        }
    }
}

impl NormalOptions {
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=180.).contains(&self.angle) {
            return Err("Angle runs from 0° to 180°".into());
        }
        if !(0.0..=5f64.to_radians() + 1e-12).contains(&self.coplanar_tolerance) {
            return Err("Coplanar Tolerance runs from 0° to 5°".into());
        }
        if self.use_custom_name && self.map_name.trim().is_empty() {
            return Err("The custom normal map name is empty".into());
        }
        Ok(())
    }
    /// VN-6: Area Weighting and Full Weighting are linked; turning one on
    /// turns the other off.
    pub fn set_area_weighting(&mut self, on: bool) {
        self.area_weighting = on;
        if on {
            self.full_weighting = false;
        }
    }
    pub fn set_full_weighting(&mut self, on: bool) {
        self.full_weighting = on;
        if on {
            self.area_weighting = false;
        }
    }
}

// ---------------------------------------------------------- reading

/// The normal map `mesh` draws with: the first in its order, or its only
/// one.
pub fn shown_map_name(mesh: &Mesh) -> Option<&str> {
    let at = &mesh.attributes;
    at.normal_map_order
        .first()
        .filter(|n| at.corner_normals.contains_key(*n))
        .or_else(|| {
            (at.corner_normals.len() == 1)
                .then(|| at.corner_normals.keys().next())
                .flatten()
        })
        .map(String::as_str)
}

/// The shown map's normal at polygon `f`'s corner `c` (mesh space), if it
/// sets one.
pub fn custom_corner_normal(mesh: &Mesh, f: usize, c: usize) -> Option<DVec3> {
    let name = shown_map_name(mesh)?;
    mesh.attributes
        .corner_normals
        .get(name)?
        .get(f)?
        .get(c)
        .copied()
        .flatten()
        .filter(|n| n.is_finite() && *n != DVec3::ZERO)
}

/// The hard edges (low point first).
pub fn hard_edges(mesh: &Mesh) -> BTreeSet<(u32, u32)> {
    mesh.attributes
        .edge_sets
        .get(HARD_EDGES)
        .map(|s| s.iter().map(|&(a, b)| edge_key(a, b)).collect())
        .unwrap_or_default()
}

fn set_hard(mesh: &mut Mesh, edges: BTreeSet<(u32, u32)>) {
    let sets = &mut mesh.attributes.edge_sets;
    if edges.is_empty() {
        sets.remove(HARD_EDGES);
    } else {
        sets.insert(HARD_EDGES.into(), edges);
    }
}

/// The normal map's name for a rebuild: the custom name, else the shown
/// map's, else [`DEFAULT_MAP`].
pub fn map_name(mesh: &Mesh, o: &NormalOptions) -> String {
    if o.use_custom_name {
        return o.map_name.clone();
    }
    shown_map_name(mesh).unwrap_or(DEFAULT_MAP).to_owned()
}

// ---------------------------------------------------------- recalculate

/// Twice the polygon's area along its normal (Newell).
fn area(mesh: &Mesh, f: usize) -> f64 {
    let v = &mesh.faces[f].vertices;
    let mut n = DVec3::ZERO;
    for i in 0..v.len() {
        let a = mesh.vertices[v[i] as usize];
        let b = mesh.vertices[v[(i + 1) % v.len()] as usize];
        n += a.cross(b);
    }
    n.length() / 2.
}

/// The angle of polygon `f` at its corner `c` (radians).
fn corner_angle(mesh: &Mesh, f: usize, c: usize) -> f64 {
    let v = &mesh.faces[f].vertices;
    let n = v.len();
    let p = mesh.vertices[v[c] as usize];
    let a = mesh.vertices[v[(c + n - 1) % n] as usize] - p;
    let b = mesh.vertices[v[(c + 1) % n] as usize] - p;
    if a.length_squared() < 1e-30 || b.length_squared() < 1e-30 {
        return 0.;
    }
    a.angle_between(b)
}

/// VN-1 / VN-2: the normal map rebuilt (created if missing) on every
/// surface corner of `mesh`, splitting at hard edges (with Harden Edges)
/// and smoothing-group borders (with Harden Smoothing Groups).
pub fn recalculate(mesh: &Mesh, o: &NormalOptions) -> Result<Mesh> {
    o.validate()?;
    let hard = if o.harden_edges {
        hard_edges(mesh)
    } else {
        BTreeSet::new()
    };
    let normals: Vec<DVec3> = (0..mesh.faces.len()).map(|f| mesh.normal(f)).collect();
    let around = mesh.point_faces();
    let mut rows: Vec<Vec<Option<DVec3>>> = Vec::with_capacity(mesh.faces.len());
    for (f, face) in mesh.faces.iter().enumerate() {
        if mesh.is_polyline(f) || normals[f] == DVec3::ZERO {
            rows.push(vec![None; face.vertices.len()]);
            continue;
        }
        let mut row = Vec::with_capacity(face.vertices.len());
        for &v in &face.vertices {
            row.push(Some(corner(mesh, o, &hard, &normals, around.of(v), f, v)));
        }
        rows.push(row);
    }
    let name = map_name(mesh, o);
    let mut out = mesh.clone();
    let at = &mut out.attributes;
    at.corner_normals.insert(name.clone(), rows);
    if !at.normal_map_order.contains(&name) {
        // A new map is drawn: it goes first.
        at.normal_map_order.insert(0, name);
    }
    // The order lists exactly the maps there are.
    let maps: Vec<String> = at.corner_normals.keys().cloned().collect();
    at.normal_map_order.retain(|n| maps.contains(n));
    for n in maps {
        if !at.normal_map_order.contains(&n) {
            at.normal_map_order.push(n);
        }
    }
    out.validate()?;
    Ok(out)
}

/// The smoothed normal of polygon `f`'s corner `c` at point `v`.
#[allow(clippy::too_many_arguments)]
fn corner(
    mesh: &Mesh,
    o: &NormalOptions,
    hard: &BTreeSet<(u32, u32)>,
    normals: &[DVec3],
    around: &[u32],
    f: usize,
    v: u32,
) -> DVec3 {
    // The polygons reachable from `f` round point `v` without crossing a
    // hard edge or (with Harden Smoothing Groups) a group border.
    let neighbours = |g: usize| -> [u32; 2] {
        let w = &mesh.faces[g].vertices;
        let n = w.len();
        let k = w.iter().position(|&x| x == v).unwrap_or(0);
        [w[(k + n - 1) % n], w[(k + 1) % n]]
    };
    let mut region = vec![f];
    let mut i = 0;
    while i < region.len() {
        let g = region[i];
        i += 1;
        for w in neighbours(g) {
            let e = edge_key(v, w);
            if hard.contains(&e) {
                continue;
            }
            for &h in around {
                let h = h as usize;
                if region.contains(&h)
                    || mesh.is_polyline(h)
                    || normals[h] == DVec3::ZERO
                    || !neighbours(h).contains(&w)
                {
                    continue;
                }
                if o.harden_smoothing_groups && !crate::shading::share_group(mesh, g, h) {
                    continue;
                }
                region.push(h);
            }
        }
    }
    // Weights: area × corner angle (VN-Q1).
    let weight = |g: usize| -> f64 {
        let at = mesh.faces[g]
            .vertices
            .iter()
            .position(|&x| x == v)
            .unwrap_or(0);
        let mut w = 1.;
        if o.area_weighting {
            w *= area(mesh, g);
        }
        if o.angle_weighting {
            w *= corner_angle(mesh, g, at);
        }
        w
    };
    // Groups of (nearly) coplanar polygons: direction, weight, area.
    let mut groups: Vec<(DVec3, f64, f64)> = Vec::new();
    for &g in &region {
        let n = normals[g];
        let (w, a) = (weight(g), area(mesh, g));
        let slot = o
            .sum_coplanar_areas
            .then(|| {
                groups
                    .iter()
                    .position(|(d, _, _)| d.angle_between(n) <= o.coplanar_tolerance)
            })
            .flatten();
        match slot {
            Some(k) => {
                groups[k].1 += w;
                groups[k].2 += a;
            }
            None => groups.push((n, w, a)),
        }
    }
    if !o.full_weighting && groups.len() > 1 {
        let mut by_area: Vec<&(DVec3, f64, f64)> = groups.iter().collect();
        by_area.sort_by(|a, b| b.2.total_cmp(&a.2));
        if by_area[0].2 > DOMINANCE * by_area[1].2 {
            return by_area[0].0;
        }
    }
    let sum: DVec3 = groups.iter().map(|(d, w, _)| *d * *w).sum();
    let n = sum.normalize_or_zero();
    if n == DVec3::ZERO {
        // Weights that cancel (or are all zero): the plain average.
        let plain: DVec3 = groups.iter().map(|(d, _, _)| *d).sum();
        let p = plain.normalize_or_zero();
        if p == DVec3::ZERO { normals[f] } else { p }
    } else {
        n
    }
}

// ---------------------------------------------------------- harden / soften

/// VN-3 Harden Vertex Normals' Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardenMode {
    /// Edges at the selected points.
    Vertices,
    /// The selected edges.
    #[default]
    Edges,
    /// The outer border of the selected polygons.
    Polygons,
    /// The selected edges (all when none) sharper than Angle.
    ByAngle,
    /// UV seams of the current UV map, or polygon UV-island borders with
    /// polygons selected.
    UvAuto,
    /// Edges on UV seams.
    UvEdge,
    /// Borders of the selected polygons' UV islands.
    UvPolygon,
}

impl HardenMode {
    pub const ALL: [HardenMode; 7] = [
        HardenMode::Vertices,
        HardenMode::Edges,
        HardenMode::Polygons,
        HardenMode::ByAngle,
        HardenMode::UvAuto,
        HardenMode::UvEdge,
        HardenMode::UvPolygon,
    ];
    pub fn label(self) -> &'static str {
        match self {
            HardenMode::Vertices => "Vertices",
            HardenMode::Edges => "Edges",
            HardenMode::Polygons => "Polygons",
            HardenMode::ByAngle => "By Angle",
            HardenMode::UvAuto => "UV Auto Boundaries",
            HardenMode::UvEdge => "UV Edge Boundaries",
            HardenMode::UvPolygon => "UV Polygon Boundaries",
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            HardenMode::Vertices => "vertices",
            HardenMode::Edges => "edges",
            HardenMode::Polygons => "polygons",
            HardenMode::ByAngle => "angle",
            HardenMode::UvAuto => "uvAuto",
            HardenMode::UvEdge => "uvEdge",
            HardenMode::UvPolygon => "uvPolygon",
        }
    }
    pub fn from_word(w: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|m| m.word().eq_ignore_ascii_case(w) || m.label().eq_ignore_ascii_case(w))
    }
}

/// What the selection picks, for Harden and Soften.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picked {
    pub vertices: Vec<u32>,
    pub edges: Vec<(u32, u32)>,
    pub faces: Vec<u32>,
}

impl Picked {
    pub fn of(selection: &crate::Selection) -> Self {
        Self {
            vertices: selection.vertices.clone(),
            edges: selection.edges.clone(),
            faces: selection.faces.clone(),
        }
    }
}

/// Every surface edge of `mesh` (low point first).
fn surface_edges(mesh: &Mesh) -> BTreeSet<(u32, u32)> {
    mesh.edge_faces().keys().copied().collect()
}

/// Edges at `points`: any edge touching one, or (Connected Verts) only
/// those with both ends among them.
fn edges_at(mesh: &Mesh, points: &[u32], connected: bool) -> BTreeSet<(u32, u32)> {
    let set: BTreeSet<u32> = points.iter().copied().collect();
    surface_edges(mesh)
        .into_iter()
        .filter(|&(a, b)| {
            if connected {
                set.contains(&a) && set.contains(&b)
            } else {
                set.contains(&a) || set.contains(&b)
            }
        })
        .collect()
}

/// The outer border of `faces`: edges with exactly one of them on a side.
fn border(mesh: &Mesh, faces: &[u32]) -> BTreeSet<(u32, u32)> {
    let chosen: BTreeSet<usize> = faces.iter().map(|&f| f as usize).collect();
    mesh.edge_faces()
        .iter()
        .filter(|(_, fs)| fs.iter().filter(|f| chosen.contains(f)).count() == 1)
        .map(|(e, _)| *e)
        .collect()
}

/// Edges whose two polygons meet more sharply than `degrees`.
fn sharper_than(mesh: &Mesh, among: &BTreeSet<(u32, u32)>, degrees: f64) -> BTreeSet<(u32, u32)> {
    let limit = degrees.to_radians();
    let faces = mesh.edge_faces();
    among
        .iter()
        .copied()
        .filter(|e| {
            faces.get(e).is_some_and(|fs| {
                fs.len() >= 2 && {
                    let (a, b) = (mesh.normal(fs[0]), mesh.normal(fs[1]));
                    a != DVec3::ZERO && b != DVec3::ZERO && a.angle_between(b) > limit + 1e-9
                }
            })
        })
        .collect()
}

/// UV seams of the current UV map: edges whose two polygons give either
/// end a different UV.
fn uv_seams(mesh: &Mesh, map: Option<&str>) -> BTreeSet<(u32, u32)> {
    let Some(rows) = map
        .and_then(|m| mesh.uv_maps.get(m))
        .or_else(|| mesh.uv_maps.values().next())
    else {
        return BTreeSet::new();
    };
    let uv_at = |f: usize, v: u32| -> Option<glam::DVec2> {
        let c = mesh.faces[f].vertices.iter().position(|&x| x == v)?;
        rows.get(f)?.get(c).copied().flatten()
    };
    mesh.edge_faces()
        .iter()
        .filter(|((a, b), fs)| {
            let (a, b) = (*a, *b);
            fs.len() >= 2 && {
                let (f, g) = (fs[0], fs[1]);
                let differs = |v| match (uv_at(f, v), uv_at(g, v)) {
                    (Some(x), Some(y)) => x.distance(y) > 1e-9,
                    (None, None) => false,
                    _ => true,
                };
                differs(a) || differs(b)
            }
        })
        .map(|(e, _)| *e)
        .collect()
}

/// The UV islands of `faces` (polygons joined by an edge whose UVs match).
fn uv_island_borders(mesh: &Mesh, faces: &[u32], map: Option<&str>) -> BTreeSet<(u32, u32)> {
    let seams = uv_seams(mesh, map);
    // Borders of the islands the selection reaches: seams touching a
    // selected polygon, and the selection's outer border.
    let chosen: BTreeSet<usize> = faces.iter().map(|&f| f as usize).collect();
    let edge_faces = mesh.edge_faces();
    let mut out: BTreeSet<(u32, u32)> = seams
        .into_iter()
        .filter(|e| {
            edge_faces
                .get(e)
                .is_some_and(|fs| fs.iter().any(|f| chosen.contains(f)))
        })
        .collect();
    out.extend(
        edge_faces
            .iter()
            .filter(|(_, fs)| fs.len() == 1 && chosen.contains(&fs[0]))
            .map(|(e, _)| *e),
    );
    out
}

/// The edges Harden (or Soften) picks in `mode`.
pub fn picked_edges(
    mesh: &Mesh,
    picked: &Picked,
    mode: HardenMode,
    connected: bool,
    o: &NormalOptions,
    uv_map: Option<&str>,
) -> BTreeSet<(u32, u32)> {
    let selected: BTreeSet<(u32, u32)> =
        picked.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
    match mode {
        HardenMode::Vertices => edges_at(mesh, &picked.vertices, connected),
        HardenMode::Edges => selected,
        HardenMode::Polygons => border(mesh, &picked.faces),
        HardenMode::ByAngle => {
            let among = if selected.is_empty() {
                surface_edges(mesh)
            } else {
                selected
            };
            sharper_than(mesh, &among, o.angle)
        }
        HardenMode::UvAuto if !picked.faces.is_empty() => {
            uv_island_borders(mesh, &picked.faces, uv_map)
        }
        HardenMode::UvAuto | HardenMode::UvEdge => uv_seams(mesh, uv_map),
        HardenMode::UvPolygon => uv_island_borders(mesh, &picked.faces, uv_map),
    }
}

/// VN-3 Harden Vertex Normals: `edges` join HardEdges (with Soften Other
/// Edges, they become the whole set), then the map is rebuilt.
pub fn harden(
    mesh: &Mesh,
    edges: &BTreeSet<(u32, u32)>,
    soften_others: bool,
    o: &NormalOptions,
) -> Result<Mesh> {
    let mut out = mesh.clone();
    let mut hard = if soften_others {
        BTreeSet::new()
    } else {
        hard_edges(mesh)
    };
    hard.extend(edges.iter().copied());
    set_hard(&mut out, hard);
    recalculate(&out, o)
}

/// VN-4 Soften Vertex Normals: `edges` (every edge with `None`, the plain
/// Soften button) leave HardEdges, then the map is rebuilt.
pub fn soften(
    mesh: &Mesh,
    edges: Option<&BTreeSet<(u32, u32)>>,
    o: &NormalOptions,
) -> Result<Mesh> {
    let mut out = mesh.clone();
    let hard = match edges {
        Some(e) => hard_edges(mesh).difference(e).copied().collect(),
        None => BTreeSet::new(),
    };
    set_hard(&mut out, hard);
    recalculate(&out, o)
}

// ---------------------------------------------------------- select

/// VN-5 Select: Hard or Soft Edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeKind {
    #[default]
    Hard,
    Soft,
}

/// VN-5 Mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectMode {
    #[default]
    Set,
    Add,
    Remove,
}

/// The edge selection after VN-5 Select Hard / Soft Edges.
pub fn select_edges(
    mesh: &Mesh,
    current: &[(u32, u32)],
    kind: EdgeKind,
    mode: SelectMode,
) -> Vec<(u32, u32)> {
    let hard = hard_edges(mesh);
    let pick: BTreeSet<(u32, u32)> = match kind {
        EdgeKind::Hard => hard,
        EdgeKind::Soft => surface_edges(mesh).difference(&hard).copied().collect(),
    };
    let mut now: BTreeSet<(u32, u32)> = current.iter().map(|&(a, b)| edge_key(a, b)).collect();
    match mode {
        SelectMode::Set => now = pick,
        SelectMode::Add => now.extend(pick),
        SelectMode::Remove => now.retain(|e| !pick.contains(e)),
    }
    now.into_iter().collect()
}

// ---------------------------------------------------------- VN-7..VN-10

/// VN-7 Toggle Vertex Normal Maps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleMaps {
    /// Removes the maps if any exist, otherwise recreates one.
    #[default]
    Auto,
    On,
    Off,
}

pub fn toggle_maps(mesh: &Mesh, mode: ToggleMaps, o: &NormalOptions) -> Result<Mesh> {
    let has = !mesh.attributes.corner_normals.is_empty();
    let off = match mode {
        ToggleMaps::Auto => has,
        ToggleMaps::On => false,
        ToggleMaps::Off => true,
    };
    if off {
        let mut out = mesh.clone();
        out.attributes.corner_normals.clear();
        out.attributes.normal_map_order.clear();
        return Ok(out);
    }
    recalculate(mesh, o)
}

/// VN-9 Convert to Hard Edges: every edge where shading splits (by the
/// normal map when there is one, else the smoothing angle and groups)
/// joins HardEdges. Remove Smoothing Groups clears the groups after.
pub fn convert_to_hard_edges(
    mesh: &Mesh,
    smoothing: &crate::shading::SmoothingSettings,
    remove_groups: bool,
) -> Result<Mesh> {
    let shading = crate::shading::mesh_corner_normals(mesh, smoothing);
    let normal_at = |f: usize, v: u32| -> Option<DVec3> {
        let c = mesh.faces[f].vertices.iter().position(|&x| x == v)?;
        custom_corner_normal(mesh, f, c).or_else(|| shading.get(f)?.get(c).copied())
    };
    let mut hard = hard_edges(mesh);
    for (&(a, b), fs) in mesh.edge_faces().iter() {
        if fs.len() < 2 {
            continue;
        }
        let split = |v: u32| match (normal_at(fs[0], v), normal_at(fs[1], v)) {
            (Some(x), Some(y)) => x.angle_between(y) > 1e-6,
            _ => false,
        };
        if split(a) || split(b) {
            hard.insert((a, b));
        }
    }
    let mut out = mesh.clone();
    set_hard(&mut out, hard);
    if remove_groups {
        out.attributes.smoothing_labels.clear();
    }
    out.validate()?;
    Ok(out)
}

/// VN-10 Rename Normal Maps: the shown map takes `name`.
pub fn rename_maps(mesh: &Mesh, name: &str) -> Result<Mesh> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The normal map name is empty".into());
    }
    let old = shown_map_name(mesh)
        .ok_or("The mesh has no vertex normal map to rename")?
        .to_owned();
    if old == name {
        return Ok(mesh.clone());
    }
    if mesh.attributes.corner_normals.contains_key(name) {
        return Err(format!("The mesh already has a normal map called {name}"));
    }
    let mut out = mesh.clone();
    let at = &mut out.attributes;
    let rows = at.corner_normals.remove(&old).unwrap_or_default();
    at.corner_normals.insert(name.into(), rows);
    for n in &mut at.normal_map_order {
        if *n == old {
            *n = name.into();
        }
    }
    if at.normal_map_order.is_empty() {
        at.normal_map_order.push(name.into());
    }
    out.validate()?;
    Ok(out)
}

fn rotate<T>(row: &mut [T], k: usize) {
    if !row.is_empty() {
        row.rotate_left(k % row.len());
    }
}

/// VN-10 Spin Polygon Vertex Indices: each of `faces` (all when empty)
/// starts its point list `offset` places on (negative: the other way). The
/// shape stays; every corner's data moves with its point.
pub fn spin_polygon_indices(mesh: &Mesh, faces: &[u32], offset: i32) -> Result<Mesh> {
    let chosen: Vec<usize> = if faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        faces.iter().map(|&f| f as usize).collect()
    };
    let mut out = mesh.clone();
    let spin = |n: usize| -> usize { (offset.rem_euclid(n.max(1) as i32)) as usize };
    for &f in &chosen {
        if f >= out.faces.len() || out.is_polyline(f) {
            continue;
        }
        let n = out.faces[f].vertices.len();
        let k = spin(n);
        if k == 0 {
            continue;
        }
        let face = &mut out.faces[f];
        face.vertices.rotate_left(k);
        if let Some(uv) = &mut face.uv {
            rotate(uv, k);
        }
        for rows in out.uv_maps.values_mut() {
            if let Some(row) = rows.get_mut(f) {
                rotate(row, k);
            }
        }
        let at = &mut out.attributes;
        for rows in at.corner_normals.values_mut() {
            if let Some(row) = rows.get_mut(f) {
                rotate(row, k);
            }
        }
        for rows in at.corner_weights.values_mut() {
            if let Some(row) = rows.get_mut(f) {
                rotate(row, k);
            }
        }
        for rows in at.corner_colors.values_mut() {
            if let Some(row) = rows.get_mut(f) {
                rotate(row, k);
            }
        }
        for rows in at.uv_records.values_mut() {
            if let Some(row) = rows.get_mut(f) {
                rotate(row, k);
            }
        }
        for map in at.typed_maps.values_mut() {
            if let Some(row) = map.corners.get_mut(f) {
                rotate(row, k);
            }
        }
    }
    out.validate()?;
    Ok(out)
}

/// VN-10 Morph to Normals: each corner's normal points along morph map
/// `morph`'s offset at its point (corners whose point doesn't move keep
/// their normal).
pub fn morph_to_normals(mesh: &Mesh, morph: &str, o: &NormalOptions) -> Result<Mesh> {
    let offsets = mesh
        .attributes
        .relative_morphs
        .get(morph)
        .ok_or_else(|| format!("The mesh has no morph map called {morph}"))?
        .clone();
    let base = if shown_map_name(mesh).is_some() {
        mesh.clone()
    } else {
        recalculate(mesh, o)?
    };
    let name = shown_map_name(&base).unwrap_or(DEFAULT_MAP).to_owned();
    let mut out = base.clone();
    let rows = out
        .attributes
        .corner_normals
        .get_mut(&name)
        .ok_or("Missing normal map")?;
    for (f, face) in base.faces.iter().enumerate() {
        if base.is_polyline(f) {
            continue;
        }
        for (c, v) in face.vertices.iter().enumerate() {
            if let Some(d) = offsets.get(v).map(|d| d.normalize_or_zero())
                && d != DVec3::ZERO
            {
                rows[f][c] = Some(d);
            }
        }
    }
    out.validate()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> Mesh {
        Mesh::box_mesh(DVec3::ZERO, DVec3::splat(0.5), 0).unwrap()
    }

    #[test]
    fn options_link_area_and_full_weighting() {
        let mut o = NormalOptions::default();
        o.set_full_weighting(true);
        assert!(!o.area_weighting);
        o.set_area_weighting(true);
        assert!(!o.full_weighting);
    }

    #[test]
    fn no_hard_edges_smooth_every_cube_corner() {
        let m = recalculate(&cube(), &NormalOptions::default()).unwrap();
        for f in 0..m.faces.len() {
            for c in 0..4 {
                let n = custom_corner_normal(&m, f, c).unwrap();
                for axis in n.to_array() {
                    assert!((axis.abs() - 0.57735).abs() < 1e-4, "{n}");
                }
            }
        }
    }
}

// ---------------------------------------------------------- editor

/// One Vertex Normal Tools request (the palette's buttons and their
/// commands).
#[derive(Clone, Debug, PartialEq)]
pub enum NormalRequest {
    /// VN-2, on every selected mesh.
    Recalculate,
    /// VN-3, on the active mesh's selection.
    Harden {
        mode: HardenMode,
        soften_others: bool,
        connected: bool,
    },
    /// VN-4; `None` softens every edge (the plain Soften button).
    Soften {
        mode: Option<HardenMode>,
        connected: bool,
    },
    /// VN-5.
    Select { kind: EdgeKind, mode: SelectMode },
    /// VN-7, on every selected mesh.
    ToggleMaps(ToggleMaps),
    /// VN-8, from the first shown background mesh to the active one.
    Transfer { world: bool },
    /// VN-9, on every selected mesh.
    ConvertToHard { remove_groups: bool },
    /// VN-10, on every selected mesh: the custom map name.
    Rename,
    /// VN-10, the active mesh's selected polygons (all when none).
    Spin { offset: i32 },
    /// VN-10, the active mesh.
    MorphToNormals { morph: String },
}

impl NormalRequest {
    pub fn label(&self) -> &'static str {
        match self {
            NormalRequest::Recalculate => "Recalculate Vertex Normals",
            NormalRequest::Harden { .. } => "Harden Vertex Normals",
            NormalRequest::Soften { .. } => "Soften Vertex Normals",
            NormalRequest::Select { .. } => "Select Hard / Soft Edges",
            NormalRequest::ToggleMaps(_) => "Toggle Vertex Normal Maps",
            NormalRequest::Transfer { .. } => "Transfer Normals",
            NormalRequest::ConvertToHard { .. } => "Convert to Hard Edges",
            NormalRequest::Rename => "Rename Normal Maps",
            NormalRequest::Spin { .. } => "Spin Polygon Vertex Indices",
            NormalRequest::MorphToNormals { .. } => "Morph to Normals",
        }
    }
}

/// A request with the VN-6 options it runs under.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalCommand {
    pub request: NormalRequest,
    pub options: NormalOptions,
}

impl crate::Editor {
    /// Runs a Vertex Normal Tools request as one undo step.
    pub(crate) fn vertex_normals(&mut self, cmd: NormalCommand) -> Result<()> {
        self.require_idle()?;
        let NormalCommand { request, options } = cmd;
        options.validate()?;
        let label = request.label();
        let before = self.scene.clone();
        let mut scene = self.scene.clone();
        let active = scene
            .active()
            .filter(|i| i.is_mesh())
            .map(|i| i.id)
            .ok_or("Select a mesh")?;
        let mut meshes = scene.active_meshes();
        if meshes.is_empty() {
            meshes = vec![active];
        }
        let picked = Picked::of(&scene.selection);
        let uv_map = scene
            .item(active)
            .and_then(|i| i.mesh.attributes.current_uv_map.clone());
        let edit =
            |scene: &mut crate::Scene, id: u64, f: &dyn Fn(&Mesh) -> Result<Mesh>| -> Result<()> {
                let item = scene.item_mut(id).ok_or("Missing mesh")?;
                let mesh = f(&item.mesh)?;
                item.mesh = mesh.into();
                Ok(())
            };
        match &request {
            NormalRequest::Recalculate => {
                for &id in &meshes {
                    edit(&mut scene, id, &|m| recalculate(m, &options))?;
                }
            }
            NormalRequest::ToggleMaps(mode) => {
                for &id in &meshes {
                    edit(&mut scene, id, &|m| toggle_maps(m, *mode, &options))?;
                }
            }
            NormalRequest::ConvertToHard { remove_groups } => {
                let smoothing = scene.smoothing.clone();
                for &id in &meshes {
                    edit(&mut scene, id, &|m| {
                        convert_to_hard_edges(m, &smoothing, *remove_groups)
                    })?;
                }
            }
            NormalRequest::Rename => {
                let name = options.map_name.clone();
                for &id in &meshes {
                    let has = scene
                        .item(id)
                        .is_some_and(|i| shown_map_name(&i.mesh).is_some());
                    if has {
                        edit(&mut scene, id, &|m| rename_maps(m, &name))?;
                    }
                }
            }
            NormalRequest::Harden {
                mode,
                soften_others,
                connected,
            } => {
                edit(&mut scene, active, &|m| {
                    let edges =
                        picked_edges(m, &picked, *mode, *connected, &options, uv_map.as_deref());
                    if edges.is_empty() && !*soften_others {
                        return Err(format!(
                            "Harden ({}): nothing selected to harden",
                            mode.label()
                        ));
                    }
                    harden(m, &edges, *soften_others, &options)
                })?;
            }
            NormalRequest::Soften { mode, connected } => {
                edit(&mut scene, active, &|m| match mode {
                    Some(mode) => {
                        let edges = picked_edges(
                            m,
                            &picked,
                            *mode,
                            *connected,
                            &options,
                            uv_map.as_deref(),
                        );
                        soften(m, Some(&edges), &options)
                    }
                    None => soften(m, None, &options),
                })?;
            }
            NormalRequest::Select { kind, mode } => {
                let mesh = scene.item(active).ok_or("Missing mesh")?.mesh.clone();
                let edges = select_edges(&mesh, &scene.selection.edges, *kind, *mode);
                let s = &mut scene.selection;
                s.mode = crate::Mode::Edge;
                s.tag = None;
                s.edges = edges;
            }
            NormalRequest::Spin { offset } => {
                let faces = if scene.selection.mode == crate::Mode::Polygon {
                    scene.selection.faces.clone()
                } else {
                    Vec::new()
                };
                edit(&mut scene, active, &|m| {
                    spin_polygon_indices(m, &faces, *offset)
                })?;
            }
            NormalRequest::MorphToNormals { morph } => {
                edit(&mut scene, active, &|m| {
                    morph_to_normals(m, morph, &options)
                })?;
            }
            NormalRequest::Transfer { world } => {
                let source_id = crate::background_constraint::background_ids(&scene)
                    .into_iter()
                    .find(|&id| id != active)
                    .ok_or("Transfer Normals: show a background mesh to copy from")?;
                let to_local = if *world {
                    scene.world_matrix(active)?.inverse() * scene.world_matrix(source_id)?
                } else {
                    glam::DMat4::IDENTITY
                };
                let source = scene.item(source_id).ok_or("Missing item")?;
                let mut placed = crate::duplicate::transformed_copy(&source.mesh, to_local, false);
                // The source's own normal map turns with it.
                let turn = glam::DMat3::from_mat4(to_local).inverse().transpose();
                for rows in placed.attributes.corner_normals.values_mut() {
                    for n in rows.iter_mut().flatten().flatten() {
                        *n = (turn * *n).normalize_or_zero();
                    }
                }
                let name = map_name(&scene.item(active).ok_or("Missing mesh")?.mesh, &options);
                let points: Vec<u32> = {
                    let m = &scene.item(active).ok_or("Missing mesh")?.mesh;
                    match scene.selection.mode {
                        crate::Mode::Vertex if !scene.selection.vertices.is_empty() => {
                            scene.selection.vertices.clone()
                        }
                        _ => (0..m.vertices.len() as u32).collect(),
                    }
                };
                edit(&mut scene, active, &|m| {
                    let mut out = m.clone();
                    crate::generator_ops::transfer_vertex_map(
                        &mut out,
                        &points,
                        &placed,
                        crate::generator_ops::TransferType::VertexNormal,
                        &name,
                        crate::generator_ops::TransferSampling::Distance,
                    )?;
                    out.validate()?;
                    Ok(out)
                })?;
            }
        }
        if scene == before {
            return Ok(());
        }
        let ids_before = self.identities.current.clone();
        self.scene = scene;
        if let Err(e) = self
            .scene
            .validate()
            .and_then(|()| self.identities.synchronize(&self.scene))
        {
            self.scene = before;
            self.identities.current = ids_before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }
}
