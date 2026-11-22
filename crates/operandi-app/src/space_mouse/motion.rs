//! From the puck's axes to a view change.
//!
//! The puck moves the scene ("object" feel, provisional): push the cap
//! right and the scene follows right (the view pans left); lift it and the
//! scene rises; pull it toward you and the scene comes closer (zoom in);
//! twist and tilt it and the scene turns the same way, about the orbit
//! centre. Every rate is per second at full deflection, so the motion
//! doesn't depend on the frame rate.
use super::decode::{FULL_SCALE, RawAxes};
use super::prefs::{AxisLock, SpaceMousePreferences};
use crate::camera::{Camera, View};

/// Orbit rate at full deflection, radians per second (provisional).
pub const ROTATE_RATE: f64 = 2.;
/// Pan rate at full deflection, view distances per second (provisional).
pub const PAN_RATE: f64 = 1.;
/// Zoom rate at full deflection: the distance changes by e^ZOOM_RATE per
/// second, about 4.5× (provisional).
pub const ZOOM_RATE: f64 = 1.5;
/// The longest step one frame may take, so a stalled frame doesn't jump.
pub const MAX_STEP: f64 = 0.1;

/// One axis as -1..1, with the dead zone cut out and the rest rescaled so
/// motion starts from zero at its edge.
pub fn shape(raw: i16, dead_zone: f32) -> f64 {
    let v = (f32::from(raw) / FULL_SCALE).clamp(-1., 1.);
    let dz = dead_zone.clamp(0., 0.95);
    if v.abs() <= dz {
        0.
    } else {
        f64::from(v.signum() * (v.abs() - dz) / (1. - dz))
    }
}

/// A view change for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Step {
    /// Radians added to the camera's yaw, pitch and roll.
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
    /// The target moves along the view's right and up, in view distances.
    pub right: f64,
    pub up: f64,
    /// The distance is multiplied by this.
    pub zoom: f64,
}

impl Step {
    pub fn is_rest(&self) -> bool {
        self.yaw == 0.
            && self.pitch == 0.
            && self.roll == 0.
            && self.right == 0.
            && self.up == 0.
            && self.zoom == 1.
    }
}

/// Whether `view` is one of the fixed views (Top, Front ...), which the
/// puck pans and zooms but never turns.
pub fn is_fixed(view: View) -> bool {
    !matches!(view, View::Perspective | View::Orthographic)
}

/// The view change for `axes` held for `dt` seconds. In a `fixed` view
/// (Top, Front ...) the cap's twist and tilt do nothing, pushing it
/// forward and back pans up and down, and lifting and pressing it zooms
/// (Jason, 2026-11-10: a flat view reads like a sheet on the desk).
pub fn step(axes: RawAxes, prefs: &SpaceMousePreferences, dt: f64, fixed: bool) -> Step {
    let rest = Step {
        zoom: 1.,
        ..Step::default()
    };
    if !prefs.enabled || dt <= 0. {
        return rest;
    }
    let dt = dt.min(MAX_STEP);
    let dz = f32::from(prefs.dead_zone) / 100.;
    let flip = |on: bool| if on { -1. } else { 1. };
    let speed = prefs.speed.clamp(
        *super::prefs::SPEED_RANGE.start(),
        *super::prefs::SPEED_RANGE.end(),
    );
    let k = f64::from(speed) * dt;
    let i = prefs.invert;
    let mut s = rest;
    if prefs.axes != AxisLock::PanOnly && !fixed {
        let rate = ROTATE_RATE * k;
        // Device frame: X right, Y toward the user, Z down.
        s.pitch = shape(axes.r[0], dz) * rate * flip(i.tilt);
        s.yaw = shape(axes.r[2], dz) * rate * flip(i.twist);
        if prefs.roll {
            // Negative: the view banks the way the cap tilts (Jason,
            // 2026-11-09, SpaceNavigator through 3DxWare).
            s.roll = -shape(axes.r[1], dz) * rate * flip(i.roll);
        }
    }
    if prefs.axes != AxisLock::RotateOnly {
        let rate = PAN_RATE * k;
        s.right = -shape(axes.t[0], dz) * rate * flip(i.pan_x);
        // Device Y is toward the user and Z is down, so a push forward
        // and a lift are both negative: the scene rises / comes closer.
        let (pan_axis, zoom_axis) = if fixed {
            (axes.t[1], -axes.t[2])
        } else {
            (axes.t[2], axes.t[1])
        };
        s.up = shape(pan_axis, dz) * rate * flip(i.pan_y);
        s.zoom = (-shape(zoom_axis, dz) * ZOOM_RATE * k * flip(i.zoom)).exp();
    }
    s
}

