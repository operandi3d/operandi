//! The shared pivot service (specifications/ac-symmetry-rulebook-2026-11-19,
//! ACS-xx). One place answers, for the active tool, where it pivots, along
//! which axes, which points each pivot drives and, with Symmetry on, which
//! side is the *primary* side (the one that gets typed values and the drag
//! as entered) and how the other side follows.
//!
//! Inputs (rulebook §1): the Action Center as two independent parts, a
//! centre rule (`CenterMode`) and an axis rule (`AxisOrientation`); the
//! tool's base scene (selection, item transform, item hierarchy); the
//! Symmetry setting (axis, offset, topology); and a centre placed by a
//! viewport click or typed into the tool (ACS-07).
//!
//! Outputs ([`Resolved`], AC-RULES v6): the handle's frame and, for each
//! mesh the tool moves, its *groups*: one group for a centre rule with one
//! centre, one per cluster under Local. A group has a stable identity, the
//! world frame its points take the values in, and its targets: every
//! point it drives with that point's Symmetry role and side. The transform
//! tools' preview (drag and typed values), their handle and their fields
//! all read this one answer; none of them works out targets, clusters,
//! mirror roles or frames for itself.
//!
//! Every tool that pivots or picks a Symmetry side asks this module:
//! Move, Rotate, Scale, Transform (and Stretch, Scale's non-uniform use),
//! the polygon stage tools' handles (Bevel, Extrude, Smooth Shift,
//! Thicken), Edge Extend, the slide/deform tools' side choice, Bend's
//! side, Twist Bevel's and the two-pass phase 2 tools. Nothing else
//! computes a handle centre under Symmetry or picks a side.
use crate::symmetry::{Side, Symmetry, match_tolerance};
use crate::{ActionContext, ActionFrame, AxisOrientation, CenterMode, LocalPiece};
use crate::{Mesh, Mode, Result, Scene, Selection, Shared, Tool};
use glam::{DMat3, DMat4, DVec3};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

/// The service's whole answer for one tool on one base scene.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// Where the handle sits and how its arrows point (world). The primary
    /// frame, or under Local the frame of the cluster holding the
    /// last-selected element (AC-75).
    pub handle: ActionFrame,
    /// The primary side under Symmetry (Positive or Negative, never
    /// Plane); `None` with Symmetry off or not applying (Item mode,
    /// several active meshes).
    pub primary: Option<Side>,
    /// One entry per mesh the tool moves; empty in Item mode.
    pub items: Vec<ItemPivots>,
}

/// The groups of one mesh item.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemPivots {
    pub item: u64,
    /// The mirror rules for this mesh (its world matrix, its Symmetry
    /// plane and the primary side).
    pub map: MirrorMap,
    pub groups: Vec<Group>,
}

/// Which group this is, the same for the same selection however it was
/// made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupId {
    /// Every point the tool moves on this mesh (a centre rule with one
    /// centre).
    Whole,
    /// A Local cluster, numbered in the order of the clusters' lowest
    /// points.
    Cluster(u32),
}

/// One pivot and the points it drives.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: GroupId,
    /// The world frame its points take the tool's values in.
    pub frame: ActionFrame,
    /// The frame a point from the other side of the Symmetry plane is
    /// edited in once it has been mirrored over (it is mirrored back
    /// afterwards, S-04): `frame` itself for one shared centre; a Local
    /// cluster's mirror image, which is its twin's frame (S-09).
    pub twin: ActionFrame,
    pub targets: Vec<Target>,
}

/// One point a group drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub point: u32,
    /// How it follows the primary pivot (S-03..S-05).
    pub role: Role,
    /// The side of the Symmetry plane it starts on; `None` with Symmetry
    /// off.
    pub side: Option<Side>,
}

impl Resolved {
    /// Every group with the mesh it belongs to.
    pub fn groups(&self) -> impl Iterator<Item = (&ItemPivots, &Group)> {
        self.items
            .iter()
            .flat_map(|item| item.groups.iter().map(move |g| (item, g)))
    }
}

