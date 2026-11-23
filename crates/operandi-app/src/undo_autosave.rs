//! Preferences > Defaults > Application (undo limits) and Defaults >
//! Auto-Save (periodic scene backups), from UNDO-AUTOSAVE spec v1.
//!
//! Stored per user in `undo-autosave.json` beside the other preferences;
//! settings take effect as soon as they change. Values the spec has not
//! measured yet are marked *provisional* where they are defined.
use crate::file_chooser::{Choose, Purpose};
use crate::unavailable::Gate;
use eframe::egui::{self, Align2, Context, Id, vec2};
use operandi_core::{HistorySettings, Scene, persistence};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

/// Seconds the "Auto-save in N s" countdown runs before each backup.
pub const COUNTDOWN: Duration = Duration::from_secs(10);
/// Backups older than this are deleted from the Backup Directory on quit.
pub const MAX_BACKUP_AGE: Duration = Duration::from_secs(31 * 24 * 60 * 60);

pub const MAX_UNDO_RANGE: std::ops::RangeInclusive<u32> = 1..=10_000;
pub const SUB_COMMAND_RANGE: std::ops::RangeInclusive<u32> = 1..=10_000;
pub const INTERVAL_RANGE: std::ops::RangeInclusive<u32> = 1..=1_440;
pub const REVISION_RANGE: std::ops::RangeInclusive<u32> = 1..=100;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UndoAutosavePreferences {
    version: u8,
    pub max_undo_level: u32,
    pub group_selection: bool,
    pub max_sub_commands: u32,
    pub autosave: bool,
    pub interval_minutes: u32,
    /// `None` uses the default folder in the app's data folder.
    pub backup_dir: Option<PathBuf>,
    pub revisions: u32,
    /// Experimental: Animation (preview), the animation switch's
    /// preference (anim_switch.rs).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub animation_preview: bool,
    /// Experimental: Python scripting (preview), the scripting switch's
    /// preference (python_switch.rs).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub python_preview: bool,
}
impl Default for UndoAutosavePreferences {
    fn default() -> Self {
        let history = HistorySettings::default();
        Self {
            version: 1,
            max_undo_level: history.max_undo_level as u32,
            group_selection: history.group_selection,
            max_sub_commands: history.max_sub_commands,
            // Provisional (AS-Q1, not yet measured): off, so nothing is
            // written unless the user turns it on.
            autosave: false,
            // Provisional (AS-Q1).
            interval_minutes: 10,
            // Provisional (AS-Q1): `autosave` in the data folder.
            backup_dir: None,
            // Provisional (AS-Q1).
            revisions: 5,
            animation_preview: false,
            python_preview: false,
        }
    }
}
impl UndoAutosavePreferences {
    pub fn history(&self) -> HistorySettings {
        HistorySettings {
            max_undo_level: self.max_undo_level as usize,
            group_selection: self.group_selection,
            max_sub_commands: self.max_sub_commands,
        }
    }
    fn valid(&self) -> bool {
        self.version == 1
            && MAX_UNDO_RANGE.contains(&self.max_undo_level)
            && SUB_COMMAND_RANGE.contains(&self.max_sub_commands)
            && INTERVAL_RANGE.contains(&self.interval_minutes)
            && REVISION_RANGE.contains(&self.revisions)
            && self.backup_dir.as_ref().is_none_or(|d| d.is_absolute())
    }
    pub fn interval(&self) -> Duration {
        Duration::from_secs(u64::from(self.interval_minutes) * 60)
    }
    /// The folder backups go to, given the app's data folder.
    pub fn backup_dir(&self, data_root: &Path) -> PathBuf {
        self.backup_dir
            .clone()
            .unwrap_or_else(|| data_root.join("autosave"))
    }
    pub fn load(path: &Path) -> Self {
        fs::metadata(path)
            .ok()
            .filter(|m| m.len() <= 16 * 1024)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(Self::valid)
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid undo or auto-save preference",
            ));
        }
        let bytes = serde_json::to_vec(self).map_err(io::Error::other)?;
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
}

