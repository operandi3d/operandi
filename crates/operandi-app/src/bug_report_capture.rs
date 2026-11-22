//! What Help > Report Bug or Idea… puts in a report besides the build and
//! system (BUG-01..23): a screenshot of the window, the tool and selection,
//! the recent commands and the recent log, each behind a checkbox. These
//! are plain functions (text, link fitting, the reports folder) with unit
//! tests; the dialog and capture live in report_bug_dialog.rs.
//!
//! Nothing here sends anything. The link opens GitHub's own form, which is
//! the preview: the user reads, edits and submits it there (BUG-15).
use crate::bug_report::{self, Environment, FIELD_ENVIRONMENT, FIELD_WHAT};
use operandi_core::CommandRecord;
use std::path::{Path, PathBuf};

/// Bug form field for the tool, selection and recent commands (BUG-10).
pub const FIELD_ACTIVITY: &str = "activity";
/// Whether the reports repository's bug form has the [`FIELD_ACTIVITY`]
/// textarea yet. Until it does, GitHub would drop that field from the
/// link, so the same text goes into the Environment field instead. Turn
/// on once the form has the field (operandi3d/operandi#1 and its
/// follow-up).
pub const ACTIVITY_FIELD_ON_FORM: bool = false;
/// Commands a report lists (BUG-12).
pub const COMMAND_LINES: usize = 20;
/// Subfolder of the data folder that holds report screenshots (BUG-07).
pub const REPORTS_FOLDER: &str = "reports";
/// Screenshots kept there; older ones are deleted on save (BUG-07, BUG-18).
pub const KEEP_SCREENSHOTS: usize = 10;
/// The report command, left out of its own Recent Commands (BUG-20).
pub const REPORT_COMMAND: &str = "operandi.help.reportBug";

/// The dialog's four checkboxes (BUG-02); all on by default (Q1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportOptions {
    pub screenshot: bool,
    pub tool_state: bool,
    pub commands: bool,
    pub log: bool,
}
impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            screenshot: true,
            tool_state: true,
            commands: true,
            log: true,
        }
    }
}

// ---------------------------------------------------------------- platform

/// The paste shortcut and the file manager's name, as the user's system
/// says them (BUG-22/23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Platform {
    pub paste: &'static str,
    pub files: &'static str,
}
pub const MAC: Platform = Platform {
    paste: "Cmd+V",
    files: "Finder",
};
pub const WINDOWS: Platform = Platform {
    paste: "Ctrl+V",
    files: "File Explorer",
};
pub const LINUX: Platform = Platform {
    paste: "Ctrl+V",
    files: "the file manager",
};
pub fn platform() -> Platform {
    if cfg!(target_os = "macos") {
        MAC
    } else if cfg!(target_os = "windows") {
        WINDOWS
    } else {
        LINUX
    }
}

/// The `what` field's hint when a screenshot is taken (BUG-10).
pub fn paste_hint(p: Platform) -> String {
    format!(
        "(Paste the screenshot here with {}: it's on your clipboard.)",
        p.paste
    )
}

// ---------------------------------------------------------------- home folder

/// The user's home folder, for [`hide_home`].
pub fn home_folder() -> Option<String> {
    let key = if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    };
    std::env::var(key)
        .ok()
        .map(|h| h.trim_end_matches(['/', '\\']).to_owned())
        .filter(|h| h.len() > 1)
}

