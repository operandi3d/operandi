//! VAC-A01..05: transient tool context, deliberately separate from scene pivots
//! and persistence. Advanced frame/cluster weighting remains a handoff question.
use crate::{Mode, Result, Scene};
use glam::{DMat3, DVec3};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CenterMode {
    #[default]
    None,
    Selection,
    Origin,
    /// ACT-AUTO-01/02: the bounding-box centre of the points the tool moves
    /// (the whole mesh with nothing selected).
    Automatic,
    /// C-BORDER: the middle of the box round the selection's border edges
    /// (edges shared with an unselected polygon); Selection's centre when
    /// there are none.
    SelectionBorder,
    /// Selection's centre with axes from the selection (F3).
    SelectionAutoAxis,
    /// The clicked element, as a world point (F4).
    Element(PickedPoint),
    /// A point fixed by a viewport click, in world space (F6).
    Screen(PickedPoint),
    /// Each connected piece about its own centre (F7).
    Local,
    /// The parent item's pivot (its world position; the world origin
    /// without a parent). Rulebook ACS table, `pivot_service.rs`.
    Parent,
    /// The mesh item's own pivot (its world position). Rulebook ACS table.
    Pivot,
}

/// A world point stored in nanometre steps, so contexts stay comparable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PickedPoint([i64; 3]);
impl PickedPoint {
    /// The world origin.
    pub const ORIGIN: Self = Self([0; 3]);
    pub fn new(p: DVec3) -> Self {
        Self(p.to_array().map(|c| (c * 1e9).round() as i64))
    }
    pub fn point(self) -> DVec3 {
        DVec3::from_array(self.0.map(|c| c as f64 * 1e-9))
    }
}
impl CenterMode {
    /// Selection and Selection Center Auto Axis centre on the middle of
    /// the selection's bounding box, in the item's own space (AC-rotate-
    /// Selection, 2026-11-18; the space is provisional, AC-Q20).
    pub fn uses_selection_box(self) -> bool {
        matches!(self, Self::Selection | Self::SelectionAutoAxis)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Selection => "Selection",
            Self::Origin => "Origin",
            Self::Automatic => "Automatic",
            Self::SelectionBorder => "Selection Border",
            Self::SelectionAutoAxis => "Selection Center Auto Axis",
            Self::Element(_) => "Element",
            Self::Screen(_) => "Screen",
            Self::Local => "Local",
            Self::Parent => "Parent",
            Self::Pivot => "Pivot",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AxisOrientation {
    #[default]
    World,
    /// A-SEL: the principal axes of the points the tool moves
    /// (`selection_axes.rs`).
    Selection,
    /// An explicitly chosen admitted major workplane frame; not camera state.
    Workplane(u8),
    /// Axes fitted to a clicked element (snaps-precision panel §4, Element
    /// action center): Z along the element's normal, or along an axis set
    /// from two snapped points (Planar Rotate Snap Axis).
    Element(PickedFrame),
    /// AC-40: the view's frame when Screen was chosen (X right, Y up, Z
    /// toward the viewer).
    Screen(PickedFrame),
    /// The parent item's axes (Parent, Pivot Center Parent Axis; ACS
    /// table). Resolved by `pivot_service::item_pivot_frame`.
    Parent,
    /// The mesh item's own axes (Pivot; ACS table).
    Item,
}
/// An orthonormal frame stored in nanometre steps like `PickedPoint`, so
/// contexts stay comparable. Columns X, Y, Z.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PickedFrame([[i64; 3]; 3]);
impl PickedFrame {
    /// The World axes.
    pub const WORLD: Self = Self([
        [1_000_000_000, 0, 0],
        [0, 1_000_000_000, 0],
        [0, 0, 1_000_000_000],
    ]);
    /// The frame whose Z is `z` (normalised); X and Y are any right-handed
    /// pair across it, X kept as level as it can be. `None` for a zero axis.
    pub fn from_z(z: DVec3) -> Option<Self> {
        let z = z.try_normalize()?;
        let helper = if z.y.abs() < 0.9 { DVec3::Y } else { DVec3::X };
        let x = helper.cross(z).normalize();
        let y = z.cross(x);
        Some(Self::from_basis(DMat3::from_cols(x, y, z)))
    }
    pub fn from_basis(basis: DMat3) -> Self {
        let q = |v: DVec3| v.to_array().map(|c| (c * 1e9).round() as i64);
        Self([q(basis.x_axis), q(basis.y_axis), q(basis.z_axis)])
    }
    pub fn basis(self) -> DMat3 {
        let v = |c: [i64; 3]| DVec3::from_array(c.map(|x| x as f64 * 1e-9)).normalize_or_zero();
        DMat3::from_cols(v(self.0[0]), v(self.0[1]), v(self.0[2]))
    }
}
impl AxisOrientation {
    pub fn label(self) -> &'static str {
        match self {
            Self::World => "World",
            Self::Selection => "Selection",
            Self::Workplane(0) => "Work Plane XZ",
            Self::Workplane(1) => "Work Plane YZ",
            Self::Workplane(_) => "Work Plane XY",
            Self::Element(_) => "Element",
            Self::Screen(_) => "Screen",
            Self::Parent => "Parent",
            Self::Item => "Item",
        }
    }
    pub fn basis(self) -> Result<DMat3> {
        match self {
            Self::World => Ok(DMat3::IDENTITY),
            Self::Selection => Err("Selection axes come from the selection".into()),
            Self::Workplane(p) if p <= 2 => Ok(DMat3::from_cols(
                crate::mesh::orient(DVec3::X, p),
                crate::mesh::orient(DVec3::Y, p),
                crate::mesh::orient(DVec3::Z, p),
            )),
            Self::Element(frame) | Self::Screen(frame) => Ok(frame.basis()),
            Self::Parent | Self::Item => Err("Item axes come from the item hierarchy".into()),
            _ => Err("Unsupported action-axis plane".into()),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionContext {
    pub center: CenterMode,
    pub axis: AxisOrientation,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionFrame {
    pub center: DVec3,
    pub basis: DMat3,
}

/// Unique selected vertices, or the admitted foreground component scope.
pub fn transform_vertices(scene: &Scene) -> Result<BTreeSet<u32>> {
    let item = scene
        .active()
        .ok_or("Select a mesh; empty item selection targets nothing")?;
    let s = &scene.selection;
    // Gathered in a list and collected once: a set built from a whole
    // list is sorted and filled in one go, where inserting a million
    // selected points one at a time was most of a whole-mesh drag frame
    // (heavy-mesh-speed). Same set either way.
    let mut vertices: Vec<u32> = Vec::new();
    match s.mode {
        Mode::Vertex => vertices.extend(s.vertices.iter().copied()),
        Mode::Edge => {
            for &(a, b) in &s.edges {
                vertices.extend([a, b]);
            }
        }
        Mode::Polygon => {
            for &f in &s.faces {
                vertices.extend(
                    item.mesh
                        .faces
                        .get(f as usize)
                        .ok_or("Invalid selected face")?
                        .vertices
                        .iter()
                        .copied(),
                );
            }
        }
        Mode::Item => return Err("Component scope required".into()),
    }
    if vertices.is_empty() {
        vertices.extend(0..item.mesh.vertices.len() as u32);
    }
    if vertices.is_empty()
        || vertices
            .iter()
            .any(|&v| v as usize >= item.mesh.vertices.len())
    {
        return Err("No eligible vertices or invalid selection".into());
    }
    Ok(vertices.into_iter().collect())
}
/// A-SEL (AC-RULES v6): the Selection axes of the active mesh's selection,
/// the principal axes of the points the tool moves (X the largest spread,
/// Z the least; `selection_axes.rs`). Positions are read as they are in
/// `scene`, so callers hand it a scene baked into world space.
///
/// It never refuses a selection for its shape: wires, cancelling normals
/// and closed bands all get a frame, and too little to go on gives World.
/// Only Item mode, which has no component selection, is turned away.
pub fn selection_basis(scene: &Scene) -> Result<DMat3> {
    let item = scene.active().ok_or("Select a visible mesh")?;
    if scene.selection.mode == Mode::Item {
        return Err("Selection axes need a component selection; use World or Work Plane".into());
    }
    Ok(crate::selection_axes::selection_axes(
        &item.mesh,
        &scene.selection,
        glam::DMat4::IDENTITY,
        None,
    ))
}
/// A frame with Y along normal `y`, Z as near world Z as it can be (world
/// X when Y is along Z). Local's lone points use it (`piece_axes`); the
/// Selection axes no longer do (A-SEL, `selection_basis`).
fn basis_from_up(y: DVec3) -> DMat3 {
    let projected = DVec3::Z - y * y.z;
    let z = if projected.length() > 1e-8 {
        projected.normalize()
    } else {
        (DVec3::X - y * y.x).normalize()
    };
    let x = y.cross(z).normalize();
    DMat3::from_cols(x, y, x.cross(y).normalize())
}
impl ActionContext {
    pub fn resolve(self, scene: &Scene) -> Result<ActionFrame> {
        let item = scene
            .active()
            .ok_or("Select a mesh; empty item selection targets nothing")?;
        if scene.selection.mode == Mode::Item {
            // IT-07: a child item's typed Move changes its own local channels.
            if self != Self::default() {
                // Spec: VAC-Q04
                return Err(
                    "This action center isn't supported for items yet; use None or World".into(),
                );
            }
            // Preserve the admitted item-local Euler/scale workflow. No mesh
            // centroid or component center is silently applied to item edits.
            return Ok(ActionFrame {
                center: item.transform.translation,
                basis: DMat3::IDENTITY,
            });
        }
        if !scene.shows(item) || scene.world_matrix(item.id)? != glam::DMat4::IDENTITY {
            return Err(
                "Component action centers require a visible identity world transform".into(),
            );
        }
        let vertices = transform_vertices(scene)?;
        let center = if let CenterMode::Element(p) | CenterMode::Screen(p) = self.center {
            p.point()
        } else if self.center == CenterMode::SelectionBorder {
            // C-BORDER: the box middle of the border edges' points; with no
            // border edge (or in Edge / Vertex mode) as Selection.
            let border =
                crate::selection_axes::selection_border_points(&item.mesh, &scene.selection);
            if border.is_empty() {
                box_centre(vertices.iter().map(|&v| item.mesh.vertices[v as usize]))
            } else {
                box_centre(border.iter().map(|&v| item.mesh.vertices[v as usize]))
            }
        } else if self.center == CenterMode::Origin {
            DVec3::ZERO
        } else {
            // None is like Automatic (rulebook ACS table); Selection, its
            // Auto Axis twin, Local's handle and the item centres' fallback
            // take the bounding-box middle too, not the vertex average.
            box_centre(vertices.iter().map(|&v| item.mesh.vertices[v as usize]))
        };
        if !center.is_finite() {
            return Err("Selection center must be finite".into());
        }
        Ok(ActionFrame {
            center,
            // AC-22: Selection Center Auto Axis takes its centre from the
            // selection but its axes from the chosen axis rule (World major
            // axes by default), never from the selection's normals.
            basis: if self.axis == AxisOrientation::Selection {
                selection_basis(scene)?
            } else {
                self.axis.basis()?
            },
        })
    }
}

/// The middle of the bounding box of `points` (NaN for none).
pub fn box_centre(points: impl Iterator<Item = DVec3>) -> DVec3 {
    let (lo, hi) = points.fold(
        (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(p), hi.max(p)),
    );
    (lo + hi) / 2.
}

/// Local (AC-scale-Local, AC-rotate-Local, ACL-01..04): each moved point's
/// piece frame, pieces joined by edges whose both ends move. A piece moves,
/// turns and scales about the middle of its own item-space bounding box
/// (as Selection, AC-Q20) and with its own axes (`piece_axes`): the
/// principal directions of its points, local Y the one nearest the equal
/// vote of its polygons' normals (the selected ones in Polygon mode, those
/// around the selected edges or points otherwise, every polygon with
/// nothing selected), local X the other one nearest the item's X. A piece
/// with neither a spread nor a normal keeps the tool's axes; a straight
/// line turns the tool axis nearest it onto itself (AC-73). Item space;
/// `LocalPiece::frame` places it in the world. Built once per preview,
/// from the selection's neighbourhood.
pub fn local_piece_frames(
    mesh: &crate::Mesh,
    moved: &BTreeSet<u32>,
    selection: &crate::Selection,
) -> std::collections::BTreeMap<u32, LocalPiece> {
    use std::collections::BTreeMap;
    let index: BTreeMap<u32, usize> = moved.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let mut parent: Vec<usize> = (0..moved.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    // AC-03: clusters join through the selection itself: polygons sharing
    // a point, edges sharing a point, points joined by an edge with both
    // ends selected. Two polygon rows with an unselected row between them
    // stay two clusters, though the edges across that row have both ends
    // moving (Jason, 2026-11-21: the mug wall's upper and lower clusters).
    let mut join = |a: u32, b: u32| {
        if let (Some(&i), Some(&j)) = (index.get(&a), index.get(&b)) {
            let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
            parent[ri] = rj;
        }
    };
    match selection.mode {
        Mode::Polygon if !selection.faces.is_empty() => {
            for &f in &selection.faces {
                if let Some(face) = mesh.faces.get(f as usize) {
                    for pair in face.vertices.windows(2) {
                        join(pair[0], pair[1]);
                    }
                }
            }
        }
        Mode::Edge if !selection.edges.is_empty() => {
            for &(a, b) in &selection.edges {
                join(a, b);
            }
        }
        _ => {
            for (a, b) in mesh.edges() {
                join(a, b);
            }
        }
    }
    // The polygons that vote, each with a moved point naming its piece.
    let mut voters: BTreeSet<(u32, u32)> = BTreeSet::new();
    let face_ok = |f: u32| mesh.faces.get(f as usize).is_some();
    let point_faces = mesh.point_faces();
    let nothing_selected = match selection.mode {
        Mode::Polygon => selection.faces.is_empty(),
        Mode::Edge => selection.edges.is_empty(),
        Mode::Vertex => selection.vertices.is_empty(),
        Mode::Item => true,
    };
    if nothing_selected {
        for (f, face) in mesh.faces.iter().enumerate() {
            if let Some(&v) = face.vertices.iter().find(|v| index.contains_key(v)) {
                voters.insert((f as u32, v));
            }
        }
    } else {
        match selection.mode {
            Mode::Polygon => {
                for &f in selection.faces.iter().filter(|&&f| face_ok(f)) {
                    if let Some(&v) = mesh.faces[f as usize].vertices.first() {
                        voters.insert((f, v));
                    }
                }
            }
            Mode::Edge => {
                for &(a, b) in &selection.edges {
                    for &f in point_faces.of(a) {
                        let face = &mesh.faces[f as usize].vertices;
                        let n = face.len();
                        let uses = (0..n).any(|i| {
                            let (p, q) = (face[i], face[(i + 1) % n]);
                            (p, q) == (a, b) || (q, p) == (a, b)
                        });
                        if uses {
                            voters.insert((f, a));
                        }
                    }
                }
            }
            _ => {
                for &v in &selection.vertices {
                    for &f in point_faces.of(v) {
                        voters.insert((f, v));
                    }
                }
            }
        }
    }
    let mut normals: BTreeMap<usize, Vec<DVec3>> = BTreeMap::new();
    let mut counted: BTreeSet<(usize, u32)> = BTreeSet::new();
    for &(f, v) in &voters {
        let Some(&i) = index.get(&v) else { continue };
        let r = root(&mut parent, i);
        if counted.insert((r, f)) {
            let n = mesh.normal(f as usize).normalize_or_zero();
            if n.is_finite() && n != DVec3::ZERO {
                normals.entry(r).or_default().push(n);
            }
        }
    }
    let points: Vec<u32> = moved.iter().copied().collect();
    let mut members: BTreeMap<usize, Vec<DVec3>> = BTreeMap::new();
    for (i, &v) in points.iter().enumerate() {
        let r = root(&mut parent, i);
        members
            .entry(r)
            .or_default()
            .push(mesh.vertices[v as usize]);
    }
    let pieces: BTreeMap<usize, LocalPiece> = members
        .into_iter()
        .map(|(r, cloud)| {
            let (lo, hi) = cloud.iter().fold(
                (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
                |(lo, hi), &p| (lo.min(p), hi.max(p)),
            );
            let up = normals.get_mut(&r).and_then(|list| {
                // Sorted, so the sum doesn't depend on storage order.
                list.sort_by(|a, b| {
                    a.x.total_cmp(&b.x)
                        .then(a.y.total_cmp(&b.y))
                        .then(a.z.total_cmp(&b.z))
                });
                let sum: DVec3 = list.iter().copied().sum();
                (sum.is_finite() && sum.length() > list.len() as f64 * LOCAL_CANCELLED)
                    .then(|| sum.normalize())
            });
            let center = (lo + hi) / 2.;
            let line = is_line(&cloud).then(|| line_direction(&cloud));
            let axes = if line.is_some() {
                None
            } else {
                piece_axes(&cloud, up)
            };
            // The column that carries the normal (`normal_on_nearest_axis`).
            let normal_axis = match (axes, up) {
                (Some(a), Some(n)) => (0..3)
                    .max_by(|&i, &j| a.col(i).dot(n).abs().total_cmp(&a.col(j).dot(n).abs()))
                    .unwrap_or(1),
                _ => 1,
            };
            (
                r,
                LocalPiece {
                    center,
                    axes,
                    line,
                    normal_axis,
                },
            )
        })
        .collect();
    points
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, pieces[&root(&mut parent, i)]))
        .collect()
}

/// One Local piece in item space: its box middle and its axes (columns X,
/// Y, Z, right-handed; `None`: the tool's own axes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalPiece {
    pub center: DVec3,
    pub axes: Option<DMat3>,
    /// A straight line (one edge, or edges in a row): its direction, item
    /// space. Its frame is the tool's axes turned onto it (AC-73).
    pub line: Option<DVec3>,
    /// Which column of `axes` is the polygons' normal (1, Y, unless the
    /// normal is nearer the item's Z or X).
    pub normal_axis: usize,
}
impl LocalPiece {
    /// The piece's world frame for an item with world matrix `world`;
    /// `fallback` axes when it has none.
    pub fn frame(self, world: glam::DMat4, fallback: DMat3) -> ActionFrame {
        let linear = DMat3::from_mat4(world);
        if let Some(along) = self.line {
            return ActionFrame {
                center: world.transform_point3(self.center),
                basis: line_frame_basis(fallback, linear * along),
            };
        }
        // The normal goes to the world by the inverse transpose; the next
        // axis by the item's own matrix, then squared up against it.
        let basis = self
            .axes
            .and_then(|a| {
                let k = self.normal_axis.min(2);
                let (k1, k2) = ((k + 1) % 3, (k + 2) % 3);
                let n = (linear.inverse().transpose() * a.col(k)).normalize_or_zero();
                let c2 = n.cross(linear * a.col(k1)).normalize_or_zero();
                let ok = |v: DVec3| v.is_finite() && v != DVec3::ZERO;
                (ok(n) && ok(c2)).then(|| {
                    let mut cols = [DVec3::ZERO; 3];
                    cols[k] = n;
                    cols[k1] = c2.cross(n);
                    cols[k2] = c2;
                    DMat3::from_cols(cols[0], cols[1], cols[2])
                })
            })
            .unwrap_or(fallback);
        ActionFrame {
            center: world.transform_point3(self.center),
            basis,
        }
    }
    /// Its mirror image across `symmetry` (the frame a point brought over
    /// from the other side is transformed in, SYM-TR-02), kept
    /// right-handed.
    pub fn mirrored(self, symmetry: crate::Symmetry) -> Self {
        Self {
            center: symmetry.mirror(self.center),
            axes: self.axes.map(|a| {
                // X keeps pointing along +X (ACL-02), so a mirrored
                // island's frame is its partner's.
                let mut x = symmetry.mirror_vector(a.x_axis);
                if x.x < 0. {
                    x = -x;
                }
                let y = symmetry.mirror_vector(a.y_axis);
                DMat3::from_cols(x, y, x.cross(y))
            }),
            line: self.line.map(|d| symmetry.mirror_vector(d)),
            normal_axis: self.normal_axis,
        }
    }
}

/// FG-Q02 (Jason, 2026-11-22; forum report "funky gizmo"): a Local
/// piece whose polygons' normals mostly cancel has no common normal, and
/// its axes come from its shape (the least-spread direction: a ring's
/// plane normal, the tunnel's direction). Mostly: their average is
/// shorter than a third of a unit normal. Polygons spread over half a
/// cylinder average 0.64 and keep their normal; around a hole they cancel
/// to 0, but uneven segments (an arch ring with 12 segments bunched at the
/// top: 0.30) left a small leftover that spun the gizmo by an arbitrary
/// angle about the tunnel. Past about 260° of a cylinder counts as around.
pub const LOCAL_CANCELLED: f64 = 1. / 3.;

/// Relative size under which two spreads (or two closenesses to X) count
/// as equal (ACL-03, ACL-Q3).
const LOCAL_TIE: f64 = 1e-4;

/// A Local piece's item-space axes (ACL-02/03, provisional ACL-Q1..Q4):
/// the principal directions of its points (each point weighs the same).
/// Y is the one nearest `up` (its polygons' normal), pointed with it, or
/// the one of least spread when the piece has no normal. X is, of the
/// other two, the one nearest the item's X, pointed along +X; when both
/// are as near, the one of more spread (an L of three quads takes its
/// long diagonal); when both spread the same (a square, turned or not),
/// the item's X laid into the piece's plane. Z = X × Y. With no spread
/// (one point), D18-A's frame from the normal; with neither, `None`.
/// Straight lines take `line_frame_basis` instead.
fn piece_axes(points: &[DVec3], up: Option<DVec3>) -> Option<DMat3> {
    let from_up = || up.map(basis_from_up);
    if points.len() < 2 {
        return from_up();
    }
    let axes = principal_axes(points);
    let most = axes[0].0;
    if !(most.is_finite() && most > 1e-18) {
        return from_up();
    }
    // A straight line (one edge, or edges in a row) never gets here: its
    // frame is `line_frame_basis` (AC-73).
    let y_at = match up {
        // Ties go to the one of least spread (searched from the end).
        Some(n) => (0..3)
            .rev()
            .max_by(|&a, &b| {
                let (da, db) = (axes[a].1.dot(n).abs(), axes[b].1.dot(n).abs());
                da.total_cmp(&db).then(b.cmp(&a))
            })
            .unwrap_or(2),
        None => 2,
    };
    let mut y = axes[y_at].1;
    let toward = up.unwrap_or(DVec3::Y);
    if y.dot(toward) < 0. {
        y = -y;
    }
    let rest: Vec<usize> = (0..3).filter(|&k| k != y_at).collect();
    let (a, b) = (rest[0], rest[1]);
    let in_plane = DVec3::X - y * y.x;
    let x = if (axes[a].0 - axes[b].0).abs() <= LOCAL_TIE * most && in_plane.length() > 1e-6 {
        in_plane
    } else {
        let (da, db) = (axes[a].1.x.abs(), axes[b].1.x.abs());
        if db > da + LOCAL_TIE {
            axes[b].1
        } else {
            axes[a].1
        }
    };
    let mut x = (x - y * x.dot(y)).normalize_or_zero();
    if x == DVec3::ZERO || !x.is_finite() {
        return from_up();
    }
    if x.x < 0. {
        x = -x;
    }
    Some(normal_on_nearest_axis(x, y))
}

/// Which local axis carries a polygon cluster's normal (Jason, 2026-11-21,
/// provisional AC-Q5): the item axis nearest the normal, as a line takes
/// the axis nearest it (AC-73). A cluster facing up keeps Y on its normal
/// (ACS-11/12: Rotate Y turns about the normal); one on a wall facing Z
/// puts Z on its normal, Y up the wall as far as the surface allows and X
/// around it; one facing X puts X on its normal and Y up. Ties keep Y,
/// then Z. `x` and `y` come from `piece_axes` (X the in-plane principal
/// direction nearest item X, Y the normal).
fn normal_on_nearest_axis(x: DVec3, normal: DVec3) -> DMat3 {
    let (ax, ay, az) = (normal.x.abs(), normal.y.abs(), normal.z.abs());
    if ay >= ax && ay >= az {
        return DMat3::from_cols(x, normal, x.cross(normal));
    }
    if az >= ax {
        // Z out of the surface; X the in-plane direction nearest item X,
        // turned so Y = Z × X points up.
        let mut x = x;
        if normal.cross(x).y < 0. {
            x = -x;
        }
        let y = normal.cross(x);
        return DMat3::from_cols(x, y, normal);
    }
    // X out of the surface; Y the in-plane direction nearest up.
    let other = normal.cross(x).normalize_or_zero();
    let mut y = if other.y.abs() > x.y.abs() { other } else { x };
    if y.y < 0. {
        y = -y;
    }
    DMat3::from_cols(normal, y, normal.cross(y))
}

/// Whether `points` lie on one line with some length (one edge, or edges
/// in a row).
fn is_line(points: &[DVec3]) -> bool {
    if points.len() < 2 {
        return false;
    }
    let axes = principal_axes(points);
    let most = axes[0].0;
    most.is_finite() && most > 1e-18 && axes[1].0 <= LOCAL_TIE * most
}

/// A straight-line piece's direction (item space), for AC-73.
fn line_direction(points: &[DVec3]) -> DVec3 {
    principal_axes(points)[0].1
}

/// AC-73 (provisional, AC-Q1): the tool axis nearest a line's direction
/// `along` (world) is turned onto the line, keeping its sign, and the other
/// two axes follow by the smallest rotation. Ties go to Y, then X, then Z.
pub fn line_frame_basis(tool_axes: DMat3, along: DVec3) -> DMat3 {
    let along = along.normalize_or_zero();
    if along == DVec3::ZERO || !along.is_finite() {
        return tool_axes;
    }
    let columns = [tool_axes.y_axis, tool_axes.x_axis, tool_axes.z_axis];
    let mut best = columns[0];
    for c in columns {
        if c.dot(along).abs() > best.dot(along).abs() + 1e-12 {
            best = c;
        }
    }
    let target = if best.dot(along) < 0. { -along } else { along };
    let turn = glam::DQuat::from_rotation_arc(best.normalize(), target);
    DMat3::from_quat(turn) * tool_axes
}

/// The spreads and directions of `points` about their average, most
/// spread first: the covariance's eigenpairs by cyclic Jacobi rotations.
pub(crate) fn principal_axes(points: &[DVec3]) -> [(f64, DVec3); 3] {
    let n = points.len() as f64;
    let mean = points.iter().copied().sum::<DVec3>() / n;
    let mut c = [[0f64; 3]; 3];
    for p in points {
        let d = (*p - mean).to_array();
        for i in 0..3 {
            for j in 0..3 {
                c[i][j] += d[i] * d[j] / n;
            }
        }
    }
    // Columns of `v` are the eigenvectors.
    let mut v = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..50 {
        let off = c[0][1].abs() + c[0][2].abs() + c[1][2].abs();
        if off < 1e-30 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if c[p][q].abs() < 1e-300 {
                continue;
            }
            let theta = (c[q][q] - c[p][p]) / (2. * c[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.).sqrt());
            let t = if theta == 0. { 1. } else { t };
            let cos = 1. / (t * t + 1.).sqrt();
            let sin = t * cos;
            for row in &mut c {
                let (a, b) = (row[p], row[q]);
                row[p] = cos * a - sin * b;
                row[q] = sin * a + cos * b;
            }
            let (rp, rq) = (c[p], c[q]);
            c[p] = std::array::from_fn(|k| cos * rp[k] - sin * rq[k]);
            c[q] = std::array::from_fn(|k| sin * rp[k] + cos * rq[k]);
            for row in &mut v {
                let (a, b) = (row[p], row[q]);
                row[p] = cos * a - sin * b;
                row[q] = sin * a + cos * b;
            }
        }
    }
    let mut out = [0, 1, 2].map(|k| {
        (
            c[k][k],
            DVec3::new(v[0][k], v[1][k], v[2][k]).normalize_or_zero(),
        )
    });
    out.sort_by(|a, b| b.0.total_cmp(&a.0));
    out
}

// ---- Handle frames of the edge and vertex tools (tool_card.rs
// `HandleAnchor::Edge`, `EdgeRail` and `Vertex`). Each reads only the
// tool's base scene (the scene before the drag), in world space.
//
// ACS-Q-HANDLE1 (answered): where each tool's handle sits.

/// How an edge or vertex tool places its handle (ACS-Q-HANDLE1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlePick {
    /// The centre of the selection's bounding box (Edge Bevel, Extrude,
    /// Chamfer, Vertex Extrude); pick order doesn't matter. Its directions
    /// come from the selected element nearest that centre.
    BoxCentre,
    /// The first-picked edge or point (Edge Slide, Vertex Slide).
    FirstPicked,
    /// The selected element nearest the selection's box centre, lowest
    /// index on a tie (the other edge and vertex tools; Edge Weight is
    /// unchecked, ACS-Q-HANDLE2).
    Nearest,
}

/// The handle rule of `tool` (ACS-Q-HANDLE1). Under Symmetry every rule
/// picks among the primary side's elements.
pub fn handle_pick(tool: crate::Tool) -> HandlePick {
    use crate::Tool;
    match tool {
        Tool::EdgeBevel | Tool::EdgeExtrude | Tool::EdgeChamfer | Tool::VertexExtrude => {
            HandlePick::BoxCentre
        }
        Tool::EdgeSlide | Tool::VertexSlide => HandlePick::FirstPicked,
        _ => HandlePick::Nearest,
    }
}

/// The selected points a handle picks from: with Symmetry on, those on the
/// primary side (`Scene::symmetry_primary_side`, the pivot service's rule).
fn handle_side_points(base: &Scene, symmetry: crate::Symmetry, points: &[u32]) -> Vec<u32> {
    let Some(mesh) = base.active().map(|i| &i.mesh) else {
        return points.to_vec();
    };
    match base.symmetry_primary_side(symmetry) {
        Some(side) => crate::pivot_service::side_points(mesh, points, symmetry, side),
        None => points.to_vec(),
    }
}

/// Of `candidates` (a sort key and an item-space position each), the one
/// nearest their box centre; the lowest key wins a tie.
fn nearest_box_centre<K: Ord + Copy>(candidates: &[(K, DVec3)]) -> Option<K> {
    let centre = box_centre(candidates.iter().map(|c| c.1));
    candidates
        .iter()
        .min_by(|a, b| {
            a.1.distance_squared(centre)
                .total_cmp(&b.1.distance_squared(centre))
                .then(a.0.cmp(&b.0))
        })
        .map(|c| c.0)
}

/// The selected edge a handle sits on (see above). `None` with no edges
/// selected.
pub fn handle_edge(
    base: &Scene,
    symmetry: crate::Symmetry,
    pick: HandlePick,
) -> Option<(u32, u32)> {
    let mesh = &base.active()?.mesh;
    let edges = &base.selection.edges;
    let valid = |v: u32| (v as usize) < mesh.vertices.len();
    let ends: Vec<u32> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    let side: BTreeSet<u32> = handle_side_points(base, symmetry, &ends)
        .into_iter()
        .collect();
    let choose = |want: &dyn Fn(u32, u32) -> bool| -> Vec<((u32, u32), DVec3)> {
        edges
            .iter()
            .filter(|&&(a, b)| valid(a) && valid(b) && want(a, b))
            .map(|&(a, b)| {
                let key = (a.min(b), a.max(b));
                (
                    key,
                    (mesh.vertices[a as usize] + mesh.vertices[b as usize]) / 2.,
                )
            })
            .collect()
    };
    let mut candidates = choose(&|a, b| side.contains(&a) && side.contains(&b));
    if candidates.is_empty() {
        candidates = choose(&|_, _| true);
    }
    // Candidates keep the selection's order, so the first is the first
    // picked.
    let key = match pick {
        HandlePick::FirstPicked => candidates.first()?.0,
        _ => nearest_box_centre(&candidates)?,
    };
    // Keep the selection's own orientation of the edge.
    edges
        .iter()
        .copied()
        .find(|&(a, b)| (a.min(b), a.max(b)) == key)
}

/// The selected point a handle sits on (see above).
pub fn handle_vertex(base: &Scene, symmetry: crate::Symmetry, pick: HandlePick) -> Option<u32> {
    let mesh = &base.active()?.mesh;
    let points: Vec<u32> = base
        .selection
        .vertices
        .iter()
        .copied()
        .filter(|&v| (v as usize) < mesh.vertices.len())
        .collect();
    let side: BTreeSet<u32> = handle_side_points(base, symmetry, &points)
        .into_iter()
        .collect();
    // In the selection's order, so the first is the first picked.
    let candidates: Vec<(u32, DVec3)> = points
        .iter()
        .filter(|v| side.contains(v))
        .map(|&v| (v, mesh.vertices[v as usize]))
        .collect();
    match pick {
        HandlePick::FirstPicked => candidates.first().map(|c| c.0),
        _ => nearest_box_centre(&candidates),
    }
}

/// The world centre of the bounding box of the selected edges' ends (or
/// the selected points), the primary side's under Symmetry
/// ([`HandlePick::BoxCentre`]).
fn handle_box_centre(base: &Scene, symmetry: crate::Symmetry, edges: bool) -> Option<DVec3> {
    let item = base.active()?;
    let mesh = &item.mesh;
    let points: Vec<u32> = if edges {
        base.selection
            .edges
            .iter()
            .flat_map(|&(a, b)| [a, b])
            .collect()
    } else {
        base.selection.vertices.clone()
    };
    let points: Vec<u32> = points
        .into_iter()
        .filter(|&v| (v as usize) < mesh.vertices.len())
        .collect();
    let side = handle_side_points(base, symmetry, &points);
    if side.is_empty() {
        return None;
    }
    let world = base.world_matrix(item.id).ok()?;
    let centre = box_centre(
        side.iter()
            .map(|&v| world.transform_point3(mesh.vertices[v as usize])),
    );
    centre.is_finite().then_some(centre)
}

/// Edge Slide's handle frame: the handle edge's midpoint
/// ([`handle_edge`]), the average left-rail direction of its ends, and
/// that rail length.
pub fn edge_slide_handle_frame(
    base: &Scene,
    symmetry: crate::Symmetry,
) -> Option<(DVec3, DVec3, f64)> {
    let item = base.active()?;
    // Edge Bevel with nothing selected works on every edge (TA-06): the
    // handle sits on the mesh's first edge.
    let (a, b) = match handle_edge(base, symmetry, HandlePick::FirstPicked) {
        Some(edge) => edge,
        None => *item.mesh.edges().first()?,
    };
    let world = base.world_matrix(item.id).ok()?;
    let mesh = &item.mesh;
    // Half way, so the probe never merges points (ES-04) and keeps ids.
    let probe = crate::edge_slide_geometry(mesh, &base.selection.edges, 0.5).ok()?;
    let pa = world.transform_point3(mesh.vertices[a as usize]);
    let pb = world.transform_point3(mesh.vertices[b as usize]);
    let qa = world.transform_point3(probe.vertices[a as usize]);
    let qb = world.transform_point3(probe.vertices[b as usize]);
    let rail = (qa - pa) + (qb - pb);
    Some(((pa + pb) / 2., rail.normalize_or_zero(), rail.length()))
}

/// The vertex tools' handle frame: the handle point ([`handle_vertex`]),
/// its polygons' average normal, the direction to its first neighbour (the
/// rail) and that edge's length.
pub fn vertex_handle_frame(
    base: &Scene,
    symmetry: crate::Symmetry,
    pick: HandlePick,
) -> Option<(DVec3, DVec3, DVec3, f64)> {
    let item = base.active()?;
    let v = handle_vertex(base, symmetry, pick)?;
    let world = base.world_matrix(item.id).ok()?;
    let mesh = &item.mesh;
    let normal = (0..mesh.faces.len())
        .filter(|&f| mesh.faces[f].vertices.contains(&v))
        .map(|f| mesh.normal(f))
        .sum::<DVec3>();
    let p = mesh.vertices[v as usize];
    let rail = crate::vertex_slide_direction(mesh, &[v]).unwrap_or(DVec3::X);
    let length = mesh
        .edges()
        .into_iter()
        .find_map(|(a, b)| match (a == v, b == v) {
            (true, _) => Some(b),
            (_, true) => Some(a),
            _ => None,
        })
        .map_or(1., |w| {
            world
                .transform_point3(mesh.vertices[w as usize])
                .distance(world.transform_point3(p))
        });
    let at = match pick {
        HandlePick::BoxCentre => handle_box_centre(base, symmetry, false),
        _ => None,
    };
    Some((
        at.unwrap_or_else(|| world.transform_point3(p)),
        world.transform_vector3(normal).normalize_or(DVec3::Z),
        world.transform_vector3(rail).normalize_or(DVec3::X),
        length,
    ))
}

/// What [`selection_normal`] averages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NormalOf {
    /// The selected polygons' normals.
    Polygons,
    /// Each selected edge's normal: the average of its polygons' (the
    /// diagonal on a box's edge).
    Edges,
    /// Each selected point's normal: the average of its polygons'.
    Points,
}

/// The Bevels' handle direction (2026-11-23): the selection's average
/// normal in world space, so the gizmo follows a polygon's normal, an
/// edge's diagonal or a point's normal, and the average over a group. Under
/// Symmetry only the primary side counts, as for the handle's place.
/// `None` with nothing selected or when the normals cancel out (opposite
/// polygons), so callers keep their single-element direction.
pub fn selection_normal(base: &Scene, symmetry: crate::Symmetry, of: NormalOf) -> Option<DVec3> {
    let item = base.active()?;
    let mesh = &item.mesh;
    let n = mesh.vertices.len() as u32;
    let sel = &base.selection;
    // Every point the selection touches, cut to the primary side.
    let touched: Vec<u32> = match of {
        NormalOf::Polygons => sel
            .faces
            .iter()
            .filter(|&&f| (f as usize) < mesh.faces.len())
            .flat_map(|&f| mesh.faces[f as usize].vertices.iter().copied())
            .collect(),
        NormalOf::Edges => sel.edges.iter().flat_map(|&(a, b)| [a, b]).collect(),
        NormalOf::Points => sel.vertices.clone(),
    };
    let touched: Vec<u32> = touched.into_iter().filter(|&v| v < n).collect();
    let side: std::collections::HashSet<u32> = handle_side_points(base, symmetry, &touched)
        .into_iter()
        .collect();
    if side.is_empty() {
        return None;
    }
    let sum = match of {
        NormalOf::Polygons => sel
            .faces
            .iter()
            .filter(|&&f| (f as usize) < mesh.faces.len())
            .filter(|&&f| {
                mesh.faces[f as usize]
                    .vertices
                    .iter()
                    .all(|v| side.contains(v))
            })
            .map(|&f| mesh.normal(f as usize))
            .sum::<DVec3>(),
        NormalOf::Edges | NormalOf::Points => {
            // The polygons around each point the selection touches, found in
            // one pass over the mesh.
            let mut around: std::collections::HashMap<u32, Vec<usize>> =
                side.iter().map(|&v| (v, Vec::new())).collect();
            for (f, face) in mesh.faces.iter().enumerate() {
                for v in &face.vertices {
                    if let Some(list) = around.get_mut(v) {
                        list.push(f);
                    }
                }
            }
            let unit = |faces: &mut dyn Iterator<Item = usize>| {
                faces
                    .map(|f| mesh.normal(f))
                    .sum::<DVec3>()
                    .normalize_or_zero()
            };
            if of == NormalOf::Edges {
                sel.edges
                    .iter()
                    .filter(|(a, b)| side.contains(a) && side.contains(b))
                    .map(|(a, b)| {
                        let b_faces = &around[b];
                        unit(&mut around[a].iter().copied().filter(|f| b_faces.contains(f)))
                    })
                    .sum::<DVec3>()
            } else {
                sel.vertices
                    .iter()
                    .filter(|v| side.contains(v))
                    .map(|v| unit(&mut around[v].iter().copied()))
                    .sum::<DVec3>()
            }
        }
    };
    let world = base.world_matrix(item.id).ok()?;
    let normal = world.transform_vector3(sum);
    (normal.length() > 1e-9).then(|| normal.normalize())
}

/// The edge tools' handle frame (Edge Extrude, Weight, Bevel, Chamfer,
/// Relax): the handle edge's midpoint ([`handle_edge`]), its neighbours'
/// average normal and the in-plane direction into its first neighbour
/// (Width).
pub fn edge_handle_frame(
    base: &Scene,
    symmetry: crate::Symmetry,
    pick: HandlePick,
) -> Option<(DVec3, DVec3, DVec3)> {
    let item = base.active()?;
    // Edge Bevel with nothing selected works on every edge (TA-06): the
    // handle sits on the mesh's first edge.
    let (a, b) = match handle_edge(base, symmetry, pick) {
        Some(edge) => edge,
        None => *item.mesh.edges().first()?,
    };
    let world = base.world_matrix(item.id).ok()?;
    let mesh = &item.mesh;
    let faces: Vec<usize> = mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.vertices.contains(&a) && f.vertices.contains(&b))
        .map(|(i, _)| i)
        .collect();
    let first = *faces.first()?;
    let up = faces.iter().map(|&f| mesh.normal(f)).sum::<DVec3>();
    let (pa, pb) = (mesh.vertices[a as usize], mesh.vertices[b as usize]);
    let mid = (pa + pb) / 2.;
    let into = (mesh.center(first) - mid).reject_from(pb - pa);
    let at = match pick {
        HandlePick::BoxCentre => handle_box_centre(base, symmetry, true),
        _ => None,
    };
    Some((
        at.unwrap_or_else(|| world.transform_point3(mid)),
        world.transform_vector3(up).normalize_or_zero(),
        world.transform_vector3(into).normalize_or_zero(),
    ))
}
