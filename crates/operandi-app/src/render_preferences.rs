//! Preferences > Rendering (spec preferences-2026-11-06, Tier 2): final
//! render threads, Block System Sleep While Rendering, and the Preview's
//! Start Paused and max samples. Saved per user in
//! `render-preferences.json` (the Preview's samples stay in its own file).
use crate::prefs_file::PrefsFile;
use crate::unavailable::Gate;
use eframe::egui;
use serde::{Deserialize, Serialize};

pub const THREAD_RANGE: std::ops::RangeInclusive<u32> = 1..=256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RenderPreferences {
    version: u8,
    /// Automatic Render Threads: every core. Provisional: on.
    pub automatic_threads: bool,
    /// Render Threads when not automatic. Provisional: 4.
    pub threads: u32,
    /// Keep the Mac awake while a final render runs. Provisional: on.
    pub block_sleep: bool,
    /// The Preview opens paused (as it has so far). Provisional: on.
    pub preview_start_paused: bool,
}
impl Default for RenderPreferences {
    fn default() -> Self {
        Self {
            version: 1,
            automatic_threads: true,
            threads: 4,
            block_sleep: true,
            preview_start_paused: true,
        }
    }
}
impl RenderPreferences {
    /// Cycles' thread count for a final render: 0 means every core.
    pub fn render_threads(&self) -> u32 {
        if self.automatic_threads {
            0
        } else {
            self.threads
        }
    }
}
impl crate::prefs_file::PrefsFile for RenderPreferences {
    const INVALID: &'static str = "Invalid render preference";
    fn valid(&self) -> bool {
        self.version == 1 && THREAD_RANGE.contains(&self.threads)
    }
}

