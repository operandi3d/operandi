# Milestone 2 Edge panel — acceptance packet (2026-10-30)

Branch `codex/m2-edge-tools` (worktree `Operandi-Rust-m2-edge-tools`, uncommitted).
Behavior follows the public Modo 17.1 user guide (approved for implementers;
"always follow the guide"), within the M2 basic minimums in
`specifications/modeling-toolset-milestones-2026-10-28/M2_EDGE_BASE.md` (removed 2026-11-05).
Nothing here claims reference parity beyond the listed basic workflows.

## How to check

- Automated gate: `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `cargo test --locked --no-fail-fast --workspace`, `cargo build --locked --workspace`.
- Demonstration scenes: `scenes/<row>.before.opscene` → `scenes/<row>.after.opscene`,
  produced through the Editor (same operation as the panel and command line),
  each with a one-step Undo/Redo check, saved and reloaded natively.
  Kept current by `crates/operandi-core/tests/m2_demo_scenes.rs`
  (regenerate with `OPERANDI_WRITE_M2_DEMO=1`).
- GUI: open a `before` scene, select as listed (Edge mode), run the GUI route,
  compare with the `after` scene. Native GUI checks are delegated to Codex
  (`docs/archive/GUI_QA_HANDOFF_M1FIX_M2.md` (removed 2026-11-05), items 1–6b and 14–25); not yet run.

## Row matrix

Shortcut column: **G** = confirmed in the guide's keyboard-shortcut appendix
or its default conditional commands; — = no binding established (unassigned,
remappable). Command column: **G** = spelling confirmed in the guide;
**C** = reviewed command catalog; **P** = provisional Operandi spelling
(needs confirmation through the investigation process).

| Row | GUI route (Edge tab) | Command | Shortcut | Icon | Scene | Focused tests |
|---|---|---|---|---|---|---|
| ME-T01 Edge Bevel | Edge Bevel row; drag Inset handle or fields Inset / Round Level / Depth; Drop | `tool.set edge.bevel on` **P** | — | edge.bevel | ME-T01-edge-bevel | core `edge_bevel.rs`; app `edge_bevel_chamfer_relax_rows_shortcut_handle_and_bridge` |
| ME-T02 Edge Extrude | Edge Extrude row or Shift+X; Extrude / Width | `tool.set edge.extrude on` **P** | Shift+X **G** | edge.extrude | ME-T02-edge-extrude | core `edge_extend_extrude.rs`; app `shift_x_is_edge_extrude_in_edge_mode_and_extrude_elsewhere` |
| ME-T03 Extend | Extend row or Z; X/Y/Z handles | `tool.set edge.extend on` **P** | Z **G** | edge.extend | ME-T03-extend | core `edge_extend_extrude.rs`; app `edge_extend_handle_drag_and_edge_extrude_fields` |
| ME-T04 Bridge | Bridge row (Edge mode, two border chains); Apply | `tool.set edge.bridge on`, `tool.apply` **C** | — | bridge | ME-T04-bridge | core `edge_commands.rs` bridge test; app test above |
| ME-T05 Slide | Slide row or S; drag handle | `tool.set edge.slide on` **P** | S **G** | edge.slide | ME-T05-slide | app `slide_shortcut_and_handle_drag` |
| ME-T06 Edge Relax | Edge Relax row; Offset handle/field | `tool.set edge.relax on` **P** | — | edge.relax | ME-T06-edge-relax | core `edge_bevel.rs` relax test |
| ME-T07 Edge Chamfer | Edge Chamfer row or B; Offset / Segments / Depth | `tool.set edge.chamfer on` **G** | B **G** (Edge mode) | edge.bevel (lookup edge.chamfer) | ME-T07-edge-chamfer | core `edge_bevel.rs`; app test above |
| ME-T08 Add Loop | Add Loop row, click an edge (Ctrl: middle) | `edge.addLoop [position:%]` **P** | — | edge.addLoop | ME-T08-add-loop | core `edge_insert.rs`; app `edge_tab_add_loop_click_and_commands` |
| ME-T09 Add Point | Add Point row, click an edge | `edge.addPoint [position:%]` **P** | — | edge.addPoint | ME-T09-add-point | 〃 |
| ME-T10 Edge Crease | Edge Crease row; Crease field/handle | `tool.set edge.crease on` **P** | — | **placeholder (artwork gap)** | ME-T10-edge-crease | app `edge_crease_tool_starts_at_current_weight_and_records_once` |
| ME-C01 Grow Quads | Grow Quads row | `edge.growQuads` **P** | Shift+Q **G** | text row | ME-C01-grow-quads | core `edge_commands.rs` |
| ME-C02 Fill Quads | Fill Quads row | `edge.fillQuads` **P** | — | text row | ME-C02-fill-quads | 〃 |
| ME-C03 Set Position | Set Position... row (dialog) | `vert.set` **C** | — | text row | ME-C03-set-position | 〃 (editor test) |
| ME-C04 Split | Split row | `edge.split` **P** | — | text row | ME-C04-split | 〃 |
| ME-C05 Split Normals | Split Normals row | `edge.splitNormals` **P** | — | text row | ME-C05-split-normals | 〃 |
| ME-C06 Spin | Spin row | `edge.spin` **P** | V **G** (Edge mode) | text row | ME-C06-spin | 〃 |
| ME-C07 Join | Join row (two edges, first moves) | `edge.join` **P** | — | text row | ME-C07-join | 〃 |
| ME-C08 Join Averaged | Join Averaged row | `edge.joinAverage` **P** | — | text row | ME-C08-join-averaged | 〃 |
| ME-C09 Collapse | Collapse row | `edge.collapse` **P** | Ctrl+Backspace **G** (Edge mode) | text row | ME-C09-collapse | 〃 |
| ME-C10 Remove | Remove row | `edge.remove` **P** | Shift+Backspace **G** (Edge mode) | text row | ME-C10-remove | 〃 |
| ME-C11 Edges To Curves | Edges To Curves row | `edge.toCurves` **P** | — | text row | ME-C11-edges-to-curves | 〃 |
| ME-C12 Unbevel | Unbevel row (two loops) | `edge.unbevel` **P** | — | text row | ME-C12-unbevel | 〃 |
| ME-C13 Edge Subdivide | Edge Subdivide row | `edge.subdivide` **G** | D **G** (Edge mode) | text row | ME-C13-edge-subdivide | 〃; app `edge_command_rows_shortcuts_and_verbs_share_one_operation` |

All rows: `help <command>` documents the route and the current shortcut;
`every_m2_edge_row_is_callable_from_the_command_line` checks each route.
Option values of the new tools are panel fields only: the guide documents no
`tool.attr` names for them, so per CC-005 they are listed as UI-only rather
than given invented attribute names.

## Open items (need a decision or input outside implementation)

1. **Command spellings marked P** — confirm through the investigation process.
   Confirmed from the guide: `tool.set edge.chamfer on`, `edge.subdivide`.
2. **Edge Crease artwork** — the admitted dictionary has no glyph; a draft
   original candidate is in `edge-crease-candidate.svg` for review. Until it
   is admitted, the row shows the marked placeholder.
3. **B in Edge mode** — the M2 plan listed B for Edge Bevel (to verify); the
   guide's default conditional command maps the bevel key to Edge Chamfer in
   Edge mode, so B runs Edge Chamfer (guide precedence). Edge Bevel is
   unbound and remappable.
4. **GUI pass** — delegated to Codex; not yet run. Commit/merge await Jason.

## Deferred (later focused passes, per the plan)

Bevel/Chamfer junctions, shared-point chains, profiles, Width/Absolute modes,
material override; curved and unequal edge bridges; Relax propagation; Fill
Quads grid mode; Split gap/caps; Split Normals intermediate levels; Edge
Subdivide Percent/Segments options; Unbevel Flat/Square; interpolating
UVs/corner maps for new points (refused while a map holds values); tool.attr
for the new tools.
