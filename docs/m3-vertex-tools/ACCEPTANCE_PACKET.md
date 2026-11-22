# Milestone 3 Vertex panel — acceptance packet (2026-10-30)

Branch `codex/m3-vertex-tools` (worktree `Operandi-Rust-m3-vertex-tools`,
uncommitted). Base: a snapshot of the uncommitted `codex/m2-edge-tools` tree
taken 2026-10-30 23:11 CDT (M1 fixes + M2 Edge panel); later M2 edits are not
included and need merging at integration. Behavior follows the public Modo
17.1 user guide's Vertex Tools pages where they describe it (guide
precedence), within the draft minimums of
`specifications/modeling-toolset-milestones-2026-10-28/M3_VERTEX_BASE.md` (removed 2026-11-05).
Acceptance criteria were written first:
[ACCEPTANCE_CRITERIA.md](ACCEPTANCE_CRITERIA.md). Nothing here claims
reference parity beyond the listed basic workflows.

## How to check

- Automated gate: `cargo fmt --all --check`,
  `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --no-fail-fast --workspace`, `cargo build --locked --workspace`.
- Core oracles and demonstration scenes:
  `crates/operandi-core/tests/m3_vertex_tools.rs` — one geometry oracle per row,
  editor history/refusals, live-tool preview/Cancel/Drop, and
  `scenes/<row>.before.opscene` → `scenes/<row>.after.opscene` produced through
  the Editor with a one-step Undo/Redo check and a native save/reload
  (regenerate with `OPERANDI_WRITE_M3_DEMO=1`).
- App routing: `crates/operandi-app/src/vertex_panel_tests.rs` — panel rows,
  shortcuts and verbs give the same mesh; tool shortcuts and handle drags;
  Create and Weld clicks; every command, its help, and malformed-input
  refusal; Show Vertices changes no scene/history; Sync two-step.
- GUI: open a `before` scene, select the listed points (Vertex mode, tab
  "Vertex"), run the GUI route, compare with the `after` scene. Native GUI
  checks are not yet run (GUI pass checklist below).

## Validation run (2026-10-30, Claude Opus 5.5)

`cargo fmt --all --check` pass; `cargo clippy --locked --workspace
--all-targets -- -D warnings` pass; `cargo test --locked --no-fail-fast
--workspace` 1164 passed, 0 failed, 7 ignored (existing opt-in tests);
`cargo build --locked --workspace` pass. After two lint-only fixes the touched
sets were rerun: `m3_vertex_tools` 19/19, `vertex_panel_tests` 6/6. Native GUI
not run; no physical display, Windows or Linux validation claimed.

Native GUI pass retry (2026-10-31, Claude Opus 5.5): **not run** for every
checklist item (Vertex tab, Create, B/Shift+X/S, Merge Tool/Tear handles,
Weld, the four dialogs, Show Vertices). The QA app `Operandi M3 Vertex QA`
(`org.operandi.rust.m3vertex.qa`) launched with the MV-T02 scene, but computer
use again classified it as a browser (read-only tier) despite
`LSApplicationCategoryType = public.app-category.graphics-design`. Per
instructions no workaround was tried; the app was quit. Needs a controller
that grants this bundle full tier, or Jason running the checklist by hand.

## Row matrix

Shortcut: **G** = guide keyboard appendix / default conditional command;
— = no binding established (unassigned, remappable). Command: **C** =
reviewed command catalog; **P** = provisional Operandi spelling (the plan marks
every M3 route "Resolve before dispatch"; needs confirming through the
investigation process).