/// Block System Sleep While Rendering: a `caffeinate` child that holds an
/// idle-sleep assertion for as long as it runs (macOS only).
#[derive(Default)]
pub struct Awake {
    child: Option<std::process::Child>,
}
impl Awake {
    /// Hold the assertion while `wanted`, let it go otherwise.
    pub fn hold(&mut self, wanted: bool) {
        if wanted == self.child.is_some() {
            return;
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        if cfg!(target_os = "macos") {
            // -i: no idle sleep; -w: also ends if Operandi exits.
            self.child = std::process::Command::new("/usr/bin/caffeinate")
                .args(["-i", "-w", &std::process::id().to_string()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok();
        }
    }
}
impl Drop for Awake {
    fn drop(&mut self) {
        self.hold(false);
    }
}

impl super::Operandi {
    /// Preferences > Rendering > Render Window (IV-12, IV-13, IV-14).
    fn render_window_preferences(&mut self, ui: &mut egui::Ui) {
        use crate::image_viewer::{MAX_SLOTS_RANGE, ViewTransform};
        let before = self.render.prefs.clone();
        let default_folder = self.environment.paths().recent_frames();
        let mut browse = false;
        let mut reset_folder = false;
        use crate::form::{Unit, rows};
        let d = crate::image_viewer::ViewerPreferences::default();
        let views: Vec<_> = ViewTransform::ALL.iter().map(|&t| (t, t.label())).collect();
        let p = &mut self.render.prefs;
        rows::section(ui, "Render Window", false, |ui| {
            let folder = p.folder(&default_folder).display().to_string();
            rows::status(p.recent_frames_folder.is_some());
            rows::value(ui, "Recent Frames Folder", &folder)
                .on_hover_text("Where the render history keeps its full-precision files");
            // UIC-116: the field's buttons on the rows under it; UIC-03:
            // it asks for a folder, so "...".
            browse = rows::button(ui, "Choose...")
                .on_hover_text("Pick the Recent Frames folder")
                .clicked();
            ui.gated_ui(
                crate::unavailable::unless(p.recent_frames_folder.is_some(), "No folder is set"),
                |ui| {
                    reset_folder = rows::button(ui, "Default")
                        .on_hover_text(default_folder.display().to_string())
                        .clicked();
                },
            );
            rows::number_d(
                ui,
                "Max Render Slots",
                &mut p.max_render_slots,
                d.max_render_slots,
                Unit::Count,
                MAX_SLOTS_RANGE,
                "Renders the history keeps; the oldest unlocked one goes first. Loaded images don't count.",
            );
            rows::check_d(
                ui,
                "Also Save Every Render to Pictures",
                &mut p.save_to_pictures,
                d.save_to_pictures,
                "Off: renders stay in the history; save one with Save Image... On: each render is also saved to ~/Pictures/Operandi Renders.",
            );
            rows::choice_d(
                ui,
                "Default View Colourspace",
                &mut p.default_view,
                &d.default_view,
                &views,
                "The Render window's display transform when it opens",
            );
        });
        if browse
            && !self.choose(
                crate::file_chooser::Choose::Folder {
                    folder: self.render.prefs.folder(&default_folder),
                    title: "Recent Frames Folder",
                },
                crate::file_chooser::Purpose::RecentFramesFolder,
            )
        {
            self.status = "Folder panel unavailable".into();
        }
        if reset_folder {
            self.set_recent_frames_folder(None);
        }
        let slots_changed = self.render.prefs.max_render_slots != before.max_render_slots;
        if self.render.prefs != before {
            self.save_viewer_preferences();
        }
        if slots_changed {
            // A lower cap takes effect at once.
            for gone in self.render.history.trim(self.render.prefs.max_render_slots) {
                self.render.viewer.forget(gone.id);
            }
            let _ = self.render.history.save_index();
        }
    }

    pub(crate) fn save_viewer_preferences(&mut self) {
        if let Err(error) = self
            .render
            .prefs
            .save(&self.environment.paths().image_viewer_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }

    /// Recent Frames Folder: the history moves to the new folder's (the
    /// old folder's files stay where they are).
    pub(crate) fn set_recent_frames_folder(&mut self, folder: Option<std::path::PathBuf>) {
        if self.render.busy() {
            self.status = "Wait for the render to finish before moving the history".into();
            return;
        }
        self.render.prefs.recent_frames_folder = folder;
        self.save_viewer_preferences();
        self.load_render_history();
        self.render.picture = None;
        self.render.outputs.clear();
        self.render.meta.clear();
        self.render.shown = None;
        self.render.texture_stale = true;
    }

    /// Once per frame: keep the Mac awake while a final render runs.
    pub(super) fn keep_awake_while_rendering(&mut self) {
        let wanted = self.render_preferences.block_sleep && self.render.busy();
        self.awake.hold(wanted);
    }
    /// Preferences > Rendering.
    pub(super) fn rendering_page(&mut self, ui: &mut egui::Ui) {
        let before = self.render_preferences;
        let samples_before = self.preview.saved.samples;
        let p = &mut self.render_preferences;
        use crate::form::{Unit, rows};
        let d = RenderPreferences::default();
        let samples = &mut self.preview.saved.samples;
        let samples_default = crate::preview::Saved::default().samples;
        rows::section(ui, "Final Rendering", false, |ui| {
            rows::check_d(
                ui,
                "Automatic Render Threads",
                &mut p.automatic_threads,
                d.automatic_threads,
                "Use every core for final renders",
            );
            ui.gated_ui(
                crate::unavailable::unless(
                    !p.automatic_threads,
                    "Turn off Automatic Render Threads first",
                ),
                |ui| {
                    rows::number_d(
                        ui,
                        "Render Threads",
                        &mut p.threads,
                        d.threads,
                        Unit::Count,
                        THREAD_RANGE,
                        "How many cores final renders use",
                    );
                },
            );
            rows::check_d(
                ui,
                "Block System Sleep While Rendering",
                &mut p.block_sleep,
                d.block_sleep,
                "The Mac stays awake while a final render runs",
            );
        });
        rows::section(ui, "Preview", false, |ui| {
            rows::check_d(
                ui,
                "Start Paused",
                &mut p.preview_start_paused,
                d.preview_start_paused,
                "The Preview opens paused; Play starts it",
            );
            rows::number_d(
                ui,
                "Max Samples",
                samples,
                samples_default,
                Unit::Count,
                1..=65_536,
                "Samples per pixel the Preview refines to (also in Preview Settings)",
            );
        });
        self.render_window_preferences(ui);
        crate::form::rows::note(
            ui,
            "Network rendering, the geometry cache and the other renderers' panels are \
             left out; baking comes later.",
        );
        if self.preview.saved.samples != samples_before {
            self.save_preview_settings();
        }
        if self.render_preferences != before
            && let Err(error) = self
                .render_preferences
                .save(&self.environment.paths().render_preferences())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threads_and_roundtrip() {
        let d = RenderPreferences::default();
        assert_eq!(d.render_threads(), 0, "every core");
        let p = RenderPreferences {
            automatic_threads: false,
            threads: 6,
            ..d
        };
        assert_eq!(p.render_threads(), 6);
        let path =
            std::env::temp_dir().join(format!("operandi-render-prefs-{}.json", std::process::id()));
        p.save(&path).unwrap();
        assert_eq!(RenderPreferences::load(&path), p);
        assert!(RenderPreferences { threads: 0, ..p }.save(&path).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn preview_opens_running_when_start_paused_is_off() {
        let mut a = crate::Operandi::empty();
        a.open_preview();
        assert!(!a.preview.autostart, "opens paused as before");
        a.preview.open = false;
        a.render_preferences.preview_start_paused = false;
        a.open_preview();
        assert!(a.preview.autostart);
    }
}
