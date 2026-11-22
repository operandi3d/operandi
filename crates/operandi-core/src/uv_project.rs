//! Project (E2, BEHAVIOR-UV-TOOLS §2, UVT-10..19): writes UVs by projecting
//! polygons from 3D onto a flat shape. One engine; Project and Project from
//! View are its two tools.
//!
//! - **Planar** (UVT-11): seen straight down the Axis.
//! - **Cylindrical** (UVT-12): U once round the Axis from Seam Angle, V
//!   along it; a polygon across the seam is cut so it doesn't span the tile.
//! - **Spherical** (UVT-13): U as Cylindrical, V pole to pole.
//! - **Cubic** (UVT-14): each polygon flat along the one of six directions
//!   it faces most, never mirrored.
//! - **Barycentric** (UVT-15): every polygon fills the tile by itself.
//! - **View** (UVT-19, UVE-1 §4): each point's place on screen.
//!
//! Atlas and Atlas 2 (UVT-16/17) are the xatlas port, built separately.
//!
//! Provisional (docs/SPECIFICATION_QUESTIONS.md UVT-Q02, UVT-Q09,
//! UVT-P01..P04): starting values Planar on Y; Automatic keeps the aspect,
//! the longer side spanning the tile from its lower-left corner (Planar,
//! Cubic, View) while Cylindrical and Spherical fill 0..1 in V over the
//! selection's height; points are measured in the item's own space (View:
//! world space, as the view sees them).
use crate::topology::edge_key;
use crate::uv_tools::Axis3;
use crate::{Mesh, Mode, Result, Scene};
use glam::{DMat3, DMat4, DVec2, DVec3};
use std::collections::BTreeSet;

/// Projection Type (UVT-10 order; Atlas types come later).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProjectionKind {
    /// UVT-Q02 (build guess): the starting type.
    #[default]
    Planar,
    Cylindrical,
    Spherical,
    Barycentric,
    Cubic,
    /// Automatic charts, many and small (UVT-16; uv_charts.rs).
    Atlas,
    /// Automatic charts, fewer and better shaped (UVT-17).
    Atlas2,
}
impl ProjectionKind {
    /// In UVT-10 order.
    pub const ALL: [Self; 7] = [
        Self::Planar,
        Self::Cylindrical,
        Self::Spherical,
        Self::Atlas,
        Self::Barycentric,
        Self::Atlas2,
        Self::Cubic,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Planar => "Planar",
            Self::Cylindrical => "Cylindrical",
            Self::Spherical => "Spherical",
            Self::Barycentric => "Barycentric",
            Self::Cubic => "Cubic",
            Self::Atlas => "Atlas",
            Self::Atlas2 => "Atlas 2",
        }
    }
    pub fn is_atlas(self) -> bool {
        matches!(self, Self::Atlas | Self::Atlas2)
    }
    /// Whether Axis means anything for this type.
    pub fn uses_axis(self) -> bool {
        !matches!(self, Self::Barycentric | Self::Atlas | Self::Atlas2)
    }
    /// Whether Seam Angle means anything for this type.
    pub fn uses_seam(self) -> bool {
        matches!(self, Self::Cylindrical | Self::Spherical)
    }
    /// Whether Size, Center and Rotation (Settings) mean anything.
    pub fn uses_frame(self) -> bool {
        !matches!(self, Self::Barycentric | Self::Atlas | Self::Atlas2)
    }
}

