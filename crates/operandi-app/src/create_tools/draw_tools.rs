//! Create > Draw curve tools: Curve, B-Spline and Bezier Curve
//! (draw-tools-2026-11-05, BEHAVIOR-DRAW-BUILD.md DRW-01).
//!
//! The tool is a session held by the app. The curve itself lives in the
//! mesh from its second point on: every click or drag ends in one core
//! `SetCurve` edit, so each is one undo step and the tool stays on. The
//! first point of a new curve waits in the tool until the second arrives.
//! Dropping the tool removes a curve with too few points (DRW-Q08).
use super::*;
use crate::form::Unit;
use crate::tool_form::{self as tf, ToolFields, ToolForm};
use operandi_core::CurveEdit;
use operandi_core::curves::{self, CurveKind, bezier_anchors, bezier_points};

/// Which Draw tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawKind {
    Curve,
    BSpline,
    Bezier,
    Sketch,
    Patch,
}
impl DrawKind {
    pub fn label(self) -> &'static str {
        match self {
            DrawKind::Curve => "Curve",
            DrawKind::BSpline => "B-Spline",
            DrawKind::Bezier => "Bezier Curve",
            DrawKind::Sketch => "Sketch",
            DrawKind::Patch => "Draw Patch",
        }
    }
    /// Curve, B-Spline and Bezier edit points; Sketch and Draw Patch don't.
    pub fn edits_points(self) -> bool {
        matches!(self, DrawKind::Curve | DrawKind::BSpline | DrawKind::Bezier)
    }
    fn matches(self, kind: CurveKind) -> bool {
        matches!(
            (self, kind),
            (DrawKind::Curve, CurveKind::Curve { .. })
                | (DrawKind::BSpline, CurveKind::BSpline)
                | (DrawKind::Bezier, CurveKind::Bezier)
        )
    }
}

/// What a click does to the shape (SPEC-DRAW-POINT-MODE DPM-01..05).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawMode {
    /// Click empty space: a new point after the current one.
    Add,
    /// Bezier only: click near the curve to split it there, keeping its
    /// shape (DPM-05).
    Insert,
    /// Clicks never add points; drag points and handles to tidy up.
    Edit,
    /// Click a point to remove it.
    Delete,
}

/// DPM-11: each point-drawing tool's Mode comes back as last used. The
/// Tube also keeps its current path point here (its path lives in the
/// shape session, which has no notion of one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointModes {
    pub curve: DrawMode,
    pub bspline: DrawMode,
    pub bezier: DrawMode,
    pub pen: DrawMode,
    pub tube: DrawMode,
    pub tube_current: Option<usize>,
    /// The path tools' Mode (path_tool_panel.rs).
    pub path: crate::path_tool_panel::PathMode,
}
impl Default for PointModes {
    fn default() -> Self {
        Self {
            curve: DrawMode::Add,
            bspline: DrawMode::Add,
            bezier: DrawMode::Add,
            pen: DrawMode::Add,
            tube: DrawMode::Add,
            tube_current: None,
            path: Default::default(),
        }
    }
}
impl PointModes {
    pub fn for_kind(&self, kind: DrawKind) -> DrawMode {
        match kind {
            DrawKind::BSpline => self.bspline,
            DrawKind::Bezier => self.bezier,
            _ => self.curve,
        }
    }
    pub fn remember(&mut self, kind: DrawKind, mode: DrawMode) {
        match kind {
            DrawKind::Curve => self.curve = mode,
            DrawKind::BSpline => self.bspline = mode,
            DrawKind::Bezier => self.bezier = mode,
            DrawKind::Sketch | DrawKind::Patch => {}
        }
    }
}

/// The Mode choices for a tool (DPM-01): Insert only on Bezier.
pub(crate) fn mode_options(bezier: bool) -> Vec<(DrawMode, &'static str)> {
    let mut v = vec![(DrawMode::Add, "Add")];
    if bezier {
        v.push((DrawMode::Insert, "Insert"));
    }
    v.extend([(DrawMode::Edit, "Edit"), (DrawMode::Delete, "Delete")]);
    v
}

/// DPM-11/UIC-04: the Mode row's tooltip names each mode's one-click
/// gesture, which works in every mode (DPM-06).
pub(crate) fn mode_tip(bezier: bool) -> String {
    let mut tip = String::from(
        "Add: click empty space adds a point after the current one; click a point to pick it, drag to move it",
    );
    if bezier {
        tip.push_str("\nInsert: click on the curve splits it there without changing its shape");
    }
    tip.push_str(
        "\nEdit: clicks never add points; drag points and handles\n\
         Delete: click a point to remove it (in any mode: middle-click; Shift+middle-click also removes every later point)",
    );
    tip
}

/// The Mode row for tools whose mode lives in the app rather than in
/// their panel's values (Pen, Tube): joined segments (UIC-89) with the
/// gestures in the tooltip (UIC-04). Changing it never ends the shape
/// (DPM-14).
pub(crate) fn app_mode_row<P>(
    get: fn(&PointModes) -> DrawMode,
    set: fn(&mut PointModes, DrawMode),
) -> tf::Item<P> {
    tf::Item::Custom(std::rc::Rc::new(move |app: &mut Operandi, ui: &mut Ui| {
        let options = mode_options(false);
        let names: Vec<&str> = options.iter().map(|(_, n)| *n).collect();
        let now = get(&app.point_modes);
        let mut index = options.iter().position(|(m, _)| *m == now).unwrap_or(0);
        let row = ui.scope(|ui| crate::form::rows::segments(ui, "Mode", &mut index, &names));
        row.response.on_hover_text(mode_tip(false));
        if row.inner
            && let Some((mode, _)) = options.get(index)
        {
            set(&mut app.point_modes, *mode);
        }
    }))
}

/// DPM-05: split span `span` of the Bezier anchors `e` (flattened
/// in, anchor, out) at `t` by de Casteljau, so the curve keeps its shape:
/// the neighbours' facing handles shorten and the new anchor's handles lie
/// along the curve. Returns the new anchor's index.
pub(crate) fn split_bezier(e: &mut Vec<DVec3>, span: usize, t: f64) -> usize {
    let n = e.len() / 3;
    let next = (span + 1) % n;
    let (p0, p1, p2, p3) = (
        e[span * 3 + 1],
        e[span * 3 + 2],
        e[next * 3],
        e[next * 3 + 1],
    );
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let q2 = p2.lerp(p3, t);
    let r0 = q0.lerp(q1, t);
    let r1 = q1.lerp(q2, t);
    let s = r0.lerp(r1, t);
    e[span * 3 + 2] = q0;
    e[next * 3] = q2;
    let at = span + 1;
    e.splice(at * 3..at * 3, [r0, s, r1]);
    at
}

/// Bezier Tangent Handle: what dragging one handle does to the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TangentMode {
    KeepTangent,
    UniformLength,
    Break,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawTool {
    pub kind: DrawKind,
    pub mode: DrawMode,
    pub item: Option<u64>,
    /// The curve polygon being edited, once it has two points.
    pub face: Option<u32>,
    /// A new curve's first point, until the second click.
    pub pending: Option<DVec3>,
    /// Current point (Curve/B-Spline) or anchor (Bezier).
    pub current: Option<usize>,
    pub closed: bool,
    /// CV-14: editing a curve that was selected when the tool came on
    /// (its line draws in the selection colour), not drawing a new one.
    pub reopened: bool,
    pub start_control: bool,
    pub end_control: bool,
    pub tangent: TangentMode,
    /// The points shown while a drag is under way (world space).
    pub draft: Option<Vec<DVec3>>,
    pub sketch: operandi_core::sketch::SketchOptions,
    /// Sketch: the stroke being drawn (world, on the plane).
    pub stroke: Vec<DVec3>,
    /// Sketch: the last stroke, rebuilt in place when Type changes.
    pub last_stroke: Option<LastStroke>,
    /// Draw Patch: boundary curves picked so far (polygon indices).
    pub picks: Vec<u32>,
    pub patch: PatchOptions,
    /// Draw Patch: the last patch, rebuilt in place when an option changes.
    pub last_patch: Option<LastPatch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LastPatch {
    pub picks: Vec<u32>,
    pub mark: u64,
    pub step_mark: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LastStroke {
    pub samples: Vec<DVec3>,
    pub spacing: f64,
    pub normal: DVec3,
    /// Recording mark after adding the stroke, and the checkpoint that
    /// must still be on top when rebuilding (Undo does not record).
    pub mark: u64,
    pub step_mark: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatchOptions {
    pub perpendicular: u32,
    pub parallel: u32,
    pub flip: bool,
    pub make_uvs: bool,
    /// PC-6 Type 1: spacing across, with Perpendicular.
    pub type1: operandi_core::patch::PatchSpacing,
    /// PC-6 Type 2: spacing along the first curve, with Parallel.
    pub type2: operandi_core::patch::PatchSpacing,
    /// PC-7: on, the patch is frozen into polygons; off (the default), it
    /// is one live patch polygon that follows its curves.
    pub freeze: bool,
}
impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            perpendicular: 10,
            parallel: 10,
            flip: false,
            make_uvs: false,
            type1: Default::default(),
            type2: Default::default(),
            freeze: false,
        }
    }
}

