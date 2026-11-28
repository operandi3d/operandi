# Operandi

Operandi is an original polygon modeler in Rust (egui/eframe + wgpu), built
from behavior specifications and meant to be open-sourced. macOS first;
Windows and Linux are design targets, not yet validated.

These rules are shared by every agent working here (Codex reads this file;
Claude reads it through `CLAUDE.md`). The maintainer owns the project and
decides scope, merges and releases.

**Contributors' agents follow these rules too**, with one difference: the
"Clean-room boundary" section is how the maintainer's own build agents
work. For a contributor's agent the minimum is the Modo rules in
[CONTRIBUTING.md](CONTRIBUTING.md) and [docs/CLEAN_ROOM.md](docs/CLEAN_ROOM.md)
(nothing from reverse engineering Modo, nothing copied from it); keeping
the stricter boundary is welcome. Where a rule names a folder, apply it to
the matching place on your machine.

**The job is working, tested modeling tools.** Checks and notes exist to
support that, not to replace it. When in doubt, build the tool.

## Clean-room boundary

This repo is the implementation side. Modo is studied in a separate, private
Research project, which turns what it learns into behavior specs. Here you
work only from the specs (the private `Operandi-specs` checkout next to
this one; see "Where things are") and this repo's own code and docs.

Never read, search, open or launch:

- `<your study folder>` (the research project, Modo's guide, old research
  and SDK archives), or anything else outside this repo that holds Modo material;
- the Modo application, its kits, preferences or SDK, and the Modo bridge's
  data folder;
- the Modo user guide or HTML help (they were removed from this repo on
  2026-10-31; the Research side reads them and writes specs instead);
- anything from reverse engineering Modo (decompiler or disassembler
  output), code from earlier prototypes, and chats or Codex sessions about
  Modo;
- other DCCs' source code, unless it is open source under a licence
  compatible with GPL v3 (Blender and its Cycles renderer qualify). Even
  then, never read code that imitates Modo, and don't copy code without
  keeping its licence notice and recording it in `docs/DEPENDENCIES.md`.
  Code adapted from Blender goes in its own GPL-2.0-or-later module with
  Blender's notice and an "Adapted from Blender `<path>` at `<commit>`" line
  (see "Licences" in CONTRIBUTING.md).

Don't run experiments to work out Modo's behavior either; that's research.
If a spec doesn't answer something, see "Gaps in a spec" below. If you
realize you've seen excluded material, stop and tell the maintainer what and
where.

Older spec folders contain process instructions from earlier agent setups
(directors, leases, "no commits"). Read specs for behavior only; process
comes from this file.

## Where things are

```
crates/operandi-core/   mesh, scene, selection, tools, commands, undo, files (no GUI)
crates/operandi-app/    egui app: panels, viewport, input, rendering
specifications/         only the command-name map (command-names/). The behavior
                        specs and their oracle data live in the private
                        ../Operandi-specs checkout (moved 2026-11-23); tests that
                        read them build only when it's found (cfg `operandi_specs`)
tools/icon-studio/      icon studio: rendered icon scenes (all new icons)
tools/operandi-icons/   old flat icon generator (deprecated; keeps existing SVGs)
docs/                   status (PROGRESS.md), open questions
                        (SPECIFICATION_QUESTIONS.md), per-milestone packets
fixtures/               original test scenes
assets/branding/        approved logo, app icon and Dock icon (see branding-manifest.json)
development/            local QA builds and output (git-ignored)
```

Past decisions live in `docs/SPECIFICATION_QUESTIONS.md` and the spec
folders in `Operandi-specs`. Don't add long entries.

## Building a tool

1. Read the tool's spec. Write 5–8 acceptance criteria before coding:
   include empty or invalid selection, cancel, undo/redo and save/reopen where
   they apply.
2. Build the whole workflow: core operation, live preview, panel controls,
   shortcut and command. Not a function nobody can reach, and not buttons
   with nothing behind them.
3. Write a focused test for each criterion. Turn bugs into regression tests.
4. Run the checks (below).
5. Add the tool's main gesture to the milestone's GUI checklist.
6. Write a short status entry: what works, what's provisional, open questions.
   A line or two per item.
7. Move on to the next tool.

Never describe a stub, partial workflow or approximation as Modo parity.

### Checks

Use the pinned toolchain (`rust-toolchain.toml`). There are two tiers.

