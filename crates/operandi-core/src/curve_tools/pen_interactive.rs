//! Interactive Polygon Pen. Pen creates geometry vertex
//! by vertex by clicking in the viewport; with Type Polygons each click adds
//! a vertex to the polygon. Points are captured in world space on the
//! viewport's construction plane; on Drop the polygon is added to the active
//! mesh (empty or not), mapped into the item's local space. Scene data is
//! published once, on valid Drop.
use crate::*;
use glam::DVec3;

/// Panel options of the interactive Pen (spec `pen-2026-11-05`). Flip
/// reverses the polygon; Close applies to Polyline and to walls. Wall Mode
/// turns the clicked path into a strip of quads; Inset and Segments round
/// its bends; Make UVs maps each corner to its world X and Y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenOptions {
    pub output: PenType,
    pub flip: bool,
    pub close: bool,
    pub select_new: bool,
    pub wall: WallMode,
    /// Wall half-width (Both) or width (Inner/Outer), metres.
    pub offset: f64,
    /// Wall corner rounding radius, metres (0: mitred corners).
    pub inset: f64,
    /// Straight pieces per rounded corner.
    pub segments: u32,
    /// UV map "Texture" (Project To Backdrop Item): from the picture of the
    /// Backdrop drawn in `uv_view` (BD-62), else world X and Y (PN12, PN13).
    pub make_uvs: bool,
    /// The view the points are drawn in, for Make UVs (BD-62). `None` (the
    /// numeric route, or a view not showing Backdrops) keeps world X and Y.
    pub uv_view: Option<crate::backdrop::ViewKind>,
    /// The drawing plane's normal: walls lie in this plane.
    pub plane_normal: DVec3,
    /// Make Quads (SPEC-PEN-MAKE-QUADS): the clicks build a strip of quads.
    /// Only the surface types without Wall Mode use it ([`Self::strip`]).
    pub make_quads: bool,
}
impl Default for PenOptions {
    fn default() -> Self {
        Self {
            output: PenType::Polygon,
            flip: false,
            close: false,
            select_new: true,
            wall: WallMode::Off,
            offset: 0.,
            inset: 0.,
            segments: 10,
            make_uvs: false,
            uv_view: None,
            plane_normal: DVec3::Y,
            make_quads: false,
        }
    }
}

/// Pen Wall Mode: the points are the centre line of a strip. Inner puts the
/// strip on the left of the direction of travel (the line is its other
/// edge), Outer on the right, Both centres it (2 × Offset wide).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WallMode {
    #[default]
    Off,
    BothSides,
    Inner,
    Outer,
}

