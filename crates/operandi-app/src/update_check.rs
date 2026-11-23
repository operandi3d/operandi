//! The update check (Jason, 2026-11-23): Preferences > Defaults >
//! Application > Updates picks the Release or Test channel; on launch (if
//! Check for Updates on Launch is on) and from Help > Check for Updates
//! (`operandi.update.check`) the app asks GitHub for the newest build on
//! that channel and, when it is newer, offers to open its download page.
//! Nothing is ever downloaded or installed by the app.
//!
//! This file holds the parts without a window: the preferences, the feed
//! parsing and comparison, and the fetch. update_check_ui.rs puts them on
//! screen.
//!
//! - Release channel: the repository's releases with a version tag
//!   (`v0.1.0`; pre-releases count, drafts and `test-build` don't). The
//!   newest is the highest version; it is offered when it is newer than
//!   this build's (`CARGO_PKG_VERSION`, the version release-macos.sh puts
//!   in the bundle).
//! - Test channel: the rolling `test-build` pre-release. Its notes carry a
//!   hidden `<!-- operandi-build mac=<sha> mac_time=<unix> windows=<sha>
//!   windows_time=<unix> -->` line: the build is offered when this
//!   platform's time is newer than this build's commit time. Without the
//!   line, the "built from `1a2b3c4d`" line for this platform is compared
//!   with this build's commit; a development build (`+dirty`, `unknown`)
//!   can't be compared that way and says nothing at launch.
//! - Linux has no test build yet, so it always follows the Release channel.
//!
//! The fetch is a plain GET to api.github.com by the system's `curl` on a
//! background thread (no HTTP library in the app); it sends nothing about
//! the user. Any failure is silent at launch. `OPERANDI_UPDATE_FEED=<file>`
//! reads that JSON file instead, so QA can fake a newer build.
use crate::prefs_file::PrefsFile;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::sync::mpsc;

/// The public repository builds are published from.
pub const REPOSITORY: &str = "operandi3d/operandi";
/// The rolling test build's tag.
pub const TEST_TAG: &str = "test-build";
/// Replaces the GitHub request with this JSON file (QA).
pub const FEED_ENV: &str = "OPERANDI_UPDATE_FEED";

// ------------------------------------------------------------ preferences

/// Which builds the check follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    #[default]
    #[serde(alias = "release")]
    Release,
    #[serde(alias = "test")]
    Test,
}
impl Channel {
    pub const ALL: [Self; 2] = [Self::Release, Self::Test];
    pub fn label(self) -> &'static str {
        match self {
            Self::Release => "Release",
            Self::Test => "Test",
        }
    }
    /// The channel a fresh install follows: Release, or Test for a build
    /// made with `OPERANDI_DEFAULT_UPDATE_CHANNEL=test` (test builds).
    pub fn build_default() -> Self {
        match option_env!("OPERANDI_DEFAULT_UPDATE_CHANNEL") {
            Some(c) if c.eq_ignore_ascii_case("test") => Self::Test,
            _ => Self::Release,
        }
    }
}

/// Preferences > Defaults > Application > Updates, saved in
/// `update-preferences.json`. `pref.value updates.<field>` sets each.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UpdatePreferences {
    version: u8,
    pub channel: Channel,
    pub check_on_launch: bool,
    /// The build Skip This Build answered (a release tag or a test
    /// build's commit); the launch check doesn't offer it again. Empty:
    /// none. Not shown on the page.
    pub skipped: String,
}
impl Default for UpdatePreferences {
    fn default() -> Self {
        Self {
            version: 1,
            channel: Channel::build_default(),
            check_on_launch: true,
            skipped: String::new(),
        }
    }
}
impl PrefsFile for UpdatePreferences {
    const INVALID: &'static str = "invalid update preferences";
    fn valid(&self) -> bool {
        self.version == 1 && self.skipped.len() <= 128
    }
}

// ------------------------------------------------------------ this build

