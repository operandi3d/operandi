# Evaluation graph proposal (F2, master plan AR-03)

Status: proposal, read-only design. Nothing here is built. 2026-11-22.

## In plain English

Today, every time something changes (a key, a drag, a frame step), Operandi
works out what to show by asking each part "is your input the same object
as last time?". Each part keeps its own memory: MeshOps compares recipes,
the viewport compares mesh pointers, the Cycles preview does the same.
That works, and it is fast for the cases we have. It does not scale to
rigs, where one controller moves a hundred bones that move ten meshes,
and where we must answer "what depends on this?" without walking
everything.

An evaluation graph is a written-down list of who reads whom: "the cube's
position comes from channel X; the Bend deformer reads the cube's
position; the cube's shown mesh comes from the Bend". When something
changes we mark it **dirty** and walk the arrows forward, marking what
depends on it. When someone asks for a result, only dirty things are
recomputed. Nothing else is touched.

The graph is **not document data**. It is rebuilt from the scene, lives
next to the document (one per open document), is never saved and is never
part of undo. Undo and files keep holding the scene; the graph is a cache
that can always be thrown away and rebuilt.

## What happens today (the study)

| Step | Where | How it decides what to redo |
|---|---|---|
| Time change | `Editor::set_current_time` (anim_edit.rs) | `animation::pose` writes every channel's value into item fields, for the scene and the live tool's base. Bumps `Editor::revision`. |
| Key / curve edit | anim_edit.rs, anim_graph.rs | Each command ends with `animation::repose(&mut e.scene)`: every channel re-posed, not only the edited one. |
| Other times | `animation::evaluate_at` | Clones the scene and poses the copy (USD export, renders of a range). |
| Evaluated scene | `evaluate::evaluate` (evaluate.rs) | Purpose Display / Final / Pick. MeshOps substitution, live patches, replicators. Takes `time` but ignores it. Each layer clones the scene when it changes anything and scans every item. |
| MeshOps | `meshops::Runtime` (meshops.rs) | Per-item `Chain` of cached op results; `valid_prefix` compares base mesh pointer, op values and `OpInputs` (links, time). Background jobs for Display. Pull-and-compare, not dirty flags. |
| Viewport | gpu_scene.rs | Per-item geometry keyed on `Shared::same` mesh pointers plus a generation; patches changed ranges (`Patch`). |
| Cycles preview | preview.rs | Compares item meshes by pointer (`Shared::same`) and fields to decide what to resend. |

Where whole-scene work happens on a drag or frame step:

1. `pose` / `repose` loop over all channels on every frame step and after
   every key edit.
2. `evaluate` (Display) runs every frame: `with_patches` and
   `replicator::expand` scan all items; MeshOps `substituted` clones the
   scene when any item has a stack.
3. Each reader (viewport, preview, framing) compares item by item.

None of this is wrong today. With rigs and deformers it becomes the
bottleneck: a frame step would re-pose and re-check everything.

## Prerequisites (not part of this plan)

From the approved audit remediation (`hardening` branch, batch 4):

- **Document state** (`operandi-core/src/document_state.rs`, Astra
  P2-001): every `Editor` field is sorted into **Document** (this scene's
  data, history, open tool, ids and caches of its items) or **Shared**
  (app-wide settings and tool context), with an exhaustive list that
  doesn't compile until a new field is sorted.
  `Editor::swap_document_state` swaps the Document ones when scene tabs
  switch; the MeshOps cache already travels with its tab this way.
- **Undo reports success truthfully** (P2-001 stage 2).

This proposal does not change the Editor field layout beyond adding its
own fields, `swap_document_state` beyond listing them, or undo.

## Design

### Nodes and edges

A node is one value someone can ask for:

- `Channel(item, name)`: a curve's value at the current time.
- `ItemField(item, field)`: a transform or property as posed.
- `WorldTransform(item)`: parent chain (later, rigs).
- `ShownMesh(item)`: Base Mesh → MeshOps → (later) morphs, skinning,
  deformers, sim cache, in the AR-05 order.
- `Time`: the document's current time; one node everyone time-dependent
  reads.

