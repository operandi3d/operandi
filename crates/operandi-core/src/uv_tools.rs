//! UV Unwrap and Pack UVs (spec `uv-unwrap-pack-relax` v1.1): which
//! polygons, seams and map they use, and how the result is laid out. The
//! flattening and packing maths are in the `operandi-uv` crate (adapted from
//! Blender).
//!
//! Provisional readings (UV-Q.. in docs/SPECIFICATION_QUESTIONS.md):
//! - Method: Conformal runs least squares conformal maps, Angle Based runs
//!   ABF++ first (UV-Q4, the option help's pairing).
//! - Iterations 0 leaves the Initial Projection; any other value solves.
//!   The count itself doesn't change the direct solvers (UV-Q17).
//! - Relative Gap Size: stored 0.2 gives about 0.005 at the tile edge, so
//!   the island margin is `gap * 0.025` of the layout (UV-Q16).
//! - Pack Gaps: stored 0.2 gives 0.01 at the tile edge and about 0.004
//!   between islands (UVP-08): the edge inset is `gap * 0.05` and the space
//!   between islands `gap * 0.02` (UV-Q16).
use crate::topology::edge_key;
use crate::{Mesh, Result};
use glam::{DVec2, DVec3};
use operandi_uv as uv;
use std::collections::{BTreeMap, BTreeSet};

/// Default map name when a mesh has none (UVW-13).
pub const DEFAULT_UV_MAP: &str = "Texture";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Axis3 {
    X,
    #[default]
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UvAxis {
    #[default]
    U,
    V,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InitialProjection {
    #[default]
    Planar,
    Cylindrical,
    Spherical,
    UseCurrentMap,
    GroupNormal,
}

impl InitialProjection {
    /// Projection Axis only applies to these (UVW-05).
    pub fn uses_axis(self) -> bool {
        matches!(self, Self::Planar | Self::Cylindrical | Self::Spherical)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UnwrapSolver {
    #[default]
    Conformal,
    AngleBased,
}

/// Where seams come from (UV Seam Type).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SeamSource {
    #[default]
    SelectedEdges,
    /// A seam map. Operandi has no seam maps yet, so this cuts nothing.
    SeamMap(String),
    /// An edge selection set.
    SelectionSet(String),
}

/// UV Unwrap's Tool Properties, in the spec's order.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UnwrapOptions {
    pub iterations: u32,
    pub relative_gap: f64,
    pub segment_by_material: bool,
    pub segment_by_part: bool,
    pub initial_projection: InitialProjection,
    pub pinning_axis: UvAxis,
    pub projection_axis: Axis3,
    /// Degrees.
    pub offset_angle: f64,
    pub seal_holes: bool,
    pub update_layout: bool,
    pub method: UnwrapSolver,
    pub symmetrize: bool,
    pub symmetry_axis: Axis3,
    pub mirror_axis: UvAxis,
    pub seams: SeamSource,
    /// The map to write (the `name` attribute); `None` picks the mesh's
    /// UV map as `target_uv_map` does. Not remembered between uses.
    #[serde(skip)]
    pub uv_map: Option<String>,
}

impl Default for UnwrapOptions {
    fn default() -> Self {
        Self {
            iterations: 100,
            relative_gap: 0.2,
            segment_by_material: false,
            segment_by_part: false,
            initial_projection: InitialProjection::Planar,
            pinning_axis: UvAxis::U,
            projection_axis: Axis3::Y,
            offset_angle: 0.0,
            seal_holes: true,
            update_layout: true,
            method: UnwrapSolver::Conformal,
            symmetrize: false,
            symmetry_axis: Axis3::X,
            mirror_axis: UvAxis::U,
            seams: SeamSource::SelectedEdges,
            uv_map: None,
        }
    }
}

impl UnwrapOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.relative_gap.is_finite() || self.relative_gap < 0.0 {
            return Err("Relative Gap Size can't be negative".into());
        }
        if !self.offset_angle.is_finite() {
            return Err("Offset Angle must be a number".into());
        }
        Ok(())
    }
}

/// The map a UV tool writes: `preferred` if the mesh has it, else the
/// mesh's selected map (UVE-1 section 1, uv_maps.rs), else "Texture", else
/// the first map; a new "Texture" when there are none (UV-Q7 provisional,
/// UVW-13).
pub fn target_uv_map(mesh: &Mesh, preferred: Option<&str>) -> String {
    if let Some(p) = preferred
        && mesh.uv_maps.contains_key(p)
    {
        return p.to_owned();
    }
    if let Some(c) = mesh.attributes.current_uv_map.as_deref()
        && mesh.uv_maps.contains_key(c)
    {
        return c.to_owned();
    }
    if mesh.uv_maps.contains_key(DEFAULT_UV_MAP) || mesh.uv_maps.is_empty() {
        return DEFAULT_UV_MAP.into();
    }
    mesh.uv_maps
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| DEFAULT_UV_MAP.into())
}

fn mesh_face(scene: &crate::Scene, f: usize) -> Vec<u32> {
    scene
        .active()
        .and_then(|i| i.mesh.faces.get(f))
        .map(|face| face.vertices.clone())
        .unwrap_or_default()
}

