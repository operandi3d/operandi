# What Operandi needs for a procedural stack (checked against main 6c41cff2)

This is about our own code, not the reference app.

## What we already have that helps

- **Undo is whole-scene before/after checkpoints** (`Editor::record`,
  `Checkpoint { before: Scene, after: Scene }`) with copy-on-write mesh data
  (`Shared<Mesh>`, an `Arc`). A recipe edit costs a pointer copy per mesh,
  so the undo model needs **no redesign**: recipe changes are scene changes
  like any other, and gesture folding already makes one drag = one step.
- **Tool kernels are already mostly pure functions** of mesh + selection +
  values, e.g. `inset_geometry(&Mesh, &[u32], f64) -> Result<..>`,
  `bevel_geometry…`, `edge_bevel_geometry…`, `globe_geometry(..) -> Mesh`.
  Wrapping one as an op is adapter work, not a rewrite. The op must call
  the kernel, **not** the live-tool session (live Extrude stages, mid-tool
  undo, numeric sessions carry interactive state an op must not have).
- **`Mesh::validate`** exists and is memoised per allocation: cheap output
  check for SAFE-04.
- **Panics unwind** (no `panic = "abort"` in the workspace profile), so
  `catch_unwind` around each op works (SAFE-04). Keep it that way; add a
  note in `Cargo.toml`.
- The app already runs save, autosave and import on worker threads; the
  evaluator can follow the same pattern.
- `correspondence.rs` already returns construction roles for extrusion
  (which new faces came from what). That is the seed of the output groups
  **Select by Previous Operation** needs (MO-53).
- The command registry exists; every op type and every stack command gets
  an entry through the registry thread (UIC-165, no invented names).

## What we must add

1. **Data:** `Item` keeps `mesh` as the **Base Mesh** (so saves, undo and
   every existing tool keep their meaning) and gains an optional op stack
   (ordered ops: type, version, values, enable, inputs by item id).
2. **Evaluated cache outside `Scene`:** keyed by item id + recipe revision,
   held by the editor/app, not in `Scene`, so it is never in checkpoints,
   `PartialEq` or saves (SAFE-03).
3. **Evaluator:** worker thread, revision numbers, cancel, per-op guard,
   last-good fallback, memory budget (SAFE-01/02/04/15).
4. **The big audit: who reads `item.mesh`.** There are ~1,750 `.mesh` reads
   in ~250 source files. Each one must say which it wants: the **Base
   Mesh** (direct tools in Edit Base Mesh mode) or the **evaluated result**
   (viewport drawing, picking, snapping, measuring, Cycles export, LXO/OBJ
   export, statistics, item bounds, Fit/Center). Recommended: two explicit
   accessors and a lint that bans bare `.mesh` outside them. This audit is
   the largest single piece of Stage 0, and the place most of the
   reference app's crash classes live (SAFE-05/06).
5. **Selection / identity:** `identity.rs` invalidates handles on mesh
   change; good. Component selection on a procedural mesh must be tagged
   with the result revision (SAFE-06).
6. **File format:** new native version for stacks (MO-90, SAFE-12).
7. **UI:** Mesh Ops tab (Item List + Mesh Operations list + Properties),
   ghost drawing, red failed-op rows. Follows the UI bible; the list is an
   egui tree like the Item List and Shader Tree.
8. **Deformer items:** don't exist yet (Deform tab is destructive presets);
   later stage.
