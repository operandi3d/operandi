//! Bug, idea and crash reports. Everything here is user-initiated: the app
//! never sends anything by itself and holds no tokens. A report is the
//! reports repository's GitHub issue form ([`BUG_FORM`]), pre-filled from
//! the link and opened in the user's browser; they read it, edit it and
//! press Submit there (or don't). The bug form links to the idea form at its
//! top, so one menu item covers both.
//!
//! - Help > Report Bug or Idea… (`operandi.help.reportBug`, help_menu.rs)
//!   opens the bug form with its Environment field filled in (version,
//!   commit, OS, GPU, the last log lines from log_ring.rs).
//! - A panic hook writes a crash file to `<data folder>/crashes/`. On the next
//!   launch the app asks whether to report it; Report opens the bug form with
//!   a summary and a trimmed backtrace and shows the full file in Finder so
//!   the user can drag it into the issue.
//! - On macOS, native crashes (for example in the Cycles C++ code, which a
//!   Rust panic hook can't see) leave a report in
//!   ~/Library/Logs/DiagnosticReports. New ones for this executable are
//!   offered the same way.
//!
//! URL building, body formatting, truncation, crash-file writing and parsing
//! and DiagnosticReports matching are plain functions with unit tests below.
use crate::log_ring;
use eframe::egui::*;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// The repository issues are filed against: the public code repository.
/// The one place to change it.
pub const REPORT_REPO_URL: &str = "https://github.com/operandi3d/operandi";
/// The issue form every report opens, in the repository's
/// `.github/ISSUE_TEMPLATE` (label `bug`). Its field ids are the query keys
/// below; the idea form (`idea.yml`) is linked from its top.
pub const BUG_FORM: &str = "bug.yml";
/// Bug form field ids: what happened, and the environment (a text block).
/// The form's `steps` and `expected` are left for the user.
pub const FIELD_WHAT: &str = "what";
pub const FIELD_ENVIRONMENT: &str = "environment";
/// GitHub refuses very long new-issue links; stay well under its limit.
pub const MAX_URL: usize = 7000;
/// Log lines included in a report.
pub const LOG_LINES: usize = 30;
/// Subfolder of the data folder that holds crash files and their state.
pub const CRASH_FOLDER: &str = "crashes";
const STATE_FILE: &str = "state.json";
/// A native crash report this close after a panic file is the same crash
/// (a panic that can't unwind aborts, and macOS records the abort too).
const SAME_CRASH_SECS: u64 = 120;
/// Native reports read at most this much (they can be large).
const MAX_NATIVE_READ: u64 = 512 * 1024;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Short commit hash from build.rs, "+dirty" with local changes.
pub const COMMIT: &str = env!("OPERANDI_GIT_COMMIT");

// ---------------------------------------------------------------- environment

/// What a report says about the build and machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Environment {
    pub version: String,
    pub commit: String,
    pub os: String,
    pub gpu: String,
}
impl Environment {
    pub fn current(gpu: &str) -> Self {
        Self {
            version: VERSION.into(),
            commit: COMMIT.into(),
            os: os_description().into(),
            gpu: gpu.into(),
        }
    }
    /// The lines at the top of the form's Environment field (a plain text
    /// block, so no Markdown).
    pub fn text(&self) -> String {
        format!(
            "Operandi: {} ({})\nOS: {}\nGPU: {}\n",
            self.version, self.commit, self.os, self.gpu
        )
    }
}

/// "macOS 15.2 (aarch64)" and similar; computed once.
pub fn os_description() -> &'static str {
    static OS: OnceLock<String> = OnceLock::new();
    OS.get_or_init(|| {
        let arch = std::env::consts::ARCH;
        let name = os_name_and_version().unwrap_or_else(|| std::env::consts::OS.to_owned());
        format!("{name} ({arch})")
    })
}
fn command_text(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (out.status.success() && !text.is_empty()).then_some(text)
}
fn os_name_and_version() -> Option<String> {
    if cfg!(target_os = "macos") {
        command_text("sw_vers", &["-productVersion"]).map(|v| format!("macOS {v}"))
    } else if cfg!(target_os = "linux") {
        let text = std::fs::read_to_string("/etc/os-release").ok()?;
        os_release_pretty_name(&text)
    } else if cfg!(target_os = "windows") {
        command_text("cmd", &["/C", "ver"])
    } else {
        None
    }
}
fn os_release_pretty_name(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_owned())
}

// ---------------------------------------------------------------- issue links

/// Percent-encodes everything except RFC 3986 unreserved characters.
pub fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(byte as char);
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// A GitHub issue-form link: the bug form with `title` and each non-empty
/// field (form field id, text) filled in. Empty fields are left out, so the
/// form shows its own placeholder there.
pub fn issue_url(title: &str, fields: &[(&str, &str)]) -> String {
    let mut url = format!(
        "{REPORT_REPO_URL}/issues/new?template={}&title={}",
        percent_encode(BUG_FORM),
        percent_encode(title)
    );
    for (key, value) in fields {
        if !value.is_empty() {
            let _ = write!(url, "&{key}={}", percent_encode(value));
        }
    }
    url
}