/// Seam edges: the source's edges plus material / part borders.
fn seam_edges(
    mesh: &Mesh,
    used: &[bool],
    options: &UnwrapOptions,
    selected: &[(u32, u32)],
) -> BTreeSet<(u32, u32)> {
    let mut seams: BTreeSet<(u32, u32)> = match &options.seams {
        SeamSource::SelectedEdges => selected.iter().map(|&(a, b)| edge_key(a, b)).collect(),
        SeamSource::SeamMap(name) => crate::uv_seams::seam_edges(mesh, name),
        SeamSource::SelectionSet(name) => mesh
            .attributes
            .edge_sets
            .get(name)
            .map(|s| s.iter().map(|&(a, b)| edge_key(a, b)).collect())
            .unwrap_or_default(),
    };
    if options.segment_by_material || options.segment_by_part {
        let mut owner: std::collections::HashMap<(u32, u32), usize> = Default::default();
        for (fi, f) in mesh.faces.iter().enumerate() {
            if !used[fi] {
                continue;
            }
            for i in 0..f.vertices.len() {
                let k = edge_key(f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
                match owner.get(&k) {
                    None => {
                        owner.insert(k, fi);
                    }
                    Some(&g) => {
                        let (a, b) = (&mesh.faces[g], f);
                        if (options.segment_by_material && a.material != b.material)
                            || (options.segment_by_part && a.part != b.part)
                        {
                            seams.insert(k);
                        }
                    }
                }
            }
        }
    }
    seams
}

fn planar(p: DVec3, axis: Axis3) -> DVec2 {
    // Seen from the positive end of the axis.
    match axis {
        Axis3::X => DVec2::new(-p.z, p.y),
        Axis3::Y => DVec2::new(p.x, -p.z),
        Axis3::Z => DVec2::new(p.x, p.y),
    }
}

fn axis_frame(axis: Axis3) -> (DVec3, DVec3, DVec3) {
    // (side, up-along-axis, other side)
    match axis {
        Axis3::X => (DVec3::Y, DVec3::X, DVec3::Z),
        Axis3::Y => (DVec3::Z, DVec3::Y, DVec3::X),
        Axis3::Z => (DVec3::X, DVec3::Z, DVec3::Y),
    }
}

/// The Initial Projection of every used corner (`None` for unused faces).
fn initial_uvs(
    mesh: &Mesh,
    used: &[bool],
    current: Option<&Vec<Vec<Option<DVec2>>>>,
    options: &UnwrapOptions,
) -> Vec<Vec<DVec2>> {
    let centre = {
        let pts: Vec<DVec3> = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(i, _)| used[*i])
            .flat_map(|(_, f)| f.vertices.iter().map(|&v| mesh.vertices[v as usize]))
            .collect();
        if pts.is_empty() {
            DVec3::ZERO
        } else {
            pts.iter().sum::<DVec3>() / pts.len() as f64
        }
    };
    let group_normal = {
        let mut n = DVec3::ZERO;
        for (fi, f) in mesh.faces.iter().enumerate() {
            if used[fi] {
                let co: Vec<DVec3> = f
                    .vertices
                    .iter()
                    .map(|&v| mesh.vertices[v as usize])
                    .collect();
                for i in 1..co.len().saturating_sub(1) {
                    n += (co[i] - co[0]).cross(co[i + 1] - co[0]);
                }
            }
        }
        n.try_normalize().unwrap_or(DVec3::Y)
    };
    let (gu, gv) = {
        let u = group_normal.any_orthonormal_vector();
        (u, group_normal.cross(u))
    };
    mesh.faces
        .iter()
        .enumerate()
        .map(|(fi, f)| {
            f.vertices
                .iter()
                .enumerate()
                .map(|(ci, &v)| {
                    let p = mesh.vertices[v as usize] - centre;
                    match options.initial_projection {
                        InitialProjection::Planar => planar(p, options.projection_axis),
                        InitialProjection::Cylindrical => {
                            let (a, up, b) = axis_frame(options.projection_axis);
                            let ang = p.dot(b).atan2(p.dot(a));
                            DVec2::new(ang / std::f64::consts::TAU + 0.5, p.dot(up))
                        }
                        InitialProjection::Spherical => {
                            let (a, up, b) = axis_frame(options.projection_axis);
                            let ang = p.dot(b).atan2(p.dot(a));
                            let r = p.length().max(1e-12);
                            DVec2::new(
                                ang / std::f64::consts::TAU + 0.5,
                                (p.dot(up) / r).clamp(-1.0, 1.0).asin() / std::f64::consts::PI
                                    + 0.5,
                            )
                        }
                        InitialProjection::UseCurrentMap => current
                            .and_then(|m| m.get(fi))
                            .and_then(|row| row.get(ci).copied().flatten())
                            .unwrap_or(DVec2::ZERO),
                        InitialProjection::GroupNormal => DVec2::new(p.dot(gu), p.dot(gv)),
                    }
                })
                .collect()
        })
        .collect()
}

/// Turns every island so its edges run along U and V as far as they can
/// (the length-weighted mean edge direction, modulo 90°), then by the
/// Pinning Axis and Offset Angle. Blender's minimum-area turn would set a
/// cube's cross of faces at 45°; this keeps it square to the tile (UV-Q18).
fn rotate_islands(
    uvs: &mut [Vec<DVec2>],
    islands: &[Option<usize>],
    count: usize,
    angle: f64,
    pinning: UvAxis,
    orient: bool,
) {
    let mut lo = vec![DVec2::splat(f64::MAX); count];
    let mut hi = vec![DVec2::splat(f64::MIN); count];
    let mut dir = vec![DVec2::ZERO; count];
    for (f, row) in uvs.iter().enumerate() {
        if let Some(i) = islands[f] {
            for k in 0..row.len() {
                let d = row[(k + 1) % row.len()] - row[k];
                let a = 4.0 * d.y.atan2(d.x);
                dir[i] += DVec2::new(a.cos(), a.sin()) * d.length();
            }
        }
    }
    let turn: Vec<f64> = dir
        .iter()
        .map(|d| {
            if orient && d.length() > 1e-12 {
                -d.y.atan2(d.x) / 4.0
            } else {
                0.0
            }
        })
        .collect();
    let rot = |p: DVec2, a: f64| {
        let (s, c) = a.sin_cos();
        DVec2::new(c * p.x - s * p.y, s * p.x + c * p.y)
    };
    for (f, row) in uvs.iter().enumerate() {
        if let Some(i) = islands[f] {
            for p in row {
                let q = rot(*p, turn[i]);
                lo[i] = lo[i].min(q);
                hi[i] = hi[i].max(q);
            }
        }
    }
    for (f, row) in uvs.iter_mut().enumerate() {
        let Some(i) = islands[f] else { continue };
        let size = hi[i] - lo[i];
        let mut a = angle;
        // Pinning Axis: the island's longer side runs along it.
        if orient {
            let along_u = size.x >= size.y - 1e-9 * size.y.abs();
            if along_u != (pinning == UvAxis::U) {
                a += std::f64::consts::FRAC_PI_2;
            }
        }
        let c = (lo[i] + hi[i]) * 0.5;
        for p in row.iter_mut() {
            let q = rot(*p, turn[i]);
            *p = c + rot(q - c, a);
        }
    }
}

