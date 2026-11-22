//! Preferences > Input > Remapping: wheel and drag zoom, tooltip delay and
//! list scrolling (spec preferences-navigation-2026-11-04). Saved with the
//! mouse preset in `input-preferences.json`. Values the spec has not
//! measured yet are marked *provisional* where they are defined.
use crate::prefs_file::PrefsFile;
use eframe::egui::{self, Event, RawInput, Vec2};
use serde::{Deserialize, Serialize};

/// Which drag direction zooms in during a drag-zoom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoomDragAxis {
    /// Default from BEHAVIOR-VIEWPORT-BUTTONS (VB-Q5, provisional): the
    /// header's Zoom button and the in-view drag-zoom share this setting.
    #[default]
    LeftToRight,
    RightToLeft,
    BottomToTop,
    TopToBottom,
}
impl ZoomDragAxis {
    pub const ALL: [Self; 4] = [
        Self::LeftToRight,
        Self::RightToLeft,
        Self::BottomToTop,
        Self::TopToBottom,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::LeftToRight => "Left to Right",
            Self::RightToLeft => "Right to Left",
            Self::BottomToTop => "Bottom to Top",
            Self::TopToBottom => "Top to Bottom",
        }
    }
    /// How far a drag of `delta` points zooms in (negative zooms out).
    pub fn zoom_in(self, delta: Vec2) -> f32 {
        match self {
            Self::LeftToRight => delta.x,
            Self::RightToLeft => -delta.x,
            Self::BottomToTop => -delta.y,
            Self::TopToBottom => delta.y,
        }
    }
}

/// Input Device: Mouse drags keep going past the screen edge (the
/// pointer is put back), Tablet drags stop at it; Hybrid treats tool drags
/// as a mouse and view navigation as a tablet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputDevice {
    /// Measured default.
    #[default]
    Mouse,
    Tablet,
    Hybrid,
}
impl InputDevice {
    pub const ALL: [Self; 3] = [Self::Mouse, Self::Tablet, Self::Hybrid];
    pub fn label(self) -> &'static str {
        match self {
            Self::Mouse => "Mouse",
            Self::Tablet => "Tablet",
            Self::Hybrid => "Hybrid",
        }
    }
}