| Row | GUI route (Vertex tab) | Command | Shortcut | Icon | Scene |
|---|---|---|---|---|---|
| MV-T01 Create | Create row; click the work plane; X/Y/Z of last point; Drop | `tool.set vert.create on` **P** | — | prim.makeVertex | MV-T01-create |
| MV-T02 Vertex Bevel | Row or B; Inset handle/field; Drop | `tool.set vert.bevel on` **P** | B (Vertex mode) **G** | vert.bevel | MV-T02-vertex-bevel |
| MV-T03 Vertex Extrude | Row or Shift+X; Extrude/Width handles | `tool.set vert.extrude on` **P** | Shift+X (Vertex mode) **G** | vert.extrude | MV-T03-vertex-extrude |
| MV-T04 Merge Tool | Row; Distance handle/field | `tool.set vert.mergeTool on` **P** | — | **placeholder (artwork gap)** | MV-T04-merge-tool |
| MV-T05 Weld | Row arms click mode; click target (or source then target) | `tool.set vert.weld on`, `vert.weld target:<i>` **P** | — | **placeholder (artwork gap)** | MV-T05-weld |
| MV-T06 Tear | Row; X/Y/Z handles | `tool.set vert.tear on` **P** | — | **placeholder (artwork gap)** | MV-T06-tear |
| MV-T07 Slide | Row or S; Slide handle | `tool.set vert.slide on` **P** | S **G** | **placeholder (artwork gap)** | MV-T07-slide |
| MV-C01 New Vertex | New Vertex... (Create New Vertex dialog) | `vert.new <x> <y> <z>` **P** | — | text row | MV-C01-new-vertex |
| MV-C02 Make Polygon | Make Polygon row or P | `poly.make auto` **C** (Vertex mode) | P **G** | text row | MV-C02-make-polygon |
| MV-C03 Show Vertices | Show Vertices row (toggle) | `view.showVertices [on\|off\|?]` **P** | — | text row | — (display only) |
| MV-C04 Split | Split row | `vert.split` **P** | — | text row | MV-C04-split |
| MV-C05 Split Edge | Split Edge row | `vert.splitEdge` **P** | — | text row | MV-C05-split-edge |
| MV-C06 Split Polygon | Split Polygon row or Ctrl+L | `poly.split` **C** | Ctrl+L **G** | text row | MV-C06-split-polygon |
| MV-C07 Set Position | Set Position... dialog | `vert.set` **C** | — | text row | MV-C07-set-position |
| MV-C08 Align | Align... dialog (Axis, Uniform) | `vert.align [axis:] [uniform:]` **P** | — | text row | MV-C08-align |
| MV-C09 Merge | Merge... dialog (Range, Distance) | `vert.merge [range:auto\|fixed dist:]` **P** | — | text row | MV-C09-merge |
| MV-C10 Join | Join row | `vert.join` **P** | — | text row | MV-C10-join |
| MV-C11 Join Averaged | Join Averaged row | `vert.join average:true` **P** | — | text row | MV-C11-join-averaged |
| MV-C12 Collapse | Collapse row or Ctrl+Backspace | `vert.collapse` **P** | Ctrl+Backspace (Vertex) **G** | text row | MV-C12-collapse |
| MV-C13 Remove | Remove row or Shift+Backspace | `vert.remove` **P** | Shift+Backspace (Vertex) **G** | text row | MV-C13-remove |
| MV-C14 Remove Edge | Remove Edge row | `vert.removeEdge` **P** | — | text row | MV-C14-remove-edge |
| MV-C15 Reorder Index | Reorder Index... dialog (Swap/Top/Bottom) | `vert.reorder mode:` **P** | — | text row | MV-C15-reorder-index |
| MV-C16 Sync Index Order | Sync Index Order row, run twice (source, then target) | `vert.syncOrder` **P** | — | text row | MV-C16-sync-index-order |

Command convention (CC-001..012): tools use `tool.set <id> on|off|?` through
the shared catalog (`?` reports 1/0 with no change; targeted `off` refuses
when another tool is active); `tool.apply`/`tool.drop`/`tool.reset` reach the
same lifecycle as the panel; one-shots are verbs with whitespace-separated,
quote/brace-groupable `key:value` arguments, each bound once; unknown,
duplicate, missing or extra arguments refuse before mutation; `help` lists
every topic and `help <verb>` describes it with its current shortcut. Tool
options of the new tools are panel fields only: the guide documents no
`tool.attr` names for them, so per CC-005 none are invented.

## Provisional readings (no guide page, or the guide is silent)

1. **Weld** and **Tear** have no page in the supplied guide. Weld: selected
   points (or a first clicked source) weld onto the clicked target. Tear
   (set by Jason, 2026-10-31): select polygons and a point on them; the
   selected polygons' corner at that point tears off with its own point.