/// A question to the service.
#[derive(Clone, Copy)]
pub struct PivotRequest<'a> {
    /// The tool's base scene (the scene before the drag).
    pub scene: &'a Scene,
    pub tool: Tool,
    pub context: ActionContext,
    pub symmetry: Symmetry,
    /// ACS-07: a centre set where a viewport click landed, or typed into
    /// the tool (world space). It is used unmirrored and its side becomes
    /// the primary side.
    pub placed: Option<DVec3>,
}

/// Whether a centre rule takes its centre from the selection (ACS-01
/// applies to it under Symmetry).
pub fn selection_based(center: CenterMode) -> bool {
    matches!(
        center,
        CenterMode::None
            | CenterMode::Automatic
            | CenterMode::Selection
            | CenterMode::SelectionAutoAxis
            | CenterMode::SelectionBorder
            | CenterMode::Local
    )
}

/// ACS-03 / ACS-08: the primary side for a centre at `centre` (the mesh
/// item's own space): the side it is on, a centre on the (offset) plane
/// counting as the − side.
pub fn primary_side(symmetry: Symmetry, centre: DVec3, tolerance: f64) -> Side {
    match symmetry.side(centre, tolerance) {
        Some(Side::Positive) => Side::Positive,
        _ => Side::Negative,
    }
}

/// The side opposite a primary side.
pub fn other(primary: Side) -> Side {
    match primary {
        Side::Positive => Side::Negative,
        _ => Side::Positive,
    }
}

/// ACS-01: the selected points a selection-based centre is taken from
/// under Symmetry: those strictly on the + side; when there are none, those
/// strictly on the − side; when there are none either (everything on the
/// plane), all of them. Points exactly on the plane don't count toward a
/// half (provisional, ACS-Q5).
pub fn half_points(mesh: &Mesh, points: &[u32], symmetry: Symmetry) -> Vec<u32> {
    if symmetry.axis.is_none() {
        return points.to_vec();
    }
    let tolerance = match_tolerance(mesh);
    let on = |want: Side| -> Vec<u32> {
        points
            .iter()
            .copied()
            .filter(|&v| {
                mesh.vertices
                    .get(v as usize)
                    .is_some_and(|&p| symmetry.side(p, tolerance) == Some(want))
            })
            .collect()
    };
    let positive = on(Side::Positive);
    if !positive.is_empty() {
        return positive;
    }
    let negative = on(Side::Negative);
    if !negative.is_empty() {
        return negative;
    }
    points.to_vec()
}

/// The points of `points` on side `side` (strictly), or `half_points`'
/// choice when `side` has none. Used where the primary side is already
/// decided (a click on the − side, ACS-07) and a handle follows that half.
pub fn side_points(mesh: &Mesh, points: &[u32], symmetry: Symmetry, side: Side) -> Vec<u32> {
    if symmetry.axis.is_none() {
        return points.to_vec();
    }
    let tolerance = match_tolerance(mesh);
    let picked: Vec<u32> = points
        .iter()
        .copied()
        .filter(|&v| {
            mesh.vertices
                .get(v as usize)
                .is_some_and(|&p| symmetry.side(p, tolerance) == Some(side))
        })
        .collect();
    if picked.is_empty() {
        half_points(mesh, points, symmetry)
    } else {
        picked
    }
}

