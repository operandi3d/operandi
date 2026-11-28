//! Python scripting in the app (master plan F15, kits plan K3): the Script
//! Console, Run Script…, the scripts folder and the commands behind them.
//! The interpreter and the `operandi` module live in operandi-python.
//!
//! A script runs on the Python thread; its calls into Operandi are
//! answered here between frames ([`Operandi::python_frame`]), through the
//! same path as a typed line (`execute_reporting`, UIC-160), so the window
//! stays live and Stop works while it runs. Hidden behind the Python
//! scripting switch (python_switch.rs).
use super::*;
use crate::command_handlers::Commands;
use crate::unavailable::Gate;
use operandi_core::command_registry::{self as table, Area};
use operandi_python::{Host, Outcome, Printed, Script, Source};
use std::time::Duration;

/// How long a frame spends answering a running script's calls.
const FRAME_BUDGET: Duration = Duration::from_millis(12);
/// Console lines kept (oldest go first).
const LOG_MAX: usize = 2000;
/// Remembered console inputs.
const HISTORY_MAX: usize = 200;

/// One line of console output.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Line {
    /// What was typed (shown after the prompt).
    Input(String),
    Output(String),
    /// stderr, tracebacks and refusals: shown in the error colour.
    Error(String),
    /// Started, finished, stopped.
    Note(String),
}

#[derive(Default)]
pub(crate) struct PythonConsole {
    pub(crate) open: bool,
    pub(crate) input: String,
    pub(crate) log: Vec<Line>,
    /// Earlier inputs, oldest first, for Up/Down.
    history: Vec<String>,
    recall: Option<usize>,
    running: Option<(Script, String)>,
    /// The last finished script, for tests and the bridge.
    pub(crate) last: Option<Outcome>,
}

impl PythonConsole {
    pub(crate) fn running(&self) -> bool {
        self.running.is_some()
    }
    fn push(&mut self, line: Line) {
        self.log.push(line);
        let excess = self.log.len().saturating_sub(LOG_MAX);
        self.log.drain(..excess);
    }
    fn printed(&mut self, printed: Vec<Printed>) {
        for p in printed {
            let text = p.text.trim_end_matches('\n').to_owned();
            if text.is_empty() && !p.text.is_empty() {
                continue;
            }
            self.push(if p.error {
                Line::Error(text)
            } else {
                Line::Output(text)
            });
        }
    }
}

/// The app as a script's host: lines run as typed lines do.
struct AppHost<'a>(&'a mut Operandi);

impl Host for AppHost<'_> {
    fn run(&mut self, line: &str) -> std::result::Result<Option<String>, String> {
        let name = line.split_whitespace().next().unwrap_or("");
        // A query answers and changes nothing (UIC-163).
        if let Some(answer) = operandi_python::answer_editor(&self.0.editor, line) {
            return answer.map(Some);
        }
        if name.starts_with("operandi.python.") {
            return Err("A script can't start another script".into());
        }
        if let Some(refused) = operandi_python::check_undo_redo(&self.0.editor, line) {
            refused?;
        }
        self.0
            .execute_reporting(operandi_core::Source::Script, line)
            .map(|()| None)
    }
    fn editor(&mut self) -> &mut operandi_core::Editor {
        &mut self.0.editor
    }
}

const RUN_ARGS: &[table::Arg] = &[
    table::opt("code", table::ArgType::Text, table::Unit::None, None),
    table::opt("file", table::ArgType::Path, table::Unit::None, None),
];
const OPEN_ARGS: &[table::Arg] = &[table::opt(
    "open",
    table::ArgType::Choice(&["toggle", "true", "false"]),
    table::Unit::None,
    None,
)];

/// `key:value` words of a host command's arguments.
fn words(rest: &str) -> std::result::Result<Vec<(String, String)>, String> {
    let words = operandi_core::command_line::tokens(rest).map_err(str::to_owned)?;
    words
        .into_iter()
        .map(|w| match w.split_once(':') {
            Some((k, v)) => Ok((k.to_ascii_lowercase(), v.to_owned())),
            None => Err(format!("Give arguments as name:value ({w})")),
        })
        .collect()
}