2. **Split Edge** (Vertex panel): a middle point on each edge between two
   selected points.
3. **Sync Index Order**: Operandi selections are per item, so the guide's
   "select two polygons on each mesh" is a two-step capture (run on the source,
   then on the target).
4. **Show Vertices**: draws every point in Edge/Polygon modes too (Vertex mode
   already does).
5. **Slide in Vertex mode**: a separate input action "Vertex Slide" (S in
   Vertex mode) beside the Edge Slide action; the guide describes one Slide
   tool whose handles adapt to the component type.

## Open items (need a decision or input outside implementation)

1. Command spellings marked **P** — confirm through the investigation process.
2. Artwork gaps: Merge Tool, Weld, Tear and Slide (Vertex) show the marked
   Vertex placeholder until original artwork is admitted.
3. GUI pass — not yet run (checklist below). Commit/merge await Jason.
4. Integration: this branch started from a snapshot of the in-progress M2
   tree; M2 edits made after 23:11 CDT need merging.

## GUI-pass checklist

- Vertex tab: fourth category tab; tool rows, Weld row, Commands rows render
  with icons/placeholders and shortcuts.
- Create: click the work plane in Front/Top views; points appear; X/Y/Z edit
  the last point; Drop keeps them selected; Esc removes them.
- B / Shift+X / S on a selected interior point: Inset, Extrude/Width, Slide
  handles appear on the point and drag sensibly; Drop is one undo step.
- Merge Tool Distance handle; Tear X/Y/Z handles show the torn corner.
- Weld: click target with points selected; with none, source then target.
- Dialogs: New Vertex, Align, Merge, Reorder Index open, Enter/OK commit,
  Esc/Cancel close without change.
- Show Vertices toggles point dots in Polygon/Edge mode.

## Deferred (later focused passes, per the plan)

Vertex Bevel Round Level / Use Material and boundary points; Vertex Extrude
on boundary points and Use Material; Merge Keep 1-Vertex Polygons, Test Morph
Map Positions, Merge Discontinuous UVs; Align planar constraints beyond the
axis mask; Make Polygon types other than Faces (Subdivs, Curves, Polyline,
Auto from neighbours); Weld multiple targets; corner-data interpolation for
new corners (refused while maps hold values, as in M2); falloff/symmetry/
action-center composition for the vertex tools.

## Jason's GUI test notes (2026-10-31) — to do, not started

- MV-T02 Vertex Bevel: works. Change requested: like Modo, a drag anywhere in
  the viewport should adjust the Inset, not only a drag on the handle.
  Dragging right increases, dragging left decreases.
- MV-T03 Vertex Extrude: works, but behaves differently from Modo. Jason may
  prefer Operandi's behaviour. Leave as is for now and revisit later.
- MV-T05 Weld: bug. The tool can't be dropped once it's active.
- MV-T06 Tear: wrong behaviour. Tear should need a polygon AND a vertex
  selected before it runs. Only the selected polygon's corner at that vertex
  is torn off, and that corner is what the handles move.
- MV-T07 Slide: works.
- MV-C02 Make Polygon: bug. It refuses with "would join an edge already used
  twice or reverse its neighbour" instead of making the polygon.
- MV-C03 Show Vertices: should be a checkbox, not a tool. When it's unchecked,
  unselected vertices aren't drawn on the model in the viewport.


## Codex GUI pass (2026-10-31)

Retried after Jason rebooted the computer. Tested the refreshed `development/m3-vertex-gui-20261031/Operandi M3 Vertex QA.app` through `org.operandi.rust.m3vertex.qa` in `~/Developer/Operandi-m3-vertex-tools`, branch `codex/m3-vertex-tools`. `pgrep` found no Operandi process before launch. Opened scenes through File > Open. Native input worked after the reboot; the earlier `noWindowsAvailable` blocker did not recur.

