# Release checklist

What has to be true before an Operandi build goes up on GitHub for other
people to download. Work top to bottom: each stage assumes the ones above it
are done. Tick boxes as you go.

Status column: **Done** = already in the repo (checked 2026-11-05),
**Partly** = started, **To do** = not started, **Decide** = needs a call
from Jason before anyone can do it.

## The short version

1. Clean up what's in the repo and its history (stage 1).
2. Put the licence and credits in place (stage 2).
3. Scrub Modo's name from anything that reads like promotion (stage 3).
4. Get Report Bug pointing at the right place (stage 4).
5. Hit the minimum feature bar (stage 5).
6. Private alpha with a few invited testers, unsigned is fine (stage 6).
7. Pay Apple, sign and notarize (stage 7).
8. Public release (stage 8).

Stages 1 to 3 are the ones you can't undo once the repo is public: anything
pushed to a public repo should be treated as copied forever, even if you
delete it later.

---

## Stage 1: Clean the repo and its history

The main repo has 404 commits on `main`, starting 2026-10-23. Git keeps every
version of every file ever committed, so deleting a file today doesn't remove
it from history. Anyone who clones a public repo gets all of it.

| | Item | Status |
|---|---|---|
| [ ] | Remove the old Word files. Seven `.docx` paths appear in history (`docs/*.docx` and `docs/architecture/*.docx`); three are still in `docs/` today. | To do |
| [ ] | Re-derive any early spec that may have had help from looking inside Modo. Several files under `specifications/` (mostly `behavioral-milestone-1*` and the October amendments) have provenance or review notes that should be read by a research thread, not a build thread, to decide which ones need redoing from documentation and observed behaviour only. | Decide |
| [ ] | **Gate for `docs/ABOUT.md`:** its "Built clean" section says Operandi was made without reverse engineering. That is only true once every spec flagged above is re-derived and the public repo starts from the cleaned tree. Don't publish ABOUT.md (or say this anywhere) before then. | To do |
| [ ] | Decide what the public repo carries at all. Today it holds 950 files that mention Modo by name, including `specifications/`, `docs/archive/`, and 74 `.lxo` test files under `specifications/*/fixtures/`. These are fine privately but read badly in public. | Decide |
| [ ] | Confirm every `.lxo` test file was made by you, not copied from Modo's sample content (that folder must never be committed). | To do |
| [ ] | Check for personal paths, emails or tokens: `git grep -nE '/Users/[^/]+/|@privaterelay|ghp_|sk-'`. | To do |

**Recommendation: start the public repo with a fresh history.** Rewriting
404 commits to cut out specific files (with a tool like `git filter-repo`)
works, but it changes every commit ID, breaks the 35 worktrees and every
`.bundle` file in the Library, and you still have to trust you caught
everything. A fresh start is simpler: when the cleanup above is done, make
one commit of the cleaned tree ("Operandi 0.1.0") in a new, empty repo and
keep the current repo private as the archive. You lose public history, which
nobody outside needs yet.

The other option is to keep history and run `git filter-repo` to delete the
`.docx` files and any spec folders that fail review. Pick this only if
public history matters to you.

## Stage 2: Licence and credits (GPL v3)

GPL v3 was decided on 2026-11-04. The repo doesn't say so yet.

| | Item | Status |
|---|---|---|
| [x] | Add a `LICENSE` file at the repo root with the full GPL v3 text (branch `licence-credits`). | Done |
| [x] | Decided 2026-11-05: GPL v3 or later. Was: "GPL v3 only" or "GPL v3 or later". "Or later" lets the code move to a future GPL v4 without asking every contributor; "only" doesn't. Most projects (Blender included) pick "or later". | Decide |
| [ ] | Set `license = "GPL-3.0-or-later"` (or `-only`) in each `Cargo.toml`. | To do |
| [ ] | Fix `docs/DEPENDENCIES.md`: its first line still says "No project-wide license is selected." | To do |
| [ ] | Add Cycles and everything it pulls in to `docs/DEPENDENCIES.md` and `docs/licenses/`. Today the crate table covers the Rust side only. The app bundle ships these extra libraries: Cycles itself (Apache-2.0, pinned commit `3eb35f8`), OpenImageIO, OpenImageDenoise, Embree, oneTBB (all Apache-2.0), OpenEXR, Imath, OpenColorIO (BSD-3-Clause) and OpenJPH (BSD-2-Clause). Copy each one's licence text and any `NOTICE` file. All are GPL v3 compatible. | To do |
| [ ] | Credit any other open-source program whose code was read (the AGENTS.md rule); keep the list in `docs/DEPENDENCIES.md`. | Partly |
| [x] | `CONTRIBUTING.md` with the clean-room rule for contributors and the sign-off rule (DCO), which doubles as their clean-room promise. On branch `about-clean-room`. | Done |

