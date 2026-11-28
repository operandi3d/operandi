//! Drag creation for the panel-driven primitives (Cone, Ellipsoid, Torus,
//! Capsule), matching Cube, Sphere and Cylinder (spec
//! drag-creation-2026-11-06; Jason's bug report 2026-11-06): without Ctrl
//! the first drag draws a flat base on the construction plane, centred on
//! the press, each in-plane size following the drag; a drag that starts on
//! the flat base's centre handle then gives it height along the plane's
//! normal. Ctrl keeps the uniform drag. The height gesture is provisional
//! (the spec's height stage is open, Q-DC-01).
use super::*;
use operandi_core::shapes::ShapeParameters;

/// Pixels from the centre handle that still grab it.
const HEIGHT_HANDLE_PIXELS: f32 = 14.;

impl Operandi {
    /// The flat session base's centre (world) and normal axis, if any.
    pub(super) fn session_flat_base(&self) -> Option<(DVec3, usize)> {
        if let Some(s) = &self.editor.session.cone {
            let p = &s.requested;
            return (p.size[p.axis.min(2)] == 0.).then_some((p.position, p.axis.min(2)));
        }
        if let Some(s) = &self.editor.session.ellipsoid {
            let p = &s.requested;
            return (p.radius[p.axis.min(2)] == 0.).then_some((p.position, p.axis.min(2)));
        }
        match &self.editor.session.shape.as_ref()?.requested {
            ShapeParameters::Torus(t) => (t.height == 0.).then_some((t.position, t.axis.min(2))),
            ShapeParameters::Capsule(c) => {
                let a = c.axis.min(2);
                (c.radius[a] == 0.).then_some((c.position, a))
            }
            _ => None,
        }
    }

    /// The flat base's height handle on screen.
    pub(super) fn session_height_handle(&self, r: Rect) -> Option<Pos2> {
        let (center, _) = self.session_flat_base()?;
        self.camera.project(center, r).map(|(p, _)| p)
    }

    /// A press on the height handle starts the height drag.
    pub(super) fn begin_session_height(&self, pos: Pos2, r: Rect) -> Option<input::GestureKind> {
        let handle = self.session_height_handle(r)?;
        if handle.distance(pos) > HEIGHT_HANDLE_PIXELS {
            return None;
        }
        let (base, axis) = self.session_flat_base()?;
        Some(input::GestureKind::SessionHeight { base, axis })
    }

    /// Height from the vertical drag (up is +normal); the base stays on the
    /// plane.
    pub(super) fn session_height(
        &mut self,
        base: DVec3,
        axis: usize,
        start: Pos2,
        pos: Pos2,
        r: Rect,
    ) {
        let height = (start.y - pos.y) as f64 * self.camera.distance / (r.height() as f64 * 0.9);
        if height.abs() < 1e-6 {
            return;
        }
        let mut center = base;
        center[axis] += height * 0.5;
        let half = height.abs() * 0.5;
        if let Some(s) = &self.editor.session.cone {
            let mut p = s.requested.clone();
            p.size[axis] = half;
            p.position = center;
            self.run(Command::ConfigureCone(p));
        } else if let Some(s) = &self.editor.session.ellipsoid {
            let mut p = s.requested.clone();
            p.radius[axis] = half;
            p.position = center;
            self.run(Command::ConfigureEllipsoid(p));
        } else if let Some(s) = &self.editor.session.shape {
            let mut p = s.requested.clone();
            match &mut p {
                ShapeParameters::Torus(t) => {
                    t.height = half;
                    t.position = center;
                }
                ShapeParameters::Capsule(c) => {
                    // Rounded ends first, then straight sides. This used to
                    // clamp End Size to the previous frame's value, so the
                    // first small drag step shrank the ends to almost
                    // nothing and left a cylinder.
                    c.axis = axis;
                    c.set_drawn_height(half);
                    c.position = center;
                }
                _ => return,
            }
            self.run(Command::ConfigureShape(p));
        }
        self.clear_fields();
    }
}

/// The drag's extent in each axis on the plane, 0 along `normal`.
pub(super) fn flat_extent(anchor: DVec3, q: DVec3, normal: usize) -> DVec3 {
    let mut e = (q - anchor).abs();
    e[normal] = 0.;
    e
}