/// `text` with every mention of the home folder `home` written as `~`
/// (BUG-16): `/Users/jason/x` and `C:\Users\Jason\x` (any case, either
/// slash) become `~/x` and `~\x`. `/Users/jasonx` is someone else's
/// folder and stays.
pub fn hide_home(text: &str, home: &str) -> String {
    let home = home.trim_end_matches(['/', '\\']);
    if home.len() < 2 {
        return text.to_owned();
    }
    // Windows paths are case-insensitive and appear with either slash.
    let windows = home.contains('\\') || home.as_bytes().get(1) == Some(&b':');
    let norm = |s: &str| -> String {
        if windows {
            s.replace('\\', "/").to_ascii_lowercase()
        } else {
            s.to_owned()
        }
    };
    let needle = norm(home);
    // `norm` keeps byte offsets (ASCII-only changes), so matches in the
    // normalised text are matches in `text`.
    let hay = norm(text);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = hay[at..].find(&needle) {
        let start = at + found;
        let end = start + needle.len();
        let next = text[end..].chars().next();
        let boundary = next.is_none_or(|c| !(c.is_alphanumeric() || "_-.".contains(c)));
        out.push_str(&text[at..start]);
        out.push_str(if boundary { "~" } else { &text[start..end] });
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

// ---------------------------------------------------------------- tool and selection

/// What the Tool and Selection block says (BUG-11): user-visible names and
/// counts, never item names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolState {
    /// The open tool's name, `None` when no tool is open.
    pub tool: Option<String>,
    /// "Polygons", "Edges", "Vertices" or "Items".
    pub mode: String,
    pub action_center: String,
    pub snapping: bool,
    /// How many of `mode`'s elements are selected.
    pub selected: usize,
    pub meshes: usize,
    pub polygons: usize,
    pub lights: usize,
    pub cameras: usize,
    /// Every other item (locators, backdrops, environments...).
    pub other_items: usize,
    pub layout: String,
}

/// "18,302".
pub fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", thousands(n), if n == 1 { one } else { many })
}

impl ToolState {
    pub fn lines(&self) -> Vec<String> {
        let tool = match &self.tool {
            Some(t) => format!("{t} ({})", self.mode),
            None => format!("none ({})", self.mode),
        };
        let (one, many) = match self.mode.as_str() {
            "Vertices" => ("vertex", "vertices"),
            "Edges" => ("edge", "edges"),
            "Items" => ("item", "items"),
            _ => ("polygon", "polygons"),
        };
        let selection = if self.selected == 0 {
            "nothing".to_owned()
        } else {
            count(self.selected, one, many)
        };
        let mut scene = vec![
            count(self.meshes, "mesh", "meshes"),
            count(self.polygons, "polygon", "polygons"),
        ];
        if self.lights > 0 {
            scene.push(count(self.lights, "light", "lights"));
        }
        if self.cameras > 0 {
            scene.push(count(self.cameras, "camera", "cameras"));
        }
        if self.other_items > 0 {
            scene.push(count(self.other_items, "other item", "other items"));
        }
        vec![
            format!(
                "Tool: {tool}, Action Center: {}, Snapping: {}",
                self.action_center,
                if self.snapping { "on" } else { "off" }
            ),
            format!("Selection: {selection}"),
            format!("Scene: {}", scene.join(", ")),
            format!("Layout: {}", self.layout),
        ]
    }
}

// ---------------------------------------------------------------- recent commands

fn is_report_line(line: &str) -> bool {
    operandi_core::command_registry::find_line(line).is_some_and(|(e, _)| e.name == REPORT_COMMAND)
}

/// The Recent Commands lines (BUG-12): the last [`COMMAND_LINES`] top-level
/// records, oldest first, without the report command itself. `undone`
/// says whether a record's step is on the redo list now. `live` is the
/// open tool's `tool.set` line while its session isn't applied yet. Home
/// folders are not hidden here; [`report_link`] does that for every field.
pub fn command_lines(
    records: &[CommandRecord],
    undone: impl Fn(&CommandRecord) -> bool,
    live: Option<&str>,
) -> Vec<String> {
    let mut lines: Vec<String> = records
        .iter()
        .filter(|r| !is_report_line(&r.line))
        .map(|r| {
            let source = format!("[{}]", r.source.label());
            let mut line = format!("{source:<6} {}", r.line);
            let pointer = operandi_core::command_registry::find_line(&r.line)
                .is_some_and(|(e, _)| matches!(e.name, "app.undo" | "app.redo"));
            if !r.ok {
                line.push_str("  -> failed");
                if !r.message.is_empty() {
                    line.push_str(": ");
                    line.push_str(&r.message);
                }
            } else if pointer && !r.message.is_empty() {
                line.push_str(&format!("  ({})", r.message));
            } else if undone(r) {
                line.push_str("  (undone)");
            }
            line
        })
        .collect();
    if let Some(live) = live {
        // The session's own lines wait for its drop; until then it is one
        // line (BUG-12).
        lines.push(format!("{:<6} {live}  (not applied yet)", "[live]"));
    }
    let skip = lines.len().saturating_sub(COMMAND_LINES);
    lines.drain(..skip);
    lines
}