Edges come from what each node reads: a channel writes one field; a
MeshOps link reads another item's shown mesh; a time-using op reads
`Time`. MeshOps already lists its links (`OpDef::links`) and its time use
(`OpInputs::time`), so its edges are already known.

### Dirty flags

- Each node has a `dirty` bit and a cached value.
- An edit marks its source nodes dirty and walks forward (marking stops at
  nodes already dirty, so a walk is linear in what changed).
- A read pulls: if the node is dirty, recompute from its inputs, clear the
  bit. Pull keeps "who is asking" (Display in the background, Final now)
  exactly as `Purpose` does today.
- The graph is rebuilt (structure only) when items, channels, parents or
  MeshOps links are added or removed. A `structure` counter on the scene's
  edit path tells it when; field edits only dirty nodes.

### Where it lives

The engine's per-document state is new `Editor` fields, each listed under
**Document** in `document_state.rs` (and swapped in
`swap_document_state`), as that file's header asks:

```
Editor
  scene: Scene              document data: undo + files (unchanged)
  meshops: Runtime          Document already (MeshOps cache)
  eval: EvalGraph           NEW, Document: graph, dirty bits, node caches
  evaluated: EvalCache      NEW, Document: the last evaluated scene per Purpose
```

One graph per open scene, nothing global per app. None of it is undo
data or saved.

### Out of undo (AR-04, AR-15)

- The graph, its dirty bits and its caches are never in undo checkpoints,
  `PartialEq` or files (as MeshOps already is, SAFE-03).
- Undo restores the scene; the graph then marks everything the restored
  items touch dirty (or rebuilds when the structure changed). Undo never
  records "dirty" state.
- Time changes stay out of undo (TL rule), and only dirty `Time` and what
  reads it.

### Rest, curve and evaluated values stay apart

Today rest lives in `AnimChannel::rest`, the curve in `AnimChannel::curve`,
and posing writes the curve's value into the item field (the field is the
evaluated value; a pending edit is "field differs from curve").

The graph keeps that, and makes the third one explicit:

- **Rest**: document data, edited only by explicit commands.
- **Curve**: document data.
- **Evaluated**: a graph node's cached value. Step 6 moves posing from
  "write into the item field" to "the node holds it; readers ask the
  graph". Until then the graph only tracks which fields posing touched, so
  behaviour does not change.

Rigs (phase 4) add more layers (constraints, IK) that are nodes reading
evaluated values, never writing rest.

### Schematic later (AR-21 in the master plan; not the Array spec's AR-21)

The graph exposes a read-only description: `nodes()` (id, kind, label,
item) and `edges()` (from, to). A Schematic view draws that list. It needs
nothing else from the engine, and it can show dirty nodes as a debug
overlay. No Schematic is built in this plan.

## Build order

Each step is one small branch-sized change with its own tests and a
perf check: `tool_speed_guard_tests.rs` still passing, the 16 ms drag
frame on the 16k grid (`tool_frames_subdivided` probe), and the GE-X7 probe
(`graph_editor_speed_probe`) for animation steps.

