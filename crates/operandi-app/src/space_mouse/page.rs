//! Preferences > Input > 3D Mouse. Row edits run `pref.value
//! spaceMouse.<field>` (UIC-164), which saves; rows share one label edge
//! (UIC-111) and show their status circle against the default (UIC-110).
use super::prefs::{AxisLock, DEAD_ZONE_RANGE, MAX_BUTTONS, SPEED_RANGE, SpaceMousePreferences};
use crate::Operandi;
use crate::theme;
use crate::unavailable::{Gate, unless};

/// Why the page's rows are greyed.
const OFF: &str = "Turn Enabled on first";
use eframe::egui;

impl Operandi {
    pub(crate) fn space_mouse_page(&mut self, ui: &mut egui::Ui) {
        use crate::form::{Unit, rows};
        let status = self.space_mouse.status();
        let p = &mut self.space_mouse.prefs;
        let d = SpaceMousePreferences::default();
        rows::section(ui, "3D Mouse", false, |ui| {
            rows::value(ui, "Device", &status);
            rows::check_d(
                ui,
                "Enabled",
                &mut p.enabled,
                d.enabled,
                "Move the view under the pointer with a 3D mouse",
            );
            ui.gated_ui(unless(p.enabled, OFF), |ui| {
                rows::number_d(
                    ui,
                    "Speed",
                    &mut p.speed,
                    d.speed,
                    Unit::Suffix("×"),
                    SPEED_RANGE,
                    "How fast every motion moves the view",
                );
                rows::number_d(
                    ui,
                    "Dead Zone",
                    &mut p.dead_zone,
                    d.dead_zone,
                    Unit::Percent,
                    DEAD_ZONE_RANGE,
                    "How far the cap moves before the view does",
                );
                let locks: Vec<_> = AxisLock::ALL.iter().map(|&x| (x, x.label())).collect();
                rows::choice_d(
                    ui,
                    "Motions",
                    &mut p.axes,
                    &d.axes,
                    &locks,
                    "Which of the cap's motions move the view",
                );
                rows::check_d(
                    ui,
                    "Roll with Sideways Tilt",
                    &mut p.roll,
                    d.roll,
                    "Tilting the cap left or right banks the view",
                );
            });
        });
        ui.gated_ui(unless(p.enabled, OFF), |ui| {
            rows::section(ui, "Invert", false, |ui| {
                let i = &mut p.invert;
                let rows = [
                    ("Push Left/Right", &mut i.pan_x, "Pan the other way"),
                    ("Lift/Press", &mut i.pan_y, "Pan up and down the other way"),
                    ("Push/Pull", &mut i.zoom, "Zoom the other way"),
                    ("Tilt Forward/Back", &mut i.tilt, "Orbit over the other way"),
                    ("Twist", &mut i.twist, "Orbit around the other way"),
                    ("Tilt Sideways", &mut i.roll, "Roll the other way"),
                ];
                for (label, value, tip) in rows {
                    rows::check_d(ui, label, value, false, tip);
                }
            });
        });
        let enabled = p.enabled;
        ui.gated_ui(unless(enabled, OFF), |ui| {
            rows::section(ui, "Buttons", false, |ui| self.space_mouse_button_rows(ui));
        });
        rows::note(
            ui,
            "Twist and tilt the cap to orbit, push it sideways or lift it to pan, push it \
             away or pull it toward you to zoom. The view under the pointer moves. \
             Each button presses the shortcut set here; press a puck button to pick its \
             row. With 3DxWare installed Operandi takes the buttons itself, so 3DxWare's \
             own button settings don't apply inside Operandi.",
        );
    }

    /// One row per button: click, then press the shortcut (Esc cancels),
    /// like the Input Map (UIC-111 label edge, UIC-110 status circle).
    fn space_mouse_button_rows(&mut self, ui: &mut egui::Ui) {
        use crate::form::rows;
        // Take this frame's keys for the row being recorded.
        let m = &mut self.space_mouse;
        m.keys_taken = m.recording.is_some();
        if let Some(button) = m.recording {
            let captured =
                ui.input(|i| crate::input_map_editor::capture(&i.events, &self.native_numbers));
            use crate::input_map_editor::Captured;
            match captured {
                Some(Captured::Cancel) => m.recording = None,
                Some(Captured::Reserved) => {
                    self.status =
                        "Plain Space is reserved for navigation; press another key or add a modifier"
                            .into();
                }
                Some(Captured::Chord(chord)) => {
                    m.prefs.buttons.0[button] = chord;
                    m.recording = None;
                }
                None => {}
            }
            // Keep the keys away from text fields and focus travel.
            ui.ctx().memory_mut(|mem| {
                if let Some(id) = mem.focused() {
                    mem.surrender_focus(id);
                }
            });
        }
        let m = &mut self.space_mouse;
        let count = m
            .seen_buttons
            .max(m.prefs.buttons.assigned_through())
            .max(m.recording.map_or(0, |b| b + 1))
            .clamp(2, MAX_BUTTONS);
        for button in 0..count {
            let shortcut = m.prefs.buttons.0[button].clone();
            let recording = m.recording == Some(button);
            rows::status(!shortcut.is_empty());
            let label = format!("Button {}", button + 1);
            rows::row(ui, &label, |ui| {
                let clear = ui
                    .gated(
                        unless(!shortcut.is_empty(), "This button has no shortcut"),
                        egui::Button::new("Clear").small(),
                    )
                    .on_hover_text("No shortcut for this button");
                if clear.clicked() {
                    m.prefs.buttons.0[button].clear();
                }
                let text = if recording {
                    egui::RichText::new("Press keys…").color(theme::on_orange())
                } else if shortcut.is_empty() {
                    egui::RichText::new("—").color(theme::weak_text())
                } else {
                    egui::RichText::new(crate::input::display_chord(&shortcut))
                };
                let mut field =
                    egui::Button::new(text).min_size(egui::vec2(ui.available_width(), 0.));
                if recording {
                    field = field.fill(theme::orange());
                }
                let tip = if recording {
                    "Press the keys; Esc cancels"
                } else {
                    "Click, then press the shortcut this button presses"
                };
                if ui.add(field).on_hover_text(tip).clicked() {
                    m.recording = (!recording).then_some(button);
                }
            });
        }
    }
}
