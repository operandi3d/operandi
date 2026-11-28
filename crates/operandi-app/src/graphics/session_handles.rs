//! Handles on the panel-driven primitives (#37, Steve Hill): once a Cone,
//! Ellipsoid, Torus, Capsule or Text is drawn, three move arrows at its
//! centre position it, as the Cube's, Sphere's and Cylinder's do; the
//! solids also get a square at each end of each axis that sets that radius.
//!
//! Provisional (Q-CN-06, NPR-Q01: the specs leave handles open):
//! - Ellipsoid and Torus resize from the centre, like the Sphere (the
//!   Ellipsoid's Handle Mode default is "Resize from Center"); Cone and
//!   Capsule keep the opposite side still, like the Cylinder (the Cone's
//!   default is "Resize").
//! - The Torus squares set its Radius X/Y/Z as the panel does (Modo's
//!   convention, NPR-TOR-03): along the axis Height, across it Cross
//!   Section with Hole Size kept.
//! - Text, Arc and Teapot get the move arrows only; their sizes stay on
//!   the panel (#42 for Arc and Teapot).
//! - A flat base (no height yet) keeps its green height handle instead
//!   (session_draw.rs).
use super::*;
use operandi_core::shapes::ShapeParameters;

/// Where the open session's shape is, read at the press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SessionShape {
    pub position: DVec3,
    /// Half size along each world axis; `None` for Text (move only).
    pub radius: Option<DVec3>,
    /// The squares resize about the centre (else the far side stays).
    pub from_center: bool,
}

impl Operandi {
    /// The open, drawn Cone/Ellipsoid/Torus/Capsule/Text/Arc/Teapot
    /// session's shape.
    pub(crate) fn session_shape(&self) -> Option<SessionShape> {
        if self.editor.session.live.is_some() || self.session_flat_base().is_some() {
            return None;
        }
        let shape = |position, radius: Option<DVec3>, from_center| {
            Some(SessionShape {
                position,
                radius: radius.map(DVec3::abs),
                from_center,
            })
        };
        if let Some(s) = &self.editor.session.cone {
            s.preview.as_ref()?;
            return shape(s.requested.position, Some(s.requested.size), false);
        }
        if let Some(s) = &self.editor.session.ellipsoid {
            s.preview.as_ref()?;
            return shape(s.requested.position, Some(s.requested.radius), true);
        }
        let s = self.editor.session.shape.as_ref()?;
        s.preview.as_ref()?;
        match &s.requested {
            ShapeParameters::Torus(t) => shape(t.position, Some(t.radius()), true),
            ShapeParameters::Capsule(c) => shape(c.position, Some(c.radius), false),
            ShapeParameters::Text(t) => shape(t.position, None, false),
            // #42: Arc and Teapot had no handles; their sizes are single
            // numbers, so like Text they get the move arrows only.
            ShapeParameters::Arc(arc) => shape(arc.center, None, false),
            ShapeParameters::Teapot(t) => shape(t.position, None, false),
            _ => None,
        }
    }

    /// The session's handles: move arrows 0..3, then the radius squares
    /// `3 + axis * 2 + (1 on the + side)`, as Cube, Sphere and Cylinder.
    pub(crate) fn session_handles(&self, r: Rect) -> Vec<(usize, Pos2, Pos2, String)> {
        let Some(shape) = self.session_shape() else {
            return Vec::new();
        };
        if self.editor.is_armed() || !self.handle_preferences.selected_shown() {
            return Vec::new();
        }
        let Some((center, _)) = self.camera.project(shape.position, r) else {
            return Vec::new();
        };
        let zoom = self.handle_zoom_at(shape.position);
        self.handle_zoom.set(zoom);
        let s = self.handle_preferences.scale * zoom;
        let mut out = Vec::new();
        for axis in 0..3 {
            let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
            if let Some((_, arrow)) = self.handle_arrow(shape.position, direction, 72. * s, r) {
                out.push((
                    axis,
                    center,
                    center + arrow,
                    format!("Move {}", ["X", "Y", "Z"][axis]),
                ));
            }
        }
        let Some(radius) = shape.radius else {
            return out;
        };
        for axis in 0..3 {
            let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
            if radius[axis] <= 0. {
                continue;
            }
            for maximum in [false, true] {
                let sign = if maximum { 1. } else { -1. };
                let at = shape.position + direction * radius[axis] * sign;
                if let Some((point, _)) = self.camera.project(at, r) {
                    out.push((
                        3 + axis * 2 + usize::from(maximum),
                        point,
                        point,
                        format!(
                            "{}{} radius",
                            if maximum { "+" } else { "−" },
                            ["X", "Y", "Z"][axis]
                        ),
                    ));
                }
            }
        }
        out
    }