/// Backup file name: `<scene>.autosave-0001.opscene`. Provisional (AS-Q3):
/// revisions count up per scene name; the lowest numbers are the oldest.
pub fn backup_name(stem: &str, revision: u64) -> String {
    format!("{stem}.autosave-{revision:04}.opscene")
}
/// The scene name and revision of a backup file name, if it is one.
fn parse_backup(name: &str) -> Option<(&str, u64)> {
    let rest = name.strip_suffix(".opscene")?;
    let (stem, number) = rest.rsplit_once(".autosave-")?;
    (!number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
        .then(|| number.parse().ok().map(|n| (stem, n)))
        .flatten()
}
/// Revisions of `stem` in `dir`, oldest first.
fn revisions(dir: &Path, stem: &str) -> Vec<(u64, PathBuf)> {
    let mut found: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (s, n) = parse_backup(&name)?;
            (s == stem).then(|| (n, entry.path()))
        })
        .collect();
    found.sort();
    found
}

/// Write the next backup of `scene` and drop the oldest past `keep`.
/// Only files named as backups are ever replaced or removed, and never
/// `user_file`, the scene's own file.
pub fn write_backup(
    dir: &Path,
    stem: &str,
    keep: u32,
    scene: &Scene,
    user_file: Option<&Path>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Auto-save folder {}: {e}", dir.display()))?;
    let next = revisions(dir, stem).last().map_or(1, |(n, _)| n + 1);
    let path = dir.join(backup_name(stem, next));
    if user_file.is_some_and(|f| same_file(f, &path)) {
        return Err("Auto-save would replace the scene's own file; choose another folder".into());
    }
    persistence::save(&path, scene).map_err(|e| format!("Auto-save {}: {e}", path.display()))?;
    let all = revisions(dir, stem);
    let excess = all.len().saturating_sub(keep.max(1) as usize);
    for (_, old) in &all[..excess] {
        if user_file.is_none_or(|f| !same_file(f, old)) {
            let _ = fs::remove_file(old);
        }
    }
    Ok(path)
}
fn same_file(a: &Path, b: &Path) -> bool {
    a == b
        || fs::canonicalize(a)
            .ok()
            .zip(fs::canonicalize(b).ok())
            .is_some_and(|(a, b)| a == b)
}