impl PivotRequest<'_> {
    /// Symmetry as it applies to this request: on, a component selection
    /// of one mesh (several meshes share one frame and Symmetry mirrors
    /// each in its own space, MAM-Q09).
    pub fn mesh_symmetry(&self) -> Option<Symmetry> {
        self.symmetry.axis?;
        if self.scene.selection.mode == Mode::Item
            || self.scene.component_transform_meshes().is_some()
        {
            return None;
        }
        self.scene.active()?;
        Some(self.symmetry)
    }

    /// The primary pivot's frame: where the handle sits and the tool
    /// pivots, and the axes its values are entered in.
    ///
    /// Under Symmetry one side is asked for both. The centre: a placed
    /// centre as placed (S-07); a fixed centre unmoved (S-10); a
    /// selection-based one from the + side's points (S-01). The axes:
    /// Selection axes come from that same side's points only (S-02), so
    /// the mirror image of a selection can't tilt them; the other axis
    /// rules don't read the selection. The side the centre is on then
    /// drives the edit (S-03, `MirrorMap`), and the other side gets the
    /// mirror image of this frame (S-04).
    pub fn frame(&self) -> Result<ActionFrame> {
        let plain = self.plain_frame()?;
        let Some(symmetry) = self.mesh_symmetry() else {
            let center = self.placed.unwrap_or(plain.center);
            return Ok(ActionFrame {
                center,
                basis: self.single_centre_local_axes(center).unwrap_or(plain.basis),
            });
        };
        let item = self.scene.active().ok_or("Select a mesh")?;
        let world = self.scene.world_matrix(item.id)?;
        let mesh = &item.mesh;
        let center = match self.placed {
            Some(at) => at,
            // Origin, Parent, Pivot, Element and Screen have fixed centres.
            None if !selection_based(self.context.center) => plain.center,
            None => {
                let moved: Vec<u32> = crate::transform_vertices(self.scene)?.into_iter().collect();
                let at = |v: &u32| mesh.vertices[*v as usize];
                let half = |points: &[u32]| half_points(mesh, points, symmetry);
                let center = match self.context.center {
                    // Automatic and None take their box in world space (as
                    // their plain frame does).
                    CenterMode::Automatic | CenterMode::None => crate::box_centre(
                        half(&moved).iter().map(|v| world.transform_point3(at(v))),
                    ),
                    CenterMode::SelectionBorder => {
                        let border: Vec<u32> = crate::selection_axes::selection_border_points(
                            mesh,
                            &self.scene.selection,
                        )
                        .into_iter()
                        .collect();
                        if border.is_empty() {
                            // No border edge: as Selection.
                            world.transform_point3(crate::box_centre(half(&moved).iter().map(at)))
                        } else {
                            crate::box_centre(
                                half(&border).iter().map(|v| world.transform_point3(at(v))),
                            )
                        }
                    }
                    // Selection, its Auto Axis twin and Local's handle: the
                    // box in the item's own space (AC-Q20).
                    _ => world.transform_point3(crate::box_centre(half(&moved).iter().map(at))),
                };
                if center.is_finite() {
                    center
                } else {
                    plain.center
                }
            }
        };
        let basis = if self.context.axis == AxisOrientation::Selection {
            // S-02: the + side's points; with a placed centre, the side it
            // was placed on (provisional, ACR-Q03). None there gives World.
            let side = match self.placed {
                Some(at) => side_of_world(self.scene, symmetry, at),
                None => Side::Positive,
            };
            let tolerance = crate::symmetry::match_tolerance_shared(mesh.shared());
            crate::selection_axes::selection_axes(
                mesh,
                &self.scene.selection,
                world,
                Some(&|p| symmetry.side(p, tolerance) == Some(side)),
            )
        } else {
            self.single_centre_local_axes(center).unwrap_or(plain.basis)
        };
        Ok(ActionFrame { center, basis })
    }

    /// The mixing rule for Local axes with a centre rule that has one
    /// centre (AC-RULES v6, section 3): the axes of the cluster holding the
    /// last-selected element. `centre` is the primary centre (world): with
    /// Symmetry, a cluster on the other side of the plane gives its mirror
    /// image, the frame its twin has (provisional, ACR-Q21). `None` when
    /// the axes aren't Local, the centre is Local too (each cluster then
    /// has its own frame), or nothing is selected (the tool's own axes).
    fn single_centre_local_axes(&self, centre: DVec3) -> Option<DMat3> {
        if self.context.axis != AxisOrientation::Local || self.context.center == CenterMode::Local {
            return None;
        }
        let item = self.scene.active()?;
        let world = self.scene.world_matrix(item.id).ok()?;
        let last = self.last_selected_point()?;
        let moved: BTreeSet<u32> = crate::transform_vertices(self.scene).ok()?;
        let clusters = crate::local_clusters(&item.mesh, &moved, &self.scene.selection);
        let piece = clusters.pieces[*clusters.of.get(&last)?];
        let piece = match self.mesh_symmetry() {
            Some(symmetry) => {
                let map = MirrorMap::new(self.scene, item.id, symmetry, centre).ok()?;
                match map.role(*item.mesh.vertices.get(last as usize)?) {
                    Role::Other => piece.mirrored(symmetry),
                    _ => piece,
                }
            }
            None => piece,
        };
        Some(piece.frame(world, DMat3::IDENTITY).basis)
    }

    /// A point of the element selected last (the polygon's first corner,
    /// the edge's first end, the point), in pick order: it names the
    /// cluster the Local handle sits on (AC-75). `None` in Item mode or
    /// with nothing selected.
    fn last_selected_point(&self) -> Option<u32> {
        let selection = &self.scene.selection;
        let item = self.scene.active()?;
        match selection.mode {
            Mode::Polygon => item
                .mesh
                .faces
                .get(*selection.faces.last()? as usize)?
                .vertices
                .first()
                .copied(),
            Mode::Edge => Some(selection.edges.last()?.0),
            Mode::Vertex => selection.vertices.last().copied(),
            Mode::Item => None,
        }
    }

    /// The frame without the Symmetry rules: the centre rule and axis rule
    /// of the rulebook table for the whole selection.
    fn plain_frame(&self) -> Result<ActionFrame> {
        crate::Editor::extrude_frame(self.context, self.tool, self.scene)
    }
    /// The primary side for this request: the side of the primary pivot's
    /// centre (ACS-03), in the mesh item's own space (ACS-08; ACS-Q4 open).
    /// `None` when Symmetry doesn't apply.
    pub fn primary(&self) -> Option<Side> {
        let symmetry = self.mesh_symmetry()?;
        let frame = self.frame().ok()?;
        Some(side_of_world(self.scene, symmetry, frame.center))
    }

    /// The full answer about the primary frame the rules give
    /// ([`Self::frame`]): the handle and every group.
    pub fn resolve(&self) -> Result<Resolved> {
        self.resolve_about(self.frame()?, &|_| Ok(Vec::new()))
    }

    /// The full answer about `primary`, the frame the tool holds (the
    /// rules' frame, or one a Move step carried along with the selection).
    /// `reach` names the points of a mesh that the falloff moves beyond the
    /// selection (FO-45/47/48); they join the groups like selected points.
    ///
    /// Groups: with a Local centre, one per cluster, each about its own
    /// centre, with its own axes under Local axes or `primary`'s axes
    /// under any other axis rule (the mixing rule); otherwise one group
    /// about `primary`. Every target carries its Symmetry role and side,
    /// worked out once here from the base scene.
    pub fn resolve_about(
        &self,
        primary: ActionFrame,
        reach: &dyn Fn(u64) -> Result<Vec<u32>>,
    ) -> Result<Resolved> {
        let scene = self.scene;
        let mut resolved = Resolved {
            handle: primary,
            primary: self
                .mesh_symmetry()
                .map(|s| side_of_world(scene, s, primary.center)),
            items: Vec::new(),
        };
        if scene.selection.mode == Mode::Item {
            return Ok(resolved);
        }
        // Every active mesh with a selection moves about the one shared
        // frame (multi_transform.rs); just the primary when it is the only
        // one.
        let meshes: Vec<(u64, BTreeSet<u32>)> = match scene.component_transform_meshes() {
            Some(ids) => ids
                .into_iter()
                .map(|id| Ok((id, scene.mesh_transform_vertices(id)?)))
                .collect::<Result<_>>()?,
            None => vec![(
                scene.active().ok_or("Select a mesh")?.id,
                crate::transform_vertices(scene)?,
            )],
        };
        let local_centres = self.context.center == CenterMode::Local;
        let local_axes = self.context.axis == AxisOrientation::Local;
        let last = self.last_selected_point();
        for (id, mut points) in meshes {
            points.extend(reach(id)?);
            let item = scene.item(id).ok_or("Select a mesh")?;
            // The frames are in world space: each point goes there, takes
            // the transform, and comes back to the item's own space.
            let world = scene.world_matrix(id)?;
            if world.determinant() == 0. || !world.determinant().is_finite() {
                return Err("Component tools need an invertible item transform".into());
            }
            // Several meshes share one frame and each mirrors in its own
            // space (MAM-Q09).
            let map = MirrorMap::new(scene, id, self.symmetry, primary.center)?;
            let target = |v: u32| {
                let p = item.mesh.vertices[v as usize];
                Target {
                    point: v,
                    role: map.role(p),
                    side: map.symmetry.side(p, map.tolerance),
                }
            };
            let groups = if local_centres {
                let selection = if scene.selection.item == Some(id) {
                    std::borrow::Cow::Borrowed(&scene.selection)
                } else {
                    std::borrow::Cow::Owned(scene.selection.sole(id))
                };
                let clusters = crate::local_clusters(&item.mesh, &points, &selection);
                let frame = |piece: LocalPiece| {
                    if local_axes {
                        piece.frame(world, primary.basis)
                    } else {
                        ActionFrame {
                            center: world.transform_point3(piece.center),
                            basis: primary.basis,
                        }
                    }
                };
                let mut groups: Vec<Group> = clusters
                    .pieces
                    .iter()
                    .enumerate()
                    .map(|(i, &piece)| Group {
                        id: GroupId::Cluster(i as u32),
                        frame: frame(piece),
                        twin: frame(piece.mirrored(self.symmetry)),
                        targets: Vec::new(),
                    })
                    .collect();
                for (&v, &cluster) in &clusters.of {
                    groups[cluster].targets.push(target(v));
                }
                // AC-75 / AC-Q2: the one handle sits on the cluster holding
                // the last-selected element and shows that cluster's frame.
                if scene.selection.item == Some(id)
                    && let Some(&cluster) = last.and_then(|v| clusters.of.get(&v))
                {
                    resolved.handle = groups[cluster].frame;
                }
                groups
            } else {
                vec![Group {
                    id: GroupId::Whole,
                    frame: primary,
                    twin: primary,
                    targets: points.iter().map(|&v| target(v)).collect(),
                }]
            };
            resolved.items.push(ItemPivots {
                item: id,
                map,
                groups,
            });
        }
        Ok(resolved)
    }
}