**What GPL v3 asks of you when you hand out an app (not just code):**

- Ship the licence with it. Put `LICENSE` and the third-party notices inside
  the app (`Contents/Resources/`) and show them from an About or Help >
  Licences window.
- Make the exact matching source available. Easiest way: every downloadable
  build is attached to a GitHub Release whose tag points at the commit it was
  built from. GitHub adds the source zip automatically.
- "Source" includes the build scripts. `scripts/build-cycles.sh` and
  `scripts/bundle-dev.sh` (or the release version of it) must be in that
  commit, so someone else could rebuild the same app.
- Don't add extra restrictions. Signing is fine; anything that stops people
  modifying and running their own copy is not.

## Stage 3: No Modo names in promotion

Modo's licence terms ask that its name isn't used to promote another
product. Plain factual statements about file compatibility are normal
("opens .lxo files"); "a free Modo replacement" is not.

| | Item | Status |
|---|---|---|
| [ ] | README: three mentions today (lines 6, 229, 340), all of the "not claiming Modo parity" kind. Rewrite so the README doesn't name Modo at all except, if needed, one factual line about LXO files. | To do |
| [ ] | Text inside the app. Examples found: "Import Modo Scene" (`import.rs`), "Modo LXO, OBJ, …" (`file_chooser.rs`), "Modo Scene (.lxo)" (`panels.rs`), "Shown at the default recorded from Modo." (`vertex_panel.rs`), "Modo's result at Width 0 wasn't measured" (`vertex_panel.rs`). Change to "LXO scene" and remove the ones that talk about where a value came from. Search with `grep -rn '"[^"]*Modo' crates`. | To do |
| [ ] | Release notes, GitHub repo description, topics, screenshots, website, posts: no Modo name, logo or screenshots of Modo. | To do |
| [ ] | The app's look: make sure no icon, splash or screenshot is a copy of Modo's. | To do |

## Stage 4: Report Bug and crash reports

Mostly built already (see `docs/BUG_REPORTS.md`): Report Bug opens a filled-in
GitHub issue in the browser, crashes leave a file, and the next launch offers
to report it. Nothing is sent without the user pressing Submit.

| | Item | Status |
|---|---|---|
| [x] | Report Bug button, crash files, next-launch prompt, native (Cycles) crash pick-up on macOS. | Done |
| [ ] | `REPORT_REPO_URL` in `crates/operandi-app/src/bug_report.rs` is `https://github.com/manfriday/Operandi-BugReports` (public, issues only). Keep it, or point it at the public code repo once that exists. | Decide |
| [ ] | Issues must be switched on in the GitHub repo settings. | To do |
| [ ] | Add `.github/ISSUE_TEMPLATE/` for bug reports and crash reports, plus `bug` and `crash` labels, so reports from the app land tidily. Keep the fields matching what the app fills in. | To do |
| [ ] | Run the 7-step GUI checklist in `docs/BUG_REPORTS.md` against a signed, release-mode build (stage 7), not just a QA app. | To do |
| [ ] | Check a report's text for the user's home folder path (`/Users/<name>`) in log lines and backtraces; swap it for `~` before it goes in the link. | To do |
| [ ] | Add a `SECURITY.md` saying how to report something private (an email address, not an issue). | To do |

## Stage 5: Minimum feature bar

This is a proposal to edit. The idea: a stranger can download it, make
something simple, save it, and come back to it without losing work or being
confused by half-built bits.

| | Item | Status |
|---|---|---|
| [ ] | Make, select and edit basic meshes: the primitives, transform, and the core polygon/edge/vertex tools. | Partly |
| [ ] | Save and reopen without losing anything; autosave and crash recovery work. | Partly |
| [ ] | Undo and redo are reliable for every tool that ships. | Partly |
| [ ] | Import and export at least OBJ and glTF. | Done |
| [ ] | No crash in a 30-minute normal session on two different Macs. | To do |
| [ ] | Anything unfinished is hidden or clearly marked "Preview" in the UI, including Render (Cycles) if it isn't solid yet. | Decide |
| [ ] | A short Getting Started page (README or Help) with the default keys and mouse controls. | To do |
| [ ] | About window with version, commit and licences (stage 2). | To do |
| [ ] | Version number set on purpose: the app crate says `0.1.0` while the bundle script says `0.1.1`. Pick one and tag releases `v0.1.0-alpha.1`, `v0.1.0-alpha.2`, … | To do |
| [ ] | Real app ID: the bundle uses `org.operandi.rust.m11.dev`; release builds need a permanent one (for example `org.operandi.Operandi`). Changing it later loses users' saved preferences. | Decide |