/// The platforms builds are published for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Mac,
    Windows,
    Linux,
}
impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }
    /// The channel actually checked: Linux has no test build yet.
    pub fn feed_channel(self, channel: Channel) -> Channel {
        match self {
            Self::Linux => Channel::Release,
            _ => channel,
        }
    }
    /// The key in the hidden build line, and the name in the notes.
    fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::Mac => ("mac", "**macOS**"),
            Self::Windows => ("windows", "**Windows**"),
            Self::Linux => ("linux", "**Linux**"),
        }
    }
}

/// What this build is.
#[derive(Clone, Debug, PartialEq)]
pub struct ThisBuild {
    pub version: String,
    /// The build's commit as build.rs stamped it (short hash, `+dirty`,
    /// or `unknown`).
    pub commit: String,
    /// The commit's time, Unix seconds.
    pub commit_time: Option<i64>,
}
impl ThisBuild {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            commit: env!("OPERANDI_GIT_COMMIT").into(),
            commit_time: env!("OPERANDI_GIT_COMMIT_TIME").trim().parse().ok(),
        }
    }
    /// The commit, unless this is a development build (`+dirty`,
    /// `unknown`) whose commit says nothing about what it holds.
    fn clean_commit(&self) -> Option<&str> {
        let c = self.commit.as_str();
        (is_hex(c) && c.len() >= 7).then_some(c)
    }
    /// The commit without `+dirty`, for messages.
    fn short(&self) -> &str {
        let c = self.commit.trim_end_matches("+dirty");
        &c[..c.len().min(8)]
    }
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Whether two commit hashes (full or short) name the same commit.
fn same_commit(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
    let n = a.len().min(b.len());
    n >= 7 && a[..n] == b[..n]
}

// ------------------------------------------------------------ versions

/// A release version (`v1.2.3`, `0.2.0-beta.1`), compared the semver way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    pre: Vec<String>,
}
impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text
            .strip_prefix('v')
            .or_else(|| text.strip_prefix('V'))
            .unwrap_or(text);
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (text, None),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        let mut numbers = [0u64; 3];
        for (slot, part) in numbers.iter_mut().zip(&parts) {
            if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            *slot = part.parse().ok()?;
        }
        let pre = match pre {
            Some(p) if p.is_empty() || p.split('.').any(str::is_empty) => return None,
            Some(p) => p.split('.').map(str::to_owned).collect(),
            None => Vec::new(),
        };
        Some(Self { core: numbers, pre })
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| {
            // A pre-release comes before its release.
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => {
                    for (a, b) in self.pre.iter().zip(&other.pre) {
                        let order = match (a.parse::<u64>(), b.parse::<u64>()) {
                            (Ok(x), Ok(y)) => x.cmp(&y),
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => a.cmp(b),
                        };
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    self.pre.len().cmp(&other.pre.len())
                }
            }
        })
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ------------------------------------------------------------ verdicts

/// A newer build to offer.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub channel: Channel,
    /// The release's tag (its download page).
    pub tag: String,
    /// What Skip This Build remembers: the tag, or the test build's commit.
    pub key: String,
    /// The prompt's message.
    pub message: String,
}
impl Offer {
    /// The release page. Built from the tag (checked to be plain), never
    /// taken from the feed, so the browser only ever opens GitHub.
    pub fn url(&self) -> String {
        format!("https://github.com/{REPOSITORY}/releases/tag/{}", self.tag)
    }
}

/// What a check found.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Newer(Offer),
    /// This build is the newest on its channel.
    Newest(Channel),
    /// Can't tell (no build yet, a development build); the reason.
    Unknown(String),
}
impl Verdict {
    /// The manual check's report when there is nothing to offer.
    pub fn report(&self) -> String {
        match self {
            Self::Newer(offer) => offer.message.clone(),
            Self::Newest(Channel::Release) => "You have the newest release.".into(),
            Self::Newest(Channel::Test) => "You have the newest test build.".into(),
            Self::Unknown(why) => why.clone(),
        }
    }
}