/// Parent and Pivot (rulebook table): the item's pivot (its world position)
/// or its parent's (the world origin without a parent), with the item's or
/// the parent's axes. Pivot Center Parent Axis is Pivot's centre with the
/// parent's axes (ACS-Q6 open). `None` for the other centre rules.
pub fn item_pivot_frame(context: ActionContext, scene: &Scene) -> Option<Result<ActionFrame>> {
    if !matches!(context.center, CenterMode::Parent | CenterMode::Pivot) {
        return None;
    }
    Some((|| {
        let item = scene.active().ok_or("Select a mesh")?;
        let world = scene.world_matrix(item.id)?;
        let parent = match item.parent {
            Some(p) => scene.world_matrix(p)?,
            None => DMat4::IDENTITY,
        };
        let axes = |m: DMat4| {
            let l = DMat3::from_mat4(m);
            DMat3::from_cols(
                l.x_axis.normalize_or(DVec3::X),
                l.y_axis.normalize_or(DVec3::Y),
                l.z_axis.normalize_or(DVec3::Z),
            )
        };
        let center = if context.center == CenterMode::Parent {
            parent.w_axis.truncate()
        } else {
            world.w_axis.truncate()
        };
        let basis = match context.axis {
            crate::AxisOrientation::Parent => axes(parent),
            crate::AxisOrientation::Item => axes(world),
            other => other.basis().unwrap_or(DMat3::IDENTITY),
        };
        Ok(ActionFrame { center, basis })
    })())
}

