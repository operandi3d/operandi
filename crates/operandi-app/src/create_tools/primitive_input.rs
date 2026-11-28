//! Shape-specific drag interpretation. Shared viewport arbitration dispatches
//! captured primitive gestures here; preview transactions, targeting, history
//! and mesh insertion remain in portable core services. A future primitive adds
//! its own gesture/parameter generator here without changing shared move/release
//! ownership or scene transaction logic.
use super::*;
#[derive(Clone)]
pub enum Gesture {
    Draw(Parameters, u8, DVec3),
    RadialDraw(Parameters, u8, DVec3),
    RadialSide(usize, bool, Parameters, Tool),
    Side(usize, bool, Parameters, bool),
    Segments(Parameters, u8),
}
impl Operandi {
    pub(super) fn update_primitive(
        &mut self,
        gesture: &Gesture,
        g: &input::Navigation,
        pos: Pos2,
        r: Rect,
    ) {
        match gesture {
            Gesture::RadialSide(axis, maximum, base, tool) => {
                let mut params = base.clone();
                let Some(radial) = params.radial.as_mut() else {
                    return;
                };
                let direction = [DVec3::X, DVec3::Y, DVec3::Z][*axis];
                let (Some((origin, _)), Some((unit, _))) = (
                    self.camera.project(base.position, r),
                    self.camera.project(base.position + direction, r),
                ) else {
                    return;
                };
                let screen = unit - origin;
                if screen.length() < 1. {
                    self.status =
                        "This axis is viewed end-on. Orbit or use numeric Radius fields.".into();
                    return;
                }
                let mut amount = (pos - g.start).dot(screen) as f64 / screen.length_sq() as f64;
                // PM-C04: approved provisional Operandi policy. Reuse the
                // picked side's world axis; no dimensions or stored mode change.
                if g.modifiers.ctrl {
                    params.position[*axis] += amount;
                    self.run(Command::Preview(params));
                    self.clear_fields();
                    return;
                }
                let sign = if *maximum { 1. } else { -1. };
                // SN2-41: the dragged side snaps, staying on its axis. Either
                // shape's side moves by `amount` from where it was.
                let side = base.position[*axis] + sign * radial.radius[*axis];
                let mut at = base.position;
                at[*axis] = side + amount;
                amount = self.snap_placement(at, &[], Some((at, direction)), r)[*axis] - side;
                let Some(radial) = params.radial.as_mut() else {
                    return;
                };
                if *tool == Tool::Sphere {
                    radial.radius[*axis] += amount * sign;
                } else {
                    radial.radius[*axis] += amount * sign * 0.5;
                    params.position[*axis] += amount * 0.5;
                }
                if radial.radius[*axis] < 0. {
                    self.status =
                        "Crossed bounds are unavailable; drag outward or use numeric fields."
                            .into();
                    return;
                }
                self.run(Command::Preview(params));
                self.clear_fields();
            }
            Gesture::RadialDraw(base, plane, center) => {
                // The press point (snapped at the press, SN2-41) starts the
                // drag; the opposite corner snaps after Coordinate Rounding.
                let (Some(a), Some(b)) =
                    (Some(*center), self.placement_point(pos, r, *plane, *center))
                else {
                    self.status =
                        "View is parallel to the construction plane; align the view before drawing"
                            .into();
                    return;
                };
                let normal = match plane {
                    1 => 0,
                    2 => 2,
                    _ => 1,
                };
                let mut p = base.clone();
                let Some(radial) = p.radial.as_mut() else {
                    return;
                };
                // Sphere and Cylinder centre on the drag start; the drag's
                // extent in each axis is that radius (drag-creation D3/D4).
                if g.modifiers.ctrl {
                    let (position, size) = initial_draw_bounds(a, b, normal, true);
                    p.position = position;
                    radial.radius = size * 0.5;
                } else {
                    let mut extent = (b - a).abs();
                    extent[normal] = 0.;
                    p.position = a;
                    radial.radius = extent;
                }
                if (0..3)
                    .filter(|&axis| axis != normal)
                    .all(|axis| radial.radius[axis] > 1e-8)
                {
                    self.run(Command::Preview(p));
                    self.clear_fields();
                }
            }
            Gesture::Segments(base, plane) => {
                let (u, v) = match plane {
                    1 => (1, 2),
                    2 => (0, 1),
                    _ => (0, 2),
                };
                let delta = pos - g.start;
                let mut params = base.clone();
                for (axis, pixels) in [(u, delta.x as f64), (v, -delta.y as f64)] {
                    let count =
                        base.segments[axis] as f64 + (pixels / self.segment_detent_pixels).trunc();
                    params.segments[axis] = count.clamp(1., u32::MAX as f64) as u32;
                }
                if self
                    .editor
                    .session
                    .live
                    .as_ref()
                    .is_some_and(|live| live.params != params)
                {
                    self.run(Command::Preview(params));
                    self.clear_fields();
                }
            }
            Gesture::Side(axis, maximum, params, jump) => {
                if *jump && g.start.distance(pos) < 1. {
                    return;
                }
                let direction = [DVec3::X, DVec3::Y, DVec3::Z][*axis];
                let Some((origin, _)) = self.camera.project(params.position, r) else {
                    return;
                };
                let Some((unit, _)) = self.camera.project(params.position + direction, r) else {
                    return;
                };
                let screen = unit - origin;
                if screen.length() < 1. {
                    self.status =
                        "Can't tell which Cube side to drag in this view; orbit or use the numeric bounds"
                            .into();
                    return;
                }
                let bounds = BoxBounds::new(params.position, params.size);
                let Ok(bounds) = bounds else {
                    return;
                };
                let original = if *maximum {
                    bounds.max[*axis]
                } else {
                    bounds.min[*axis]
                };
                let coordinate = if *jump {
                    params.position[*axis]
                        + (pos - origin).dot(screen) as f64 / screen.length_sq() as f64
                } else {
                    original + (pos - g.start).dot(screen) as f64 / screen.length_sq() as f64
                };
                // SN2-41: the dragged side (height drags too) snaps, staying
                // on its axis.
                let mut at = params.position;
                at[*axis] = coordinate;
                let coordinate = self.snap_placement(at, &[], Some((at, direction)), r)[*axis];
                let mode = if g.modifiers.ctrl {
                    BoxHandleMode::MoveAll
                } else {
                    params.box_handle_mode
                };
                match bounds.haul_side(*axis, *maximum, coordinate, mode) {
                    Ok(bounds) => {
                        let mut p = params.clone();
                        p.position = bounds.position();
                        p.size = bounds.size();
                        self.run(Command::Preview(p));
                        self.clear_fields();
                    }
                    Err(error) => self.status = error,
                }
            }
            Gesture::Draw(params, plane, center) => {
                // As for RadialDraw: the snapped press point and a snapped
                // opposite corner (SN2-41).
                if let (Some(a), Some(b)) =
                    (Some(*center), self.placement_point(pos, r, *plane, *center))
                {
                    let mut p = params.clone();
                    let normal = match plane {
                        1 => 0,
                        2 => 2,
                        _ => 1,
                    };
                    let (position, size) = initial_draw_bounds(a, b, normal, g.modifiers.ctrl);
                    p.size = size;
                    if (0..3)
                        .filter(|&axis| axis != normal)
                        .all(|axis| p.size[axis] > 1e-6)
                    {
                        p.position = position;
                        if self.editor.dispatch(Command::Preview(p)).is_ok() {
                            self.clear_fields();
                        }
                    }
                } else {
                    self.status="View is parallel to the construction plane; align the view before dragging".into();
                }
            }
        }
    }
}

