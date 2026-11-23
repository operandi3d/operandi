# Command names waiting for the registry thread

Placeholders under `operandi.*` until the registry thread names them
(AGENTS.md "Command, tool and argument names follow the reference name
map"). Each is registered in `command_registry.rs` and has a Help line.

Layouts stage 2 (layouts-2026-11-16 stage 2): the name map has
`layout.restore`, `layout.lockThumbs` and `layout.lockDividers` (used as
is) and nothing for these, so they are placeholders:

| Placeholder | What it does |
|---|---|
| `operandi.layout.create` (`copy:`) | New layout, blank or a copy |
| `operandi.layout.save` / `operandi.layout.saveAs` | Save Layout / Save Layout As |
| `operandi.layout.rename` / `operandi.layout.delete` | Rename / Delete a user layout |
| `operandi.layout.revert` (alias `operandi.layout.reset`) | Reset to the shipped default |
| `operandi.layout.clear` | Clear Layout |
| `operandi.layout.star` / `.starredOnly` / `.collapse` / `.move` | The bar's stars, filter, collapse and order |
| `operandi.layout.switcher` | Recent Layouts (quick switcher) |
| `operandi.viewport.saveAs` / `.save` / `.use` / `.revert` / `.delete` | Saved viewports and groups (pieces) |
| `operandi.panel.kind` | The content arrow: what a slot shows |

Input Map actions: "Layout Switcher" (Ctrl+Shift+Tab) and "Pie: Layouts"
(Ctrl+`).

Help menu (Jason 2026-11-16): nothing in the name map, so placeholders:

| Placeholder | What it does |
|---|---|
| `operandi.help.reportBug` | Help > Report Bug: a pre-filled GitHub issue in the browser |
| `operandi.help.documentation` | Help > Documentation: the docs in the browser (greyed "Coming soon" until `help_menu::DOCS_URL` is set) |
| `operandi.update.check` | Help > Check for Updates: the newest build on the update channel, offered if newer (2026-11-23) |

UV editing stage 1 (uv-editing-2026-11-16): the name map has
`vertMap.new` (now also `type:uv`) but nothing for these, so they are
placeholders:

| Placeholder | What it does |
|---|---|
| `operandi.uvMap.select` | Select a UV map of the active mesh |
| `operandi.uvMap.rename` | Rename a UV map |
| `operandi.uvMap.delete` | Delete one or several UV maps |
| `operandi.uvMap.copy` / `operandi.uvMap.paste` | Copy a whole map, paste it over another |
| `operandi.uv.fit` | Fit UVs |
| `operandi.uv.flip` | Flip U / V |
| `operandi.uv.mirror` | Mirror U / V about a Center |
| `operandi.uv.align` | Axis Align Up / Down / Left / Right, Align UVs |

The E7 buttons are also tool preset rows (`uv.fitUVs`, `uv.flipU`,
`uv.flipV`, `uv.mirrorU`, `uv.mirrorV`, `uv.alignUp`, `uv.alignDown`,
`uv.alignLeft`, `uv.alignRight`), Operandi ids until the map names them.

UV tools finish (2026-11-18): the name map has the `uv.unwrap`,
`uv.relax` and `uv.pelt` tools (used as their `tool.set` ids), but nothing
for these, so they are placeholders:

| Placeholder | What it does |
|---|---|
| `operandi.uv.project` (tool) | Project: Planar, Cylindrical, Spherical, Barycentric, Cubic |
| `operandi.uv.viewProject` (tool) | Project from View |
| `operandi.uv.quickAlign` / `islandAlign` / `orient` / `toOrigin` | Quick Align, Island Align, Orient Pieces, To Origin |
| `operandi.uv.texel` / `realWorldSize` | Texel Density buttons, Real World Size |
| `operandi.uv.split` / `sew` / `moveSew` | Split, Sew, Move and Sew |
| `operandi.uv.selectBorder` / `selectErrors` / `selectUnmapped` | UV selections |
| `operandi.uv.copy` / `cut` / `paste` / `delete` / `setValue` / `rotatePolygons` / `mergeMaps` | UV values |
| `operandi.uv.dialog <kind>` | Opens one of those buttons' options |
| `operandi.uv.seam action:add/clear/select/delete/marker/use name` | Cut Map's seam map buttons, Seam Marker |
| `operandi.uv.unwrapSeams [pelt]` | Cut Map's Unwrap Tool / Unwrap & Relax |
| `operandi.uv.symmetry axis center` / `symmetryCenter` / `symmetrize` | UV Symmetry layer, Center, Symmetrize |
| `operandi.uv.selectTile` / `nudgeTile` | UDIM Indicator's region select and arrows |
| `operandi.uv.peeler` (tool) | Peeler |
| `operandi.uv.boxTransform` (tool) | Box Transform |
| `operandi.uv.brush [split\|sew\|off]` | Split and Sew brush |
| `operandi.uv.export format filename size` / `toMesh` / `transfer` | Export UVs to SVG / EPS, Convert UVs to Mesh, Transfer UVs |

Layouts stage 3 (2026-11-16) gives each E7 button a command line,
`operandi.uv.preset <id> [plain|shift|alt|options]`, so toolbox forms can
hold them, and adds `operandi.form.revert <form id>` (LAY-87). Both are
Operandi names until the map names them. Batch 2 adds
`operandi.notBuilt <name>`, a toolbox placeholder for a command Operandi
doesn't have yet.

Layouts stage 4 (2026-11-17, workbench edit mode, LAY-82): nothing in the
name map, so placeholders:

| Placeholder | What it does |
|---|---|
| `operandi.workbench.edit` (`on:`, `form:`) | Edit Workbench: shows or hides a workbench's edit tools |
| `operandi.workbench.pick` (`on:`) | Pick to Add |
| `operandi.workbench.add` / `.addProperty` / `.addDivider` | Add a button, a Tool Properties row, a divider |
| `operandi.workbench.addPipe` (`as:` preset, macro or script) | Add the current tool pipe as a preset |
| `operandi.workbench.move` / `.set` / `.chooseIcon` | Reorder; set a right-click property; Custom Icon... |
| `operandi.workbench.duplicate` / `.delete` / `.undo` | Duplicate, Delete, Undo Edit |
| `operandi.workbench.topBar` (`on:`) | Show or hide the workbench row along the top |

Layouts stage 5 (2026-11-17, per-layout key maps, LAY-20): nothing in the
name map, so placeholders (class "ui"):

| Placeholder | What it does |
|---|---|
| `operandi.layout.setKey` (`action:`, `key:`, `layout:`) | Set one key in a layout's own key map; `key:none` unbinds it there, no `key:` gives it back to the global map |
| `operandi.layout.clearKeys` (`layout:`) | Clear a layout's own keys |
| `operandi.layout.revert` gains `keys:` | `keys:true` also clears the layout's keys (they stay otherwise, LAY-Q56) |

Layouts stage 6 (2026-11-17, palettes, LAY-50..55): `layout.window` (in
the map) now takes every palette's `hash:`; one placeholder (class "ui"):

| Placeholder | What it does |
|---|---|
| `operandi.layout.togglePalettes` | Toggle Palettes (`): hide every palette, or show them again |

Layouts stage 7 (2026-11-17, the Form Editor, LAY-83): nothing in the
name map, so placeholders (class "ui"); `layout.window` takes
`hash:formEditor`:

| Placeholder | What it does |
|---|---|
| `operandi.form.newGroup` (`name:`) | New Group in the Form Editor's tree |
| `operandi.form.new` (`id:`, `label:`, `style:`, `group:`) | New Form |
| `operandi.form.add` / `.addForm` / `.addDivider` | Add a command, a sub-form, a divider |
| `operandi.form.move` / `.delete` | Reorder (the tree's drag); delete a control |
| `operandi.form.set` (`attr:`, `value:`, `index:`, `form:`) | Set a form's or control's property |
| `operandi.form.select` (`form:`, `index:`) | Show in Form Editor |
| `operandi.form.find` / `.preview` (`on:`) | Find Form; Preview Form |
| `operandi.form.export` (`form:`, `filename:`) | Save Form to its own file |

Menus as forms (2026-11-17, LAY-Q74): a command for every menu entry
that had none. Nothing in the name map, so placeholders, with their
class:

| Placeholder | Class | What it does |
|---|---|---|
| `operandi.scene.current` (`index:`) | files | File > Current: make an open scene current |
| `operandi.project.new` / `.set` (`path:`, `name:`) | files | New Project / Set Project (no path: the dialog) |
| `operandi.project.openRecent` (`index:`) | files | File > Recent Projects |
| `operandi.project.loadPreset` / `.savePreset` (`preset:`) | presets | File > Project Presets |
| `operandi.inputMap` | preferences | Edit > Input Map |
| `operandi.select.polygonTagType` (`type:`) | selection | Select > Polygon Tag Type |
| `operandi.lasso.style` (`style:`) / `operandi.lasso.partial` (`on:`) | selection | Select > Lasso Style |
| `operandi.view.showLocked` / `.showFalloffs` (`on:`) | viewport options | View > Show Locked Mesh Items / Show Falloffs |
| `operandi.viewport.close` | view | View > Close View |
| `operandi.help.navigation` | help | View > Navigation Help |
| `operandi.window.diagnostics` / `.frameTime` (`on:`) | debug | Window > Diagnostics / Frame Time |
| `operandi.mcp.bridge` (`on:`) | debug | Window > MCP Bridge |
| `operandi.window.size` (`size:`) | view | Window > Compact / Standard |

Two came from the name map: `viewport.split` (View > Split View) and
`viewport.detach` (View > Tear Off View). `operandi.layout.delete` and
`.revert` gained `ask:true` (the menu's confirmation).

TextureCraft pieces (2026-11-17): Operandi-only, nothing in the name
map, so placeholders:

| Placeholder | Class | What it does |
|---|---|---|
| `operandi.uv.atlas` (`name:`, `angle:`, `gaps:`, `resolution:`) | undoable | Texture > Atlas Unwrap…: automatic charts packed at one density |
| `operandi.bake.curvature` (`filename:`, `size:`, `padding:`, `fill:`, `uvMap:`, `rays:`, `radius:`) | side effect | Texture > Bake Curvature…: 16-bit PNG |
| `operandi.bake.thickness` (… `distance:` instead of `radius:`) | side effect | Texture > Bake Thickness… |
| `operandi.bake.padImage` (`filename:`, `output:`, `padding:`, `uvMap:`) | side effect | Texture > Pad Texture to UVs… (the menu writes `<name>_padded.png`) |

The MaterialX commands and the `preset.saveAs mtlx:` extra were ruled on
2026-11-12; see "Operandi-only today" in `docs/COMMAND_REGISTRY.md`.

Node editing (A3, NE-01..NE-19; node_view_commands.rs): nothing in the
name map yet, so placeholders. Wire targets are `<node>.<input>` or
`layer:<id>.<channel>`.

| Placeholder | What it does |
|---|---|
| `operandi.nodes.add` | Add a free MaterialX node to a Material Group's node view |
| `operandi.nodes.connect` / `.disconnect` | Wire a free node's output into an input / remove that wire |
| `operandi.nodes.delete` | Delete free nodes or layer boxes (mixing nodes refused) |
| `operandi.nodes.setValue` | A free node's input value |
| `operandi.nodes.move` | Place a box |
| `operandi.nodes.hide` / `.show` / `.showAll` | Hide boxes in the view, show them again |
