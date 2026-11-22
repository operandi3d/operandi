# Command registry (extensibility plan, phase 0, stage 1)

Branch `command-registry`. UI conventions bible section 11 (UIC-160..164).

## What it is

`crates/operandi-core/src/command_registry.rs` is one table of every
command Operandi's command line accepts: 338 entries (99 script commands
such as `poly.flip` or `tool.set`, and 239 plain action phrases such as
`Grow` or `Top View`, which are the Input Map's names). Each entry has:

- its name and any other names the parser takes for it (`Box` for `Cube`,
  `pen` for `operandi.pen.create`);
- its arguments: name, type, unit (metres, 0–1 percent, radians), default,
  and whether it's optional;
- flags: undoable, query (answers `?`), needs the window (opens a dialog or
  only changes a view), starts a tool;
- one line of help and an example line;
- an area (modeling, selection, tools, sculpt, view, viewport, items, shading,
  presets, files, render, preferences, help, debug).

`API_VERSION` is 1. Old names (`selset.use`, `edge.spin`, `tweak`, …) still
come from `command_names.rs`; the registry finds the entry through them and
reports "Deprecated command name: old is now new" (plan L5). `tool.set` ids
are one list, `TOOL_IDS` (62 tools).

Since stage 3 (below) the registry also runs commands: `execute` looks a
line up in the run-time registry and calls its handler. Lines no handler
takes yet still go through the app's older parsers (`command.rs` and the
panel files).

## New commands

- `command.list` shows how many commands there are per area.
  `command.list <area or word>` names the matching ones (for example
  `command.list sculpt`); the full lines go to the terminal (stderr).
- `help <name>` now answers for every command. Commands with their own long
  help keep it; the others show the registry's usage line, flags and help.
  Multi-word phrases work too: `help top view`.

## Tests that hold it together

In `crates/operandi-app/src/command_registry_tests.rs`:

1. Every registry name (and each alias, and each example) is understood by
   the parser, and `help <name>` answers.