/// Apply `s` to `camera`, turning about its target. A fixed view (Top,
/// Front ...) doesn't turn: the puck only pans and zooms it, so it stays
/// that view (Jason, 2026-11-10). Perspective and the free orthographic
/// view turn as before. Returns whether anything moved.
pub fn apply(camera: &mut Camera, mut s: Step) -> bool {
    if is_fixed(camera.view) {
        s.yaw = 0.;
        s.pitch = 0.;
        s.roll = 0.;
    }
    if s.is_rest() {
        return false;
    }
    if s.yaw != 0. || s.pitch != 0. || s.roll != 0. {
        camera.yaw += s.yaw;
        // The same limit as a mouse orbit: never over the pole.
        camera.pitch = (camera.pitch + s.pitch).clamp(-1.5, 1.5);
        if s.roll != 0. {
            camera.roll_by(s.roll);
        }
    }
    if s.right != 0. || s.up != 0. {
        let (_, right, up, _) = camera.basis();
        camera.target += (right * s.right + up * s.up) * camera.distance;
    }
    if s.zoom != 1. {
        camera.distance = (camera.distance * s.zoom).clamp(0.03, 1e7);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axes(t: [i16; 3], r: [i16; 3]) -> RawAxes {
        RawAxes { t, r }
    }
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn the_dead_zone_is_cut_out_and_the_rest_rescaled() {
        assert_eq!(shape(10, 0.05), 0., "inside the dead zone");
        assert_eq!(shape(-17, 0.05), 0.);
        assert!(close(shape(350, 0.05), 1.));
        assert!(close(shape(-350, 0.05), -1.));
        assert!(close(shape(1000, 0.05), 1.), "harder pushes clip");
        // Halfway between the dead zone's edge and full.
        let half = ((0.05 + 0.95 / 2.) * 350.) as i16;
        assert!((shape(half, 0.05) - 0.5).abs() < 0.01);
        assert!(close(shape(175, 0.), 0.5), "no dead zone: linear");
        assert!(shape(18, 0.05) > 0., "motion starts just past the edge");
    }

    #[test]
    fn steps_are_frame_rate_independent() {
        let p = SpaceMousePreferences::default();
        let a = axes([200, -150, 90], [120, 0, -300]);
        let one = step(a, &p, 1. / 30., false);
        let half = step(a, &p, 1. / 60., false);
        assert!(close(one.yaw, 2. * half.yaw));
        assert!(close(one.pitch, 2. * half.pitch));
        assert!(close(one.right, 2. * half.right));
        assert!(close(one.up, 2. * half.up));
        assert!(close(one.zoom, half.zoom * half.zoom));

        // Each motion on its own lands in the same place at either rate
        // (mixed, they agree only as the frames get shorter: each frame
        // pans along the view as the last one left it).
        for a in [
            axes([0; 3], [120, 0, -300]),
            axes([200, 0, 90], [0; 3]),
            axes([0, -150, 0], [0; 3]),
        ] {
            let mut at_30 = Camera::default();
            let mut at_60 = Camera::default();
            apply(&mut at_30, step(a, &p, 1. / 30., false));
            let half = step(a, &p, 1. / 60., false);
            apply(&mut at_60, half);
            apply(&mut at_60, half);
            assert!(close(at_30.yaw, at_60.yaw), "{a:?}");
            assert!(close(at_30.pitch, at_60.pitch), "{a:?}");
            assert!(close(at_30.distance, at_60.distance), "{a:?}");
            assert!((at_30.target - at_60.target).length() < 1e-9, "{a:?}");
        }
    }

    #[test]
    fn full_deflection_moves_at_the_stated_rates() {
        let p = SpaceMousePreferences {
            dead_zone: 0,
            ..Default::default()
        };
        let s = step(axes([0, 0, 0], [0, 0, 350]), &p, 0.05, false);
        assert!(close(s.yaw, ROTATE_RATE * 0.05));
        let s = step(axes([-350, 0, 0], [0; 3]), &p, 0.05, false);
        assert!(
            close(s.right, PAN_RATE * 0.05),
            "push left: view pans right"
        );
        let s = step(axes([0, 350, 0], [0; 3]), &p, 0.05, false);
        assert!(close(s.zoom, (-ZOOM_RATE * 0.05).exp()), "pull: closer");
        let fast = SpaceMousePreferences {
            speed: 2.,
            ..p.clone()
        };
        let s2 = step(axes([0; 3], [0, 0, 350]), &fast, 0.05, false);
        assert!(close(s2.yaw, 2. * ROTATE_RATE * 0.05), "speed scales");
    }

    #[test]
    fn each_motion_goes_the_way_jason_confirmed() {
        // SM-Q01, tried on a SpaceNavigator through 3DxWare (2026-11-09):
        // every direction right with all Invert switches off, the
        // sideways tilt after flipping its sign.
        let p = SpaceMousePreferences {
            dead_zone: 0,
            roll: true,
            ..Default::default()
        };
        let one = |t: [i16; 3], r: [i16; 3]| step(axes(t, r), &p, 0.05, false);
        assert!(one([0; 3], [0, 300, 0]).roll < 0., "sideways tilt");
        assert!(one([0; 3], [0, 0, 300]).yaw > 0., "twist");
        assert!(one([0; 3], [300, 0, 0]).pitch > 0., "tilt forward/back");
        assert!(one([300, 0, 0], [0; 3]).right < 0., "push right");
        assert!(one([0, 0, 300], [0; 3]).up > 0., "press down");
        assert!(one([0, 300, 0], [0; 3]).zoom < 1., "pull toward you");
    }

    #[test]
    fn a_long_frame_is_capped() {
        let p = SpaceMousePreferences::default();
        let a = axes([0; 3], [0, 0, 350]);
        assert_eq!(step(a, &p, 3., false), step(a, &p, MAX_STEP, false));
    }

    #[test]
    fn locks_inverts_roll_and_disabled() {
        let p = SpaceMousePreferences::default();
        let a = axes([300, 300, 300], [300, 300, 300]);
        let all = step(a, &p, 0.05, false);
        assert!(all.yaw != 0. && all.pitch != 0. && all.right != 0. && all.zoom != 1.);
        assert_eq!(all.roll, 0., "roll is off by default");

        let rot = step(
            a,
            &SpaceMousePreferences {
                axes: AxisLock::RotateOnly,
                ..p.clone()
            },
            0.05,
            false,
        );
        assert!(rot.yaw != 0. && rot.right == 0. && rot.up == 0. && rot.zoom == 1.);
        let pan = step(
            a,
            &SpaceMousePreferences {
                axes: AxisLock::PanOnly,
                ..p.clone()
            },
            0.05,
            false,
        );
        assert!(pan.yaw == 0. && pan.pitch == 0. && pan.right != 0. && pan.zoom != 1.);

        let mut inv = p.clone();
        inv.roll = true;
        inv.invert.twist = true;
        inv.invert.zoom = true;
        inv.invert.pan_x = true;
        let s = step(a, &inv, 0.05, false);
        assert!(close(s.yaw, -all.yaw));
        assert!(close(s.right, -all.right));
        assert!(close(s.zoom, 1. / all.zoom));
        assert!(close(s.up, all.up), "only the flipped ones change");
        assert!(s.roll != 0.);

        let off = SpaceMousePreferences {
            enabled: false,
            ..p.clone()
        };
        assert!(step(a, &off, 0.05, false).is_rest());
        assert!(
            step(axes([3, -4, 5], [6, -7, 8]), &p, 0.05, false).is_rest(),
            "jitter"
        );
    }

    #[test]
    fn a_fixed_view_pans_on_forward_back_and_zooms_on_lift() {
        // Jason, 2026-11-10: in Top, Front ... pushing forward and back
        // pans up and down and lifting zooms; left and right as before.
        let p = SpaceMousePreferences {
            dead_zone: 0,
            ..Default::default()
        };
        let fixed = |t: [i16; 3], r: [i16; 3]| step(axes(t, r), &p, 0.05, true);
        let free = |t: [i16; 3]| step(axes(t, [0; 3]), &p, 0.05, false);
        let fwd = fixed([0, -300, 0], [0; 3]);
        assert!(fwd.up < 0. && fwd.zoom == 1., "push forward: scene rises");
        assert!(close(fwd.up, free([0, 0, -300]).up), "same as a free lift");
        let back = fixed([0, 300, 0], [0; 3]);
        assert!(back.up > 0. && back.zoom == 1.);
        let lift = fixed([0, 0, -300], [0; 3]);
        assert!(lift.zoom < 1. && lift.up == 0., "lift: closer");
        assert!(close(lift.zoom, free([0, 300, 0]).zoom), "as a free pull");
        assert!(fixed([0, 0, 300], [0; 3]).zoom > 1., "press: further");
        assert_eq!(fixed([300, 0, 0], [0; 3]), free([300, 0, 0]), "sideways");
        assert!(
            fixed([0; 3], [300, 300, 300]).is_rest(),
            "twist and tilt do nothing"
        );
        // Perspective and the free ortho view keep their axes.
        assert!(free([0, -300, 0]).zoom > 1. && free([0, -300, 0]).up == 0.);
        assert!(free([0, 0, -300]).up < 0. && free([0, 0, -300]).zoom == 1.);
    }

    #[test]
    fn fixed_views_pan_and_zoom_but_never_turn() {
        let turn = Step {
            yaw: 0.3,
            pitch: 0.2,
            roll: 0.1,
            zoom: 1.,
            ..Default::default()
        };
        for view in [
            View::Top,
            View::Bottom,
            View::Front,
            View::Back,
            View::Left,
            View::Right,
        ] {
            let mut c = Camera {
                view,
                ..Default::default()
            };
            let start = c.clone();
            assert!(!apply(&mut c, turn), "{view:?}: turning alone does nothing");
            assert_eq!(c.view, view);
            assert_eq!(
                (c.yaw, c.pitch, c.roll),
                (start.yaw, start.pitch, start.roll)
            );

            // Turning mixed with a pan and zoom: the pan and zoom happen,
            // and the view stays the same fixed view.
            let (_, right, up, _) = c.basis();
            let d = c.distance;
            assert!(apply(
                &mut c,
                Step {
                    right: 0.5,
                    up: -0.25,
                    zoom: 0.5,
                    ..turn
                }
            ));
            assert_eq!(c.view, view, "{view:?} stays fixed");
            assert_eq!(
                (c.yaw, c.pitch, c.roll),
                (start.yaw, start.pitch, start.roll)
            );
            let moved = (right * 0.5 - up * 0.25) * d;
            assert!(
                (c.target - (start.target + moved)).length() < 1e-9,
                "{view:?}"
            );
            assert!(close(c.distance, d * 0.5));
        }

        // Perspective still turns.
        let mut p = Camera::default();
        assert_eq!(p.view, View::Perspective);
        let yaw = p.yaw;
        assert!(apply(&mut p, turn));
        assert!(close(p.yaw, yaw + 0.3));
    }

    #[test]
    fn applying_orbits_about_the_target_in_the_free_views() {
        let mut c = Camera {
            target: glam::DVec3::new(1., 2., 3.),
            view: View::Orthographic,
            ..Default::default()
        };
        let before = (c.target, c.distance);
        assert!(apply(
            &mut c,
            Step {
                yaw: 0.3,
                pitch: 0.2,
                zoom: 1.,
                ..Default::default()
            }
        ));
        assert_eq!(c.view, crate::camera::View::Orthographic);
        assert_eq!((c.target, c.distance), before, "orbit keeps the centre");
        let pitch = c.pitch;
        apply(
            &mut c,
            Step {
                pitch: 10.,
                zoom: 1.,
                ..Default::default()
            },
        );
        assert!(c.pitch <= 1.5 && c.pitch >= pitch);

        let (_, right, _, _) = c.basis();
        let d = c.distance;
        apply(
            &mut c,
            Step {
                right: 0.5,
                zoom: 0.5,
                ..Default::default()
            },
        );
        assert!((c.target - (before.0 + right * 0.5 * d)).length() < 1e-9);
        assert!(close(c.distance, d * 0.5));
        assert!(!apply(
            &mut c,
            Step {
                zoom: 1.,
                ..Default::default()
            }
        ));
    }
}
