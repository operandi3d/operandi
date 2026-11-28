//! Sketch strokes and the Pen draft in the viewport: the plane a new point
//! lands on, pressing, snapping and dragging Pen points.
use super::*;

impl Operandi {
    pub(crate) fn sketch_plane_point(
        camera: &Camera,
        rect: Rect,
        pos: Pos2,
        center: DVec3,
        normal: DVec3,
    ) -> Option<DVec3> {
        let (origin, direction) = camera.ray(pos, rect);
        let den = direction.dot(normal);
        if den.abs() < 1e-9 {
            return None;
        }
        let t = (center - origin).dot(normal) / den;
        (t >= 0. && t.is_finite()).then_some(origin + direction * t)
    }
    pub(crate) fn sketch_feedback_points(&self) -> Vec<DVec3> {
        if let Some(input::Navigation {
            kind: input::GestureKind::SketchStroke(stroke),
            ..
        }) = &self.navigation
        {
            let mut points = stroke.samples.clone();
            if points
                .last()
                .is_some_and(|p| p.distance(stroke.current) > 1e-9)
            {
                points.push(stroke.current);
            }
            return points;
        }
        self.editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::SketchExtrude)
            .map(|l| l.params.sketch_path.clone())
            .unwrap_or_default()
    }
    pub(super) fn sketch_stroke_move(
        &mut self,
        stroke: &mut input::SketchStroke,
        pos: Pos2,
        finish: bool,
    ) {
        let Some(point) = Self::sketch_plane_point(
            &stroke.camera,
            stroke.rect,
            pos,
            stroke.plane_center,
            stroke.plane_normal,
        ) else {
            self.status = "Sketch Extrude cannot project this pointer onto its frozen plane".into();
            if let Some(live) = &mut self.editor.session.live {
                live.preview_error = Some(self.status.clone());
            }
            return;
        };
        stroke.current = point;
        if pos.distance(stroke.last_sample) > 1e-4 {
            stroke.started = true;
        }
        if !stroke.started {
            return;
        }
        let mut path = stroke.samples.clone();
        if path
            .last()
            .is_some_and(|p| p.distance(stroke.current) > 1e-9)
        {
            path.push(stroke.current);
        }
        if path.len() > 257 {
            self.status = "Sketch Extrude path exceeds 257 points".into();
            if let Some(live) = &mut self.editor.session.live {
                live.preview_error = Some(self.status.clone());
            }
            return;
        }
        let Some(live) = self
            .editor
            .session
            .live
            .as_ref()
            .filter(|l| l.tool == Tool::SketchExtrude)
        else {
            return;
        };
        let mut params = live.params.clone();
        params.sketch_path = if path.len() >= 2 {
            path.clone()
        } else {
            Vec::new()
        };
        match self.editor.dispatch(Command::Preview(params)) {
            Ok(()) => {
                self.status = "Ready".into();
                self.clear_fields();
                if path.len() >= 2
                    && (finish || pos.distance(stroke.last_sample) as f64 >= stroke.precision)
                {
                    stroke.samples = path;
                    stroke.last_sample = pos;
                }
            }
            Err(error) => {
                self.status = error.clone();
                if let Some(live) = &mut self.editor.session.live {
                    live.preview_error = Some(error);
                }
            }
        }
    }
    /// The Pen draft's construction plane: fixed by its first point, so
    /// orbiting mid-draft cannot put points on different planes.
    pub(super) fn pen_plane_point(&self, pos: Pos2, r: Rect) -> Option<DVec3> {
        let (plane, center) = match (&self.editor.session.pen_draft, self.pen_plane) {
            (Some(d), Some(fixed)) if !d.points().is_empty() => fixed,
            _ => (self.actual_plane(), self.workplane_center()),
        };
        self.work_point(pos, r, plane, center)
    }
    /// Press with a Pen draft: pick a draft point under the pointer, else add
    /// one where the pointer meets the plane. Returns the point's index.
    /// `end` (Ctrl+click with Make Quads) adds one point that closes the
    /// strip with a triangle.
    /// The Pen draft point under `pos`, if any (nearest within 8 px).
    pub(crate) fn pen_hit(&self, pos: Pos2, r: Rect) -> Option<usize> {
        let draft = self.editor.session.pen_draft.as_ref()?;
        draft
            .points()
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| self.camera.project(p, r).map(|(q, _)| (i, q.distance(pos))))
            .filter(|&(_, d)| d <= 8.)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    pub(crate) fn pen_press(&mut self, pos: Pos2, r: Rect, end: bool) -> Option<usize> {
        let mode = self.point_modes.pen;
        if let Some(i) = self.pen_hit(pos, r) {
            // DPM-04/10: Delete mode removes the clicked point.
            if mode == crate::draw_tools::DrawMode::Delete {
                self.run(Command::PenDelete {
                    index: i,
                    and_after: false,
                });
                self.clear_fields();
                return None;
            }
            self.run(Command::PenSelect(i));
            return Some(i);
        }
        // DPM-03: Edit and Delete clicks in empty space add nothing.
        if mode != crate::draw_tools::DrawMode::Add {
            return None;
        }
        let draft = self.editor.session.pen_draft.as_ref()?;
        let first = draft.points().is_empty();
        let before = draft.points().len();
        let Some(point) = self.pen_plane_point(pos, r) else {
            self.status = "Click below the horizon: the pointer must meet the Pen plane".into();
            return None;
        };
        // DSN-07: a placed point snaps as a dragged one does.
        self.snap_session = Default::default();
        let point = self.pen_snap(point, None, r);
        // SYM3-23: a point placed near the symmetry plane lands on it.
        let point = self.symmetry_seam_snap(point, r);
        self.sync_pen_uv_view();
        self.run(if end {
            Command::PenCaptureEnd(point)
        } else {
            Command::PenCapture(point)
        });
        self.clear_fields();
        let draft = self.editor.session.pen_draft.as_ref()?;
        if draft.points().len() <= before {
            return None;
        }
        // A Make Quads click adds the point and its partner; the clicked
        // point (now current) is the one a drag moves.
        let index = draft.current().unwrap_or(before);
        if first {
            self.pen_plane = Some((self.actual_plane(), self.workplane_center()));
        }
        self.sync_pen_plane();
        Some(index)
    }
    /// DPM-06: a middle-click on a Pen draft point or Tube path point
    /// removes it (`and_after`: it and every later one). One undo step.
    pub(super) fn point_delete_press(&mut self, pos: Pos2, r: Rect, and_after: bool) {
        if self.editor.session.pen_draft.is_some() {
            if let Some(index) = self.pen_hit(pos, r) {
                self.run(Command::PenDelete { index, and_after });
                self.clear_fields();
            }
        } else if let Some(index) = self.tube_hit(pos, r) {
            self.delete_tube_point(index, and_after);
        }
    }
    /// DSN-08/10: `point` (on the Pen plane) snapped to the active targets;
    /// the draft's other points are targets too, `dragged` (being moved)
    /// isn't. Unchanged when snapping is off or nothing is in range.
    pub(super) fn pen_snap(&mut self, point: DVec3, dragged: Option<usize>, r: Rect) -> DVec3 {
        let Some(draft) = self.editor.session.pen_draft.as_ref() else {
            self.note_snap_drag();
            return point;
        };
        let points = draft.points();
        let moving = dragged.and_then(|i| points.get(i).copied());
        let own: Vec<DVec3> = points
            .iter()
            .enumerate()
            .filter(|&(i, _)| Some(i) != dragged)
            .map(|(_, &p)| p)
            .collect();
        // The new point goes after the current one; a dragged one sits
        // after the point before it.
        let from_index = match dragged {
            Some(i) => i.checked_sub(1),
            None => draft.current().or(points.len().checked_sub(1)),
        };
        let from = from_index.and_then(|i| points.get(i)).copied();
        let before = from_index
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| points.get(i))
            .copied();
        let plane = self.pen_plane.map_or(self.actual_plane(), |p| p.0);
        let normal = [DVec3::Y, DVec3::X, DVec3::Z][plane.min(2) as usize];
        let path = crate::snap_kinds::Path {
            from,
            before,
            normal,
        };
        // SNH-Q01: direction locks act only while a point is dragged
        // (press-drag to place, or moving one), never on a plain click.
        self.snap_path_point(point, moving, &own, &path, dragged.is_some(), r)
    }
    /// The Pen's and Pen Slice's shared snap (DSN-08/10, SN2-34..37):
    /// `point` snapped to the active targets (`own` are the path's other
    /// points, targets too; `moving` is the dragged point's old place,
    /// never one); with no point target in range, the segment from
    /// `path.from` locks to World Axis / Straight Line / Right Angle when
    /// those are ticked. Unchanged when snapping is off.
    pub(crate) fn snap_path_point(
        &mut self,
        point: DVec3,
        moving: Option<DVec3>,
        own: &[DVec3],
        path: &crate::snap_kinds::Path,
        dragging: bool,
        r: Rect,
    ) -> DVec3 {
        self.note_snap_drag();
        let targets = self.snap_targets();
        if !targets.any() {
            return point;
        }
        let moving: Vec<DVec3> = moving.into_iter().collect();
        // SN2-37: a point target in range wins over a direction lock.
        if let Some(snapped) = self.snap_draw_point(point, &moving, own, None, r) {
            return snapped;
        }
        if !dragging {
            return point;
        }
        let camera = &self.camera;
        crate::snap_kinds::lock_segment(
            targets.world_axis,
            targets.straight_line,
            targets.right_angle,
            path,
            point,
            |p| camera.project(p, r).map(|q| q.0),
            self.snapping.inner_range,
        )
    }
    /// Drag of Pen point `index` to the pointer on the draft's plane.
    pub(super) fn pen_drag(&mut self, index: usize, pos: Pos2, r: Rect) {
        let Some(point) = self.pen_plane_point(pos, r) else {
            return;
        };
        let point = self.pen_snap(point, Some(index), r);
        let point = self.symmetry_seam_snap(point, r);
        if self
            .editor
            .session
            .pen_draft
            .as_ref()
            .and_then(|d| d.current())
            != Some(index)
        {
            self.run(Command::PenSelect(index));
        }
        self.run(Command::PenRefine(point));
        self.clear_fields();
    }
}