/// A tag safe to put in a URL.
fn plain_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 64
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// The Release channel's verdict from a `releases` list.
pub fn evaluate_releases(json: &str, me: &ThisBuild) -> Result<Verdict, String> {
    let list: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("GitHub's answer wasn't readable ({e})"))?;
    let list = list
        .as_array()
        .ok_or("GitHub's answer wasn't a list of releases")?;
    let newest = list
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| r["tag_name"].as_str())
        .filter(|tag| *tag != TEST_TAG && plain_tag(tag))
        .filter_map(|tag| Version::parse(tag).map(|v| (v, tag)))
        .max_by(|a, b| a.0.cmp(&b.0));
    let Some((version, tag)) = newest else {
        return Ok(Verdict::Unknown(
            "No release has been published yet.".into(),
        ));
    };
    let Some(mine) = Version::parse(&me.version) else {
        return Ok(Verdict::Unknown(format!(
            "This build's version ({}) can't be compared.",
            me.version
        )));
    };
    if version <= mine {
        return Ok(Verdict::Newest(Channel::Release));
    }
    let shown = tag.trim_start_matches(['v', 'V']);
    Ok(Verdict::Newer(Offer {
        channel: Channel::Release,
        tag: tag.to_owned(),
        key: tag.to_owned(),
        message: format!(
            "Operandi {shown} is available. You have {}. OK opens the download page.",
            me.version
        ),
    }))
}

/// The hidden `<!-- operandi-build k=v ... -->` line's pairs.
fn build_line(body: &str) -> Option<Vec<(&str, &str)>> {
    let start = body.find("<!-- operandi-build")? + "<!-- operandi-build".len();
    let rest = &body[start..];
    let end = rest.find("-->")?;
    Some(
        rest[..end]
            .split_whitespace()
            .filter_map(|pair| pair.split_once('='))
            .collect(),
    )
}

/// The short hash in the notes' "- **macOS**: ... built from [`1a2b3c4d`]"
/// line for `name`.
fn built_from<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let line = body.lines().find(|l| l.contains(name))?;
    let after = &line[line.find("built from")? + "built from".len()..];
    let open = after.find('`')? + 1;
    let close = after[open..].find('`')? + open;
    let hash = &after[open..close];
    is_hex(hash).then_some(hash)
}

/// The Test channel's verdict from the `test-build` release.
pub fn evaluate_test(json: &str, me: &ThisBuild, platform: Platform) -> Result<Verdict, String> {
    let release: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("GitHub's answer wasn't readable ({e})"))?;
    let tag = release["tag_name"]
        .as_str()
        .filter(|t| plain_tag(t))
        .ok_or("GitHub's answer wasn't a release")?;
    let body = release["body"].as_str().unwrap_or("");
    let (key, name) = platform.names();
    let none_yet = || Verdict::Unknown("There is no test build for this platform yet.".into());
    let offer = |remote: &str, built: Option<i64>| {
        let short = &remote[..remote.len().min(8)];
        let theirs = match built {
            Some(t) => format!("built {} from {short}", day(t)),
            None => format!("built from {short}"),
        };
        let mine = match me.commit_time {
            Some(t) => format!("{}, built {}", me.short(), day(t)),
            None => me.short().to_owned(),
        };
        Verdict::Newer(Offer {
            channel: Channel::Test,
            tag: tag.to_owned(),
            key: remote.to_ascii_lowercase(),
            message: format!(
                "A newer test build is available, {theirs}. You have {mine}. \
                 OK opens the download page."
            ),
        })
    };
    let pairs = build_line(body).unwrap_or_default();
    let get = |k: &str| pairs.iter().find(|(p, _)| *p == k).map(|(_, v)| *v);
    let remote_sha = get(key).filter(|s| is_hex(s));
    let remote_time = get(&format!("{key}_time")).and_then(|t| t.parse::<i64>().ok());
    if let (Some(sha), Some(time)) = (remote_sha, remote_time) {
        let Some(mine) = me.commit_time else {
            return Ok(Verdict::Unknown(
                "This build doesn't know when it was made, so it can't be compared.".into(),
            ));
        };
        if time > mine && !same_commit(sha, &me.commit) {
            return Ok(offer(sha, Some(time)));
        }
        return Ok(Verdict::Newest(Channel::Test));
    }
    let Some(sha) = built_from(body, name) else {
        return Ok(none_yet());
    };
    let Some(mine) = me.clean_commit() else {
        return Ok(Verdict::Unknown(format!(
            "This is a development build ({}), so it can't be compared with the test build \
             (built from {sha}).",
            me.commit
        )));
    };
    if same_commit(sha, mine) {
        Ok(Verdict::Newest(Channel::Test))
    } else {
        Ok(offer(sha, None))
    }
}