impl Operandi {
    /// SN2-41: a primitive's placed point: the work plane point under
    /// `pos` after Coordinate Rounding, then snapped (the order the path
    /// tools use: `work_point`, then the snap).
    fn placement_point(&mut self, pos: Pos2, r: Rect, plane: u8, center: DVec3) -> Option<DVec3> {
        let p = self.work_point(pos, r, plane, center)?;
        Some(self.snap_placement(p, &[], None, r))
    }
    /// Start drawing the armed numeric Cone or Ellipsoid on the construction
    /// plane (Jason, 2026-11-02; EA-Q01 is open for the reference's own
    /// handles, so this drawing is provisional). The press sets the centre.
    pub(super) fn begin_solid_draw(
        &mut self,
        cone: bool,
        pos: Pos2,
        r: Rect,
    ) -> input::GestureKind {
        let plane = self.actual_plane();
        self.snap_session = Default::default();
        match self.placement_point(pos, r, plane, self.workplane_center()) {
            Some(anchor) => input::GestureKind::SolidDraw {
                cone,
                plane,
                anchor,
            },
            None => {
                self.status =
                    "View is parallel to the construction plane; align the view before drawing"
                        .into();
                input::GestureKind::Blocked
            }
        }
    }
    /// Dragging sets the size: the distance on the plane is the radius; the
    /// Ellipsoid is round, the Cone stands on the plane 1.5 radii tall (the
    /// default proportions), both along the plane's normal.
    pub(super) fn solid_draw(
        &mut self,
        cone: bool,
        plane: u8,
        anchor: DVec3,
        pos: Pos2,
        r: Rect,
        uniform: bool,
    ) {
        let Some(q) = self.placement_point(pos, r, plane, self.workplane_center()) else {
            return;
        };
        let radius = q.distance(anchor);
        if radius < 1e-6 {
            return;
        }
        let normal = match plane {
            0 => 1,
            1 => 0,
            _ => 2,
        };
        // Without Ctrl: a flat base on the plane centred on the press, each
        // in-plane size following the drag (drag-creation-2026-11-06; height
        // comes from the centre handle, `session_draw`).
        if !uniform {
            let extent = crate::session_draw::flat_extent(anchor, q, normal);
            if (0..3).filter(|&i| i != normal).any(|i| extent[i] < 1e-6) {
                return;
            }
            if cone {
                let Some(session) = &self.editor.session.cone else {
                    return;
                };
                let mut p = session.requested.clone();
                p.position = anchor;
                p.size = extent;
                p.axis = normal;
                self.run(Command::ConfigureCone(p));
            } else {
                let Some(session) = &self.editor.session.ellipsoid else {
                    return;
                };
                let mut p = session.requested.clone();
                p.position = anchor;
                p.radius = extent;
                p.axis = normal;
                self.run(Command::ConfigureEllipsoid(p));
            }
            self.clear_fields();
            return;
        }
        if cone {
            let Some(session) = &self.editor.session.cone else {
                return;
            };
            let mut p = session.requested.clone();
            let mut size = DVec3::splat(radius);
            size[normal] = radius * 1.5;
            let mut position = anchor;
            position[normal] += size[normal];
            p.position = position;
            p.size = size;
            p.axis = normal;
            self.run(Command::ConfigureCone(p));
        } else {
            let Some(session) = &self.editor.session.ellipsoid else {
                return;
            };
            let mut p = session.requested.clone();
            p.position = anchor;
            p.radius = DVec3::splat(radius);
            p.axis = normal;
            self.run(Command::ConfigureEllipsoid(p));
        }
        self.clear_fields();
    }
    pub(super) fn begin_primitive_alternate(
        &mut self,
        live: Live,
        plane: u8,
    ) -> input::GestureKind {
        match live.tool {
            Tool::Box => input::GestureKind::segments(live.params, plane),
            _ => {
                self.status =
                    "Alternate mouse hauling is unavailable; use numeric Sides and Segments".into();
                input::GestureKind::Blocked
            }
        }
    }
    /// MN-03.1: a middle press with a drawn Cube, Sphere or Cylinder keeps
    /// it and puts a copy with the same settings (size, segments, sides…)
    /// at the pointer on the work plane; the drag then adjusts the copy as
    /// a drag after drawing does (its nearest side).
    pub(super) fn begin_primitive_copy(
        &mut self,
        live: Live,
        pos: Pos2,
        r: Rect,
    ) -> input::GestureKind {
        let tool = live.tool;
        if self.editor.scene == live.base {
            self.status = format!(
                "Draw a {} first; a middle-drag then draws copies of it",
                tool.label()
            );
            return input::GestureKind::Blocked;
        }
        let plane = self.actual_plane();
        let Some(anchor) = self.work_point(pos, r, plane, live.params.position) else {
            self.status =
                "View is parallel to the construction plane; align the view before drawing".into();
            return input::GestureKind::Blocked;
        };
        let normal = match plane {
            1 => 0,
            2 => 2,
            _ => 1,
        };
        let mut copy = live.params.clone();
        let mut at = anchor;
        at[normal] = copy.position[normal];
        copy.position = at;
        let mut staged = self.editor.clone();
        if let Err(error) = staged
            .dispatch(Command::Drop)
            .and_then(|()| staged.dispatch(Command::Activate(tool)))
            .and_then(|()| staged.dispatch(Command::Preview(copy)))
        {
            self.status = format!("Couldn't copy the {}: {error}", tool.label());
            return input::GestureKind::Blocked;
        }
        self.editor = staged;
        self.clear_fields();
        let next = self
            .editor
            .session
            .live
            .clone()
            .expect("Activate opened the tool");
        self.begin_primitive(next, pos, r, Modifiers::NONE)
    }
    pub(super) fn begin_primitive(
        &mut self,
        live: Live,
        pos: Pos2,
        r: Rect,
        modifiers: Modifiers,
    ) -> input::GestureKind {
        // Jason decision 2026-11-01 (BOX-Q05): Shift-drag away from the
        // handles keeps the current primitive and draws the next one, which
        // starts with the same settings (Drop stores them, Activate reuses
        // them). With nothing drawn yet there is nothing to keep, so the
        // rules below apply.
        if modifiers.shift
            && !modifiers.mac_cmd
            && self.editor.scene != live.base
            && self.handle_hit(pos, r).is_none()
        {
            let tool = live.tool;
            let mut staged = self.editor.clone();
            if let Err(error) = staged
                .dispatch(Command::Drop)
                .and_then(|()| staged.dispatch(Command::Activate(tool)))
            {
                self.status = format!("Couldn't start another {}: {error}", tool.label());
                return input::GestureKind::Blocked;
            }
            self.editor = staged;
            self.clear_fields();
            let next = self
                .editor
                .session
                .live
                .clone()
                .expect("Activate opened the tool");
            let modifiers = Modifiers {
                shift: false,
                ..modifiers
            };
            return self.begin_primitive(next, pos, r, modifiers);
        }
        if matches!(live.tool, Tool::Sphere | Tool::Cylinder) {
            if modifiers.shift || modifiers.mac_cmd {
                // Spec: PM-Q02
                self.status = "Shift-drag a handle or Cmd-drag isn't supported yet; Shift-drag away from the handles to draw another".into();
                return input::GestureKind::Blocked;
            }
            let started = live.applied
                || live
                    .params
                    .radial
                    .is_some_and(|p| p.radius.min_element() == 0.);
            if started {
                // N21: the move arrows (0..3) position the shape.
                if let Some(id) = self.handle_hit(pos, r).filter(|&id| id < 3) {
                    return input::GestureKind::Handle(id, live.params, live.tool);
                }
                if let Some(id) = self.handle_hit(pos, r) {
                    return input::GestureKind::Primitive(Gesture::RadialSide(
                        (id - 3) / 2,
                        (id - 3) % 2 == 1,
                        live.params,
                        live.tool,
                    ));
                }
                // Away from the handles the nearest side handle is dragged,
                // as for the Cube (BOX-Q02; Jason, 2026-11-04; DRAG-Q01).
                // With Ctrl that moves the whole shape (HM-52, PM-C04).
                let side = self
                    .handles(r)
                    .into_iter()
                    .filter(|h| h.0 >= 3)
                    .min_by(|a, b| {
                        a.2.distance_sq(pos)
                            .total_cmp(&b.2.distance_sq(pos))
                            .then(a.0.cmp(&b.0))
                    });
                if let Some((id, _, _, _)) = side {
                    return input::GestureKind::Primitive(Gesture::RadialSide(
                        (id - 3) / 2,
                        (id - 3) % 2 == 1,
                        live.params,
                        live.tool,
                    ));
                }
                self.status = "Orbit the view to see a side handle to drag".into();
                return input::GestureKind::Blocked;
            }
            let plane = self.actual_plane();
            self.snap_session = Default::default();
            let Some(anchor) = self.placement_point(pos, r, plane, self.workplane_center()) else {
                self.status =
                    "View is parallel to the construction plane; align the view before drawing"
                        .into();
                return input::GestureKind::Blocked;
            };
            return input::GestureKind::Primitive(Gesture::RadialDraw(live.params, plane, anchor));
        }
        if live.tool != Tool::Box {
            self.status = "Primitive gesture provider is unavailable".into();
            return input::GestureKind::Blocked;
        }
        if modifiers.shift || (modifiers.ctrl && modifiers.mac_cmd) {
            // Spec: PM-Q02
            self.status="Shift-drag a handle or Ctrl+Cmd-drag isn't supported yet; Shift-drag away from the handles to draw another Cube".into();
            return input::GestureKind::Blocked;
        }
        if let Some(handle) = self.handle_hit(pos, r) {
            return if handle >= 3 {
                input::GestureKind::side(
                    (handle - 3) / 2,
                    (handle - 3) % 2 == 1,
                    live.params,
                    false,
                )
            } else {
                input::GestureKind::Handle(handle, live.params, live.tool)
            };
        }
        if !live.applied {
            let plane = self.actual_plane();
            self.snap_session = Default::default();
            let Some(anchor) = self.placement_point(pos, r, plane, self.workplane_center()) else {
                self.status =
                    "View is parallel to the construction plane; align the view before drawing"
                        .into();
                return input::GestureKind::Blocked;
            };
            return input::GestureKind::draw(live.params, plane, anchor);
        }
        // BOX-Q02 approved bounded selector, separate from reference fidelity.
        // With Ctrl the nearest side moves the whole Cube (HM-52, Move All).
        let side = self
            .handles(r)
            .into_iter()
            .filter(|h| h.0 >= 3)
            .min_by(|a, b| {
                a.2.distance_sq(pos)
                    .total_cmp(&b.2.distance_sq(pos))
                    .then(a.0.cmp(&b.0))
            });
        if let Some((id, _, _, _)) = side {
            input::GestureKind::side((id - 3) / 2, (id - 3) % 2 == 1, live.params, true)
        } else {
            input::GestureKind::Blocked
        }
    }
}

/// Shared Cube/Sphere/Cylinder construction policy (IC-P01..04). Returned size
/// is full extent; radial tools convert it to half-extents at their boundary.
/// Normal displacement never drives a major-plane gesture.
fn initial_draw_bounds(a: DVec3, b: DVec3, normal: usize, constrained: bool) -> (DVec3, DVec3) {
    let mut extent = (b - a).abs();
    extent[normal] = 0.;
    if constrained {
        (a, DVec3::splat(2. * extent.max_element()))
    } else {
        ((a + b) * 0.5, extent)
    }
}
