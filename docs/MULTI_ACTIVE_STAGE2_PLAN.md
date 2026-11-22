# Multiple active meshes, stage 2: transforms across meshes (plan)

Target (Jason, 2026-11-08): with several meshes selected, Move, Rotate,
Scale and Transform move every active mesh's selected components together,
with one handle at the centre of the combined selection, as one undo step.
In Item mode they move every selected item together.

Built on branch multi-active-transforms (2026-11-08); tests in
`crates/operandi-core/tests/multi_active_transforms.rs`. The section below
describes main before stage 2.

## What main does today

(Before stage 2.)

- **Item mode.** `Editor::preview` (editor.rs, the `Tool::Transform | Move |
  Rotate | Scale` arm) changes only `scene.active_mut()`, the primary item.
  Rotate and Scale add to that item's own channels, about its own origin.
  The handle comes from `ActionContext::resolve` (action_center.rs), which in
  Item mode returns the primary's `transform.translation`. Any other action
  centre is refused (VAC-Q04).
- **Component modes.** The same arm moves `transform_vertices(&live.base)`,
  which reads the primary's selection only. The frame comes from
  `Editor::extrude_frame`, which resolves against the primary in world space.
- **The `selection.also` loops** at editor.rs ~2046 and ~2168 are not
  transforms. They are Camera and Light property edits (CP-X3, LP-51) that
  follow every selected camera or light, all or nothing. They are a useful
  pattern for "apply to each selected item, all or nothing".

## Component modes across meshes

1. **Targets.** Add `Scene::transform_targets() -> Vec<(u64, BTreeSet<u32>)>`
   in core. It holds one entry per active mesh with something of the current
   mode selected: the primary from its fields, the others from
   `Selection::others`. It reuses `transform_vertices` through
   `as_sole_mesh`, so empty-means-all and the Polygon/Edge expansion stay in
   one place. Rule: an empty selection means "all" only when no active mesh
   has a selection; otherwise, meshes with nothing selected don't move. This
   replaces the MAM-01 guard for these four tools.
2. **Combined centre.** Generalise `extrude_frame` / `resolve` to fold the
   world-space positions of every target (each mesh's points through its own
   `world_matrix`). Selection = mean of all moved points. Automatic = bounds
   centre of all of them. Border = border points per mesh, pooled.
   Origin, Element and Screen don't change. The basis follows the same
   rules: Selection axis votes pooled across meshes; Local stays per
   connected piece, per mesh (`local_piece_centres` per target). Computed
   once per drag (a `BaseMemo` entry), not per frame.
3. **Apply.** In the preview arm, loop over the targets. For each mesh: its
   world matrix from `live.base`, its inverse, its vertices, then the
   existing per-point code (falloff weight, symmetry and local pieces).
   Everything stays in world space, so the shared frame works unchanged.
   One `Live` session covers every mesh. `live.base` is already the whole
   scene, so Cancel restores all of them and Drop records one checkpoint
   (the scene snapshot covers every mesh). No new undo machinery.
4. **Falloffs.** They're already world-space (`falloff_weights` evaluates
   `world.transform_point3`). Apply the same field to each mesh's points.
5. **Symmetry.** It's per mesh in its own coordinates today (M·T·M, plane
   tolerance from that mesh). Keep it per mesh. The "entered side" comes
   from the primary's first pick. Mirror highlights stay primary-only until
   asked (MAM-Q06).
6. **Speed.** Targets and frames are computed once per drag. Per-frame work
   touches only the targets' moved points. GPU patches already work per
   item (`ResidentCache::item_placed` keys on each mesh). Add a
   two-mesh case to `tool_speed_guard_tests.rs`.
7. **Handles and drawing.** The handle draws at the combined frame. The
   viewport already reads `live.action_frame`, so nothing changes there.

## Item mode across items

- Targets are `selected_items()`, minus locked items (CP-A02; refuse the
  whole drag, as today) and children whose ancestor is also selected (so a
  parent and child don't move twice).
- Move adds the delta to each item's world translation, mapped through its
  parent. Rotate and Scale with the default centre turn and scale each item
  about its own origin, matching what one item does today. A shared pivot
  (Selection or Automatic centre across items) is an action-centre
  question: log it, don't guess. VAC-Q04 already refuses non-default
  centres for items.
- The handle is at the primary's position, as today, until the pivot
  question is answered.
- It's one session and one Drop, like the component case.

## Building on the transform-retarget thread

That thread re-activates the open transform tool when the item-tree
selection changes. Stage 2 should compute `transform_targets()` at
activation and re-activation only, never per frame. A retarget then
rebuilds the targets and the combined frame through the same route. The
MAM-01 guard stays for every non-transform tool.

## Tests to write then

- Two meshes, one polygon each: one handle at the pooled centre, both
  move, and one undo step restores both.
- A mesh with nothing selected while another has a selection: it doesn't
  move.
- Falloff across two meshes: weights are read in world space.
- Symmetry on two meshes: each mirrors in its own space.
- Item mode: all selected items move; a locked one refuses; a selected
  parent and child move once.
- Cancel restores every mesh. Save and reopen after Drop.