## Stage 6: Private alpha

A small round with people you trust before anything is public.

| | Item | Status |
|---|---|---|
| [ ] | Pick 3 to 10 testers, ideally with a mix of Intel and Apple-chip Macs and a couple of macOS versions (the app says it needs macOS 12 or later; test the oldest you claim). | To do |
| [ ] | Keep the repo private and add testers as collaborators with Read access. That lets them download from Releases and open issues, so Report Bug works for them. | To do |
| [ ] | Publish the build as a GitHub **pre-release**. | To do |
| [ ] | Unsigned is fine here, but tell testers how to open it: the first time, macOS blocks it; they go to System Settings > Privacy & Security and press "Open Anyway". | To do |
| [ ] | Ask testers to use Report Bug for everything, so you also test the reporting path. | To do |
| [ ] | Exit rule: no open crash bugs, nothing that loses work, and each tester has done one real piece of work start to finish. | Decide |

## Stage 7: Apple signing and notarization

Without this, every public user hits the "Open Anyway" dance above, and many
will assume it's malware.

**Cost:** the Apple Developer Program is US$99 a year (individual). If you
stop paying, apps you already signed keep working; you just can't sign new
ones. (Price as of this writing; check developer.apple.com before paying.)

| | Item | Status |
|---|---|---|
| [ ] | Join the Apple Developer Program as an individual. Approval can take a day or two. | To do |
| [ ] | In Xcode > Settings > Accounts, create a **Developer ID Application** certificate. It lands in your Keychain. Back it up (export as `.p12` with a password) somewhere safe. | To do |
| [ ] | Make an app-specific password at account.apple.com for the notarization tool, and save it once: `xcrun notarytool store-credentials operandi --apple-id <you> --team-id <TEAMID>`. | To do |
| [ ] | Release bundle script: today `bundle-dev.sh` signs with `-` (a blank, local-only signature). A release script must sign every library in `Contents/Frameworks/` first, then the app, with `codesign --force --timestamp --options runtime --sign "Developer ID Application: <name> (<TEAMID>)"`. `--options runtime` (Apple's "hardened runtime") is required for notarization. | To do |
| [ ] | Test the signed app with Cycles rendering. If the hardened runtime blocks something Cycles needs, add only that permission in an entitlements file. | To do |
| [ ] | Pack it as a `.dmg` (or `.zip`), then send it to Apple: `xcrun notarytool submit Operandi.dmg --keychain-profile operandi --wait`. Takes a few minutes. | To do |
| [ ] | Attach Apple's approval to the file so it works offline: `xcrun stapler staple Operandi.dmg`. | To do |
| [ ] | Check it like a stranger would: download it from the GitHub Release on another Mac (or another user account) and open it. It should open with only the usual "downloaded from the internet" question. `spctl -a -vv Operandi.app` should say "Notarized Developer ID". | To do |
| [ ] | Your Mac's Xcode quirk: run the tools with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` in front (as the bundle script already does), since `xcode-select` still points at the command-line tools. | Partly |

## Stage 8: Public release

| | Item | Status |
|---|---|---|
| [ ] | Make the cleaned repo public (or create the fresh one from stage 1). | To do |
| [ ] | Tag the release commit and attach the notarized `.dmg` to a GitHub Release, with short notes: what works, what's known broken, "this is alpha, keep backups". | To do |
| [ ] | Repo front page: description, topics, a screenshot of Operandi (not Modo), link to Getting Started. | To do |
| [ ] | Turn on GitHub Discussions if you want questions kept out of Issues. | Decide |
| [ ] | Rerun stage 3 on the release notes and anything you post. | To do |
| [ ] | First week: read every issue, label it, and say thanks. | To do |

## Not covered yet

Windows and Linux builds (untested today), automatic update checks, and
building releases on GitHub's machines instead of your Mac. All can wait
until after the first public Mac build.

*Not legal advice. The GPL and Modo licence points are a practical reading,
not a lawyer's.*
