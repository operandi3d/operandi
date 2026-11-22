# Worktree survey — 2026-10-31

Survey of all 93 worktrees of Operandi-Rust (main checkout + 92 task
worktrees), taken 2026-10-31 07:35–07:50 CDT. Tables reflect the state after both
commit passes (second pass approved by Jason 2026-10-31). This file is not committed.

## How the work is laid out (the big picture)

- `main` is at `1d7e106` (2026-10-25). Only 10 commits exist after it; almost
  all work since then lives as **uncommitted files** in worktrees.
- Committed history after main is one straight line:
  `1d7e106 → 7154b25 → 15100fa → 1d5ebac → 11c0c73 → 51274ff` (checkpoint,
  2026-10-27). Two side commits: `52cafca` (extrude-dedupe-validation) and the
  10-23 `integration` branch (Bridge S1 + Pen S1 merges, superseded by 51274ff).
- Each new task worktree was started by **copying the previous worktree's
  uncommitted files forward**. A file-by-file fingerprint comparison shows a
  single chain where every file of the older worktree exists in the newer one
  (newer versions may have edited it):

  ```
  ~56 task worktrees on 51274ff (LXO, Smooth Shift, Cone, Extrude, M1 tools…)
    → m1-integration-a32 … a39 → a40 (codex/aa4c) → a41 (codex/671b)
    → m1-make-basic → m1-common-baselines → m1-poly-fixes
    → m2-edge-tools → m3-vertex-tools   (newest; contains everything above)
  ```

  So `m3-vertex-tools` (and one step earlier, `m2-edge-tools`) already holds
  the carried-forward work of nearly every other worktree. The exception is
  `m15-completion` (M1.5 Select More/Less/Between/Fill), built separately from
  main: 2 of its files are not in m3 and the shared files differ.

## Flags

- **Possibly active:** `m3-vertex-tools` (files edited 07:32 today, minutes
  before the survey). Also `~/.codex/worktrees/m1-make-basic` is the working
  folder of several open Claude sessions. Neither was written to.
- **QUOTA_HANDOVER with writer_stopped:** only
  `Operandi-Rust-m1-integration-a34/docs/m1-integration-a34-20261029/QUOTA_HANDOVER.json`
  (old owner: a Claude Opus session; new owner: GPT-6.1 Terra session
  `01a18076…`). Not written to.
- **Seal files** (`*SEAL*.json`, e.g. `FINAL_SOURCE_SEAL.json`) appear as
  untracked files in **62 worktrees**. Jason approved committing them
  as-is on 2026-10-31; 61 were committed in the second pass (below).
- **Files that must not be committed and were left untracked:** 338 `*.opscene`
  scenes (mostly `docs/*/scenes/`, demo/validation scenes), 7 `.docx` files in
  the main checkout's `docs/`, and `target-m1-scale-offset/` (≈49,600 build
  files in `Operandi-Rust-m1-scale-offset`, not covered by `.gitignore`, which
  only ignores `/target/`). Suggest adding `/target-*/` to `.gitignore`.
- **Committed `.gitignore` on main lacks the Modo guide rules**; only the main
  checkout's uncommitted `.gitignore` has them. Worktrees branched from main
  would show the guide files as untracked if they were copied in (none were
  found in this survey).
- **Ignored `development/` folders hold validation logs, scenes and dev app
  bundles** (e.g. `final-results.json`, `Untitled.opscene`,
  `Operandi Rust Pen S1 Dev.app`). `git worktree remove` would delete them,
  and AGENTS.md protects retained validation logs and user scenes, so no
  worktree was removed (see group (a)).

## AGENTS.md versions (not committed anywhere in this pass)

| Version (blob) | Worktrees | What it adds over main's AGENTS.md |
|---|---|---|
| unchanged | 28 | — (all the 10-23…10-27 stabilization/review worktrees) |
| `b83ba8b` | 46 | +61 lines: "Model selection and reasoning effort" (GPT-6.1 Terra policy, 10-28) |
| `34bff41` | 10 | +37 lines: "Native GUI access and QA preflight" (LXO acceptance worktrees, 3716) |
| `227acc3` | 1 | +32 lines: earlier Native GUI QA preflight (bevel-weld-acceptance) |
| `bbac36f` | 1 | +98 lines: adds "Generated-file cleanup — 2026-10-30" (671b / a41) |
| `8e91dd5` | 2 | rewrite with "Read first: delivery priority — 2026-10-30" (m1-make-basic, m1-common-baselines) |
| `b0f2fab` | 1 | same rewrite, later revision (m1-poly-fixes) |
| `8aacb7d` | 4 | newest rewrite (main checkout, m15-completion, m2-edge-tools, m3-vertex-tools) |