/// Lays islands out side by side (no rotation), then scales them together
/// so the layout's longer side spans the tile, from (0, 0) (UVW-11).
fn layout(uvs: &mut [Vec<DVec2>], islands: &[Option<usize>], count: usize, gap: f64) {
    if count == 0 {
        return;
    }
    let margin = gap * 0.025;
    let pack_once = |uvs: &[Vec<DVec2>], swap: bool| -> Vec<Vec<DVec2>> {
        let t = |p: DVec2| if swap { DVec2::new(p.y, p.x) } else { p };
        let mut pack: Vec<uv::pack::PackIsland> = (0..count)
            .map(|i| uv::pack::PackIsland::new(1.0, false, i))
            .collect();
        for (f, row) in uvs.iter().enumerate() {
            if let Some(i) = islands[f] {
                let r: Vec<DVec2> = row.iter().map(|&p| t(p)).collect();
                pack[i].add_polygon(&r);
            }
        }
        let params = uv::pack::PackParams {
            rotate_method: uv::pack::RotationMethod::None,
            shape_method: uv::pack::ShapeMethod::Aabb,
            margin_method: if margin > 0.0 {
                uv::pack::MarginMethod::Fraction
            } else {
                uv::pack::MarginMethod::Add
            },
            margin,
            // A hair over 1 breaks the strip packer's ties toward a layout
            // wider than tall (UVW-11's 1 x 0.75).
            target_aspect_y: 1.0 + 1e-9,
            ..Default::default()
        };
        let scale = uv::pack::pack_islands(&mut pack, &params);
        uvs.iter()
            .enumerate()
            .map(|(f, row)| match islands[f] {
                Some(i) => row
                    .iter()
                    .map(|&p| t(pack[i].transform(t(p), scale, DVec2::ZERO)))
                    .collect(),
                None => row.clone(),
            })
            .collect()
    };
    let packed = pack_once(uvs, false);
    uvs.clone_from_slice(&packed);
    // Fit: the longer side spans 0..1 with the margin at the edge.
    let (mut lo, mut hi) = (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN));
    for (f, row) in uvs.iter().enumerate() {
        if islands[f].is_some() {
            for p in row {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
        }
    }
    let size = (hi - lo).max_element();
    if size > 0.0 && size.is_finite() {
        let k = (1.0 - 2.0 * margin) / size;
        for (f, row) in uvs.iter_mut().enumerate() {
            if islands[f].is_some() {
                for p in row.iter_mut() {
                    *p = (*p - lo) * k + DVec2::splat(margin);
                }
            }
        }
    }
}

/// UV Unwrap on a mesh: `hidden` marks polygons to leave alone, `selected`
/// is the edge selection (the default seam source). Returns the mesh with
/// `map` written for the used polygons, the map created when missing.
pub fn unwrap_mesh(
    mesh: &Mesh,
    map: &str,
    hidden: &[bool],
    selected_edges: &[(u32, u32)],
    options: &UnwrapOptions,
) -> Result<Mesh> {
    options.validate()?;
    let mut out = mesh.clone();
    let nf = mesh.faces.len();
    let used: Vec<bool> = (0..nf)
        .map(|f| !hidden.get(f).copied().unwrap_or(false))
        .collect();
    let current = mesh.uv_maps.get(map);
    let start = initial_uvs(mesh, &used, current, options);

    let mut seams = seam_edges(mesh, &used, options, selected_edges);
    let used_faces: Vec<usize> = (0..nf).filter(|&f| used[f]).collect();
    let mut result: Vec<Vec<DVec2>> = start.clone();
    let mut islands: Vec<Option<usize>> = vec![None; nf];

    if !used_faces.is_empty() {
        let build = |seams: &BTreeSet<(u32, u32)>| uv::UvMesh {
            points: mesh.vertices.clone(),
            weights: None,
            faces: used_faces
                .iter()
                .map(|&f| {
                    mesh.faces[f]
                        .vertices
                        .iter()
                        .zip(&start[f])
                        .map(|(&p, &q)| uv::Corner::new(p, q))
                        .collect()
                })
                .collect(),
            seams: seams.iter().map(|&(a, b)| [a, b]).collect(),
        };
        let construct = uv::ConstructOptions {
            fill_holes: options.seal_holes,
            topology_from_uvs: false,
            aspect_y: 1.0,
        };
        // A closed piece with no seam to open it: every polygon becomes its
        // own island (UVW-10).
        let probe = uv::Param::new(&build(&seams), construct).map_err(|e| e.to_string())?;
        if probe.failed_chart_count() > 0 {
            for (k, chart) in probe.face_charts().iter().enumerate() {
                if chart.is_none() {
                    let v = &mesh.faces[used_faces[k]].vertices;
                    for i in 0..v.len() {
                        seams.insert(edge_key(v[i], v[(i + 1) % v.len()]));
                    }
                }
            }
        }
        let input = build(&seams);
        let solved = if options.iterations == 0 {
            // Initial Projection only: islands from the cut surface.
            let p = uv::Param::new(&input, construct).map_err(|e| e.to_string())?;
            let fc = p.face_charts();
            (
                input
                    .faces
                    .iter()
                    .map(|f| f.iter().map(|c| c.uv).collect())
                    .collect::<Vec<_>>(),
                fc,
                p.chart_count(),
            )
        } else {
            let o = uv::UnwrapOptions {
                method: match options.method {
                    UnwrapSolver::Conformal => uv::UnwrapMethod::Conformal,
                    UnwrapSolver::AngleBased => uv::UnwrapMethod::AngleBased,
                },
                construct,
                average_scale: true,
                ..Default::default()
            };
            let r = uv::unwrap(&input, &o).map_err(|e| e.to_string())?;
            (r.uvs, r.islands, r.island_count)
        };
        let count = solved.2;
        for (k, &f) in used_faces.iter().enumerate() {
            result[f] = solved.0[k].clone();
            islands[f] = solved.1[k];
        }
        rotate_islands(
            &mut result,
            &islands,
            count,
            options.offset_angle.to_radians(),
            options.pinning_axis,
            options.iterations > 0,
        );
        if options.update_layout || options.iterations > 0 {
            layout(&mut result, &islands, count, options.relative_gap);
        }
    }

    let rows = out.uv_maps.entry(map.to_owned()).or_insert_with(|| {
        mesh.faces
            .iter()
            .map(|f| vec![None; f.vertices.len()])
            .collect()
    });
    for &f in &used_faces {
        rows[f] = result[f].iter().map(|&p| Some(p)).collect();
    }
    finish_map(&mut out, map);
    out.validate()?;
    Ok(out)
}

/// Keeps the map's per-corner records consistent after its UVs changed.
fn finish_map(mesh: &mut Mesh, map: &str) {
    mesh.attributes.point_uvs.remove(map);
    if mesh.attributes.uv_records.contains_key(map) {
        let records = crate::corner_uv::records_from_uvs(&mesh.faces, &mesh.uv_maps[map]);
        mesh.attributes.uv_records.insert(map.to_owned(), records);
    }
}

// ---------------------------------------------------------------------------
// Pack UVs

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PackDirection {
    Horizontal,
    Vertical,
    #[default]
    Auto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PackRegion {
    #[default]
    Normalized,
    Nearest,
    Udim,
    Manual,
    Tiles,
    PolygonTag,
    ByItem,
}

impl PackRegion {
    /// Regions Operandi builds so far (all of them since the UV tools
    /// finish, 2026-11-18).
    pub fn is_built(self) -> bool {
        true
    }
}

/// The Polygon Tag region's tag (UVS-1 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PackTag {
    #[default]
    Material,
    Part,
    /// Operandi's smoothing is one flag per polygon: smooth and flat
    /// polygons make two groups (UVP-P02, provisional).
    SmoothingGroup,
    /// A polygon's first polygon selection set by name; polygons in none
    /// make their own group.
    SelectionSet,
}

