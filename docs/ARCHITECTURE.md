# Operandi architecture

A map of the code for new contributors: what each crate does, how an edit
travels from a click to the saved file, and where the big subsystems live.
It's an overview; each subsystem's own doc in `docs/` has the detail. The
older design notes are in [Operandi_Architecture_v0_1.md](Operandi_Architecture_v0_1.md).

## The crates

| Crate | What it is | Rough size |
|---|---|---|
| `operandi-core` | The model: meshes, scene and items, selection, every modelling operation, commands, undo history, shading data, file formats. No window, GPU or OS code, so all of it is testable headless. | 124k lines |
| `operandi-app` | The egui/eframe application: panels, toolbox, Tool Properties, viewport input and wgpu drawing, menus, preferences, dialogs, OS windows. Turns input into core commands and draws the result. | 176k lines |
| `operandi-files` | The one safe file writer: a unique temporary beside the file, synced, then renamed over it. Scenes, images, exports and bakes save through it. | 0.1k lines |
| `operandi-layout` | The workspace layout model: panes in splits and tab groups, torn-off windows, saved layouts. Panes are named, not tied to widgets. | 2k lines |
| `operandi-cycles` | Final renders through Cycles (Blender's path tracer, Apache-2.0) as a C++ library behind a small C interface (`cycles/operandi_cycles.h`). Off unless built with `--features cycles`. | 10k lines + C++ |
| `operandi-uv` | UV unwrapping and packing, a port of Blender's UV parametrizer, packer and SLIM solver (GPL-2.0-or-later / MPL-2.0). | 7k lines |
| `operandi-usd` | USD export through OpenUSD: the prims are planned in Rust, and a C++ bridge (`bridge/`, built by `scripts/build-usd.sh`, the `usd` feature) writes them. Part of the bridge is Blender-derived. | 1k lines + C++ |
| `operandi-alembic` | Alembic read and write through a small C shim over the vendored Alembic library (BSD-3-Clause). | <1k lines + C++ |
| `operandi-mcp` | Operandi's own Model Context Protocol bridge, so AI agents and scripts can drive Operandi (a headless sidecar or the running app over a local socket). | 4k lines |
| `operandi-preset-convert` | A small command-line converter for material presets. | <1k lines |

Outside the crates: `vendor/egui-winit` (a patched egui-winit for keypad
keys), `tools/operandi-icons` (the Python icon generator), `tools/blender-export`
(a Blender-side material exporter), `scripts/` (bundles, Cycles and release
builds) and `fixtures/` (original test scenes).

## How an edit flows

```
 mouse / keys / menu / command line / MCP
                 │
     operandi-app: input map, toolbox, panels, viewport handles
                 │   (every entry point resolves names through
                 │    command_names.rs and the command registry)
                 ▼
     operandi-core: Command  ──►  Editor  ──►  Scene (+ meshes)
                 │                   │
                 │                   └─► undo history: one step per command,
                 │                       or one per dropped tool
                 ▼
     operandi-app: viewport patches GPU buffers, panels redraw
                 │
     save: persistence.rs (.opscene JSON, packed big meshes)
```

- **Commands.** `operandi_core::editor::Command` is the list of edits the core
  can make. Buttons, menus, shortcuts, the command line and the MCP bridge all
  end in the same commands, so they behave the same and are all undoable.
- **The command registry** (`command_registry.rs`, see
  [COMMAND_REGISTRY.md](COMMAND_REGISTRY.md)) lists every command the command
  line accepts, with arguments, units and help. Tests hold the parser, the
  registry and every drawn button and menu entry to each other.
- **Names.** Command, tool and argument names follow the reference name map
  so scripts port; renames and deprecated names live in
  `command_names.rs` ([COMMAND_NAMES.md](COMMAND_NAMES.md)).
- **Tools and live preview.** An interactive tool (Bevel, Extrude, Move…)
  keeps the scene it started from, recomputes a preview from it on each
  drag frame, and commits one undo step when dropped (with per-tweak steps
  inside the tool). Cancel restores the start. Tool Properties panels are
  field lists driven by the same parameters (`tool_form.rs`,
  `NumericSession`).
- **Tool cards and the tool chain.** Every tool is a stack of shared layers
  (action centre, falloff, symmetry, snapping), then its own geometry.
  `operandi-core/src/tools/tool_card.rs` gives each tool one card: its family,
  starting modes, which layers it uses, its handle layout and anchor,
  multi-mesh and continue support. Shared code asks the card instead of
  keeping its own list of tools; the card's match has no wildcard, so a new
  tool doesn't compile until its card is filled in. The geometry (one arm
  per tool in `Editor::preview_one`) is not on the card yet.