/// The wall for `path` (pen-2026-11-05 rules 8–15): points and polygons
/// (indices into the points), facing like a polygon drawn through the same
/// points. Corners are mitred; with `inset` > 0 each bend is rounded by an
/// arc of that radius in `segments` pieces (both sides with Both, the
/// outside of the bend with Inner, filled with a fan from the inside point).
/// Outer with Inset is refused (Q-PEN-01).
pub fn pen_wall(path: &[DVec3], o: &PenOptions) -> Result<(Vec<DVec3>, Vec<Vec<u32>>)> {
    let n = path.len();
    let closed = o.close && n >= 3;
    if n < 2 || o.wall == WallMode::Off {
        return Ok((Vec::new(), Vec::new()));
    }
    if o.inset > 0. && o.wall == WallMode::Outer {
        return Err("Pen Outer walls with Inset aren't specified yet; use Both or Inner".into());
    }
    let normal = o.plane_normal.normalize_or(DVec3::Y);
    let segment_count = if closed { n } else { n - 1 };
    let dir = |k: usize| (path[(k + 1) % n] - path[k]).normalize_or_zero();
    if (0..segment_count).any(|k| dir(k) == DVec3::ZERO) {
        return Err("Pen wall points must be apart".into());
    }
    let left = |k: usize| normal.cross(dir(k)).normalize_or_zero();
    let (sl, sr) = match o.wall {
        WallMode::BothSides => (o.offset, -o.offset),
        WallMode::Inner => (o.offset, 0.),
        _ => (0., -o.offset),
    };
    let mut points: Vec<DVec3> = Vec::new();
    let push = |q: DVec3, points: &mut Vec<DVec3>| {
        points.push(q);
        (points.len() - 1) as u32
    };
    // Per path point: its left run and right run of points.
    let mut runs: Vec<(Vec<u32>, Vec<u32>)> = Vec::with_capacity(n);
    for i in 0..n {
        let incoming = if i > 0 {
            Some(i - 1)
        } else if closed {
            Some(n - 1)
        } else {
            None
        };
        let outgoing = (i < segment_count).then_some(i);
        let side = |s: f64, outside: bool, points: &mut Vec<DVec3>| -> Result<Vec<u32>> {
            let p = path[i];
            let (k0, k1) = match (incoming, outgoing) {
                (Some(a), Some(b)) => (a, b),
                // Open ends: offset at right angles to the end segment.
                (Some(a), None) | (None, Some(a)) => {
                    return Ok(vec![push(p + left(a) * s, points)]);
                }
                (None, None) => unreachable!(),
            };
            let (n0, n1, d0, d1) = (left(k0), left(k1), dir(k0), dir(k1));
            let denominator = 1. + n0.dot(n1);
            if denominator < 1e-9 {
                return Err("Pen walls can't turn back on themselves".into());
            }
            let mitre = p + (n0 + n1) * (s / denominator);
            let turn = d0.cross(d1).dot(normal);
            let angle = d0.angle_between(d1);
            let round = o.inset > 0. && o.segments > 0 && angle > 1e-9 && outside;
            if !round {
                return Ok(vec![push(mitre, points)]);
            }
            // Fillet of radius Inset tangent to both edges.
            let reach = o.inset * (angle / 2.).tan();
            let (l0, l1) = (
                (path[(k0 + 1) % n] - path[k0]).length(),
                (path[(k1 + 1) % n] - path[k1]).length(),
            );
            if reach >= l0.min(l1) {
                return Err("Pen Inset is too large for these wall segments".into());
            }
            let sign = turn.signum();
            let start = mitre - d0 * reach;
            let center = start + n0 * (o.inset * sign);
            let arm = start - center;
            Ok((0..=o.segments)
                .map(|j| {
                    let r = glam::DQuat::from_axis_angle(
                        normal,
                        sign * angle * j as f64 / o.segments as f64,
                    );
                    push(center + r * arm, points)
                })
                .collect())
        };
        let turn = match (incoming, outgoing) {
            (Some(a), Some(b)) => dir(a).cross(dir(b)).dot(normal),
            _ => 0.,
        };
        // The outside of a left bend is the right side, and the other way.
        let (round_left, round_right) = match o.wall {
            WallMode::BothSides => (true, true),
            _ => (turn < 0., turn > 0.),
        };
        let l = side(sl, round_left, &mut points)?;
        let r = side(sr, round_right, &mut points)?;
        runs.push((l, r));
    }
    let mut polygons: Vec<Vec<u32>> = Vec::new();
    for (l, r) in &runs {
        match (l.len(), r.len()) {
            (a, b) if a == b => {
                for j in 0..a - 1 {
                    polygons.push(vec![l[j], r[j], r[j + 1], l[j + 1]]);
                }
            }
            (1, _) => {
                for j in 0..r.len() - 1 {
                    polygons.push(vec![l[0], r[j], r[j + 1]]);
                }
            }
            _ => {
                for j in 0..l.len() - 1 {
                    polygons.push(vec![l[j], r[0], l[j + 1]]);
                }
            }
        }
    }
    for k in 0..segment_count {
        let (a, b) = (&runs[k], &runs[(k + 1) % n]);
        polygons.push(vec![
            *a.0.last().unwrap(),
            *a.1.last().unwrap(),
            b.1[0],
            b.0[0],
        ]);
    }
    // Facing: as a polygon drawn through the same points.
    let winding: DVec3 = (0..n).map(|i| path[i].cross(path[(i + 1) % n])).sum();
    let reverse = winding.dot(normal) < 0.;
    if reverse != o.flip {
        for f in &mut polygons {
            f.reverse();
        }
    }
    Ok((points, polygons))
}