/// Pack UVs dialog values, in the spec's order (UVP starting values).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PackOptions {
    pub pack: bool,
    pub stretch: bool,
    pub orient: bool,
    pub keep_size: bool,
    pub direction: PackDirection,
    pub gaps: f64,
    pub gaps_by_pixel: bool,
    pub pixel_size: f64,
    /// Texel Density Image Size used by Pixel Size (UV-Q10: 1024 guessed).
    pub image_size: f64,
    pub bounding_box: bool,
    pub lock_stacked: bool,
    pub region: PackRegion,
    pub udim: u32,
    pub region_uv: DVec2,
    pub region_size: DVec2,
    pub tiles: [u32; 2],
    pub find_best_box_rotation: bool,
    pub polygon_tag: PackTag,
    /// Background Constraint: the other visible meshes' islands in the map
    /// of the same name stay where they are and are packed round
    /// (UVP-P03, provisional reading).
    pub background: bool,
    /// Pack UVs to New Map: the result goes to a new map and the original
    /// stays.
    pub write_new: bool,
    /// The new map's name ("" picks a free "Texture N").
    pub new_name: String,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            pack: true,
            stretch: true,
            orient: false,
            keep_size: false,
            direction: PackDirection::Auto,
            gaps: 0.2,
            gaps_by_pixel: false,
            pixel_size: 10.24,
            image_size: 1024.0,
            bounding_box: false,
            lock_stacked: false,
            region: PackRegion::Normalized,
            udim: 1001,
            region_uv: DVec2::ZERO,
            region_size: DVec2::ONE,
            tiles: [1, 1],
            find_best_box_rotation: true,
            polygon_tag: PackTag::Material,
            background: false,
            write_new: false,
            new_name: String::new(),
        }
    }
}

impl PackOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.region.is_built() {
            return Err("That Pack Region isn't available yet".into());
        }
        if !(self.gaps.is_finite() && self.gaps >= 0.0) {
            return Err("Gaps can't be negative".into());
        }
        if self.gaps_by_pixel && !(self.pixel_size >= 0.0 && self.image_size > 0.0) {
            return Err("Pixel Size and Image Size must be positive".into());
        }
        if self.region == PackRegion::Udim && self.udim < 1001 {
            return Err("UDIM tiles start at 1001".into());
        }
        if self.region == PackRegion::Manual
            && !(self.region_size.x > 0.0 && self.region_size.y > 0.0 && self.region_uv.is_finite())
        {
            return Err("The region needs a positive width and height".into());
        }
        if self.region == PackRegion::Tiles && (self.tiles[0] == 0 || self.tiles[1] == 0) {
            return Err("Tiles needs at least one tile each way".into());
        }
        Ok(())
    }

    /// Fraction of the region left between neighbouring islands.
    fn between(&self) -> f64 {
        if self.gaps_by_pixel {
            self.pixel_size / self.image_size
        } else {
            self.gaps * 0.02
        }
    }

    /// Fraction of the region left at its edge.
    fn edge(&self) -> f64 {
        if self.gaps_by_pixel {
            self.pixel_size / self.image_size
        } else {
            self.gaps * 0.05
        }
    }

    /// Lower-left corner and size of the target region.
    fn region_rect(&self) -> (DVec2, DVec2) {
        match self.region {
            PackRegion::Udim => {
                let t = self.udim - 1001;
                (DVec2::new((t % 10) as f64, (t / 10) as f64), DVec2::ONE)
            }
            PackRegion::Manual => (self.region_uv, self.region_size),
            PackRegion::Tiles => (
                DVec2::ZERO,
                DVec2::new(self.tiles[0] as f64, self.tiles[1] as f64),
            ),
            _ => (DVec2::ZERO, DVec2::ONE),
        }
    }
}

/// Pack UVs on one map. `hidden` polygons are ignored; when `selected`
/// polygons are given, only the islands holding them move and the others
/// stay put as obstacles (UVP-09). Polygons with no UVs in `map` are skipped.
pub fn pack_mesh(
    mesh: &Mesh,
    map: &str,
    hidden: &[bool],
    selected: &[u32],
    options: &PackOptions,
) -> Result<Mesh> {
    pack_mesh_with(mesh, map, hidden, selected, options, &[])
}

/// The tile (lower-left corner) holding `p`.
fn tile_of(p: DVec2) -> DVec2 {
    DVec2::new(p.x.floor(), p.y.floor())
}

/// `pack_mesh` with `obstacles`: UV polygons (another mesh's islands,
/// Background Constraint) that stay put and are packed round. Regions that
/// split the islands into groups (Nearest, Polygon Tag) pack each group
/// into its own tile.
pub fn pack_mesh_with(
    mesh: &Mesh,
    map: &str,
    hidden: &[bool],
    selected: &[u32],
    options: &PackOptions,
    obstacles: &[Vec<DVec2>],
) -> Result<Mesh> {
    options.validate()?;
    let rows = mesh.uv_maps.get(map).ok_or("Select a UV map to pack")?;
    let nf = mesh.faces.len();
    let used: Vec<usize> = (0..nf)
        .filter(|&f| {
            !hidden.get(f).copied().unwrap_or(false) && rows[f].iter().all(Option::is_some)
        })
        .collect();
    let mut out = mesh.clone();
    if used.is_empty() || (!options.pack && !options.stretch) {
        return Ok(out);
    }
    let target = if options.write_new {
        let name = if options.new_name.trim().is_empty() {
            crate::uv_maps::free_name(mesh, DEFAULT_UV_MAP)
        } else {
            options.new_name.trim().to_owned()
        };
        out.uv_maps.insert(name.clone(), rows.clone());
        name
    } else {
        map.to_owned()
    };
    // The groups and the region each one fills.
    let groups: Vec<(Vec<usize>, (DVec2, DVec2))> = match options.region {
        PackRegion::Nearest => {
            // Each island to the tile nearest its middle (UVP-P01: islands
            // are grouped by the tile their bounding box's centre is in).
            let faces: Vec<Vec<(u32, DVec2)>> = used
                .iter()
                .map(|&f| {
                    mesh.faces[f]
                        .vertices
                        .iter()
                        .zip(&rows[f])
                        .map(|(&p, q)| (p, q.unwrap()))
                        .collect()
                })
                .collect();
            let (ids, count) = uv::pack::uv_islands(&faces);
            let mut lo = vec![DVec2::splat(f64::MAX); count];
            let mut hi = vec![DVec2::splat(f64::MIN); count];
            for (k, f) in faces.iter().enumerate() {
                for (_, p) in f {
                    lo[ids[k]] = lo[ids[k]].min(*p);
                    hi[ids[k]] = hi[ids[k]].max(*p);
                }
            }
            let mut by_tile: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
            for (k, &f) in used.iter().enumerate() {
                let t = tile_of((lo[ids[k]] + hi[ids[k]]) * 0.5);
                by_tile.entry((t.x as i64, t.y as i64)).or_default().push(f);
            }
            by_tile
                .into_iter()
                .map(|((u, v), faces)| (faces, (DVec2::new(u as f64, v as f64), DVec2::ONE)))
                .collect()
        }
        PackRegion::PolygonTag => {
            let tag = |f: usize| -> String {
                let face = &mesh.faces[f];
                match options.polygon_tag {
                    PackTag::Material => face.material.clone(),
                    PackTag::Part => face.part.clone().unwrap_or_default(),
                    PackTag::SmoothingGroup => face.smooth.to_string(),
                    PackTag::SelectionSet => mesh
                        .attributes
                        .polygon_sets
                        .iter()
                        .find(|(_, set)| set.contains(&(f as u32)))
                        .map(|(n, _)| n.clone())
                        .unwrap_or_default(),
                }
            };
            let mut by_tag: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            for &f in &used {
                by_tag.entry(tag(f)).or_default().push(f);
            }
            // Each tag to its own tile along U from 1001 (ten to a row).
            by_tag
                .into_values()
                .enumerate()
                .map(|(k, faces)| {
                    (
                        faces,
                        (DVec2::new((k % 10) as f64, (k / 10) as f64), DVec2::ONE),
                    )
                })
                .collect()
        }
        _ => vec![(used.clone(), options.region_rect())],
    };
    for (faces, (origin, size)) in groups {
        let uvs = pack_faces(
            mesh, rows, &faces, selected, options, origin, size, obstacles,
        );
        write_back(&mut out, &target, &faces, &uvs);
    }
    if options.write_new {
        out.attributes.current_uv_map = Some(target);
    }
    Ok(out)
}

