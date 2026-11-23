# Code map

Where to look for common jobs, so a fix starts in the right file. For what
each crate is and how an edit flows, read [ARCHITECTURE.md](ARCHITECTURE.md);
this page is the "which files" companion. Paths are under `crates/`.

## Finding things fast

- Core modules are named after the tool or system (`bevel.rs`,
  `twist_bevel.rs`, `falloff.rs`). The first `//!` line of each file says
  what it is and which spec it follows:
  `grep -m1 '^//!' crates/operandi-core/src/*.rs` lists them all.
- Spec IDs (`TWB-03`, `UIC-51`, `VX-B11`) appear in code comments and test
  names, so grepping an ID finds the code and its test.
- A tool's command name (`poly.bevel`, `xfrm.bend`) finds its registry row
  (`operandi-core/src/command_registry.rs`), its handler and its tests.
- App tests live next to the code as `*_tests.rs` in `operandi-app/src`;
  core tests are one file per tool or spec in `operandi-core/tests/`.

## The files most fixes touch

| Job | Start here |
|---|---|
| Tool list, tool enum, every `Command` | `operandi-core/src/editor.rs` (`enum Tool`, `enum Command`, `preview_one` = per-tool geometry arm) |
| What a tool is (family, modes, layers, handles) | `operandi-core/src/tool_card.rs` |
| Preset buttons (Soft Move, Flex, Flare, Flip It ...): pipe layers + tool | `operandi-core/presets/factory-tool-presets.toml` (data), `operandi-core/src/tool_preset.rs` (activate / drop rules) |
| Drag controls (hauls): which value each mouse button drags per tool | `operandi-core/presets/factory-hauls.toml` (data), `operandi-core/src/haul.rs` (axis lock, steps, feel numbers), `operandi-app/src/haul_view.rs` (viewport) |
| A tool's geometry | its own core module (`bevel.rs`, `edge_extend.rs`, ...) |
| Command names, renames, deprecated names | `operandi-core/src/command_names.rs`, `docs/COMMAND_NAMES.md` |
| Command-line commands and arguments | `operandi-core/src/command_registry.rs`; app handlers in `operandi-app/src/command_handlers.rs` |
| Undo inside and after tools | `operandi-core/src/mid_tool_undo.rs`, `history_policy.rs` |
| Scene, items, hierarchy | `operandi-core/src/scene.rs`, `channels.rs`, `item_*.rs` |
| Mesh data and topology memo | `operandi-core/src/mesh.rs`, `polygon.rs`, `topology_cache.rs` |
| Saving and opening `.opscene` | `operandi-core/src/persistence.rs`, `mesh_pack.rs` |
| Import / export (OBJ, FBX, glTF, LXO...) | `operandi-core/src/interchange/`, `lxo/` |
| USD import / export (OpenUSD bridge, reader, scene builder, import dialog) | `operandi-usd/src/` (`import.rs`, `import_materials.rs`, `stage.rs`), `operandi-usd/bridge/`, `operandi-app/src/usd_import_dialog.rs` |
| MeshOps (procedural stack), op types, adding an op | `operandi-core/src/meshops.rs`, `meshop_types.rs`, `meshops_builtin.rs`; `operandi-app/src/mesh_ops_ui.rs`; guide in [MESHOPS.md](MESHOPS.md) |
| Node View panel (registry kind, follows the Shader Tree selection, docks under the views) | `operandi-app/src/node_material_ui.rs`, `node_edit_ui.rs`, `panel_docking.rs` `reveal_registry_panel` |
| Closing a registry panel (header ×, Close Panel, Node View toggle, window close) | `operandi-app/src/panel_close.rs`, `node_view_open.rs` `toggle_node_view` |
| Shader tree and materials | `operandi-core/src/shader_tree.rs`, `shader_eval.rs`, `shader_wgsl.rs`, `material.rs`; MaterialX graphs `materialx/` (`cycles.rs` translation, `graph.rs`, the tree's own form `tree.rs` + `tree_library.rs`, docs/MATERIALX_GRAPHS.md); graph layers in the viewport `graph_view.rs` (+ `shader_func.rs`/`.wgsl`, `cycles_noise.rs`) |
| Action centre, falloff, symmetry, snapping layers | `action_center.rs`, `falloff.rs`, `symmetry*.rs` (core); `snapping.rs` (app) |
| App start-up, module list, top-level frame loop | `operandi-app/src/main.rs` |
| Toolbox buttons, icons and tooltips per tool | `operandi-app/src/tool_info.rs`, `toolbox.rs`, `icons.rs` |
| Tool Properties panel | `operandi-app/src/live_fields.rs` (live tools), `tool_form.rs` (field lists), `form/` (shared rows) |
| Viewport input, drags, handles | `operandi-app/src/viewport.rs`, `viewport/*_handles.rs`, `input.rs` |
| Viewport drawing on the GPU | `operandi-app/src/gpu.rs`, `gpu_scene.rs`, `material_gpu.rs` |
| Menus | `operandi-app/src/app_menu.rs` |
| Shortcuts and the input map | `operandi-app/src/input.rs`, `preferences.rs` |
| Greyed-control reasons | `operandi-app/src/unavailable.rs` |
| Viewport header buttons / Properties popover | `operandi-app/src/viewport_buttons.rs`, `viewport_properties.rs` |
| Shading tab | `operandi-app/src/shading_panel.rs` |
| Floating windows, layout | `operandi-app/src/floating.rs`, `workspace.rs`, `operandi-layout` |
| Layout files and switching (`layout.restore`), shipped layouts | `operandi-app/src/layouts.rs`, `assets/layouts/*.json`, `operandi-layout/src/window.rs` |
| Toolbox forms (every tab: Create, UV, Select, Deform, Duplicate, Vertex, Edge, Polygon, Edit, Sculpt; Select's drop-downs in `select_panel.rs`): form files, user copies, Revert, slots, button state from commands (`operandi.form.revert`) | `operandi-app/assets/forms/*.json`, `operandi-app/src/form/command_form.rs` (data), `toolbox_form.rs` (drawing), `command_handlers.rs` `answers` (lit/greyed per command) |
| Workbenches (layouts stage 4: Edit Workbench, Pick to Add, reorder, right-click properties, tool-pipe presets, the top row; `operandi.workbench.*`) | `operandi-app/src/workbench.rs`, `assets/forms/toolbox.workbench.json`, `toolbar.workbench.json`; property rows via `tool_form.rs` `only_field` |
| Form Editor (layouts stage 7: System > Form Editor palette, tree/search/Find Form, properties, `operandi.form.*`), menus drawn from forms (`menu.*` forms, `menu_from_form`, `Action::Line`) | `operandi-app/src/form_editor.rs`, `app_menu.rs`, `assets/forms/menu.*.json`, `command_handlers.rs` `menu_answers` |
| Menus as forms: the whole menu bar (`menu.bar` and `menu.*` forms), live lists (`LISTS`), `{title}` placeholders, the in-window bar, menu snapshots | `operandi-app/src/menu_forms.rs`, `menu_forms_tests.rs`, `menu_snapshots/`, `app_menu.rs`, `native_menu.rs`, `command_handlers.rs` `menu_commands` / `menu_answers` |
| Palettes (layouts stage 6: palette files, open/close toggle, Toggle Palettes, Layout > Windows/Palettes, floating views app-wide across switches, `layout.window`) | `operandi-app/src/palettes.rs`, `assets/palettes/*.json`, `floating.rs` (side-panel palettes), `uv_editor.rs` |
| Per-layout key maps (layouts stage 5: `keys` in a layout file, the effective `bindings`, `operandi.layout.setKey` / `clearKeys`; the Input Map's "Keys for" scope) | `operandi-app/src/layout_keys.rs`, `input_map_editor.rs` |
| User layouts, layout bar, quick switcher, pieces, content arrow (`operandi.layout.*`, `operandi.viewport.*`, `operandi.panel.kind`) | `operandi-app/src/layouts_edit.rs` |
| Final renders (Cycles) | `operandi-cycles/src/lib.rs`, `operandi-cycles/cycles/operandi_cycles.{h,cpp}` |
| 3D painting (painting builds only): images, image commands, PSD | `operandi-paint/src/` (pixels, blend, io, psd), `operandi-core/src/paint.rs`, `operandi-app/src/painting.rs`; vendored `vendor/craft-psd` |
| MCP bridge | `operandi-mcp/src/bridge.rs`, `operandi-app/src/mcp.rs` |
| Python scripting (interpreter, `operandi` module, Script Console, switch) | `operandi-python/src/` (`lib.rs`, `embed.rs`, `operandi.py`), `operandi-app/src/python_ui.rs`, `python_switch.rs`, `operandi-core/src/script_mesh.rs` |
| Command records, History/Results logs, Command History panel (`cmds.batch`, `cmds.undoTo`, `history.*`) | `operandi-core/src/command_record.rs`, `operandi-app/src/command_history.rs`, `command_history_ui.rs` |

## Adding a tool: the usual file list

Twist Bevel (commit `89763be0`) is a good recent example to copy. A new
interactive tool normally touches:

- **core:** its own module (`<tool>.rs`) and `lib.rs`; `editor.rs` (`Tool`
  variant, `Command`, `preview_one` arm); `tool_card.rs`;
  `command_registry.rs`; `mid_tool_undo.rs` / `history_policy.rs` when it
  has steps; `identity_editor.rs`.
- **core tests:** `tests/<tool>.rs`, plus the lists in `tests/tool_card.rs`
  and `tests/tool_list.rs`.
- **app:** `tool_info.rs` and `toolbox.rs` (button), `live_fields.rs` /
  `tool_form.rs` (Tool Properties), `viewport.rs` and
  `viewport/<tool>_handles.rs` (handles), `input.rs`, `command.rs`,
  `app_menu.rs`, `about.rs`; `tool_speed_guard_tests.rs`,
  `performance_tests.rs` and `tool_card_view_tests.rs` (they won't compile
  until the tool is in them); `<tool>_tests.rs`.
- **docs:** `PROGRESS.md`, `COMMAND_NAMES.md`, `<TOOL>_GUI_QA.md`, and
  `SPECIFICATION_QUESTIONS.md` for gaps.
- **icons:** a recipe in `tools/operandi-icons`, then the generated SVG in
  `operandi-app/assets/icons` (see "Icons" in AGENTS.md).

## Which tests cover what

`scripts/check-fast.sh` picks the scope from what the branch changed since
main:

| You changed | It runs |
|---|---|
| Only `operandi-app` | clippy and tests for `operandi-app` |
| Anything else in `crates/`, Cargo files or `vendor/` | clippy and tests for the whole workspace (most crates are built on core) |
| Only docs, specs or icon sources | `cargo fmt --check` only |

Test locations:

- `operandi-core/tests/*.rs`: one test program, `tests/all.rs`; one file's
  tests are `cargo test --locked --workspace -- <file>::`.
- `operandi-core/src`: unit tests inside modules (`mod tests`).
- `operandi-app/src/*_tests.rs`: app tests, all in the app's one test
  program (they're modules of `main.rs`).
- The slowest tests (20-35 s each, in debug builds): `meshops_stress`,
  `drag_undo_guard_tests::every_tool_drag_records_at_most_one_undo_step`,
  `gpu_chunks`, the wire-fade tests. They set the floor on a full run.

See "Checks" in [AGENTS.md](../AGENTS.md) for the fast and full tiers.