/// A field's text in three parts: `head`, then as many `lines` as fit, then
/// `tail`. `keep_newest` keeps the last lines (logs); otherwise the first (a
/// backtrace, whose top frames matter most).
pub struct TrimmedBody<'a> {
    pub head: String,
    pub lines: &'a [String],
    pub keep_newest: bool,
    pub tail: String,
}
impl TrimmedBody<'_> {
    /// The text keeping `keep` of the lines, with a note when some are cut.
    fn keeping(&self, keep: usize) -> String {
        let n = self.lines.len();
        let kept = if self.keep_newest {
            &self.lines[n - keep..]
        } else {
            &self.lines[..keep]
        };
        let mut text = self.head.clone();
        if keep < n {
            let note = format!(
                "… {} of {n} lines left out to fit the link; the full text is in the attached file.",
                n - keep
            );
            if self.keep_newest {
                text.push_str(&note);
                text.push('\n');
            }
            push_lines(&mut text, kept);
            if !self.keep_newest {
                text.push_str(&note);
                text.push('\n');
            }
        } else {
            push_lines(&mut text, kept);
        }
        text.push_str(&self.tail);
        text
    }
}

/// The issue-form link with `fields` as they are and `body` in field
/// `key`, cutting the body's lines (with a note saying so) until the whole
/// link fits in `max` characters. If the rest alone is too long, the
/// longest texts are cut too, so the result always fits.
pub fn fit_issue_url(
    title: &str,
    fields: &[(&str, &str)],
    key: &str,
    body: &TrimmedBody,
    max: usize,
) -> String {
    let title: String = title.chars().take(200).collect();
    let url_with = |text: &str| {
        let mut all = fields.to_vec();
        all.push((key, text));
        issue_url(&title, &all)
    };
    let n = body.lines.len();
    // The most lines that fit: the link only grows with more lines, so a
    // binary search finds it (a line-by-line search took seconds on a long
    // backtrace).
    let full = url_with(&body.keeping(n));
    if full.len() <= max {
        return full;
    }
    let (mut fits, mut too_long) = (None, n);
    let mut low = 0;
    while low < too_long {
        let mid = (low + too_long) / 2;
        let url = url_with(&body.keeping(mid));
        if url.len() <= max {
            fits = Some(url);
            low = mid + 1;
        } else {
            too_long = mid;
        }
    }
    if let Some(url) = fits {
        return url;
    }
    // Even with no lines the fixed text is too long: cut the longest text
    // (usually the body's head; a huge panic message in `what`) by
    // characters until it fits.
    let mut texts: Vec<(&str, String)> =
        fields.iter().map(|(k, v)| (*k, (*v).to_owned())).collect();
    texts.push((key, format!("{}{}", body.head, body.tail)));
    loop {
        let pairs: Vec<(&str, &str)> = texts.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let url = issue_url(&title, &pairs);
        if url.len() <= max {
            return url;
        }
        let Some((_, text)) = texts
            .iter_mut()
            .max_by_key(|(_, v)| v.len())
            .filter(|(_, v)| !v.is_empty())
        else {
            return url;
        };
        let cut = text.chars().count() * 9 / 10;
        *text = text.chars().take(cut).collect();
    }
}
fn push_lines(text: &mut String, lines: &[String]) {
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
}

/// The Environment field of a manual report: build and system, then the
/// recent log (newest lines kept).
pub fn bug_report_environment<'a>(env: &Environment, logs: &'a [String]) -> TrimmedBody<'a> {
    TrimmedBody {
        head: format!("{}\nRecent log:\n", env.text()),
        lines: logs,
        keep_newest: true,
        tail: String::new(),
    }
}

/// Help > Report Bug or Idea…'s link: the bug form with only Environment
/// filled in (what happened, steps and expected are the user's to write).
pub fn bug_report_url(gpu: &str) -> String {
    let logs = log_ring::tail(LOG_LINES);
    let env = Environment::current(gpu);
    fit_issue_url(
        "Bug: ",
        &[],
        FIELD_ENVIRONMENT,
        &bug_report_environment(&env, &logs),
        MAX_URL,
    )
}

// ---------------------------------------------------------------- open / reveal

/// Opens `url` in the default browser.
pub fn open_url(url: &str) -> std::io::Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    } else if cfg!(target_os = "windows") {
        // `start` treats its first quoted argument as a window title.
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]).arg(url);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    command.spawn().map(|_| ())
}

/// Shows `path` selected in Finder (Explorer / the file manager elsewhere).
pub fn reveal(path: &Path) -> std::io::Result<()> {
    let mut command = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(path);
        c
    } else if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("explorer");
        c.arg(format!("/select,{}", path.display()));
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path.parent().unwrap_or(path));
        c
    };
    command.spawn().map(|_| ())
}

// ---------------------------------------------------------------- time

/// Seconds since 1970 as "2026-11-05T14:51:04Z".
pub fn utc_timestamp(secs: u64) -> String {
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60
    )
}
/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn modified_secs(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

// ---------------------------------------------------------------- crash files

/// One Rust panic, as written to and read back from a crash file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrashRecord {
    pub time: String,
    pub version: String,
    pub commit: String,
    pub os: String,
    pub gpu: String,
    pub thread: String,
    pub location: String,
    pub message: String,
    pub backtrace: String,
    pub log: Vec<String>,
}

const CRASH_HEADER: &str = "Operandi crash report";

/// The crash file's text: header fields, then message, backtrace and log
/// sections. Plain text so a person can read it before attaching it.
pub fn crash_file_text(r: &CrashRecord) -> String {
    let mut s = format!(
        "{CRASH_HEADER}\ntime: {}\nversion: {}\ncommit: {}\nos: {}\ngpu: {}\nthread: {}\nlocation: {}\n",
        r.time, r.version, r.commit, r.os, r.gpu, r.thread, r.location
    );
    let _ = write!(
        s,
        "\n--- message ---\n{}\n\n--- backtrace ---\n{}\n\n--- recent log ---\n",
        r.message.trim_end(),
        r.backtrace.trim_end()
    );
    push_lines(&mut s, &r.log);
    s
}