/// Project's Tool Properties (UVT §2.3, in order).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ProjectOptions {
    /// The map written; `None` is the mesh's selected map ("Texture" made
    /// when there is none, UVT-01).
    #[serde(skip)]
    pub uv_map: Option<String>,
    pub kind: ProjectionKind,
    /// UVT-Q02 (build guess): Y.
    pub axis: Axis3,
    /// Settings: Automatic (false) sizes the shape to the selection's
    /// bounds; Manual (true) uses Size, Center and Rotation.
    pub manual: bool,
    pub size: DVec3,
    pub center: DVec3,
    /// Degrees about X, Y, Z (applied X, then Y, then Z).
    pub rotation: DVec3,
    /// Degrees: where Cylindrical and Spherical wrap, turned about Axis.
    pub seam_angle: f64,
    /// Atlas types (UVT §2.3): Relative Gap Size, segment borders,
    /// Orientation, Atlas 2's Segments and Iterations.
    pub atlas: crate::uv_charts::AtlasChartOptions,
}
impl Default for ProjectOptions {
    fn default() -> Self {
        Self {
            uv_map: None,
            kind: ProjectionKind::Planar,
            axis: Axis3::Y,
            manual: false,
            size: DVec3::ONE,
            center: DVec3::ZERO,
            rotation: DVec3::ZERO,
            seam_angle: 0.,
            atlas: Default::default(),
        }
    }
}
impl ProjectOptions {
    pub fn validate(&self) -> Result<()> {
        if !self.size.is_finite() || self.size.min_element() <= 1e-12 {
            return Err("Project Size must be more than zero on every axis".into());
        }
        if !self.center.is_finite() || !self.rotation.is_finite() || !self.seam_angle.is_finite() {
            return Err("Project Center, Rotation and Seam Angle must be numbers".into());
        }
        Ok(())
    }
}

/// Project from View's options (UVT-19). `view` is where the last click
/// was made; until then the tool has nothing to project.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViewProjectOptions {
    #[serde(skip)]
    pub uv_map: Option<String>,
    /// Fit the result into the 0–1 tile (on, build guess).
    pub auto_scale: bool,
    /// Leave out polygons facing away from the view (off, build guess).
    pub hidden_face: bool,
    #[serde(skip)]
    pub view: Option<ViewFrame>,
}
impl Default for ViewProjectOptions {
    fn default() -> Self {
        Self {
            uv_map: None,
            auto_scale: true,
            hidden_face: false,
            view: None,
        }
    }
}

/// The view a Project from View click was made in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewFrame {
    /// World point → the view's screen, x right and y up, the view's
    /// height spanning −1..1 (so its width spans more on a wide view and a
    /// round thing stays round). `project_point3` divides by depth.
    pub matrix: DMat4,
    /// The eye (perspective) or `None` for an orthographic view.
    pub eye: Option<DVec3>,
    /// The direction the view looks.
    pub forward: DVec3,
}

/// The polygons a UV tool writes (UVT-02, UVT-10): the selected polygons;
/// in Edge and Vertex mode those whose corners are all selected; all
/// visible polygons when nothing is selected. Hidden polygons and
/// polylines never.
pub fn target_faces(scene: &Scene) -> Result<Vec<u32>> {
    let item = scene.active().ok_or("Select a mesh")?;
    let mesh = &item.mesh;
    let hidden = crate::Protection::of(mesh, &item.elements)
        .map(|p| p.face_hidden)
        .unwrap_or_default();
    let usable = |f: usize| !hidden.get(f).copied().unwrap_or(false) && !mesh.is_polyline(f);
    let s = &scene.selection;
    let chosen: Option<Vec<u32>> = match s.mode {
        Mode::Polygon if !s.faces.is_empty() => Some(s.faces.clone()),
        Mode::Vertex if !s.vertices.is_empty() => {
            let on: BTreeSet<u32> = s.vertices.iter().copied().collect();
            Some(
                (0..mesh.faces.len() as u32)
                    .filter(|&f| {
                        mesh.faces[f as usize]
                            .vertices
                            .iter()
                            .all(|v| on.contains(v))
                    })
                    .collect(),
            )
        }
        Mode::Edge if !s.edges.is_empty() => {
            let on: BTreeSet<(u32, u32)> = s.edges.iter().map(|&(a, b)| edge_key(a, b)).collect();
            Some(
                (0..mesh.faces.len() as u32)
                    .filter(|&f| {
                        let v = &mesh.faces[f as usize].vertices;
                        (0..v.len()).all(|i| on.contains(&edge_key(v[i], v[(i + 1) % v.len()])))
                    })
                    .collect(),
            )
        }
        _ => None,
    };
    let faces: Vec<u32> = match chosen {
        Some(list) => list.into_iter().filter(|&f| usable(f as usize)).collect(),
        None => (0..mesh.faces.len() as u32)
            .filter(|&f| usable(f as usize))
            .collect(),
    };
    if faces.is_empty() {
        return Err("No visible polygons to project".into());
    }
    Ok(faces)
}