/// `channel`'s verdict from its feed.
pub fn evaluate(
    channel: Channel,
    json: &str,
    me: &ThisBuild,
    platform: Platform,
) -> Result<Verdict, String> {
    match platform.feed_channel(channel) {
        Channel::Release => evaluate_releases(json, me),
        Channel::Test => evaluate_test(json, me, platform),
    }
}

/// "Nov 23" (UTC) for Unix seconds `t`.
fn day(t: i64) -> String {
    // Days to civil date (Howard Hinnant's algorithm).
    let z = t.div_euclid(86_400) + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} {d}", MONTHS[(m - 1) as usize])
}

// ------------------------------------------------------------ fetching

/// The GitHub API address for `channel`'s feed.
pub fn feed_url(channel: Channel) -> String {
    match channel {
        Channel::Release => {
            format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=30")
        }
        Channel::Test => {
            format!("https://api.github.com/repos/{REPOSITORY}/releases/tags/{TEST_TAG}")
        }
    }
}

/// `channel`'s feed: `OPERANDI_UPDATE_FEED`'s file, else GitHub by curl.
/// Blocks; run it off the UI thread.
pub fn fetch(channel: Channel) -> Result<String, String> {
    if let Some(path) = std::env::var_os(FEED_ENV).filter(|p| !p.is_empty()) {
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("Couldn't read {FEED_ENV} ({e})"));
    }
    if cfg!(test) {
        return Err("Tests don't go online".into());
    }
    let mut command = std::process::Command::new("curl");
    command
        .args(["-fsSL", "--max-time", "10"])
        .args(["-H", "Accept: application/vnd.github+json"])
        .args(["-H", "User-Agent: Operandi"])
        .arg(feed_url(channel))
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up for curl.exe.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let out = command.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => "curl isn't installed".to_owned(),
        _ => format!("curl couldn't run ({e})"),
    })?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr);
        let why = why.trim().trim_start_matches("curl: ");
        return Err(if why.is_empty() {
            "GitHub could not be reached".into()
        } else {
            format!("GitHub could not be reached ({why})")
        });
    }
    String::from_utf8(out.stdout).map_err(|_| "GitHub's answer wasn't text".into())
}

// ------------------------------------------------------------ app state

/// A check running on its thread.
struct Running {
    channel: Channel,
    /// Help > Check for Updates (or the command) asked: report whatever
    /// comes back. The launch check stays silent unless there's an offer.
    manual: bool,
    result: mpsc::Receiver<Result<String, String>>,
}

/// What a finished check asks the app to show.
#[derive(Clone, Debug, PartialEq)]
pub enum Show {
    /// The Update Available prompt.
    Offer(Offer),
    /// The manual check's report (no offer, or an error).
    Report(String),
}

