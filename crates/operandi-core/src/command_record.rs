//! Command records (UIC-224..227, ARC-10, CMD-01..03, CMD-14, HIS-12):
//! what every run of a command line leaves behind.
//!
//! A [`CommandRecord`] is small: the line as fired, its class, where it
//! came from, how it ended, the first few lines it ran itself and when.
//! The app keeps two logs of them (the Command History's History and
//! Results tabs) and each undo step keeps the records that made it
//! ([`StepRecords`] on [`Checkpoint`](crate::Checkpoint)). None of this is
//! saved in the scene file.
use crate::command_registry::{self, ArgType, Entry};
use std::collections::VecDeque;

/// How a command shows in the Command History (CMD-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandClass {
    /// Changes the scene: one step on the undo stack.
    Undoable,
    /// Changes only the interface; in the history, not the undo list.
    Ui,
    /// Acts outside the scene and can't be undone (save, export, quit).
    SideEffect,
    /// Plumbing and queries: only in the History tab (CMD-23).
    Quiet,
}

impl CommandClass {
    pub fn label(self) -> &'static str {
        match self {
            CommandClass::Undoable => "undoable",
            CommandClass::Ui => "UI",
            CommandClass::SideEffect => "side effect",
            CommandClass::Quiet => "quiet",
        }
    }
}

/// Where a line came from (UIC-224).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// Typed in the command entry, or run from the Command History panel.
    Typed,
    Key,
    Button,
    Menu,
    Pie,
    Script,
    Macro,
    /// The MCP bridge.
    Bridge,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Typed => "typed",
            Source::Key => "key",
            Source::Button => "button",
            Source::Menu => "menu",
            Source::Pie => "pie",
            Source::Script => "script",
            Source::Macro => "macro",
            Source::Bridge => "bridge",
        }
    }
}

/// One run of one command line.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandRecord {
    /// The line as fired, in canonical form ([`canonical_line`]).
    pub line: String,
    pub class: CommandClass,
    pub source: Source,
    /// `false` when the command was refused or failed.
    pub ok: bool,
    /// The failure message, or what a success reported (a query's answer).
    pub message: String,
    /// The lines this one ran itself (a batch or preset), first ones only
    /// (Max Sub-Commands Recorded, HIS-12).
    pub sub: Vec<String>,
    /// Sub-command lines past the cap: counted, not kept.
    pub sub_dropped: u32,
    /// Seconds since the app started.
    pub time: f64,
    /// The scene time (exact ticks, time_tick.rs) the command acts at, for commands that
    /// will act at a time: "set a key at frame 24" (AR-02, master plan
    /// phase 0 door 5). `None` for every command today; nothing animates
    /// yet.
    pub scene_time: Option<crate::time_tick::Tick>,
}

impl CommandRecord {
    pub fn new(line: impl Into<String>, class: CommandClass, source: Source, time: f64) -> Self {
        Self {
            line: line.into(),
            class,
            source,
            ok: true,
            message: String::new(),
            sub: Vec::new(),
            sub_dropped: 0,
            time,
            scene_time: None,
        }
    }
    /// This record, acting at scene time `at`.
    pub fn at_scene_time(mut self, at: crate::time_tick::Tick) -> Self {
        self.scene_time = Some(at);
        self
    }
    /// A record for `line` with the class its registry entry gives.
    pub fn of_line(line: &str, source: Source, time: f64) -> Self {
        Self::new(canonical_line(line), class_of_line(line), source, time)
    }
    /// Keeps a line this record ran, up to `cap` of them.
    pub fn push_sub(&mut self, line: String, cap: u32) {
        if self.sub.len() < cap as usize {
            self.sub.push(line);
        } else {
            self.sub_dropped = self.sub_dropped.saturating_add(1);
        }
    }
}

/// The records an undo step keeps (UIC-226). Never part of a step's
/// identity: two editors whose scenes match compare equal whatever lines
/// made them (like `FreeMemory`), so undo comparisons are unchanged.
#[derive(Clone, Debug, Default)]
pub struct StepRecords(pub Vec<CommandRecord>);