2. Every name the parser accepts is in the registry: the test reads the
   parser's tables (tool names, Input Map actions, edge/vertex verbs, panel
   command tables) and the string patterns of its match arms, so a new arm
   without a registry entry fails.
   A guard test lists the sub-parsers `execute` hands lines to, so a new
   sub-parser (like `image.reload`'s) fails until its table is read too.
3. Every `tool.set` id answers `tool.set <id> ?`; the pie phrases' word
   lists (`view top`, `toggle grid`, …) match the registry's choices.
4. Every old name in `command_names.rs` reaches an entry and still runs.
5. **UIC-164**: the test draws the app headlessly (toolbox tabs, every
   tool's Tool Properties, the Properties panel for a mesh, Backdrop, camera
   and every light type, every shader-tree layer type, the Viewport
   Properties tabs, every Preferences page) and records each row and
   toolbox button the shared widgets draw, plus every menu entry. Each one
   must run a registered command or be listed in
   `crates/operandi-app/src/command_registry_gaps.rs`. A new click-only
   control fails the test; so does a listed gap that has since got a
   command (the list only shrinks).

## The gaps today

1,210 controls found; 238 run a command; 953 are click-only (known gaps):

| Area | Gaps |
|---|---|
| Lights | 121 |
| Cameras | 11 |
| Shader tree | 157 |
| Textures | 122 |
| Viewport options | 45 |
| Preferences | 246 |
| Tool properties | 150 |
| Other (menus, toolbox, item rows) | 101 |

Notes:

- A row shared by several panels (Transform's Position, a tool's Segments)
  is counted once, in the first place the test met it, so cameras look
  small: most camera rows are counted under the shared Item/Transform tabs
  in "other".
- Tool properties: a few tools' settings are already scriptable with
  `tool.attr` (Spike, Smooth Shift, Bridge, Edge Bevel/Chamfer, Polygon
  Reduction), but rows are recorded by label, and labels like "Segments" are
  shared by tools that aren't, so they stay listed until `tool.attr` covers
  every tool.
- Toolbox (38): about half are placeholders for tools not built yet (Bend,
  Vortex, Dicer, Lazy Selection…); the rest run an operation that has a
  command under another name ("Add Loop", "Freeze…", "Vertex New Vertex")
  and only need the button to name it.
- Menus (37): files, projects, presets, layout and window entries.
- Only controls drawn with the shared row component (`form::rows`,
  `form::panel`) and toolbox buttons are recorded. Hand-drawn controls
  (shader-tree list rows, item-list rows, viewport header buttons) aren't
  yet; stage 4 should move them onto shared widgets or record them.

## Things found on the way

- `open <path>` and `spike strength <v>` only work in lower case; every
  other phrase ignores case. The registry lists them in lower case.
- `tool.set` ids with spaces (`Curve Slice`, `Radial Sweep`) need braces:
  `tool.set {Curve Slice} on`. Sculpt brush ids don't accept braces.
- `tool.doApply` was on the parser's list of verbs but only gave a generic
  "Unsupported" message; it now says to use `tool.apply`.
- `selset.ungroup` exists as a command but isn't in `command_names.rs`'s
  rename table (there's no new name for it in the map yet).
- The Image Viewer's seven Input Map actions are handled only inside that
  window, not by the command line.
- No name clashes with `command_names.rs` were found: every old name maps to
  exactly one entry.

## Stage 2 (branch command-registry-2)

- **One runner without a window.** The MCP bridge's `command_run` table
  (`SPECS`: 52 commands, their JSON arguments and help) and its runner
  moved to core as `command_exec.rs`. The registry marks those 52 commands
  `headless`; a test keeps the two lists identical. (Stage 3 folded
  `SPECS` into the entries themselves.)
- **MCP reads the registry (L6).** The bridge's command list, its
  `command_run` schema and `command_help` come from the registry's headless
  entries, and `command_run` refuses anything the registry doesn't mark
  headless. Tool names and argument fields are unchanged
  (`operandi_mcp::commands::*` still works).
- **The same commands typed.** Core's `command_line.rs` holds the shared
  tokenizer and turns a typed line's `name:value` or positional words into
  the runner's JSON arguments (`item.transform translation:1,2,3`). The app
  runs these lines like the bridge does (refused while a dialog or tool is
  in use, staged so an error changes nothing). The 11 bridge commands
  that had no typed form now do: `mesh.new`, `item.select`, `item.delete`,
  `item.rename`, `item.visibility`, `item.parent`, `item.drop`,
  `item.transform`, `symmetry.set`, `select.symmetrical`, `smoothing.set`.
  No new names were made up; these are the bridge's existing names.
- **Fixes.** `open` and `spike strength` now ignore case (and "open render
  window" no longer tries to open a file of that name); sculpt brush ids
  take braces like every other tool id.
- **Window chrome exempt (UIC-165).** Lock Dividers, Lock Thumbs,
  Compact, Standard, Float, Tear Off View, Split View, Close View and
  Partial may stay click-only: they moved from the known gaps to a
  permanent `CHROME_EXEMPT` list. Known gaps are now 944 (other: 92).
- **Not done.** Commands that already had a typed parser in the app still
  run through it, so their behaviour is unchanged. Moving those parsers
  into core is the rest of stage 2. `selset.ungroup` stays as it is: the
  name map has no match for it (COMMAND_NAMES.md), and UIC-165 rules out
  inventing one.

## Stage 2b: Modo names (UIC-165, specifications/command-names/MODO-NAMES-SUBSET.txt)

- **Renames, old names kept as deprecated aliases:** `item.add` →
  `item.create`, `item.rename` → `item.name` (MCP too), `backdrop.attr` →
  `backdrop.edit`, `image.reload` → `clip.reload`, `preset.save` →
  `preset.saveAs` (`path:` → `filename:`), `preset.load` → `preset.do`
  (`path:` → `preset:`). `open` is now `scene.open` (`Open` stays as its
  phrase; "Open Render Window" is still its own command).
- **backdrop.edit** takes the map's named options (`project`, `aspect`,
  `flip`, `invert`, `blend`, `contrast`, `brightness`, `transparency` as 0–1,
  `image`) and still takes the old `<name> <value>` form, with the old
  units. Several named options are one undo step each (provisional).
  `backdrop.image` and `backdrop.size` stay their own Operandi commands:
  the map's `backdrop.edit` has no size option.
- **clip.reload** reloads every image when bare (the map's form) and still
  takes an image name or path.
- **Modo-named commands with the phrases as aliases:** `app.undo`/`app.redo`,
  `select.all`/`more`/`less`, `select.convert <vertex|edge|polygon>`,
  `lock.sel`/`unsel`/`invert`, `unlock`, `hide.sel`/`unsel`/`invert`,
  `unhide`, `cut`/`copy`/`paste`, `layer.mergeMeshes`/`layer.unmerge`,
  `viewport.fit [selected:]`, `viewport.fitSelected`, `viewport.maximize`,
  `render`, `cmds.showHelp` (`help`). The registry's `spelled` field names
  the phrase the app's parser runs (`parser_spelling`); the map's extra
  arguments on these (`app.undo steps:`, `select.more space:`) are refused
  for now rather than ignored.
- **Operandi-only names are marked** (`operandi` flag, shown in help): the
  `operandi.*` commands, `command.list`, `vert.weld`, `selset.ungroup`,
  `item.pattern`, `item.editorColor`, `backdrop.image`, `backdrop.size`,
  `image.list` and the bridge's `mesh.new`, `item.select`, `item.visibility`,
  `item.parent`, `item.drop`, `item.transform`, `symmetry.set`,
  `smoothing.set`.

### Left alone, needs a decision

- `vert.splitEdge` / `vert.removeEdge` as aliases of `edge.split` /
  `edge.remove`: COMMAND_NAMES.md records that these are different
  operations (point-based vs edge-based), so an alias would change what
  the old names do. Kept separate.
- `Delete Item` → `item.delete`: the phrase asks before deleting (and
  deletes shading layers in the Shading tab); the headless `item.delete`
  deletes at once. Not made aliases.
- `Falloff`/`Symmetry`/`Action Center` → `falloff.state`/`symmetry.state`/
  `actionCenter.state`: the map's commands only switch on or off; ours pick
  a type, axis or centre. Not made aliases.
- About 180 action phrases (tool starters such as `Cube` → `tool.set
  prim.cube on`, mode switches, view directions) still have no Modo-named
  command. They need the rest of the name map.

### Not built yet

- Generic setters `item.channel`, `pref.value` and the `view3d.*` set, and
  the Viewport Properties rows running them (UIC-164/165). Channel and
  preference keys will be Operandi's own until they're matched to Modo's.
- The integer meanings of the map's enum arguments (`view3d.shadingStyle
  style`, `scene.open mode` …) aren't in the map; string values will be
  accepted alongside (open question).

## Stage 2c: Modo targets for phrases and buttons (MODO-BUTTON-INVOCATIONS.txt)

- Every phrase with a Modo button invocation now records it (`modo`, shown
  in help): `Cube` is `tool.set prim.cube on`, `SDS Subdivide` is
  `poly.subdivide ccsds`, and so on. A test runs every invocation through
  the parser.
- The selection phrases became Modo commands with the phrase as alias:
  `select.expand` (Grow), `select.contract` (Shrink), `select.connect`,
  `select.invert`, `select.boundary`, `select.between`, `select.closeLoop`,
  `select.loop [next|prev]`, `select.ring`, `@ppi_invert_selection.py`
  (Invert Connected).
- `tool.set` takes Modo's ids for tools Operandi starts by phrase
  (`tool.set util.ruler on` runs Ruler; on and off only for now):
  prim.text, prim.curve, prim.bspline, prim.bezier, prim.sketch, patch.draw,
  paste.tool, QuickAlign, GroundAlign, util.ruler, util.protractor,
  util.bounds, xfrm.magnet, xfrm.shear, xfrm.twist, Swirl, xfrm.taper,
  xfrm.pole.
- `tool.doApply` (Modo's Apply Tool) is now an alias of `tool.apply`.
- 13 toolbox buttons now count as running their Modo command (Add Loop,
  Freeze…, the selection set buttons…). Known gaps: 931 (other: 79). The
  toolbox buttons still listed are tools Operandi hasn't built (Bend,
  Vortex, Dicer, Lazy Selection…) plus Collapse, Convert and Mesh Cleanup,
  whose button differs from the Modo command (Mesh Cleanup opens a
  dialog; bare `mesh.cleanup` needs its switches).
- **UIC-165 test:** every command must be a name in the admitted lists
  under specifications/command-names (or recorded in COMMAND_NAMES.md /
  the sculpt spec), a phrase with a known Modo invocation, or marked
  Operandi-only. A new command fails until it is matched or marked. It
  found that `vert.splitEdge` / `vert.removeEdge` aren't Modo names; they're
  now marked Operandi-only.

### Operandi-only today

Operandi-only commands and phrases (106): command.list, operandi.solidSketch.strip, operandi.materialx.import, operandi.materialx.export, operandi.pen.create, operandi.remesh, operandi.sketch.sweep, vert.splitEdge, vert.removeEdge, vert.weld, operandi.view3d.matcap, item.pattern, item.editorColor, mesh.new, item.visibility, item.drop, item.transform, symmetry.set, smoothing.set, backdrop.image, backdrop.size, image.list, operandi.nodeMaterial.import, operandi.nodeMaterial.showGraph, operandi.nodeMaterial.setDefault, operandi.pbview.thumbnailSize, operandi.pbview.kindFilter, operandi.pbview.onlyUsable, selset.ungroup, operandi.bench.orbit, operandi.bench.drag, Cancel, Continue, Tool Properties, Ellipsoid Unit, Ellipsoid New Unit, Smooth Shift Clone, Spike Strength, Transform, Move, Rotate, Scale, Edge Weight, Delete Item, Delete Item Forward, New Mesh, Instance, Toggle Subdivision, Toggle Catmull-Clark, Freeze, Absolute Scaling, Snaps and Precision, Preset Browser, Preset Browser Popover, Detriangulate, UV Unwrap, UV View, Remesh, Symmetrize Positive, Symmetrize Negative, Vertex Split Edge, Vertex Remove Edge, Vertices, Edges, Polygons, Items, Materials, Selection Fill, Select Through, Select Set, Falloff, Symmetry, Action Center, Top View, Front View, Right View, Perspective View, View, Zoom In, Zoom Out, Re-centre, Turn, Turn Upright, Align Selected, Fit and Align Selected, Viewport Properties, Shaded View, Wireframe View, Wireframe Overlay None, Wireframe Overlay Uniform, Wireframe Overlay Colored, Shading, Toggle, Snapping Toggle, Snapping Options, Pie: View Direction, Pie: Selection & Transforms, Pie: Shading Style, Pie: Viewport Toggles, Pie: Falloffs, Pie: Action Centers, Pie: Modeling, Open Render Window, Preview, Preferences, Reset Preferences

Environment images (registry thread, 2026-11-12):

- `operandi.environment.loadImage [filename:PATH] [item:ENV]`
  (Operandi-only): Load Image… on an environment. No filename opens the
  file panel; no item uses the top (newest) environment, made when there
  is none. It replaces the picture of the environment's image layer (its
  top Image Map layer), keeping its rotation, opacity and blend, or adds
  an Image Map layer directly above the Environment Material. One undo
  step.
- `texture.delete item:LAYER`: deletes a texture layer from the Shader
  Tree or an environment, with the Texture Locator no other layer uses.
  Clear Image runs it on the environment's image layer. One undo step.
- Rotation runs `transform.channel rot.Y <angle> item:<locator>` on the
  image layer's Texture Locator (radians; `30deg` accepted). No new
  command: `item.channel` doesn't take transform channels; the existing
  `transform.channel` does.

Environment presets (registry thread, 2026-11-12). An environment preset
is a `.openv` file: a cut-down Operandi scene file with a preset header of
kind Environment, like `.oppreset` (which older builds used for it and is
still read). It holds every environment with its layers and Texture
Locators, optionally every light and the selected meshes, never cameras or
render settings; pictures are named relative to the file when beside or
under it, else absolutely.

- `preset.saveAs item:ENV filename:PATH [lights:BOOL] [meshes:BOOL]
  [cameras:BOOL]`: `lights`, `meshes` and `cameras` are Operandi-only
  extras (default false; meshes = the selected meshes). Never render
  settings. `kind:environment` works as `item:` does. No
  filename opens the save panel; bare (no lights:/meshes:/cameras:) opens
  the Save Environment Preset dialog: Include Lights and Include Selected
  Meshes start ticked, Include Cameras unticked, Copy Images Next To
  Preset ticked; it passes them explicitly. Saving needs an environment
  (the Shader Tree's right-click passes the clicked one as item:; the
  File menu uses the top one).
- `preset.do preset:PATH [mode:replace|add]`: `mode` is an Operandi-only
  extra (default replace; 0/1 are aliases). The preset's environments go
  in as environments; its lights, meshes and cameras arrive in one item
  group named after the preset file, its materials under a Shader Tree
  group of that name, both marked as from that preset (found even when
  renamed). Replace removes every environment with its layers and
  locators and every group an earlier preset left, keeping hand-made
  items outside them; Add puts the preset's environments on top and
  removes nothing. One undo step either way.

MaterialX (registry ruling 2026-11-12):

- `operandi.materialx.import [filename:PATH] [layer:ID]`: no filename
  opens the file panel; the new layers go directly above layer `ID` (in
  its group), or at the top of the shader tree without one. One undo
  step. File > Import MaterialX… passes the selected layer.
- `operandi.materialx.export [filename:PATH] [item:MATERIAL]`: no
  filename opens the save panel; no item uses the material or material
  group selected in the Shading panel (`MATERIAL` is its id or name). Not
  undoable; with nothing suitable it says so.
- `preset.saveAs ... mtlx:<true|false>`: **`mtlx` is an Operandi extra**
  (default off) that also writes a `.mtlx` beside a material preset. The
  Preset Browser gear toggle "Also write MaterialX for material presets"
  sets it on File > Save … Preset and the panels' save buttons. No new
  command.

Most are phrases waiting for Modo names (views, viewport display, pies,
render windows, preferences); the `view3d.*` work covers the viewport ones.

Coming on the library-pack branch (not in this registry yet, so not in
the count above): `operandi.pbview.libraryGet [url:<URL>]` opens the
library web page (v1 doesn't download; no path argument; no undo).
Locate Library… is a folder-picker button that runs the map's
`pbview.basePathAdd`, so it has no command of its own.

Pack UVs runs the map's `uv.pack` (no longer Operandi-only). The
registry thread's names (MeshOps, Remesh, the preset browser, the sculpt
brushes, `tool.*`, Backdrop) are lines in
`specifications/command-names/MODO-NAMES-SUBSET.txt` since 2026-11-11, so
the test's hand-kept RECORDED list is empty.

Naming decisions (registry owner, 2026-11-11):

- `poly.freeze` face value 2 (2 Point Polygons) is Operandi-only.
- Export formats are lowercase extensions: usd, usda, usdc, usdz, abc,
  ply, plyAscii, stl, stlAscii (and obj, gltf, glb, geo, svg, lxo). A
  `scene.saveAs` to a foreign format acts as `export:true`.
- MeshOps saved types: effector.clone.item with pmodel.mirror /
  pmodel.array for Mirror and Array, pmodel.triangulate.item, and
  index.selop.item for Select by Index. **Not done yet:** `meshops::OP_TYPES`
  still saves Mirror and Array as mirror.clone.item / array.clone.item.
  Changing that changes saved scenes (it needs a format version and a
  reader for the old ids), so it's left for its own change (CMD-Q).
- The Node Material names (operandi.nodeMaterial.import, .showGraph,
  .setDefault) are confirmed; shared graphs are scene data, not items, so
  setDefault stays its own command rather than an item.channel.

## item.channel (general setter, UIC-165)

`item.channel name:<channel> value:<value|?> [item:<id>] [layer:<id>] [scene:true]`
sets one channel of an item (default the active item), of a shader-tree
layer (`layer:`, Operandi's own argument) or of the scene itself
(`scene:true`, Operandi's own: `fps`, `start`, `end`, `time`, in seconds;
scene_time.rs), as one undo step, without the
window (MCP `command_run` and typed lines alike). `value:?` answers the
current value and changes nothing. An unknown channel is refused with the
target's channel list. The map's `mode` argument isn't supported yet.

Channel names are **Operandi's own for now** (provisional): they still
need matching to Modo's channel names.

Wired so far:

- **Lights:** `intensity` (float), `color` (colour; `color.r|g|b` one part),
  `visCamera` (bool), `shadowType` (enum none|raytrace), `render` (enum
  default|on|off), `radius` (distance), `dissolve` (percent 0–1),
  `lpeLabel` (text), `prototype` (item, Mesh Light), `volumetric` (bool),
  `volSamples` (int).
- **Kit items** (kit_item.rs, KIT-1): their kit's values read by name
  (`count`, nested ones dotted: `source.file`); setting one is refused
  while the kit is missing. Every-item channels still set.
- **The scene** (`scene:true`): `fps`, `start`, `end`, `time` (seconds,
  provisional names). Every channel reads through how it is held
  (`channels::Held`, AR-08): a constant today.
- **Every item:** `rotOrder` (enum XYZ|ZXY; Item Transform > Rotation
  Order runs it).
- **Lights (every Light Properties value):** also `falloff`, `spread`,
  `length`, `width`, `height`, `shape`, `envOnly`, `visReflection`,
  `visRefraction`, `coneAngle`, `softEdge` (angles in radians), `diffuse`,
  `specular`, `importance`, `opacity` (0–1), `usePrototypeTransform`,
  `blendMode`, `enable` (the light material). The map's own commands cover
  the rest: `item.setType` (light Type), `light.temperature` (Set
  Temperature, kelvins) and `light.mesh` (Mesh Light Prototype). Every Light
  Properties row runs its command (`camera_light_ui::run_light`): core
  builds the one `Command` from the line and the app runs it the usual
  way, so drags and colour picks still fold into one undo step. A test
  checks each kind of light edit gives the same scene by command as
  directly. Item Name rows run `item.name`.
- **Every shader-tree layer (common block):** `enable`, `invert` (bool),
  `blend` (enum of the blend modes), `opacity` (percent 0–1).
- **Environment items** (`environment_channels.rs`): `intensity` (float),
  `visCamera`, `visIndirect`, `visReflection`, `visRefraction` (bool),
  `lpeLabel` (text).
- **Environment Material** (`layer:<its id>`): `enable`, `invert`, `blend`,
  `opacity`; `type` (enum Constant | 2 Color Gradient | 4 Color Gradient |
  Overcast | Physically-based Daylight); `zenColor`, `skyColor`,
  `gndColor`, `nadColor`, `albedo`, `discColor` (colour); `skyExp`,
  `gndExp`, `haze`, `skyGamma`, `fogDensity`, `fogFalloff` (float);
  `discSize`, `discInScatter` (percent); `clampSky` (bool); `fogType` (enum
  None | Linear | Exponential | Underwater | Layered); `fogStart`, `fogEnd`,
  `fogAltitude` (distance). The Sun Light is `environment.light`.
- **Texture layers under an environment** (`layer:<id>`): `enable`,
  `invert`, `blend`, `opacity`, `effect` (enum Environment Color | Driver
  A–D | Group Mask | Layer Mask), `color` and `value` (Constant), `image`
  (Image Map: a scene image's name, or none). Environment layer ids start
  above 2^32, clear of the Shader Tree's.

`item.name <name> layer:<id>` renames a shader-tree layer (environment
layers too) the same way `item.channel` targets one.

**Shader-tree layers' own settings.** Every field of a layer's kind
(material, shader, group, gradient, procedural…) is an `item.channel`
name on `layer:<id>`: its stored name (`diffuse_amount`, `more.dispersion`;
a Render Output's one value is `effect`), in stored units (provisional:
names and units are Operandi's own, to be matched to Modo's). The optional
blocks a save leaves out at their defaults (`more`, a group's `scale`) are
filled in, so every setting has a name. The Shading panel's one-setting
edits run `item.channel` / `item.name` (`replace_layer`): core builds the
Replace from the line, and it is used only when it is exactly the panel's
node, so behaviour can't drift; drags keep folding. A test changes every
number and switch of every layer type, and the common block and name,
and checks the line builds the same node. Render Output settings (Bloom,
Exposure…) are kept with the tree's outputs; they are `item.channel`
names on the output layer too (built as SetOutput).

The map's own commands cover the fields that aren't plain values:
`texture.setIMap image:<name|path|none> layer:<id>`, `texture.setLocator
locator:<id|none> layer:<id>`, `texture.setVMap name:<map|none>
layer:<id>`, `mask.setPTag name:<tag|(all)|(none)> layer:<id>` and
`mask.setPTagType name:<type> layer:<id>` (`layer:` is Operandi's own
argument; the map's `name` on setIMap isn't supported yet). The panel's
Image, Locator, Vertex Map, Polygon Tag and Tag Type rows run them.

**Row evidence:** `shading_rows_driven_through_the_panel_run_their_commands`
draws one layer of each type, clicks and then drags every row through the
panel, and checks that each row that changed the layer ran the command for
that row's own setting. Rows it can't change that way (dropdowns, text)
are counted, not driven; rows that change the layer without a command and
aren't claimed (Ignore Scale Group) stay gaps.

### Every item: Display, Assembly, targeting

`item_common_channels.rs`: visible, label, showLabel, labelOffsetX/Y,
pivot, center, select, lock, target (Set Target; none removes it),
redirect, pickWalkUp/Down/Left/Right, tooltip, userTag, dropAssembly,
dropSource, dropDestination (a Drop Action's label, or `(none)`). The
Display and Assembly rows and every Set Target row run them for one
selected item; with several selected they stay one direct edit (one undo
step), since item.channel sets one item. CMD-Q: should item.channel
without `item:` set every selected item, as one step?

### How to add channels

Channels live in `crates/operandi-core/src/channels.rs`. The dispatcher
(`find`, `get`, `set`, value parsing) never changes for a new channel:

1. Write a table `static MY_KIND: &[Channel] = &[...]` (or add entries to
   an existing one). Each `Channel` has:
   - `name`: the channel's script name;
   - `ty`: `Bool`, `Int`, `Float`, `Text`, `Enum(&[names])`, `Colour`,
     `ItemRef`, `Distance` (metres, units accepted) or `Percent` (0–1 at
     the boundary, `50%` accepted). The value reaches `set` already parsed
     (percent as a fraction, distance in metres, enum as an index);
   - `applies(scene, target)`: does this target have the channel (e.g. is
     the item an environment, or the layer an environment material);
   - `get(scene, target)`: the current value;
   - `set(scene, target, value)`: the **one** `Command` that makes the
     change (dispatched as one undo step; the editor validates it).
2. Add the table to `TABLES`.
3. Add a line to the channel test (`every_value_type_sets_and_reads_back`)
   for each new channel, and list the names here.

Colour parts (`.r/.g/.b`) and `?` work for every channel without extra
code. Targets are `Target::Item(id)` and `Target::Layer(node id)`.

## pref.value (general setter, UIC-165)

`pref.value <name> <value|?>` sets one preference and saves it the way
the Preferences window does; `?` answers the value. Keys are **Operandi's
own for now** (provisional, to be matched to Modo's preference names).
Percents are 0–1 at the command boundary.

The window runs `pref.value` for every row edit on a page that has
entries in `crates/operandi-app/src/prefs_commands.rs` (`PREFS`): after a
page draws, each changed value is put back and set again through the
command. Wired: the Remapping page (14 rows: `input.device`,
`input.tooltipDelay`, `input.wheelAcceleration`, `input.wheelZoomAtCursor`,
`input.invertWheelZoom`, `input.zoomDragAxis`, `input.reversePanWidget`,
`input.invertWheelScrolling`, `input.keypadEnterAsTab`,
`input.dialogsUnderMouse`, `input.trackball`, `input.trackballOverride`,
`input.trackballSpeedMouse`, `input.trackballSpeedTablet`).

**Every other page goes by its preference group** (`GROUPS`): any field
of the group a page saves is `pref.value <group>.<field>` (nested fields
joined with dots), e.g. `pref.value application.<field> 50`. Groups:
`input`, `selection`, `units`, `display`, `appearance`, `handles`,
`defaults`, `application`, `meshItems`, `ui`, `rendering`, `renderWindow`,
`fileIo`. Values
are JSON-like text; percents and angles are in **map units** (0–1 and
radians; `50%` and `30deg` also work) and converted to what the field
stores (`prefs_commands::UNITS`; a test reads the pages for every Percent
or Angle row and round-trips each through the command). Ranged rows are
checked against the same constants their Preferences rows use
(`prefs_commands::ranges()`), and an out-of-range value is refused with the
range in map units. A test lists every ranged row from the page sources
and tries each at its max and just outside both ends. The Rendering page's
Render Window section is its own group (`renderWindow`). A wrong type is refused and
changes nothing. After a page draws, the window compares its group before
and after, puts it back, and runs `pref.value` for each changed field, so
every row of the group runs the command.

**Colours:** `pref.value color.<key> <#rrggbb|default|?>`, one key per
Display > Colors colour, generated from the colour table (`theme::COLORS`
keys such as `color.viewport.background`).

Still click-only on the Preferences pages (15): the buttons (Choose…,
Clear, Browse…, Default, Input Map…, the colour Scheme row) and five rows
kept outside their page's group: Inner/Outer Range (snapping), Max Samples
(the Preview), Item Drawing: Current Scene (the scene) and Lasso: Allow
Partial Element Selection. Known gaps overall: 657.

To add a page by hand instead: add one `Pref` per row (key, page, row label, value type,
get, set in memory, and the `after` hook that saves the page's group, as
the page's own code does). No window code changes: the page's rows start
running the command as soon as their entries exist. Known gaps: 874
(prefs 232).

## Headless additions (branch mcp-gaps, 2026-11-11)

Now `headless` with a runner in `command_exec`: `poly.freeze`,
`scale.absolute`, `vertMap.new`, `select.vertexMap`,
`operandi.nodeMaterial.import`, `operandi.nodeMaterial.setDefault` and
`scene.saveAs` (export only: filename plus format; the save panel and
plain .opscene saves stay the app's). Typed lines still reach the app's own handlers first (dialogs when bare);
the runner is what `command_run` and scripts without a window use. No names
changed. `sculpt.liveWeightEdit` stays window-only: its setting is the
app's sculpt session state.

## Typed parsing in core: where it stopped, and why

Core's `command_line.rs` holds the shared tokenizer, the typed-argument
runner for every headless command, and the argument parsers of the
vertex, sculpt, `tool.set`, `vert.set`, `chamfer.edit`, `poly.setPart`,
`poly.subdivide`/`unsubdiv`, Backdrop and selection-set commands. What
stays in the app is choosing which app action a command name runs:
opening dialogs, panels and pies, starting tools and changing views.
Those actions only exist with the window, so moving that dispatch into
core would add indirection without letting anything more run headless.
The few argument parsers left in the app are small (the legacy smooth
shift/bridge setter phrases, `edge.addLoop position`, `poly.unify force`).

## tool.attr for every Tool Properties field

`tool.attr <tool> <field> <value|?>` now reaches every data-driven Tool
Properties field (`tool_form::ToolFields` panels), not just the
hand-written Spike, Smooth Shift, Bridge, Edge Bevel/Chamfer and Polygon
Reduction settings. The field is found by its key or label (spaces and
case ignored); numbers take map units (percent 0–1, radians; `50%`,
`30deg` and the field's own units work too), choices take their names or
indexes, and the value goes through the panel's own publish route, so a
refusal is the panel's. `<tool>` must name the active tool. A test
activates every tool, reads every field with `?` and sets it to its own
value. UIC-164: tool-property gaps 150 → 0; a row counts only when it is a field of the tool whose panel drew it.

## File menu (scene.*, preset.*)

The File menu runs the map's commands (`file_commands.rs`):
New `scene.new`, Open… `scene.open` (bare opens the panel), Save
`scene.save`, Save As… `scene.saveAs`, each Export entry `scene.saveAs
format:<f> export:true`, Import… `scene.importReference`, Close Scene
`scene.close`, Close All… `scene.closeAll`; Load Preset… `preset.do`, Save
Item/Assembly/Environment Preset… `preset.saveAs [kind:…]`. With a
`filename:` they act without a panel.

Provisional:
- `format:` values (obj, stl, stlAscii, gltf, glb, geo, svg, lxo) are
  Operandi's own; the map names no format strings. scene.save's `mode` and
  `format`, scene.saveAs's `index` and scene.open's `mode` aren't supported.
- `scene.importReference [filename:PATH] [meshes] [shaders] [lights]
  [environments] [cameras]` (registry ruling 2026-11-12, preset browser
  PG-07): adds the file's items to the **current** scene, selected, one
  undo step; the format is picked by extension (Operandi and Modo scenes,
  the interchange formats). The five switches are booleans, default true.
  No filename opens the file panel. File ▸ Import… and the Preset Browser's
  Import Into Current Scene run it; opening in a new tab is `scene.open`.
- No map name for: switching to an open scene (File ▸ Current), projects
  (New/Set/Recent Project, Project Presets). They stay gaps until the map
  has names.

Preset browser P1 (registry ruling 2026-11-12):

- `operandi.view3d.matcap ... all:<true|false>` (also 1/0): **an Operandi
  extra**. True sets the matcap in every 3D view and switches each to the
  Matcap style (the browser's Use in All 3D Views, MC-06).
- `preset.saveAs ... centerOnGround:<true|false>`: **an Operandi extra**
  (default false, American spelling like the map's center args): the saved
  copy of an item preset is centred on X/Z and rests on Y = 0 (PG-04). The
  browser's Save Mesh Preset dialog has it as Centre on Ground.

## Later stages (separate branches, after this is reviewed)

2. Move typed parsing into core and dispatch from the registry (one parser
   instead of ~12); the MCP bridge reads the registry instead of its own
   tables (L6). Fold action phrases that are exact twins of script commands
   (`Flip` / `poly.flip`) into aliases.
3. Query commands for what panels show (`select.count`, item and channel
   values; UIC-163), and API version reporting over MCP.
4. Backfill click-only controls area by area (lights, cameras, shader tree,
   textures, viewport options, preferences, tool properties, menus) until
   `command_registry_gaps.rs` is empty; record hand-drawn controls too.

## poly.freeze Curves As values (2026-11-11)

`poly.freeze face:` takes 0 = Faces and 1 = Polylines, the reference's two
values. **2 = 2 Point Polygons (`twoPointPolygons`) is an Operandi
addition**: the reference's scripted form has no value for the dialog's
third choice. Any other value is rejected (the reference leaves the curve
and adds stray points). Line in `MODO-NAMES-SUBSET.txt`.

## pbview.mode values (branch browser-views, 2026-11-11)

`pbview.mode mode:` takes 0 = Vertical (named `thumbnails` before the
Horizontal view existed; still accepted), 1 = List, 2 = Horizontal, or the
names `vertical`, `list`, `horizontal`; `mode:?` answers with the name.
These integers are fixed for good in Operandi (scripts store them). **The
reference app's integer meanings for `pbview.mode` are not confirmed yet**;
if they turn out different, the names stay and it becomes a spec question
rather than a renumbering.

## Remesh (branch remesh, 2026-11-11)

Names from the registry thread:

- `mesh.retopo` (Automatic Retopology): reference name and arguments
  `countMode number|percent`, `polyTargetNum`, `polyTargetPct`,
  `creaseAngle` (radians, 0 off), `boundaryAlign`, `vertNormals`, `newMesh`.
  **`seed` is Operandi-only.** `enableSymmetry` and `quadsOnly:false` are
  refused (not built); `scalingStrength`, `symmetryAxis`,
  `symmetryThreshold`, `symmetrySide` aren't accepted.
- `sculpt.remesh` (Voxel Remesh): reference name, `density` 0–1 (higher is
  finer). **`voxelSize`, `keepMaps` and `vertNormals` are Operandi-only.**
- `operandi.remesh` (Blocks / Smooth / Sharp, Operandi-only): `mode
  blocks|smooth|sharp`, `depth`, `scale`, `sharpness`, `removeDisconnected`,
  `threshold`, `vertNormals`.
- Phrases: `Remesh` (Operandi-only) and `Automatic Retopology`
  (`mesh.retopo`) open the dialogs, as do the bare commands.

## MeshOps (registry MESHOPS-NAMES rev 3)

- Ops aren't scene items in Operandi; scripts name one `op:<item>:<op>`
  (`op:<item>:base` is the Base Mesh row). `item.delete`, `item.parent
  position:`, `item.channel`, `select.item`, `deformer.enable`,
  `deformer.freeze`, `selop.selection` take it.
- Mirror and Array are one op here and two items in files: a Clone
  Effector (`effector.clone.item`, channels `merge`, `dist`) with a
  `pmodel.mirror` / `pmodel.array` generator in its Tool Pipe (the other
  channels). `item.channel` takes the **effector** (`item:op:<item>:<op>`)
  for every channel of both, effector's and generator's; there is no
  separate generator id to address.
- `mirror.clone.create` / `array.clone.create` add them (same as
  `meshop.create type:mirror.clone.item` / `array.clone.item`, the menu
  entries, which are never saved).

## Stage 3: commands carry their handler (architecture review 2026-11-14, pick E)

- **One list.** `command_exec::SPECS` is gone: each headless entry in
  `COMMANDS` carries its JSON arguments, script description and handler
  (`Entry::json`, built with `.headless(about, args)`).
  `command_exec::specs()` / `spec()` read them for MCP help, the
  `command_run` schema and docs/MCP_BRIDGE.md (now in registry order).
- **Run-time registry** (`operandi-core/src/command_runtime.rs`):
  `Registry<H>` is built from `COMMANDS` at start-up. The host attaches
  handlers to built-in commands (`handle`, two different handlers for one
  command are refused) and adds its own window-only commands
  (`add_host_command`, flagged "needs the window"). Kits and plugins use
  `register(namespace, entry, handler)`: every name must be
  `namespace.something`, and built-in names, old spellings and every
  namespace a built-in uses (plus operandi, app, kit, plugin, cmds,
  command) are refused. A kit's headless command runs through
  `Registry::command_for` like a built-in.
- **The app runs through it.** `execute` (main.rs) calls
  `run_registered` right after the name rewrites. The old typed `match`
  (about 170 phrases) is now handlers in `operandi-app/src/command_handlers.rs`;
  the benchmarks `operandi.bench.orbit` / `.drag` moved out of core's
  table and register from the app. `help` and `command.list` read the
  run-time registry.
- **What a handler gets.** A command without arguments takes only its bare
  line; required arguments must be present and a required choice must be
  one of its choices, else the line goes on to the older parsers (which
  report it as before). A Modo-named command typed as its phrase takes no
  arguments. On a headless command a host handler takes only the bare line
  (the Delete key's `delete`); lines with words run on the editor.
- **Still on the old chain** (move a few at a time): the sculpt lines,
  `canonical_command` and its ten sub-parsers, `pen`, Smooth Shift /
  Spike / Bridge attributes, `open`, `select set`, the pie phrases, and the
  headless path (`command_line`). Built-in headless commands share one
  editor handler (`command_exec::built_in`, a `match` on the name); kit
  commands bring their own. MCP `command_run` and typed headless lines
  still use the static table, so kit headless commands don't reach them
  yet. API_VERSION stays 1 (no name or argument changed meaning).
- **Tests.** Core `tests/command_runtime.rs` (built-in table, handlers,
  host commands, namespaces, a kit headless command). App
  `registry_handlers_take_lines_no_older_parser_takes` (a handled line has
  one owner) and `app_commands_need_the_window`; the existing "every
  registry example runs" test now walks the run-time registry.
