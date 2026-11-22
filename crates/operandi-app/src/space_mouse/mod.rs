//! 3D mice (stage 1: navigation). A SpaceMouse moves the view under the
//! pointer (the hovered view, which becomes the active one, as view keys
//! do: UIC-70): twist and tilt orbit about the orbit centre, sideways
//! pushes and lifts pan, pushing forward or pulling back zooms. Each
//! button can press a keyboard shortcut (`buttons.rs`).
//!
//! Input arrives through a [`source::Source`]: the 3DxWare driver on
//! macOS when it's installed, else raw HID. Only the
//! focused app reacts, since raw HID input also arrives while another app
//! is in front.
pub mod buttons;
// Devices are read only on macOS and Windows; elsewhere this code is unused.
#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
pub mod decode;
#[cfg(target_os = "macos")]
mod framework;
pub mod motion;
mod page;
pub mod prefs;
// Devices are read only on macOS and Windows; elsewhere this code is unused.
#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
pub mod source;

use crate::Operandi;
use decode::DeviceState;
use eframe::egui::Context;
pub use prefs::SpaceMousePreferences;
use source::{Event, Source};
use std::time::Instant;

/// A 60 Hz frame: the first step after rest, before there's a previous
/// frame to measure from.
const FIRST_STEP: f64 = 1. / 60.;

#[derive(Default)]
pub struct SpaceMouse {
    pub prefs: SpaceMousePreferences,
    source: Option<Box<dyn Source>>,
    /// Whether a source was started (tests never start one).
    started: bool,
    pub state: DeviceState,
    /// The open device's name.
    pub device: Option<String>,
    /// Why a found device can't be read.
    pub problem: Option<String>,
    /// When the view last moved, for the next step's length.
    last: Option<Instant>,
    /// Buttons that went down since the last frame (0-based).
    presses: Vec<usize>,
    /// How many buttons the device has shown so far (rows on the page).
    pub seen_buttons: usize,
    /// The button whose shortcut the 3D Mouse page is recording.
    pub recording: Option<usize>,
    /// The page used this frame's keys (so Esc doesn't also close
    /// Preferences).
    pub keys_taken: bool,
    /// Button presses for the next frame's input.
    pending: buttons::Pending,
}

impl SpaceMouse {
    /// One line for Preferences: which device, or why there's none.
    pub fn status(&self) -> String {
        if !self.prefs.enabled {
            return "Off".into();
        }
        if let Some(device) = &self.device {
            return format!("{device} connected");
        }
        if let Some(problem) = &self.problem {
            return problem.clone();
        }
        if cfg!(any(target_os = "macos", target_os = "windows")) {
            "No 3D mouse found".into()
        } else {
            "3D mice are not supported on this system yet".into()
        }
    }

    /// Take a source's event.
    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Connected(name) => {
                self.device = Some(name);
                self.problem = None;
                self.state = DeviceState::default();
            }
            Event::Disconnected => {
                self.device = None;
                self.state = DeviceState::default();
            }
            Event::State(state) => {
                let pressed = buttons::newly_pressed(self.state.buttons, state.buttons);
                self.presses.extend(pressed);
                if let Some(&last) = self.presses.iter().max() {
                    self.seen_buttons = self.seen_buttons.max(last + 1);
                }
                self.state = state;
            }
            Event::Problem(problem) => {
                self.device = None;
                self.problem = Some(problem);
                self.state = DeviceState::default();
            }
        }
    }

    /// Start or stop reading as Enabled changes.
    fn follow_enabled(&mut self, ctx: &Context) {
        if self.prefs.enabled && !self.started {
            self.started = true;
            if cfg!(not(test)) {
                self.source = source::start(ctx);
            }
        } else if !self.prefs.enabled && self.started {
            self.started = false;
            self.source = None;
            self.device = None;
            self.problem = None;
            self.state = DeviceState::default();
        }
    }

    /// How long the view's next step is: the time since the last one, or
    /// one frame after rest.
    fn step_length(&mut self, now: Instant) -> f64 {
        let dt = self
            .last
            .map_or(FIRST_STEP, |t| now.duration_since(t).as_secs_f64());
        self.last = Some(now);
        dt
    }
}

