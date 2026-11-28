//! The command registry at run time: every command together with what
//! runs it (architecture review 2026-11-14, pick E; extensibility plan L1
//! and L8).
//!
//! [`command_registry::COMMANDS`] stays the built-in table. A [`Registry`]
//! is built from it at start-up; headless entries bring their editor
//! handler (`Entry::json`). The host (the app) then:
//! - attaches its handlers to the built-in commands it runs ([`Registry::handle`]),
//! - adds its own window-only commands ([`Registry::add_host_command`],
//!   flagged "needs the window").
//!
//! Kits and plugins add commands with [`Registry::register`], under a
//! namespace of their own (`mykit.doThing`). Built-in names, their old
//! spellings and every namespace a built-in uses are protected, so a kit
//! can't take or shadow one.
//!
//! The app's `execute` looks a line up here and calls the handler; lines
//! no handler takes yet still go through its older parsers (moved over a
//! few at a time).
use crate::command_registry::{self, ArgType, COMMANDS, Entry, Kind};
use crate::{Command, Editor, command_exec, command_names};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};

/// Runs a command in the host (the app, with its window). The host reports
/// its own outcome (the app's status line).
pub type HostHandler<H> = fn(&mut H, &Call<'_>);

/// A command's answer about itself right now (UIC-223): whether it can
/// run, and whether its `?` query answers "on". Buttons, menus and forms
/// read their greying and lit state from this, never from logic of their
/// own.
pub type StateHandler<H> = fn(&H, &Call<'_>) -> CommandState;

/// What a [`StateHandler`] answers. The default is "can run, not on":
/// what a command without an answer reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandState {
    /// Why it can't run now (`None` = it can); shown as the tooltip of a
    /// greyed control.
    pub unavailable: Option<String>,
    /// Its `?` query answers "on": a button running it is lit.
    pub on: bool,
    /// What it calls itself right now, when that changes ("Undo Cube",
    /// "Restore Viewport Layout"): a menu entry's or submenu's `{title}`
    /// placeholder shows it (menus as forms). `None`: the form's label.
    pub title: Option<String>,
}

impl CommandState {
    /// Runnable, and lit when `on`.
    pub fn lit(on: bool) -> Self {
        CommandState {
            unavailable: None,
            on,
            title: None,
        }
    }
    /// The same answer, calling itself `title`.
    pub fn titled(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    /// Greyed with `why` when it is set; never lit.
    pub fn unless(why: Option<&str>) -> Self {
        CommandState {
            unavailable: why.map(str::to_owned),
            on: false,
            title: None,
        }
    }
}

/// One line being run: the command it named and its arguments as typed.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    pub entry: &'a Entry,
    /// The whole line, trimmed.
    pub line: &'a str,
    /// The words after the command's name, trimmed.
    pub rest: &'a str,
}

impl Call<'_> {
    /// The whole line in lower case, as the app's phrase parsers read it.
    pub fn lowered(&self) -> String {
        self.line.to_lowercase()
    }
}

/// Where a command came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// `command_registry::COMMANDS`.
    BuiltIn,
    /// Added by the host: needs its window.
    Host,
    /// Registered by a kit or plugin under this namespace.
    Kit(String),
}

/// A command and what runs it.
pub struct Registered<H> {
    pub entry: Entry,
    /// Runs it in the host. Headless commands run on the editor through
    /// `entry.json`; one with a host handler too gives the host only the
    /// bare line (its richer form: a key, a dialog).
    pub host: Option<HostHandler<H>>,
    /// The host handler takes only the bare line ([`Registry::handle_bare`]):
    /// lines with words go on to the host's other parsers.
    pub bare_only: bool,
    /// The command's answer about its state (UIC-223), when it has one.
    pub state: Option<StateHandler<H>>,
    pub origin: Origin,
}

