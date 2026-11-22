# Architecture check, 2026-11-07

Branch `colour-nav-tag-modes`. For each item in note 09 (F1–F9) and each
"What Operandi needs now" list in notes 01, 02, 03, 05, 06 and 08: **already
fine**, **small change now**, or **needs a design**, with evidence, and where
it lands in the approved waves:

- wave 1: merged.
- wave 2: #1 attribute transport, then #3 evaluated mesh + Item kind.
- wave 3: #5 command registry + typed error, #12 reference command names
  with aliases (merged as `command-names`), then #7 shader-graph IR + generic
  C ABI.
- wave 4 (before sculpt): #2 chunked mesh / binary file, rest of #4 Tool
  trait, #9 topology cache + rayon. #6 ongoing.

No architecture review document with the numbered items is in the repo
(only the older `docs/Operandi_Architecture_v0_1.md`), so the wave mapping
uses the numbers as given in the brief. Line numbers are on this branch.

Scaffolding built now (cheap, nothing depends on it yet):
`crates/operandi-core/src/scaffold.rs` — `ToolKind { Direct, Procedural,
Schematic }` with `Tool::kind()` (all Direct), `Proficiency { Basic,
Standard, Advanced }` defaulting to Standard with `shown_at`, and channel
description types `ChannelId`, `ChannelType`, `ChannelValue`, `ChannelDef`.
Also built in this branch and useful beyond colours: `Editor::undo_fold`
(editor.rs, `record`): every undo step recorded under one gesture id folds
into one, for any command.

## Note 09: future systems