/// Pack `used`'s islands into the region `origin`, `size`: the UVs of each
/// used polygon, in order.
#[allow(clippy::too_many_arguments)]
fn pack_faces(
    mesh: &Mesh,
    rows: &[Vec<Option<DVec2>>],
    used: &[usize],
    selected: &[u32],
    options: &PackOptions,
    origin: DVec2,
    size: DVec2,
    obstacles: &[Vec<DVec2>],
) -> Vec<Vec<DVec2>> {
    let faces: Vec<Vec<(u32, DVec2)>> = used
        .iter()
        .map(|&f| {
            mesh.faces[f]
                .vertices
                .iter()
                .zip(&rows[f])
                .map(|(&p, q)| (p, q.unwrap()))
                .collect()
        })
        .collect();
    let (ids, count) = uv::pack::uv_islands(&faces);
    let mut uvs: Vec<Vec<DVec2>> = faces
        .iter()
        .map(|f| f.iter().map(|c| c.1).collect())
        .collect();

    let selected: BTreeSet<u32> = selected.iter().copied().collect();
    let mut movable = vec![selected.is_empty(); count];
    for (k, &f) in used.iter().enumerate() {
        if selected.contains(&(f as u32)) {
            movable[ids[k]] = true;
        }
    }

    // Stretch: each moving island's UV area matches its 3D area, relative to
    // the others (uniform per island, UV-Q19).
    if options.stretch && !options.keep_size {
        stretch_islands(mesh, used, &mut uvs, &ids, count, &movable);
    }

    // Lock Stacked: islands with identical UVs pack as one.
    let mut follow: Vec<Option<usize>> = vec![None; count];
    if options.lock_stacked {
        let key = |i: usize| -> Vec<(i64, i64)> {
            let mut pts: Vec<(i64, i64)> = uvs
                .iter()
                .zip(&ids)
                .filter(|(_, id)| **id == i)
                .flat_map(|(r, _)| {
                    r.iter()
                        .map(|p| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64))
                })
                .collect();
            pts.sort();
            pts.dedup();
            pts
        };
        let keys: Vec<_> = (0..count).map(key).collect();
        for i in 0..count {
            if follow[i].is_some() {
                continue;
            }
            for j in i + 1..count {
                if follow[j].is_none() && keys[i] == keys[j] {
                    follow[j] = Some(i);
                }
            }
        }
    }
    if !options.pack {
        return uvs;
    }

    // Islands that follow a stacked partner don't take part.
    let leader: Vec<usize> = (0..count).map(|i| follow[i].unwrap_or(i)).collect();
    let rotate = if !options.orient {
        uv::pack::RotationMethod::None
    } else {
        match options.direction {
            PackDirection::Horizontal => uv::pack::RotationMethod::AxisAlignedX,
            PackDirection::Vertical => uv::pack::RotationMethod::AxisAlignedY,
            PackDirection::Auto if options.find_best_box_rotation => uv::pack::RotationMethod::Any,
            PackDirection::Auto => uv::pack::RotationMethod::Cardinal,
        }
    };
    let between = options.between();
    // The packer leaves half the gap around each island, the layout's own
    // edge included, so only the rest of the edge inset is added.
    let inner_edge = (options.edge() - between * 0.5).max(0.0);
    // Pack into a unit square in the region's own frame, then map to the
    // region; the edge inset is kept clear around it.
    let aspect = size.y / size.x;
    let to_unit = |p: DVec2| (p - origin) / size;
    let from_unit = |p: DVec2| origin + p * size;
    let mut pack: Vec<uv::pack::PackIsland> = (0..count)
        .map(|i| uv::pack::PackIsland::new(aspect, !movable[leader[i]], i))
        .collect();
    for (k, row) in uvs.iter().enumerate() {
        let i = leader[ids[k]];
        let unit: Vec<DVec2> = row.iter().map(|&p| to_unit(p)).collect();
        pack[i].add_polygon(&unit);
    }
    let take: Vec<usize> = (0..count).filter(|&i| leader[i] == i).collect();
    let mut islands: Vec<uv::pack::PackIsland> = take.iter().map(|&i| pack[i].clone()).collect();
    // Background Constraint: obstacles inside the region stay put.
    for (k, poly) in obstacles.iter().enumerate() {
        let unit: Vec<DVec2> = poly.iter().map(|&p| to_unit(p)).collect();
        if unit
            .iter()
            .all(|p| p.x >= 0. && p.y >= 0. && p.x <= 1. && p.y <= 1.)
        {
            let mut island = uv::pack::PackIsland::new(aspect, true, count + k);
            island.add_polygon(&unit);
            islands.push(island);
        }
    }
    let any_fixed = islands.iter().any(|i| i.pinned);
    let params = uv::pack::PackParams {
        rotate_method: rotate,
        scale_to_fit: !options.keep_size,
        pin_method: if any_fixed {
            uv::pack::PinMethod::LockAll
        } else {
            uv::pack::PinMethod::None
        },
        margin: between * 0.5,
        margin_method: uv::pack::MarginMethod::Fraction,
        target_extent: 1.0 - 2.0 * inner_edge,
        target_aspect_y: 1.0,
        shape_method: if options.bounding_box {
            uv::pack::ShapeMethod::Aabb
        } else {
            uv::pack::ShapeMethod::Concave
        },
        ..Default::default()
    };
    let scale = uv::pack::pack_islands(&mut islands, &params);
    for (n, &i) in take.iter().enumerate() {
        pack[i] = islands[n].clone();
    }
    let _ = &islands;
    let inset = if any_fixed {
        DVec2::ZERO
    } else {
        DVec2::splat(inner_edge)
    };
    for (k, row) in uvs.iter_mut().enumerate() {
        let i = leader[ids[k]];
        let isl = &pack[i];
        if isl.pinned {
            continue;
        }
        let s = if isl.can_scale(&params) { scale } else { 1.0 };
        for p in row.iter_mut() {
            *p = from_unit(isl.transform(to_unit(*p), s, inset));
        }
    }
    uvs
}