/// The Make Quads strip through `points` (SPEC-PEN-MAKE-QUADS): the points
/// come in pairs (left, right), the first pair being the starting edge; pair
/// k−1 and pair k close the quad `[l(k−1), r(k−1), r(k), l(k)]`. An odd last
/// point closes the end with the triangle `[l, r, point]`. Facing follows
/// that order (provisional, PMQ-Q2); `flip` reverses every polygon.
pub fn pen_strip(points: &[DVec3], flip: bool) -> Vec<Vec<u32>> {
    let pairs = points.len() / 2;
    let mut polygons: Vec<Vec<u32>> = (1..pairs)
        .map(|k| {
            let (l0, r0, l1, r1) = (2 * k - 2, 2 * k - 1, 2 * k, 2 * k + 1);
            vec![l0 as u32, r0 as u32, r1 as u32, l1 as u32]
        })
        .collect();
    if pairs >= 1 && points.len() % 2 == 1 {
        let l = 2 * pairs - 2;
        polygons.push(vec![l as u32, l as u32 + 1, points.len() as u32 - 1]);
    }
    if flip {
        for f in &mut polygons {
            f.reverse();
        }
    }
    polygons
}

/// The partner of a strip click at `p` (PEN-06): `p` moved by the strip's
/// last rung (the last pair's left → right), turned by as much as the
/// strip's side turns at the last left point, so a curving strip keeps
/// its rungs across it instead of copying one world offset (PMQ-Q4,
/// provisional). The first click after the starting edge has no turn yet
/// and takes the starting edge as drawn. `points` holds whole pairs.
pub fn pen_strip_partner(points: &[DVec3], p: DVec3) -> Option<DVec3> {
    let pairs = points.len() / 2;
    if pairs == 0 {
        return None;
    }
    let (left, right) = (points[2 * pairs - 2], points[2 * pairs - 1]);
    let rung = right - left;
    if pairs < 2 {
        return Some(p + rung);
    }
    // The left side's last step and the step this click adds.
    let before = (left - points[2 * pairs - 4]).normalize_or_zero();
    let now = (p - left).normalize_or_zero();
    // Nearly doubling back has no single turn: keep the rung as it is.
    if before == DVec3::ZERO || now == DVec3::ZERO || before.dot(now) < -0.999 {
        return Some(p + rung);
    }
    Some(p + glam::DQuat::from_rotation_arc(before, now) * rung)
}

