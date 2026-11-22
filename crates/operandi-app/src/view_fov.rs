//! Alt+Shift+wheel over a Perspective view changes its field of view
//! (Jason, 2026-11-20; not in the reference shortcut list, which binds only
//! the plain wheel, SC-039). The lens and the view's distance change
//! together, as Flatness of Perspective does (`Camera::reflatten`): what
//! sits at the orbit centre keeps its size on screen and only the strength
//! of the perspective changes. A readout at the top of the view shows the
//! field of view and its full-frame focal length while it changes.
//! Alt+Shift+middle-click puts the default lens back (Jason, 2026-11-21),
//! unless the navigation preset uses that press. Provisional: not from a
//! spec.
use crate::Operandi;
use crate::camera::{Camera, MAX_DISTANCE, View};
use eframe::egui::{self, Align2, Event, FontId, MouseWheelUnit, Painter, Rect, Ui, vec2};

/// The widest and narrowest lens Alt+Shift+wheel reaches: about 150° and
/// 5° of vertical field of view.
pub const MIN_LENS: f64 = 0.134;
pub const MAX_LENS: f64 = 11.4;
/// How long the readout stays after the last wheel step, in seconds.
const READOUT_SECONDS: f64 = 1.5;
/// Film height for the focal length readout: a 36 x 24 mm full frame.
const FILM_HEIGHT_MM: f64 = 24.;

/// A Perspective view's vertical field of view in degrees for `lens` (the
/// view's half height spans 1 / (2 lens) at unit depth).
pub fn vertical_fov_degrees(lens: f64) -> f64 {
    2. * (0.5 / lens).atan().to_degrees()
}

/// The full-frame focal length in millimetres with the same vertical
/// field of view.
pub fn focal_length_mm(lens: f64) -> f64 {
    FILM_HEIGHT_MM * lens
}

/// "Field of View 58° · 22 mm".
pub fn readout(lens: f64) -> String {
    format!(
        "Field of View {:.0}° · {:.0} mm",
        vertical_fov_degrees(lens),
        focal_length_mm(lens)
    )
}

impl Camera {
    /// Lengthen the lens by `ratio` (> 1 narrows the field of view), within
    /// [`MIN_LENS`, `MAX_LENS`], moving the eye so the orbit centre keeps
    /// its size. Only free Perspective views: Camera and Light view take
    /// their lens from the item, and orthographic views have no field of
    /// view. `false` when nothing changed.
    pub fn change_fov(&mut self, ratio: f64) -> bool {
        if self.view != View::Perspective
            || self.looks_through()
            || !(ratio.is_finite() && ratio > 0.)
        {
            return false;
        }
        let lens = (self.lens * ratio).clamp(MIN_LENS, MAX_LENS);
        let ratio = lens / self.lens;
        let distance = (self.distance * ratio).min(MAX_DISTANCE.max(self.distance));
        if lens == self.lens {
            return false;
        }
        self.lens = lens;
        self.distance = distance;
        true
    }
}

impl Operandi {
    /// Does an Alt+Shift+middle press with `mods` reset the field of view?
    /// Not when the navigation preset navigates with it (Maya's Alt+middle
    /// pans), so no preset loses a gesture.
    pub(super) fn is_fov_reset_press(&self, mods: egui::Modifiers) -> bool {
        mods.alt
            && mods.shift
            && !mods.ctrl
            && !mods.mac_cmd
            && self
                .input_preferences
                .navigation
                .navigation(egui::PointerButton::Middle, mods)
                .is_none()
    }

    /// Alt+Shift+middle-click: the view's lens back to the Flatness of
    /// Perspective lens (58° at the default 60%), the orbit centre keeping
    /// its size, with the readout.
    pub(super) fn reset_fov(&mut self, now: f64) {
        if self.camera.view != View::Perspective || self.camera.looks_through() {
            self.status = "Field of View: Perspective views only".into();
            return;
        }
        self.camera
            .change_fov(crate::camera::default_lens() / self.camera.lens);
        self.fov_readout = Some((self.drawing_view, now + READOUT_SECONDS));
        self.status = format!("{} (default)", readout(self.camera.lens));
    }