**Fast tier, while you work:** `scripts/check-fast.sh`, or
`scripts/check-fast.sh <name>` to run only tests whose name contains
`<name>`. It runs fmt, then clippy and tests for what changed since main:
just `operandi-app` when that's all you touched, otherwise the workspace.

**Full tier, before calling a tool done and before every merge:**

```bash
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

`scripts/test-all.sh` runs the same tests as the third command with every
test program at once (about twice as fast); either counts as the full tier.
Heavy probes are `#[ignore]`d with a note saying how to run them.

The spec-backed tests (oracles, cases) need the private `../Operandi-specs`
checkout (or `OPERANDI_SPECS`); without it they are left out of the build,
and `scripts/merge-check.sh` refuses to run.

Run the full tier once before calling a tool done, and again only if you
change shared code after that. At merge time it runs on the branch with
main merged in, and main moves only when it passes: `scripts/merge-check.sh`
does that run (and the strict area dependency check, as CI does; see
CONTRIBUTING.md step 5), plus the Cycles, USD and Python extras (they only build
with `--features cycles` / `usd` / `python`, so plain `cargo test` skips
them): each is compiled on its own, then the tests run twice, with Cycles
and USD on and then with Python on. 3D
painting is a default feature (the maintainer, 2026-11-19), so every
build and test run includes it; `scripts/check-no-painting.sh` checks the
`--no-default-features` build. `merge-check.sh` builds in one shared merge
checkout, `../Operandi-merge-check` (it makes it the first time), which
keeps its own warm build folder, so a merge check rebuilds only what the
branch changed. It checks the branch's commit plus uncommitted changes to
tracked files; files you haven't `git add`ed are left out. One merge check
runs at a time (a second waits). Agents run it outside the sandbox: it
writes next to the checkout. Never build or test in
`../Operandi-merge-check` by hand: that skips the lock and can break a
merge check running there. Use your own worktree.

`command_contract_tests.rs` runs every undoable registry command and checks
that undo, redo and save/reopen agree. A new command is covered the day it
lands; if it fails for a reason you can't fix now, add it to `KNOWN` there
with what goes wrong. `monkey_tests.rs` runs seeded random sessions
(selections, commands, undo, redo, saves); a failure prints the seed and
its steps, cut to the shortest that fails, and
`OPERANDI_MONKEY_SEED=<seed>` replays one. A nightly run (`scripts/nightly.sh`, no AI agent) writes
`<test results folder>/latest.md`; see docs/CI.md and "Maintainer
machine notes" below.

- Find files with [docs/CODEMAP.md](docs/CODEMAP.md) before searching.
- Don't narrow a build to one library crate or test program
  (`cargo test -p operandi-core`, `--test all`). Cargo then picks different
  dependency features than for the workspace and recompiles core and part
  of its dependencies a second time (1-1.5 min). Use the script,
  `--workspace` with a name filter after `--`, or `-p operandi-app`.
- Core's integration tests are one test program, `tests/all.rs`; run one
  file with `cargo test --locked --workspace -- <file>::`. A new
  `tests/*.rs` file needs its two lines in `all.rs`; the
  `every_test_file_is_listed` test fails, naming them, until it has them.
- Clippy does everything `cargo check` does; don't run both.
- Don't rerun passing checks to "confirm" them, and don't build extra
  evidence logs, matrices or inventories unless the maintainer asks.

### Gaps in a spec

Don't stop. Write the question in `docs/SPECIFICATION_QUESTIONS.md` (spec ID,
starting state, actions, what's unclear), build the most conservative reading
marked *provisional*, and keep going. The maintainer takes the question to Research.
Ask the maintainer directly only when no reasonable reading exists or the choice is
hard to undo.

### Time box

If one problem has blocked a tool for about 45 minutes, stop and tell the maintainer in
three lines: what's blocking, what you tried, what you need decided.

### GUI checks

Do GUI checks in one pass after a milestone's tools are built, not during
implementation, unless a bug can't be reproduced in tests. Use the existing
QA app identity under `development/` with its own data folder
(`OPERANDI_DEV_DATA`); never run over an installed Operandi or real user scenes.
Only one agent drives the app at a time. If screen control is blocked
(permissions, "Invalid app"), try one fix, then record it and tell the maintainer.

### QA builds