// ---------------------------------------------------------------- the link

/// Everything a report can hold; the [`ReportOptions`] pick what goes in.
pub struct ReportParts<'a> {
    pub env: &'a Environment,
    pub tool: Vec<String>,
    pub commands: Vec<String>,
    pub log: Vec<String>,
}

fn note(cut: usize, of: usize) -> String {
    // Unlike a crash report there is no attached file to point to (BUG-14).
    format!("… {cut} of {of} lines left out to fit the link\n")
}
/// "heading\n" then the newest `keep` of `lines`, with a note when some
/// are left out.
fn section(heading: &str, lines: &[String], keep: usize) -> String {
    let mut text = format!("{heading}\n");
    if keep < lines.len() {
        text.push_str(&note(lines.len() - keep, lines.len()));
    }
    for line in &lines[lines.len() - keep..] {
        text.push_str(line);
        text.push('\n');
    }
    text
}

/// The bug form's fields for `parts` as `options` choose, keeping the
/// newest `keep_commands` commands and `keep_log` log lines.
fn report_fields(
    parts: &ReportParts,
    options: ReportOptions,
    p: Platform,
    keep_commands: usize,
    keep_log: usize,
) -> Vec<(&'static str, String)> {
    let mut activity = String::new();
    if options.tool_state {
        for line in &parts.tool {
            activity.push_str(line);
            activity.push('\n');
        }
    }
    if options.commands {
        if !activity.is_empty() {
            activity.push('\n');
        }
        activity.push_str(&section(
            "Recent commands (newest last):",
            &parts.commands,
            keep_commands,
        ));
    }
    let mut environment = parts.env.text();
    if !ACTIVITY_FIELD_ON_FORM && !activity.is_empty() {
        environment.push('\n');
        environment.push_str(&activity);
    }
    if options.log {
        environment.push('\n');
        environment.push_str(&section("Recent log:", &parts.log, keep_log));
    }
    let mut fields = vec![
        (
            FIELD_WHAT,
            if options.screenshot {
                paste_hint(p)
            } else {
                String::new()
            },
        ),
        (FIELD_ENVIRONMENT, environment),
    ];
    if ACTIVITY_FIELD_ON_FORM {
        fields.push((FIELD_ACTIVITY, activity));
    }
    fields
}