/// The side of a world point against the active mesh's Symmetry plane
/// (in the item's own space), on-plane counting as − (ACS-03, ACS-08).
pub fn side_of_world(scene: &Scene, symmetry: Symmetry, centre: DVec3) -> Side {
    let Some(item) = scene.active() else {
        return Side::Negative;
    };
    let local = scene
        .world_matrix(item.id)
        .map(|m| m.inverse().transform_point3(centre))
        .unwrap_or(centre);
    primary_side(
        symmetry,
        local,
        crate::symmetry::match_tolerance_shared(item.mesh.shared()),
    )
}

/// How a selected point follows the primary pivot (ACS-03..05).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// On the primary side: takes the edit as entered.
    Primary,
    /// On the other side: mirrored, edited, mirrored back (ACS-04).
    Other,
    /// On the plane: takes the primary edit, then back onto the plane
    /// (ACS-05).
    Seam,
}

/// The mirror rules for one mesh: applies a primary-pivot edit to any of
/// its points (ACS-04..06). Without Symmetry every point is `Primary`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MirrorMap {
    pub symmetry: Symmetry,
    pub primary: Side,
    pub world: DMat4,
    pub local: DMat4,
    pub tolerance: f64,
}

impl MirrorMap {
    /// The map for item `item` of `scene` whose primary pivot is at world
    /// point `centre`.
    pub fn new(scene: &Scene, item: u64, symmetry: Symmetry, centre: DVec3) -> Result<Self> {
        let mesh = &scene.item(item).ok_or("Select a mesh")?.mesh;
        let world = scene.world_matrix(item)?;
        let local = world.inverse();
        // Remembered per mesh allocation: a drag asks every frame.
        let tolerance = crate::symmetry::match_tolerance_shared(mesh.shared());
        Ok(Self {
            symmetry,
            primary: primary_side(symmetry, local.transform_point3(centre), tolerance),
            world,
            local,
            tolerance,
        })
    }
    /// The role of a point at `p` (item space).
    pub fn role(&self, p: DVec3) -> Role {
        match self.symmetry.side(p, self.tolerance) {
            None => Role::Primary,
            Some(Side::Plane) => Role::Seam,
            Some(s) if s == self.primary => Role::Primary,
            Some(_) => Role::Other,
        }
    }
    /// ACS-04: the mirror image of a world frame across the plane (a
    /// reflection, so left-handed; `apply` edits through it by mirroring
    /// the point instead).
    pub fn mirror_frame(&self, frame: ActionFrame) -> ActionFrame {
        let m = |p: DVec3| {
            self.world
                .transform_point3(self.symmetry.mirror(self.local.transform_point3(p)))
        };
        let v = |d: DVec3| {
            self.world
                .transform_vector3(self.symmetry.mirror_vector(self.local.transform_vector3(d)))
        };
        ActionFrame {
            center: m(frame.center),
            basis: DMat3::from_cols(
                v(frame.basis.x_axis),
                v(frame.basis.y_axis),
                v(frame.basis.z_axis),
            ),
        }
    }
    /// ACS-04/05: point `p` (item space) after the primary pivot's world
    /// edit `edit`: as entered on the primary side; mirrored, edited and
    /// mirrored back on the other side; edited and put back onto the plane
    /// on the seam.
    pub fn apply(&self, p: DVec3, edit: impl Fn(DVec3) -> DVec3) -> DVec3 {
        self.apply_as(self.role(p), p, edit)
    }
    /// [`Self::apply`] for a point whose role is already known (a
    /// [`Target`]'s).
    pub fn apply_as(&self, role: Role, p: DVec3, edit: impl Fn(DVec3) -> DVec3) -> DVec3 {
        match role {
            Role::Primary => self
                .local
                .transform_point3(edit(self.world.transform_point3(p))),
            Role::Seam => self.symmetry.onto_plane(
                self.local
                    .transform_point3(edit(self.world.transform_point3(p))),
            ),
            Role::Other => {
                let q = self.symmetry.mirror(p);
                let q = self
                    .local
                    .transform_point3(edit(self.world.transform_point3(q)));
                self.symmetry.mirror(q)
            }
        }
    }
    /// ACS-13: where a point's falloff weight is read (item space): every
    /// point reads it at its + side position (a − side point at its mirror
    /// position), so the fade on the two sides is mirror images. Measured
    /// for both primary sides (FAL-B2-04: with the centre on the plane, so
    /// − primary, a − point still reads at its + mirror); ACS-13's hand
    /// check had the handle on +, where this is "the mirror position" for
    /// the other side.
    pub fn falloff_sample(&self, p: DVec3) -> DVec3 {
        self.falloff_sample_on(self.symmetry.side(p, self.tolerance), p)
    }
    /// [`Self::falloff_sample`] for a point whose side is already known (a
    /// [`Target`]'s).
    pub fn falloff_sample_on(&self, side: Option<Side>, p: DVec3) -> DVec3 {
        match side {
            Some(Side::Negative) => self.symmetry.mirror(p),
            _ => p,
        }
    }
}