fn stretch_islands(
    mesh: &Mesh,
    used: &[usize],
    uvs: &mut [Vec<DVec2>],
    ids: &[usize],
    count: usize,
    movable: &[bool],
) {
    let mut a3 = vec![0.0; count];
    let mut au = vec![0.0; count];
    let mut lo = vec![DVec2::splat(f64::MAX); count];
    let mut hi = vec![DVec2::splat(f64::MIN); count];
    for (k, &f) in used.iter().enumerate() {
        let i = ids[k];
        let v = &mesh.faces[f].vertices;
        let row = &uvs[k];
        for j in 1..v.len() - 1 {
            let (p0, p1, p2) = (
                mesh.vertices[v[0] as usize],
                mesh.vertices[v[j] as usize],
                mesh.vertices[v[j + 1] as usize],
            );
            a3[i] += 0.5 * (p1 - p0).cross(p2 - p0).length();
            au[i] += 0.5 * (row[j] - row[0]).perp_dot(row[j + 1] - row[0]);
        }
        for p in row {
            lo[i] = lo[i].min(*p);
            hi[i] = hi[i].max(*p);
        }
    }
    let t3: f64 = (0..count).map(|i| a3[i]).sum();
    let tu: f64 = (0..count).map(|i| au[i].abs()).sum();
    if t3 == 0.0 || tu == 0.0 {
        return;
    }
    let total = t3 / tu;
    for (k, row) in uvs.iter_mut().enumerate() {
        let i = ids[k];
        if !movable[i] || a3[i] == 0.0 || au[i] == 0.0 {
            continue;
        }
        let s = (a3[i] / au[i].abs() / total).sqrt();
        let c = (lo[i] + hi[i]) * 0.5;
        for p in row.iter_mut() {
            *p = c + (*p - c) * s;
        }
    }
}

fn write_back(out: &mut Mesh, map: &str, used: &[usize], uvs: &[Vec<DVec2>]) {
    if let Some(rows) = out.uv_maps.get_mut(map) {
        for (k, &f) in used.iter().enumerate() {
            rows[f] = uvs[k].iter().map(|&p| Some(p)).collect();
        }
    }
    finish_map(out, map);
}

impl crate::Editor {
    /// Per-polygon hidden flags of the active item.
    pub(crate) fn uv_hidden_faces(&self) -> Vec<bool> {
        let Some(item) = self.scene.active() else {
            return Vec::new();
        };
        crate::Protection::of(&item.mesh, &item.elements)
            .map(|p| p.face_hidden)
            .unwrap_or_else(|| vec![false; item.mesh.faces.len()])
    }

    pub(crate) fn publish_uv_edit(&mut self, mesh: Mesh, label: &str) -> Result<()> {
        let before = self.scene.clone();
        self.scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }

    /// UV Unwrap (UVW-01..13): every visible polygon of the active mesh,
    /// whatever polygons are selected (UVW-12).
    pub(crate) fn uv_unwrap(&mut self, options: UnwrapOptions) -> Result<()> {
        self.edge_insert_target("UV Unwrap")?;
        options.validate()?;
        self.uv_unwrap_preset = UnwrapOptions {
            uv_map: None,
            ..options.clone()
        };
        let hidden = self.uv_hidden_faces();
        let edges = if self.scene.selection.mode == crate::Mode::Edge {
            self.scene.selection.edges.clone()
        } else {
            Vec::new()
        };
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let map = match &options.uv_map {
            Some(name) if !name.trim().is_empty() => name.trim().to_owned(),
            _ => target_uv_map(mesh, None),
        };
        let out = unwrap_mesh(mesh, &map, &hidden, &edges, &options)?;
        self.publish_uv_edit(out, "UV Unwrap")
    }

    /// Select Connected in a UV view (UVV-04): every polygon (or its
    /// points or edges, by mode) of the UV islands holding the selection.
    pub(crate) fn select_uv_islands(&mut self, map: Option<String>) -> Result<()> {
        self.require_idle()?;
        let item = self.scene.active().ok_or("Select a mesh")?;
        let mesh = &item.mesh;
        let map = map.unwrap_or_else(|| target_uv_map(mesh, None));
        let rows = mesh
            .uv_maps
            .get(&map)
            .ok_or("The mesh has no UV map to follow")?;
        let used: Vec<usize> = (0..mesh.faces.len())
            .filter(|&f| rows[f].iter().all(Option::is_some))
            .collect();
        let faces: Vec<Vec<(u32, DVec2)>> = used
            .iter()
            .map(|&f| {
                mesh.faces[f]
                    .vertices
                    .iter()
                    .zip(&rows[f])
                    .map(|(&p, q)| (p, q.unwrap()))
                    .collect()
            })
            .collect();
        let (ids, count) = uv::pack::uv_islands(&faces);
        let sel = &self.scene.selection;
        let points: BTreeSet<u32> = sel.vertices.iter().copied().collect();
        let edges: BTreeSet<(u32, u32)> = sel.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
        let polys: BTreeSet<u32> = sel.faces.iter().copied().collect();
        let mode = sel.mode;
        let mut hit = vec![false; count];
        for (k, &f) in used.iter().enumerate() {
            let v = &mesh.faces[f].vertices;
            let touches = match mode {
                crate::Mode::Polygon => polys.contains(&(f as u32)),
                crate::Mode::Vertex => v.iter().any(|p| points.contains(p)),
                crate::Mode::Edge => {
                    (0..v.len()).any(|i| edges.contains(&edge_key(v[i], v[(i + 1) % v.len()])))
                }
                crate::Mode::Item => false,
            };
            if touches {
                hit[ids[k]] = true;
            }
        }
        let chosen: Vec<usize> = used
            .iter()
            .enumerate()
            .filter(|(k, _)| hit[ids[*k]])
            .map(|(_, &f)| f)
            .collect();
        let before = self.scene.clone();
        let sel = &mut self.scene.selection;
        match mode {
            crate::Mode::Polygon => {
                let mut all: BTreeSet<u32> = sel.faces.iter().copied().collect();
                all.extend(chosen.iter().map(|&f| f as u32));
                sel.faces = all.into_iter().collect();
            }
            crate::Mode::Vertex => {
                let mut all: BTreeSet<u32> = sel.vertices.iter().copied().collect();
                for &f in &chosen {
                    all.extend(mesh_face(&before, f));
                }
                sel.vertices = all.into_iter().collect();
            }
            crate::Mode::Edge => {
                let mut all: BTreeSet<(u32, u32)> =
                    sel.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
                for &f in &chosen {
                    let v = mesh_face(&before, f);
                    for i in 0..v.len() {
                        all.insert(edge_key(v[i], v[(i + 1) % v.len()]));
                    }
                }
                sel.edges = all.into_iter().collect();
            }
            crate::Mode::Item => return Err("Select Connected needs a component mode".into()),
        }
        if self.scene != before {
            self.record_selection("Select Connected".into(), before);
        }
        Ok(())
    }