| # | Item | Verdict | Evidence | Wave |
|---|---|---|---|---|
| F1 | Typed channels on every item, read through one evaluator | **Needs a design** | Items are plain structs with one optional block per kind: `Item` (scene.rs:47–81) holds `transform`, `mesh`, `camera`, `light`, `texture`, `backdrop`, `common`. The nearest thing to channels is the shader tree's per-node accessor list `Channel<'a>` (shader_tree.rs:466) and user channels `ChannelType` (item_extras.rs:121), neither has ids, defaults or one read path. Scaffold types added (scaffold.rs). | Design with #3 (wave 2), build after #5 (wave 3) so commands can set channels generically. |
| F2 | Scene time known to the evaluator | **Needs a design** (small once F1/F3 exist) | No frame, rate or range anywhere in core (no `frame`/`fps`/`time` fields in scene.rs:129–165). | After F3; trivial struct then, but useless before an evaluator exists. |
| F3 | Evaluated scene/mesh separate from stored data, cached, dependency-invalidated | **Needs a design** (this is #3) | Stored mesh is drawn and rendered directly; the only evaluated data are app-side caches: subdivision `SubdivisionCache` (subdivision_view.rs:37) and the core topology memo (topology_cache.rs). No deformer or procedural stack (no `deformer` in core). `Shared<Mesh>` (shared.rs:62) gives cheap copies, a good base. | Wave 2 (#3). |
| F4 | Item kind registry; new kinds without touching a central enum | **Needs a design** (#3) | Kinds are fields on `Item` (scene.rs:64–74) and `is_mesh()` checks; adding a kind edits `Item`, persistence, Items list, Properties routing. | Wave 2 (#3), together with F3. |
| F5 | Links/relationships as scene data | **Small change now** for the model, **needs a design** for evaluation | Parent is `Item.parent` (scene.rs:54) with `set_parent` (scene.rs:568); item groups (scene.rs:159). No channel links or deformer/constraint roles. A `Relationship { kind, from, to }` list on `Scene` is a small serde addition, but has no consumer until F1/F3. | Model with F1 (wave 3); don't add it empty now. |
| F6 | Panels registered by kind, layouts as data | **Small change now** | Panes already are data: `operandi_layout::Pane { Toolbox, Viewport(n), Items, Properties }` (operandi-layout/src/lib.rs:21) in an `egui_tiles` tree saved/loaded (views.rs:367, 375). Side panels are still hard-coded (workspace.rs:100 `right_sidebar`, floating.rs:10, PANEL-Q01). Adding a `Pane` variant per new panel kind is the registry for now. | #6 ongoing; side panels into the tree first (see note 01). |
| F7 | Undo that copes with big/time-varying data | **Already fine** for now; **needs a design** before keys/caches | Each step stores whole `Scene` before/after (`Checkpoint`, editor.rs:327–336) but meshes are `Shared` (Arc) so untouched meshes cost nothing; `max_undo_level` caps it (history_settings.rs:9). Keys/sim caches must be excluded or chunked: that's #2. | Wave 4 (#2). |
| F8 | File format storing channels generically, versioned sections | **Needs a design** (#2) | One serde JSON document, single `CURRENT_VERSION = 21` (persistence.rs:10); `deny_unknown_fields` on `Scene`/`Item` means every new field is a version bump. No sections. | Wave 4 (#2). |
| F9 | Cycles C ABI generic enough for curves, points, instances, motion time samples | **Needs a design** (#7) | ABI v6 (operandi_cycles.h:29) passes triangle meshes only (`OcMesh` :199) with a fixed `OcSurface` (:121) plus a texture op list `OcExpr` (:79); one entry point `oc_render(meshes, …)` (:394). No curves, point clouds, instances or time. | Wave 3 (#7). |

Animation notes: transforms are one fixed `Transform { translation,
rotation, scale, zxy }` per item (scene.rs:8–13) — **needs a design** to
become an ordered stack of transform channels; it fits F1 (each entry a
channel group) and F8 (sections). Pivot/centre: no pivot field yet.
Topology work (closest point on other meshes) needs a per-evaluated-mesh BVH:
**needs a design** with #9 (wave 4); picking today projects meshes in the
app (viewport.rs), not a reusable core service.

## Note 01: switcher and layouts

| # | Need | Verdict | Evidence |
|---|---|---|---|
| 1 | Layout registry (id, name, starred, ordinal, shipped default, user copy); ship "Model" | **Small change now** | One layout exists implicitly: the views tree (views.rs:367 `save_views`) plus side-panel widths (layout.rs). A `LayoutId` + registry struct wrapping today's tree is small. |
| 2 | Per-layout state incl. toolbox choice; user copy only when different | **Needs a design** (small) | Toolbox tab is app-global state (toolbox.rs:414); the tile tree is saved as one file. Needs the "overrides only" save rule from colour themes (color_theme.rs). |
| 3 | Switch = one command used by menus, bar, keys, scripts | **Small change now** | Commands route through `execute` with name rewriting (main.rs:2359, command_names.rs:108). One `layout.restore`-style command plus alias fits. |
| 4 | Panels registered by kind so a layout can name them | **Small change now** | `Pane` enum is the kind list (operandi-layout/src/lib.rs:21); new kinds (UV View, Timeline, Schematic) are new variants. |
| 5 | Switcher bar hidden until a second layout | **Already fine** (nothing to hide yet) | — |
| SL-Q1 | Side panels not in the layout tree | **Needs a design** | Toolbox/Items/Properties are `SidePanel`s (workspace.rs:100, panels.rs) with a floating flag (floating.rs:10), outside the tile tree. Moving them in is the real cost of per-layout toolboxes: medium (layout tests in layout_tests.rs cover widths and scrolling). |

## Note 02: tool properties placement

| Need | Verdict | Evidence |
|---|---|---|
| `ToolKind { Direct, Procedural, Schematic }` on each tool/op | **Small change now — done** | scaffold.rs `ToolKind`, `Tool::kind()` (all Direct). `Tool` is one enum (editor.rs:5–66). |
| Properties routing reads the kind | **Small change** when the first procedural op exists | Tool properties are drawn per tool in the left panel (tool panels: bevel_panel.rs, edge_panel.rs …); nothing to route yet. |
| One property-form description drives both places | **Needs a design** (#4 Tool trait) | Each tool's panel is hand-written egui code; parameters live in per-tool structs (`Parameters`, `BridgeParameters`, …). A shared form description is the #4 work. |

## Note 03: Form View Options

| Need | Verdict | Evidence |
|---|---|---|
| Key bindings scoped to the panel under the pointer | **Needs a design** — the real architecture item | Bindings are global name→chord pairs (input.rs:158 `ACTIONS`), gated only by selection mode and live tool (`input::available`, input.rs:626). `O` is already taken globally by Viewport Properties (input.rs:167). Needs a "context" on each binding (viewport / form / list) and the hovered panel's context read in `keyboard` (main.rs:2075). Medium; touches the input map editor. |
| Every form control carries a proficiency tag, default Standard | **Small change now — scaffold done** | `Proficiency` in scaffold.rs. Rows are drawn by `option_row::row/field_row/check` (option_row.rs:81–103): adding an optional tag argument there covers most forms. Not wired yet (no consumer). |
| Per-view options struct saved with the layout | **Small change now** | Per-view settings already save by view key (`view_display::Settings`, views.rs:251–256); a forms counterpart follows the same pattern once forms are panes. |

## Note 05: kits (plugins)

| Need | Verdict | Evidence |
|---|---|---|
| Manifest format | **Small change now** (just a serde struct) but no use until 3 | — |
| Registry read at startup; enabled set in prefs | **Small change** after 3 | Preferences already load per-file structs (units_preferences.rs:205–230 pattern). |
| KEY CHECK: plugins add through the same registries as built-ins | **Needs a design** | There are no registries: commands are one enum (`Command`, editor.rs:461–935) matched in `dispatch`; command text is a big `match` in `execute` (main.rs:2359+); tools are the `Tool` enum; panels are `Pane` variants; menus are built in app_menu.rs/panels.rs. Only names have a table (command_names.rs). This is #5 (wave 3) and #4 (wave 4). |
| Toolbar-button visibility per plugin pref | **Small change** after 3 | Toolbox rows are code (toolbox.rs). |
| Plugin language, sandboxing, GPL | **Needs a design** (later) | — |

## Note 06: schematic

| Need | Verdict | Evidence |
|---|---|---|
| Everything node-able is an item with stable id and typed channels | **Needs a design** (F1/F4) | Shader layers are tree nodes with ids (`NodeId = u64`, shader_tree.rs:21) but not items; procedural ops/deformers/falloffs don't exist as items (falloffs are editor state, `spatial_falloff`). |
| Links as scene data (`ChannelLink`, `Relationship`) | **Needs a design** (F5) | Only `Item.parent` and groups (scene.rs:54, 159). |
| Node positions / workspace membership as view data | **Small change** when the schematic exists | View data already saves apart from the scene (views.rs). |
| Channel types with a conversion table | **Small change now — types done** | `ChannelType` in scaffold.rs; the conversion table comes with links. |
| #7 shader-graph IR = the same item/channel/link graph | **Needs a design** (#7) | Today the shader tree compiles to `OcExpr` ops and a fixed `OcSurface` (operandi_cycles.h:79, 121). |

## Note 08: channels

| Need | Verdict | Evidence |
|---|---|---|
| Every editable value has id, type (+unit kind), default, min/max, lock | **Needs a design**; types scaffolded | `ChannelId`/`ChannelType`/`ChannelDef` in scaffold.rs. Values are struct fields edited through per-kind edit enums (`LightEdit`, camera_light.rs:1165; `CameraEdit` :1318; `ShaderTreeEdit`). Lock: items have Lock in `ItemCommon` (item-level only). |
| Values read through one place ("evaluate at time t, with links") | **Needs a design** (F1+F3) | Readers access fields directly (renderers, viewport, Properties). |
| Storage can later hold static / key curve / driven | **Needs a design** (F8) | Plain `f64`/`[f64; 3]` fields. |
| Commands set values through a generic `channel.value` path | **Small change now** once F1 has names | The per-kind edit enums are already "set one value" commands with undo; a generic `SetChannel { id, value }` can dispatch to them through a name table. With `undo_fold`, gang edits across items are already one step. |
| CH-Q1 (estimate only): default/changed dot + right-click Reset | **Small** (~1 day) | Preferences colours already show overridden vs default and reset (preferences.rs `color_row`); the colour field has Reset to Default (`Options::with_default`). A dot needs each row to know its default: cheap for shader channels (defaults exist in `Default` impls), per-row work elsewhere. |

## Suggested order (no change to the waves)

1. Wave 2: #3 should include F4 (kind registry) and F3 (evaluated mesh,
   deformer stack slot) together; design F1 channel ids alongside so the
   registry hands out `ChannelDef` lists per kind.
2. Wave 3: #5 command registry → generic `SetChannel` (note 08 #4), plugin
   registration path (note 05 #3); #7 IR shares the channel/link model.
3. Before layouts work: move side panels into the tile tree (SL-Q1) and add
   binding contexts (note 03), both medium, both independent of the waves.
4. Wave 4: #2 sections + chunked mesh covers F7/F8; #9 adds the BVH needed by
   topology tools.
