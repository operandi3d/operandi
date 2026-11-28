//! Command records in the app (UIC-224..227, HIS-20..22, HIS-50, CMD-14).
//!
//! Every line `execute` runs leaves a [`CommandRecord`]: the line as fired
//! (canonical, `command_record::canonical_line`), its class, its source
//! and how it ended. Records go to:
//!
//! - the **History** log: every top-level line, undone or not, quiet ones
//!   too (HIS-20), capped by `history.max`;
//! - the **Results** log: lines typed in the command entry or run from the
//!   Command History panel, with their outcome (HIS-50);
//! - the **undo step** they made (UIC-226): an undoable line waits in
//!   `Editor::pending_records` while it runs, and the step it records takes
//!   it. A tool session's lines (tool on, one `tool.attr` per changed value
//!   at mouse-up, never per mouse move) wait there until its drop.
//!
//! Lines a line runs itself (`cmds.batch`, presets) are its sub-commands
//! (UIC-227, HIS-12), capped by Max Sub-Commands Recorded. Nothing here is
//! saved with the scene, and nothing runs per frame of a drag: tool values
//! are compared only when a button is released, a key pressed or a command
//! runs.
use super::*;
use operandi_core::command_record::{self, canonical_line};
use operandi_core::{CommandClass, CommandRecord, RecordLog, Source};
use std::time::Instant;

/// History tab length when nothing set it (provisional, CH-Q02).
pub const HISTORY_MAX: usize = 1000;
/// Results tab length (provisional, CH-Q02).
pub const RESULTS_MAX: usize = 200;
/// Typed lines Up/Down steps through (HIS-61).
const RECALL_MAX: usize = 100;
/// Longest message a record keeps.
const MESSAGE_MAX: usize = 240;

/// The Command History's tabs, Undos first (HIS-10..50). Scripts waits for
/// the macro stage (UIC-228) and isn't shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Undos,
    History,
    Commands,
    Results,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Undos, Tab::History, Tab::Commands, Tab::Results];
    pub fn label(self) -> &'static str {
        match self {
            Tab::Undos => "Undos",
            Tab::History => "History",
            Tab::Commands => "Commands",
            Tab::Results => "Results",
        }
    }
}

/// The live tool's values when last recorded: what the next comparison
/// looks for changes against.
#[derive(Clone, Debug, PartialEq)]
struct ToolValues {
    tool: String,
    values: Vec<(String, String)>,
}

/// Session-only Command History state (not saved with the scene). Each
/// tab's F filter is its tree list's search row (`HistoryTrees`).
pub struct CommandHistory {
    pub log: RecordLog,
    pub results: RecordLog,
    /// Typed lines, oldest first, for Up/Down in the entry (HIS-61).
    pub recall: Vec<String>,
    /// Where Up/Down is in `recall`; `None` when the entry is being typed.
    pub recall_at: Option<usize>,
    /// The source `execute` gives a line when its caller names none: a
    /// menu sets Menu while it draws, a pie Pie (UIC-224).
    pub ui_source: Source,
    /// How deep inside `execute` the app is: lines run below the top level
    /// are sub-commands of the top-level line.
    depth: u32,
    /// The top-level line running now.
    current: Option<CommandRecord>,
    /// A bare line opened its dialog: the line the dialog then runs is
    /// recorded with this source (CMD-14).
    dialog_source: Option<Source>,
    tool_values: Option<ToolValues>,
    started: Instant,
    // ---- the panel
    pub tab: Tab,
    /// Height of the open panel above the entry; 0 is collapsed to the
    /// entry line (HIS-01).
    pub open_height: f32,
    /// The selected row of the History, Results or Commands tab.
    pub selected: Option<usize>,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self {
            log: RecordLog::new(HISTORY_MAX),
            results: RecordLog::new(RESULTS_MAX),
            recall: Vec::new(),
            recall_at: None,
            ui_source: Source::Button,
            depth: 0,
            current: None,
            dialog_source: None,
            tool_values: None,
            started: Instant::now(),
            tab: Tab::Undos,
            open_height: 0.,
            selected: None,
        }
    }
}

impl CommandHistory {
    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

fn short(message: &str) -> String {
    let mut m: String = message.chars().take(MESSAGE_MAX).collect();
    if m.len() < message.len() {
        m.push('…');
    }
    m
}

/// `app.undo`, `app.redo` and `cmds.undoTo` move the undo pointer and are
/// never steps themselves (CMD-02).
fn moves_undo_pointer(line: &str) -> bool {
    operandi_core::command_registry::find_line(line)
        .is_some_and(|(e, _)| matches!(e.name, "app.undo" | "app.redo" | "cmds.undoTo"))
}

impl Operandi {
    /// Runs `text` as a line from `source` and records it (UIC-224).
    pub(super) fn execute_from(&mut self, source: Source, text: &str) {
        let _ = self.execute_reporting(source, text);
    }