/// Reads a crash file back; None if it isn't one.
pub fn parse_crash_file(text: &str) -> Option<CrashRecord> {
    let mut lines = text.lines();
    if lines.next()? != CRASH_HEADER {
        return None;
    }
    let mut r = CrashRecord::default();
    let mut section: Option<&str> = None;
    let mut message = Vec::new();
    let mut backtrace = Vec::new();
    for line in lines {
        if let Some(name) = line
            .strip_prefix("--- ")
            .and_then(|l| l.strip_suffix(" ---"))
        {
            section = Some(match name {
                "message" => "message",
                "backtrace" => "backtrace",
                "recent log" => "log",
                _ => "other",
            });
            continue;
        }
        match section {
            None => {
                if let Some((key, value)) = line.split_once(": ") {
                    let value = value.to_owned();
                    match key {
                        "time" => r.time = value,
                        "version" => r.version = value,
                        "commit" => r.commit = value,
                        "os" => r.os = value,
                        "gpu" => r.gpu = value,
                        "thread" => r.thread = value,
                        "location" => r.location = value,
                        _ => {}
                    }
                }
            }
            Some("message") => message.push(line),
            Some("backtrace") => backtrace.push(line),
            Some("log") if !line.is_empty() => r.log.push(line.to_owned()),
            _ => {}
        }
    }
    r.message = message.join("\n").trim().to_owned();
    r.backtrace = backtrace
        .join("\n")
        .trim_matches('\n')
        .trim_end()
        .to_owned();
    Some(r)
}

/// "crash-20261105-145104-1234.txt": sorts by time, unique per process.
pub fn crash_file_name(secs: u64, pid: u32) -> String {
    let stamp = utc_timestamp(secs);
    let compact: String = stamp
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'T')
        .collect::<String>()
        .replace('T', "-");
    format!("crash-{compact}-{pid}.txt")
}
fn is_crash_file_name(name: &str) -> bool {
    name.starts_with("crash-") && name.ends_with(".txt")
}

/// Writes one crash file into `dir` (creating it) and returns its path.
pub fn write_crash_file(dir: &Path, record: &CrashRecord, secs: u64) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(crash_file_name(secs, std::process::id()));
    std::fs::write(&path, crash_file_text(record))?;
    Ok(path)
}

/// Backtrace frames from Operandi's own code (with their "at" lines), for
/// the issue link; falls back to the whole trace when none match (for
/// example a stripped build). The crash file keeps every frame.
pub fn operandi_frames(backtrace: &str) -> Vec<String> {
    let mut frames: Vec<Vec<&str>> = Vec::new();
    for line in backtrace.lines() {
        let trimmed = line.trim_start();
        let is_frame = trimmed
            .split_once(": ")
            .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if is_frame || frames.is_empty() {
            frames.push(vec![line]);
        } else if let Some(frame) = frames.last_mut() {
            frame.push(line);
        }
    }
    let own: Vec<String> = frames
        .iter()
        .filter(|f| f[0].contains("operandi"))
        .flatten()
        .map(|l| l.to_string())
        .collect();
    if own.is_empty() {
        backtrace.lines().map(str::to_owned).collect()
    } else {
        own
    }
}

// ---------------------------------------------------------------- panic hook

struct HookContext {
    dir: PathBuf,
    gpu: String,
}
static HOOK: OnceLock<Mutex<HookContext>> = OnceLock::new();

/// Installs the crash-file panic hook in front of the default one (which
/// still prints the panic). `dir` is `<data folder>/crashes`.
pub fn install_panic_hook(dir: PathBuf) {
    // Work out the OS now: the hook shouldn't start processes.
    let _ = os_description();
    if HOOK
        .set(Mutex::new(HookContext {
            dir,
            gpu: "not known yet".into(),
        }))
        .is_err()
    {
        return;
    }
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        write_panic(info);
        default(info);
    }));
}

/// Tells the panic hook which GPU the app is using.
pub fn set_gpu(gpu: &str) {
    if let Some(context) = HOOK.get() {
        context.lock().unwrap_or_else(|e| e.into_inner()).gpu = gpu.into();
    }
}

fn write_panic(info: &std::panic::PanicHookInfo) {
    thread_local! {
        static IN_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    // A panic while writing the file must not loop.
    if IN_HOOK.with(|f| f.replace(true)) {
        return;
    }
    let Some(context) = HOOK.get() else {
        return;
    };
    let (dir, gpu) = match context.try_lock() {
        Ok(c) => (c.dir.clone(), c.gpu.clone()),
        Err(std::sync::TryLockError::Poisoned(e)) => {
            let c = e.into_inner();
            (c.dir.clone(), c.gpu.clone())
        }
        Err(std::sync::TryLockError::WouldBlock) => return,
    };
    let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "(no message)".to_owned()
    };
    let secs = now_secs();
    let record = CrashRecord {
        time: utc_timestamp(secs),
        version: VERSION.into(),
        commit: COMMIT.into(),
        os: os_description().into(),
        gpu,
        thread: std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_owned(),
        location: info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into()),
        message,
        backtrace: std::backtrace::Backtrace::force_capture().to_string(),
        log: log_ring::try_tail(LOG_LINES),
    };
    match write_crash_file(&dir, &record, secs) {
        Ok(path) => eprintln!("Operandi crash report written to {}", path.display()),
        Err(e) => eprintln!("Operandi could not write a crash report: {e}"),
    }
    IN_HOOK.with(|f| f.set(false));
}

// ---------------------------------------------------------------- test crash