/// (side U, up along the axis, side V) for an axis, as `uv_tools`' Initial
/// Projection frames.
fn axis_frame(axis: Axis3) -> (DVec3, DVec3, DVec3) {
    match axis {
        Axis3::X => (DVec3::Y, DVec3::X, DVec3::Z),
        Axis3::Y => (DVec3::Z, DVec3::Y, DVec3::X),
        Axis3::Z => (DVec3::X, DVec3::Z, DVec3::Y),
    }
}

/// Planar U and V seen from the positive end of `axis` (as Unwrap's
/// Planar Initial Projection, UV-Q22): Y is the Top view (U = X, V = −Z).
fn planar(p: DVec3, axis: Axis3) -> DVec2 {
    match axis {
        Axis3::X => DVec2::new(-p.z, p.y),
        Axis3::Y => DVec2::new(p.x, -p.z),
        Axis3::Z => DVec2::new(p.x, p.y),
    }
}

fn rotation(degrees: DVec3) -> DMat3 {
    let r = degrees * (std::f64::consts::PI / 180.);
    DMat3::from_rotation_z(r.z) * DMat3::from_rotation_y(r.y) * DMat3::from_rotation_x(r.x)
}

/// Bounds of `points`.
fn bounds(points: impl Iterator<Item = DVec3>) -> (DVec3, DVec3) {
    points.fold((DVec3::MAX, DVec3::MIN), |(lo, hi), p| {
        (lo.min(p), hi.max(p))
    })
}

/// Writes `uvs` (one row per entry of `faces`) into `map` of a copy of
/// `mesh`, making the map when missing (UVT-01). Other polygons keep theirs.
pub(crate) fn write_rows(
    mesh: &Mesh,
    map: &str,
    faces: &[u32],
    uvs: &[Vec<DVec2>],
) -> Result<Mesh> {
    let mut out = mesh.clone();
    let rows = out.uv_maps.entry(map.to_owned()).or_insert_with(|| {
        mesh.faces
            .iter()
            .map(|f| vec![None; f.vertices.len()])
            .collect()
    });
    for (&f, row) in faces.iter().zip(uvs) {
        if row.iter().any(|p| !p.is_finite()) {
            return Err("Projected UVs must be numbers".into());
        }
        rows[f as usize] = row.iter().map(|&p| Some(p)).collect();
    }
    out.attributes.point_uvs.remove(map);
    if out.attributes.uv_records.contains_key(map) {
        let records = crate::corner_uv::records_from_uvs(&out.faces, &out.uv_maps[map]);
        out.attributes.uv_records.insert(map.to_owned(), records);
    }
    if out.attributes.current_uv_map.is_none() {
        out.attributes.current_uv_map = Some(map.to_owned());
    }
    Ok(out)
}

/// Scale and move `rows` so their bounds' longer side spans 0..1 from the
/// tile's lower-left corner, aspect kept (UVT-Q09, build guess).
fn fit_keep_aspect(rows: &mut [Vec<DVec2>]) {
    let (lo, hi) = rows
        .iter()
        .flatten()
        .fold((DVec2::MAX, DVec2::MIN), |(lo, hi), &p| {
            (lo.min(p), hi.max(p))
        });
    let span = (hi - lo).max_element();
    if span.is_nan() || span <= 1e-12 {
        return;
    }
    for p in rows.iter_mut().flatten() {
        *p = (*p - lo) / span;
    }
}

/// A polygon's wrapped U values made continuous: one that straddles the
/// seam gets 1 added to its low side, so it doesn't span the tile (UVT-12).
fn unwrap_seam(row: &mut [DVec2]) {
    let (lo, hi) = row.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.x), hi.max(p.x))
    });
    if hi - lo > 0.5 {
        for p in row.iter_mut() {
            if p.x < 0.5 {
                p.x += 1.;
            }
        }
    }
}

