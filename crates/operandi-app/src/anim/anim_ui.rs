//! The app's side of animation (BEHAVIOR-TIME-KEYS TK-60..66,
//! BEHAVIOR-TIMELINE TL-70/71): the Input Map actions for the animation
//! keys, playback each frame, the status-light requests from property
//! rows, and the jobs only the app can run: Render Frame Range with
//! Cycles and the viewport Playblast.
//!
//! Every action runs one of core's `operandi.time.*` / `operandi.key.*`
//! lines (UIC-160), so a key, a button, a menu entry and a typed line do
//! the same thing.
use super::*;
use crate::command_handlers::Commands;
use operandi_core::animation::{self, AnimRequest, ChannelKey};
use operandi_core::command_registry::{self as table, Area};
use operandi_core::command_runtime::Call;
use std::time::Instant;

use crate::anim_keys::ACTIONS;

fn run_action(a: &mut Operandi, c: &Call<'_>) {
    if let Some(line) = crate::anim_keys::action_line(c.entry.name) {
        a.execute_line(line);
    }
}

const TIMELINE_ARGS: &[table::Arg] = &[table::opt(
    "open",
    table::ArgType::Choice(&["tall", "true", "false"]),
    table::Unit::None,
    None,
)];
const GRAPH_OPEN_ARGS: &[table::Arg] = &[table::opt(
    "open",
    table::ArgType::Choice(&["true", "false"]),
    table::Unit::None,
    None,
)];
const TOGGLE: table::ArgType = table::ArgType::Choice(&["toggle", "true", "false"]);
const GRAPH_VIEW_ARGS: &[table::Arg] = &[
    table::opt(
        "fit",
        table::ArgType::Choice(&["all", "selected", "time", "values", "visible"]),
        table::Unit::None,
        None,
    ),
    table::opt("normalise", TOGGLE, table::Unit::None, None),
    table::opt("stacked", TOGGLE, table::Unit::None, None),
    table::opt("isolate", TOGGLE, table::Unit::None, None),
    table::opt("velocity", TOGGLE, table::Unit::None, None),
    table::opt("shadow", TOGGLE, table::Unit::None, None),
    table::opt("animatedOnly", TOGGLE, table::Unit::None, None),
    table::opt("lock", TOGGLE, table::Unit::None, None),
    table::opt("link", TOGGLE, table::Unit::None, None),
    table::opt("boxScale", TOGGLE, table::Unit::None, None),
    table::opt("retime", TOGGLE, table::Unit::None, None),
    table::opt("speed", TOGGLE, table::Unit::None, None),
    table::opt("rows", table::ArgType::Int, table::Unit::None, None),
    table::opt("rowOffset", table::ArgType::Int, table::Unit::None, None),
];
const DIALOG_ARGS: &[table::Arg] = &[table::opt(
    "kind",
    table::ArgType::Choice(&["ranges", "settings", "render", "playblast"]),
    table::Unit::None,
    None,
)];