impl<H> Clone for Registered<H> {
    fn clone(&self) -> Self {
        Registered {
            entry: self.entry,
            host: self.host,
            bare_only: self.bare_only,
            state: self.state,
            origin: self.origin.clone(),
        }
    }
}

/// Every command and its handlers. `H` is the host's context (the app).
pub struct Registry<H> {
    commands: Vec<Registered<H>>,
    /// Every name and alias, lower case, to its command.
    by_name: HashMap<String, usize>,
    /// First segments of every built-in and host dotted name, lower case.
    reserved: BTreeSet<String>,
}

impl<H> Clone for Registry<H> {
    fn clone(&self) -> Self {
        Registry {
            commands: self.commands.clone(),
            by_name: self.by_name.clone(),
            reserved: self.reserved.clone(),
        }
    }
}

/// Namespaces no kit may use even before a built-in command takes them.
pub(crate) const ALWAYS_RESERVED: &[&str] =
    &["operandi", "app", "kit", "plugin", "cmds", "command"];

impl<H> Default for Registry<H> {
    fn default() -> Self {
        Self::built_in()
    }
}

impl<H> Registry<H> {
    /// The built-in table, with the headless commands' editor handlers.
    pub fn built_in() -> Self {
        let mut registry = Registry {
            commands: Vec::with_capacity(COMMANDS.len()),
            by_name: HashMap::new(),
            reserved: ALWAYS_RESERVED.iter().map(|s| s.to_string()).collect(),
        };
        for entry in COMMANDS {
            registry
                .insert(*entry, None, Origin::BuiltIn)
                .expect("built-in names are unique (command_registry tests)");
        }
        registry
    }

    fn insert(
        &mut self,
        entry: Entry,
        host: Option<HostHandler<H>>,
        origin: Origin,
    ) -> Result<(), String> {
        let names: Vec<String> = entry.names().map(str::to_ascii_lowercase).collect();
        if let Some(taken) = names.iter().find(|n| self.by_name.contains_key(*n)) {
            return Err(format!("{taken} is already a command"));
        }
        let index = self.commands.len();
        for name in names {
            if !matches!(origin, Origin::Kit(_))
                && entry.kind == Kind::Script
                && let Some((space, _)) = name.split_once('.')
            {
                self.reserved.insert(space.to_owned());
            }
            self.by_name.insert(name, index);
        }
        self.commands.push(Registered {
            entry,
            host,
            bare_only: false,
            state: None,
            origin,
        });
        Ok(())
    }

    /// Attaches the host's handler to the built-in command `name` (any of
    /// its names). Attaching the same handler again is fine; a different
    /// one is refused, so two parsers can't both own a command.
    pub fn handle(&mut self, name: &str, run: HostHandler<H>) -> Result<(), String> {
        self.attach(name, run, false)
    }

    /// [`handle`](Self::handle) for a handler that only takes the bare
    /// line: the command's arguments are read by another of the host's
    /// parsers (`Bridge` starts the tool; `bridge segments 3` sets one).
    pub fn handle_bare(&mut self, name: &str, run: HostHandler<H>) -> Result<(), String> {
        self.attach(name, run, true)
    }

    fn attach(&mut self, name: &str, run: HostHandler<H>, bare_only: bool) -> Result<(), String> {
        let index = *self
            .by_name
            .get(&name.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("{name} isn't a command"))?;
        let command = &mut self.commands[index];
        match command.host {
            None => {
                command.host = Some(run);
                command.bare_only = bare_only;
            }
            Some(have) if std::ptr::fn_addr_eq(have, run) && command.bare_only == bare_only => {}
            Some(_) => {
                return Err(format!(
                    "{} already has a handler (from {name})",
                    command.entry.name
                ));
            }
        }
        Ok(())
    }