/// The update check's state in the app.
#[derive(Default)]
pub struct UpdateCheck {
    pub prefs: UpdatePreferences,
    /// The launch check ran (or was skipped) this launch.
    pub launched: bool,
    running: Option<Running>,
    /// Waiting to be shown (after any other dialog closes).
    pub show: Option<Show>,
}
impl UpdateCheck {
    /// Starts a check; `wake` is called from the thread when it's done.
    /// A manual check while one runs makes that one report back.
    pub fn start(&mut self, manual: bool, wake: impl Fn() + Send + 'static) {
        if let Some(running) = &mut self.running {
            running.manual |= manual;
            return;
        }
        let channel = Platform::current().feed_channel(self.prefs.channel);
        let (send, result) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("update-check".into())
            .spawn(move || {
                let _ = send.send(fetch(channel));
                wake();
            });
        match spawned {
            Ok(_) => {
                self.running = Some(Running {
                    channel,
                    manual,
                    result,
                })
            }
            Err(e) if manual => {
                self.show = Some(Show::Report(format!("Couldn't check for updates ({e}).")))
            }
            Err(_) => {}
        }
    }
    /// Whether a check is running.
    pub fn busy(&self) -> bool {
        self.running.is_some()
    }
    /// Takes a finished check's result and decides what to show.
    pub fn poll(&mut self) {
        let Some(running) = &self.running else {
            return;
        };
        let fetched = match running.result.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("The check stopped".into()),
        };
        let Some(running) = self.running.take() else {
            return;
        };
        let me = ThisBuild::current();
        let verdict =
            fetched.and_then(|json| evaluate(running.channel, &json, &me, Platform::current()));
        self.show = decide(verdict, running.manual, &self.prefs.skipped);
    }
}

