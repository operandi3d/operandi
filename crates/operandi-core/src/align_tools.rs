//! Create panel Align group (BEHAVIOR-ALIGN.md): Ground Align and Quick
//! Align. Both work on the active mesh's points in Vertices, Edges or
//! Polygons mode, and each apply is one undo step (ALN-03).
//!
//! Ground Align (GAL-05..10):
//! - Drop To Ground: the points move straight down in world Y until the
//!   lowest one sits at Y = 0. Drop To Origin: the bottom-centre of their
//!   world box lands on 0,0,0. Rotate To Ground: they turn about their
//!   centre so the selected polygons face straight down (no drop, GAL-09).
//! - What moves: nothing selected, the whole mesh; otherwise the connected
//!   pieces the selection touches (Connected Geometry on, the default), or
//!   only the selected points (off).
//! - By Selection (Shift-click): the lowest point of the selection, not of
//!   the whole piece, decides the drop (GAL-10).
//!
//! Quick Align (QAL-04..07):
//! - Position: the selection moves along one world axis so its centre has
//!   the target's coordinate on that axis. With Flatten (Ctrl), every
//!   selected point gets that coordinate. With Connected (Shift), the whole
//!   connected piece moves by the same amount.
//! - Orientation: the selection turns about its own centre so its facing
//!   matches the target's normal (`opposite` faces away).
//! - Selection Axis Mode: each selected island aligns on its own
//!   (Selection Island, the default) or the whole selection as one (Total
//!   Selection). Mesh Island is treated like Selection Island (provisional).
use crate::{Editor, Mesh, Mode, Result};
use glam::{DMat3, DMat4, DQuat, DVec3};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroundMode {
    #[default]
    DropToGround,
    DropToOrigin,
    RotateToGround,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundAlign {
    pub mode: GroundMode,
    pub connected: bool,
    pub by_selection: bool,
}
impl Default for GroundAlign {
    /// ALN-02 factory values.
    fn default() -> Self {
        Self {
            mode: GroundMode::DropToGround,
            connected: true,
            by_selection: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionAxisMode {
    #[default]
    SelectionIsland,
    MeshIsland,
    TotalSelection,
}
/// One Quick Align apply.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuickAlign {
    /// The target element's centre (world).
    pub target: DVec3,
    pub kind: QuickAlignKind,
    pub connected: bool,
    pub flatten: bool,
    pub islands: SelectionAxisMode,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuickAlignKind {
    /// Line up on world axis 0, 1 or 2.
    Position(usize),
    /// Face along the target's normal (world), or against it.
    Orientation { normal: DVec3, opposite: bool },
}

/// The selected points of `mesh` in `mode`.
fn selected_points(mesh: &Mesh, sel: &crate::scene::Selection) -> BTreeSet<u32> {
    sel.points(mesh)
        .into_iter()
        .filter(|&v| (v as usize) < mesh.vertices.len())
        .collect()
}

/// Every point in the connected pieces that hold any of `points`.
fn connected_points(mesh: &Mesh, points: &BTreeSet<u32>) -> BTreeSet<u32> {
    let mut out = points.clone();
    for piece in crate::item_merge::pieces(mesh) {
        let piece_points: BTreeSet<u32> = piece
            .iter()
            .flat_map(|&f| mesh.faces[f].vertices.iter().copied())
            .collect();
        if piece_points.iter().any(|v| points.contains(v)) {
            out.extend(piece_points);
        }
    }
    out
}

/// Islands of selected points joined through the mesh's edges.
fn islands(mesh: &Mesh, points: &BTreeSet<u32>) -> Vec<BTreeSet<u32>> {
    let list: Vec<u32> = points.iter().copied().collect();
    let index = |v: u32| list.binary_search(&v).ok();
    let mut parent: Vec<usize> = (0..list.len()).collect();
    fn root(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for f in 0..mesh.faces.len() {
        for (a, b) in mesh.element_spans(f) {
            if let (Some(i), Some(j)) = (index(a), index(b)) {
                let (x, y) = (root(&mut parent, i), root(&mut parent, j));
                parent[x.max(y)] = x.min(y);
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, BTreeSet<u32>> = Default::default();
    for (i, &v) in list.iter().enumerate() {
        groups.entry(root(&mut parent, i)).or_default().insert(v);
    }
    groups.into_values().collect()
}

fn average_normal(mesh: &Mesh, sel: &crate::scene::Selection, points: &BTreeSet<u32>) -> DVec3 {
    let faces: Vec<usize> = match sel.mode {
        Mode::Polygon => sel.faces.iter().map(|&f| f as usize).collect(),
        // Vertices and edges: the polygons around them (provisional, GAL-Q04).
        _ => (0..mesh.faces.len())
            .filter(|&f| {
                !mesh.is_polyline(f) && mesh.faces[f].vertices.iter().any(|v| points.contains(v))
            })
            .collect(),
    };
    faces
        .iter()
        .filter(|&&f| f < mesh.faces.len() && !mesh.is_polyline(f))
        .map(|&f| mesh.normal(f))
        .sum::<DVec3>()
        .normalize_or_zero()
}

impl Editor {
    fn create_align_target(&self, what: &str) -> Result<(u64, DMat4)> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err(format!("Finish the selection gesture before {what}"));
        }
        if self.scene.selection.mode == Mode::Item {
            return Err(format!(
                "{what}: the selection mode has to be vertices, edges or polygons"
            ));
        }
        let item = self
            .scene
            .active()
            .filter(|i| i.is_mesh() && self.scene.shows(i))
            .ok_or_else(|| format!("Select a visible mesh for {what}"))?;
        let world = self.scene.world_matrix(item.id)?;
        if world.determinant().abs() < 1e-15 {
            return Err(format!("{what}: the mesh's transform is flat"));
        }
        Ok((item.id, world))
    }

    /// Move the active mesh's points by a world-space function, as one step.
    fn publish_points(
        &mut self,
        item: u64,
        world: DMat4,
        moving: &BTreeSet<u32>,
        label: &str,
        f: impl Fn(u32, DVec3) -> DVec3,
    ) -> Result<()> {
        let inverse = world.inverse();
        let before = self.scene.clone();
        let target = self.scene.item_mut(item).ok_or("Missing mesh")?;
        let mut mesh: Mesh = (*target.mesh).clone();
        for &v in moving {
            let p = world.transform_point3(mesh.vertices[v as usize]);
            mesh.vertices[v as usize] = inverse.transform_point3(f(v, p));
        }
        if mesh == *target.mesh {
            return Ok(());
        }
        mesh.validate()?;
        target.mesh = mesh.into();
        if let Err(e) = self.scene.validate() {
            self.scene = before;
            return Err(e);
        }
        self.record(label.into(), before);
        Ok(())
    }

    pub(crate) fn ground_align(&mut self, options: GroundAlign) -> Result<()> {
        let (item, world) = self.create_align_target("Ground Align")?;
        let mesh: Mesh = (*self.scene.active().ok_or("Select a mesh")?.mesh).clone();
        let selected = selected_points(&mesh, &self.scene.selection);
        let all: BTreeSet<u32> = (0..mesh.vertices.len() as u32).collect();
        let moving = if selected.is_empty() {
            all
        } else if options.connected {
            connected_points(&mesh, &selected)
        } else {
            selected.clone()
        };
        if moving.is_empty() {
            return Err("Ground Align: the mesh has no points".into());
        }
        let reference = if options.by_selection && !selected.is_empty() {
            &selected
        } else {
            &moving
        };
        let wp = |v: &u32| world.transform_point3(mesh.vertices[*v as usize]);
        let (lo, hi) = reference.iter().map(wp).fold(
            (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        );
        match options.mode {
            GroundMode::DropToGround => {
                let shift = DVec3::new(0., -lo.y, 0.);
                self.publish_points(item, world, &moving, "Ground Align", |_, p| p + shift)
            }
            GroundMode::DropToOrigin => {
                let shift = -DVec3::new((lo.x + hi.x) / 2., lo.y, (lo.z + hi.z) / 2.);
                self.publish_points(item, world, &moving, "Ground Align", |_, p| p + shift)
            }
            GroundMode::RotateToGround => {
                if selected.is_empty() {
                    return Err(
                        "Ground Align: select polygons (or points) to rotate them to the ground"
                            .into(),
                    );
                }
                let local = average_normal(&mesh, &self.scene.selection, &selected);
                let normal =
                    (DMat3::from_mat4(world).inverse().transpose() * local).normalize_or_zero();
                if normal == DVec3::ZERO {
                    return Err("Ground Align: the selection has no facing to turn".into());
                }
                let turn = DQuat::from_rotation_arc(normal, DVec3::NEG_Y);
                let points: Vec<DVec3> = moving.iter().map(wp).collect();
                let (lo, hi) = points.iter().fold(
                    (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
                    |(lo, hi), p| (lo.min(*p), hi.max(*p)),
                );
                let centre = (lo + hi) / 2.;
                self.publish_points(item, world, &moving, "Ground Align", |_, p| {
                    centre + turn * (p - centre)
                })
            }
        }
    }

    pub(crate) fn quick_align(&mut self, q: QuickAlign) -> Result<()> {
        let (item, world) = self.create_align_target("Quick Align")?;
        if !q.target.is_finite() {
            return Err("Quick Align: pick a target".into());
        }
        let mesh: Mesh = (*self.scene.active().ok_or("Select a mesh")?.mesh).clone();
        let selected = selected_points(&mesh, &self.scene.selection);
        if selected.is_empty() {
            return Err("Quick Align: select what to move first".into());
        }
        let groups = match q.islands {
            SelectionAxisMode::TotalSelection => vec![selected.clone()],
            _ => islands(&mesh, &selected),
        };
        let wp = |v: u32| world.transform_point3(mesh.vertices[v as usize]);
        // Each group's own move, then the points it carries.
        let mut moves: Vec<(u32, DVec3)> = Vec::new();
        let mut owner: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut plans = Vec::new();
        for (g, group) in groups.iter().enumerate() {
            let centre = group.iter().map(|&v| wp(v)).sum::<DVec3>() / group.len() as f64;
            let carried = if q.connected {
                connected_points(&mesh, group)
            } else {
                group.clone()
            };
            for &v in &carried {
                owner.entry(v).or_insert(g);
            }
            plans.push((centre, group.clone()));
        }
        let normal_of = |group: &BTreeSet<u32>| {
            let sel = &self.scene.selection;
            let local = average_normal(&mesh, sel, group);
            (DMat3::from_mat4(world).inverse().transpose() * local).normalize_or_zero()
        };
        for (&v, &g) in &owner {
            let (centre, group) = &plans[g];
            let p = wp(v);
            let moved = match q.kind {
                QuickAlignKind::Position(axis) => {
                    let axis = axis.min(2);
                    let mut out = p;
                    if q.flatten && group.contains(&v) {
                        out[axis] = q.target[axis];
                    } else {
                        out[axis] += q.target[axis] - centre[axis];
                    }
                    out
                }
                QuickAlignKind::Orientation { normal, opposite } => {
                    let from = normal_of(group);
                    let to = if opposite { -normal } else { normal }.normalize_or_zero();
                    if from == DVec3::ZERO || to == DVec3::ZERO {
                        p
                    } else {
                        *centre + DQuat::from_rotation_arc(from, to) * (p - *centre)
                    }
                }
            };
            moves.push((v, moved));
        }
        let moving: BTreeSet<u32> = moves.iter().map(|(v, _)| *v).collect();
        let table: std::collections::BTreeMap<u32, DVec3> = moves.into_iter().collect();
        self.publish_points(item, world, &moving, "Quick Align", |v, p| {
            table.get(&v).copied().unwrap_or(p)
        })
    }
}

/// Tack (TOPOLOGY-CLEANUP TC-20..22): put the selection onto a picked
/// polygon. World space throughout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tack {
    /// The picked polygon's centre (TC-22: the click lands on the centre,
    /// wherever on the polygon it was).
    pub target: DVec3,
    /// The picked polygon's normal (front side).
    pub normal: DVec3,
    pub options: TackOptions,
}

/// Tack's options (TC-21 starting values).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TackOptions {
    /// Turn the selection so its face lies flat on the target, facing the
    /// other way (TC-22).
    pub align: bool,
    /// Distance off the surface, along its normal.
    pub offset: f64,
    /// 1.0 = 100 %.
    pub size: f64,
    /// Carry everything connected to the selection (TC-22: the whole cube).
    pub connected: bool,
    /// Degrees about the target normal, turned after the tack (TC-21's
    /// extra twist; Rotate X/Y read 0 after each click and stay unused).
    pub twist: f64,
    /// Copy Geometry (TC-22): the original stays and a copy is tacked.
    pub copy: bool,
    /// Make Bridge (TC-22): onto another item it does nothing at all; the
    /// app refuses the click (same-item bridging isn't built, TC-Q5).
    pub bridge: bool,
}
impl Default for TackOptions {
    fn default() -> Self {
        Self {
            align: true,
            offset: 0.,
            size: 1.,
            connected: true,
            twist: 0.,
            copy: false,
            bridge: false,
        }
    }
}

impl Editor {
    /// One Tack: one undo step (TC-22).
    pub(crate) fn tack(&mut self, t: Tack) -> Result<()> {
        let (item, world) = self.create_align_target("Tack")?;
        let o = t.options;
        if !t.target.is_finite() || !t.normal.is_finite() || t.normal.length() < 1e-12 {
            return Err("Tack: click a polygon to tack onto".into());
        }
        if !(o.offset.is_finite() && o.size.is_finite() && o.twist.is_finite()) || o.size <= 0. {
            return Err("Tack: Offset and Twist must be numbers and Size more than 0".into());
        }
        let mesh: Mesh = (*self.scene.active().ok_or("Select a mesh")?.mesh).clone();
        let selected = selected_points(&mesh, &self.scene.selection);
        if selected.is_empty() {
            return Err("Tack: select the polygon (or points) to move first".into());
        }
        let moving = if o.connected {
            connected_points(&mesh, &selected)
        } else {
            selected.clone()
        };
        let wp = |v: u32| world.transform_point3(mesh.vertices[v as usize]);
        let centre = selected.iter().map(|&v| wp(v)).sum::<DVec3>() / selected.len() as f64;
        let n_target = t.normal.normalize();
        let local_n = average_normal(&mesh, &self.scene.selection, &selected);
        let n_source = world.transform_vector3(local_n).normalize_or_zero();
        let turn = if o.align && n_source != DVec3::ZERO {
            glam::DQuat::from_rotation_arc(n_source, -n_target)
        } else {
            glam::DQuat::IDENTITY
        };
        let twist = glam::DQuat::from_axis_angle(n_target, o.twist.to_radians());
        let to = t.target + n_target * o.offset;
        let place = |p: DVec3| to + twist * (turn * ((p - centre) * o.size));
        if !o.copy {
            return self.publish_points(item, world, &moving, "Tack", |_, p| place(p));
        }
        // Copy Geometry: the polygons wholly inside the moving points (and
        // those points) are copied, and the copy is placed.
        let n = mesh.vertices.len() as u32;
        let order: Vec<u32> = moving.iter().copied().collect();
        let new_of: std::collections::BTreeMap<u32, u32> = order
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, n + i as u32))
            .collect();
        let inverse = world.inverse();
        let mut vertices = mesh.vertices.clone();
        vertices.extend(
            order
                .iter()
                .map(|&v| inverse.transform_point3(place(wp(v)))),
        );
        let mut faces: Vec<crate::edge_commands::OutFace> = (0..mesh.faces.len())
            .map(|f| crate::edge_commands::OutFace::copy(&mesh, f, mesh.faces[f].vertices.clone()))
            .collect();
        for (f, face) in mesh.faces.iter().enumerate() {
            if face.vertices.iter().all(|v| new_of.contains_key(v)) {
                faces.push(crate::edge_commands::OutFace::copy(
                    &mesh,
                    f,
                    face.vertices.iter().map(|v| new_of[v]).collect(),
                ));
            }
        }
        let copies: Vec<(u32, u32)> = new_of.iter().map(|(&old, &new)| (new, old)).collect();
        let out = crate::edge_commands::rebuild(&mesh, vertices, faces, &copies, "Tack")?;
        out.validate()?;
        let keep = self.scene.selection.faces.clone();
        self.publish_polygon_edit(item, out, "Tack", keep)
    }
}