/// QA/dev only: `OPERANDI_TEST_PANIC=1` panics after the first frame;
/// `OPERANDI_TEST_PANIC=abort` aborts instead (a native crash, for the
/// DiagnosticReports path). Packaged release builds ignore it.
pub fn test_crash_after_first_frame() {
    static MODE: OnceLock<Option<String>> = OnceLock::new();
    static FRAMES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let mode = MODE.get_or_init(|| {
        if option_env!("OPERANDI_MILESTONE1_PACKAGE") == Some("1") {
            return None;
        }
        std::env::var("OPERANDI_TEST_PANIC")
            .ok()
            .filter(|v| !v.is_empty() && v != "0")
    });
    let Some(mode) = mode else {
        return;
    };
    if FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) == 1 {
        log_ring::log_line!("OPERANDI_TEST_PANIC={mode}: crashing on purpose");
        if mode == "abort" {
            std::process::abort();
        }
        panic!("OPERANDI_TEST_PANIC: deliberate test crash");
    }
}

// ---------------------------------------------------------------- native reports

/// Whether a DiagnosticReports file belongs to `exe` ("operandi-app"):
/// macOS names them "<process>-<date>.ips" (older systems ".crash").
pub fn native_report_matches(file_name: &str, exe: &str) -> bool {
    let Some(rest) = file_name.strip_prefix(exe) else {
        return false;
    };
    (rest.starts_with('-') || rest.starts_with('_') || rest.starts_with('.'))
        && (file_name.ends_with(".ips") || file_name.ends_with(".crash"))
}

/// A short description of a native crash report: its exception type and
/// signal, else its first non-empty lines.
pub fn native_summary(text: &str) -> String {
    // .ips: a JSON header line, then a JSON body with
    // "exception" : {"type":"EXC_BAD_ACCESS", "signal":"SIGSEGV", ...}.
    if let Some(at) = text.find("\"exception\"") {
        let object = &text[at..];
        let object = &object[..object.find('}').unwrap_or(object.len())];
        let field = |key: &str| {
            let pattern = format!("\"{key}\"");
            let after = &object[object.find(&pattern)? + pattern.len()..];
            let after = after.trim_start().strip_prefix(':')?.trim_start();
            let after = after.strip_prefix('"')?;
            Some(after[..after.find('"')?].to_owned())
        };
        let parts: Vec<String> = [field("type"), field("signal")]
            .into_iter()
            .flatten()
            .collect();
        if !parts.is_empty() {
            return format!("Exception: {}", parts.join(" / "));
        }
    }
    // .crash: "Exception Type:  EXC_BAD_ACCESS (SIGSEGV)".
    if let Some(line) = text.lines().find(|l| l.starts_with("Exception Type:")) {
        return line.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(3)
        .map(|l| l.chars().take(160).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The folder macOS writes native crash reports to.
pub fn diagnostic_reports_dir() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Logs/DiagnosticReports"))
}

// ---------------------------------------------------------------- pending crashes

/// What the crash prompt remembers, in `crashes/state.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrashState {
    /// Crash files (by name) the user reported or said not to ask about.
    #[serde(default)]
    pub handled: Vec<String>,
    /// Native reports older than this (seconds since 1970) are never offered:
    /// it's set the first time the app looks, so old crashes from before this
    /// feature (or from other builds long ago) don't all come up at once.
    #[serde(default)]
    pub native_since: Option<u64>,
}
impl CrashState {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        crate::resource_write::write_bytes(path, &temp, &bytes)
    }
    pub fn is_handled(&self, name: &str) -> bool {
        self.handled.iter().any(|h| h == name)
    }
    pub fn mark(&mut self, name: &str) {
        if !self.is_handled(name) {
            self.handled.push(name.to_owned());
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashKind {
    /// A Rust panic file this app wrote.
    Panic,
    /// A macOS DiagnosticReports file.
    Native,
}

/// One crash waiting for the user's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingCrash {
    pub path: PathBuf,
    pub kind: CrashKind,
    /// One or two lines for the dialog.
    pub summary: String,
    pub url: String,
}
impl PendingCrash {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

/// The issue link for a panic crash file: the panic in What happened; the
/// environment, Operandi's backtrace frames and the log in Environment.
pub fn panic_issue_url(record: &CrashRecord, file_name: &str) -> String {
    let first_line = record.message.lines().next().unwrap_or("");
    let title = format!("Crash: {}", first_line.chars().take(80).collect::<String>());
    let env = Environment {
        version: record.version.clone(),
        commit: record.commit.clone(),
        os: record.os.clone(),
        gpu: record.gpu.clone(),
    };
    let what = format!(
        "Operandi crashed (Rust panic) at {}.\n\n\
         **Please drag the crash file `{file_name}` (shown in Finder) into this issue.**\n\n\
         ```text\n{}\nat {} (thread {})\n```\n",
        record.time,
        record.message.trim(),
        record.location,
        record.thread,
    );
    // Frames first, then the log: cutting to fit drops the log before the
    // top frames.
    let mut lines = operandi_frames(&record.backtrace);
    if !record.log.is_empty() {
        lines.push(String::new());
        lines.push("Recent log:".into());
        lines.extend(record.log.iter().cloned());
    }
    let environment = TrimmedBody {
        head: format!("{}\nBacktrace (Operandi frames):\n", env.text()),
        lines: &lines,
        keep_newest: false,
        tail: String::new(),
    };
    fit_issue_url(
        &title,
        &[(FIELD_WHAT, &what)],
        FIELD_ENVIRONMENT,
        &environment,
        MAX_URL,
    )
}

/// The issue link for a native (macOS) crash report: its summary in What
/// happened, the reporting app's environment in Environment (the crash
/// report holds the rest).
pub fn native_issue_url(file_name: &str, summary: &str, env: &Environment) -> String {
    let what = format!(
        "Operandi crashed (native crash; macOS crash report `{file_name}`).\n\n\
         **Please drag the crash report `{file_name}` (shown in Finder) into this issue.**\n\n\
         ```text\n{}\n```\n",
        summary.trim_end()
    );
    let environment = TrimmedBody {
        head: format!("Reported by:\n{}", env.text()),
        lines: &[],
        keep_newest: false,
        tail: String::new(),
    };
    fit_issue_url(
        "Crash (native): ",
        &[(FIELD_WHAT, &what)],
        FIELD_ENVIRONMENT,
        &environment,
        MAX_URL,
    )
}

/// Every crash not yet answered: panic files in `crash_dir`, then (macOS)
/// native reports for `exe` in `native_dir` newer than `state.native_since`.
/// Updates `state`: sets `native_since` on first use, marks native reports
/// that duplicate a panic file, and forgets handled files that are gone.
pub fn find_pending(
    crash_dir: &Path,
    native_dir: Option<&Path>,
    exe: &str,
    state: &mut CrashState,
    now: u64,
    gpu: &str,
) -> Vec<PendingCrash> {
    let mut pending = Vec::new();
    let mut panic_times = Vec::new();
    let mut present = Vec::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(crash_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    names.sort();
    for path in names {
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        if !is_crash_file_name(&name) {
            continue;
        }
        present.push(name.clone());
        if let Some(t) = modified_secs(&path) {
            panic_times.push(t);
        }
        if state.is_handled(&name) {
            continue;
        }
        let Some(record) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| parse_crash_file(&t))
        else {
            continue;
        };
        pending.push(PendingCrash {
            summary: format!(
                "{}\nat {}",
                record.message.lines().next().unwrap_or(""),
                record.location
            ),
            url: panic_issue_url(&record, &name),
            path,
            kind: CrashKind::Panic,
        });
    }
    if let Some(native_dir) = native_dir {
        let since = *state.native_since.get_or_insert(now);
        let mut reports: Vec<(u64, PathBuf, String)> = std::fs::read_dir(native_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_owned();
                native_report_matches(&name, exe).then_some(())?;
                let path = e.path();
                Some((modified_secs(&path)?, path, name))
            })
            .collect();
        reports.sort();
        let env = Environment::current(gpu);
        for (time, path, name) in reports {
            present.push(name.clone());
            if time < since || state.is_handled(&name) {
                continue;
            }
            if panic_times
                .iter()
                .any(|&p| time >= p && time - p <= SAME_CRASH_SECS)
            {
                // The abort after a panic: the panic file already covers it.
                state.mark(&name);
                continue;
            }
            let summary = read_prefix(&path)
                .map(|t| native_summary(&t))
                .unwrap_or_default();
            pending.push(PendingCrash {
                url: native_issue_url(&name, &summary, &env),
                summary: format!("macOS crash report {name}\n{summary}"),
                path,
                kind: CrashKind::Native,
            });
        }
    }
    state.handled.retain(|h| present.contains(h));
    pending
}
fn read_prefix(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_NATIVE_READ)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// This executable's file name, which macOS uses for its crash reports.
pub fn executable_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "operandi-app".into())
}