    /// Attaches the command's state answer (UIC-223) to `name`. Like
    /// handlers, a second, different answer is refused.
    pub fn answer(&mut self, name: &str, state: StateHandler<H>) -> Result<(), String> {
        let index = *self
            .by_name
            .get(&name.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("{name} isn't a command"))?;
        let command = &mut self.commands[index];
        match command.state {
            None => command.state = Some(state),
            Some(have) if std::ptr::fn_addr_eq(have, state) => {}
            Some(_) => {
                return Err(format!(
                    "{} already has a state answer (from {name})",
                    command.entry.name
                ));
            }
        }
        Ok(())
    }

    /// What the command `line` names answers about itself now (UIC-223);
    /// `None` when the line names no command (UIC-221: shown red). A
    /// command without an answer can run and isn't on.
    pub fn state(&self, host: &H, line: &str) -> Option<CommandState> {
        let line = line.trim();
        let (command, rest) = self.find_line(line)?;
        Some(match command.state {
            Some(answer) => answer(
                host,
                &Call {
                    entry: &command.entry,
                    line,
                    rest,
                },
            ),
            None => CommandState::default(),
        })
    }

    /// Adds one of the host's own commands (benchmarks, window things):
    /// flagged as needing the window.
    pub fn add_host_command(
        &mut self,
        mut entry: Entry,
        run: HostHandler<H>,
    ) -> Result<(), String> {
        entry.flags.ui_only = true;
        self.insert(entry, Some(run), Origin::Host)
    }