Build QA apps with `scripts/bundle-dev.sh` and no output overrides. It builds
optimized with the incremental `qa` profile (a rebuild after a small change
takes seconds) and puts the app in `<build output folder>` (next to the
checkout) as `Operandi <branch> <commit>.app`, replacing that branch's
previous build, so every build is in one place. Give the maintainer that
path. Set
`OPERANDI_DEV_NAME` only for a clearer name than the branch; don't set
`OPERANDI_DEV_OUTPUT`, and never point it at `development/`. Don't hand out
debug builds.

## Code

- Geometry, selection, commands, undo and file formats live in
  `operandi-core` and are testable without a window or GPU. The app
  translates input and draws.
- Component edits change mesh data; item transforms change item state. Keep
  them apart.
- Each tool category has its own lifecycle (activate, drag, handles, live
  properties, drop, cancel, undo). Reuse shared services where the semantics
  match; keep tool-specific code local.
- Previews, committed edits, undo/redo and save/reopen must agree. Cancel and
  errors leave the scene as it was.
- Buttons, menus, shortcuts and commands go through the same operations.
  Shortcuts shown on buttons and tooltips come from the editable input map.
- Every new command has a class (UIC-225): undoable, UI, side effect or
  quiet, set on its registry entry (`.ui()`, `.side_effect()`, `.quiet()`;
  undoable otherwise). `every_command_in_the_table_has_a_class` in
  `command_record.rs` fails until it has one.
- Command, tool and argument names follow the reference name map so scripts
  port (the maintainer, 2026-11-06). Renames and the deprecated old names live in
  `operandi-core/src/commands/command_names.rs`; every entry point (`execute`, the
  MCP bridge) resolves through it. New tools take the map's name when it has
  one. See `docs/COMMAND_NAMES.md`.