/// Delete backups in `dir` last written before `now - MAX_BACKUP_AGE`.
/// Other files in the folder are left alone. Returns how many went.
pub fn remove_old_backups(dir: &Path, now: SystemTime) -> usize {
    let Some(cutoff) = now.checked_sub(MAX_BACKUP_AGE) else {
        return 0;
    };
    let mut removed = 0;
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let is_backup = entry
            .file_name()
            .to_str()
            .is_some_and(|n| parse_backup(n).is_some());
        let old = entry
            .metadata()
            .ok()
            .filter(|m| m.is_file())
            .and_then(|m| m.modified().ok())
            .is_some_and(|t| t < cutoff);
        if is_backup && old && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Auto-save timing for this run of the app (not saved).
#[derive(Default)]
pub struct Autosave {
    /// When the next countdown starts.
    next_due: Option<Instant>,
    /// When the running countdown ends.
    countdown_end: Option<Instant>,
    /// The countdown ended while a tool was open; back up once it closes.
    waiting: bool,
    /// The scene as last backed up, to skip unchanged scenes.
    last_backup: Option<Scene>,
    job: Option<std::thread::JoinHandle<Result<PathBuf, String>>>,
    /// (enabled, interval) the timer was started with.
    timer_for: Option<(bool, u32)>,
}

/// What a frame of the auto-save timer asks the app to do.
#[derive(Debug, PartialEq)]
pub enum Tick {
    Idle,
    /// Show the countdown with this many whole seconds left.
    Countdown(u64),
    /// Take a backup now.
    Backup,
}

impl Autosave {
    /// Advance the timer to `now`. `changed` says whether the scene differs
    /// from its saved file and the last backup; `ready` whether it holds only
    /// committed work.
    pub fn tick(
        &mut self,
        prefs: &UndoAutosavePreferences,
        now: Instant,
        changed: bool,
        ready: bool,
    ) -> Tick {
        let key = (prefs.autosave, prefs.interval_minutes);
        if self.timer_for != Some(key) {
            // Turning auto-save on or changing the interval restarts the wait.
            self.timer_for = Some(key);
            self.restart(now, prefs);
        }
        if !prefs.autosave {
            return Tick::Idle;
        }
        if self.job.is_some() {
            return Tick::Idle;
        }
        if let Some(end) = self.countdown_end {
            if now < end {
                return Tick::Countdown((end - now).as_secs_f64().ceil() as u64);
            }
            self.countdown_end = None;
            self.next_due = None;
            self.waiting = true;
        } else if self.next_due.is_some_and(|due| now >= due) {
            // Provisional (AS-Q2): an unchanged scene is not backed up again,
            // and gets no countdown.
            if changed {
                self.countdown_end = Some(now + COUNTDOWN);
                return Tick::Countdown(COUNTDOWN.as_secs());
            }
            self.restart(now, prefs);
        }
        if self.waiting && ready {
            self.waiting = false;
            self.restart(now, prefs);
            return if changed { Tick::Backup } else { Tick::Idle };
        }
        Tick::Idle
    }
    /// Cancel skips this backup only; the next comes a full interval later.
    pub fn cancel(&mut self, now: Instant, prefs: &UndoAutosavePreferences) {
        self.countdown_end = None;
        self.waiting = false;
        self.restart(now, prefs);
    }
    fn restart(&mut self, now: Instant, prefs: &UndoAutosavePreferences) {
        self.countdown_end = None;
        self.waiting = false;
        self.next_due = prefs.autosave.then(|| now + prefs.interval());
    }
    /// A backup is being written.
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn changed_since_backup(&self, scene: &Scene) -> bool {
        self.last_backup.as_ref() != Some(scene)
    }
    /// How long until the timer next needs a frame.
    fn wake_in(&self, now: Instant) -> Option<Duration> {
        if self.job.is_some() {
            return Some(Duration::from_millis(100));
        }
        if self.countdown_end.is_some() {
            return Some(Duration::from_millis(250));
        }
        self.next_due.map(|d| d.saturating_duration_since(now))
    }
    /// Wait for a backup in progress (tests).
    #[cfg(test)]
    pub fn finish(&mut self) -> Option<Result<PathBuf, String>> {
        self.job.take().map(|j| j.join().unwrap())
    }
}

impl super::Operandi {
    /// One frame of undo settings and the auto-save timer, including the
    /// countdown shown over the 3D view.
    pub(super) fn autosave_frame(&mut self, ctx: &Context) {
        self.editor.history = self.undo_autosave.history();
        if let Some(job) = self.autosave.job.take_if(|j| j.is_finished()) {
            let result = job
                .join()
                .unwrap_or_else(|_| Err("Auto-save failed".into()));
            self.autosave_result(result);
        }
        let now = Instant::now();
        let changed = self.dirty() && self.autosave.changed_since_backup(&self.editor.scene);
        let ready = self.editor.backup_ready();
        match self.autosave.tick(&self.undo_autosave, now, changed, ready) {
            Tick::Idle => {}
            Tick::Countdown(seconds) => {
                if self.autosave_countdown(ctx, seconds) {
                    self.autosave.cancel(now, &self.undo_autosave);
                    self.status = "Auto-save skipped".into();
                }
            }
            Tick::Backup => self.start_backup(),
        }
        if let Some(wait) = self.autosave.wake_in(now) {
            ctx.request_repaint_after(wait);
        }
    }
    /// An auto-save finished. A failure shows a dialog the first time, then
    /// only the status line until an auto-save succeeds.
    pub(super) fn autosave_result(&mut self, result: Result<PathBuf, String>) {
        match result {
            Ok(path) => {
                self.status = format!("Auto-saved {}", path.display());
                self.autosave_error_shown = false;
                // Changed images are backed up beside it (PNT-35).
                #[cfg(feature = "painting")]
                self.autosave_images(&path);
            }
            Err(e) => {
                self.status = e.clone();
                if !std::mem::replace(&mut self.autosave_error_shown, true) {
                    self.report_file_error(
                        "Auto-save",
                        format!(
                            "The auto-save backup couldn't be written.\n\n{e}\n\n\
                             This won't be shown again until an auto-save succeeds."
                        ),
                    );
                }
            }
        }
    }
    /// Draws the countdown; true when Cancel was pressed.
    fn autosave_countdown(&self, ctx: &Context, seconds: u64) -> bool {
        // Provisional (AS-Q2): a small panel at the top centre of the 3D view.
        let rect = self.viewport_rect;
        let anchor = if rect.is_finite() && rect.is_positive() {
            rect.center_top() + vec2(0., 12.)
        } else {
            ctx.content_rect().center_top() + vec2(0., 48.)
        };
        egui::Area::new(Id::new("autosave-countdown"))
            .order(egui::Order::Foreground)
            .pivot(Align2::CENTER_TOP)
            .fixed_pos(anchor)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Auto-save in {seconds} s"));
                            ui.button("Cancel").clicked()
                        })
                        .inner
                    })
                    .inner
            })
            .inner
    }
    pub(super) fn start_backup(&mut self) {
        let scene = self.editor.scene.clone();
        let dir = self
            .undo_autosave
            .backup_dir(self.environment.paths().data_root());
        // Several untitled scenes back up as "Untitled", "Untitled (2)" …
        let stem = self
            .path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.scene_label());
        let keep = self.undo_autosave.revisions;
        let user_file = self.path.clone();
        self.autosave.last_backup = Some(scene.clone());
        self.autosave.job = Some(std::thread::spawn(move || {
            write_backup(&dir, &stem, keep, &scene, user_file.as_deref())
        }));
    }
    /// On quit: let a backup in progress finish, then delete old backups.
    pub(super) fn autosave_quit(&mut self) {
        if let Some(job) = self.autosave.job.take() {
            let _ = job.join();
        }
        let dir = self
            .undo_autosave
            .backup_dir(self.environment.paths().data_root());
        remove_old_backups(&dir, SystemTime::now());
    }
    pub(super) fn save_undo_autosave(&mut self) {
        if let Err(error) = self
            .undo_autosave
            .save(&self.environment.paths().undo_autosave())
        {
            self.status = format!("Preference changed; could not save it: {error}");
        }
    }
    pub(super) fn set_backup_dir(&mut self, dir: PathBuf) {
        self.undo_autosave.backup_dir = Some(dir);
        self.save_undo_autosave();
    }

    /// Preferences > Defaults > Application.
    pub(super) fn application_page(&mut self, ui: &mut egui::Ui) {
        let before = self.undo_autosave.clone();
        let p = &mut self.undo_autosave;
        use crate::form::{Unit, rows};
        let d = UndoAutosavePreferences::default();
        rows::section(ui, "Undo", false, |ui| {
            // UIC-229 / HIS-15, HIS-16: the reference's names.
            rows::number_d(
                ui,
                "Max Undo Levels",
                &mut p.max_undo_level,
                d.max_undo_level,
                Unit::Count,
                MAX_UNDO_RANGE,
                "How many steps Undo can take back",
            );
            rows::check_d(
                ui,
                "Group Selection Commands",
                &mut p.group_selection,
                d.group_selection,
                "Selection changes in a row undo as one step",
            );
            // HIS-12: it also caps the lines a step keeps of a command
            // that runs commands (a batch, a preset), so it is never greyed.
            rows::number_d(
                ui,
                "Max Sub-Commands Recorded",
                &mut p.max_sub_commands,
                d.max_sub_commands,
                Unit::Count,
                SUB_COMMAND_RANGE,
                "How many selection changes one grouped step holds, and how many lines the \
                 Command History keeps under a command that runs commands",
            );
        });
        rows::note(
            ui,
            "A lower Max Undo Level drops the oldest steps when the next step is recorded.",
        );
        rows::section(ui, "Experimental", false, |ui| {
            rows::check_d(
                ui,
                "Animation (preview)",
                &mut p.animation_preview,
                d.animation_preview,
                "Show the animation tools: the Animate layout and menu, the timeline, the \
                 graph editor and keying. OPERANDI_ANIMATION=1 or 0 in the environment wins",
            );
            rows::check_d(
                ui,
                "Python scripting (preview)",
                &mut p.python_preview,
                d.python_preview,
                "Show the Script Console and System > Run Script…, and let scripts and the \
                 MCP bridge's python_run run. OPERANDI_PYTHON=1 or 0 in the environment wins",
            );
        });
        if self.undo_autosave != before {
            crate::anim_switch::set_preference(self.undo_autosave.animation_preview);
            crate::python_switch::set_preference(self.undo_autosave.python_preview);
            self.save_undo_autosave();
        }
        ui.add_space(8.);
        self.application_defaults_ui(ui);
    }
    /// Preferences > Defaults > Auto-Save.
    pub(super) fn autosave_page(&mut self, ui: &mut egui::Ui) {
        let before = self.undo_autosave.clone();
        let default_dir = self
            .undo_autosave
            .backup_dir(self.environment.paths().data_root());
        let mut browse = false;
        let p = &mut self.undo_autosave;
        use crate::form::{Unit, rows};
        let d = UndoAutosavePreferences::default();
        rows::section(ui, "Auto-Save", false, |ui| {
            rows::check_d(
                ui,
                "Enable",
                &mut p.autosave,
                d.autosave,
                "Save a backup of the scene every few minutes",
            );
            ui.gated_ui(
                crate::unavailable::unless(p.autosave, "Turn on Auto Save's Enable first"),
                |ui| {
                    rows::number_d(
                        ui,
                        "Time Interval",
                        &mut p.interval_minutes,
                        d.interval_minutes,
                        Unit::Suffix(" min"),
                        INTERVAL_RANGE,
                        "Minutes between backups",
                    );
                    rows::status(p.backup_dir.is_some());
                    rows::value(ui, "Backup Directory", &default_dir.display().to_string())
                        .on_hover_text("Where backups are saved");
                    // UIC-116: the field's buttons on the rows under it.
                    browse = rows::button(ui, "Browse…")
                        .on_hover_text("Pick the backup folder")
                        .clicked();
                    ui.gated_ui(
                        crate::unavailable::unless(
                            p.backup_dir.is_some(),
                            "Already using the default folder",
                        ),
                        |ui| {
                            if rows::button(ui, "Default")
                                .on_hover_text("Back to the default backup folder")
                                .clicked()
                            {
                                p.backup_dir = None;
                            }
                        },
                    );
                    rows::number_d(
                        ui,
                        "Number of Revisions",
                        &mut p.revisions,
                        d.revisions,
                        Unit::Count,
                        REVISION_RANGE,
                        "How many backups of each scene are kept",
                    );
                },
            );
        });
        rows::note(
            ui,
            "Backups go to their own folder; your scene file is never overwritten. \
             After a crash, open the latest backup from that folder. Backups older \
             than 31 days are deleted when Operandi quits.",
        );
        if self.undo_autosave != before {
            self.save_undo_autosave();
        }
        if browse
            && !self.choose(
                Choose::Folder {
                    folder: default_dir,
                    title: "Backup Directory",
                },
                Purpose::BackupDirectory,
            )
        {
            self.status = "Folder panel unavailable".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as D;

    fn temp_dir(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "operandi-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
    fn on() -> UndoAutosavePreferences {
        UndoAutosavePreferences {
            autosave: true,
            interval_minutes: 1,
            ..Default::default()
        }
    }

    #[test]
    fn preferences_roundtrip_and_fall_back_to_defaults() {
        let root = temp_dir("undo-autosave-prefs");
        let path = root.join("undo-autosave.json");
        let d = UndoAutosavePreferences::load(&path);
        assert_eq!(d, UndoAutosavePreferences::default());
        assert_eq!(d.max_undo_level, 100);
        let p = UndoAutosavePreferences {
            max_undo_level: 250,
            group_selection: true,
            max_sub_commands: 7,
            autosave: true,
            interval_minutes: 3,
            backup_dir: Some(root.join("b")),
            revisions: 2,
            ..Default::default()
        };
        p.save(&path).unwrap();
        assert_eq!(UndoAutosavePreferences::load(&path), p);
        let original = fs::read(&path).unwrap();
        for bad in [
            UndoAutosavePreferences {
                max_undo_level: 0,
                ..Default::default()
            },
            UndoAutosavePreferences {
                interval_minutes: 0,
                ..Default::default()
            },
            UndoAutosavePreferences {
                backup_dir: Some("relative".into()),
                ..Default::default()
            },
        ] {
            assert!(bad.save(&path).is_err());
        }
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::write(&path, "invalid").unwrap();
        assert_eq!(
            UndoAutosavePreferences::load(&path),
            UndoAutosavePreferences::default()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn countdown_starts_after_the_interval_and_backs_up_after_ten_seconds() {
        let p = on();
        let mut a = Autosave::default();
        let t0 = Instant::now();
        assert_eq!(a.tick(&p, t0, true, true), Tick::Idle);
        assert_eq!(a.tick(&p, t0 + D::from_secs(59), true, true), Tick::Idle);
        let t1 = t0 + D::from_secs(60);
        assert_eq!(a.tick(&p, t1, true, true), Tick::Countdown(10));
        assert_eq!(
            a.tick(&p, t1 + D::from_millis(3500), true, true),
            Tick::Countdown(7)
        );
        assert_eq!(a.tick(&p, t1 + COUNTDOWN, true, true), Tick::Backup);
        // The next comes a full interval later.
        let t2 = t1 + COUNTDOWN;
        assert_eq!(a.tick(&p, t2 + D::from_secs(59), true, true), Tick::Idle);
        assert_eq!(
            a.tick(&p, t2 + D::from_secs(60), true, true),
            Tick::Countdown(10)
        );
    }

    #[test]
    fn cancel_skips_one_backup_only() {
        let p = on();
        let mut a = Autosave::default();
        let t0 = Instant::now();
        a.tick(&p, t0, true, true);
        let t1 = t0 + D::from_secs(60);
        assert_eq!(a.tick(&p, t1, true, true), Tick::Countdown(10));
        a.cancel(t1 + D::from_secs(2), &p);
        assert_eq!(a.tick(&p, t1 + D::from_secs(30), true, true), Tick::Idle);
        let t2 = t1 + D::from_secs(62);
        assert_eq!(a.tick(&p, t2, true, true), Tick::Countdown(10));
        assert_eq!(a.tick(&p, t2 + COUNTDOWN, true, true), Tick::Backup);
    }

    #[test]
    fn disabled_unchanged_or_busy_scenes_wait() {
        let mut a = Autosave::default();
        let t0 = Instant::now();
        let off = UndoAutosavePreferences::default();
        assert!(!off.autosave, "provisional default is off");
        a.tick(&off, t0, true, true);
        assert_eq!(
            a.tick(&off, t0 + D::from_secs(3600), true, true),
            Tick::Idle
        );
        // Turning it on starts a fresh interval.
        let p = on();
        let t1 = t0 + D::from_secs(3600);
        assert_eq!(a.tick(&p, t1, true, true), Tick::Idle);
        // Unchanged scene: no countdown, the wait restarts.
        assert_eq!(a.tick(&p, t1 + D::from_secs(60), false, true), Tick::Idle);
        assert_eq!(a.tick(&p, t1 + D::from_secs(70), true, true), Tick::Idle);
        let t2 = t1 + D::from_secs(120);
        assert_eq!(a.tick(&p, t2, true, true), Tick::Countdown(10));
        // A tool is open when the countdown ends: back up once it closes.
        assert_eq!(a.tick(&p, t2 + COUNTDOWN, true, false), Tick::Idle);
        assert_eq!(
            a.tick(&p, t2 + COUNTDOWN + D::from_secs(5), true, false),
            Tick::Idle
        );
        assert_eq!(
            a.tick(&p, t2 + COUNTDOWN + D::from_secs(6), true, true),
            Tick::Backup
        );
    }

    #[test]
    fn backups_keep_the_newest_revisions_and_never_touch_other_files() {
        let dir = temp_dir("autosave-revisions");
        let scene = Scene::default();
        let user = dir.join("Robot.opscene");
        fs::write(&user, b"user scene").unwrap();
        fs::write(dir.join("notes.txt"), b"keep").unwrap();
        fs::write(dir.join(backup_name("Other", 1)), b"other scene").unwrap();
        let mut written = vec![];
        for _ in 0..5 {
            written.push(write_backup(&dir, "Robot", 3, &scene, Some(&user)).unwrap());
        }
        let kept: Vec<u64> = revisions(&dir, "Robot")
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(kept, vec![3, 4, 5]);
        assert_eq!(written[4], dir.join("Robot.autosave-0005.opscene"));
        assert_eq!(persistence::load(&written[4]).unwrap(), scene);
        assert_eq!(fs::read(&user).unwrap(), b"user scene");
        assert!(dir.join("notes.txt").exists());
        assert!(dir.join(backup_name("Other", 1)).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn quitting_removes_only_backups_older_than_31_days() {
        let dir = temp_dir("autosave-age");
        let scene = Scene::default();
        let fresh = write_backup(&dir, "A", 5, &scene, None).unwrap();
        let other = dir.join("A.opscene");
        fs::write(&other, b"user").unwrap();
        // Judge as if 40 days from now: the backup is old, the user file stays.
        let later = SystemTime::now() + D::from_secs(40 * 24 * 3600);
        assert_eq!(remove_old_backups(&dir, SystemTime::now()), 0);
        assert!(fresh.exists());
        assert_eq!(remove_old_backups(&dir, later), 1);
        assert!(!fresh.exists());
        assert!(other.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn backup_names_parse_back() {
        assert_eq!(
            parse_backup("Robot.autosave-0012.opscene"),
            Some(("Robot", 12))
        );
        assert_eq!(
            parse_backup("a.autosave-b.autosave-0001.opscene"),
            Some(("a.autosave-b", 1))
        );
        assert_eq!(parse_backup("Robot.opscene"), None);
        assert_eq!(parse_backup("Robot.autosave-.opscene"), None);
        assert_eq!(parse_backup("Robot.autosave-01x.opscene"), None);
    }

    #[test]
    fn app_backup_goes_to_the_backup_folder_not_the_scene_file() {
        let dir = temp_dir("autosave-app");
        let mut a = crate::Operandi::empty();
        a.run(crate::Command::NewMesh);
        let user = dir.join("Robot.opscene");
        assert!(a.save(user.clone()));
        let saved = fs::read(&user).unwrap();
        a.run(crate::Command::NewMesh);
        a.undo_autosave.backup_dir = Some(dir.join("backups"));
        a.start_backup();
        let path = a.autosave.finish().unwrap().unwrap();
        assert_eq!(path, dir.join("backups/Robot.autosave-0001.opscene"));
        assert_eq!(persistence::load(&path).unwrap(), a.editor.scene);
        assert_eq!(fs::read(&user).unwrap(), saved, "own file untouched");
        assert!(!a.autosave.changed_since_backup(&a.editor.scene));
        fs::remove_dir_all(dir).unwrap();
    }

    // App route: the Preferences value reaches the editor's undo history.
    #[test]
    fn max_undo_level_preference_limits_app_history() {
        use crate::tests::draw;
        let mut a = crate::Operandi::empty();
        let ctx = Context::default();
        a.undo_autosave.max_undo_level = 2;
        draw(&mut a, &ctx, vec2(1440., 900.), vec![]);
        assert_eq!(a.editor.history.max_undo_level, 2);
        for _ in 0..4 {
            a.run(crate::Command::NewMesh);
        }
        assert_eq!(a.editor.undo.len(), 2);
    }

    /// A quad grid with a UV map, `side` × `side` polygons.
    fn uv_grid(side: u32) -> operandi_core::Mesh {
        use glam::{DVec2, DVec3};
        let mut mesh = operandi_core::Mesh::default();
        for y in 0..=side {
            for x in 0..=side {
                mesh.vertices
                    .push(DVec3::new(f64::from(x) / 3., f64::from(y) / 7., 0.));
            }
        }
        let mut uvs = Vec::new();
        for y in 0..side {
            for x in 0..side {
                let a = y * (side + 1) + x;
                mesh.faces.push(operandi_core::Face {
                    vertices: vec![a, a + 1, a + side + 2, a + side + 1],
                    uv: None,
                    material: "Default".into(),
                    smooth: true,
                    part: None,
                    material_label: None,
                });
                let (u, v) = (f64::from(x) / 9., f64::from(y) / 11.);
                uvs.push(vec![Some(DVec2::new(u, v)); 4]);
            }
        }
        mesh.uv_maps.insert("Texture".into(), uvs);
        mesh
    }

    #[test]
    fn auto_save_writes_scenes_past_the_old_64_mib_limit() {
        let dir = temp_dir("autosave-big");
        let mut scene = Scene::default();
        scene.new_mesh().unwrap();
        scene.items[0].mesh = uv_grid(700).into();
        let path = write_backup(&dir, "Big", 5, &scene, None).unwrap();
        let size = fs::metadata(&path).unwrap().len();
        assert!(size > 64 << 20, "{size} bytes");
        let back = persistence::load(&path).unwrap();
        assert_eq!(back.items[0].mesh.faces.len(), 490_000);
        assert_eq!(back, scene);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_auto_save_shows_its_dialog_once_until_one_succeeds() {
        let dir = temp_dir("autosave-fails");
        let mut a = crate::Operandi::empty();
        a.run(crate::Command::NewMesh);
        // The backup folder's name is taken by a file: every write fails.
        let blocked = dir.join("blocked");
        fs::write(&blocked, b"x").unwrap();
        a.undo_autosave.backup_dir = Some(blocked.clone());
        let fail = |a: &mut crate::Operandi| {
            a.start_backup();
            let result = a.autosave.finish().unwrap();
            assert!(result.is_err());
            a.autosave_result(result);
        };
        fail(&mut a);
        let (title, message) = *a.file_error.take().expect("first failure: a dialog");
        assert_eq!(title, "Auto-save");
        assert!(message.contains("Auto-save folder"), "{message}");
        fail(&mut a);
        assert!(a.file_error.is_none(), "second failure: status line only");
        assert!(a.status.contains("Auto-save folder"), "{}", a.status);
        // One success, then the next failure is shown again.
        a.undo_autosave.backup_dir = Some(dir.join("backups"));
        a.start_backup();
        let ok = a.autosave.finish().unwrap();
        a.autosave_result(ok);
        assert!(a.status.starts_with("Auto-saved"), "{}", a.status);
        a.undo_autosave.backup_dir = Some(blocked);
        fail(&mut a);
        assert!(a.file_error.is_some());
        fs::remove_dir_all(dir).unwrap();
    }
}
