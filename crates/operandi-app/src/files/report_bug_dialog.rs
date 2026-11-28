//! Help > Report Bug or Idea…'s dialog and screenshot (BUG-01..23).
//!
//! The bare command first asks for a picture of the main window (the
//! app's own framebuffer, `ViewportCommand::Screenshot`, so no OS
//! permission; BUG-05/06) and opens the dialog when it arrives: four
//! checkbox rows in the shared form dialog (UIC-98/99, UIC-87), ticks kept
//! across restarts (UIC-95). OK runs the full line
//! `operandi.help.reportBug screenshot:… toolState:… commands:… log:…`,
//! which is also what scripts give to skip the dialog (UIC-162). Cancel
//! writes nothing and opens nothing (BUG-03).
//!
//! On send the picture is saved in `<data folder>/reports/` (newest 10
//! kept), put on the clipboard and shown in Finder / File Explorer; the
//! report's text is built in bug_report_capture.rs.
use super::*;
use crate::bug_report_capture::{self as capture, ReportOptions};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Tag on our screenshot request, so the MCP bridge's captures and ours
/// never take each other's pictures.
#[derive(Clone, Copy)]
struct ReportShot;

/// How long to wait for the window picture before going on without it.
const SHOT_WAIT: Duration = Duration::from_millis(1500);
/// The dialog's title: the command's name without "…" (UIC-81).
const TITLE: &str = "Report Bug or Idea";

/// What to do once the picture is in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Then {
    /// Open the dialog (the bare command).
    Dialog,
    /// Send with these options (the full line, screenshot on).
    Send(ReportOptions),
}

#[derive(Default)]
enum Stage {
    #[default]
    Closed,
    /// The picture is asked for (or on its way).
    Capturing {
        then: Then,
        since: Instant,
        asked: bool,
    },
    Dialog {
        form: form::Dialog,
        shot: Option<Arc<ColorImage>>,
    },
    /// OK was pressed: the full line runs with this picture.
    Sending { shot: Option<Arc<ColorImage>> },
}

/// Report Bug's state between frames.
#[derive(Default)]
pub(super) struct ReportBug {
    stage: Stage,
    /// A picture to put on the clipboard next frame (needs the context).
    copy: Option<Arc<ColorImage>>,
}

impl ReportBug {
    /// Whether this counts as an open dialog (while capturing for the
    /// dialog, too, so the bare line is recorded by the line OK runs,
    /// CMD-14).
    pub(super) fn is_modal(&self) -> bool {
        match &self.stage {
            Stage::Closed => false,
            Stage::Capturing { then, .. } => *then == Then::Dialog,
            Stage::Dialog { .. } | Stage::Sending { .. } => true,
        }
    }
}

fn rows(defaults: ReportOptions) -> Vec<form::Row> {
    vec![
        // Says what OK does next (Jason 2026-11-23: a tester didn't know OK opens GitHub). Message row next to options, as in Delete Item; UIC-82 exception OK'd by Jason.
        form::Row::message("OK opens a new issue on GitHub in your browser. Describe the problem there, then press Submit. The ticked items below are filled in for you."),
        form::Row::check("screenshot", "Screenshot", defaults.screenshot).tip(
            "A picture of the Operandi window. Saved on your computer; it only goes into the report if you paste or drag it in.",
        ),
        form::Row::check("toolState", "Tool and Selection", defaults.tool_state).tip(
            "The current tool, selection mode, Action Center and snapping, and how many items and polygons the scene has.",
        ),
        form::Row::check("commands", "Recent Commands", defaults.commands).tip(
            "The last 20 commands you ran, including undone ones. Item names and text you typed can appear here.",
        ),
        form::Row::check("log", "Recent Log", defaults.log)
            .tip("The last 30 lines of the app's log."),
    ]
}

/// The full line for `o` (what OK runs and History shows).
pub(super) fn report_line(o: ReportOptions) -> String {
    format!(
        "{} screenshot:{} toolState:{} commands:{} log:{}",
        capture::REPORT_COMMAND,
        o.screenshot,
        o.tool_state,
        o.commands,
        o.log
    )
}