- **One file per tool, one sorted line per shared list** (the maintainer,
  2026-11-19). A new tool or command lives in its own new file(s). Each
  shared file that has to know about it gets at most one added line, put
  in its sorted place, never at the end. Two people adding tools at once
  then rarely touch neighbouring lines, so their changes merge cleanly.
  - Sorted and tested (case-insensitive; the test names the entry that's
    out of place): `crates/operandi-core/tests/all.rs` (mod lines);
    `command_names.rs` `TOOL_RENAMES`, `COMMAND_RENAMES`,
    `COMMAND_EXPANSIONS`, `ARG_RENAMES`; `command_registry/tool_ids.rs`
    `TOOL_PHRASES`; `operandi-mcp` `bridge.rs` `TOOLS`, `APP_TOOLS` and
    `protocol.rs` `NON_EDITING_TOOLS`.
  - `mod` lines in `operandi-core/src/lib.rs` and `operandi-app/src/main.rs`:
    add yours inside an existing block of `mod` lines; `cargo fmt` sorts
    each block. Source files live in area folders (`polygons/`, `render/`,
    `window_layout/` ...; the list is in
    [docs/CODEMAP.md](docs/CODEMAP.md)): put a new file in its area's
    folder and give its `mod` line a `#[path = "<area>/<file>.rs"]`, as
    its neighbours have. The folders are only for filing; module paths
    stay `crate::<module>`.
  - Not sorted yet (put your line in sorted place within its section if
    there is one, but don't reorder them in a feature branch):
    `COMMANDS` (one file per area under
    `command_registry/commands/`, joined in the order `commands.rs` lists
    them; that order is the `command.list` and help order), the icon table in
    `operandi-app/src/controls/icons.rs` (about 280 lines plus the `Icon` enum,
    which derives `Ord`), the MCP `tools/list` definitions in `bridge.rs`
    (the order clients see), and the handler registrations in
    `command_handlers.rs`.
  - Never sorted, because the order is what the user sees or what gets
    saved: menu, toolbox, pie and form JSON in `operandi-app/assets/forms`,
    `TOOL_IDS` (a choice list's display order), and enums whose order is
    serialised or shown.
- **UI work follows the UI conventions bible, [docs/UI_BIBLE.md](docs/UI_BIBLE.md).**
  Before you build or change anything a user sees or touches (a panel,
  menu, button, field, dialog, list, tool behaviour, shortcut), read the
  bible's section for it. Cite its rule numbers (UIC-nn) in code comments,
  PRs and reviews when you follow or break a rule. Departures need the
  maintainer's OK; never renumber a rule.
- **No guide page, no merge (UIC-300).** A branch that adds or changes
  anything a user can see or do ships its User Guide page (new or updated,
  in `docs/guide/`) in the same branch, or says "no docs needed" in its
  merge note. Rules: [docs/guide/RULES.md](docs/guide/RULES.md).
- New icons come only from the icon studio (rendered); see "Icons" below.
- Safe Rust; any `unsafe` needs a written safety reason. Ordinary user errors
  return errors, not panics.
- Write readable code: clear names, comments for non-obvious decisions.
  Make the changes a tool needs; save refactors for when the maintainer asks.
- Rendering (the maintainer, 2026-11-13): Cycles is the source of truth for physics;
  no Modo-imitating fudges on top of it. Fix render differences only when
  they are real bugs (units, missing feature, colour, ignored setting). The
  shader tree compiles to plain Cycles / OpenPBR materials. See
  `docs/RENDERING.md`.
- Record new dependencies with their licenses in `docs/DEPENDENCIES.md`.
- Changing the Cycles C interface (`operandi_cycles.h`: a struct, enum value
  or function)? Bump `OC_ABI_VERSION` there and in `operandi-cycles/src/lib.rs`,
  then rerun `scripts/build-cycles.sh`.

### Icons

**Every new icon is rendered with the icon studio** (the maintainer,
2026-11-20): a small 3D scene in `tools/icon-studio` rendered with Cycles,
so the whole set shares one camera, light rig and set of materials. Read
`tools/icon-studio/README.md` before touching icons.

- **Add an icon:** model its scene inside the studio's icon box (in
  Operandi, or as a `Sample` in `operandi-icon-studio/src/studio.rs`), tag
  polygons with the studio materials (Selected yellow-orange always means
  "selected", New cyan new geometry, Axis X/Y/Z red/green/blue), and save it
  as `tools/icon-studio/icons/<slug>.opscene`.
- **Render and install:** `scripts/render-icons.sh --install` renders every
  icon at 16-64 px in a dark-outline and a light-outline set, writes
  `development/icon-studio/contact-sheet.png`, and copies the PNGs into
  `crates/operandi-app/assets/rendered-icons`. Then add the icon's line
  (`<slug> <Icon variant>`, in sorted place) to `icons.txt` in that folder;
  the app's build embeds the listed PNGs. A brand-new icon has no SVG: add its
  `Icon` variant and put it under `rendered only:` in the icon table in
  `operandi-app/src/controls/icons.rs`; forms and presets name it by its slug.
- **Review:** show the maintainer the contact sheet. New or changed artwork
  needs their visual approval.
- Change the shared look (camera, lights, materials) only in the studio, and
  only with the maintainer's OK.
- The studio is a developer tool in its own crate, `operandi-icon-studio`
  (the app doesn't depend on it); the app ships only the PNGs.
- Never draw from screenshots or other applications' icons, and never use
  image generation for final artwork.
- **Any icon not made with the studio is a placeholder.** A contributor
  (or their agent) may add one so a feature has a button, but must say
  "placeholder icon: <name>" in the pull request. The maintainer replaces
  it with a studio icon so the set keeps one consistent look. Don't spend
  effort polishing a placeholder.

**The old flat generator** (`tools/operandi-icons`) is deprecated: don't
add icons to it. It only keeps the existing SVGs, which stay in the app as
fallbacks and for icons not yet rendered; `app_icons_match_the_generator`
still checks them. Fix an existing SVG there only if the maintainer asks,
and never hand-edit a generated SVG.

### Fast drags

A tool's preview runs on every frame of a drag, on meshes of 100k+
polygons. Every interactive tool, old or new, follows these rules:

- Work near the selection. Don't build whole-mesh maps or sets per frame
  when only the selection's neighbourhood is read; flag arrays
  (`Mesh::used_points`) beat sets, and never scan every polygon per point.
- Topology results come from the shared, per-topology memo: `Mesh::edges`,
  `Mesh::edge_counts` and `Mesh::validate` (`topology_cache.rs`). Add new
  topology-only results there instead of recomputing them.
- Anything derived only from the drag's base scene (handle frames, rails)
  is computed once per drag (`BaseMemo` in `viewport.rs`).
- The viewport updates what changed: subdivision and GPU geometry are
  patched locally when only points or creases change; draw overlays on the
  graphics card rather than with CPU hidden-line tests.
- Selection changes and paint drags patch only the polygons whose state
  changed, and picking projects the mesh once per pointer move, not once
  per pixel along it.
- Add every new tool to `tool_speed_guard_tests.rs` (it won't compile
  otherwise). It fails when a drag re-evaluates the surface, GPU geometry,
  picking or topology after its first frames. Check the per-frame cost with
  the opt-in probes before calling a tool done:
  `cargo test --release -p operandi-core --test all tool_speed_probe:: -- --ignored --nocapture`
  and `OPERANDI_BENCH_FILTER=tool/ cargo test --release -p operandi-app tool_frames_subdivided -- --ignored --nocapture`.
  On the 16k-polygon grid a drag frame should stay under about 16 ms.

## Git

The maintainer is new to Git. Explain any choice they need to make in plain
language.

- One branch per milestone or bounded feature (e.g. `m4-polygon-tools`),
  started from `main`. Never work directly on `main` without the maintainer's OK.
- At most one worktree per branch, at `../Operandi-<branch>` (next to the
  main checkout). Make it with `scripts/new-worktree.sh <branch>`: it seeds
  the new worktree's build cache from the main checkout where it can, so
  the first build doesn't start cold. Reuse
  it when you come back. Don't make worktrees or branches for single tools,
  checks, experiments or restarted sessions. If your app already put you in
  its own worktree, work there and don't make another.
- The first build in a new worktree can fail with "cmake configure failed"
  (from `manifold-csg-sys`, whose copied build output still points at the
  other checkout). Delete its build output (`target/*/build/manifold-csg-sys-*`,
  or the same under your Cargo build folder) and build again.
- Commit on your branch when a tool is done or the maintainer asks. Don't commit:
  `target/`, `development/`, `.docx` files, user scenes (`*.opscene` outside
  `fixtures/`), anything from Modo, or secrets.
- Commit with a sign-off (`git commit -s`), as every contributor must
  (DCO; see "Signing off your work" in CONTRIBUTING.md). The sign-off is
  the human's name, never the AI tool's.
- AI-assisted commits end the commit message with a `Co-Authored-By` line
  for the AI tool used, for example
  `Co-Authored-By: Claude <noreply@anthropic.com>`. Say which tool you used
  in the pull request too.
- Don't push, merge into `main`, rebase, reset, stash, force-delete or rewrite
  history without the maintainer's OK. Propose a merge and wait.
- Merging: one merge at a time (one merge slot; wait for the one in progress
  to land). Bring the branch up to date by merging `main` into it, never by
  rebasing. Then `scripts/merge-check.sh` runs the full tier on the result,
  and `main` moves by fast-forward only. The maintainer does the merge into
  `main` and every push.
- After the maintainer merges a branch, remove its worktree (`git worktree remove`,
  never `--force`) and delete the branch with `git branch -d`.
- Specs arrive as their own commits in the private `Operandi-specs` repo
  (`spec: <tool> (from <source sha>)`). Treat them as input. The public repo
  never carries specs, oracle data or Modo measurements (2026-11-23).

## Cleanup

You may delete files you generated for your own task (build output, temp
scenes, scratch files). Don't delete anything you didn't make, user
scenes, fixtures, or another agent's output.

## Maintainer machine notes (optional)

These describe the maintainer's own setup. Nothing above depends on them;
skip them on your machine.

- `scripts/new-worktree.sh` clones the main checkout's Cargo build folder
  and its built Cycles and USD libraries into the new worktree (an instant
  copy on APFS). Without those, the worktree simply builds from scratch.
- The Cargo build folder can live on another disk (an external drive, set
  with Cargo's `build-dir`); paths under `target/` above then mean that
  folder.
- The merge checkout (`~/Developer/Operandi-merge-check`, a clone that
  borrows the main repository's objects) has its build folder there too,
  about 25-35 GB; `merge-check.sh` starts it over past 150 GB. The hourly
  cache cleanup never deletes it: it keeps every build folder that belongs
  to a clone rather than to one of the main repository's worktrees.
- `scripts/nightly.sh` runs every night from a launchd job
  (`scripts/nightly-install.sh`) and writes its report to a test results
  folder; see docs/CI.md.
- CI's macOS and Linux jobs run on the maintainer's own machines while the
  repository is private (docs/CI.md).