    /// A press on one of the session's handles starts its drag.
    pub(crate) fn begin_session_handle(&self, pos: Pos2, r: Rect) -> Option<input::GestureKind> {
        let base = self.session_shape()?;
        let id = self.handle_hit(pos, r).filter(|&id| id < 9)?;
        Some(input::GestureKind::SessionHandle { id, base })
    }

    /// Drags handle `id` from `start` to `pos`: an arrow moves the shape
    /// along its axis; a square moves that side (both sides with
    /// `from_center`). The moved point snaps on its axis (SN2-41).
    pub(crate) fn session_handle_drag(
        &mut self,
        id: usize,
        base: SessionShape,
        start: Pos2,
        pos: Pos2,
        r: Rect,
    ) {
        let axis = if id < 3 { id } else { (id - 3) / 2 };
        let direction = [DVec3::X, DVec3::Y, DVec3::Z][axis];
        let (Some((origin, _)), Some((unit, _))) = (
            self.camera.project(base.position, r),
            self.camera.project(base.position + direction, r),
        ) else {
            return;
        };
        let screen = unit - origin;
        if screen.length() < 1. {
            self.status = "This axis is viewed end-on. Orbit or use the numeric fields.".into();
            return;
        }
        let amount = (pos - start).dot(screen) as f64 / screen.length_sq() as f64;
        if id < 3 {
            let mut at = base.position;
            at[axis] += amount;
            let at = self.snap_placement(at, &[], Some((at, direction)), r);
            let mut position = base.position;
            position[axis] = at[axis];
            self.set_session_shape(position, None);
            return;
        }
        let Some(radius) = base.radius else {
            return;
        };
        let sign = if (id - 3) % 2 == 1 { 1. } else { -1. };
        let side = base.position[axis] + sign * radius[axis];
        let mut at = base.position;
        at[axis] = side + amount;
        let amount = self.snap_placement(at, &[], Some((at, direction)), r)[axis] - side;
        let mut position = base.position;
        let value = if base.from_center {
            radius[axis] + amount * sign
        } else {
            position[axis] += amount * 0.5;
            radius[axis] + amount * sign * 0.5
        };
        if value <= 1e-9 {
            self.status =
                "Crossed bounds are unavailable; drag outward or use numeric fields.".into();
            return;
        }
        self.set_session_shape(position, Some((axis, value)));
    }

    /// Reconfigures the open session at `position`, with radius `axis` set
    /// to the value when given.
    fn set_session_shape(&mut self, position: DVec3, radius: Option<(usize, f64)>) {
        if let Some(s) = &self.editor.session.cone {
            let mut p = s.requested.clone();
            p.position = position;
            if let Some((axis, value)) = radius {
                p.size[axis] = value;
            }
            self.run(Command::ConfigureCone(p));
        } else if let Some(s) = &self.editor.session.ellipsoid {
            let mut p = s.requested.clone();
            p.position = position;
            if let Some((axis, value)) = radius {
                p.radius[axis] = value;
            }
            self.run(Command::ConfigureEllipsoid(p));
        } else if let Some(s) = &self.editor.session.shape {
            let mut p = s.requested.clone();
            match &mut p {
                ShapeParameters::Torus(t) => {
                    t.position = position;
                    if let Some((axis, value)) = radius {
                        t.set_radius(axis, value);
                    }
                }
                ShapeParameters::Capsule(c) => {
                    c.position = position;
                    if let Some((axis, value)) = radius {
                        c.radius[axis] = value;
                        // The rounded ends can't be longer than the capsule.
                        c.end_size = c.end_size.min(c.radius[c.axis.min(2)]);
                    }
                }
                ShapeParameters::Text(t) => t.position = position,
                ShapeParameters::Arc(arc) => arc.center = position,
                ShapeParameters::Teapot(t) => t.position = position,
                _ => return,
            }
            self.run(Command::ConfigureShape(p));
        }
        self.clear_fields();
    }
}
