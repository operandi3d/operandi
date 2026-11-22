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
//! Outputs: a list of [`Pivot`]s, each a world frame, the points it drives
//! and whether it is the primary pivot or its mirror image. Without
//! Symmetry there is one pivot (one per island under Local); with Symmetry
//! there is the primary pivot and its mirror.
//!
//! Every tool that pivots or picks a Symmetry side asks this module:
//! Move, Rotate, Scale, Transform (and Stretch, Scale's non-uniform use),
//! the polygon stage tools' handles (Bevel, Extrude, Smooth Shift,
//! Thicken), Edge Extend, the slide/deform tools' side choice, Bend's
//! side, Twist Bevel's and the two-pass phase 2 tools. Nothing else
//! computes a handle centre under Symmetry or picks a side.
use crate::symmetry::{Side, Symmetry, match_tolerance};
use crate::{ActionContext, ActionFrame, CenterMode, Mesh, Mode, Result, Scene, Tool};
use glam::{DMat3, DMat4, DVec3};
use std::collections::BTreeSet;

/// One pivot: a world frame, the item's points it drives and whether it is
/// the mirror image of the primary pivot (ACS-04).
#[derive(Clone, Debug, PartialEq)]
pub struct Pivot {
    pub frame: ActionFrame,
    pub points: Vec<u32>,
    pub mirrored: bool,
}