/// What a press grabbed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawGrab {
    /// Point (or Bezier anchor) `index`.
    Point(usize),
    /// Bezier handle: anchor, 0 in / 2 out.
    Handle(usize, usize),
    /// Bezier press-drag in empty space: the new anchor's handles.
    NewHandles(usize),
    /// Sketch: drawing a stroke.
    Stroke,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawDrag {
    pub grab: DrawGrab,
    /// Control points (Bezier: anchors flattened in, a, out) at the press.
    pub base: Vec<DVec3>,
    /// The grabbed point and the plane point under the press.
    pub origin: DVec3,
    pub press: DVec3,
    /// Ctrl: constrained to one world axis, chosen after a few pixels.
    pub constrained: bool,
    /// Ctrl+Shift: later points move too.
    pub chain: bool,
    pub axis: Option<DVec3>,
    pub shift: bool,
    /// A point added by this press: written on release even without a
    /// move, so the click and any drag are one undo step.
    pub new_point: bool,
}

const GRAB: f32 = 8.;

pub(crate) fn plane_axes(plane: u8) -> [DVec3; 2] {
    match plane {
        0 => [DVec3::X, DVec3::Z],
        1 => [DVec3::Y, DVec3::Z],
        _ => [DVec3::X, DVec3::Y],
    }
}

impl DrawTool {
    pub fn new(kind: DrawKind) -> Self {
        Self {
            kind,
            mode: DrawMode::Add,
            item: None,
            face: None,
            pending: None,
            current: None,
            closed: false,
            reopened: false,
            start_control: false,
            end_control: false,
            tangent: TangentMode::KeepTangent,
            draft: None,
            sketch: Default::default(),
            stroke: Vec::new(),
            last_stroke: None,
            picks: Vec::new(),
            patch: Default::default(),
            last_patch: None,
        }
    }
    pub fn curve_kind(&self) -> CurveKind {
        match self.kind {
            DrawKind::Curve => CurveKind::Curve {
                start_control: self.start_control,
                end_control: self.end_control,
            },
            DrawKind::BSpline => CurveKind::BSpline,
            DrawKind::Bezier => CurveKind::Bezier,
            DrawKind::Sketch | DrawKind::Patch => CurveKind::Line,
        }
    }
}

impl Operandi {
    /// Turn a Draw curve tool on, or off when it's already on. A selected
    /// curve of the same kind (Polygons mode) is resumed (DRW-01).
    pub(super) fn activate_draw(&mut self, kind: DrawKind) {
        if self.draw.as_ref().is_some_and(|d| d.kind == kind) {
            self.drop_draw();
            return;
        }
        if !self.confirm_fields() {
            return;
        }
        self.drop_draw();
        self.utility = None;
        if self.editor.session.has_active_task() {
            self.run(Command::Drop);
            if self.editor.session.has_active_task() {
                return;
            }
        }
        let mut tool = DrawTool::new(kind);
        // DPM-11: Mode comes back as last used.
        tool.mode = self.point_modes.for_kind(kind);
        // CV-20/21/22/25: the first selected curve of this kind opens in
        // Edit mode with its first point current; otherwise a new curve
        // starts in the remembered mode (Add on first use).
        let s = &self.editor.scene.selection;
        let reopen = (s.mode == Mode::Polygon)
            .then(|| self.editor.scene.active())
            .flatten()
            .and_then(|item| {
                s.faces.iter().find_map(|&f| {
                    match item.mesh.attributes.representations.get(f as usize) {
                        Some(&operandi_core::PolygonRepresentation::Polyline { closed, curve })
                            if kind.matches(curve) =>
                        {
                            Some((item, f, closed, curve))
                        }
                        _ => None,
                    }
                })
            });
        if let Some((item, f, closed, curve)) = reopen {
            tool.mode = DrawMode::Edit;
            tool.item = Some(item.id);
            tool.face = Some(f);
            tool.closed = closed;
            tool.reopened = true;
            if let CurveKind::Curve {
                start_control,
                end_control,
            } = curve
            {
                tool.start_control = start_control;
                tool.end_control = end_control;
            }
            tool.current = Some(0);
        }
        self.draw = Some(tool);
        self.tool_properties_open = true;
        self.status = match kind {
            DrawKind::Sketch => "Sketch · drag in the viewport to draw a stroke".into(),
            DrawKind::Patch => {
                "Draw Patch · click the handles of three or four curves that share their ends, in order around, then the first again".into()
            }
            _ => format!(
                "{} · click to add points, drag to move · right-click the first point closes · middle-click deletes · Shift-click starts a new curve",
                kind.label()
            ),
        };
    }

    /// End the Draw tool; a curve with too few points goes (DRW-Q08).
    pub(super) fn drop_draw(&mut self) {
        let Some(tool) = self.draw.take() else {
            return;
        };
        if let Some(face) = self.valid_face(&tool) {
            let n = self.curve_points(&tool).len();
            if n < tool.curve_kind().minimum_points() {
                self.run(Command::SetCurve(Box::new(CurveEdit {
                    item: tool.item,
                    face: Some(face),
                    points: Vec::new(),
                    closed: false,
                    kind: tool.curve_kind(),
                    label: format!("Remove short {}", tool.kind.label()),
                })));
            }
        }
        self.status = format!("{} off", tool.kind.label());
    }

    fn valid_face(&self, tool: &DrawTool) -> Option<u32> {
        let face = tool.face?;
        let item = self
            .editor
            .scene
            .items
            .iter()
            .find(|i| Some(i.id) == tool.item)?;
        match item.mesh.attributes.representations.get(face as usize) {
            Some(operandi_core::PolygonRepresentation::Polyline { curve, .. })
                if tool.kind.matches(*curve) =>
            {
                Some(face)
            }
            _ => None,
        }
    }

    /// Called once a frame: forget a curve that Undo took away, and turn
    /// the tool off when a modeling tool comes on.
    pub(super) fn keep_draw_valid(&mut self) {
        if self.draw.is_some() && (self.editor.session.has_active_task() || self.utility.is_some())
        {
            self.draw = None;
            return;
        }
        let Some(tool) = self.draw.clone() else {
            return;
        };
        let face = self.valid_face(&tool);
        let n = self.curve_points_len(&tool, face);
        let closed = face.and_then(|f| {
            let item = self
                .editor
                .scene
                .items
                .iter()
                .find(|i| Some(i.id) == tool.item)?;
            match item.mesh.attributes.representations.get(f as usize) {
                Some(operandi_core::PolygonRepresentation::Polyline { closed, .. }) => {
                    Some(*closed)
                }
                _ => None,
            }
        });
        if let Some(t) = self.draw.as_mut() {
            if face.is_none() {
                t.face = None;
            }
            if let Some(c) = closed {
                t.closed = c;
            }
            // While a press is held the new point is only in the draft.
            if let Some(c) = t.current
                && c >= n
                && t.draft.is_none()
            {
                t.current = n.checked_sub(1);
            }
        }
    }

    fn curve_points_len(&self, tool: &DrawTool, face: Option<u32>) -> usize {
        let mut t = tool.clone();
        t.face = face;
        let points = self.curve_points(&t);
        if tool.kind == DrawKind::Bezier {
            bezier_anchors(&points, t.closed).len()
        } else {
            points.len()
        }
    }

    /// The tool's stored control points in world space.
    pub(super) fn curve_points(&self, tool: &DrawTool) -> Vec<DVec3> {
        if let Some(face) = self.valid_face(tool)
            && let Some(item) = self
                .editor
                .scene
                .items
                .iter()
                .find(|i| Some(i.id) == tool.item)
        {
            return item.mesh.faces[face as usize]
                .vertices
                .iter()
                .map(|&v| {
                    self.editor
                        .scene
                        .world_point(item.id, item.mesh.vertices[v as usize])
                })
                .collect();
        }
        match (tool.kind, tool.pending) {
            (DrawKind::Bezier, Some(p)) => vec![p],
            (_, Some(p)) => vec![p],
            _ => Vec::new(),
        }
    }

