//! Python scripting (master plan F15 / A23, kits plan K3): an embedded
//! CPython and the `operandi` module scripts import.
//!
//! What a script can do, all through the same doors as the command line,
//! menus and the MCP bridge (UIC-160..165):
//! - `operandi.run("line")` runs any registered command and reports the
//!   command's own result: success, or failure with its message. Nothing
//!   is inferred from the undo list.
//! - `operandi.query("line")` runs a query (UIC-163) and gives its answer
//!   (JSON answers come back as Python values).
//! - `with operandi.undo_group("name"):` folds every step the block makes
//!   into one undo step, named `name`.
//! - A whole script is one undo step too (UIC-227: a command that runs
//!   commands is one step), named after the script, or after its one
//!   group when it made just that.
//! - `operandi.mesh_get` / `operandi.mesh_set` read and write a mesh
//!   item's points and polygons in bulk (K9), one undo step per write.
//!
//! How it runs. Python lives on one worker thread for the life of the
//! process (it is Python's main thread). A script runs there; each call
//! into Operandi is a [`Request`] sent to whoever started the script,
//! which answers it on its own thread through a [`Host`]: the app between
//! frames (so the window stays live and Stop works), a test or the
//! headless bridge in a loop ([`run_blocking`]). A Python error never
//! crosses into Rust as a panic: the script ends with its traceback in
//! [`Outcome::error`]. [`Script::stop`] raises `KeyboardInterrupt` in the
//! script.
//!
//! Without the `python` feature the crate still builds; every script then
//! reports that Python isn't in this build.
use operandi_core::Editor;
use operandi_core::script_mesh::MeshData;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

#[cfg(feature = "python")]
mod embed;

/// What a finished script left.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    /// It ran to the end without an exception.
    pub ok: bool,
    /// Everything it printed (stdout and stderr, in order), including
    /// what [`Script::take_output`] already handed out.
    pub output: String,
    /// The traceback of the exception that ended it.
    pub error: Option<String>,
}

/// One piece of printed text.
#[derive(Clone, Debug, PartialEq)]
pub struct Printed {
    pub text: String,
    /// From stderr (shown in red, as errors are).
    pub error: bool,
}

/// A call from a script into Operandi.
#[derive(Debug)]
pub enum Request {
    /// A command line; the answer text for a line that answers.
    Run(String),
    /// A query line; its answer.
    Query(String),
    /// `undo_group` opened, with its name.
    GroupBegin(String),
    /// The innermost `undo_group` closed.
    GroupEnd,
    MeshGet(Option<u64>),
    MeshSet {
        item: Option<u64>,
        points: Vec<[f64; 3]>,
        polygons: Option<Vec<Vec<u32>>>,
    },
}

/// The answer to a [`Request`].
#[derive(Debug)]
pub enum Reply {
    Text(Option<String>),
    Mesh(u64, MeshData),
    Failed(String),
}

/// Script → host traffic.
#[derive(Debug)]
#[cfg_attr(not(feature = "python"), allow(dead_code))]
pub(crate) enum Msg {
    Request(Request),
    Print(Printed),
    Done(Outcome),
}

/// Who answers a script's calls: the app, or an editor alone.
pub trait Host {
    /// Runs one command line as typed (UIC-160) and returns the command's
    /// own result: `Ok(Some(text))` for a line that answers.
    fn run(&mut self, line: &str) -> Result<Option<String>, String>;
    /// The editor the line runs on (undo groups, bulk mesh data).
    fn editor(&mut self) -> &mut Editor;
    /// Answers a query line; refuses a line that isn't one.
    fn query(&mut self, line: &str) -> Result<String, String> {
        query_editor(self.editor(), line)
    }
}

/// A query on an editor alone: answers and changes nothing (UIC-163).
pub fn query_editor(e: &Editor, line: &str) -> Result<String, String> {
    answer_editor(e, line).unwrap_or_else(|| {
        let name = line.split_whitespace().next().unwrap_or(line);
        Err(format!("{name} isn't a query"))
    })
}