/// The four options from a line's words: `None` for a bare line, an error
/// when only some are given or a value isn't a boolean.
fn parse_options(rest: &str) -> std::result::Result<Option<ReportOptions>, String> {
    let words = command::tokens(rest).map_err(|e| e.to_string())?;
    if words.is_empty() {
        return Ok(None);
    }
    let names = ["screenshot", "toolState", "commands", "log"];
    let bound = operandi_core::command_args::bind(capture::REPORT_COMMAND, &words, &names)
        .map_err(|e| e.to_string())?;
    let mut values = [false; 4];
    for (i, value) in bound.iter().enumerate() {
        let Some(value) = value else {
            return Err(format!(
                "{}: give all of screenshot, toolState, commands and log, or none to open the dialog",
                capture::REPORT_COMMAND
            ));
        };
        values[i] =
            operandi_core::command_args::boolean(names[i], value).map_err(|e| e.to_string())?;
    }
    Ok(Some(ReportOptions {
        screenshot: values[0],
        tool_state: values[1],
        commands: values[2],
        log: values[3],
    }))
}

/// The first-run tip (BUG-24): a status line, nothing else on screen.
pub(super) const FIRST_RUN_TIP: &str =
    "Found a bug? Help > Report Bug or Idea… sends a report with a screenshot.";

/// One-time tips already shown, in `<data folder>/tips.json`; its
/// `reportBugShown` is the `tips.reportBugShown` flag (BUG-24).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct Tips {
    pub report_bug_shown: bool,
}
impl crate::prefs_file::PrefsFile for Tips {
    const INVALID: &'static str = "Invalid tips file";
    fn valid(&self) -> bool {
        true
    }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Vertex => "Vertices",
        Mode::Edge => "Edges",
        Mode::Polygon => "Polygons",
        Mode::Item => "Items",
    }
}

impl Operandi {
    /// At launch: the Report Bug tip on the status line, once per machine
    /// (existing users too), then never again (BUG-24).
    pub(super) fn first_run_tip(&mut self) {
        use crate::prefs_file::PrefsFile;
        let path = self.environment.paths().tips();
        let mut tips = Tips::load(&path);
        if tips.report_bug_shown {
            return;
        }
        self.status = FIRST_RUN_TIP.into();
        tips.report_bug_shown = true;
        if let Err(e) = tips.save(&path) {
            log_ring::log_line!("Tips not saved: {e}");
        }
    }

    /// `operandi.help.reportBug`: bare opens the dialog (after the
    /// picture); with all four options it sends at once (BUG-19).
    pub(super) fn report_bug_command(&mut self, rest: &str) {
        let options = match parse_options(rest) {
            Ok(o) => o,
            Err(e) => {
                self.status = e.clone();
                self.last_failure = Some(e);
                return;
            }
        };
        if let Stage::Sending { shot } = &mut self.report_bug.stage {
            // The dialog's OK running its line.
            let shot = shot.take();
            self.report_bug.stage = Stage::Closed;
            self.send_report(options.unwrap_or_default(), shot);
            return;
        }
        if !matches!(self.report_bug.stage, Stage::Closed) || self.has_modal() {
            let e = "Close the open dialog before Report Bug or Idea".to_owned();
            self.status = e.clone();
            self.last_failure = Some(e);
            return;
        }
        let then = match options {
            None => Then::Dialog,
            Some(o) if o.screenshot => Then::Send(o),
            Some(o) => {
                self.send_report(o, None);
                return;
            }
        };
        // The picture first, so it shows the window as it was (BUG-05).
        self.report_bug.stage = Stage::Capturing {
            then,
            since: Instant::now(),
            asked: false,
        };
    }