/// The live tool's [`Resolved`] answer, kept beside what it was worked out
/// from so a drag resolves its base once and every later frame, the handle
/// and the fields read the same answer. Derived data: it never takes part
/// in comparing editors, and it is checked against its inputs on every
/// read, so a stale copy can't be used.
#[derive(Default)]
pub struct PivotCache(Mutex<Option<Arc<Cached>>>);

/// What a cached answer was resolved from: everything the service reads.
struct Cached {
    tool: Tool,
    context: ActionContext,
    symmetry: Symmetry,
    frame: ActionFrame,
    /// The falloff, when it reaches past the selection: its settings then
    /// choose which points move (FO-45/47/48).
    reaching: Option<crate::ActiveFalloff>,
    /// The base scene's selection, and each moved mesh with its world
    /// matrix (the same allocation is the same mesh).
    selection: Selection,
    meshes: Vec<(u64, Shared<Mesh>, DMat4)>,
    resolved: Arc<Resolved>,
}

impl Cached {
    /// Whether `request` about `frame` would resolve to this same answer:
    /// the Action Center, Symmetry, the frame (a placed centre is in it),
    /// the selection, and the moved meshes and where their items are (the
    /// hierarchy) are all as they were.
    fn answers(
        &self,
        request: &PivotRequest<'_>,
        frame: ActionFrame,
        reaching: &Option<crate::ActiveFalloff>,
    ) -> bool {
        let scene = request.scene;
        self.tool == request.tool
            && self.context == request.context
            && self.symmetry == request.symmetry
            && self.frame == frame
            && self.reaching == *reaching
            && self.selection == scene.selection
            && self.meshes.iter().all(|(id, mesh, world)| {
                scene
                    .item(*id)
                    .is_some_and(|item| Shared::same(item.mesh.shared(), mesh))
                    && scene.world_matrix(*id).ok() == Some(*world)
            })
    }
}