impl Operandi {
    /// Once per frame: take the device's input and move the hovered view.
    pub(super) fn tick_space_mouse(&mut self, ctx: &Context) {
        self.space_mouse.follow_enabled(ctx);
        while let Some(event) = self.space_mouse.source.as_mut().and_then(|s| s.try_next()) {
            self.space_mouse.handle(event);
        }
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true) || i.focused);
        self.space_mouse_buttons(ctx, focused);
        self.space_mouse_step(ctx, focused, Instant::now());
    }

    /// Move the hovered view for the device's current axes at `now`.
    pub(super) fn space_mouse_step(&mut self, ctx: &Context, focused: bool, now: Instant) {
        let axes = self.space_mouse.state.axes;
        if axes.is_zero() || !focused || !self.space_mouse.prefs.enabled {
            self.space_mouse.last = None;
            return;
        }
        let dt = self.space_mouse.step_length(now);
        let fixed = |camera: &crate::camera::Camera| motion::is_fixed(camera.view);
        if motion::step(axes, &self.space_mouse.prefs, dt, false).is_rest() {
            // Within the dead zone: keep the clock so leaving it is smooth.
            return;
        }
        if self.pointer_over_a_view(ctx) {
            self.activate_view(self.view_under_pointer(ctx));
        }
        // Worked out for the view it lands in: a fixed view only pans and
        // zooms, along other axes (motion::step).
        let mut step = motion::step(axes, &self.space_mouse.prefs, dt, fixed(&self.camera));
        if step.is_rest() {
            return;
        }
        // Push/pull heads for the surface at the view centre, as the Zoom
        // In/Out keys do, instead of stalling 3 cm from the orbit centre
        // (fix-stuck-zoom). Perspective only; elsewhere `apply` zooms.
        if step.zoom != 1. {
            let r = self.nav_rect();
            if self.zoom_toward_surface(r.center(), r, step.zoom) {
                step.zoom = 1.;
            }
        }
        // The puck takes over from a turntable.
        self.view_motion = None;
        let turns = step.yaw != 0. || step.pitch != 0. || step.roll != 0.;
        // Orbit Selected turns about the selection, as a mouse orbit does.
        let pivot =
            (turns && !self.camera.looks_through() && self.display.settings.drawing.orbit_selected)
                .then(|| self.orbit_pivot())
                .flatten();
        let before = self.camera.basis();
        motion::apply(&mut self.camera, step);
        if let Some(pivot) = pivot {
            crate::orbit_selected::turn_about(&mut self.camera, before, pivot);
        }
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::decode::RawAxes;
    use super::*;

    #[test]
    fn status_says_what_is_connected_or_why_not() {
        let mut m = SpaceMouse::default();
        assert!(m.status().contains("No 3D mouse") || m.status().contains("not supported"));
        m.handle(Event::Connected("SpaceMouse Compact".into()));
        assert_eq!(m.status(), "SpaceMouse Compact connected");
        m.handle(Event::State(DeviceState {
            axes: RawAxes {
                t: [1, 2, 3],
                r: [0; 3],
            },
            buttons: 1,
        }));
        m.handle(Event::Disconnected);
        assert_eq!(m.state, DeviceState::default(), "unplug: back at rest");
        assert!(m.device.is_none());
        m.handle(Event::Problem(
            "SpaceMouse found but could not be opened".into(),
        ));
        assert!(m.status().contains("could not be opened"));
        m.prefs.enabled = false;
        assert_eq!(m.status(), "Off");
    }

    #[test]
    fn button_shortcuts_are_set_and_cleared_through_pref_value() {
        // UIC-164: the Buttons rows and scripts take the same path.
        let mut a = Operandi::empty();
        a.execute("pref.value spaceMouse.buttons.button2 Cmd+F");
        assert_eq!(
            a.space_mouse.prefs.buttons.get(1),
            Some("Cmd+F"),
            "{}",
            a.status
        );
        a.execute("pref.value spaceMouse.buttons.button2 ?");
        assert_eq!(a.status, "Cmd+F");
        // A row's Clear, as the page sends it.
        let before = a.pref_snapshot(crate::preferences::Page::SpaceMouse);
        a.space_mouse.prefs.buttons.0[1].clear();
        a.pref_rows_through_commands(before);
        assert_eq!(a.space_mouse.prefs.buttons.get(1), None, "{}", a.status);
        a.execute("pref.value spaceMouse.buttons.button2 Hyper+Q");
        assert_eq!(a.space_mouse.prefs.buttons.get(1), None, "not a shortcut");
    }

    #[test]
    fn the_first_step_after_rest_is_one_frame() {
        let mut m = SpaceMouse::default();
        let t0 = Instant::now();
        assert_eq!(m.step_length(t0), FIRST_STEP);
        let t1 = t0 + std::time::Duration::from_millis(20);
        assert!((m.step_length(t1) - 0.02).abs() < 1e-9);
    }
}