    /// [`Self::execute_from`] with the line's own result, as its record
    /// says: `Err` with the command's message when it refused or failed.
    /// Scripts report this; nothing is read from the undo list.
    pub(super) fn execute_reporting(
        &mut self,
        source: Source,
        text: &str,
    ) -> std::result::Result<(), String> {
        if self.history.depth > 0 {
            // A line run by a line: one of its sub-commands (UIC-227).
            let cap = self.editor.history.settings.max_sub_commands;
            let line = canonical_line(text);
            if let Some(current) = self.history.current.as_mut() {
                current.push_sub(line, cap);
            }
            self.history.depth += 1;
            self.execute_line(text);
            self.history.depth -= 1;
            return match &self.last_failure {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            };
        }
        if text.trim().is_empty() {
            self.execute_line(text);
            return Ok(());
        }
        // Values changed by the panel or a drag since the last record go
        // before this line (the reference's order).
        self.record_tool_values(true);
        let source = if self.has_modal() {
            self.history.dialog_source.take().unwrap_or(source)
        } else {
            source
        };
        let mut record = CommandRecord::of_line(text, source, self.history.now());
        let pointer = moves_undo_pointer(text);
        let tool_before = self.history_tool();
        let serial = self.editor.history.history_serial;
        let modal_before = self.has_modal();
        let status_before = self.status.clone();
        if record.class == CommandClass::Undoable && !pointer {
            self.editor.history.pending_records.0.push(record.clone());
        }
        self.last_failure = None;
        self.history.current = Some(record.clone());
        self.history.depth = 1;
        self.execute_line(text);
        self.history.depth = 0;
        if let Some(current) = self.history.current.take() {
            record = current;
        }
        let failure = self.last_failure.take();
        match &failure {
            Some(message) => {
                record.ok = false;
                record.message = short(message);
            }
            None if self.status != status_before || source == Source::Typed => {
                record.message = short(&self.status);
            }
            None => {}
        }
        let stepped = self.editor.history.history_serial != serial;
        let tool_after = self.history_tool();
        if stepped && !pointer {
            self.attach_to_step(&record, tool_after.is_some() && tool_after != tool_before);
        }
        if tool_after.is_none() {
            // No session open: lines that made no step have nowhere to go.
            self.editor.history.pending_records.0.clear();
        } else if !stepped {
            // A refused line inside a session isn't part of it.
            if !record.ok {
                let time = record.time;
                self.editor
                    .history
                    .pending_records
                    .0
                    .retain(|r| r.time != time);
            }
        }
        let opened_dialog = !modal_before && self.has_modal() && !stepped && record.ok;
        if opened_dialog {
            // The line after the dialog, with its values, is the record
            // (CMD-14); this bare line isn't.
            self.history.dialog_source = Some(source);
        } else {
            self.log_record(record.clone());
            if let Some(tool) = tool_before.as_ref()
                && tool_after.as_ref() != Some(tool)
                && !record
                    .line
                    .eq_ignore_ascii_case(&format!("tool.set {tool} off"))
            {
                self.log_tool_off(tool, stepped);
            }
        }
        if source == Source::Typed {
            self.history.results.push(record);
        }
        // What the line itself set is recorded by the line (`tool.attr`,
        // Reset): start comparing afresh.
        self.record_tool_values(false);
        match failure {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// The step this line made keeps the line's final record (with its
    /// outcome and sub-commands). `moved_to_session`: the line started a
    /// new tool session, so it waits for that session's drop instead.
    fn attach_to_step(&mut self, record: &CommandRecord, moved_to_session: bool) {
        let Some(step) = self
            .editor
            .history
            .undo
            .last_mut()
            .map(std::sync::Arc::make_mut)
        else {
            return;
        };
        let at = step
            .records
            .0
            .iter()
            .position(|r| r.time == record.time && r.line == record.line);
        if moved_to_session {
            if let Some(i) = at {
                step.records.0.remove(i);
            }
            self.editor.history.pending_records.0.push(record.clone());
            return;
        }
        match at {
            Some(i) => step.records.0[i] = record.clone(),
            None => {
                let cap = self.editor.history.settings.max_sub_commands as usize;
                step.records.extend_capped(vec![record.clone()], cap);
            }
        }
    }

    /// The record of an MCP bridge call (UIC-224, source bridge).
    // Unused on Windows, where the bridge has no transport yet (mcp.rs).
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(super) fn bridge_record(&self, tool: &str, args: &serde_json::Value) -> CommandRecord {
        let line = match (tool, args.get("command").and_then(|c| c.as_str())) {
            ("command_run", Some(command)) => {
                let rest = args
                    .get("arguments")
                    .filter(|a| a.as_object().is_some_and(|m| !m.is_empty()))
                    .map(|a| format!(" {a}"))
                    .unwrap_or_default();
                format!("{command}{rest}")
            }
            _ => format!("{tool} {args}"),
        };
        let mut r = CommandRecord::of_line(&line, Source::Bridge, self.history.now());
        if tool != "command_run" {
            r.line = line;
            r.class = CommandClass::Undoable;
        }
        r
    }

    /// After a bridge call: drop its waiting record when no step came of
    /// it, note a failure, and log it.
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(super) fn bridge_recorded(
        &mut self,
        mut r: CommandRecord,
        serial: u64,
        response: Option<&serde_json::Value>,
    ) {
        if self.editor.history.history_serial == serial {
            let time = r.time;
            self.editor
                .history
                .pending_records
                .0
                .retain(|p| p.time != time);
        }
        if let Some(resp) = response
            && resp["result"]["isError"] == serde_json::json!(true)
        {
            r.ok = false;
            r.message = short(resp["result"]["content"][0]["text"].as_str().unwrap_or(""));
        }
        self.history.log.push(r);
    }

    fn log_record(&mut self, record: CommandRecord) {
        self.history.log.push(record);
    }

    /// `tool.set <tool> off` when a session ends without that line (HIS-20).
    fn log_tool_off(&mut self, tool: &str, stepped: bool) {
        let off = CommandRecord::new(
            format!("tool.set {tool} off"),
            CommandClass::Undoable,
            Source::Button,
            self.history.now(),
        );
        if stepped
            && let Some(step) = self
                .editor
                .history
                .undo
                .last_mut()
                .map(std::sync::Arc::make_mut)
        {
            let cap = self.editor.history.settings.max_sub_commands as usize;
            step.records.extend_capped(vec![off.clone()], cap);
        }
        self.history.log.push(off);
    }

    /// The live tool's script id (`poly.bevel`), if a tool is open.
    pub(super) fn history_tool(&self) -> Option<String> {
        self.command_active()
            .map(|i| command::IDS[i].0.to_owned())
            .or_else(|| {
                let live = self.editor.session.live.as_ref()?;
                Some(tool_info::info(live.tool).command_id.to_owned())
            })
    }

    /// Compares the live tool's values with the last record (CMD-14):
    /// `emit` records one `tool.attr` line per changed value; otherwise the
    /// current values just become the new baseline. Called on a button
    /// release, a key press and around each command, never per drag frame.
    pub(super) fn record_tool_values(&mut self, emit: bool) {
        let Some(tool) = self.history_tool() else {
            self.history.tool_values = None;
            return;
        };
        // A tool without a data-driven panel records no values; an empty
        // list keeps it from being looked up again every frame.
        let values = self.tool_attr_values().unwrap_or_default();
        let now = ToolValues { tool, values };
        let Some(before) = self.history.tool_values.replace(now.clone()) else {
            return;
        };
        if !emit || before.tool != now.tool || before == now {
            return;
        }
        let time = self.history.now();
        for (key, value) in &now.values {
            let old = before.values.iter().find(|(k, _)| k == key);
            if old.is_some_and(|(_, v)| v == value) {
                continue;
            }
            let record = CommandRecord::new(
                command_record::tool_attr_line(&now.tool, key, value),
                CommandClass::Undoable,
                Source::Button,
                time,
            );
            self.editor.history.pending_records.0.push(record.clone());
            self.history.log.push(record);
        }
    }

    /// Twice a frame, cheaply: compare tool values only on frames where a
    /// button was released or a key pressed (no cost while dragging).
    /// `start` runs before the frame's input lands: a viewport drag's
    /// final values are recorded then. At the end of a frame whose release
    /// was over a 3D view the values the drop of that drag resets (a
    /// Move's per-drag offset) only become the new baseline.
    pub(super) fn history_frame(&mut self, ctx: &Context, start: bool) {
        let (released, keyed, at) = ctx.input(|i| {
            (
                i.pointer.any_released(),
                i.events
                    .iter()
                    .any(|e| matches!(e, Event::Key { pressed: true, .. })),
                i.pointer.interact_pos(),
            )
        });
        let in_view = at.is_some_and(|p| self.viewport_rect.contains(p));
        if (released || keyed) && self.history.tool_values.is_some() {
            let emit = start || keyed || !in_view;
            self.record_tool_values(emit);
        } else if self.history.tool_values.is_none() && self.history_tool().is_some() {
            // A tool opened without a line (a viewport gesture): baseline.
            self.record_tool_values(false);
        }
    }

    /// Remembers a typed line for Up/Down (HIS-61).
    pub(super) fn remember_typed(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.history.recall.retain(|l| l != line);
        self.history.recall.push(line.to_owned());
        if self.history.recall.len() > RECALL_MAX {
            self.history.recall.remove(0);
        }
        self.history.recall_at = None;
    }

    /// Up (`back`) or Down through earlier typed lines (HIS-61).
    pub(super) fn recall(&mut self, back: bool) {
        let n = self.history.recall.len();
        if n == 0 {
            return;
        }
        let at = match (self.history.recall_at, back) {
            (None, true) => Some(n - 1),
            (None, false) => None,
            (Some(0), true) => Some(0),
            (Some(i), true) => Some(i - 1),
            (Some(i), false) if i + 1 < n => Some(i + 1),
            (Some(_), false) => None,
        };
        self.history.recall_at = at;
        self.command = at
            .map(|i| self.history.recall[i].clone())
            .unwrap_or_default();
    }

    // ------------------------------------------------------------ commands

    /// `cmds.batch label:<text> c1:<line> ...`: the lines as one undo step
    /// (CMD-40, UIC-227). Stops at the first line that fails.
    pub(super) fn command_batch(&mut self, rest: &str) {
        let words = match operandi_core::command_line::tokens(rest) {
            Ok(w) => w,
            Err(e) => return self.history_fail(e),
        };
        let names = [
            "label", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9",
        ];
        let bound = match operandi_core::command_args::bind("cmds.batch", &words, &names) {
            Ok(b) => b,
            Err(e) => return self.history_fail(&e),
        };
        let label = bound[0].unwrap_or("Batch").to_owned();
        let lines: Vec<String> = bound[1..]
            .iter()
            .flatten()
            .map(|l| (*l).to_owned())
            .collect();
        if lines.is_empty() {
            return self.history_fail("cmds.batch: give at least one line (c1:<line>)");
        }
        let mark = self.editor.history_mark();
        for line in &lines {
            self.last_failure = None;
            self.execute(line);
            if let Some(e) = self.last_failure.clone() {
                self.status = format!("cmds.batch stopped at \"{line}\": {e}");
                self.last_failure = Some(self.status.clone());
                break;
            }
        }
        // By mark, not by list length: at Max Undo Level every new step
        // trims an old one and the length doesn't grow (Astra P4-003).
        if self.editor.steps_since(mark) > 0 {
            operandi_core::command_exec::fold_steps(&mut self.editor, mark, &label);
        }
        if self.last_failure.is_none() {
            self.status = format!("{label}: {} lines", lines.len());
        }
    }

    /// `cmds.undoTo <index>`: undo or redo until `index` steps are done
    /// (HIS-11). Stops when Undo or Redo can go no further.
    pub(super) fn command_undo_to(&mut self, rest: &str) {
        let Ok(target) = rest
            .trim()
            .trim_start_matches("index:")
            .trim()
            .parse::<usize>()
        else {
            return self.history_fail("Use cmds.undoTo <index>");
        };
        loop {
            let done = self.editor.history.undo.len();
            let phrase = if done > target && !self.editor.history.undo.is_empty() {
                "Undo"
            } else if done < target && !self.editor.history.redo.is_empty() {
                "Redo"
            } else {
                break;
            };
            self.execute(phrase);
            if self.editor.history.undo.len() == done {
                break;
            }
        }
    }

    /// `history.clear`, `history.max`, `history.resultsMax` (HIS-22, HIS-50).
    pub(super) fn command_history_log(&mut self, name: &str, rest: &str) {
        let rest = rest.trim();
        match name {
            "history.clear" => {
                let results = rest
                    .trim_start_matches("results:")
                    .eq_ignore_ascii_case("true")
                    || rest == "1";
                if results {
                    self.history.results.clear();
                } else {
                    self.history.log.clear();
                }
                self.status = "Ready".into();
            }
            _ => {
                let value = rest.trim_start_matches("count:").trim();
                let log = if name == "history.max" {
                    &mut self.history.log
                } else {
                    &mut self.history.results
                };
                if value == "?" {
                    self.status = log.max.to_string();
                    return;
                }
                match value.parse::<usize>() {
                    Ok(n) if (1..=100_000).contains(&n) => {
                        log.set_max(n);
                        self.status = "Ready".into();
                    }
                    _ => self.history_fail(&format!("Use {name} <1..100000|?>")),
                }
            }
        }
    }

    fn history_fail(&mut self, message: &str) {
        self.status = message.into();
        self.last_failure = Some(message.into());
    }
}