**Findings:** Create's coordinate entries did not move its points, and Escape did not cancel Create. Five command dialogs opened in Polygon mode despite the requested Vertex-only availability rule (details below). Their normal Vertex-mode results passed. No crashes, freezes or persistent viewport corruption were observed. Hover help was visible on every requested row, including Show Vertices over its checkbox.

No rebuild or bundle changes on this retry. The earlier requested app build passed. No source, tests, admitted scenes or other report sections edited; no Modo, guide or REA inspection; no commit, stash, merge or push. Existing uncommitted work was preserved. Retained 39 GUI Save As copies in `development/m3-gui-pass-20261031/`; original scene pairs were not overwritten. The test process (PID 3945) was terminated at the end; final `pgrep` found no Operandi process.

Results below apply to the observed checks, with incomplete variants stated explicitly. Saved results were compared read-only with the supplied after-scene mesh/item data. One-step Undo used Edit > Undo; the listed tool/command shortcuts were tested separately. Wrong-mode testing used Polygon mode as the representative other mode; Edge/Item-mode combinations were not run exhaustively.

| Row | Result | Observed notes |
|---|---|---|
| MV-T05 Weld | pass | Q, Space, Escape over the viewport and Drop Weld all ended it. Selected point 5 welded onto 6; no-selection source-then-target also worked. Saved result matched after; one Undo restored the grid. Blank target click said “click closer to a point”; Polygon mode was refused. Help present. |
| MV-T02 Vertex Bevel | pass | B activated it. Off-handle right drag raised Inset to 0.25981; left drag lowered it to 0.1299; handle drag also changed it. At Inset 0.2, saved result matched after; Drop was one Undo. Cancel removed a changed preview. Empty selection and Polygon mode refused; help present. |
| MV-T06 Tear | pass | Selected polygon 4, switched to Vertex mode and selected point 5. Handle moved only that polygon's corner. Offset X/Y 0.2 matched after; one Undo restored the grid; Cancel removed a later changed preview. No-polygon selection was refused with a clear message. Wrong mode/empty points refused; help present. |
| MV-C02 Make Polygon | pass | P on supplied four points matched after; one Undo removed the face. On a temporary hole made by removing grid polygon 4, both clockwise and counter-clockwise border selections filled it with neighboring winding. Interior surface points were refused with a clear message. Help present. Drag/lasso fill remained inconclusive; see unrun notes. Dedicated wrong-mode row check not run. |
| MV-C03 Show Vertices | pass | Checkbox was on by default. Off hid unselected dots while keeping the selected point; on restored dots. Help appeared over the checkbox. Hovered-only dot appearance remained inconclusive. No independent saved-selection/history comparison for this display toggle. |
| MV-T01 Create | fail | Front-view work-plane clicks added points; Drop selected all three and one Undo removed them. X/Y/Z field text changed but saved points retained clicked positions. A separated X=1/Enter retry confirmed no coordinate movement. Escape left the visible preview/tool active; Cancel removed it and saved zero points. Polygon mode refused; help present. Reproductions F1/F2. |
| MV-T04 Merge Tool | pass | Distance handle changed value to 0.086; field accepted 0.05. Drop result matched after; one Undo restored the points. Cancel closed the tool. Empty selection and Polygon mode refused; help present. |
| MV-C01 New Vertex | fail | Vertex-mode Enter at (1.5, 1.5, 1) matched after; one Undo restored mesh. Escape and Cancel closed without an edit; “nonsense” coordinate showed “Expected a number.” OK route exercised. Dialog also opened in Polygon mode, contrary to requested availability rule. Help present. F3. |
| MV-C04 Split | pass | Row duplicated point 5 per incident polygon; saved result matched after. Saved one-Undo item/mesh data exactly matched before. Empty selection and Polygon mode refused; help present. |
| MV-C05 Split Edge | pass | Row inserted the expected midpoint; saved result matched after. One Undo removed it. Empty selection and Polygon mode refused; help present. |
| MV-C06 Split Polygon | pass | Row and Ctrl+L made the same diagonal; saved row result matched after. One Undo removed it before shortcut check. Empty selection and Polygon mode refused; help present. |
| MV-C07 Set Position | fail | Z=0.5 dialog result matched after, and saved one-Undo items matched before. Empty selection refused. With a polygon selected, the Vertex-tab row opened Set Vertex Position in Polygon mode; Cancel closed it. Help present. Availability mismatch F4. |
| MV-C08 Align | fail | Axis Y with Uniform off, and All with Uniform on, both matched after; one Undo restored original uneven row. Fixture already has evenly spaced X positions, so Uniform coverage is limited. Empty selection refused on OK. Dialog opened in Polygon mode, then OK refused “Align works in Vertex mode.” Help present. F5. |
| MV-C09 Merge | fail | Automatic and Fixed 0.05 both matched after; one Undo restored original points. Distance disabled in Automatic, editable in Fixed. Empty selection refused on OK. Dialog opened in Polygon mode, then OK refused “Merge works in Vertex mode.” Help present. F5. |
| MV-C10 Join | pass | Row joined onto the last selected point; saved result matched after. One Undo restored original grid/selection. Empty selection and Polygon mode refused; help present. |
| MV-C11 Join Averaged | pass | Row joined at the midpoint; saved result matched after. One Undo restored original grid/selection. Empty selection and Polygon mode refused; help present. |
| MV-C12 Collapse | pass | Row and Ctrl+Backspace removed the point and healed surrounding faces without a hole. Saved row result matched after; one Undo restored grid before shortcut check. Empty selection and Polygon mode refused; help present. |
| MV-C13 Remove | pass | Row and Shift+Backspace removed the point and incident faces, leaving the expected hole. Saved row result matched after; one Undo restored grid before shortcut check. Empty selection and Polygon mode refused; help present. |
| MV-C14 Remove Edge | pass | Shared edge dissolved; saved result matched after. One Undo restored edge. Empty selection and Polygon mode refused; help present. |
| MV-C15 Reorder Index | fail | Top matched after exactly. Swap and Bottom changed numbering while preserving face geometry; each had one Undo entry. Empty selection refused on OK. Dialog opened in Polygon mode, then OK refused “Reorder Index works in Vertex mode.” Help present. Availability mismatch F5. |
| MV-C16 Sync Index Order | pass | Polygon-mode source capture then target run completed. Pair order 0 then 1 on both meshes matched after exactly. Saved one-Undo items matched before. Reversing both pairs gave different numbering, with unchanged shape; order matters on this symmetric fixture. Temporarily hid overlapping source to select target, then restored visibility. Vertex mode refused; help present. |
| MV-T03 Vertex Extrude | not run | Deliberately skipped per Jason; no behavior or fix proposed. |

