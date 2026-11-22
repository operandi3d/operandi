# Procedural evaluation safety — v1 (2026-11-11)

Shared rule set for anything that re-computes geometry from a recipe:
MeshOps stacks, deformers in the stack, and MeshFusion-style live booleans.
The MeshFusion spec uses the same rules; if the two ever differ, ask Jason.
These rules win over the behaviour specs.

## Why this exists (evidence)

Jason's worry: procedural stacks and live booleans are what made the
reference app crashy. The public release notes back that up. Across
versions 10.x to 17.1 they list about **109 crash/hang fixes for the
procedural stack** and about **47 for MeshFusion**. Rough grouping of the
stack ones (an entry can sit in two groups):

| Kind of failure | ~count |
|---|---|
| Selection and element tracking (selecting, hovering, edge tracking on an evaluated mesh) | 26 |
| Cross-item references (Boolean, Merge Meshes, replicators, particles, clones) | 23 |
| Graph edits and self-reference (linking, dragging into stacks, an op as its own source, aliases) | 19 |
| Hidden / display / UI state (hidden meshes, ghost mode, deformers toggled off, play mode) | 19 |
| Freeze / duplicate / convert | 15 |
| Direct edits on a procedural mesh or its Base Mesh (Pen, drawing, moving) | 12 |
| Load / save / versioning (scenes crash on open, ops reload with different defaults) | 9 |
| Degenerate values (size 0, point count 2, one-point polygons, irregular input) | 8 |
| Other single-op bugs | ~24 |

The lesson: **most crashes were not bad geometry maths. They were state
and lifecycle**: something holding on to an evaluated mesh that had since
changed, a direct tool poking a mesh owned by a stack, a link forming a
loop, a scene loading in an unexpected shape. The rules below target those.

## Rules

### Isolation
- **SAFE-01 Snapshot in, mesh out.** An evaluation gets an immutable copy of
  the recipe and its inputs (Operandi's `Shared<Mesh>` makes this a pointer
  copy) and returns a new mesh. It never reads or writes the live scene,
  editor, UI or GPU state.
- **SAFE-02 Off the UI thread, cancellable, stale results dropped.** Every
  evaluation carries the recipe's revision number. Starting a newer one
  cancels the older one; a result whose revision is out of date is thrown
  away. The UI never waits on an evaluation.
- **SAFE-03 The result is a cache, not the document.** Evaluated meshes are
  never part of undo checkpoints, never compared for "did the scene
  change", never saved as truth, never edited.
- **SAFE-04 One op fails, the stack survives.** Each op runs inside a guard
  that catches a panic, a time limit and size limits (see SAFE-10). The
  output is validated (`Mesh::validate`) before use. On failure the op is
  **passed through** (acts as if disabled), its row turns red with a short
  reason, the ops above still run, and the scene data is untouched.
  During a drag, the viewport keeps showing the **last good result** until
  a new good one arrives.

### Direct tools vs stacks
- **SAFE-05 No silent edits under a stack.** A direct modelling tool never
  edits an evaluated result. On a mesh with ops it either edits the Base
  Mesh explicitly (Edit Base Mesh mode, ghost shown, MO-27) or refuses with
  a one-line message offering **Edit Base Mesh** or **Freeze**.
- **SAFE-06 No borrowed element numbers.** Nothing outside the evaluator
  may keep vertex/edge/polygon numbers into an evaluated mesh across a
  re-evaluation: not selection, hover, picking, snapping, tool handles or
  edge tracking. They re-resolve against the current result every time,
  or are tagged with the result's revision and ignored when it changed.
  Component selection captured into an op (Select by Index) is copied into
  the op's own values at capture time.

### Links between items
- **SAFE-07 No loops, ever.** Every link (op input, Merge Meshes source,
  Boolean operand, falloff) is checked for a cycle **before** it is made;
  a loop is refused with the dependency-loop warning. Load re-checks, and
  breaks any loop found by disabling the offending link with a warning.
- **SAFE-08 Links are by item id, and may dangle.** Deleting, hiding or
  un-parenting a referenced item never crashes: the op shows "missing
  input" and passes through. Undo restores the link.
- **SAFE-09 Visibility doesn't change evaluation.** Hidden meshes evaluate
  the same way (lazily, when something needs them). Ghost mode, Enable
  Deformers, Pause and Defer change **what is drawn or when**, never what a
  stack computes.

### Inputs and limits
- **SAFE-10 Values are checked twice.** Each op's values are clamped by its
  UI (minimums, finite numbers) **and** re-checked by the op when it runs
  (a file or script can bypass the UI). Below-minimum or non-finite = op
  failure (SAFE-04), not a crash. Per-op output limits (polygon count,
  memory) and a per-evaluation time limit; limits are preferences.
- **SAFE-11 Deterministic.** Same recipe + same inputs = same result, byte
  for byte, on every platform build we ship. Random seeds are op values
  and never depend on time, frame or thread count.

### Files and undo
- **SAFE-12 Every value saved, versioned per op.** Save each op's type,
  its own version number and every value. Loading an older op version runs
  an explicit upgrade; an unknown op type (newer file, missing plugin) loads
  as a placeholder that keeps its data, passes through, and saves back
  unchanged.
- **SAFE-13 Undo stores the recipe only.** Adding, removing, reordering,
  enabling, editing values, freezing: each is one undo step of recipe +
  Base Mesh changes (one step per drag, using the existing gesture fold).
  Evaluation never records undo. Undo/redo during an evaluation just bumps
  the revision (SAFE-02).
- **SAFE-14 Freeze is a recipe edit.** Freeze writes the current evaluated
  result as the new Base Mesh and removes the frozen ops, in one step,
  only from a **valid, current** result (never a failed or stale one).

- **SAFE-17 Opening a scene never waits on evaluation.** Load reads the
  recipe only. Procedural items show a saved speed cache (if valid for the
  recipe) or a placeholder, then evaluate in the background under SAFE-02.
  A stack that fails on load fails per SAFE-04; the scene still opens.

### Memory
- **SAFE-15 Bounded cache.** Per-op results are cached up to a memory
  budget (preference). Least-recently-used results go first; deleting an
  item frees its cache.

### Proof
- **SAFE-16 Stress harness before shipping.** Like the booleans harness:
  random stacks of real ops on random meshes, with random reorder, enable,
  value edits (including zero/huge/NaN via script), delete-referenced-item,
  undo/redo storms, save/load round-trips and concurrent edits during
  evaluation. Pass = no panic escapes, no hang past the time limit, saved
  result reopens identical. Runs in CI on every op added.