/// The geometry a Pen Drop builds from `points` (pen-2026-11-05 rules 1–7):
/// world positions, polygons and their types.
pub fn pen_geometry(points: &[DVec3], o: &PenOptions) -> Result<Mesh> {
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    };
    let mut mesh = Mesh::default();
    if o.wall != WallMode::Off {
        let (vertices, polygons) = pen_wall(points, o)?;
        mesh.vertices = vertices;
        mesh.faces = polygons.into_iter().map(face).collect();
    } else if o.strip() {
        mesh.vertices = points.to_vec();
        mesh.faces = pen_strip(points, o.flip).into_iter().map(face).collect();
        let representation = match o.output {
            PenType::Subdivs => Some(PolygonRepresentation::Subdivs),
            PenType::CatmullClark => Some(PolygonRepresentation::CatmullClark),
            _ => None,
        };
        if let Some(r) = representation {
            mesh.attributes.representations = vec![r; mesh.faces.len()];
        }
    } else {
        mesh.vertices = points.to_vec();
        let all: Vec<u32> = (0..points.len() as u32).collect();
        match o.output {
            PenType::Vertices => {}
            PenType::Polyline | PenType::BSpline => {
                mesh.faces.push(face(all));
                mesh.attributes
                    .representations
                    .push(PolygonRepresentation::Polyline {
                        closed: o.close,
                        curve: if o.output == PenType::BSpline {
                            crate::curves::CurveKind::BSpline
                        } else {
                            crate::curves::CurveKind::Line
                        },
                    });
            }
            surface => {
                let mut vertices = all;
                if o.flip {
                    vertices.reverse();
                }
                mesh.faces.push(face(vertices));
                let representation = match surface {
                    PenType::Subdivs => PolygonRepresentation::Subdivs,
                    PenType::CatmullClark => PolygonRepresentation::CatmullClark,
                    _ => PolygonRepresentation::Face,
                };
                if representation != PolygonRepresentation::Face {
                    mesh.attributes.representations.push(representation);
                }
            }
        }
    }
    if o.make_uvs && !mesh.faces.is_empty() {
        let rows = mesh
            .faces
            .iter()
            .map(|f| {
                f.vertices
                    .iter()
                    .map(|&v| {
                        let q = mesh.vertices[v as usize];
                        Some(glam::DVec2::new(q.x, q.y))
                    })
                    .collect()
            })
            .collect();
        mesh.uv_maps.insert("Texture".into(), rows);
    }
    Ok(mesh)
}
/// BD-62: replace the world X/Y Texture UVs with each point's place on the
/// Backdrop picture under it (`mesh` still in world space).
fn backdrop_uvs(mesh: &mut Mesh, frame: &crate::backdrop::PictureFrame) {
    let Some(rows) = mesh.uv_maps.get_mut("Texture") else {
        return;
    };
    for (face, row) in mesh.faces.iter().zip(rows.iter_mut()) {
        for (uv, &v) in row.iter_mut().zip(&face.vertices) {
            *uv = Some(frame.uv(mesh.vertices[v as usize]));
        }
    }
}
impl PenOptions {
    /// Make Quads is in effect: on, a polygon type, and no Wall Mode.
    pub fn strip(&self) -> bool {
        self.make_quads && self.output.surface() && self.wall == WallMode::Off
    }
    pub fn validate(&self) -> Result<()> {
        if self.wall != WallMode::Off && !self.output.surface() {
            return Err("Pen Wall Mode applies to the polygon types".into());
        }
        if self.make_quads && self.wall != WallMode::Off {
            return Err("Make Quads with Wall Mode isn't specified yet; turn one off".into());
        }
        if !self.offset.is_finite() || self.offset < 0. || self.offset > 1e6 {
            return Err("Pen Offset must be 0 or more".into());
        }
        if !self.inset.is_finite() || self.inset < 0. || self.inset > 1e6 {
            return Err("Pen Inset must be 0 or more".into());
        }
        if !(1..=256).contains(&self.segments) {
            return Err("Pen Segments must be 1–256".into());
        }
        if !self.plane_normal.is_finite() || self.plane_normal.length() < 1e-9 {
            return Err("Pen needs a drawing plane".into());
        }
        Ok(())
    }
    /// Fewest captured points that make this type.
    pub fn minimum_points(&self) -> usize {
        if self.wall != WallMode::Off {
            return 2;
        }
        if self.strip() {
            // Two starting-edge points and a Ctrl-click triangle.
            return 3;
        }
        match self.output {
            PenType::Polyline | PenType::BSpline => 2,
            PenType::Vertices => 1,
            _ => 3,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PenDraft {
    target: u64,
    points: Vec<DVec3>,
    current: Option<usize>,
    options: PenOptions,
    /// A Make Quads strip faces the other way (PenReverse keeps the pairs).
    strip_flipped: bool,
    /// DPM-13: the draft before each add, delete and drag, newest last;
    /// Pen's Undo steps back through it.
    history: Vec<(Vec<DVec3>, Option<usize>)>,
    /// A drag is under way (or a click just added a point the drag may
    /// move): further refinements belong to the same undo step.
    refining: bool,
    /// The Make Quads click just placed at this index: until another
    /// point is picked, dragging it guesses its partner again (PMQ-Q4).
    guessing: Option<usize>,
}

/// Most undo steps a Pen draft keeps (each holds a copy of its points).
const PEN_HISTORY: usize = 256;

impl PenDraft {
    fn remember(&mut self) {
        if self.history.len() >= PEN_HISTORY {
            self.history.remove(0);
        }
        self.history.push((self.points.clone(), self.current));
    }
    /// The points a delete of `index` removes: in a Make Quads strip the
    /// point's pair (the starting edge, or a clicked point and its partner;
    /// a closing triangle point alone), otherwise just the point (D2).
    fn delete_range(&self, index: usize) -> std::ops::Range<usize> {
        if !self.options.strip() {
            return index..index + 1;
        }
        let start = index - index % 2;
        start..(start + 2).min(self.points.len())
    }
}
impl PenDraft {
    pub fn options(&self) -> PenOptions {
        self.options
    }
    pub fn points(&self) -> &[DVec3] {
        &self.points
    }
    pub fn current(&self) -> Option<usize> {
        self.current
    }
    /// The options the draft builds with: a reversed strip flips its polygons.
    pub fn build_options(&self) -> PenOptions {
        let mut o = self.options;
        o.flip ^= self.options.strip() && self.strip_flipped;
        o
    }
    /// The strip ends with its Ctrl-click triangle; it takes no more points.
    pub fn strip_ended(&self) -> bool {
        self.options.strip() && self.points.len() >= 3 && self.points.len() % 2 == 1
    }
}

/// The mirror half of a Pen draft, in world space, for the live preview
/// (SYM3-24): its points and its polygons or lines as point rings.
#[derive(Clone, Debug, PartialEq)]
pub struct PenMirrorPreview {
    pub points: Vec<DVec3>,
    pub rings: Vec<Vec<DVec3>>,
    /// True for open polylines (drawn without closing the ring).
    pub open: bool,
    pub creation: crate::symmetry::Creation,
}

impl Editor {
    /// What Drop would add as the draft's mirror half, in world space;
    /// `None` with Symmetry off or nothing drawn yet.
    pub fn pen_mirror_preview(&self) -> Option<PenMirrorPreview> {
        let symmetry = self.active_symmetry();
        symmetry.axis?;
        let draft = self.session.pen_draft.as_ref()?;
        if draft.points.is_empty() {
            return None;
        }
        let mut options = draft.build_options();
        if draft.points.len() < options.minimum_points() {
            // The points so far, as Drop with too few points would leave.
            options.output = PenType::Vertices;
            options.make_quads = false;
            options.wall = WallMode::Off;
        }
        options.make_uvs = false;
        let world = self.scene.world_matrix(draft.target).ok()?;
        let local = world.inverse();
        let mut mesh = pen_geometry(&draft.points, &options).ok()?;
        let open = matches!(options.output, PenType::Polyline | PenType::BSpline) && !options.close;
        for v in &mut mesh.vertices {
            *v = local.transform_point3(*v);
        }
        let (n, m) = (mesh.vertices.len(), mesh.faces.len());
        let (out, creation) = symmetry.welded(mesh).ok()?;
        let to_world = |v: u32| world.transform_point3(out.vertices[v as usize]);
        Some(PenMirrorPreview {
            points: (n..out.vertices.len())
                .map(|v| to_world(v as u32))
                .collect(),
            rings: out.faces[m.min(out.faces.len())..]
                .iter()
                .map(|f| f.vertices.iter().map(|&v| to_world(v)).collect())
                .collect(),
            open,
            creation,
        })
    }
    pub(crate) fn interactive_pen_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::PenStart => {
                if self.session.pen_draft.is_some() {
                    return Ok(());
                }
                if self.selection_start.is_some() {
                    return Err("Finish selection before Pen".into());
                }
                // Another open tool was dropped first by `tool_switch`.
                if self.session.has_active_task() || self.i2_active() {
                    return Err("Finish the current task before Pen".into());
                }
                self.scene.validate()?;
                if self.scene.selection.mode != Mode::Polygon || self.spatial_falloff.is_some() {
                    return Err("Pen requires Polygon mode without a falloff".into());
                }
                let item = self
                    .scene
                    .active()
                    .filter(|i| self.scene.shows(i))
                    .ok_or("Select a visible mesh for Pen")?;
                if self.scene.world_matrix(item.id)?.determinant() == 0. {
                    return Err("Pen needs an invertible mesh transform".into());
                }
                self.session.pen_draft = Some(PenDraft {
                    target: item.id,
                    points: Vec::new(),
                    current: None,
                    options: self.pen_options,
                    strip_flipped: false,
                    history: Vec::new(),
                    refining: false,
                    guessing: None,
                });
                Ok(())
            }
            Command::PenCapture(point) => {
                if !point.is_finite() || point.abs().max_element() > 1e9 {
                    return Err("Capture a finite point within the scene range".into());
                }
                // Constrain to Background: each new point lands on it.
                let point = self.constrain_new_point(point)?;
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                // Points on top of each other are kept (pen-2026-11-05 rule 6).
                if draft.points.len() >= 4096 {
                    return Err("Pen keeps at most 4096 points".into());
                }
                if draft.strip_ended() {
                    return Err(
                        "This strip ends with its triangle; Shift+click starts a new strip".into(),
                    );
                }
                // Make Quads: after the starting edge each click adds the
                // point and its partner, closing one more quad (PEN-06).
                let partner = draft
                    .options
                    .strip()
                    .then(|| pen_strip_partner(&draft.points, point))
                    .flatten();
                // The partner lands on the background too (PMQ-Q5).
                let partner = match partner {
                    Some(p) if p.is_finite() => Some(self.constrain_strip_partner(point, p)?),
                    other => other,
                };
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                if partner.is_some_and(|p| !p.is_finite() || p.abs().max_element() > 1e9) {
                    return Err("The strip's next point would leave the scene range".into());
                }
                // DPM-13: the click and the drag that may follow it are one
                // undo step.
                draft.remember();
                draft.refining = true;
                draft.guessing = None;
                if let Some(partner) = partner {
                    draft.points.extend([point, partner]);
                    draft.current = Some(draft.points.len() - 2);
                    draft.guessing = draft.current;
                } else {
                    // DPM-09: a new point goes right after the current one
                    // (at the end while the last point is current). Strips
                    // always grow at the end: their points come in pairs.
                    let at = draft
                        .current
                        .map_or(draft.points.len(), |c| (c + 1).min(draft.points.len()));
                    draft.points.insert(at, point);
                    draft.current = Some(at);
                }
                Ok(())
            }
            Command::PenCaptureEnd(point) => {
                if !point.is_finite() || point.abs().max_element() > 1e9 {
                    return Err("Capture a finite point within the scene range".into());
                }
                let point = self.constrain_new_point(point)?;
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                if !draft.options.strip() {
                    return Err("Ctrl+click ends a Make Quads strip; turn Make Quads on".into());
                }
                if draft.points.len() < 2 {
                    return Err("Click the strip's starting edge (two points) first".into());
                }
                if draft.strip_ended() {
                    return Err("This strip already ends with a triangle".into());
                }
                if draft.points.len() >= 4096 {
                    return Err("Pen keeps at most 4096 points".into());
                }
                // A single point closes the strip's end with a triangle.
                draft.guessing = None;
                draft.remember();
                draft.refining = true;
                draft.points.push(point);
                draft.current = Some(draft.points.len() - 1);
                Ok(())
            }
            Command::PenSelect(index) => {
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                if index >= draft.points.len() {
                    return Err("Current Point is outside the captured points".into());
                }
                draft.current = Some(index);
                // Picking a point starts a new drag (a new undo step), and
                // its partner stays where it is.
                draft.refining = false;
                draft.guessing = None;
                Ok(())
            }
            Command::PenRefine(point) => {
                if !point.is_finite() || point.abs().max_element() > 1e9 {
                    return Err("Pen coordinates must be finite and in range".into());
                }
                let point = self.constrain_new_point(point)?;
                let draft = self
                    .session
                    .pen_draft
                    .as_ref()
                    .ok_or("Activate Pen first")?;
                let index = draft.current.ok_or("Capture a point before refinement")?;
                if draft.points[index] == point {
                    return Ok(());
                }
                // The drag right after a Make Quads click carries the
                // click's partner along, guessed again from where the
                // point is now (PMQ-Q4).
                let partner = match draft.guessing {
                    Some(at) if at == index && index + 1 < draft.points.len() => {
                        match pen_strip_partner(&draft.points[..index], point) {
                            Some(p) if p.is_finite() => {
                                Some(self.constrain_strip_partner(point, p)?)
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                }
                .filter(|p| p.abs().max_element() <= 1e9);
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                // DPM-13: one undo step per drag, however many moves.
                if !draft.refining {
                    draft.remember();
                    draft.refining = true;
                }
                draft.points[index] = point;
                if let Some(partner) = partner {
                    draft.points[index + 1] = partner;
                }
                Ok(())
            }
            Command::PenDelete { index, and_after } => {
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                if index >= draft.points.len() {
                    return Err("No Pen point there to delete".into());
                }
                let range = draft.delete_range(index);
                draft.remember();
                draft.refining = false;
                draft.guessing = None;
                if and_after {
                    draft.points.truncate(range.start);
                } else {
                    draft.points.drain(range.clone());
                }
                // The point before the gap is current, so the next Add
                // click goes back where the deleted one was (DPM-T12).
                draft.current = if draft.points.is_empty() {
                    None
                } else {
                    Some(range.start.saturating_sub(1).min(draft.points.len() - 1))
                };
                Ok(())
            }
            Command::PenOptions(options) => {
                options.validate()?;
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                // Strip points come in pairs; reading them another way
                // mid-draft would scramble the polygon (provisional).
                if !draft.points.is_empty() && options.strip() != draft.options.strip() {
                    return Err(
                        "Make Quads can't change mid-draft; Drop or Shift+click to start anew"
                            .into(),
                    );
                }
                draft.options = options;
                // Kept for the next Pen session, like other tools' options.
                self.pen_options = options;
                Ok(())
            }
            Command::PenReverse => {
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                draft.refining = false;
                draft.guessing = None;
                if draft.options.strip() {
                    draft.strip_flipped = !draft.strip_flipped;
                    return Ok(());
                }
                draft.points.reverse();
                // Earlier steps are in the old order: reverse them too, so
                // Undo still takes back the last point added.
                for (points, current) in &mut draft.history {
                    let last = points.len().saturating_sub(1);
                    points.reverse();
                    *current = current.map(|i| last - i);
                }
                let last = draft.points.len().saturating_sub(1);
                draft.current = draft.current.map(|i| last - i);
                Ok(())
            }
            Command::Undo => {
                let draft = self
                    .session
                    .pen_draft
                    .as_mut()
                    .ok_or("Activate Pen first")?;
                draft.refining = false;
                draft.guessing = None;
                // DPM-13/T14: step back over the last add, drag or delete;
                // a deleted point returns at its index.
                if let Some((points, current)) = draft.history.pop() {
                    draft.points = points;
                    draft.current = current;
                    return Ok(());
                }
                // A strip click took two points: undo takes both
                // (provisional, PMQ-Q3).
                let pair =
                    draft.options.strip() && draft.points.len() >= 4 && draft.points.len() % 2 == 0;
                if pair {
                    draft.points.pop();
                }
                if draft.points.pop().is_none() {
                    return Err("No captured Pen point to undo".into());
                }
                draft.current = draft.points.len().checked_sub(1);
                Ok(())
            }
            Command::Cancel => {
                self.session.pen_draft = None;
                Ok(())
            }
            Command::Drop => self.finish_interactive_pen(),
            _ => Err("Finish or Cancel the Pen draft before changing tools or selection".into()),
        }
    }
    fn finish_interactive_pen(&mut self) -> Result<()> {
        let draft = self
            .session
            .pen_draft
            .as_ref()
            .ok_or("Activate Pen first")?;
        let mut options = draft.build_options();
        options.validate()?;
        let count = draft.points.len();
        // Drop with too few points (draw-tools Pen, seen): a polygon of two
        // points leaves its two points; anything shorter is discarded.
        if count < options.minimum_points() && count > 0 {
            if options.output.surface() && count == 2 && options.wall == WallMode::Off {
                options.output = PenType::Vertices;
                options.flip = false;
                options.make_quads = false;
            } else {
                self.session.pen_draft = None;
                return Ok(());
            }
        }
        if count < options.minimum_points() {
            return Err(match options.output {
                PenType::Polyline | PenType::BSpline => {
                    "Pen Polyline needs at least two captured points; continue or Cancel"
                }
                PenType::Vertices => "Capture at least one point; continue or Cancel",
                _ => "Pen needs at least three captured points; continue or Cancel",
            }
            .into());
        }
        let world = self.scene.world_matrix(draft.target)?;
        let local = world.inverse();
        if options.wall != WallMode::Off && options.offset <= 0. {
            return Err("Pen Wall Mode needs an Offset above 0".into());
        }
        // Built in world space (UVs come from world X and Y), then mapped
        // into the item's local space.
        let mut mesh = pen_geometry(&draft.points, &options)?;
        if let Some(frame) = options
            .uv_view
            .filter(|_| options.make_uvs)
            .and_then(|view| crate::backdrop::topmost_backdrop_in(&self.scene, view))
            .and_then(|id| crate::backdrop::picture_frame(&self.scene, id))
        {
            backdrop_uvs(&mut mesh, &frame);
        }
        for v in &mut mesh.vertices {
            *v = local.transform_point3(*v);
        }
        // The polygon is built as entered, even through repeated points
        // (pen-2026-11-05 rules 1 and 6).
        mesh.validate()?;
        // Symmetry's creation path, Weld seam (SYM3-20..25): the mirror half
        // shares the points drawn on the plane. Strips along the seam and
        // polygons crossing it are made once.
        let (mesh, creation) = self.active_symmetry().welded(mesh)?;
        self.symmetry_note = creation.message().map(String::from);
        let elements = mesh.faces.len() as u32;
        let points_made = mesh.vertices.len() as u32;
        let mut candidate = self.scene.clone();
        let item = candidate
            .active_mut()
            .filter(|i| i.id == draft.target && i.visible)
            .ok_or("Pen target changed")?;
        let new_face =
            u32::try_from(item.mesh.faces.len()).map_err(|_| "Mesh face budget exceeded")?;
        let new_point =
            u32::try_from(item.mesh.vertices.len()).map_err(|_| "Mesh point budget exceeded")?;
        crate::append_geometry(&mut item.mesh, mesh)?;
        if options.select_new {
            // The new element is the selection; a points-only result selects
            // its points (provisional, PEN-Q-I01).
            candidate.selection.faces = (new_face..new_face + elements).collect();
            candidate.selection.vertices = if elements == 0 {
                (new_point..new_point + points_made).collect()
            } else {
                Vec::new()
            };
            candidate.selection.edges.clear();
        }
        candidate.validate()?;
        let before = std::mem::replace(&mut self.scene, candidate);
        self.session.pen_draft = None;
        let label = match options.output {
            _ if options.wall != WallMode::Off => "Pen Wall",
            _ if options.strip() => "Pen Quads",
            PenType::Vertices => "Pen Vertices",
            PenType::Polyline | PenType::BSpline => "Pen Polyline",
            _ => "Pen Polygon",
        };
        self.record(label.into(), before);
        Ok(())
    }
}