// ---------------------------------------------------------------- the prompt

/// "Operandi crashed last time. Report it?" — shown at launch while any
/// crash is unanswered.
#[derive(Default)]
pub struct CrashPrompt {
    pending: Vec<PendingCrash>,
    state: CrashState,
    state_path: PathBuf,
    /// "Not now": hide every prompt until the next launch.
    snoozed: bool,
}

/// What the user chose in the prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrashAnswer {
    Report,
    NotNow,
    DontAsk,
}

impl CrashPrompt {
    /// Looks for unanswered crashes in `<data folder>/crashes` and, on macOS,
    /// DiagnosticReports.
    pub fn at_launch(crash_dir: &Path, gpu: &str) -> Self {
        let state_path = crash_dir.join(STATE_FILE);
        let mut state = CrashState::load(&state_path);
        let native = diagnostic_reports_dir();
        let pending = find_pending(
            crash_dir,
            native.as_deref(),
            &executable_name(),
            &mut state,
            now_secs(),
            gpu,
        );
        if let Err(e) = state.save(&state_path) {
            log_ring::log_line!("Crash report state not saved: {e}");
        }
        Self {
            pending,
            state,
            state_path,
            snoozed: false,
        }
    }

    pub fn is_open(&self) -> bool {
        !self.snoozed && !self.pending.is_empty()
    }

    /// Applies an answer to the first pending crash. Report opens the issue
    /// and reveals the file; Report and Don't Ask mark it answered.
    /// Returns a status line for the app.
    pub fn answer(&mut self, answer: CrashAnswer) -> Option<String> {
        let crash = self.pending.first()?.clone();
        let status = match answer {
            CrashAnswer::NotNow => {
                self.snoozed = true;
                return None;
            }
            CrashAnswer::Report => {
                let opened = open_url(&crash.url);
                let _ = reveal(&crash.path);
                Some(match opened {
                    Ok(()) => format!(
                        "Opened a crash issue in your browser; drag {} into it",
                        crash.name()
                    ),
                    Err(e) => format!("Couldn't open the browser: {e}"),
                })
            }
            CrashAnswer::DontAsk => None,
        };
        self.state.mark(&crash.name());
        if let Err(e) = self.state.save(&self.state_path) {
            log_ring::log_line!("Crash report state not saved: {e}");
        }
        self.pending.remove(0);
        status
    }