**Failure reproductions**

- **F1 — Create coordinate fields:** Open `MV-T01-create.before.opscene`, Front view, Vertex tab > Create, click a point away from X=1. Replace Last point X with `1`, observe the field, then press Enter. Point remains in place. Drop and Save As. In `MV-T01-coordinate-retry.opscene`, saved point was `[-0.20377068324339734, -0.19593334927249742, 0]`, despite accepted X text `1`. Earlier X/Y/Z edits likewise left all three points at clicked positions (`MV-T01-result.opscene`).
- **F2 — Create Escape:** Open the Create before scene, activate Create, click in the viewport, wait until its one-point preview is visible, then press Escape with the viewport as the last clicked area. Create remains highlighted with one preview point. Cancel button removes it; `MV-T01-cancel.opscene` contains zero points. Drop and Undo work.
- **F3 — New Vertex mode availability:** Switch to Polygon mode while the Vertex tab is open; click New Vertex. Create New Vertex dialog opens instead of refusing. OK was exercised in this mode; this observation is an availability failure against the requested rule. Normal Vertex-mode geometry passed.
- **F4 — Set Position mode availability:** In the grid, select a polygon in Polygon mode, leave the Vertex tab open, click Set Position. Set Vertex Position dialog opens; Cancel closes it. No Polygon-mode edit was needed to demonstrate availability.
- **F5 — Align / Merge / Reorder Index availability:** In Polygon mode, click each of those Vertex-tab rows. Each opens its dialog. OK then refuses with “<command> works in Vertex mode,” leaving geometry unchanged. This is a dialog-availability mismatch, not a demonstrated wrong-mode mesh edit.