/// The app's script commands (UIC-160: every action is a command).
pub(crate) fn register(r: &mut Commands) {
    r.add_host_command(
        table::script(
            "operandi.python.console",
            Area::Debug,
            "Shows or hides the Script Console (Python scripting preview).",
        )
        .args(OPEN_ARGS)
        .operandi()
        .ui(),
        |a, c| {
            if !crate::python_switch::enabled() {
                a.refuse(crate::python_switch::OFF);
                return;
            }
            a.python.open = match c.rest.trim() {
                "open:true" => true,
                "open:false" => false,
                _ => !a.python.open,
            };
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.python.run",
            Area::Debug,
            "Runs Python: code:{...} or file:<path.py>. The script is one undo step (UIC-227); its output goes to the Script Console.",
        )
        .args(RUN_ARGS)
        .operandi()
        .side_effect()
        .example("operandi.python.run code:{print(1 + 1)}"),
        |a, c| {
            if !crate::python_switch::enabled() {
                a.refuse(crate::python_switch::OFF);
                return;
            }
            let source = match words(c.rest).and_then(|w| source_of(&w)) {
                Ok(s) => s,
                Err(e) => {
                    a.refuse(&e);
                    return;
                }
            };
            a.python_start(source);
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.python.runScript",
            Area::Debug,
            "Run Script…: chooses a .py file (starting in the Scripts folder) and runs it.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !crate::python_switch::enabled() {
                a.refuse(crate::python_switch::OFF);
                return;
            }
            let folder = a.scripts_folder();
            let _ = std::fs::create_dir_all(&folder);
            if !a.choose(
                crate::file_chooser::Choose::Script { folder },
                crate::file_chooser::Purpose::RunScript,
            ) {
                a.refuse("No file panel is available; use operandi.python.run file:<path>");
            }
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
    r.add_host_command(
        table::script(
            "operandi.python.stop",
            Area::Debug,
            "Stops the running script (KeyboardInterrupt); Ctrl+C in the Script Console does the same.",
        )
        .operandi()
        .ui(),
        |a, _| {
            if !a.python_stop() {
                a.status = "No script is running".into();
            }
        },
    )
    .unwrap_or_else(|e| panic!("{e}"));
}

fn source_of(words: &[(String, String)]) -> std::result::Result<Source, String> {
    let get = |k: &str| {
        words
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.as_str())
    };
    for (k, _) in words {
        if !matches!(k.as_str(), "code" | "file") {
            return Err(format!("operandi.python.run takes code or file, not {k}"));
        }
    }
    match (get("code"), get("file")) {
        (Some(code), None) => Ok(Source::file_text(code, "<command>")),
        (None, Some(file)) => {
            let code =
                std::fs::read_to_string(file).map_err(|e| format!("Couldn't read {file}: {e}"))?;
            Ok(Source::file_text(code, file))
        }
        _ => Err("Give code:{...} or file:<path.py>".into()),
    }
}

impl Operandi {
    /// A refusal: the status line and the line's record both say it.
    fn refuse(&mut self, message: &str) {
        self.status = message.into();
        self.last_failure = Some(message.into());
    }

    /// The Scripts folder in the user's Operandi data folder.
    pub(super) fn scripts_folder(&self) -> PathBuf {
        self.environment.paths().data_root().join("Scripts")
    }

    /// Runs the file the Run Script… panel chose.
    pub(super) fn python_run_file(&mut self, path: &std::path::Path) {
        let line = format!("operandi.python.run file:{{{}}}", path.display());
        self.execute_from(operandi_core::Source::Menu, &line);
    }

    /// Starts `source` (one script at a time); the console shows it.
    pub(super) fn python_start(&mut self, source: Source) {
        if self.python.running() {
            self.refuse(operandi_python::BUSY);
            return;
        }
        let label = if source.console {
            source.code.clone()
        } else {
            source.name.clone()
        };
        match Script::start(source) {
            Ok(script) => {
                if !label.contains('\n')
                    && self.python.log.last() != Some(&Line::Input(label.clone()))
                {
                    self.python.push(Line::Note(format!("Running {label}")));
                }
                self.python.running = Some((script, label));
                self.status = "Script running".into();
            }
            Err(e) => {
                self.python.push(Line::Error(e.clone()));
                self.refuse(&e);
            }
        }
    }

    /// Stop / Ctrl+C: `false` when nothing runs.
    pub(super) fn python_stop(&mut self) -> bool {
        match &self.python.running {
            Some((script, _)) => {
                script.stop();
                self.python.push(Line::Note("Stopping…".into()));
                true
            }
            None => false,
        }
    }

    /// Once a frame: answer the running script's calls (up to
    /// [`FRAME_BUDGET`]), show what it printed, and the console window.
    pub(super) fn python_frame(&mut self, ctx: &Context) {
        if let Some((mut script, label)) = self.python.running.take() {
            let outcome = script.pump(&mut AppHost(self), FRAME_BUDGET);
            let printed = script.take_output();
            self.python.printed(printed);
            match outcome {
                Some(outcome) => self.python_finished(outcome, &label),
                None => {
                    self.python.running = Some((script, label));
                    ctx.request_repaint();
                }
            }
        }
        self.python_console_window(ctx);
    }

    fn python_finished(&mut self, outcome: Outcome, label: &str) {
        match &outcome.error {
            Some(tb) => {
                for line in tb.trim_end().lines() {
                    self.python.push(Line::Error(line.to_owned()));
                }
                let last = tb.trim_end().lines().last().unwrap_or("Script failed");
                self.status = format!("Script failed: {last}");
            }
            None => {
                if !label.contains('\n') && label.starts_with('<') {
                    self.python.push(Line::Note("Done".into()));
                }
                self.status = "Script finished".into();
            }
        }
        self.python.last = Some(outcome);
    }