/// What to show for a check's `verdict`: a launch check shows only an
/// offer it wasn't told to skip; a manual one always reports.
pub fn decide(verdict: Result<Verdict, String>, manual: bool, skipped: &str) -> Option<Show> {
    match verdict {
        Ok(Verdict::Newer(offer)) if manual || !offer.key.eq_ignore_ascii_case(skipped) => {
            Some(Show::Offer(offer))
        }
        Ok(v) if manual => Some(Show::Report(v.report())),
        Err(e) if manual => Some(Show::Report(format!("Couldn't check for updates: {e}."))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(version: &str, commit: &str, time: Option<i64>) -> ThisBuild {
        ThisBuild {
            version: version.into(),
            commit: commit.into(),
            commit_time: time,
        }
    }

    const RELEASES: &str = r#"[
        {"tag_name": "test-build", "draft": false, "prerelease": true},
        {"tag_name": "v0.3.0", "draft": true, "prerelease": false},
        {"tag_name": "v0.2.0-beta.2", "draft": false, "prerelease": true},
        {"tag_name": "v0.1.0", "draft": false, "prerelease": true},
        {"tag_name": "nightly", "draft": false, "prerelease": true}
    ]"#;

    #[test]
    fn versions_compare_the_semver_way() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("v0.2.0") > v("0.1.9"));
        assert!(v("0.2.0") > v("0.2.0-beta.2"));
        assert!(v("0.2.0-beta.10") > v("0.2.0-beta.2"));
        assert!(v("0.2.0-beta") > v("0.2.0-alpha.5"));
        assert!(v("0.2.0-alpha.1") > v("0.2.0-alpha"));
        assert!(v("1.0.0-1") < v("1.0.0-a"));
        assert_eq!(v("v1.2"), v("1.2.0"));
        assert_eq!(v("1.2.3+build.7"), v("1.2.3"));
        for bad in [
            "test-build",
            "nightly",
            "1..2",
            "1.2.3.4",
            "1.2.x",
            "1.2.3-",
            "",
        ] {
            assert!(Version::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn release_list_skips_drafts_and_the_test_build() {
        // Draft v0.3.0 and test-build don't count; the beta does.
        let me = build("0.1.0", "9f8e7d6c", None);
        let Ok(Verdict::Newer(offer)) = evaluate_releases(RELEASES, &me) else {
            panic!("expected an offer");
        };
        assert_eq!(offer.tag, "v0.2.0-beta.2");
        assert_eq!(offer.key, "v0.2.0-beta.2");
        assert_eq!(
            offer.message,
            "Operandi 0.2.0-beta.2 is available. You have 0.1.0. OK opens the download page."
        );
        assert_eq!(
            offer.url(),
            "https://github.com/operandi3d/operandi/releases/tag/v0.2.0-beta.2"
        );
    }

    #[test]
    fn release_same_or_older_is_newest() {
        for mine in ["0.2.0-beta.2", "0.2.0", "1.0.0"] {
            assert_eq!(
                evaluate_releases(RELEASES, &build(mine, "abc1234", None)),
                Ok(Verdict::Newest(Channel::Release)),
                "{mine}"
            );
        }
        let none = r#"[{"tag_name": "test-build", "draft": false}]"#;
        assert!(matches!(
            evaluate_releases(none, &build("0.1.0", "abc1234", None)),
            Ok(Verdict::Unknown(_))
        ));
        assert!(evaluate_releases("{}", &build("0.1.0", "x", None)).is_err());
        assert!(evaluate_releases("not json", &build("0.1.0", "x", None)).is_err());
    }

    #[test]
    fn a_tag_that_is_not_plain_is_never_offered() {
        let list = r#"[{"tag_name": "v9.0.0/../../evil", "draft": false}]"#;
        assert!(matches!(
            evaluate_releases(list, &build("0.1.0", "x", None)),
            Ok(Verdict::Unknown(_))
        ));
    }

    const NOV_21: i64 = 1_763_683_200; // 2025-11-21 00:00 UTC
    const NOV_23: i64 = NOV_21 + 2 * 86_400;

    fn test_release(body: &str) -> String {
        serde_json::json!({"tag_name": "test-build", "prerelease": true, "body": body}).to_string()
    }
    const NOTES: &str = "Rolling test build.\n\n\
        - **macOS**: `Operandi-Test-macOS.zip`, built from [`1a2b3c4d`](https://github.com/x/commit/1a2b3c4d)\n\
        - **Windows**: `Operandi-Test-Windows-x64.zip`, built from [`5e6f7a8b`](https://github.com/x/commit/5e6f7a8b)\n";

    fn hidden() -> String {
        format!(
            "{NOTES}\n<!-- operandi-build mac=1a2b3c4d00000000000000000000000000000000 \
             mac_time={NOV_23} windows=5e6f7a8b00000000000000000000000000000000 \
             windows_time={NOV_21} -->"
        )
    }

    #[test]
    fn test_build_with_hidden_line_compares_times() {
        let feed = test_release(&hidden());
        // Older here: offered.
        let me = build("0.1.0", "9f8e7d6c", Some(NOV_21));
        let Ok(Verdict::Newer(offer)) = evaluate_test(&feed, &me, Platform::Mac) else {
            panic!("expected an offer");
        };
        assert_eq!(
            offer.message,
            "A newer test build is available, built Nov 23 from 1a2b3c4d. \
             You have 9f8e7d6c, built Nov 21. OK opens the download page."
        );
        assert_eq!(offer.key, "1a2b3c4d00000000000000000000000000000000");
        assert_eq!(
            offer.url(),
            "https://github.com/operandi3d/operandi/releases/tag/test-build"
        );
        // Windows' build is as old as this one: nothing newer.
        assert_eq!(
            evaluate_test(&feed, &me, Platform::Windows),
            Ok(Verdict::Newest(Channel::Test))
        );
        // Newer here, or the same commit: newest.
        let newer = build("0.1.0", "9f8e7d6c", Some(NOV_23 + 60));
        assert_eq!(
            evaluate_test(&feed, &newer, Platform::Mac),
            Ok(Verdict::Newest(Channel::Test))
        );
        let same = build("0.1.0", "1a2b3c4d", Some(NOV_21));
        assert_eq!(
            evaluate_test(&feed, &same, Platform::Mac),
            Ok(Verdict::Newest(Channel::Test))
        );
        // A dirty build still compares by time.
        let dirty = build("0.1.0", "9f8e7d6c+dirty", Some(NOV_21));
        assert!(matches!(
            evaluate_test(&feed, &dirty, Platform::Mac),
            Ok(Verdict::Newer(_))
        ));
    }

    #[test]
    fn test_build_without_hidden_line_compares_commits() {
        let feed = test_release(NOTES);
        let me = build("0.1.0", "9f8e7d6c", None);
        let Ok(Verdict::Newer(offer)) = evaluate_test(&feed, &me, Platform::Windows) else {
            panic!("expected an offer");
        };
        assert_eq!(offer.key, "5e6f7a8b");
        assert_eq!(
            offer.message,
            "A newer test build is available, built from 5e6f7a8b. You have 9f8e7d6c. \
             OK opens the download page."
        );
        let same = build("0.1.0", "1a2b3c4d", Some(NOV_21));
        assert_eq!(
            evaluate_test(&feed, &same, Platform::Mac),
            Ok(Verdict::Newest(Channel::Test))
        );
        // Development builds can't be compared by commit.
        for commit in ["1a2b3c4d+dirty", "unknown"] {
            assert!(matches!(
                evaluate_test(&feed, &build("0.1.0", commit, None), Platform::Mac),
                Ok(Verdict::Unknown(_))
            ));
        }
        // No line for this platform.
        let mac_only = test_release(
            NOTES
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join("\n")
                .as_str(),
        );
        assert!(matches!(
            evaluate_test(&mac_only, &me, Platform::Windows),
            Ok(Verdict::Unknown(_))
        ));
    }

    #[test]
    fn linux_follows_releases() {
        assert_eq!(
            Platform::Linux.feed_channel(Channel::Test),
            Channel::Release
        );
        assert_eq!(Platform::Mac.feed_channel(Channel::Test), Channel::Test);
        let me = build("0.1.0", "9f8e7d6c", None);
        assert!(matches!(
            evaluate(Channel::Test, RELEASES, &me, Platform::Linux),
            Ok(Verdict::Newer(Offer {
                channel: Channel::Release,
                ..
            }))
        ));
    }

    #[test]
    fn launch_is_quiet_unless_offered_and_not_skipped() {
        let offer = Offer {
            channel: Channel::Release,
            tag: "v0.2.0".into(),
            key: "v0.2.0".into(),
            message: "m".into(),
        };
        let newer = || Ok(Verdict::Newer(offer.clone()));
        assert_eq!(decide(newer(), false, ""), Some(Show::Offer(offer.clone())));
        assert_eq!(decide(newer(), false, "v0.2.0"), None);
        // The manual check shows a skipped build all the same.
        assert_eq!(
            decide(newer(), true, "v0.2.0"),
            Some(Show::Offer(offer.clone()))
        );
        assert_eq!(decide(Ok(Verdict::Newest(Channel::Test)), false, ""), None);
        assert_eq!(decide(Err("offline".into()), false, ""), None);
        assert_eq!(
            decide(Ok(Verdict::Newest(Channel::Release)), true, ""),
            Some(Show::Report("You have the newest release.".into()))
        );
        assert_eq!(
            decide(Ok(Verdict::Newest(Channel::Test)), true, ""),
            Some(Show::Report("You have the newest test build.".into()))
        );
        assert_eq!(
            decide(Err("GitHub could not be reached".into()), true, ""),
            Some(Show::Report(
                "Couldn't check for updates: GitHub could not be reached.".into()
            ))
        );
    }

    #[test]
    fn dates_read_as_month_and_day() {
        assert_eq!(day(NOV_21), "Nov 21");
        assert_eq!(day(0), "Jan 1");
        assert_eq!(day(951_782_400), "Feb 29"); // 2000-02-29
    }

    #[test]
    fn preferences_round_trip_and_default_to_launch_checks() {
        let p = UpdatePreferences::default();
        assert!(p.check_on_launch);
        assert_eq!(p.channel, Channel::build_default());
        let dir =
            std::env::temp_dir().join(format!("operandi-update-prefs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("update-preferences.json");
        let mut q = p.clone();
        q.channel = Channel::Test;
        q.skipped = "v0.2.0".into();
        q.save(&path).unwrap();
        assert_eq!(UpdatePreferences::load(&path), q);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