impl Clone for PivotCache {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.get()))
    }
}
impl PartialEq for PivotCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl std::fmt::Debug for PivotCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PivotCache")
    }
}
impl PivotCache {
    fn get(&self) -> Option<Arc<Cached>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    fn set(&self, cached: Cached) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(cached));
    }
}

impl crate::Editor {
    /// The live transform tool's pivots (Move, Rotate, Scale, Transform):
    /// the one answer its preview, its handle and its fields read. Resolved
    /// once per drag base and again whenever the selection, the Action
    /// Center, a placed centre, Symmetry, the item hierarchy or the work
    /// plane (through Symmetry's plane) changes. `None` without such a
    /// tool. (A tool in a UV view runs on a stand-in scene built for each
    /// preview, `uv_pipe.rs`; its answer is about that scene.)
    pub fn live_pivots(&self) -> Option<Result<Arc<Resolved>>> {
        let live = self.session.live.as_ref()?;
        let frame = live.action_frame?;
        if !matches!(
            live.tool,
            Tool::Move | Tool::Rotate | Tool::Scale | Tool::Transform
        ) {
            return None;
        }
        let request = self.pivot_request(&live.base, live.tool);
        // A falloff that reaches past the selection also moves the points
        // its own settings choose (FO-45/47/48).
        let reaching = self.active_falloff().filter(|f| f.reaches());
        if let Some(cached) = live
            .pivots
            .get()
            .filter(|c| c.answers(&request, frame, &reaching))
        {
            return Some(Ok(cached.resolved.clone()));
        }
        let reach = |id: u64| match &reaching {
            Some(falloff) => falloff.reach(&live.base, id),
            None => Ok(Vec::new()),
        };
        Some(request.resolve_about(frame, &reach).map(|r| {
            let resolved = Arc::new(r);
            live.pivots.set(Cached {
                tool: request.tool,
                context: request.context,
                symmetry: request.symmetry,
                frame,
                reaching: reaching.clone(),
                selection: live.base.selection.clone(),
                meshes: resolved
                    .items
                    .iter()
                    .filter_map(|i| {
                        let item = live.base.item(i.item)?;
                        Some((i.item, item.mesh.shared().clone(), i.map.world))
                    })
                    .collect(),
                resolved: resolved.clone(),
            });
            resolved
        }))
    }