- **Undo.** Each history step keeps the scene before and after. Meshes are
  shared between steps unless changed, so deep history stays cheap.
- **Component vs item edits.** Point, edge and polygon edits change mesh
  data; item transforms change item channels. They're kept apart.

## Inside the core

- **Mesh** (`mesh.rs`, `polygon.rs`): points, polygons with types (face,
  subdivision, curve), edges derived from polygons, vertex maps (UVs,
  weights, morphs), polygon tags (material, part, selection sets).
  Topology-only results (`edges`, `edge_counts`, `validate`) are memoised per
  topology in `topology_cache.rs` so drag frames stay fast.
- **Scene** (`scene.rs`): items (meshes, groups, locators, cameras, lights,
  backdrops, environments, render item), their transforms and hierarchy,
  the shader tree, images and selection.
- **Modelling operations**: one module or folder per tool family (bevel,
  extrude, slice, subdivide, booleans with Manifold, drills, duplicate,
  deformers and falloffs, sculpt brushes, vertex maps, freeze, Mesh Stack
  procedural stack).
- **Shading** (`shader_tree.rs`, `material.rs`, `node_material.rs`,
  `shader_eval.rs`, `shader_wgsl.rs`): the layered shader tree and node
  materials, evaluated on the CPU and compiled to WGSL for the viewport.
  `blender_principled.rs` is the Blender-derived principled surface.
- **Files**: `persistence.rs` (native `.opscene`), `lxo/` (LXO import and
  export, from the public format description), `interchange/` (OBJ, STL,
  glTF/GLB, PLY, FBX import through ufbx, Alembic, VideoScape GEO, SVG;
  USD export plugs in from `operandi-usd`). See [FILE_FORMATS.md](FILE_FORMATS.md).

## Inside the app

- **Viewport** (`viewport.rs`, `viewport/`, `gpu.rs`, `gpu_scene.rs`,
  `material_gpu.rs`): wgpu drawing of meshes, subdivision surfaces,
  wireframes, selection, handles and overlays. Only what changed is
  patched on the GPU; overlays are drawn on the GPU, not by CPU hit tests.
  Picking projects the mesh once per pointer move.
- **Input**: an editable input map (shortcuts shown on buttons come from
  it), mouse presets, pen tablets and 3D mice (`space_mouse/`).
- **Panels and forms**: a shared form component for every property panel
  and dialog (UI conventions UIC-80..125), toolbox tabs, the item tree,
  floating palettes in their own OS windows.
- **Fast drags**: the rules in [AGENTS.md](../AGENTS.md) and the speed
  guard test (`tool_speed_guard_tests.rs`) keep each drag frame under about
  16 ms on a 16k-polygon mesh.

## Rendering with Cycles

`operandi-cycles` builds Cycles at a pinned commit (`scripts/build-cycles.sh`)
into `liboperandi_cycles`, which the Rust side links when the `cycles`
feature is on. The app syncs the scene into a kept Cycles session (meshes in
local space, instanced by key, materials from the shader tree) and streams
progressive renders back. The C structs are versioned: change
`operandi_cycles.h` and you bump `OC_ABI_VERSION` there and in
`operandi-cycles/src/lib.rs`. See [CYCLES_SYNC_GUI_QA.md](CYCLES_SYNC_GUI_QA.md)
and [RELEASE_BUILDS.md](RELEASE_BUILDS.md).

## Scripting and AI agents

`operandi-mcp` maps MCP tool calls onto core commands, either in a headless
sidecar (for tests and batch work) or inside the running app over a local
socket. [MCP_BRIDGE.md](MCP_BRIDGE.md) lists the tools.

## Kits

Kits are optional add-ons installed into a Kits folder in the data folder
and turned on by the user. Kit API 1 lets a kit add file types to import:
a helper program in the kit's own process converts the file into one
Operandi reads (`kit_manifest`, `kit_folder`, `kit_import` in core;
`operandi.kit.*` in the app). [KITS.md](KITS.md) is the contract.

## Where the specs fit

Behaviour comes from behaviour specs (one folder per tool or system,
admitted under the [clean-room protocol](CLEAN_ROOM.md)), kept in a private
repository next to this one. Unanswered
questions go in [SPECIFICATION_QUESTIONS.md](SPECIFICATION_QUESTIONS.md);
[PROGRESS.md](PROGRESS.md) is the running build log and [STATUS.md](STATUS.md)
the summary of what's built.
