//! Viewport side of the Slice tools (toolbox-panels-2026-11-06, EDIT.md
//! "Slice section"): presses, drags and the overlays. The geometry and the
//! tools' state live in `operandi_core::slice`; every change here goes
//! through `Command::Preview`, so a press-to-release is one tweak of the
//! tool (mid-tool undo) and Drop records the session as one step.
//!
//! Cut points and rings refer to the session's base mesh (the scene as it
//! was when the tool started), so picking here reads that mesh, not the
//! cut preview on screen.
use super::*;
use operandi_core::{LoopSliceEdit, Mesh, PlaneCut, SlicePoint, SliceShow};

/// Marks the base item's id in the subdivision cache (`slice_base_item`).
const SLICE_BASE: u64 = 1 << 62;

/// Screen distance (points) at which a press grabs a point, handle or bar.
pub(super) const GRAB: f32 = 8.;

/// A Slice tool drag in progress.
#[derive(Clone)]
pub enum SliceDrag {
    /// Loop Slice: slice `index` along the slider bar.
    LoopSlider {
        index: usize,
    },
    /// Loop Slice: one of the bar's end dots (`right` or left).
    LoopBarEnd {
        right: bool,
    },
    /// Edge Slice: point `index` along its edge, whose screen ends (and
    /// depths) were taken at the press.
    EdgePoint {
        index: usize,
        ends: [(Pos2, f64); 2],
    },
    /// Polygon Slice: the current cut's start, end, or the whole line (from
    /// its place at the press, moved by the drag on the work plane).
    PlaneStart,
    PlaneEnd,
    PlaneLine {
        cut: PlaneCut,
        from: DVec3,
    },
    /// Waves 2 and 3 (path_slice_view.rs).
    Path(crate::path_slice_view::PathDrag),
    /// Contour's line, from where the press was (contour_ui.rs).
    Contour {
        from: Pos2,
    },
}

/// The slider bar of Loop Slice in view rect `r`: its left and right ends.
fn bar_ends(o: &operandi_core::LoopSlice, r: Rect) -> (Pos2, Pos2) {
    let left = pos2(r.left() + o.slider_x as f32, r.top() + o.slider_y as f32);
    (left, left + vec2(o.length as f32, 0.))
}

impl Operandi {
    /// The live Slice tool, if one is active.
    pub(super) fn slice_tool(&self) -> Option<Tool> {
        self.editor
            .session
            .live
            .as_ref()
            .map(|l| l.tool)
            .filter(|t| t.is_slice())
    }

    /// The work plane's normal for a new Polygon Slice cut: the fixed work
    /// plane, or the one facing the view.
    pub(super) fn slice_workplane_normal(&self) -> DVec3 {
        match self.actual_plane() {
            1 => DVec3::X,
            2 => DVec3::Z,
            _ => DVec3::Y,
        }
    }

    /// The session's base mesh and its world matrix.
    pub(super) fn slice_base(&self) -> Option<(&Mesh, glam::DMat4)> {
        let live = self.editor.session.live.as_ref()?;
        let item = live.base.active()?;
        Some((&item.mesh, live.base.world_matrix(item.id).ok()?))
    }

    pub(super) fn slice_params(&self) -> Option<Parameters> {
        self.editor.session.live.as_ref().map(|l| l.params.clone())
    }

    /// The session's base item (its own id marked, so its smooth surface
    /// is evaluated once and kept apart from the cut preview's) and its
    /// world matrix.
    fn slice_base_item(&self) -> Option<(operandi_core::Item, glam::DMat4)> {
        let live = self.editor.session.live.as_ref()?;
        let item = live.base.active()?;
        let matrix = live.base.world_matrix(item.id).ok()?;
        let mut base = item.clone();
        base.id |= SLICE_BASE;
        Some((base, matrix))
    }