`8aacb7d` (in the main checkout) is the newest and the one to settle on, unless
Jason wants something from an older variant.

## Commits made in this pass (each on the worktree's own branch)

| Worktree | Branch | New commit | Files | Left uncommitted |
|---|---|---|---|---|
| Operandi-Rust-m15-completion | codex/m15-completion | cf372c9 | 14 | AGENTS.md |
| Operandi-Rust-stab00 | codex/post-i2-stab00 | ce58406 | 54 | 1 `.opscene` |
| Operandi-Rust-stab07-environment | codex/stab07-environment | 01af64d | 16 | — |
| Operandi-Rust-stab07c-writers | codex/stab07c-writers | c2420c7 | 15 | — |
| Operandi-Rust-stab09-wire-contract | codex/stab09-wire-contract | acbce3d | 29 | — |
| Operandi-Rust-i2-fastpath-guard-fix | codex/i2-fastpath-guard-fix | d81280b | 9 | — |
| Operandi-Rust-i1-identity-correspondence | codex/i1-identity-correspondence | 888fd7d | 26 | — |
| .codex/worktrees/smooth-shift-fix | codex/smooth-shift-fix | 6198ad7 | 31 | AGENTS.md |
| .codex/worktrees/3716 | codex/checkpoint-bridge-stack | ffd34ef | 5 | AGENTS.md |
| .codex/worktrees/0872 | codex/bevel-weld-readiness | 221d9b6 | 102 | AGENTS.md |
| Operandi-Rust-fastpath-review-20261027 | codex/fastpath-review-20261027 | a7adef7 | 3 | — |
| Operandi-Rust-i2-live-history | codex/i2-live-history | b682640 | 5 | — |
| Operandi-Rust-shared-review-20261027 | codex/shared-review-20261027 | b72bdc2 | 2 | — |
| Operandi-Rust-stab-integration-plan | codex/stab-integration-plan | eed2e40 | 7 | — |
| Operandi-Rust-stab01-review | codex/stab01a-review | 882afce | 1 | — |
| Operandi-Rust-stab02-review | codex/stab02b-review | 46ff15e | 6 | — |
| Operandi-Rust-stab02c-activation-plan | codex/stab02c-activation-plan | d9f4875 | 6 | — |
| Operandi-Rust-stab03-outcome-plan | codex/stab03-outcome-plan | e53f37c | 7 | — |
| Operandi-Rust-stab07-review | codex/stab07b-review | 5e09de9 | 2 | — |
| Operandi-Rust-stab10-assessment | codex/stab10-mesh-assessment | 2d84867 | 14 | — |
| Operandi-Rust-tool-readiness | codex/tool-readiness | 9b87d15 | 28 | — |

Only `m15-completion` was build-checked (`cargo build --workspace` passes).
None were marked WIP; the others were not built.

## Second commit pass — seal-holding worktrees (approved by Jason)

61 worktrees committed with "Snapshot <branch> work: …" messages, each on its
own branch, not build-checked: pen-s1-stabilized-integration,
bevel-lxo-validation, m1-make-integration-a41, m1-integration-a40,
lxo-native-safety, all 13 other lxo-* branches, m1-common-baselines,
bevel-weld-acceptance, bridge-recovery-save, cone-new-retention,
cone-repeat-history, extrude-numeric-panel, extrude-spike-chain,
extrude-uv-availability, m1-align, m1-collapse, m1-existing-basics,
m1-flatten-ngons, m1-flip, m1-integration-a32/a36/a37/a38/a39, m1-merge,
m1-pen-interactive, m1-poly-fixes, m1-polygon-split, m1-quadruple,
m1-reduction, m1-remove, m1-scale-offset, m1-set-material, m1-set-part,
m1-set-position, m1-spin-quad, m1-triple, m1-unify, m2-edge-tools,
new-scene-empty-mesh, smooth-shift-escape-focus, smooth-shift-native-oracle,
smooth-shift-picking-freshness, stab03a-review, stab11-inherited-lints,
stabilization-claude-handoff, surface-identity-cache,
surface-tool-states-live, surface-tool-states-maintenance, thicken-shell-live.
Commit hashes are in the HEAD column of the tables below. `m2-edge-tools`
(df11f02) was build-checked afterwards: `cargo build --workspace` passes.