    /// UV Relax as one command (BRQ-110): the tool's run with `options`,
    /// one undo step. Iterations 0 changes nothing and records nothing,
    /// as the tool at 0 does (UVS-1 §3).
    pub(crate) fn uv_relax_once(&mut self, options: crate::uv_relax::RelaxOptions) -> Result<()> {
        self.edge_insert_target("UV Relax")?;
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        if let Some(name) = options.uv_map.as_deref()
            && !mesh.uv_maps.contains_key(name)
        {
            return Err(format!("The mesh has no UV map to relax named {name}"));
        }
        let out = crate::uv_relax::relax_mesh(&self.scene, &options, None)?;
        if *self.scene.active().ok_or("Select a mesh")?.mesh == out {
            return Ok(());
        }
        self.publish_uv_edit(out, "UV Relax")
    }

    /// Pack UVs (UVP-01..09): the selected polygons' islands, or all.
    pub(crate) fn pack_uvs(&mut self, options: PackOptions) -> Result<()> {
        self.edge_insert_target("Pack UVs")?;
        options.validate()?;
        self.uv_pack_preset = options.clone();
        let hidden = self.uv_hidden_faces();
        let selected = if self.scene.selection.mode == crate::Mode::Polygon {
            self.scene.selection.faces.clone()
        } else {
            Vec::new()
        };
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        if mesh.uv_maps.is_empty() {
            return Err("Select a UV map to pack".into());
        }
        let map = target_uv_map(mesh, None);
        if options.region == PackRegion::ByItem {
            return self.pack_by_item(&map, &options);
        }
        // Background Constraint: the other shown meshes' islands in a map
        // of this name are obstacles (UVP-P03).
        let active = self.scene.active().map(|i| i.id);
        let obstacles: Vec<Vec<DVec2>> = if options.background {
            self.scene
                .shown_active_meshes()
                .into_iter()
                .chain(self.scene.items.iter().map(|i| i.id))
                .collect::<BTreeSet<u64>>()
                .into_iter()
                .filter(|&id| Some(id) != active)
                .filter_map(|id| self.scene.item(id))
                .filter(|i| i.visible)
                .filter_map(|i| i.mesh.uv_maps.get(&map))
                .flat_map(|rows| {
                    rows.iter()
                        .filter_map(|r| r.iter().copied().collect::<Option<Vec<DVec2>>>())
                })
                .collect()
        } else {
            Vec::new()
        };
        let mesh = &self.scene.active().ok_or("Select a mesh")?.mesh;
        let out = pack_mesh_with(mesh, &map, &hidden, &selected, &options, &obstacles)?;
        self.publish_uv_edit(out, "Pack UVs")
    }

    /// Pack Region By Item: each active mesh's islands to their own UDIM
    /// tile, from 1001 in the order the meshes are listed; with Pack UVs to
    /// New Map each mesh's new map is named after it. One undo step.
    fn pack_by_item(&mut self, map: &str, options: &PackOptions) -> Result<()> {
        let mut ids = self.scene.active_meshes();
        if ids.is_empty() {
            ids.extend(self.scene.active().map(|i| i.id));
        }
        let before = self.scene.clone();
        for (k, id) in ids.iter().enumerate() {
            let Some(item) = self.scene.item(*id) else {
                continue;
            };
            if !item.mesh.uv_maps.contains_key(map) {
                continue;
            }
            let hidden = crate::Protection::of(&item.mesh, &item.elements)
                .map(|p| p.face_hidden)
                .unwrap_or_default();
            let o = PackOptions {
                region: PackRegion::Udim,
                udim: 1001 + k as u32,
                new_name: if options.write_new && options.new_name.trim().is_empty() {
                    item.name.clone()
                } else {
                    options.new_name.clone()
                },
                ..options.clone()
            };
            let out = pack_mesh(&item.mesh, map, &hidden, &[], &o)?;
            self.scene.item_mut(*id).ok_or("Missing mesh")?.mesh = out.into();
        }
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record("Pack UVs".into(), before);
        Ok(())
    }
}

type Args = serde_json::Map<String, serde_json::Value>;

fn arg_bool(a: &Args, k: &str, v: &mut bool) -> Result<()> {
    if let Some(b) = crate::command_exec::opt_bool(a, k)? {
        *v = b;
    }
    Ok(())
}

fn arg_num(a: &Args, k: &str, v: &mut f64) -> Result<()> {
    if let Some(x) = a.get(k).filter(|x| !x.is_null()) {
        *v = crate::command_exec::num(x)?;
    }
    Ok(())
}

fn arg_u32(a: &Args, k: &str, v: &mut u32) -> Result<()> {
    if let Some(x) = a.get(k).filter(|x| !x.is_null()) {
        *v = crate::command_exec::as_u32(x)?;
    }
    Ok(())
}

fn arg_word<'a>(a: &'a Args, k: &str) -> Result<Option<&'a str>> {
    crate::command_exec::opt_str(a, k)
}

/// An enum argument: one of `words`, or its index (the reference's
/// integers are aliases).
fn arg_choice(a: &Args, k: &str, words: &[&str]) -> Result<Option<usize>> {
    match a.get(k) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .map(|i| i as usize)
            .filter(|&i| i < words.len())
            .map(Some)
            .ok_or_else(|| format!("{k} must be 0 to {}", words.len() - 1)),
        Some(serde_json::Value::String(w)) => {
            if let Ok(i) = w.parse::<usize>()
                && i < words.len()
            {
                return Ok(Some(i));
            }
            words
                .iter()
                .position(|x| x.eq_ignore_ascii_case(w))
                .map(Some)
                .ok_or_else(|| format!("{k} must be one of {}", words.join(", ")))
        }
        Some(_) => Err(format!("{k} must be a word or a number")),
    }
}

fn refuse_on(a: &Args, k: &str, what: &str) -> Result<()> {
    if crate::command_exec::opt_bool(a, k)? == Some(true) {
        return Err(format!("{what} isn't built yet"));
    }
    Ok(())
}

