# Bug and crash reports

User-initiated only: Operandi never sends anything by itself and holds no
tokens. Every report is a GitHub issue form, pre-filled from the link, that
opens in the user's browser; they read and edit it there and decide whether
to submit. Code: `crates/operandi-app/src/bug_report.rs` (links, crash files,
prompt) and `log_ring.rs` (recent log lines). Reports go to the public code repository,
operandi3d/operandi (the one constant `REPORT_REPO_URL`).

That repository has two issue forms in `.github/ISSUE_TEMPLATE`:
`bug.yml` (label `bug`; fields `what`, `steps`, `expected`, `environment`)
and `idea.yml` (label `enhancement`; fields `idea`, `why`, `environment`).
Every link opens the bug form (`issues/new?template=bug.yml&title=…&<field
id>=…`, values percent-encoded, empty fields left out); its top links to the
idea form, so one menu item covers both. Environment is a text block, so
it is filled with plain lines (no code fences).

- **Help > Report Bug or Idea…** (`operandi.help.reportBug`; "Report Bug"
  until 2026-11-20, and until 2026-11-16 a button at the top right of the
  window): opens the bug form titled "Bug: " with What happened / Steps /
  Expected left empty and Environment filled in: version and git commit (`build.rs` stamps `OPERANDI_GIT_COMMIT`,
  `+dirty` with local changes), OS and version, GPU adapter and backend, and
  the last 30 log lines. If the browser can't be opened, the link goes on
  the clipboard.
- **Log lines:** `log_ring::log_line!` prints like `eprintln!` and keeps the
  line (200 kept in memory); status-bar messages are recorded each time they
  change.
- **Crash files:** a panic hook (in front of Rust's default one) writes
  `<data folder>/crashes/crash-<UTC time>-<pid>.txt`: time, version/commit,
  OS, GPU, thread, panic message and location, full backtrace, last 30 log
  lines. QA apps use their own data folder (`OPERANDI_DEV_DATA`).
- **Next launch:** "Operandi crashed last time. Report it?" with
  **Report** (opens the bug form titled "Crash: <message>": the panic
  message and location in What happened; the environment, Operandi's
  backtrace frames and the log in Environment; and shows the crash file in
  Finder to drag into the issue),
  **Not now** (asks again next launch) and **Don't ask for this one**.
  Answers live in `crashes/state.json`. Links are cut to stay under 7,000
  characters by cutting Environment's backtrace and log lines, with a note
  saying lines were left out.
- **Native crashes (macOS):** crashes a panic hook can't see (for example in
  the Cycles C++ code) leave `operandi-app-<date>.ips` (older systems
  `.crash`) in `~/Library/Logs/DiagnosticReports`. Reports for this
  executable newer than the first time the app looked are offered through
  the same prompt (summary: exception type and signal, in What happened;
  the reporting app's environment in Environment), and Report reveals that
  file. A native report within two minutes after a panic file is the
  same crash (the abort after the panic) and isn't asked about twice.

Provisional: Windows and Linux open/reveal commands (`start`, `explorer
/select`, `xdg-open`) are untested; native crash reports are found only on
macOS. The "+dirty" mark can lag until the next rebuild of operandi-app.

## Testing a crash (QA/dev builds only)

Packaged release builds (`OPERANDI_MILESTONE1_PACKAGE=1`) ignore these.

- `OPERANDI_TEST_PANIC=1`: panics after the first frame (crash file path).
- `OPERANDI_TEST_PANIC=abort`: aborts after the first frame (a native crash;
  macOS writes an `.ips` report).

## GUI checklist (QA app: Operandi report-bug)

Run the QA app's executable from Terminal with its own data folder, e.g.
`OPERANDI_DEV_DATA=/tmp/operandi-bugqa "<app>/Contents/MacOS/operandi-app"`
(add `OPERANDI_TEST_PANIC=1` in front for step 4).

1. Choose Help > Report Bug or Idea…: the browser opens the Bug report
   form on operandi3d/operandi, titled "Bug: ", with What
   happened, Steps and Expected empty and Environment filled in. Nothing is
   submitted until you press Submit.
2. The Environment field shows version 0.1.0 and this build's commit, your
   macOS version, the GPU (Metal · your adapter) and recent log lines
   (including status-bar messages from what you just did).
3. The form's "Suggest an idea" link at the top opens the Idea form.
4. Quit. Start with `OPERANDI_TEST_PANIC=1`: the app crashes after its first
   frame and Terminal prints "Operandi crash report written to …".
5. Start normally: "Operandi crashed last time. Report it?" appears. Report:
   the Bug report form opens with the panic message and location in What
   happened and the backtrace in Environment, and Finder shows the crash
   file selected. Drag it into the issue.
6. Crash again; at the prompt choose Not now. Relaunch: it asks again.
   Choose Don't ask for this one. Relaunch: no prompt.
7. Native crash: start the app, then `kill -SEGV <pid>` (or run with
   `OPERANDI_TEST_PANIC=abort`). Wait for macOS to write the report (a few
   seconds; it appears in Console > Crash Reports), relaunch: the prompt
   names `operandi-app-….ips` and its exception; Report reveals that file.
8. (Gone with the button: Preferences > Display > Colors > Report Bug
   Icon no longer tints anything.)