/// The projection of `faces` of the scene's active mesh into its map.
pub fn project_mesh(scene: &Scene, faces: &[u32], o: &ProjectOptions) -> Result<Mesh> {
    o.validate()?;
    let item = scene.active().ok_or("Select a mesh")?;
    let mesh = &item.mesh;
    let map = crate::uv_tools::target_uv_map(mesh, o.uv_map.as_deref());
    let corners = |f: u32| -> Vec<DVec3> {
        mesh.faces[f as usize]
            .vertices
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect()
    };
    // The shape's frame: Manual's own, or the selection's bounds.
    let (centre, size, turn) = if o.manual {
        (o.center, o.size, rotation(o.rotation))
    } else {
        let (lo, hi) = bounds(faces.iter().flat_map(|&f| corners(f)));
        (
            (lo + hi) * 0.5,
            (hi - lo).max(DVec3::splat(1e-9)),
            DMat3::IDENTITY,
        )
    };
    let local = |p: DVec3| turn.transpose() * (p - centre);
    let seam = o.seam_angle.to_radians();
    let tau = std::f64::consts::TAU;
    let mut rows: Vec<Vec<DVec2>> = Vec::with_capacity(faces.len());
    match o.kind {
        ProjectionKind::Planar => {
            for &f in faces {
                rows.push(
                    corners(f)
                        .into_iter()
                        .map(|p| planar(local(p), o.axis))
                        .collect(),
                );
            }
            if o.manual {
                // Size spans the tile; Center lands on its middle.
                let s = planar(size, o.axis).abs();
                for p in rows.iter_mut().flatten() {
                    *p = *p / s.max(DVec2::splat(1e-12)) + DVec2::splat(0.5);
                }
            } else {
                fit_keep_aspect(&mut rows);
            }
        }
        ProjectionKind::Cylindrical | ProjectionKind::Spherical => {
            let (a, up, b) = axis_frame(o.axis);
            let height = size.dot(up).abs().max(1e-12);
            for &f in faces {
                let pts: Vec<DVec3> = corners(f).into_iter().map(local).collect();
                // Corners on the axis take the polygon's own direction.
                let mid = pts.iter().sum::<DVec3>() / pts.len().max(1) as f64;
                let angle_of = |p: DVec3| {
                    let q = if (p - up * p.dot(up)).length() < 1e-9 {
                        mid
                    } else {
                        p
                    };
                    let ang = q.dot(b).atan2(q.dot(a)) - seam;
                    (ang / tau).rem_euclid(1.)
                };
                let mut row: Vec<DVec2> = pts
                    .iter()
                    .map(|&p| {
                        let v = if o.kind == ProjectionKind::Cylindrical {
                            p.dot(up) / height + 0.5
                        } else {
                            let r = p.length().max(1e-12);
                            (p.dot(up) / r).clamp(-1., 1.).asin() / std::f64::consts::PI + 0.5
                        };
                        DVec2::new(angle_of(p), v)
                    })
                    .collect();
                unwrap_seam(&mut row);
                rows.push(row);
            }
        }
        ProjectionKind::Cubic => {
            // Each polygon along the direction it faces most, laid out so it
            // reads the right way round from outside (UVT-14).
            for &f in faces {
                let pts: Vec<DVec3> = corners(f).into_iter().map(local).collect();
                let n = turn.transpose() * mesh.normal(f as usize);
                let k = n.abs().max_position();
                let positive = n[k] >= 0.;
                let row = pts
                    .iter()
                    .map(|&p| match (k, positive) {
                        (0, true) => DVec2::new(-p.z, p.y),
                        (0, false) => DVec2::new(p.z, p.y),
                        (1, true) => DVec2::new(p.x, -p.z),
                        (1, false) => DVec2::new(p.x, p.z),
                        (_, true) => DVec2::new(p.x, p.y),
                        (_, false) => DVec2::new(-p.x, p.y),
                    })
                    .collect();
                rows.push(row);
            }
            if o.manual {
                let s = size.max_element().max(1e-12);
                for p in rows.iter_mut().flatten() {
                    *p = *p / s + DVec2::splat(0.5);
                }
            } else {
                fit_keep_aspect(&mut rows);
            }
        }
        ProjectionKind::Barycentric => {
            for &f in faces {
                rows.push(barycentric(mesh.faces[f as usize].vertices.len()));
            }
        }
        ProjectionKind::Atlas | ProjectionKind::Atlas2 => {
            let o2 = crate::uv_charts::AtlasChartOptions {
                fewer: o.kind == ProjectionKind::Atlas2,
                ..o.atlas
            };
            let charted = crate::uv_charts::atlas_uvs(mesh, faces, &o2)?;
            // Pack the charts alone (other polygons aren't obstacles: they
            // keep their UVs and may overlap), at one scale.
            let mut temp = Mesh {
                vertices: mesh.vertices.clone(),
                faces: mesh.faces.clone(),
                ..Default::default()
            };
            let mut temp_rows: Vec<Vec<Option<DVec2>>> = mesh
                .faces
                .iter()
                .map(|f| vec![None; f.vertices.len()])
                .collect();
            for (&f, row) in faces.iter().zip(&charted) {
                temp_rows[f as usize] = row.iter().map(|&p| Some(p)).collect();
            }
            temp.uv_maps.insert("Atlas".into(), temp_rows);
            let packed = crate::uv_tools::pack_mesh(
                &temp,
                "Atlas",
                &[],
                &[],
                &crate::uv_tools::PackOptions {
                    stretch: false,
                    orient: o.kind == ProjectionKind::Atlas2,
                    gaps: o2.relative_gap,
                    // UVA-P04: charts pack by their boxes, so the live tool
                    // stays quick (the shape packer's search is many
                    // times slower on hundreds of charts).
                    bounding_box: true,
                    ..Default::default()
                },
            )?;
            let packed = &packed.uv_maps["Atlas"];
            for &f in faces {
                rows.push(
                    packed[f as usize]
                        .iter()
                        .map(|p| p.unwrap_or_default())
                        .collect(),
                );
            }
        }
    }
    let out = write_rows(mesh, &map, faces, &rows)?;
    out.validate()?;
    Ok(out)
}