    /// The picture arrived (or never will): open the dialog or send.
    pub(super) fn report_shot_arrived(&mut self, shot: Option<Arc<ColorImage>>) {
        let Stage::Capturing { then, .. } = std::mem::take(&mut self.report_bug.stage) else {
            return;
        };
        match then {
            Then::Dialog => {
                let form = self.form_dialog(TITLE, rows(ReportOptions::default()));
                self.report_bug.stage = Stage::Dialog { form, shot };
            }
            Then::Send(o) => self.send_report(o, shot),
        }
    }

    /// One frame: ask for the picture, take it when it comes, show the
    /// dialog, and put a sent picture on the clipboard.
    pub(super) fn report_bug_frame(&mut self, ctx: &Context) {
        if let Some(image) = self.report_bug.copy.take() {
            // Fails quietly where the clipboard takes no images (BUG-08).
            ctx.copy_image((*image).clone());
        }
        let arrived = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                Event::Screenshot {
                    user_data, image, ..
                } if user_data
                    .data
                    .as_ref()
                    .is_some_and(|d| d.downcast_ref::<ReportShot>().is_some()) =>
                {
                    Some(image.clone())
                }
                _ => None,
            })
        });
        if let Stage::Capturing { since, asked, .. } = &mut self.report_bug.stage {
            if arrived.is_some() {
                self.report_shot_arrived(arrived);
            } else if since.elapsed() > SHOT_WAIT {
                self.report_shot_arrived(None);
            } else {
                if !*asked {
                    *asked = true;
                    ctx.send_viewport_cmd_to(
                        ViewportId::ROOT,
                        ViewportCommand::Screenshot(egui::UserData::new(ReportShot)),
                    );
                }
                ctx.request_repaint();
            }
        }
        let Stage::Dialog { form, .. } = &mut self.report_bug.stage else {
            return;
        };
        // `show_form` needs `self`; the form goes back unless answered.
        let mut form = std::mem::replace(form, form::Dialog::with_defaults(TITLE, Vec::new()));
        let outcome = self.show_form(ctx, &mut form);
        let Stage::Dialog { form: kept, shot } = &mut self.report_bug.stage else {
            return;
        };
        match outcome {
            form::Outcome::Ok => {
                let shot = shot.take();
                self.remember_form(&form);
                self.report_dialog_ok(&form.values, shot);
            }
            form::Outcome::Cancel => {
                // Nothing saved, nothing opened (BUG-03).
                self.report_bug.stage = Stage::Closed;
                self.status = "Report Bug or Idea canceled".into();
            }
            _ => *kept = form,
        }
    }

    /// OK: the dialog's ticks as the full line, recorded in History.
    fn report_dialog_ok(&mut self, values: &form::Values, shot: Option<Arc<ColorImage>>) {
        let o = ReportOptions {
            screenshot: values.bool("screenshot"),
            tool_state: values.bool("toolState"),
            commands: values.bool("commands"),
            log: values.bool("log"),
        };
        self.report_bug.stage = Stage::Sending { shot };
        self.execute(&report_line(o));
        self.report_bug.stage = Stage::Closed;
    }

    /// The Tool and Selection block's facts (BUG-11).
    fn report_tool_state(&self) -> capture::ToolState {
        let scene = &self.editor.scene;
        let sel = &scene.selection;
        let tool = match &self.editor.session.live {
            Some(live) => Some(live.tool.label().to_owned()),
            None => self.command_active().map(|i| command::IDS[i].0.to_owned()),
        };
        let selected = match sel.mode {
            Mode::Vertex => {
                sel.vertices.len() + sel.others.values().map(|c| c.vertices.len()).sum::<usize>()
            }
            Mode::Edge => {
                sel.edges.len() + sel.others.values().map(|c| c.edges.len()).sum::<usize>()
            }
            Mode::Polygon => {
                sel.faces.len() + sel.others.values().map(|c| c.faces.len()).sum::<usize>()
            }
            Mode::Item => usize::from(sel.item.is_some()) + sel.also.len(),
        };
        let mut s = capture::ToolState {
            tool,
            mode: mode_name(sel.mode).into(),
            action_center: self.editor.action_context.center.label().into(),
            snapping: self.snapping.enabled,
            selected,
            layout: self.layouts.current.clone(),
            ..Default::default()
        };
        for item in &scene.items {
            if item.is_mesh() {
                s.meshes += 1;
                s.polygons += item.mesh.read().as_read().polygon_count();
            } else if item.light.is_some() {
                s.lights += 1;
            } else if item.camera.is_some() {
                s.cameras += 1;
            } else {
                s.other_items += 1;
            }
        }
        s
    }

    /// The report's link for `o`, from the app's state now.
    pub(super) fn report_link(&self, o: ReportOptions) -> String {
        let env = bug_report::Environment::current(&self.gpu);
        let undone = |r: &operandi_core::CommandRecord| {
            self.editor.history.redo.iter().any(|step| {
                step.records
                    .0
                    .iter()
                    .any(|s| s.time == r.time && s.line == r.line)
            })
        };
        let live = self
            .history_tool()
            .map(|tool| format!("tool.set {tool} on"));
        let records: Vec<_> = self.history.log.records.iter().cloned().collect();
        let parts = capture::ReportParts {
            env: &env,
            tool: if o.tool_state {
                self.report_tool_state().lines()
            } else {
                Vec::new()
            },
            commands: if o.commands {
                capture::command_lines(&records, undone, live.as_deref())
            } else {
                Vec::new()
            },
            log: if o.log {
                log_ring::tail(bug_report::LOG_LINES)
            } else {
                Vec::new()
            },
        };
        capture::report_link(
            &parts,
            o,
            capture::platform(),
            capture::home_folder().as_deref(),
            bug_report::MAX_URL,
        )
    }

    /// Save the picture, open the form, copy and show the picture, and
    /// say what happened (BUG-07..09).
    fn send_report(&mut self, o: ReportOptions, shot: Option<Arc<ColorImage>>) {
        let p = capture::platform();
        let url = self.report_link(o);
        let mut saved = None;
        let mut problem = None;
        if o.screenshot {
            match &shot {
                Some(image) => {
                    let dir = self.environment.paths().reports();
                    let [w, h] = image.size;
                    let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                    let secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs());
                    match capture::save_screenshot(&dir, w as u32, h as u32, &rgba, secs) {
                        Ok(path) => saved = Some(path),
                        Err(e) => problem = Some(format!("the screenshot couldn't be saved ({e})")),
                    }
                }
                None => problem = Some("the window couldn't be captured".to_owned()),
            }
        }
        let opened = self.open_report_link(&url);
        if let Some(path) = &saved {
            if opened.is_ok() {
                // The link takes the clipboard when the browser fails.
                self.report_bug.copy = shot.clone();
            }
            self.reveal_report_file(path);
        }
        let mut status = match &opened {
            Ok(()) => "Opened a bug report in your browser.".to_owned(),
            Err(e) => {
                format!("Couldn't open the browser ({e}); the issue link is on the clipboard.")
            }
        };
        match (&saved, &opened) {
            (Some(_), Ok(())) => status.push_str(&format!(
                " The screenshot is on the clipboard and in {}: paste it into the report ({}) or drag it in.",
                p.files, p.paste
            )),
            (Some(_), Err(_)) => status.push_str(&format!(
                " The screenshot is in {}: drag it into the report.",
                p.files
            )),
            _ => {}
        }
        if let Some(problem) = problem {
            status.push_str(&format!(" No screenshot: {problem}."));
        }
        self.status = status;
    }

    /// The browser, or the clipboard when it fails (help_menu.rs).
    fn open_report_link(&mut self, url: &str) -> std::io::Result<()> {
        let opened = self.open_in_browser(url);
        if opened.is_err() {
            self.copy_next = Some(url.to_owned());
        }
        opened
    }

    /// Shows the saved picture in Finder / File Explorer (tests record it).
    fn reveal_report_file(&mut self, path: &std::path::Path) {
        self.revealed_path = Some(path.to_owned());
        if !cfg!(test) {
            let _ = bug_report::reveal(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Operandi {
        // Its own data folder, so reports/ starts empty.
        Operandi::empty()
    }
    fn shot(w: usize, h: usize) -> Arc<ColorImage> {
        Arc::new(ColorImage::new(
            [w, h],
            vec![Color32::from_rgb(10, 20, 30); w * h],
        ))
    }
    fn reports(a: &Operandi) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(a.environment.paths().reports())
            .map(|d| d.filter_map(|e| Some(e.ok()?.path())).collect())
            .unwrap_or_default()
    }

    #[test]
    fn the_report_bug_tip_shows_once_per_machine() {
        let mut a = app();
        let path = a.environment.paths().tips();
        assert!(!path.exists());
        a.first_run_tip();
        assert_eq!(a.status, FIRST_RUN_TIP);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"reportBugShown\":true"), "{text}");
        // The next launch on this machine: no tip.
        let mut b = Operandi::empty_at(a.environment.paths().data_root().to_owned());
        b.status = "Ready".into();
        b.first_run_tip();
        assert_eq!(b.status, "Ready");
    }

    #[test]
    fn the_full_line_opens_no_dialog_and_writes_no_file() {
        let mut a = app();
        a.execute("operandi.help.reportBug screenshot:false toolState:true commands:true log:true");
        assert!(!a.report_bug.is_modal() && matches!(a.report_bug.stage, Stage::Closed));
        let url = a.opened_url.take().expect("the form opens at once");
        assert!(url.contains("issues/new"), "{url}");
        assert!(reports(&a).is_empty());
        assert!(a.report_bug.copy.is_none(), "nothing for the clipboard");
        assert!(a.revealed_path.is_none());
        assert!(a.status.starts_with("Opened a bug report"), "{}", a.status);
        // Part of the options is refused; so is a non-boolean.
        a.execute("operandi.help.reportBug screenshot:false");
        assert!(
            a.opened_url.is_none() && a.status.contains("give all of"),
            "{}",
            a.status
        );
        a.execute("operandi.help.reportBug screenshot:maybe toolState:1 commands:on log:off");
        assert!(a.opened_url.is_none(), "{}", a.status);
    }

    #[test]
    fn bare_line_captures_then_asks_and_cancel_sends_nothing() {
        let mut a = app();
        a.execute("operandi.help.reportBug");
        assert!(matches!(
            a.report_bug.stage,
            Stage::Capturing {
                then: Then::Dialog,
                ..
            }
        ));
        assert!(a.has_modal(), "the capture counts as the dialog opening");
        assert!(a.opened_url.is_none(), "nothing opens before OK (BUG-01)");
        a.report_shot_arrived(Some(shot(8, 6)));
        let Stage::Dialog { form, shot: held } = &a.report_bug.stage else {
            panic!("the dialog opens once the picture is in");
        };
        assert_eq!(form.title, TITLE);
        assert_eq!(
            held.as_ref().unwrap().size,
            [8, 6],
            "held in memory, not saved"
        );
        let keys: Vec<_> = form.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(
            keys,
            ["message", "screenshot", "toolState", "commands", "log"]
        );
        assert!(
            form.rows[0].label.contains("GitHub"),
            "the top line says OK opens GitHub"
        );
        assert!(
            ["screenshot", "toolState", "commands", "log"]
                .iter()
                .all(|k| form.values.bool(k))
        );
        assert!(reports(&a).is_empty(), "nothing written before OK (BUG-05)");
        // Cancel.
        let ctx = Context::default();
        let _ = ctx.run(
            RawInput {
                events: vec![Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| a.report_bug_frame(ctx),
        );
        assert!(matches!(a.report_bug.stage, Stage::Closed));
        assert!(a.opened_url.is_none() && reports(&a).is_empty());
    }

    #[test]
    fn ok_saves_the_picture_copies_it_reveals_it_and_records_the_line() {
        let mut a = app();
        a.execute_from(operandi_core::Source::Menu, "operandi.help.reportBug");
        a.report_shot_arrived(Some(shot(40, 30)));
        let Stage::Dialog { form, .. } = &mut a.report_bug.stage else {
            panic!("dialog");
        };
        form.values.set("log", form::Value::Bool(false));
        let values = form.values.clone();
        let Stage::Dialog { shot, .. } = &mut a.report_bug.stage else {
            unreachable!()
        };
        let shot = shot.take();
        a.report_dialog_ok(&values, shot);
        assert!(matches!(a.report_bug.stage, Stage::Closed));
        let url = a.opened_url.take().expect("OK opens the form");
        assert!(!url.contains("Recent%20log"), "Recent Log unticked");
        let files = reports(&a);
        assert_eq!(files.len(), 1, "{files:?}");
        let png = image::open(&files[0]).unwrap();
        assert_eq!((png.width(), png.height()), (40, 30), "full size");
        assert_eq!(a.revealed_path.as_ref(), Some(&files[0]));
        assert!(
            a.report_bug.copy.is_some(),
            "picture queued for the clipboard"
        );
        assert!(a.status.contains("clipboard"), "{}", a.status);
        // History holds the full line once (UIC-164), with the menu as
        // its source, and the report leaves itself out.
        let lines: Vec<_> = a
            .history
            .log
            .records
            .iter()
            .filter(|r| r.line.starts_with("operandi.help.reportBug"))
            .collect();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(
            lines[0].line,
            "operandi.help.reportBug screenshot:true toolState:true commands:true log:false"
        );
        assert_eq!(lines[0].source, operandi_core::Source::Menu);
        assert_eq!(lines[0].class, operandi_core::CommandClass::SideEffect);
        assert!(a.editor.history.undo.is_empty(), "no undo step (BUG-20)");
        let again = a.report_link(ReportOptions::default());
        assert!(
            !again.contains("operandi.help.reportBug"),
            "excluded from its own report"
        );
        std::fs::remove_dir_all(a.environment.paths().reports()).unwrap();
    }

    #[test]
    fn a_missed_picture_still_opens_the_dialog_and_says_so_on_send() {
        let mut a = app();
        a.execute(
            "operandi.help.reportBug screenshot:true toolState:false commands:false log:false",
        );
        assert!(matches!(
            a.report_bug.stage,
            Stage::Capturing {
                then: Then::Send(_),
                ..
            }
        ));
        assert!(!a.has_modal(), "a scripted capture isn't a dialog");
        a.report_shot_arrived(None);
        assert!(a.opened_url.take().is_some());
        assert!(a.status.contains("No screenshot"), "{}", a.status);
        assert!(reports(&a).is_empty());
    }

    #[test]
    fn the_report_names_the_tool_selection_and_undone_commands() {
        let mut a = crate::tests::box_app();
        let polygons = |a: &Operandi| {
            a.editor
                .scene
                .items
                .iter()
                .map(|i| i.mesh.read().as_read().polygon_count())
                .sum::<usize>()
        };
        a.run(Command::Mode(Mode::Polygon));
        a.editor.scene.selection.faces = vec![1, 2];
        a.execute_from(operandi_core::Source::Key, "poly.flip");
        a.execute_from(operandi_core::Source::Key, "app.undo");
        let s = a.report_tool_state();
        assert_eq!(s.mode, "Polygons");
        assert_eq!(s.selected, 2);
        assert_eq!(s.polygons, polygons(&a));
        let records: Vec<_> = a.history.log.records.iter().cloned().collect();
        let undone = |r: &operandi_core::CommandRecord| {
            a.editor.history.redo.iter().any(|step| {
                step.records
                    .0
                    .iter()
                    .any(|s| s.time == r.time && s.line == r.line)
            })
        };
        let lines = capture::command_lines(&records, undone, None);
        let flip = lines.iter().find(|l| l.contains("poly.flip")).unwrap();
        assert!(flip.ends_with("(undone)"), "{lines:?}");
        let undo = lines.iter().find(|l| l.contains("app.undo")).unwrap();
        assert!(
            !undo.contains("failed") && !undo.contains("(undone)"),
            "{undo}"
        );
        assert_eq!(s.meshes, 1);
    }
}