    /// Editable points: Curve/B-Spline points, or Bezier anchors flattened
    /// as (in, anchor, out).
    fn editable(&self, tool: &DrawTool) -> Vec<DVec3> {
        let points = self.curve_points(tool);
        if tool.kind == DrawKind::Bezier {
            if points.len() == 1 {
                return vec![points[0]; 3];
            }
            bezier_anchors(&points, tool.closed)
                .into_iter()
                .flatten()
                .collect()
        } else {
            points
        }
    }

    fn stored(tool: &DrawTool, editable: &[DVec3]) -> Vec<DVec3> {
        if tool.kind == DrawKind::Bezier {
            let anchors: Vec<[DVec3; 3]> = editable
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| [c[0], c[1], c[2]])
                .collect();
            bezier_points(&anchors, tool.closed)
        } else {
            editable.to_vec()
        }
    }

    /// Write `editable` points to the mesh (one undo step) or, with one
    /// point, hold it as pending.
    fn commit_curve(&mut self, editable: Vec<DVec3>, label: &str) {
        let Some(tool) = self.draw.clone() else {
            return;
        };
        let per = if tool.kind == DrawKind::Bezier { 3 } else { 1 };
        let count = editable.len() / per;
        let face = self.valid_face(&tool);
        if count < 2 {
            if let Some(face) = face {
                self.run(Command::SetCurve(Box::new(CurveEdit {
                    item: tool.item,
                    face: Some(face),
                    points: Vec::new(),
                    closed: false,
                    kind: tool.curve_kind(),
                    label: label.into(),
                })));
            }
            if let Some(t) = self.draw.as_mut() {
                t.face = None;
                t.pending = editable.get(per / 2).copied();
                t.current = t.pending.map(|_| 0);
            }
            return;
        }
        let points = Self::stored(&tool, &editable);
        let ok = self.run_ok(Command::SetCurve(Box::new(CurveEdit {
            item: tool.item,
            face,
            points,
            closed: tool.closed,
            kind: tool.curve_kind(),
            label: label.into(),
        })));
        if ok
            && let Some(item) = self.editor.scene.active()
            && let Some(t) = self.draw.as_mut()
        {
            t.item = Some(item.id);
            t.face = Some(item.mesh.faces.len() as u32 - 1);
            t.pending = None;
        }
    }

    /// Screen hits: (grab, distance), nearest first.
    fn draw_hit(&self, tool: &DrawTool, pos: Pos2, r: Rect) -> Option<DrawGrab> {
        let e = self.editable(tool);
        let mut best: Option<(f32, DrawGrab)> = None;
        let mut consider = |p: DVec3, grab: DrawGrab| {
            if let Some((q, _)) = self.camera.project(p, r) {
                let d = q.distance(pos);
                if d <= GRAB && best.is_none_or(|(b, _)| d < b) {
                    best = Some((d, grab));
                }
            }
        };
        if tool.kind == DrawKind::Bezier {
            for (i, c) in e.as_chunks::<3>().0.iter().enumerate() {
                consider(c[1], DrawGrab::Point(i));
                if (c[0] - c[1]).length() > 1e-9 {
                    consider(c[0], DrawGrab::Handle(i, 0));
                }
                if (c[2] - c[1]).length() > 1e-9 {
                    consider(c[2], DrawGrab::Handle(i, 2));
                }
            }
        } else {
            for (i, p) in e.iter().enumerate() {
                consider(*p, DrawGrab::Point(i));
            }
        }
        best.map(|(_, g)| g)
    }

    /// The nearest visible mesh point within grabbing distance, other than
    /// the curve's own points: new and dragged curve points snap to it, so
    /// curves can share ends (Draw Patch needs that). Points hidden behind
    /// the surface don't count, and meshes of any size do.
    fn draw_snap(&self, pos: Pos2, r: Rect, skip: &[DVec3]) -> Option<DVec3> {
        let world = self.snap_service.world(&self.editor.scene);
        let own = |owner: &crate::snap_service::Owner| match *owner {
            crate::snap_service::Owner::Point { item, v } => world
                .point_of(item, v)
                .is_some_and(|w| skip.iter().any(|s| (*s - w).length() < 1e-12)),
            _ => false,
        };
        self.snap_scene_point(pos, crate::snap_service::Role::JoinPoint, r, &own)
            .map(|h| h.hit.point)
    }

    /// DPM-05: the Bezier span and parameter nearest `pos` on screen, when
    /// the curve passes within the Insert reach of it.
    fn bezier_span_hit(
        &self,
        tool: &DrawTool,
        e: &[DVec3],
        pos: Pos2,
        r: Rect,
    ) -> Option<(usize, f64)> {
        /// Screen pixels: how near the curve an Insert click must be.
        const REACH: f32 = 12.;
        const STEPS: usize = 64;
        let n = e.len() / 3;
        if n < 2 {
            return None;
        }
        let spans = if tool.closed { n } else { n - 1 };
        let mut best: Option<(f32, usize, f64)> = None;
        for span in 0..spans {
            let next = (span + 1) % n;
            let c = [
                e[span * 3 + 1],
                e[span * 3 + 2],
                e[next * 3],
                e[next * 3 + 1],
            ];
            for s in 1..STEPS {
                let t = s as f64 / STEPS as f64;
                let u = 1. - t;
                let p = c[0] * (u * u * u)
                    + c[1] * (3. * u * u * t)
                    + c[2] * (3. * u * t * t)
                    + c[3] * (t * t * t);
                if let Some((q, _)) = self.camera.project(p, r) {
                    let d = q.distance(pos);
                    if d <= REACH && best.is_none_or(|(b, ..)| d < b) {
                        best = Some((d, span, t));
                    }
                }
            }
        }
        best.map(|(_, span, t)| (span, t))
    }

    fn plane_hit(&self, pos: Pos2, r: Rect, through: DVec3) -> Option<DVec3> {
        self.work_point(pos, r, self.actual_plane(), through)
    }

    /// A press while a Draw tool is on (`button`: 0 left, 1 right, 2 middle).
    pub(super) fn draw_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        modifiers: Modifiers,
        button: u8,
    ) -> Option<input::GestureKind> {
        let tool = self.draw.clone()?;
        match tool.kind {
            DrawKind::Sketch => return self.sketch_press(pos, r, button),
            DrawKind::Patch => return self.patch_press(pos, r, button),
            _ => {}
        }
        let hit = self.draw_hit(&tool, pos, r);
        let per = if tool.kind == DrawKind::Bezier { 3 } else { 1 };
        let mut e = self.editable(&tool);
        let label = format!("{} Point", tool.kind.label());
        match button {
            // Right-click the first point closes (DRW-05); repeating
            // doesn't reopen.
            1 => {
                if hit == Some(DrawGrab::Point(0)) && !tool.closed && e.len() / per >= 2 {
                    if let Some(t) = self.draw.as_mut() {
                        t.closed = true;
                    }
                    // Bezier: closing adds the last anchor's out and the
                    // first anchor's in, which `stored` writes.
                    self.commit_curve(e, &format!("Close {}", tool.kind.label()));
                }
                return Some(input::GestureKind::Blocked);
            }
            // Middle-click deletes the point; Shift deletes it and every
            // later point.
            2 => {
                if let Some(DrawGrab::Point(i)) = hit {
                    if modifiers.shift {
                        e.truncate(i * per);
                    } else {
                        e.drain(i * per..(i + 1) * per);
                    }
                    if let Some(t) = self.draw.as_mut() {
                        t.current = (e.len() / per).checked_sub(1).map(|c| c.min(i));
                    }
                    self.commit_curve(e, &format!("Delete {}", label));
                }
                return Some(input::GestureKind::Blocked);
            }
            _ => {}
        }
        if let Some(grab) = hit {
            // DPM-04: a click removes the point; the shape re-forms through
            // the rest and stays open or closed. A Bezier anchor goes with
            // its two handles. Clicking a handle end does nothing.
            if tool.mode == DrawMode::Delete {
                if let DrawGrab::Point(i) = grab {
                    e.drain(i * per..(i + 1) * per);
                    if let Some(t) = self.draw.as_mut() {
                        t.current = (e.len() / per).checked_sub(1).map(|c| c.min(i));
                    }
                    self.commit_curve(e, &format!("Delete {}", label));
                }
                return Some(input::GestureKind::Blocked);
            }
            if let (DrawGrab::Point(i), Some(t)) = (grab, self.draw.as_mut()) {
                t.current = Some(i);
            }
            let origin = match grab {
                DrawGrab::Point(i) => e[i * per + per / 2],
                DrawGrab::Handle(i, s) => e[i * 3 + s],
                DrawGrab::NewHandles(i) => e[i * 3 + 1],
                DrawGrab::Stroke => return Some(input::GestureKind::Blocked),
            };
            let press = self.plane_hit(pos, r, origin).unwrap_or(origin);
            // Snap targets are gathered afresh for each drag.
            self.snap_session = Default::default();
            return Some(input::GestureKind::Draw(DrawDrag {
                grab,
                base: e,
                origin,
                press,
                constrained: modifiers.ctrl || modifiers.mac_cmd,
                chain: (modifiers.ctrl || modifiers.mac_cmd) && modifiers.shift,
                axis: None,
                shift: modifiers.shift,
                new_point: false,
            }));
        }
        if tool.mode == DrawMode::Insert && tool.kind == DrawKind::Bezier {
            if let Some((span, t)) = self.bezier_span_hit(&tool, &e, pos, r) {
                let at = split_bezier(&mut e, span, t);
                if let Some(d) = self.draw.as_mut() {
                    d.current = Some(at);
                }
                self.commit_curve(e, "Insert Bezier Point");
            }
            return Some(input::GestureKind::Blocked);
        }
        // DPM-03: Edit (and Delete) clicks in empty space change nothing.
        if tool.mode != DrawMode::Add {
            return Some(input::GestureKind::Blocked);
        }
        // Shift-click in empty space starts a separate curve (adds nothing).
        if modifiers.shift {
            if let Some(t) = self.draw.as_mut() {
                t.face = None;
                t.pending = None;
                t.current = None;
                t.reopened = false;
            }
            self.status = format!(
                "{}: new curve · click to place its first point",
                tool.kind.label()
            );
            return Some(input::GestureKind::Blocked);
        }
        // A new point follows the current one; in a view without
        // perspective it keeps that point's depth (placement_center).
        let from = tool.current.and_then(|c| e.get(c * per + per / 2).copied());
        let center = self.placement_center(from);
        // DSN-07: a placed point goes through snapping as a dragged one
        // does; with snapping off it still joins a mesh point under the
        // pointer.
        let placed = if self.snap_targets().any() {
            self.snap_session = Default::default();
            self.plane_hit(pos, r, center)
                .map(|p| self.snap_draw_point(p, &[], &[], None, r).unwrap_or(p))
        } else {
            None
        };
        let Some(p) = placed
            .or_else(|| self.draw_snap(pos, r, &e))
            .or_else(|| self.plane_hit(pos, r, center))
        else {
            self.status = "View is parallel to the construction plane".into();
            return Some(input::GestureKind::Blocked);
        };
        // DRW-04: a new point goes right after the current one.
        let count = e.len() / per;
        let at = tool.current.map_or(count, |c| (c + 1).min(count));
        let fresh: Vec<DVec3> = vec![p; per];
        e.splice(at * per..at * per, fresh);
        if let Some(t) = self.draw.as_mut() {
            t.current = Some(at);
            t.draft = Some(e.clone());
        }
        let base = e;
        // Bezier: dragging on from the press pulls out the new anchor's
        // handles; others: the new point follows the drag.
        Some(input::GestureKind::Draw(DrawDrag {
            grab: if tool.kind == DrawKind::Bezier {
                DrawGrab::NewHandles(at)
            } else {
                DrawGrab::Point(at)
            },
            base,
            origin: p,
            press: p,
            constrained: false,
            chain: false,
            axis: None,
            shift: false,
            new_point: true,
        }))
    }

    fn dragged(&mut self, d: &mut DrawDrag, start: Pos2, pos: Pos2, r: Rect) -> Vec<DVec3> {
        let tool = match &self.draw {
            Some(t) => t.clone(),
            None => return d.base.clone(),
        };
        let here = self.plane_hit(pos, r, d.origin).unwrap_or(d.press);
        let mut delta = here - d.press;
        if d.constrained {
            if d.axis.is_none() && start.distance(pos) >= 4. {
                let [a, b] = plane_axes(self.actual_plane());
                d.axis = Some(if delta.dot(a).abs() >= delta.dot(b).abs() {
                    a
                } else {
                    b
                });
            }
            delta = d.axis.map_or(DVec3::ZERO, |a| a * delta.dot(a));
        }
        let per = if tool.kind == DrawKind::Bezier { 3 } else { 1 };
        // DSN-08: the grabbed point (or handle end) runs the active snap
        // targets on every move, so the draft shows where it will land.
        // Everything moving with it is left out (DSN-10); Ctrl keeps the
        // snap on the constrained line (DSN-12).
        let (grabbed, moving): (Option<DVec3>, Vec<DVec3>) = match d.grab {
            DrawGrab::Stroke => (None, Vec::new()),
            DrawGrab::Point(i) => {
                let end = if d.chain { d.base.len() / per } else { i + 1 };
                (
                    Some(d.base[i * per + per / 2]),
                    d.base[i * per..end * per].to_vec(),
                )
            }
            // The anchor and both its handles move or pivot together.
            DrawGrab::Handle(i, s) => (Some(d.base[i * 3 + s]), d.base[i * 3..i * 3 + 3].to_vec()),
            DrawGrab::NewHandles(i) => (Some(d.base[i * 3 + 1]), d.base[i * 3..i * 3 + 3].to_vec()),
        };
        let mut pulled = here - d.press;
        self.note_snap_drag();
        if let Some(from) = grabbed {
            let line = d
                .constrained
                .then_some(d.axis)
                .flatten()
                .map(|axis| (from, axis));
            // New handles: the out end follows the pointer from the anchor.
            let wanted = from
                + if matches!(d.grab, DrawGrab::NewHandles(_)) {
                    pulled
                } else {
                    delta
                };
            if self.snap_targets().any() {
                if d.constrained && line.is_none() {
                    // Ctrl before the axis is chosen: nothing moves yet.
                    self.snap_marker = None;
                } else if let Some(snapped) = self.snap_draw_point(wanted, &moving, &[], line, r) {
                    if matches!(d.grab, DrawGrab::NewHandles(_)) {
                        pulled = snapped - from;
                    } else {
                        delta = snapped - from;
                    }
                }
            } else if !d.constrained
                && matches!(d.grab, DrawGrab::Point(_))
                && let Some(snapped) = self.draw_snap(pos, r, &d.base)
            {
                // Snapping off: points still join other meshes' points
                // under the pointer, so curves can share ends (Draw Patch).
                delta = snapped - d.origin;
            }
        }
        let mut e = d.base.clone();
        match d.grab {
            DrawGrab::Stroke => {}
            DrawGrab::Point(i) => {
                let end = if d.chain { e.len() / per } else { i + 1 };
                for j in i..end {
                    for k in 0..per {
                        e[j * per + k] += delta;
                    }
                }
            }
            DrawGrab::NewHandles(i) => {
                let a = e[i * 3 + 1];
                let out = a + pulled;
                e[i * 3 + 2] = out;
                e[i * 3] = a * 2. - out;
            }
            DrawGrab::Handle(i, s) => {
                let a = e[i * 3 + 1];
                let moved = e[i * 3 + s] + delta;
                e[i * 3 + s] = moved;
                let other = 2 - s;
                let mode = if d.shift {
                    TangentMode::KeepTangent
                } else {
                    tool.tangent
                };
                let dir = (a - moved).normalize_or_zero();
                match mode {
                    TangentMode::Break => {}
                    TangentMode::KeepTangent => {
                        let len = (e[i * 3 + other] - a).length();
                        e[i * 3 + other] = a + dir * len;
                    }
                    TangentMode::UniformLength => {
                        e[i * 3 + other] = a * 2. - moved;
                    }
                }
            }
        }
        e
    }

    pub(super) fn draw_drag(&mut self, d: &mut DrawDrag, start: Pos2, pos: Pos2, r: Rect) {
        if d.grab == DrawGrab::Stroke {
            self.sketch_move(d, pos, r);
            return;
        }
        let e = self.dragged(d, start, pos, r);
        if let Some(t) = self.draw.as_mut() {
            t.draft = Some(e);
        }
    }

    pub(super) fn draw_release(&mut self, d: &mut DrawDrag, start: Pos2, pos: Pos2, r: Rect) {
        if d.grab == DrawGrab::Stroke {
            self.sketch_move(d, pos, r);
            self.sketch_finish(start, r);
            return;
        }
        let e = self.dragged(d, start, pos, r);
        if let Some(t) = self.draw.as_mut() {
            t.draft = None;
        }
        let label = format!(
            "{} Point",
            self.draw.as_ref().map_or("", |t| t.kind.label())
        );
        if d.new_point {
            let e = if start.distance(pos) < 2. {
                d.base.clone()
            } else {
                e
            };
            self.commit_curve(e, &label);
            return;
        }
        // A click that didn't move changes nothing more.
        if start.distance(pos) < 2. || e == d.base {
            return;
        }
        let label = match d.grab {
            DrawGrab::NewHandles(_) | DrawGrab::Handle(..) => "Bezier Handle".to_string(),
            DrawGrab::Point(_) | DrawGrab::Stroke => format!(
                "Move {} Point",
                self.draw.as_ref().map_or("", |t| t.kind.label())
            ),
        };
        self.commit_curve(e, &label);
    }

    // ------------------------------------------------------------- panel

    // ------------------------------------------------------------- draw

    /// What a press at `hover` would grab, for the hover highlight. None
    /// while a drag is under way: the dragged point is the current one.
    pub(super) fn draw_hover(
        &self,
        tool: &DrawTool,
        hover: Option<Pos2>,
        r: Rect,
    ) -> Option<DrawGrab> {
        if tool.draft.is_some() || !tool.kind.edits_points() {
            return None;
        }
        self.draw_hit(tool, hover?, r)
    }

    /// Fill and outline of each point (Bezier: anchor) of the curve being
    /// drawn. The current point and the one under the pointer are Active
    /// (yellow); the rest are Handle (blue). Only the current one is filled.
    pub(super) fn curve_point_looks(
        &self,
        hover: Option<Pos2>,
        r: Rect,
    ) -> Vec<(Color32, Color32)> {
        let Some(tool) = &self.draw else {
            return Vec::new();
        };
        let per = if tool.kind == DrawKind::Bezier { 3 } else { 1 };
        let count = tool
            .draft
            .as_ref()
            .map_or_else(|| self.editable(tool).len(), Vec::len)
            / per;
        let hovered = self.draw_hover(tool, hover, r);
        let (active, idle) = (theme::viewport::handle_active(), theme::viewport::handle());
        (0..count)
            .map(|i| {
                // DPM-12: in Delete mode the point a click would remove
                // shows in the warning colour.
                if tool.mode == DrawMode::Delete && hovered == Some(DrawGrab::Point(i)) {
                    (Color32::TRANSPARENT, theme::error())
                } else if tool.current == Some(i) {
                    (active, active)
                } else if hovered == Some(DrawGrab::Point(i)) {
                    (Color32::TRANSPARENT, active)
                } else {
                    (Color32::TRANSPARENT, idle)
                }
            })
            .collect()
    }

    /// CV-14: the curve line is grey while a new curve is drawn, and the
    /// selection colour while a reopened (selected) curve is edited.
    pub(super) fn curve_line_colour(&self) -> Color32 {
        if self.draw.as_ref().is_some_and(|t| t.reopened) {
            theme::viewport::selection()
        } else {
            theme::viewport::label()
        }
    }

    pub(super) fn draw_curve_tool(&self, painter: &egui::Painter, r: Rect, hover: Option<Pos2>) {
        let Some(tool) = &self.draw else {
            return;
        };
        if !tool.kind.edits_points() {
            self.draw_sketch_and_patch(tool, painter, r);
            return;
        }
        let e = tool.draft.clone().unwrap_or_else(|| self.editable(tool));
        if e.is_empty() {
            return;
        }
        let per = if tool.kind == DrawKind::Bezier { 3 } else { 1 };
        let stored = Self::stored(tool, &e);
        let accent = theme::viewport::handle_active();
        let cage = theme::viewport::label().gamma_multiply(0.5);
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        let samples = curves::sample(
            &stored,
            tool.curve_kind(),
            tool.closed,
            curves::FREEZE_STEPS,
        );
        let mut line: Vec<Pos2> = samples.iter().filter_map(|p| project(*p)).collect();
        if tool.closed
            && let Some(&first) = line.first()
        {
            line.push(first);
        }
        painter.add(egui::Shape::line(
            line,
            Stroke::new(2f32, self.curve_line_colour()),
        ));
        // Control cage, thin.
        if tool.kind != DrawKind::Curve || tool.start_control || tool.end_control {
            let cage_points: Vec<Pos2> = stored.iter().filter_map(|p| project(*p)).collect();
            for pair in cage_points.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(1f32, cage));
            }
        }
        let looks = self.curve_point_looks(hover, r);
        let hovered = self.draw_hover(tool, hover, r);
        let idle = theme::viewport::handle();
        for (i, chunk) in e.chunks_exact(per).enumerate() {
            let anchor = chunk[per / 2];
            let Some(q) = project(anchor) else { continue };
            let (fill, outline) = looks
                .get(i)
                .copied()
                .unwrap_or((Color32::TRANSPARENT, idle));
            painter.rect(
                Rect::from_center_size(q, vec2(7., 7.)),
                0.,
                fill,
                Stroke::new(1.5f32, outline),
                egui::StrokeKind::Middle,
            );
            if per == 3 {
                for (side, h) in [(0, chunk[0]), (2, chunk[2])] {
                    if (h - anchor).length() > 1e-9
                        && let Some(hq) = project(h)
                    {
                        painter.line_segment([q, hq], Stroke::new(1f32, cage));
                        let lit = hovered == Some(DrawGrab::Handle(i, side));
                        painter.circle_filled(hq, 3.5, if lit { accent } else { idle });
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- Sketch

impl Operandi {
    fn plane_normal(&self) -> DVec3 {
        match self.actual_plane() {
            0 => DVec3::Y,
            1 => DVec3::X,
            _ => DVec3::Z,
        }
    }

    fn sketch_press(&mut self, pos: Pos2, r: Rect, button: u8) -> Option<input::GestureKind> {
        if button != 0 {
            return Some(input::GestureKind::Blocked);
        }
        let center = self.workplane_center();
        let Some(p) = self.plane_hit(pos, r, center) else {
            self.status = "View is parallel to the construction plane".into();
            return Some(input::GestureKind::Blocked);
        };
        if let Some(t) = self.draw.as_mut() {
            t.stroke = vec![p];
        }
        Some(input::GestureKind::Draw(DrawDrag {
            grab: DrawGrab::Stroke,
            base: Vec::new(),
            origin: p,
            press: p,
            constrained: false,
            chain: false,
            axis: None,
            shift: false,
            new_point: false,
        }))
    }

    fn sketch_move(&mut self, d: &DrawDrag, pos: Pos2, r: Rect) {
        let Some(p) = self.plane_hit(pos, r, d.origin) else {
            return;
        };
        if let Some(t) = self.draw.as_mut()
            && t.stroke.last().is_none_or(|q| (*q - p).length() > 1e-9)
            && t.stroke.len() < 100_000
        {
            t.stroke.push(p);
        }
    }

    /// Spacing in world units: Precision pixels at the stroke's start
    /// (SKT-Q01 choice).
    fn sketch_spacing(&self, start: Pos2, r: Rect, precision: f64, through: DVec3) -> f64 {
        let a = self.plane_hit(start, r, through);
        let b = self.plane_hit(start + vec2(precision as f32, 0.), r, through);
        match (a, b) {
            (Some(a), Some(b)) if (b - a).length() > 1e-12 => (b - a).length(),
            _ => 0.1,
        }
    }

    fn sketch_finish(&mut self, start: Pos2, r: Rect) {
        let Some(tool) = self.draw.clone() else {
            return;
        };
        if let Some(t) = self.draw.as_mut() {
            t.stroke.clear();
        }
        let Some(&first) = tool.stroke.first() else {
            return;
        };
        let spacing = self.sketch_spacing(start, r, tool.sketch.precision, first);
        let samples = operandi_core::sketch::resample(&tool.stroke, spacing);
        if samples.len() < 2 {
            self.status = "Sketch: drag further to draw a stroke".into();
            return;
        }
        let normal = self.plane_normal();
        self.add_sketch(samples, spacing, normal, &tool.sketch);
    }

    fn add_sketch(
        &mut self,
        samples: Vec<DVec3>,
        spacing: f64,
        normal: DVec3,
        options: &operandi_core::sketch::SketchOptions,
    ) {
        match operandi_core::sketch::sketch_geometry(&samples, spacing, options, normal) {
            Ok(mesh) => {
                if self.run_ok(Command::AddGeometry(Box::new(operandi_core::AddGeometry {
                    item: None,
                    mesh,
                    label: "Sketch".into(),
                    weld: Vec::new(),
                    join: Vec::new(),
                }))) && let Some(t) = self.draw.as_mut()
                {
                    t.last_stroke = Some(LastStroke {
                        samples,
                        spacing,
                        normal,
                        mark: self.editor.history_mark(),
                        step_mark: self.editor.history.undo.last().map(|s| s.mark),
                    });
                }
            }
            Err(e) => self.status = e,
        }
    }

    /// Changing Type (or another option) rebuilds the last stroke in
    /// place while nothing else has been done since.
    fn rebuild_sketch(&mut self) {
        let Some(tool) = self.draw.clone() else {
            return;
        };
        let Some(last) = tool.last_stroke.clone() else {
            return;
        };
        if self.editor.recorded_since(last.mark)
            || self
                .editor
                .history
                .undo
                .last()
                .is_none_or(|s| Some(s.mark) != last.step_mark || s.label != "Sketch")
        {
            if let Some(t) = self.draw.as_mut() {
                t.last_stroke = None;
            }
            return;
        }
        self.run(Command::Undo);
        self.editor.history.redo.clear();
        self.add_sketch(last.samples, last.spacing, last.normal, &tool.sketch);
    }

    #[cfg(test)]
    pub(super) fn rebuild_sketch_for_test(&mut self) {
        self.rebuild_sketch();
    }

    fn draw_sketch_and_patch(&self, tool: &DrawTool, painter: &egui::Painter, r: Rect) {
        let accent = theme::viewport::handle_active();
        let project = |p: DVec3| self.camera.project(p, r).map(|(q, _)| q);
        if tool.kind == DrawKind::Sketch {
            let line: Vec<Pos2> = tool.stroke.iter().filter_map(|p| project(*p)).collect();
            if line.len() > 1 {
                painter.add(egui::Shape::line(line, Stroke::new(2f32, accent)));
            }
            return;
        }
        self.draw_patch_handles(tool, painter, r);
    }
}

// --------------------------------------------------------- Draw Patch

/// Ends closer than this count as shared (PCH choice, kept for G7 PC-2: the
/// reference needs merged points; Operandi's drawn curves have their own).
const END_TOLERANCE: f64 = 1e-4;

/// One open curve of the active mesh that Draw Patch can pick.
#[derive(Clone)]
struct PatchCurve {
    face: u32,
    /// Sampled world points, first to last.
    samples: Vec<DVec3>,
    /// The handle at the curve's middle (PC-2).
    handle: DVec3,
    /// The mesh points at the curve's two ends (local indices).
    ends: [u32; 2],
}

impl Operandi {
    /// Open line elements of the active mesh.
    fn patch_candidates(&self) -> Vec<PatchCurve> {
        let Some(item) = self.editor.scene.active().filter(|i| i.is_mesh()) else {
            return Vec::new();
        };
        let m = &item.mesh;
        (0..m.faces.len())
            .filter_map(|f| match m.attributes.representations.get(f) {
                Some(operandi_core::PolygonRepresentation::Polyline {
                    closed: false,
                    curve,
                }) => {
                    let ids = &m.faces[f].vertices;
                    let control: Vec<DVec3> = ids
                        .iter()
                        .map(|&v| {
                            self.editor
                                .scene
                                .world_point(item.id, m.vertices[v as usize])
                        })
                        .collect();
                    let samples = curves::sample(&control, *curve, false, curves::FREEZE_STEPS);
                    if samples.len() < 2 {
                        return None;
                    }
                    let middle = operandi_core::patch::by_length(&samples, 3)[1];
                    // The mesh point sitting at each end of the sampled curve.
                    let at = |p: DVec3| -> u32 {
                        let k = (0..control.len())
                            .min_by(|&a, &b| {
                                (control[a] - p)
                                    .length()
                                    .total_cmp(&(control[b] - p).length())
                            })
                            .unwrap_or(0);
                        ids[k]
                    };
                    let ends = [at(samples[0]), at(*samples.last().unwrap())];
                    Some(PatchCurve {
                        face: f as u32,
                        samples,
                        handle: middle,
                        ends,
                    })
                }
                _ => None,
            })
            .collect()
    }

    fn patch_press(&mut self, pos: Pos2, r: Rect, button: u8) -> Option<input::GestureKind> {
        if button != 0 {
            return Some(input::GestureKind::Blocked);
        }
        let candidates = self.patch_candidates();
        let hit = candidates
            .iter()
            .filter_map(|c| {
                let (q, _) = self.camera.project(c.handle, r)?;
                (q.distance(pos) <= GRAB * 1.5).then_some((c.face, q.distance(pos)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(f, _)| f);
        let Some(face) = hit else {
            self.status = "Draw Patch: click a curve's middle handle".into();
            return Some(input::GestureKind::Blocked);
        };
        let tool = self.draw.clone()?;
        match pick_patch_curve(&candidates, &tool.picks, face) {
            Ok(PatchPick::Picked(picks)) => {
                let closable =
                    picks.len() >= 3 && touches(&candidates, picks[picks.len() - 1], picks[0]);
                self.status = if closable {
                    format!(
                        "Draw Patch: {} curves picked · click the first curve again to close the loop",
                        picks.len()
                    )
                } else {
                    format!("Draw Patch: {} curves picked", picks.len())
                };
                if let Some(t) = self.draw.as_mut() {
                    t.picks = picks;
                    t.last_patch = None;
                }
            }
            Ok(PatchPick::Close) => {
                let picks = tool.picks.clone();
                if self.add_patch(&candidates, &picks, &tool.patch)
                    && let Some(t) = self.draw.as_mut()
                {
                    t.picks.clear();
                    t.last_patch = Some(LastPatch {
                        picks,
                        mark: self.editor.history_mark(),
                        step_mark: self.editor.history.undo.last().map(|s| s.mark),
                    });
                }
            }
            Err(why) => self.status = format!("Draw Patch: {why}"),
        }
        Some(input::GestureKind::Blocked)
    }

    /// Builds the patch on the picked loop as one undo step (PC-5).
    fn add_patch(&mut self, candidates: &[PatchCurve], picks: &[u32], o: &PatchOptions) -> bool {
        let Some(item) = self.editor.scene.active().map(|i| i.id) else {
            return false;
        };
        let find = |f: u32| candidates.iter().find(|c| c.face == f);
        let n = picks.len();
        // Orient each curve to run on from the end of the one before.
        let mut chain: Vec<Vec<DVec3>> = Vec::new();
        let mut starts: Vec<u32> = Vec::new();
        let mut finishes: Vec<u32> = Vec::new();
        for (k, &f) in picks.iter().enumerate() {
            let Some(c) = find(f) else {
                self.status = "Draw Patch: a picked curve no longer exists".into();
                return false;
            };
            let mut s = c.samples.clone();
            let (mut start, mut finish) = (c.ends[0], c.ends[1]);
            let reverse = if k == 0 {
                let next = find(picks[1])
                    .map(|c| c.samples.clone())
                    .unwrap_or_default();
                let next_ends = [next[0], *next.last().unwrap()];
                !next_ends
                    .iter()
                    .any(|q| (*s.last().unwrap() - *q).length() <= END_TOLERANCE)
            } else {
                chain.last().is_some_and(|prev: &Vec<DVec3>| {
                    (s[0] - *prev.last().unwrap()).length() > END_TOLERANCE
                })
            };
            if reverse {
                s.reverse();
                (start, finish) = (finish, start);
            }
            chain.push(s);
            starts.push(start);
            finishes.push(finish);
        }
        let boundary: Vec<&[DVec3]> = chain.iter().map(|c| c.as_slice()).collect();
        let mut grid = match operandi_core::patch::coons_patch(
            &boundary,
            o.parallel,
            o.perpendicular,
            o.type2,
            o.type1,
            o.make_uvs,
        ) {
            Ok(g) => g,
            Err(e) => {
                self.status = e;
                return false;
            }
        };
        // PCH-05 / PC-8 choice: face the viewer; Flip reverses.
        let mesh = &mut grid.mesh;
        let toward_viewer = self.camera.basis().0 - mesh.vertices[0];
        let facing = mesh.normal(0).dot(toward_viewer) >= 0.;
        if !o.freeze {
            return self.add_live_patch(item, &chain, &starts, &finishes, facing == o.flip);
        }
        if facing == o.flip {
            for f in &mut mesh.faces {
                f.vertices.reverse();
                if let Some(uv) = &mut f.uv {
                    uv.reverse();
                }
            }
        }
        // PC-5: the corners are the curves' own points.
        let weld: Vec<(u32, u32)> = grid.corners.iter().copied().zip(starts).collect();
        debug_assert_eq!(weld.len(), n);
        self.run_ok(Command::AddGeometry(Box::new(operandi_core::AddGeometry {
            item: Some(item),
            mesh: grid.mesh,
            label: "Draw Patch".into(),
            weld,
            join: Vec::new(),
        })))
    }

    /// PC-7 Freeze off: one live patch polygon on the loop's corners, which
    /// are the curves' own points; each corner's two curve ends are joined
    /// into one point so moving it moves both curves. `reverse` turns the
    /// patch round (facing), keeping the first curve as its first side.
    fn add_live_patch(
        &mut self,
        item: u64,
        chain: &[Vec<DVec3>],
        starts: &[u32],
        finishes: &[u32],
        reverse: bool,
    ) -> bool {
        let n = chain.len();
        let order: Vec<usize> = match (reverse, n) {
            (false, _) => (0..n).collect(),
            (true, 3) => vec![1, 0, 2],
            (true, _) => vec![1, 0, 3, 2],
        };
        let mut mesh = operandi_core::Mesh {
            vertices: order.iter().map(|&k| chain[k][0]).collect(),
            ..Default::default()
        };
        mesh.faces.push(operandi_core::Face {
            vertices: (0..n as u32).collect(),
            uv: None,
            material: String::new(),
            smooth: false,
            part: None,
            material_label: None,
        });
        mesh.attributes.representations = vec![operandi_core::PolygonRepresentation::Patch];
        let weld = order
            .iter()
            .enumerate()
            .map(|(new, &k)| (new as u32, starts[k]))
            .collect();
        let join = (0..n)
            .map(|k| (finishes[k], starts[(k + 1) % n]))
            .filter(|(a, b)| a != b)
            .collect();
        self.run_ok(Command::AddGeometry(Box::new(operandi_core::AddGeometry {
            item: Some(item),
            mesh,
            label: "Draw Patch".into(),
            weld,
            join,
        })))
    }

    /// Changing an option rebuilds the last patch in place while nothing
    /// else has been done since: the live preview of PC-7 with Freeze on.
    fn rebuild_patch(&mut self) {
        let Some(tool) = self.draw.clone() else {
            return;
        };
        let Some(last) = tool.last_patch.clone() else {
            return;
        };
        if self.editor.recorded_since(last.mark)
            || self
                .editor
                .history
                .undo
                .last()
                .is_none_or(|s| Some(s.mark) != last.step_mark || s.label != "Draw Patch")
        {
            if let Some(t) = self.draw.as_mut() {
                t.last_patch = None;
            }
            return;
        }
        self.run(Command::Undo);
        self.editor.history.redo.clear();
        let candidates = self.patch_candidates();
        let ok = self.add_patch(&candidates, &last.picks, &tool.patch);
        if let Some(t) = self.draw.as_mut() {
            t.last_patch = ok.then_some(LastPatch {
                picks: last.picks,
                mark: self.editor.history_mark(),
                step_mark: self.editor.history.undo.last().map(|s| s.mark),
            });
        }
    }

    #[cfg(test)]
    pub(super) fn rebuild_patch_for_test(&mut self) {
        self.rebuild_patch();
    }

    fn draw_patch_handles(&self, tool: &DrawTool, painter: &egui::Painter, r: Rect) {
        let accent = theme::viewport::handle_active();
        for c in self.patch_candidates() {
            let picked = tool.picks.contains(&c.face);
            let latest = tool.picks.last() == Some(&c.face);
            if picked {
                let line: Vec<Pos2> = c
                    .samples
                    .iter()
                    .filter_map(|p| self.camera.project(*p, r).map(|(q, _)| q))
                    .collect();
                painter.add(egui::Shape::line(line, Stroke::new(3f32, accent)));
            }
            if let Some((q, _)) = self.camera.project(c.handle, r) {
                // PC-2: picked handles are filled, the latest pick larger.
                painter.circle(
                    q,
                    if latest { 7.5 } else { 5. },
                    if picked { accent } else { Color32::TRANSPARENT },
                    Stroke::new(1.5f32, accent),
                );
            }
        }
    }
}

/// What a click on a curve's handle does (PC-2).
#[derive(Debug, PartialEq)]
enum PatchPick {
    /// The curve joins the loop: the picks so far.
    Picked(Vec<u32>),
    /// The first curve again, closing a loop of three or four: build.
    Close,
}

fn touches(candidates: &[PatchCurve], a: u32, b: u32) -> bool {
    let ends = |f: u32| {
        candidates
            .iter()
            .find(|c| c.face == f)
            .map(|c| [c.samples[0], *c.samples.last().unwrap()])
    };
    match (ends(a), ends(b)) {
        (Some(x), Some(y)) => x
            .iter()
            .any(|p| y.iter().any(|q| (*p - *q).length() <= END_TOLERANCE)),
        _ => false,
    }
}

fn pick_patch_curve(
    candidates: &[PatchCurve],
    picks: &[u32],
    face: u32,
) -> std::result::Result<PatchPick, &'static str> {
    if picks.first() == Some(&face) {
        if picks.len() < 3 {
            return Err("a loop needs three or four curves; pick the next one");
        }
        if !touches(candidates, picks[picks.len() - 1], face) {
            return Err("the last curve doesn't meet the first; pick the next one");
        }
        return Ok(PatchPick::Close);
    }
    if picks.contains(&face) {
        return Err("that curve is already picked");
    }
    if picks.len() >= 4 {
        return Err("click the first curve again to close the loop");
    }
    if let Some(&last) = picks.last()
        && !touches(candidates, last, face)
    {
        return Err("pick the next curve that shares an end with the last one");
    }
    if picks.len() == 3 && !touches(candidates, face, picks[0]) {
        return Err("the fourth curve has to close the loop back to the first");
    }
    let mut out = picks.to_vec();
    out.push(face);
    Ok(PatchPick::Picked(out))
}

/// The draw tools' Tool Properties (Curve, B-Spline, Bezier, Sketch and
/// Draw Patch).
pub(super) struct DrawFields;

/// What the draw panel edits: the tool's options and its editable points.
#[derive(Clone, PartialEq)]
pub(super) struct DrawP {
    tool: DrawTool,
    points: Vec<DVec3>,
}

impl ToolFields for DrawFields {
    type P = DrawP;
    fn params(app: &Operandi) -> Option<DrawP> {
        let tool = app.draw.clone()?;
        let points = if tool.kind.edits_points() {
            app.editable(&tool)
        } else {
            Vec::new()
        };
        Some(DrawP { tool, points })
    }
    fn form(_: &Operandi, d: &DrawP) -> ToolForm<DrawP> {
        let mut f = ToolForm::new();
        let tool = &d.tool;
        f.title(tool.kind.label());
        match tool.kind {
            DrawKind::Sketch => sketch_form(tool, &mut f),
            k if !k.edits_points() => patch_form(tool, &mut f),
            _ => curve_form(d, &mut f),
        }
        f.separator()
            .wide(tf::Button::run("Drop", |app: &mut Operandi| {
                app.drop_draw()
            }));
        f
    }
    fn publish(app: &mut Operandi, d: DrawP) -> Result<()> {
        let tool = app.draw.clone().ok_or("The tool isn't active")?;
        if !tool.kind.edits_points() {
            let rebuild = d.tool.sketch != tool.sketch;
            let rebuild_patch = d.tool.patch != tool.patch;
            if let Some(t) = app.draw.as_mut() {
                t.sketch = d.tool.sketch;
                t.patch = d.tool.patch;
            }
            if rebuild {
                app.rebuild_sketch();
            }
            if rebuild_patch {
                app.rebuild_patch();
            }
            return Ok(());
        }
        let label = tool.kind.label();
        if d.points != app.editable(&tool) {
            app.commit_curve(d.points.clone(), &format!("Move {label} Point"));
        }
        let Some(now) = app.draw.clone() else {
            return Ok(());
        };
        let shape_changed = (d.tool.closed, d.tool.start_control, d.tool.end_control)
            != (now.closed, now.start_control, now.end_control);
        let e = app.editable(&now);
        let per = if now.kind == DrawKind::Bezier { 3 } else { 1 };
        // DPM-14: changing Mode never ends the shape; DPM-11: remembered.
        if d.tool.mode != now.mode {
            app.point_modes.remember(now.kind, d.tool.mode);
        }
        app.draw = Some(DrawTool {
            mode: d.tool.mode,
            current: d.tool.current,
            closed: d.tool.closed,
            start_control: d.tool.start_control,
            end_control: d.tool.end_control,
            tangent: d.tool.tangent,
            ..now
        });
        if shape_changed && e.len() / per >= 2 {
            app.commit_curve(e, &format!("{label} Options"));
        }
        Ok(())
    }
}

/// Curve, B-Spline and Bezier: mode, the current point and the shape.
fn curve_form(d: &DrawP, f: &mut ToolForm<DrawP>) {
    let tool = &d.tool;
    // DPM-11: a short joined-segment choice at the top (UIC-89), the
    // active mode in accent (UIC-01), its gestures in the tooltip (UIC-04).
    let bezier = tool.kind == DrawKind::Bezier;
    f.field(
        tf::segments(
            "draw-mode",
            "Mode",
            &mode_options(bezier),
            |d: &DrawP| d.tool.mode,
            |d, m| d.tool.mode = m,
        )
        .tip(mode_tip(bezier))
        .mini(),
    );
    let per = if tool.kind == DrawKind::Bezier { 3 } else { 1 };
    let count = d.points.len() / per;
    f.note(format!("Number of Points {count}"));
    if let Some(c) = tool.current.filter(|c| *c < count) {
        f.field(
            tf::number(
                "drw_current",
                "Current Point",
                Unit::Count,
                move |_: &DrawP| c as f64 + 1.,
                move |d, v| {
                    d.tool.current = Some((v.round().max(1.) as usize - 1).min(count - 1));
                    Ok(())
                },
            )
            .suffix(""),
        );
        let point = move |d: &DrawP| d.points.get(c * per + per / 2).copied().unwrap_or_default();
        // Moving the point carries its handles with it.
        f.fields_from(
            tf::vector(
                "drw_point",
                "Position",
                Unit::Distance,
                point,
                move |d, a, v| {
                    let mut moved = point(d);
                    moved[a] = v;
                    let delta = moved - point(d);
                    for k in 0..per {
                        if let Some(q) = d.points.get_mut(c * per + k) {
                            *q += delta;
                        }
                    }
                    Ok(())
                },
            )
            .into_iter()
            .map(tf::Field::mini)
            .collect(),
        );
        if per == 3 {
            for (key, label, slot) in [("drw_in", "In", 0), ("drw_out", "Out", 2)] {
                f.fields_from(tf::vector(
                    key,
                    label,
                    Unit::Distance,
                    move |d: &DrawP| d.points.get(c * 3 + slot).copied().unwrap_or_default(),
                    move |d, a, v| {
                        if let Some(q) = d.points.get_mut(c * 3 + slot) {
                            q[a] = v;
                        }
                        Ok(())
                    },
                ));
            }
        }
    }
    f.field(
        tf::check(
            "draw_closed",
            "Closed",
            |d: &DrawP| d.tool.closed,
            |d, on| d.tool.closed = on,
        )
        .mini(),
    );
    if tool.kind == DrawKind::Curve {
        f.field(
            tf::check(
                "draw_start_control",
                "Start Control",
                |d: &DrawP| d.tool.start_control,
                |d, on| d.tool.start_control = on,
            )
            .mini(),
        )
        .field(
            tf::check(
                "draw_end_control",
                "End Control",
                |d: &DrawP| d.tool.end_control,
                |d, on| d.tool.end_control = on,
            )
            .mini(),
        );
    }
    if tool.kind == DrawKind::Bezier {
        f.field(tf::choice(
            "draw-tangent",
            "Tangent Handle",
            &[
                (TangentMode::KeepTangent, "Keep Tangent"),
                (TangentMode::UniformLength, "Uniform Length"),
                (TangentMode::Break, "Break"),
            ],
            |d: &DrawP| d.tool.tangent,
            |d, t| d.tool.tangent = t,
        ));
    }
    f.note(
        "Click: add a point after the current one · drag: move · Ctrl-drag: along one axis · \
         right-click the first point: close · middle-click: delete · Shift-click: new curve",
    );
}

fn sketch_form(tool: &DrawTool, f: &mut ToolForm<DrawP>) {
    use operandi_core::sketch::SketchType;
    let _ = tool;
    let n =
        |key: &str, label: &str, unit: Unit, get: fn(&DrawP) -> f64, set: fn(&mut DrawP, f64)| {
            tf::number(key, label, unit, get, move |d, v| {
                set(d, v);
                Ok(())
            })
        };
    f.field(tf::choice(
        "sketch-type",
        "Type",
        &[
            (SketchType::Curve, "Curve"),
            (SketchType::Face, "Face"),
            (SketchType::Lines, "Lines"),
            (SketchType::Quads, "Quads"),
            (SketchType::Bezier, "Bezier"),
            (SketchType::BSpline, "B-Spline"),
        ],
        |d: &DrawP| d.tool.sketch.kind,
        |d, k| d.tool.sketch.kind = k,
    ))
    .field(n(
        "skt_precision",
        "Precision",
        Unit::Suffix(" px"),
        |d| d.tool.sketch.precision,
        |d, v| d.tool.sketch.precision = v.clamp(1., 1000.),
    ))
    .field(n(
        "skt_width",
        "Width",
        Unit::Distance,
        |d| d.tool.sketch.width,
        |d, v| d.tool.sketch.width = v.max(0.),
    ))
    .field(n(
        "skt_slice",
        "Slice",
        Unit::Count,
        |d| d.tool.sketch.slice as f64,
        |d, v| d.tool.sketch.slice = v.round().clamp(1., 100.) as u32,
    ))
    .field(n(
        "skt_segments",
        "Segments",
        Unit::Count,
        |d| d.tool.sketch.segments as f64,
        |d, v| d.tool.sketch.segments = v.round().clamp(0., 10_000.) as u32,
    ))
    .note("Drag in the viewport to draw. Changing an option rebuilds the last stroke.");
}

fn patch_form(tool: &DrawTool, f: &mut ToolForm<DrawP>) {
    let spacings: Vec<(operandi_core::patch::PatchSpacing, &str)> =
        operandi_core::patch::PatchSpacing::ALL
            .into_iter()
            .map(|s| (s, s.label()))
            .collect();
    let n = |key: &str, label: &str, get: fn(&DrawP) -> f64, set: fn(&mut DrawP, u32)| {
        tf::number(key, label, Unit::Count, get, move |d, v| {
            set(d, v.round().clamp(1., 1000.) as u32);
            Ok(())
        })
    };
    // PC-7: a live patch is drawn at the mesh's Patch Subdivisions, so the
    // grid options only act when it is frozen (UIC greyed with a reason).
    let frozen = tool.patch.freeze;
    let live = "A live patch uses the mesh's Patch Subdivisions; turn Freeze on to set this";
    f.field(
        tf::check(
            "pch_freeze",
            "Freeze",
            |d: &DrawP| d.tool.patch.freeze,
            |d, on| d.tool.patch.freeze = on,
        )
        .tip("On: freeze the patch into polygons. Off: one live patch polygon that follows its curves"),
    )
    .field(
        n(
            "pch_perp",
            "Perpendicular",
            |d| d.tool.patch.perpendicular as f64,
            |d, v| d.tool.patch.perpendicular = v,
        )
        .enabled_if(frozen, live),
    )
    .field(
        n(
            "pch_par",
            "Parallel",
            |d| d.tool.patch.parallel as f64,
            |d, v| d.tool.patch.parallel = v,
        )
        .enabled_if(frozen, live),
    )
    .field(
        tf::choice(
            "pch_type1",
            "Type 1",
            &spacings,
            |d: &DrawP| d.tool.patch.type1,
            |d, s| d.tool.patch.type1 = s,
        )
        .tip("Spacing across the patch, with Perpendicular: Length (even arc length) or Knot (even per span between control points)")
        .enabled_if(frozen, live),
    )
    .field(
        tf::choice(
            "pch_type2",
            "Type 2",
            &spacings,
            |d: &DrawP| d.tool.patch.type2,
            |d, s| d.tool.patch.type2 = s,
        )
        .tip("Spacing along the first picked curve, with Parallel: Length (even arc length) or Knot (even per span between control points)")
        .enabled_if(frozen, live),
    )
    .field(tf::check(
        "pch_flip",
        "Flip",
        |d: &DrawP| d.tool.patch.flip,
        |d, on| d.tool.patch.flip = on,
    ))
    .field(
        tf::check(
            "pch_make_uvs",
            "Make UVs",
            |d: &DrawP| d.tool.patch.make_uvs,
            |d, on| d.tool.patch.make_uvs = on,
        )
        .enabled_if(frozen, "A live patch takes its corners' UVs; turn Freeze on to make a grid of UVs"),
    )
    .note(format!("{} curves picked", tool.picks.len()))
    .note(
        "Click the middle handles of three or four curves around a loop, then the first \
         again to close it. Their ends must meet. Changing an option rebuilds the last patch.",
    );
}