/// UVT-15: one polygon of `n` corners filling the tile. Quads hit the four
/// corners, triangles take the bottom corners and the top middle, larger
/// polygons sit on the tile's inscribed circle from its lower left.
pub fn barycentric(n: usize) -> Vec<DVec2> {
    match n {
        4 => vec![
            DVec2::new(0., 0.),
            DVec2::new(1., 0.),
            DVec2::new(1., 1.),
            DVec2::new(0., 1.),
        ],
        3 => vec![DVec2::new(0., 0.), DVec2::new(1., 0.), DVec2::new(0.5, 1.)],
        _ => (0..n)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64 - 0.75 * std::f64::consts::PI;
                DVec2::new(0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin())
            })
            .collect(),
    }
}

/// Project from View (UVT-19): each corner's place in the clicked view.
pub fn view_project_mesh(scene: &Scene, faces: &[u32], o: &ViewProjectOptions) -> Result<Mesh> {
    let view = o.view.ok_or("Click in a 3D view to project from it")?;
    let item = scene.active().ok_or("Select a mesh")?;
    let mesh = &item.mesh;
    let world = scene.world_matrix(item.id)?;
    let map = crate::uv_tools::target_uv_map(mesh, o.uv_map.as_deref());
    let normal_matrix = DMat3::from_mat4(world).inverse().transpose();
    let mut kept = Vec::new();
    let mut rows = Vec::new();
    for &f in faces {
        let face = &mesh.faces[f as usize];
        let pts: Vec<DVec3> = face
            .vertices
            .iter()
            .map(|&v| world.transform_point3(mesh.vertices[v as usize]))
            .collect();
        if o.hidden_face {
            let n = normal_matrix * mesh.normal(f as usize);
            let mid = pts.iter().sum::<DVec3>() / pts.len().max(1) as f64;
            let towards = view.eye.map_or(-view.forward, |eye| eye - mid);
            if n.dot(towards) <= 0. {
                continue;
            }
        }
        let row: Vec<DVec2> = pts
            .iter()
            .map(|&p| {
                let q = view.matrix.project_point3(p);
                // The view's height is the tile; its middle the tile's.
                DVec2::new((q.x + 1.) * 0.5, (q.y + 1.) * 0.5)
            })
            .collect();
        kept.push(f);
        rows.push(row);
    }
    if kept.is_empty() {
        return Err("Every polygon faces away from this view".into());
    }
    if o.auto_scale {
        fit_keep_aspect(&mut rows);
    }
    let out = write_rows(mesh, &map, &kept, &rows)?;
    out.validate()?;
    Ok(out)
}