/// The bug form's link for a report (BUG-10, BUG-14): home folders shown
/// as `~` (BUG-16), and, to stay under `max`, the oldest log lines cut
/// first, then the oldest commands; the build, system and Tool and
/// Selection lines are never cut.
pub fn report_link(
    parts: &ReportParts,
    options: ReportOptions,
    p: Platform,
    home: Option<&str>,
    max: usize,
) -> String {
    let url = |keep_commands: usize, keep_log: usize| {
        let fields = report_fields(parts, options, p, keep_commands, keep_log);
        let fields: Vec<(&str, String)> = fields
            .into_iter()
            .map(|(k, v)| (k, home.map_or_else(|| v.clone(), |h| hide_home(&v, h))))
            .collect();
        let pairs: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
        bug_report::issue_url("Bug: ", &pairs)
    };
    let (nc, nl) = (parts.commands.len(), parts.log.len());
    let full = url(nc, nl);
    if full.len() <= max {
        return full;
    }
    // The most lines that fit; the link only grows with more lines.
    let most = |n: usize, make: &dyn Fn(usize) -> String| -> Option<String> {
        let (mut fits, mut low, mut high) = (None, 0, n + 1);
        while low < high {
            let mid = (low + high) / 2;
            let u = make(mid);
            if u.len() <= max {
                fits = Some(u);
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        fits
    };
    if let Some(u) = most(nl, &|k| url(nc, k)) {
        return u;
    }
    most(nc, &|k| url(k, 0)).unwrap_or_else(|| url(0, 0))
}

// ---------------------------------------------------------------- screenshots

/// "report-20261122-110404.png" for a save at `secs` (UTC, BUG-07).
pub fn screenshot_name(secs: u64) -> String {
    let stamp = bug_report::utc_timestamp(secs); // 2026-11-22T11:04:04Z
    let digits: String = stamp.chars().filter(char::is_ascii_digit).collect();
    format!("report-{}-{}.png", &digits[..8], &digits[8..])
}
fn is_screenshot_name(name: &str) -> bool {
    name.starts_with("report-") && name.ends_with(".png")
}

/// Writes `image` (straight RGBA, `width` x `height`) as a PNG in `dir`,
/// then deletes all but the newest [`KEEP_SCREENSHOTS`] report
/// screenshots there (BUG-07). Returns the new file.
pub fn save_screenshot(
    dir: &Path,
    width: u32,
    height: u32,
    rgba: &[u8],
    secs: u64,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(screenshot_name(secs));
    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        std::io::BufWriter::new(file),
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    );
    image::ImageEncoder::write_image(
        encoder,
        rgba,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| e.to_string())?;
    prune_screenshots(dir, KEEP_SCREENSHOTS);
    Ok(path)
}

/// Deletes the oldest report screenshots in `dir` until `keep` are left
/// (names sort by time). Other files are never touched.
pub fn prune_screenshots(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| is_screenshot_name(n))
        .collect();
    names.sort();
    let extra = names.len().saturating_sub(keep);
    for name in &names[..extra] {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bug_report::MAX_URL;
    use operandi_core::{CommandClass, Source};

    fn env() -> Environment {
        Environment {
            version: "0.1.0".into(),
            commit: "abc1234".into(),
            os: "macOS 15.2 (aarch64)".into(),
            gpu: "Metal".into(),
        }
    }
    fn record(line: &str, source: Source) -> CommandRecord {
        CommandRecord::new(line, CommandClass::Undoable, source, 0.)
    }
    fn query(url: &str, key: &str) -> Option<String> {
        let start = url
            .find(&format!("?{key}="))
            .or_else(|| url.find(&format!("&{key}=")))?
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
        Some(String::from_utf8(bytes).unwrap())
    }
    fn parts(env: &Environment) -> ReportParts<'_> {
        ReportParts {
            env,
            tool: ToolState {
                tool: Some("Bevel".into()),
                mode: "Polygons".into(),
                action_center: "Selection".into(),
                snapping: false,
                selected: 12,
                meshes: 4,
                polygons: 18_302,
                lights: 2,
                cameras: 1,
                other_items: 0,
                layout: "Model".into(),
            }
            .lines(),
            commands: vec!["[menu] select.typeFrom polygon".into()],
            log: vec!["10:00:00 opened /Users/jason/Desktop/mug.opscene".into()],
        }
    }
    /// All the report's text, whichever field it went to.
    fn all_text(url: &str) -> String {
        [FIELD_WHAT, FIELD_ENVIRONMENT, FIELD_ACTIVITY]
            .iter()
            .filter_map(|k| query(url, k))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn tool_and_selection_block_reads_like_the_spec() {
        let env = env();
        assert_eq!(
            parts(&env).tool,
            [
                "Tool: Bevel (Polygons), Action Center: Selection, Snapping: off",
                "Selection: 12 polygons",
                "Scene: 4 meshes, 18,302 polygons, 2 lights, 1 camera",
                "Layout: Model",
            ]
        );
        let none = ToolState {
            mode: "Items".into(),
            action_center: "Automatic".into(),
            meshes: 1,
            polygons: 6,
            layout: "Model".into(),
            ..ToolState::default()
        };
        assert_eq!(
            none.lines()[0],
            "Tool: none (Items), Action Center: Automatic, Snapping: off"
        );
        assert_eq!(none.lines()[1], "Selection: nothing");
        assert_eq!(none.lines()[2], "Scene: 1 mesh, 6 polygons");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn each_checkbox_adds_only_its_own_text() {
        let env = env();
        let p = parts(&env);
        let all = ReportOptions::default();
        let text = |o: ReportOptions| all_text(&report_link(&p, o, MAC, None, MAX_URL));
        let full = text(all);
        for needle in [
            "Cmd+V",
            "Tool: Bevel",
            "Recent commands",
            "Recent log:",
            "Operandi: 0.1.0",
        ] {
            assert!(full.contains(needle), "{needle} missing from {full}");
        }
        let without = |o: ReportOptions, gone: &str| {
            let t = text(o);
            assert!(!t.contains(gone), "{gone} still in {t}");
            assert!(
                t.contains("Operandi: 0.1.0") && t.contains("GPU: Metal"),
                "{t}"
            );
        };
        without(
            ReportOptions {
                screenshot: false,
                ..all
            },
            "Paste the screenshot",
        );
        without(
            ReportOptions {
                tool_state: false,
                ..all
            },
            "Tool: Bevel",
        );
        without(
            ReportOptions {
                commands: false,
                ..all
            },
            "Recent commands",
        );
        without(ReportOptions { log: false, ..all }, "Recent log");
        let none = ReportOptions {
            screenshot: false,
            tool_state: false,
            commands: false,
            log: false,
        };
        let url = report_link(&p, none, MAC, None, MAX_URL);
        assert!(
            query(&url, FIELD_WHAT).is_none(),
            "empty fields are left out"
        );
        assert!(query(&url, FIELD_ACTIVITY).is_none());
        assert_eq!(query(&url, FIELD_ENVIRONMENT).unwrap(), env.text());
        // Windows wording.
        assert!(text(all).contains("Cmd+V"));
        let win = all_text(&report_link(&p, all, WINDOWS, None, MAX_URL));
        assert!(win.contains("Ctrl+V") && !win.contains("Cmd+V"), "{win}");
    }

    #[test]
    fn activity_goes_to_its_own_field_only_once_the_form_has_it() {
        let env = env();
        let url = report_link(&parts(&env), ReportOptions::default(), MAC, None, MAX_URL);
        let environment = query(&url, FIELD_ENVIRONMENT).unwrap();
        if ACTIVITY_FIELD_ON_FORM {
            let activity = query(&url, FIELD_ACTIVITY).unwrap();
            assert!(activity.contains("Tool: Bevel") && activity.contains("Recent commands"));
            assert!(!environment.contains("Tool: Bevel"));
        } else {
            assert!(query(&url, FIELD_ACTIVITY).is_none());
            assert!(environment.contains("Tool: Bevel") && environment.contains("Recent commands"));
        }
        // The order inside: build, then tool, commands, log.
        let all = all_text(&url);
        let at = |s: &str| all.find(s).unwrap();
        assert!(at("Tool: Bevel") < at("Recent commands") || ACTIVITY_FIELD_ON_FORM);
        assert!(at("Operandi: 0.1.0") < at("Recent log:"));
    }

    #[test]
    fn home_folders_become_a_tilde_on_mac_and_windows() {
        assert_eq!(
            hide_home(
                "open /Users/jason/Desktop/a.opscene and /Users/jason",
                "/Users/jason"
            ),
            "open ~/Desktop/a.opscene and ~"
        );
        assert_eq!(
            hide_home("/Users/jasonx/a and /Users/jason.old", "/Users/jason/"),
            "/Users/jasonx/a and /Users/jason.old",
            "someone else's folder stays"
        );
        assert_eq!(
            hide_home(
                r"C:\Users\Jason\Desktop\a.lxo; c:/users/jason/b",
                r"C:\Users\Jason"
            ),
            r"~\Desktop\a.lxo; ~/b"
        );
        assert_eq!(hide_home("é /Users/jason/é", "/Users/jason"), "é ~/é");
        // In the link: commands and log both.
        let env = env();
        let mut p = parts(&env);
        p.commands
            .push("[typed] scene.open \"/Users/jason/Desktop/x.opscene\"".into());
        let url = report_link(
            &p,
            ReportOptions::default(),
            MAC,
            Some("/Users/jason"),
            MAX_URL,
        );
        let all = all_text(&url);
        assert!(!all.contains("/Users/jason"), "{all}");
        assert!(all.contains("~/Desktop/mug.opscene") && all.contains("~/Desktop/x.opscene"));
    }

    #[test]
    fn long_reports_cut_the_log_first_then_commands_never_the_rest() {
        let env = env();
        let mut p = parts(&env);
        p.commands = (0..20)
            .map(|i| format!("[key]  command.number{i} {}", "x".repeat(60)))
            .collect();
        p.log = (0..30)
            .map(|i| format!("log line {i} {}", "y".repeat(150)))
            .collect();
        let url = report_link(&p, ReportOptions::default(), MAC, None, MAX_URL);
        assert!(url.len() <= MAX_URL, "{}", url.len());
        let all = all_text(&url);
        assert!(all.contains("Tool: Bevel") && all.contains("Layout: Model"));
        assert!(
            all.contains("command.number0 "),
            "commands kept while log is cut"
        );
        assert!(all.contains("log line 29"), "newest log line kept");
        assert!(!all.contains("log line 0 "), "oldest log line cut");
        assert!(all.contains("lines left out to fit the link\n"), "{all}");
        assert!(
            !all.contains("attached file"),
            "no file to point to (BUG-14)"
        );

        // So long that even no log is too long: commands go next, oldest first.
        p.commands = (0..20)
            .map(|i| format!("[key]  command.number{i} {}", "x".repeat(400)))
            .collect();
        let url = report_link(&p, ReportOptions::default(), MAC, None, MAX_URL);
        assert!(url.len() <= MAX_URL, "{}", url.len());
        let all = all_text(&url);
        assert!(!all.contains("log line"), "{all}");
        assert!(all.contains("command.number19 ") && !all.contains("command.number0 "));
        assert!(all.contains("Tool: Bevel") && all.contains("GPU: Metal"));
    }

    #[test]
    fn recent_commands_mark_undone_and_failed_and_skip_the_report() {
        let mut failed = record("item.name \"Mug Handle\"", Source::Typed);
        failed.ok = false;
        failed.message = "no item selected".into();
        let mut undo = record("app.undo", Source::Key);
        undo.message = "Undo Bevel".into();
        let mut records = vec![
            record("select.typeFrom polygon", Source::Menu),
            record("tool.set poly.bevel on", Source::Key),
            undo,
            failed,
            record("operandi.help.reportBug", Source::Menu),
            record(
                "operandi.help.reportBug screenshot:false toolState:true commands:true log:true",
                Source::Typed,
            ),
        ];
        records[1].time = 7.;
        let lines = command_lines(&records, |r| r.time == 7., Some("tool.set poly.bevel on"));
        assert_eq!(
            lines,
            [
                "[menu] select.typeFrom polygon",
                "[key]  tool.set poly.bevel on  (undone)",
                "[key]  app.undo  (Undo Bevel)",
                "[typed] item.name \"Mug Handle\"  -> failed: no item selected",
                "[live] tool.set poly.bevel on  (not applied yet)",
            ]
        );
        let many: Vec<CommandRecord> = (0..30)
            .map(|i| record(&format!("poly.flip {i}"), Source::Button))
            .collect();
        let lines = command_lines(&many, |_| false, None);
        assert_eq!(lines.len(), COMMAND_LINES);
        assert!(lines[0].ends_with("poly.flip 10") && lines[19].ends_with("poly.flip 29"));
    }

    #[test]
    fn screenshots_are_full_size_pngs_and_the_eleventh_deletes_the_oldest() {
        let dir = std::env::temp_dir().join(format!(
            "operandi-report-shots-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        let (w, h) = (37u32, 21u32);
        let rgba = vec![200u8; (w * h * 4) as usize];
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("notes.txt"), "keep me").unwrap();
        let base = 1_795_000_000;
        let mut first = None;
        for i in 0..11 {
            let path = save_screenshot(&dir, w, h, &rgba, base + i).unwrap();
            first.get_or_insert(path);
        }
        let first = first.unwrap();
        assert!(!first.exists(), "the oldest went on the 11th save");
        let shots = std::fs::read_dir(&dir)
            .unwrap()
            .filter(|e| is_screenshot_name(&e.as_ref().unwrap().file_name().to_string_lossy()))
            .count();
        assert_eq!(shots, KEEP_SCREENSHOTS);
        assert!(dir.join("notes.txt").exists(), "other files stay");
        let newest = dir.join(screenshot_name(base + 10));
        let decoded = image::open(&newest).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (w, h));
        assert_eq!(screenshot_name(1_795_000_000), "report-20261118-110640.png");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