Left uncommitted on purpose: AGENTS.md in each; `.opscene` scenes
(including `docs/i2-gap-plan/wire/`, which holds only two scenes);
`target-m1-scale-offset/`.

**Not committed: `p01-identity`.** Its untracked files are 64 MB of old
prototype material (`prototypes/*/evidence`, vendored `third_party/` folders,
three ~9 MB JSON fixtures) on a branch older than main. Jason decided
(2026-10-31) to leave it out of Git; the files stay on disk in that worktree.

## Third pass — Jason's decisions (2026-10-31)

- **Committed** `m3-vertex-tools` (f3588e7, marked WIP: Jason hasn't tested
  M3; don't merge it yet) and `m1-make-basic` (77992ad). Jason confirmed
  nobody was working in them.
- **AGENTS.md:** keep the newest version, `8aacb7d` (main checkout). Jason
  plans to improve it once this cleanup is done. Still uncommitted
  everywhere.
- **p01-identity:** left out of Git by decision.

## Removed

`git worktree remove` (no `--force`), approved by Jason, for four clean
worktrees whose commits are in other branches. Their ignored `development/`
folders (logs, scenes, dev app bundles) went with them:

| Worktree | Branch | Commit is also in |
|---|---|---|
| Operandi-Rust-bridge-s1 | codex/bridge-s1 | codex/integration |
| Operandi-Rust-pen-numeric-stage-1 | codex/pen-numeric-stage-1 | codex/integration |
| Operandi-Rust-extrude-drag-fast-path | codex/extrude-drag-fast-path | 51274ff line (e.g. codex/bevel-lxo-validation) |
| Operandi-Rust-shared-mesh-speed | codex/shared-mesh-speed | 51274ff line |

The four branch names remain: `git branch -d` refuses them because their
commits aren't in main yet. They take no disk space.

## Proposed merge order (nothing merged)

1. **`codex/m2-edge-tools`** (m3 waits until Jason has tested Milestone 3) — **or `codex/m3-vertex-tools`** — once m3 is idle and
   both are committed. They contain the carried-forward work of all M1 tools,
   LXO, Smooth Shift, Bridge, Pen and stabilization. Both start from main
   `1d7e106`, so the first one merges into main with **no conflicts**.
   Pick m2 if Milestone 3 is not released yet (AGENTS.md says M3 is later
   scope); merging m3 after m2 would conflict in the ~16 files both changed
   (`command.rs`, `icons.rs`, `input.rs`, `main.rs`, `panels.rs`, `qa.rs`,
   `toolbox.rs`, icon `manifest.json`, …).
2. **`codex/m15-completion`** — expected conflicts in `selection.rs`,
   `editor.rs`, `lib.rs`, app `main.rs`, `input.rs`, `panels.rs`,
   `app_menu.rs`, `viewport.rs`, `tests.rs`, `IMPLEMENTATION_BASELINE.md`,
   `docs/MILESTONE_1_5.md`, `docs/SPECIFICATION_QUESTIONS.md`. Build and run
   the selection tests after resolving.
3. **Everything else: superseded — do not merge.** The 51274ff checkpoint line,
   all task worktrees on it, the a32–a41 integrations, m1-make-basic,
   m1-common-baselines, m1-poly-fixes, the stab* branches and `integration`
   are older copies of what is in m2/m3. Keep them as an archive, or delete
   them later (deleting unmerged branches needs `git branch -D`, which this
   pass was not allowed to do).
   Worth a quick look before discarding, because they hold files not found in
   m3: `extrude-dedupe-validation` (52cafca, a small Extrude perf change),
   `stab10-assessment` (3 experiment files), `integration` (1 file).

## Per-worktree tables

Columns: Base = merge-base with main; Modified/Untracked = counts after this
pass (untracked counts include `.opscene`, seals and evidence folders);
Newest file excludes `target/`, `development/`; AGENTS.md = blob of the
working copy, or "same" when unchanged from the branch.

### (a) Already contained elsewhere — safe to remove (blocked: see note) — 4

| Path | Branch | HEAD | Base | Ahead/behind main | Modified | Untracked | Newest file | Contained? | AGENTS.md | What it is |
|---|---|---|---|---|---|---|---|---|---|---|
| `~/Documents/Operandi-Rust-bridge-s1` | `bridge-s1` | 5db41c4b 2026-10-23 | 162b920 | +1/−1 | 0 | 0 | 10-23 20:16 | in another branch | same | bridge s1 |
| `~/Documents/Operandi-Rust-extrude-drag-fast-path` | `extrude-drag-fast-path` | 11c0c734 2026-10-27 | 1d7e106 | +4/−0 | 0 | 0 | 10-27 14:04 | in another branch | same | extrude drag fast path |
| `~/Documents/Operandi-Rust-pen-numeric-stage-1` | `pen-numeric-stage-1` | 75c07a33 2026-10-23 | 162b920 | +1/−1 | 0 | 0 | 10-23 20:19 | in another branch | same | pen numeric stage 1 |
| `~/Documents/Operandi-Rust-shared-mesh-speed` | `shared-mesh-speed` | 1d5ebac2 2026-10-27 | 1d7e106 | +3/−0 | 0 | 0 | 10-26 23:06 | in another branch | same | shared mesh speed |

### (b) Committed but not merged into main — 84

| Path | Branch | HEAD | Base | Ahead/behind main | Modified | Untracked | Newest file | Contained? | AGENTS.md | What it is |
|---|---|---|---|---|---|---|---|---|---|---|
| `~/.codex/worktrees/0872/Operandi-Rust` | `bevel-weld-readiness` | 221d9b6f 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 21:09 | — | b83ba8b | Readiness assessments for 18 queued tasks (bevel weld, bridge, cone, extrude, LXO) (committed today) |
| `~/.codex/worktrees/3716/Operandi-Rust` | `checkpoint-bridge-stack` | ffd34ef9 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-27 23:16 | — | 34bff41 | Checkpoint notes for the Bridge stack (committed today) |
| `~/.codex/worktrees/457d/Operandi-Rust` | `pen-s1-stabilized-integration` | e29e8e08 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 21:38 | — | same | Pen S1 stabilized integration + seals (committed today) |
| `~/.codex/worktrees/6186/Operandi-Rust` | `bevel-lxo-validation` | 2185950b 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 08:41 | — | b83ba8b | Bevel/LXO validation, plus native-QA folders for cone/ellipsoid/extrude (committed today) |
| `~/.codex/worktrees/671b/Operandi-Rust` | `m1-make-integration-a41` | e69099da 2026-10-31 | 1d7e106 | +1/−0 | 1 | 5 | 10-30 13:51 | — | bbac36f | M1 Make integration A41 (carries all M1 tool work forward) (committed today) |
| `~/.codex/worktrees/aa4c/Operandi-Rust` | `m1-integration-a40` | d1b59ce3 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 23:53 | — | b83ba8b | M1 integration A40 (Flatten N-gons UI etc.) (committed today) |
| `~/.codex/worktrees/b334/Operandi-Rust` | `lxo-native-safety` | ebcce146 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 04:26 | — | b83ba8b | LXO native safety checks (committed today) |
| `~/.codex/worktrees/lxo-dependency-refusal/Operandi-Rust` | `lxo-dependency-refusal` | 503cba13 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 11:52 | — | b83ba8b | LXO dependency refusal (committed today) |
| `~/.codex/worktrees/lxo-geometry-block-acceptance/Operandi-Rust` | `lxo-geometry-block-acceptance` | 31c6de22 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 10:50 | — | 34bff41 | LXO geometry block acceptance (committed today) |
| `~/.codex/worktrees/lxo-graph-reference-acceptance/Operandi-Rust` | `lxo-graph-reference-acceptance` | 8d91bfa6 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 10:10 | — | 34bff41 | LXO graph reference acceptance (committed today) |
| `~/.codex/worktrees/lxo-map-tag-acceptance/Operandi-Rust` | `lxo-map-tag-acceptance` | d2ee15b0 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 10:25 | — | 34bff41 | LXO map tag acceptance (committed today) |
| `~/.codex/worktrees/lxo-numeric-safety/Operandi-Rust` | `lxo-numeric-safety` | 474d0422 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 11:21 | — | b83ba8b | LXO numeric safety (committed today) |
| `~/.codex/worktrees/lxo-profile-refusal/Operandi-Rust` | `lxo-profile-refusal` | 61dc7538 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 11:38 | — | b83ba8b | LXO profile refusal (committed today) |
| `~/.codex/worktrees/m1-common-baselines/Operandi-Rust` | `m1-common-baselines` | f0f35342 2026-10-31 | 1d7e106 | +1/−0 | 1 | 46 | 10-30 14:47 | — | 8e91dd5 | M1 common baselines (new-mesh rows, demonstrations, native fixtures) (committed today) |
| `~/.codex/worktrees/smooth-shift-fix/Operandi-Rust` | `smooth-shift-fix` | 6198ad75 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 15:41 | — | b83ba8b | Integration of LXO acceptance, Bridge recovery-save, Smooth Shift fixes (committed today) |
| `~/Documents/Operandi-Rust-bevel-weld-acceptance` | `bevel-weld-acceptance` | e132d0bf 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-27 23:16 | — | 227acc3 | bevel weld acceptance (committed today) |
| `~/Documents/Operandi-Rust-bridge-recovery-save` | `bridge-recovery-save` | 94a04f15 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 15:19 | — | b83ba8b | bridge recovery save (committed today) |
| `~/Documents/Operandi-Rust-cone-new-retention` | `cone-new-retention` | e8dd78a6 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 20:12 | — | b83ba8b | cone new retention (committed today) |
| `~/Documents/Operandi-Rust-cone-repeat-history` | `cone-repeat-history` | fa84d1d7 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 20:45 | — | b83ba8b | cone repeat history (committed today) |
| `~/Documents/Operandi-Rust-extrude-dedupe-validation` | `extrude-dedupe-validation` | 52cafca0 2026-10-27 | 1d7e106 | +4/−0 | 0 | 0 | 10-27 08:32 | — | same | Validate each mesh once per Extrude preview sample (perf) |
| `~/Documents/Operandi-Rust-extrude-numeric-panel` | `extrude-numeric-panel` | e9818547 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 17:58 | — | b83ba8b | extrude numeric panel (committed today) |
| `~/Documents/Operandi-Rust-extrude-spike-chain` | `extrude-spike-chain` | 03feda08 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 19:01 | — | b83ba8b | extrude spike chain (committed today) |
| `~/Documents/Operandi-Rust-extrude-uv-availability` | `extrude-uv-availability` | 04faf9bf 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 19:26 | — | b83ba8b | extrude uv availability (committed today) |
| `~/Documents/Operandi-Rust-fastpath-review-20261027` | `fastpath-review-20261027` | a7adef7e 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 14:23 | — | same | fastpath review 20261027 (committed today) |
| `~/Documents/Operandi-Rust-i1-identity-correspondence` | `i1-identity-correspondence` | 888fd7d6 2026-10-31 | 1d7e106 | +1/−0 | 0 | 0 | 10-25 11:47 | — | same | I2 planning notes (committed today) |
| `~/Documents/Operandi-Rust-i2-fastpath-guard-fix` | `i2-fastpath-guard-fix` | d81280b0 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 14:24 | — | same | Guard I2 Extrude fast path (committed today) |
| `~/Documents/Operandi-Rust-i2-live-history` | `i2-live-history` | b682640e 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 20:01 | — | same | Post-I2 STAB00 notes (committed today) |
| `~/Documents/Operandi-Rust-integration` | `integration` | 6096e32f 2026-10-23 | 162b920 | +5/−1 | 0 | 0 | 10-23 20:38 | — | same | Old (10-23) merge of Bridge S1 + Pen S1; superseded by checkpoint 51274ff |
| `~/Documents/Operandi-Rust-lxo-allocation-measurement` | `lxo-allocation-measurement` | fb613aaf 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 08:38 | — | 34bff41 | LXO allocation measurement (committed today) |
| `~/Documents/Operandi-Rust-lxo-budget-preflight` | `lxo-budget-preflight` | 39b998d7 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 08:56 | — | 34bff41 | LXO budget preflight (committed today) |
| `~/Documents/Operandi-Rust-lxo-malformed-acceptance` | `lxo-malformed-acceptance` | 2cc2f351 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-27 23:49 | — | 34bff41 | LXO malformed acceptance (committed today) |
| `~/Documents/Operandi-Rust-lxo-resource-safety` | `lxo-resource-safety` | 1968d80b 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 07:53 | — | 34bff41 | LXO resource safety (committed today) |
| `~/Documents/Operandi-Rust-lxo-string-face-acceptance` | `lxo-string-face-acceptance` | fad518c9 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 08:08 | — | 34bff41 | LXO string face acceptance (committed today) |
| `~/Documents/Operandi-Rust-lxo-transaction-isolation` | `lxo-transaction-isolation` | 395c1886 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 09:09 | — | 34bff41 | LXO transaction isolation (committed today) |
| `~/Documents/Operandi-Rust-m1-align` | `m1-align` | cc36d60f 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 08:47 | — | b83ba8b | M1 align (committed today) |
| `~/Documents/Operandi-Rust-m1-collapse` | `m1-collapse` | a8519b0c 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 12:59 | — | b83ba8b | M1 collapse (committed today) |
| `~/Documents/Operandi-Rust-m1-existing-basics` | `m1-existing-basics` | 99aec6e2 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 00:22 | — | b83ba8b | M1 existing basics (committed today) |
| `~/Documents/Operandi-Rust-m1-flatten-ngons` | `m1-flatten-ngons` | ea68a397 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 16:50 | — | b83ba8b | M1 flatten ngons (committed today) |
| `~/Documents/Operandi-Rust-m1-flip` | `m1-flip` | f17f6f64 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 22:56 | — | b83ba8b | M1 flip (committed today) |
| `~/Documents/Operandi-Rust-m1-integration-a32` | `m1-integration-a32` | f5bc98bf 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 02:19 | — | b83ba8b | M1 integration a32 (committed today) |
| `~/Documents/Operandi-Rust-m1-integration-a36` | `m1-integration-a36` | 04cbee90 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 07:07 | — | b83ba8b | M1 integration a36 (committed today) |
| `~/Documents/Operandi-Rust-m1-integration-a37` | `m1-integration-a37` | 8e19b4dc 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 16:31 | — | b83ba8b | M1 integration a37 (committed today) |
| `~/Documents/Operandi-Rust-m1-integration-a38` | `m1-integration-a38` | 563f4ebb 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 14:11 | — | b83ba8b | M1 integration a38 (committed today) |
| `~/Documents/Operandi-Rust-m1-integration-a39` | `m1-integration-a39` | e740d3b4 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 20:32 | — | b83ba8b | M1 integration a39 (committed today) |
| `~/Documents/Operandi-Rust-m1-merge` | `m1-merge` | 24e4302f 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 08:14 | — | b83ba8b | M1 merge (committed today) |
| `~/Documents/Operandi-Rust-m1-pen-interactive` | `m1-pen-interactive` | 0aab4d70 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 23:47 | — | b83ba8b | M1 pen interactive (committed today) |
| `~/Documents/Operandi-Rust-m1-poly-fixes` | `m1-poly-fixes` | a4d6b0ee 2026-10-31 | 1d7e106 | +1/−0 | 1 | 46 | 10-30 17:55 | — | b0f2fab | M1 Polygon fix pass (committed today) |
| `~/Documents/Operandi-Rust-m1-polygon-split` | `m1-polygon-split` | 61795000 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 04:08 | — | b83ba8b | M1 polygon split (committed today) |
| `~/Documents/Operandi-Rust-m1-quadruple` | `m1-quadruple` | c22dc7c7 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 13:43 | — | b83ba8b | M1 quadruple (committed today) |
| `~/Documents/Operandi-Rust-m1-reduction` | `m1-reduction` | 908d9725 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 01:40 | — | b83ba8b | M1 reduction (committed today) |
| `~/Documents/Operandi-Rust-m1-remove` | `m1-remove` | 7e8d90f8 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 23:38 | — | b83ba8b | M1 remove (committed today) |
| `~/Documents/Operandi-Rust-m1-scale-offset` | `m1-scale-offset` | e5397847 2026-10-31 | 1d7e106 | +6/−0 | 1 | 49441 | 10-29 15:05 | — | b83ba8b | M1 scale offset (committed today) |
| `~/Documents/Operandi-Rust-m1-set-material` | `m1-set-material` | da321e44 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 02:07 | — | b83ba8b | M1 set material (committed today) |
| `~/Documents/Operandi-Rust-m1-set-part` | `m1-set-part` | 472602f0 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 21:50 | — | b83ba8b | M1 set part (committed today) |
| `~/Documents/Operandi-Rust-m1-set-position` | `m1-set-position` | 2c8a8d98 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 00:52 | — | b83ba8b | M1 set position (committed today) |
| `~/Documents/Operandi-Rust-m1-spin-quad` | `m1-spin-quad` | 0509bfa6 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 05:53 | — | b83ba8b | M1 spin quad (committed today) |
| `~/Documents/Operandi-Rust-m1-triple` | `m1-triple` | 8986add7 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 22:44 | — | b83ba8b | M1 triple (committed today) |
| `~/Documents/Operandi-Rust-m1-unify` | `m1-unify` | ccc35300 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-29 10:57 | — | b83ba8b | M1 unify (committed today) |
| `~/Documents/Operandi-Rust-m15-completion` | `m15-completion` | cf372c98 2026-10-31 | 1d7e106 | +1/−0 | 1 | 0 | 10-30 23:22 | — | 8aacb7d | M1.5 Select More/Less/Between/Fill (committed today) |
| `~/Documents/Operandi-Rust-m2-edge-tools` | `m2-edge-tools` | df11f021 2026-10-31 | 1d7e106 | +1/−0 | 1 | 92 | 10-30 23:05 | — | 8aacb7d | Milestone 2 edge tools (current priority per AGENTS.md) (committed today) |
| `~/Documents/Operandi-Rust-new-scene-empty-mesh` | `new-scene-empty-mesh` | 66a4da11 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 18:46 | — | b83ba8b | new scene empty mesh (committed today) |
| `~/Documents/Operandi-Rust-shared-review-20261027` | `shared-review-20261027` | b72bdc2e 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 14:41 | — | same | shared review 20261027 (committed today) |
| `~/Documents/Operandi-Rust-smooth-shift-escape-focus` | `smooth-shift-escape-focus` | 57cd4b3e 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 13:43 | — | b83ba8b | smooth shift escape focus (committed today) |
| `~/Documents/Operandi-Rust-smooth-shift-native-oracle` | `smooth-shift-native-oracle` | 89f47ada 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 12:52 | — | b83ba8b | smooth shift native oracle (committed today) |
| `~/Documents/Operandi-Rust-smooth-shift-picking-freshness` | `smooth-shift-picking-freshness` | a1f5f379 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 14:16 | — | b83ba8b | smooth shift picking freshness (committed today) |
| `~/Documents/Operandi-Rust-stab-integration-plan` | `stab-integration-plan` | eed2e409 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 21:27 | — | same | stab integration plan (committed today) |
| `~/Documents/Operandi-Rust-stab00` | `post-i2-stab00` | ce584065 2026-10-31 | 1d7e106 | +2/−0 | 0 | 1 | 10-26 22:06 | — | same | History policy + session ownership (STAB00-02) (committed today) |
| `~/Documents/Operandi-Rust-stab01-review` | `stab01a-review` | 882afce6 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 21:19 | — | same | stab01 review (committed today) |
| `~/Documents/Operandi-Rust-stab02-review` | `stab02b-review` | 46ff15e4 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 21:56 | — | same | stab02 review (committed today) |
| `~/Documents/Operandi-Rust-stab02c-activation-plan` | `stab02c-activation-plan` | d9f48751 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 14:29 | — | same | stab02c activation plan (committed today) |
| `~/Documents/Operandi-Rust-stab03-outcome-plan` | `stab03-outcome-plan` | e53f37c3 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 15:19 | — | same | stab03 outcome plan (committed today) |
| `~/Documents/Operandi-Rust-stab03a-review` | `stab03a-review` | bfbcd438 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 15:52 | — | same | stab03a review (committed today) |
| `~/Documents/Operandi-Rust-stab07-environment` | `stab07-environment` | 01af64dc 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 22:16 | — | same | Shared app environment (STAB07a/b) (committed today) |
| `~/Documents/Operandi-Rust-stab07-review` | `stab07b-review` | 5e09de9f 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 22:18 | — | same | stab07 review (committed today) |
| `~/Documents/Operandi-Rust-stab07c-writers` | `stab07c-writers` | c2420c7b 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 14:45 | — | same | Shared resource writer (STAB07c) (committed today) |
| `~/Documents/Operandi-Rust-stab09-wire-contract` | `stab09-wire-contract` | acbce3db 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 16:11 | — | same | History completion + mode eligibility (STAB09) (committed today) |
| `~/Documents/Operandi-Rust-stab10-assessment` | `stab10-mesh-assessment` | 2d84867e 2026-10-31 | 1d7e106 | +2/−0 | 0 | 0 | 10-26 21:35 | — | same | Mesh adjacency assessment/experiment (STAB10) (committed today) |
| `~/Documents/Operandi-Rust-stab11-inherited-lints` | `stab11-inherited-lints` | bd12133e 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 16:01 | — | same | Clippy lint cleanup (STAB11) (committed today) |
| `~/Documents/Operandi-Rust-stabilization-handoff` | `stabilization-claude-handoff` | 1f872d9a 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 16:54 | — | same | Resumed stabilization closeout/handoff to Claude (committed today) |
| `~/Documents/Operandi-Rust-surface-identity-cache` | `surface-identity-cache` | 544a09e1 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 17:15 | — | b83ba8b | surface identity cache (committed today) |
| `~/Documents/Operandi-Rust-surface-tool-states-live` | `surface-tool-states-live` | 7c37fdd2 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 16:05 | — | b83ba8b | surface tool states live (committed today) |
| `~/Documents/Operandi-Rust-surface-tool-states-maintenance` | `surface-tool-states-maintenance` | 732c6d55 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 16:39 | — | b83ba8b | surface tool states maintenance (committed today) |
| `~/Documents/Operandi-Rust-thicken-shell-live` | `thicken-shell-live` | 5a53a1dd 2026-10-31 | 1d7e106 | +6/−0 | 1 | 0 | 10-28 15:30 | — | b83ba8b | thicken shell live (committed today) |
| `~/Documents/Operandi-Rust-tool-readiness` | `tool-readiness` | 9b87d15d 2026-10-31 | 1d7e106 | +5/−0 | 0 | 0 | 10-27 21:09 | — | same | Tool readiness + Bridge/Pen/command-alignment reviews (committed today) |

### (c) Still has uncommitted work — left out of Git by decision — 1

| Path | Branch | HEAD | Base | Ahead/behind main | Modified | Untracked | Newest file | Contained? | AGENTS.md | What it is |
|---|---|---|---|---|---|---|---|---|---|---|
| `~/Documents/Operandi-Rust-p01-identity` | `p01-identity` | 162b9204 2026-10-23 | 162b920 | +0/−1 | 0 | 2923 | 10-25 09:15 | base shared | same | P01 identity prototype; only untracked prototype evidence (2,900 files) |

### (d) Possibly active or unclear — not touched — 4

| Path | Branch | HEAD | Base | Ahead/behind main | Modified | Untracked | Newest file | Contained? | AGENTS.md | What it is |
|---|---|---|---|---|---|---|---|---|---|---|
| `~/Documents/Operandi-Rust` | `main` | 1d7e1069 2026-10-25 | 1d7e106 | +0/−0 | 4 | 134 | 10-31 07:44 | in another branch | 8aacb7d | main — main checkout; uncommitted docs/specs; not a task branch, so not committed here |
| `~/.codex/worktrees/m1-make-basic/Operandi-Rust` | `m1-make-basic` | 1d7e1069 2026-10-25 | 1d7e106 | +0/−0 | 71 | 691 | 10-30 14:45 | in another branch | 8e91dd5 | M1 Make Basic tool — several Claude sessions currently have this as their working folder |
| `~/Documents/Operandi-Rust-m1-integration-a34` | `m1-integration-a34` | 51274ff1 2026-10-27 | 1d7e106 | +5/−0 | 33 | 132 | 10-29 03:47 | in another branch | b83ba8b | M1 integration a34 — QUOTA_HANDOVER.json writer_stopped=true; ownership handed to a GPT-6.1 Terra session |
| `~/Documents/Operandi-Rust-m3-vertex-tools` | `m3-vertex-tools` | 1d7e1069 2026-10-25 | 1d7e106 | +0/−0 | 77 | 1006 | 10-31 07:32 | in another branch | 8aacb7d | Milestone 3 vertex tools + GUI QA handoff — edited minutes ago (07:32 today); someone is working here |


_Group (a) rows above were removed on 2026-10-31; m3-vertex-tools and m1-make-basic were committed after these tables were generated._