**Remaining checks / decisions**

- Drag/lasso hole-fill variant: not successfully run. Straight drags selected two points, so P correctly refused. The rectangle-style attempt did not establish a four-point drag selection; no tool failure claimed. Needs a manual gesture check.
- Hovered-only dot with Show Vertices off: inconclusive after retry; selected-dot retention and unselected-dot hiding were clear, but the controller cursor obscured the hovered corner. Needs a manual observation.
- Invalid tests used empty selections, plus Tear's missing polygon and Make Polygon's occupied surface; all malformed topology/selection variants were not exhaustively tested. After the refusal/cancel pass, saved mesh/items exactly matched `invalid-baseline.opscene`; Undo still named the earlier selection edit rather than a refused command.
- Decide whether the Vertex-only availability requirement should block dialog opening. New Vertex and Set Position expose shared workflows outside Vertex mode; Align/Merge/Reorder allow opening but guard on OK. Those five rows are marked fail against the stated rule, even though their tested Vertex-mode geometry passed.
- Decide whether Sync's order sensitivity should be documented or changed; order 0 then 1 matches the supplied oracle. No change requested to Vertex Extrude.
- No source fixes performed. Broader platform/display combinations, all save/reopen variants, and exhaustive dialog/shortcut combinations were not run. Two long controller batches timed out and were reconnected; this was not an app freeze. No repeat of Invalid app, browser/read-only tier, or noWindowsAvailable on this post-reboot retry.

## Fixes after the Codex GUI pass (2026-10-31, Claude Opus 5.5)

- F1 fixed: Create's Last point X/Y/Z fields now move the last clicked point
  (their `create_point0..2` keys weren't handled by field confirmation).
- F2 fixed: Escape over the viewport cancels Create and removes its preview
  points, as the panel says.
- F3–F5 resolved (Jason, 2026-10-31): New Vertex, Set Position, Align, Merge
  and Reorder Index are greyed out outside Vertex mode, with a "Switch to
  Vertex mode" tooltip; their shortcuts and commands refuse the same way.
  Uses a new provisional `chrome::DISABLED_TEXT` token.
- MV-C16 Sync Index Order: pick order matters by design. Each source/target
  pick pair says which point corresponds to which, so a different order
  describes a different correspondence and gives different numbering. No change.
- Regression tests: `create_xyz_fields_move_the_last_point_and_escape_cancels`,
  `vertex_only_dialog_rows_are_greyed_out_in_other_modes`.
- Still to check by hand: lasso four border points then P fills the hole;
  Show Vertices off still shows the hovered dot.
- Lasso fix (Jason's GUI test, 2026-10-31): right-drag lasso skipped points
  that belong to no polygon (fresh Create/New Vertex points), because the
  facing rule found no facing polygon. Such points are now always eligible.
  Test `right_drag_lasso_selects_loose_points`. Lasso + P on four loose points
  makes the polygon (confirmed by Jason).
- Point lasso visibility (Jason, 2026-10-31): right-drag lasso in Vertex mode
  now takes enclosed points that no surface hides, the same test clicking
  uses. It replaces M1.5's facing-only rule for points only; edge and polygon
  lassos keep facing-only. Test `right_drag_lasso_takes_points_nothing_hides`.

## Tool options GUI checks (vertex-tool-options-2026-11-03)

On a 3×3 grid, Vertex mode, interior point selected:

- [ ] Vertex Extrude (Shift+X): Extrude 0.2, Width 0.2, tick Use Material,
      type a name: the four new triangles get that material; untick and
      they don't.
- [ ] Vertex Bevel (B): Inset 0.2 with Use Material on tags only the cap.
      Round Level shows greyed with "not recorded".
- [ ] Vertex Slide (S): Percent field shows %, 100% merges the point into
      its neighbour (point count drops by one); Merge Vertices off leaves
      two points on top of each other. Distance mode drags in metres.
- [ ] Greyed options (Slide's Face Handle, Stop at Edges, Duplicate…,
      Merge Tool's Keep 1-Vertex Polygons, Create's Point Polygon) can't be
      changed and explain why on hover.