/// App commands: the key actions above, the Animate menu's dialogs.
pub(crate) fn register(r: &mut Commands) {
    let names: Vec<&str> = ACTIONS.iter().map(|(name, _, _)| *name).collect();
    crate::command_handlers::on_words(r, &names, run_action);
    r.add_host_command(
        table::script(
            "operandi.anim.timeline",
            Area::View,
            "Shows or hides the timeline strip (TK-70); open:tall shows it tall with markers and tracks, as the Animate layout does (TL-01).",
        )
        .args(TIMELINE_ARGS)
        .operandi()
        .ui(),
        |a, c| match c.rest.trim() {
            "open:tall" => a.timeline.open_tall(),
            "open:true" => a.timeline.open = true,
            "open:false" => a.timeline.open = false,
            _ => a.timeline.open = !a.timeline.open,
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.graph.open",
            Area::View,
            "Opens or closes the Graph Editor (GE-01).",
        )
        .args(GRAPH_OPEN_ARGS)
        .operandi()
        .ui(),
        |a, c| {
            a.graph.open = match c.rest.trim() {
                "open:true" => true,
                "open:false" => false,
                _ => !a.graph.open,
            }
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.anim.editors",
            Area::View,
            "Animation Editors: opens the tall timeline (track rows) and the Graph Editor (with its channel list) together (GE-01).",
        )
        .operandi()
        .ui(),
        |a, _| {
            a.timeline.open_tall();
            a.graph.open = true;
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.graph.view",
            Area::View,
            "The Graph Editor's view: fit (all, selected, time, values, visible; GE-21), Normalised, Stacked (GE-82), Isolate (GE-X6), Velocity (GE-81), Shadow, Animated Only and Lock (GE-11).",
        )
        .args(GRAPH_VIEW_ARGS)
        .operandi()
        .ui(),
        |a, c| a.graph_view(c.rest),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.anim.dialog",
            Area::View,
            "Opens an Animate menu dialog: kind:ranges (Time Ranges…), settings, render (Render Frame Range…) or playblast (Playblast…); OK runs one command (UIC-80, UIC-162).",
        )
        .args(DIALOG_ARGS)
        .operandi()
        .ui(),
        |a, c| {
            let kind = c.rest.trim().trim_start_matches("kind:");
            a.open_anim_dialog(kind);
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

/// An Animate menu dialog (UIC-80..97): its fields, and how OK builds its
/// one command line.
pub(crate) struct AnimDialog {
    kind: String,
    title: &'static str,
    /// (label, argument, value).
    fields: Vec<(&'static str, &'static str, String)>,
    /// The shared command dialog showing `fields` (#50: these were plain
    /// windows, against UIC-81..85). Time Settings has none: its rows
    /// apply as they change, so it keeps its own window.
    pub(crate) form: Option<form::Dialog>,
}

impl AnimDialog {
    /// The one command OK runs: the dialog's command and each non-empty
    /// field as `arg:value` (braced when it has spaces).
    pub(crate) fn command_line(&self) -> String {
        let command = match self.kind.as_str() {
            "ranges" => "operandi.time.settings",
            "render" => "operandi.render.frameRange",
            _ => "operandi.view.playblast",
        };
        let mut line = command.to_owned();
        for (_, arg, value) in &self.fields {
            let v = match &self.form {
                Some(form) => form.values.text(arg),
                None => value.as_str(),
            }
            .trim();
            if v.is_empty() {
                continue;
            }
            if v.contains(' ') {
                line.push_str(&format!(" {arg}:{{{v}}}"));
            } else {
                line.push_str(&format!(" {arg}:{v}"));
            }
        }
        line
    }
}

/// A running frame render or playblast.
pub(crate) struct Job {
    playblast: bool,
    frames: Vec<i64>,
    next: usize,
    folder: PathBuf,
    name: String,
    /// Render: what `render.finished` was when this frame's render began.
    started: Option<u64>,
    /// Playblast: the frame whose screenshot is on its way.
    shot: Option<i64>,
    scale: f32,
    open: bool,
    samples: Option<u32>,
    size: Option<[u32; 2]>,
    camera: Option<u64>,
    /// The time to go back to afterwards.
    restore: operandi_core::time_tick::Tick,
    written: Vec<PathBuf>,
    /// Playblast: the movie the frames become (TL-71), the frames then
    /// going in a hidden folder beside it.
    movie: Option<PathBuf>,
    fps: f64,
}

/// A playblast screenshot's tag.
struct PlayblastShot(i64);

/// Per-app animation UI state.
#[derive(Default)]
pub(crate) struct AnimUi {
    /// Playback's last tick and the frames owed since (TK-61).
    last_tick: Option<Instant>,
    owed: f64,
    pub(crate) job: Option<Job>,
    pub(crate) dialog: Option<AnimDialog>,
    /// The scene time last frame, and whether it differs this frame (a
    /// scrub): the viewport then draws moved items through a matrix.
    seen_time: Option<operandi_core::time_tick::Tick>,
    pub(crate) time_moved: bool,
    /// A playblast movie being encoded, and whether to open it.
    encoding: Option<(
        std::thread::JoinHandle<std::result::Result<PathBuf, String>>,
        bool,
    )>,
}

/// `name_0000.png` in `folder`.
pub(crate) fn frame_path(folder: &std::path::Path, name: &str, frame: i64) -> PathBuf {
    let sign = if frame < 0 { "-" } else { "" };
    folder.join(format!("{name}_{sign}{:04}.png", frame.abs()))
}

impl Operandi {
    fn open_anim_dialog(&mut self, kind: &str) {
        let s = &self.editor.scene;
        let rate = s.time.rate;
        let f = |t: operandi_core::time_tick::Tick| {
            format!("{}", animation::frames(rate, t).round() as i64)
        };
        let (r0, r1) = animation::current_range(s);
        let folder = self
            .job_folder(None)
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let (title, fields) = match kind {
            "ranges" => (
                "Time Ranges",
                vec![
                    ("Scene Start", "start", f(s.time.start)),
                    ("Scene End", "end", f(s.time.end)),
                    ("Current Start", "rangeStart", f(r0)),
                    ("Current End", "rangeEnd", f(r1)),
                ],
            ),
            "settings" => ("Time Settings", Vec::new()),
            "render" => (
                "Render Frame Range",
                vec![
                    ("First Frame", "start", f(r0)),
                    ("Last Frame", "end", f(r1)),
                    ("Folder", "folder", folder),
                    ("File Name", "name", "Render".into()),
                ],
            ),
            "playblast" => (
                "Playblast",
                vec![
                    ("First Frame", "start", f(r0)),
                    ("Last Frame", "end", f(r1)),
                    ("Size (1, 0.5, 0.25)", "scale", "1".into()),
                    ("Folder", "folder", folder),
                    ("File Name", "name", "Playblast".into()),
                ],
            ),
            other => {
                self.status = format!("No animation dialog {other}");
                return;
            }
        };
        let form = (kind != "settings").then(|| {
            let rows = fields
                .iter()
                .map(|(label, arg, value)| form::Row::text(arg, label, value).fresh())
                .collect();
            self.form_dialog(title, rows)
        });
        self.anim_ui.dialog = Some(AnimDialog {
            kind: kind.to_owned(),
            title,
            fields,
            form,
        });
    }

    fn anim_dialog(&mut self, ctx: &Context) {
        let Some(mut d) = self.anim_ui.dialog.take() else {
            return;
        };
        let done = if let Some(mut form) = d.form.take() {
            // UIC-81..85: the shared dialog, as every other command's.
            let outcome = self.show_form(ctx, &mut form);
            d.form = Some(form);
            match outcome {
                form::Outcome::Ok => Some(true),
                form::Outcome::Cancel => Some(false),
                _ => None,
            }
        } else {
            // Time Settings: its rows apply as they change.
            let mut open = true;
            egui::Window::new(d.title)
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| self.time_settings(ui));
            (!open).then_some(false)
        };
        match done {
            Some(true) => {
                let line = d.command_line();
                self.execute(&line);
            }
            Some(false) => {}
            None => self.anim_ui.dialog = Some(d),
        }
    }

    /// Once a frame, before drawing: playback, row-light requests, jobs.
    pub(super) fn anim_frame(&mut self, ctx: &Context) {
        for request in crate::form::rows::take_anim_requests() {
            self.execute(&request);
        }
        // RG-24's one-off hint, and Compensation kept in the preferences.
        if let Some(note) = self.editor.anim.note.take() {
            self.status = note;
        }
        if self.editor.rig_compensation != self.undo_autosave.rig_compensation {
            self.undo_autosave.rig_compensation = self.editor.rig_compensation;
            self.save_undo_autosave();
        }
        self.tick_playback(ctx);
        let requests = std::mem::take(&mut self.editor.anim.requests);
        for r in requests {
            self.start_job(r);
        }
        self.run_job(ctx);
        self.poll_encoding();
        self.anim_dialog(ctx);
        // The timeline's and graph editor's one-value dialogs.
        self.timeline_dialog(ctx);
        self.graph_editor_window(ctx);
        self.channel_links_window(ctx);
        self.setup_outline(ctx);
        self.schematic_window(ctx);
        self.skeleton_window(ctx);
    }

    /// SK-01: in Setup mode every 3D view gets a yellow outline.
    fn setup_outline(&self, ctx: &Context) {
        if !self.editor.scene.animation.setup || !crate::anim_switch::enabled() {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("setup-outline"),
        ));
        let yellow = crate::form::status::edited_yellow();
        for r in self
            .view_rects
            .values()
            .chain(std::iter::once(&self.viewport_rect))
        {
            if r.is_positive() {
                painter.rect_stroke(
                    r.shrink(1.),
                    0.,
                    egui::Stroke::new(2.0f32, yellow),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    /// TK-60/61/64: advance playback; no undo steps, and only the final
    /// time goes into the history when it stops.
    fn tick_playback(&mut self, ctx: &Context) {
        let t = self.editor.scene.time.current;
        self.anim_ui.time_moved = self
            .anim_ui
            .seen_time
            .replace(t)
            .is_some_and(|seen| seen != t);
        let Some(backwards) = self.editor.anim.playing else {
            if self.anim_ui.last_tick.take().is_some() {
                let frame = self.editor.scene.time.current_frame();
                self.execute(&format!("operandi.time.set frame:{frame}"));
            }
            return;
        };
        let now = Instant::now();
        let s = &self.editor.scene;
        let fps = s.time.rate.as_f64();
        let settings = s.animation.settings.clone();
        let due = match self.anim_ui.last_tick.replace(now) {
            None => 0.,
            Some(last) if settings.real_time => now.duration_since(last).as_secs_f64() * fps,
            Some(_) => 1.,
        };
        self.anim_ui.owed += due;
        let step = if settings.snap || !settings.real_time {
            self.anim_ui.owed.floor()
        } else {
            self.anim_ui.owed
        };
        if step > 0. {
            self.anim_ui.owed -= step;
            let s = &self.editor.scene;
            let start = self.editor.anim.play_start.unwrap_or(s.time.current);
            let (t, stop) = animation::playback_step(s, s.time.current, start, step, backwards);
            let _ = self.editor.set_current_time(t, true);
            if stop {
                self.editor.anim.playing = None;
            }
        }
        if self.editor.anim.playing.is_some() {
            ctx.request_repaint();
        }
    }

    fn job_folder(&self, folder: Option<String>) -> std::result::Result<PathBuf, String> {
        if let Some(f) = folder {
            return Ok(PathBuf::from(f));
        }
        if let Some(f) = self.project_folder(crate::project_files::RENDERS) {
            return Ok(f);
        }
        crate::render_ui::render_folder()
    }

    fn start_job(&mut self, request: AnimRequest) {
        let wants_movie = matches!(request, AnimRequest::Playblast { movie: true, .. });
        let (playblast, start, end, folder, name) = match &request {
            AnimRequest::Cancel => {
                if let Some(job) = self.anim_ui.job.take() {
                    let _ = self.editor.set_current_time(job.restore, false);
                    self.editor.anim.progress = None;
                    self.status = format!("Stopped after {} frames", job.next);
                }
                return;
            }
            AnimRequest::RenderRange {
                start,
                end,
                folder,
                name,
                ..
            } => (false, *start, *end, folder.clone(), name.clone()),
            AnimRequest::Playblast {
                start,
                end,
                folder,
                name,
                ..
            } => (true, *start, *end, folder.clone(), name.clone()),
        };
        if self.anim_ui.job.is_some() {
            self.status = "A frame render or playblast is already running".into();
            return;
        }
        if !playblast && self.render.busy() {
            self.status = "A render is already running".into();
            return;
        }
        let folder = match self.job_folder(folder) {
            Ok(f) => f,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        if let Err(e) = std::fs::create_dir_all(&folder) {
            self.status = format!("{}: {e}", folder.display());
            return;
        }
        let (scale, open, samples, size, camera) = match request {
            AnimRequest::Playblast { scale, open, .. } => (scale, open, None, None, None),
            AnimRequest::RenderRange {
                samples,
                size,
                camera,
                ..
            } => (1., false, samples, size, camera),
            AnimRequest::Cancel => unreachable!(),
        };
        self.editor.anim.playing = None;
        self.editor.anim.progress =
            Some((0, (end - start + 1) as usize, folder.display().to_string()));
        // TL-71: a movie, when asked for and ffmpeg is here; its frames go
        // in a hidden folder that goes once the movie is written.
        let movie = if wants_movie && start >= 0 {
            match ffmpeg() {
                Some(_) => Some(folder.join(format!("{name}.mp4"))),
                None => {
                    self.status = "No ffmpeg found: the playblast is a PNG sequence instead".into();
                    None
                }
            }
        } else {
            None
        };
        let frames_folder = match &movie {
            Some(_) => {
                let f = folder.join(format!(".{name}-frames"));
                let _ = std::fs::remove_dir_all(&f);
                if let Err(e) = std::fs::create_dir_all(&f) {
                    self.status = format!("{}: {e}", f.display());
                    return;
                }
                f
            }
            None => folder,
        };
        self.anim_ui.job = Some(Job {
            playblast,
            frames: (start..=end).collect(),
            next: 0,
            folder: frames_folder,
            name,
            started: None,
            shot: None,
            scale,
            open,
            samples,
            size,
            camera,
            restore: self.editor.scene.time.current,
            written: Vec::new(),
            movie,
            fps: self.editor.scene.time.rate.as_f64(),
        });
    }

    /// TL-70/71: one step of the running job each frame.
    fn run_job(&mut self, ctx: &Context) {
        let Some(mut job) = self.anim_ui.job.take() else {
            return;
        };
        ctx.request_repaint();
        if job.playblast {
            self.playblast_step(ctx, &mut job);
        } else {
            self.render_step(&mut job);
        }
        self.editor.anim.progress =
            Some((job.next, job.frames.len(), job.folder.display().to_string()));
        if job.next >= job.frames.len() {
            let _ = self.editor.set_current_time(job.restore, false);
            self.editor.anim.progress = None;
            self.status = format!(
                "{} {} frames to {}",
                if job.playblast {
                    "Playblast:"
                } else {
                    "Rendered"
                },
                job.written.len(),
                job.folder.display()
            );
            if let (true, Some(movie), Some(first)) = (
                job.playblast,
                job.movie.clone(),
                job.frames.first().copied(),
            ) && !job.written.is_empty()
            {
                // Encode off the UI thread, then open (TL-71).
                let (folder, name, fps) = (job.folder.clone(), job.name.clone(), job.fps);
                self.status = format!("Playblast: writing {}…", movie.display());
                self.anim_ui.encoding = Some((
                    std::thread::spawn(move || encode_movie(&folder, &name, first, fps, &movie)),
                    job.open,
                ));
                return;
            }
            if job.playblast
                && job.open
                && let Some(first) = job.written.first()
            {
                open_in_system(first);
            }
            return;
        }
        self.anim_ui.job = Some(job);
    }

    /// TL-71: a playblast is drawing its frames (the view hides its
    /// overlays meanwhile).
    pub(crate) fn playblasting(&self) -> bool {
        self.anim_ui.job.as_ref().is_some_and(|j| j.playblast)
    }

    /// The movie encode finished: report it and open it.
    fn poll_encoding(&mut self) {
        let done = self
            .anim_ui
            .encoding
            .as_ref()
            .is_some_and(|(h, _)| h.is_finished());
        if !done {
            return;
        }
        let Some((handle, open)) = self.anim_ui.encoding.take() else {
            return;
        };
        match handle.join() {
            Ok(Ok(movie)) => {
                self.status = format!("Playblast: {}", movie.display());
                if open {
                    open_in_system(&movie);
                }
            }
            Ok(Err(e)) => self.status = format!("Playblast: {e}"),
            Err(_) => self.status = "Playblast: the movie encoder stopped".into(),
        }
    }

    fn render_step(&mut self, job: &mut Job) {
        if self.render.busy() {
            return;
        }
        if let Some(started) = job.started.take() {
            if self.render.finished > started {
                if let Some(Err(e)) = &self.render.last {
                    self.status = format!("Frame {}: {e}", job.frames[job.next]);
                    job.next = job.frames.len();
                    return;
                }
                job.written
                    .push(frame_path(&job.folder, &job.name, job.frames[job.next]));
                job.next += 1;
            }
            if job.next >= job.frames.len() {
                return;
            }
        }
        let frame = job.frames[job.next];
        let t = self.editor.scene.time.rate.tick_of(frame);
        let _ = self.editor.set_current_time(t, true);
        self.render.overrides = Some(crate::render_ui::RenderOverrides {
            size: job.size,
            samples: job.samples,
            path: Some(frame_path(&job.folder, &job.name, frame)),
            camera: job.camera,
        });
        let before = self.render.finished;
        self.begin_render(self.render.pixels_per_point, false);
        self.render.overrides = None;
        if !self.render.busy() {
            self.status = format!("Frame {frame}: {}", self.status);
            job.next = job.frames.len();
            return;
        }
        job.started = Some(before);
    }

    fn playblast_step(&mut self, ctx: &Context, job: &mut Job) {
        // The screenshot of the frame drawn last time.
        let shot: Option<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                Event::Screenshot {
                    user_data, image, ..
                } => user_data
                    .data
                    .as_ref()
                    .and_then(|d| d.downcast_ref::<PlayblastShot>())
                    .filter(|s| Some(s.0) == job.shot)
                    .map(|_| image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let frame = job.shot.take().unwrap_or_default();
            let path = frame_path(&job.folder, &job.name, frame);
            match save_view(
                &image,
                self.viewport_rect,
                ctx.pixels_per_point(),
                job.scale,
                &path,
            ) {
                Ok(()) => job.written.push(path),
                Err(e) => {
                    self.status = format!("Playblast: {e}");
                    job.next = job.frames.len();
                    return;
                }
            }
            job.next += 1;
            return;
        }
        if job.shot.is_some() || job.next >= job.frames.len() {
            return;
        }
        // Pose this frame; the screenshot is taken of the next drawn frame.
        let frame = job.frames[job.next];
        let t = self.editor.scene.time.rate.tick_of(frame);
        let _ = self.editor.set_current_time(t, true);
        job.shot = Some(frame);
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            ViewportCommand::Screenshot(egui::UserData::new(PlayblastShot(frame))),
        );
    }

    /// Gives the next property row the status light of `channel` on the
    /// selected items (UIC-112) and makes it keyable (TK-20/24). With no
    /// keys anywhere, the plain None/Constant light by `non_default`.
    pub(crate) fn anim_row(&self, channel: &str, non_default: bool) {
        self.anim_row_names(&[channel.to_owned()], non_default);
    }

    /// UIC-112 for any property row, from the edit it makes (`command` at
    /// the shown value): every panel's shared row helpers call this, so a
    /// keyable row (light, camera, item channels) gets its light, its
    /// key click and its key menu. Rows that aren't keyable are left as
    /// they are.
    pub(crate) fn anim_row_for(&self, command: &Command, non_default: bool) {
        if let Some((_, names)) = animation::channel_of_command(&self.editor.scene, command) {
            self.anim_row_names(&names, non_default);
        }
    }

    /// [`Self::anim_row`] for a row that is several channels (a colour's
    /// components).
    pub(crate) fn anim_row_names(&self, names: &[String], non_default: bool) {
        // The animation switch: no keying on the rows while off.
        if !crate::anim_switch::enabled() {
            return;
        }
        let keys: Vec<ChannelKey> = names.iter().flat_map(|n| self.row_channels(n)).collect();
        self.anim_row_keys(&keys, non_default);
    }

    /// The status light and keying of the next row for exactly `keys`
    /// (the channel panel's rows, CP-50).
    pub(crate) fn anim_row_keys(&self, keys: &[ChannelKey], non_default: bool) {
        use crate::form::status::ChannelStatus;
        if !crate::anim_switch::enabled() {
            return;
        }
        let s = &self.editor.scene;
        let now = s.time.current;
        let at = self.editor.key_time();
        let state = |k: &ChannelKey| match s.animation.channels.get(k) {
            // RG-04: a rig piece drives it, or it feeds one.
            _ if operandi_core::rig::driven_by(s, k.item, &k.channel).is_some() => {
                ChannelStatus::Driven
            }
            Some(c) if !c.curve.is_empty() => {
                if animation::is_pending(s, k) {
                    ChannelStatus::Edited
                } else if c.curve.key_at(at).is_some() || c.curve.key_at(now).is_some() {
                    ChannelStatus::Key
                } else {
                    ChannelStatus::Animated
                }
            }
            _ => ChannelStatus::from_non_default(non_default),
        };
        let states: Vec<ChannelStatus> = keys.iter().map(state).collect();
        let status = match states.first() {
            None => ChannelStatus::from_non_default(non_default),
            Some(first) if states.iter().all(|x| x == first) => *first,
            Some(_) => ChannelStatus::Mixed,
        };
        crate::form::rows::set_status(status);
        crate::form::rows::anim_channel(crate::form::rows::AnimRow {
            selected: keys
                .iter()
                .any(|k| self.editor.anim.selected_channels.contains(k)),
            channels: keys.iter().map(ChannelKey::text).collect(),
        });
    }

    /// The channels a property row's status light stands for: `channel` of
    /// every selected item that has it (UIC-112, several items selected).
    pub(crate) fn row_channels(&self, channel: &str) -> Vec<ChannelKey> {
        self.editor
            .scene
            .selected_items()
            .into_iter()
            .map(|id| ChannelKey::new(id, channel))
            .filter(|k| animation::read(&self.editor.scene, k).is_some())
            .collect()
    }
}

/// The view's pixels from a window screenshot, scaled, as a PNG.
fn save_view(
    image: &egui::ColorImage,
    rect: Rect,
    ppp: f32,
    scale: f32,
    path: &std::path::Path,
) -> std::result::Result<(), String> {
    let [iw, ih] = image.size;
    let x0 = ((rect.min.x * ppp).round().max(0.) as usize).min(iw);
    let y0 = ((rect.min.y * ppp).round().max(0.) as usize).min(ih);
    let x1 = ((rect.max.x * ppp).round().max(0.) as usize).min(iw);
    let y1 = ((rect.max.y * ppp).round().max(0.) as usize).min(ih);
    let (w, h) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
    if w < 2 || h < 2 {
        return Err("The view has no visible area".into());
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in y0..y1 {
        for p in &image.pixels[y * iw + x0..y * iw + x1] {
            rgba.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
        }
    }
    let picture = image::RgbaImage::from_raw(w as u32, h as u32, rgba).ok_or("Wrong size")?;
    let picture = if scale < 1. {
        let (sw, sh) = (
            ((w as f32 * scale).round() as u32).max(1),
            ((h as f32 * scale).round() as u32).max(1),
        );
        image::imageops::resize(&picture, sw, sh, image::imageops::FilterType::Triangle)
    } else {
        picture
    };
    picture
        .save_with_format(path, image::ImageFormat::Png)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// ffmpeg, if installed (an outside program: Operandi doesn't link it).
pub(crate) fn ffmpeg() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    let exe = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    dirs.into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}

/// Encodes `<name>_0000.png`.. in `frames` (from frame `first`) into an
/// H.264 .mp4 at `fps`, then removes the frames.
pub(crate) fn encode_movie(
    frames: &std::path::Path,
    name: &str,
    first: i64,
    fps: f64,
    movie: &std::path::Path,
) -> std::result::Result<PathBuf, String> {
    let ffmpeg = ffmpeg().ok_or("no ffmpeg")?;
    let out = std::process::Command::new(ffmpeg)
        .args(["-y", "-loglevel", "error", "-framerate"])
        .arg(format!("{fps}"))
        .arg("-start_number")
        .arg(first.to_string())
        .arg("-i")
        .arg(frames.join(format!("{name}_%04d.png")))
        .args([
            "-vf",
            "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(movie)
        .output()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg: {} (the frames are kept in {})",
            String::from_utf8_lossy(&out.stderr).trim(),
            frames.display()
        ));
    }
    let _ = std::fs::remove_dir_all(frames);
    Ok(movie.to_path_buf())
}

/// TL-71: the system player opens the result.
fn open_in_system(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
