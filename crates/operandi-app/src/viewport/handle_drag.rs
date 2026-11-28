//! Dragging the live tool's handles: the axis, free and off-handle drags,
//! moving the transform centre, axis constraints, and publishing the result
//! to the tool's parameters.
use super::*;

impl Operandi {
    /// The least a Scale handle goes: 0 % for components (GitHub #24),
    /// 0.1 % for items, whose scale can't be zero (IT-04).
    pub(crate) fn scale_floor(&self) -> f64 {
        if self.editor.scene.selection.mode == Mode::Item {
            0.001
        } else {
            0.
        }
    }
    pub(crate) fn update_handle(
        &mut self,
        axis: usize,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        tool: Tool,
        r: Rect,
    ) {
        if axis == FREE_HANDLE {
            self.update_free_handle(base, start, p, tool, r);
            return;
        }
        if axis == OFF_HANDLE {
            self.update_off_handle(base, start, p, tool, r);
            return;
        }
        if is_plane_handle(axis) {
            self.update_plane_handle(axis, base, start, p, tool, r);
            return;
        }
        if precision_handles::is_precision_handle(axis) {
            self.update_precision_handle(axis, base, p, tool, r);
            return;
        }
        if tool == Tool::TwistBevel {
            self.update_twist_bevel_handle(axis, base, start, p, r);
            return;
        }
        if tool == Tool::EdgeExtend && matches!(axis, EXTEND_INSET_HANDLE | EXTEND_SHIFT_HANDLE) {
            self.update_extend_local_handle(axis, base, start, p, r);
            return;
        }
        let combined = combined_handles(tool);
        let family = axis / 3;
        let axis = if combined { axis % 3 } else { axis };
        let tool = if combined {
            match family {
                0 => Tool::Move,
                1 => Tool::Rotate,
                _ => Tool::Scale,
            }
        } else {
            tool
        };
        if tool == Tool::Extrude && axis == EXTRUDE_FREE_HANDLE {
            // Free movement parallel to the view, at the pivot's depth scale
            // used by the axis handles.
            let (_, right, up, _) = self.camera.basis();
            let scale = self.camera.distance / (r.height() as f64 * 0.9);
            let d = p - start;
            let mut params = base;
            params.extent += (right * d.x as f64 - up * d.y as f64) * scale;
            self.publish_handle_params(tool, params);
            return;
        }
        if tool == Tool::VertexBevel && axis == VERTEX_BEVEL_FREE_HANDLE {
            // Off the handle: dragging right widens the Inset, left narrows it
            // (Jason's GUI test, 2026-10-31), at the axis handles' scale.
            let pixels = (p.x - start.x) as f64;
            let mut params = base;
            params.inset =
                (params.inset + pixels * self.camera.distance / (r.height() as f64 * 0.9)).max(0.);
            self.publish_handle_params(tool, params);
            return;
        }
        if tool == Tool::Scale && axis == SCALE_UNIFORM_HANDLE {
            // Right or up enlarges, left or down shrinks; ratios are kept.
            let pixels = ((p.x - start.x) - (p.y - start.y)) as f64;
            let mut params = base;
            // #24: down to 0%. A drag that starts at 0 on every axis has no
            // ratio to keep, so it grows all three from 0 together.
            let floor = self.scale_floor();
            if params.delta.abs().max_element() <= 1e-9 {
                params.delta = DVec3::splat((pixels * 0.01).max(floor));
            } else {
                let least = params.delta.min_element();
                let factor = (1. + pixels * 0.01).max(if least > 0. { floor / least } else { 0. });
                params.delta *= factor;
            }
            self.run(Command::Preview(params));
            self.clear_fields();
            return;
        }
        let direction = self.handle_axis(axis);
        let (_, right, up, _) = self.camera.basis();
        let screen = vec2(direction.dot(right) as f32, -direction.dot(up) as f32);
        let pixels = (p - start).dot(screen) / screen.length_sq().max(0.001);
        // #40: the arrow stays under the pointer: the distance along its
        // line to where the pointer ray passes. The screen-scaled amount is
        // the fallback for an end-on arrow.
        let amount = self
            .handle_origin
            .get()
            .filter(|_| direction.length() > 1e-9)
            .and_then(|origin| self.camera.axis_drag(origin, direction, start, p, r))
            .map(|t| t / direction.length())
            .unwrap_or(pixels as f64 * self.camera.distance / (r.height() as f64 * 0.9));
        let mut params = base;
        let saved_delta = params.delta;
        if combined {
            params.delta = match family {
                1 => params.rotation_delta,
                2 => params.scale_delta,
                _ => params.delta,
            };
        }
        match tool {
            // Same feel as Bevel's Inset handle: dragging out, away from the
            // polygon's middle, makes the inner copy bigger (less Inset).
            Tool::Inset => params.inset -= amount,
            Tool::Shift => params.shift += amount,
            // Twist Bevel's handles have their own drags (twist_bevel_handles.rs).
            Tool::TwistBevel => return,
            // Tweak has no handles in TWK-1; it drags through its own gesture.
            Tool::PolyHaul | Tool::SketchExtrude | Tool::Tweak => return,
            // No handles: they drag anywhere (`update_off_handle`).
            Tool::Push | Tool::Smooth | Tool::SymmetryTool => return,
            Tool::Jitter | Tool::Quantize | Tool::RadialAlign | Tool::LinearAlign => return,
            // The UV tab's tools have no handles: fields, a click (Project
            // from View) or a drag anywhere (Iterations).
            Tool::UvProject
            | Tool::UvViewProject
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvPeeler
            | Tool::UvBoxTransform => {
                return;
            }
            // Bend drags through its own gesture (bend_view.rs).
            Tool::Bend => return,
            // The Slice tools drag through their own gestures (slice_view.rs).
            Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice
            | Tool::Contour => return,
            // So do the Duplicate tools (duplicate_view.rs).
            Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::ScatterClone
            | Tool::Symmetrize => return,
            // The path tools drag their path points (path_tool_view.rs).
            Tool::CurveExtrude | Tool::CurveClone => return,
            Tool::Transform => return,
            // N21: the move arrows (0..3) position the shape; its bound
            // squares drag through the primitive gesture.
            Tool::Sphere | Tool::Cylinder if axis < 3 => params.position[axis] += amount,
            Tool::Sphere | Tool::Cylinder => return,
            Tool::Box => params.position[axis] += amount,
            Tool::Chamfer => {
                params.chamfer.offset = (params.chamfer.offset + amount).max(0.);
            }
            Tool::EdgeExtend => params.delta[axis] += amount,
            Tool::EdgeCrease => {
                // Dragging the handle by the camera's quarter-view raises
                // the weight by 100 %; it runs from -100 % to 100 % (EW-01).
                let span = (self.camera.distance * 0.25).max(1e-9);
                params.shift = (params.shift + amount / span).clamp(-1., 1.);
            }
            // Edge Bevel's second handle: Support Edges (2026-11-23).
            Tool::EdgeBevel if axis == 1 => {
                params.edge_bevel.support = (params.edge_bevel.support + amount).max(0.);
            }
            Tool::EdgeBevel | Tool::EdgeChamfer => {
                params.chamfer.offset = (params.chamfer.offset + amount).max(0.);
            }
            Tool::EdgeRelax => params.shift += amount,
            Tool::VertexCreate => return,
            Tool::VertexBevel => params.inset = (params.inset + amount).max(0.),
            Tool::VertexMerge => params.shift = (params.shift + amount).max(0.),
            Tool::VertexExtrude => {
                if axis == 0 {
                    params.shift += amount;
                } else {
                    params.inset = (params.inset + amount).max(0.);
                }
            }
            Tool::VertexTear => params.delta[axis] += amount,
            Tool::VertexSlide => {
                // World drag along the rail: a share of its length (Percent,
                // ±100 %), or the distance itself in Distance mode.
                if params.edge_slide_distance {
                    params.shift += amount;
                } else {
                    let rail = self.vertex_frame().map_or(1., |f| f.3.max(1e-9));
                    params.shift += amount / rail;
                    // Loop Slide keeps the point on its edge; Stop Negative
                    // Slide keeps the share at 0..100 %.
                    if params.slide_extras.loop_slide {
                        params.shift = params.shift.clamp(-1., 1.);
                    }
                }
                if params.slide_extras.stop_negative {
                    params.shift = params.shift.max(0.);
                }
            }
            Tool::EdgeSlide => {
                // World drag along the rail: a fraction of the rail length, or
                // the distance itself in Distance mode. Not clamped (ES-05).
                if params.edge_slide_distance {
                    params.shift += amount;
                } else {
                    let rail = self.edge_slide_frame().map_or(1., |f| f.2.max(1e-9));
                    params.shift += amount / rail;
                }
                // Stop Negative Slide: the drag can't take the amount below 0
                // (the core also caps each point at its rail's end).
                if params.slide_extras.stop_negative {
                    params.shift = params.shift.max(0.);
                }
            }
            Tool::EdgeExtrude => {
                if axis == 0 {
                    params.shift += amount;
                } else {
                    params.inset = (params.inset + amount).max(0.);
                }
            }
            Tool::Bevel => {
                if axis == 0 {
                    params.shift += amount;
                } else {
                    // User correction 2026-10-30: dragging the square out,
                    // away from the selection center, enlarges the cap (less
                    // Inset); dragging in toward the center shrinks it.
                    params.inset -= amount;
                }
            }
            // D18-A: visible resolved axes write equivalent WORLD Extent XYZ.
            Tool::Extrude => params.extent += direction * amount,
            Tool::SmoothShift | Tool::Thicken => {
                if axis == 0 {
                    params.smooth_shift.offset += amount;
                } else {
                    params.smooth_shift.scale += pixels as f64 * 0.01;
                }
            }
            Tool::Move => params.delta[axis] += amount,
            Tool::Rotate => {
                if let Some((o, _)) = self.camera.project(self.transform_handle_center(), r) {
                    let a = start - o;
                    let b = p - o;
                    let angle = (a.x * b.y - a.y * b.x).atan2(a.dot(b));
                    params.delta[axis] +=
                        angle.to_degrees() as f64 * direction.dot(self.camera.basis().3).signum();
                    // Ctrl held as the drag began: whole steps of Rotation
                    // Snap Angle.
                    if self.handle_constrain {
                        params.delta[axis] = self
                            .handle_preferences
                            .round_to_snap_angle(params.delta[axis]);
                    }
                }
            }
            Tool::Scale => {
                // #24: an axis handle reaches 0% (it stopped at 0.1%).
                let floor = self.scale_floor();
                let scaled = (params.delta[axis] + pixels as f64 * 0.01).max(floor);
                if self.handle_constrain {
                    // HM-30: Ctrl on one axis handle scales all three axes by
                    // the same factor (equal when they start equal). From 0
                    // there is no factor: all three take the dragged value.
                    if params.delta[axis].abs() <= 1e-9 {
                        params.delta = DVec3::splat(scaled);
                    } else {
                        let factor = scaled / params.delta[axis];
                        params.delta = (params.delta * factor).max(DVec3::splat(floor));
                    }
                } else {
                    params.delta[axis] = scaled;
                }
            }
        }
        if combined && family > 0 {
            if family == 1 {
                params.rotation_delta = params.delta;
            } else {
                params.scale_delta = params.delta;
            }
            params.delta = saved_delta;
        }
        // SN-08: a Move drag's handle snaps to the nearest target; on an
        // axis arrow it stays on that axis (SN2-30b Constraint Snapping).
        let line = (!combined || family == 0).then_some(crate::snapping::AxisDrag {
            axis,
            base_delta: saved_delta,
            moved: p - start,
        });
        let params = self.snap_move(tool, params, line, r);
        self.publish_handle_params(tool, params);
    }
    /// Publish one handle sample: through the captured I2 stage for Extrude
    /// when one is active, else as an ordinary live preview.
    /// C-AUTO's and C-SCREEN's press with transform tool `tool` on
    /// (Automatic, None and Screen; AC-RULES v6, GitHub #39): every press
    /// off the handle puts the tool's center where the press lands on the
    /// work plane. The two coordinates in the plane come from that hit;
    /// the third keeps the current center's value. With `snapped` it then
    /// snaps to whatever is under the pointer when Snapping is on; Screen's
    /// never does (AC-12), and takes this view's axes with it (A-SCREEN).
    /// What was done before the press is kept as its own step, and the
    /// same press drags from the new center like the center handle
    /// (Scale: uniformly).
    pub(super) fn place_center_press(
        &mut self,
        tool: Tool,
        pos: Pos2,
        r: Rect,
        snapped: bool,
    ) -> input::GestureKind {
        let center = self.transform_handle_center();
        let (origin, direction) = self.camera.ray(pos, r);
        // The work plane this view draws on: XZ, YZ or XY through the
        // work plane's centre. `normal` is the axis square to it.
        let normal = [1, 0, 2][self.actual_plane().min(2) as usize];
        let height = self.workplane_center()[normal];
        let hit = (direction[normal].abs() > 1e-9)
            .then(|| (height - origin[normal]) / direction[normal])
            .filter(|t| t.is_finite() && *t > 0.)
            .map(|t| origin + direction * t);
        let mut point = match hit {
            Some(mut hit) => {
                hit[normal] = center[normal];
                hit
            }
            // The press never meets the work plane (it is seen edge-on, or
            // lies behind the view): the press at the center's own depth.
            None => {
                let (_, _, _, forward) = self.camera.basis();
                let along = direction.dot(forward);
                if along.abs() < 1e-9 {
                    return input::GestureKind::Blocked;
                }
                origin + direction * ((center - origin).dot(forward) / along)
            }
        };
        // The step so far is kept before the centre moves, so the snap
        // below sees the mesh as it now is.
        if self.editor.live.as_ref().is_some_and(|l| l.applied) {
            self.run(Command::Drop);
            self.activate(tool);
        }
        // SN-12: the new center snaps (Rotate's and Scale's centre too).
        if snapped && let Some(target) = self.snap_placed_center(point, pos, r) {
            point = target;
        }
        let placed = if snapped {
            self.run_ok(Command::SetActionCenter(point))
        } else {
            self.run_ok(Command::ActionContext(self.screen_context(point)))
        };
        if !placed {
            return input::GestureKind::Blocked;
        }
        self.free_transform_drag(tool)
    }
    /// C-ELEM's press with transform tool `tool` on: the polygon, edge or
    /// point under the pointer becomes the Element pick (any shown mesh,
    /// selected or not; the selection stays as it is), and the same press
    /// drags the tool. A press on empty space changes nothing. What was
    /// done before the press is kept as its own step, as for a placed
    /// centre.
    pub(super) fn element_press(&mut self, tool: Tool, pos: Pos2, r: Rect) -> input::GestureKind {
        let picked = self.element_pick_any(pos, r).and_then(|pick| {
            operandi_core::element_frame(&self.editor.scene, pick.grab.item, pick.grab.element).ok()
        });
        let context = match picked {
            Some(frame) => self.editor.action_context.picked(frame),
            None => self.editor.action_context,
        };
        if context != self.editor.action_context {
            if self.editor.live.as_ref().is_some_and(|l| l.applied) {
                self.run(Command::Drop);
                self.activate(tool);
            }
            if !self.run_ok(Command::ActionContext(context)) {
                return input::GestureKind::Blocked;
            }
        }
        self.free_transform_drag(tool)
    }
    /// FO-45: an Element falloff press picks the element under the pointer
    /// (the falloff's Mode decides which kinds), puts the action centre on
    /// it and drags the tool freely. Each press is its own tool
    /// application, so its own undo step.
    pub(super) fn begin_element_falloff_drag(
        &mut self,
        tool: Tool,
        pos: Pos2,
        r: Rect,
    ) -> input::GestureKind {
        let Some(pick) = self.tweak_pick(pos, r) else {
            self.status = "Element falloff: press on a vertex, edge or polygon".into();
            return input::GestureKind::Blocked;
        };
        if self.editor.live.as_ref().is_some_and(|l| l.applied) {
            self.run(Command::Drop);
            self.activate(tool);
        }
        if !self.run_ok(Command::FalloffElementPick(Some(pick.grab)))
            || !self.run_ok(Command::SetActionCenter(pick.at))
        {
            return input::GestureKind::Blocked;
        }
        self.free_transform_drag(tool)
    }
    /// Start a free drag of the live transform tool, as if its center
    /// handle were grabbed (Scale: uniform scale).
    pub(super) fn free_transform_drag(&mut self, tool: Tool) -> input::GestureKind {
        let axis = if tool == Tool::Scale {
            SCALE_UNIFORM_HANDLE
        } else {
            FREE_HANDLE
        };
        match i2_route::capture(&mut self.editor, axis, false) {
            Ok((params, ticket)) => {
                self.i2_gesture_ticket = ticket;
                self.clear_fields();
                input::GestureKind::Handle(axis, params, tool)
            }
            Err(e) => {
                self.status = e;
                input::GestureKind::Blocked
            }
        }
    }
    /// HM-01: the press decides the modifiers for the whole drag. Ctrl is
    /// the constrain key on every platform; a new press forgets the last
    /// drag's axis lock.
    pub(crate) fn begin_constrain(&mut self, modifiers: egui::Modifiers) {
        self.handle_constrain = modifiers.ctrl;
        self.haul_axis = None;
    }
    /// HM-10/12/71: the axis a Ctrl drag is locked to, picked once the
    /// pointer has travelled `HAUL_AXIS_PICK_PIXELS` from the press and kept
    /// until the next press (HM-12a). Shared by Move/Transform hauls and
    /// Tweak so they follow the same rule (HM-70).
    pub(crate) fn haul_lock_axis(&mut self, travel: Vec2) -> Option<DVec3> {
        let axes = crate::draw_tools::plane_axes(self.actual_plane());
        self.lock_axis_among(travel, axes)
    }
    /// `haul_lock_axis` choosing between two given axes (a plane handle's,
    /// PH-22).
    pub(crate) fn lock_axis_among(&mut self, travel: Vec2, axes: [DVec3; 2]) -> Option<DVec3> {
        if self.haul_axis.is_none() && travel.length() >= HAUL_AXIS_PICK_PIXELS {
            self.haul_axis = Some(self.pick_axis_among(travel, axes));
        }
        self.haul_axis
    }
    /// HM-12 (confirmed): of the work plane's two axes, the one whose
    /// on-screen direction is closest to the pointer's travel so far. The
    /// same axes as the curve tools' Ctrl drag (CRV-05).
    pub(super) fn pick_axis_among(&self, travel: Vec2, [a, b]: [DVec3; 2]) -> DVec3 {
        let (_, right, up, _) = self.camera.basis();
        let travel = travel.normalized();
        let closeness = |a: DVec3| {
            let screen = vec2(a.dot(right) as f32, -a.dot(up) as f32);
            if screen.length_sq() < 1e-6 {
                0.
            } else {
                screen.normalized().dot(travel).abs()
            }
        };
        if closeness(a) >= closeness(b) { a } else { b }
    }
    /// Dragging the center handle (or off the model, after the click that
    /// put the center there): Move and Transform follow the pointer in the
    /// view plane; Rotate turns about the frame axes nearest the view's up
    /// (left/right drag) and right (up/down drag), half a degree per pixel.
    /// Provisional (Jason, 2026-11-03); the reference's free rules aren't
    /// measured (HND-Q01/Q02).
    pub(super) fn update_free_handle(
        &mut self,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        tool: Tool,
        r: Rect,
    ) {
        let Some(mut frame) = self.editor.live.as_ref().and_then(|l| l.action_frame) else {
            return;
        };
        // AC-75: under Local the handle's own axes.
        if let Some(basis) = self.local_handle_basis() {
            frame.basis = basis;
        }
        let (_, right, up, _) = self.camera.basis();
        let d = p - start;
        let mut params = base;
        match tool {
            Tool::Move | Tool::Transform if self.handle_constrain => {
                // HM-10/11: a Ctrl haul moves along one axis of the work
                // plane, picked once (HM-12) and kept for the whole drag.
                let Some(axis) = self.haul_lock_axis(d) else {
                    return;
                };
                // Distance along the axis as its own arrow would measure it.
                let screen = vec2(axis.dot(right) as f32, -axis.dot(up) as f32);
                let pixels = d.dot(screen) / screen.length_sq().max(0.001);
                let scale = self.camera.distance / (r.height() as f64 * 0.9);
                // #40: along the axis line through the handle, under the
                // pointer; screen-scaled when the axis is end-on.
                let along = self
                    .camera
                    .axis_drag(self.transform_handle_center(), axis, start, p, r)
                    .unwrap_or(pixels as f64 * scale);
                let world = axis.normalize_or_zero() * along;
                params.delta += frame.basis.transpose() * world;
            }
            Tool::Move | Tool::Transform => {
                // #40: on the view-facing plane through the handle, so the
                // handle stays under the pointer at any depth.
                let scale = self.camera.distance / (r.height() as f64 * 0.9);
                let world = self
                    .camera
                    .view_plane_drag(self.transform_handle_center(), start, p, r)
                    .unwrap_or_else(|| (right * d.x as f64 - up * d.y as f64) * scale);
                params.delta += frame.basis.transpose() * world;
            }
            Tool::Rotate => {
                let nearest = |v: DVec3| {
                    (0..3)
                        .max_by(|&a, &b| {
                            frame
                                .basis
                                .col(a)
                                .dot(v)
                                .abs()
                                .total_cmp(&frame.basis.col(b).dot(v).abs())
                        })
                        .unwrap_or(0)
                };
                let (about_up, about_right) = (nearest(up), nearest(right));
                params.delta[about_up] +=
                    d.x as f64 * 0.5 * frame.basis.col(about_up).dot(up).signum();
                params.delta[about_right] +=
                    d.y as f64 * 0.5 * frame.basis.col(about_right).dot(right).signum();
                // HM-20/21: Ctrl (held at the press) turns in whole steps of
                // Rotation Snap Angle, on a haul as on a ring.
                if self.handle_constrain {
                    for a in [about_up, about_right] {
                        params.delta[a] =
                            self.handle_preferences.round_to_snap_angle(params.delta[a]);
                    }
                }
            }
            _ => return,
        }
        let params = self.snap_move(tool, params, None, r);
        self.publish_handle_params(tool, params);
    }
    /// A drag away from the handles (Jason, 2026-11-04): up/down acts like
    /// the move arrow, left/right like the scale widget, at the axis
    /// handles' scale. Tools with one size-like value (Inset, Offset,
    /// Distance, Crease) take it from left/right; the slides follow their
    /// rail as if its handle were dragged; Extend and Tear move freely in the
    /// view plane. Provisional (DRAG-Q01).
    pub(super) fn update_off_handle(
        &mut self,
        base: Parameters,
        start: Pos2,
        p: Pos2,
        tool: Tool,
        r: Rect,
    ) {
        let d = p - start;
        let scale = self.camera.distance / (r.height() as f64 * 0.9);
        let vertical = -d.y as f64 * scale;
        let horizontal = d.x as f64 * scale;
        let mut params = base;
        match tool {
            Tool::Bevel => {
                params.shift += vertical;
                // Like the Inset square: out (right) enlarges the cap.
                params.inset -= horizontal;
            }
            Tool::SmoothShift | Tool::Thicken => {
                params.smooth_shift.offset += vertical;
                params.smooth_shift.scale += d.x as f64 * 0.01;
            }
            Tool::EdgeExtrude | Tool::VertexExtrude => {
                params.shift += vertical;
                params.inset = (params.inset + horizontal).max(0.);
            }
            Tool::Chamfer | Tool::EdgeBevel | Tool::EdgeChamfer => {
                params.chamfer.offset = (params.chamfer.offset + horizontal).max(0.);
            }
            // As Bevel's: right enlarges the inner copy (less Inset); Shift
            // follows up/down like a move arrow.
            Tool::Inset => params.inset -= horizontal,
            Tool::Shift => params.shift += vertical,
            // TB-Q08 (provisional): a drag off the handles changes Shift,
            // like Bevel's plain drag.
            Tool::TwistBevel => params.twist_bevel.shift += vertical,
            Tool::EdgeRelax => params.shift += horizontal,
            // Push: up pushes out, down pulls in, like the move arrow.
            Tool::Push => params.shift += vertical,
            // Symmetry Tool: right raises Threshold, left lowers it.
            Tool::SymmetryTool => {
                params.symmetry_fix.threshold =
                    (params.symmetry_fix.threshold + horizontal).max(0.);
            }
            // Jitter: right widens Range on every enabled axis (a world
            // distance, like the move arrows).
            Tool::Jitter => {
                let r = (params.jitter.range.max_element() + horizontal).max(0.);
                params.jitter.range = DVec3::new(
                    if params.jitter.enable[0] {
                        r
                    } else {
                        params.jitter.range.x
                    },
                    if params.jitter.enable[1] {
                        r
                    } else {
                        params.jitter.range.y
                    },
                    if params.jitter.enable[2] {
                        r
                    } else {
                        params.jitter.range.z
                    },
                );
            }
            // Quantize: right makes the grid step bigger on every axis.
            Tool::Quantize => {
                params.quantize.step =
                    DVec3::splat((params.quantize.step.max_element() + horizontal).max(0.));
            }
            // Radial / Linear Align: right raises Weight (200 px = 100%).
            Tool::RadialAlign => {
                params.radial_align.weight =
                    (params.radial_align.weight + d.x as f64 / 200.).clamp(0., 1.);
            }
            Tool::LinearAlign => {
                params.linear_align.weight =
                    (params.linear_align.weight + d.x as f64 / 200.).clamp(0., 1.);
            }
            // Smooth: right raises Strength, left lowers it (200 px = 1).
            Tool::Smooth => {
                params.smooth.strength =
                    (params.smooth.strength + d.x as f64 / 200.).clamp(0., 100.);
            }
            Tool::VertexMerge => params.shift = (params.shift + horizontal).max(0.),
            Tool::EdgeCrease => {
                let span = (self.camera.distance * 0.25).max(1e-9);
                params.shift = (params.shift + horizontal / span).clamp(-1., 1.);
            }
            Tool::EdgeSlide | Tool::VertexSlide => {
                self.update_handle(0, params, start, p, tool, r);
                return;
            }
            // Edge Extend's Haul Local: horizontal = Inset, vertical = Shift.
            Tool::EdgeExtend if params.edge_extend.haul == operandi_core::ExtendHaul::Local => {
                params.inset += horizontal;
                params.shift += vertical;
            }
            Tool::EdgeExtend | Tool::VertexTear => {
                let (_, right, up, _) = self.camera.basis();
                let world = (right * d.x as f64 - up * d.y as f64) * scale;
                let frame = self.editor.live.as_ref().and_then(|l| l.action_frame);
                params.delta += frame.map_or(world, |f| f.basis.transpose() * world);
            }
            // UV Unwrap, Relax and Unwrap & Relax: Iterations (UV-Q6).
            Tool::UvUnwrap | Tool::UvRelax | Tool::UvPelt => {
                match self.uv_iterations_drag(&params, tool, d.x as f64) {
                    Some(p) => params = p,
                    None => return,
                }
            }
            _ => return,
        }
        self.publish_handle_params(tool, params);
    }
    pub(super) fn publish_handle_params(&mut self, tool: Tool, params: Parameters) {
        if tool == Tool::Extrude && (self.editor.i2_active() || self.i2_gesture_ticket.is_some()) {
            let Some(ticket) = self.i2_gesture_ticket else {
                self.status = "Stale extrusion gesture; recapture the current handle".into();
                return;
            };
            match i2_route::move_captured(&mut self.editor, ticket, params) {
                Ok(()) => {
                    self.i2_gesture_ticket = self.editor.extrude_stage_ticket().ok();
                    self.status = "Ready".into();
                    self.clear_fields();
                }
                Err(error) => self.status = error,
            }
        } else {
            self.run(Command::Preview(params));
            self.clear_fields();
        }
    }
}