    /// This frame's Alt+Shift+wheel over the hovered view: changes the
    /// field of view and shows the readout. `true` while the gesture owns
    /// the wheel (Alt+Shift held, or the readout still up from it), so the
    /// wheel doesn't zoom as well (egui spreads a notch over a few frames).
    pub(super) fn fov_wheel(&mut self, ui: &Ui) -> bool {
        let (scroll, held, now) = ui.input(|i| {
            // egui's default line height for wheel lines.
            let line = 40.;
            let page = i.viewport_rect().height();
            let scroll: f32 = i
                .events
                .iter()
                .filter_map(|e| match e {
                    Event::MouseWheel {
                        unit,
                        delta,
                        modifiers,
                    } if modifiers.alt && modifiers.shift => {
                        // macOS turns Shift+wheel sideways; either axis counts.
                        let d = delta.x + delta.y;
                        Some(match unit {
                            MouseWheelUnit::Point => d,
                            MouseWheelUnit::Line => d * line,
                            MouseWheelUnit::Page => d * page,
                        })
                    }
                    _ => None,
                })
                .sum();
            (scroll, i.modifiers.alt && i.modifiers.shift, i.time)
        });
        if scroll != 0. {
            // The wheel zoom's speed and direction: rolling the way that
            // zooms in narrows the view.
            let factor = self.input_preferences.options.wheel_zoom_factor(scroll);
            if self.camera.change_fov(1. / factor) {
                self.fov_readout = Some((self.drawing_view, now + READOUT_SECONDS));
                self.status = readout(self.camera.lens);
            } else if self.camera.view != View::Perspective || self.camera.looks_through() {
                self.status = "Field of View: Perspective views only".into();
            }
        }
        let recent = self.fov_readout.is_some_and(|(view, until)| {
            view == self.drawing_view && until - now > READOUT_SECONDS - 0.3
        });
        held || scroll != 0. || recent
    }

    /// The readout at the top of the view whose field of view just changed.
    pub(super) fn draw_fov_readout(&self, painter: &Painter, r: Rect) {
        let Some((view, until)) = self.fov_readout else {
            return;
        };
        let now = painter.ctx().input(|i| i.time);
        if view != self.drawing_view || now > until {
            return;
        }
        painter.text(
            r.center_top() + vec2(0., 34.),
            Align2::CENTER_TOP,
            readout(self.camera.lens),
            FontId::proportional(14.),
            crate::theme::text(),
        );
        painter
            .ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(until - now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::DEFAULT_LENS;
    use crate::tests::{box_app, draw};
    use eframe::egui::{Context, Modifiers, Vec2};
    use glam::DVec3;

    const SIZE: Vec2 = vec2(1440., 900.);

    fn wheel(a: &mut Operandi, ctx: &Context, dy: f32, modifiers: Modifiers) {
        let at = a.viewport_rect.center() + vec2(0., 60.);
        draw(
            a,
            ctx,
            SIZE,
            vec![
                Event::PointerMoved(at),
                Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: vec2(0., dy),
                    modifiers,
                },
            ],
        );
        for _ in 0..10 {
            draw(a, ctx, SIZE, vec![]);
        }
    }

    /// The width on screen of one unit across the view at the orbit centre.
    fn centre_size(a: &Operandi) -> f32 {
        let (_, right, _, _) = a.camera.basis();
        let p = |x: f64| {
            a.camera
                .project(a.camera.target + right * x, a.viewport_rect)
                .unwrap()
                .0
                .x
        };
        p(1.) - p(0.)
    }

    #[test]
    fn the_default_lens_reads_about_58_degrees() {
        assert!((vertical_fov_degrees(DEFAULT_LENS) - 58.1).abs() < 0.1);
        assert_eq!(readout(DEFAULT_LENS), "Field of View 58° · 22 mm");
        assert!((vertical_fov_degrees(MIN_LENS) - 150.).abs() < 0.5);
        assert!((vertical_fov_degrees(MAX_LENS) - 5.).abs() < 0.1);
    }

    #[test]
    fn alt_shift_wheel_changes_the_field_of_view_and_keeps_the_centre() {
        let mut a = box_app();
        let ctx = Context::default();
        a.camera.view = View::Perspective;
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        let (target, distance, size) = (a.camera.target, a.camera.distance, centre_size(&a));
        let alt_shift = Modifiers::ALT | Modifiers::SHIFT;
        // Rolling the way that zooms in narrows the view.
        wheel(&mut a, &ctx, 200., alt_shift);
        assert!(a.camera.lens > DEFAULT_LENS * 1.2, "{}", a.camera.lens);
        let ratio = a.camera.lens / DEFAULT_LENS;
        assert!((a.camera.distance / distance - ratio).abs() < 1e-9);
        assert_eq!(a.camera.target, target);
        assert!((centre_size(&a) - size).abs() < 0.01);
        assert_eq!(a.status, readout(a.camera.lens));
        assert!(a.fov_readout.is_some());
        // And back the other way, wider.
        wheel(&mut a, &ctx, -400., alt_shift);
        assert!(a.camera.lens < DEFAULT_LENS);
        assert!((centre_size(&a) - size).abs() < 0.01);
        // A plain wheel still zooms and leaves the lens alone.
        let lens = a.camera.lens;
        let distance = a.camera.distance;
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        wheel(&mut a, &ctx, 100., Modifiers::NONE);
        assert_eq!(a.camera.lens, lens);
        assert!(a.camera.distance < distance);
    }

    fn middle(a: &mut Operandi, ctx: &Context, modifiers: Modifiers) {
        let pos = a.viewport_rect.center() + vec2(0., 60.);
        let press = |pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Middle,
            pressed,
            modifiers,
        };
        draw(a, ctx, SIZE, vec![Event::PointerMoved(pos), press(true)]);
        draw(a, ctx, SIZE, vec![press(false)]);
        draw(a, ctx, SIZE, vec![]);
    }