    /// Nearest edge of the base mesh under `p` with the pointer's place
    /// along it (0..1 from its first point). With `raycast`, edges hidden
    /// behind the surface are skipped. Edges pick where they show: on a
    /// smooth surface, not its cage.
    pub(super) fn slice_pick_edge(
        &self,
        p: Pos2,
        r: Rect,
        raycast: bool,
    ) -> Option<(u32, u32, f64)> {
        let (item, matrix) = self.slice_base_item()?;
        let surface = self.pick_surface();
        let mut candidates = crate::picking::PickCandidates::default();
        self.item_pick_candidates(
            &item,
            matrix,
            Mode::Edge,
            Rect::from_center_size(p, Vec2::splat(24.)),
            &|_| true,
            &self.camera.projector(r),
            &mut candidates,
        );
        let mut best: Option<(f32, u32, u32, f64, [DVec3; 2])> = None;
        for (a, b, [(qa, za), (qb, zb)], [wa, wb]) in candidates.edge_spans() {
            let distance = input::segment_distance(p, qa, qb);
            if distance >= best.map_or(12., |b| b.0) {
                continue;
            }
            let d = qb - qa;
            let s = ((p - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
            let t = picking::span_world_parameter(&self.camera, s, za, zb);
            if !raycast || surface.visible(&self.camera, wa.lerp(wb, t), r) {
                best = Some((distance, a, b, t, [wa, wb]));
            }
        }
        let (_, a, b, t, piece) = best?;
        let path = crate::subdivision_view::edge_path(
            &self.subdivision_cache,
            &item,
            matrix,
            (a.min(b), a.max(b)),
        );
        let t = crate::subdivision_view::path_parameter(&path, piece, t);
        Some((a, b, if a <= b { t } else { 1. - t }))
    }

    /// World position of an Edge Slice point: where it shows, on a smooth
    /// surface's traced edge or on the cage edge.
    pub(super) fn slice_point_world(&self, point: &SlicePoint) -> Option<DVec3> {
        let (item, matrix) = self.slice_base_item()?;
        Some(match *point {
            SlicePoint::Edge { a, b, t } => {
                let count = item.mesh.vertices.len() as u32;
                if a >= count || b >= count {
                    return None;
                }
                let path = crate::subdivision_view::edge_path(
                    &self.subdivision_cache,
                    &item,
                    matrix,
                    (a.min(b), a.max(b)),
                );
                crate::subdivision_view::path_point(&path, if a <= b { t } else { 1. - t })?
            }
            SlicePoint::Inside { at, .. } => matrix.transform_point3(at),
            SlicePoint::Vertex { v } => {
                if v as usize >= item.mesh.vertices.len() {
                    return None;
                }
                matrix.transform_point3(self.subdivision_cache.shown_point(&item, v)?)
            }
        })
    }

    /// Nearest point of the base mesh within [`GRAB`] of `p`, where it
    /// shows (provisional, SL-Q17). With `raycast`, points hidden behind
    /// the surface are skipped.
    fn slice_pick_vertex(&self, p: Pos2, r: Rect, raycast: bool) -> Option<u32> {
        let (item, matrix) = self.slice_base_item()?;
        let surface = self.pick_surface();
        let mut candidates = crate::picking::PickCandidates::default();
        self.item_pick_candidates(
            &item,
            matrix,
            Mode::Vertex,
            Rect::from_center_size(p, Vec2::splat(2. * GRAB)),
            &|_| true,
            &self.camera.projector(r),
            &mut candidates,
        );
        candidates
            .points
            .iter()
            .map(|&(_, v, q, world)| (q.distance(p), v, world))
            .filter(|&(d, _, world)| {
                d <= GRAB && (!raycast || surface.visible(&self.camera, world, r))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, v, _)| v)
    }

    fn slice_point_screen(&self, point: &SlicePoint, r: Rect) -> Option<Pos2> {
        self.camera
            .project(self.slice_point_world(point)?, r)
            .map(|(q, _)| q)
    }

    /// Screen ends (and depths) of base edge `a`–`b`, where its ends show.
    fn slice_edge_ends(&self, a: u32, b: u32, r: Rect) -> Option<[(Pos2, f64); 2]> {
        let (item, matrix) = self.slice_base_item()?;
        let project = |v: u32| {
            self.camera.project(
                matrix.transform_point3(self.subdivision_cache.shown_point(&item, v)?),
                r,
            )
        };
        Some([project(a)?, project(b)?])
    }

    /// The work-plane point under `p` for Polygon Slice.
    fn slice_plane_point(&self, p: Pos2, r: Rect, up: DVec3) -> Option<DVec3> {
        let plane = if up == DVec3::X {
            1
        } else if up == DVec3::Z {
            2
        } else {
            0
        };
        self.camera
            .plane_point(p, r, plane, self.editor.scene.workplane.center)
    }

    /// Run a preview of `p`; on refusal the status says why. Returns whether
    /// it was accepted.
    pub(super) fn slice_publish(&mut self, p: Parameters) -> bool {
        let ok = self.run_ok(Command::Preview(p));
        if ok {
            self.clear_fields();
        }
        ok
    }

    /// A press in the viewport with a Slice tool active. Returns the drag
    /// to follow, or `None` when the press isn't the tool's.
    pub(super) fn slice_press(
        &mut self,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
        button: PointerButton,
    ) -> Option<input::GestureKind> {
        let tool = self.slice_tool()?;
        let p = self.slice_params()?;
        // Off the bar with Edit = Move, a left drag anywhere moves the
        // current slice (LS-20/21, the drag layer); a click that doesn't
        // move still picks a ring (LS-03).
        if (tool, button) == (Tool::LoopSlice, PointerButton::Primary)
            && p.slice.loop_slice.edit == LoopSliceEdit::Move
            && !self.loop_slice_bar_hit(&p, pos, r)
            && let Some(kind) = self.begin_haul(
                operandi_core::haul::Button::Left,
                pos,
                r,
                crate::haul_view::HaulClick::LoopSliceRing(mods),
            )
        {
            return Some(kind);
        }
        let drag = match (tool, button) {
            (Tool::LoopSlice, PointerButton::Primary) => self.loop_slice_press(p, pos, r, mods),
            (Tool::EdgeSlice, PointerButton::Primary) => self.edge_slice_press(p, pos, r, mods),
            (Tool::PolygonSlice, PointerButton::Primary) => {
                self.polygon_slice_press(p, pos, r, mods)
            }
            (Tool::PolygonSlice, PointerButton::Middle) => {
                self.polygon_slice_repeat(p, pos, r);
                None
            }
            (
                Tool::PenSlice | Tool::CurveSlice | Tool::PrimitiveSlice | Tool::AxisSlice,
                PointerButton::Primary,
            ) => self
                .path_slice_press(tool, p, pos, r, mods)
                .map(SliceDrag::Path),
            (Tool::Contour, PointerButton::Primary) => self.contour_press(p, pos, r),
            _ => return None,
        };
        Some(drag.map_or(input::GestureKind::Blocked, input::GestureKind::Slice))
    }

    /// Whether `pos` is on Loop Slice's slider bar or its end dots (the
    /// tool's own on-screen control, which wins over hauls: DL-01).
    pub(super) fn loop_slice_bar_hit(&self, p: &Parameters, pos: Pos2, r: Rect) -> bool {
        let (left, right) = bar_ends(&p.slice.loop_slice, r);
        left.distance(pos) <= GRAB
            || right.distance(pos) <= GRAB
            || ((pos.y - left.y).abs() <= GRAB * 1.5
                && pos.x >= left.x - GRAB
                && pos.x <= right.x + GRAB)
    }

    fn loop_slice_press(
        &mut self,
        mut p: Parameters,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) -> Option<SliceDrag> {
        let o = &mut p.slice.loop_slice;
        let (left, right) = bar_ends(o, r);
        for (end, is_right) in [(left, false), (right, true)] {
            if end.distance(pos) <= GRAB {
                return Some(SliceDrag::LoopBarEnd { right: is_right });
            }
        }
        let on_bar = (pos.y - left.y).abs() <= GRAB * 1.5
            && pos.x >= left.x - GRAB
            && pos.x <= right.x + GRAB;
        if on_bar {
            let t = ((pos.x - left.x) / (right.x - left.x).max(1.)) as f64;
            let nearest = o
                .positions
                .iter()
                .enumerate()
                .map(|(i, &q)| (i, ((q - t).abs() * o.length) as f32))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match o.edit {
                LoopSliceEdit::Move => {
                    let (index, _) = nearest?;
                    o.set_position(index, t);
                    let index = o.current;
                    self.slice_publish(p);
                    return Some(SliceDrag::LoopSlider { index });
                }
                LoopSliceEdit::Add => {
                    o.add(t);
                    let index = o.current;
                    self.slice_publish(p);
                    return Some(SliceDrag::LoopSlider { index });
                }
                LoopSliceEdit::Remove => {
                    if let Some((index, distance)) = nearest
                        && distance <= GRAB
                    {
                        if o.remove(index) {
                            self.slice_publish(p);
                        } else {
                            self.status = "Loop Slice keeps at least one slice".into();
                        }
                    }
                    return None;
                }
            }
        }
        self.loop_slice_pick_ring(p, pos, r, mods);
        None
    }

    /// A click on an edge picks the ring (Shift adds one; provisional,
    /// SL-Q01).
    pub(super) fn loop_slice_pick_ring(
        &mut self,
        mut p: Parameters,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) {
        let o = &mut p.slice.loop_slice;
        match self.slice_pick_edge(pos, r, true) {
            Some((a, b, _)) => {
                if !mods.shift {
                    o.rings.clear();
                }
                let e = (a.min(b), a.max(b));
                if !o.rings.contains(&e) {
                    o.rings.push(e);
                }
                let previous = self.slice_params();
                if !self.slice_publish(p.clone()) {
                    // A ring that can't be cut (no quad beside it) isn't kept.
                    let status = self.status.clone();
                    if let Some(previous) = previous {
                        self.slice_publish(previous);
                    }
                    self.status = status;
                }
            }
            None => self.status = "Loop Slice: click an edge to cut across its ring".into(),
        }
    }

    fn edge_slice_press(
        &mut self,
        mut p: Parameters,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) -> Option<SliceDrag> {
        let o = &p.slice.edge_slice;
        // A press on a placed point: the first one closes the loop; any
        // point can be dragged along its edge.
        let hit = o
            .points
            .iter()
            .enumerate()
            .filter_map(|(i, pt)| Some((i, self.slice_point_screen(pt, r)?.distance(pos))))
            .filter(|(_, d)| *d <= GRAB)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        if let Some(index) = hit {
            if index == 0 && !o.closed && o.points.len() >= 3 {
                p.slice.edge_slice.closed = true;
                if self.slice_publish(p) {
                    self.status = "Edge Slice: closed".into();
                }
                return None;
            }
            if let SlicePoint::Edge { a, b, .. } = o.points[index] {
                let ends = self.slice_edge_ends(a, b, r)?;
                return Some(SliceDrag::EdgePoint { index, ends });
            }
            return None;
        }
        if o.closed {
            self.status = "Edge Slice: the loop is closed; Drop (Q) keeps it".into();
            return None;
        }
        let raycast = o.always_raycast
            || self.display.settings.active.style != view_display::Style::Wireframe;
        // A press on an existing point cuts through it (provisional,
        // SL-Q17); it wins over the edges that meet there.
        if let Some(v) = self.slice_pick_vertex(pos, r, raycast) {
            p.slice.edge_slice.points.push(SlicePoint::Vertex { v });
            self.slice_publish(p);
            return None;
        }
        if let Some((a, b, t)) = self.slice_pick_edge(pos, r, raycast) {
            let middle = mods.ctrl || o.split_middle;
            let t = if middle { 0.5 } else { o.snapped(t) };
            p.slice.edge_slice.points.push(SlicePoint::Edge { a, b, t });
            let index = p.slice.edge_slice.points.len() - 1;
            if !self.slice_publish(p) {
                return None;
            }
            self.slice_readout(index, r);
            if middle {
                return None;
            }
            let ends = self.slice_edge_ends(a, b, r)?;
            return Some(SliceDrag::EdgePoint { index, ends });
        }
        if o.inside
            && let Some(point) = self.slice_inside_point(&p, pos, r)
        {
            p.slice.edge_slice.points.push(point);
            self.slice_publish(p);
            return None;
        }
        self.status = if o.points.is_empty() {
            "Edge Slice: click an edge, a point or inside a polygon to start".into()
        } else {
            "Edge Slice: click an edge or point of a polygon next to the last point".into()
        };
        None
    }

    /// Inside Polygons: the point under `pos` inside a base polygon next to
    /// the last cut point, or inside the nearest polygon under the pointer
    /// when the path is empty (a path may start inside one, SL-Q16).
    fn slice_inside_point(&self, p: &Parameters, pos: Pos2, r: Rect) -> Option<SlicePoint> {
        let (mesh, matrix) = self.slice_base()?;
        let candidates: Vec<usize> = match p.slice.edge_slice.points.last() {
            // One ray test per polygon, once per click.
            None => (0..mesh.faces.len())
                .filter(|&f| !mesh.is_polyline(f))
                .collect(),
            Some(&SlicePoint::Vertex { v }) => mesh
                .point_faces()
                .of(v)
                .iter()
                .map(|&f| f as usize)
                .collect(),
            Some(&SlicePoint::Edge { a, b, .. }) => mesh
                .point_faces()
                .of(a)
                .iter()
                .map(|&f| f as usize)
                .filter(|&f| {
                    let vs = &mesh.faces[f].vertices;
                    let n = vs.len();
                    (0..n).any(|i| {
                        let (x, y) = (vs[i], vs[(i + 1) % n]);
                        (x, y) == (a, b) || (y, x) == (a, b)
                    })
                })
                .collect(),
            Some(&SlicePoint::Inside { face, .. }) => vec![face as usize],
        };
        let local = matrix.inverse();
        let (o, d) = self.camera.ray(pos, r);
        let (o, d) = (local.transform_point3(o), local.transform_vector3(d));
        // The nearest polygon the ray enters in front of the camera.
        let mut best: Option<(f64, SlicePoint)> = None;
        for face in candidates {
            let n = mesh.normal(face);
            let c = mesh.center(face);
            let denom = n.dot(d);
            if denom.abs() < 1e-12 {
                continue;
            }
            let along = n.dot(c - o) / denom;
            if along <= 0. || best.is_some_and(|(b, _)| along >= b) {
                continue;
            }
            let at = o + d * along;
            // Inside test: the hit is on the inner side of every edge.
            let vs = &mesh.faces[face].vertices;
            let inside = (0..vs.len()).all(|i| {
                let (x, y) = (
                    mesh.vertices[vs[i] as usize],
                    mesh.vertices[vs[(i + 1) % vs.len()] as usize],
                );
                (y - x).cross(at - x).dot(n) >= 0.
            });
            if inside {
                let point = SlicePoint::Inside {
                    face: face as u32,
                    at,
                };
                best = Some((along, point));
            }
        }
        best.map(|(_, point)| point)
    }

    /// Edge Slice's Show readout for point `index`, beside the pointer.
    fn slice_readout(&mut self, index: usize, r: Rect) {
        let Some(p) = self.slice_params() else {
            return;
        };
        let o = &p.slice.edge_slice;
        let Some(&SlicePoint::Edge { a, b, t }) = o.points.get(index) else {
            return;
        };
        let length = self.slice_base().map_or(0., |(mesh, matrix)| {
            matrix
                .transform_point3(mesh.vertices[a as usize])
                .distance(matrix.transform_point3(mesh.vertices[b as usize]))
        });
        let text = match o.show {
            SliceShow::None => return,
            SliceShow::Position => format!("{:.1}%", t * 100.),
            SliceShow::Distance => format!("{:.3} m · {:.3} m", t * length, (1. - t) * length),
        };
        self.status = format!("Edge Slice: {text}");
        if let Some(at) = self.slice_point_screen(&o.points[index], r) {
            self.edge_insert_label = Some((at, text));
        }
    }

    /// Angle Snap: with snapping on (or Ctrl), the line from `start` to
    /// `end` turns to the nearest multiple of Angle in the work plane
    /// (provisional, SL-Q10).
    fn slice_snap_angle(
        &self,
        o: &operandi_core::PolygonSlice,
        start: DVec3,
        end: DVec3,
        up: DVec3,
        ctrl: bool,
    ) -> DVec3 {
        if !o.angle_snap || !(self.snapping.enabled || ctrl) || o.angle <= 0. {
            return end;
        }
        let (u, v) = if up == DVec3::X {
            (DVec3::Y, DVec3::Z)
        } else if up == DVec3::Z {
            (DVec3::X, DVec3::Y)
        } else {
            (DVec3::X, DVec3::Z)
        };
        start
            + operandi_core::angle_snap::round_in_plane(end - start, u, v, up, o.angle.to_radians())
    }

    fn polygon_slice_press(
        &mut self,
        mut p: Parameters,
        pos: Pos2,
        r: Rect,
        mods: Modifiers,
    ) -> Option<SliceDrag> {
        let o = &mut p.slice.polygon_slice;
        if !mods.shift
            && let Some(cut) = o.cuts.last().copied()
        {
            let project = |w: DVec3| self.camera.project(w, r).map(|(q, _)| q);
            if let (Some(a), Some(b)) = (project(cut.start), project(cut.end)) {
                if a.distance(pos) <= GRAB {
                    return Some(SliceDrag::PlaneStart);
                }
                if b.distance(pos) <= GRAB {
                    return Some(SliceDrag::PlaneEnd);
                }
                if input::segment_distance(pos, a, b) <= GRAB * 0.75 {
                    let from = self.slice_plane_point(pos, r, cut.up)?;
                    return Some(SliceDrag::PlaneLine { cut, from });
                }
            }
        }
        let up = self.slice_workplane_normal();
        let Some(start) = self.slice_plane_point(pos, r, up) else {
            self.status = "Polygon Slice: the view is edge-on to the work plane".into();
            return None;
        };
        // A plain drag away from the line draws the current cut again;
        // Shift+drag starts another, keeping the earlier ones.
        let cut = PlaneCut {
            start,
            end: start,
            up,
        };
        if mods.shift || o.cuts.is_empty() {
            o.cuts.push(cut);
        } else if let Some(last) = o.cuts.last_mut() {
            *last = cut;
        }
        o.held = o.fast_slice;
        self.slice_publish(p);
        Some(SliceDrag::PlaneEnd)
    }

    /// Middle-click: another cut with the current one's length and
    /// direction, starting at the pointer (provisional, SL-Q11).
    fn polygon_slice_repeat(&mut self, mut p: Parameters, pos: Pos2, r: Rect) {
        let o = &mut p.slice.polygon_slice;
        let Some(last) = o.cuts.last().copied() else {
            self.status = "Polygon Slice: drag a line first".into();
            return;
        };
        let Some(start) = self.slice_plane_point(pos, r, last.up) else {
            return;
        };
        o.cuts.push(PlaneCut {
            start,
            end: start + (last.end - last.start),
            up: last.up,
        });
        // A click is one tweak of its own.
        self.release_pointer();
        self.slice_publish(p);
    }

    /// Pointer moved while a Slice drag is held.
    pub(super) fn slice_drag(&mut self, drag: &SliceDrag, pos: Pos2, r: Rect, mods: Modifiers) {
        if let SliceDrag::Contour { from } = drag {
            self.contour_drag(*from, pos, r);
            return;
        }
        if let SliceDrag::Path(d) = drag {
            self.path_slice_drag(d, pos, r, mods);
            return;
        }
        let Some(mut p) = self.slice_params() else {
            return;
        };
        match drag {
            SliceDrag::Path(_) | SliceDrag::Contour { .. } => {}
            SliceDrag::LoopSlider { index } => {
                let o = &mut p.slice.loop_slice;
                let (left, right) = bar_ends(o, r);
                let t = ((pos.x - left.x) / (right.x - left.x).max(1.)) as f64;
                o.set_position(*index, t);
            }
            SliceDrag::LoopBarEnd { right } => {
                let o = &mut p.slice.loop_slice;
                let (left, end) = bar_ends(o, r);
                if *right {
                    o.length = ((pos.x - left.x) as f64).clamp(40., 2000.);
                } else {
                    // The left dot moves the bar's start; its right end stays.
                    let x = (pos.x - r.left()).clamp(0., end.x - r.left() - 40.);
                    o.length = (end.x - r.left() - x) as f64;
                    o.slider_x = x as f64;
                }
                o.slider_y = ((pos.y - r.top()) as f64).clamp(0., r.height() as f64);
            }
            SliceDrag::EdgePoint { index, ends } => {
                let o = &mut p.slice.edge_slice;
                let [(qa, za), (qb, zb)] = *ends;
                let d = qb - qa;
                let s = ((pos - qa).dot(d) / d.length_sq().max(1e-8)).clamp(0., 1.);
                let t = picking::span_world_parameter(&self.camera, s, za, zb);
                let t = if mods.ctrl || o.split_middle {
                    0.5
                } else {
                    o.snapped(t)
                };
                if let Some(SlicePoint::Edge { t: placed, .. }) = o.points.get_mut(*index) {
                    if *placed == t {
                        return;
                    }
                    *placed = t;
                }
            }
            SliceDrag::PlaneStart | SliceDrag::PlaneEnd | SliceDrag::PlaneLine { .. } => {
                let o = &p.slice.polygon_slice;
                let Some(mut cut) = o.cuts.last().copied() else {
                    return;
                };
                let Some(at) = self.slice_plane_point(pos, r, cut.up) else {
                    return;
                };
                match drag {
                    SliceDrag::PlaneStart => {
                        cut.start = self.slice_snap_angle(o, cut.end, at, cut.up, mods.ctrl)
                    }
                    SliceDrag::PlaneEnd => {
                        cut.end = self.slice_snap_angle(o, cut.start, at, cut.up, mods.ctrl)
                    }
                    SliceDrag::PlaneLine {
                        cut: from_cut,
                        from,
                    } => {
                        let delta = at - *from;
                        cut.start = from_cut.start + delta;
                        cut.end = from_cut.end + delta;
                    }
                    _ => {}
                }
                let o = &mut p.slice.polygon_slice;
                o.held = o.fast_slice;
                if let Some(last) = o.cuts.last_mut() {
                    *last = cut;
                }
            }
        }
        if self.slice_publish(p)
            && let SliceDrag::EdgePoint { index, .. } = drag
        {
            self.slice_readout(*index, r);
        }
    }

    /// The Slice drag ended: Fast Slice makes its cut now.
    pub(super) fn slice_release(&mut self, drag: &SliceDrag) {
        if let SliceDrag::Contour { .. } = drag {
            self.contour_release();
            return;
        }
        if let SliceDrag::Path(d) = drag {
            self.path_slice_release(d);
            return;
        }
        if matches!(
            drag,
            SliceDrag::PlaneStart | SliceDrag::PlaneEnd | SliceDrag::PlaneLine { .. }
        ) && let Some(mut p) = self.slice_params()
            && p.slice.polygon_slice.held
        {
            p.slice.polygon_slice.held = false;
            self.slice_publish(p);
        }
        self.edge_insert_label = None;
    }

    /// Overlays of the active Slice tool.
    pub(super) fn draw_slice(&self, painter: &egui::Painter, r: Rect) {
        let Some(tool) = self.slice_tool() else {
            return;
        };
        let Some(p) = self.slice_params() else {
            return;
        };
        let bar = theme::viewport::slice_bar();
        let point = theme::viewport::slice_point();
        let close = theme::viewport::slice_close();
        let hover = painter.ctx().pointer_hover_pos();
        match tool {
            Tool::LoopSlice => {
                let o = &p.slice.loop_slice;
                let (left, right) = bar_ends(o, r);
                painter.line_segment([left, right], Stroke::new(2.0f32, bar));
                for end in [left, right] {
                    painter.circle_filled(end, 4., bar);
                }
                for (i, &t) in o.positions.iter().enumerate() {
                    let x = left.x + (right.x - left.x) * t as f32;
                    let tip = pos2(x, left.y + 2.);
                    let color = if i == o.current { point } else { bar };
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, tip + vec2(-6., 10.), tip + vec2(6., 10.)],
                        color,
                        Stroke::NONE,
                    ));
                    if i == o.current {
                        painter.text(
                            tip + vec2(0., 12.),
                            Align2::CENTER_TOP,
                            format!("{:.1}%", t * 100.),
                            FontId::proportional(11.),
                            theme::viewport::label(),
                        );
                    }
                }
            }
            Tool::EdgeSlice => {
                let o = &p.slice.edge_slice;
                let screen: Vec<Option<Pos2>> = o
                    .points
                    .iter()
                    .map(|pt| self.slice_point_screen(pt, r))
                    .collect();
                for w in screen.windows(2) {
                    if let (Some(a), Some(b)) = (w[0], w[1]) {
                        painter.line_segment([a, b], Stroke::new(1.5f32, point));
                    }
                }
                if o.closed
                    && let (Some(Some(a)), Some(Some(b))) = (screen.last(), screen.first())
                {
                    painter.line_segment([*a, *b], Stroke::new(1.5f32, point));
                }
                // The first point turns the closing colour when the pointer
                // is on it and a click would close the loop.
                let closing = !o.closed
                    && o.points.len() >= 3
                    && hover
                        .zip(screen.first().copied().flatten())
                        .is_some_and(|(h, first)| h.distance(first) <= GRAB);
                for (i, q) in screen.iter().enumerate() {
                    if let Some(q) = q {
                        let color = if i == 0 && closing { close } else { point };
                        painter.circle_filled(*q, if i == 0 && closing { 6. } else { 4. }, color);
                    }
                }
                if o.multi_slice
                    && !o.closed
                    && let (Some(Some(last)), Some(h)) = (screen.last(), hover)
                    && r.contains(h)
                {
                    painter.extend(egui::Shape::dashed_line(
                        &[*last, h],
                        Stroke::new(1.0f32, point),
                        4.,
                        3.,
                    ));
                }
                if let Some((at, text)) = &self.edge_insert_label {
                    painter.text(
                        *at + vec2(12., -14.),
                        Align2::LEFT_BOTTOM,
                        text,
                        FontId::proportional(11.),
                        theme::viewport::label(),
                    );
                }
            }
            Tool::PenSlice | Tool::CurveSlice | Tool::PrimitiveSlice | Tool::AxisSlice => {
                self.draw_path_slice(painter, r, tool, &p)
            }
            Tool::Contour => self.draw_contour(painter, r, &p),
            _ => {
                let o = &p.slice.polygon_slice;
                let count = o.cuts.len();
                for (i, cut) in o.cuts.iter().enumerate() {
                    let (Some((a, _)), Some((b, _))) = (
                        self.camera.project(cut.start, r),
                        self.camera.project(cut.end, r),
                    ) else {
                        continue;
                    };
                    let current = i + 1 == count;
                    painter
                        .line_segment([a, b], Stroke::new(if current { 2.0f32 } else { 1.0 }, bar));
                    if current {
                        for end in [a, b] {
                            painter.circle_filled(end, 5., point);
                        }
                    }
                }
            }
        }
    }
}