    /// Runs a console input line (Enter).
    fn python_console_submit(&mut self) {
        let code = std::mem::take(&mut self.python.input);
        if code.trim().is_empty() {
            return;
        }
        self.python.push(Line::Input(code.clone()));
        if self.python.history.last() != Some(&code) {
            self.python.history.push(code.clone());
            let excess = self.python.history.len().saturating_sub(HISTORY_MAX);
            self.python.history.drain(..excess);
        }
        self.python.recall = None;
        self.python_start(Source::console(code));
    }

    /// The Script Console: a floating window (UIC-183) with the output
    /// on top, the input line below with Run and Stop. Up/Down recall
    /// earlier inputs; errors show in the error colour.
    fn python_console_window(&mut self, ctx: &Context) {
        if !self.python.open || !crate::python_switch::enabled() {
            return;
        }
        const PLACE: &str = "scriptConsole";
        let place = self.palettes.places.get(PLACE).copied();
        let size = place.map_or([640., 320.], |p| p.size);
        let spec = crate::floating::OsWindow {
            id: egui::ViewportId::from_hash_of("operandi-script-console"),
            title: "Script Console".into(),
            size,
            position: place.and_then(|p| p.position),
            min_size: [360., 160.],
        };
        let shown = self.os_window(ctx, spec, |app, ctx| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(ctx.style().visuals.panel_fill)
                        .inner_margin(6.),
                )
                .show(ctx, |ui| app.python_console_ui(ui));
        });
        let mut place = place.unwrap_or(crate::palettes::Place {
            size,
            position: None,
        });
        if let Some(s) = shown.size
            && s.iter().all(|v| *v >= 100.)
        {
            place.size = s;
        }
        if let Some(at) = shown.position {
            place.position = Some(at);
        }
        self.palettes.places.insert(PLACE.into(), place);
        if !shown.open {
            self.python.open = false;
        }
    }

    pub(super) fn python_console_ui(&mut self, ui: &mut Ui) {
        let error = ui.visuals().error_fg_color;
        let weak = ui.visuals().weak_text_color();
        let row = ui.text_style_height(&egui::TextStyle::Monospace) + 8.;
        let output_height = (ui.available_height() - row - 8.).max(40.);
        egui::ScrollArea::vertical()
            .max_height(output_height)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                // A first look: lines to try (UIC-04's usage hint, in place).
                if self.python.log.is_empty() {
                    ui.label(egui::RichText::new("Try these (operandi is imported):").color(weak));
                    for line in operandi_python::TRY_THESE {
                        ui.add(
                            egui::Label::new(egui::RichText::new(*line).monospace().color(weak))
                                .selectable(true),
                        );
                    }
                }
                for line in &self.python.log {
                    let (text, colour) = match line {
                        Line::Input(t) => (format!(">>> {t}"), None),
                        Line::Output(t) => (t.clone(), None),
                        Line::Error(t) => (t.clone(), Some(error)),
                        Line::Note(t) => (t.clone(), Some(weak)),
                    };
                    let mut rich = egui::RichText::new(text).monospace();
                    if let Some(c) = colour {
                        rich = rich.color(c);
                    }
                    ui.add(egui::Label::new(rich).selectable(true));
                }
            });
        ui.separator();
        let running = self.python.running();
        ui.horizontal(|ui| {
            let run_width = 120.;
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.python.input)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Python, e.g. operandi.run(\"operandi.time.set frame:12\")")
                    .desired_width((ui.available_width() - run_width).max(80.)),
            );
            crate::numeric_tab::register(ui, &field, Some(&self.python.input));
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if field.has_focus() {
                let (up, down, stop) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.modifiers.ctrl && i.key_pressed(egui::Key::C) && running,
                    )
                });
                if up || down {
                    self.python_recall(up);
                }
                if stop {
                    self.execute("operandi.python.stop");
                }
            }
            // UIC-04: tooltips name the action and its key.
            if ui
                .gated(
                    running.then_some("A script is running; Stop it first"),
                    egui::Button::new("Run"),
                )
                .on_hover_text("Run the line (Enter)")
                .clicked()
                || (enter && !running)
            {
                self.python_console_submit();
                field.request_focus();
            }
            if ui
                .gated(
                    (!running).then_some("No script is running"),
                    egui::Button::new("Stop"),
                )
                .on_hover_text("Stop the running script (Ctrl+C)\noperandi.python.stop")
                .clicked()
            {
                self.execute("operandi.python.stop");
            }
            if ui
                .button("Clear")
                .on_hover_text("Clear the output")
                .clicked()
            {
                self.python.log.clear();
            }
        });
    }

    fn python_recall(&mut self, back: bool) {
        let h = &self.python.history;
        if h.is_empty() {
            return;
        }
        let at = match (self.python.recall, back) {
            (None, true) => Some(h.len() - 1),
            (None, false) => None,
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < h.len() => Some(i + 1),
            (Some(_), false) => None,
        };
        self.python.recall = at;
        self.python.input = at.map(|i| h[i].clone()).unwrap_or_default();
    }
}