impl PartialEq for StepRecords {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl StepRecords {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// The step's canonical lines joined for the Undos tab's second column.
    pub fn fired(&self) -> String {
        self.0
            .iter()
            .map(|r| r.line.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
    /// Adds `more` after the lines already kept, up to `cap` lines.
    pub fn extend_capped(&mut self, more: Vec<CommandRecord>, cap: usize) {
        for r in more {
            if self.0.len() >= cap.max(1) {
                break;
            }
            self.0.push(r);
        }
    }
}

/// A capped, append-only list of records: the History and Results tabs
/// (HIS-20, HIS-22, HIS-50). Undo never removes from it.
#[derive(Clone, Debug)]
pub struct RecordLog {
    pub records: VecDeque<CommandRecord>,
    pub max: usize,
}

impl RecordLog {
    pub fn new(max: usize) -> Self {
        Self {
            records: VecDeque::new(),
            max: max.max(1),
        }
    }
    pub fn push(&mut self, record: CommandRecord) {
        self.records.push_back(record);
        self.trim();
    }
    pub fn set_max(&mut self, max: usize) {
        self.max = max.max(1);
        self.trim();
    }
    pub fn clear(&mut self) {
        self.records.clear();
    }
    fn trim(&mut self) {
        while self.records.len() > self.max {
            self.records.pop_front();
        }
    }
}

/// A registry entry's class (CMD-03): `side_effect` and `quiet` flags
/// first, then undoable (and tools, whose drop is a step), then UI, then
/// queries (quiet, CMD-23). `None` when the entry has no class flag; the
/// registry test refuses that.
pub fn class(entry: &Entry) -> Option<CommandClass> {
    let f = entry.flags;
    if f.side_effect {
        Some(CommandClass::SideEffect)
    } else if f.quiet {
        Some(CommandClass::Quiet)
    } else if f.undoable || f.tool {
        Some(CommandClass::Undoable)
    } else if f.ui_only {
        Some(CommandClass::Ui)
    } else if f.query {
        Some(CommandClass::Quiet)
    } else {
        None
    }
}

/// The class of the command `line` names. A line that asks (`?` for a
/// value) is quiet (CMD-23); an unknown line counts as undoable so it is
/// never hidden.
pub fn class_of_line(line: &str) -> CommandClass {
    let Some((entry, rest)) = command_registry::find_line(line) else {
        return CommandClass::Undoable;
    };
    if entry.flags.query
        && rest
            .split_whitespace()
            .any(|w| w == "?" || w.ends_with(":?"))
    {
        return CommandClass::Quiet;
    }
    class(entry).unwrap_or(CommandClass::Undoable)
}

/// The line as recorded (UIC-224, CMD-10, CMD-14): the command's current
/// script name (an Input Map phrase becomes its Modo invocation, `Cube` →
/// `tool.set prim.cube on`), and every argument written out by name in the
/// order the registry lists them. Values with spaces are quoted. A line the
/// registry doesn't know, or whose arguments don't bind, is kept as typed.
pub fn canonical_line(line: &str) -> String {
    let line = line.trim();
    let Some((entry, rest)) = command_registry::find_line(line) else {
        return line.to_owned();
    };
    if rest.is_empty()
        && let Some(modo) = entry.modo
    {
        return modo.to_owned();
    }
    let name = entry.name;
    if rest.is_empty() {
        return name.to_owned();
    }
    let Ok(words) = crate::command_line::tokens(rest) else {
        return format!("{name} {rest}");
    };
    let names: Vec<&str> = entry.args.iter().map(|a| a.name).collect();
    if names.is_empty() {
        // A headless command lists its arguments in its JSON table.
        if let Some(spec) = crate::command_exec::spec(name) {
            let names: Vec<&str> = spec.args.iter().map(|a| a.name).collect();
            if let Ok(bound) = crate::command_args::bind(name, &words, &names) {
                return written_out(name, &names, &bound, |_| false);
            }
        }
        return format!("{name} {}", quote_all(&words));
    }
    match crate::command_args::bind(name, &words, &names) {
        Ok(bound) => written_out(name, &names, &bound, |i| {
            // `tool.set`, `tool.attr`: the reference writes these by
            // position (`tool.set poly.bevel on`), as its history shows.
            matches!(name, "tool.set" | "tool.attr") || entry.args[i].ty == ArgType::Json
        }),
        Err(_) => format!("{name} {}", quote_all(&words)),
    }
}

fn written_out(
    name: &str,
    names: &[&str],
    bound: &[Option<&str>],
    positional: impl Fn(usize) -> bool,
) -> String {
    let mut out = name.to_owned();
    for (i, value) in bound.iter().enumerate() {
        let Some(value) = value else { continue };
        out.push(' ');
        if !positional(i) {
            out.push_str(names[i]);
            out.push(':');
        }
        out.push_str(&quote(value));
    }
    out
}

fn quote_all(words: &[String]) -> String {
    words.iter().map(|w| quote(w)).collect::<Vec<_>>().join(" ")
}

/// A value as a line holds it: quoted when it has spaces (JSON in braces
/// stays as it is).
pub fn quote(value: &str) -> String {
    if value.chars().any(char::is_whitespace) && !value.starts_with('{') {
        format!("\"{value}\"")
    } else if value.is_empty() {
        "\"\"".to_owned()
    } else {
        value.to_owned()
    }
}

/// `tool.attr <tool> <attr> <value>` as recorded for a changed tool value
/// (CMD-14, HIS-20).
pub fn tool_attr_line(tool: &str, attr: &str, value: &str) -> String {
    format!("tool.attr {tool} {attr} {}", quote(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_written_out_by_name() {
        assert_eq!(
            canonical_line("poly.setPart Road"),
            "poly.setPart name:Road"
        );
        assert_eq!(
            canonical_line("poly.setPart {Road Part}"),
            "poly.setPart name:\"Road Part\""
        );
        assert_eq!(
            canonical_line("tool.set prim.cube on"),
            "tool.set prim.cube on"
        );
        assert_eq!(
            canonical_line("not a command at all"),
            "not a command at all"
        );
    }

    #[test]
    fn phrases_record_their_modo_invocation() {
        let cube = command_registry::find("Cube").unwrap();
        assert_eq!(canonical_line("Cube"), cube.modo.unwrap_or(cube.name));
    }

    #[test]
    fn every_command_in_the_table_has_a_class() {
        let missing: Vec<&str> = command_registry::COMMANDS
            .iter()
            .filter(|e| class(e).is_none())
            .map(|e| e.name)
            .collect();
        assert!(missing.is_empty(), "UIC-225: no class: {missing:?}");
    }

    #[test]
    fn queries_are_quiet() {
        assert_eq!(class_of_line("tool.set prim.cube ?"), CommandClass::Quiet);
        assert_eq!(class_of_line("cmds.showHelp"), CommandClass::Quiet);
    }

    #[test]
    fn sub_commands_stop_at_the_cap() {
        let mut r = CommandRecord::new("cmds.batch", CommandClass::Undoable, Source::Typed, 0.);
        for i in 0..5 {
            r.push_sub(format!("line {i}"), 3);
        }
        assert_eq!(r.sub.len(), 3);
        assert_eq!(r.sub_dropped, 2);
    }

    #[test]
    fn logs_keep_the_newest_records() {
        let mut log = RecordLog::new(2);
        for i in 0..4 {
            log.push(CommandRecord::new(
                format!("l{i}"),
                CommandClass::Ui,
                Source::Key,
                0.,
            ));
        }
        let lines: Vec<_> = log.records.iter().map(|r| r.line.as_str()).collect();
        assert_eq!(lines, ["l2", "l3"]);
    }
}
