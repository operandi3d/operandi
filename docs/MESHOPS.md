# MeshOps: how the stack works and how to add an op type

MeshOps are procedural mesh operations stacked on a mesh item
(specifications/meshops-2026-11-11). This page is for anyone adding an op
type, built in or from a kit.

## The pieces

| File | What it holds |
|---|---|
| `operandi-core/src/meshop_types.rs` | The op type table: `OpDef`, form `Field`s, `OpContext`, item links (`Source`, `Linked`), `register` for kits |
| `operandi-core/src/meshops_builtin.rs` | Operandi's own op types and their table entries |
| `operandi-core/src/meshops.rs` | The stack (`MeshOpStack`, `MeshOp`, `OpKind`), saving, the guard, the cache and background evaluation (`Runtime`), loop checks |
| `operandi-core/src/meshops_editor.rs` | The commands that edit stacks (`meshop.create`, `item.channel`, `deformer.enable` ...) |
| `operandi-app/src/mesh_ops_ui.rs` | The Mesh Ops tab and the op Properties form |

`meshops.rs` never names a particular op type. Everything about a type comes
from its table entry.

## Adding an op type

1. Write the values as a struct with serde names equal to the saved
   channel names (the reference attribute names, so `item.channel` scripts
   port). `Default` gives the direct tool's defaults (MO-40). Use
   `#[serde(deny_unknown_fields)]`.
2. Implement `TypedOp::run(&self, ctx)`: return the new mesh from
   `ctx.input`. Re-check values a script could set out of range and return
   an error for them (SAFE-10). Use `ctx.chosen()` / `ctx.chosen_or_all()`
   for the Selection input and `ctx.sources()` for linked items. Call the
   direct tool's kernel; never its live session.
3. Add a table entry: `Typed::<YourOp>::new(id, label, folder,
   takes_selection, fields)`. Built-ins go in `builtin()`; a kit calls
   `operandi_core::meshops::register(Arc::new(...))` once at start.
   - `id` is the saved type id. A built-in uses the reference id from the
     registry thread's MESHOPS-NAMES list. A kit uses its own dotted
     namespace (`mykit.cable.item`).
   - Every saved value needs a `Field` (KIT-4: the app draws the form).
     `FieldKind::Item` / `Items` fields are item links.
4. Tests: one per acceptance criterion, plus the op shows up in the
   stress harness automatically (`meshops_stress.rs` picks from the table).

Nothing else changes: the menu, Properties, saving, undo, loop checks and
caching all read the table.

## Rules every op gets for free

- **Guarded (SAFE-04).** A panic, an error, an invalid or over-limit result
  fails only that op; it passes its input through and its row turns red.
- **Cached (MO-03).** An op re-runs only when its values, the mesh below,
  its linked items' results or their placement change. Ops that say
  `uses_time()` also re-run when the scene time changes.
- **Item links (SAFE-07..09).** An op's linked items are evaluated first
  (their own stacks included) and handed over with the matrix into this
  item's space (`Source::local_mesh`). A link that would make a loop is
  refused when made; a file with a loop opens with the closing op turned
  off. A link to a deleted item fails only that op ("Missing input"), and
  undo brings it back. Hidden items evaluate as usual.
- **Unknown types (SAFE-12, K7).** A file with an op type nobody has
  registered loads it as a placeholder that keeps its data and saves back
  unchanged. Once a kit registers the type, the op runs.
- **Selection input.** With `takes_selection`, `ctx.selection` is the
  polygons the op's Select by Index and selection ops picked (`None` =
  whole mesh). When they pick nothing the op isn't run at all.
- **Groups for Select by Previous Operation (PG-xx).** Call
  `ctx.tag(&mut mesh, "front", faces)` on your result for each group, and
  list them with `Typed::with_groups(&[("front", "Front")])` so the panel
  can offer them. Groups follow untouched polygons through later ops and
  never reach the shown result.
- **Saved values are versioned (SAFE-12).** Bump `OpDef::version` when a
  value changes meaning, and upgrade older values in `OpDef::read`.