    /// Registers a kit's or plugin's command under `namespace`: every name
    /// must be `namespace.something`, and neither the namespace nor any
    /// name may belong to a built-in (or an old built-in spelling). It
    /// needs a host handler, an editor handler (`entry.json`), or both.
    pub fn register(
        &mut self,
        namespace: &str,
        entry: Entry,
        host: Option<HostHandler<H>>,
    ) -> Result<(), String> {
        let space = namespace.trim();
        if space.is_empty()
            || !space.starts_with(|c: char| c.is_ascii_alphabetic())
            || !space.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(format!(
                "Namespace \"{space}\" must be letters, digits or _, starting with a letter"
            ));
        }
        if self.reserved.contains(&space.to_ascii_lowercase()) {
            return Err(format!(
                "Namespace {space} belongs to Operandi's own commands"
            ));
        }
        if entry.kind != Kind::Script {
            return Err(format!(
                "{}: kits register dotted script names, not phrases",
                entry.name
            ));
        }
        for name in entry.names() {
            let inside = name
                .split_once('.')
                .is_some_and(|(head, tail)| head.eq_ignore_ascii_case(space) && !tail.is_empty());
            if !inside {
                return Err(format!("{name} must start with {space}."));
            }
            if command_registry::find(name).is_some()
                || command_names::renamed_command(name).is_some()
            {
                return Err(format!("{name} is one of Operandi's own commands"));
            }
        }
        if host.is_none() && entry.json.is_none() {
            return Err(format!("{} has nothing to run it", entry.name));
        }
        self.insert(entry, host, Origin::Kit(space.to_owned()))
    }

    /// The command named `name` (any case), by its name, an alias, or an
    /// old spelling from `command_names`.
    pub fn find(&self, name: &str) -> Option<&Registered<H>> {
        let name = name.trim();
        let exact = |n: &str| {
            self.by_name
                .get(&n.to_ascii_lowercase())
                .map(|&i| &self.commands[i])
        };
        exact(name).or_else(|| {
            command_names::renamed_command(name)
                .or_else(|| {
                    command_names::COMMAND_EXPANSIONS
                        .iter()
                        .find(|(old, _, _)| old.eq_ignore_ascii_case(name))
                        .map(|(_, new, _)| *new)
                })
                .and_then(exact)
        })
    }

    /// The command a whole line starts with, and the rest of the line.
    /// Multi-word names win over shorter ones ("Spike Reset" over "Spike").
    pub fn find_line<'l>(&self, line: &'l str) -> Option<(&Registered<H>, &'l str)> {
        let line = line.trim();
        let mut ends: Vec<usize> = line
            .char_indices()
            .filter(|(_, c)| c.is_whitespace())
            .map(|(i, _)| i)
            .collect();
        ends.push(line.len());
        ends.into_iter()
            .rev()
            .find_map(|end| self.find(&line[..end]).map(|c| (c, line[end..].trim())))
    }

    /// The host handler for `line` and the call to give it, when its
    /// command has one and the line fits: a command without arguments, or
    /// a headless one, only takes the bare line (anything after it is for
    /// another parser).
    ///
    /// The words are checked against the entry's arguments only as far as
    /// that's safe for every parser: each required argument has a word,
    /// and a required choice is one of its choices (or its index). The
    /// handler reads the rest.
    pub fn host_call<'l>(&self, line: &'l str) -> Option<(HostHandler<H>, Entry, &'l str)> {
        let (command, rest) = self.find_line(line)?;
        let run = command.host?;
        let entry = &command.entry;
        // A headless command with a host handler: the host has its richer
        // form of the bare line (the Delete key, a dialog); lines with
        // words run on the editor (`command_for`).
        if !rest.is_empty() && (command.bare_only || entry.args.is_empty() || entry.json.is_some())
        {
            return None;
        }
        // A Modo-named command typed as its phrase (`Convert to Edges` for
        // select.convert): phrases take no arguments (UIC-165).
        let trimmed = line.trim();
        let head = trimmed[..trimmed.len() - rest.len()].trim();
        if entry.spelled.is_some() && !head.eq_ignore_ascii_case(entry.name) {
            return rest.is_empty().then_some((run, command.entry, rest));
        }
        // Bound as the handler binds them: NAME:VALUE in any order or by
        // position. Reading words by position only refused a menu's
        // `attr:width` as no choice of `attr`, so the line fell through to
        // "Unknown command" (layouts stage 4's Alignment > Normal).
        let tokens = crate::command_line::tokens(rest).unwrap_or_default();
        let names: Vec<&str> = entry.args.iter().map(|a| a.name).collect();
        let bound = crate::command_args::bind(entry.name, &tokens, &names);
        let words: Vec<Option<&str>> = match &bound {
            Ok(bound) => bound.clone(),
            // A line the handler will refuse: let it say why.
            Err(_) => return Some((run, command.entry, rest)),
        };
        for (i, arg) in entry.args.iter().enumerate().filter(|(_, a)| !a.optional) {
            let word = (*words.get(i)?)?;
            if let ArgType::Choice(choices) = arg.ty
                && !choices.iter().any(|c| c.eq_ignore_ascii_case(word))
                && !word.parse::<usize>().is_ok_and(|n| n < choices.len())
            {
                return None;
            }
        }
        Some((run, command.entry, rest))
    }

    /// Every command, built-ins first in table order, then host and kit
    /// commands as added.
    pub fn iter(&self) -> impl Iterator<Item = &Registered<H>> {
        self.commands.iter()
    }

    /// The entries in `filter`'s area, or whose name contains it (all when
    /// empty), sorted by area then name.
    pub fn matching(&self, filter: &str) -> Vec<&Entry> {
        command_registry::matching_in(self.commands.iter().map(|c| &c.entry), filter)
    }

    /// The one `Command` that runs the headless command `given` with these
    /// JSON arguments (a built-in's or a kit's): `command_exec::command_for`
    /// for every command here.
    pub fn command_for(
        &self,
        e: &mut Editor,
        given: &str,
        given_args: &Map<String, Value>,
    ) -> Result<Option<Command>, String> {
        let (name, args) = command_exec::resolve_command(given, given_args);
        let Some(json) = self.find(&name).and_then(|c| c.entry.json) else {
            return Err(format!("Unknown command {name}"));
        };
        command_exec::check_json_args(&name, json.args, &args)?;
        (json.run)(e, &name, &args)
    }
}
