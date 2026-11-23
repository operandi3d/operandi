//! Preferences > Defaults > UI (spec preferences-2026-11-06, Tier 2):
//! Popup Transparency, Popover Dismissal Behavior and Default Form Icon
//! Size, plus Interface Scale. Saved per user in `ui-preferences.json`.
use crate::prefs_file::PrefsFile;
use eframe::egui;
use serde::{Deserialize, Serialize};

/// Popover Dismissal Behavior for simple popovers (Viewport Properties).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dismissal {
    /// Each popover's own rule (measured default).
    #[default]
    Default,
    /// Stays until a click elsewhere.
    ClickOff,
    /// Closes when the pointer leaves it.
    RollOff,
}
impl Dismissal {
    pub const ALL: [Self; 3] = [Self::Default, Self::ClickOff, Self::RollOff];
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::ClickOff => "Force Click-Off",
            Self::RollOff => "Force Roll-Off",
        }
    }
}

/// Default Form Icon Size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IconSize {
    Small,
    /// Measured default.
    #[default]
    Normal,
    Large,
}
impl IconSize {
    pub const ALL: [Self; 3] = [Self::Small, Self::Normal, Self::Large];
    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Normal => "Normal",
            Self::Large => "Large",
        }
    }
    /// Provisional scales (PREF-Q22).
    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.8,
            Self::Normal => 1.,
            Self::Large => 1.25,
        }
    }
}

/// Interface Scale steps, in percent of the system's own display scale.
/// 100 follows the system exactly; the others are for a display whose
/// system scale leaves the interface too small or too large (a 4K screen
/// left at 100%, say).
pub const INTERFACE_SCALES: [u16; 8] = [75, 90, 100, 110, 125, 150, 175, 200];

thread_local! {
    /// The applied Interface Scale as a zoom factor (each frame reads it in
    /// [`sync_zoom`]). Per thread like the toolbox icon scale, so parallel
    /// tests don't see each other's choice.
    static ZOOM: std::cell::Cell<f32> = const { std::cell::Cell::new(1.) };
}

/// Make `ctx` draw at the chosen Interface Scale. egui multiplies its zoom
/// factor into `pixels_per_point`, so text, icons (they pick their picture
/// size from `pixels_per_point`), wires, gizmos and hit tests all follow
/// it. Window sizes are saved in the system's points
/// (`window_state::Tracker::observe_zoomed`).
pub fn sync_zoom(ctx: &egui::Context) {
    let zoom = zoom_factor();
    if (ctx.zoom_factor() - zoom).abs() > 1e-4 {
        ctx.set_zoom_factor(zoom);
    }
}
/// The applied Interface Scale (1 = the system's scale).
pub fn zoom_factor() -> f32 {
    ZOOM.with(std::cell::Cell::get)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiPreferences {
    version: u8,
    /// Popup Transparency, percent (0–90). Provisional: 0, opaque as before.
    pub popup_transparency: u8,
    pub dismissal: Dismissal,
    pub icon_size: IconSize,
    /// Behave like the original application where Operandi deliberately
    /// differs (original_compatible.rs). Off by default. The field name is
    /// the preference key: `pref.value ui.original_compatible`.
    pub original_compatible: bool,
    /// Slider Min/Max Labels: number fields with both limits show a range
    /// bar and their limits under them (form/range_labels.rs; an approved
    /// change to UIC-90). Off by default, which keeps every row as before.
    pub slider_min_max_labels: bool,
    /// Rendered Icon Outline (Display > Colors): which rendered icon set
    /// the interface uses (rendered_icons.rs). Stored with the UI page's
    /// values; the Colors page shows and resets it.
    pub rendered_icon_outline: crate::rendered_icons::OutlineChoice,
    /// Interface Scale, percent of the system scale ([`INTERFACE_SCALES`]).
    pub interface_scale: u16,
}
impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            popup_transparency: 0,
            dismissal: Dismissal::Default,
            icon_size: IconSize::Normal,
            original_compatible: false,
            slider_min_max_labels: false,
            rendered_icon_outline: Default::default(),
            interface_scale: 100,
        }
    }
}
impl UiPreferences {
    /// Make the look follow these (at launch and on change).
    pub fn apply(&self) {
        crate::theme::set_popup_transparency(self.popup_transparency);
        crate::toolbox::set_icon_scale(self.icon_size.scale());
        crate::original_compatible::set_on(self.original_compatible);
        crate::form::range_labels::set_on(self.slider_min_max_labels);
        crate::rendered_icons::set_choice(self.rendered_icon_outline);
        ZOOM.with(|z| z.set(f32::from(self.interface_scale) / 100.));
    }
    /// Force Click-Off: the popover stays until a click outside it
    /// (Default and Force Roll-Off close it when the pointer leaves).
    pub fn click_off(&self) -> bool {
        self.dismissal == Dismissal::ClickOff
    }
}
fn scale_label(percent: u16) -> String {
    if percent == 100 {
        "100% (System)".into()
    } else {
        format!("{percent}%")
    }
}
impl crate::prefs_file::PrefsFile for UiPreferences {
    const INVALID: &'static str = "Invalid UI preference";
    fn valid(&self) -> bool {
        self.version == 1
            && self.popup_transparency <= 90
            && INTERFACE_SCALES.contains(&self.interface_scale)
    }
}