    pub fn ui(&mut self, ctx: &Context) -> Option<String> {
        if !self.is_open() {
            return None;
        }
        let crash = &self.pending[0];
        let count = self.pending.len();
        let of = if count > 1 {
            format!(" (1 of {count} crashes)")
        } else {
            String::new()
        };
        // UIC-97: a confirmation in the shared dialog frame. OK reports,
        // Cancel is "not now", and the extra button stops asking about
        // this crash.
        let message = format!(
            "Operandi crashed last time{of}: {}\n\nOK opens a new GitHub issue in your \
             browser with a summary filled in and shows the crash file in Finder so you \
             can drag it in. Nothing is sent until you submit the issue yourself.",
            crash.summary
        );
        let key = format!("Report Crash {}", crash.name());
        let outcome = crate::form::Dialog::show_kept(ctx, &key, || {
            crate::form::Dialog::confirm("Report Crash", &message).extras(&["Don't Ask"])
        });
        let answer = match outcome {
            crate::form::Outcome::Ok => Some(CrashAnswer::Report),
            crate::form::Outcome::Cancel => Some(CrashAnswer::NotNow),
            crate::form::Outcome::Extra(_) => Some(CrashAnswer::DontAsk),
            crate::form::Outcome::Open | crate::form::Outcome::No => None,
        };
        answer.and_then(|a| self.answer(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "operandi-bug-report-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn env() -> Environment {
        Environment {
            version: "0.1.0".into(),
            commit: "abc1234+dirty".into(),
            os: "macOS 15.2 (aarch64)".into(),
            gpu: "Metal · Apple M3".into(),
        }
    }
    fn has(url: &str, key: &str) -> bool {
        url.contains(&format!("?{key}=")) || url.contains(&format!("&{key}="))
    }
    fn query(url: &str, key: &str) -> String {
        let start = url
            .find(&format!("?{key}="))
            .or_else(|| url.find(&format!("&{key}=")))
            .unwrap_or_else(|| panic!("no {key} in {url}"))
            + key.len()
            + 2;
        let raw = &url[start..];
        let raw = &raw[..raw.find('&').unwrap_or(raw.len())];
        let mut bytes = Vec::new();
        let mut it = raw.bytes();
        while let Some(b) = it.next() {
            if b == b'%' {
                let hex = [it.next().unwrap(), it.next().unwrap()];
                bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).unwrap(), 16).unwrap());
            } else {
                bytes.push(b);
            }
        }
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn issue_links_open_the_bug_form_and_encode_everything() {
        assert_eq!(percent_encode("a b&c=d/é#"), "a%20b%26c%3Dd%2F%C3%A9%23");
        let url = issue_url("Bug: x & y", &[("what", "line 1\nline #2"), ("steps", "")]);
        assert!(url.starts_with(
            "https://github.com/operandi3d/operandi/issues/new?template=bug.yml&title="
        ));
        assert!(!url[url.find('?').unwrap()..].contains(' '));
        assert_eq!(query(&url, "title"), "Bug: x & y");
        assert_eq!(query(&url, "what"), "line 1\nline #2");
        assert!(!has(&url, "steps"), "empty fields are left to the form");
        assert!(!has(&url, "body"), "issue forms take field ids, not a body");
    }

    #[test]
    fn bug_link_fills_only_environment_with_the_newest_log_lines() {
        let logs: Vec<String> = (0..30).map(|i| format!("12:00:{i:02} line {i}")).collect();
        let url = fit_issue_url(
            "Bug: ",
            &[],
            FIELD_ENVIRONMENT,
            &bug_report_environment(&env(), &logs),
            MAX_URL,
        );
        assert!(url.contains("?template=bug.yml&"), "{url}");
        assert_eq!(query(&url, "title"), "Bug: ");
        for key in ["what", "steps", "expected"] {
            assert!(!has(&url, key), "{key} is the user's to write");
        }
        let environment = query(&url, "environment");
        for part in [
            "Operandi: 0.1.0 (abc1234+dirty)\n",
            "OS: macOS 15.2 (aarch64)\n",
            "GPU: Metal · Apple M3\n",
            "Recent log:\n",
            "line 0\n",
            "line 29\n",
        ] {
            assert!(
                environment.contains(part),
                "missing {part:?} in {environment}"
            );
        }
        assert!(!environment.contains("```"), "a text block needs no fences");
        assert!(!environment.contains("left out"), "30 short lines fit");
    }

    #[test]
    fn long_reports_are_cut_to_fit_with_a_note() {
        let logs: Vec<String> = (0..300)
            .map(|i| format!("{i:04} {}", "x".repeat(60)))
            .collect();
        let url = fit_issue_url(
            "Bug: ",
            &[],
            FIELD_ENVIRONMENT,
            &bug_report_environment(&env(), &logs),
            MAX_URL,
        );
        assert!(url.len() <= MAX_URL, "{}", url.len());
        let environment = query(&url, "environment");
        assert!(environment.contains("lines left out to fit the link"));
        assert!(environment.contains("0299 "), "the newest log line is kept");
        assert!(!environment.contains("0000 "), "the oldest is dropped");
        assert!(
            environment.starts_with("Operandi: 0.1.0"),
            "the environment lines survive"
        );

        // A backtrace keeps its top frames instead; the other fields stay whole.
        let frames: Vec<String> = (0..300).map(|i| format!("{i:4}: frame_{i:04}")).collect();
        let body = TrimmedBody {
            head: "head\n".into(),
            lines: &frames,
            keep_newest: false,
            tail: "tail\n".into(),
        };
        let url = fit_issue_url("t", &[("what", "summary")], "environment", &body, 3000);
        assert!(url.len() <= 3000);
        assert_eq!(query(&url, "what"), "summary");
        let text = query(&url, "environment");
        assert!(text.contains("frame_0000") && !text.contains("frame_0299"));
        assert!(
            text.ends_with(
                "left out to fit the link; the full text is in the attached file.\ntail\n"
            )
        );

        // Even oversized fixed parts are cut rather than overflowing.
        let huge = TrimmedBody {
            head: "y".repeat(20_000),
            lines: &[],
            keep_newest: true,
            tail: String::new(),
        };
        let what = "z".repeat(20_000);
        let url = fit_issue_url("t", &[("what", &what)], "environment", &huge, MAX_URL);
        assert!(url.len() <= MAX_URL, "{}", url.len());
        assert!(has(&url, "what") && has(&url, "environment"));
    }