/// The answer to `line` when it is a query (`transform.channel` with
/// `value:?`, `operandi.anim.query`); `None` for a line that acts.
pub fn answer_editor(e: &Editor, line: &str) -> Option<Result<String, String>> {
    use operandi_core::{command_exec, command_line, command_registry};
    let words = match command_line::tokens(line) {
        Ok(w) => w,
        Err(e) => return Some(Err(e.into())),
    };
    let name = words.first()?;
    let entry = command_registry::find(name)?;
    let args = match command_line::json_args(entry.name, &words[1..]) {
        Ok(a) => a,
        Err(e) => return Some(Err(e)),
    };
    command_exec::answer(e, name, &args)
}

/// An editor on its own as a host: headless commands only (tests, the
/// headless MCP bridge).
pub struct EditorHost<'a>(pub &'a mut Editor);

impl Host for EditorHost<'_> {
    fn run(&mut self, line: &str) -> Result<Option<String>, String> {
        operandi_core::command_line::run_line(self.0, line).unwrap_or_else(|| {
            let name = line.split_whitespace().next().unwrap_or(line);
            Err(format!(
                "{name} needs the running app (it has no headless form)"
            ))
        })
    }
    fn editor(&mut self) -> &mut Editor {
        self.0
    }
}

/// Whether this build has Python, and why not when it doesn't.
pub fn available() -> Result<(), String> {
    #[cfg(feature = "python")]
    {
        embed::ensure()
    }
    #[cfg(not(feature = "python"))]
    {
        Err(NOT_BUILT.into())
    }
}

/// The refusal while another script runs.
pub const BUSY: &str = "A script is already running";

#[cfg(not(feature = "python"))]
const NOT_BUILT: &str = "Python isn't in this build (feature `python`)";

/// The Python version, e.g. "3.13.2".
pub fn version() -> Option<String> {
    #[cfg(feature = "python")]
    {
        embed::version()
    }
    #[cfg(not(feature = "python"))]
    {
        None
    }
}

/// What a script is: a file's text with its name for tracebacks, or a
/// console line (an expression's value prints; names persist between
/// console lines).
#[derive(Clone, Debug)]
pub struct Source {
    pub code: String,
    /// Shown in tracebacks (a file path, or `<console>`).
    pub name: String,
    pub console: bool,
}

impl Source {
    pub fn file_text(code: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            console: false,
        }
    }
    pub fn console(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            name: "<console>".into(),
            console: true,
        }
    }
}

/// A running script. Answer its calls with [`Script::pump`] until it
/// gives its [`Outcome`].
pub struct Script {
    msgs: Receiver<Msg>,
    replies: Sender<Reply>,
    printed: Vec<Printed>,
    output: String,
    /// Undo steps when each open `undo_group` began, outermost first.
    groups: Vec<(usize, String)>,
    /// Undo steps when the script started (first pump).
    start: Option<usize>,
    /// The undo label for the whole script (UIC-227).
    label: String,
    done: Option<Outcome>,
}

/// The undo label of a script: the file's name, or the console line.
#[cfg_attr(not(feature = "python"), allow(dead_code))]
fn label_of(source: &Source) -> String {
    if source.console {
        let line: String = source
            .code
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(40)
            .collect();
        format!("Console: {line}")
    } else {
        let name = std::path::Path::new(&source.name)
            .file_name()
            .map_or(source.name.clone(), |n| n.to_string_lossy().into_owned());
        format!("Script {name}")
    }
}

impl Script {
    /// Starts `source` on the Python thread. Refused while another script
    /// runs (one at a time) or when this build has no Python.
    pub fn start(source: Source) -> Result<Script, String> {
        #[cfg(feature = "python")]
        {
            let label = label_of(&source);
            let (msgs, replies) = embed::start(source)?;
            Ok(Script {
                msgs,
                replies,
                printed: Vec::new(),
                output: String::new(),
                groups: Vec::new(),
                start: None,
                label,
                done: None,
            })
        }
        #[cfg(not(feature = "python"))]
        {
            let _ = source;
            Err(NOT_BUILT.into())
        }
    }

    /// Raises `KeyboardInterrupt` in the running script (Stop, Ctrl+C).
    /// A script inside a long C call (a sleep) stops when it returns.
    pub fn stop(&self) {
        #[cfg(feature = "python")]
        embed::interrupt();
    }

    /// Printed text since the last call.
    pub fn take_output(&mut self) -> Vec<Printed> {
        std::mem::take(&mut self.printed)
    }