| # | Step | Tests | Can start before batch 4? |
|---|---|---|---|
| 1 | `eval_graph.rs` in operandi-core: node ids, edge list, dirty bits, forward marking, pull with a compute callback. Pure data structure, no Scene. | unit: marking stops at dirty, cycles refused, pull computes once | **Yes** (pure core) |
| 2 | `EvalGraph::build(&Scene)`: nodes and edges from items, channels, MeshOps links and time use. | build on fixtures; edges match `OpDef::links`; structure counter | **Yes** |
| 3 | Channel nodes: `pose` and `repose` through the graph, so a key edit re-poses only the edited channel and a frame step only time-dependent channels. Same results as today. | every animation test unchanged; new test counts channel evaluations | **Yes**, as a pure function taking `&mut Scene` + `&mut EvalGraph` owned by the caller (a test or a temporary local), no Editor field |
| 4 | `evaluate()` reads the graph: Display/Final skip items whose shown mesh is clean (no full scans). MeshOps chain lookup becomes a node pull; its recipe check stays as a safety net. | doors.rs, meshops tests; speed guard; 16 ms probe | Partly: the core function yes; wiring to `Editor::evaluated` waits |
| 5 | `Editor.eval` / `Editor.evaluated` fields, listed under Document in `document_state.rs`; `Editor::evaluated` and edit paths mark dirty. | tab switch keeps each scene's graph; undo/redo marks dirty; contract + monkey tests | **No**: needs batch 4 (document_state.rs) on main |
| 6 | Evaluated values held by the graph, not written into item fields; pending = field vs node. Readers (viewport, Cycles preview, export, bridge) ask the graph. | command contract, save/reopen, USD animation export unchanged | **No**: needs step 5 and stage 2 |
| 7 | Viewport and Cycles preview take a "changed items" list from the graph instead of comparing every item. | gpu_scene patch tests; preview sync tests; speed probes | **No** |
| 8 | `nodes()` / `edges()` description and an MCP query (`operandi.eval.graph`), for tests now and the Schematic later. | bridge test | Description yes; bridge hookup after 5 |

After step 8, phase 4 rigging adds node kinds (parent, constraint, IK,
skin) without touching the engine.

## Risks

- **Two sources of truth while migrating.** Steps 3-4 keep today's
  compare-by-pointer checks as a safety net; remove them only in step 7
  once tests prove the dirty walk is complete.
- **Missed dirty marks** (an edit path that forgets to mark) show as stale
  views. Mitigation: a debug check that recomputes clean nodes and
  compares, run in the monkey tests.
- **Structure changes** are easy to miss (paste, import, kit items).
  Rebuild on any structure counter bump; it is cheap at our scene sizes.
- **Live tools** edit `live.base` as well as the scene; both must mark the
  same graph or the drag preview and the commit disagree.
- **Background Display evaluation**: dirty marks must cancel or supersede
  running jobs (MeshOps already cancels with `Job::cancel`).
- **Batch 4 timing**: steps 5+ wait for document_state.rs on main; doing
  1-4 first means a short period where the graph is used by functions but
  not owned by the Editor.

## Codex candidates

Mechanical enough for `codex exec` with a tight brief (file, signatures,
tests to write, "no other files"):

- Step 1: the graph data structure and its unit tests, from a written
  API.
- Step 2's edge extraction for MeshOps links, given `OpDef::links`.
- Step 8's description struct and MCP query plumbing (after 5).
- Converting each `animation::repose(&mut e.scene)` call site to the
  step 3 entry point (a dozen sites in anim_edit.rs and anim_graph.rs).

Keep for a Claude builder: steps 4-7 (where dirty marks enter the Editor,
undo and the viewport), and the stale-view debug check.

## Open questions

- Master plan IDs AR-03, AR-04, AR-15 and AR-21 are not in this repo; this
  doc uses them as the coordinator described them. AR-21 also names an
  Array spec rule here, so the Schematic one should get a distinct label.
- Should Final (render/export) share Display's caches when both are
  current, or always pull separately? Today they share MeshOps results.

## As built (2026-11-23)

- Step 1 `eval_graph.rs`, step 2 `eval_build.rs` (a9357a98); step 5
  `eval_state.rs` as the `Editor::eval` field under Document in
  document_state.rs, and step 8's `operandi.eval.graph` query with a
  bridge test (edb54136). The graph holds structure only; posing and
  evaluation are unchanged.
- Measured first (`eval_probe`, release, 2,000 animated items, 8,000
  channels): a frame step poses everything in 0.47 ms, the viewport's
  evaluated scene costs 0.07 ms a frame, a key edit 4 ms (mostly the
  scene copy and undo record, not posing).
- So steps 3, 4, 6 and 7 are not built yet: with today's work per frame
  they would save well under a millisecond and add a second source of
  truth (a cached channel value goes stale when any path changes a curve
  without telling the graph, such as a scene swapped in whole). They pay
  off once rigs (phase 4) give the graph expensive nodes (constraints,
  IK, skinning, deformers); build them with the first such node, each
  with the stale-view check from "Risks".