pub const WHEEL_ACCELERATION_RANGE: std::ops::RangeInclusive<u32> = 10..=1000;
pub const TOOLTIP_DELAY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=5.0;
pub const TRACKBALL_SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.1..=10.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NavigationOptions {
    /// Wheel zoom speed in percent. Provisional default 100 (today's speed).
    pub wheel_acceleration: u32,
    /// Wheel zoom centres on the pointer. Provisional default on (today).
    pub wheel_zoom_at_cursor: bool,
    /// Perspective zoom (wheel, drag, Zoom In/Out keys) moves toward the
    /// surface under the pointer (or the view centre), taking the orbit
    /// centre along, so it never stalls short of the model. Off: zoom only
    /// shrinks the distance to the fixed orbit centre. Default on; no UI
    /// yet (Jason may drop it).
    pub zoom_to_surface: bool,
    /// Provisional default off.
    pub invert_wheel_zoom: bool,
    pub zoom_drag_axis: ZoomDragAxis,
    /// Reverse Pan Widget Direction: the view header's Pan button moves
    /// the view instead of grabbing the scene. In-view pan is unchanged.
    /// Provisional default off.
    pub reverse_pan_widget: bool,
    /// Seconds of hover before a tooltip shows. Measured: 0.8.
    pub tooltip_delay: f32,
    /// Reverse wheel scrolling in lists, panels and forms (not the 3D
    /// views, which zoom). Provisional default off.
    pub invert_wheel_scrolling: bool,
    /// Trackball Rotation: orbit turns freely about the view's own axes
    /// instead of around world up. Measured default off. A view can choose
    /// for itself (Viewport Properties) unless the override below is on.
    pub trackball: bool,
    /// Override Viewport-Level Trackball Option: every view follows the
    /// preference. Provisional default off.
    pub trackball_override: bool,
    /// Trackball orbit speed with a mouse. Provisional default 1.
    pub trackball_speed_mouse: f32,
    /// Same with a tablet (used once Input Device is Tablet). Provisional 1.
    pub trackball_speed_tablet: f32,
    /// Numeric Enter as Tab: the keypad's Enter moves to the next field.
    /// Provisional default off.
    pub keypad_enter_as_tab: bool,
    /// Open Dialogs under Mouse: dialogs open at the pointer instead of the
    /// middle of the window. Provisional default off.
    pub dialogs_under_mouse: bool,
    pub input_device: InputDevice,
}
impl Default for NavigationOptions {
    fn default() -> Self {
        Self {
            wheel_acceleration: 100,
            wheel_zoom_at_cursor: true,
            zoom_to_surface: true,
            invert_wheel_zoom: false,
            zoom_drag_axis: ZoomDragAxis::default(),
            reverse_pan_widget: false,
            tooltip_delay: 0.8,
            invert_wheel_scrolling: false,
            trackball: false,
            trackball_override: false,
            trackball_speed_mouse: 1.,
            trackball_speed_tablet: 1.,
            keypad_enter_as_tab: false,
            dialogs_under_mouse: false,
            input_device: InputDevice::Mouse,
        }
    }
}
impl NavigationOptions {
    pub fn valid(&self) -> bool {
        WHEEL_ACCELERATION_RANGE.contains(&self.wheel_acceleration)
            && TOOLTIP_DELAY_RANGE.contains(&self.tooltip_delay)
            && TRACKBALL_SPEED_RANGE.contains(&self.trackball_speed_mouse)
            && TRACKBALL_SPEED_RANGE.contains(&self.trackball_speed_tablet)
    }
    /// Whether orbit is a trackball turn in a view whose own choice is
    /// `view` (None: no choice of its own).
    pub fn trackball_in(&self, view: Option<bool>) -> bool {
        if self.trackball_override {
            self.trackball
        } else {
            view.unwrap_or(self.trackball)
        }
    }
    /// Whether view navigation treats the device as a tablet (Tablet and
    /// Hybrid).
    pub fn tablet_navigation(&self) -> bool {
        self.input_device != InputDevice::Mouse
    }
    /// The trackball speed for the device navigating.
    pub fn trackball_speed(&self) -> f32 {
        if self.tablet_navigation() {
            self.trackball_speed_tablet
        } else {
            self.trackball_speed_mouse
        }
    }
    /// Numeric Enter as Tab: turn the first `keypad` Enter presses of this
    /// frame (counted by the macOS key monitor, since egui reports both
    /// Enter keys alike) into Tab presses, and their releases with them.
    /// `held` remembers a converted press until its release arrives.
    pub fn keypad_enter_to_tab(&self, events: &mut [Event], mut keypad: usize, held: &mut bool) {
        if !self.keypad_enter_as_tab {
            return;
        }
        for event in events {
            if let Event::Key { key, pressed, .. } = event
                && *key == egui::Key::Enter
            {
                if *pressed && keypad > 0 {
                    keypad -= 1;
                    *key = egui::Key::Tab;
                    *held = true;
                } else if !*pressed && *held {
                    *key = egui::Key::Tab;
                    *held = false;
                }
            }
        }
    }
    /// Camera distance multiplier for a frame's wheel scroll. `scroll` is
    /// egui's smoothed delta, already reversed if list scrolling is
    /// inverted; the 3D views undo that so they only follow Invert Mouse
    /// Wheel Zoom.
    pub fn wheel_zoom_factor(&self, scroll: f32) -> f64 {
        let mut scroll = scroll as f64;
        if self.invert_wheel_scrolling {
            scroll = -scroll;
        }
        if self.invert_wheel_zoom {
            scroll = -scroll;
        }
        (-scroll * 0.002 * self.wheel_acceleration as f64 / 100.).exp()
    }
    /// Camera distance multiplier for a drag-zoom step of `delta` points.
    pub fn drag_zoom_factor(&self, delta: Vec2) -> f64 {
        (-self.zoom_drag_axis.zoom_in(delta) as f64 * 0.01).exp()
    }
    /// Reverse wheel events for lists when Invert Mouse Wheel Scrolling is
    /// on (before egui sees them).
    pub fn adjust_input(&self, input: &mut RawInput) {
        if !self.invert_wheel_scrolling {
            return;
        }
        for event in &mut input.events {
            if let Event::MouseWheel { delta, .. } = event {
                *delta = -*delta;
            }
        }
    }
    /// Apply the tooltip delay to egui's style (at launch and on change).
    pub fn apply_style(&self, ctx: &egui::Context) {
        if ctx.style().interaction.tooltip_delay != self.tooltip_delay {
            ctx.style_mut(|s| s.interaction.tooltip_delay = self.tooltip_delay);
        }
    }
}