impl super::Operandi {
    /// Preferences > Defaults > UI.
    pub(super) fn ui_page(&mut self, ui: &mut egui::Ui) {
        let before = self.ui_preferences;
        let p = &mut self.ui_preferences;
        let d = UiPreferences::default();
        use crate::form::{Unit, rows};
        rows::section(ui, "UI", false, |ui| {
            let labels: Vec<_> = INTERFACE_SCALES.iter().map(|&x| scale_label(x)).collect();
            let scales: Vec<_> = INTERFACE_SCALES
                .iter()
                .zip(&labels)
                .map(|(&x, l)| (x, l.as_str()))
                .collect();
            rows::choice_d(
                ui,
                "Interface Scale",
                &mut p.interface_scale,
                &d.interface_scale,
                &scales,
                "Size of the whole interface on top of the system's display scale. \
                 100% follows the system; raise it when text and icons are too \
                 small on a high-resolution display",
            );
            rows::number_d(
                ui,
                "Popup Transparency",
                &mut p.popup_transparency,
                d.popup_transparency,
                Unit::Percent,
                0..=90,
                "How see-through menus, popups and popovers are",
            );
            let dismissals: Vec<_> = Dismissal::ALL.iter().map(|&x| (x, x.label())).collect();
            rows::choice_d(
                ui,
                "Popover Dismissal Behavior",
                &mut p.dismissal,
                &d.dismissal,
                &dismissals,
                "When the Viewport Properties popover closes",
            );
            let sizes: Vec<_> = IconSize::ALL.iter().map(|&x| (x, x.label())).collect();
            rows::choice_d(
                ui,
                "Default Form Icon Size",
                &mut p.icon_size,
                &d.icon_size,
                &sizes,
                "Icon size in the toolbox",
            );
            rows::check_d(
                ui,
                "Original-Compatible Behavior",
                &mut p.original_compatible,
                d.original_compatible,
                "Where Operandi deliberately behaves differently from the original \
                 application, behave like the original instead",
            );
            rows::note(ui, &crate::original_compatible::summary());
            rows::check_d(
                ui,
                "Slider Min/Max Labels",
                &mut p.slider_min_max_labels,
                d.slider_min_max_labels,
                "Number fields with a lowest and highest value show a bar of where \
                 the value sits, with the two limits underneath",
            );
        });
        rows::note(
            ui,
            "Left out: Tidy Layouts (no doubled borders to tidy) and Tool Properties \
             layout (Operandi has one). Base Form Proficiency Level comes later.",
        );
        if self.ui_preferences == before {
            return;
        }
        self.ui_preferences.apply();
        ui.ctx().request_repaint();
        if let Err(error) = self
            .ui_preferences
            .save(&self.environment.paths().ui_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_preferences_apply_and_roundtrip() {
        let p = UiPreferences {
            popup_transparency: 40,
            dismissal: Dismissal::RollOff,
            icon_size: IconSize::Large,
            ..Default::default()
        };
        p.apply();
        assert_eq!(crate::toolbox::cell_height(), (34. * 1.25f32).round());
        assert!(crate::theme::popup_fill().a() < crate::theme::window().a());
        assert!(!p.click_off());
        let d = UiPreferences::default();
        assert!(!d.click_off());
        // The original-compatible switch and min/max labels ship off.
        assert!(!d.original_compatible);
        assert!(!d.slider_min_max_labels);
        assert!(!crate::form::range_labels::on());
        d.apply();
        assert_eq!(crate::toolbox::cell_height(), 34.);
        assert_eq!(crate::theme::popup_fill(), crate::theme::window());
        let path =
            std::env::temp_dir().join(format!("operandi-ui-prefs-{}.json", std::process::id()));
        p.save(&path).unwrap();
        assert_eq!(UiPreferences::load(&path), p);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn interface_scale_zooms_the_interface_and_rejects_odd_steps() {
        let ctx = egui::Context::default();
        let p = UiPreferences {
            interface_scale: 150,
            ..Default::default()
        };
        assert!(p.valid());
        // As the app does: synced inside a frame, taking hold from the next.
        let frame = || {
            let _ = ctx.run(egui::RawInput::default(), sync_zoom);
        };
        p.apply();
        frame();
        frame();
        assert_eq!(ctx.zoom_factor(), 1.5);
        // egui multiplies the zoom into the scale everything draws and
        // picks icon pictures at.
        assert_eq!(ctx.pixels_per_point(), 1.5);
        UiPreferences::default().apply();
        frame();
        frame();
        assert_eq!(ctx.zoom_factor(), 1.);
        assert_eq!(ctx.pixels_per_point(), 1.);
        assert_eq!(scale_label(100), "100% (System)");
        assert_eq!(scale_label(125), "125%");
        let odd = UiPreferences {
            interface_scale: 133,
            ..Default::default()
        };
        assert!(!odd.valid());
        // Older files without the field load at 100%.
        let old: UiPreferences = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert_eq!(old.interface_scale, 100);
    }
}