    #[test]
    fn alt_shift_middle_click_resets_the_field_of_view() {
        let mut a = box_app();
        let ctx = Context::default();
        a.camera.view = View::Perspective;
        for _ in 0..3 {
            draw(&mut a, &ctx, SIZE, vec![]);
        }
        let (distance, size) = (a.camera.distance, centre_size(&a));
        a.camera.change_fov(2.5);
        let alt_shift = Modifiers::ALT | Modifiers::SHIFT;
        middle(&mut a, &ctx, alt_shift);
        assert!(
            (a.camera.lens - DEFAULT_LENS).abs() < 1e-12,
            "{}",
            a.camera.lens
        );
        assert!((a.camera.distance - distance).abs() < 1e-9);
        assert!((centre_size(&a) - size).abs() < 0.01);
        assert_eq!(a.status, "Field of View 58° · 22 mm (default)");
        assert!(a.fov_readout.is_some());
        // A plain middle click leaves the lens alone.
        a.camera.change_fov(2.);
        let lens = a.camera.lens;
        middle(&mut a, &ctx, Modifiers::NONE);
        assert_eq!(a.camera.lens, lens);
        // Maya's preset pans with Alt+middle, so the press stays a pan.
        a.input_preferences.navigation = crate::preferences::NavigationPreset::Maya;
        assert!(!a.is_fov_reset_press(alt_shift));
        a.input_preferences.navigation = crate::preferences::NavigationPreset::Operandi;
        assert!(a.is_fov_reset_press(alt_shift));
        assert!(!a.is_fov_reset_press(Modifiers::ALT));
    }

    #[test]
    fn the_field_of_view_stops_at_its_limits_and_skips_other_views() {
        let mut c = Camera::default();
        for _ in 0..100 {
            c.change_fov(2.);
        }
        assert_eq!(c.lens, MAX_LENS);
        assert!(!c.change_fov(2.), "already at the limit");
        for _ in 0..100 {
            c.change_fov(0.5);
        }
        assert_eq!(c.lens, MIN_LENS);
        let mut top = Camera {
            view: View::Top,
            ..Camera::default()
        };
        assert!(!top.change_fov(2.));
        assert_eq!(top.lens, Camera::default().lens);
        let mut through = Camera {
            through: true,
            ..Camera::default()
        };
        assert!(!through.change_fov(2.));
        // The orbit centre stays put.
        let mut c = Camera {
            target: DVec3::new(1., 2., 3.),
            ..Camera::default()
        };
        c.change_fov(1.5);
        assert_eq!(c.target, DVec3::new(1., 2., 3.));
    }
}