/// `uv.unwrap` arguments over `base` (the last-used values). Returns the
/// options and the map named by `name`, if any.
pub fn unwrap_args(a: &Args, base: &UnwrapOptions) -> Result<UnwrapOptions> {
    let mut o = base.clone();
    arg_u32(a, "iter", &mut o.iterations)?;
    arg_num(a, "gaps", &mut o.relative_gap)?;
    arg_bool(a, "matr", &mut o.segment_by_material)?;
    arg_bool(a, "part", &mut o.segment_by_part)?;
    const PROJ: [InitialProjection; 5] = [
        InitialProjection::Planar,
        InitialProjection::Cylindrical,
        InitialProjection::Spherical,
        InitialProjection::UseCurrentMap,
        InitialProjection::GroupNormal,
    ];
    if let Some(i) = arg_choice(
        a,
        "project",
        &[
            "planar",
            "cylindrical",
            "spherical",
            "current",
            "groupNormal",
        ],
    )? {
        o.initial_projection = PROJ[i];
    }
    let uv = [UvAxis::U, UvAxis::V];
    let axes = [Axis3::X, Axis3::Y, Axis3::Z];
    if let Some(i) = arg_choice(a, "pinning", &["u", "v"])? {
        o.pinning_axis = uv[i];
    }
    if let Some(i) = arg_choice(a, "axis", &["x", "y", "z"])? {
        o.projection_axis = axes[i];
    }
    let mut angle = o.offset_angle.to_radians();
    arg_num(a, "angle", &mut angle)?;
    o.offset_angle = angle.to_degrees();
    arg_bool(a, "seal", &mut o.seal_holes)?;
    arg_bool(a, "layout", &mut o.update_layout)?;
    if let Some(i) = arg_choice(a, "mode", &["conformal", "angleBased"])? {
        o.method = [UnwrapSolver::Conformal, UnwrapSolver::AngleBased][i];
    }
    refuse_on(a, "symmetrize", "Symmetrize")?;
    if let Some(i) = arg_choice(a, "symmAxis", &["x", "y", "z"])? {
        o.symmetry_axis = axes[i];
    }
    if let Some(i) = arg_choice(a, "symmComp", &["u", "v"])? {
        o.mirror_axis = uv[i];
    }
    if let Some(i) = arg_choice(a, "seam", &["selectedEdges", "seamMap", "selectionSet"])? {
        o.seams = match i {
            1 => SeamSource::SeamMap(arg_word(a, "seamName")?.unwrap_or_default().to_owned()),
            2 => SeamSource::SelectionSet(arg_word(a, "pickName")?.unwrap_or_default().to_owned()),
            _ => SeamSource::SelectedEdges,
        };
    }
    o.uv_map = arg_word(a, "name")?.map(str::to_owned);
    o.validate()?;
    Ok(o)
}

/// `uv.relax` arguments (BRQ-110) over the tool's Reset values, named as
/// the reference's tool attributes: mode, iter, seal, boundary, area,
/// uniform, noEdge, name. Interactive pins need clicks in a UV view, so
/// the command form has no `live`.
pub fn relax_args(a: &Args) -> Result<crate::uv_relax::RelaxOptions> {
    use crate::uv_relax::{RelaxBoundary, RelaxMode};
    let mut o = crate::uv_relax::RelaxOptions::default();
    if let Some(i) = arg_choice(
        a,
        "mode",
        &["spring", "conformal", "angleBased", "adaptive"],
    )? {
        o.mode = RelaxMode::ALL[i];
    }
    arg_u32(a, "iter", &mut o.iterations)?;
    arg_bool(a, "seal", &mut o.seal_holes)?;
    if let Some(i) = arg_choice(
        a,
        "boundary",
        &["smooth", "locked", "maintain", "straighten"],
    )? {
        o.boundary = RelaxBoundary::ALL[i];
    }
    arg_num(a, "area", &mut o.area_weight)?;
    arg_bool(a, "uniform", &mut o.uniform_spans)?;
    arg_bool(a, "noEdge", &mut o.ignore_edge_selection)?;
    o.uv_map = arg_word(a, "name")?.map(str::to_owned);
    o.validate()?;
    Ok(o)
}

/// `uv.pack` arguments over `base` (the dialog's last values).
pub fn pack_args(a: &Args, base: &PackOptions) -> Result<PackOptions> {
    let mut o = base.clone();
    arg_bool(a, "pack", &mut o.pack)?;
    arg_bool(a, "stretch", &mut o.stretch)?;
    arg_bool(a, "orient", &mut o.orient)?;
    arg_bool(a, "keepSize", &mut o.keep_size)?;
    if let Some(i) = arg_choice(a, "direction", &["horizontal", "vertical", "auto"])? {
        o.direction = [
            PackDirection::Horizontal,
            PackDirection::Vertical,
            PackDirection::Auto,
        ][i];
    }
    arg_num(a, "gaps", &mut o.gaps)?;
    arg_bool(a, "byPixel", &mut o.gaps_by_pixel)?;
    arg_num(a, "gapsByPixel", &mut o.pixel_size)?;
    arg_bool(a, "bbox", &mut o.bounding_box)?;
    arg_bool(a, "stack", &mut o.lock_stacked)?;
    const REGIONS: [PackRegion; 7] = [
        PackRegion::Normalized,
        PackRegion::Nearest,
        PackRegion::Udim,
        PackRegion::Manual,
        PackRegion::Tiles,
        PackRegion::PolygonTag,
        PackRegion::ByItem,
    ];
    if let Some(i) = arg_choice(
        a,
        "region",
        &[
            "normalized",
            "nearest",
            "udim",
            "manual",
            "tiles",
            "polygonTag",
            "byItem",
        ],
    )? {
        o.region = REGIONS[i];
    }
    arg_u32(a, "udim", &mut o.udim)?;
    arg_num(a, "regionX", &mut o.region_uv.x)?;
    arg_num(a, "regionY", &mut o.region_uv.y)?;
    arg_num(a, "regionW", &mut o.region_size.x)?;
    arg_num(a, "regionH", &mut o.region_size.y)?;
    arg_u32(a, "tileU", &mut o.tiles[0])?;
    arg_u32(a, "tileV", &mut o.tiles[1])?;
    if let Some(i) = arg_choice(
        a,
        "polygonTag",
        &["material", "part", "smoothingGroup", "selectionSet"],
    )? {
        o.polygon_tag = [
            PackTag::Material,
            PackTag::Part,
            PackTag::SmoothingGroup,
            PackTag::SelectionSet,
        ][i];
    }
    arg_bool(a, "background", &mut o.background)?;
    arg_bool(a, "writeNew", &mut o.write_new)?;
    if let Some(n) = crate::command_exec::opt_str(a, "newName")? {
        o.new_name = n.to_owned();
    }
    arg_bool(a, "bestrot", &mut o.find_best_box_rotation)?;
    o.validate()?;
    Ok(o)
}