    /// Where the live transform tool's handle sits and how its arrows
    /// point: the pivot service's answer ([`Resolved::handle`]), the frame
    /// the tool's values are entered in. `None` without such a tool, when
    /// its pivots can't be resolved, or for a tool in a UV view, whose
    /// handle is its UV frame (`Live::action_frame`).
    pub fn handle_frame(&self) -> Option<ActionFrame> {
        if self.session.live.as_ref()?.params.uv.is_some() {
            return None;
        }
        Some(self.live_pivots()?.ok()?.handle)
    }

    /// The pivot service's request for `tool` on `scene` with the current
    /// Action Center and Symmetry.
    pub fn pivot_request<'a>(&self, scene: &'a Scene, tool: Tool) -> PivotRequest<'a> {
        PivotRequest {
            scene,
            tool,
            context: self.action_context,
            symmetry: self.active_symmetry(),
            placed: None,
        }
    }

    /// The primary side for the live tool (or, with no tool, for the
    /// selection): the side of the live pivot's centre (ACS-03), else of
    /// the selection-based centre (ACS-01). Positive or Negative.
    pub fn symmetry_primary(&self, scene: &Scene) -> Side {
        let symmetry = self.active_symmetry();
        if let Some(frame) = self.session.live.as_ref().and_then(|l| l.action_frame)
            && scene.active().is_some()
        {
            return side_of_world(scene, symmetry, frame.center);
        }
        let tool = self.session.live.as_ref().map_or(Tool::Move, |l| l.tool);
        self.pivot_request(scene, tool)
            .primary()
            .unwrap_or(Side::Negative)
    }

    /// Where the live tool's handle sits under Symmetry (world), for the
    /// anchors that don't follow the action frame: the polygon stage
    /// (Bevel, Extrude, Smooth Shift, Thicken: the live caps on the
    /// primary side, so the handle follows the edit) and the edge and
    /// vertex tools (the primary pivot's centre). `None` with Symmetry off,
    /// several meshes, or nothing to show.
    pub fn symmetry_handle_centre(&self) -> Option<DVec3> {
        use crate::tool_card::HandleAnchor;
        let live = self.session.live.as_ref()?;
        let request = self.pivot_request(&live.base, live.tool);
        let symmetry = request.mesh_symmetry()?;
        let frame = live.action_frame?;
        let anchor = live.tool.card().handles.anchor;
        match anchor {
            HandleAnchor::PolygonStage
                if selection_based(self.action_context.center)
                    && self.action_context.center != CenterMode::Local =>
            {
                let primary = side_of_world(&live.base, symmetry, frame.center);
                let item = self.scene.active()?;
                let world = self.scene.world_matrix(item.id).ok()?;
                let mut points = BTreeSet::new();
                for &f in &self.scene.selection.faces {
                    points.extend(item.mesh.faces.get(f as usize)?.vertices.iter().copied());
                }
                let points: Vec<u32> = points.into_iter().collect();
                if points.is_empty() {
                    return None;
                }
                let half = side_points(&item.mesh, &points, symmetry, primary);
                let local = crate::box_centre(half.iter().map(|&v| item.mesh.vertices[v as usize]));
                local.is_finite().then(|| world.transform_point3(local))
            }
            // Edge and Vertex Slide sit on the first-picked element of the
            // primary side instead (ACS-Q-HANDLE1).
            HandleAnchor::Edge | HandleAnchor::Vertex | HandleAnchor::EdgeRail
                if !live.tool.transforms_items()
                    && crate::handle_pick(live.tool) != crate::HandlePick::FirstPicked =>
            {
                Some(frame.center)
            }
            _ => None,
        }
    }
}