    /// Answers the script's calls for up to `budget`, then returns; gives
    /// the outcome once the script has ended.
    pub fn pump(&mut self, host: &mut dyn Host, budget: Duration) -> Option<Outcome> {
        let until = Instant::now() + budget;
        if self.start.is_none() {
            self.start = Some(host.editor().undo.len());
        }
        loop {
            if let Some(done) = self.done.take() {
                return Some(done);
            }
            let wait = until.saturating_duration_since(Instant::now());
            match self.msgs.recv_timeout(wait) {
                Ok(msg) => self.handle(host, msg),
                Err(RecvTimeoutError::Timeout) => return None,
                Err(RecvTimeoutError::Disconnected) => {
                    return Some(self.finish(Outcome {
                        ok: false,
                        output: String::new(),
                        error: Some("The Python thread stopped".into()),
                    }));
                }
            }
        }
    }

    fn handle(&mut self, host: &mut dyn Host, msg: Msg) {
        match msg {
            Msg::Print(p) => {
                self.output.push_str(&p.text);
                self.printed.push(p);
            }
            Msg::Request(r) => {
                let reply = self.serve(host, r);
                // The script may have ended meanwhile; nothing to tell.
                let _ = self.replies.send(reply);
            }
            Msg::Done(outcome) => {
                let finished = self.finish(outcome);
                // UIC-227: the script is one undo step. One step already
                // (a single command or group) keeps its own name.
                let e = host.editor();
                if let Some(from) = self.start
                    && e.undo.len() > from + 1
                {
                    operandi_core::command_exec::fold_steps(e, from, &self.label);
                }
                self.done = Some(finished);
            }
        }
    }

    /// The outcome with everything printed. A group the script left open
    /// (killed mid-block) is folded too.
    fn finish(&mut self, mut outcome: Outcome) -> Outcome {
        outcome.output = std::mem::take(&mut self.output);
        self.groups.clear();
        outcome
    }

    fn serve(&mut self, host: &mut dyn Host, request: Request) -> Reply {
        match request {
            Request::Run(line) => match host.run(&line) {
                Ok(text) => Reply::Text(text),
                Err(e) => Reply::Failed(e),
            },
            Request::Query(line) => match host.query(&line) {
                Ok(text) => Reply::Text(Some(text)),
                Err(e) => Reply::Failed(e),
            },
            Request::GroupBegin(name) => {
                let from = host.editor().undo.len();
                self.groups.push((from, name));
                Reply::Text(None)
            }
            Request::GroupEnd => {
                let Some((from, name)) = self.groups.pop() else {
                    return Reply::Failed("No undo group is open".into());
                };
                // Only the outermost group folds; inner ones are part of it.
                if self.groups.is_empty() {
                    let e = host.editor();
                    let from = from.min(e.undo.len());
                    if e.undo.len() > from {
                        operandi_core::command_exec::fold_steps(e, from, &name);
                    }
                }
                Reply::Text(None)
            }
            Request::MeshGet(item) => {
                let e = host.editor();
                let id = item.or(e.scene.selection.item);
                match operandi_core::script_mesh::mesh_data(e, id) {
                    Ok(d) => Reply::Mesh(id.unwrap_or_default(), d),
                    Err(e) => Reply::Failed(e),
                }
            }
            Request::MeshSet {
                item,
                points,
                polygons,
            } => match host.editor().set_mesh_data(item, points, polygons) {
                Ok(()) => Reply::Text(None),
                Err(e) => Reply::Failed(e),
            },
        }
    }
}

/// Runs `source` to the end on this thread's `host` (tests, the headless
/// bridge, the app's `python_run`). A refusal to start comes back as a
/// failed outcome. Waits its turn (up to a minute) while another script
/// runs.
pub fn run_blocking(host: &mut dyn Host, source: Source) -> Outcome {
    let give_up = Instant::now() + Duration::from_secs(60);
    let mut script = loop {
        match Script::start(source.clone()) {
            Ok(s) => break s,
            Err(e) if e == BUSY && Instant::now() < give_up => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Outcome {
                    ok: false,
                    output: String::new(),
                    error: Some(e),
                };
            }
        }
    };
    loop {
        if let Some(outcome) = script.pump(host, Duration::from_millis(50)) {
            return outcome;
        }
    }
}

#[cfg(all(test, feature = "python"))]
mod tests;