/// Everything a tool needs to apply its values once per pivot.
#[derive(Clone, Debug, PartialEq)]
pub struct Pivots {
    pub pivots: Vec<Pivot>,
    /// The primary side under Symmetry (Positive or Negative, never
    /// Plane); `None` with Symmetry off or not applying (Item mode,
    /// several active meshes).
    pub primary: Option<Side>,
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

/// The selection-based centre of `points` (item space) for `center`: the
/// bounding-box middle (Automatic, None, Selection, Selection Center Auto
/// Axis, Local's handle; the rulebook table) or the border centre
/// (Selection Border: the average of the border points among `points`).
fn centre_of(mesh: &Mesh, points: &[u32], center: CenterMode, border: &BTreeSet<u32>) -> DVec3 {
    let at = |v: &u32| mesh.vertices[*v as usize];
    if center == CenterMode::SelectionBorder {
        let on_border: Vec<DVec3> = points
            .iter()
            .filter(|v| border.contains(v))
            .map(at)
            .collect();
        if !on_border.is_empty() {
            return on_border.iter().copied().sum::<DVec3>() / on_border.len() as f64;
        }
    }
    crate::box_centre(points.iter().map(at))
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
    /// pivots. The axes always come from the whole selection (ACS-02); the
    /// centre follows the rulebook table, ACS-01 and ACS-07.
    pub fn frame(&self) -> Result<ActionFrame> {
        let plain = self.plain_frame()?;
        if let Some(at) = self.placed {
            return Ok(ActionFrame {
                center: at,
                ..plain
            });
        }
        let Some(symmetry) = self.mesh_symmetry() else {
            return Ok(plain);
        };
        if !selection_based(self.context.center) {
            // Origin, Parent, Pivot, Element and Screen have fixed centres.
            return Ok(plain);
        }
        let item = self.scene.active().ok_or("Select a mesh")?;
        let world = self.scene.world_matrix(item.id)?;
        let points: Vec<u32> = crate::transform_vertices(self.scene)?.into_iter().collect();
        let half = half_points(&item.mesh, &points, symmetry);
        let border = border_points(self.scene, &points);
        let center = match self.context.center {
            // Automatic and None take their box in world space (as their
            // plain frame does); the others in the item's own space
            // (AC-Q20).
            CenterMode::Automatic | CenterMode::None => crate::box_centre(
                half.iter()
                    .map(|&v| world.transform_point3(item.mesh.vertices[v as usize])),
            ),
            c => world.transform_point3(centre_of(&item.mesh, &half, c, &border)),
        };
        if !center.is_finite() {
            return Ok(plain);
        }
        Ok(ActionFrame { center, ..plain })
    }

    /// AC-75: under Local, the one handle's axes (world): the frame of the
    /// cluster holding the last selected element (polygon, edge or point,
    /// in pick order), so the handle's arrows match what that cluster
    /// does. `None` for other centre rules, in Item mode or with nothing
    /// selected (the handle keeps the tool's axes then).
    pub fn local_handle_basis(&self) -> Option<DMat3> {
        self.local_handle_frame().map(|f| f.basis)
    }

    /// AC-75 / AC-Q2 (Jason, 2026-11-21): the Local handle sits on the last
    /// selected element's cluster, at its centre (box middle), with its
    /// axes; not between the clusters.
    pub fn local_handle_frame(&self) -> Option<ActionFrame> {
        if self.context.center != CenterMode::Local {
            return None;
        }
        let selection = &self.scene.selection;
        let item = self.scene.active()?;
        let last = match selection.mode {
            Mode::Polygon => item
                .mesh
                .faces
                .get(*selection.faces.last()? as usize)?
                .vertices
                .first()
                .copied()?,
            Mode::Edge => selection.edges.last()?.0,
            Mode::Vertex => *selection.vertices.last()?,
            Mode::Item => return None,
        };
        let tool_axes = self.plain_frame().ok()?.basis;
        let points: BTreeSet<u32> = crate::transform_vertices(self.scene).ok()?;
        let pieces = crate::local_piece_frames(&item.mesh, &points, selection);
        let world = self.scene.world_matrix(item.id).ok()?;
        Some(pieces.get(&last)?.frame(world, tool_axes))
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

    /// The full answer: the primary pivot and, with Symmetry, its mirror
    /// image, each with the selected points it drives. Under Local, one
    /// pivot per island (each mirrored island's own pivot, ACS-10/11).
    pub fn pivots(&self) -> Result<Pivots> {
        let frame = self.frame()?;
        let item = self.scene.active().ok_or("Select a mesh")?;
        let points: Vec<u32> = if self.scene.selection.mode == Mode::Item {
            Vec::new()
        } else {
            crate::transform_vertices(self.scene)?.into_iter().collect()
        };
        let world = self.scene.world_matrix(item.id)?;
        if self.context.center == CenterMode::Local && !points.is_empty() {
            let moved: BTreeSet<u32> = points.iter().copied().collect();
            let pieces = crate::local_piece_frames(&item.mesh, &moved, &self.scene.selection);
            let mut out: Vec<Pivot> = Vec::new();
            for (&v, piece) in &pieces {
                let f = piece.frame(world, frame.basis);
                match out.iter_mut().find(|p| p.frame == f) {
                    Some(p) => p.points.push(v),
                    None => out.push(Pivot {
                        frame: f,
                        points: vec![v],
                        mirrored: false,
                    }),
                }
            }
            return Ok(Pivots {
                pivots: out,
                primary: self
                    .mesh_symmetry()
                    .map(|s| side_of_world(self.scene, s, frame.center)),
            });
        }
        let Some(symmetry) = self.mesh_symmetry() else {
            return Ok(Pivots {
                pivots: vec![Pivot {
                    frame,
                    points,
                    mirrored: false,
                }],
                primary: None,
            });
        };
        let map = MirrorMap::new(self.scene, item.id, symmetry, frame.center)?;
        let (mut primary_points, mut mirrored_points) = (Vec::new(), Vec::new());
        for &v in &points {
            match map.role(item.mesh.vertices[v as usize]) {
                Role::Other => mirrored_points.push(v),
                _ => primary_points.push(v),
            }
        }
        let mut pivots = vec![Pivot {
            frame,
            points: primary_points,
            mirrored: false,
        }];
        pivots.push(Pivot {
            frame: map.mirror_frame(frame),
            points: mirrored_points,
            mirrored: true,
        });
        Ok(Pivots {
            pivots,
            primary: Some(map.primary),
        })
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
#[derive(Clone, Copy, Debug)]
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
        match self.role(p) {
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
        match self.symmetry.side(p, self.tolerance) {
            Some(Side::Negative) => self.symmetry.mirror(p),
            _ => p,
        }
    }
}

/// Points on the border of the selected polygons (edges used by one
/// selected polygon); all of `moved` when there is no such border.
fn border_points(scene: &Scene, moved: &[u32]) -> BTreeSet<u32> {
    let (Some(item), Mode::Polygon) = (scene.active(), scene.selection.mode) else {
        return moved.iter().copied().collect();
    };
    let mut uses: std::collections::BTreeMap<(u32, u32), usize> = Default::default();
    for &f in &scene.selection.faces {
        let Some(face) = item.mesh.faces.get(f as usize) else {
            continue;
        };
        let n = face.vertices.len();
        for i in 0..n {
            let (a, b) = (face.vertices[i], face.vertices[(i + 1) % n]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    let border: BTreeSet<u32> = uses
        .iter()
        .filter(|(_, n)| **n == 1)
        .flat_map(|((a, b), _)| [*a, *b])
        .collect();
    if border.is_empty() {
        moved.iter().copied().collect()
    } else {
        border
    }
}

impl crate::Editor {
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
        if let Some(frame) = self.live.as_ref().and_then(|l| l.action_frame)
            && scene.active().is_some()
        {
            return side_of_world(scene, symmetry, frame.center);
        }
        let tool = self.live.as_ref().map_or(Tool::Move, |l| l.tool);
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
        let live = self.live.as_ref()?;
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