    #[test]
    fn crash_files_round_trip_and_ignore_other_files() {
        let record = CrashRecord {
            time: utc_timestamp(1_793_890_264),
            version: "0.1.0".into(),
            commit: "abc1234".into(),
            os: "macOS 15.2 (aarch64)".into(),
            gpu: "Metal · Apple M3".into(),
            thread: "main".into(),
            location: "crates/operandi-app/src/main.rs:10:5".into(),
            message: "index out of bounds\nsecond line".into(),
            backtrace: "   0: std::panicking::begin\n   1: operandi_app::main\n             at src/main.rs:10:5".into(),
            log: vec!["14:51:00 status: Ready".into(), "14:51:03 drag".into()],
        };
        let text = crash_file_text(&record);
        assert!(text.starts_with("Operandi crash report\ntime: 2026-11-05T"));
        assert_eq!(parse_crash_file(&text), Some(record.clone()));
        assert_eq!(parse_crash_file("hello\nworld"), None);

        let dir = temp("write");
        let path = write_crash_file(&dir.join("crashes"), &record, 1_793_890_264).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(is_crash_file_name(name), "{name}");
        assert!(name.starts_with("crash-20261105-"), "{name}");
        let back = parse_crash_file(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, record);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn utc_dates_are_right() {
        assert_eq!(utc_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc_timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc_timestamp(1_793_890_264), "2026-11-05T14:51:04Z");
    }

    #[test]
    fn issue_backtraces_keep_operandi_frames() {
        let trace = "   0: std::backtrace::Backtrace::force_capture\n             at /rustc/std/src/backtrace.rs:1\n   1: operandi_app::viewport::draw\n             at ./src/viewport.rs:42:9\n   2: eframe::run\n   3: operandi_app::main";
        assert_eq!(
            operandi_frames(trace),
            vec![
                "   1: operandi_app::viewport::draw",
                "             at ./src/viewport.rs:42:9",
                "   3: operandi_app::main",
            ]
        );
        // A stripped trace with no names: keep it whole.
        assert_eq!(operandi_frames("   0: <unknown>").len(), 1);
    }

    #[test]
    fn panic_issue_link_has_summary_and_asks_for_the_file() {
        let record = CrashRecord {
            time: "2026-11-05T14:51:04Z".into(),
            message: "OPERANDI_TEST_PANIC: deliberate test crash".into(),
            location: "src/bug_report.rs:1:1".into(),
            thread: "main".into(),
            backtrace: (0..2000)
                .map(|i| format!("{i:4}: operandi_app::f{i}\n             at src/f.rs:{i}:1"))
                .collect::<Vec<_>>()
                .join("\n"),
            ..Default::default()
        };
        let url = panic_issue_url(&record, "crash-x.txt");
        assert!(url.len() <= MAX_URL);
        assert!(url.contains("?template=bug.yml&"), "{url}");
        assert_eq!(
            query(&url, "title"),
            "Crash: OPERANDI_TEST_PANIC: deliberate test crash"
        );
        let what = query(&url, "what");
        assert!(what.contains("drag the crash file `crash-x.txt`"));
        assert!(what.contains("at src/bug_report.rs:1:1 (thread main)"));
        let environment = query(&url, "environment");
        assert!(environment.starts_with("Operandi: "), "{environment}");
        assert!(environment.contains("Backtrace (Operandi frames):\n"));
        assert!(environment.contains("operandi_app::f0\n") && environment.contains("left out"));

        // A short trace fits whole, with the log after the frames.
        let record = CrashRecord {
            backtrace: "   0: operandi_app::main".into(),
            log: vec!["14:51:03 drag".into()],
            ..record
        };
        let environment = query(&panic_issue_url(&record, "crash-x.txt"), "environment");
        let frame = environment.find("operandi_app::main").unwrap();
        let log = environment.find("Recent log:\n14:51:03 drag\n").unwrap();
        assert!(frame < log, "{environment}");
    }

    #[test]
    fn native_reports_match_this_executable_only() {
        assert!(native_report_matches(
            "operandi-app-2026-11-05-145104.ips",
            "operandi-app"
        ));
        assert!(native_report_matches(
            "operandi-app_2026-11-05-145104_Mac.crash",
            "operandi-app"
        ));
        assert!(!native_report_matches(
            "operandi-app-helper-2026.txt",
            "operandi-app"
        ));
        assert!(!native_report_matches(
            "operandi-application-2026.ips",
            "operandi-app"
        ));
        assert!(!native_report_matches(
            "Finder-2026-11-05.ips",
            "operandi-app"
        ));
    }

    #[test]
    fn native_summaries_name_the_exception() {
        let ips = "{\"app_name\":\"operandi-app\",\"bug_type\":\"309\"}\n{\n  \"exception\" : {\"codes\":\"0x1\",\"type\":\"EXC_BAD_ACCESS\",\"signal\":\"SIGSEGV\",\"subtype\":\"KERN_INVALID_ADDRESS\"},\n  \"threads\" : []\n}";
        assert_eq!(native_summary(ips), "Exception: EXC_BAD_ACCESS / SIGSEGV");
        let crash = "Process: operandi-app [123]\nException Type:  EXC_CRASH (SIGABRT)\n";
        assert_eq!(native_summary(crash), "Exception Type: EXC_CRASH (SIGABRT)");
        assert_eq!(
            native_summary("\nfirst\nsecond\nthird\nfourth"),
            "first\nsecond\nthird"
        );
    }

    #[test]
    fn pending_crashes_respect_answers_first_use_and_duplicates() {
        let root = temp("pending");
        let crashes = root.join("crashes");
        let native = root.join("DiagnosticReports");
        std::fs::create_dir_all(&native).unwrap();
        let record = CrashRecord {
            message: "boom".into(),
            location: "a.rs:1:1".into(),
            ..Default::default()
        };
        let panic_path = write_crash_file(&crashes, &record, 1).unwrap();
        let panic_name = panic_path.file_name().unwrap().to_str().unwrap().to_owned();
        std::fs::write(crashes.join("notes.txt"), "not a crash").unwrap();
        let mut state = CrashState::default();

        // First look: the panic file is offered; native reports that already
        // exist are not (native_since starts now).
        std::fs::write(native.join("operandi-app-old.ips"), "{}").unwrap();
        let now = now_secs() + 1;
        let found = find_pending(
            &crashes,
            Some(&native),
            "operandi-app",
            &mut state,
            now,
            "gpu",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, CrashKind::Panic);
        assert_eq!(found[0].summary, "boom\nat a.rs:1:1");
        assert_eq!(state.native_since, Some(now));

        // A later native report is offered; one right after a panic file is
        // the same crash and is marked answered instead.
        let mut state = CrashState {
            native_since: Some(0),
            ..Default::default()
        };
        std::fs::write(
            native.join("operandi-app-new.ips"),
            "{\"exception\":{\"type\":\"EXC_BAD_ACCESS\",\"signal\":\"SIGSEGV\"}}",
        )
        .unwrap();
        std::fs::write(native.join("Other-new.ips"), "{}").unwrap();
        let found = find_pending(
            &crashes,
            Some(&native),
            "operandi-app",
            &mut state,
            now,
            "gpu",
        );
        // All three files were written just now, so the native ones count as
        // the panic's duplicates.
        assert_eq!(found.len(), 1);
        assert!(state.is_handled("operandi-app-new.ips"));

        // Without the panic file nearby, the native one is offered.
        let mut state = CrashState {
            native_since: Some(0),
            handled: vec![panic_name.clone()],
        };
        std::fs::remove_file(&panic_path).unwrap();
        let found = find_pending(
            &crashes,
            Some(&native),
            "operandi-app",
            &mut state,
            now,
            "gpu",
        );
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().all(|c| c.kind == CrashKind::Native));
        let new = found
            .iter()
            .find(|c| c.name() == "operandi-app-new.ips")
            .unwrap();
        assert!(new.summary.contains("EXC_BAD_ACCESS / SIGSEGV"));
        assert!(query(&new.url, "what").contains("`operandi-app-new.ips`"));
        assert!(query(&new.url, "what").contains("EXC_BAD_ACCESS / SIGSEGV"));
        assert!(query(&new.url, "environment").contains("GPU: gpu\n"));
        assert!(
            !state.is_handled(&panic_name),
            "answers for files that are gone are forgotten"
        );

        // State survives a save and load.
        state.mark("operandi-app-new.ips");
        let path = crashes.join(STATE_FILE);
        state.save(&path).unwrap();
        assert_eq!(CrashState::load(&path), state);
        let found = find_pending(
            &crashes,
            Some(&native),
            "operandi-app",
            &mut state,
            now,
            "gpu",
        );
        assert_eq!(found.len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prompt_answers_snooze_or_mark_crashes() {
        let root = temp("prompt");
        let crashes = root.join("crashes");
        for secs in [1, 2] {
            let record = CrashRecord {
                message: format!("boom {secs}"),
                ..Default::default()
            };
            std::fs::create_dir_all(&crashes).unwrap();
            std::fs::write(
                crashes.join(crash_file_name(secs, 7)),
                crash_file_text(&record),
            )
            .unwrap();
        }
        let state_path = crashes.join(STATE_FILE);
        let mut state = CrashState::default();
        let pending = find_pending(&crashes, None, "operandi-app", &mut state, 10, "gpu");
        let mut prompt = CrashPrompt {
            pending,
            state,
            state_path: state_path.clone(),
            snoozed: false,
        };
        assert!(prompt.is_open());
        assert_eq!(prompt.answer(CrashAnswer::DontAsk), None);
        assert_eq!(prompt.pending.len(), 1);
        assert_eq!(CrashState::load(&state_path).handled.len(), 1);
        assert_eq!(prompt.answer(CrashAnswer::NotNow), None);
        assert!(!prompt.is_open(), "Not now hides the prompt this launch");
        assert_eq!(prompt.pending.len(), 1, "but keeps the crash for next time");
        let mut state = CrashState::load(&state_path);
        let again = find_pending(&crashes, None, "operandi-app", &mut state, 10, "gpu");
        assert_eq!(again.len(), 1, "the unanswered one is asked about again");
        assert!(again[0].summary.starts_with("boom 2"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn os_release_names_are_read() {
        assert_eq!(
            os_release_pretty_name("NAME=Ubuntu\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n").as_deref(),
            Some("Ubuntu 24.04 LTS")
        );
        assert!(!os_description().is_empty());
        assert!(!COMMIT.is_empty());
    }
}