impl super::Operandi {
    /// Preferences > Input > Remapping.
    pub(super) fn remapping_page(&mut self, ui: &mut egui::Ui) {
        let before = self.input_preferences;
        let n = &mut self.input_preferences.options;
        let d = NavigationOptions::default();
        use crate::form::{Unit, rows};
        let devices: Vec<_> = InputDevice::ALL.iter().map(|&x| (x, x.label())).collect();
        let axes: Vec<_> = ZoomDragAxis::ALL.iter().map(|&x| (x, x.label())).collect();
        rows::section(ui, "Remapping", false, |ui| {
            rows::choice_d(
                ui,
                "Input Device",
                &mut n.input_device,
                &d.input_device,
                &devices,
                "Mouse: view drags from the header buttons never meet the screen edge. Tablet: they stop there. Hybrid: tablet for navigation.",
            );
            rows::number_d(
                ui,
                "Tooltip Delay",
                &mut n.tooltip_delay,
                d.tooltip_delay,
                Unit::Suffix(" s"),
                TOOLTIP_DELAY_RANGE,
                "How long the pointer rests before a tooltip shows",
            );
            rows::number_d(
                ui,
                "Mouse Wheel Acceleration",
                &mut n.wheel_acceleration,
                d.wheel_acceleration,
                Unit::Percent,
                WHEEL_ACCELERATION_RANGE,
                "How much faster quick wheel turns zoom",
            );
            rows::check_d(
                ui,
                "Wheel Zoom at Mouse Cursor",
                &mut n.wheel_zoom_at_cursor,
                d.wheel_zoom_at_cursor,
                "The wheel zooms toward the pointer instead of the view centre",
            );
            rows::check_d(
                ui,
                "Invert Mouse Wheel Zoom",
                &mut n.invert_wheel_zoom,
                d.invert_wheel_zoom,
                "Turning the wheel the other way zooms in",
            );
            rows::choice_d(
                ui,
                "Zoom Drag Axis",
                &mut n.zoom_drag_axis,
                &d.zoom_drag_axis,
                &axes,
                "Which way a zoom drag moves",
            );
            rows::check_d(
                ui,
                "Reverse Pan Widget Direction",
                &mut n.reverse_pan_widget,
                d.reverse_pan_widget,
                "The view header's Pan button moves the view instead of the scene",
            );
            rows::check_d(
                ui,
                "Invert Mouse Wheel Scrolling",
                &mut n.invert_wheel_scrolling,
                d.invert_wheel_scrolling,
                "Lists scroll the other way with the wheel",
            );
            rows::check_d(
                ui,
                "Numeric Enter as Tab",
                &mut n.keypad_enter_as_tab,
                d.keypad_enter_as_tab,
                "The keypad's Enter moves to the next field",
            );
            rows::check_d(
                ui,
                "Open Dialogs under Mouse",
                &mut n.dialogs_under_mouse,
                d.dialogs_under_mouse,
                "Dialogs open at the pointer instead of mid-window",
            );
            rows::check_d(
                ui,
                "Trackball Rotation",
                &mut n.trackball,
                d.trackball,
                "Orbit turns freely, by where the drag starts in the view",
            );
            rows::check_d(
                ui,
                "Override Viewport-Level Trackball Option",
                &mut n.trackball_override,
                d.trackball_override,
                "Every view follows Trackball Rotation, whatever it chose",
            );
            rows::number_d(
                ui,
                "Trackball Speed Multiplier (Mouse)",
                &mut n.trackball_speed_mouse,
                d.trackball_speed_mouse,
                Unit::Suffix("×"),
                TRACKBALL_SPEED_RANGE,
                "How fast trackball orbit turns with a mouse",
            );
            rows::number_d(
                ui,
                "Trackball Speed Multiplier (Tablet)",
                &mut n.trackball_speed_tablet,
                d.trackball_speed_tablet,
                Unit::Suffix("×"),
                TRACKBALL_SPEED_RANGE,
                "How fast trackball orbit turns with a tablet",
            );
        });
        rows::note(
            ui,
            "Allow Modifier Chording and Invert Drag Scrolling are left out (PREF-Q6): Operandi has no mouse-button \
             chords to restrict, and its lists scroll by wheel or trackpad, which \
             Invert Mouse Wheel Scrolling already covers.",
        );
        if self.input_preferences.options.tooltip_delay != before.options.tooltip_delay {
            self.input_preferences.options.apply_style(ui.ctx());
        }
        crate::dialog_place::set_under_mouse(self.input_preferences.options.dialogs_under_mouse);
        if self.input_preferences != before
            && let Err(error) = self
                .input_preferences
                .save(&self.environment.paths().input_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    #[test]
    fn defaults_keep_todays_navigation_and_the_measured_tooltip_delay() {
        let d = NavigationOptions::default();
        assert_eq!(d.tooltip_delay, 0.8);
        // Scrolling up (positive) zooms in at today's rate.
        assert!((d.wheel_zoom_factor(100.) - (-0.2f64).exp()).abs() < 1e-12);
        // VB-Q5: dragging right zooms in by default (it was up).
        assert!(
            d.drag_zoom_factor(vec2(10., 0.)) < 1.,
            "drag right zooms in"
        );
        assert_eq!(d.drag_zoom_factor(vec2(0., -10.)), 1., "up/down ignored");
        assert!(!d.reverse_pan_widget);
        assert!(d.wheel_zoom_at_cursor);
        assert!(d.zoom_to_surface);
    }

    #[test]
    fn wheel_acceleration_and_inversion_change_the_zoom() {
        let fast = NavigationOptions {
            wheel_acceleration: 200,
            ..Default::default()
        };
        assert!((fast.wheel_zoom_factor(100.) - (-0.4f64).exp()).abs() < 1e-12);
        let inverted = NavigationOptions {
            invert_wheel_zoom: true,
            ..Default::default()
        };
        assert!(inverted.wheel_zoom_factor(100.) > 1., "scroll up zooms out");
        // Inverted list scrolling arrives reversed; the view undoes it.
        let lists = NavigationOptions {
            invert_wheel_scrolling: true,
            ..Default::default()
        };
        assert_eq!(
            lists.wheel_zoom_factor(-100.),
            NavigationOptions::default().wheel_zoom_factor(100.)
        );
    }

    #[test]
    fn zoom_drag_axis_picks_the_direction_that_zooms_in() {
        for (axis, inward) in [
            (ZoomDragAxis::LeftToRight, vec2(10., 0.)),
            (ZoomDragAxis::RightToLeft, vec2(-10., 0.)),
            (ZoomDragAxis::BottomToTop, vec2(0., -10.)),
            (ZoomDragAxis::TopToBottom, vec2(0., 10.)),
        ] {
            let o = NavigationOptions {
                zoom_drag_axis: axis,
                ..Default::default()
            };
            assert!(o.drag_zoom_factor(inward) < 1., "{axis:?}");
            assert!(o.drag_zoom_factor(-inward) > 1., "{axis:?}");
        }
    }

    #[test]
    fn invert_wheel_scrolling_reverses_wheel_events_only_when_on() {
        let wheel = || RawInput {
            events: vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(1., 5.),
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut input = wheel();
        NavigationOptions::default().adjust_input(&mut input);
        assert_eq!(input.events, wheel().events);
        NavigationOptions {
            invert_wheel_scrolling: true,
            ..Default::default()
        }
        .adjust_input(&mut input);
        assert!(matches!(
            input.events[0],
            Event::MouseWheel { delta, .. } if delta == vec2(-1., -5.)
        ));
    }

    #[test]
    fn files_saved_before_remapping_load_with_its_defaults() {
        let dir = std::env::temp_dir().join(format!("operandi-remap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("input-preferences.json");
        std::fs::write(&path, r#"{"version":1,"navigation":"Maya"}"#).unwrap();
        let p = crate::preferences::InputPreferences::load(&path);
        assert_eq!(p.navigation, crate::preferences::NavigationPreset::Maya);
        assert_eq!(p.options, NavigationOptions::default());
        let mut changed = p;
        changed.options = NavigationOptions {
            zoom_drag_axis: ZoomDragAxis::LeftToRight,
            tooltip_delay: 0.2,
            ..Default::default()
        };
        changed.save(&path).unwrap();
        assert_eq!(crate::preferences::InputPreferences::load(&path), changed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    // App routes: the options reach the views through real input events.
    use crate::tests::draw;
    use crate::{Command, Operandi};
    use eframe::egui::Context;
    fn scroll(a: &mut Operandi, ctx: &Context, dy: f32) {
        let p = a.viewport_rect.center();
        for events in [
            vec![Event::PointerMoved(p)],
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., dy),
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            draw(a, ctx, vec2(1440., 900.), events);
        }
        // Let egui's smoothed scrolling run out.
        for _ in 0..30 {
            draw(a, ctx, vec2(1440., 900.), vec![]);
        }
    }
    #[test]
    fn invert_wheel_zoom_reverses_the_view_zoom() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let start = a.camera.distance;
        scroll(&mut a, &ctx, 50.);
        let normal = a.camera.distance;
        assert!(normal < start, "scroll up zooms in");
        a.input_preferences.options.invert_wheel_zoom = true;
        scroll(&mut a, &ctx, 50.);
        assert!(a.camera.distance > normal, "inverted: scroll up zooms out");
    }
    fn drag_orbit(a: &mut Operandi, ctx: &Context, from: egui::Pos2, to: egui::Pos2) {
        let press = |pos, pressed| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::ALT,
        };
        let mut events = vec![Event::PointerMoved(from), press(from, true)];
        for k in 1..=8 {
            events.push(Event::PointerMoved(from + (to - from) * (k as f32 / 8.)));
        }
        events.push(press(to, false));
        for e in events {
            draw(a, ctx, vec2(1440., 900.), vec![e]);
        }
    }
    #[test]
    fn trackball_orbit_turns_freely_and_views_can_choose() {
        let mut a = Operandi::empty();
        a.run(Command::NewMesh);
        let ctx = Context::default();
        for _ in 0..2 {
            draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        }
        let c = a.viewport_rect.center();
        // Off (the default): a long vertical drag stops at the pole.
        drag_orbit(&mut a, &ctx, c, c + vec2(0., 400.));
        assert!(a.camera.pitch <= 1.5 + 1e-9);
        assert_eq!(a.camera.roll, 0.);
        // On: the same drag goes over the top, and a drag round the edge
        // rolls the view.
        a.input_preferences.options.trackball = true;
        let before = a.camera.basis().3;
        drag_orbit(&mut a, &ctx, c, c + vec2(0., 400.));
        let after = a.camera.basis().3;
        assert!(
            before.angle_between(after) > 0.5,
            "turned: {before} → {after}"
        );
        let edge = c + vec2(300., 0.);
        let roll = a.camera.roll;
        drag_orbit(&mut a, &ctx, edge, c + vec2(0., 300.));
        assert!((a.camera.roll - roll).abs() > 0.2, "edge drag rolls");
        // Orientation stays orthonormal.
        let (_, right, up, forward) = a.camera.basis();
        assert!(right.dot(up).abs() < 1e-9 && up.dot(forward).abs() < 1e-9);
        // A view that chose Off keeps two-axis orbit...
        a.display.settings.drawing.trackball = Some(false);
        assert!(!a.input_preferences.options.trackball_in(Some(false)));
        let pitch = a.camera.pitch;
        drag_orbit(&mut a, &ctx, c, c + vec2(0., 400.));
        assert!(a.camera.pitch <= 1.5 + 1e-9 && a.camera.pitch != pitch);
        // ...until the preference overrides every view.
        a.input_preferences.options.trackball_override = true;
        assert!(a.input_preferences.options.trackball_in(Some(false)));
        let o = NavigationOptions::default();
        assert!(o.trackball_in(Some(true)) && !o.trackball_in(None));
    }
    #[test]
    fn trackball_speed_scales_the_turn() {
        let r = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(800., 600.));
        let turn = |speed| {
            let mut c = crate::camera::Camera::default();
            let before = c.basis().3;
            c.trackball(r.center(), r.center() + vec2(40., 0.), r, speed);
            before.angle_between(c.basis().3)
        };
        assert!((turn(2.) / turn(1.) - 2.).abs() < 0.05);
    }
    #[test]
    fn keypad_enter_becomes_tab_only_when_on() {
        let key = |key, pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let frame = || vec![key(egui::Key::Enter, true), key(egui::Key::Enter, false)];
        let mut held = false;
        let mut events = frame();
        NavigationOptions::default().keypad_enter_to_tab(&mut events, 1, &mut held);
        assert_eq!(events, frame(), "off: Enter stays Enter");
        let on = NavigationOptions {
            keypad_enter_as_tab: true,
            ..Default::default()
        };
        let mut events = frame();
        on.keypad_enter_to_tab(&mut events, 0, &mut held);
        assert_eq!(events, frame(), "Return is not the keypad's Enter");
        let mut events = vec![key(egui::Key::Enter, true)];
        on.keypad_enter_to_tab(&mut events, 1, &mut held);
        assert_eq!(events, vec![key(egui::Key::Tab, true)]);
        assert!(held);
        // Its release, a frame later, goes with it.
        let mut events = vec![key(egui::Key::Enter, false)];
        on.keypad_enter_to_tab(&mut events, 0, &mut held);
        assert_eq!(events, vec![key(egui::Key::Tab, false)]);
        assert!(!held);
    }
    #[test]
    fn tooltip_delay_reaches_the_style() {
        let ctx = Context::default();
        NavigationOptions {
            tooltip_delay: 1.5,
            ..Default::default()
        }
        .apply_style(&ctx);
        assert_eq!(ctx.style().interaction.tooltip_delay, 1.5);
    }
}
